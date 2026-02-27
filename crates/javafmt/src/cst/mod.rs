use crate::lexer::LexedSource;

#[derive(Debug, Clone)]
pub struct Cst<'a> {
    pub source: &'a str,
    pub token_count: usize,
}

impl<'a> Cst<'a> {
    pub fn from_lexed(lexed: &LexedSource<'a>) -> Self {
        Self {
            source: lexed.source,
            token_count: lexed.tokens.len(),
        }
    }
}
