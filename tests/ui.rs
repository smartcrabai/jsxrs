use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn public_macro_ui_fixtures_match_their_contracts() -> Result<(), Box<dyn Error>> {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let root = std::env::temp_dir().join(format!(
        "jsxrs_ui_{}_{}",
        std::process::id(),
        nanos
    ));
    let consumer = root.join("consumer");
    fs::create_dir_all(consumer.join("src"))?;
    copy_dir_all(&repository.join("tests/fixtures/ui"), &consumer.join("tests/fixtures/ui"))?;

    let jsxrs_path = repository.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"");
    fs::write(
        consumer.join("Cargo.toml"),
        format!(
            "[package]\nname = \"jsxrs_ui_consumer\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies]\njsxrs = {{ path = \"{jsxrs_path}\", features = [\"axum\"] }}\n"
        ),
    )?;

    let target = root.join("target");
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    for (directory, should_pass) in [("pass", true), ("fail", false)] {
        let fixtures = repository.join("tests/ui").join(directory);
        let mut entries = fs::read_dir(fixtures)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            if path.extension().is_none_or(|extension| extension != "rs") {
                continue;
            }
            fs::copy(&path, consumer.join("src/main.rs"))?;
            let result = Command::new(&cargo)
                .arg("check")
                .arg("--offline")
                .arg("--quiet")
                .arg("--manifest-path")
                .arg(consumer.join("Cargo.toml"))
                .env("CARGO_TARGET_DIR", &target)
                .current_dir(&consumer)
                .output()?;
            let stderr = String::from_utf8_lossy(&result.stderr);
            let case = path.file_stem().map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
            if should_pass {
                assert!(
                    result.status.success(),
                    "public-macro pass fixture `{case}` failed: {stderr}"
                );
            } else {
                assert!(
                    !result.status.success(),
                    "public-macro fail fixture `{case}` unexpectedly compiled"
                );
                let expected = expected_diagnostic(&case).ok_or_else(|| {
                    std::io::Error::other(format!("missing expected diagnostic for `{case}`"))
                })?;
                assert!(
                    stderr.contains(expected),
                    "public-macro fail fixture `{case}` did not report `{expected}`: {stderr}"
                );
            }
        }
    }

    fs::remove_dir_all(root)?;
    Ok(())
}

fn expected_diagnostic(case: &str) -> Option<&'static str> {
    match case {
        "event_constructor_prop" | "event_function_prop" | "event_alias_prop" => Some("undefined prop"),
        "child_unknown_prop" => Some("undefined prop"),
        "child_missing_required" => Some("missing required prop"),
        "child_text_children" | "child_jsx_children" => {
            Some("children on imported component")
        }
        "route_page_missing_prop" => Some("page requires prop `id`"),
        "route_layout_missing_prop" => Some("layout requires prop `locale`"),
        "route_page_optional_access" => Some("page accesses prop `locale`"),
        "route_layout_optional_access" => Some("layout accesses prop `locale`"),
        "route_alias_cycle" => Some("recursive default-export aliases are unsupported"),
        "executable_top_level"
        | "top_level_initializer"
        | "class_static_initializer"
        | "class_super_initializer"
        | "tagged_template_initializer"
        | "computed_class_method_key" => {
            Some("executable top-level statements are unsupported")
        }
        "field_identifier_collision" => Some("duplicate Rust field"),
        "unknown_root_prop" => Some("unknown_root_prop.tsx:2:"),
        "imported_prop_error" => Some("diagnostic_child.tsx:2:"),
        _ => None,
    }
}

fn copy_dir_all(source: &Path, destination: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        let target = destination.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &target)?;
        } else {
            fs::copy(path, target)?;
        }
    }
    Ok(())
}
