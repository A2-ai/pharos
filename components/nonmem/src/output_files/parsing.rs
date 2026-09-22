use crate::estimation::{EstimationMethod, extract_estimation_method};

/// Common configuration for parsing NONMEM output files
#[derive(Debug, Clone)]
pub struct ParseContext {
    pub only_method: Option<EstimationMethod>,
    pub only_last: bool,
}

/// Find all table start positions in the lines (lines starting with "TABLE NO.")
pub fn find_table_positions(lines: &[&str]) -> Vec<usize> {
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim().starts_with("TABLE NO."))
        .map(|(i, _)| i)
        .collect()
}

/// Select which lines to parse based on method and last table options
pub fn select_lines_to_parse<'a>(
    lines: &'a [&'a str],
    table_positions: &[usize],
    context: &ParseContext,
) -> &'a [&'a str] {
    if context.only_last {
        let last_pos = *table_positions.last().unwrap();
        &lines[last_pos..]
    } else if let Some(target_method) = context.only_method {
        let mut found_range = None;
        for (i, &start) in table_positions.iter().enumerate() {
            let end = table_positions.get(i + 1).copied().unwrap_or(lines.len());
            if let Some(method) = extract_estimation_method(lines[start].trim())
                && method == target_method
            {
                found_range = Some(start..end);
                break;
            }
        }
        match found_range {
            Some(range) => &lines[range],
            None => &[], // No matching table found
        }
    } else {
        lines // Parse all tables
    }
}

/// Format CSV header with proper escaping for parameters that contain commas or spaces
pub fn format_csv_header(parameters: &[String]) -> String {
    let header: Vec<String> = parameters
        .iter()
        .map(|param| {
            if param.contains(',') || param.contains(' ') {
                format!("\"{}\"", param)
            } else {
                param.clone()
            }
        })
        .collect();
    header.join(",")
}

/// Parse parameter names from ITERATION header line
pub fn parse_iteration_header(line: &str) -> Vec<String> {
    split_table_row(line)
        .into_iter()
        .map(str::to_string)
        .collect()
}

/// Parse numeric values from a data row
pub fn parse_numeric_row(line: &str) -> Vec<f64> {
    split_table_row(line)
        .into_iter()
        .filter_map(|s| s.parse().ok())
        .collect()
}

/// Split a NONMEM output-table row.
/// It's mostly space separated but it could also be comma separated.
/// There could be commas in the headers so we can't just split on it.
/// https://nmhelp.tingjieguo.com/format
pub fn split_table_row(line: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut in_parens = false;
    let mut in_quote = false;
    let mut start: Option<usize> = None;
    for (i, c) in line.char_indices() {
        let is_separator = !in_quote && !in_parens && (c.is_whitespace() || c == ',');
        if is_separator {
            if let Some(s) = start.take() {
                tokens.push(strip_quotes(&line[s..i]));
            }
            continue;
        }
        if start.is_none() {
            start = Some(i);
        }
        if c == '"' {
            in_quote = !in_quote;
        } else if !in_quote {
            match c {
                '(' => in_parens = true,
                ')' => in_parens = false,
                _ => {}
            }
        }
    }
    if let Some(s) = start {
        tokens.push(strip_quotes(&line[s..]));
    }
    tokens
}

fn strip_quotes(s: &str) -> &str {
    s.strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .unwrap_or(s)
}
