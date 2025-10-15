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
    line.split_whitespace().map(|s| s.to_string()).collect()
}

/// Parse numeric values from a data row
pub fn parse_numeric_row(line: &str) -> Vec<f64> {
    line.split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect()
}
