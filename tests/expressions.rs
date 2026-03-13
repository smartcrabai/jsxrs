mod common;

use jsxrs::render_string;
use serde_json::json;

use common::{extract_body, minimal_config};

// --- Variable references ---

#[test]
fn should_resolve_prop_reference_when_given_simple_variable() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r"export default function Page(props) {
  return <span>{props.greeting}</span>;
}";
    let props = json!({"greeting": "hi"});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config())?;

    // Then
    assert_eq!(extract_body(&result).ok_or("missing body")?, "<span>hi</span>");
    Ok(())
}

#[test]
fn should_resolve_nested_member_access_when_given_deep_prop() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r"export default function Page(props) {
  return <span>{props.user.name}</span>;
}";
    let props = json!({"user": {"name": "Alice"}});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config())?;

    // Then
    assert_eq!(extract_body(&result).ok_or("missing body")?, "<span>Alice</span>");
    Ok(())
}

// --- Ternary operator ---

#[test]
fn should_render_consequent_when_ternary_condition_is_truthy() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r"export default function Page(props) {
  return <div>{props.show ? <span>visible</span> : <span>hidden</span>}</div>;
}";
    let props = json!({"show": true});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config())?;

    // Then
    assert_eq!(extract_body(&result).ok_or("missing body")?, "<div><span>visible</span></div>");
    Ok(())
}

#[test]
fn should_render_alternate_when_ternary_condition_is_falsy() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r"export default function Page(props) {
  return <div>{props.show ? <span>visible</span> : <span>hidden</span>}</div>;
}";
    let props = json!({"show": false});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config())?;

    // Then
    assert_eq!(extract_body(&result).ok_or("missing body")?, "<div><span>hidden</span></div>");
    Ok(())
}

#[test]
fn should_render_text_consequent_when_ternary_returns_string() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r#"export default function Page(props) {
  return <span>{props.active ? "on" : "off"}</span>;
}"#;
    let props = json!({"active": true});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config())?;

    // Then
    assert_eq!(extract_body(&result).ok_or("missing body")?, "<span>on</span>");
    Ok(())
}

// --- Logical AND ---

#[test]
fn should_render_element_when_logical_and_lhs_is_truthy() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r"export default function Page(props) {
  return <div>{props.visible && <span>shown</span>}</div>;
}";
    let props = json!({"visible": true});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config())?;

    // Then
    assert_eq!(extract_body(&result).ok_or("missing body")?, "<div><span>shown</span></div>");
    Ok(())
}

#[test]
fn should_render_nothing_when_logical_and_lhs_is_falsy() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r"export default function Page(props) {
  return <div>{props.visible && <span>shown</span>}</div>;
}";
    let props = json!({"visible": false});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config())?;

    // Then
    assert_eq!(extract_body(&result).ok_or("missing body")?, "<div></div>");
    Ok(())
}

// --- Logical OR ---

#[test]
fn should_render_lhs_when_logical_or_lhs_is_truthy() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r#"export default function Page(props) {
  return <span>{props.name || "anonymous"}</span>;
}"#;
    let props = json!({"name": "Alice"});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config())?;

    // Then
    assert_eq!(extract_body(&result).ok_or("missing body")?, "<span>Alice</span>");
    Ok(())
}

#[test]
fn should_render_rhs_when_logical_or_lhs_is_falsy() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r#"export default function Page(props) {
  return <span>{props.name || "anonymous"}</span>;
}"#;
    let props = json!({"name": ""});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config())?;

    // Then
    assert_eq!(extract_body(&result).ok_or("missing body")?, "<span>anonymous</span>");
    Ok(())
}

// --- Logical NOT ---

#[test]
fn should_negate_boolean_when_given_logical_not() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r"export default function Page(props) {
  return <div>{!props.hidden && <span>shown</span>}</div>;
}";
    let props = json!({"hidden": false});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config())?;

    // Then
    assert_eq!(extract_body(&result).ok_or("missing body")?, "<div><span>shown</span></div>");
    Ok(())
}

// --- Template literals ---

#[test]
fn should_interpolate_template_literal_with_prop_value() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r"export default function Page(props) {
  return <span>{`Hello, ${props.name}!`}</span>;
}";
    let props = json!({"name": "World"});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config())?;

    // Then
    assert_eq!(extract_body(&result).ok_or("missing body")?, "<span>Hello, World!</span>");
    Ok(())
}

#[test]
fn should_interpolate_multiple_expressions_in_template_literal() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r"export default function Page(props) {
  return <span>{`${props.first} ${props.last}`}</span>;
}";
    let props = json!({"first": "John", "last": "Doe"});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config())?;

    // Then
    assert_eq!(extract_body(&result).ok_or("missing body")?, "<span>John Doe</span>");
    Ok(())
}
