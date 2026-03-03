use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum JsxrsError {
    #[error("parse error: {0}")]
    Parse(String),

    #[error("no default export found")]
    NoDefaultExport,

    #[error("undefined prop: {0}")]
    UndefinedProp(String),

    #[error("unsupported expression: {0}")]
    Unsupported(String),

    #[error("import resolution failed: {path}")]
    ImportNotFound { path: String },

    #[error("base_dir not set but imports exist")]
    BaseDirRequired,

    #[error("file not found: {0}")]
    FileNotFound(PathBuf),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}
