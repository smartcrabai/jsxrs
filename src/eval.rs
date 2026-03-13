use serde_json::Value;
use swc_ecma_ast::{BinExpr, BinaryOp, Expr, Lit, MemberExpr, MemberProp, Tpl};

use crate::error::JsxrsError;

/// Scope for evaluating expressions. Holds props and local variables (e.g. map callback param).
#[derive(Debug, Clone)]
pub struct EvalContext {
    props: Value,
    locals: Vec<(String, Value)>,
}

impl EvalContext {
    pub fn new(props: Value) -> Self {
        Self {
            props,
            locals: Vec::new(),
        }
    }

    pub fn with_local(&self, name: String, value: Value) -> Self {
        let mut ctx = self.clone();
        ctx.locals.push((name, value));
        ctx
    }

    fn lookup_local(&self, name: &str) -> Option<&Value> {
        self.locals
            .iter()
            .rev()
            .find_map(|(k, v)| if k == name { Some(v) } else { None })
    }
}

/// If `val` is a `{ "__html": "..." }` object, return the raw HTML string.
/// This sentinel is used to pass pre-rendered HTML through the layout chain
/// without escaping.
pub fn extract_raw_html(val: &Value) -> Option<&str> {
    if let Value::Object(map) = val
        && let Some(Value::String(raw)) = map.get("__html")
    {
        return Some(raw.as_str());
    }
    None
}

/// JS truthiness rules: null, false, 0, "" are falsy.
pub fn is_truthy(val: &Value) -> bool {
    match val {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

pub fn value_to_string(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Number(n) => format_number(n.as_f64().unwrap_or(f64::NAN)),
        Value::Bool(b) => b.to_string(),
        Value::Null | Value::Array(_) | Value::Object(_) => String::new(),
    }
}

fn format_number(f: f64) -> String {
    if f.is_nan() {
        return "NaN".to_string();
    }
    // 9_007_199_254_740_992.0 == 2^53, the max integer exactly representable as f64.
    // Within this range, fract() == 0.0 guarantees the value is an exact safe integer.
    if f.fract() == 0.0 && f.abs() < 9_007_199_254_740_992.0 {
        // Format as float with no decimal point then strip trailing ".0" suffix
        // so that e.g. 42.0 becomes "42" rather than "42.0".
        let s = format!("{f:.0}");
        return s;
    }
    format!("{f}")
}

/// JS `ToNumber` coercion: `true`→1, `false`→0, `null`→0, string→parse or NaN.
fn to_numeric(val: &Value) -> f64 {
    match val {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Null => 0.0,
        Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                0.0
            } else {
                trimmed.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        Value::Array(_) | Value::Object(_) => f64::NAN,
    }
}

pub fn eval_expr(expr: &Expr, ctx: &EvalContext) -> Result<Value, JsxrsError> {
    match expr {
        Expr::Lit(lit) => eval_lit(lit),
        Expr::Ident(id) => eval_ident(&id.sym, ctx),
        Expr::Member(member) => eval_member(member, ctx),
        Expr::Bin(bin) => eval_bin(bin, ctx),
        Expr::Cond(cond) => eval_cond(cond, ctx),
        Expr::Unary(unary) => eval_unary(unary, ctx),
        Expr::Tpl(tpl) => eval_template(tpl, ctx),
        Expr::Paren(paren) => eval_expr(&paren.expr, ctx),
        _ => Err(JsxrsError::Unsupported(format!(
            "expression type: {expr:?}"
        ))),
    }
}

fn eval_lit(lit: &Lit) -> Result<Value, JsxrsError> {
    match lit {
        Lit::Str(s) => Ok(Value::String(s.value.to_string_lossy().into_owned())),
        Lit::Num(n) => Ok(serde_json::Number::from_f64(n.value)
            .map_or(Value::Null, Value::Number)),
        Lit::Bool(b) => Ok(Value::Bool(b.value)),
        Lit::Null(_) => Ok(Value::Null),
        _ => Err(JsxrsError::Unsupported("literal type".into())),
    }
}

fn eval_ident(name: &str, ctx: &EvalContext) -> Result<Value, JsxrsError> {
    if let Some(val) = ctx.lookup_local(name) {
        return Ok(val.clone());
    }
    if name == "props" {
        return Ok(ctx.props.clone());
    }
    Err(JsxrsError::UndefinedProp(name.into()))
}

fn eval_member(member: &MemberExpr, ctx: &EvalContext) -> Result<Value, JsxrsError> {
    let obj = eval_expr(&member.obj, ctx)?;
    let key = match &member.prop {
        MemberProp::Ident(id) => id.sym.to_string(),
        MemberProp::Computed(c) => {
            let val = eval_expr(&c.expr, ctx)?;
            value_to_string(&val)
        }
        MemberProp::PrivateName(_) => return Err(JsxrsError::Unsupported("member property type".into())),
    };
    access_value(&obj, &key)
}

fn access_value(obj: &Value, key: &str) -> Result<Value, JsxrsError> {
    match obj {
        Value::Object(map) => match map.get(key) {
            Some(v) => Ok(v.clone()),
            None => Err(JsxrsError::UndefinedProp(key.into())),
        },
        Value::Array(arr) => {
            if let Ok(idx) = key.parse::<usize>() {
                arr.get(idx)
                    .cloned()
                    .ok_or_else(|| JsxrsError::UndefinedProp(key.into()))
            } else {
                Err(JsxrsError::UndefinedProp(key.into()))
            }
        }
        _ => Err(JsxrsError::UndefinedProp(key.into())),
    }
}

fn eval_bin(bin: &BinExpr, ctx: &EvalContext) -> Result<Value, JsxrsError> {
    match bin.op {
        BinaryOp::LogicalAnd => eval_logical_and(&bin.left, &bin.right, ctx),
        BinaryOp::LogicalOr => eval_logical_or(&bin.left, &bin.right, ctx),
        BinaryOp::EqEqEq | BinaryOp::EqEq => eval_equality(&bin.left, &bin.right, ctx),
        BinaryOp::NotEqEq | BinaryOp::NotEq => {
            let eq = eval_equality(&bin.left, &bin.right, ctx)?;
            Ok(Value::Bool(!eq.as_bool().unwrap_or(false)))
        }
        BinaryOp::Gt => eval_comparison(&bin.left, &bin.right, ctx, |a, b| a > b),
        BinaryOp::GtEq => eval_comparison(&bin.left, &bin.right, ctx, |a, b| a >= b),
        BinaryOp::Lt => eval_comparison(&bin.left, &bin.right, ctx, |a, b| a < b),
        BinaryOp::LtEq => eval_comparison(&bin.left, &bin.right, ctx, |a, b| a <= b),
        BinaryOp::Add => eval_add(&bin.left, &bin.right, ctx),
        _ => Err(JsxrsError::Unsupported(format!(
            "binary operator: {:?}",
            bin.op
        ))),
    }
}

fn eval_logical_and(left: &Expr, right: &Expr, ctx: &EvalContext) -> Result<Value, JsxrsError> {
    let lval = eval_expr(left, ctx)?;
    if !is_truthy(&lval) {
        return Ok(lval);
    }
    eval_expr(right, ctx)
}

fn eval_logical_or(left: &Expr, right: &Expr, ctx: &EvalContext) -> Result<Value, JsxrsError> {
    let lval = eval_expr(left, ctx)?;
    if is_truthy(&lval) {
        return Ok(lval);
    }
    eval_expr(right, ctx)
}

fn eval_equality(left: &Expr, right: &Expr, ctx: &EvalContext) -> Result<Value, JsxrsError> {
    let l = eval_expr(left, ctx)?;
    let r = eval_expr(right, ctx)?;
    Ok(Value::Bool(l == r))
}

fn eval_comparison(
    left: &Expr,
    right: &Expr,
    ctx: &EvalContext,
    cmp: fn(f64, f64) -> bool,
) -> Result<Value, JsxrsError> {
    let l = eval_expr(left, ctx)?;
    let r = eval_expr(right, ctx)?;
    let ln = to_numeric(&l);
    let rn = to_numeric(&r);
    if ln.is_nan() || rn.is_nan() {
        return Ok(Value::Bool(false));
    }
    Ok(Value::Bool(cmp(ln, rn)))
}

fn eval_add(left: &Expr, right: &Expr, ctx: &EvalContext) -> Result<Value, JsxrsError> {
    let l = eval_expr(left, ctx)?;
    let r = eval_expr(right, ctx)?;
    if l.is_string() || r.is_string() {
        let s = format!("{}{}", value_to_string(&l), value_to_string(&r));
        return Ok(Value::String(s));
    }
    let ln = to_numeric(&l);
    let rn = to_numeric(&r);
    Ok(serde_json::Number::from_f64(ln + rn)
        .map_or(Value::Null, Value::Number))
}

fn eval_cond(cond: &swc_ecma_ast::CondExpr, ctx: &EvalContext) -> Result<Value, JsxrsError> {
    let test = eval_expr(&cond.test, ctx)?;
    if is_truthy(&test) {
        eval_expr(&cond.cons, ctx)
    } else {
        eval_expr(&cond.alt, ctx)
    }
}

fn eval_unary(unary: &swc_ecma_ast::UnaryExpr, ctx: &EvalContext) -> Result<Value, JsxrsError> {
    match unary.op {
        swc_ecma_ast::UnaryOp::Bang => {
            let val = eval_expr(&unary.arg, ctx)?;
            Ok(Value::Bool(!is_truthy(&val)))
        }
        swc_ecma_ast::UnaryOp::Minus => {
            let val = eval_expr(&unary.arg, ctx)?;
            let n = to_numeric(&val);
            Ok(serde_json::Number::from_f64(-n)
                .map_or(Value::Null, Value::Number))
        }
        swc_ecma_ast::UnaryOp::Plus => {
            let val = eval_expr(&unary.arg, ctx)?;
            let n = to_numeric(&val);
            Ok(serde_json::Number::from_f64(n)
                .map_or(Value::Null, Value::Number))
        }
        _ => Err(JsxrsError::Unsupported(format!(
            "unary operator: {:?}",
            unary.op
        ))),
    }
}

fn eval_template(tpl: &Tpl, ctx: &EvalContext) -> Result<Value, JsxrsError> {
    let mut result = String::new();
    for (i, quasi) in tpl.quasis.iter().enumerate() {
        result.push_str(&quasi.raw);
        if let Some(expr) = tpl.exprs.get(i) {
            let val = eval_expr(expr, ctx)?;
            result.push_str(&value_to_string(&val));
        }
    }
    Ok(Value::String(result))
}
