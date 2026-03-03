mod common;

use jsxrs::render_string;
use serde_json::json;

use common::{extract_body, minimal_config};

#[test]
fn should_render_simple_div_when_given_basic_jsx() {
    // Given
    let source = r#"export default function Page() {
  return <div>Hello</div>;
}"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    let body = extract_body(&result);
    assert_eq!(body, "<div>Hello</div>");
}

#[test]
fn should_render_nested_elements_when_given_nested_jsx() {
    // Given
    let source = r#"export default function Page() {
  return <div><span>inner</span></div>;
}"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    let body = extract_body(&result);
    assert_eq!(body, "<div><span>inner</span></div>");
}

#[test]
fn should_render_self_closing_tags_when_given_void_elements() {
    // Given
    let source = r#"export default function Page() {
  return <div><br/><hr/></div>;
}"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    let body = extract_body(&result);
    assert_eq!(body, "<div><br><hr></div>");
}

#[test]
fn should_render_img_with_attributes_when_given_self_closing_element() {
    // Given
    let source = r#"export default function Page() {
  return <img src="test.png" alt="test" />;
}"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    let body = extract_body(&result);
    assert_eq!(body, r#"<img src="test.png" alt="test">"#);
}

#[test]
fn should_render_fragment_children_when_given_jsx_fragment() {
    // Given
    let source = r#"export default function Page() {
  return <><div>first</div><span>second</span></>;
}"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    let body = extract_body(&result);
    assert_eq!(body, "<div>first</div><span>second</span>");
}

#[test]
fn should_render_mixed_text_and_elements_when_given_interleaved_content() {
    // Given
    let source = r#"export default function Page() {
  return <div>Hello <strong>World</strong>!</div>;
}"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    let body = extract_body(&result);
    assert_eq!(body, "<div>Hello <strong>World</strong>!</div>");
}

#[test]
fn should_render_deeply_nested_elements_when_given_multiple_nesting_levels() {
    // Given
    let source = r#"export default function Page() {
  return <div><section><article><p>deep</p></article></section></div>;
}"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    let body = extract_body(&result);
    assert_eq!(
        body,
        "<div><section><article><p>deep</p></article></section></div>"
    );
}

#[test]
fn should_render_multiple_children_when_given_sibling_elements() {
    // Given
    let source = r#"export default function Page() {
  return (
    <ul>
      <li>one</li>
      <li>two</li>
      <li>three</li>
    </ul>
  );
}"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    let body = extract_body(&result);
    assert_eq!(body, "<ul><li>one</li><li>two</li><li>three</li></ul>");
}

#[test]
fn should_render_props_in_text_when_given_jsx_expression_with_props() {
    // Given
    let source = r#"export default function Greeting(props) {
  return <div>Hello, {props.name}!</div>;
}"#;
    let config = minimal_config();
    let props = json!({"name": "World"});

    // When
    let result = render_string(source, "greeting.jsx", &props, &config).unwrap();

    // Then
    let body = extract_body(&result);
    assert_eq!(body, "<div>Hello, World!</div>");
}

#[test]
fn should_render_tsx_source_when_given_tsx_file_name() {
    // Given
    let source = r#"
interface Props {
  name: string;
}
export default function Page(props: Props) {
  return <div>{props.name}</div>;
}"#;
    let config = minimal_config();
    let props = json!({"name": "TypeScript"});

    // When
    let result = render_string(source, "page.tsx", &props, &config).unwrap();

    // Then
    let body = extract_body(&result);
    assert_eq!(body, "<div>TypeScript</div>");
}

#[test]
fn should_render_arrow_function_default_export() {
    // Given
    let source = r#"const Page = () => <div>arrow</div>;
export default Page;"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    let body = extract_body(&result);
    assert_eq!(body, "<div>arrow</div>");
}

#[test]
fn should_render_number_prop_as_text_when_given_numeric_expression() {
    // Given
    let source = r#"export default function Page(props) {
  return <span>{props.count}</span>;
}"#;
    let config = minimal_config();
    let props = json!({"count": 42});

    // When
    let result = render_string(source, "page.jsx", &props, &config).unwrap();

    // Then
    let body = extract_body(&result);
    assert_eq!(body, "<span>42</span>");
}

#[test]
fn should_render_anonymous_arrow_default_export() {
    // Given
    let source = r#"export default () => <div>anon-arrow</div>;"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    let body = extract_body(&result);
    assert_eq!(body, "<div>anon-arrow</div>");
}

#[test]
fn should_render_anonymous_function_default_export() {
    // Given
    let source = r#"export default function() {
  return <div>anon-fn</div>;
}"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    let body = extract_body(&result);
    assert_eq!(body, "<div>anon-fn</div>");
}

#[test]
fn should_escape_html_in_text_content() {
    // Given
    let source = r#"export default function Page(props) {
  return <div>{props.text}</div>;
}"#;
    let config = minimal_config();
    let props = json!({"text": "<script>alert('xss')</script>"});

    // When
    let result = render_string(source, "page.jsx", &props, &config).unwrap();

    // Then
    let body = extract_body(&result);
    assert_eq!(body, "<div>&lt;script&gt;alert('xss')&lt;/script&gt;</div>");
}

#[test]
fn should_escape_quotes_in_attribute_values() {
    // Given
    let source = r#"export default function Page(props) {
  return <div title={props.title}>content</div>;
}"#;
    let config = minimal_config();
    let props = json!({"title": "a\"b"});

    // When
    let result = render_string(source, "page.jsx", &props, &config).unwrap();

    // Then
    let body = extract_body(&result);
    assert_eq!(body, r#"<div title="a&quot;b">content</div>"#);
}

#[test]
fn should_render_empty_element_when_given_no_children() {
    // Given
    let source = r#"export default function Page() {
  return <div></div>;
}"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config).unwrap();

    // Then
    let body = extract_body(&result);
    assert_eq!(body, "<div></div>");
}
