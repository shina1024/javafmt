use crate::cst::Cst;
use crate::lexer::LexedSource;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CommentAttachment {
    pub line_comment_count: usize,
}

pub fn attach(cst: &Cst<'_>, _lexed: &LexedSource<'_>) -> CommentAttachment {
    CommentAttachment {
        line_comment_count: cst.source.match_indices("//").count(),
    }
}
