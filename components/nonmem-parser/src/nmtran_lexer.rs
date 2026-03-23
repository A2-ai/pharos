use std::fmt;
use std::ops::Range;

use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq, Eq)]
pub enum NmtranToken {
    #[token("**")]
    StarStar,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("=")]
    Equals,
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token(",")]
    Comma,

    // Line continuation
    #[token("&")]
    Ampersand,

    // Fortran comparison operators (case-insensitive)
    #[regex(r"\.[Ee][Qq]\.")]
    DotEq,
    #[regex(r"\.[Nn][Ee]\.")]
    DotNe,
    #[regex(r"\.[Ll][Tt]\.")]
    DotLt,
    #[regex(r"\.[Ll][Ee]\.")]
    DotLe,
    #[regex(r"\.[Gg][Tt]\.")]
    DotGt,
    #[regex(r"\.[Gg][Ee]\.")]
    DotGe,

    // Fortran logical operators (case-insensitive)
    #[regex(r"\.[Aa][Nn][Dd]\.")]
    DotAnd,
    #[regex(r"\.[Oo][Rr]\.")]
    DotOr,

    // Numbers — no leading minus (unary minus is an operator)
    // Float must come before Int so logos prefers the longer match.
    // Supports Fortran D-exponent (1.5D2) in addition to E-exponent (1.5E2).
    #[regex(r"[0-9]+\.[0-9]*([EeDd][+-]?[0-9]+)?")]
    #[regex(r"\.[0-9]+([EeDd][+-]?[0-9]+)?")]
    #[regex(r"[0-9]+[EeDd][+-]?[0-9]+")]
    Float,

    #[regex(r"[0-9]+")]
    Int,

    // Identifiers: variable names, function names, keywords (IF, THEN, etc.)
    #[regex(r"[A-Za-z_][A-Za-z0-9_]*")]
    Ident,

    // Trivia
    #[token("\n")]
    Newline,
    #[regex(r"[ \t\r]+")]
    Whitespace,
    #[regex(r";[^\n]*", allow_greedy = true)]
    Comment,
}

impl fmt::Display for NmtranToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NmtranToken::StarStar => write!(f, "'**'"),
            NmtranToken::Star => write!(f, "'*'"),
            NmtranToken::Slash => write!(f, "'/'"),
            NmtranToken::Plus => write!(f, "'+'"),
            NmtranToken::Minus => write!(f, "'-'"),
            NmtranToken::Equals => write!(f, "'='"),
            NmtranToken::LeftParen => write!(f, "'('"),
            NmtranToken::RightParen => write!(f, "')'"),
            NmtranToken::Comma => write!(f, "','"),
            NmtranToken::Ampersand => write!(f, "'&'"),
            NmtranToken::DotEq => write!(f, "'.EQ.'"),
            NmtranToken::DotNe => write!(f, "'.NE.'"),
            NmtranToken::DotLt => write!(f, "'.LT.'"),
            NmtranToken::DotLe => write!(f, "'.LE.'"),
            NmtranToken::DotGt => write!(f, "'.GT.'"),
            NmtranToken::DotGe => write!(f, "'.GE.'"),
            NmtranToken::DotAnd => write!(f, "'.AND.'"),
            NmtranToken::DotOr => write!(f, "'.OR.'"),
            NmtranToken::Float => write!(f, "a float"),
            NmtranToken::Int => write!(f, "an integer"),
            NmtranToken::Ident => write!(f, "an identifier"),
            NmtranToken::Newline => write!(f, "newline"),
            NmtranToken::Whitespace => write!(f, "whitespace"),
            NmtranToken::Comment => write!(f, "a comment"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NmtranSpannedToken {
    pub token: NmtranToken,
    pub span: Range<usize>,
    pub text: String,
}

/// Lex NMTRAN code blocks ($PK, $ERR etc)
///
/// `source` is the code block text (after the control record, e.g. after `$PK\n`).
/// `offset` is the byte offset of `source[0]` in the original full input,
/// so that all spans are absolute for the model file.
pub fn lex_nmtran(source: &str, offset: usize) -> Vec<NmtranSpannedToken> {
    let source = source.replace("\r\n", "\n");
    let mut tokens = Vec::new();
    for (result, span) in NmtranToken::lexer(&source).spanned() {
        let text = source[span.clone()].to_string();
        let absolute_span = (span.start + offset)..(span.end + offset);
        match result {
            Ok(token) => tokens.push(NmtranSpannedToken {
                token,
                span: absolute_span,
                text,
            }),
            Err(()) => {
                // Skip unrecognized characters (e.g. stray chars).
                // The parser will handle any structural issues.
            }
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token_types(input: &str) -> Vec<(NmtranToken, &str)> {
        let tokens = lex_nmtran(input, 0);
        tokens
            .iter()
            .filter(|t| !matches!(t.token, NmtranToken::Whitespace | NmtranToken::Newline))
            .map(|t| (t.token.clone(), &input[t.span.clone()]))
            .collect()
    }

    #[test]
    fn fortran_d_exponent() {
        let toks = token_types("X=1.5D2+1.5d2+3D-1+.5D+3");
        let nums: Vec<_> = toks
            .iter()
            .filter(|(t, _)| matches!(t, NmtranToken::Float))
            .map(|(_, s)| *s)
            .collect();
        assert_eq!(nums, vec!["1.5D2", "1.5d2", "3D-1", ".5D+3"]);
    }

    #[test]
    fn absolute_spans() {
        let offset = 100;
        let tokens = lex_nmtran("KA=1", offset);
        assert_eq!(tokens[0].span, 100..102); // KA
        assert_eq!(tokens[1].span, 102..103); // =
        assert_eq!(tokens[2].span, 103..104); // 1
    }
}
