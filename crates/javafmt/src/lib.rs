pub mod comments;
pub mod cst;
pub mod emit;
pub mod ir;
pub mod lexer;
pub mod parser;
pub mod printer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatResult {
    pub output: String,
    pub changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineEnding {
    Lf,
    Crlf,
}

pub fn format_str(input: &str) -> FormatResult {
    let line_ending = detect_line_ending(input);
    let normalized = normalize_newlines(input);
    let lexed = lexer::lex(&normalized);
    let cst = parser::parse(&lexed);
    let attachments = comments::attach(&cst, &lexed);
    let format_ir = ir::build(&cst, attachments);
    let printed = printer::print(&format_ir);
    let output = apply_line_ending_policy(emit::emit(printed), line_ending);
    let changed = output != input;
    FormatResult { output, changed }
}

fn detect_line_ending(input: &str) -> LineEnding {
    let bytes = input.as_bytes();
    let mut i = 0usize;
    let mut crlf_count = 0usize;
    let mut other_newline_count = 0usize;

    while i < bytes.len() {
        if bytes[i] == b'\r' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                crlf_count += 1;
                i += 2;
            } else {
                other_newline_count += 1;
                i += 1;
            }
            continue;
        }
        if bytes[i] == b'\n' {
            other_newline_count += 1;
        }
        i += 1;
    }

    if crlf_count > 0 && other_newline_count == 0 {
        LineEnding::Crlf
    } else {
        LineEnding::Lf
    }
}

fn apply_line_ending_policy(input: String, line_ending: LineEnding) -> String {
    if line_ending == LineEnding::Lf || !input.contains('\n') {
        return input;
    }

    let mut out = String::with_capacity(input.len() + input.matches('\n').count());
    for ch in input.chars() {
        if ch == '\n' {
            out.push('\r');
        }
        out.push(ch);
    }
    out
}

fn normalize_newlines(input: &str) -> String {
    if !input.contains('\r') {
        return input.to_owned();
    }

    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\r' {
            if matches!(chars.peek(), Some('\n')) {
                chars.next();
            }
            out.push('\n');
            continue;
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::format_str;

    #[test]
    fn keeps_formatted_text_unchanged() {
        let input = "class A {}\n";
        let result = format_str(input);
        assert_eq!(result.output, input);
        assert!(!result.changed);
    }

    #[test]
    fn preserves_crlf_if_input_uses_crlf() {
        let input = "class A {}\r\n";
        let result = format_str(input);
        assert_eq!(result.output, "class A {}\r\n");
        assert!(!result.changed);
    }

    #[test]
    fn mixed_newlines_fall_back_to_lf_output() {
        let input = "class A {\r\n}\n";
        let result = format_str(input);
        assert!(!result.output.contains('\r'));
        assert!(result.output.ends_with('\n'));
        assert!(result.changed);
    }

    #[test]
    fn keeps_text_block_intact() {
        let input = "class A{String f(){return \"\"\"\nline1\nline2\n\"\"\";}}\n";
        let result = format_str(input);
        assert!(result.output.contains("\"\"\"\nline1\nline2\n\"\"\""));
    }

    #[test]
    fn appends_trailing_newline() {
        let input = "class A {}";
        let result = format_str(input);
        assert_eq!(result.output, "class A {}\n");
        assert!(result.changed);
    }

    #[test]
    fn trims_trailing_whitespace() {
        let input = "class A {}   \n";
        let result = format_str(input);
        assert_eq!(result.output, "class A {}\n");
        assert!(result.changed);
    }
}
