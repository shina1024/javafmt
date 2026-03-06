#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Doc {
    Nil,
    Text(String),
    Line(LineMode),
    Concat(Vec<Doc>),
    Group(Box<Doc>),
    Indent(usize, Box<Doc>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineMode {
    Soft,
    Hard,
}

impl Doc {
    pub(crate) fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    pub(crate) fn soft_line() -> Self {
        Self::Line(LineMode::Soft)
    }

    pub(crate) fn hard_line() -> Self {
        Self::Line(LineMode::Hard)
    }

    pub(crate) fn concat<I>(docs: I) -> Self
    where
        I: IntoIterator<Item = Doc>,
    {
        let docs = docs.into_iter().collect::<Vec<_>>();
        match docs.as_slice() {
            [] => Self::Nil,
            [single] => single.clone(),
            _ => Self::Concat(docs),
        }
    }

    pub(crate) fn group(doc: Doc) -> Self {
        Self::Group(Box::new(doc))
    }

    pub(crate) fn indent(spaces: usize, doc: Doc) -> Self {
        Self::Indent(spaces, Box::new(doc))
    }
}
