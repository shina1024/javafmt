use crate::lexer::{LexedSource, Token};

#[derive(Debug, Clone)]
pub struct Cst<'a> {
    pub source: &'a str,
    pub tokens: Vec<Token>,
}

impl<'a> Cst<'a> {
    pub fn from_lexed(lexed: &LexedSource<'a>) -> Self {
        Self {
            source: lexed.source,
            tokens: lexed.tokens.clone(),
        }
    }
}
