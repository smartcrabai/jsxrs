use std::path::Path;

use serde_json::Value;

use crate::config::RenderConfig;
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
) -> Result<String, JsxrsError> {
    let fragment_config = RenderConfig {
        fragment: true,
        base_dir: Some(page_file.parent().unwrap_or(Path::new(".")).to_path_buf()),
        ..config.clone()
    };

    // Render the page itself
    let mut html = crate::render_file(page_file, props, &fragment_config)?;

    // Apply layouts from innermost to outermost
    for layout_path in layouts.iter().rev() {
        let mut layout_props = match props {
            Value::Object(map) => Value::Object(map.clone()),
            _ => Value::Object(serde_json::Map::new()),
        };
        if let Value::Object(ref mut map) = layout_props {
            map.insert(
                "children".to_string(),
                serde_json::json!({ "__html": html }),
            );
        }
        let layout_config = RenderConfig {
            fragment: true,
            base_dir: Some(layout_path.parent().unwrap_or(Path::new(".")).to_path_buf()),
            ..config.clone()
        };
        html = crate::render_file(layout_path, &layout_props, &layout_config)?;
    }

    Ok(html)
}

use std::path::PathBuf;
