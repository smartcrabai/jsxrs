use bytes_str::BytesStr;
use swc_common::{FileName, SourceMap, sync::Lrc};
use swc_ecma_ast::Module;
use swc_ecma_parser::{EsSyntax, Parser, StringInput, Syntax, TsSyntax, lexer::Lexer};

use crate::error::JsxrsError;

fn syntax_for_file(file_name: &str) -> Syntax {
    if file_name.ends_with(".tsx") || file_name.ends_with(".ts") {
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

pub fn parse_source(source: &str, file_name: &str) -> Result<(Module, Lrc<SourceMap>), JsxrsError> {
    let cm: Lrc<SourceMap> = Default::default();
    let src: BytesStr = source.to_owned().into();
    let fm = cm.new_source_file(FileName::Custom(file_name.into()).into(), src);

    let syntax = syntax_for_file(file_name);
    let lexer = Lexer::new(syntax, Default::default(), StringInput::from(&*fm), None);
    let mut parser = Parser::new_from(lexer);

    let module = parser
        .parse_module()
        .map_err(|e| JsxrsError::Parse(format!("{e:?}")))?;

    Ok((module, cm))
}
