pub mod codegen;
pub mod discovery;
pub mod export;
pub mod graph;
mod parser;
pub mod resolver;
mod schema;
mod validate;

mod error {
    use std::path::PathBuf;

    #[derive(Debug, thiserror::Error)]
    pub enum JsxrsError {
        #[error("parse error: {0}")]
        Parse(String),
        #[error("no default export found")]
        NoDefaultExport,
        #[error("undefined prop: {0}")]
        UndefinedProp(String),
        #[error("undefined prop `{key}` at byte position {position}")]
        UndefinedPropAt { key: String, position: u32 },
        #[error("{path}:{line}:{column}: {message}", path = .path.display())]
        ValidationDiagnostic {
            path: PathBuf,
            line: usize,
            column: usize,
            message: String,
        },
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
}

pub use codegen::generate_types;
pub use error::JsxrsError;
pub use graph::{
    resolve_component_graph, resolve_component_graph_snapshot,
    resolve_component_graph_snapshot_with_source_map,
};
pub use parser::parse_source;
pub use schema::{PropsField, PropsSchema, extract_props_schema};
pub use validate::{validate_component, validate_component_accesses};
