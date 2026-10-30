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

    assert_eq!(html.matches("<html>").count(), 1, "{html}");
    assert_eq!(html.matches("<head>").count(), 1, "{html}");
    assert_eq!(html.matches("<body>").count(), 1, "{html}");
    assert!(html.starts_with("<!DOCTYPE html><html><head>"), "{html}");
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
fn merges_page_head_into_existing_root_head_and_respects_pretty_config()
-> Result<(), Box<dyn std::error::Error>> {
    let sources = Arc::new(SourceMap::new(vec![
        (
            "app/layout.tsx",
            r#"export default function Root(props) { return <html><head><meta charset="utf-8" /></head><body>{props.children}</body></html>; }"#,
        ),
        (
            "app/page.tsx",
            r"export default function Page() { return <><Head><title>Page</title></Head><span>page</span></>; }",
        ),
    ]));
    let context = CheckedRenderContext::new().with_source_map(sources);
    let config = RenderConfig {
        pretty: true,
        ..RenderConfig::default()
    };
    let html = render_with_layouts_and_context(
        Path::new("app/page.tsx"),
        &[PathBuf::from("app/layout.tsx")],
        &json!({}),
        &config,
        &context,
    )?;
    assert!(html.starts_with("<!DOCTYPE html>\n<html>"), "{html}");
    assert_eq!(html.matches("<html>").count(), 1, "{html}");
    assert_eq!(html.matches("<head>").count(), 1, "{html}");
    let head = html
        .split_once("<head>")
        .ok_or("missing head")?
        .1
        .split_once("</head>")
        .ok_or("missing head end")?
        .0;
    assert!(head.contains("<title>Page</title>"), "{head}");
    assert!(head.contains(r#"<meta charset="utf-8">"#), "{head}");
    Ok(())
}

#[test]
fn render_layout_preserves_non_object_page_props() -> Result<(), Box<dyn std::error::Error>> {
    let sources = Arc::new(SourceMap::new(vec![
        (
            "app/layout.tsx",
            "export default function Layout(props) { return <main>{props.pageProps}:{props.children}</main>; }",
        ),
        (
            "app/page.tsx",
            "export default function Page() { return <span>page</span>; }",
        ),
    ]));
    let context = CheckedRenderContext::new().with_source_map(sources);
    let html = render_with_layouts_and_context(
        Path::new("app/page.tsx"),
        &[PathBuf::from("app/layout.tsx")],
        &json!("original"),
        &RenderConfig::default(),
        &context,
    )?;
    assert!(
        html.contains("<body><main>original:<span>page</span></main></body>"),
        "{html}"
    );
    Ok(())
}

#[test]
fn tailwind_classes_are_collected_across_page_and_layout() -> Result<(), Box<dyn std::error::Error>>
{
    let sources = Arc::new(SourceMap::new(vec![
        (
            "app/layout.tsx",
            r#"export default function Layout(props) { return <div className="bg-blue-500">{props.children}</div>; }"#,
        ),
        (
            "app/page.tsx",
            r#"export default function Page() { return <span className="text-xl">page</span>; }"#,
        ),
    ]));
    let context = CheckedRenderContext::new().with_source_map(sources);
    let config = RenderConfig {
        tailwind: true,
        ..RenderConfig::default()
    };
    let html = render_with_layouts_and_context(
        Path::new("app/page.tsx"),
        &[PathBuf::from("app/layout.tsx")],
        &json!({}),
        &config,
        &context,
    )?;
    assert_eq!(html.matches("<style>").count(), 1, "{html}");
    let style = html
        .split_once("<style>")
        .ok_or("missing Tailwind style")?
        .1
        .split_once("</style>")
        .ok_or("missing Tailwind style end")?
        .0;
    assert!(style.contains(".bg-blue-500"), "{style}");
    assert!(style.contains(".text-xl"), "{style}");
    Ok(())
}

#[test]
fn in_memory_imports_reject_missing_and_escaping_paths() {
    let missing = Arc::new(SourceMap::new(vec![(
        "app/page.tsx",
        r#"import Missing from "./missing"; export default function Page() { return <Missing />; }"#,
    )]));
    let escaping = Arc::new(SourceMap::new(vec![(
        "app/page.tsx",
        r#"import Escaping from "../../../outside"; export default function Page() { return <Escaping />; }"#,
    )]));
    let config = RenderConfig::default();
    for sources in [missing, escaping] {
        let context = CheckedRenderContext::new().with_source_map(sources);
        let result = jsxrs::render_file_with_context(
            Path::new("app/page.tsx"),
            &json!({}),
            &config,
            &context,
        );
        match result {
            Err(jsxrs::JsxrsError::ImportNotFound { .. }) => {}
            other => panic!("expected ImportNotFound, got {other:?}"),
        }
    }
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
