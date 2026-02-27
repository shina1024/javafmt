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

pub fn format_str(input: &str) -> FormatResult {
    let output = normalize_newlines(input);
    let output = ensure_trailing_newline(output);
    let changed = output != input;
    FormatResult { output, changed }
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

fn ensure_trailing_newline(mut input: String) -> String {
    if input.is_empty() || input.ends_with('\n') {
        return input;
    }
    input.push('\n');
    input
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
    fn normalizes_crlf() {
        let input = "class A {}\r\n";
        let result = format_str(input);
        assert_eq!(result.output, "class A {}\n");
        assert!(result.changed);
    }

    #[test]
    fn appends_trailing_newline() {
        let input = "class A {}";
        let result = format_str(input);
        assert_eq!(result.output, "class A {}\n");
        assert!(result.changed);
    }
}
