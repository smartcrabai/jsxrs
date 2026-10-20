use std::collections::HashMap;

use swc_ecma_ast::{
    ArrowFunctionBody, Decl, DefaultDecl, Expr, FnExpr, Function, ImportDecl, ImportSpecifier,
    ModuleDecl, ModuleItem, Pat, ReturnStmt, Stmt,
};

use crate::JsxrsError;

#[must_use]
pub fn collect_imports(items: &[ModuleItem]) -> HashMap<String, String> {
    let mut imports = HashMap::new();
    for item in items {
        if let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = item {
            collect_import_specifiers(import, &mut imports);
        }
    }
    imports
}

fn collect_import_specifiers(import: &ImportDecl, imports: &mut HashMap<String, String>) {
    let source = import.src.value.to_string_lossy().into_owned();
    for specifier in &import.specifiers {
        if let ImportSpecifier::Default(default) = specifier {
            imports.insert(default.local.sym.to_string(), source.clone());
        }
    }
}

/// Locate and extract the JSX-producing expression from a default component export.
///
/// # Errors
///
/// Returns an error when no supported default component export or return expression exists.
pub fn find_default_export_jsx(items: &[ModuleItem]) -> Result<Expr, JsxrsError> {
    for item in items {
        match item {
            ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultDecl(export)) => {
                return extract_from_default_decl(&export.decl);
            }
            ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(export)) => {
                return resolve_default_expr(&export.expr, items);
            }
            _ => {}
        }
    }
    Err(JsxrsError::NoDefaultExport)
}

fn extract_from_default_decl(decl: &DefaultDecl) -> Result<Expr, JsxrsError> {
    match decl {
        DefaultDecl::Fn(function) => extract_return_from_function(&function.function),
        _ => Err(JsxrsError::Unsupported(
            "non-function default export".into(),
        )),
    }
}

fn resolve_default_expr(expr: &Expr, items: &[ModuleItem]) -> Result<Expr, JsxrsError> {
    match expr {
        Expr::Arrow(_) | Expr::Fn(_) => return resolve_arrow_or_fn(expr),
        Expr::Ident(identifier) => {
            let name = &*identifier.sym;
            for item in items {
                if let ModuleItem::Stmt(Stmt::Decl(Decl::Var(variables))) = item {
                    for declaration in &variables.decls {
                        if let Pat::Ident(pattern) = &declaration.name
                            && &*pattern.sym == name
                            && let Some(initializer) = &declaration.init
                        {
                            return resolve_arrow_or_fn(initializer);
                        }
                    }
                }
            }
            for item in items {
                if let ModuleItem::Stmt(Stmt::Decl(Decl::Fn(function))) = item
                    && &*function.ident.sym == name
                {
                    return extract_return_from_function(&function.function);
                }
            }
        }
        _ => {}
    }
    Err(JsxrsError::NoDefaultExport)
}

fn resolve_arrow_or_fn(expr: &Expr) -> Result<Expr, JsxrsError> {
    match expr {
        Expr::Arrow(arrow) => match arrow.body.as_ref() {
            ArrowFunctionBody::Expr(expression) => Ok(*expression.clone()),
            ArrowFunctionBody::FunctionBody(body) => find_return_expr(&body.stmts),
        },
        Expr::Fn(FnExpr { function, .. }) => extract_return_from_function(function),
        _ => Err(JsxrsError::NoDefaultExport),
    }
}

fn extract_return_from_function(function: &Function) -> Result<Expr, JsxrsError> {
    let body = function.body.as_ref().ok_or(JsxrsError::NoDefaultExport)?;
    find_return_expr(&body.stmts)
}

fn find_return_expr(statements: &[Stmt]) -> Result<Expr, JsxrsError> {
    for statement in statements {
        if let Stmt::Return(ReturnStmt {
            arg: Some(argument),
            ..
        }) = statement
        {
            return Ok(*argument.clone());
        }
    }
    Err(JsxrsError::NoDefaultExport)
}
