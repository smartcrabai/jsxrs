# jsxrs-core

Shared implementation crate used by the `jsxrs` runtime and `jsxrs-macros`. It contains the SWC parser, import resolver, checked component validator, props schema extraction, legacy Rust type generator, and file-route discovery.

Most applications should depend on `jsxrs` rather than this internal support crate. The `jsxrs` crate reexports the public runtime API and the `component!` macro.
