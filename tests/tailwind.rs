mod common;

use jsxrs::render_string;
use serde_json::json;

use common::{config_with_tailwind, extract_head, minimal_config};

#[test]
fn should_generate_tailwind_css_in_head_when_classes_used() -> Result<(), Box<dyn std::error::Error>>
{
    // Given
    let source = r#"export default function Page() {
  return <div className="bg-blue-500 text-white p-4">content</div>;
}"#;
    let config = config_with_tailwind();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config)?;

    // Then
    let head = extract_head(&result).ok_or("missing head")?;
    assert!(head.contains("<style>"));
    assert!(head.contains("</style>"));
    Ok(())
}

#[test]
fn should_not_add_style_tag_when_no_tailwind_classes_used() -> Result<(), Box<dyn std::error::Error>>
{
    // Given
    let source = r"export default function Page() {
  return <div>plain content</div>;
}";
    let config = config_with_tailwind();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config)?;

    // Then
    let head = extract_head(&result).ok_or("missing head")?;
    assert!(!head.contains("<style>"));
    Ok(())
}

#[test]
fn should_not_process_tailwind_when_disabled_in_config() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r#"export default function Page() {
  return <div className="bg-blue-500 text-white">content</div>;
}"#;
    let config = minimal_config(); // tailwind: false

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config)?;

    // Then
    let head = extract_head(&result).ok_or("missing head")?;
    assert!(!head.contains("<style>"));
    Ok(())
}

#[test]
fn should_collect_classes_from_nested_elements() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = r#"export default function Page() {
  return (
    <div className="p-4">
      <h1 className="text-xl font-bold">title</h1>
      <p className="mt-2">text</p>
    </div>
  );
}"#;
    let config = config_with_tailwind();

    // When
    let result = render_string(source, "page.jsx", &json!({}), &config)?;

    // Then
    let head = extract_head(&result).ok_or("missing head")?;
    assert!(head.contains("<style>"));
    Ok(())
}

#[test]
fn should_render_tailwind_css_from_fixture_file() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let path = common::fixtures_dir().join("tailwind.jsx");
    let mut config = config_with_tailwind();
    config.base_dir = Some(common::fixtures_dir());

    // When
    let result = jsxrs::render_file(&path, &json!({}), &config)?;

    // Then
    let head = extract_head(&result).ok_or("missing head")?;
    assert!(head.contains("<style>"));
    Ok(())
}
