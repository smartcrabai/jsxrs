# jsxrs-macros

Procedural macros reexported by the `jsxrs` runtime crate.

- `jsxrs::component!(pub module_name, "path/to/component.tsx")` reads the source path relative to the consuming crate, generates a Rust props type for supported local props schemas, and tracks the component's local default-import graph.
- With the `axum` feature enabled, `jsxrs::routes!("app")` discovers routes at compile time and embeds a fixed route-entry table.

Applications should normally depend on `jsxrs` and invoke its reexports instead of adding a direct dependency on this implementation crate. The macros read source files at compile time and the generated render/router paths still require those files at runtime.
