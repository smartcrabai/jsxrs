use std::collections::{HashMap, HashSet};

use swc_ecma_ast::{
    Decl, DefaultDecl, Expr, FnExpr, Module, ModuleDecl, ModuleItem, Pat, Stmt, TsEntityName,
    TsInterfaceDecl, TsKeywordTypeKind, TsPropertySignature, TsType, TsTypeAliasDecl,
    TsTypeElement, TsUnionOrIntersectionType,
};

use crate::JsxrsError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropsSchema {
    pub type_name: String,
    pub fields: Vec<PropsField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropsField {
    pub name: String,
    pub rust_type: String,
    pub optional: bool,
    pub nullable: bool,
}

enum TypeDecl<'a> {
    Interface(Vec<&'a TsInterfaceDecl>),
    Alias(&'a TsTypeAliasDecl),
}

/// Extract the local closed-object schema named by the selected component's props annotation.
///
/// Returns `None` for an unannotated props parameter.
///
/// # Errors
///
/// Returns an error for unresolved, recursive, or unsupported annotated props types.
pub fn extract_props_schema(module: &Module) -> Result<Option<PropsSchema>, JsxrsError> {
    let Some(annotation) = default_props_annotation(module)? else {
        return Ok(None);
    };
    let declarations = collect_type_declarations(module);
    let (type_name, fields) = match annotation {
        TsType::TsTypeRef(reference) => {
            let name = entity_name(&reference.type_name).ok_or_else(|| {
                JsxrsError::Unsupported("qualified props type references are unsupported".into())
            })?;
            if reference.type_params.is_some() {
                return Err(JsxrsError::Unsupported(format!(
                    "generic props type {name} is unsupported"
                )));
            }
            let mut visiting = HashSet::new();
            let fields = resolve_named_object(name, &declarations, &mut visiting)?;
            (name.to_owned(), fields)
        }
        TsType::TsTypeLit(literal) => (
            "Props".to_owned(),
            fields_from_members(&literal.members, &declarations, &mut HashSet::new())?,
        ),
        _ => {
            return Err(JsxrsError::Unsupported(
                "props annotation must reference a local interface/type or an inline object".into(),
            ));
        }
    };
    Ok(Some(PropsSchema { type_name, fields }))
}

fn default_props_annotation(module: &Module) -> Result<Option<&TsType>, JsxrsError> {
    for item in &module.body {
        match item {
            ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultDecl(export)) => {
                if let DefaultDecl::Fn(function) = &export.decl {
                    return function_annotation(&function.function.params);
                }
            }
            ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(export)) => {
                return annotation_for_default_expr(&export.expr, module, &mut HashSet::new());
            }
            _ => {}
        }
    }
    Err(JsxrsError::NoDefaultExport)
}

fn annotation_for_default_expr<'a>(
    expr: &'a Expr,
    module: &'a Module,
    visited: &mut HashSet<String>,
) -> Result<Option<&'a TsType>, JsxrsError> {
    match expr {
        Expr::Arrow(arrow) => annotation_for_pat_params(&arrow.params),
        Expr::Fn(FnExpr { function, .. }) => function_annotation(&function.params),
        Expr::Ident(name) => {
            if !visited.insert(name.sym.to_string()) {
                return Err(JsxrsError::Unsupported(
                    "recursive default-export aliases are unsupported".into(),
                ));
            }
            for item in &module.body {
                if let ModuleItem::Stmt(Stmt::Decl(Decl::Var(vars))) = item {
                    for declaration in &vars.decls {
                        if let Pat::Ident(binding) = &declaration.name
                            && binding.id.sym == name.sym
                            && let Some(initializer) = &declaration.init
                        {
                            return annotation_for_default_expr(initializer, module, visited);
                        }
                    }
                }
                if let ModuleItem::Stmt(Stmt::Decl(Decl::Fn(function))) = item
                    && function.ident.sym == name.sym
                {
                    return function_annotation(&function.function.params);
                }
            }
            Ok(None)
        }
        _ => Ok(None),
    }
}

fn function_annotation(params: &[swc_ecma_ast::Param]) -> Result<Option<&TsType>, JsxrsError> {
    annotation_for_pat_refs(params.iter().map(|parameter| &parameter.pat))
}

fn annotation_for_pat_params(params: &[Pat]) -> Result<Option<&TsType>, JsxrsError> {
    annotation_for_pat_refs(params.iter())
}

fn annotation_for_pat_refs<'a>(
    mut patterns: impl Iterator<Item = &'a Pat>,
) -> Result<Option<&'a TsType>, JsxrsError> {
    let Some(pattern) = patterns.next() else {
        return Ok(None);
    };
    if patterns.next().is_some() {
        return Err(JsxrsError::Unsupported(
            "component props must use a single parameter".into(),
        ));
    }
    let Pat::Ident(binding) = pattern else {
        return Err(JsxrsError::Unsupported(
            "component props parameter must be a simple identifier".into(),
        ));
    };
    if binding.id.sym != "props" {
        return Err(JsxrsError::Unsupported(
            "component props parameter must be named props".into(),
        ));
    }
    Ok(binding
        .type_ann
        .as_ref()
        .map(|annotation| &*annotation.type_ann))
}

fn collect_type_declarations<'a>(module: &'a Module) -> HashMap<&'a str, TypeDecl<'a>> {
    let mut declarations = HashMap::new();
    for item in &module.body {
        let declaration = match item {
            ModuleItem::Stmt(Stmt::Decl(declaration)) => Some(declaration),
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export)) => Some(&export.decl),
            _ => None,
        };
        match declaration {
            Some(Decl::TsInterface(interface)) => {
                let name = interface.id.sym.as_ref();
                if let Some(TypeDecl::Interface(interfaces)) = declarations.get_mut(name) {
                    interfaces.push(interface);
                } else {
                    declarations.insert(name, TypeDecl::Interface(vec![interface]));
                }
            }
            Some(Decl::TsTypeAlias(alias)) => {
                declarations.insert(alias.id.sym.as_ref(), TypeDecl::Alias(alias));
            }
            _ => {}
        }
    }
    declarations
}

fn resolve_named_object<'a>(
    name: &'a str,
    declarations: &HashMap<&'a str, TypeDecl<'a>>,
    visiting: &mut HashSet<&'a str>,
) -> Result<Vec<PropsField>, JsxrsError> {
    if !visiting.insert(name) {
        return Err(JsxrsError::Unsupported(format!(
            "cyclic props type reference: {name}"
        )));
    }
    let result = match declarations.get(name) {
        Some(TypeDecl::Interface(interfaces)) => {
            let mut fields = Vec::new();
            for interface in interfaces {
                for base in &interface.extends {
                    let Expr::Ident(identifier) = base.expr.as_ref() else {
                        return Err(JsxrsError::Unsupported(
                            "qualified or computed interface inheritance is unsupported".into(),
                        ));
                    };
                    if base.type_args.is_some() {
                        return Err(JsxrsError::Unsupported(
                            "generic interface inheritance is unsupported".into(),
                        ));
                    }
                    let inherited =
                        resolve_named_object(identifier.sym.as_ref(), declarations, visiting)?;
                    merge_fields(&mut fields, inherited)?;
                }
                let own = fields_from_members(&interface.body.body, declarations, visiting)?;
                merge_interface_fields(&mut fields, own)?;
            }
            Ok(fields)
        }
        Some(TypeDecl::Alias(alias)) => match alias.type_ann.as_ref() {
            TsType::TsTypeLit(literal) => {
                fields_from_members(&literal.members, declarations, visiting)
            }
            TsType::TsTypeRef(reference) => {
                let target = entity_name(&reference.type_name).ok_or_else(|| {
                    JsxrsError::Unsupported("qualified props type reference is unsupported".into())
                })?;
                if reference.type_params.is_some() {
                    return Err(JsxrsError::Unsupported(format!(
                        "generic props type {name} is unsupported"
                    )));
                }
                resolve_named_object(target, declarations, visiting)
            }
            _ => Err(JsxrsError::Unsupported(format!(
                "props type alias {name} must resolve to an object"
            ))),
        },
        None => Err(JsxrsError::Unsupported(format!(
            "props type {name} is not declared in this module"
        ))),
    };
    visiting.remove(name);
    result
}

fn merge_fields(
    existing: &mut Vec<PropsField>,
    incoming: Vec<PropsField>,
) -> Result<(), JsxrsError> {
    for field in incoming {
        if existing.iter().any(|current| current.name == field.name) {
            return Err(JsxrsError::Unsupported(format!(
                "duplicate props field {} from interface inheritance",
                field.name
            )));
        }
        existing.push(field);
    }
    Ok(())
}

fn merge_interface_fields(
    existing: &mut Vec<PropsField>,
    incoming: Vec<PropsField>,
) -> Result<(), JsxrsError> {
    for field in incoming {
        if let Some(current) = existing.iter().find(|current| current.name == field.name) {
            if current != &field {
                return Err(JsxrsError::Unsupported(format!(
                    "conflicting duplicate props field {}",
                    field.name
                )));
            }
        } else {
            existing.push(field);
        }
    }
    Ok(())
}

fn fields_from_members<'a>(
    members: &'a [TsTypeElement],
    declarations: &HashMap<&'a str, TypeDecl<'a>>,
    visiting: &mut HashSet<&'a str>,
) -> Result<Vec<PropsField>, JsxrsError> {
    let mut fields = Vec::new();
    for member in members {
        let TsTypeElement::TsPropertySignature(property) = member else {
            return Err(JsxrsError::Unsupported(
                "props object supports property signatures only".into(),
            ));
        };
        let name = property_name(property)?;
        let Some(annotation) = &property.type_ann else {
            return Err(JsxrsError::Unsupported(format!(
                "props field {name} must have an explicit type"
            )));
        };
        let (rust_type, nullable) = rust_type_for(&annotation.type_ann, declarations, visiting)?;
        fields.push(PropsField {
            name,
            rust_type,
            optional: property.optional,
            nullable,
        });
    }
    Ok(fields)
}

fn property_name(property: &TsPropertySignature) -> Result<String, JsxrsError> {
    match property.key.as_ref() {
        Expr::Ident(identifier) => Ok(identifier.sym.to_string()),
        Expr::Lit(swc_ecma_ast::Lit::Str(string)) if !property.computed => {
            Ok(string.value.to_string_lossy().into_owned())
        }
        _ => Err(JsxrsError::Unsupported(
            "props field names must be identifiers or string literals".into(),
        )),
    }
}

fn rust_type_for<'a>(
    ts_type: &'a TsType,
    declarations: &HashMap<&'a str, TypeDecl<'a>>,
    visiting: &mut HashSet<&'a str>,
) -> Result<(String, bool), JsxrsError> {
    match ts_type {
        TsType::TsKeywordType(keyword) => {
            let rust_type = match keyword.kind {
                TsKeywordTypeKind::TsStringKeyword => "String",
                TsKeywordTypeKind::TsNumberKeyword => "f64",
                TsKeywordTypeKind::TsBooleanKeyword => "bool",
                TsKeywordTypeKind::TsAnyKeyword | TsKeywordTypeKind::TsUnknownKeyword => {
                    "serde_json::Value"
                }
                TsKeywordTypeKind::TsNullKeyword | TsKeywordTypeKind::TsUndefinedKeyword => {
                    return Ok(("()".into(), true));
                }
                _ => {
                    return Err(JsxrsError::Unsupported(format!(
                        "unsupported props field type {keyword:?}"
                    )));
                }
            };
            Ok((rust_type.to_owned(), false))
        }
        TsType::TsTypeLit(_) => Err(JsxrsError::Unsupported(
            "nested props object fields are unsupported".into(),
        )),
        TsType::TsArrayType(array) => {
            let (inner_type, nullable) = rust_type_for(&array.elem_type, declarations, visiting)?;
            if nullable {
                return Err(JsxrsError::Unsupported(
                    "nullable array element types are unsupported".into(),
                ));
            }
            Ok((format!("Vec<{inner_type}>"), false))
        }
        TsType::TsParenthesizedType(parenthesized) => {
            rust_type_for(&parenthesized.type_ann, declarations, visiting)
        }
        TsType::TsUnionOrIntersectionType(TsUnionOrIntersectionType::TsUnionType(union)) => {
            if union.types.len() != 2 {
                return Err(JsxrsError::Unsupported(
                    "props unions must contain one value type and null/undefined".into(),
                ));
            }
            let (value_type, nullable_type) = if is_nullish(&union.types[0]) {
                (&union.types[1], &union.types[0])
            } else {
                (&union.types[0], &union.types[1])
            };
            if !is_nullish(nullable_type) {
                return Err(JsxrsError::Unsupported(
                    "only null/undefined nullable unions are supported for props".into(),
                ));
            }
            let (rust_type, _) = rust_type_for(value_type, declarations, visiting)?;
            Ok((rust_type, true))
        }
        TsType::TsTypeRef(reference) => {
            let name = entity_name(&reference.type_name).ok_or_else(|| {
                JsxrsError::Unsupported("qualified props field type is unsupported".into())
            })?;
            if reference.type_params.is_some() {
                return Err(JsxrsError::Unsupported(format!(
                    "generic props field type {name} is unsupported"
                )));
            }
            match name {
                "String" => Ok(("String".into(), false)),
                "Number" => Ok(("f64".into(), false)),
                "Boolean" => Ok(("bool".into(), false)),
                "Array" => Err(JsxrsError::Unsupported(
                    "generic Array props field types are unsupported".into(),
                )),
                _ => {
                    if !visiting.insert(name) {
                        return Err(JsxrsError::Unsupported(format!(
                            "cyclic props field type reference: {name}"
                        )));
                    }
                    let result = match declarations.get(name) {
                        Some(TypeDecl::Alias(alias)) => {
                            rust_type_for(&alias.type_ann, declarations, visiting)
                        }
                        Some(TypeDecl::Interface(_)) => Err(JsxrsError::Unsupported(format!(
                            "props field type {name} is a nested object type"
                        ))),
                        None => Err(JsxrsError::Unsupported(format!(
                            "props field type {name} is not declared in this module"
                        ))),
                    };
                    visiting.remove(&name);
                    result
                }
            }
        }
        _ => Err(JsxrsError::Unsupported(format!(
            "unsupported props field type {ts_type:?}"
        ))),
    }
}

fn is_nullish(ts_type: &TsType) -> bool {
    matches!(
        ts_type,
        TsType::TsKeywordType(keyword)
            if matches!(keyword.kind, TsKeywordTypeKind::TsNullKeyword | TsKeywordTypeKind::TsUndefinedKeyword)
    )
}

fn entity_name(name: &TsEntityName) -> Option<&str> {
    match name {
        TsEntityName::Ident(identifier) => Some(identifier.sym.as_ref()),
        TsEntityName::TsQualifiedName(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::extract_props_schema;
    use crate::{JsxrsError, parse_source};

    fn extract(source: &str) -> Result<Option<super::PropsSchema>, JsxrsError> {
        let (module, _) = parse_source(source, "component.tsx")?;
        extract_props_schema(&module)
    }

    #[test]
    fn accepts_zero_props_parameters() -> Result<(), JsxrsError> {
        assert!(extract("export default function Page() { return <div />; }")?.is_none());
        Ok(())
    }

    #[test]
    fn accepts_one_simple_annotated_props_parameter() -> Result<(), JsxrsError> {
        let schema = extract(
            "interface Props { name: string; }\n\
             export default function Page(props: Props) { return <p>{props.name}</p>; }",
        )?;
        assert!(schema.is_some());
        Ok(())
    }

    #[test]
    fn merges_repeated_interface_declarations() -> Result<(), JsxrsError> {
        let schema = extract(
            "interface Props { first: string; }\n\
             interface Props { second: number; }\n\
             export default function Page(props: Props) { return <p>{props.first}{props.second}</p>; }",
        )?
        .ok_or_else(|| JsxrsError::Unsupported("expected a props schema".into()))?;

        assert_eq!(schema.fields.len(), 2);
        assert_eq!(schema.fields[0].name, "first");
        assert_eq!(schema.fields[1].name, "second");
        Ok(())
    }

    #[test]
    fn rejects_conflicting_fields_in_repeated_interface_declarations() -> Result<(), JsxrsError> {
        let error = extract(
            "interface Props { value: string; }\n\
             interface Props { value: number; }\n\
             export default function Page(props: Props) { return <p>{props.value}</p>; }",
        )
        .err()
        .ok_or_else(|| JsxrsError::Unsupported("expected conflicting props fields to fail".into()))?;

        assert!(matches!(
            error,
            JsxrsError::Unsupported(message) if message.contains("conflicting duplicate props field value")
        ));
        Ok(())
    }

    #[test]
    fn rejects_multiple_props_parameters() -> Result<(), JsxrsError> {
        let error = extract(
            "interface Props { name: string; }\n\
             export default function Page(props: Props, extra: string) { return <p>{props.name}</p>; }",
        )
        .err()
        .ok_or_else(|| JsxrsError::Unsupported("multiple props parameters passed".into()))?;
        assert!(
            matches!(error, JsxrsError::Unsupported(message) if message.contains("single parameter"))
        );
        Ok(())
    }

    #[test]
    fn rejects_unsupported_props_parameter_patterns() -> Result<(), JsxrsError> {
        let error = extract(
            "interface Props { name: string; }\n\
             export default function Page({ name }: Props) { return <p>{name}</p>; }",
        )
        .err()
        .ok_or_else(|| JsxrsError::Unsupported("destructured props parameter passed".into()))?;
        assert!(
            matches!(error, JsxrsError::Unsupported(message) if message.contains("simple identifier"))
        );
        Ok(())
    }
}
