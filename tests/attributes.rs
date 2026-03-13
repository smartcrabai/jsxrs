mod common;

use jsxrs::render_string;
use serde_json::json;

use common::{extract_body, minimal_config};

#[test]
fn should_convert_classname_to_class_when_given_classname_attribute()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r#"export default function Page() {
  return <div className="container">content</div>;
}"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config)?;

    // Then
    let body = extract_body(&result).ok_or("missing body")?;
    assert_eq!(body, r#"<div class="container">content</div>"#);
    Ok(())
}

#[test]
fn should_convert_htmlfor_to_for_when_given_htmlfor_attribute()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r#"export default function Page() {
  return <label htmlFor="email">Email</label>;
}"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config)?;

    // Then
    let body = extract_body(&result).ok_or("missing body")?;
    assert_eq!(body, r#"<label for="email">Email</label>"#);
    Ok(())
}

#[test]
fn should_convert_style_object_to_css_string_when_given_camelcase_styles()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r"export default function Page() {
  return <div style={{color: 'red', fontSize: '14px', backgroundColor: 'blue'}}>styled</div>;
}";
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config)?;

    // Then
    let body = extract_body(&result).ok_or("missing body")?;
    assert!(body.contains(r#"style=""#));
    assert!(body.contains("color: red"));
    assert!(body.contains("font-size: 14px"));
    assert!(body.contains("background-color: blue"));
    Ok(())
}

#[test]
fn should_render_boolean_attribute_name_only_when_value_is_true()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r"export default function Page() {
  return <input disabled={true} />;
}";
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config)?;

    // Then
    let body = extract_body(&result).ok_or("missing body")?;
    assert_eq!(body, "<input disabled>");
    Ok(())
}

#[test]
fn should_omit_boolean_attribute_when_value_is_false() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r"export default function Page() {
  return <input disabled={false} />;
}";
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config)?;

    // Then
    let body = extract_body(&result).ok_or("missing body")?;
    assert_eq!(body, "<input>");
    Ok(())
}

#[test]
fn should_exclude_event_handler_attributes_from_html_output()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r"export default function Page() {
  return <button onClick={handler} onMouseOver={handler}>click</button>;
}";
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config)?;

    // Then
    let body = extract_body(&result).ok_or("missing body")?;
    assert_eq!(body, "<button>click</button>");
    Ok(())
}

#[test]
fn should_pass_through_standard_html_attributes() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r#"export default function Page() {
  return <a href="/link" target="_blank" rel="noopener">link</a>;
}"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config)?;

    // Then
    let body = extract_body(&result).ok_or("missing body")?;
    assert_eq!(
        body,
        r#"<a href="/link" target="_blank" rel="noopener">link</a>"#
    );
    Ok(())
}

#[test]
fn should_pass_through_data_attributes() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r#"export default function Page() {
  return <div data-id="123" data-testid="item">content</div>;
}"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config)?;

    // Then
    let body = extract_body(&result).ok_or("missing body")?;
    assert!(body.contains(r#"data-id="123""#));
    assert!(body.contains(r#"data-testid="item""#));
    Ok(())
}

#[test]
fn should_convert_tabindex_to_lowercase_when_given_camelcase()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r"export default function Page() {
  return <div tabIndex={0}>focusable</div>;
}";
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config)?;

    // Then
    let body = extract_body(&result).ok_or("missing body")?;
    assert_eq!(body, r#"<div tabindex="0">focusable</div>"#);
    Ok(())
}

#[test]
fn should_render_shorthand_boolean_attribute_when_no_value_specified()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r"export default function Page() {
  return <input required />;
}";
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config)?;

    // Then
    let body = extract_body(&result).ok_or("missing body")?;
    assert_eq!(body, "<input required>");
    Ok(())
}

#[test]
fn should_render_dynamic_attribute_value_from_props() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r"export default function Page(props) {
  return <div id={props.elementId}>content</div>;
}";
    let config = minimal_config();
    let props = json!({"elementId": "main-content"});

    // When
    let result = render_string(source, "page.jsx", &props, &config)?;

    // Then
    let body = extract_body(&result).ok_or("missing body")?;
    assert_eq!(body, r#"<div id="main-content">content</div>"#);
    Ok(())
}

#[test]
fn should_handle_multiple_react_specific_attributes() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r#"export default function Page() {
  return <div className="wrapper" tabIndex={-1}><label htmlFor="name">Name</label></div>;
}"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config)?;

    // Then
    let body = extract_body(&result).ok_or("missing body")?;
    assert!(body.contains(r#"class="wrapper""#));
    assert!(body.contains(r#"tabindex="-1""#));
    assert!(body.contains(r#"for="name""#));
    Ok(())
}
