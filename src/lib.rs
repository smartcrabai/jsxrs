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
pub use source::SourceMap;

#[doc(hidden)]
pub mod __private {
    pub use crate::source::CheckedSourceSet;
    pub use serde;
    pub use serde_json;
}

use std::borrow::Cow;
use std::path::{Path, PathBuf};

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

/// Render a page with its layouts, from the outermost path to the innermost.
///
/// Layouts receive the page props and `children`, containing the already-rendered inner HTML.
/// Head content and Tailwind classes are collected across the entire chain.
///
/// # Errors
///
/// Returns an error if any source cannot be read, parsed, or rendered.
pub fn render_with_layouts(
    page_file: &Path,
    layouts: &[PathBuf],
    props: &Value,
    config: &RenderConfig,
) -> Result<String, JsxrsError> {
    render_with_layouts_and_context(
        page_file,
        layouts,
        props,
        config,
        &CheckedRenderContext::default(),
    )
}

/// Render a page and its layouts using a source context, such as a [`SourceMap`].
///
/// # Errors
///
/// Returns an error if any source cannot be resolved, parsed, or rendered.
pub fn render_with_layouts_and_context(
    page_file: &Path,
    layouts: &[PathBuf],
    props: &Value,
    config: &RenderConfig,
    checked_context: &CheckedRenderContext,
) -> Result<String, JsxrsError> {
    let mut state = renderer::RenderState::new();
    let page_config = RenderConfig {
        base_dir: page_file.parent().map(Path::to_path_buf),
        fragment: true,
        ..config.clone()
    };
    let mut html = render_path_into(page_file, props, &page_config, checked_context, &mut state)?;

    for layout_path in layouts.iter().rev() {
        let mut layout_props = props.as_object().cloned().unwrap_or_default();
        if !props.is_object() {
            layout_props.insert("pageProps".to_owned(), props.clone());
        }
        layout_props.insert("children".to_owned(), serde_json::json!({ "__html": html }));
        let layout_props = Value::Object(layout_props);
        let layout_config = RenderConfig {
            base_dir: layout_path.parent().map(Path::to_path_buf),
            fragment: true,
            ..config.clone()
        };
        html = render_path_into(
            layout_path,
            &layout_props,
            &layout_config,
            checked_context,
            &mut state,
        )?;
    }

    if config.fragment {
        return Ok(html);
    }
    let head_html = build_head(&state, config).join("");
    Ok(document::build_document(&head_html, &html, config.pretty))
}

/// Render a TSX file with an optional checked or in-memory source context.
///
/// This keeps source state out of [`RenderConfig`] while allowing generated components to retain
/// their compile-time source set or use an embedded [`SourceMap`].
///
/// # Errors
///
/// Returns an error if the file cannot be read or resolved, or if parsing or rendering fails.
pub fn render_file_with_context(
    path: &Path,
    props: &Value,
    config: &RenderConfig,
    checked_context: &CheckedRenderContext,
) -> Result<String, JsxrsError> {
    let mut state = renderer::RenderState::new();
    let body_html = render_path_into(path, props, config, checked_context, &mut state)?;
    if config.fragment {
        return Ok(body_html);
    }
    let head_html = build_head(&state, config).join("");
    Ok(document::build_document(
        &head_html,
        &body_html,
        config.pretty,
    ))
}

fn render_path_into(
    path: &Path,
    props: &Value,
    config: &RenderConfig,
    checked_context: &CheckedRenderContext,
    state: &mut renderer::RenderState,
) -> Result<String, JsxrsError> {
    let (source, file_identity) = if let Some(source_map) = checked_context.source_map() {
        let identity = path.to_path_buf();
        (Cow::Borrowed(source_map.source(&identity)?), Some(identity))
    } else if let Some(checked_sources) = checked_context.checked_sources() {
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
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| JsxrsError::FileNotFound(path.to_path_buf()))?;
    let source_context = file_identity.map_or_else(
        || checked_context.clone(),
        |identity| checked_context.clone().with_source_path(identity),
    );
    let (module, _) = parser::parse_source(source.as_ref(), &file_name)?;
    renderer::render_module_with_state(&module.body, props, config, &source_context, state)
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
