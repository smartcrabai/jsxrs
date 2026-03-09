use std::path::{Path, PathBuf};

/// A discovered route entry.
#[derive(Debug, Clone)]
pub struct RouteEntry {
    /// Axum path pattern (e.g., "/blog/:slug")
    pub axum_path: String,
    /// Absolute path to the page file
    pub page_file: PathBuf,
    /// Layout files from root to innermost
    pub layouts: Vec<PathBuf>,
}

/// Scan a directory tree for Next.js-style routes.
pub fn scan_routes(app_dir: &Path) -> Result<Vec<RouteEntry>, std::io::Error> {
    let app_dir = app_dir.canonicalize()?;
    let mut entries = Vec::new();
    collect_routes(&app_dir, &app_dir, "", &[], &mut entries)?;
    // Sort so that static routes come before dynamic ones
    entries.sort_by(|a, b| route_priority(&a.axum_path).cmp(&route_priority(&b.axum_path)));
    Ok(entries)
}

fn route_priority(path: &str) -> (usize, String) {
    let dynamic_count = path.matches('{').count();
    (dynamic_count, path.to_string())
}

#[allow(clippy::only_used_in_recursion)]
fn collect_routes(
    app_dir: &Path,
    current_dir: &Path,
    url_prefix: &str,
    parent_layouts: &[PathBuf],
    entries: &mut Vec<RouteEntry>,
) -> Result<(), std::io::Error> {
    // Check for layout in current directory
    let mut layouts = parent_layouts.to_vec();
    if let Some(layout) = find_file(current_dir, "layout") {
        layouts.push(layout);
    }

    // Check for page in current directory
    if let Some(page) = find_file(current_dir, "page") {
        let axum_path = if url_prefix.is_empty() {
            "/".to_string()
        } else {
            url_prefix.to_string()
        };
        entries.push(RouteEntry {
            axum_path,
            page_file: page,
            layouts: layouts.clone(),
        });
    }

    // Recurse into subdirectories
    let mut subdirs: Vec<_> = std::fs::read_dir(current_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .collect();
    subdirs.sort_by_key(|e| e.file_name());

    for entry in subdirs {
        let dir_name = entry.file_name().to_string_lossy().to_string();
        let segment = dir_name_to_segment(&dir_name);
        let child_prefix = match segment {
            Segment::Static(s) => format!("{url_prefix}/{s}"),
            Segment::Dynamic(param) => format!("{url_prefix}/{{{param}}}"),
            Segment::CatchAll(param) => format!("{url_prefix}/{{*{param}}}"),
            Segment::Group => url_prefix.to_string(), // no URL effect
        };
        collect_routes(app_dir, &entry.path(), &child_prefix, &layouts, entries)?;
    }

    Ok(())
}

enum Segment {
    Static(String),
    Dynamic(String),
    CatchAll(String),
    Group,
}

fn dir_name_to_segment(name: &str) -> Segment {
    if name.starts_with('(') && name.ends_with(')') {
        Segment::Group
    } else if name.starts_with("[...") && name.ends_with(']') {
        let param = &name[4..name.len() - 1];
        Segment::CatchAll(param.to_string())
    } else if name.starts_with('[') && name.ends_with(']') {
        let param = &name[1..name.len() - 1];
        Segment::Dynamic(param.to_string())
    } else {
        Segment::Static(name.to_string())
    }
}

fn find_file(dir: &Path, base_name: &str) -> Option<PathBuf> {
    for ext in &["tsx", "jsx"] {
        let path = dir.join(format!("{base_name}.{ext}"));
        if path.exists() {
            return Some(path);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_app(base: &Path) {
        let dirs = ["", "about", "blog", "blog/[slug]", "(marketing)/contact"];
        for d in &dirs {
            let dir = base.join(d);
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join("page.tsx"),
                "export default function Page() { return <div>test</div>; }",
            )
            .unwrap();
        }
        // Root layout
        fs::write(
            base.join("layout.tsx"),
            "export default function Layout() { return <html><body>{props.children}</body></html>; }",
        )
        .unwrap();
        // Blog layout
        fs::write(
            base.join("blog/layout.tsx"),
            "export default function BlogLayout() { return <div class=\"blog\">{props.children}</div>; }",
        )
        .unwrap();
    }

    #[test]
    fn test_scan_routes() {
        let tmp = std::env::temp_dir().join("jsxrs_test_discovery");
        let _ = fs::remove_dir_all(&tmp);
        create_test_app(&tmp);

        let routes = scan_routes(&tmp).unwrap();

        let paths: Vec<&str> = routes.iter().map(|r| r.axum_path.as_str()).collect();
        assert!(paths.contains(&"/"), "should have root route");
        assert!(paths.contains(&"/about"), "should have /about");
        assert!(paths.contains(&"/blog"), "should have /blog");
        assert!(
            paths.contains(&"/blog/{slug}"),
            "should have /blog/{{slug}}"
        );
        assert!(
            paths.contains(&"/contact"),
            "should have /contact (group stripped)"
        );

        // Check layouts
        let blog_slug = routes
            .iter()
            .find(|r| r.axum_path == "/blog/{slug}")
            .unwrap();
        assert_eq!(
            blog_slug.layouts.len(),
            2,
            "should have root + blog layouts"
        );

        let contact = routes.iter().find(|r| r.axum_path == "/contact").unwrap();
        assert_eq!(contact.layouts.len(), 1, "should have root layout only");

        let _ = fs::remove_dir_all(&tmp);
    }
}
