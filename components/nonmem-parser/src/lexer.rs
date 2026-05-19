use std::fmt;
use std::ops::Range;

use logos::Logos;
use serde::{Deserialize, Serialize};

#[derive(Logos, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    #[regex(r"-?[0-9]+\.[0-9]*([Ee][+-]?[0-9]+)?|-?\.[0-9]+([Ee][+-]?[0-9]+)?|[0-9]+[Ee][+-]?[0-9]+")]
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

    // Catch-all: any contiguous non-whitespace, non-structural characters.
    // Handles identifiers, keywords, file paths, single characters like # or @, etc.
    #[regex(r"[^\s,;()=\n]+", priority = 1)]
    Symbol,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::QuotedString => write!(f, "a quoted string"),
            Token::LeftParen => write!(f, "'('"),
            Token::RightParen => write!(f, "')'"),
            Token::Comma => write!(f, "','"),
            Token::Equals => write!(f, "'='"),
            Token::Newline => write!(f, "newline"),
            Token::Whitespace => write!(f, "whitespace"),
            Token::Float => write!(f, "a number"),
            Token::Infinity => write!(f, "INF"),
            Token::Int => write!(f, "an integer"),
            Token::ControlRecord => write!(f, "a control record"),
            Token::Comment => write!(f, "a comment"),
            Token::Symbol => write!(f, "a name"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Range<usize>,
    pub text: String,
}

pub fn lex(input: &str) -> Vec<SpannedToken> {
    let mut tokens = Vec::new();
    for (result, span) in Token::lexer(input).spanned() {
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
