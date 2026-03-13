#![allow(dead_code)]

use std::path::{Path, PathBuf};

use jsxrs::{HeadElement, RenderConfig};

pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

pub fn minimal_config() -> RenderConfig {
    RenderConfig::default()
}

pub fn pretty_config() -> RenderConfig {
    RenderConfig {
        pretty: true,
        ..Default::default()
    }
}

pub fn config_with_base_dir(base_dir: PathBuf) -> RenderConfig {
    RenderConfig {
        base_dir: Some(base_dir),
        ..Default::default()
    }
}

pub fn config_with_tailwind() -> RenderConfig {
    RenderConfig {
        tailwind: true,
        ..Default::default()
    }
}

pub fn config_with_head(head_elements: Vec<HeadElement>) -> RenderConfig {
    RenderConfig {
        head_elements,
        ..Default::default()
    }
}

pub fn fragment_config() -> RenderConfig {
    RenderConfig {
        fragment: true,
        ..Default::default()
    }
}

/// Extracts the content inside <body>...</body> from full HTML document output.
///
/// Returns `None` if the body tags are not found.
pub fn extract_body(html: &str) -> Option<&str> {
    let start = html.find("<body>")? + "<body>".len();
    let end = html.find("</body>")?;
    Some(&html[start..end])
}

/// Extracts the content inside <head>...</head> from full HTML document output.
///
/// Returns `None` if the head tags are not found.
pub fn extract_head(html: &str) -> Option<&str> {
    let start = html.find("<head>")? + "<head>".len();
    let end = html.find("</head>")?;
    Some(&html[start..end])
}
