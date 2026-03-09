use std::collections::HashMap;

use serde_json::Value;
use swc_ecma_ast::{
    Expr, JSXAttrOrSpread, JSXAttrValue, JSXElement, JSXElementChild, JSXElementName, JSXExpr,
    JSXExprContainer, JSXFragment, ModuleItem,
};

use crate::attributes;
use crate::config::RenderConfig;
use crate::error::JsxrsError;
use crate::escape;
use crate::eval::{self, EvalContext};
use crate::export;
use crate::head::HeadContent;
use crate::jsx_expr;
use crate::parser;
use crate::resolver;

const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Accumulated state during rendering.
pub struct RenderState {
    pub head: HeadContent,
    pub class_names: Vec<String>,
}

impl RenderState {
    pub fn new() -> Self {
        Self {
            head: HeadContent::default(),
            class_names: Vec::new(),
        }
    }
}

/// Render a parsed module to body HTML.
pub fn render_module(
    items: &[ModuleItem],
    props: &Value,
    config: &RenderConfig,
) -> Result<(String, RenderState), JsxrsError> {
    let imports = export::collect_imports(items);
    let ctx = EvalContext::new(props.clone());
    let mut state = RenderState::new();

    let jsx = export::find_default_export_jsx(items)?;
    let html = render_expr(&jsx, &ctx, config, &imports, &mut state)?;

    Ok((html, state))
}

pub(crate) fn render_expr(
    expr: &Expr,
    ctx: &EvalContext,
    config: &RenderConfig,
    imports: &HashMap<String, String>,
    state: &mut RenderState,
) -> Result<String, JsxrsError> {
    match expr {
        Expr::JSXElement(el) => render_element(el, ctx, config, imports, state),
        Expr::JSXFragment(f) => render_fragment(f, ctx, config, imports, state),
        Expr::Paren(p) => render_expr(&p.expr, ctx, config, imports, state),
        _ => {
            let val = eval::eval_expr(expr, ctx)?;
            if let Value::Object(map) = &val
                && let Some(Value::String(raw)) = map.get("__html")
            {
                return Ok(raw.clone());
            }
            Ok(escape::escape_html(&eval::value_to_string(&val)))
        }
    }
}

pub(crate) fn render_element(
    el: &JSXElement,
    ctx: &EvalContext,
    config: &RenderConfig,
    imports: &HashMap<String, String>,
    state: &mut RenderState,
) -> Result<String, JsxrsError> {
    let tag = element_name_str(&el.opening.name);

    if tag == "Head" {
        return render_head(el, ctx, config, imports, state);
    }
    if tag.starts_with(|c: char| c.is_ascii_uppercase()) {
        return render_component(&tag, el, ctx, config, imports, state);
    }
    render_html_tag(&tag, el, ctx, config, imports, state)
}

fn render_head(
    el: &JSXElement,
    ctx: &EvalContext,
    config: &RenderConfig,
    imports: &HashMap<String, String>,
    state: &mut RenderState,
) -> Result<String, JsxrsError> {
    for child in &el.children {
        let html = render_child(child, ctx, config, imports, state)?;
        if !html.is_empty() {
            state.head.push(html);
        }
    }
    Ok(String::new())
}

fn render_component(
    tag: &str,
    el: &JSXElement,
    ctx: &EvalContext,
    config: &RenderConfig,
    imports: &HashMap<String, String>,
    state: &mut RenderState,
) -> Result<String, JsxrsError> {
    let import_path = imports
        .get(tag)
        .ok_or_else(|| JsxrsError::ImportNotFound { path: tag.into() })?;
    let base_dir = config
        .base_dir
        .as_ref()
        .ok_or(JsxrsError::BaseDirRequired)?;
    let resolved = resolver::resolve_import(import_path, base_dir)?;

    let source = std::fs::read_to_string(&resolved)
        .map_err(|_| JsxrsError::FileNotFound(resolved.clone()))?;
    let fname = resolved.to_string_lossy().to_string();
    let (module, _) = parser::parse_source(&source, &fname)?;

    let component_props = build_component_props(&el.opening.attrs, ctx)?;
    let (html, inner_state) = render_module(&module.body, &component_props, config)?;

    state.head.extend(inner_state.head);
    state.class_names.extend(inner_state.class_names);
    Ok(html)
}

fn build_component_props(
    attrs: &[JSXAttrOrSpread],
    ctx: &EvalContext,
) -> Result<Value, JsxrsError> {
    let mut map = serde_json::Map::new();
    for attr in attrs {
        let JSXAttrOrSpread::JSXAttr(a) = attr else {
            continue;
        };
        let name = match &a.name {
            swc_ecma_ast::JSXAttrName::Ident(id) => id.sym.to_string(),
            _ => continue,
        };
        let value = match &a.value {
            None => Value::Bool(true),
            Some(JSXAttrValue::Str(s)) => Value::String(s.value.to_string_lossy().into_owned()),
            Some(JSXAttrValue::JSXExprContainer(JSXExprContainer { expr, .. })) => match expr {
                JSXExpr::Expr(e) => eval::eval_expr(e, ctx)?,
                JSXExpr::JSXEmptyExpr(_) => Value::Null,
            },
            _ => continue,
        };
        map.insert(name, value);
    }
    Ok(Value::Object(map))
}

fn render_html_tag(
    tag: &str,
    el: &JSXElement,
    ctx: &EvalContext,
    config: &RenderConfig,
    imports: &HashMap<String, String>,
    state: &mut RenderState,
) -> Result<String, JsxrsError> {
    let mut out = format!("<{tag}");
    for attr in &el.opening.attrs {
        if let Some(r) = attributes::render_jsx_attr(attr, ctx)? {
            if r.name == "class"
                && let Some(ref v) = r.value
            {
                state.class_names.push(v.clone());
            }
            out.push(' ');
            out.push_str(&r.name);
            if let Some(v) = &r.value {
                out.push_str("=\"");
                out.push_str(&escape::escape_attr(v));
                out.push('"');
            }
        }
    }
    out.push('>');

    if VOID_ELEMENTS.contains(&tag) {
        return Ok(out);
    }

    for child in &el.children {
        out.push_str(&render_child(child, ctx, config, imports, state)?);
    }
    out.push_str(&format!("</{tag}>"));
    Ok(out)
}

pub(crate) fn render_fragment(
    frag: &JSXFragment,
    ctx: &EvalContext,
    config: &RenderConfig,
    imports: &HashMap<String, String>,
    state: &mut RenderState,
) -> Result<String, JsxrsError> {
    let mut out = String::new();
    for child in &frag.children {
        out.push_str(&render_child(child, ctx, config, imports, state)?);
    }
    Ok(out)
}

fn render_child(
    child: &JSXElementChild,
    ctx: &EvalContext,
    config: &RenderConfig,
    imports: &HashMap<String, String>,
    state: &mut RenderState,
) -> Result<String, JsxrsError> {
    match child {
        JSXElementChild::JSXText(t) => Ok(escape::escape_html(&normalize_jsx_text(&t.value))),
        JSXElementChild::JSXElement(el) => render_element(el, ctx, config, imports, state),
        JSXElementChild::JSXFragment(f) => render_fragment(f, ctx, config, imports, state),
        JSXElementChild::JSXExprContainer(c) => {
            render_expr_container(c, ctx, config, imports, state)
        }
        JSXElementChild::JSXSpreadChild(_) => Ok(String::new()),
    }
}

fn render_expr_container(
    container: &JSXExprContainer,
    ctx: &EvalContext,
    config: &RenderConfig,
    imports: &HashMap<String, String>,
    state: &mut RenderState,
) -> Result<String, JsxrsError> {
    match &container.expr {
        JSXExpr::Expr(e) => jsx_expr::render_jsx_expression(e, ctx, config, imports, state),
        JSXExpr::JSXEmptyExpr(_) => Ok(String::new()),
    }
}

fn normalize_jsx_text(raw: &str) -> String {
    let lines: Vec<&str> = raw.split('\n').collect();
    if lines.len() == 1 {
        return raw.to_string();
    }
    let mut parts = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = if i == 0 {
            line.trim_end()
        } else if i == lines.len() - 1 {
            line.trim_start()
        } else {
            line.trim()
        };
        if !trimmed.is_empty() {
            parts.push(trimmed);
        }
    }
    parts.join("")
}

fn element_name_str(name: &JSXElementName) -> String {
    match name {
        JSXElementName::Ident(id) => id.sym.to_string(),
        JSXElementName::JSXMemberExpr(m) => {
            format!("{}.{}", jsx_obj_str(&m.obj), m.prop.sym)
        }
        JSXElementName::JSXNamespacedName(n) => format!("{}:{}", n.ns.sym, n.name.sym),
    }
}

fn jsx_obj_str(obj: &swc_ecma_ast::JSXObject) -> String {
    match obj {
        swc_ecma_ast::JSXObject::Ident(id) => id.sym.to_string(),
        swc_ecma_ast::JSXObject::JSXMemberExpr(m) => {
            format!("{}.{}", jsx_obj_str(&m.obj), m.prop.sym)
        }
    }
}
