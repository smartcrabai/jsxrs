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
mod source;
mod tailwind;

pub mod codegen;

#[cfg(feature = "axum")]
pub mod router;

pub use config::{CheckedRenderContext, HeadElement, RenderConfig};
pub use error::JsxrsError;
pub use jsxrs_macros::component;
#[cfg(feature = "axum")]
pub use jsxrs_macros::routes;

#[doc(hidden)]
pub mod __private {
    pub use crate::source::CheckedSourceSet;
    pub use serde;
    pub use serde_json;
}

use std::borrow::Cow;
use std::path::Path;

use serde_json::Value;

/// Render a JSX/TSX source string to a complete HTML document.
///
/// # Errors
///
/// Returns an error if the source cannot be parsed or if rendering fails.
pub fn render_string(
    source: &str,
    file_name: &str,
    props: &Value,
    config: &RenderConfig,
) -> Result<String, JsxrsError> {
    render_string_with_context(
        source,
        file_name,
        props,
        config,
        &CheckedRenderContext::default(),
    )
}

fn render_string_with_context(
    source: &str,
    file_name: &str,
    props: &Value,
    config: &RenderConfig,
    checked_context: &CheckedRenderContext,
) -> Result<String, JsxrsError> {
    if source.is_empty() {
        return Err(JsxrsError::NoDefaultExport);
    }

    let (module, _cm) = parser::parse_source(source, file_name)?;
    let (body_html, state) = renderer::render_module(&module.body, props, config, checked_context)?;

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
///
/// # Errors
///
/// Returns an error if the file cannot be read, parsed, or if rendering fails.
pub fn render_file(
    path: &Path,
    props: &Value,
    config: &RenderConfig,
) -> Result<String, JsxrsError> {
    render_file_with_context(path, props, config, &CheckedRenderContext::default())
}

/// Render a checked component using a separate source-root context.
///
/// This keeps checked-source state out of [`RenderConfig`] while allowing
/// generated component modules to preserve their compile-time source set.
///
/// # Errors
///
/// Returns an error if checked sources are unavailable, changed, or cannot be rendered.
pub fn render_file_with_context(
    path: &Path,
    props: &Value,
    config: &RenderConfig,
    checked_context: &CheckedRenderContext,
) -> Result<String, JsxrsError> {
    let (source, file_identity) = if let Some(checked_sources) = checked_context.checked_sources() {
        let source_root = source::runtime_source_root(checked_context.source_root())?;
        checked_sources.validate(&source_root)?;
        let identity = source::source_identity(path, &source_root)?;
        let source = checked_sources.source(&identity)?;
        (Cow::Borrowed(source), Some(identity))
    } else {
        (
            Cow::Owned(
                std::fs::read_to_string(path)
                    .map_err(|_| JsxrsError::FileNotFound(path.to_path_buf()))?,
            ),
            None,
        )
    };
    let file_name = file_identity
        .as_deref()
        .unwrap_or(path)
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .ok_or_else(|| JsxrsError::FileNotFound(path.to_path_buf()))?;

    let checked_context = file_identity.map_or_else(
        || checked_context.clone(),
        |identity| checked_context.clone().with_source_path(identity),
    );
    render_string_with_context(source.as_ref(), &file_name, props, config, &checked_context)
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
