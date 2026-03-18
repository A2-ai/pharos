use std::fmt;
use std::ops::Range;
use std::path::{Path, PathBuf};

use crate::lexer::Token;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub(crate) message: String,
    pub(crate) span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReportError {
    pub(crate) message: String,
    pub(crate) file: PathBuf,
    pub(crate) source: String,
    pub(crate) span: Range<usize>,
    pub(crate) notes: Vec<Note>,
}

impl ReportError {
    pub fn new(message: String, file: &Path, source: &str, span: Range<usize>) -> Self {
        Self {
            message,
            file: file.to_path_buf(),
            source: source.to_string(),
            span,
            notes: Vec::new(),
        }
    }

    pub fn add_note(&mut self, message: String, span: Range<usize>) {
        self.notes.push(Note { message, span });
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    UnexpectedToken { expected: Vec<Token>, found: Token },
    UnexpectedEof { expected: Vec<Token> },
    InvalidLabel { text: String },
    Message(String),
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseErrorKind::UnexpectedToken { expected, found } => match expected.len() {
                1 => write!(f, "expected {}, found {}", expected[0], found),
                _ => {
                    write!(f, "expected one of ")?;
                    for (i, tok) in expected.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{tok}")?;
                    }
                    write!(f, ", found {found}")
                }
            },
            ParseErrorKind::UnexpectedEof { expected } => {
                if expected.is_empty() {
                    write!(f, "unexpected end of file")
                } else {
                    write!(f, "unexpected end of file, expected ")?;
                    for (i, tok) in expected.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{tok}")?;
                    }
                    Ok(())
                }
            }
            ParseErrorKind::InvalidLabel { text } => write!(
                f,
                "invalid label '{text}': must start with a letter and contain only letters, digits, or underscores"
            ),
            ParseErrorKind::Message(s) => write!(f, "{s}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    Parse(ParseErrorKind),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Parse(kind) => write!(f, "{kind}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub kind: ErrorKind,
    pub span: Range<usize>,
    pub notes: Vec<Note>,
}

impl Diagnostic {
    pub fn new(kind: ErrorKind, span: Range<usize>) -> Self {
        Self {
            kind,
            span,
            notes: Vec::new(),
        }
    }

    pub fn parse(kind: ParseErrorKind, span: Range<usize>) -> Self {
        Self::new(ErrorKind::Parse(kind), span)
    }

    pub fn with_note(mut self, message: impl Into<String>, span: Range<usize>) -> Self {
        self.notes.push(Note {
            message: message.into(),
            span,
        });
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl std::error::Error for Diagnostic {}
