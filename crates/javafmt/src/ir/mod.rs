use crate::comments::CommentAttachment;
use crate::cst::Cst;

#[derive(Debug, Clone)]
pub struct FormatIr<'a> {
    pub source: &'a str,
    pub token_count: usize,
    pub line_comment_count: usize,
    pub block_comment_count: usize,
}

pub fn build<'a>(cst: &Cst<'a>, comments: CommentAttachment) -> FormatIr<'a> {
    FormatIr {
        source: cst.source,
        token_count: cst.token_count,
        line_comment_count: comments.line_comment_count,
        block_comment_count: comments.block_comment_count,
    }
}
