pub(super) fn finalize_declarations(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending_blank_line = false;

    for raw_line in text.lines() {
        if raw_line.is_empty() {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push('\n');
            continue;
        }

        let line = wrap_multivariable_declaration_line(&normalize_modifier_order(raw_line))
            .unwrap_or_else(|| normalize_modifier_order(raw_line));

        for segment in line.lines() {
            let trimmed = segment.trim();

            if trimmed == "}" && out.ends_with("\n\n") {
                out.pop();
            }

            if starts_member_javadoc(trimmed)
                && ends_member_declaration(previous_nonempty_line(&out))
            {
                pending_blank_line = true;
            }

            if pending_blank_line && !out.is_empty() && !out.ends_with("\n\n") {
                out.push('\n');
            }
            pending_blank_line = false;

            out.push_str(segment);
            out.push('\n');
        }
    }

    out
}

fn normalize_modifier_order(line: &str) -> String {
    [
        ("non-sealed private ", "private non-sealed "),
        ("non-sealed protected ", "protected non-sealed "),
        ("non-sealed public ", "public non-sealed "),
        ("sealed private ", "private sealed "),
        ("sealed protected ", "protected sealed "),
        ("sealed public ", "public sealed "),
    ]
    .into_iter()
    .fold(line.to_owned(), |current, (from, to)| {
        current.replace(from, to)
    })
}

fn wrap_multivariable_declaration_line(line: &str) -> Option<String> {
    if line.chars().count() < 100
        || !line.trim_end().ends_with(';')
        || line.contains("//")
        || line.contains("/*")
        || line.contains('=')
        || line.contains("->")
        || line.contains('?')
        || line.contains(':')
        || line.contains('(')
        || line.contains(')')
        || line.contains('{')
        || line.contains('}')
    {
        return None;
    }

    let mut angle_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut comma_indexes = Vec::new();
    for (index, ch) in line.char_indices() {
        match ch {
            '<' => angle_depth += 1,
            '>' => angle_depth = angle_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            ',' if angle_depth == 0 && bracket_depth == 0 => comma_indexes.push(index),
            _ => {}
        }
    }

    if comma_indexes.is_empty() {
        return None;
    }

    let trimmed = line.trim_start();
    let indent_width = line.len().saturating_sub(trimmed.len());
    let continuation_indent = " ".repeat(indent_width + 4);

    let mut out = String::with_capacity(line.len() + comma_indexes.len() * (indent_width + 5));
    let mut start = 0usize;
    for (position, comma_index) in comma_indexes.iter().enumerate() {
        let end = comma_index + 1;
        if position == 0 {
            out.push_str(line[start..end].trim_end());
        } else {
            out.push_str(&continuation_indent);
            out.push_str(line[start..end].trim());
        }
        out.push('\n');
        start = end;
    }
    out.push_str(&continuation_indent);
    out.push_str(line[start..].trim());
    Some(out)
}

fn starts_member_javadoc(trimmed_line: &str) -> bool {
    trimmed_line.starts_with("/**")
}

fn ends_member_declaration(previous_line: Option<&str>) -> bool {
    previous_line.is_some_and(|line| {
        let trimmed = line.trim();
        trimmed.ends_with(';')
            || trimmed.ends_with("}")
            || (trimmed.ends_with("{") && !is_type_declaration_header_line(trimmed))
    })
}

fn previous_nonempty_line(text: &str) -> Option<&str> {
    text.lines().rev().find(|line| !line.trim().is_empty())
}

fn is_type_declaration_header_line(line: &str) -> bool {
    let trimmed = line.trim().trim_end_matches('{').trim_end();
    trimmed.contains("@interface")
        || trimmed
            .split_whitespace()
            .any(|word| matches!(word, "class" | "interface" | "enum" | "record"))
}

#[cfg(test)]
mod tests {
    use super::finalize_declarations;

    #[test]
    fn reorders_access_modifier_before_sealed() {
        let input = "class T {\n  sealed private interface A permits I {}\n}\n";
        let expected = "class T {\n  private sealed interface A permits I {}\n}\n";
        assert_eq!(finalize_declarations(input), expected);
    }

    #[test]
    fn wraps_long_multivariable_declaration() {
        let input = "class T {\n  int xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx, yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy, zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz;\n}\n";
        let expected = "class T {\n  int xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx,\n      yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy,\n      zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz;\n}\n";
        assert_eq!(finalize_declarations(input), expected);
    }
}
