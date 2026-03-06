pub(super) fn normalize_line_comment_text(text: &str) -> String {
    if let Some(content) = text.strip_prefix("//")
        && !content.is_empty()
        && !content.starts_with(' ')
    {
        return format!("// {content}");
    }
    text.to_owned()
}

pub(super) fn normalize_block_comment_text(text: &str) -> String {
    if text.contains('\n')
        || !text.starts_with("/*")
        || !text.ends_with("*/")
        || text.starts_with("/**")
    {
        return text.to_owned();
    }

    let inner = &text[2..text.len() - 2];
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return text.to_owned();
    }

    if trimmed.contains('=') {
        return format!("/* {trimmed} */");
    }

    text.to_owned()
}

pub(super) fn write_with_indent(
    out: &mut String,
    at_line_start: &mut bool,
    indent: usize,
    text: &str,
) {
    if *at_line_start {
        for _ in 0..indent {
            out.push_str("  ");
        }
        *at_line_start = false;
    }
    out.push_str(text);
}

pub(super) fn ensure_space(out: &mut String, at_line_start: bool) {
    if at_line_start {
        return;
    }
    if let Some(ch) = out.chars().last() {
        if ch != ' ' && ch != '\n' {
            out.push(' ');
        }
    }
}

pub(super) fn trim_redundant_blank_lines(input: &str) -> String {
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
