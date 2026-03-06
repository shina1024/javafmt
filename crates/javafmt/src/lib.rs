pub mod bench_support;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineEnding {
    Lf,
    Crlf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ImportBlock {
    start: usize,
    end: usize,
    suffix_start: usize,
}

pub fn format_str(input: &str) -> FormatResult {
    let line_ending = detect_line_ending(input);
    let normalized = normalize_newlines(input);
    let lexed = lexer::lex(&normalized);
    let cst = parser::parse(&lexed);
    let attachments = comments::attach(&cst, &lexed);
    let format_ir = ir::build(&cst, attachments);
    let printed = printer::print(&format_ir);
    let emitted = emit::emit(printed);
    let output = apply_line_ending_policy(reorder_top_level_imports(emitted), line_ending);
    let changed = output != input;
    FormatResult { output, changed }
}

fn reorder_top_level_imports(input: String) -> String {
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

#[cfg(test)]
mod tests {
    use super::{format_str, reorder_top_level_imports};

    #[test]
    fn keeps_formatted_text_unchanged() {
        let input = "class A {}\n";
        let result = format_str(input);
        assert_eq!(result.output, input);
        assert!(!result.changed);
    }

    #[test]
    fn preserves_crlf_if_input_uses_crlf() {
        let input = "class A {}\r\n";
        let result = format_str(input);
        assert_eq!(result.output, "class A {}\r\n");
        assert!(!result.changed);
    }

    #[test]
    fn mixed_newlines_fall_back_to_lf_output() {
        let input = "class A {\r\n}\n";
        let result = format_str(input);
        assert!(!result.output.contains('\r'));
        assert!(result.output.ends_with('\n'));
        assert!(result.changed);
    }

    #[test]
    fn keeps_text_block_intact() {
        let input = "class A{String f(){return \"\"\"\nline1\nline2\n\"\"\";}}\n";
        let result = format_str(input);
        assert!(result.output.contains("\"\"\"\nline1\nline2\n\"\"\""));
    }

    #[test]
    fn appends_trailing_newline() {
        let input = "class A {}";
        let result = format_str(input);
        assert_eq!(result.output, "class A {}\n");
        assert!(result.changed);
    }

    #[test]
    fn trims_trailing_whitespace() {
        let input = "class A {}   \n";
        let result = format_str(input);
        assert_eq!(result.output, "class A {}\n");
        assert!(result.changed);
    }

    #[test]
    fn sorts_static_imports_before_normal_imports() {
        let input = "package p;\nimport java.util.List;\nimport static java.util.Collections.emptyList;\nclass A{List<String> xs=emptyList();}\n";
        let result = format_str(input);
        assert!(
            result.output.contains(
                "import static java.util.Collections.emptyList;\n\nimport java.util.List;"
            )
        );
    }

    #[test]
    fn sorts_imports_lexicographically_within_groups() {
        let input = "package p;\nimport java.util.List;\nimport java.util.Date;\nimport static java.util.Collections.singletonList;\nimport static java.util.Collections.emptyList;\nclass A{List<Date> a=emptyList();List<Date> b=singletonList(new Date());}\n";
        let result = format_str(input);
        assert!(result.output.contains("import static java.util.Collections.emptyList;\nimport static java.util.Collections.singletonList;"));
        assert!(
            result
                .output
                .contains("import java.util.Date;\nimport java.util.List;")
        );
    }

    #[test]
    fn keeps_import_order_when_comments_are_in_import_block() {
        let input = "package p;\nimport java.util.List;\n// c\nimport static java.util.Collections.emptyList;\nclass A{List<String> xs=emptyList();}\n";
        let result = format_str(input);
        assert!(
            result
                .output
                .contains("import java.util.List;\n// c\nimport static")
        );
    }

    #[test]
    fn import_reorder_skips_non_package_prefix_content() {
        let input = "class A {}\nimport java.util.List;\n";
        assert_eq!(reorder_top_level_imports(input.to_owned()), input);
    }

    #[test]
    fn import_reorder_preserves_comment_adjacent_suffix() {
        let input = "package p;\nimport java.util.List;\n\n// keep with the type\nclass A {}\n";
        assert_eq!(reorder_top_level_imports(input.to_owned()), input);
    }

    #[test]
    fn import_reorder_rebuilds_prefix_groups_and_suffix() {
        let input = "package p;\n\nimport java.util.List;\nimport static java.util.Collections.emptyList;\n\n\nclass A {}\n";
        let expected = "package p;\n\nimport static java.util.Collections.emptyList;\n\nimport java.util.List;\n\nclass A {}\n";
        assert_eq!(reorder_top_level_imports(input.to_owned()), expected);
    }
}
