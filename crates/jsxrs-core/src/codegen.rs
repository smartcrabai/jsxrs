use std::collections::HashSet;
use std::fmt::Write;
use std::path::Path;

use swc_ecma_ast::{
    Decl, ModuleItem, Stmt, TsInterfaceDecl, TsKeywordTypeKind, TsType, TsTypeElement,
};

use crate::{JsxrsError, parse_source};

/// Generate Rust structs from TypeScript interface declarations.
///
/// This preserves the legacy code generator's permissive fallback behavior:
/// unannotated and unsupported property types become `serde_json::Value`, and
/// non-property interface members are omitted.
///
/// # Errors
///
/// Returns an error if a source cannot be read or parsed, or output cannot be written.
pub fn generate_types(
    tsx_paths: &[impl AsRef<Path>],
    output_path: &Path,
) -> Result<(), JsxrsError> {
    let mut output = String::from("use serde::{Serialize, Deserialize};\n\n");
    for path in tsx_paths {
        let path = path.as_ref();
        let source = std::fs::read_to_string(path)
            .map_err(|_| JsxrsError::FileNotFound(path.to_path_buf()))?;
        let file_name = path.to_string_lossy().into_owned();
        let (module, _) = parse_source(&source, &file_name)?;
        for item in &module.body {
            if let Some(interface) = extract_ts_interface(item) {
                write_struct(&mut output, interface)?;
            }
        }
    }
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output_path, output)?;
    Ok(())
}

fn extract_ts_interface(item: &ModuleItem) -> Option<&TsInterfaceDecl> {
    match item {
        ModuleItem::Stmt(Stmt::Decl(Decl::TsInterface(interface))) => Some(interface),
        _ => None,
    }
}

fn write_struct(output: &mut String, interface: &TsInterfaceDecl) -> Result<(), JsxrsError> {
    let name = &interface.id.sym;
    let _ = writeln!(output, "#[derive(Debug, Clone, Serialize, Deserialize)]");
    let _ = writeln!(output, "pub struct {name} {{");
    let mut field_index = 0;
    let mut rust_names = HashSet::new();
    for member in &interface.body.body {
        if let TsTypeElement::TsPropertySignature(property) = member {
            let field_name = match property.key.as_ref() {
                swc_ecma_ast::Expr::Ident(identifier) => identifier.sym.as_ref(),
                _ => continue,
            };
            let base_type = match &property.type_ann {
                Some(annotation) => ts_type_to_rust(&annotation.type_ann),
                None => "serde_json::Value".to_owned(),
            };
            let (rust_name, rust_type) =
                lower_rust_field(&field_name, field_index, &base_type, property.optional);
            if !rust_names.insert(rust_name.clone()) {
                return Err(JsxrsError::Unsupported(format!(
                    "duplicate Rust field `{rust_name}` in interface {name}"
                )));
            }
            field_index += 1;
            let _ = writeln!(output, "    #[serde(rename = {field_name:?})]");
            let _ = writeln!(output, "    pub {rust_name}: {rust_type},");
        }
    }
    let _ = writeln!(output, "}}\n");
    Ok(())
}

/// Lower a TypeScript field name and base Rust type for generated Rust props.
pub fn lower_rust_field(
    name: &str,
    index: usize,
    rust_type: &str,
    optional: bool,
) -> (String, String) {
    const KEYWORDS: &[&str] = &[
        "abstract", "as", "async", "await", "become", "box", "break", "const", "continue",
        "crate", "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for",
        "gen", "if", "impl", "in", "let", "loop", "macro", "match", "mod", "move", "mut",
        "override", "priv", "pub", "ref", "return", "self", "Self", "static", "struct",
        "super", "trait", "true", "try", "type", "typeof", "union", "unsafe", "unsized",
        "use", "virtual", "where", "while", "yield",
    ];
    let rust_name = if is_rust_identifier(name) {
        let is_keyword = KEYWORDS.contains(&name);
        if is_keyword
            && name != "self"
            && name != "Self"
            && name != "super"
            && name != "crate"
        {
            format!("r#{name}")
        } else if is_keyword {
            format!("field_{index}")
        } else {
            name.to_owned()
        }
    } else {
        format!("field_{index}")
    };
    let rust_type = if optional {
        format!("Option<{rust_type}>")
    } else {
        rust_type.to_owned()
    };
    (rust_name, rust_type)
}

fn is_rust_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_alphabetic()) && chars.all(|ch| ch == '_' || ch.is_alphanumeric())
}

fn ts_type_to_rust(ts_type: &TsType) -> String {
    match ts_type {
        TsType::TsKeywordType(keyword) => match keyword.kind {
            TsKeywordTypeKind::TsStringKeyword => "String".to_owned(),
            TsKeywordTypeKind::TsNumberKeyword => "f64".to_owned(),
            TsKeywordTypeKind::TsBooleanKeyword => "bool".to_owned(),
            TsKeywordTypeKind::TsNullKeyword | TsKeywordTypeKind::TsUndefinedKeyword => {
                "Option<()>".to_owned()
            }
            _ => "serde_json::Value".to_owned(),
        },
        TsType::TsArrayType(array) => format!("Vec<{}>", ts_type_to_rust(&array.elem_type)),
        _ => "serde_json::Value".to_owned(),
    }
}
