use super::doc::Doc;
use super::legacy;
use crate::syntax::ParsedFile;

pub(crate) fn format_file(parsed: &ParsedFile<'_>) -> Doc {
    // Keep soft-line primitives compiled into the production path until the
    // structured formatter starts emitting width-sensitive groups.
    let _softline_scaffold = Doc::soft_line();
    let legacy_output = legacy::format(parsed);
    let line_doc = text_to_doc(&legacy_output);
    Doc::group(Doc::concat([Doc::indent(0, line_doc), Doc::Nil]))
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
