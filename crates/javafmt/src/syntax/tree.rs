use crate::comments::CommentAttachment;
use crate::cst::Cst;

#[derive(Debug, Clone)]
pub(crate) struct ParsedFile<'a> {
    pub(crate) cst: Cst<'a>,
    pub(crate) comments: CommentAttachment,
}
