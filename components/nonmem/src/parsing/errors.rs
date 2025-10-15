use std::fmt;

use crate::parsing::utils::Span;

fn get_line_starts(source: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(source.match_indices('\n').map(|(i, _)| i + 1))
        .collect()
}

pub fn generate_report(error: &SyntaxError, source: &str) -> String {
    let line_starts: Vec<_> = get_line_starts(source);
    let start_line = error.span.start_line;
    let start_col = error.span.start_col;
    let spacing = " ";
    let line = if start_line == line_starts.len() {
        &source[line_starts[start_line - 1]..]
    } else {
        &source[line_starts[start_line - 1]..line_starts[start_line]]
    }
    .trim_end_matches('\n');
    let mut underline = String::with_capacity(100);
    let underline_offset = if start_col > 0 { start_col - 1 } else { 0 };
    for c in line.chars().take(underline_offset) {
        match c {
            '\t' => underline.push('\t'),
            _ => underline.push(' '),
        }
    }
    // TODO: push variable amount of - depending on end_col-start_col
    underline.push_str(" ^---");
    let message = &error.message;

    format!(
        "{spacing} --> [{start_line}:{start_col}]\n\
         {spacing} |\n\
         {start_line} | {line}\n\
         {spacing} | {underline}\n\
         {spacing} = {message}"
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError {
    pub(crate) message: String,
    pub(crate) span: Span,
    pub(crate) report: String,
}

impl SyntaxError {
    pub fn new(message: String, span: &Span) -> Self {
        Self {
            message,
            span: span.clone(),
            report: String::new(),
        }
    }

    pub fn generate_report(&mut self, source: &str) {
        self.report = generate_report(self, source);
    }
}

impl std::error::Error for SyntaxError {}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.report)
    }
}
