use crate::comments::CommentAttachment;
use crate::lexer::LexedSource;

#[derive(Debug, Clone)]
pub(crate) struct FileOutline {
    pub(crate) items: Vec<TopLevelItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TopLevelItemKind {
    Package,
    Import,
    TypeDeclaration,
    ModuleDeclaration,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TopLevelItem {
    pub(crate) kind: TopLevelItemKind,
    pub(crate) start_token: usize,
    pub(crate) end_token: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedFile<'a> {
    pub(crate) lexed: LexedSource<'a>,
    pub(crate) comments: CommentAttachment,
    pub(crate) outline: FileOutline,
}
