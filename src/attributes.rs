use swc_ecma_ast::{
    Expr, JSXAttrName, JSXAttrOrSpread, JSXAttrValue, JSXExprContainer, KeyValueProp, Lit,
    ObjectLit, Prop, PropName, PropOrSpread,
};

use crate::error::JsxrsError;
use crate::eval::{self, EvalContext};

const REACT_TO_HTML: &[(&str, &str)] = &[
    ("className", "class"),
    ("htmlFor", "for"),
    ("tabIndex", "tabindex"),
    ("readOnly", "readonly"),
    ("maxLength", "maxlength"),
    ("cellSpacing", "cellspacing"),
    ("cellPadding", "cellpadding"),
    ("rowSpan", "rowspan"),
    ("colSpan", "colspan"),
    ("encType", "enctype"),
    ("crossOrigin", "crossorigin"),
    ("autoComplete", "autocomplete"),
    ("autoFocus", "autofocus"),
    ("autoPlay", "autoplay"),
];

fn map_attr_name(name: &str) -> &str {
    for &(react, html) in REACT_TO_HTML {
        if name == react {
            return html;
        }
    }
    name
}

fn is_event_handler(name: &str) -> bool {
    name.len() > 2
        && name.starts_with("on")
        && name
            .as_bytes()
            .get(2)
            .is_some_and(|b| b.is_ascii_uppercase())
}

fn camel_to_kebab(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            result.push('-');
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

fn render_style_object(obj: &ObjectLit) -> String {
    let mut parts = Vec::new();
    for prop in &obj.props {
        if let PropOrSpread::Prop(prop) = prop {
            if let Prop::KeyValue(KeyValueProp { key, value }) = prop.as_ref() {
                let key_str = match key {
                    PropName::Ident(id) => camel_to_kebab(&id.sym),
                    PropName::Str(s) => camel_to_kebab(&s.value.to_string_lossy()),
                    _ => continue,
                };
                let val_str = match value.as_ref() {
                    Expr::Lit(Lit::Str(s)) => s.value.to_string_lossy().into_owned(),
                    Expr::Lit(Lit::Num(n)) => n.value.to_string(),
                    _ => continue,
                };
                parts.push(format!("{key_str}: {val_str}"));
            }
        }
    }
    parts.join("; ")
}

pub struct RenderedAttr {
    pub name: String,
    pub value: Option<String>,
}

/// Render a single JSX attribute to its HTML form.
/// Returns `None` if the attribute should be excluded (event handlers, false booleans).
pub fn render_jsx_attr(
    attr: &JSXAttrOrSpread,
    ctx: &EvalContext,
) -> Result<Option<RenderedAttr>, JsxrsError> {
    let attr = match attr {
        JSXAttrOrSpread::JSXAttr(a) => a,
        JSXAttrOrSpread::SpreadElement(_) => return Ok(None),
    };

    let raw_name = jsx_attr_name_str(&attr.name);
    if is_event_handler(&raw_name) {
        return Ok(None);
    }

    let html_name = map_attr_name(&raw_name).to_string();

    match &attr.value {
        None => Ok(Some(RenderedAttr {
            name: html_name,
            value: None,
        })),
        Some(val) => render_attr_value(&html_name, &raw_name, val, ctx),
    }
}

fn render_attr_value(
    html_name: &str,
    raw_name: &str,
    val: &JSXAttrValue,
    ctx: &EvalContext,
) -> Result<Option<RenderedAttr>, JsxrsError> {
    match val {
        JSXAttrValue::Str(s) => Ok(Some(RenderedAttr {
            name: html_name.to_string(),
            value: Some(s.value.to_string_lossy().into_owned()),
        })),
        JSXAttrValue::JSXExprContainer(JSXExprContainer { expr, .. }) => {
            render_expr_attr(html_name, raw_name, expr, ctx)
        }
        _ => Ok(None),
    }
}

fn render_expr_attr(
    html_name: &str,
    raw_name: &str,
    expr: &swc_ecma_ast::JSXExpr,
    ctx: &EvalContext,
) -> Result<Option<RenderedAttr>, JsxrsError> {
    let expr = match expr {
        swc_ecma_ast::JSXExpr::Expr(e) => e,
        swc_ecma_ast::JSXExpr::JSXEmptyExpr(_) => return Ok(None),
    };

    if raw_name == "style" {
        if let Expr::Object(obj) = expr.as_ref() {
            let css = render_style_object(obj);
            return Ok(Some(RenderedAttr {
                name: html_name.to_string(),
                value: Some(css),
            }));
        }
    }

    if let Expr::Lit(Lit::Bool(b)) = expr.as_ref() {
        return if b.value {
            Ok(Some(RenderedAttr {
                name: html_name.to_string(),
                value: None,
            }))
        } else {
            Ok(None)
        };
    }

    let val = eval::eval_expr(expr, ctx)?;
    let s = eval::value_to_string(&val);
    Ok(Some(RenderedAttr {
        name: html_name.to_string(),
        value: Some(s),
    }))
}

fn jsx_attr_name_str(name: &JSXAttrName) -> String {
    match name {
        JSXAttrName::Ident(id) => id.sym.to_string(),
        JSXAttrName::JSXNamespacedName(ns) => {
            format!("{}:{}", ns.ns.sym, ns.name.sym)
        }
    }
}
