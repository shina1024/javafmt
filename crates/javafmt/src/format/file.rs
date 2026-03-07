use super::doc::Doc;
use super::tokens;
use crate::lexer::TokenKind;
use crate::syntax::{ParsedFile, TopLevelItemKind};

pub(crate) fn format_file(parsed: &ParsedFile<'_>) -> Doc {
    if let Some(doc) = format_package_import_file(parsed) {
        return doc;
    }

    // Keep soft-line primitives compiled into the production path until the
    // structured formatter starts emitting width-sensitive groups.
    let _softline_scaffold = Doc::soft_line();
    Doc::group(Doc::indent(0, tokens::format_doc(parsed)))
}

fn format_package_import_file(parsed: &ParsedFile<'_>) -> Option<Doc> {
    if parsed.comments.line_comment_count > 0 || parsed.comments.block_comment_count > 0 {
        return None;
    }

    if parsed.outline.items.is_empty() {
        return None;
    }

    let mut package_doc = None;
    let mut import_docs = Vec::new();

    for item in &parsed.outline.items {
        match item.kind {
            TopLevelItemKind::Package => {
                if package_doc.is_some() {
                    return None;
                }
                package_doc = Some(format_package_item(
                    parsed,
                    item.start_token,
                    item.end_token,
                )?);
            }
            TopLevelItemKind::Import => {
                import_docs.push(format_import_item(
                    parsed,
                    item.start_token,
                    item.end_token,
                )?);
            }
            _ => return None,
        }
    }

    let mut docs = Vec::new();
    if let Some(package_doc) = package_doc {
        docs.push(package_doc);
        if !import_docs.is_empty() {
            docs.push(Doc::hard_line());
            docs.push(Doc::hard_line());
        } else {
            docs.push(Doc::hard_line());
        }
    }

    for import_doc in import_docs {
        docs.push(import_doc);
        docs.push(Doc::hard_line());
    }

    Some(Doc::concat(docs))
}

fn format_package_item(
    parsed: &ParsedFile<'_>,
    start_token: usize,
    end_token: usize,
) -> Option<Doc> {
    let tokens = meaningful_tokens(parsed, start_token, end_token);
    if tokens.len() < 3
        || token_text(parsed, tokens[0]) != "package"
        || token_text(parsed, *tokens.last()?) != ";"
    {
        return None;
    }

    let path = join_token_texts(parsed, &tokens[1..tokens.len() - 1]);
    if path.is_empty() {
        return None;
    }

    Some(Doc::concat([
        Doc::text("package "),
        Doc::text(path),
        Doc::text(";"),
    ]))
}

fn format_import_item(
    parsed: &ParsedFile<'_>,
    start_token: usize,
    end_token: usize,
) -> Option<Doc> {
    let tokens = meaningful_tokens(parsed, start_token, end_token);
    if tokens.len() < 3
        || token_text(parsed, tokens[0]) != "import"
        || token_text(parsed, *tokens.last()?) != ";"
    {
        return None;
    }

    let mut prefix = "import ";
    let body_start = if token_text(parsed, tokens[1]) == "static" {
        prefix = "import static ";
        2
    } else {
        1
    };

    let path = join_token_texts(parsed, &tokens[body_start..tokens.len() - 1]);
    if path.is_empty() {
        return None;
    }

    Some(Doc::concat([
        Doc::text(prefix),
        Doc::text(path),
        Doc::text(";"),
    ]))
}

fn meaningful_tokens(parsed: &ParsedFile<'_>, start_token: usize, end_token: usize) -> Vec<usize> {
    (start_token..end_token)
        .filter(|index| {
            !matches!(
                parsed.lexed.tokens[*index].kind,
                TokenKind::Whitespace | TokenKind::Newline
            )
        })
        .collect::<Vec<_>>()
}

fn join_token_texts(parsed: &ParsedFile<'_>, token_indexes: &[usize]) -> String {
    let mut out = String::new();
    for token_index in token_indexes {
        out.push_str(token_text(parsed, *token_index));
    }
    out
}

fn token_text<'a>(parsed: &'a ParsedFile<'_>, token_index: usize) -> &'a str {
    let token = &parsed.lexed.tokens[token_index];
    &parsed.lexed.source[token.start..token.end]
}
