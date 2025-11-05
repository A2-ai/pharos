use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::Result;
use fs_err as fs;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::parsing::{self, ParseContext};
use crate::estimation::{EstimationMethod, extract_estimation_method};

// Custom serialization for BTreeMap<(String, String), f64>
fn serialize_correlations<S>(
    correlations: &BTreeMap<(String, String), f64>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let string_map: BTreeMap<String, f64> = correlations
        .iter()
        .map(|((param1, param2), value)| (format!("{param1}-{param2}"), *value))
        .collect();
    string_map.serialize(serializer)
}

fn deserialize_correlations<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<(String, String), f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let string_map: BTreeMap<String, f64> = BTreeMap::deserialize(deserializer)?;
    let tuple_map = string_map
        .into_iter()
        .filter_map(|(key, value)| {
            let parts: Vec<&str> = key.split('-').collect();
            if parts.len() == 2 {
                Some(((parts[0].to_string(), parts[1].to_string()), value))
            } else {
                None
            }
        })
        .collect();
    Ok(tuple_map)
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CorrelationMatrix {
    /// Estimation method extracted from TABLE header
    pub method: Option<EstimationMethod>,
    /// Parameter names from the NAME header line. This is the same as the first column.
    /// Only used if we want to recreate the correlation matrix
    pub parameters: Vec<String>,
    /// Correlation values stored as parameter pair -> correlation value
    /// Both (param1, param2) and (param2, param1) are stored for symmetric access
    #[serde(
        serialize_with = "serialize_correlations",
        deserialize_with = "deserialize_correlations"
    )]
    pub correlations: BTreeMap<(String, String), f64>,
}

impl CorrelationMatrix {
    pub fn new(method: Option<EstimationMethod>) -> Self {
        Self {
            method,
            parameters: Vec::new(),
            correlations: BTreeMap::new(),
        }
    }

    /// Get correlation between two parameters (order doesn't matter due to symmetry)
    pub fn get_correlation(&self, param1: &str, param2: &str) -> Option<f64> {
        self.correlations
            .get(&(param1.to_string(), param2.to_string()))
            .copied()
    }

    pub fn to_csv(&self) -> String {
        let mut lines = Vec::new();

        // Create header: NAME + parameter names
        let mut header = vec!["NAME".to_string()];
        header.extend(self.parameters.iter().cloned());
        lines.push(parsing::format_csv_header(&header));

        // Create rows for each parameter
        for row_param in &self.parameters {
            let mut row = vec![row_param.clone()];
            for col_param in &self.parameters {
                let correlation = self.get_correlation(row_param, col_param).unwrap_or(0.0);
                row.push(format!("{:.5E}", correlation));
            }
            lines.push(row.join(","));
        }

        lines.join("\n")
    }

    pub fn get_parameters_over_threshold(&self, threshold: f64) -> Vec<((&str, &str), f64)> {
        let mut out = Vec::new();

        for ((param1, param2), val) in &self.correlations {
            if param1 == param2 {
                continue;
            }
            if (*val).abs() >= threshold {
                // Check if we haven't already added it the other way around
                let mut already_present = false;
                for ((p1, p2), _) in &out {
                    if param1 == p2 && param2 == p1 {
                        already_present = true;
                        break;
                    }
                }
                if !already_present {
                    out.push(((param1.as_str(), param2.as_str()), *val));
                }
            }
        }

        out
    }
}

/// Builder-style reader for COR files with filtering options.
#[derive(Clone)]
pub struct CorReader {
    /// Only keep table for specific estimation method
    only_method: Option<EstimationMethod>,
    /// Only keep last table (default: true)
    only_last: bool,
}

impl Default for CorReader {
    fn default() -> Self {
        Self {
            only_method: None,
            only_last: true,
        }
    }
}

impl CorReader {
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

    pub fn parse_file(&self, path: impl AsRef<Path>) -> Result<Vec<CorrelationMatrix>> {
        let file = fs::File::open(path.as_ref())?;
        let reader = BufReader::new(file);
        self.parse(reader)
    }

    pub fn parse<R: BufRead>(&self, mut reader: R) -> Result<Vec<CorrelationMatrix>> {
        // Read entire content into memory
        let mut content = String::new();
        reader.read_to_string(&mut content)?;

        let lines: Vec<&str> = content.lines().collect();
        let table_positions = parsing::find_table_positions(&lines);

        if table_positions.is_empty() {
            return Ok(Vec::new());
        }

        let parse_context = ParseContext {
            only_method: self.only_method,
            only_last: self.only_last,
        };

        let lines_to_parse =
            parsing::select_lines_to_parse(&lines, &table_positions, &parse_context);

        if lines_to_parse.is_empty() {
            return Ok(Vec::new());
        }

        let mut matrices = Vec::new();
        let mut current_matrix = None;
        let mut current_row_idx = 0;

        for line in lines_to_parse {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with("TABLE NO.") {
                if let Some(matrix) = current_matrix.take() {
                    matrices.push(matrix);
                }
                let mut matrix = CorrelationMatrix::new(None);
                matrix.method = extract_estimation_method(trimmed);
                current_matrix = Some(matrix);
                current_row_idx = 0;
                continue;
            }

            if trimmed.starts_with("NAME") {
                let all_params = parsing::parse_iteration_header(trimmed)
                    .into_iter()
                    .skip(1)
                    .collect::<Vec<_>>();
                if let Some(matrix) = current_matrix.as_mut() {
                    matrix.parameters = all_params;
                }
                continue;
            }

            // We're past the header, parse correlation matrix rows
            if let Some(matrix) = current_matrix.as_mut() {
                let values = parsing::parse_numeric_row(trimmed);
                let row_name = &matrix.parameters[current_row_idx];

                for (col_name, value) in matrix.parameters.iter().zip(values.into_iter()) {
                    matrix
                        .correlations
                        .insert((row_name.to_owned(), col_name.to_owned()), value);
                }
                current_row_idx += 1;
            }
        }

        if let Some(matrix) = current_matrix {
            matrices.push(matrix);
        }

        Ok(matrices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::{assert_debug_snapshot, glob};
    use std::path::PathBuf;

    #[test]
    fn can_parse_cor_files() {
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data");
        glob!(test_dir.join("cor"), "*.cor", |path| {
            let reader = CorReader::default();
            let result = reader.parse_file(path).unwrap();
            assert_debug_snapshot!(result[0]);
        });
    }
}
