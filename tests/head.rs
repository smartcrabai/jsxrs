mod common;

use jsxrs::{HeadElement, RenderConfig, render_file, render_string};
use serde_json::json;

use common::{config_with_head, extract_body, extract_head, fixtures_dir, minimal_config};

#[test]
fn should_render_title_in_head_when_jsx_contains_head_component() {
    // Given
    let source = r#"export default function Page() {
  return (
    <>
      <Head><title>My Page</title></Head>
      <div>content</div>
    </>
  );
}"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    let head = extract_head(&result);
    assert!(head.contains("<title>My Page</title>"));
}

#[test]
fn should_render_meta_in_head_when_jsx_contains_head_with_meta() {
    // Given
    let source = r#"export default function Page() {
  return (
    <>
      <Head>
        <meta name="description" content="A test page" />
      </Head>
      <div>content</div>
    </>
  );
}"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    let head = extract_head(&result);
    assert!(head.contains(r#"<meta name="description" content="A test page">"#));
}

#[test]
fn should_render_link_in_head_when_jsx_contains_head_with_link() {
    // Given
    let source = r#"export default function Page() {
  return (
    <>
      <Head>
        <link rel="stylesheet" href="/style.css" />
      </Head>
      <div>content</div>
    </>
  );
}"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    let head = extract_head(&result);
    assert!(head.contains(r#"<link rel="stylesheet" href="/style.css">"#));
}

#[test]
fn should_not_render_head_component_in_body() {
    // Given
    let source = r#"export default function Page() {
  return (
    <>
      <Head><title>Page</title></Head>
      <div>body content</div>
    </>
  );
}"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    let body = common::extract_body(&result);
    assert!(!body.contains("<Head>"));
    assert!(!body.contains("<title>"));
    assert!(body.contains("<div>body content</div>"));
}

#[test]
fn should_override_jsx_title_when_api_title_is_specified() {
    // Given: JSX has title "JSX Title", API specifies "API Title"
    let source = r#"export default function Page() {
  return (
    <>
      <Head><title>JSX Title</title></Head>
      <div>content</div>
    </>
  );
}"#;
    let config = config_with_head(vec![HeadElement::Title("API Title".to_string())]);

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    let head = extract_head(&result);
    assert!(head.contains("<title>API Title</title>"));
    assert!(!head.contains("JSX Title"));
}

#[test]
fn should_override_jsx_meta_when_api_specifies_same_name_meta() {
    // Given: JSX has description meta, API overrides it
    let source = r#"export default function Page() {
  return (
    <>
      <Head>
        <meta name="description" content="JSX description" />
      </Head>
      <div>content</div>
    </>
  );
}"#;
    let config = config_with_head(vec![HeadElement::Meta {
        name: "description".to_string(),
        content: "API description".to_string(),
    }]);

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    let head = extract_head(&result);
    assert!(head.contains("API description"));
    assert!(!head.contains("JSX description"));
}

#[test]
fn should_keep_both_link_tags_when_api_and_jsx_specify_links() {
    // Given: JSX has one link, API specifies another
    let source = r#"export default function Page() {
  return (
    <>
      <Head>
        <link rel="stylesheet" href="/jsx.css" />
      </Head>
      <div>content</div>
    </>
  );
}"#;
    let config = config_with_head(vec![HeadElement::Link {
        rel: "stylesheet".to_string(),
        href: "/api.css".to_string(),
    }]);

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    let head = extract_head(&result);
    assert!(head.contains("/jsx.css"));
    assert!(head.contains("/api.css"));
}

#[test]
fn should_render_api_head_elements_when_no_head_component_in_jsx() {
    // Given: no Head in JSX, API specifies head elements
    let source = r#"export default function Page() {
  return <div>content</div>;
}"#;
    let config = config_with_head(vec![
        HeadElement::Title("API Title".to_string()),
        HeadElement::Meta {
            name: "viewport".to_string(),
            content: "width=device-width".to_string(),
        },
    ]);

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    let head = extract_head(&result);
    assert!(head.contains("<title>API Title</title>"));
    assert!(head.contains(r#"<meta name="viewport" content="width=device-width">"#));
}

#[test]
fn should_render_head_from_fixture_file() {
    // Given
    let path = fixtures_dir().join("with_head.jsx");
    let config = minimal_config();

    // When
    let result = render_file(&path, &json!({}), &config).unwrap();

    // Then
    let head = extract_head(&result);
    assert!(head.contains("<title>My Page</title>"));
    assert!(head.contains(r#"<meta name="description" content="A test page">"#));
    assert!(head.contains(r#"<link rel="stylesheet" href="/style.css">"#));
}

#[test]
fn should_propagate_child_component_head_to_parent_output() {
    // Given: parent and child both have <Head> elements
    let path = fixtures_dir().join("with_child_head.jsx");
    let config = RenderConfig {
        pretty: false,
        base_dir: Some(fixtures_dir()),
        head_elements: Vec::new(),
        tailwind: false,
    };

    // When
    let result = render_file(&path, &json!({}), &config).unwrap();

    // Then: both parent and child head elements appear in <head>
    let head = extract_head(&result);
    assert!(
        head.contains("<title>Parent Title</title>"),
        "parent <Head> title missing from output head: {head}"
    );
    assert!(
        head.contains(r#"<meta name="child-meta" content="from-child">"#),
        "child component <Head> meta missing from output head: {head}"
    );
    // Child body content appears in <body>
    let body = extract_body(&result);
    assert!(
        body.contains("<section>hello</section>"),
        "child body content missing: {body}"
    );
}
