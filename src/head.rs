use crate::config::HeadElement;

/// Tag classification for dedup during merge.
#[derive(Debug)]
enum HeadTag {
    Title,
    Meta { name: String },
    Other,
}

/// A single collected head element with structural metadata.
#[derive(Debug)]
struct HeadEntry {
    html: String,
    tag: HeadTag,
}

/// Collected head content from JSX `<Head>` component.
#[derive(Debug, Default)]
pub struct HeadContent {
    entries: Vec<HeadEntry>,
}

impl HeadContent {
    pub fn push(&mut self, html: String) {
        let tag = classify_head_tag(&html);
        self.entries.push(HeadEntry { html, tag });
    }

    pub fn extend(&mut self, other: HeadContent) {
        self.entries.extend(other.entries);
    }
}

fn classify_head_tag(html: &str) -> HeadTag {
    let trimmed = html.trim();
    if trimmed.starts_with("<title") {
        return HeadTag::Title;
    }
    if (trimmed.starts_with("<meta ") || trimmed.starts_with("<meta>"))
        && let Some(name) = extract_meta_name(trimmed)
    {
        return HeadTag::Meta { name };
    }
    HeadTag::Other
}

fn extract_meta_name(html: &str) -> Option<String> {
    let name_pos = html.find("name=")?;
    let after = &html[name_pos + 5..];
    let quote = after.as_bytes().first()?;
    if *quote != b'"' && *quote != b'\'' {
        return None;
    }
    let inner = &after[1..];
    let end = inner.find(*quote as char)?;
    Some(inner[..end].to_string())
}

/// Merge JSX-collected head elements with API-specified head elements.
/// API elements take priority: title and same-name meta are overridden,
/// link/style/script are appended (both kept).
pub fn merge_head(jsx_head: &HeadContent, api_elements: &[HeadElement]) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let mut has_api_title = false;
    let mut api_meta_names: Vec<&str> = Vec::new();

    for el in api_elements {
        match el {
            HeadElement::Title(_) => has_api_title = true,
            HeadElement::Meta { name, .. } => api_meta_names.push(name),
            _ => {}
        }
    }

    for entry in &jsx_head.entries {
        match &entry.tag {
            HeadTag::Title if has_api_title => continue,
            HeadTag::Meta { name } if api_meta_names.contains(&name.as_str()) => continue,
            _ => result.push(entry.html.clone()),
        }
    }

    for el in api_elements {
        result.push(render_head_element(el));
    }

    result
}

fn render_head_element(el: &HeadElement) -> String {
    match el {
        HeadElement::Title(t) => format!("<title>{t}</title>"),
        HeadElement::Meta { name, content } => {
            format!(r#"<meta name="{name}" content="{content}">"#)
        }
        HeadElement::Link { rel, href } => {
            format!(r#"<link rel="{rel}" href="{href}">"#)
        }
        HeadElement::Style(css) => format!("<style>{css}</style>"),
        HeadElement::Script(js) => format!("<script>{js}</script>"),
    }
}
