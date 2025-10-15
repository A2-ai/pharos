use std::io::{BufRead, BufReader};
use std::path::Path;

use super::parsing::{self, ParseContext};
use crate::estimation::{EstimationMethod, extract_estimation_method};
use anyhow::Result;
use fs_err as fs;

#[derive(Debug, Clone)]
pub struct GradientRow {
    pub iteration: isize,
    pub gradients: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct GradientTable {
    /// Estimation method extracted from TABLE header
    pub method: Option<EstimationMethod>,
    /// Parameter gradient column names (ITERATION, GRD(1), GRD(2), etc.)
    pub parameters: Vec<String>,
    /// Gradient data rows
    pub rows: Vec<GradientRow>,
}

impl GradientTable {
    pub fn to_csv(&self) -> String {
        let mut lines = Vec::new();
        lines.push(parsing::format_csv_header(&self.parameters));

        for row in &self.rows {
            let mut values = Vec::new();
            values.push(row.iteration.to_string());
            values.extend(row.gradients.iter().map(|v| v.to_string()));
            lines.push(values.join(","));
        }

        lines.join("\n")
    }
}

#[derive(Debug, Default, Clone)]
pub struct GrdReader {
    /// Only keep table for specific estimation method
    only_method: Option<EstimationMethod>,
    /// Only keep last table (default: true)
    only_last: bool,
}

impl GrdReader {
    /// Only keep the table corresponding to the given method.
    /// Exclusive of `only_last`: it will be set to false if this method is called
    pub fn only_method(mut self, method: EstimationMethod) -> Self {
        self.only_method = Some(method);
        self.only_last = false;
        self
    }

    /// Keep all tables from the input file.
    /// This will disable `only_last` and `only_method`
    pub fn keep_all_tables(mut self) -> Self {
        self.only_last = false;
        self.only_method = None;
        self
    }

    /// Only keep the last table.
    /// This is the default.
    pub fn only_last(mut self) -> Self {
        self.only_last = true;
        self.only_method = None;
        self
    }

    pub fn parse_file(&self, path: impl AsRef<Path>) -> Result<Vec<GradientTable>> {
        let file = fs::File::open(path.as_ref())?;
        let reader = BufReader::new(file);
        self.parse(reader)
    }

    pub fn parse<R: BufRead>(&self, mut reader: R) -> Result<Vec<GradientTable>> {
        // Read entire content into memory
        let mut content = String::new();
        reader.read_to_string(&mut content)?;

        let lines: Vec<&str> = content.lines().collect();
        let table_positions = parsing::find_table_positions(&lines);

        if table_positions.is_empty() {
            return Ok(Vec::new());
        }

        let context = ParseContext {
            only_method: self.only_method,
            only_last: self.only_last,
        };
        let lines_to_parse = parsing::select_lines_to_parse(&lines, &table_positions, &context);

        if lines_to_parse.is_empty() {
            return Ok(Vec::new());
        }

        // Parse the selected lines
        let mut tables = Vec::new();
        let mut current_method = None;
        let mut current_parameters = None;
        let mut current_rows = Vec::new();

        for line in lines_to_parse {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with("TABLE NO.") {
                // Save previous table if exists
                if let Some(params) = current_parameters.take() {
                    tables.push(GradientTable {
                        method: current_method,
                        parameters: params,
                        rows: std::mem::take(&mut current_rows),
                    });
                }

                current_method = extract_estimation_method(trimmed);
                continue;
            }

            if trimmed.starts_with("ITERATION") {
                let all_params = parsing::parse_iteration_header(trimmed);
                current_parameters = Some(all_params);
                continue;
            }

            // We're past the header, parse gradient data rows
            if current_parameters.is_some() {
                let values = parsing::parse_numeric_row(trimmed);

                if values.len() >= 2 {
                    let iteration = values[0].round() as isize;
                    let gradients = values[1..].to_vec();
                    current_rows.push(GradientRow {
                        iteration,
                        gradients,
                    });
                }
            }
        }

        // Don't forget the last table
        if let Some(params) = current_parameters {
            tables.push(GradientTable {
                method: current_method,
                parameters: params,
                rows: current_rows,
            });
        }

        Ok(tables)
    }
}

#[cfg(test)]
mod tests {
    use insta::{assert_snapshot, glob};
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn can_parse_grd_files() {
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/grd");
        glob!(test_dir, "*.grd", |path| {
            let reader = GrdReader::default();
            let result = reader.parse_file(path).unwrap();
            assert_snapshot!(result[0].to_csv());
        });
    }
}
