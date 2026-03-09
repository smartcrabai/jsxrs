mod attributes;
mod config;
mod document;
mod error;
mod escape;
mod eval;
mod export;
mod head;
mod jsx_expr;
mod parser;
mod renderer;
mod resolver;
mod tailwind;

pub mod codegen;

pub use config::{HeadElement, RenderConfig};
pub use error::JsxrsError;

use std::path::Path;

use serde_json::Value;

/// Render a JSX/TSX source string to a complete HTML document.
pub fn render_string(
    source: &str,
    file_name: &str,
    props: &Value,
    config: &RenderConfig,
) -> Result<String, JsxrsError> {
    if source.is_empty() {
        return Err(JsxrsError::NoDefaultExport);
    }

    let (module, _cm) = parser::parse_source(source, file_name)?;
    let (body_html, state) = renderer::render_module(&module.body, props, config)?;

    if config.fragment {
        return Ok(body_html);
    }

    let head_parts = build_head(&state, config);
    let head_html = head_parts.join("");

    Ok(document::build_document(
        &head_html,
        &body_html,
        config.pretty,
    ))
}

/// Render a JSX/TSX file to a complete HTML document.
pub fn render_file(
    path: &Path,
    props: &Value,
    config: &RenderConfig,
) -> Result<String, JsxrsError> {
    let source =
        std::fs::read_to_string(path).map_err(|_| JsxrsError::FileNotFound(path.to_path_buf()))?;
    let file_name = path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .ok_or_else(|| JsxrsError::FileNotFound(path.to_path_buf()))?;

    render_string(&source, &file_name, props, config)
}

fn build_head(state: &renderer::RenderState, config: &RenderConfig) -> Vec<String> {
    let mut head_parts = head::merge_head(&state.head, &config.head_elements);

    if config.tailwind {
        let css = tailwind::generate_css(&state.class_names);
        if !css.is_empty() {
            head_parts.push(format!("<style>{css}</style>"));
        }
    }

    head_parts
}
