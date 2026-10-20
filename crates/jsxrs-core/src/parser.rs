use std::fmt::Write as _;

use bytes_str::BytesStr;
use swc_common::{FileName, SourceMap, Spanned, sync::Lrc};
use swc_ecma_ast::{EsVersion, Module};
use swc_ecma_parser::{EsSyntax, Parser, StringInput, Syntax, TsSyntax, lexer::Lexer};

use crate::JsxrsError;

fn syntax_for_file(file_name: &str) -> Syntax {
    let ext = std::path::Path::new(file_name)
        .extension()
        .map(|extension| extension.to_string_lossy().to_lowercase());
    if ext.as_deref() == Some("tsx") || ext.as_deref() == Some("ts") {
        Syntax::Typescript(TsSyntax {
            tsx: true,
            ..Default::default()
        })
    } else {
        Syntax::Es(EsSyntax {
            jsx: true,
            ..Default::default()
        })
    }
}

/// Parse JavaScript/JSX or TypeScript/TSX using the source file extension.
///
/// # Errors
///
/// Returns an error when SWC cannot parse the input as a module.
pub fn parse_source(source: &str, file_name: &str) -> Result<(Module, Lrc<SourceMap>), JsxrsError> {
    let source_map: Lrc<SourceMap> = Lrc::default();
    let source_file = source_map.new_source_file(
        FileName::Custom(file_name.into()).into(),
        BytesStr::from(source.to_owned()),
    );
    let lexer = Lexer::new(
        syntax_for_file(file_name),
        EsVersion::default(),
        StringInput::from(&*source_file),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    let parsed = parser.parse_module();
    let mut errors = parser.take_errors();
    match parsed {
        Ok(module) if errors.is_empty() => Ok((module, source_map)),
        Ok(_) => Err(parse_errors(&errors, &source_map, file_name)),
        Err(error) => {
            errors.push(error);
            Err(parse_errors(&errors, &source_map, file_name))
        }
    }
}

fn parse_errors(
    errors: &[swc_ecma_parser::error::Error],
    source_map: &SourceMap,
    file_name: &str,
) -> JsxrsError {
    let mut messages = String::new();
    for (index, error) in errors.iter().enumerate() {
        if index > 0 {
            messages.push('\n');
        }
        let span = error.span();
        let (line, column) = if span.is_dummy() {
            (1, 1)
        } else {
            let location = source_map.lookup_char_pos(span.lo);
            (location.line, location.col_display + 1)
        };
        let _ = write!(messages, "{file_name}:{line}:{column}: {:?}", error.kind());
    }
    JsxrsError::Parse(messages)
}

#[cfg(test)]
mod tests {
    use super::parse_source;
    use crate::JsxrsError;

    #[test]
    fn parse_errors_include_the_source_position() -> Result<(), JsxrsError> {
        let error = parse_source(
            "export default function Page() { return <div>; }",
            "page.tsx",
        )
        .err()
        .ok_or_else(|| JsxrsError::Unsupported("invalid TSX unexpectedly parsed".into()))?;
        assert!(matches!(error, JsxrsError::Parse(message) if message.contains("page.tsx:1:")));
        Ok(())
    }

    #[test]
    fn multiple_parse_errors_keep_order_and_newline_separators() -> Result<(), JsxrsError> {
        let error = parse_source("const first = ;\nconst second = ;", "multiple.tsx")
            .err()
            .ok_or_else(|| JsxrsError::Unsupported("invalid TSX unexpectedly parsed".into()))?;
        let JsxrsError::Parse(message) = error else {
            return Err(JsxrsError::Unsupported(
                "invalid TSX returned a non-parse error".into(),
            ));
        };
        let mut lines = message.lines();
        let first = lines
            .next()
            .ok_or_else(|| JsxrsError::Unsupported("missing first parse error".into()))?;
        let second = lines
            .next()
            .ok_or_else(|| JsxrsError::Unsupported("missing second parse error".into()))?;
        assert!(first.starts_with("multiple.tsx:1:"));
        assert!(second.starts_with("multiple.tsx:2:"));
        assert!(!message.ends_with('\n'));
        Ok(())
    }
}
