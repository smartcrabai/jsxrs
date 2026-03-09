# jsxrs

A Rust library for rendering JSX/TSX to complete HTML documents at build-time or server-side.

## Overview

jsxrs parses JSX/TSX files and renders them to full HTML documents. It leverages [SWC](https://swc.rs/) for high-performance parsing and supports modern JSX features including components, props, fragments, and TypeScript.

## Features

- **JSX & TSX Support** - Parse and render both JavaScript and TypeScript JSX files
- **Props & Expressions** - Pass runtime values to components via props
- **Component Imports** - Import and render other JSX/TSX components
- **Special `<Head>` Component** - Add elements to the document `<head>` (title, meta, link, style, script)
- **Tailwind CSS Integration** - Automatically generate CSS from Tailwind class names
- **TypeScript Codegen** - Generate Rust structs from TypeScript interfaces
- **XSS Protection** - Automatic HTML escaping for user content

## Installation

Add `jsxrs` to your `Cargo.toml`:

```toml
[dependencies]
jsxrs = "0.1"
```

## Usage

### Basic Rendering

```rust
use jsxrs::{render_string, RenderConfig};
use serde_json::json;

let source = r#"
export default function Greeting(props) {
  return <div>Hello, {props.name}!</div>;
}
"#;

let config = RenderConfig::default();
let props = json!({"name": "World"});
let html = render_string(source, "page.jsx", &props, &config).unwrap();

println!("{}", html);
```

### Rendering from File

```rust
use jsxrs::{render_file, RenderConfig};
use serde_json::json;
use std::path::PathBuf;

let path = PathBuf::from("src/page.tsx");
let config = RenderConfig {
    pretty: true,
    base_dir: Some(PathBuf::from("src")),
    ..Default::default()
};
let html = render_file(&path, &json!({}), &config).unwrap();
```

### Configuration

```rust
use jsxrs::{HeadElement, RenderConfig};

let config = RenderConfig {
    pretty: true,                          // Format output with indentation
    base_dir: Some(PathBuf::from(".")),    // Base directory for component resolution
    head_elements: vec![
        HeadElement::Title("My Page".to_string()),
        HeadElement::Meta {
            name: "description".to_string(),
            content: "A wonderful page".to_string()
        },
    ],
    tailwind: false,                       // Enable Tailwind CSS generation
    fragment: false,                       // Return fragment (body HTML only) instead of full document
};
```

### Tailwind CSS

Enable Tailwind CSS generation to automatically collect and generate CSS from class names:

```rust
let config = RenderConfig {
    tailwind: true,
    ..Default::default()
};
```

The library will extract all `class` attributes and generate the corresponding Tailwind CSS.

### Using the `<Head>` Component

Add elements to the document head using the special `<Head>` component:

```tsx
export default function Page() {
  return (
    <>
      <Head>
        <title>My Page</title>
        <meta name="description" content="Welcome" />
        <link rel="stylesheet" href="/styles.css" />
      </Head>
      <div>Page content here</div>
    </>
  );
}
```

### Fragment Rendering (HTMX)

When using jsxrs as an HTMX backend, return a full HTML document for the initial page load and an HTML fragment for subsequent HTMX requests (identified by the `HX-Request` header).

Set `fragment: true` to skip the document wrapper and return only the body HTML:

```rust
use jsxrs::{render_string, RenderConfig};
use serde_json::json;

// In your request handler:
let is_htmx_request = request.headers().get("HX-Request").is_some();

let config = RenderConfig {
    fragment: is_htmx_request,
    ..Default::default()
};

let html = render_string(source, "page.jsx", &props, &config).unwrap();
// Returns full document for normal requests, body HTML only for HTMX requests
```

In fragment mode:
- The `<!DOCTYPE html><html><head>...</head><body>...</body></html>` wrapper is omitted
- `<Head>` component content is ignored (already loaded by the full-page response)
- Tailwind CSS `<style>` tags are skipped (already loaded by the full-page response)

### Component Imports

Import and render other components:

```tsx
import Button from './components/button';

export default function Page() {
  return (
    <div>
      <Button label="Click me" />
    </div>
  );
}
```

Ensure `base_dir` is configured in `RenderConfig` for import resolution.

### TypeScript Codegen

Generate Rust structs from TypeScript interfaces:

```rust
use jsxrs::codegen;

codegen::generate_types(
    &["src/types.tsx"],
    "src/generated/types.rs",
).unwrap();
```

Given a TypeScript interface:

```tsx
interface User {
  id: number;
  name: string;
  email?: string;
}
```

This generates:

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: f64,
    pub name: String,
    pub email: Option<String>,
}
```

## API Reference

### `render_string`

Renders a JSX/TSX source string to a complete HTML document.

```rust
pub fn render_string(
    source: &str,
    file_name: &str,
    props: &Value,
    config: &RenderConfig,
) -> Result<String, JsxrsError>
```

### `render_file`

Renders a JSX/TSX file to a complete HTML document.

```rust
pub fn render_file(
    path: &Path,
    props: &Value,
    config: &RenderConfig,
) -> Result<String, JsxrsError>
```

### `generate_types`

Generates Rust struct definitions from TypeScript interface declarations.

```rust
pub fn generate_types(
    tsx_paths: &[impl AsRef<Path>],
    output_path: &Path,
) -> Result<(), JsxrsError>
```

### `RenderConfig`

Configuration for the rendering pipeline:

| Field | Type | Description |
|-------|------|-------------|
| `pretty` | `bool` | Format output with indentation |
| `base_dir` | `Option<PathBuf>` | Base directory for component resolution |
| `head_elements` | `Vec<HeadElement>` | Static head elements to include |
| `tailwind` | `bool` | Enable Tailwind CSS generation |
| `fragment` | `bool` | Return body HTML only, skipping the full document wrapper |

### `HeadElement`

Elements that can be added to the document head:

- `Title(String)` - Page title
- `Meta { name, content }` - Meta tags
- `Link { rel, href }` - Link tags
- `Style(String)` - Inline styles
- `Script(String)` - Inline scripts

## Supported JSX Features

- HTML elements (div, span, p, etc.)
- Self-closing tags (img, br, hr, input, etc.)
- JSX Fragments (`<>...</>`)
- Props and expressions (`{props.value}`, `{1 + 1}`)
- Type annotations in TSX files
- Arrow function and function declarations
- Component composition

## Security

jsxrs automatically escapes HTML content to prevent XSS attacks. User-supplied content in text nodes and attributes is properly escaped.

