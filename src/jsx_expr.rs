use std::collections::HashMap;

use serde_json::Value;
use swc_ecma_ast::{BinaryOp, CallExpr, Callee, Expr, MemberProp};

use crate::config::RenderConfig;
use crate::error::JsxrsError;
use crate::escape;
use crate::eval::{self, EvalContext};
use crate::renderer::{self, RenderState};

pub(crate) fn render_jsx_expression(
    expr: &Expr,
    ctx: &EvalContext,
    config: &RenderConfig,
    imports: &HashMap<String, String>,
    state: &mut RenderState,
) -> Result<String, JsxrsError> {
    match expr {
        Expr::JSXElement(el) => renderer::render_element(el, ctx, config, imports, state),
        Expr::JSXFragment(f) => renderer::render_fragment(f, ctx, config, imports, state),
        Expr::Cond(c) => {
            let test = eval::eval_expr(&c.test, ctx)?;
            let branch = if eval::is_truthy(&test) {
                &c.cons
            } else {
                &c.alt
            };
            render_jsx_expression(branch, ctx, config, imports, state)
        }
        Expr::Bin(b) if b.op == BinaryOp::LogicalAnd => {
            let lval = eval::eval_expr(&b.left, ctx)?;
            if !eval::is_truthy(&lval) {
                return Ok(String::new());
            }
            render_jsx_expression(&b.right, ctx, config, imports, state)
        }
        Expr::Bin(b) if b.op == BinaryOp::LogicalOr => {
            let lval = eval::eval_expr(&b.left, ctx)?;
            if eval::is_truthy(&lval) {
                return Ok(escape::escape_html(&eval::value_to_string(&lval)));
            }
            render_jsx_expression(&b.right, ctx, config, imports, state)
        }
        Expr::Call(call) => {
            if let Some((arr, param, body)) = eval_array_map(call, ctx)? {
                return render_map(arr, &param, &body, ctx, config, imports, state);
            }
            let val = eval::eval_expr(expr, ctx)?;
            Ok(escape::escape_html(&eval::value_to_string(&val)))
        }
        _ => {
            let val = eval::eval_expr(expr, ctx)?;
            Ok(escape::escape_html(&eval::value_to_string(&val)))
        }
    }
}

fn render_map(
    arr: Value,
    param: &str,
    body: &Expr,
    ctx: &EvalContext,
    config: &RenderConfig,
    imports: &HashMap<String, String>,
    state: &mut RenderState,
) -> Result<String, JsxrsError> {
    let items = match arr {
        Value::Array(a) => a,
        _ => return Err(JsxrsError::Unsupported("map() on non-array".into())),
    };
    let mut out = String::new();
    for item in items {
        let local_ctx = ctx.with_local(param.to_string(), item);
        out.push_str(&renderer::render_expr(
            body, &local_ctx, config, imports, state,
        )?);
    }
    Ok(out)
}

fn eval_array_map(
    call: &CallExpr,
    ctx: &EvalContext,
) -> Result<Option<(Value, String, Expr)>, JsxrsError> {
    let callee_member = match &call.callee {
        Callee::Expr(e) => match e.as_ref() {
            Expr::Member(m) => m,
            _ => return Ok(None),
        },
        _ => return Ok(None),
    };

    let method = match &callee_member.prop {
        MemberProp::Ident(id) => id.sym.to_string(),
        _ => return Ok(None),
    };

    if method != "map" {
        return Ok(None);
    }

    let arr_val = eval::eval_expr(&callee_member.obj, ctx)?;
    let callback = call
        .args
        .first()
        .ok_or_else(|| JsxrsError::Unsupported("map() requires a callback argument".into()))?;

    let (param_name, body) = extract_arrow_callback(&callback.expr)?;

    Ok(Some((arr_val, param_name, body)))
}

fn extract_arrow_callback(expr: &Expr) -> Result<(String, Expr), JsxrsError> {
    match expr {
        Expr::Arrow(arrow) => {
            let param_name = match arrow.params.first() {
                Some(swc_ecma_ast::Pat::Ident(id)) => id.sym.to_string(),
                _ => {
                    return Err(JsxrsError::Unsupported(
                        "only simple parameter names in map callbacks".into(),
                    ));
                }
            };
            let body = match arrow.body.as_ref() {
                swc_ecma_ast::BlockStmtOrExpr::Expr(e) => *e.clone(),
                _ => {
                    return Err(JsxrsError::Unsupported(
                        "only expression bodies in map callbacks".into(),
                    ));
                }
            };
            Ok((param_name, body))
        }
        Expr::Paren(p) => extract_arrow_callback(&p.expr),
        _ => Err(JsxrsError::Unsupported(
            "only arrow function callbacks in map()".into(),
        )),
    }
}
