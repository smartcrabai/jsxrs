use std::collections::HashMap;

use swc_ecma_ast::{
    BlockStmtOrExpr, Decl, DefaultDecl, Expr, FnDecl, FnExpr, Function, ImportDecl,
    ImportSpecifier, ModuleDecl, ModuleItem, Pat, ReturnStmt, Stmt, VarDeclarator,
};

use crate::error::JsxrsError;

pub(crate) fn collect_imports(items: &[ModuleItem]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for item in items {
        if let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = item {
            collect_import_specifiers(import, &mut map);
        }
    }
    map
}

fn collect_import_specifiers(import: &ImportDecl, map: &mut HashMap<String, String>) {
    let src = import.src.value.to_string_lossy().into_owned();
    for spec in &import.specifiers {
        if let ImportSpecifier::Default(def) = spec {
            map.insert(def.local.sym.to_string(), src.clone());
        }
    }
}

pub(crate) fn find_default_export_jsx(items: &[ModuleItem]) -> Result<Expr, JsxrsError> {
    let mut var_decls: Vec<&VarDeclarator> = Vec::new();
    let mut fn_decls: Vec<&FnDecl> = Vec::new();

    for item in items {
        match item {
            ModuleItem::Stmt(Stmt::Decl(Decl::Var(v))) => {
                for decl in &v.decls {
                    var_decls.push(decl);
                }
            }
            ModuleItem::Stmt(Stmt::Decl(Decl::Fn(f))) => fn_decls.push(f),
            _ => {}
        }
    }

    for item in items {
        match item {
            ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultDecl(e)) => {
                return extract_from_default_decl(&e.decl);
            }
            ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(e)) => {
                return resolve_default_expr(&e.expr, &var_decls, &fn_decls);
            }
            _ => {}
        }
    }

    Err(JsxrsError::NoDefaultExport)
}

fn extract_from_default_decl(decl: &DefaultDecl) -> Result<Expr, JsxrsError> {
    match decl {
        DefaultDecl::Fn(f) => extract_return_from_function(&f.function),
        _ => Err(JsxrsError::Unsupported(
            "non-function default export".into(),
        )),
    }
}

fn resolve_default_expr(
    expr: &Expr,
    vars: &[&VarDeclarator],
    fns: &[&FnDecl],
) -> Result<Expr, JsxrsError> {
    match expr {
        Expr::Arrow(_) | Expr::Fn(_) => return resolve_arrow_or_fn(expr),
        Expr::Ident(id) => {
            let name = &*id.sym;
            for decl in vars {
                if let Pat::Ident(pat) = &decl.name
                    && &*pat.sym == name
                    && let Some(init) = &decl.init
                {
                    return resolve_arrow_or_fn(init);
                }
            }
            for decl in fns {
                if &*decl.ident.sym == name {
                    return extract_return_from_function(&decl.function);
                }
            }
        }
        _ => {}
    }
    Err(JsxrsError::NoDefaultExport)
}

fn resolve_arrow_or_fn(expr: &Expr) -> Result<Expr, JsxrsError> {
    match expr {
        Expr::Arrow(a) => match a.body.as_ref() {
            BlockStmtOrExpr::Expr(e) => Ok(*e.clone()),
            BlockStmtOrExpr::BlockStmt(b) => find_return_expr(&b.stmts),
        },
        Expr::Fn(FnExpr { function, .. }) => extract_return_from_function(function),
        _ => Err(JsxrsError::NoDefaultExport),
    }
}

fn extract_return_from_function(func: &Function) -> Result<Expr, JsxrsError> {
    let body = func.body.as_ref().ok_or(JsxrsError::NoDefaultExport)?;
    find_return_expr(&body.stmts)
}

fn find_return_expr(stmts: &[Stmt]) -> Result<Expr, JsxrsError> {
    for stmt in stmts {
        if let Stmt::Return(ReturnStmt { arg: Some(arg), .. }) = stmt {
            return Ok(*arg.clone());
        }
    }
    Err(JsxrsError::NoDefaultExport)
}
