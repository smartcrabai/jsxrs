use std::path::PathBuf;
use std::sync::Arc;

/// Head element specified via the Rust API.
#[derive(Debug, Clone)]
pub enum HeadElement {
    Title(String),
    Meta { name: String, content: String },
    Link { rel: String, href: String },
    Style(String),
    Script(String),
}

/// Configuration for the JSX-to-HTML rendering pipeline.
#[derive(Debug, Clone, Default)]
pub struct RenderConfig {
    pub pretty: bool,
    pub base_dir: Option<PathBuf>,
    pub head_elements: Vec<HeadElement>,
    pub tailwind: bool,
    pub fragment: bool,
}

/// Runtime context for checked components, routes, and in-memory source maps.
#[derive(Debug, Clone, Default)]
pub struct CheckedRenderContext {
    source_root: Option<PathBuf>,
    checked_sources: Option<Arc<crate::__private::CheckedSourceSet>>,
    source_map: Option<Arc<crate::source::SourceMap>>,
    source_path: Option<PathBuf>,
}

impl CheckedRenderContext {
    /// Create a context with no explicit source-root override. Macro-generated checked renders
    /// default to the consuming crate's manifest directory.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the runtime root used to validate and locate checked source files.
    #[must_use]
    pub fn with_source_root(mut self, source_root: impl Into<PathBuf>) -> Self {
        self.source_root = Some(source_root.into());
        self
    }

    /// Attach a macro-generated checked source set.
    #[doc(hidden)]
    #[must_use]
    pub fn __with_checked_sources(
        mut self,
        default_source_root: impl Into<PathBuf>,
        checked_sources: Arc<crate::__private::CheckedSourceSet>,
    ) -> Self {
        if self.source_root.is_none() {
            self.source_root = Some(default_source_root.into());
        }
        self.checked_sources = Some(checked_sources);
        self
    }

    /// Resolve the requested source files and their relative imports from memory.
    #[must_use]
    pub fn with_source_map(mut self, source_map: Arc<crate::SourceMap>) -> Self {
        self.source_map = Some(source_map);
        self
    }

    pub(crate) fn source_root(&self) -> Option<&std::path::Path> {
        self.source_root.as_deref()
    }

    pub(crate) fn checked_sources(&self) -> Option<&Arc<crate::__private::CheckedSourceSet>> {
        self.checked_sources.as_ref()
    }

    pub(crate) fn source_map(&self) -> Option<&Arc<crate::SourceMap>> {
        self.source_map.as_ref()
    }

    pub(crate) fn source_path(&self) -> Option<&std::path::Path> {
        self.source_path.as_deref()
    }

    pub(crate) fn with_source_path(mut self, source_path: PathBuf) -> Self {
        self.source_path = Some(source_path);
        self
    }
}
