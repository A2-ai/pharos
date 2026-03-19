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

/// Lex NMTRAN abbreviated code.
///
/// `source` is the code block text (after the control record, e.g. after `$PK\n`).
/// `offset` is the byte offset of `source[0]` in the original full input,
/// so that all spans are absolute.
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
    fn simple_assignment() {
        let toks = token_types("KA=THETA(1)");
        assert_eq!(
            toks,
            vec![
                (NmtranToken::Ident, "KA"),
                (NmtranToken::Equals, "="),
                (NmtranToken::Ident, "THETA"),
                (NmtranToken::LeftParen, "("),
                (NmtranToken::Int, "1"),
                (NmtranToken::RightParen, ")"),
            ]
        );
    }

    #[test]
    fn power_operator() {
        // TVCLM=THETA(1)*WT**THETA(2)
        let toks = token_types("TVCLM=THETA(1)*WT**THETA(2)");
        assert_eq!(
            toks,
            vec![
                (NmtranToken::Ident, "TVCLM"),
                (NmtranToken::Equals, "="),
                (NmtranToken::Ident, "THETA"),
                (NmtranToken::LeftParen, "("),
                (NmtranToken::Int, "1"),
                (NmtranToken::RightParen, ")"),
                (NmtranToken::Star, "*"),
                (NmtranToken::Ident, "WT"),
                (NmtranToken::StarStar, "**"),
                (NmtranToken::Ident, "THETA"),
                (NmtranToken::LeftParen, "("),
                (NmtranToken::Int, "2"),
                (NmtranToken::RightParen, ")"),
            ]
        );
    }

    #[test]
    fn complex_expression() {
        // TVCLM=WT*(THETA(1)-THETA(2)*CPSS2/(THETA(3)+CPSS2))
        let toks = token_types("TVCLM=WT*(THETA(1)-THETA(2)*CPSS2/(THETA(3)+CPSS2))");
        let types: Vec<_> = toks.iter().map(|(t, _)| t.clone()).collect();
        assert_eq!(
            types,
            vec![
                NmtranToken::Ident,      // TVCLM
                NmtranToken::Equals,      // =
                NmtranToken::Ident,       // WT
                NmtranToken::Star,        // *
                NmtranToken::LeftParen,   // (
                NmtranToken::Ident,       // THETA
                NmtranToken::LeftParen,   // (
                NmtranToken::Int,         // 1
                NmtranToken::RightParen,  // )
                NmtranToken::Minus,       // -
                NmtranToken::Ident,       // THETA
                NmtranToken::LeftParen,   // (
                NmtranToken::Int,         // 2
                NmtranToken::RightParen,  // )
                NmtranToken::Star,        // *
                NmtranToken::Ident,       // CPSS2
                NmtranToken::Slash,       // /
                NmtranToken::LeftParen,   // (
                NmtranToken::Ident,       // THETA
                NmtranToken::LeftParen,   // (
                NmtranToken::Int,         // 3
                NmtranToken::RightParen,  // )
                NmtranToken::Plus,        // +
                NmtranToken::Ident,       // CPSS2
                NmtranToken::RightParen,  // )
                NmtranToken::RightParen,  // )
            ]
        );
    }

    #[test]
    fn function_calls() {
        let toks = token_types("TVLCLM=THETA(1)+THETA(2)*LOG(WT)");
        let texts: Vec<_> = toks.iter().map(|(_, t)| *t).collect();
        assert_eq!(
            texts,
            vec![
                "TVLCLM", "=", "THETA", "(", "1", ")", "+", "THETA", "(", "2", ")", "*", "LOG",
                "(", "WT", ")"
            ]
        );
    }

    #[test]
    fn comparison_operators() {
        let toks = token_types("IF (ANUM.EQ.2) ASY=0");
        assert_eq!(
            toks,
            vec![
                (NmtranToken::Ident, "IF"),
                (NmtranToken::LeftParen, "("),
                (NmtranToken::Ident, "ANUM"),
                (NmtranToken::DotEq, ".EQ."),
                (NmtranToken::Int, "2"),
                (NmtranToken::RightParen, ")"),
                (NmtranToken::Ident, "ASY"),
                (NmtranToken::Equals, "="),
                (NmtranToken::Int, "0"),
            ]
        );
    }

    #[test]
    fn comparison_operators_case_insensitive() {
        let toks = token_types("X.eq.Y .GT. Z .le. W");
        let types: Vec<_> = toks.iter().map(|(t, _)| t.clone()).collect();
        assert_eq!(
            types,
            vec![
                NmtranToken::Ident,
                NmtranToken::DotEq,
                NmtranToken::Ident,
                NmtranToken::DotGt,
                NmtranToken::Ident,
                NmtranToken::DotLe,
                NmtranToken::Ident,
            ]
        );
    }

    #[test]
    fn logical_operators() {
        let toks = token_types("A.AND.B .OR. C");
        assert_eq!(
            toks,
            vec![
                (NmtranToken::Ident, "A"),
                (NmtranToken::DotAnd, ".AND."),
                (NmtranToken::Ident, "B"),
                (NmtranToken::DotOr, ".OR."),
                (NmtranToken::Ident, "C"),
            ]
        );
    }

    #[test]
    fn block_if() {
        let input = "IF (ANUM.EQ.1) THEN\n  ASY=1\nELSE\n  ASY=0\nENDIF";
        let toks = token_types(input);
        let texts: Vec<_> = toks.iter().map(|(_, t)| *t).collect();
        assert_eq!(
            texts,
            vec![
                "IF", "(", "ANUM", ".EQ.", "1", ")", "THEN", "ASY", "=", "1", "ELSE", "ASY", "=",
                "0", "ENDIF"
            ]
        );
    }

    #[test]
    fn eta_references() {
        let toks = token_types("CL=THETA(1)\nCL=CL+ETA(1)");
        let texts: Vec<_> = toks.iter().map(|(_, t)| *t).collect();
        assert_eq!(
            texts,
            vec!["CL", "=", "THETA", "(", "1", ")", "CL", "=", "CL", "+", "ETA", "(", "1", ")"]
        );
    }

    #[test]
    fn float_literals() {
        let toks = token_types("X=1.5+.5+1E-3+2.0E+10");
        let nums: Vec<_> = toks
            .iter()
            .filter(|(t, _)| matches!(t, NmtranToken::Float))
            .map(|(_, s)| *s)
            .collect();
        assert_eq!(nums, vec!["1.5", ".5", "1E-3", "2.0E+10"]);
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
    fn comments() {
        let toks = lex_nmtran("KA=THETA(1) ; absorption rate\n", 0);
        let comment = toks
            .iter()
            .find(|t| t.token == NmtranToken::Comment)
            .unwrap();
        assert_eq!(comment.text, "; absorption rate");
    }

    #[test]
    fn line_continuation() {
        let toks = token_types("X=A+&\nB");
        let texts: Vec<_> = toks.iter().map(|(_, t)| *t).collect();
        assert_eq!(texts, vec!["X", "=", "A", "+", "&", "B"]);
    }

    #[test]
    fn absolute_spans() {
        let offset = 100;
        let tokens = lex_nmtran("KA=1", offset);
        assert_eq!(tokens[0].span, 100..102); // KA
        assert_eq!(tokens[1].span, 102..103); // =
        assert_eq!(tokens[2].span, 103..104); // 1
    }

    #[test]
    fn division_expression() {
        let toks = token_types("V=CL/K");
        assert_eq!(
            toks,
            vec![
                (NmtranToken::Ident, "V"),
                (NmtranToken::Equals, "="),
                (NmtranToken::Ident, "CL"),
                (NmtranToken::Slash, "/"),
                (NmtranToken::Ident, "K"),
            ]
        );
    }

    #[test]
    fn nested_if() {
        let input = "\
IF (ICU.EQ.1) THEN
  IF (AGE.GT.50) THEN
    TVCL=THETA(1)
  ELSE
    TVCL=THETA(2)
  ENDIF
ELSE
  TVCL=THETA(3)
ENDIF";
        let toks = token_types(input);
        let texts: Vec<_> = toks.iter().map(|(_, t)| *t).collect();
        assert_eq!(
            texts,
            vec![
                "IF", "(", "ICU", ".EQ.", "1", ")", "THEN", "IF", "(", "AGE", ".GT.", "50", ")",
                "THEN", "TVCL", "=", "THETA", "(", "1", ")", "ELSE", "TVCL", "=", "THETA", "(",
                "2", ")", "ENDIF", "ELSE", "TVCL", "=", "THETA", "(", "3", ")", "ENDIF"
            ]
        );
    }
}
