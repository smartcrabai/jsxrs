use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitByteStr, LitStr, Token, Visibility, parse_macro_input};

struct ComponentInput {
    visibility: Visibility,
    module: Ident,
    source_path: LitStr,
}

impl Parse for ComponentInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let visibility = input.parse()?;
        let module = input.parse()?;
        input.parse::<Token![,]>()?;
        let source_path = input.parse()?;
        if !input.is_empty() {
            return Err(input.error("unexpected tokens after component source path"));
        }
        Ok(Self {
            visibility,
            module,
            source_path,
        })
    }
}

#[proc_macro]
pub fn component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ComponentInput);
    match expand_component(input) {
        Ok(expansion) => expansion.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro]
pub fn routes(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as LitStr);
    match expand_routes(&input) {
        Ok(expansion) => expansion.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand_routes(input: &LitStr) -> syn::Result<proc_macro2::TokenStream> {
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| syn::Error::new(input.span(), "CARGO_MANIFEST_DIR is not set"))?;
    let manifest_dir = manifest_dir.canonicalize().map_err(|error| {
        syn::Error::new(
            input.span(),
            format!("unable to resolve crate source root: {error}"),
        )
    })?;
    let route_dir = PathBuf::from(input.value());
    let route_dir = if route_dir.is_absolute() {
        route_dir
    } else {
        manifest_dir.join(route_dir)
    };
    let route_dir = route_dir.canonicalize().map_err(|error| {
        syn::Error::new(
            input.span(),
            format!(
                "{}: unable to resolve route directory: {error}",
                input.value()
            ),
        )
    })?;
    let entries = jsxrs_core::discovery::scan_routes(&route_dir).map_err(|error| {
        syn::Error::new(
            input.span(),
            format!("{}: route discovery failed: {error}", input.value()),
        )
    })?;
    let mut checked_files = Vec::new();
    let mut seen_files = HashSet::new();
    let mut route_schemas = HashMap::<&PathBuf, Option<jsxrs_core::PropsSchema>>::new();
    let mut route_accesses = HashMap::<&PathBuf, HashSet<String>>::new();
    for entry in &entries {
        for source_path in std::iter::once(&entry.page_file).chain(entry.layouts.iter()) {
            if route_schemas.contains_key(&source_path) {
                continue;
            }
            let source = std::fs::read_to_string(source_path).map_err(|error| {
                syn::Error::new(
                    input.span(),
                    format!(
                        "{}: unable to read route source: {error}",
                        source_path.display()
                    ),
                )
            })?;
            let file_name = source_path.to_string_lossy().into_owned();
            let (module, source_map) = jsxrs_core::parse_source(&source, &file_name)
                .map_err(|error| syn::Error::new(input.span(), format!("{file_name}: {error}")))?;
            let schema = jsxrs_core::extract_props_schema(&module)
                .map_err(|error| syn::Error::new(input.span(), format!("{file_name}: {error}")))?;
            route_schemas.insert(source_path, schema);
            let graph = jsxrs_core::resolve_component_graph_snapshot_with_source_map(
                source_path,
                &module,
                Some(&source),
                source_map,
            )
            .map_err(|error| {
                syn::Error::new(input.span(), graph_error_message(&file_name, error))
            })?;
            let accessed = jsxrs_core::validate_component_accesses(&module)
                .map_err(|error| syn::Error::new(input.span(), format!("{file_name}: {error}")))?;
            route_accesses.insert(source_path, accessed);
            for file in graph.files {
                if seen_files.insert(file.path.clone()) {
                    checked_files.push(file);
                }
            }
        }
    }
    validate_route_props(&entries, &route_schemas, &route_accesses)
        .map_err(|message| syn::Error::new(input.span(), message))?;
    let runtime_path = runtime_crate_path(input.span())?;
    let source_root = LitStr::new(&manifest_dir.to_string_lossy(), input.span());
    let checked_sources =
        checked_sources_tokens(&checked_files, &manifest_dir, input.span(), &runtime_path)?;
    let route_entries = route_entries_tokens(&entries, &manifest_dir, input.span(), &runtime_path)?;
    Ok(quote! {{
        let checked_sources = #checked_sources;
        #runtime_path::router::JsxRouter::__from_checked_sources(
            vec![#(#route_entries),*],
            checked_sources,
            ::std::path::PathBuf::from(#source_root),
        )
    }})
}

fn route_entries_tokens(
    entries: &[jsxrs_core::discovery::RouteEntry],
    manifest_dir: &Path,
    span: proc_macro2::Span,
    runtime_path: &proc_macro2::TokenStream,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    entries
        .iter()
        .map(|entry| {
            let axum_path = LitStr::new(&entry.axum_path, span);
            let page_file = relative_source_path(&entry.page_file, manifest_dir, span)?;
            let layouts = entry
                .layouts
                .iter()
                .map(|path| relative_source_path(path, manifest_dir, span))
                .collect::<syn::Result<Vec<_>>>()?;
            Ok(quote! {
                #runtime_path::router::RouteEntry {
                    axum_path: #axum_path.to_owned(),
                    page_file: ::std::path::PathBuf::from(#page_file),
                    layouts: vec![#(::std::path::PathBuf::from(#layouts)),*],
                }
            })
        })
        .collect()
}

fn validate_route_props(
    entries: &[jsxrs_core::discovery::RouteEntry],
    schemas: &HashMap<&PathBuf, Option<jsxrs_core::PropsSchema>>,
    accesses: &HashMap<&PathBuf, HashSet<String>>,
) -> Result<(), String> {
    let mut layout_props = HashMap::<&PathBuf, HashSet<&str>>::new();
    for entry in entries {
        let supplied = route_parameter_names(&entry.axum_path);
        if let Some(Some(schema)) = schemas.get(&&entry.page_file) {
            ensure_required_route_props(schema, &supplied, &entry.page_file, "page")?;
        }
        if let Some(accessed) = accesses.get(&&entry.page_file) {
            ensure_accessed_route_props(accessed, &supplied, &entry.page_file, "page")?;
        }
        for layout in &entry.layouts {
            if let Some(available) = layout_props.get_mut(&layout) {
                available.retain(|name| supplied.contains(name));
            } else {
                layout_props.insert(layout, supplied.clone());
            }
        }
    }

    for (layout, mut supplied) in layout_props {
        supplied.insert("children");
        if let Some(Some(schema)) = schemas.get(&layout) {
            ensure_required_route_props(schema, &supplied, layout, "layout")?;
        }
        if let Some(accessed) = accesses.get(&layout) {
            ensure_accessed_route_props(accessed, &supplied, layout, "layout")?;
        }
    }
    Ok(())
}

fn ensure_accessed_route_props(
    accessed: &HashSet<String>,
    supplied: &HashSet<&str>,
    source_path: &Path,
    component_kind: &str,
) -> Result<(), String> {
    for name in accessed {
        if !supplied.contains(name.as_str()) {
            return Err(format!(
                "{}: {component_kind} accesses prop `{name}` that is not supplied by its route context",
                source_path.display()
            ));
        }
    }
    Ok(())
}

fn route_parameter_names(route: &str) -> HashSet<&str> {
    route
        .split('/')
        .filter_map(|segment| {
            segment
                .strip_prefix('{')
                .and_then(|parameter| parameter.strip_suffix('}'))
                .map(|parameter| parameter.strip_prefix('*').unwrap_or(parameter))
        })
        .collect()
}

fn ensure_required_route_props(
    schema: &jsxrs_core::PropsSchema,
    supplied: &HashSet<&str>,
    source_path: &Path,
    component_kind: &str,
) -> Result<(), String> {
    for field in schema.fields.iter().filter(|field| !field.optional) {
        if !supplied.contains(field.name.as_str()) {
            return Err(format!(
                "{}: {component_kind} requires prop `{}` that is not supplied by its route context",
                source_path.display(),
                field.name
            ));
        }
    }
    Ok(())
}

fn expand_component(input: ComponentInput) -> syn::Result<proc_macro2::TokenStream> {
    let (source_path, module_ast, checked_files, source_root) = read_component_source(&input)?;
    let display_path = input.source_path.value();
    let schema = jsxrs_core::extract_props_schema(&module_ast).map_err(|error| {
        syn::Error::new(input.source_path.span(), format!("{display_path}: {error}"))
    })?;
    let source_identity =
        relative_source_path(&source_path, &source_root, input.source_path.span())?;
    let source_root_literal = LitStr::new(&source_root.to_string_lossy(), input.source_path.span());
    let runtime_path = runtime_crate_path(input.source_path.span())?;
    let checked_sources = checked_sources_tokens(
        &checked_files,
        &source_root,
        input.source_path.span(),
        &runtime_path,
    )?;
    let (props_definition, render_body, props_type) = match schema {
        Some(schema) => generate_typed_component(
            &schema,
            &input.source_path,
            &source_identity,
            &source_root_literal,
            &checked_sources,
            &runtime_path,
        )?,
        None => generate_untyped_component(
            &source_identity,
            &source_root_literal,
            &checked_sources,
            &runtime_path,
        ),
    };
    let visibility = input.visibility;
    let module_name = input.module;
    Ok(quote! {
        #visibility mod #module_name {
            #props_definition

            pub fn render(
                props: #props_type,
                config: &#runtime_path::RenderConfig,
            ) -> ::std::result::Result<String, #runtime_path::JsxrsError> {
                render_with_context(props, config, &#runtime_path::CheckedRenderContext::default())
            }

            pub fn render_with_context(
                props: #props_type,
                config: &#runtime_path::RenderConfig,
                context: &#runtime_path::CheckedRenderContext,
            ) -> ::std::result::Result<String, #runtime_path::JsxrsError> {
                #render_body
            }
        }
    })
}

fn runtime_crate_path(span: proc_macro2::Span) -> syn::Result<proc_macro2::TokenStream> {
    match crate_name("jsxrs") {
        Ok(FoundCrate::Itself) => Ok(quote!(crate)),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name.replace('-', "_"), span);
            Ok(quote!(::#ident))
        }
        Err(error) => Err(syn::Error::new(
            span,
            format!("component! requires the jsxrs runtime dependency: {error}"),
        )),
    }
}

fn read_component_source(
    input: &ComponentInput,
) -> syn::Result<(
    PathBuf,
    swc_ecma_ast::Module,
    Vec<jsxrs_core::graph::ResolvedSource>,
    PathBuf,
)> {
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").ok_or_else(|| {
        syn::Error::new(input.source_path.span(), "CARGO_MANIFEST_DIR is not set")
    })?);
    let requested_path = PathBuf::from(input.source_path.value());
    let source_path = if requested_path.is_absolute() {
        requested_path
    } else {
        manifest_dir.join(requested_path)
    };
    let source_path = source_path.canonicalize().map_err(|error| {
        syn::Error::new(
            input.source_path.span(),
            format!(
                "{}: unable to resolve component source: {error}",
                input.source_path.value()
            ),
        )
    })?;
    let source = std::fs::read_to_string(&source_path).map_err(|error| {
        syn::Error::new(
            input.source_path.span(),
            format!(
                "{}: unable to read component source: {error}",
                input.source_path.value()
            ),
        )
    })?;
    let display_path = input.source_path.value();
    let (module, source_map) =
        jsxrs_core::parse_source(&source, &display_path).map_err(|error| {
            syn::Error::new(input.source_path.span(), format!("{display_path}: {error}"))
        })?;
    let checked_files = jsxrs_core::resolve_component_graph_snapshot_with_source_map(
        &source_path,
        &module,
        Some(&source),
        source_map,
    )
    .map_err(|error| {
        syn::Error::new(
            input.source_path.span(),
            graph_error_message(&display_path, error),
        )
    })?;
    let source_root = manifest_dir.canonicalize().map_err(|error| {
        syn::Error::new(
            input.source_path.span(),
            format!("unable to resolve crate source root: {error}"),
        )
    })?;
    Ok((source_path, module, checked_files.files, source_root))
}

fn graph_error_message(path: &str, error: jsxrs_core::JsxrsError) -> String {
    match error {
        diagnostic @ jsxrs_core::JsxrsError::ValidationDiagnostic { .. } => diagnostic.to_string(),
        error => format!("{path}: {error}"),
    }
}

fn relative_source_path(
    path: &Path,
    source_root: &Path,
    span: proc_macro2::Span,
) -> syn::Result<LitStr> {
    let canonical_path = path.canonicalize().map_err(|error| {
        syn::Error::new(
            span,
            format!(
                "{}: unable to resolve checked source path: {error}",
                path.display()
            ),
        )
    })?;
    let relative = canonical_path.strip_prefix(source_root).map_err(|_| {
        syn::Error::new(
            span,
            format!(
                "checked source is outside the crate source root: {}",
                path.display()
            ),
        )
    })?;
    Ok(LitStr::new(&normalized_relative_identity(relative), span))
}

fn normalized_relative_identity(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn checked_sources_tokens(
    files: &[jsxrs_core::graph::ResolvedSource],
    source_root: &Path,
    span: proc_macro2::Span,
    runtime_path: &proc_macro2::TokenStream,
) -> syn::Result<proc_macro2::TokenStream> {
    let mut file_entries = Vec::new();
    let mut import_entries = Vec::new();
    for file in files {
        let identity = relative_source_path(&file.path, source_root, span)?;
        let relative = identity.value();
        let source_bytes = LitByteStr::new(file.source.as_bytes(), span);
        file_entries.push(quote!({
            let _ = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/", #relative));
            (#identity, #source_bytes as &'static [u8])
        }));
        for import in &file.imports {
            let specifier = LitStr::new(&import.specifier, span);
            let target = relative_source_path(&import.target, source_root, span)?;
            import_entries.push(quote!((#identity, #specifier, #target)));
        }
    }
    Ok(quote!(::std::sync::Arc::new(
        #runtime_path::__private::CheckedSourceSet::new(
            vec![#(#file_entries),*],
            vec![#(#import_entries),*],
        )
    )))
}

fn generate_typed_component(
    schema: &jsxrs_core::PropsSchema,
    span_source: &LitStr,
    source_path: &LitStr,
    source_root: &LitStr,
    checked_sources: &proc_macro2::TokenStream,
    runtime_path: &proc_macro2::TokenStream,
) -> syn::Result<(
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
)> {
    let serde_crate = LitStr::new(
        &format!("{runtime_path}::__private::serde"),
        span_source.span(),
    );
    let fields = generate_struct_fields(
        &schema.fields,
        &runtime_path.to_string(),
        span_source.span(),
    )?;
    let props_definition = quote! {
        #[derive(
            #runtime_path::__private::serde::Serialize,
            #runtime_path::__private::serde::Deserialize,
            Debug,
            Clone
        )]
        #[serde(crate = #serde_crate)]
        pub struct Props {
            #(#fields)*
        }
    };
    let render_body = quote! {
        let value = #runtime_path::__private::serde_json::to_value(props)?;
        let checked_context = context.clone().__with_checked_sources(
            #source_root,
            #checked_sources,
        );
        #runtime_path::render_file_with_context(
            ::std::path::Path::new(#source_path),
            &value,
            config,
            &checked_context,
        )
    };
    Ok((props_definition, render_body, quote!(&Props)))
}

fn generate_struct_fields(
    fields: &[jsxrs_core::PropsField],
    runtime_prefix: &str,
    span: proc_macro2::Span,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let mut rust_fields = std::collections::HashMap::<String, &str>::new();
    fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let base_type = if field.rust_type.contains("serde_json::") {
                Cow::Owned(field.rust_type.replace(
                    "serde_json::",
                    &format!("{runtime_prefix}::__private::serde_json::"),
                ))
            } else {
                Cow::Borrowed(field.rust_type.as_str())
            };
            let (ident_name, lowered_type) = jsxrs_core::codegen::lower_rust_field(
                &field.name,
                index,
                base_type.as_ref(),
                field.optional || field.nullable,
            );
            let ident: Ident = syn::parse_str(&ident_name).map_err(|error| {
                syn::Error::new(
                    span,
                    format!("{}: invalid generated Rust field name: {error}", field.name),
                )
            })?;
            let canonical_ident = ident_name.strip_prefix("r#").unwrap_or(&ident_name);
            if let Some(previous) = rust_fields.insert(canonical_ident.to_owned(), field.name.as_str()) {
                return Err(syn::Error::new(
                    span,
                    format!(
                        "TypeScript props `{previous}` and `{}` map to duplicate Rust field `{ident_name}`",
                        field.name
                    ),
                ));
            }
            let ts_name = LitStr::new(&field.name, span);
            let rust_type: syn::Type = syn::parse_str(&lowered_type).map_err(|error| {
                syn::Error::new(
                    span,
                    format!("{}: invalid generated Rust type: {error}", field.name),
                )
            })?;
            Ok(quote! {
                #[serde(rename = #ts_name)]
                pub #ident: #rust_type,
            })
        })
        .collect()
}

fn generate_untyped_component(
    source_path: &LitStr,
    source_root: &LitStr,
    checked_sources: &proc_macro2::TokenStream,
    runtime_path: &proc_macro2::TokenStream,
) -> (
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
) {
    let render_body = quote! {
        let checked_context = context.clone().__with_checked_sources(
            #source_root,
            #checked_sources,
        );
        #runtime_path::render_file_with_context(
            ::std::path::Path::new(#source_path),
            props,
            config,
            &checked_context,
        )
    };
    (
        quote! {},
        render_body,
        quote!(&#runtime_path::__private::serde_json::Value),
    )
}

#[cfg(test)]
mod tests {
    use super::normalized_relative_identity;
    use std::path::{Path, PathBuf};

    #[test]
    fn windows_style_relative_identity_becomes_a_nested_native_path() {
        let serialized = normalized_relative_identity(Path::new(r"components\card.tsx"));
        assert_eq!(serialized, "components/card.tsx");
        assert_eq!(
            PathBuf::from(serialized),
            Path::new("components").join("card.tsx")
        );
    }
}
