use super::*;

pub(super) fn active_indent(base_indent: usize, stack: &[BlockFrame]) -> usize {
    match stack.last() {
        Some(frame) if frame.kind == BlockKind::Switch && frame.in_switch_case => base_indent + 1,
        _ => base_indent,
    }
}

pub(super) fn is_switch_label(word: &str, stack: &[BlockFrame]) -> bool {
    matches!(word, "case" | "default")
        && stack
            .last()
            .is_some_and(|frame| frame.kind == BlockKind::Switch)
}

pub(super) fn is_type_declaration_keyword(word: &str) -> bool {
    matches!(word, "class" | "interface" | "record")
}

pub(super) fn top_level_directive_from_word(word: &str) -> Option<TopLevelDirective> {
    match word {
        "package" => Some(TopLevelDirective::Package),
        "import" => Some(TopLevelDirective::Import),
        _ => None,
    }
}

pub(super) fn module_directive_from_word(word: &str) -> Option<ModuleDirective> {
    match word {
        "requires" => Some(ModuleDirective::Requires),
        "exports" => Some(ModuleDirective::Exports),
        "opens" => Some(ModuleDirective::Opens),
        "uses" => Some(ModuleDirective::Uses),
        "provides" => Some(ModuleDirective::Provides),
        _ => None,
    }
}

pub(super) fn next_top_level_directive(
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

pub(super) fn next_module_directive(
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

pub(super) fn is_label_colon_context(
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

pub(super) fn collect_meaningful_tokens<'a>(ir: &'a FormatIr<'a>) -> Vec<&'a Token> {
    ir.tokens
        .iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline))
        .collect::<Vec<_>>()
}

pub(super) fn starts_block_lambda_rhs(tokens: &[&Token], mut index: usize, source: &str) -> bool {
    let mut local_paren_depth = 0usize;
    while index < tokens.len() {
        let token = tokens[index];
        if token.kind == TokenKind::Symbol {
            let (symbol, consumed) = read_symbol(tokens, index, source);
            match symbol {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ChainContinuationMetrics {
    pub keep_leading_dots: usize,
}

pub(super) fn chain_continuation_metrics(
    tokens: &[&Token],
    mut index: usize,
    source: &str,
) -> Option<ChainContinuationMetrics> {
    let mut local_paren_depth = 0usize;
    let mut local_bracket_depth = 0usize;
    let mut scanned_chars = 0usize;
    let mut top_level_dot_count = 0usize;
    let mut dot_after_call_count = 0usize;
    let mut keep_leading_dots = 0usize;
    let mut first_top_level_call_dot = 0usize;
    let mut saw_top_level_method_call = false;

    while index < tokens.len() {
        let token = tokens[index];
        scanned_chars += token.end.saturating_sub(token.start);
        if token.kind == TokenKind::Symbol {
            let (symbol, consumed) = read_symbol(tokens, index, source);
            match symbol {
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
                "." if local_paren_depth == 0 && local_bracket_depth == 0 => {
                    top_level_dot_count += 1;
                    let member_call_name =
                        next_chain_member_call_name(tokens, index + consumed, source);
                    if first_top_level_call_dot == 0 && member_call_name.is_some() {
                        first_top_level_call_dot = top_level_dot_count;
                    }
                    if index > 0 && token_text(source, tokens[index - 1]) == ")" {
                        dot_after_call_count += 1;
                    }
                    if top_level_dot_count <= 2
                        && let Some(member_name) = member_call_name
                        && is_chain_prefix_method(member_name)
                    {
                        keep_leading_dots = top_level_dot_count;
                    }
                }
                _ => {}
            }
            index += consumed;
        } else {
            index += 1;
        }
    }

    let long_member_chain =
        top_level_dot_count >= 5 && scanned_chars >= 80 && !saw_top_level_method_call;
    if keep_leading_dots == 0 && first_top_level_call_dot >= 2 {
        keep_leading_dots = first_top_level_call_dot;
    }
    if long_member_chain && keep_leading_dots == 0 {
        keep_leading_dots = 1;
    }

    (dot_after_call_count >= 1 || long_member_chain)
        .then_some(ChainContinuationMetrics { keep_leading_dots })
}

pub(super) fn should_break_annotation_arguments(
    tokens: &[&Token],
    mut index: usize,
    source: &str,
) -> bool {
    let mut local_paren_depth = 0usize;
    let mut local_brace_depth = 0usize;
    let mut saw_top_level_equals = false;
    let mut saw_top_level_comma = false;

    while index < tokens.len() {
        let token = tokens[index];
        if token.kind == TokenKind::Symbol {
            let (symbol, consumed) = read_symbol(tokens, index, source);
            match symbol {
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

pub(super) fn call_arguments_break_metrics(
    tokens: &[&Token],
    mut index: usize,
    source: &str,
) -> Option<CallArgumentMetrics> {
    let mut local_paren_depth = 0usize;
    let mut local_bracket_depth = 0usize;
    let mut local_brace_depth = 0usize;
    let mut scanned_chars = 0usize;
    let mut top_level_comma_count = 0usize;
    let mut top_level_call_arg_count = 0usize;
    let mut first_arg_has_call = false;
    let mut current_arg_has_call = false;
    let mut current_input_line_args = 1usize;
    let mut max_input_line_args = 1usize;
    let mut saw_any_argument_token = false;
    let mut saw_first_arg = false;
    let mut saw_top_level_newline = false;
    let mut has_comment = false;
    let mut has_top_level_initializer = false;

    while index < tokens.len() {
        let token = tokens[index];
        scanned_chars += token.end.saturating_sub(token.start);
        if matches!(token.kind, TokenKind::BlockComment | TokenKind::LineComment) {
            has_comment = true;
        }
        if token.kind == TokenKind::Symbol {
            let (symbol, consumed) = read_symbol(tokens, index, source);
            let next_index = index + consumed;
            let token_end = tokens[next_index - 1].end;
            let top_level_newline_after = next_index < tokens.len()
                && local_paren_depth == 0
                && local_bracket_depth == 0
                && local_brace_depth == 0
                && source_gap_has_newline(source, token_end, tokens[next_index].start);
            match symbol {
                "(" => {
                    saw_any_argument_token = true;
                    if local_paren_depth == 0 && local_bracket_depth == 0 && local_brace_depth == 0
                    {
                        current_arg_has_call = true;
                    }
                    local_paren_depth += 1;
                }
                ")" => {
                    if local_paren_depth == 0 {
                        if saw_any_argument_token {
                            if !saw_first_arg {
                                first_arg_has_call = current_arg_has_call;
                            }
                            if current_arg_has_call {
                                top_level_call_arg_count += 1;
                            }
                            max_input_line_args = max_input_line_args.max(current_input_line_args);
                        }
                        break;
                    }
                    local_paren_depth -= 1;
                }
                "[" => {
                    saw_any_argument_token = true;
                    local_bracket_depth += 1;
                }
                "]" => {
                    saw_any_argument_token = true;
                    local_bracket_depth = local_bracket_depth.saturating_sub(1);
                }
                "{" => {
                    saw_any_argument_token = true;
                    if local_paren_depth == 0 && local_bracket_depth == 0 && local_brace_depth == 0
                    {
                        has_top_level_initializer = true;
                    }
                    local_brace_depth += 1;
                }
                "}" => {
                    saw_any_argument_token = true;
                    local_brace_depth = local_brace_depth.saturating_sub(1);
                }
                "," if local_paren_depth == 0
                    && local_bracket_depth == 0
                    && local_brace_depth == 0 =>
                {
                    top_level_comma_count += 1;
                    if !saw_first_arg {
                        first_arg_has_call = current_arg_has_call;
                        saw_first_arg = true;
                    }
                    if current_arg_has_call {
                        top_level_call_arg_count += 1;
                    }
                    current_arg_has_call = false;
                    if top_level_newline_after {
                        saw_top_level_newline = true;
                        max_input_line_args = max_input_line_args.max(current_input_line_args);
                        current_input_line_args = 1;
                    } else {
                        current_input_line_args += 1;
                    }
                }
                _ => saw_any_argument_token = true,
            }
            if next_index < tokens.len()
                && local_paren_depth == 0
                && local_bracket_depth == 0
                && local_brace_depth == 0
                && top_level_newline_after
                && !(symbol == ","
                    && local_paren_depth == 0
                    && local_bracket_depth == 0
                    && local_brace_depth == 0)
            {
                saw_top_level_newline = true;
                max_input_line_args = max_input_line_args.max(current_input_line_args);
                current_input_line_args = 1;
            }
            index = next_index;
        } else {
            saw_any_argument_token = true;
            let next_index = index + 1;
            if next_index < tokens.len()
                && local_paren_depth == 0
                && local_bracket_depth == 0
                && local_brace_depth == 0
                && source_gap_has_newline(source, token.end, tokens[next_index].start)
            {
                saw_top_level_newline = true;
                max_input_line_args = max_input_line_args.max(current_input_line_args);
                current_input_line_args = 1;
            }
            index = next_index;
        }
    }

    Some(CallArgumentMetrics {
        scanned_chars,
        top_level_comma_count,
        top_level_call_arg_count,
        first_arg_has_call,
        max_input_line_args,
        saw_top_level_newline,
        has_comment,
        has_top_level_initializer,
    })
}

pub(super) struct CallArgumentMetrics {
    scanned_chars: usize,
    top_level_comma_count: usize,
    top_level_call_arg_count: usize,
    first_arg_has_call: bool,
    max_input_line_args: usize,
    saw_top_level_newline: bool,
    has_comment: bool,
    has_top_level_initializer: bool,
}

impl CallArgumentMetrics {
    pub(super) fn should_break_short(&self) -> bool {
        self.has_top_level_initializer
            || (self.scanned_chars >= 30 && self.top_level_comma_count >= 1)
    }

    pub(super) fn should_break_long(&self) -> bool {
        self.has_top_level_initializer
            || (self.scanned_chars >= 60 && self.top_level_comma_count >= 1)
    }

    pub(super) fn should_force_vertical(&self) -> bool {
        self.has_top_level_initializer
            || (self.saw_top_level_newline && self.max_input_line_args > 2)
            || self.first_arg_has_call
            || self.top_level_call_arg_count >= 2
    }

    pub(super) fn has_comment(&self) -> bool {
        self.has_comment
    }
}

pub(super) fn should_keep_inline_annotation(
    tokens: &[&Token],
    index: usize,
    source: &str,
    in_type_body: bool,
    top_level_declaration: bool,
    annotation_name: Option<&str>,
    annotation_started_line_start: bool,
    annotation_has_args: bool,
    paren_depth: usize,
) -> bool {
    if paren_depth > 0 || !annotation_started_line_start {
        return true;
    }

    if top_level_declaration {
        return false;
    }

    if !in_type_body {
        return !annotation_has_args;
    }

    if annotation_has_args || is_declaration_like_annotation(annotation_name) {
        return false;
    }

    let next_text = next_symbol_text(tokens, index, source);
    match next_text {
        Some("@") => is_type_use_friendly_annotation(annotation_name),
        Some(word)
            if is_declaration_modifier(word)
                || is_type_declaration_keyword(word)
                || is_primitive_or_void(word) =>
        {
            false
        }
        Some(word) if is_word_like_text(word) => looks_like_typed_member(tokens, index, source),
        _ => false,
    }
}

pub(super) fn should_start_annotation_inline(prev_text: Option<&str>, in_type_body: bool) -> bool {
    matches!(prev_text, Some(text) if is_declaration_modifier(text))
        || (!in_type_body && matches!(prev_text, Some("new")))
        || matches!(prev_text, Some("@" | "." | "::"))
}

pub(super) fn looks_like_typed_member(tokens: &[&Token], index: usize, source: &str) -> bool {
    let Some(after_type) = consume_type_like(tokens, index, source) else {
        return false;
    };
    tokens
        .get(after_type)
        .is_some_and(|token| token.kind == TokenKind::Word)
}

pub(super) fn consume_type_like(
    tokens: &[&Token],
    mut index: usize,
    source: &str,
) -> Option<usize> {
    let token = tokens.get(index)?;
    if token.kind != TokenKind::Word {
        return None;
    }

    let word = token_text(source, token);
    if is_declaration_modifier(word)
        || is_type_declaration_keyword(word)
        || is_primitive_or_void(word)
    {
        return None;
    }

    index += 1;
    loop {
        let Some(token) = tokens.get(index) else {
            return Some(index);
        };
        if token.kind != TokenKind::Symbol {
            return Some(index);
        }

        let (symbol, consumed) = read_symbol(tokens, index, source);
        match symbol {
            "." => {
                let next = tokens.get(index + consumed)?;
                if next.kind != TokenKind::Word {
                    return Some(index);
                }
                index += consumed + 1;
            }
            "<" if looks_like_type_argument_list(tokens, index, source) => {
                index = skip_type_arguments(tokens, index, source);
            }
            "[" => {
                if next_symbol_text(tokens, index + consumed, source).as_deref() != Some("]") {
                    return Some(index);
                }
                index += consumed;
                let (_, close_consumed) = read_symbol(tokens, index, source);
                index += close_consumed;
            }
            _ => return Some(index),
        }
    }
}

pub(super) fn skip_type_arguments(tokens: &[&Token], mut index: usize, source: &str) -> usize {
    let mut depth = 0usize;
    while index < tokens.len() {
        let token = tokens[index];
        if token.kind == TokenKind::Symbol {
            let (symbol, consumed) = read_symbol(tokens, index, source);
            match symbol {
                "<" => depth += 1,
                ">" => depth = depth.saturating_sub(1),
                ">>" => depth = depth.saturating_sub(2),
                ">>>" => depth = depth.saturating_sub(3),
                _ => {}
            }
            index += consumed;
            if depth == 0 {
                break;
            }
        } else {
            index += 1;
        }
    }
    index
}

pub(super) fn is_declaration_like_annotation(annotation_name: Option<&str>) -> bool {
    matches!(
        annotation_name,
        Some(
            "Deprecated"
                | "Override"
                | "SuppressWarnings"
                | "SafeVarargs"
                | "FunctionalInterface"
                | "Documented"
                | "Inherited"
                | "Repeatable"
                | "Retention"
                | "Target"
                | "Native"
        )
    )
}

pub(super) fn is_type_use_friendly_annotation(annotation_name: Option<&str>) -> bool {
    matches!(
        annotation_name,
        Some("Nullable" | "NonNull" | "Nonnull" | "CheckForNull" | "PolyNull" | "Untainted")
    )
}

pub(super) fn is_wrappable_invocation_open_paren(
    tokens: &[&Token],
    index: usize,
    source: &str,
    prev_kind: &Option<TokenKind>,
    prev_text: Option<&str>,
) -> bool {
    if is_explicit_type_argument_call(tokens, index, source) {
        return true;
    }

    if matches!(
        prev_text,
        Some("if" | "for" | "while" | "switch" | "catch" | "synchronized" | "try")
    ) {
        return false;
    }

    if !matches!(prev_kind, Some(TokenKind::Word)) && !matches!(prev_text, Some(">" | ">>" | ">>>"))
    {
        return false;
    }

    if index < 2 {
        return false;
    }

    match tokens[index - 2].kind {
        TokenKind::Symbol => {
            let (symbol, _) = read_symbol(tokens, index - 2, source);
            matches!(
                symbol,
                "." | "::"
                    | "{"
                    | ";"
                    | "("
                    | "["
                    | ","
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
                    | "?"
                    | ":"
                    | "->"
            )
        }
        TokenKind::Word => matches!(
            token_text(source, tokens[index - 2]),
            "return" | "throw" | "yield" | "case" | "new" | "assert"
        ),
        _ => false,
    }
}

pub(super) fn is_explicit_type_argument_call(
    tokens: &[&Token],
    index: usize,
    source: &str,
) -> bool {
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
                return matches!(symbol, ">" | ">>" | ">>>");
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

pub(super) fn should_break_assignment_rhs(
    tokens: &[&Token],
    mut index: usize,
    source: &str,
) -> bool {
    if let Some(next) = tokens.get(index)
        && next.kind == TokenKind::Word
        && token_text(source, next) == "switch"
    {
        return true;
    }

    let mut local_paren_depth = 0usize;
    let mut local_bracket_depth = 0usize;
    let mut top_level_dot_count = 0usize;
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
            match symbol {
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
                    top_level_dot_count += 1;
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
    let long_call_chain = dot_after_call_count >= 3 && scanned_chars >= 90;
    let long_generic_call = scanned_chars >= 90 && saw_top_level_method_call && saw_top_level_angle;
    let long_member_chain = top_level_dot_count >= 5 && scanned_chars >= 80;
    sorted_chain || long_call_chain || long_generic_call || long_member_chain
}

pub(super) fn next_member_looks_like_method(
    tokens: &[&Token],
    mut index: usize,
    source: &str,
) -> bool {
    let mut local_paren_depth = 0usize;
    let mut saw_signature_paren = false;

    while index < tokens.len() {
        let token = tokens[index];
        if token.kind == TokenKind::Symbol {
            let (symbol, consumed) = read_symbol(tokens, index, source);
            match symbol {
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

pub(super) fn next_symbol_text<'a>(
    tokens: &[&Token],
    mut index: usize,
    source: &'a str,
) -> Option<&'a str> {
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
        return Some(token_text(source, token));
    }
    index += 1;
    if index < tokens.len() {
        return Some(token_text(source, tokens[index]));
    }
    None
}

pub(super) fn read_symbol<'a>(
    tokens: &[&Token],
    index: usize,
    source: &'a str,
) -> (&'a str, usize) {
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
        (".", Some("."), Some(".")) if contiguous12 && contiguous23 => Some("..."),
        (">", Some(">"), Some(">")) if contiguous12 && contiguous23 => Some(">>>"),
        _ => None,
    };
    if let Some(op) = combined3 {
        let fourth = tokens.get(index + 3).map(|token| token_text(source, token));
        if matches!((op, fourth), (">>>", Some("="))) {
            return (">>>=", 4);
        }
        return (op, 3);
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
            return ("<<=", 3);
        }
        if matches!((op, third), (">>", Some("="))) {
            return (">>=", 3);
        }
        return (op, 2);
    }

    (first, 1)
}

pub(super) fn needs_space_before(
    prev_text: &Option<String>,
    curr_text: &str,
    at_line_start: bool,
) -> bool {
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

pub(super) fn needs_space_before_open_paren(prev_text: &Option<String>) -> bool {
    let Some(prev) = prev_text else {
        return false;
    };
    matches!(
        prev.as_str(),
        "if" | "for" | "while" | "switch" | "catch" | "synchronized" | "try"
    )
}

pub(super) fn is_generic_open_angle(
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

pub(super) fn looks_like_type_argument_list(
    tokens: &[&Token],
    mut index: usize,
    source: &str,
) -> bool {
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
                match symbol {
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

pub(super) fn is_tight_after_generic_close(
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

pub(super) fn is_inline_initializer_brace(
    tokens: &[&Token],
    index: usize,
    source: &str,
    prev_text: &Option<String>,
) -> bool {
    if !matches!(prev_text.as_deref(), Some("]" | "=")) {
        return false;
    }

    let mut depth = 0usize;
    let mut scanned_chars = 0usize;
    let mut top_level_comma_count = 0usize;
    let mut has_comment = false;
    let mut i = index;
    while i < tokens.len() {
        let token = tokens[i];
        scanned_chars += token.end.saturating_sub(token.start);
        if matches!(token.kind, TokenKind::LineComment | TokenKind::BlockComment) {
            has_comment = true;
        }
        if token.kind != TokenKind::Symbol {
            i += 1;
            continue;
        }
        let (symbol, consumed) = read_symbol(tokens, i, source);
        match symbol {
            "{" => depth += 1,
            "}" => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return !has_comment && top_level_comma_count <= 2 && scanned_chars <= 40;
                }
            }
            "," if depth == 1 => top_level_comma_count += 1,
            ";" if depth == 1 => return false,
            _ => {}
        }
        i += consumed;
    }
    false
}

pub(super) fn is_exponent_sign_context(prev_text: Option<&str>, next_text: Option<&str>) -> bool {
    let Some(prev) = prev_text else {
        return false;
    };
    let Some(next) = next_text else {
        return false;
    };

    let ends_with_exponent = prev.ends_with('e') || prev.ends_with('E');
    ends_with_exponent && next.chars().all(|ch| ch.is_ascii_digit() || ch == '_')
}

pub(super) fn should_break_before_chained_dot(out: &str) -> bool {
    let Some(line) = current_output_line(out) else {
        return false;
    };
    let trimmed = line.trim_start();
    if trimmed.starts_with('.') || trimmed.starts_with("return ") {
        return true;
    }
    line.chars().filter(|ch| *ch == '.').count() >= 2
}

pub(super) fn next_dotted_member_call_breaks(
    tokens: &[&Token],
    mut index: usize,
    source: &str,
) -> bool {
    while index < tokens.len() {
        let token = tokens[index];
        match token.kind {
            TokenKind::Word => {
                index += 1;
                break;
            }
            TokenKind::Symbol => {
                let (symbol, consumed) = read_symbol(tokens, index, source);
                if symbol != "<" {
                    return false;
                }
                index += consumed;
            }
            _ => index += 1,
        }
    }

    while index < tokens.len() {
        let token = tokens[index];
        if token.kind != TokenKind::Symbol {
            index += 1;
            continue;
        }
        let (symbol, consumed) = read_symbol(tokens, index, source);
        if symbol != "(" {
            return false;
        }
        return call_arguments_break_metrics(tokens, index + consumed, source).is_some_and(
            |metrics| {
                metrics.should_break_short()
                    || metrics.should_break_long()
                    || metrics.should_force_vertical()
                    || metrics.has_comment()
            },
        );
    }

    false
}

fn next_chain_member_call_name<'a>(
    tokens: &[&Token],
    mut index: usize,
    source: &'a str,
) -> Option<&'a str> {
    let mut member_name = None;
    while index < tokens.len() {
        let token = tokens[index];
        match token.kind {
            TokenKind::Word => {
                if member_name.is_none() {
                    member_name = Some(token_text(source, token));
                    index += 1;
                    continue;
                }
                return None;
            }
            TokenKind::Symbol => {
                let (symbol, consumed) = read_symbol(tokens, index, source);
                match symbol {
                    "<" => {
                        index += consumed;
                        continue;
                    }
                    "(" => return member_name,
                    _ => return None,
                }
            }
            _ => index += 1,
        }
    }
    None
}

fn is_chain_prefix_method(name: &str) -> bool {
    matches!(name, "stream" | "parallelStream" | "toBuilder")
}

pub(super) fn current_output_line(out: &str) -> Option<&str> {
    if out.is_empty() {
        return None;
    }
    Some(out.rsplit('\n').next().unwrap_or(out))
}

pub(super) fn is_word_like_text(text: &str) -> bool {
    text.chars().all(|ch| ch.is_alphanumeric() || ch == '_')
}

pub(super) fn is_declaration_modifier(text: &str) -> bool {
    matches!(
        text,
        "public"
            | "protected"
            | "private"
            | "static"
            | "final"
            | "abstract"
            | "default"
            | "native"
            | "strictfp"
            | "synchronized"
            | "transient"
            | "volatile"
            | "sealed"
            | "non"
    )
}

pub(super) fn is_primitive_or_void(text: &str) -> bool {
    matches!(
        text,
        "void" | "boolean" | "byte" | "short" | "int" | "long" | "char" | "float" | "double"
    )
}

pub(super) fn is_unary_prefix_context(prev_text: Option<&str>) -> bool {
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

pub(super) fn source_gap_has_newline(source: &str, start: usize, end: usize) -> bool {
    source[start..end].contains('\n')
}

pub(super) fn source_gap_has_blank_line(source: &str, start: usize, end: usize) -> bool {
    source[start..end].matches('\n').count() >= 2
}

pub(super) fn token_text<'a>(source: &'a str, token: &Token) -> &'a str {
    &source[token.start..token.end]
}
