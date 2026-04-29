use std::fmt;
use std::ops::Range;
use std::path::Path;

use crate::lexer::Token;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub(crate) message: String,
    pub(crate) span: Range<usize>,
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
    Lowering(String),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Parse(kind) => write!(f, "{kind}"),
            ErrorKind::Lowering(msg) => write!(f, "{msg}"),
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

    pub fn lowering(message: impl Into<String>, span: Range<usize>) -> Self {
        Self::new(ErrorKind::Lowering(message.into()), span)
    }

    pub fn with_note(mut self, message: impl Into<String>, span: Range<usize>) -> Self {
        self.notes.push(Note {
            message: message.into(),
            span,
        });
        self
    }

    /// Render this diagnostic as a compiler-style error message.
    pub fn render(&self, file: &Path, source: &str) -> String {
        let mut out = String::new();

        // header
        out.push_str(&format!("error: {}\n", self.kind));

        // main span
        Self::render_span(&mut out, file, source, &self.span);

        // notes
        for note in &self.notes {
            out.push_str(&format!("  = note: {}\n", note.message));
            if note.span != self.span {
                Self::render_span(&mut out, file, source, &note.span);
            }
        }

        out
    }

    fn render_span(out: &mut String, file: &Path, source: &str, span: &Range<usize>) {
        let (line, col) = byte_offset_to_line_col(source, span.start);
        let line_text = source_line(source, line);
        let line_num = line + 1;
        let display = file.display();

        out.push_str(&format!(" --> {display}:{line_num}:{}\n", col + 1));

        let gutter_width = line_num.to_string().len();
        let blank_gutter = " ".repeat(gutter_width);

        out.push_str(&format!("{blank_gutter} |\n"));
        out.push_str(&format!("{line_num} | {line_text}\n"));

        // underline: compute how many chars to underline
        let line_start = source[..span.start].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let underline_start = span.start - line_start;
        let underline_len = (span.end - span.start).max(1);

        out.push_str(&format!(
            "{blank_gutter} | {}{}\n",
            " ".repeat(underline_start),
            "^".repeat(underline_len)
        ));
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl std::error::Error for Diagnostic {}

/// Convert a byte offset to a (0-based line, 0-based column) pair.
fn byte_offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(source.len());
    let before = &source[..offset];
    let line = before.matches('\n').count();
    let col = before.len() - before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    (line, col)
}

/// Extract the text of a given 0-based line number.
fn source_line(source: &str, line: usize) -> &str {
    source.split('\n').nth(line).unwrap_or("")
}
