use crate::ir::FormatIr;
use crate::lexer::{Token, TokenKind};

#[derive(Debug, Clone)]
pub struct PrintedDoc {
    pub text: String,
}

pub fn print(ir: &FormatIr<'_>) -> PrintedDoc {
    if !is_supported_subset(ir) {
        return PrintedDoc {
            text: ir.source.to_owned(),
        };
    }

    PrintedDoc {
        text: format_supported_subset(ir),
    }
}

fn is_supported_subset(ir: &FormatIr<'_>) -> bool {
    if ir.line_comment_count > 0 || ir.block_comment_count > 0 {
        return false;
    }

    for token in &ir.tokens {
        match token.kind {
            TokenKind::LineComment | TokenKind::BlockComment => return false,
            TokenKind::Symbol => {
                let symbol = token_text(ir.source, token);
                if !matches!(
                    symbol,
                    "{" | "}"
                        | "("
                        | ")"
                        | ";"
                        | ","
                        | "."
                        | "="
                        | "+"
                        | "-"
                        | "*"
                        | "/"
                        | "?"
                        | ":"
                        | "["
                        | "]"
                ) {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

fn format_supported_subset(ir: &FormatIr<'_>) -> String {
    let mut out = String::with_capacity(ir.source.len());
    let mut indent = 0usize;
    let mut at_line_start = true;
    let mut prev: Option<&Token> = None;
    let mut block_stack: Vec<bool> = Vec::new();
    let mut index = 0usize;
    let meaningful = collect_meaningful_tokens(ir);

    while index < meaningful.len() {
        let token = meaningful[index];
        let text = token_text(ir.source, token);
        let next = meaningful.get(index + 1).copied();

        match text {
            "{" => {
                write_space_before_symbol(&mut out, prev, token, ir.source);
                write_with_indent(&mut out, &mut at_line_start, indent, "{");
                let is_empty_block = next.is_some_and(|tok| token_text(ir.source, tok) == "}");
                block_stack.push(!is_empty_block);
                if is_empty_block {
                    // Keep "{}" on the same line to match GJF behavior for simple empty blocks.
                } else {
                    out.push('\n');
                    at_line_start = true;
                    indent += 1;
                }
            }
            "}" => {
                let multiline = block_stack.pop().unwrap_or(false);
                if multiline {
                    indent = indent.saturating_sub(1);
                    if !at_line_start {
                        out.push('\n');
                        at_line_start = true;
                    }
                    write_with_indent(&mut out, &mut at_line_start, indent, "}");
                    if next.is_some_and(|tok| token_text(ir.source, tok) == ";") {
                        // Keep on same line for "};"
                    } else {
                        out.push('\n');
                        at_line_start = true;
                    }
                } else {
                    write_with_indent(&mut out, &mut at_line_start, indent, "}");
                    if next.is_some_and(|tok| token_text(ir.source, tok) == ";") {
                        // Keep on same line for "};"
                    } else if next.is_some() {
                        out.push('\n');
                        at_line_start = true;
                    }
                }
            }
            ";" => {
                write_with_indent(&mut out, &mut at_line_start, indent, ";");
                out.push('\n');
                at_line_start = true;
            }
            "," => {
                write_with_indent(&mut out, &mut at_line_start, indent, ",");
                out.push(' ');
            }
            "(" => {
                if should_space_before_open_paren(prev, ir.source) {
                    ensure_space(&mut out, at_line_start);
                }
                write_with_indent(&mut out, &mut at_line_start, indent, "(");
            }
            ")" | "." | "[" | "]" => {
                write_with_indent(&mut out, &mut at_line_start, indent, text);
            }
            "=" | "+" | "-" | "*" | "/" | "?" | ":" => {
                ensure_space(&mut out, at_line_start);
                write_with_indent(&mut out, &mut at_line_start, indent, text);
                out.push(' ');
            }
            _ => {
                if should_space_before_token(prev, token, ir.source) {
                    ensure_space(&mut out, at_line_start);
                }
                write_with_indent(&mut out, &mut at_line_start, indent, text);
            }
        }

        prev = Some(token);
        index += 1;
    }

    trim_redundant_blank_lines(&out)
}

fn collect_meaningful_tokens<'a>(ir: &'a FormatIr<'a>) -> Vec<&'a Token> {
    ir.tokens
        .iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline))
        .collect::<Vec<_>>()
}

fn should_space_before_open_paren(prev: Option<&Token>, source: &str) -> bool {
    let Some(prev) = prev else {
        return false;
    };
    if prev.kind != TokenKind::Word {
        return false;
    }
    matches!(
        token_text(source, prev),
        "if" | "for" | "while" | "switch" | "catch" | "synchronized"
    )
}

fn should_space_before_token(prev: Option<&Token>, curr: &Token, source: &str) -> bool {
    let Some(prev) = prev else {
        return false;
    };
    let prev_text = token_text(source, prev);
    let curr_text = token_text(source, curr);

    if matches!(prev_text, "(" | "[" | ".") {
        return false;
    }
    if matches!(curr_text, ")" | "]" | "." | "," | ";") {
        return false;
    }
    matches!(
        (prev.kind, curr.kind),
        (TokenKind::Word, TokenKind::Word)
            | (TokenKind::Word, TokenKind::StringLiteral)
            | (TokenKind::Word, TokenKind::CharLiteral)
            | (TokenKind::StringLiteral, TokenKind::Word)
            | (TokenKind::CharLiteral, TokenKind::Word)
    )
}

fn write_space_before_symbol(out: &mut String, prev: Option<&Token>, curr: &Token, source: &str) {
    if should_space_before_token(prev, curr, source) {
        ensure_space(out, false);
        return;
    }

    if let Some(prev) = prev {
        let prev_text = token_text(source, prev);
        if prev.kind == TokenKind::Word || matches!(prev_text, ")" | "]") {
            ensure_space(out, false);
        }
    }
}

fn write_with_indent(out: &mut String, at_line_start: &mut bool, indent: usize, text: &str) {
    if *at_line_start {
        for _ in 0..indent {
            out.push_str("  ");
        }
        *at_line_start = false;
    }
    out.push_str(text);
}

fn ensure_space(out: &mut String, at_line_start: bool) {
    if at_line_start {
        return;
    }
    if let Some(ch) = out.chars().last() {
        if ch != ' ' && ch != '\n' {
            out.push(' ');
        }
    }
}

fn trim_redundant_blank_lines(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut newline_run = 0usize;
    for ch in input.chars() {
        if ch == '\n' {
            newline_run += 1;
            if newline_run <= 2 {
                out.push(ch);
            }
        } else {
            newline_run = 0;
            out.push(ch);
        }
    }
    out
}

fn token_text<'a>(source: &'a str, token: &Token) -> &'a str {
    &source[token.start..token.end]
}

#[cfg(test)]
mod tests {
    use crate::comments;
    use crate::ir;
    use crate::lexer;
    use crate::parser;
    use crate::printer::print;

    #[test]
    fn formats_simple_class_body() {
        let source = "class A{int add(int a,int b){return a+b;}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert_eq!(
            printed.text,
            "class A {\n  int add(int a, int b) {\n    return a + b;\n  }\n}\n"
        );
    }

    #[test]
    fn falls_back_for_comments() {
        let source = "class A { // keep\n}\n";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert_eq!(printed.text, source);
    }

    #[test]
    fn formats_string_and_char_literals() {
        let source = "class A{String s=\"x\";char c='y';}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert_eq!(
            printed.text,
            "class A {\n  String s = \"x\";\n  char c = 'y';\n}\n"
        );
    }

    #[test]
    fn formats_ternary_expression() {
        let source = "class A{int x(){return true?1:2;}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert_eq!(
            printed.text,
            "class A {\n  int x() {\n    return true ? 1 : 2;\n  }\n}\n"
        );
    }

    #[test]
    fn falls_back_for_angle_brackets() {
        let source = "class A{java.util.List<String> xs;}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert_eq!(printed.text, source);
    }
}
