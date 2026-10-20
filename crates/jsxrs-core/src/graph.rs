use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use swc_common::{SourceMap, sync::Lrc};
use swc_ecma_ast::{
    ArrowExpr, AssignExpr, CallExpr, ClassMember, Decl, FnExpr, ImportSpecifier, Module,
    ModuleDecl, ModuleItem, NewExpr, PropName, Stmt, TaggedTpl, UpdateExpr,
};
use swc_ecma_visit::{Visit, VisitWith};

use crate::validate::validate_component_with_source;
use crate::{JsxrsError, PropsSchema, export, extract_props_schema, parse_source, resolver};

/// Resolve and parse the complete local import closure of a component source.
///
/// The returned paths are canonical and include the root file. Value imports are
/// restricted to default component imports; type-only imports are resolved and parsed.
///
/// # Errors
///
/// Returns an error for unsupported import forms, unresolved files, parse failures,
/// missing imported component exports, or import cycles.
pub fn resolve_component_graph(
    root_path: &Path,
    root_module: &Module,
) -> Result<Vec<PathBuf>, JsxrsError> {
    let graph = resolve_component_graph_snapshot(root_path, root_module, None)?;
    Ok(graph.files.into_iter().map(|file| file.path).collect())
}

/// A source file and the local imports resolved from that file during checking.
#[derive(Debug, Clone)]
pub struct ResolvedSource {
    pub path: PathBuf,
    pub source: String,
    pub imports: Vec<ResolvedImport>,
}

/// A local import pinned to the file selected during checking.
#[derive(Debug, Clone)]
pub struct ResolvedImport {
    pub specifier: String,
    pub target: PathBuf,
}

/// Resolve a component graph while retaining checked source bytes and import edges.
///
/// `root_source`, when supplied, is the exact source text used to produce
/// `root_module`. Passing it avoids rereading the root between parsing and graph
/// construction.
///
/// # Errors
///
/// Returns an error for unsupported import forms, unresolved files, parse failures,
/// missing imported component exports, or import cycles.
pub fn resolve_component_graph_snapshot(
    root_path: &Path,
    root_module: &Module,
    root_source: Option<&str>,
) -> Result<ResolvedComponentGraph, JsxrsError> {
    resolve_component_graph_snapshot_inner(root_path, root_module, root_source, None)
}

/// Resolve a component graph while reusing the source map from the root parse.
///
/// `root_source` is the exact source text used to produce `root_module`, and
/// `root_source_map` is the source map produced by that same parse.
///
/// # Errors
///
/// Returns an error for unsupported import forms, unresolved files, parse failures,
/// missing imported component exports, or import cycles.
pub fn resolve_component_graph_snapshot_with_source_map(
    root_path: &Path,
    root_module: &Module,
    root_source: Option<&str>,
    root_source_map: Lrc<SourceMap>,
) -> Result<ResolvedComponentGraph, JsxrsError> {
    resolve_component_graph_snapshot_inner(
        root_path,
        root_module,
        root_source,
        Some(root_source_map),
    )
}

fn resolve_component_graph_snapshot_inner(
    root_path: &Path,
    root_module: &Module,
    root_source: Option<&str>,
    root_source_map: Option<Lrc<SourceMap>>,
) -> Result<ResolvedComponentGraph, JsxrsError> {
    let canonical_root = root_path.canonicalize()?;
    let mut visitor = GraphVisitor {
        visiting: Vec::new(),
        visited: HashSet::new(),
        component_exports: HashSet::new(),
        files: Vec::new(),
    };
    visitor.visit(
        &canonical_root,
        root_module,
        true,
        root_source.map(str::to_owned),
        root_source_map,
    )?;
    let files = visitor.files;
    Ok(ResolvedComponentGraph { files })
}

/// The complete checked source set for a component and its imports.
#[derive(Debug, Clone)]
pub struct ResolvedComponentGraph {
    pub files: Vec<ResolvedSource>,
}

struct GraphVisitor {
    visiting: Vec<PathBuf>,
    visited: HashSet<PathBuf>,
    component_exports: HashSet<PathBuf>,
    files: Vec<ResolvedSource>,
}

struct LocalImport {
    specifier: String,
    component_local: Option<String>,
}

impl GraphVisitor {
    fn visit(
        &mut self,
        path: &Path,
        module: &Module,
        require_component: bool,
        source: Option<String>,
        source_map: Option<Lrc<SourceMap>>,
    ) -> Result<(), JsxrsError> {
        let source = match source {
            Some(source) => source,
            None => std::fs::read_to_string(path)
                .map_err(|_| JsxrsError::FileNotFound(path.to_path_buf()))?,
        };
        let source_map = if let Some(source_map) = source_map {
            source_map
        } else {
            let source_name = path.to_string_lossy().into_owned();
            parse_source(&source, &source_name)?.1
        };
        validate_top_level_items(&module.body, path)?;
        if let Some(position) = self.visiting.iter().position(|entry| entry == path) {
            let mut chain = self.visiting[position..]
                .iter()
                .map(|entry| entry.display().to_string())
                .collect::<Vec<_>>();
            chain.push(path.display().to_string());
            return Err(JsxrsError::Unsupported(format!(
                "component import cycle: {}",
                chain.join(" -> ")
            )));
        }

        let imports = local_imports(&module.body, path)?;
        let mut resolved_imports = Vec::new();
        let mut imported_components = HashMap::<String, Option<PropsSchema>>::new();
        let mut pending_imports = Vec::new();
        for import in imports {
            let is_component = import.component_local.is_some();
            let parent = path.parent().ok_or_else(|| {
                JsxrsError::Unsupported(format!(
                    "importing file has no parent directory: {}",
                    path.display()
                ))
            })?;
            let resolved =
                resolver::resolve_import(&import.specifier, parent).map_err(|error| {
                    JsxrsError::Unsupported(format!(
                        "{}: unable to resolve import {}: {error}",
                        path.display(),
                        import.specifier
                    ))
                })?;
            let resolved = resolved
                .canonicalize()
                .map_err(|_| JsxrsError::FileNotFound(resolved.clone()))?;
            let child_source = std::fs::read_to_string(&resolved)
                .map_err(|_| JsxrsError::FileNotFound(resolved.clone()))?;
            let source_name = resolved.to_string_lossy().into_owned();
            let (child_module, child_source_map) = parse_source(&child_source, &source_name)?;
            if let Some(local_name) = import.component_local {
                export::find_default_export_jsx(&child_module.body)?;
                imported_components.insert(local_name, extract_props_schema(&child_module)?);
            }
            resolved_imports.push(ResolvedImport {
                specifier: import.specifier,
                target: resolved.clone(),
            });
            pending_imports.push((
                resolved,
                child_module,
                is_component,
                child_source,
                child_source_map,
            ));
        }

        if require_component && !self.component_exports.contains(path) {
            export::find_default_export_jsx(&module.body)?;
            validate_component_with_source(module, path, &source_map, &imported_components)
                .map_err(|error| match error {
                    diagnostic @ JsxrsError::ValidationDiagnostic { .. } => diagnostic,
                    error => JsxrsError::Unsupported(format!("{}: {error}", path.display())),
                })?;
            self.component_exports.insert(path.to_path_buf());
        }
        if self.visited.contains(path) {
            return Ok(());
        }

        self.visiting.push(path.to_path_buf());
        self.files.push(ResolvedSource {
            path: path.to_path_buf(),
            source,
            imports: resolved_imports,
        });
        for (resolved, child_module, is_component, source, child_source_map) in pending_imports {
            self.visit(
                &resolved,
                &child_module,
                is_component,
                Some(source),
                Some(child_source_map),
            )?;
        }
        self.visiting.pop();
        self.visited.insert(path.to_path_buf());
        Ok(())
    }
}

fn validate_top_level_items(items: &[ModuleItem], source_path: &Path) -> Result<(), JsxrsError> {
    for item in items {
        match item {
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export)) => {
                validate_variable_initializers(&export.decl, source_path)?;
            }
            ModuleItem::ModuleDecl(_) | ModuleItem::Stmt(Stmt::Empty(_)) => {}
            ModuleItem::Stmt(Stmt::Decl(declaration)) => {
                validate_variable_initializers(declaration, source_path)?;
            }
            ModuleItem::Stmt(Stmt::Expr(expression))
                if matches!(
                    expression.expr.as_ref(),
                    swc_ecma_ast::Expr::Lit(swc_ecma_ast::Lit::Str(_))
                ) => {}
            ModuleItem::Stmt(_) => return Err(executable_top_level_error(source_path)),
        }
    }
    Ok(())
}

fn validate_variable_initializers(
    declaration: &Decl,
    source_path: &Path,
) -> Result<(), JsxrsError> {
    if let Decl::Class(class) = declaration
        && let Some(super_class) = &class.class.super_class
    {
        let mut visitor = ExecutableInitializerVisitor::default();
        super_class.visit_with(&mut visitor);
        if visitor.has_executable_operation {
            return Err(executable_top_level_error(source_path));
        }
    }
    match declaration {
        Decl::Var(variables) => {
            for variable in &variables.decls {
                if let Some(initializer) = &variable.init {
                    let mut visitor = ExecutableInitializerVisitor::default();
                    initializer.visit_with(&mut visitor);
                    if visitor.has_executable_operation {
                        return Err(executable_top_level_error(source_path));
                    }
                }
            }
        }
        Decl::Class(class)
            if class.class.body.iter().any(|member| match member {
                ClassMember::ClassProp(field) => {
                    (field.is_static && field.value.is_some())
                        || computed_key_has_executable_operation(&field.key)
                }
                ClassMember::PrivateProp(field) => field.is_static && field.value.is_some(),
                ClassMember::Method(method) => computed_key_has_executable_operation(&method.key),
                ClassMember::StaticBlock(_) => true,
                _ => false,
            }) =>
        {
            return Err(executable_top_level_error(source_path));
        }
        _ => {}
    }
    Ok(())
}

fn computed_key_has_executable_operation(key: &PropName) -> bool {
    let PropName::Computed(key) = key else {
        return false;
    };
    let mut visitor = ExecutableInitializerVisitor::default();
    key.expr.visit_with(&mut visitor);
    visitor.has_executable_operation
}

fn executable_top_level_error(source_path: &Path) -> JsxrsError {
    JsxrsError::Unsupported(format!(
        "{}: executable top-level statements are unsupported",
        source_path.display()
    ))
}

#[derive(Default)]
struct ExecutableInitializerVisitor {
    has_executable_operation: bool,
}

impl Visit for ExecutableInitializerVisitor {
    fn visit_call_expr(&mut self, _: &CallExpr) {
        self.has_executable_operation = true;
    }

    fn visit_new_expr(&mut self, _: &NewExpr) {
        self.has_executable_operation = true;
    }

    fn visit_assign_expr(&mut self, _: &AssignExpr) {
        self.has_executable_operation = true;
    }

    fn visit_update_expr(&mut self, _: &UpdateExpr) {
        self.has_executable_operation = true;
    }

    fn visit_tagged_tpl(&mut self, _: &TaggedTpl) {
        self.has_executable_operation = true;
    }

    fn visit_arrow_expr(&mut self, _: &ArrowExpr) {}

    fn visit_fn_expr(&mut self, _: &FnExpr) {}
}

fn local_imports(items: &[ModuleItem], source_path: &Path) -> Result<Vec<LocalImport>, JsxrsError> {
    let mut imports = Vec::new();
    for item in items {
        let ModuleItem::ModuleDecl(declaration) = item else {
            continue;
        };
        match declaration {
            ModuleDecl::Import(import) => {
                if import.specifiers.is_empty() {
                    return Err(JsxrsError::Unsupported(format!(
                        "{}: side-effect imports are unsupported",
                        source_path.display()
                    )));
                }
                let mut component_local = None;
                for specifier in &import.specifiers {
                    match specifier {
                        ImportSpecifier::Default(default) if !import.type_only => {
                            component_local = Some(default.local.sym.to_string());
                        }
                        ImportSpecifier::Default(_) => {}
                        specifier if import.type_only || specifier.is_type_only() => {}
                        swc_ecma_ast::ImportSpecifier::Named(_) => {
                            return Err(JsxrsError::Unsupported(format!(
                                "{}: named value imports are unsupported",
                                source_path.display()
                            )));
                        }
                        swc_ecma_ast::ImportSpecifier::Namespace(_) => {
                            return Err(JsxrsError::Unsupported(format!(
                                "{}: namespace value imports are unsupported",
                                source_path.display()
                            )));
                        }
                    }
                }
                imports.push(LocalImport {
                    specifier: import.src.value.to_string_lossy().into_owned(),
                    component_local,
                });
            }
            ModuleDecl::ExportNamed(export) if export.src.is_some() => {
                if !export.type_only {
                    return Err(JsxrsError::Unsupported(format!(
                        "{}: value re-exports are unsupported",
                        source_path.display()
                    )));
                }
                if let Some(source) = &export.src {
                    imports.push(LocalImport {
                        specifier: source.value.to_string_lossy().into_owned(),
                        component_local: None,
                    });
                }
            }
            ModuleDecl::ExportAll(export) => {
                if !export.type_only {
                    return Err(JsxrsError::Unsupported(format!(
                        "{}: value re-exports are unsupported",
                        source_path.display()
                    )));
                }
                imports.push(LocalImport {
                    specifier: export.src.value.to_string_lossy().into_owned(),
                    component_local: None,
                });
            }
            _ => {}
        }
    }
    Ok(imports)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::error::Error;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::resolve_component_graph_snapshot;
    use crate::{JsxrsError, parse_source};

    fn temporary_directory(label: &str) -> Result<PathBuf, Box<dyn Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "jsxrs_graph_{label}_{}_{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&directory)?;
        Ok(directory)
    }

    #[test]
    fn diamond_import_graph_contains_each_source_once() -> Result<(), Box<dyn Error>> {
        let directory = temporary_directory("diamond")?;
        let root_path = directory.join("page.tsx");
        let root_source = "import Left from './left';\nimport Right from './right';\nexport default function Page() { return <main><Left /><Right /></main>; }\n";
        fs::write(&root_path, root_source)?;
        fs::write(
            directory.join("left.tsx"),
            "import Shared from './shared';\nexport default function Left() { return <Shared />; }\n",
        )?;
        fs::write(
            directory.join("right.tsx"),
            "import Shared from './shared';\nexport default function Right() { return <Shared />; }\n",
        )?;
        fs::write(
            directory.join("shared.tsx"),
            "export default function Shared() { return <span />; }\n",
        )?;

        let (module, _) = parse_source(root_source, "page.tsx")?;
        let graph = resolve_component_graph_snapshot(&root_path, &module, Some(root_source))?;
        let unique_paths = graph
            .files
            .iter()
            .map(|file| file.path.clone())
            .collect::<HashSet<_>>();
        assert_eq!(graph.files.len(), 4);
        assert_eq!(unique_paths.len(), graph.files.len());
        assert_eq!(
            graph
                .files
                .iter()
                .map(|file| {
                    file.path
                        .file_name()
                        .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
                })
                .collect::<Vec<_>>(),
            [
                "page.tsx".to_owned(),
                "left.tsx".to_owned(),
                "shared.tsx".to_owned(),
                "right.tsx".to_owned(),
            ],
        );
        fs::remove_dir_all(directory)?;
        Ok(())
    }

    #[test]
    fn import_cycle_is_rejected() -> Result<(), Box<dyn Error>> {
        let directory = temporary_directory("cycle")?;
        let root_path = directory.join("first.tsx");
        let root_source = "import Second from './second';\nexport default function First() { return <Second />; }\n";
        fs::write(&root_path, root_source)?;
        fs::write(
            directory.join("second.tsx"),
            "import First from './first';\nexport default function Second() { return <First />; }\n",
        )?;

        let (module, _) = parse_source(root_source, "first.tsx")?;
        let error = resolve_component_graph_snapshot(&root_path, &module, Some(root_source))
            .err()
            .ok_or_else(|| std::io::Error::other("expected the component import cycle to fail"))?;
        assert!(matches!(
            error,
            JsxrsError::Unsupported(message) if message.contains("component import cycle")
        ));
        fs::remove_dir_all(directory)?;
        Ok(())
    }
}
