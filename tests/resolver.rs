mod common;

use jsxrs::{render_file, render_string};
use serde_json::json;

use common::{config_with_base_dir, extract_body, fixtures_dir, minimal_config};

#[test]
fn should_resolve_imported_component_when_given_relative_import()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: with_import.jsx imports ./components/button
    let path = fixtures_dir().join("with_import.jsx");
    let config = config_with_base_dir(fixtures_dir());

    // When
    let result = render_file(&path, &json!({}), &config)?;

    // Then
    let body = extract_body(&result).ok_or("missing body")?;
    assert!(body.contains("<h1>Title</h1>"));
    assert!(body.contains("<button>Click me</button>"));
    Ok(())
}

#[test]
fn should_resolve_import_without_extension_when_jsx_file_exists() {
    // Given: import path './components/button' resolved to 'button.jsx'
    let path = fixtures_dir().join("with_import.jsx");
    let config = config_with_base_dir(fixtures_dir());

    // When
    let result = render_file(&path, &json!({}), &config);

    // Then
    assert!(result.is_ok());
}

#[test]
fn should_return_error_when_imported_component_not_found() {
    // Given: source imports a non-existent component
    let source = r"import Missing from './nonexistent';
export default function Page() {
  return <Missing />;
}";
    let config = config_with_base_dir(fixtures_dir());

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config);

    // Then
    assert!(result.is_err());
}

#[test]
fn should_pass_props_to_imported_component() -> Result<(), Box<dyn std::error::Error>> {
    // Given: with_import.jsx passes label="Click me" to Button
    let path = fixtures_dir().join("with_import.jsx");
    let config = config_with_base_dir(fixtures_dir());

    // When
    let result = render_file(&path, &json!({}), &config)?;

    // Then
    let body = extract_body(&result).ok_or("missing body")?;
    assert!(body.contains("<button>Click me</button>"));
    Ok(())
}

#[test]
fn should_use_base_dir_for_import_resolution() -> Result<(), Box<dyn std::error::Error>> {
    // Given: source with relative import, base_dir set to fixtures
    let source = r#"import Button from './components/button';
export default function Page() {
  return <Button label="test" />;
}"#;
    let config = config_with_base_dir(fixtures_dir());

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config)?;

    // Then
    let body = extract_body(&result).ok_or("missing body")?;
    assert!(body.contains("<button>test</button>"));
    Ok(())
}

#[test]
fn should_return_error_when_base_dir_not_set_and_import_exists() {
    // Given: source with import, but no base_dir configured
    let source = r#"import Button from './components/button';
export default function Page() {
  return <Button label="test" />;
}"#;
    let config = minimal_config();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config);

    // Then
    assert!(result.is_err());
}
