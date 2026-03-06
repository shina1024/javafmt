use crate::syntax::ParsedFile;
use crate::{emit, ir, printer};

pub(crate) fn format(parsed: &ParsedFile<'_>) -> String {
    let format_ir = ir::build(&parsed.cst, parsed.comments);
    let printed = printer::print(&format_ir);
    emit::emit(printed)
}
