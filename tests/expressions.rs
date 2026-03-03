mod common;

use jsxrs::render_string;
use serde_json::json;

use common::{extract_body, minimal_config};

// --- Variable references ---

#[test]
fn should_resolve_prop_reference_when_given_simple_variable() {
    // Given
    let source = r#"export default function Page(props) {
  return <span>{props.greeting}</span>;
}"#;
    let props = json!({"greeting": "hi"});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config()).unwrap();

    // Then
    assert_eq!(extract_body(&result), "<span>hi</span>");
}

#[test]
fn should_resolve_nested_member_access_when_given_deep_prop() {
    // Given
    let source = r#"export default function Page(props) {
  return <span>{props.user.name}</span>;
}"#;
    let props = json!({"user": {"name": "Alice"}});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config()).unwrap();

    // Then
    assert_eq!(extract_body(&result), "<span>Alice</span>");
}

// --- Ternary operator ---

#[test]
fn should_render_consequent_when_ternary_condition_is_truthy() {
    // Given
    let source = r#"export default function Page(props) {
  return <div>{props.show ? <span>visible</span> : <span>hidden</span>}</div>;
}"#;
    let props = json!({"show": true});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config()).unwrap();

    // Then
    assert_eq!(extract_body(&result), "<div><span>visible</span></div>");
}

#[test]
fn should_render_alternate_when_ternary_condition_is_falsy() {
    // Given
    let source = r#"export default function Page(props) {
  return <div>{props.show ? <span>visible</span> : <span>hidden</span>}</div>;
}"#;
    let props = json!({"show": false});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config()).unwrap();

    // Then
    assert_eq!(extract_body(&result), "<div><span>hidden</span></div>");
}

#[test]
fn should_render_text_consequent_when_ternary_returns_string() {
    // Given
    let source = r#"export default function Page(props) {
  return <span>{props.active ? "on" : "off"}</span>;
}"#;
    let props = json!({"active": true});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config()).unwrap();

    // Then
    assert_eq!(extract_body(&result), "<span>on</span>");
}

// --- Logical AND ---

#[test]
fn should_render_element_when_logical_and_lhs_is_truthy() {
    // Given
    let source = r#"export default function Page(props) {
  return <div>{props.visible && <span>shown</span>}</div>;
}"#;
    let props = json!({"visible": true});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config()).unwrap();

    // Then
    assert_eq!(extract_body(&result), "<div><span>shown</span></div>");
}

#[test]
fn should_render_nothing_when_logical_and_lhs_is_falsy() {
    // Given
    let source = r#"export default function Page(props) {
  return <div>{props.visible && <span>shown</span>}</div>;
}"#;
    let props = json!({"visible": false});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config()).unwrap();

    // Then
    assert_eq!(extract_body(&result), "<div></div>");
}

// --- Logical OR ---

#[test]
fn should_render_lhs_when_logical_or_lhs_is_truthy() {
    // Given
    let source = r#"export default function Page(props) {
  return <span>{props.name || "anonymous"}</span>;
}"#;
    let props = json!({"name": "Alice"});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config()).unwrap();

    // Then
    assert_eq!(extract_body(&result), "<span>Alice</span>");
}

#[test]
fn should_render_rhs_when_logical_or_lhs_is_falsy() {
    // Given
    let source = r#"export default function Page(props) {
  return <span>{props.name || "anonymous"}</span>;
}"#;
    let props = json!({"name": ""});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config()).unwrap();

    // Then
    assert_eq!(extract_body(&result), "<span>anonymous</span>");
}

// --- Logical NOT ---

#[test]
fn should_negate_boolean_when_given_logical_not() {
    // Given
    let source = r#"export default function Page(props) {
  return <div>{!props.hidden && <span>shown</span>}</div>;
}"#;
    let props = json!({"hidden": false});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config()).unwrap();

    // Then
    assert_eq!(extract_body(&result), "<div><span>shown</span></div>");
}

// --- Template literals ---

#[test]
fn should_interpolate_template_literal_with_prop_value() {
    // Given
    let source = r#"export default function Page(props) {
  return <span>{`Hello, ${props.name}!`}</span>;
}"#;
    let props = json!({"name": "World"});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config()).unwrap();

    // Then
    assert_eq!(extract_body(&result), "<span>Hello, World!</span>");
}

#[test]
fn should_interpolate_multiple_expressions_in_template_literal() {
    // Given
    let source = r#"export default function Page(props) {
  return <span>{`${props.first} ${props.last}`}</span>;
}"#;
    let props = json!({"first": "John", "last": "Doe"});

    // When
    let result = render_string(source, "p.jsx", &props, &minimal_config()).unwrap();

    // Then
    assert_eq!(extract_body(&result), "<span>John Doe</span>");
}
