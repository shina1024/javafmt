#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineEnding {
    Lf,
    Crlf,
}

#[derive(Debug, Clone)]
pub(crate) struct NormalizedInput {
    pub(crate) source: String,
    pub(crate) line_ending: LineEnding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ImportBlock {
    start: usize,
    end: usize,
    suffix_start: usize,
}

pub(crate) fn normalize_input(input: &str) -> NormalizedInput {
    NormalizedInput {
        source: normalize_newlines(input),
        line_ending: detect_line_ending(input),
    }
}

pub(crate) fn finalize_output(output: String, line_ending: LineEnding) -> String {
    apply_line_ending_policy(reorder_top_level_imports(output), line_ending)
}

pub(crate) fn reorder_top_level_imports(input: String) -> String {
    if !input.contains("import ") {
        return input;
    }

    try_reorder_top_level_imports(&input).unwrap_or(input)
}

fn try_reorder_top_level_imports(input: &str) -> Option<String> {
    let lines = input.lines().collect::<Vec<_>>();
    let import_block = find_import_block(&lines)?;
    let (mut static_imports, mut normal_imports) =
        split_import_groups(&lines[import_block.start..import_block.end])?;
    static_imports.sort_unstable();
    normal_imports.sort_unstable();

    let reordered = rebuild_reordered_imports(
        &lines,
        input.len(),
        input.ends_with('\n'),
        import_block,
        &static_imports,
        &normal_imports,
    );

    (reordered != input).then_some(reordered)
}

fn find_import_block(lines: &[&str]) -> Option<ImportBlock> {
    if lines.is_empty() {
        return None;
    }

    let start = lines.iter().position(|line| line.starts_with("import "))?;
    if !lines[..start]
        .iter()
        .all(|line| line.is_empty() || line.starts_with("package "))
    {
        return None;
    }

    let end = scan_import_block_end(lines, start)?;
    Some(ImportBlock {
        start,
        end,
        suffix_start: skip_blank_lines(lines, end),
    })
}

fn scan_import_block_end(lines: &[&str], start: usize) -> Option<usize> {
    let mut end = start;
    while end < lines.len() {
        let line = lines[end];
        if line.starts_with("import ") || line.is_empty() {
            end += 1;
            continue;
        }

        return if is_import_block_comment_line(line) {
            None
        } else {
            Some(end)
        };
    }

    (end > start).then_some(end)
}

fn is_import_block_comment_line(line: &str) -> bool {
    line.starts_with("//")
        || line.starts_with("/*")
        || line.starts_with('*')
        || line.starts_with("*/")
}

fn split_import_groups<'a>(import_block_lines: &[&'a str]) -> Option<(Vec<&'a str>, Vec<&'a str>)> {
    let import_lines = import_block_lines
        .iter()
        .copied()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if import_lines.is_empty() {
        return None;
    }

    let mut static_imports = Vec::new();
    let mut normal_imports = Vec::new();
    for line in import_lines {
        if line.starts_with("import static ") {
            static_imports.push(line);
        } else {
            normal_imports.push(line);
        }
    }

    Some((static_imports, normal_imports))
}

fn rebuild_reordered_imports(
    lines: &[&str],
    input_len: usize,
    preserve_trailing_newline: bool,
    import_block: ImportBlock,
    static_imports: &[&str],
    normal_imports: &[&str],
) -> String {
    let mut out = String::with_capacity(input_len + 4);
    let mut first_line = true;

    for line in &lines[..import_block.start] {
        push_output_line(&mut out, &mut first_line, line);
    }
    for line in static_imports {
        push_output_line(&mut out, &mut first_line, line);
    }
    if !static_imports.is_empty() && !normal_imports.is_empty() {
        push_output_line(&mut out, &mut first_line, "");
    }
    for line in normal_imports {
        push_output_line(&mut out, &mut first_line, line);
    }

    if import_block.end < lines.len() {
        push_output_line(&mut out, &mut first_line, "");
        for line in &lines[import_block.suffix_start..] {
            push_output_line(&mut out, &mut first_line, line);
        }
    }

    if preserve_trailing_newline {
        out.push('\n');
    }

    out
}

fn push_output_line(out: &mut String, first_line: &mut bool, line: &str) {
    if !*first_line {
        out.push('\n');
    }
    out.push_str(line);
    *first_line = false;
}

fn skip_blank_lines(lines: &[&str], mut index: usize) -> usize {
    while index < lines.len() && lines[index].is_empty() {
        index += 1;
    }
    index
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
