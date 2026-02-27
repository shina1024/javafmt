use crate::ir::FormatIr;

#[derive(Debug, Clone, Copy)]
pub struct PrintedDoc<'a> {
    pub text: &'a str,
}

pub fn print<'a>(ir: &FormatIr<'a>) -> PrintedDoc<'a> {
    let _ = ir.token_count + ir.line_comment_count;
    PrintedDoc { text: ir.source }
}
