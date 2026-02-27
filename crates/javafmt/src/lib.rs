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

    let maybe_reordered = {
        let lines = input.lines().collect::<Vec<_>>();
        if lines.is_empty() {
            None
        } else {
            let mut first_import_idx = None;
            let mut invalid_prefix = false;
            for (idx, line) in lines.iter().enumerate() {
                if line.starts_with("import ") {
                    first_import_idx = Some(idx);
                    break;
                }
                if line.is_empty() || line.starts_with("package ") {
                    continue;
                }
                invalid_prefix = true;
                break;
            }
            if invalid_prefix {
                None
            } else if let Some(start) = first_import_idx {
                let mut end = start;
                let mut has_comment_or_other = false;
                while end < lines.len() {
                    let line = lines[end];
                    if line.starts_with("import ") || line.is_empty() {
                        end += 1;
                        continue;
                    }
                    has_comment_or_other = line.starts_with("//")
                        || line.starts_with("/*")
                        || line.starts_with('*')
                        || line.starts_with("*/");
                    break;
                }
                if has_comment_or_other || end == start {
                    None
                } else {
                    let import_lines = lines[start..end]
                        .iter()
                        .copied()
                        .filter(|line| !line.is_empty())
                        .collect::<Vec<_>>();
                    if import_lines.is_empty() {
                        None
                    } else {
                        let mut static_imports = import_lines
                            .iter()
                            .copied()
                            .filter(|line| line.starts_with("import static "))
                            .collect::<Vec<_>>();
                        let mut normal_imports = import_lines
                            .iter()
                            .copied()
                            .filter(|line| !line.starts_with("import static "))
                            .collect::<Vec<_>>();
                        static_imports.sort_unstable();
                        normal_imports.sort_unstable();

                        let mut out = String::with_capacity(input.len() + 4);
                        let mut first_line = true;
                        let mut push_line = |line: &str| {
                            if !first_line {
                                out.push('\n');
                            }
                            out.push_str(line);
                            first_line = false;
                        };

                        for line in &lines[..start] {
                            push_line(line);
                        }
                        for line in &static_imports {
                            push_line(line);
                        }
                        if !static_imports.is_empty() && !normal_imports.is_empty() {
                            push_line("");
                        }
                        for line in &normal_imports {
                            push_line(line);
                        }

                        if end < lines.len() {
                            push_line("");
                            let mut suffix_start = end;
                            while suffix_start < lines.len() && lines[suffix_start].is_empty() {
                                suffix_start += 1;
                            }
                            for line in &lines[suffix_start..] {
                                push_line(line);
                            }
                        }

                        if input.ends_with('\n') {
                            out.push('\n');
                        }
                        if out != input { Some(out) } else { None }
                    }
                }
            } else {
                None
            }
        }
    };

    maybe_reordered.unwrap_or(input)
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
    use super::format_str;

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
}
