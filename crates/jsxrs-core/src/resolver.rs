use std::path::{Path, PathBuf};

use crate::JsxrsError;

const EXTENSIONS: &[&str] = &[".tsx", ".jsx", ".ts", ".js"];
const INDEX_EXTENSIONS: &[&str] = &["/index.tsx", "/index.jsx"];

/// Resolve a relative import path using jsxrs' stable extension priority.
///
/// # Errors
///
/// Returns an error when no matching file exists under `base_dir`.
pub fn resolve_import(import_path: &str, base_dir: &Path) -> Result<PathBuf, JsxrsError> {
    resolve_import_with(import_path, base_dir, Path::is_file)
}

/// Resolve an import path with a custom candidate-existence check.
///
/// # Errors
///
/// Returns an error when no matching candidate is accepted by `exists`.
pub fn resolve_import_with(
    import_path: &str,
    base_dir: &Path,
    mut exists: impl FnMut(&Path) -> bool,
) -> Result<PathBuf, JsxrsError> {
    let candidate = base_dir.join(strip_leading_dot(import_path));
    if exists(&candidate) {
        return Ok(candidate);
    }
    for extension in EXTENSIONS {
        let with_extension = append_to_path(&candidate, extension);
        if exists(&with_extension) {
            return Ok(with_extension);
        }
    }
    for extension in INDEX_EXTENSIONS {
        let index_path = append_to_path(&candidate, extension);
        if exists(&index_path) {
            return Ok(index_path);
        }
    }
    Err(JsxrsError::ImportNotFound {
        path: import_path.to_owned(),
    })
}

fn strip_leading_dot(path: &str) -> &str {
    path.strip_prefix("./").unwrap_or(path)
}

fn append_to_path(base: &Path, suffix: &str) -> PathBuf {
    let mut value = base.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}
