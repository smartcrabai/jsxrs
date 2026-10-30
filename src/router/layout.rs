use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::config::{CheckedRenderContext, RenderConfig};
use crate::error::JsxrsError;

/// Render a page file wrapped with its layout chain.
///
/// Layouts are applied from innermost to outermost. Each layout receives
/// `props.children` as `{ "__html": "<inner html>" }` which is rendered
/// without escaping.
pub fn render_with_layouts(
    page_file: &Path,
    layouts: &[PathBuf],
    props: &Value,
    config: &RenderConfig,
    checked_context: &CheckedRenderContext,
) -> Result<String, JsxrsError> {
    crate::render_with_layouts_and_context(page_file, layouts, props, config, checked_context)
}
