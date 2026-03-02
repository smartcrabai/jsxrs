mod common;

use jsxrs::render_string;
use serde_json::json;

use common::{minimal_config, pretty_config};

#[test]
fn should_produce_complete_html_document_when_rendering_jsx() {
    // Given
    let source = r#"export default function Page() {
  return <div>Hello</div>;
}"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    assert!(result.starts_with("<!DOCTYPE html>"));
    assert!(result.contains("<html>"));
    assert!(result.contains("<head>"));
    assert!(result.contains("</head>"));
    assert!(result.contains("<body>"));
    assert!(result.contains("<div>Hello</div>"));
    assert!(result.contains("</body>"));
    assert!(result.contains("</html>"));
}

#[test]
fn should_not_contain_extra_whitespace_when_pretty_is_false() {
    // Given
    let source = r#"export default function Page() {
  return <div>Hello</div>;
}"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    assert_eq!(
        result,
        "<!DOCTYPE html><html><head></head><body><div>Hello</div></body></html>"
    );
}

#[test]
fn should_contain_indentation_when_pretty_is_true() {
    // Given
    let source = r#"export default function Page() {
  return <div>Hello</div>;
}"#;
    let config = pretty_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    assert!(result.starts_with("<!DOCTYPE html>"));
    assert!(result.contains('\n'));
    assert!(result.contains("<html>"));
    assert!(result.contains("  <head>")); // indented
    assert!(result.contains("  <body>")); // indented
    assert!(result.contains("<div>Hello</div>"));
}

#[test]
fn should_include_empty_head_when_no_head_elements_specified() {
    // Given
    let source = r#"export default function Page() {
  return <div>content</div>;
}"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    assert!(result.contains("<head></head>"));
}

#[test]
fn should_render_file_when_given_file_path() {
    // Given
    let path = common::fixtures_dir().join("simple.jsx");
    let config = minimal_config();

    // When
    let result = jsxrs::render_file(&path, &json!({}), &config).unwrap();

    // Then
    assert!(result.starts_with("<!DOCTYPE html>"));
    assert!(result.contains("<div>Hello</div>"));
}

#[test]
fn should_render_file_with_props_when_given_file_path_and_props() {
    // Given
    let path = common::fixtures_dir().join("with_props.jsx");
    let config = minimal_config();
    let props = json!({"name": "World"});

    // When
    let result = jsxrs::render_file(&path, &props, &config).unwrap();

    // Then
    assert!(result.contains("<div>Hello, World!</div>"));
}
