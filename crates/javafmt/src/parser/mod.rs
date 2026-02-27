use crate::cst::Cst;
use crate::lexer::LexedSource;

pub fn parse<'a>(lexed: &LexedSource<'a>) -> Cst<'a> {
    Cst::from_lexed(lexed)
}
