use crate::printer::PrintedDoc;

pub fn emit(doc: PrintedDoc) -> String {
    let normalized = trim_trailing_whitespace(&doc.text);
    if normalized.is_empty() {
        return String::new();
    }

    if normalized.ends_with('\n') {
        return normalized;
    }

    let mut out = String::with_capacity(normalized.len() + 1);
    out.push_str(&normalized);
    out.push('\n');
    out
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
