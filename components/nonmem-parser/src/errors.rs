use std::fmt;
use std::ops::Range;
use std::path::{Path, PathBuf};

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
pub enum ErrorKind {}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TDO")
    }
}

#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    // If the error comes from some third party libs, TODO we need that?
    pub(crate) source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as _)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}
