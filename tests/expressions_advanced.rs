mod common;

use jsxrs::render_string;
use serde_json::json;

use common::{extract_body, minimal_config};

// --- Array.map ---

#[test]
fn should_render_list_when_given_array_map() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r"export default function Page(props) {
  return <ul>{props.items.map(item => <li>{item}</li>)}</ul>;
}";
    let props = json!({"items": ["a", "b", "c"]});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config())?;

    // Then
    assert_eq!(
        extract_body(&result).ok_or("missing body")?,
        "<ul><li>a</li><li>b</li><li>c</li></ul>"
    );
    Ok(())
}

#[test]
fn should_render_empty_list_when_given_empty_array_map() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r"export default function Page(props) {
  return <ul>{props.items.map(item => <li>{item}</li>)}</ul>;
}";
    let props = json!({"items": []});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config())?;

    // Then
    assert_eq!(extract_body(&result).ok_or("missing body")?, "<ul></ul>");
    Ok(())
}

#[test]
fn should_access_object_properties_in_map_callback() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r"export default function Page(props) {
  return <ul>{props.users.map(user => <li>{user.name}</li>)}</ul>;
}";
    let props = json!({"users": [{"name": "Alice"}, {"name": "Bob"}]});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config())?;

    // Then
    assert_eq!(extract_body(&result).ok_or("missing body")?, "<ul><li>Alice</li><li>Bob</li></ul>");
    Ok(())
}

// --- Comparison operators ---

#[test]
fn should_evaluate_equality_comparison_in_ternary() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r#"export default function Page(props) {
  return <span>{props.status === "active" ? "yes" : "no"}</span>;
}"#;
    let props = json!({"status": "active"});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config())?;

    // Then
    assert_eq!(extract_body(&result).ok_or("missing body")?, "<span>yes</span>");
    Ok(())
}

#[test]
fn should_evaluate_numeric_comparison_in_ternary() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r#"export default function Page(props) {
  return <span>{props.count > 0 ? "has items" : "empty"}</span>;
}"#;
    let props = json!({"count": 5});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config())?;

    // Then
    assert_eq!(extract_body(&result).ok_or("missing body")?, "<span>has items</span>");
    Ok(())
}

// --- String concatenation ---

#[test]
fn should_concatenate_strings_with_plus_operator() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r#"export default function Page(props) {
  return <span>{props.first + " " + props.last}</span>;
}"#;
    let props = json!({"first": "Jane", "last": "Doe"});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config())?;

    // Then
    assert_eq!(extract_body(&result).ok_or("missing body")?, "<span>Jane Doe</span>");
    Ok(())
}

// --- JS Truthiness ---

#[test]
fn should_treat_null_as_falsy() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r"export default function Page(props) {
  return <div>{props.value && <span>truthy</span>}</div>;
}";
    let props = json!({"value": null});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config())?;

    // Then
    assert_eq!(extract_body(&result).ok_or("missing body")?, "<div></div>");
    Ok(())
}

#[test]
fn should_treat_zero_as_falsy() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r"export default function Page(props) {
  return <div>{props.count && <span>truthy</span>}</div>;
}";
    let props = json!({"count": 0});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config())?;

    // Then
    assert_eq!(extract_body(&result).ok_or("missing body")?, "<div></div>");
    Ok(())
}

#[test]
fn should_treat_empty_string_as_falsy() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r"export default function Page(props) {
  return <div>{props.name && <span>truthy</span>}</div>;
}";
    let props = json!({"name": ""});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config())?;

    // Then
    assert_eq!(extract_body(&result).ok_or("missing body")?, "<div></div>");
    Ok(())
}

#[test]
fn should_treat_nonzero_number_as_truthy() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r"export default function Page(props) {
  return <div>{props.count && <span>truthy</span>}</div>;
}";
    let props = json!({"count": 1});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config())?;

    // Then
    assert_eq!(extract_body(&result).ok_or("missing body")?, "<div><span>truthy</span></div>");
    Ok(())
}
