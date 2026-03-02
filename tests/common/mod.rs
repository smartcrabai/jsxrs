use std::path::{Path, PathBuf};

use jsxrs::{HeadElement, RenderConfig};

pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

pub fn minimal_config() -> RenderConfig {
    RenderConfig {
        pretty: false,
        base_dir: None,
        head_elements: Vec::new(),
        tailwind: false,
    }
}

pub fn pretty_config() -> RenderConfig {
    RenderConfig {
        pretty: true,
        base_dir: None,
        head_elements: Vec::new(),
        tailwind: false,
    }
}

pub fn config_with_base_dir(base_dir: PathBuf) -> RenderConfig {
    RenderConfig {
        pretty: false,
        base_dir: Some(base_dir),
        head_elements: Vec::new(),
        tailwind: false,
    }
}

pub fn config_with_tailwind() -> RenderConfig {
    RenderConfig {
        pretty: false,
        base_dir: None,
        head_elements: Vec::new(),
        tailwind: true,
    }
}

pub fn config_with_head(head_elements: Vec<HeadElement>) -> RenderConfig {
    RenderConfig {
        pretty: false,
        base_dir: None,
        head_elements,
        tailwind: false,
    }
}

/// Extracts the content inside <body>...</body> from full HTML document output.
pub fn extract_body(html: &str) -> &str {
    let start = html.find("<body>").expect("missing <body>") + "<body>".len();
    let end = html.find("</body>").expect("missing </body>");
    &html[start..end]
}

/// Extracts the content inside <head>...</head> from full HTML document output.
pub fn extract_head(html: &str) -> &str {
    let start = html.find("<head>").expect("missing <head>") + "<head>".len();
    let end = html.find("</head>").expect("missing </head>");
    &html[start..end]
}
