const JAVADOC_MAX_WIDTH: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
enum JavadocBlock {
    Paragraph {
        text: String,
        needs_paragraph_tag: bool,
    },
    RawLines(Vec<String>),
}

pub(super) fn normalize_line_comment_text(text: &str) -> String {
    if let Some(content) = text.strip_prefix("//")
        && !content.is_empty()
        && !content.starts_with(' ')
    {
        return format!("// {content}");
    }
    text.to_owned()
}

pub(super) fn normalize_block_comment_text(text: &str, indent: usize) -> String {
    if text.starts_with("/**") && text.contains('\n') {
        return format_javadoc_comment(text, indent);
    }

    if text.contains('\n') || !text.starts_with("/*") || !text.ends_with("*/") {
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

fn format_javadoc_comment(text: &str, indent: usize) -> String {
    let Some(inner) = text
        .strip_prefix("/**")
        .and_then(|text| text.strip_suffix("*/"))
    else {
        return text.to_owned();
    };

    let mut lines = inner
        .lines()
        .map(strip_javadoc_line_prefix)
        .collect::<Vec<_>>();
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }

    if lines.is_empty() {
        return text.to_owned();
    }

    let blocks = parse_javadoc_blocks(&lines);
    if blocks.is_empty() {
        return text.to_owned();
    }

    let indent_text = "  ".repeat(indent);
    let content_width = JAVADOC_MAX_WIDTH.saturating_sub(indent * 2 + 3).max(20);
    let mut out = String::from("/**");

    for (index, block) in blocks.iter().enumerate() {
        if index == 0 {
            out.push('\n');
        } else {
            out.push_str(&indent_text);
            out.push_str(" *");
            out.push('\n');
        }
        match block {
            JavadocBlock::Paragraph {
                text,
                needs_paragraph_tag,
            } => append_wrapped_javadoc_paragraph(
                &mut out,
                &indent_text,
                text,
                *needs_paragraph_tag,
                content_width,
            ),
            JavadocBlock::RawLines(lines) => {
                for line in lines {
                    out.push_str(&indent_text);
                    out.push_str(" *");
                    if !line.is_empty() {
                        out.push(' ');
                        out.push_str(line);
                    }
                    out.push('\n');
                }
            }
        }
    }

    out.push_str(&indent_text);
    out.push_str(" */");
    out
}

fn strip_javadoc_line_prefix(line: &str) -> String {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix('*') {
        return rest.strip_prefix(' ').unwrap_or(rest).trim_end().to_owned();
    }
    trimmed.trim_end().to_owned()
}

fn parse_javadoc_blocks(lines: &[String]) -> Vec<JavadocBlock> {
    let mut blocks = Vec::new();
    let mut paragraph = String::new();
    let mut paragraph_needs_tag = false;
    let mut raw_lines = Vec::new();
    let mut in_preformatted = false;
    let mut pending_separator = false;

    for line in lines {
        let trimmed = line.trim();

        if in_preformatted {
            raw_lines.push(line.trim_end().to_owned());
            if trimmed.starts_with("</pre>") {
                in_preformatted = false;
            }
            continue;
        }

        if trimmed.is_empty() || trimmed == "<p>" {
            flush_javadoc_paragraph(&mut blocks, &mut paragraph, paragraph_needs_tag);
            paragraph_needs_tag = false;
            flush_javadoc_raw_lines(&mut blocks, &mut raw_lines);
            pending_separator = true;
            continue;
        }

        if is_raw_javadoc_line(trimmed) {
            flush_javadoc_paragraph(&mut blocks, &mut paragraph, paragraph_needs_tag);
            paragraph_needs_tag = false;
            raw_lines.push(normalize_raw_javadoc_line(line));
            if trimmed.starts_with("<pre>") {
                in_preformatted = true;
            }
            pending_separator = false;
            continue;
        }

        flush_javadoc_raw_lines(&mut blocks, &mut raw_lines);

        let (explicit_paragraph_tag, paragraph_line) =
            if let Some(rest) = trimmed.strip_prefix("<p>") {
                (true, rest.trim_start())
            } else {
                (false, trimmed)
            };
        if paragraph.is_empty() {
            paragraph_needs_tag = explicit_paragraph_tag || pending_separator || !blocks.is_empty();
            paragraph.push_str(paragraph_line);
        } else if explicit_paragraph_tag || pending_separator {
            flush_javadoc_paragraph(&mut blocks, &mut paragraph, paragraph_needs_tag);
            paragraph_needs_tag = true;
            paragraph.push_str(paragraph_line);
        } else {
            paragraph.push(' ');
            paragraph.push_str(paragraph_line);
        }
        pending_separator = false;
    }

    flush_javadoc_paragraph(&mut blocks, &mut paragraph, paragraph_needs_tag);
    flush_javadoc_raw_lines(&mut blocks, &mut raw_lines);
    blocks
}

fn flush_javadoc_paragraph(
    blocks: &mut Vec<JavadocBlock>,
    paragraph: &mut String,
    needs_paragraph_tag: bool,
) {
    if paragraph.is_empty() {
        return;
    }
    blocks.push(JavadocBlock::Paragraph {
        text: std::mem::take(paragraph),
        needs_paragraph_tag,
    });
}

fn flush_javadoc_raw_lines(blocks: &mut Vec<JavadocBlock>, raw_lines: &mut Vec<String>) {
    if raw_lines.is_empty() {
        return;
    }
    blocks.push(JavadocBlock::RawLines(std::mem::take(raw_lines)));
}

fn is_raw_javadoc_line(trimmed: &str) -> bool {
    trimmed.starts_with("<ul>")
        || trimmed.starts_with("</ul>")
        || trimmed.starts_with("<ol>")
        || trimmed.starts_with("</ol>")
        || trimmed.starts_with("<li>")
        || trimmed.starts_with("<pre>")
        || trimmed.starts_with("</pre>")
        || trimmed.starts_with("<table>")
        || trimmed.starts_with("</table>")
        || trimmed.starts_with("@")
}

fn normalize_raw_javadoc_line(line: &str) -> String {
    let trimmed = line.trim_end();
    if let Some(rest) = trimmed.trim_start().strip_prefix("<li>") {
        return format!("  <li>{}", rest.trim_start());
    }
    trimmed.to_owned()
}

fn append_wrapped_javadoc_paragraph(
    out: &mut String,
    indent_text: &str,
    text: &str,
    needs_paragraph_tag: bool,
    content_width: usize,
) {
    let mut words = text.split_whitespace();
    let Some(first_word) = words.next() else {
        return;
    };

    let mut current = String::new();
    if needs_paragraph_tag {
        current.push_str("<p>");
    }
    current.push_str(first_word);

    for word in words {
        let next_len = current.chars().count() + 1 + word.chars().count();
        if next_len > content_width {
            out.push_str(indent_text);
            out.push_str(" * ");
            out.push_str(&current);
            out.push('\n');
            current.clear();
            current.push_str(word);
        } else {
            current.push(' ');
            current.push_str(word);
        }
    }

    out.push_str(indent_text);
    out.push_str(" * ");
    out.push_str(&current);
    out.push('\n');
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
    if let Some(ch) = out.chars().last()
        && ch != ' '
        && ch != '\n'
    {
        out.push(' ');
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
