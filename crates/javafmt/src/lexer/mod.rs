#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Newline,
    Whitespace,
    Word,
    Symbol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone)]
pub struct LexedSource<'a> {
    pub source: &'a str,
    pub tokens: Vec<Token>,
}

pub fn lex(source: &str) -> LexedSource<'_> {
    let mut tokens = Vec::new();
    let mut chars = source.char_indices().peekable();

    while let Some((start, ch)) = chars.next() {
        if ch == '\n' {
            tokens.push(Token {
                kind: TokenKind::Newline,
                start,
                end: start + ch.len_utf8(),
            });
            continue;
        }

        if ch.is_whitespace() {
            let mut end = start + ch.len_utf8();
            while let Some((next_idx, next_ch)) = chars.peek().copied() {
                if next_ch == '\n' || !next_ch.is_whitespace() {
                    break;
                }
                chars.next();
                end = next_idx + next_ch.len_utf8();
            }
            tokens.push(Token {
                kind: TokenKind::Whitespace,
                start,
                end,
            });
            continue;
        }

        if is_word_char(ch) {
            let mut end = start + ch.len_utf8();
            while let Some((next_idx, next_ch)) = chars.peek().copied() {
                if !is_word_char(next_ch) {
                    break;
                }
                chars.next();
                end = next_idx + next_ch.len_utf8();
            }
            tokens.push(Token {
                kind: TokenKind::Word,
                start,
                end,
            });
            continue;
        }

        tokens.push(Token {
            kind: TokenKind::Symbol,
            start,
            end: start + ch.len_utf8(),
        });
    }

    LexedSource { source, tokens }
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

#[cfg(test)]
mod tests {
    use super::{TokenKind, lex};

    #[test]
    fn lexes_newline_and_word_tokens() {
        let lexed = lex("class A {}\n");
        let kinds = lexed
            .tokens
            .iter()
            .map(|token| token.kind)
            .collect::<Vec<_>>();
        assert!(kinds.contains(&TokenKind::Word));
        assert!(kinds.contains(&TokenKind::Newline));
    }
}
