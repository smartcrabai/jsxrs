use std::path::{Path, PathBuf};

use crate::error::JsxrsError;

const EXTENSIONS: &[&str] = &[".tsx", ".jsx", ".ts", ".js"];
const INDEX_EXTENSIONS: &[&str] = &["/index.tsx", "/index.jsx"];

/// Resolve a relative import path to an actual file on disk.
/// `import_path` is the specifier from the import statement (e.g. "./components/button").
/// `base_dir` is the directory to resolve relative to.
pub fn resolve_import(import_path: &str, base_dir: &Path) -> Result<PathBuf, JsxrsError> {
    let candidate = base_dir.join(strip_leading_dot(import_path));

    if candidate.is_file() {
        return Ok(candidate);
    }

    for ext in EXTENSIONS {
        let with_ext = append_to_path(&candidate, ext);
        if with_ext.is_file() {
            return Ok(with_ext);
        }
    }

    for idx_ext in INDEX_EXTENSIONS {
        let idx_path = append_to_path(&candidate, idx_ext);
        if idx_path.is_file() {
            return Ok(idx_path);
        }
    }

    Err(JsxrsError::ImportNotFound {
        path: import_path.to_string(),
    })
}

fn strip_leading_dot(path: &str) -> &str {
    path.strip_prefix("./").unwrap_or(path)
}

fn append_to_path(base: &Path, suffix: &str) -> PathBuf {
    let mut s = base.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}
