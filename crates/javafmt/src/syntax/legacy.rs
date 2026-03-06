use super::tree::ParsedFile;
use crate::{comments, lexer, parser};

pub(crate) fn parse(source: &str) -> ParsedFile<'_> {
    let lexed = lexer::lex(source);
    let cst = parser::parse(&lexed);
    let comments = comments::attach(&cst, &lexed);

    let _ = lexed;

    ParsedFile { cst, comments }
}
