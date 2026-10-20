use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::error::JsxrsError;

/// Compile-time checked TSX sources and the import edges selected for them.
#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct CheckedSourceSet {
    files: HashMap<PathBuf, &'static [u8]>,
    imports: HashMap<(PathBuf, String), PathBuf>,
}

impl CheckedSourceSet {
    /// Build a checked source set from macro-generated paths and embedded bytes.
    #[must_use]
    pub fn new(
        files: Vec<(&'static str, &'static [u8])>,
        imports: Vec<(&'static str, &'static str, &'static str)>,
    ) -> Self {
        Self {
            files: files
                .into_iter()
                .map(|(path, source)| (PathBuf::from(path), source))
                .collect(),
            imports: imports
                .into_iter()
                .map(|(from, specifier, to)| {
                    ((PathBuf::from(from), specifier.to_owned()), PathBuf::from(to))
                })
                .collect(),
        }
    }

    pub(crate) fn validate(&self, source_root: &Path) -> Result<(), JsxrsError> {
        for (relative_path, embedded_source) in &self.files {
            validate_relative_path(relative_path)?;
            let runtime_path = source_root.join(relative_path);
            let runtime_source = fs::read(&runtime_path).map_err(|_| {
                JsxrsError::Unsupported(format!(
                    "checked source is unavailable at {}",
                    runtime_path.display()
                ))
            })?;
            if *embedded_source != runtime_source.as_slice() {
                return Err(JsxrsError::Unsupported(format!(
                    "checked source changed after compilation: {}",
                    relative_path.display()
                )));
            }
        }
        Ok(())
    }

    pub(crate) fn source(&self, path: &Path) -> Result<&'static str, JsxrsError> {
        let bytes = self.files.get(path).ok_or_else(|| {
            JsxrsError::Unsupported(format!(
                "source is not part of the checked source set: {}",
                path.display()
            ))
        })?;
        std::str::from_utf8(bytes).map_err(|error| {
            JsxrsError::Unsupported(format!(
                "checked source is not UTF-8 at {}: {error}",
                path.display()
            ))
        })
    }

    pub(crate) fn resolve_import(&self, from: &Path, specifier: &str) -> Result<PathBuf, JsxrsError> {
        self.imports
            .get(&(from.to_path_buf(), specifier.to_owned()))
            .cloned()
            .ok_or_else(|| {
                JsxrsError::Unsupported(format!(
                    "import edge was not present in the checked source set: {} -> {specifier}",
                    from.display()
                ))
            })
    }
}

pub(crate) fn runtime_source_root(configured: Option<&Path>) -> Result<PathBuf, JsxrsError> {
    match configured {
        Some(path) if path.is_absolute() => Ok(path.to_path_buf()),
        Some(path) => Ok(std::env::current_dir()?.join(path)),
        None => Ok(std::env::current_dir()?),
    }
}

pub(crate) fn source_identity(path: &Path, source_root: &Path) -> Result<PathBuf, JsxrsError> {
    let relative = if path.is_absolute() {
        path.strip_prefix(source_root).map_err(|_| {
            JsxrsError::Unsupported(format!(
                "source path is outside configured source root: {}",
                path.display()
            ))
        })?
    } else {
        path
    };
    validate_relative_path(relative)?;
    Ok(relative.to_path_buf())
}

fn validate_relative_path(path: &Path) -> Result<(), JsxrsError> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(JsxrsError::Unsupported(format!(
            "checked source identity must be a normalized relative path: {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::runtime_source_root;
    use std::path::Path;

    #[test]
    fn absent_source_root_uses_current_directory() -> Result<(), Box<dyn std::error::Error>> {
        let current_dir = std::env::current_dir()?;
        assert_eq!(runtime_source_root(None)?, current_dir);
        Ok(())
    }

    #[test]
    fn absolute_source_root_is_preserved() -> Result<(), Box<dyn std::error::Error>> {
        let absolute = std::env::current_dir()?.join("configured-root");
        assert_eq!(runtime_source_root(Some(&absolute))?, absolute);
        Ok(())
    }

    #[test]
    fn relative_source_root_is_joined_to_current_directory() -> Result<(), Box<dyn std::error::Error>> {
        let current_dir = std::env::current_dir()?;
        let relative = Path::new("configured-root");
        assert_eq!(
            runtime_source_root(Some(relative))?,
            current_dir.join(relative)
        );
        Ok(())
    }
}
