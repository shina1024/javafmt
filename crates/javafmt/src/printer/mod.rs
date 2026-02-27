use crate::ir::FormatIr;
use crate::lexer::{Token, TokenKind};

#[derive(Debug, Clone)]
pub struct PrintedDoc {
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    Normal,
    Switch,
    TypeBody,
    Inline,
}

#[derive(Debug, Clone, Copy)]
struct BlockFrame {
    multiline: bool,
    kind: BlockKind,
    in_switch_case: bool,
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
    let mut block_stack: Vec<BlockFrame> = Vec::new();
    let mut prev_text: Option<String> = None;
    let mut prev_kind: Option<TokenKind> = None;
    let mut annotation_active = false;
    let mut annotation_paren_depth = 0usize;
    let mut pending_switch_block = false;
    let mut pending_type_block = false;
    let mut pending_non_sealed = false;
    let mut lambda_continuation_active = false;
    let mut lambda_continuation_base_indent = 0usize;
    let mut lambda_continuation_base_block_depth = 0usize;
    let mut generic_depth = 0usize;

    while i < tokens.len() {
        let token = tokens[i];
        match token.kind {
            TokenKind::LineComment => {
                if i > 0
                    && at_line_start
                    && out.ends_with('\n')
                    && token_text(ir.source, tokens[i - 1]) == "{"
                    && !source_gap_has_newline(ir.source, tokens[i - 1].end, token.start)
                {
                    out.pop();
                    at_line_start = false;
                }
                ensure_space(&mut out, at_line_start);
                let normalized_comment = normalize_line_comment_text(token_text(ir.source, token));
                let current_indent = active_indent(indent, &block_stack);
                write_with_indent(
                    &mut out,
                    &mut at_line_start,
                    current_indent,
                    &normalized_comment,
                );
                out.push('\n');
                at_line_start = true;
                prev_text = Some(String::from("//"));
                prev_kind = Some(TokenKind::LineComment);
                i += 1;
            }
            TokenKind::BlockComment => {
                let attach_after_statement = i > 0
                    && at_line_start
                    && out.ends_with('\n')
                    && token_text(ir.source, tokens[i - 1]) == ";"
                    && !source_gap_has_newline(ir.source, tokens[i - 1].end, token.start);
                if attach_after_statement {
                    out.pop();
                    at_line_start = false;
                }
                ensure_space(&mut out, at_line_start);
                let text = token_text(ir.source, token);
                let current_indent = active_indent(indent, &block_stack);
                write_with_indent(&mut out, &mut at_line_start, current_indent, text);
                if text.ends_with('\n') {
                    at_line_start = true;
                } else if attach_after_statement {
                    out.push('\n');
                    at_line_start = true;
                }
                prev_text = Some(String::from("/*"));
                prev_kind = Some(TokenKind::BlockComment);
                i += 1;
            }
            TokenKind::Word => {
                let word = token_text(ir.source, token);
                let suppress_space_for_non_sealed = pending_non_sealed && word == "sealed";
                pending_non_sealed = false;
                if is_switch_label(word, &block_stack) {
                    if let Some(frame) = block_stack.last_mut() {
                        frame.in_switch_case = false;
                    }
                    if !at_line_start {
                        out.push('\n');
                        at_line_start = true;
                    }
                    let current_indent = active_indent(indent, &block_stack);
                    write_with_indent(&mut out, &mut at_line_start, current_indent, word);
                    i += 1;

                    while i < tokens.len() {
                        let label_token = tokens[i];
                        if label_token.kind == TokenKind::Symbol {
                            let (symbol, consumed) = read_symbol(tokens.as_slice(), i, ir.source);
                            if symbol == ":" {
                                write_with_indent(
                                    &mut out,
                                    &mut at_line_start,
                                    current_indent,
                                    ":",
                                );
                                out.push('\n');
                                at_line_start = true;
                                if let Some(frame) = block_stack.last_mut() {
                                    frame.in_switch_case = true;
                                }
                                prev_text = Some(String::from(":"));
                                prev_kind = Some(TokenKind::Symbol);
                                i += consumed;
                                break;
                            }
                            if symbol == "->" {
                                ensure_space(&mut out, at_line_start);
                                write_with_indent(
                                    &mut out,
                                    &mut at_line_start,
                                    current_indent,
                                    "->",
                                );
                                out.push(' ');
                                prev_text = Some(String::from("->"));
                                prev_kind = Some(TokenKind::Symbol);
                                i += consumed;
                                break;
                            }
                            if symbol == "," {
                                write_with_indent(
                                    &mut out,
                                    &mut at_line_start,
                                    current_indent,
                                    ",",
                                );
                                out.push(' ');
                                i += consumed;
                                continue;
                            }

                            ensure_space(&mut out, at_line_start);
                            write_with_indent(
                                &mut out,
                                &mut at_line_start,
                                current_indent,
                                &symbol,
                            );
                            i += consumed;
                            continue;
                        }

                        ensure_space(&mut out, at_line_start);
                        write_with_indent(
                            &mut out,
                            &mut at_line_start,
                            current_indent,
                            token_text(ir.source, label_token),
                        );
                        i += 1;
                    }

                    continue;
                }

                if !suppress_space_for_non_sealed
                    && !is_tight_after_generic_close(
                        prev_text.as_deref(),
                        tokens.as_slice(),
                        i,
                        ir.source,
                    )
                    && needs_space_before(&prev_text, word, at_line_start)
                {
                    ensure_space(&mut out, at_line_start);
                }
                let current_indent = active_indent(indent, &block_stack);
                write_with_indent(&mut out, &mut at_line_start, current_indent, word);
                prev_text = Some(word.to_owned());
                prev_kind = Some(TokenKind::Word);
                if word == "switch" {
                    pending_switch_block = true;
                }
                if is_type_declaration_keyword(word) {
                    pending_type_block = true;
                }
                i += 1;

                if annotation_active && annotation_paren_depth == 0 {
                    let next = next_symbol_text(tokens.as_slice(), i, ir.source);
                    if next.as_deref() != Some(".") && next.as_deref() != Some("(") {
                        out.push('\n');
                        at_line_start = true;
                        annotation_active = false;
                    }
                }
            }
            TokenKind::Symbol => {
                let (symbol, consumed) = read_symbol(tokens.as_slice(), i, ir.source);
                let next_text = next_symbol_text(tokens.as_slice(), i + consumed, ir.source);

                match symbol.as_str() {
                    "@" => {
                        if !at_line_start {
                            out.push('\n');
                            at_line_start = true;
                        }
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, "@");
                        annotation_active = true;
                        annotation_paren_depth = 0;
                    }
                    "{" => {
                        if needs_space_before(&prev_text, "{", at_line_start) {
                            ensure_space(&mut out, at_line_start);
                        }
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, "{");

                        let is_empty = next_text.as_deref() == Some("}");
                        let inline_brace = prev_text.as_deref() == Some("]") && !is_empty;
                        let kind = if pending_switch_block {
                            BlockKind::Switch
                        } else if pending_type_block {
                            BlockKind::TypeBody
                        } else if inline_brace {
                            BlockKind::Inline
                        } else {
                            BlockKind::Normal
                        };
                        pending_switch_block = false;
                        pending_type_block = false;
                        block_stack.push(BlockFrame {
                            multiline: !is_empty && !inline_brace,
                            kind,
                            in_switch_case: false,
                        });
                        if !is_empty && !inline_brace {
                            out.push('\n');
                            at_line_start = true;
                            indent += 1;
                        }
                    }
                    "}" => {
                        let frame = block_stack.pop().unwrap_or(BlockFrame {
                            multiline: true,
                            kind: BlockKind::Normal,
                            in_switch_case: false,
                        });
                        if frame.multiline {
                            indent = indent.saturating_sub(1);
                            if !at_line_start {
                                out.push('\n');
                                at_line_start = true;
                            }
                        }

                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, "}");

                        let parent_is_type_body = block_stack
                            .last()
                            .is_some_and(|parent| parent.kind == BlockKind::TypeBody);
                        if next_text.as_deref() == Some(";") {
                            // keep same line for "};"
                        } else if frame.kind == BlockKind::Inline {
                            // keep inline initializers on one line
                        } else if matches!(next_text.as_deref(), Some("else" | "catch" | "finally"))
                        {
                            out.push(' ');
                        } else if next_text.as_deref() == Some("while") {
                            out.push(' ');
                        } else if next_text.is_some() {
                            out.push('\n');
                            at_line_start = true;
                            if parent_is_type_body && next_text.as_deref() != Some("}") {
                                out.push('\n');
                            }
                        }
                        if lambda_continuation_active
                            && block_stack.len() < lambda_continuation_base_block_depth
                        {
                            indent = lambda_continuation_base_indent;
                            lambda_continuation_active = false;
                        }
                    }
                    ";" => {
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, ";");
                        pending_switch_block = false;
                        pending_type_block = false;
                        generic_depth = 0;
                        if lambda_continuation_active
                            && block_stack.len() == lambda_continuation_base_block_depth
                        {
                            indent = lambda_continuation_base_indent;
                            lambda_continuation_active = false;
                        }
                        if paren_depth == 0 {
                            out.push('\n');
                            at_line_start = true;
                            let in_type_body = block_stack
                                .last()
                                .is_some_and(|frame| frame.kind == BlockKind::TypeBody);
                            if in_type_body
                                && next_text.as_deref() != Some("}")
                                && next_member_looks_like_method(
                                    tokens.as_slice(),
                                    i + consumed,
                                    ir.source,
                                )
                            {
                                out.push('\n');
                            }
                        } else {
                            out.push(' ');
                        }
                    }
                    "," => {
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, ",");
                        out.push(' ');
                    }
                    "(" => {
                        if needs_space_before_open_paren(&prev_text) {
                            ensure_space(&mut out, at_line_start);
                        }
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, "(");
                        paren_depth += 1;
                        if annotation_active {
                            annotation_paren_depth += 1;
                        }
                    }
                    ")" => {
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, ")");
                        paren_depth = paren_depth.saturating_sub(1);
                        if annotation_active && annotation_paren_depth > 0 {
                            annotation_paren_depth -= 1;
                        }
                    }
                    "." | "[" | "]" | "::" => {
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, &symbol);
                    }
                    "++" | "--" => {
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, &symbol);
                    }
                    "+" => {
                        if is_unary_prefix_context(prev_text.as_deref()) {
                            if needs_space_before(&prev_text, "+", at_line_start) {
                                ensure_space(&mut out, at_line_start);
                            }
                            let current_indent = active_indent(indent, &block_stack);
                            write_with_indent(&mut out, &mut at_line_start, current_indent, "+");
                        } else {
                            ensure_space(&mut out, at_line_start);
                            let current_indent = active_indent(indent, &block_stack);
                            write_with_indent(
                                &mut out,
                                &mut at_line_start,
                                current_indent,
                                &symbol,
                            );
                            out.push(' ');
                        }
                    }
                    "=" => {
                        ensure_space(&mut out, at_line_start);
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, "=");
                        if starts_block_lambda_rhs(tokens.as_slice(), i + consumed, ir.source) {
                            out.push('\n');
                            at_line_start = true;
                            if !lambda_continuation_active {
                                lambda_continuation_active = true;
                                lambda_continuation_base_indent = indent;
                                lambda_continuation_base_block_depth = block_stack.len();
                                indent += 2;
                            }
                        } else {
                            out.push(' ');
                        }
                    }
                    "-" => {
                        if prev_text.as_deref() == Some("non")
                            && next_text.as_deref() == Some("sealed")
                        {
                            let current_indent = active_indent(indent, &block_stack);
                            write_with_indent(&mut out, &mut at_line_start, current_indent, "-");
                            pending_non_sealed = true;
                        } else if is_unary_prefix_context(prev_text.as_deref()) {
                            if needs_space_before(&prev_text, "-", at_line_start) {
                                ensure_space(&mut out, at_line_start);
                            }
                            let current_indent = active_indent(indent, &block_stack);
                            write_with_indent(&mut out, &mut at_line_start, current_indent, "-");
                        } else {
                            ensure_space(&mut out, at_line_start);
                            let current_indent = active_indent(indent, &block_stack);
                            write_with_indent(
                                &mut out,
                                &mut at_line_start,
                                current_indent,
                                &symbol,
                            );
                            out.push(' ');
                        }
                    }
                    "!" | "~" => {
                        if needs_space_before(&prev_text, &symbol, at_line_start) {
                            ensure_space(&mut out, at_line_start);
                        }
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, &symbol);
                    }
                    "?" | ":" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "=="
                    | "!=" | "<=" | ">=" | "&&" | "||" | "*" | "/" | "%" | "&" | "|" | "^"
                    | "->" => {
                        ensure_space(&mut out, at_line_start);
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, &symbol);
                        out.push(' ');
                    }
                    "<" => {
                        let generic_like = paren_depth == 0
                            && is_generic_open_angle(
                                tokens.as_slice(),
                                i,
                                ir.source,
                                &prev_kind,
                                prev_text.as_deref(),
                                next_text.as_deref(),
                                at_line_start,
                            );
                        if generic_like {
                            let current_indent = active_indent(indent, &block_stack);
                            write_with_indent(
                                &mut out,
                                &mut at_line_start,
                                current_indent,
                                &symbol,
                            );
                            generic_depth += 1;
                        } else {
                            ensure_space(&mut out, at_line_start);
                            let current_indent = active_indent(indent, &block_stack);
                            write_with_indent(
                                &mut out,
                                &mut at_line_start,
                                current_indent,
                                &symbol,
                            );
                            out.push(' ');
                        }
                    }
                    ">" => {
                        if generic_depth >= 1 {
                            let current_indent = active_indent(indent, &block_stack);
                            write_with_indent(&mut out, &mut at_line_start, current_indent, ">");
                            generic_depth -= 1;
                        } else {
                            ensure_space(&mut out, at_line_start);
                            let current_indent = active_indent(indent, &block_stack);
                            write_with_indent(&mut out, &mut at_line_start, current_indent, ">");
                            out.push(' ');
                        }
                    }
                    ">>" => {
                        if generic_depth >= 2 {
                            let current_indent = active_indent(indent, &block_stack);
                            write_with_indent(&mut out, &mut at_line_start, current_indent, ">>");
                            generic_depth -= 2;
                        } else {
                            ensure_space(&mut out, at_line_start);
                            let current_indent = active_indent(indent, &block_stack);
                            write_with_indent(&mut out, &mut at_line_start, current_indent, ">>");
                            out.push(' ');
                        }
                    }
                    ">>>" => {
                        if generic_depth >= 3 {
                            let current_indent = active_indent(indent, &block_stack);
                            write_with_indent(&mut out, &mut at_line_start, current_indent, ">>>");
                            generic_depth -= 3;
                        } else {
                            ensure_space(&mut out, at_line_start);
                            let current_indent = active_indent(indent, &block_stack);
                            write_with_indent(&mut out, &mut at_line_start, current_indent, ">>>");
                            out.push(' ');
                        }
                    }
                    "<<" | "<<=" | ">>=" | ">>>=" => {
                        ensure_space(&mut out, at_line_start);
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, &symbol);
                        out.push(' ');
                    }
                    _ => {
                        if needs_space_before(&prev_text, &symbol, at_line_start) {
                            ensure_space(&mut out, at_line_start);
                        }
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, &symbol);
                    }
                }

                prev_text = Some(symbol.clone());
                prev_kind = Some(TokenKind::Symbol);
                i += consumed;

                if annotation_active && annotation_paren_depth == 0 {
                    if symbol != "@" && symbol != "." {
                        let next = next_symbol_text(tokens.as_slice(), i, ir.source);
                        if next.as_deref() != Some(".") && next.as_deref() != Some("(") {
                            out.push('\n');
                            at_line_start = true;
                            annotation_active = false;
                        }
                    }
                }
            }
            TokenKind::StringLiteral | TokenKind::CharLiteral => {
                let text = token_text(ir.source, token);
                if needs_space_before(&prev_text, text, at_line_start) {
                    ensure_space(&mut out, at_line_start);
                }
                let current_indent = active_indent(indent, &block_stack);
                write_with_indent(&mut out, &mut at_line_start, current_indent, text);
                prev_text = Some(text.to_owned());
                prev_kind = Some(token.kind);
                i += 1;
            }
            TokenKind::Whitespace | TokenKind::Newline => {
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

fn active_indent(base_indent: usize, stack: &[BlockFrame]) -> usize {
    match stack.last() {
        Some(frame) if frame.kind == BlockKind::Switch && frame.in_switch_case => base_indent + 1,
        _ => base_indent,
    }
}

fn is_switch_label(word: &str, stack: &[BlockFrame]) -> bool {
    matches!(word, "case" | "default")
        && stack
            .last()
            .is_some_and(|frame| frame.kind == BlockKind::Switch)
}

fn is_type_declaration_keyword(word: &str) -> bool {
    matches!(word, "class" | "interface" | "enum" | "record")
}

fn collect_meaningful_tokens<'a>(ir: &'a FormatIr<'a>) -> Vec<&'a Token> {
    ir.tokens
        .iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline))
        .collect::<Vec<_>>()
}

fn starts_block_lambda_rhs(tokens: &[&Token], mut index: usize, source: &str) -> bool {
    let mut local_paren_depth = 0usize;
    while index < tokens.len() {
        let token = tokens[index];
        if token.kind == TokenKind::Symbol {
            let (symbol, consumed) = read_symbol(tokens, index, source);
            match symbol.as_str() {
                ";" if local_paren_depth == 0 => return false,
                "(" => {
                    local_paren_depth += 1;
                }
                ")" => {
                    local_paren_depth = local_paren_depth.saturating_sub(1);
                }
                "->" if local_paren_depth == 0 => {
                    return next_symbol_text(tokens, index + consumed, source).as_deref()
                        == Some("{");
                }
                _ => {}
            }
            index += consumed;
        } else {
            index += 1;
        }
    }
    false
}

fn next_member_looks_like_method(tokens: &[&Token], mut index: usize, source: &str) -> bool {
    let mut local_paren_depth = 0usize;
    let mut saw_signature_paren = false;

    while index < tokens.len() {
        let token = tokens[index];
        if token.kind == TokenKind::Symbol {
            let (symbol, consumed) = read_symbol(tokens, index, source);
            match symbol.as_str() {
                "(" => {
                    local_paren_depth += 1;
                    if local_paren_depth == 1 {
                        saw_signature_paren = true;
                    }
                }
                ")" => {
                    local_paren_depth = local_paren_depth.saturating_sub(1);
                }
                "=" if local_paren_depth == 0 => return false,
                "{" | ";" | "}" if local_paren_depth == 0 => return saw_signature_paren,
                _ => {}
            }
            index += consumed;
        } else {
            index += 1;
        }
    }

    false
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
    let contiguous12 = tokens
        .get(index + 1)
        .is_some_and(|next| tokens[index].end == next.start);
    let contiguous23 = tokens
        .get(index + 2)
        .is_some_and(|next| tokens[index + 1].end == next.start);

    let combined3 = match (first, second, third) {
        (">", Some(">"), Some(">")) if contiguous12 && contiguous23 => Some(">>>"),
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
        ("=", Some("=")) if contiguous12 => Some("=="),
        ("!", Some("=")) if contiguous12 => Some("!="),
        ("<", Some("=")) if contiguous12 => Some("<="),
        (">", Some("=")) if contiguous12 => Some(">="),
        ("+", Some("+")) if contiguous12 => Some("++"),
        ("-", Some("-")) if contiguous12 => Some("--"),
        ("&", Some("&")) if contiguous12 => Some("&&"),
        ("|", Some("|")) if contiguous12 => Some("||"),
        ("+", Some("=")) if contiguous12 => Some("+="),
        ("-", Some("=")) if contiguous12 => Some("-="),
        ("*", Some("=")) if contiguous12 => Some("*="),
        ("/", Some("=")) if contiguous12 => Some("/="),
        ("%", Some("=")) if contiguous12 => Some("%="),
        ("&", Some("=")) if contiguous12 => Some("&="),
        ("|", Some("=")) if contiguous12 => Some("|="),
        ("^", Some("=")) if contiguous12 => Some("^="),
        ("<", Some("<")) if contiguous12 => Some("<<"),
        (">", Some(">")) if contiguous12 => Some(">>"),
        ("-", Some(">")) if contiguous12 => Some("->"),
        (":", Some(":")) if contiguous12 => Some("::"),
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

    if matches!(
        prev.as_str(),
        "(" | "[" | "{" | "." | "@" | "::" | "<" | "<<" | "+" | "-" | "!" | "~"
    ) {
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
        "if" | "for" | "while" | "switch" | "catch" | "synchronized" | "try"
    )
}

fn is_generic_open_angle(
    tokens: &[&Token],
    index: usize,
    source: &str,
    prev_kind: &Option<TokenKind>,
    prev_text: Option<&str>,
    next_text: Option<&str>,
    at_line_start: bool,
) -> bool {
    if !looks_like_type_argument_list(tokens, index, source) {
        return false;
    }

    if at_line_start || matches!(prev_text, Some("." | "<" | "," | "?" | "&" | "new")) {
        return true;
    }

    if matches!(
        prev_kind,
        Some(TokenKind::Word | TokenKind::StringLiteral | TokenKind::CharLiteral)
    ) && matches!(next_text, Some(text) if is_word_like_text(text) || matches!(text, "?" | ">" | "," | "[" | "]"))
    {
        return true;
    }
    if matches!(prev_text, Some("<" | "," | "?"))
        && matches!(next_text, Some(text) if is_word_like_text(text) || matches!(text, "?" | ">" | "," | "[" | "]"))
    {
        return true;
    }
    false
}

fn looks_like_type_argument_list(tokens: &[&Token], mut index: usize, source: &str) -> bool {
    let mut depth = 1usize;
    let mut saw_type_token = false;
    index += 1;

    while index < tokens.len() {
        let token = tokens[index];
        match token.kind {
            TokenKind::Word => {
                let word = token_text(source, token);
                if !matches!(word, "extends" | "super") {
                    saw_type_token = true;
                }
                index += 1;
            }
            TokenKind::Symbol => {
                let (symbol, consumed) = read_symbol(tokens, index, source);
                match symbol.as_str() {
                    "<" => depth += 1,
                    ">" => {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            return saw_type_token;
                        }
                    }
                    ">>" => {
                        if depth == 0 {
                            return false;
                        }
                        if depth <= 2 {
                            return saw_type_token;
                        }
                        depth -= 2;
                    }
                    ">>>" => {
                        if depth == 0 {
                            return false;
                        }
                        if depth <= 3 {
                            return saw_type_token;
                        }
                        depth -= 3;
                    }
                    "," | "." | "?" | "[" | "]" | "&" => {}
                    ";" | "(" | ")" | "{" | "}" | "=" | "==" | "!=" | "<=" | ">=" | "+" | "-"
                    | "*" | "/" | "%" | "&&" | "||" | ":" | "->" => return false,
                    _ => return false,
                }
                index += consumed;
            }
            TokenKind::Whitespace | TokenKind::Newline => {
                index += 1;
            }
            _ => return false,
        }
    }

    false
}

fn is_tight_after_generic_close(
    prev_text: Option<&str>,
    tokens: &[&Token],
    word_index: usize,
    source: &str,
) -> bool {
    if !matches!(prev_text, Some(">" | ">>" | ">>>")) {
        return false;
    }
    next_symbol_text(tokens, word_index + 1, source).as_deref() == Some("(")
}

fn is_word_like_text(text: &str) -> bool {
    text.chars().all(|ch| ch.is_alphanumeric() || ch == '_')
}

fn is_unary_prefix_context(prev_text: Option<&str>) -> bool {
    matches!(
        prev_text,
        None | Some(
            "(" | "["
                | "{"
                | ","
                | ";"
                | ":"
                | "?"
                | "="
                | "+="
                | "-="
                | "*="
                | "/="
                | "%="
                | "&="
                | "|="
                | "^="
                | "=="
                | "!="
                | "<="
                | ">="
                | "&&"
                | "||"
                | "+"
                | "-"
                | "*"
                | "/"
                | "%"
                | "&"
                | "|"
                | "^"
                | "<"
                | ">"
                | "<<"
                | ">>"
                | ">>>"
                | "<<="
                | ">>="
                | ">>>="
                | "!"
                | "~"
                | "->"
                | "return"
                | "throw"
                | "yield"
                | "case"
        )
    )
}

fn source_gap_has_newline(source: &str, start: usize, end: usize) -> bool {
    source[start..end].contains('\n')
}

fn normalize_line_comment_text(text: &str) -> String {
    if let Some(content) = text.strip_prefix("//")
        && !content.is_empty()
        && !content.starts_with(' ')
    {
        return format!("// {content}");
    }
    text.to_owned()
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
            "class A {\n  @Override\n  public String toString() { // x\n    return \"x\";\n  }\n}\n"
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

    #[test]
    fn keeps_operator_spacing_inside_parentheses() {
        let source = "class A{int f(int n){for(int i=0;i<n;i++){if(i%2==0){continue;}else{break;}}return n;}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("i < n"));
        assert!(printed.text.contains("i % 2 == 0"));
    }

    #[test]
    fn joins_catch_and_finally() {
        let source = "class A{void f(){try{foo();}catch(Exception e){bar();}finally{baz();}}void foo(){}void bar(){}void baz(){}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("} catch (Exception e) {"));
        assert!(printed.text.contains("} finally {"));
    }

    #[test]
    fn formats_switch_case_labels() {
        let source = "class A{void f(){switch(x){case 1:foo();break;default:bar();}}int x;void foo(){}void bar(){}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("case 1:\n"));
        assert!(printed.text.contains("default:\n"));
    }

    #[test]
    fn keeps_non_sealed_keyword() {
        let source = "class A{non-sealed class B{}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("non-sealed class B"));
    }

    #[test]
    fn adds_blank_line_between_block_members_in_type_body() {
        let source = "class A{void f(){}void g(){}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("void f() {}\n\n  void g() {}"));
    }

    #[test]
    fn breaks_block_lambda_after_assignment() {
        let source = "class A{void f(){Runnable r=()->{x();};}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("Runnable r =\n"));
        assert!(printed.text.contains("() -> {\n"));
    }

    #[test]
    fn does_not_merge_binary_plus_and_unary_plus_into_increment() {
        let source = "class A{int f(int x){return -x + +x + ~x;}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("return -x + +x + ~x;"));
        assert!(!printed.text.contains("x++"));
    }

    #[test]
    fn formats_switch_arrow_labels() {
        let source = "class A{void f(int x){switch(x){case 1->System.out.println(1);default->{System.out.println(0);}}}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("case 1 -> System.out.println(1);"));
        assert!(printed.text.contains("default -> {\n"));
    }

    #[test]
    fn formats_try_with_resources_parentheses_spacing() {
        let source = "class A{void f(){try(var in=new java.io.ByteArrayInputStream(new byte[0])){in.read();}catch(java.io.IOException e){throw new RuntimeException(e);}}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("try (var in ="));
    }

    #[test]
    fn keeps_do_while_on_single_line_join() {
        let source =
            "class A{void f(){do{x();}while(cond());}void x(){}boolean cond(){return true;}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("} while (cond());"));
    }

    #[test]
    fn keeps_array_initializer_inline_for_short_literal() {
        let source = "class A{void f(){int[] a=new int[]{1,2,3};}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("new int[] {1, 2, 3};"));
    }

    #[test]
    fn keeps_generic_method_invocation_without_extra_spaces() {
        let source = "class A{void f(){this.<String>m(\"x\");} <T> void m(T t){}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("this.<String>m(\"x\");"));
        assert!(printed.text.contains("<T> void m(T t) {}"));
    }

    #[test]
    fn spaces_shift_operators_as_binary_ops() {
        let source = "class A{void f(){int x=a>>b>>>c<<d;}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("int x = a >> b >>> c << d;"));
    }

    #[test]
    fn keeps_switch_multi_labels_comma_spacing() {
        let source = "class A{int f(int x){return switch(x){case 1,2->1;default->0;};}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("case 1, 2 -> 1;"));
    }

    #[test]
    fn attaches_line_comment_after_open_brace() {
        let source = "class A{void f(){//@x\nx();}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("void f() { // @x"));
    }

    #[test]
    fn attaches_block_comment_to_previous_statement_when_inline() {
        let source = "class A{void f(){x();/*y*/z();}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("x(); /*y*/\n"));
        assert!(printed.text.contains("z();"));
    }
}
