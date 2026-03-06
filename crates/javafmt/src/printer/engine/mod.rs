use crate::ir::FormatIr;
use crate::lexer::{Token, TokenKind};

mod analysis;
mod output;

use analysis::*;
use output::*;

#[derive(Debug, Clone)]
pub struct PrintedDoc {
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    Normal,
    Switch,
    TypeBody,
    EnumBody,
    ModuleBody,
    Initializer,
    Inline,
}

#[derive(Debug, Clone, Copy)]
struct BlockFrame {
    multiline: bool,
    kind: BlockKind,
    in_switch_case: bool,
    in_enum_constants: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TopLevelDirective {
    Package,
    Import,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModuleDirective {
    Requires,
    Exports,
    Opens,
    Uses,
    Provides,
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
    let mut bracket_depth = 0usize;
    let mut block_stack: Vec<BlockFrame> = Vec::new();
    let mut prev_text: Option<String> = None;
    let mut prev_kind: Option<TokenKind> = None;
    let mut annotation_active = false;
    let mut annotation_paren_depth = 0usize;
    let mut annotation_brace_depth = 0usize;
    let mut annotation_has_args = false;
    let mut annotation_name: Option<String> = None;
    let mut annotation_started_line_start = false;
    let mut annotation_inline_run_active = false;
    let mut annotation_multiline_args_active = false;
    let mut annotation_multiline_args_paren_depth = 0usize;
    let mut annotation_multiline_base_indent = 0usize;
    let mut pending_switch_block = false;
    let mut pending_type_block = false;
    let mut pending_enum_block = false;
    let mut pending_module_block = false;
    let mut pending_non_sealed = false;
    let mut lambda_continuation_active = false;
    let mut lambda_continuation_base_indent = 0usize;
    let mut lambda_continuation_base_block_depth = 0usize;
    let mut try_resource_paren_depth = 0usize;
    let mut try_resource_continuation_active = false;
    let mut try_resource_continuation_base_indent = 0usize;
    let mut module_continuation_active = false;
    let mut module_continuation_base_indent = 0usize;
    let mut generic_depth = 0usize;
    let mut ternary_depth = 0usize;
    let mut pending_top_level_directive: Option<TopLevelDirective> = None;
    let mut pending_module_directive: Option<ModuleDirective> = None;
    let mut call_args_continuation_active = false;
    let mut call_args_continuation_paren_depth = 0usize;
    let mut call_args_continuation_base_indent = 0usize;
    let mut call_args_comment_mode = false;
    let mut call_args_vertical_mode = false;
    let mut chain_continuation_active = false;
    let mut chain_continuation_paren_depth = 0usize;
    let mut chain_continuation_bracket_depth = 0usize;
    let mut chain_continuation_dot_count = 0usize;
    let mut chain_keep_leading_dots = 0usize;
    let mut switch_arrow_comment_indent_active = false;

    while i < tokens.len() {
        let token = tokens[i];
        match token.kind {
            TokenKind::LineComment => {
                annotation_inline_run_active = false;
                let arrow_comment = prev_text.as_deref() == Some("->");
                let initializer_comment = prev_text.as_deref() == Some(",")
                    && block_stack
                        .last()
                        .is_some_and(|frame| frame.kind == BlockKind::Initializer);
                if i > 0
                    && at_line_start
                    && out.ends_with('\n')
                    && token_text(ir.source, tokens[i - 1]) == "{"
                    && !source_gap_has_newline(ir.source, tokens[i - 1].end, token.start)
                {
                    out.pop();
                    at_line_start = false;
                }
                if initializer_comment && !at_line_start {
                    out.push('\n');
                    at_line_start = true;
                }
                if arrow_comment && at_line_start && !switch_arrow_comment_indent_active {
                    indent += 2;
                    switch_arrow_comment_indent_active = true;
                }
                ensure_space(&mut out, at_line_start);
                let normalized_comment = normalize_line_comment_text(token_text(ir.source, token));
                let mut current_indent = active_indent(indent, &block_stack);
                let next = next_symbol_text(tokens.as_slice(), i + 1, ir.source);
                if at_line_start
                    && block_stack.last().is_some_and(|frame| {
                        frame.kind == BlockKind::Switch && frame.in_switch_case
                    })
                    && matches!(next.as_deref(), Some("case" | "default"))
                {
                    current_indent = current_indent.saturating_sub(1);
                }
                write_with_indent(
                    &mut out,
                    &mut at_line_start,
                    current_indent,
                    &normalized_comment,
                );
                out.push('\n');
                at_line_start = true;
                if arrow_comment && !switch_arrow_comment_indent_active {
                    indent += 2;
                    switch_arrow_comment_indent_active = true;
                }
                prev_text = Some(String::from("//"));
                prev_kind = Some(TokenKind::LineComment);
                i += 1;
            }
            TokenKind::BlockComment => {
                annotation_inline_run_active = false;
                let started_line_start = at_line_start;
                let attach_after_statement = i > 0
                    && at_line_start
                    && out.ends_with('\n')
                    && token_text(ir.source, tokens[i - 1]) == ";"
                    && !source_gap_has_newline(ir.source, tokens[i - 1].end, token.start);
                let leading_block_comment = i > 0
                    && at_line_start
                    && out.ends_with('\n')
                    && token_text(ir.source, tokens[i - 1]) == "{";
                if attach_after_statement {
                    out.pop();
                    at_line_start = false;
                }
                if prev_text.as_deref() != Some("(") {
                    ensure_space(&mut out, at_line_start);
                }
                let text = normalize_block_comment_text(token_text(ir.source, token));
                let current_indent = active_indent(indent, &block_stack);
                let comment_indent = if leading_block_comment {
                    current_indent + 1
                } else {
                    current_indent
                };
                write_with_indent(&mut out, &mut at_line_start, comment_indent, &text);
                if text.ends_with('\n') {
                    at_line_start = true;
                } else if attach_after_statement
                    || leading_block_comment
                    || (started_line_start && paren_depth == 0)
                {
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
                annotation_inline_run_active = false;
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
                    let mut label_prev_text = Some(word.to_owned());
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
                                let inline_line_comment =
                                    tokens.get(i + consumed).is_some_and(|next| {
                                        next.kind == TokenKind::LineComment
                                            && !source_gap_has_newline(
                                                ir.source,
                                                label_token.end,
                                                next.start,
                                            )
                                    });
                                if inline_line_comment {
                                    out.push(' ');
                                    at_line_start = false;
                                } else {
                                    out.push('\n');
                                    at_line_start = true;
                                }
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
                                let multiline_line_comment =
                                    tokens.get(i + consumed).is_some_and(|next| {
                                        next.kind == TokenKind::LineComment
                                            && source_gap_has_newline(
                                                ir.source,
                                                label_token.end,
                                                next.start,
                                            )
                                    });
                                if multiline_line_comment {
                                    out.push('\n');
                                    at_line_start = true;
                                } else {
                                    out.push(' ');
                                }
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
                                label_prev_text = Some(String::from(","));
                                i += consumed;
                                continue;
                            }
                            match symbol {
                                "." | "[" | "]" | "::" => {
                                    write_with_indent(
                                        &mut out,
                                        &mut at_line_start,
                                        current_indent,
                                        &symbol,
                                    );
                                }
                                "(" => {
                                    if needs_space_before_open_paren(&label_prev_text) {
                                        ensure_space(&mut out, at_line_start);
                                    }
                                    write_with_indent(
                                        &mut out,
                                        &mut at_line_start,
                                        current_indent,
                                        "(",
                                    );
                                }
                                ")" => {
                                    write_with_indent(
                                        &mut out,
                                        &mut at_line_start,
                                        current_indent,
                                        ")",
                                    );
                                }
                                "?" | "==" | "!=" | "<=" | ">=" | "&&" | "||" | "+" | "-" | "*"
                                | "/" | "%" | "<" | ">" | "&" | "|" | "^" => {
                                    ensure_space(&mut out, at_line_start);
                                    write_with_indent(
                                        &mut out,
                                        &mut at_line_start,
                                        current_indent,
                                        &symbol,
                                    );
                                    out.push(' ');
                                }
                                _ => {
                                    ensure_space(&mut out, at_line_start);
                                    write_with_indent(
                                        &mut out,
                                        &mut at_line_start,
                                        current_indent,
                                        &symbol,
                                    );
                                }
                            }
                            label_prev_text = Some(symbol.to_owned());
                            i += consumed;
                            continue;
                        }

                        if needs_space_before(
                            &label_prev_text,
                            token_text(ir.source, label_token),
                            at_line_start,
                        ) {
                            ensure_space(&mut out, at_line_start);
                        }
                        write_with_indent(
                            &mut out,
                            &mut at_line_start,
                            current_indent,
                            token_text(ir.source, label_token),
                        );
                        label_prev_text = Some(token_text(ir.source, label_token).to_owned());
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
                if word == "return"
                    && let Some(metrics) =
                        chain_continuation_metrics(tokens.as_slice(), i + 1, ir.source)
                {
                    chain_continuation_active = true;
                    chain_continuation_paren_depth = paren_depth;
                    chain_continuation_bracket_depth = bracket_depth;
                    chain_continuation_dot_count = 0;
                    chain_keep_leading_dots = metrics.keep_leading_dots;
                }
                if word == "switch" {
                    pending_switch_block = true;
                }
                if word == "enum" {
                    pending_enum_block = true;
                }
                if word == "module" {
                    pending_module_block = true;
                }
                if is_type_declaration_keyword(word) {
                    pending_type_block = true;
                }
                if block_stack.is_empty()
                    && paren_depth == 0
                    && pending_top_level_directive.is_none()
                    && let Some(directive) = top_level_directive_from_word(word)
                {
                    pending_top_level_directive = Some(directive);
                }

                let in_module_body = block_stack
                    .last()
                    .is_some_and(|frame| frame.kind == BlockKind::ModuleBody);
                if in_module_body
                    && paren_depth == 0
                    && pending_module_directive.is_none()
                    && let Some(directive) = module_directive_from_word(word)
                {
                    pending_module_directive = Some(directive);
                }

                if in_module_body && paren_depth == 0 && matches!(word, "to" | "with") {
                    out.push('\n');
                    at_line_start = true;
                    if !module_continuation_active {
                        module_continuation_active = true;
                        module_continuation_base_indent = indent;
                        indent += 2;
                    }
                }
                i += 1;

                if annotation_active && annotation_paren_depth == 0 {
                    let next = next_symbol_text(tokens.as_slice(), i, ir.source);
                    if next.as_deref() != Some(".") && next.as_deref() != Some("(") {
                        let in_type_body = block_stack.last().is_some_and(|frame| {
                            matches!(frame.kind, BlockKind::TypeBody | BlockKind::EnumBody)
                        });
                        let keep_inline = should_keep_inline_annotation(
                            tokens.as_slice(),
                            i,
                            ir.source,
                            in_type_body,
                            block_stack.is_empty() && paren_depth == 0,
                            annotation_name.as_deref(),
                            annotation_started_line_start,
                            annotation_has_args,
                            paren_depth,
                        );
                        annotation_active = false;
                        annotation_brace_depth = 0;
                        annotation_has_args = false;
                        annotation_name = None;
                        annotation_inline_run_active = keep_inline;
                        if annotation_multiline_args_active {
                            indent = annotation_multiline_base_indent;
                            annotation_multiline_args_active = false;
                            annotation_multiline_args_paren_depth = 0;
                        }
                        if !keep_inline && paren_depth == 0 && !at_line_start {
                            out.push('\n');
                            at_line_start = true;
                        }
                    }
                }

                if word == "return"
                    && tokens.get(i).is_some_and(|next| {
                        next.kind == TokenKind::StringLiteral
                            && token_text(ir.source, next).starts_with("\"\"\"")
                    })
                {
                    out.push('\n');
                    at_line_start = true;
                }
            }
            TokenKind::Symbol => {
                let (symbol, consumed) = read_symbol(tokens.as_slice(), i, ir.source);
                let next_text = next_symbol_text(tokens.as_slice(), i + consumed, ir.source);
                if symbol != "@" {
                    annotation_inline_run_active = false;
                }

                match symbol {
                    "@" => {
                        let annotation_declaration =
                            next_symbol_text(tokens.as_slice(), i + consumed, ir.source).as_deref()
                                == Some("interface");
                        let in_type_body = block_stack.last().is_some_and(|frame| {
                            matches!(frame.kind, BlockKind::TypeBody | BlockKind::EnumBody)
                        });
                        if !at_line_start {
                            if paren_depth > 0
                                || annotation_declaration
                                || annotation_inline_run_active
                                || should_start_annotation_inline(
                                    prev_text.as_deref(),
                                    in_type_body,
                                )
                            {
                                if needs_space_before(&prev_text, "@", at_line_start) {
                                    ensure_space(&mut out, at_line_start);
                                }
                            } else {
                                out.push('\n');
                                at_line_start = true;
                            }
                        }
                        annotation_started_line_start = at_line_start;
                        annotation_name =
                            next_symbol_text(tokens.as_slice(), i + consumed, ir.source)
                                .filter(|text| is_word_like_text(text))
                                .map(str::to_owned);
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, "@");
                        if annotation_declaration {
                            annotation_active = false;
                            annotation_paren_depth = 0;
                            annotation_brace_depth = 0;
                            annotation_has_args = false;
                            annotation_name = None;
                            annotation_multiline_args_active = false;
                            annotation_multiline_args_paren_depth = 0;
                        } else {
                            annotation_active = true;
                            annotation_paren_depth = 0;
                            annotation_brace_depth = 0;
                            annotation_has_args = false;
                            annotation_multiline_args_active = false;
                            annotation_multiline_args_paren_depth = 0;
                        }
                    }
                    "{" => {
                        if needs_space_before(&prev_text, "{", at_line_start) {
                            ensure_space(&mut out, at_line_start);
                        }
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, "{");
                        if annotation_active && annotation_paren_depth > 0 {
                            annotation_brace_depth += 1;
                        }

                        let is_empty = next_text.as_deref() == Some("}");
                        let initializer_brace = matches!(prev_text.as_deref(), Some("]" | "="));
                        let inline_brace = (is_inline_initializer_brace(
                            tokens.as_slice(),
                            i,
                            ir.source,
                            &prev_text,
                        ) || (annotation_active
                            && prev_text.as_deref() == Some("(")))
                            && !is_empty;
                        let kind = if pending_switch_block {
                            BlockKind::Switch
                        } else if pending_module_block {
                            BlockKind::ModuleBody
                        } else if pending_enum_block {
                            BlockKind::EnumBody
                        } else if pending_type_block {
                            BlockKind::TypeBody
                        } else if inline_brace {
                            BlockKind::Inline
                        } else if initializer_brace {
                            BlockKind::Initializer
                        } else {
                            BlockKind::Normal
                        };
                        pending_switch_block = false;
                        pending_type_block = false;
                        pending_enum_block = false;
                        pending_module_block = false;
                        block_stack.push(BlockFrame {
                            multiline: !is_empty && !inline_brace,
                            kind,
                            in_switch_case: false,
                            in_enum_constants: kind == BlockKind::EnumBody,
                        });
                        if !is_empty && !inline_brace {
                            out.push('\n');
                            at_line_start = true;
                            indent += 1;
                            if matches!(kind, BlockKind::TypeBody | BlockKind::EnumBody)
                                && tokens.get(i + consumed).is_some_and(|next| {
                                    source_gap_has_blank_line(ir.source, token.end, next.start)
                                })
                            {
                                out.push('\n');
                            }
                        }
                    }
                    "}" => {
                        let frame = block_stack.pop().unwrap_or(BlockFrame {
                            multiline: true,
                            kind: BlockKind::Normal,
                            in_switch_case: false,
                            in_enum_constants: false,
                        });
                        if annotation_active && annotation_brace_depth > 0 {
                            annotation_brace_depth -= 1;
                        }
                        if frame.multiline {
                            indent = indent.saturating_sub(1);
                            if !at_line_start {
                                out.push('\n');
                                at_line_start = true;
                            }
                        }

                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, "}");

                        let parent_is_type_body = block_stack.last().is_some_and(|parent| {
                            matches!(parent.kind, BlockKind::TypeBody | BlockKind::EnumBody)
                        });
                        if next_text.as_deref() == Some(";") {
                            // keep same line for "};"
                        } else if next_text.as_deref() == Some(")") {
                            // keep same line for "})"
                        } else if next_text.as_deref() == Some(",") {
                            // keep same line for "},"
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
                        if frame.kind == BlockKind::ModuleBody {
                            pending_module_directive = None;
                        }
                    }
                    ";" => {
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, ";");
                        pending_switch_block = false;
                        pending_type_block = false;
                        pending_enum_block = false;
                        pending_module_block = false;
                        generic_depth = 0;
                        if switch_arrow_comment_indent_active {
                            indent = indent.saturating_sub(2);
                            switch_arrow_comment_indent_active = false;
                        }
                        if paren_depth == 0 {
                            ternary_depth = 0;
                            chain_continuation_active = false;
                            chain_continuation_paren_depth = 0;
                            chain_continuation_bracket_depth = 0;
                            chain_continuation_dot_count = 0;
                            chain_keep_leading_dots = 0;
                            if call_args_continuation_active {
                                indent = call_args_continuation_base_indent;
                                call_args_continuation_active = false;
                                call_args_continuation_paren_depth = 0;
                                call_args_comment_mode = false;
                                call_args_vertical_mode = false;
                            }
                        }
                        if paren_depth == 0 {
                            if let Some(frame) = block_stack.last_mut() {
                                if frame.kind == BlockKind::EnumBody && frame.in_enum_constants {
                                    frame.in_enum_constants = false;
                                }
                            }
                        }
                        if lambda_continuation_active
                            && block_stack.len() == lambda_continuation_base_block_depth
                        {
                            indent = lambda_continuation_base_indent;
                            lambda_continuation_active = false;
                        }
                        if module_continuation_active && paren_depth == 0 {
                            indent = module_continuation_base_indent;
                            module_continuation_active = false;
                        }
                        if paren_depth == 0 {
                            out.push('\n');
                            at_line_start = true;
                            if tokens.get(i + consumed).is_some_and(|next| {
                                source_gap_has_blank_line(ir.source, token.end, next.start)
                            }) {
                                out.push('\n');
                            }
                            let in_type_body = block_stack.last().is_some_and(|frame| {
                                matches!(frame.kind, BlockKind::TypeBody | BlockKind::EnumBody)
                            });
                            let in_module_body = block_stack
                                .last()
                                .is_some_and(|frame| frame.kind == BlockKind::ModuleBody);
                            let has_blank_line_before_next =
                                tokens.get(i + consumed).is_some_and(|next| {
                                    source_gap_has_blank_line(ir.source, token.end, next.start)
                                });
                            if in_type_body
                                && (has_blank_line_before_next
                                    || (next_text.as_deref() != Some("}")
                                        && next_member_looks_like_method(
                                            tokens.as_slice(),
                                            i + consumed,
                                            ir.source,
                                        )))
                            {
                                out.push('\n');
                            }
                            if in_module_body {
                                if next_text.as_deref() == Some("}") {
                                    pending_module_directive = None;
                                } else {
                                    let next_directive = next_module_directive(
                                        tokens.as_slice(),
                                        i + consumed,
                                        ir.source,
                                    );
                                    if let Some(current_directive) = pending_module_directive.take()
                                    {
                                        if next_directive
                                            .is_some_and(|next| next != current_directive)
                                        {
                                            out.push('\n');
                                        }
                                    }
                                }
                            } else {
                                pending_module_directive = None;
                            }

                            if block_stack.is_empty() {
                                if let Some(current_directive) = pending_top_level_directive.take()
                                {
                                    let next_directive = next_top_level_directive(
                                        tokens.as_slice(),
                                        i + consumed,
                                        ir.source,
                                    );
                                    match current_directive {
                                        TopLevelDirective::Package => {
                                            if next_text.as_deref().is_some_and(|next| next != "}")
                                            {
                                                out.push('\n');
                                            }
                                        }
                                        TopLevelDirective::Import => {
                                            if next_directive != Some(TopLevelDirective::Import)
                                                && next_text
                                                    .as_deref()
                                                    .is_some_and(|next| next != "}")
                                            {
                                                out.push('\n');
                                            }
                                        }
                                    }
                                }
                            } else {
                                pending_top_level_directive = None;
                            }
                        } else {
                            if try_resource_paren_depth > 0
                                && paren_depth == try_resource_paren_depth
                            {
                                if next_text.as_deref() == Some(")") {
                                    out.push(' ');
                                } else {
                                    out.push('\n');
                                    at_line_start = true;
                                    if !try_resource_continuation_active {
                                        try_resource_continuation_active = true;
                                        try_resource_continuation_base_indent = indent;
                                        indent += 2;
                                    }
                                }
                            } else {
                                out.push(' ');
                            }
                        }
                    }
                    "," => {
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, ",");
                        let enum_constants_comma = block_stack.last().is_some_and(|frame| {
                            frame.kind == BlockKind::EnumBody
                                && frame.in_enum_constants
                                && paren_depth == 0
                        });
                        let annotation_args_comma = annotation_multiline_args_active
                            && paren_depth == annotation_multiline_args_paren_depth
                            && annotation_paren_depth == 1
                            && annotation_brace_depth == 0;
                        let call_args_comma = call_args_continuation_active
                            && paren_depth == call_args_continuation_paren_depth
                            && block_stack
                                .last()
                                .is_none_or(|frame| frame.kind != BlockKind::Initializer)
                            && (call_args_comment_mode
                                || call_args_vertical_mode
                                || current_output_line(out.as_str())
                                    .is_some_and(|line| line.chars().count() >= 60));
                        if enum_constants_comma
                            || (module_continuation_active && paren_depth == 0)
                            || annotation_args_comma
                            || call_args_comma
                        {
                            out.push('\n');
                            at_line_start = true;
                        } else {
                            out.push(' ');
                        }
                    }
                    "(" => {
                        if needs_space_before_open_paren(&prev_text) {
                            ensure_space(&mut out, at_line_start);
                        }
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, "(");
                        let explicit_type_argument_call =
                            is_explicit_type_argument_call(tokens.as_slice(), i, ir.source);
                        let wrappable_invocation_open_paren = !explicit_type_argument_call
                            && is_wrappable_invocation_open_paren(
                                tokens.as_slice(),
                                i,
                                ir.source,
                                &prev_kind,
                                prev_text.as_deref(),
                            );
                        let call_argument_metrics = if explicit_type_argument_call
                            || wrappable_invocation_open_paren
                        {
                            call_arguments_break_metrics(tokens.as_slice(), i + consumed, ir.source)
                        } else {
                            None
                        };
                        let should_break_after_open_paren = if explicit_type_argument_call {
                            call_argument_metrics
                                .as_ref()
                                .is_some_and(CallArgumentMetrics::should_break_short)
                        } else {
                            wrappable_invocation_open_paren
                                && call_argument_metrics
                                    .as_ref()
                                    .is_some_and(CallArgumentMetrics::should_break_long)
                        };
                        let dot_continuation_call = current_output_line(out.as_str())
                            .is_some_and(|line| line.trim_start().starts_with('.'));
                        paren_depth += 1;
                        if prev_text.as_deref() == Some("try") {
                            try_resource_paren_depth = paren_depth;
                        }
                        if annotation_active {
                            if annotation_paren_depth == 0 {
                                annotation_has_args = true;
                            }
                            annotation_paren_depth += 1;
                            if annotation_paren_depth == 1
                                && should_break_annotation_arguments(
                                    tokens.as_slice(),
                                    i + consumed,
                                    ir.source,
                                )
                            {
                                out.push('\n');
                                at_line_start = true;
                                if !annotation_multiline_args_active {
                                    annotation_multiline_args_active = true;
                                    annotation_multiline_args_paren_depth = paren_depth;
                                    annotation_multiline_base_indent = indent;
                                    indent += 2;
                                }
                            }
                        }
                        if should_break_after_open_paren {
                            out.push('\n');
                            at_line_start = true;
                            if !call_args_continuation_active {
                                call_args_continuation_active = true;
                                call_args_continuation_paren_depth = paren_depth;
                                call_args_continuation_base_indent = indent;
                                call_args_comment_mode = call_argument_metrics
                                    .as_ref()
                                    .is_some_and(CallArgumentMetrics::has_comment);
                                call_args_vertical_mode = call_argument_metrics
                                    .as_ref()
                                    .is_some_and(CallArgumentMetrics::should_force_vertical);
                                indent += if dot_continuation_call { 4 } else { 2 };
                            }
                        }
                    }
                    ")" => {
                        let closes_try_resource_paren =
                            try_resource_paren_depth > 0 && paren_depth == try_resource_paren_depth;
                        let closes_annotation_multiline_args = annotation_multiline_args_active
                            && paren_depth == annotation_multiline_args_paren_depth;
                        let closes_call_args_continuation = call_args_continuation_active
                            && paren_depth == call_args_continuation_paren_depth;
                        if closes_annotation_multiline_args {
                            indent = annotation_multiline_base_indent;
                        }
                        if closes_call_args_continuation {
                            indent = call_args_continuation_base_indent;
                        }
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, ")");
                        paren_depth = paren_depth.saturating_sub(1);
                        if closes_try_resource_paren {
                            try_resource_paren_depth = 0;
                            if try_resource_continuation_active {
                                indent = try_resource_continuation_base_indent;
                                try_resource_continuation_active = false;
                            }
                        }
                        if annotation_active && annotation_paren_depth > 0 {
                            annotation_paren_depth -= 1;
                        }
                        if closes_annotation_multiline_args {
                            annotation_multiline_args_active = false;
                            annotation_multiline_args_paren_depth = 0;
                        }
                        if closes_call_args_continuation {
                            call_args_continuation_active = false;
                            call_args_continuation_paren_depth = 0;
                            call_args_comment_mode = false;
                            call_args_vertical_mode = false;
                        }
                    }
                    "." | "[" | "]" | "::" => {
                        let top_level_chain_dot = symbol == "."
                            && chain_continuation_active
                            && paren_depth == chain_continuation_paren_depth
                            && bracket_depth == chain_continuation_bracket_depth;
                        if top_level_chain_dot {
                            if chain_continuation_dot_count >= chain_keep_leading_dots {
                                out.push('\n');
                                at_line_start = true;
                            }
                            chain_continuation_dot_count += 1;
                        } else if symbol == "."
                            && prev_text.as_deref() == Some(")")
                            && (((lambda_continuation_active || chain_continuation_active)
                                && should_break_before_chained_dot(out.as_str()))
                                || next_dotted_member_call_breaks(
                                    tokens.as_slice(),
                                    i + consumed,
                                    ir.source,
                                ))
                        {
                            out.push('\n');
                            at_line_start = true;
                        }
                        let current_indent = active_indent(indent, &block_stack);
                        let extra_indent = if symbol == "."
                            && (lambda_continuation_active
                                || chain_continuation_active
                                || prev_text.as_deref() == Some(")"))
                            && at_line_start
                        {
                            2
                        } else {
                            0
                        };
                        write_with_indent(
                            &mut out,
                            &mut at_line_start,
                            current_indent + extra_indent,
                            &symbol,
                        );
                        if symbol == "[" {
                            bracket_depth += 1;
                        } else if symbol == "]" {
                            bracket_depth = bracket_depth.saturating_sub(1);
                        }
                    }
                    "++" | "--" => {
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, &symbol);
                    }
                    "+" => {
                        if is_exponent_sign_context(prev_text.as_deref(), next_text.as_deref()) {
                            let current_indent = active_indent(indent, &block_stack);
                            write_with_indent(&mut out, &mut at_line_start, current_indent, "+");
                        } else if is_unary_prefix_context(prev_text.as_deref()) {
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
                        if starts_block_lambda_rhs(tokens.as_slice(), i + consumed, ir.source)
                            || should_break_assignment_rhs(
                                tokens.as_slice(),
                                i + consumed,
                                ir.source,
                            )
                        {
                            out.push('\n');
                            at_line_start = true;
                            if let Some(metrics) = chain_continuation_metrics(
                                tokens.as_slice(),
                                i + consumed,
                                ir.source,
                            ) {
                                chain_continuation_active = true;
                                chain_continuation_paren_depth = paren_depth;
                                chain_continuation_bracket_depth = bracket_depth;
                                chain_continuation_dot_count = 0;
                                chain_keep_leading_dots = metrics.keep_leading_dots;
                            }
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
                        } else if is_exponent_sign_context(
                            prev_text.as_deref(),
                            next_text.as_deref(),
                        ) {
                            let current_indent = active_indent(indent, &block_stack);
                            write_with_indent(&mut out, &mut at_line_start, current_indent, "-");
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
                    "?" => {
                        ensure_space(&mut out, at_line_start);
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, "?");
                        out.push(' ');
                        ternary_depth += 1;
                    }
                    ":" => {
                        if ternary_depth > 0 {
                            ensure_space(&mut out, at_line_start);
                            let current_indent = active_indent(indent, &block_stack);
                            write_with_indent(&mut out, &mut at_line_start, current_indent, ":");
                            out.push(' ');
                            ternary_depth = ternary_depth.saturating_sub(1);
                        } else if is_label_colon_context(
                            &prev_kind,
                            prev_text.as_deref(),
                            next_text.as_deref(),
                            paren_depth,
                        ) {
                            let current_indent = active_indent(indent, &block_stack);
                            write_with_indent(&mut out, &mut at_line_start, current_indent, ":");
                            out.push('\n');
                            at_line_start = true;
                        } else {
                            ensure_space(&mut out, at_line_start);
                            let current_indent = active_indent(indent, &block_stack);
                            write_with_indent(&mut out, &mut at_line_start, current_indent, ":");
                            out.push(' ');
                        }
                    }
                    "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "==" | "!=" | "<="
                    | ">=" | "&&" | "||" | "*" | "/" | "%" | "&" | "|" | "^" | "->" => {
                        ensure_space(&mut out, at_line_start);
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, &symbol);
                        out.push(' ');
                    }
                    "<" => {
                        let generic_like = is_generic_open_angle(
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
                    "..." => {
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, "...");
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

                prev_text = Some(symbol.to_owned());
                prev_kind = Some(TokenKind::Symbol);
                i += consumed;

                if annotation_active && annotation_paren_depth == 0 {
                    if symbol != "@" && symbol != "." {
                        let next = next_symbol_text(tokens.as_slice(), i, ir.source);
                        if next.as_deref() != Some(".") && next.as_deref() != Some("(") {
                            let in_type_body = block_stack.last().is_some_and(|frame| {
                                matches!(frame.kind, BlockKind::TypeBody | BlockKind::EnumBody)
                            });
                            let keep_inline = should_keep_inline_annotation(
                                tokens.as_slice(),
                                i,
                                ir.source,
                                in_type_body,
                                block_stack.is_empty() && paren_depth == 0,
                                annotation_name.as_deref(),
                                annotation_started_line_start,
                                annotation_has_args,
                                paren_depth,
                            );
                            annotation_active = false;
                            annotation_brace_depth = 0;
                            annotation_has_args = false;
                            annotation_name = None;
                            annotation_inline_run_active = keep_inline;
                            if annotation_multiline_args_active {
                                indent = annotation_multiline_base_indent;
                                annotation_multiline_args_active = false;
                                annotation_multiline_args_paren_depth = 0;
                            }
                            if !keep_inline && paren_depth == 0 && !at_line_start {
                                out.push('\n');
                                at_line_start = true;
                            }
                        }
                    }
                }
            }
            TokenKind::StringLiteral | TokenKind::CharLiteral => {
                annotation_inline_run_active = false;
                let text = token_text(ir.source, token);
                if needs_space_before(&prev_text, text, at_line_start) {
                    ensure_space(&mut out, at_line_start);
                }
                if at_line_start && text.starts_with("\"\"\"") {
                    out.push_str(text);
                    at_line_start = false;
                } else {
                    let current_indent = active_indent(indent, &block_stack);
                    write_with_indent(&mut out, &mut at_line_start, current_indent, text);
                }
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
