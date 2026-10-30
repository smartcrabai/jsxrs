use std::path::{Path, PathBuf};
use std::sync::Arc;

use jsxrs::{CheckedRenderContext, RenderConfig, SourceMap, render_with_layouts_and_context};
use serde_json::json;

#[test]
fn render_layout_chain_resolves_components_and_merges_head_from_memory()
-> Result<(), Box<dyn std::error::Error>> {
    let sources = Arc::new(SourceMap::new(vec![
        (
            "app/layout.tsx",
            r"export default function Root(props) { return <html><body><Head><title>Root</title></Head>{props.site}:{props.children}</body></html>; }",
        ),
        (
            "app/blog/layout.tsx",
            r#"export default function Blog(props) { return <section><Head><title>Blog</title><meta name="description" content="blog" /></Head>{props.section}:{props.children}</section>; }"#,
        ),
        (
            "app/blog/page.tsx",
            r#"import Badge from "./components/badge"; export default function Page(props) { return <main><Head><title>Page</title><meta name="keywords" content="page" /></Head>{props.name}<Badge label={props.label} /></main>; }"#,
        ),
        (
            "app/blog/components/badge.tsx",
            r"export default function Badge(props) { return <em>{props.label}</em>; }",
        ),
    ]));
    let context = CheckedRenderContext::new().with_source_map(Arc::clone(&sources));
    let config = RenderConfig::default();
    let html = render_with_layouts_and_context(
        Path::new("app/blog/page.tsx"),
        &[
            PathBuf::from("app/layout.tsx"),
            PathBuf::from("app/blog/layout.tsx"),
        ],
        &json!({"site": "site", "section": "posts", "name": "entry", "label": "tag"}),
        &config,
        &context,
    )?;

    assert!(html.contains("<title>Root</title>"), "{html}");
    assert!(html.contains("<title>Blog</title>"), "{html}");
    assert!(html.contains("<title>Page</title>"), "{html}");
    assert!(
        html.contains(r#"<meta name="description" content="blog">"#),
        "{html}"
    );
    assert!(
        html.contains(r#"<meta name="keywords" content="page">"#),
        "{html}"
    );
    assert!(
        html.contains("<body>site:<section>posts:<main>entry<em>tag</em></main></section></body>"),
        "{html}"
    );
    Ok(())
}

#[test]
fn source_map_rejects_paths_outside_source_root() -> Result<(), Box<dyn std::error::Error>> {
    let sources = Arc::new(SourceMap::new(vec![(
        "app/page.tsx",
        "export default function Page() { return <main />; }",
    )]));
    let context = CheckedRenderContext::new().with_source_map(sources);
    let config = RenderConfig::default();
    let props = json!({});
    let outside = Path::new("../app/page.tsx");
    let absolute = std::env::current_dir()?.join("app/page.tsx");
    for path in [outside, absolute.as_path()] {
        assert!(
            jsxrs::render_file_with_context(path, &props, &config, &context).is_err(),
            "accepted source path outside source root: {}",
            path.display()
        );
    }
    Ok(())
}
