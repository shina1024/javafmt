use crate::cst::Cst;
use crate::lexer::{LexedSource, TokenKind};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CommentAttachment {
    pub line_comment_count: usize,
    pub block_comment_count: usize,
}

pub fn attach(cst: &Cst<'_>, lexed: &LexedSource<'_>) -> CommentAttachment {
    let _ = cst.token_count;
    let mut line_comment_count = 0;
    let mut block_comment_count = 0;
    for token in &lexed.tokens {
        match token.kind {
            TokenKind::LineComment => line_comment_count += 1,
            TokenKind::BlockComment => block_comment_count += 1,
            _ => {}
        }
    }

    CommentAttachment {
        line_comment_count,
        block_comment_count,
    }
}
