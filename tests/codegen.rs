mod common;

use std::fs;
use std::process::Command;

use jsxrs::codegen::generate_types;

use common::fixtures_dir;

#[test]
fn should_generate_rust_struct_from_tsx_interface() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let tsx_path = fixtures_dir().join("typed_interface.tsx");
    let output_dir = std::env::temp_dir().join("jsxrs_test_codegen_interface");
    fs::create_dir_all(&output_dir)?;
    let output_path = output_dir.join("generated.rs");

    // When
    let result = generate_types(&[tsx_path], &output_path);

    // Then
    assert!(result.is_ok());
    let generated = fs::read_to_string(&output_path)?;
    assert!(generated.contains("struct CardProps"));
    assert!(generated.contains("title"));
    assert!(generated.contains("count"));
    assert!(generated.contains("active"));
    assert!(generated.contains("tags"));

    // Cleanup
    let _ = fs::remove_dir_all(&output_dir);
    Ok(())
}

#[test]
fn should_generate_rust_struct_with_correct_field_types() -> Result<(), Box<dyn std::error::Error>>
{
    // Given: typed_interface.tsx has string, number, boolean, string[]
    let tsx_path = fixtures_dir().join("typed_interface.tsx");
    let output_dir = std::env::temp_dir().join("jsxrs_test_codegen_types");
    fs::create_dir_all(&output_dir)?;
    let output_path = output_dir.join("generated.rs");

    // When
    generate_types(&[tsx_path], &output_path)?;

    // Then
    let generated = fs::read_to_string(&output_path)?;
    assert!(generated.contains("String")); // string -> String
    assert!(generated.contains("f64") || generated.contains("i64")); // number -> numeric
    assert!(generated.contains("bool")); // boolean -> bool
    assert!(generated.contains("Vec<String>")); // string[] -> Vec<String>

    // Cleanup
    let _ = fs::remove_dir_all(&output_dir);
    Ok(())
}

#[test]
fn should_generate_serde_derive_on_struct() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let tsx_path = fixtures_dir().join("typed_interface.tsx");
    let output_dir = std::env::temp_dir().join("jsxrs_test_codegen_serde");
    fs::create_dir_all(&output_dir)?;
    let output_path = output_dir.join("generated.rs");

    // When
    generate_types(&[tsx_path], &output_path)?;

    // Then
    let generated = fs::read_to_string(&output_path)?;
    assert!(generated.contains("Serialize"));
    assert!(generated.contains("Deserialize"));

    // Cleanup
    let _ = fs::remove_dir_all(&output_dir);
    Ok(())
}

#[test]
fn should_generate_structs_from_multiple_tsx_files() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let tsx_paths = vec![
        fixtures_dir().join("typed_interface.tsx"),
        fixtures_dir().join("typed.tsx"),
    ];
    let output_dir = std::env::temp_dir().join("jsxrs_test_codegen_multi");
    fs::create_dir_all(&output_dir)?;
    let output_path = output_dir.join("generated.rs");

    // When
    let result = generate_types(&tsx_paths, &output_path);

    // Then
    assert!(result.is_ok());
    let generated = fs::read_to_string(&output_path)?;
    assert!(generated.contains("CardProps"));
    assert!(generated.contains("Props"));

    // Cleanup
    let _ = fs::remove_dir_all(&output_dir);
    Ok(())
}

#[test]
fn should_return_error_when_tsx_file_not_found() -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let tsx_path = fixtures_dir().join("nonexistent.tsx");
    let output_dir = std::env::temp_dir().join("jsxrs_test_codegen_missing");
    fs::create_dir_all(&output_dir)?;
    let output_path = output_dir.join("generated.rs");

    // When
    let result = generate_types(&[tsx_path], &output_path);

    // Then
    assert!(result.is_err());

    // Cleanup
    let _ = fs::remove_dir_all(&output_dir);
    Ok(())
}

#[test]
fn should_preserve_legacy_fallback_for_unsupported_interface_properties()
-> Result<(), Box<dyn std::error::Error>> {
    let tsx_path = fixtures_dir().join("codegen_legacy.tsx");
    let output_dir =
        std::env::temp_dir().join(format!("jsxrs_test_codegen_legacy_{}", std::process::id()));
    fs::create_dir_all(&output_dir)?;
    let output_path = output_dir.join("generated.rs");

    let generated = generate_types(&[tsx_path], &output_path)
        .and_then(|()| fs::read_to_string(&output_path).map_err(jsxrs::JsxrsError::from));
    let cleanup = fs::remove_dir_all(&output_dir);

    let generated = generated?;
    cleanup?;
    assert!(generated.contains("pub struct LegacyProps"));
    assert!(generated.contains("pub untyped: serde_json::Value"));
    assert!(generated.contains("pub unsupported: serde_json::Value"));
    assert!(!generated.contains("method"));
    Ok(())
}

#[test]
fn should_reject_duplicate_lowered_rust_field_names() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!(
        "jsxrs_test_codegen_collision_{}",
        std::process::id()
    ));
    fs::create_dir_all(&root)?;
    let tsx_path = root.join("collision.tsx");
    let output_path = root.join("generated.rs");
    fs::write(
        &tsx_path,
        "interface Props { self: string; field_0: string; }\n",
    )?;

    let result = generate_types(&[tsx_path], &output_path);
    assert!(result.is_err());
    assert!(!output_path.exists());

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn should_compile_generated_keyword_field_in_edition_2024() -> Result<(), Box<dyn std::error::Error>>
{
    let root = std::env::temp_dir().join(format!("jsxrs_test_codegen_2024_{}", std::process::id()));
    let source_dir = root.join("src");
    fs::create_dir_all(&source_dir)?;
    let tsx_path = fixtures_dir().join("codegen_gen.tsx");
    let generated_path = source_dir.join("generated.rs");
    generate_types(&[tsx_path], &generated_path)?;
    let generated = fs::read_to_string(&generated_path)?;
    assert!(generated.contains("#[serde(rename = \"gen\")]"));
    assert!(generated.contains("pub r#gen: String"));

    fs::write(source_dir.join("lib.rs"), "include!(\"generated.rs\");\n")?;
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"jsxrs_codegen_2024\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies]\nserde = { version = \"1\", features = [\"derive\"] }\nserde_json = \"1\"\n",
    )?;
    fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock"),
        root.join("Cargo.lock"),
    )?;
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let result = Command::new(cargo)
        .arg("check")
        .arg("--offline")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target"))
        .output();
    let cleanup = fs::remove_dir_all(&root);
    let result = result?;
    cleanup?;
    assert!(
        result.status.success(),
        "edition-2024 consumer failed to compile generated code: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    Ok(())
}
