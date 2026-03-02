mod common;

use jsxrs::render_string;
use serde_json::json;

use common::{extract_body, minimal_config};

// --- Array.map ---

#[test]
fn should_render_list_when_given_array_map() {
    // Given
    let source = r#"export default function Page(props) {
  return <ul>{props.items.map(item => <li>{item}</li>)}</ul>;
}"#;
    let props = json!({"items": ["a", "b", "c"]});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config()).unwrap();

    // Then
    assert_eq!(
        extract_body(&result),
        "<ul><li>a</li><li>b</li><li>c</li></ul>"
    );
}

#[test]
fn should_render_empty_list_when_given_empty_array_map() {
    // Given
    let source = r#"export default function Page(props) {
  return <ul>{props.items.map(item => <li>{item}</li>)}</ul>;
}"#;
    let props = json!({"items": []});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config()).unwrap();

    // Then
    assert_eq!(extract_body(&result), "<ul></ul>");
}

#[test]
fn should_access_object_properties_in_map_callback() {
    // Given
    let source = r#"export default function Page(props) {
  return <ul>{props.users.map(user => <li>{user.name}</li>)}</ul>;
}"#;
    let props = json!({"users": [{"name": "Alice"}, {"name": "Bob"}]});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config()).unwrap();

    // Then
    assert_eq!(extract_body(&result), "<ul><li>Alice</li><li>Bob</li></ul>");
}

// --- Comparison operators ---

#[test]
fn should_evaluate_equality_comparison_in_ternary() {
    // Given
    let source = r#"export default function Page(props) {
  return <span>{props.status === "active" ? "yes" : "no"}</span>;
}"#;
    let props = json!({"status": "active"});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config()).unwrap();

    // Then
    assert_eq!(extract_body(&result), "<span>yes</span>");
}

#[test]
fn should_evaluate_numeric_comparison_in_ternary() {
    // Given
    let source = r#"export default function Page(props) {
  return <span>{props.count > 0 ? "has items" : "empty"}</span>;
}"#;
    let props = json!({"count": 5});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config()).unwrap();

    // Then
    assert_eq!(extract_body(&result), "<span>has items</span>");
}

// --- String concatenation ---

#[test]
fn should_concatenate_strings_with_plus_operator() {
    // Given
    let source = r#"export default function Page(props) {
  return <span>{props.first + " " + props.last}</span>;
}"#;
    let props = json!({"first": "Jane", "last": "Doe"});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config()).unwrap();

    // Then
    assert_eq!(extract_body(&result), "<span>Jane Doe</span>");
}

// --- JS Truthiness ---

#[test]
fn should_treat_null_as_falsy() {
    // Given
    let source = r#"export default function Page(props) {
  return <div>{props.value && <span>truthy</span>}</div>;
}"#;
    let props = json!({"value": null});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config()).unwrap();

    // Then
    assert_eq!(extract_body(&result), "<div></div>");
}

#[test]
fn should_treat_zero_as_falsy() {
    // Given
    let source = r#"export default function Page(props) {
  return <div>{props.count && <span>truthy</span>}</div>;
}"#;
    let props = json!({"count": 0});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config()).unwrap();

    // Then
    assert_eq!(extract_body(&result), "<div></div>");
}

#[test]
fn should_treat_empty_string_as_falsy() {
    // Given
    let source = r#"export default function Page(props) {
  return <div>{props.name && <span>truthy</span>}</div>;
}"#;
    let props = json!({"name": ""});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config()).unwrap();

    // Then
    assert_eq!(extract_body(&result), "<div></div>");
}

#[test]
fn should_treat_nonzero_number_as_truthy() {
    // Given
    let source = r#"export default function Page(props) {
  return <div>{props.count && <span>truthy</span>}</div>;
}"#;
    let props = json!({"count": 1});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config()).unwrap();

    // Then
    assert_eq!(extract_body(&result), "<div><span>truthy</span></div>");
}
