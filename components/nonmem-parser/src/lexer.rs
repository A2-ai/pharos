use std::ops::Range;

use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
pub enum Token {
    // ATOMS -----
    #[regex(r#""[^"]*""#)]
    #[regex(r"'[^']*'")]
    QuotedString,

    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token(",")]
    Comma,
    #[token("=")]
    Equals,
    #[token("\n")]
    Newline,
    #[regex(r"[ \t\r]+")]
    Whitespace,

    // Float must come before Int so logos prefers the longer match.
    #[regex(r"-?[0-9]+\.[0-9]*([Ee][+-]?[0-9]+)?|\.[0-9]+([Ee][+-]?[0-9]+)?|[0-9]+[Ee][+-]?[0-9]+")]
    Float,

    #[regex(r"[+-]?(INFINITY|INFIN|INF)")]
    Infinity,

    // Plain integer
    #[regex("-?[0-9]+")]
    Int,

    // REST -----
    #[regex(r"\$[A-Za-z]+")]
    ControlRecord,

    #[regex(r";[^\n]*", allow_greedy = true)]
    Comment,

    // #[regex(r"[^\n]+", allow_greedy = true, priority = 0)]
    // Text,

    // Catch-all: any contiguous non-whitespace, non-structural characters.
    // Handles identifiers, keywords, file paths, single characters like # or @, etc.
    #[regex(r"[^\s,;()=\n]+", priority = 1)]
    Symbol,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Range<usize>,
    pub text: String,
}

pub fn lex(input: &str) -> Vec<SpannedToken> {
    let input = input.replace("\r\n", "\n");
    let mut tokens = Vec::new();
    for (result, span) in Token::lexer(&input).spanned() {
        let text = input[span.clone()].to_string();
        match result {
            Ok(token) => tokens.push(SpannedToken { token, span, text }),
            Err(()) => {
                unreachable!("should not fail");
            }
        }
    }
    tokens
}
