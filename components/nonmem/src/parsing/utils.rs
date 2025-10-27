use std::fmt;
use std::fmt::Formatter;
use std::ops::{Deref, Range};

use rand::prelude::*;

#[derive(Clone, PartialEq, Eq, Default)]
pub struct Span {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub range: Range<usize>,
}

impl Span {
    pub fn expand(&mut self, other: &Span) {
        self.end_line = other.end_line;
        self.end_col = other.end_col;
        self.range = self.range.start..other.range.end;
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            " @ {}:{}-{}:{} ({:?})",
            self.start_line, self.start_col, self.end_line, self.end_col, self.range,
        )
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            " @ {}:{}-{}:{}",
            self.start_line, self.start_col, self.end_line, self.end_col,
        )
    }
}

#[derive(Clone, PartialEq)]
pub struct Spanned<T: fmt::Debug> {
    node: Box<T>,
    span: Span,
}

impl<T: fmt::Debug> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self {
            node: Box::new(node),
            span,
        }
    }

    pub fn span(&self) -> &Span {
        &self.span
    }

    pub fn span_mut(&mut self) -> &mut Span {
        &mut self.span
    }

    pub(crate) fn node(&self) -> &T {
        &self.node
    }
}

impl<T: fmt::Debug> fmt::Debug for Spanned<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}{:?}", self.node, self.span)
    }
}

impl<T: fmt::Debug> Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl<T: fmt::Debug> std::ops::DerefMut for Spanned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.node
    }
}

pub(crate) fn round_arbitrary_precision(original: &str, new_value: f64) -> f64 {
    if new_value == 0.0 {
        return new_value;
    }

    let num_decimals = if let Some(dot_pos) = original.find('.') {
        original.len() - dot_pos - 1
    } else {
        0 // No decimal point means 0 decimal places
    };
    let num_decimals_wanted = num_decimals + 2;

    let factor = 10_f64.powi(num_decimals_wanted as i32);
    (new_value * factor).round() / factor
}

/// Apply jittering to a parameter value using uniform distribution with boundary enforcement
pub(crate) fn apply_jittering(
    value: f64,
    jitter_percentage: f64,
    rng: &mut StdRng,
    lower_bound: Option<f64>,
    upper_bound: Option<f64>,
    original_str: &str,
) -> f64 {
    let random_factor: f64 = rng.random_range(-1.0..=1.0);
    let perturbation_multiplier = 1.0 + (jitter_percentage * random_factor);
    let mut jittered_value = value * perturbation_multiplier;

    // Enforce boundaries
    if let Some(lower) = lower_bound
        && jittered_value < lower
    {
        jittered_value = lower;
    }
    if let Some(upper) = upper_bound
        && jittered_value > upper
    {
        jittered_value = upper;
    }

    // Use existing rounding logic with +2 precision
    round_arbitrary_precision(original_str, jittered_value)
}

pub(crate) fn replace_stem_in_path(
    path: &str,
    original_stem: &str,
    new_stem: &str,
) -> Option<String> {
    if !path.contains(original_stem) {
        return None;
    }

    Some(path.replace(original_stem, new_stem))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterOrdering {
    /// Row-major ordering used in EXT files: (1,1), (2,1), (2,2), (3,1), (3,2), (3,3)
    RowMajor,
    /// Column-major ordering used in GRD files: (1,1), (2,1), (3,1), (2,2), (3,2), (3,3)
    ColumnMajor,
}

/// Iterator for generating (row, col) coordinate pairs based on parameter ordering
pub enum ParameterCoordinates {
    RowMajor {
        size: usize,
        current_row: usize,
        current_col: usize,
    },
    ColumnMajor {
        size: usize,
        current_col: usize,
        current_row: usize,
    },
}

impl ParameterCoordinates {
    pub fn new(size: usize, ordering: ParameterOrdering) -> Self {
        match ordering {
            ParameterOrdering::RowMajor => Self::RowMajor {
                size,
                current_row: 0,
                current_col: 0,
            },
            ParameterOrdering::ColumnMajor => Self::ColumnMajor {
                size,
                current_col: 0,
                current_row: 0,
            },
        }
    }
}

impl Iterator for ParameterCoordinates {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::RowMajor {
                size,
                current_row,
                current_col,
            } => {
                if *current_row >= *size {
                    return None;
                }

                let result = (*current_row, *current_col);

                // Advance to next position
                if *current_col >= *current_row {
                    *current_row += 1;
                    *current_col = 0;
                } else {
                    *current_col += 1;
                }

                Some(result)
            }
            Self::ColumnMajor {
                size,
                current_col,
                current_row,
            } => {
                if *current_col >= *size {
                    return None;
                }

                let result = (*current_row, *current_col);

                // Advance to next position
                if *current_row >= *size - 1 {
                    *current_col += 1;
                    *current_row = *current_col;
                } else {
                    *current_row += 1;
                }

                Some(result)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_round_to_significant_digit() {
        let tests = vec![
            (0.0, 0.0, 0.0),
            (1.5, 1.657474576, 1.657),
            (-0.5, -0.44563545364634, -0.446),
            (1.0, 1.23456789, 1.23),
            (3.14, 3.12828123182, 3.1283),
        ];

        for (n, n2, expected) in tests {
            assert_eq!(round_arbitrary_precision(&n.to_string(), n2), expected);
        }
    }

    #[test]
    fn test_replace_stem_in_path() {
        let test_cases = vec![
            ("run001par.tab", "run001", "run002", Some("run002par.tab")),
            ("run001.msof", "run001", "run001b23", Some("run001b23.msof")),
        ];

        for (path, original_stem, new_stem, expected) in test_cases {
            let result = replace_stem_in_path(path, original_stem, new_stem);
            let expected = expected.map(|s| s.to_string());
            assert_eq!(result, expected, "Failed for path: {}", path);
        }
    }
}
