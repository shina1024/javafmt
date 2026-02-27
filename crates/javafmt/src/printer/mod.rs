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
    EnumBody,
    ModuleBody,
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
    let mut block_stack: Vec<BlockFrame> = Vec::new();
    let mut prev_text: Option<String> = None;
    let mut prev_kind: Option<TokenKind> = None;
    let mut annotation_active = false;
    let mut annotation_paren_depth = 0usize;
    let mut annotation_brace_depth = 0usize;
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
                let leading_block_comment = i > 0
                    && at_line_start
                    && out.ends_with('\n')
                    && token_text(ir.source, tokens[i - 1]) == "{";
                if attach_after_statement {
                    out.pop();
                    at_line_start = false;
                }
                ensure_space(&mut out, at_line_start);
                let text = token_text(ir.source, token);
                let current_indent = active_indent(indent, &block_stack);
                let comment_indent = if leading_block_comment {
                    current_indent + 1
                } else {
                    current_indent
                };
                write_with_indent(&mut out, &mut at_line_start, comment_indent, text);
                if text.ends_with('\n') {
                    at_line_start = true;
                } else if attach_after_statement {
                    out.push('\n');
                    at_line_start = true;
                } else if leading_block_comment {
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
                                label_prev_text = Some(String::from(","));
                                i += consumed;
                                continue;
                            }
                            match symbol.as_str() {
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
                            label_prev_text = Some(symbol);
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
                        out.push('\n');
                        at_line_start = true;
                        annotation_active = false;
                        annotation_brace_depth = 0;
                        if annotation_multiline_args_active {
                            indent = annotation_multiline_base_indent;
                            annotation_multiline_args_active = false;
                            annotation_multiline_args_paren_depth = 0;
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

                match symbol.as_str() {
                    "@" => {
                        if !at_line_start {
                            out.push('\n');
                            at_line_start = true;
                        }
                        let current_indent = active_indent(indent, &block_stack);
                        write_with_indent(&mut out, &mut at_line_start, current_indent, "@");
                        let annotation_declaration =
                            next_symbol_text(tokens.as_slice(), i + consumed, ir.source).as_deref()
                                == Some("interface");
                        if annotation_declaration {
                            annotation_active = false;
                            annotation_paren_depth = 0;
                            annotation_brace_depth = 0;
                            annotation_multiline_args_active = false;
                            annotation_multiline_args_paren_depth = 0;
                        } else {
                            annotation_active = true;
                            annotation_paren_depth = 0;
                            annotation_brace_depth = 0;
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
                        if paren_depth == 0 {
                            ternary_depth = 0;
                            if call_args_continuation_active {
                                indent = call_args_continuation_base_indent;
                                call_args_continuation_active = false;
                                call_args_continuation_paren_depth = 0;
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
                            let in_type_body = block_stack.last().is_some_and(|frame| {
                                matches!(frame.kind, BlockKind::TypeBody | BlockKind::EnumBody)
                            });
                            let in_module_body = block_stack
                                .last()
                                .is_some_and(|frame| frame.kind == BlockKind::ModuleBody);
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
                                out.push('\n');
                                at_line_start = true;
                                if !try_resource_continuation_active {
                                    try_resource_continuation_active = true;
                                    try_resource_continuation_base_indent = indent;
                                    indent += 2;
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
                        if enum_constants_comma
                            || (module_continuation_active && paren_depth == 0)
                            || annotation_args_comma
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
                        let should_break_after_open_paren =
                            is_explicit_type_argument_call(tokens.as_slice(), i, ir.source)
                                && should_break_call_arguments(
                                    tokens.as_slice(),
                                    i + consumed,
                                    ir.source,
                                );
                        paren_depth += 1;
                        if prev_text.as_deref() == Some("try") {
                            try_resource_paren_depth = paren_depth;
                        }
                        if annotation_active {
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
                                indent += 2;
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
                        }
                    }
                    "." | "[" | "]" | "::" => {
                        if symbol == "."
                            && lambda_continuation_active
                            && prev_text.as_deref() == Some(")")
                        {
                            out.push('\n');
                            at_line_start = true;
                        }
                        let current_indent = active_indent(indent, &block_stack);
                        let extra_indent =
                            if symbol == "." && lambda_continuation_active && at_line_start {
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
                            annotation_brace_depth = 0;
                            if annotation_multiline_args_active {
                                indent = annotation_multiline_base_indent;
                                annotation_multiline_args_active = false;
                                annotation_multiline_args_paren_depth = 0;
                            }
                        }
                    }
                }
            }
            TokenKind::StringLiteral | TokenKind::CharLiteral => {
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
    matches!(word, "class" | "interface" | "record")
}

fn top_level_directive_from_word(word: &str) -> Option<TopLevelDirective> {
    match word {
        "package" => Some(TopLevelDirective::Package),
        "import" => Some(TopLevelDirective::Import),
        _ => None,
    }
}

fn module_directive_from_word(word: &str) -> Option<ModuleDirective> {
    match word {
        "requires" => Some(ModuleDirective::Requires),
        "exports" => Some(ModuleDirective::Exports),
        "opens" => Some(ModuleDirective::Opens),
        "uses" => Some(ModuleDirective::Uses),
        "provides" => Some(ModuleDirective::Provides),
        _ => None,
    }
}

fn next_top_level_directive(
    tokens: &[&Token],
    mut index: usize,
    source: &str,
) -> Option<TopLevelDirective> {
    while index < tokens.len() {
        let token = tokens[index];
        match token.kind {
            TokenKind::Word => return top_level_directive_from_word(token_text(source, token)),
            TokenKind::Symbol => {
                let (symbol, consumed) = read_symbol(tokens, index, source);
                if symbol == "}" {
                    return None;
                }
                index += consumed;
            }
            _ => {
                index += 1;
            }
        }
    }
    None
}

fn next_module_directive(
    tokens: &[&Token],
    mut index: usize,
    source: &str,
) -> Option<ModuleDirective> {
    while index < tokens.len() {
        let token = tokens[index];
        match token.kind {
            TokenKind::Word => return module_directive_from_word(token_text(source, token)),
            TokenKind::Symbol => {
                let (symbol, consumed) = read_symbol(tokens, index, source);
                if symbol == "}" {
                    return None;
                }
                index += consumed;
            }
            _ => {
                index += 1;
            }
        }
    }
    None
}

fn is_label_colon_context(
    prev_kind: &Option<TokenKind>,
    prev_text: Option<&str>,
    next_text: Option<&str>,
    paren_depth: usize,
) -> bool {
    if paren_depth != 0 || !matches!(prev_kind, Some(TokenKind::Word)) {
        return false;
    }
    if matches!(prev_text, Some("case" | "default")) {
        return false;
    }
    if matches!(next_text, Some(":" | ";" | "," | ")" | "]" | "}")) {
        return false;
    }
    next_text.is_some()
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

fn should_break_annotation_arguments(tokens: &[&Token], mut index: usize, source: &str) -> bool {
    let mut local_paren_depth = 0usize;
    let mut local_brace_depth = 0usize;
    let mut saw_top_level_equals = false;
    let mut saw_top_level_comma = false;

    while index < tokens.len() {
        let token = tokens[index];
        if token.kind == TokenKind::Symbol {
            let (symbol, consumed) = read_symbol(tokens, index, source);
            match symbol.as_str() {
                "(" => local_paren_depth += 1,
                ")" => {
                    if local_paren_depth == 0 {
                        break;
                    }
                    local_paren_depth -= 1;
                }
                "{" => local_brace_depth += 1,
                "}" => local_brace_depth = local_brace_depth.saturating_sub(1),
                "=" if local_paren_depth == 0 && local_brace_depth == 0 => {
                    saw_top_level_equals = true;
                }
                "," if local_paren_depth == 0 && local_brace_depth == 0 => {
                    saw_top_level_comma = true;
                }
                _ => {}
            }
            index += consumed;
        } else {
            index += 1;
        }
    }

    saw_top_level_equals && saw_top_level_comma
}

fn should_break_call_arguments(tokens: &[&Token], mut index: usize, source: &str) -> bool {
    let mut local_paren_depth = 0usize;
    let mut local_bracket_depth = 0usize;
    let mut local_brace_depth = 0usize;
    let mut scanned_chars = 0usize;
    let mut top_level_comma_count = 0usize;

    while index < tokens.len() {
        let token = tokens[index];
        scanned_chars += token.end.saturating_sub(token.start);
        if token.kind == TokenKind::Symbol {
            let (symbol, consumed) = read_symbol(tokens, index, source);
            match symbol.as_str() {
                "(" => local_paren_depth += 1,
                ")" => {
                    if local_paren_depth == 0 {
                        break;
                    }
                    local_paren_depth -= 1;
                }
                "[" => local_bracket_depth += 1,
                "]" => local_bracket_depth = local_bracket_depth.saturating_sub(1),
                "{" => local_brace_depth += 1,
                "}" => local_brace_depth = local_brace_depth.saturating_sub(1),
                "," if local_paren_depth == 0
                    && local_bracket_depth == 0
                    && local_brace_depth == 0 =>
                {
                    top_level_comma_count += 1;
                }
                _ => {}
            }
            index += consumed;
        } else {
            index += 1;
        }
    }

    scanned_chars >= 30 && top_level_comma_count >= 1
}

fn is_explicit_type_argument_call(tokens: &[&Token], index: usize, source: &str) -> bool {
    if index == 0
        || !tokens
            .get(index - 1)
            .is_some_and(|token| token.kind == TokenKind::Word)
    {
        return false;
    }

    let mut cursor = index.saturating_sub(2);
    loop {
        let Some(token) = tokens.get(cursor) else {
            return false;
        };
        match token.kind {
            TokenKind::Symbol => {
                let (symbol, _) = read_symbol(tokens, cursor, source);
                return matches!(symbol.as_str(), ">" | ">>" | ">>>");
            }
            TokenKind::Word | TokenKind::StringLiteral | TokenKind::CharLiteral => return false,
            _ => {}
        }
        if cursor == 0 {
            break;
        }
        cursor -= 1;
    }
    false
}

fn should_break_assignment_rhs(tokens: &[&Token], mut index: usize, source: &str) -> bool {
    if let Some(next) = tokens.get(index)
        && next.kind == TokenKind::Word
        && token_text(source, next) == "switch"
    {
        return true;
    }

    let mut local_paren_depth = 0usize;
    let mut local_bracket_depth = 0usize;
    let mut dot_after_call_count = 0usize;
    let mut scanned_chars = 0usize;
    let mut saw_sorted_call = false;
    let mut saw_top_level_method_call = false;
    let mut saw_top_level_angle = false;

    while index < tokens.len() {
        let token = tokens[index];
        scanned_chars += token.end.saturating_sub(token.start);
        if token.kind == TokenKind::Word && token_text(source, token) == "sorted" {
            saw_sorted_call = true;
        }
        if token.kind == TokenKind::Symbol {
            let (symbol, consumed) = read_symbol(tokens, index, source);
            match symbol.as_str() {
                ";" if local_paren_depth == 0 && local_bracket_depth == 0 => break,
                "(" => {
                    if local_paren_depth == 0 && local_bracket_depth == 0 {
                        saw_top_level_method_call = true;
                    }
                    local_paren_depth += 1;
                }
                ")" => local_paren_depth = local_paren_depth.saturating_sub(1),
                "[" => local_bracket_depth += 1,
                "]" => local_bracket_depth = local_bracket_depth.saturating_sub(1),
                "<" | ">" | ">>" | ">>>" if local_paren_depth == 0 && local_bracket_depth == 0 => {
                    saw_top_level_angle = true;
                }
                "." if local_paren_depth == 0 && local_bracket_depth == 0 => {
                    if index > 0 && token_text(source, tokens[index - 1]) == ")" {
                        dot_after_call_count += 1;
                    }
                }
                _ => {}
            }
            index += consumed;
        } else {
            index += 1;
        }
    }

    let sorted_chain = dot_after_call_count >= 4 && scanned_chars >= 60 && saw_sorted_call;
    let long_generic_call = scanned_chars >= 90 && saw_top_level_method_call && saw_top_level_angle;
    sorted_chain || long_generic_call
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
    if next_text == Some(">")
        && next_symbol_text(tokens, index + 2, source).as_deref() == Some("(")
        && (matches!(
            prev_kind,
            Some(TokenKind::Word | TokenKind::StringLiteral | TokenKind::CharLiteral)
        ) || matches!(prev_text, Some("." | "new")))
    {
        return true;
    }

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

fn is_inline_initializer_brace(
    tokens: &[&Token],
    index: usize,
    source: &str,
    prev_text: &Option<String>,
) -> bool {
    if !matches!(prev_text.as_deref(), Some("]" | "=")) {
        return false;
    }

    let mut depth = 0usize;
    let mut i = index;
    while i < tokens.len() {
        let token = tokens[i];
        if token.kind != TokenKind::Symbol {
            i += 1;
            continue;
        }
        let (symbol, consumed) = read_symbol(tokens, i, source);
        match symbol.as_str() {
            "{" => depth += 1,
            "}" => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return true;
                }
            }
            ";" if depth == 1 => return false,
            _ => {}
        }
        i += consumed;
    }
    false
}

fn is_exponent_sign_context(prev_text: Option<&str>, next_text: Option<&str>) -> bool {
    let Some(prev) = prev_text else {
        return false;
    };
    let Some(next) = next_text else {
        return false;
    };

    let ends_with_exponent = prev.ends_with('e') || prev.ends_with('E');
    ends_with_exponent && next.chars().all(|ch| ch.is_ascii_digit() || ch == '_')
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
    fn keeps_plain_array_initializer_inline_for_short_literal() {
        let source = "class A{void f(){int[] a={1,2,3};}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("int[] a = {1, 2, 3};"));
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
    fn keeps_switch_guard_method_call_spacing() {
        let source = "class A{void f(){var p=switch(x){case String s when s.length()>3->s;default->\"\";};}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("when s.length() > 3 -> s;"));
    }

    #[test]
    fn keeps_scientific_notation_sign_tight() {
        let source = "class A{void f(){double d=1.23e-4;double e=2.0E+8;}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("double d = 1.23e-4;"));
        assert!(printed.text.contains("double e = 2.0E+8;"));
    }

    #[test]
    fn breaks_return_before_text_block_literal() {
        let source = "class A{String f(){return \"\"\"\nline1\nline2\n\"\"\";}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("return\n\"\"\"\nline1"));
    }

    #[test]
    fn breaks_assignment_before_switch_expression_rhs() {
        let source = "class A{void f(){var p=switch(x){case String s when s.length()>3->s;default->\"\";};}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("var p =\n"));
        assert!(printed.text.contains("switch (x) {"));
    }

    #[test]
    fn breaks_long_chained_call_assignment() {
        let source = "class A{void f(){var x=java.util.stream.Stream.of(1,2,3,4,5).map(i->i+1).filter(i->i%2==0).sorted().toList();}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("var x =\n"));
        assert!(printed.text.contains("\n            .map("));
    }

    #[test]
    fn formats_try_with_multiple_resources_multiline() {
        let source = "class A{void f(){try(var in1=open();var in2=open2()){use(in1,in2);}catch(Exception e){x();}}java.io.InputStream open(){return null;}java.io.InputStream open2(){return null;}void use(java.io.InputStream a,java.io.InputStream b){}void x(){}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("try (var in1 = open();\n"));
        assert!(printed.text.contains("var in2 = open2())"));
    }

    #[test]
    fn keeps_annotation_array_inline_when_short() {
        let source = "class A{@SuppressWarnings({\"unchecked\",\"rawtypes\"}) void f(){}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(
            printed
                .text
                .contains("@SuppressWarnings({\"unchecked\", \"rawtypes\"})")
        );
    }

    #[test]
    fn formats_enum_constants_on_separate_lines() {
        let source = "class A{enum E{A(1),B(2);final int n;E(int n){this.n=n;}}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("A(1),\n    B(2);"));
    }

    #[test]
    fn formats_module_to_and_with_clauses_multiline() {
        let source = "module m.example{requires java.base;exports a.b;opens a.c to x.y,z.w;uses a.spi.S;provides a.spi.S with a.impl.SImpl;}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("opens a.c to\n"));
        assert!(printed.text.contains("x.y,\n"));
        assert!(printed.text.contains("provides a.spi.S with\n"));
    }

    #[test]
    fn formats_top_level_package_and_import_blank_lines() {
        let source = "package p;import java.util.List;class A{}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(
            printed
                .text
                .starts_with("package p;\n\nimport java.util.List;\n\nclass A {}\n")
        );
    }

    #[test]
    fn formats_statement_label_without_space_before_colon() {
        let source = "class A{void f(){outer:for(int i=0;i<1;i++){break outer;}}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("outer:\n"));
        assert!(!printed.text.contains("outer :"));
    }

    #[test]
    fn keeps_enum_constant_body_comma_on_same_line() {
        let source = "class A{enum E{A{int v(){return 1;}},B;}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("},\n    B;"));
    }

    #[test]
    fn keeps_module_requires_group_compact() {
        let source =
            "module m.probe{requires transitive java.base;requires static java.sql;exports p.api;}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains(
            "requires transitive java.base;\n  requires static java.sql;\n\n  exports p.api;"
        ));
    }

    #[test]
    fn keeps_annotation_interface_keyword_together() {
        let source = "class A{@interface B{}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("@interface B {"));
    }

    #[test]
    fn keeps_block_comment_on_its_own_line_after_open_brace() {
        let source =
            "class A{void f(){do{/*x*/a();}while(cond());}void a(){}boolean cond(){return true;}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("do {\n        /*x*/\n      a();"));
    }

    #[test]
    fn breaks_named_annotation_arguments_multiline() {
        let source = "class A{@Anno(values={1,2,3},name=\"x\") void f(){} @interface Anno{int[] values();String name();}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("@Anno(\n"));
        assert!(printed.text.contains("values = {1, 2, 3},\n"));
        assert!(printed.text.contains("name = \"x\")"));
    }

    #[test]
    fn breaks_long_generic_assignment_rhs() {
        let source = "class A{void f(){var m=java.util.Map.<String,java.util.List<java.util.Set<Integer>>>of(\"k\",java.util.List.of(java.util.Set.of(1,2)));}}";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("var m =\n"));
    }

    #[test]
    fn keeps_diamond_operator_without_spaces() {
        let source = "class A{record R<T>(T x){} R<Integer> r=new R<>(1);} ";
        let lexed = lexer::lex(source);
        let cst = parser::parse(&lexed);
        let attachments = comments::attach(&cst, &lexed);
        let ir = ir::build(&cst, attachments);
        let printed = print(&ir);
        assert!(printed.text.contains("new R<>(1);"));
        assert!(!printed.text.contains("R < >"));
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
