use super::doc::Doc;
use crate::syntax::ParsedFile;
use crate::{emit, ir, printer};

pub(crate) fn format_doc(parsed: &ParsedFile<'_>) -> Doc {
    let format_ir = ir::build(&parsed.lexed, parsed.comments);
    let printed = printer::print(&format_ir);
    let fallback_output = emit::emit(printed);
    text_to_doc(&fallback_output)
}

fn text_to_doc(text: &str) -> Doc {
    if text.is_empty() {
        return Doc::Nil;
    }

    let mut docs = Vec::new();
    for segment in text.split_inclusive('\n') {
        let line = segment.strip_suffix('\n').unwrap_or(segment);
        if !line.is_empty() {
            docs.push(Doc::text(line));
        }
        if segment.ends_with('\n') {
            docs.push(Doc::hard_line());
        }
    }

    Doc::concat(docs)
}
