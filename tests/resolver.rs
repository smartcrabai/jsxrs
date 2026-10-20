mod common;

use std::fs;

use jsxrs::{RenderConfig, render_file, render_string};
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

#[test]
fn should_resolve_nested_imports_relative_to_the_importing_file()
-> Result<(), Box<dyn std::error::Error>> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "jsxrs_nested_import_{}_{}",
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(root.join("components/nested"))?;
    fs::write(
        root.join("page.jsx"),
        r#"import Panel from "./components/panel";
export default function Page() { return <Panel />; }"#,
    )?;
    fs::write(
        root.join("components/panel.jsx"),
        r#"import Label from "./nested/label";
export default function Panel() { return <Label />; }"#,
    )?;
    fs::write(
        root.join("components/nested/label.jsx"),
        "export default function Label() { return <strong>nested import</strong>; }",
    )?;

    let config = RenderConfig {
        base_dir: Some(root.clone()),
        fragment: true,
        ..RenderConfig::default()
    };
    let rendered = render_file(&root.join("page.jsx"), &json!({}), &config);
    let cleanup = fs::remove_dir_all(&root);

    let result = rendered?;
    cleanup?;
    assert_eq!(result, "<strong>nested import</strong>");
    Ok(())
}
