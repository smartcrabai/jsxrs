mod common;

use jsxrs::{JsxrsError, render_string};
use serde_json::json;

use common::minimal_config;

#[test]
fn should_return_parse_error_when_given_invalid_jsx_syntax()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: unclosed tag
    let source = r"export default function Page() {
  return <div><span></div>;
}";

    // When
    let result = render_string(source, "page.jsx", &json!({}), &minimal_config());

    // Then
    assert!(result.is_err());
    assert!(matches!(
        result.err().ok_or("expected Err but got Ok")?,
        JsxrsError::Parse(_)
    ));
    Ok(())
}

#[test]
fn should_return_error_when_no_default_export_found() {
    // Given: no default export
    let source = r"function Page() {
  return <div>Hello</div>;
}";

    // When
    let result = render_string(source, "page.jsx", &json!({}), &minimal_config());

    // Then
    assert!(result.is_err());
}

#[test]
fn should_return_unsupported_error_when_given_variable_declaration() {
    // Given: variable declaration in JSX expression
    let source = r"export default function Page() {
  return <div>{let x = 1}</div>;
}";

    // When
    let result = render_string(source, "page.jsx", &json!({}), &minimal_config());

    // Then
    assert!(result.is_err());
}

#[test]
fn should_return_error_when_file_not_found() {
    // Given: non-existent file path
    let path = common::fixtures_dir().join("nonexistent.jsx");

    // When
    let result = jsxrs::render_file(&path, &json!({}), &minimal_config());

    // Then
    assert!(result.is_err());
}

#[test]
fn should_return_error_when_referencing_undefined_prop() {
    // Given: source accesses prop that doesn't exist
    let source = r"export default function Page(props) {
  return <div>{props.missing}</div>;
}";
    let props = json!({});

    // When
    let result = render_string(source, "page.jsx", &props, &minimal_config());

    // Then
    assert!(result.is_err());
}

#[test]
fn should_return_error_when_import_target_not_found() {
    // Given: import that cannot be resolved
    let source = r"import Foo from './does_not_exist';
export default function Page() {
  return <Foo />;
}";
    let config = common::config_with_base_dir(common::fixtures_dir());

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config);

    // Then
    assert!(result.is_err());
}

#[test]
fn should_return_parse_error_when_given_invalid_tsx_syntax()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: invalid TSX
    let source = r"
interface Props {
  name: string
}
export default function Page(props: Props) {
  return <div><span></div>;
}";

    // When
    let result = render_string(source, "page.tsx", &json!({}), &minimal_config());

    // Then
    assert!(result.is_err());
    assert!(matches!(
        result.err().ok_or("expected Err but got Ok")?,
        JsxrsError::Parse(_)
    ));
    Ok(())
}

#[test]
fn should_return_error_when_source_is_empty() {
    // Given
    let source = "";

    // When
    let result = render_string(source, "page.jsx", &json!({}), &minimal_config());

    // Then
    assert!(result.is_err());
}
