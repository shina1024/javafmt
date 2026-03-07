use super::doc::Doc;
use crate::printer::PrintedDoc;
use crate::syntax::ParsedFile;
use crate::{ir, printer};

pub(crate) fn format_doc(parsed: &ParsedFile<'_>) -> Doc {
    let format_ir = ir::build(&parsed.lexed, parsed.comments);
    let printed = printer::print(&format_ir);
    printed_doc_to_doc(printed)
}

fn printed_doc_to_doc(doc: PrintedDoc) -> Doc {
    let normalized = trim_trailing_whitespace(&doc.text);
    if normalized.is_empty() {
        return Doc::Nil;
    }

    let terminated = if normalized.ends_with('\n') {
        normalized
    } else {
        let mut out = String::with_capacity(normalized.len() + 1);
        out.push_str(&normalized);
        out.push('\n');
        out
    };

    text_to_doc(&terminated)
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

fn trim_trailing_whitespace(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for segment in input.split_inclusive('\n') {
        let (line, has_newline) = if let Some(stripped) = segment.strip_suffix('\n') {
            (stripped, true)
        } else {
            (segment, false)
        };
        out.push_str(line.trim_end_matches([' ', '\t']));
        if has_newline {
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::trim_trailing_whitespace;

    #[test]
    fn trims_spaces_before_newline() {
        let output = trim_trailing_whitespace("class A {}   \n");
        assert_eq!(output, "class A {}\n");
    }

    #[test]
    fn keeps_internal_whitespace() {
        let output = trim_trailing_whitespace("a  b\n");
        assert_eq!(output, "a  b\n");
    }
}
