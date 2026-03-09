use std::path::PathBuf;

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
