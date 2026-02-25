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

/// Count the number of decimal places in a numeric string representation.
fn num_decimal_places(s: &str) -> usize {
    if let Some(dot_pos) = s.find('.') {
        s.len() - dot_pos - 1
    } else {
        0
    }
}

pub(crate) fn round_arbitrary_precision(original: &str, new_value: f64) -> f64 {
    if new_value == 0.0 {
        return new_value;
    }

    let num_decimals_wanted = num_decimal_places(original) + 2;

    let factor = 10_f64.powi(num_decimals_wanted as i32);
    (new_value * factor).round() / factor
}

/// Apply jittering to a parameter value using uniform distribution with boundary enforcement.
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

    // Round first so that subsequent bounds checks are against the final precision
    jittered_value = round_arbitrary_precision(original_str, jittered_value);

    // Smallest number we can add matching the required precision
    let precision_epsilon = 10_f64.powi(-((num_decimal_places(original_str) + 2) as i32));

    // Enforce strict lower bound: jittered must be > lower
    if let Some(lower) = lower_bound
        && jittered_value <= lower
    {
        jittered_value = lower + precision_epsilon;
    }
    // Enforce strict upper bound: jittered must be < upper
    if let Some(upper) = upper_bound
        && jittered_value >= upper
    {
        jittered_value = upper - precision_epsilon;
    }

    jittered_value
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

impl ParameterOrdering {
    pub fn get_coordinates(&self, block_size: usize) -> Vec<(usize, usize)> {
        match self {
            ParameterOrdering::RowMajor => (0..block_size)
                .flat_map(|row| (0..=row).map(move |col| (row, col)))
                .collect(),
            ParameterOrdering::ColumnMajor => (0..block_size)
                .flat_map(|col| (col..block_size).map(move |row| (row, col)))
                .collect(),
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

    /// Jittered values must always satisfy strict inequality: lower < init < upper.
    #[test]
    fn jittered_value_strictly_between_bounds() {
        // (value, jitter%, lower, upper, original_str)
        let cases: &[(f64, f64, Option<f64>, Option<f64>, &str)] = &[
            //  estimate ≈ -0.99 with lower = -1
            (-0.991385, 0.2, Some(-1.0), Some(1.0), "-9.91385E-01"),
            // Near lower bound with large jitter
            (0.01, 0.99, Some(0.0), None, "0.01"),
            // Near upper bound
            (0.99, 0.2, None, Some(1.0), "0.99"),
            // Rounding could push onto upper bound
            (0.999, 0.1, None, Some(1.0), "0.999"),
            // Zero value with zero lower (0 * anything = 0, must nudge)
            (0.0, 0.2, Some(0.0), Some(10.0), "0.0"),
        ];

        for (value, jitter, lower, upper, original) in cases {
            for seed in 0..500 {
                let mut rng = StdRng::seed_from_u64(seed);
                let result = apply_jittering(*value, *jitter, &mut rng, *lower, *upper, original);
                if let Some(lo) = lower {
                    assert!(
                        result > *lo,
                        "seed {seed}, case {original}: {result} <= lower {lo}"
                    );
                }
                if let Some(hi) = upper {
                    assert!(
                        result < *hi,
                        "seed {seed}, case {original}: {result} >= upper {hi}"
                    );
                }
            }
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
