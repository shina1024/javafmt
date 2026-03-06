use javafmt::format_str;
use javafmt::lexer::{TokenKind, lex};

#[derive(Debug, Clone, PartialEq, Eq)]
struct SignificantToken {
    kind: TokenKind,
    text: String,
}

#[test]
fn formatting_preserves_non_whitespace_tokens_for_representative_inputs() {
    // Top-level imports are intentionally reordered for GJF compatibility, so keep
    // this corpus focused on syntax that should preserve token order.
    let cases = [
        "class A{}",
        "class A{/*keep*/int x=1+2;}",
        "class A{void f(){if(a){b();}else{c();}}}",
        "class A{String s=\"//not-comment\";char c='x';}",
        "class A{String s=\"\"\"\nline1\nline2\n\"\"\";}",
        "@Anno(value={\"a\",\"b\"}) class A{}",
        "class A{void f(){Runnable r=()->{work();};}}",
        "class A{void f(){var x=Foo.<Bar>baz(1,2);}}",
    ];

    for input in cases {
        let formatted = format_str(input).output;
        assert_eq!(
            significant_tokens(input),
            significant_tokens(&formatted),
            "non-whitespace tokens changed for input:\n{input}\nformatted as:\n{formatted}"
        );
    }
}

fn significant_tokens(source: &str) -> Vec<SignificantToken> {
    let lexed = lex(source);
    lexed
        .tokens
        .iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline))
        .map(|token| SignificantToken {
            kind: token.kind,
            text: source[token.start..token.end].to_owned(),
        })
        .collect()
}
