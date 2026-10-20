use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use swc_ecma_ast::{
    ArrayLit, ArrowExpr, ArrowFunctionBody, BinaryOp, Callee, Decl, DefaultDecl, Expr, FnExpr,
    Function, ImportSpecifier, JSXAttrName, JSXAttrOrSpread, JSXAttrValue, JSXElement,
    JSXElementChild, JSXElementName, JSXExpr, Lit, MemberProp, Module, ModuleDecl, ModuleItem, Pat,
    Prop, PropName, PropOrSpread, Stmt, UnaryOp,
};
use swc_common::{BytePos, SourceMap};

use crate::{JsxrsError, PropsSchema, extract_props_schema};

#[derive(Clone, Copy)]
enum RenderContext {
    Root,
    Child,
    Value,
}

enum ComponentFunction<'a> {
    Arrow(&'a ArrowExpr),
    Function(&'a Function),
}

pub(crate) type ImportedComponents = HashMap<String, Option<PropsSchema>>;

#[derive(Clone)]
struct EventLocal {
    name: String,
    is_props_alias: bool,
}

/// Statically validate a default component using the checked macro policy.
///
/// Every syntactic branch is visited, independently of runtime short-circuiting.
///
/// # Errors
///
/// Returns an error for unsupported function envelopes, expressions, JSX forms,
/// or accesses to fields outside an annotated local props schema.
pub fn validate_component(module: &Module) -> Result<(), JsxrsError> {
    validate_component_accesses(module).map(|_| ())
}

/// Validate a component and return the typed props fields it accesses.
///
/// # Errors
///
/// Returns the same validation errors as [`validate_component`].
pub fn validate_component_accesses(module: &Module) -> Result<HashSet<String>, JsxrsError> {
    validate_component_inner(module, &ImportedComponents::new()).map_err(|error| match error {
        JsxrsError::UndefinedPropAt { key, .. } => JsxrsError::UndefinedProp(key),
        error => error,
    })
}

pub(crate) fn validate_component_with_source(
    module: &Module,
    source_path: &std::path::Path,
    source_map: &SourceMap,
    imported_components: &ImportedComponents,
) -> Result<(), JsxrsError> {
    validate_component_inner(module, imported_components)
        .map(|_| ())
        .map_err(|error| match error {
        JsxrsError::UndefinedPropAt { key, position } => {
            let location = source_map.lookup_char_pos(BytePos(position));
            JsxrsError::ValidationDiagnostic {
                path: source_path.to_path_buf(),
                line: location.line,
                column: location.col_display + 1,
                message: format!("undefined prop `{key}`"),
            }
        }
        error => error,
    })
}

fn validate_component_inner(
    module: &Module,
    imported_components: &ImportedComponents,
) -> Result<HashSet<String>, JsxrsError> {
    let component = find_component(module)?;
    let schema = extract_props_schema(module)?;
    let fields = schema.as_ref().map(TypedProps::new);

    match component {
        ComponentFunction::Arrow(arrow) => {
            if arrow.is_async || arrow.is_generator || arrow.type_params.is_some() {
                return Err(unsupported("async, generator, or generic components"));
            }
            validate_params(&arrow.params)?;
            match arrow.body.as_ref() {
                ArrowFunctionBody::Expr(expr) => {
                    validate_expr(expr, RenderContext::Root, fields.as_ref(), &[], module, imported_components)?;
                }
                ArrowFunctionBody::FunctionBody(body) => {
                    validate_statements(&body.stmts, fields.as_ref(), module, imported_components)?;
                }
            }
        }
        ComponentFunction::Function(function) => {
            if function.is_async
                || function.is_generator
                || function.this_param.is_some()
                || function.type_params.is_some()
            {
                return Err(unsupported(
                    "async, generator, generic, or explicit-this components",
                ));
            }
            validate_param_refs(function.params.iter().map(|parameter| &parameter.pat))?;
            let body = function.body.as_ref().ok_or(JsxrsError::NoDefaultExport)?;
            validate_statements(&body.stmts, fields.as_ref(), module, imported_components)?;
        }
    }
    Ok(fields.map_or_else(HashSet::new, |fields| fields.accessed.into_inner()))
}

struct TypedProps<'a> {
    fields: HashSet<&'a str>,
    accessed: RefCell<HashSet<String>>,
}

impl<'a> TypedProps<'a> {
    fn new(schema: &'a PropsSchema) -> Self {
        Self {
            fields: schema.fields.iter().map(|field| field.name.as_str()).collect(),
            accessed: RefCell::new(HashSet::new()),
        }
    }
}

fn find_component(module: &Module) -> Result<ComponentFunction<'_>, JsxrsError> {
    for item in &module.body {
        match item {
            ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultDecl(export)) => {
                if let DefaultDecl::Fn(function) = &export.decl {
                    return Ok(ComponentFunction::Function(&function.function));
                }
            }
            ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(export)) => {
                if let Some(component) = resolve_function(&export.expr, module, 0) {
                    return Ok(component);
                }
            }
            _ => {}
        }
    }
    Err(JsxrsError::NoDefaultExport)
}

fn resolve_function<'a>(
    expression: &'a Expr,
    module: &'a Module,
    depth: usize,
) -> Option<ComponentFunction<'a>> {
    if depth > module.body.len() {
        return None;
    }
    match expression {
        Expr::Arrow(arrow) => Some(ComponentFunction::Arrow(arrow)),
        Expr::Fn(FnExpr { function, .. }) => Some(ComponentFunction::Function(function)),
        Expr::Ident(identifier) => {
            for item in &module.body {
                match item {
                    ModuleItem::Stmt(Stmt::Decl(Decl::Var(variables))) => {
                        for declaration in &variables.decls {
                            if let Pat::Ident(binding) = &declaration.name
                                && binding.id.sym == identifier.sym
                                && let Some(initializer) = &declaration.init
                                && let Some(function) =
                                    resolve_function(initializer, module, depth + 1)
                            {
                                return Some(function);
                            }
                        }
                    }
                    ModuleItem::Stmt(Stmt::Decl(Decl::Fn(function)))
                        if function.ident.sym == identifier.sym =>
                    {
                        return Some(ComponentFunction::Function(&function.function));
                    }
                    _ => {}
                }
            }
            None
        }
        _ => None,
    }
}

fn validate_params(params: &[Pat]) -> Result<(), JsxrsError> {
    validate_param_refs(params.iter())
}

fn validate_param_refs<'a>(mut params: impl Iterator<Item = &'a Pat>) -> Result<(), JsxrsError> {
    let Some(parameter) = params.next() else {
        return Ok(());
    };
    if params.next().is_some() || !matches!(parameter, Pat::Ident(binding) if binding.id.sym == "props") {
        return Err(unsupported(
            "component parameters (expected zero parameters or one simple `props` parameter)",
        ));
    }
    Ok(())
}

fn validate_statements(
    statements: &[Stmt],
    fields: Option<&TypedProps<'_>>,
    module: &Module,
    imported_components: &ImportedComponents,
) -> Result<(), JsxrsError> {
    let mut return_expression = None;
    for statement in statements {
        match statement {
            Stmt::Empty(_) => {}
            Stmt::Expr(expression)
                if matches!(expression.expr.as_ref(), Expr::Lit(Lit::Str(_))) => {}
            Stmt::Return(statement) if return_expression.is_none() => {
                return_expression = statement.arg.as_deref();
            }
            Stmt::Return(_) => return Err(unsupported("multiple component return statements")),
            _ => {
                return Err(unsupported(
                    "component body statements other than one return",
                ));
            }
        }
    }
    let expression = return_expression.ok_or(JsxrsError::NoDefaultExport)?;
    validate_expr(expression, RenderContext::Root, fields, &[], module, imported_components)
}

fn validate_expr(
    expression: &Expr,
    context: RenderContext,
    fields: Option<&TypedProps<'_>>,
    locals: &[String],
    module: &Module,
    imported_components: &ImportedComponents,
) -> Result<(), JsxrsError> {
    match context {
        RenderContext::Root => match expression {
            Expr::JSXElement(element) => {
                validate_element(element, fields, locals, module, imported_components)
            }
            Expr::JSXFragment(fragment) => {
                validate_children(&fragment.children, fields, locals, module, imported_components)
            }
            Expr::Paren(paren) => {
                validate_expr(&paren.expr, RenderContext::Root, fields, locals, module, imported_components)
            }
            _ => validate_expr(expression, RenderContext::Value, fields, locals, module, imported_components),
        },
        RenderContext::Child => match expression {
            Expr::JSXElement(element) => {
                validate_element(element, fields, locals, module, imported_components)
            }
            Expr::JSXFragment(fragment) => {
                validate_children(&fragment.children, fields, locals, module, imported_components)
            }
            Expr::Paren(paren) => {
                validate_expr(&paren.expr, RenderContext::Child, fields, locals, module, imported_components)
            }
            Expr::Cond(conditional) => {
                validate_expr(
                    &conditional.test,
                    RenderContext::Value,
                    fields,
                    locals,
                    module,
                    imported_components,
                )?;
                validate_expr(
                    &conditional.cons,
                    RenderContext::Child,
                    fields,
                    locals,
                    module,
                    imported_components,
                )?;
                validate_expr(
                    &conditional.alt,
                    RenderContext::Child,
                    fields,
                    locals,
                    module,
                    imported_components,
                )
            }
            Expr::Bin(binary)
                if matches!(binary.op, BinaryOp::LogicalAnd | BinaryOp::LogicalOr) =>
            {
                validate_expr(&binary.left, RenderContext::Value, fields, locals, module, imported_components)?;
                validate_expr(&binary.right, RenderContext::Child, fields, locals, module, imported_components)
            }
            Expr::Call(call) => validate_map(call, fields, locals, module, imported_components),
            _ => validate_expr(expression, RenderContext::Value, fields, locals, module, imported_components),
        },
        RenderContext::Value => validate_value_expr(expression, fields, locals, module, imported_components),
    }
}

fn validate_value_expr(
    expression: &Expr,
    fields: Option<&TypedProps<'_>>,
    locals: &[String],
    module: &Module,
    imported_components: &ImportedComponents,
) -> Result<(), JsxrsError> {
    match expression {
        Expr::Lit(Lit::Str(_) | Lit::Num(_) | Lit::Bool(_) | Lit::Null(_)) => Ok(()),
        Expr::Ident(identifier)
            if identifier.sym == "props" || locals.iter().any(|name| name == &*identifier.sym) =>
        {
            Ok(())
        }
        Expr::Ident(identifier) => Err(unsupported(&format!(
            "unbound identifier `{}`",
            identifier.sym
        ))),
        Expr::Member(member) => {
            validate_props_member(member, fields, locals)?;
            validate_expr(&member.obj, RenderContext::Value, fields, locals, module, imported_components)?;
            if let MemberProp::Computed(computed) = &member.prop {
                validate_expr(&computed.expr, RenderContext::Value, fields, locals, module, imported_components)?;
            } else if matches!(member.prop, MemberProp::PrivateName(_)) {
                return Err(unsupported("private member access"));
            }
            Ok(())
        }
        Expr::Bin(binary) if supported_binary(binary.op) => {
            validate_expr(&binary.left, RenderContext::Value, fields, locals, module, imported_components)?;
            validate_expr(&binary.right, RenderContext::Value, fields, locals, module, imported_components)
        }
        Expr::Cond(conditional) => {
            validate_expr(
                &conditional.test,
                RenderContext::Value,
                fields,
                locals,
                module,
                imported_components,
            )?;
            validate_expr(
                &conditional.cons,
                RenderContext::Value,
                fields,
                locals,
                module,
                imported_components,
            )?;
            validate_expr(
                &conditional.alt,
                RenderContext::Value,
                fields,
                locals,
                module,
                imported_components,
            )
        }
        Expr::Unary(unary)
            if matches!(unary.op, UnaryOp::Bang | UnaryOp::Minus | UnaryOp::Plus) =>
        {
            validate_expr(&unary.arg, RenderContext::Value, fields, locals, module, imported_components)
        }
        Expr::Tpl(template) => {
            for expression in &template.exprs {
                validate_expr(expression, RenderContext::Value, fields, locals, module, imported_components)?;
            }
            Ok(())
        }
        Expr::Paren(paren) => {
            validate_expr(&paren.expr, RenderContext::Value, fields, locals, module, imported_components)
        }
        _ => Err(unsupported(&format!("expression `{expression:?}`"))),
    }
}

fn validate_props_member(
    member: &swc_ecma_ast::MemberExpr,
    fields: Option<&TypedProps<'_>>,
    locals: &[String],
) -> Result<(), JsxrsError> {
    if locals.iter().any(|name| name == "props") {
        return Ok(());
    }
    let Some(fields) = fields else {
        return Ok(());
    };
    let Expr::Ident(identifier) = member.obj.as_ref().unwrap_parens() else {
        return Ok(());
    };
    if identifier.sym != "props" {
        return Ok(());
    }
    validate_props_member_key(member, fields)
}

fn validate_event_props_member(
    member: &swc_ecma_ast::MemberExpr,
    fields: &TypedProps<'_>,
    locals: &[EventLocal],
) -> Result<(), JsxrsError> {
    let Expr::Ident(identifier) = member.obj.as_ref().unwrap_parens() else {
        return Ok(());
    };
    let resolves_to_props = locals
        .iter()
        .rev()
        .find(|local| local.name == identifier.sym.as_ref())
        .map_or(identifier.sym == "props", |local| local.is_props_alias);
    if resolves_to_props {
        validate_props_member_key(member, fields)
    } else {
        Ok(())
    }
}

fn validate_props_member_key(
    member: &swc_ecma_ast::MemberExpr,
    fields: &TypedProps<'_>,
) -> Result<(), JsxrsError> {
    let key = match &member.prop {
        MemberProp::Ident(identifier) => Cow::Borrowed(identifier.sym.as_ref()),
        MemberProp::Computed(computed) => match computed.expr.as_ref().unwrap_parens() {
            Expr::Lit(Lit::Str(string)) => string.value.to_string_lossy(),
            _ => return Err(unsupported("dynamic key access on typed props")),
        },
        MemberProp::PrivateName(_) => return Err(unsupported("private props access")),
    };
    if fields.fields.contains(key.as_ref()) {
        fields.accessed.borrow_mut().insert(key.into_owned());
        Ok(())
    } else {
        Err(JsxrsError::UndefinedPropAt {
            key: key.into_owned(),
            position: member.span.lo.0,
        })
    }
}

fn validate_map(
    call: &swc_ecma_ast::CallExpr,
    fields: Option<&TypedProps<'_>>,
    locals: &[String],
    module: &Module,
    imported_components: &ImportedComponents,
) -> Result<(), JsxrsError> {
    let Callee::Expr(callee) = &call.callee else {
        return Err(unsupported("non-member call expression"));
    };
    let Expr::Member(member) = callee.as_ref().unwrap_parens() else {
        return Err(unsupported("call expression"));
    };
    if !matches!(&member.prop, MemberProp::Ident(identifier) if identifier.sym == "map") {
        return Err(unsupported("call other than array `.map`"));
    }
    if call.args.len() != 1 || call.args[0].spread.is_some() {
        return Err(unsupported("map callback arguments"));
    }
    validate_expr(
        &member.obj,
        RenderContext::Value,
        fields,
        locals,
        module,
        imported_components,
    )?;
    let mut callback = call.args[0].expr.as_ref();
    while let Expr::Paren(paren) = callback {
        callback = &paren.expr;
    }
    let Expr::Arrow(arrow) = callback else {
        return Err(unsupported("map callback (expected an arrow function)"));
    };
    if arrow.is_async
        || arrow.is_generator
        || arrow.type_params.is_some()
        || arrow.params.len() != 1
    {
        return Err(unsupported("map callback parameters or async function"));
    }
    let Pat::Ident(parameter) = &arrow.params[0] else {
        return Err(unsupported(
            "map callback parameter (expected a simple identifier)",
        ));
    };
    let ArrowFunctionBody::Expr(body) = arrow.body.as_ref() else {
        return Err(unsupported("map callback block body"));
    };
    let mut nested_locals = locals.to_vec();
    nested_locals.push(parameter.id.sym.to_string());
    validate_expr(
        body,
        RenderContext::Root,
        fields,
        &nested_locals,
        module,
        imported_components,
    )
}

fn validate_element(
    element: &JSXElement,
    fields: Option<&TypedProps<'_>>,
    locals: &[String],
    module: &Module,
    imported_components: &ImportedComponents,
) -> Result<(), JsxrsError> {
    let tag = match &element.opening.name {
        JSXElementName::Ident(identifier) => identifier.sym.as_ref(),
        JSXElementName::JSXMemberExpr(_) | JSXElementName::JSXNamespacedName(_) => {
            return Err(unsupported("member or namespaced JSX tag names"));
        }
    };
    let is_component = tag.chars().next().is_some_and(char::is_uppercase);
    if is_component && tag != "Head" && !has_value_import(module, tag) {
        return Err(unsupported(&format!(
            "component `{tag}` without a default import"
        )));
    }

    if let Some(schema) = imported_components.get(tag) {
        let mut supplied = HashSet::new();
        for attribute in &element.opening.attrs {
            let JSXAttrOrSpread::JSXAttr(attribute) = attribute else {
                continue;
            };
            let name = match &attribute.name {
                JSXAttrName::Ident(identifier) => identifier.sym.as_ref(),
                JSXAttrName::JSXNamespacedName(_) => {
                    return Err(unsupported("namespaced component props"));
                }
            };
            supplied.insert(name);
            if let Some(schema) = schema
                && !schema.fields.iter().any(|field| field.name.as_str() == name)
            {
                return Err(JsxrsError::UndefinedPropAt {
                    key: name.to_owned(),
                    position: attribute.span.lo.0,
                });
            }
        }
        if let Some(schema) = schema {
            for field in &schema.fields {
                if !field.optional && !supplied.contains(field.name.as_str()) {
                    return Err(unsupported(&format!(
                        "imported component `{tag}` is missing required prop `{}`",
                        field.name
                    )));
                }
            }
        }
        if has_nonempty_children(&element.children) {
            return Err(unsupported(&format!(
                "children on imported component `{tag}` are unsupported because runtime does not forward them"
            )));
        }
    }

    for attribute in &element.opening.attrs {
        match attribute {
            JSXAttrOrSpread::SpreadElement(_) => {
                return Err(unsupported("JSX spread attributes"));
            }
            JSXAttrOrSpread::JSXAttr(attribute) => {
                let name = match &attribute.name {
                    JSXAttrName::Ident(identifier) => identifier.sym.as_ref(),
                    JSXAttrName::JSXNamespacedName(_) => {
                        return Err(unsupported("namespaced JSX attributes"));
                    }
                };
                let Some(value) = &attribute.value else {
                    continue;
                };
                match value {
                    JSXAttrValue::Str(_) => {}
                    JSXAttrValue::JSXExprContainer(container) => match &container.expr {
                        JSXExpr::JSXEmptyExpr(_) => {}
                        JSXExpr::Expr(expression) => {
                            if !is_component
                                && name.starts_with("on")
                                && name.chars().nth(2).is_some_and(char::is_uppercase)
                            {
                                if let Some(fields) = fields {
                                    let event_locals = locals
                                        .iter()
                                        .map(|name| EventLocal {
                                            name: name.clone(),
                                            is_props_alias: false,
                                        })
                                        .collect::<Vec<_>>();
                                    scan_props_accesses(expression, fields, &event_locals)?;
                                }
                                continue;
                            }
                            if !is_component && name == "style" {
                                validate_style(expression)?;
                            } else {
                                validate_expr(
                                    expression,
                                    RenderContext::Value,
                                    fields,
                                    locals,
                                    module,
                                    imported_components,
                                )?;
                            }
                        }
                    },
                    JSXAttrValue::JSXElement(_) | JSXAttrValue::JSXFragment(_) => {
                        return Err(unsupported("JSX element-valued attributes"));
                    }
                }
            }
        }
    }
    validate_children(
        &element.children,
        fields,
        locals,
        module,
        imported_components,
    )
}

fn has_nonempty_children(children: &[JSXElementChild]) -> bool {
    children.iter().any(|child| match child {
        JSXElementChild::JSXText(text) => !text.value.to_string_lossy().trim().is_empty(),
        JSXElementChild::JSXExprContainer(container) => {
            !matches!(&container.expr, JSXExpr::JSXEmptyExpr(_))
        }
        JSXElementChild::JSXElement(_) | JSXElementChild::JSXSpreadChild(_) => true,
        JSXElementChild::JSXFragment(fragment) => has_nonempty_children(&fragment.children),
    })
}

fn scan_props_accesses(
    expression: &Expr,
    fields: &TypedProps<'_>,
    locals: &[EventLocal],
) -> Result<(), JsxrsError> {
    match expression {
        Expr::Member(member) => {
            validate_event_props_member(member, fields, locals)?;
            scan_props_accesses(&member.obj, fields, locals)?;
            if let MemberProp::Computed(computed) = &member.prop {
                scan_props_accesses(&computed.expr, fields, locals)?;
            }
        }
        Expr::Bin(binary) => {
            scan_props_accesses(&binary.left, fields, locals)?;
            scan_props_accesses(&binary.right, fields, locals)?;
        }
        Expr::Cond(conditional) => {
            scan_props_accesses(&conditional.test, fields, locals)?;
            scan_props_accesses(&conditional.cons, fields, locals)?;
            scan_props_accesses(&conditional.alt, fields, locals)?;
        }
        Expr::Unary(unary) => scan_props_accesses(&unary.arg, fields, locals)?,
        Expr::Tpl(template) => {
            for expression in &template.exprs {
                scan_props_accesses(expression, fields, locals)?;
            }
        }
        Expr::Paren(paren) => scan_props_accesses(&paren.expr, fields, locals)?,
        Expr::Arrow(arrow) => {
            let mut nested_locals = locals.to_vec();
            for parameter in &arrow.params {
                let Pat::Ident(binding) = parameter else {
                    return Err(unsupported("event handler parameter pattern"));
                };
                nested_locals.push(EventLocal {
                    name: binding.id.sym.to_string(),
                    is_props_alias: false,
                });
            }
            match arrow.body.as_ref() {
                ArrowFunctionBody::Expr(expression) => {
                    scan_props_accesses(expression, fields, &nested_locals)?;
                }
                ArrowFunctionBody::FunctionBody(body) => {
                    scan_statement_sequence(&body.stmts, fields, &nested_locals)?;
                }
            }
        }
        Expr::Fn(function) => {
            scan_function_props_accesses(
                &function.function,
                function.ident.as_ref(),
                fields,
                locals,
            )?;
        }
        Expr::Call(call) => {
            if let Callee::Expr(callee) = &call.callee {
                scan_props_accesses(callee, fields, locals)?;
            }
            for argument in &call.args {
                scan_props_accesses(&argument.expr, fields, locals)?;
            }
        }
        Expr::New(new_expression) => {
            scan_props_accesses(&new_expression.callee, fields, locals)?;
            if let Some(arguments) = &new_expression.args {
                for argument in arguments {
                    scan_props_accesses(&argument.expr, fields, locals)?;
                }
            }
        }
        Expr::Seq(sequence) => {
            for expression in &sequence.exprs {
                scan_props_accesses(expression, fields, locals)?;
            }
        }
        Expr::Await(await_expression) => {
            scan_props_accesses(&await_expression.arg, fields, locals)?;
        }
        Expr::Yield(yield_expression) => {
            if let Some(argument) = &yield_expression.arg {
                scan_props_accesses(argument, fields, locals)?;
            }
        }
        Expr::Update(update) => {
            scan_props_accesses(&update.arg, fields, locals)?;
        }
        Expr::TaggedTpl(tagged_template) => {
            scan_props_accesses(&tagged_template.tag, fields, locals)?;
            for expression in &tagged_template.tpl.exprs {
                scan_props_accesses(expression, fields, locals)?;
            }
        }
        Expr::TsAs(assertion) => scan_props_accesses(&assertion.expr, fields, locals)?,
        Expr::TsTypeAssertion(assertion) => scan_props_accesses(&assertion.expr, fields, locals)?,
        Expr::TsConstAssertion(assertion) => scan_props_accesses(&assertion.expr, fields, locals)?,
        Expr::TsNonNull(assertion) => scan_props_accesses(&assertion.expr, fields, locals)?,
        Expr::TsSatisfies(assertion) => scan_props_accesses(&assertion.expr, fields, locals)?,
        Expr::Array(array) => scan_array_props_accesses(array, fields, locals)?,
        Expr::Object(object) => {
            for property in &object.props {
                match property {
                    PropOrSpread::Spread(spread) => {
                        scan_props_accesses(&spread.expr, fields, locals)?;
                    }
                    PropOrSpread::Prop(property) => match property.as_ref() {
                        Prop::KeyValue(property) => {
                            scan_props_accesses(&property.value, fields, locals)?;
                        }
                        Prop::Assign(property) => {
                            scan_props_accesses(&property.value, fields, locals)?;
                        }
                        Prop::Shorthand(_) => {}
                        Prop::Getter(_) | Prop::Setter(_) | Prop::Method(_) => {
                            return Err(unsupported("object method in event payload"));
                        }
                    },
                }
            }
        }
        Expr::JSXElement(element) => scan_element_props_accesses(element, fields, locals)?,
        Expr::JSXFragment(fragment) => {
            scan_children_props_accesses(&fragment.children, fields, locals)?;
        }
        Expr::Ident(_) | Expr::Lit(_) | Expr::This(_) | Expr::MetaProp(_) => {}
        _ => return Err(unsupported("event payload expression form")),
    }
    Ok(())
}

fn scan_function_props_accesses(
    function: &Function,
    function_name: Option<&swc_ecma_ast::Ident>,
    fields: &TypedProps<'_>,
    locals: &[EventLocal],
) -> Result<(), JsxrsError> {
    let mut nested_locals = locals.to_vec();
    if let Some(name) = function_name {
        nested_locals.push(EventLocal {
            name: name.sym.to_string(),
            is_props_alias: false,
        });
    }
    for parameter in &function.params {
        let Pat::Ident(binding) = &parameter.pat else {
            return Err(unsupported("event handler parameter pattern"));
        };
        nested_locals.push(EventLocal {
            name: binding.id.sym.to_string(),
            is_props_alias: false,
        });
    }
    if let Some(body) = &function.body {
        scan_statement_sequence(&body.stmts, fields, &nested_locals)?;
    }
    Ok(())
}

fn scan_statement_sequence(
    statements: &[Stmt],
    fields: &TypedProps<'_>,
    locals: &[EventLocal],
) -> Result<(), JsxrsError> {
    let mut nested_locals = locals.to_vec();
    for statement in statements {
        scan_statement_props_accesses(statement, fields, &nested_locals)?;
        match statement {
            Stmt::Decl(Decl::Var(variables)) => {
                let mut declaration_scope = nested_locals.clone();
                for declaration in &variables.decls {
                    let Pat::Ident(binding) = &declaration.name else {
                        return Err(unsupported("event handler local binding pattern"));
                    };
                    declaration_scope.push(EventLocal {
                        name: binding.id.sym.to_string(),
                        is_props_alias: false,
                    });
                }
                for declaration in &variables.decls {
                    let Pat::Ident(binding) = &declaration.name else {
                        return Err(unsupported("event handler local binding pattern"));
                    };
                    let is_props_alias = declaration
                        .init
                        .as_deref()
                        .is_some_and(|initializer| is_component_props_alias(initializer, &declaration_scope));
                    nested_locals.push(EventLocal {
                        name: binding.id.sym.to_string(),
                        is_props_alias,
                    });
                }
            }
            Stmt::Decl(Decl::Fn(function)) => {
                nested_locals.push(EventLocal {
                    name: function.ident.sym.to_string(),
                    is_props_alias: false,
                });
            }
            _ => {}
        }
    }
    Ok(())
}

fn is_component_props_alias(initializer: &Expr, locals: &[EventLocal]) -> bool {
    matches!(initializer.unwrap_parens(), Expr::Ident(identifier) if identifier.sym == "props")
        && !locals.iter().rev().any(|local| local.name == "props")
}

fn scan_array_props_accesses(
    array: &ArrayLit,
    fields: &TypedProps<'_>,
    locals: &[EventLocal],
) -> Result<(), JsxrsError> {
    for element in array.elems.iter().flatten() {
        scan_props_accesses(&element.expr, fields, locals)?;
    }
    Ok(())
}

fn scan_statement_props_accesses(
    statement: &Stmt,
    fields: &TypedProps<'_>,
    locals: &[EventLocal],
) -> Result<(), JsxrsError> {
    match statement {
        Stmt::Return(statement) => {
            if let Some(expression) = &statement.arg {
                scan_props_accesses(expression, fields, locals)?;
            }
        }
        Stmt::Expr(statement) => scan_props_accesses(&statement.expr, fields, locals)?,
        Stmt::Decl(Decl::Var(variables)) => {
            let mut declaration_locals = locals.to_vec();
            for declaration in &variables.decls {
                let Pat::Ident(binding) = &declaration.name else {
                    return Err(unsupported("event handler local binding pattern"));
                };
                declaration_locals.push(EventLocal {
                    name: binding.id.sym.to_string(),
                    is_props_alias: false,
                });
            }
            for declaration in &variables.decls {
                if let Some(initializer) = &declaration.init {
                    scan_props_accesses(initializer, fields, &declaration_locals)?;
                }
            }
        }
        Stmt::Decl(Decl::Fn(function)) => {
            scan_function_props_accesses(
                &function.function,
                Some(&function.ident),
                fields,
                locals,
            )?;
        }
        Stmt::If(statement) => {
            scan_props_accesses(&statement.test, fields, locals)?;
            scan_statement_props_accesses(&statement.cons, fields, locals)?;
            if let Some(alternate) = &statement.alt {
                scan_statement_props_accesses(alternate, fields, locals)?;
            }
        }
        Stmt::Block(block) => {
            scan_statement_sequence(&block.stmts, fields, locals)?;
        }
        _ => return Err(unsupported("event handler statement form")),
    }
    Ok(())
}

fn scan_element_props_accesses(
    element: &JSXElement,
    fields: &TypedProps<'_>,
    locals: &[EventLocal],
) -> Result<(), JsxrsError> {
    for attribute in &element.opening.attrs {
        if let JSXAttrOrSpread::JSXAttr(attribute) = attribute
            && let Some(JSXAttrValue::JSXExprContainer(container)) = &attribute.value
            && let JSXExpr::Expr(expression) = &container.expr
        {
            scan_props_accesses(expression, fields, locals)?;
        }
    }
    scan_children_props_accesses(&element.children, fields, locals)
}

fn scan_children_props_accesses(
    children: &[JSXElementChild],
    fields: &TypedProps<'_>,
    locals: &[EventLocal],
) -> Result<(), JsxrsError> {
    for child in children {
        match child {
            JSXElementChild::JSXExprContainer(container) => {
                if let JSXExpr::Expr(expression) = &container.expr {
                    scan_props_accesses(expression, fields, locals)?;
                }
            }
            JSXElementChild::JSXElement(element) => {
                scan_element_props_accesses(element, fields, locals)?;
            }
            JSXElementChild::JSXFragment(fragment) => {
                scan_children_props_accesses(&fragment.children, fields, locals)?;
            }
            JSXElementChild::JSXText(_) | JSXElementChild::JSXSpreadChild(_) => {}
        }
    }
    Ok(())
}

fn validate_children(
    children: &[JSXElementChild],
    fields: Option<&TypedProps<'_>>,
    locals: &[String],
    module: &Module,
    imported_components: &ImportedComponents,
) -> Result<(), JsxrsError> {
    for child in children {
        match child {
            JSXElementChild::JSXText(_) => {}
            JSXElementChild::JSXExprContainer(container) => match &container.expr {
                JSXExpr::JSXEmptyExpr(_) => {}
                JSXExpr::Expr(expression) => {
                    validate_expr(
                        expression,
                        RenderContext::Child,
                        fields,
                        locals,
                        module,
                        imported_components,
                    )?;
                }
            },
            JSXElementChild::JSXElement(element) => {
                validate_element(element, fields, locals, module, imported_components)?;
            }
            JSXElementChild::JSXFragment(fragment) => {
                validate_children(
                    &fragment.children,
                    fields,
                    locals,
                    module,
                    imported_components,
                )?;
            }
            JSXElementChild::JSXSpreadChild(_) => {
                return Err(unsupported("JSX spread children"));
            }
        }
    }
    Ok(())
}

fn validate_style(expression: &Expr) -> Result<(), JsxrsError> {
    let Expr::Object(object) = expression.unwrap_parens() else {
        return Err(unsupported("non-object style expressions"));
    };
    for property in &object.props {
        let PropOrSpread::Prop(property) = property else {
            return Err(unsupported("spread style members"));
        };
        let Prop::KeyValue(property) = property.as_ref() else {
            return Err(unsupported("non-key/value style members"));
        };
        if !matches!(&property.key, PropName::Ident(_) | PropName::Str(_))
            || !matches!(
                property.value.as_ref(),
                Expr::Lit(Lit::Str(_) | Lit::Num(_))
            )
        {
            return Err(unsupported("non-literal style members"));
        }
    }
    Ok(())
}

fn has_value_import(module: &Module, local_name: &str) -> bool {
    module.body.iter().any(|item| {
        let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = item else {
            return false;
        };
        !import.type_only
            && import.specifiers.iter().any(|specifier| {
                matches!(specifier, ImportSpecifier::Default(default) if default.local.sym == local_name)
            })
    })
}

fn supported_binary(operator: BinaryOp) -> bool {
    matches!(
        operator,
        BinaryOp::LogicalAnd
            | BinaryOp::LogicalOr
            | BinaryOp::EqEq
            | BinaryOp::EqEqEq
            | BinaryOp::NotEq
            | BinaryOp::NotEqEq
            | BinaryOp::Gt
            | BinaryOp::GtEq
            | BinaryOp::Lt
            | BinaryOp::LtEq
            | BinaryOp::Add
    )
}

fn unsupported(message: &str) -> JsxrsError {
    JsxrsError::Unsupported(format!("{message} is unsupported by checked rendering"))
}

#[cfg(test)]
mod tests {
    use super::validate_component;
    use crate::{JsxrsError, parse_source};

    fn validate(source: &str) -> Result<(), JsxrsError> {
        let (module, _) = parse_source(source, "component.tsx")?;
        validate_component(&module)
    }

    #[test]
    fn accepts_local_typed_props_and_parenthesized_jsx_children() -> Result<(), JsxrsError> {
        validate(
            "interface Props { show: boolean; }\n\
             export default function Page(props: Props) {\n\
               return <div>{props.show ? (<b>visible</b>) : null}</div>;\n\
             }",
        )
    }

    #[test]
    fn preserves_zero_single_multiple_and_unsupported_component_parameter_rules()
    -> Result<(), JsxrsError> {
        validate("export default function Page() { return <div />; }")?;
        validate("export default function Page(props) { return <div />; }")?;

        for source in [
            "export default function Page(props, extra) { return <div />; }",
            "export default function Page({ name }) { return <div />; }",
        ] {
            let error = validate(source).err().ok_or_else(|| {
                JsxrsError::Unsupported("invalid component parameters unexpectedly passed".into())
            })?;
            assert!(matches!(error, JsxrsError::Unsupported(message) if message.contains("component parameters")));
        }
        Ok(())
    }

    #[test]
    fn rejects_unsupported_expressions_in_unselected_branches() -> Result<(), JsxrsError> {
        let error = validate(
            "export default function Page() { return <div>{false && props.lookup()}</div>; }",
        )
        .err()
        .ok_or_else(|| JsxrsError::Unsupported("unsupported branch unexpectedly passed".into()))?;
        assert!(
            matches!(error, JsxrsError::Unsupported(message) if message.contains("call other than array"))
        );
        Ok(())
    }

    #[test]
    fn rejects_unknown_typed_props_even_in_event_payloads() -> Result<(), JsxrsError> {
        let error = validate(
            "interface Props { name: string; }\n\
             export default function Page(props: Props) {\n\
               return <button onClick={() => props.typo}>{props.name}</button>;\n\
             }",
        )
        .err()
        .ok_or_else(|| JsxrsError::Unsupported("unknown typed prop unexpectedly passed".into()))?;
        assert!(matches!(error, JsxrsError::UndefinedProp(field) if field == "typo"));
        Ok(())
    }

    #[test]
    fn rejects_component_body_local_declarations() -> Result<(), JsxrsError> {
        let error = validate(
            "export default function Page() { const value = 1; return <div>{value}</div>; }",
        )
        .err()
        .ok_or_else(|| JsxrsError::Unsupported("local declaration unexpectedly passed".into()))?;
        assert!(
            matches!(error, JsxrsError::Unsupported(message) if message.contains("body statements"))
        );
        Ok(())
    }
}
