use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Formatter;
use std::str::FromStr;

use crate::parsing::errors::SyntaxError;
use crate::parsing::utils::{Span, Spanned};

static NUMBER_REGEX: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"^[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?").unwrap());

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ControlRecord {
    Problem,
    Input,
    Data,
    Subroutine,
    Pk,
    Pred,
    Theta,
    Omega,
    Sigma,
    Error,
    Estimation,
    Covariance,
    Model,
    Des,
    Simulation,
    Table,
    Other(String),
}

impl ControlRecord {
    pub fn can_parse_content(&self) -> bool {
        matches!(
            self,
            ControlRecord::Input
                | ControlRecord::Data
                | ControlRecord::Subroutine
                | ControlRecord::Theta
                | ControlRecord::Omega
                | ControlRecord::Sigma
                | ControlRecord::Estimation
                | ControlRecord::Covariance
                | ControlRecord::Simulation
                | ControlRecord::Table
        )
    }
}

impl FromStr for ControlRecord {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "PROBLEM" | "PROB" | "PR" => Ok(ControlRecord::Problem),
            "INPUT" | "INPT" => Ok(ControlRecord::Input),
            "DATA" => Ok(ControlRecord::Data),
            "SUBROUTINE" | "SUBROUTINES" | "SUB" | "SUBS" => Ok(ControlRecord::Subroutine),
            "PK" => Ok(ControlRecord::Pk),
            "PRED" => Ok(ControlRecord::Pred),
            "THETA" | "THTA" => Ok(ControlRecord::Theta),
            "OMEGA" | "OMGA" => Ok(ControlRecord::Omega),
            "SIGMA" | "SGMA" => Ok(ControlRecord::Sigma),
            "ERROR" | "ERR" => Ok(ControlRecord::Error),
            "ESTIMATION" | "EST" => Ok(ControlRecord::Estimation),
            "COVARIANCE" | "COV" => Ok(ControlRecord::Covariance),
            "MODEL" | "MOD" => Ok(ControlRecord::Model),
            "DES" => Ok(ControlRecord::Des),
            "SIMULATION" | "SIM" => Ok(ControlRecord::Simulation),
            "TABLE" | "TAB" => Ok(ControlRecord::Table),
            // Catch all
            _ => Ok(ControlRecord::Other(s.to_string())),
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum Token {
    Number {
        value: f64,
        original: String,
    },
    Identifier(String),
    Keyword(String),
    QuotedString(String),
    ControlRecord {
        kind: ControlRecord,
        original: String,
    },
    LeftParen,
    RightParen,
    Comma,
    Equals,
    Comment(String),
    Whitespace(String),
    Ignored(String),
}

impl Token {
    pub fn name(&self) -> &str {
        match self {
            Token::Number { .. } => "number",
            Token::Identifier(_) => "identifier",
            Token::Keyword(_) => "keyword",
            Token::QuotedString(_) => "quoted string",
            Token::ControlRecord { .. } => "control record",
            Token::LeftParen => "(",
            Token::RightParen => ")",
            Token::Comma => ",",
            Token::Equals => "=",
            Token::Comment(_) => "comment",
            Token::Whitespace(_) => "whitespace",
            Token::Ignored(_) => "anything",
        }
    }

    pub fn is_trivia(&self) -> bool {
        matches!(self, Token::Whitespace(_) | Token::Comment(_))
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Token::Number { value, .. } => write!(f, "NUMBER({value})"),
            Token::Identifier(s) => write!(f, "IDENT({s})"),
            Token::Keyword(s) => write!(f, "KEYWORD({s})"),
            Token::QuotedString(s) => write!(f, "QUOTED_STRING({s:?})"),
            Token::ControlRecord { kind, .. } => write!(f, "CONTROL_RECORD({kind:?})"),
            Token::LeftParen => write!(f, "LEFT_PAREN"),
            Token::RightParen => write!(f, "RIGHT_PAREN"),
            Token::Comma => write!(f, "COMMA"),
            Token::Equals => write!(f, "EQUALS"),
            Token::Comment(s) => write!(f, "COMMENT({s})"),
            Token::Whitespace(s) => write!(f, "WHITESPACE({s:?})"),
            Token::Ignored(s) => write!(f, "IGNORED({s:?})"),
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Token::Number { original, .. } => write!(f, "{original}"),
            Token::Identifier(s) => write!(f, "{s}"),
            Token::Keyword(s) => write!(f, "{s}"),
            Token::QuotedString(s) => write!(f, "\"{s}\""),
            Token::ControlRecord { original, .. } => write!(f, "${original}"),
            Token::LeftParen => write!(f, "("),
            Token::RightParen => write!(f, ")"),
            Token::Comma => write!(f, ","),
            Token::Equals => write!(f, "="),
            Token::Comment(s) => write!(f, ";{s}"),
            Token::Whitespace(s) => write!(f, "{s}"),
            Token::Ignored(s) => write!(f, "{s}"),
        }
    }
}

fn is_nonmem_keyword(word: &str) -> bool {
    let keyword = word.to_uppercase();
    matches!(
        keyword.as_str(),
        "FIX" | "FIXED" | "DROP" | "SKIP" | "IGNORE" | "IGN" | "ACCEPT" | "RECORDS" | "LAST20" | "ONLYSIM" | "NULL" |
         // Subroutine keywords
         "ADVAN1" | "ADVAN2" | "ADVAN3" | "ADVAN4" | "ADVAN5" | "ADVAN6" |
         "ADVAN7" | "ADVAN8" | "ADVAN9" | "ADVAN10" | "ADVAN11" | "ADVAN12" | "ADVAN13" |
         "ADVAN14" | "ADVAN15" | "OTHER" |
         "TRANS1" | "TRANS2" | "TRANS3" | "TRANS4" | "TRANS5" | "TRANS6" |
         "TRANS7" | "TRANS8" | "TRANS9" | "TRANS10" | "TRANS11" |
         // parametrization omega/sigma
         "CORR" | "SD" | "CHOLESKY" | "BLOCK" | "SAME" | "CORRELATION" | "VALUES" |
         // Keywords indicating file paths
         "FILE" | "MSFO" | "MSFI" |
         // Estimation keywords"
         "METHOD" | "SAEM" | "IMP" | "IMPMAP" | "INTERACTION" | "NUTS" | "ITS" | "BAYES" | "INTER" |
         "COND" |
         // Table keywords
         "ONEHEADER" | "NOPRINT" | "NOAPPEND" | "FIRSTONLY" | "NOTITLE" | "NOHEADER" | "FORMAT" |
         // Subroutine options
         "TOL" |
         // Infinity bounds
         "INF" | "INFINITY" |
         // Parameter naming
         "NAMES"
    )
}

pub fn lex(input: &str) -> Result<Vec<Spanned<Token>>, SyntaxError> {
    let mut rest = input;
    let mut tokens = Vec::new();

    let mut current_line = 1;
    let mut current_col = 0;
    let mut current_byte = 0;
    let mut expecting_filepath = false;

    macro_rules! loc {
        () => {
            (current_line, current_col, current_byte)
        };
    }

    macro_rules! make_span {
        ($start:expr) => {{
            let (start_line, start_col, start_byte) = $start;
            Span {
                start_line,
                start_col,
                end_line: current_line,
                end_col: current_col,
                range: start_byte..current_byte,
            }
        }};
    }

    macro_rules! advance {
        ($num_bytes:expr) => {{
            let (skipped, new_rest) = rest.split_at($num_bytes);
            for c in skipped.chars() {
                current_byte += c.len_utf8();
                match c {
                    '\n' => {
                        current_line += 1;
                        current_col = 0;
                    }
                    _ => current_col += 1,
                }
            }
            rest = new_rest;
            skipped
        }};
    }

    macro_rules! lex_word {
        () => {{
            let word_len = rest
                .as_bytes()
                .iter()
                .enumerate()
                .take_while(|&(_, &c)| c.is_ascii_alphabetic())
                .count();
            advance!(word_len)
        }};
    }

    macro_rules! lex_until {
        ($ch:expr) => {{
            let blob_len = rest
                .as_bytes()
                .iter()
                .enumerate()
                .take_while(|&(_, &c)| c != $ch)
                .count();
            advance!(blob_len)
        }};
        ($ch:expr, $start_loc:expr, $err_msg:expr) => {{
            let blob_len = rest
                .as_bytes()
                .iter()
                .enumerate()
                .take_while(|&(_, &c)| c != $ch)
                .count();
            let content = advance!(blob_len);
            if rest.as_bytes().first() != Some(&$ch) {
                return Err(SyntaxError::new(
                    $err_msg.to_string(),
                    &make_span!($start_loc),
                ));
            }
            advance!(1); // skip the delimiter
            content
        }};
    }

    macro_rules! lex_ident {
        () => {{
            let blob_len = rest
                .as_bytes()
                .iter()
                .enumerate()
                .take_while(|&(_, c)| {
                    !(*c as char).is_whitespace()
                        && *c != b'='
                        && *c != b';'
                        && *c != b','
                        && *c != b')'
                        && *c != b'('
                })
                .count();
            advance!(blob_len)
        }};
    }

    while !rest.is_empty() {
        match rest.as_bytes().first() {
            // Control records
            Some(b'$') => {
                let start_loc = loc!();
                advance!(1);
                let record_name = lex_word!();
                let kind = ControlRecord::from_str(record_name).unwrap();
                let ignore_content = !kind.can_parse_content();
                tokens.push(Spanned::new(
                    Token::ControlRecord {
                        kind,
                        original: record_name.to_string(),
                    },
                    make_span!(start_loc),
                ));

                if ignore_content {
                    let start_loc = loc!();
                    let ignored = lex_until!(b'$');
                    tokens.push(Spanned::new(
                        Token::Ignored(ignored.to_string()),
                        make_span!(start_loc),
                    ));
                }
            }
            Some(b'(') => {
                let start_loc = loc!();
                advance!(1);
                tokens.push(Spanned::new(Token::LeftParen, make_span!(start_loc)));
            }
            Some(b')') => {
                let start_loc = loc!();
                advance!(1);
                tokens.push(Spanned::new(Token::RightParen, make_span!(start_loc)));
            }
            Some(b',') => {
                let start_loc = loc!();
                advance!(1);
                tokens.push(Spanned::new(Token::Comma, make_span!(start_loc)));
            }
            Some(b'=') => {
                let start_loc = loc!();
                advance!(1);
                tokens.push(Spanned::new(Token::Equals, make_span!(start_loc)));
            }
            Some(b'"') => {
                let start_loc = loc!();
                advance!(1); // skip opening quote
                let content = lex_until!(b'"', start_loc, "Unclosed double quote");
                tokens.push(Spanned::new(
                    Token::QuotedString(content.to_string()),
                    make_span!(start_loc),
                ));
            }
            Some(b'\'') => {
                let start_loc = loc!();
                advance!(1); // skip opening quote
                let content = lex_until!(b'\'', start_loc, "Unclosed single quote");
                tokens.push(Spanned::new(
                    Token::QuotedString(content.to_string()),
                    make_span!(start_loc),
                ));
            }
            Some(b';') => {
                let start_loc = loc!();
                advance!(1);
                let comment = lex_until!(b'\n');
                tokens.push(Spanned::new(
                    Token::Comment(comment.to_string()),
                    make_span!(start_loc),
                ));
            }
            Some(b'0'..=b'9' | b'.' | b'-' | b'+') => {
                let start_loc = loc!();
                if expecting_filepath {
                    // When expecting a file path, always parse as identifier
                    let ident = lex_ident!();
                    tokens.push(Spanned::new(
                        Token::Identifier(ident.to_string()),
                        make_span!(start_loc),
                    ));
                    expecting_filepath = false; // Reset after consuming the file path
                } else if let Some(mat) = NUMBER_REGEX.find(rest) {
                    let num = advance!(mat.end());
                    match num.parse::<f64>() {
                        Ok(value) => {
                            tokens.push(Spanned::new(
                                Token::Number {
                                    value,
                                    original: num.to_string(),
                                },
                                make_span!(start_loc),
                            ));
                        }
                        Err(_) => {
                            return Err(SyntaxError::new(
                                format!("Invalid number literal: {num}"),
                                &make_span!(start_loc),
                            ));
                        }
                    }
                } else {
                    // Not a number, try to parse it as an ident instead.
                    // It could be a path like ../something for example
                    let ident = lex_ident!();
                    tokens.push(Spanned::new(
                        Token::Identifier(ident.to_string()),
                        make_span!(start_loc),
                    ));
                }
            }
            Some(c) if (*c as char).is_whitespace() => {
                let start_loc = loc!();
                let content = advance!(1);

                // Check if we can merge with the last token
                match tokens.last_mut() {
                    Some(last_token) if matches!(**last_token, Token::Whitespace(_)) => {
                        // Modify the last token in place
                        if let Token::Whitespace(s) = &mut **last_token {
                            s.push_str(content);
                            last_token.span_mut().expand(&make_span!(start_loc));
                        }
                    }
                    _ => {
                        // Add new whitespace token
                        tokens.push(Spanned::new(
                            Token::Whitespace(content.to_string()),
                            make_span!(start_loc),
                        ));
                    }
                }
            }
            // @ and # for the $DATA <IGNORE>=?
            Some(b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'@' | b'#') => {
                let start_loc = loc!();
                let ident = lex_ident!();
                if ident.is_empty() {
                    return Err(SyntaxError::new(
                        "Empty identifier".to_string(),
                        &make_span!(start_loc),
                    ));
                }
                let tok = if expecting_filepath {
                    // If expecting file path, always treat as identifier
                    expecting_filepath = false; // Reset after consuming
                    Token::Identifier(ident.to_string())
                } else if is_nonmem_keyword(ident) {
                    Token::Keyword(ident.to_string())
                } else {
                    Token::Identifier(ident.to_string())
                };
                tokens.push(Spanned::new(tok, make_span!(start_loc)));
            }
            // Handle forward slash for absolute file paths
            Some(b'/') if expecting_filepath => {
                let start_loc = loc!();
                let ident = lex_ident!();
                tokens.push(Spanned::new(
                    Token::Identifier(ident.to_string()),
                    make_span!(start_loc),
                ));
                expecting_filepath = false; // Reset after consuming the file path
            }
            _ => {
                let start_loc = loc!();
                let content = advance!(1);

                // Check if we can merge with the last token
                match tokens.last_mut() {
                    Some(last_token) if matches!(**last_token, Token::Ignored(_)) => {
                        // Modify the last token in place
                        if let Token::Ignored(s) = &mut **last_token {
                            s.push_str(content);
                            last_token.span_mut().expand(&make_span!(start_loc));
                        }
                    }
                    _ => {
                        // Add new ignored token
                        tokens.push(Spanned::new(
                            Token::Ignored(content.to_string()),
                            make_span!(start_loc),
                        ));
                    }
                }
            }
        }

        // Update filepath expectation based on recent tokens
        if !tokens.is_empty() {
            // Find the most recent ControlRecord token
            let mut most_recent_control_idx = None;
            for (i, token) in tokens.iter().enumerate().rev() {
                if matches!(token.node(), Token::ControlRecord { .. }) {
                    most_recent_control_idx = Some(i);
                    break;
                }
            }

            if let Some(control_idx) = most_recent_control_idx {
                // Special case handling for $DATA path starting with /
                let is_data_control = matches!(
                    tokens[control_idx].node(),
                    Token::ControlRecord {
                        kind: ControlRecord::Data,
                        ..
                    }
                );

                // Check if everything since the $DATA control record is trivia
                let all_trivia_since = tokens[(control_idx + 1)..].iter().all(|t| t.is_trivia());

                if is_data_control && all_trivia_since {
                    expecting_filepath = true;
                } else if tokens.len() >= 2 {
                    // Check for FILE=, MSFO=, OTHER= patterns
                    let last_two = &tokens[tokens.len() - 2..];
                    expecting_filepath = matches!(
                        (&last_two[0].node(), &last_two[1].node()),
                        (Token::Keyword(kw), Token::Equals)
                            if kw.eq_ignore_ascii_case("FILE")
                                || kw.eq_ignore_ascii_case("MSFO")
                                || kw.eq_ignore_ascii_case("OTHER")
                    );
                } else {
                    expecting_filepath = false;
                }
            } else {
                expecting_filepath = false;
            }
        }
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fs_err as fs;
    use insta::{assert_debug_snapshot, glob};

    #[test]
    fn can_lex_mod_files() {
        glob!("../../test_data/lexer", "*.mod", |path| {
            let input = fs::read_to_string(path).unwrap().replace("\r\n", "\n");
            assert_debug_snapshot!(lex(&input));
        });
    }
}
