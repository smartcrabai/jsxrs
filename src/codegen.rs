use std::fmt::Write;
use std::path::Path;

use swc_ecma_ast::{
    Decl, ModuleItem, Stmt, TsInterfaceDecl, TsKeywordTypeKind, TsType, TsTypeElement,
};

use crate::error::JsxrsError;
use crate::parser;

/// Generate Rust struct definitions from TypeScript interface declarations in TSX files.
pub fn generate_types(
    tsx_paths: &[impl AsRef<Path>],
    output_path: &Path,
) -> Result<(), JsxrsError> {
    let mut output = String::new();
    output.push_str("use serde::{Serialize, Deserialize};\n\n");

    for path in tsx_paths {
        let path = path.as_ref();
        let source = std::fs::read_to_string(path)
            .map_err(|_| JsxrsError::FileNotFound(path.to_path_buf()))?;
        let file_name = path.to_string_lossy().to_string();
        let (module, _) = parser::parse_source(&source, &file_name)?;

        for item in &module.body {
            if let Some(iface) = extract_ts_interface(item) {
                write_struct(&mut output, iface)?;
            }
        }
    }

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output_path, &output)?;
    Ok(())
}

fn extract_ts_interface(item: &ModuleItem) -> Option<&TsInterfaceDecl> {
    match item {
        ModuleItem::Stmt(Stmt::Decl(Decl::TsInterface(iface))) => Some(iface),
        _ => None,
    }
}

fn write_struct(output: &mut String, iface: &TsInterfaceDecl) -> Result<(), JsxrsError> {
    let name = &iface.id.sym;
    writeln!(output, "#[derive(Debug, Clone, Serialize, Deserialize)]")
        .map_err(|e| JsxrsError::Unsupported(e.to_string()))?;
    writeln!(output, "pub struct {name} {{").map_err(|e| JsxrsError::Unsupported(e.to_string()))?;

    for member in &iface.body.body {
        if let TsTypeElement::TsPropertySignature(prop) = member {
            let field_name = match prop.key.as_ref() {
                swc_ecma_ast::Expr::Ident(id) => id.sym.to_string(),
                _ => continue,
            };
            let rust_type = match &prop.type_ann {
                Some(ann) => ts_type_to_rust(&ann.type_ann),
                None => "serde_json::Value".to_string(),
            };
            let rust_type = if prop.optional {
                format!("Option<{rust_type}>")
            } else {
                rust_type
            };
            writeln!(output, "    pub {field_name}: {rust_type},")
                .map_err(|e| JsxrsError::Unsupported(e.to_string()))?;
        }
    }

    writeln!(output, "}}\n").map_err(|e| JsxrsError::Unsupported(e.to_string()))?;
    Ok(())
}

fn ts_type_to_rust(ts_type: &TsType) -> String {
    match ts_type {
        TsType::TsKeywordType(kw) => match kw.kind {
            TsKeywordTypeKind::TsStringKeyword => "String".to_string(),
            TsKeywordTypeKind::TsNumberKeyword => "f64".to_string(),
            TsKeywordTypeKind::TsBooleanKeyword => "bool".to_string(),
            TsKeywordTypeKind::TsAnyKeyword => "serde_json::Value".to_string(),
            TsKeywordTypeKind::TsNullKeyword => "Option<()>".to_string(),
            TsKeywordTypeKind::TsUndefinedKeyword => "Option<()>".to_string(),
            _ => "serde_json::Value".to_string(),
        },
        TsType::TsArrayType(arr) => {
            let inner = ts_type_to_rust(&arr.elem_type);
            format!("Vec<{inner}>")
        }
        _ => "serde_json::Value".to_string(),
    }
}
