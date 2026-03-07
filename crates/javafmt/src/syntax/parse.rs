use super::tree::{FileOutline, ParsedFile, TopLevelItem, TopLevelItemKind};
use crate::lexer::TokenKind;
use crate::{comments, lexer};

pub(crate) fn parse_file(source: &str) -> ParsedFile<'_> {
    let lexed = lexer::lex(source);
    let comments = comments::attach(&lexed);
    let outline = build_outline(&lexed);

    ParsedFile {
        lexed,
        comments,
        outline,
    }
}

fn build_outline(lexed: &crate::lexer::LexedSource<'_>) -> FileOutline {
    let mut items = Vec::new();
    let mut index = 0usize;

    while let Some(start) = next_top_level_start(lexed, index) {
        let kind = classify_top_level_item(lexed, start);
        let end_token = match kind {
            TopLevelItemKind::Package | TopLevelItemKind::Import => {
                scan_statement_end(lexed, start)
            }
            _ => scan_top_level_item_end(lexed, start),
        };
        items.push(TopLevelItem {
            kind,
            start_token: start,
            end_token,
        });
        index = end_token.max(start + 1);
    }

    FileOutline { items }
}

fn next_top_level_start(lexed: &crate::lexer::LexedSource<'_>, mut index: usize) -> Option<usize> {
    while index < lexed.tokens.len() {
        let token = &lexed.tokens[index];
        match token.kind {
            TokenKind::Whitespace
            | TokenKind::Newline
            | TokenKind::LineComment
            | TokenKind::BlockComment => {
                index += 1;
            }
            _ => return Some(index),
        }
    }
    None
}

fn classify_top_level_item(
    lexed: &crate::lexer::LexedSource<'_>,
    start: usize,
) -> TopLevelItemKind {
    let mut index = start;
    while index < lexed.tokens.len() {
        let token = &lexed.tokens[index];
        match token.kind {
            TokenKind::Whitespace
            | TokenKind::Newline
            | TokenKind::LineComment
            | TokenKind::BlockComment => {
                index += 1;
            }
            TokenKind::Word => match token_text(lexed, index) {
                "package" => return TopLevelItemKind::Package,
                "import" => return TopLevelItemKind::Import,
                "class" | "interface" | "record" | "enum" => {
                    return TopLevelItemKind::TypeDeclaration;
                }
                "module" => return TopLevelItemKind::ModuleDeclaration,
                _ => index += 1,
            },
            TokenKind::Symbol => {
                if token_text(lexed, index) == ";" {
                    return TopLevelItemKind::Other;
                }
                index += 1;
            }
            _ => index += 1,
        }
    }

    TopLevelItemKind::Other
}

fn scan_statement_end(lexed: &crate::lexer::LexedSource<'_>, mut index: usize) -> usize {
    while index < lexed.tokens.len() {
        if lexed.tokens[index].kind == TokenKind::Symbol && token_text(lexed, index) == ";" {
            return index + 1;
        }
        index += 1;
    }
    lexed.tokens.len()
}

fn scan_top_level_item_end(lexed: &crate::lexer::LexedSource<'_>, mut index: usize) -> usize {
    let mut brace_depth = 0usize;
    while index < lexed.tokens.len() {
        if lexed.tokens[index].kind == TokenKind::Symbol {
            match token_text(lexed, index) {
                ";" if brace_depth == 0 => return index + 1,
                "{" => brace_depth += 1,
                "}" => {
                    brace_depth = brace_depth.saturating_sub(1);
                    if brace_depth == 0 {
                        return index + 1;
                    }
                }
                _ => {}
            }
        }
        index += 1;
    }
    lexed.tokens.len()
}

fn token_text<'a>(lexed: &'a crate::lexer::LexedSource<'_>, index: usize) -> &'a str {
    let token = &lexed.tokens[index];
    &lexed.source[token.start..token.end]
}
