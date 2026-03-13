mod common;

use std::fs;

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
fn should_generate_rust_struct_with_correct_field_types() -> Result<(), Box<dyn std::error::Error>> {
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
