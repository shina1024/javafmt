use crate::comments::CommentAttachment;
use crate::lexer::{LexedSource, Token};

#[derive(Debug, Clone)]
pub struct FormatIr<'a> {
    pub source: &'a str,
    pub tokens: Vec<Token>,
    pub line_comment_count: usize,
    pub block_comment_count: usize,
}

pub fn build<'a>(lexed: &LexedSource<'a>, comments: CommentAttachment) -> FormatIr<'a> {
    FormatIr {
        source: lexed.source,
        tokens: lexed.tokens.clone(),
        line_comment_count: comments.line_comment_count,
        block_comment_count: comments.block_comment_count,
    }
}
