use super::tree::{FileOutline, ParsedFile, TopLevelItem, TopLevelItemKind};
use crate::lexer::TokenKind;
use crate::{comments, lexer, parser};

pub(crate) fn parse(source: &str) -> ParsedFile<'_> {
    let lexed = lexer::lex(source);
    let cst = parser::parse(&lexed);
    let comments = comments::attach(&cst, &lexed);
    let outline = build_outline(&cst);

    let _ = lexed;

    ParsedFile {
        cst,
        comments,
        outline,
    }
}

fn build_outline(cst: &crate::cst::Cst<'_>) -> FileOutline {
    let mut items = Vec::new();
    let mut index = 0usize;

    while let Some(start) = next_top_level_start(cst, index) {
        let kind = classify_top_level_item(cst, start);
        let end_token = match kind {
            TopLevelItemKind::Package | TopLevelItemKind::Import => scan_statement_end(cst, start),
            _ => scan_top_level_item_end(cst, start),
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

fn next_top_level_start(cst: &crate::cst::Cst<'_>, mut index: usize) -> Option<usize> {
    while index < cst.tokens.len() {
        let token = &cst.tokens[index];
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

fn classify_top_level_item(cst: &crate::cst::Cst<'_>, start: usize) -> TopLevelItemKind {
    let mut index = start;
    while index < cst.tokens.len() {
        let token = &cst.tokens[index];
        match token.kind {
            TokenKind::Whitespace
            | TokenKind::Newline
            | TokenKind::LineComment
            | TokenKind::BlockComment => {
                index += 1;
            }
            TokenKind::Word => match token_text(cst, index) {
                "package" => return TopLevelItemKind::Package,
                "import" => return TopLevelItemKind::Import,
                "class" | "interface" | "record" | "enum" => {
                    return TopLevelItemKind::TypeDeclaration;
                }
                "module" => return TopLevelItemKind::ModuleDeclaration,
                _ => index += 1,
            },
            TokenKind::Symbol => {
                if token_text(cst, index) == ";" {
                    return TopLevelItemKind::Other;
                }
                index += 1;
            }
            _ => index += 1,
        }
    }

    TopLevelItemKind::Other
}

fn scan_statement_end(cst: &crate::cst::Cst<'_>, mut index: usize) -> usize {
    while index < cst.tokens.len() {
        if cst.tokens[index].kind == TokenKind::Symbol && token_text(cst, index) == ";" {
            return index + 1;
        }
        index += 1;
    }
    cst.tokens.len()
}

fn scan_top_level_item_end(cst: &crate::cst::Cst<'_>, mut index: usize) -> usize {
    let mut brace_depth = 0usize;
    while index < cst.tokens.len() {
        if cst.tokens[index].kind == TokenKind::Symbol {
            match token_text(cst, index) {
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
    cst.tokens.len()
}

fn token_text<'a>(cst: &'a crate::cst::Cst<'_>, index: usize) -> &'a str {
    let token = &cst.tokens[index];
    &cst.source[token.start..token.end]
}
