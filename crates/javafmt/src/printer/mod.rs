use crate::ir::FormatIr;
use crate::lexer::{Token, TokenKind};

#[derive(Debug, Clone)]
pub struct PrintedDoc {
    pub text: String,
}

pub fn print(ir: &FormatIr<'_>) -> PrintedDoc {
    PrintedDoc {
        text: format_tokens(ir),
    }
}

fn format_tokens(ir: &FormatIr<'_>) -> String {
    let tokens = collect_meaningful_tokens(ir);
    let mut out = String::with_capacity(ir.source.len() + 16);
    let mut i = 0usize;
    let mut indent = 0usize;
    let mut at_line_start = true;
    let mut paren_depth = 0usize;
    let mut block_stack: Vec<bool> = Vec::new();
    let mut prev_text: Option<String> = None;
    let mut prev_kind: Option<TokenKind> = None;

    while i < tokens.len() {
        let token = tokens[i];
        match token.kind {
            TokenKind::LineComment => {
                ensure_space(&mut out, at_line_start);
                write_with_indent(
                    &mut out,
                    &mut at_line_start,
                    indent,
                    token_text(ir.source, token),
                );
                out.push('\n');
                at_line_start = true;
                prev_text = Some(String::from("//"));
                prev_kind = Some(TokenKind::LineComment);
                i += 1;
            }
            TokenKind::BlockComment => {
                ensure_space(&mut out, at_line_start);
                let text = token_text(ir.source, token);
                write_with_indent(&mut out, &mut at_line_start, indent, text);
                if text.ends_with('\n') {
                    at_line_start = true;
                }
                prev_text = Some(String::from("/*"));
                prev_kind = Some(TokenKind::BlockComment);
                i += 1;
            }
            TokenKind::Symbol => {
                let (symbol, consumed) = read_symbol(tokens.as_slice(), i, ir.source);
                let next_text = next_symbol_text(tokens.as_slice(), i + consumed, ir.source);

                match symbol.as_str() {
                    "{" => {
                        if needs_space_before(&prev_text, "{", at_line_start) {
                            ensure_space(&mut out, at_line_start);
                        }
                        write_with_indent(&mut out, &mut at_line_start, indent, "{");

                        let is_empty = next_text.as_deref() == Some("}");
                        block_stack.push(!is_empty);
                        if !is_empty {
                            out.push('\n');
                            at_line_start = true;
                            indent += 1;
                        }
                    }
                    "}" => {
                        let multiline = block_stack.pop().unwrap_or(true);
                        if multiline {
                            indent = indent.saturating_sub(1);
                            if !at_line_start {
                                out.push('\n');
                                at_line_start = true;
                            }
                        }

                        write_with_indent(&mut out, &mut at_line_start, indent, "}");

                        if next_text.as_deref() == Some(";") {
                            // keep same line for "};"
                        } else if next_text.as_deref() == Some("else") {
                            out.push(' ');
                        } else if next_text.is_some() {
                            out.push('\n');
                            at_line_start = true;
                        }
                    }
                    ";" => {
                        write_with_indent(&mut out, &mut at_line_start, indent, ";");
                        if paren_depth == 0 {
                            out.push('\n');
                            at_line_start = true;
                        } else {
                            out.push(' ');
                        }
                    }
                    "," => {
                        write_with_indent(&mut out, &mut at_line_start, indent, ",");
                        out.push(' ');
                    }
                    "(" => {
                        if needs_space_before_open_paren(&prev_text) {
                            ensure_space(&mut out, at_line_start);
                        }
                        write_with_indent(&mut out, &mut at_line_start, indent, "(");
                        paren_depth += 1;
                    }
                    ")" => {
                        write_with_indent(&mut out, &mut at_line_start, indent, ")");
                        paren_depth = paren_depth.saturating_sub(1);
                    }
                    "[" | "]" | "." | "@" | "::" => {
                        if symbol == "@" && !at_line_start {
                            out.push('\n');
                            at_line_start = true;
                        }
                        write_with_indent(&mut out, &mut at_line_start, indent, &symbol);
                    }
                    "++" | "--" => {
                        write_with_indent(&mut out, &mut at_line_start, indent, &symbol);
                    }
                    "?" | ":" | "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^="
                    | "==" | "!=" | "<=" | ">=" | "&&" | "||" | "+" | "-" | "*" | "/" | "%"
                    | "&" | "|" | "^" | "->" => {
                        ensure_space(&mut out, at_line_start);
                        write_with_indent(&mut out, &mut at_line_start, indent, &symbol);
                        out.push(' ');
                    }
                    "<" | ">" | "<<" | ">>" | ">>>" | "<<=" | ">>=" | ">>>=" => {
                        if is_generic_angle(&prev_kind, prev_text.as_deref(), next_text.as_deref())
                        {
                            write_with_indent(&mut out, &mut at_line_start, indent, &symbol);
                        } else {
                            ensure_space(&mut out, at_line_start);
                            write_with_indent(&mut out, &mut at_line_start, indent, &symbol);
                            out.push(' ');
                        }
                    }
                    _ => {
                        if needs_space_before(&prev_text, &symbol, at_line_start) {
                            ensure_space(&mut out, at_line_start);
                        }
                        write_with_indent(&mut out, &mut at_line_start, indent, &symbol);
                    }
                }

                prev_text = Some(symbol);
                prev_kind = Some(TokenKind::Symbol);
                i += consumed;
            }
            _ => {
                let text = token_text(ir.source, token);
                if needs_space_before(&prev_text, text, at_line_start) {
                    ensure_space(&mut out, at_line_start);
                }
                write_with_indent(&mut out, &mut at_line_start, indent, text);
                prev_text = Some(text.to_owned());
                prev_kind = Some(token.kind);
                i += 1;
            }
        }
    }

    let mut normalized = trim_redundant_blank_lines(&out);
    if !normalized.is_empty() && !normalized.ends_with('\n') {
        normalized.push('\n');
    }
    normalized
}

fn collect_meaningful_tokens<'a>(ir: &'a FormatIr<'a>) -> Vec<&'a Token> {
    ir.tokens
        .iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline))
        .collect::<Vec<_>>()
}

fn next_symbol_text(tokens: &[&Token], mut index: usize, source: &str) -> Option<String> {
    if index >= tokens.len() {
        return None;
    }
    let token = tokens[index];
    if token.kind == TokenKind::Symbol {
        let (text, _) = read_symbol(tokens, index, source);
        return Some(text);
    }
    if matches!(
        token.kind,
        TokenKind::Word | TokenKind::StringLiteral | TokenKind::CharLiteral
    ) {
        return Some(token_text(source, token).to_owned());
    }
    index += 1;
    if index < tokens.len() {
        return Some(token_text(source, tokens[index]).to_owned());
    }
    None
}

fn read_symbol(tokens: &[&Token], index: usize, source: &str) -> (String, usize) {
    let first = token_text(source, tokens[index]);
    let second = tokens.get(index + 1).map(|token| token_text(source, token));
    let third = tokens.get(index + 2).map(|token| token_text(source, token));

    let combined3 = match (first, second, third) {
        (">", Some(">"), Some(">")) => Some(">>>"),
        _ => None,
    };
    if let Some(op) = combined3 {
        let fourth = tokens.get(index + 3).map(|token| token_text(source, token));
        if matches!((op, fourth), (">>>", Some("="))) {
            return (String::from(">>>="), 4);
        }
        return (op.to_owned(), 3);
    }

    let combined2 = match (first, second) {
        ("=", Some("=")) => Some("=="),
        ("!", Some("=")) => Some("!="),
        ("<", Some("=")) => Some("<="),
        (">", Some("=")) => Some(">="),
        ("+", Some("+")) => Some("++"),
        ("-", Some("-")) => Some("--"),
        ("&", Some("&")) => Some("&&"),
        ("|", Some("|")) => Some("||"),
        ("+", Some("=")) => Some("+="),
        ("-", Some("=")) => Some("-="),
        ("*", Some("=")) => Some("*="),
        ("/", Some("=")) => Some("/="),
        ("%", Some("=")) => Some("%="),
        ("&", Some("=")) => Some("&="),
        ("|", Some("=")) => Some("|="),
        ("^", Some("=")) => Some("^="),
        ("<", Some("<")) => Some("<<"),
        (">", Some(">")) => Some(">>"),
        ("-", Some(">")) => Some("->"),
        (":", Some(":")) => Some("::"),
        _ => None,
    };
    if let Some(op) = combined2 {
        let third = tokens.get(index + 2).map(|token| token_text(source, token));
        if matches!((op, third), ("<<", Some("="))) {
            return (String::from("<<="), 3);
        }
        if matches!((op, third), (">>", Some("="))) {
            return (String::from(">>="), 3);
        }
        return (op.to_owned(), 2);
    }

    (first.to_owned(), 1)
}

fn needs_space_before(prev_text: &Option<String>, curr_text: &str, at_line_start: bool) -> bool {
    if at_line_start {
        return false;
    }
    let Some(prev) = prev_text else {
        return false;
    };

    if matches!(prev.as_str(), "(" | "[" | "." | "@" | "::" | "<" | "<<") {
        return false;
    }
    if matches!(curr_text, ")" | "]" | "." | "," | ";" | "::") {
        return false;
    }
    true
}

fn needs_space_before_open_paren(prev_text: &Option<String>) -> bool {
    let Some(prev) = prev_text else {
        return false;
    };
    matches!(
        prev.as_str(),
        "if" | "for" | "while" | "switch" | "catch" | "synchronized"
    )
}

fn is_generic_angle(
    prev_kind: &Option<TokenKind>,
    prev_text: Option<&str>,
    next_text: Option<&str>,
) -> bool {
    if matches!(
        prev_kind,
        Some(TokenKind::Word | TokenKind::StringLiteral | TokenKind::CharLiteral)
    ) && matches!(next_text, Some(text) if is_word_like_text(text) || matches!(text, "?" | ">" | "," | "[" | "]"))
    {
        return true;
    }
    if matches!(prev_text, Some("<" | "," | "?"))
        && matches!(next_text, Some(text) if is_word_like_text(text) || matches!(text, "?" | ">" | "," | "["))
    {
        return true;
    }
    false
}

fn is_word_like_text(text: &str) -> bool {
    text.chars().all(|ch| ch.is_alphanumeric() || ch == '_')
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
    fn formats_annotation_and_comment() {
        let source = "class A{@Override public String toString(){//x\nreturn \"x\";}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert_eq!(
            printed.text,
            "class A {\n  @Override public String toString() {\n    //x\n    return \"x\";\n  }\n}\n"
        );
    }

    #[test]
    fn keeps_generic_without_spaces_around_angle() {
        let source = "class A{java.util.List<String> xs;}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert_eq!(printed.text, "class A {\n  java.util.List<String> xs;\n}\n");
    }
}
