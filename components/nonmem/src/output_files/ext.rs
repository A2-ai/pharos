use core::fmt;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use super::parsing::{self, ParseContext};
use crate::estimation::{EstimationMethod, extract_estimation_method};
use crate::output_files::shk::ShkTable;
use anyhow::{Result, bail};
use fs_err as fs;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

fn fmt_sig4(n: f64) -> String {
    if !n.is_finite() {
        return "NA".to_string();
    }
    if n == 0.0 {
        return "0".to_string();
    }
    let abs = n.abs();
    // Determine digits before decimal
    let digits = abs.log10().floor() as i32 + 1;
    // total significant digits = 4 -> decimal places = 4 - digits (min 0, max say 8)
    let mut dp = 4 - digits;
    if dp < 0 {
        dp = 0;
    }
    if dp > 8 {
        dp = 8;
    }
    format!("{:.*}", dp as usize, n)
}

// NONMEM iteration numbers for different row types
const FINAL_ESTIMATES_ITERATION: isize = -1000000000;
const STDERR_ITERATION: isize = -1000000001;
const FIXED_FLAGS_ITERATION: isize = -1000000006;

/// A single row of parameter estimates
#[derive(Debug, Clone)]
pub struct EstimationRow {
    pub iteration: isize,
    pub values: Vec<f64>,
}

/// Represents a single estimation table from a NONMEM .ext file
#[derive(Debug, Clone)]
pub struct EstimationTable {
    /// Estimation method (e.g., "First Order Conditional Estimation", "Iterative Two Stage")
    pub method: Option<EstimationMethod>,
    /// Parameter names from the ITER header line
    pub parameters: Vec<String>,
    /// Rows of parameter values
    /// The size of parameters and rows should match
    pub rows: Vec<EstimationRow>,
}

impl EstimationTable {
    pub fn to_csv(&self) -> String {
        let mut lines = Vec::new();
        lines.push(parsing::format_csv_header(&self.parameters));

        for row in &self.rows {
            let values: Vec<String> = row.values.iter().map(|v| v.to_string()).collect();
            lines.push(values.join(","));
        }

        lines.join("\n")
    }
}

/// Builder-style reader for EXT files with filtering and CSV formatting options.
#[derive(Clone)]
pub struct ExtReader {
    /// Only include rows starting with these prefixes (e.g., "-1000000000" for final estimates)
    line_prefixes: Vec<String>,
    /// Only keep parameter columns (exclude ITERATION and OBJ columns)
    parameters_only: bool,
    /// Only keep the table referring to that estimation method
    only_method: Option<EstimationMethod>,
    /// Only keep the last table. Defaults to true
    only_last: bool,
}

impl Default for ExtReader {
    fn default() -> Self {
        Self {
            line_prefixes: vec![],
            parameters_only: false,
            only_method: None,
            only_last: true,
        }
    }
}

impl ExtReader {
    /// Filter to specific row types by line prefix (e.g., vec!["-1000000000"] for final estimates)
    pub fn filter_by_prefix<S: Into<String>>(mut self, prefixes: Vec<S>) -> Self {
        self.line_prefixes = prefixes.into_iter().map(|s| s.into()).collect();
        self
    }

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

    /// Only include final parameter estimates
    pub fn final_estimates_only(mut self) -> Self {
        self.line_prefixes = vec![FINAL_ESTIMATES_ITERATION.to_string()];
        self
    }

    /// Only include final parameter estimates
    pub fn final_estimates_and_stderr_and_fixed(mut self) -> Self {
        self.line_prefixes = vec![
            FINAL_ESTIMATES_ITERATION.to_string(),
            STDERR_ITERATION.to_string(),
            FIXED_FLAGS_ITERATION.to_string(),
        ];
        self
    }

    /// Exclude ITERATION and OBJ columns, keep only parameter estimates
    pub fn parameters_only(mut self) -> Self {
        self.parameters_only = true;
        self
    }

    pub fn parse_file_batch(
        &self,
        paths: Vec<impl AsRef<Path>>,
    ) -> Result<Vec<(PathBuf, Vec<EstimationTable>)>> {
        let paths = paths.iter().map(|p| p.as_ref()).collect::<Vec<_>>();
        let processed: Vec<_> = paths
            .into_par_iter()
            .map(|p| (p.to_path_buf(), self.parse_file(p)))
            .collect();

        let mut results = Vec::with_capacity(processed.len());
        let mut errors = Vec::with_capacity(processed.len());

        for (p, res) in processed {
            match res {
                Ok(v) => results.push((p, v)),
                Err(e) => errors.push((p, e)),
            }
        }

        let err_msg = errors
            .iter()
            .map(|(p, e)| format!("{p:?}: {e}"))
            .collect::<Vec<_>>()
            .join(", ");

        if !err_msg.is_empty() {
            bail!(err_msg)
        } else {
            Ok(results)
        }
    }

    pub fn parse_file(&self, path: impl AsRef<Path>) -> Result<Vec<EstimationTable>> {
        let f = fs::File::open(path.as_ref())?;
        let buf = BufReader::new(f);
        self.parse(buf)
    }

    pub fn parse<R: BufRead>(&self, mut reader: R) -> Result<Vec<EstimationTable>> {
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
        let mut skip_table = false;

        for line in lines_to_parse {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with("TABLE NO.") {
                if let Some(params) = current_parameters.take() {
                    tables.push(EstimationTable {
                        method: current_method,
                        parameters: params,
                        rows: std::mem::take(&mut current_rows),
                    });
                }

                let method = extract_estimation_method(trimmed);
                if let Some(requested_method) = self.only_method
                    && let Some(curr_method) = method
                {
                    if requested_method != curr_method {
                        skip_table = true;
                    } else {
                        current_method = method;
                    }
                } else {
                    current_method = method;
                }
                continue;
            }

            if trimmed.starts_with("ITERATION") && !skip_table {
                let all_params = parsing::parse_iteration_header(trimmed);

                let params = if self.parameters_only && all_params.len() > 2 {
                    all_params[1..all_params.len() - 1].to_vec()
                } else {
                    all_params
                };

                current_parameters = Some(params);
                continue;
            }

            // We're past the header
            if current_parameters.is_some() && !skip_table {
                let include = if self.line_prefixes.is_empty() {
                    true
                } else {
                    self.line_prefixes
                        .iter()
                        .any(|prefix| trimmed.starts_with(prefix))
                };
                if !include || trimmed.is_empty() {
                    continue;
                }

                let values = parsing::parse_numeric_row(trimmed);
                let iteration = values[0].round() as isize;

                let values = if self.parameters_only && values.len() > 2 {
                    values[1..values.len() - 1].to_vec()
                } else {
                    values
                };

                current_rows.push(EstimationRow { iteration, values });
            }
        }

        // Don't forget the last table
        if let Some(params) = current_parameters {
            tables.push(EstimationTable {
                method: current_method,
                parameters: params,
                rows: current_rows,
            });
        }

        Ok(tables)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParameterType {
    Theta,
    Omega,
    Sigma,
}

impl fmt::Display for ParameterType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParameterType::Theta => write!(f, "THETA"),
            ParameterType::Omega => write!(f, "OMEGA"),
            ParameterType::Sigma => write!(f, "SIGMA"),
        }
    }
}

impl FromStr for ParameterType {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if s.starts_with("OMEGA(") {
            Ok(ParameterType::Omega)
        } else if s.starts_with("SIGMA(") {
            Ok(ParameterType::Sigma)
        } else {
            Ok(ParameterType::Theta)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ThetaEstimate {
    pub name: String,
    pub estimate: f64,
    pub stderr: Option<f64>,
    pub rse: Option<f64>,
    pub fixed: bool,
}

impl ThetaEstimate {
    // [name, estimate, se+rse,  fixed]
    pub fn as_string_pieces(&self) -> Vec<String> {
        let mut out = vec![self.name.clone()];
        out.push(fmt_sig4(self.estimate));

        if let Some(se) = self.stderr {
            let s = if let Some(rse) = self.rse {
                format!("{} ({}%)", fmt_sig4(se), fmt_sig4(rse))
            } else {
                se.to_string()
            };
            out.push(s);
        } else {
            out.push("N/A".to_string());
        }
        if self.fixed {
            out.push("yes".to_string());
        } else {
            out.push("no".to_string());
        }

        out
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OmegaEstimate {
    pub name: String,
    pub eta: String,
    pub estimate: f64,
    pub stderr: Option<f64>,
    pub rse: Option<f64>,
    pub shrinkage: Option<f64>,
    pub fixed: bool,
}

impl OmegaEstimate {
    // [name, eta, estimate, stderr+rse, shrinkage, fixed]
    pub fn as_string_pieces(&self) -> Vec<String> {
        let mut out = vec![self.name.clone(), self.eta.clone()];
        out.push(fmt_sig4(self.estimate));

        if let Some(se) = self.stderr {
            let s = if let Some(rse) = self.rse {
                format!("{} ({}%)", fmt_sig4(se), fmt_sig4(rse))
            } else {
                se.to_string()
            };
            out.push(s);
        } else {
            out.push("N/A".to_string());
        }

        if let Some(sd) = self.shrinkage {
            out.push(fmt_sig4(sd));
        } else {
            out.push("N/A".to_string());
        }

        if self.fixed {
            out.push("yes".to_string());
        } else {
            out.push("no".to_string());
        }

        out
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SigmaEstimate {
    pub name: String,
    pub eps: String,
    pub estimate: f64,
    pub stderr: Option<f64>,
    pub rse: Option<f64>,
    pub shrinkage: Option<f64>,
    pub fixed: bool,
}

impl SigmaEstimate {
    // [name, eps, estimate, stderr+rse, fixed]
    pub fn as_string_pieces(&self) -> Vec<String> {
        let mut out = vec![self.name.clone(), self.eps.clone()];
        out.push(fmt_sig4(self.estimate));

        if let Some(se) = self.stderr {
            let s = if let Some(rse) = self.rse {
                format!("{} ({}%)", fmt_sig4(se), fmt_sig4(rse))
            } else {
                se.to_string()
            };
            out.push(s);
        } else {
            out.push("N/A".to_string());
        }
        
        if let Some(sd) = self.shrinkage {
            out.push(fmt_sig4(sd));
        } else {
            out.push("N/A".to_string());
        }

        if self.fixed {
            out.push("yes".to_string());
        } else {
            out.push("no".to_string());
        }

        out
    }
}

fn is_diagonal_parameter(name: &str) -> bool {
    let name = name.trim();

    // Check if it's OMEGA or SIGMA format
    if (!name.starts_with("OMEGA(") && !name.starts_with("SIGMA(")) || !name.ends_with(')') {
        return false;
    }

    // Find the opening parenthesis and extract inner content
    let paren_pos = name.find('(').unwrap();
    let inner = &name[paren_pos + 1..name.len() - 1];
    let parts: Vec<&str> = inner.split(',').collect();

    if parts.len() == 2
        && let (Ok(i), Ok(j)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>())
    {
        return i == j;
    }

    false
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TableParameters {
    pub method: Option<EstimationMethod>,
    pub theta: Vec<ThetaEstimate>,
    pub omega: Vec<OmegaEstimate>,
    pub sigma: Vec<SigmaEstimate>,
}

pub fn get_parameter_estimates(
    path: impl AsRef<Path>,
    ext_reader: &ExtReader,
    shk_tables: Option<Vec<Vec<ShkTable>>>,
) -> Result<Vec<TableParameters>> {
    let file = fs::File::open(path.as_ref())?;
    let buf_reader = BufReader::new(file);
    let shk_tables = shk_tables.unwrap_or_default();

    let tables = ext_reader
        .clone()
        .parameters_only()
        .final_estimates_and_stderr_and_fixed()
        .parse(buf_reader)?;
    let mut results = Vec::new();

    for (table_idx, table) in tables.into_iter().enumerate() {
        if table.parameters.is_empty() {
            continue;
        }

        let values_row = table
            .rows
            .iter()
            .find(|row| row.iteration == FINAL_ESTIMATES_ITERATION);
        let stderr_row = table
            .rows
            .iter()
            .find(|row| row.iteration == STDERR_ITERATION);
        let fixed_row = table
            .rows
            .iter()
            .find(|row| row.iteration == FIXED_FLAGS_ITERATION);

        let fixed_flags = fixed_row.map(|row| {
            row.values
                .iter()
                .map(|&v| v == 1.0 || (v.is_finite() && v.abs() == 1e10))
                .collect::<Vec<bool>>()
        });

        let mut parameters = TableParameters {
            method: table.method,
            ..Default::default()
        };

        for (i, name) in table.parameters.iter().enumerate() {
            let value = values_row
                .and_then(|row| row.values.get(i).copied())
                .unwrap_or(f64::NAN);
            let fixed = fixed_flags
                .as_ref()
                .and_then(|flags| flags.get(i).copied())
                .unwrap_or(false);

            // Extract stderr and calculate RSE once for all parameter types
            let stderr = stderr_row
                .and_then(|row| row.values.get(i).copied())
                .filter(|v| v.is_finite() && *v != 0.0 && *v != 1e10);
            let rse = if value.is_sign_positive()
                && let Some(se) = stderr
            {
                Some((se / value).abs() * 100.0)
            } else {
                None
            };

            if name.starts_with("THETA") {
                parameters.theta.push(ThetaEstimate {
                    name: name.clone(),
                    estimate: value,
                    stderr,
                    rse,
                    fixed,
                });
            } else if name.starts_with("OMEGA") && is_diagonal_parameter(name) {
                let sd = if let Some(shk_table) = shk_tables.get(table_idx).and_then(|s| s.first())
                {
                    shk_table
                        .eta_shrinkage_sd
                        .as_ref()
                        .and_then(|v| v.get(parameters.omega.len()))
                        .copied()
                } else {
                    None
                };
                parameters.omega.push(OmegaEstimate {
                    name: name.clone(),
                    eta: format!("ETA{}", parameters.omega.len() + 1),
                    estimate: value,
                    stderr,
                    rse,
                    shrinkage: sd,
                    fixed,
                });
            } else if name.starts_with("SIGMA") && is_diagonal_parameter(name) {
                let sd = if let Some(shk_table) = shk_tables.get(table_idx).and_then(|s| s.first())
                {
                    shk_table
                        .eps_shrinkage_sd
                        .as_ref()
                        .and_then(|v| v.get(parameters.sigma.len()))
                        .copied()
                } else {
                    None
                };
                parameters.sigma.push(SigmaEstimate {
                    name: name.clone(),
                    eps: format!("EPS{}", parameters.sigma.len() + 1),
                    estimate: value,
                    stderr,
                    rse,
                    shrinkage: sd,
                    fixed,
                });
            }
        }
        results.push(parameters);
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use insta::{assert_snapshot, glob};

    use super::*;

    #[test]
    fn can_parse_ext_files() {
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/ext");
        glob!(test_dir, "*.ext", |path| {
            let reader = ExtReader::default();
            let result = reader.parse_file(path).unwrap();
            assert_snapshot!(result[0].to_csv());
        });
    }

    #[test]
    fn can_pick_table_by_method() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/ext/itsimp.ext");
        let reader = ExtReader::default().only_method(EstimationMethod::Its);
        let result = reader.parse_file(path).unwrap();
        assert_eq!(result.len(), 1);
        assert_snapshot!(result[0].to_csv());
    }

    #[test]
    fn can_keep_all_the_data() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/ext/itsimp.ext");
        let reader = ExtReader::default().keep_all_tables();
        let result = reader.parse_file(path).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn can_extract_parameter_estimates() {
        let reader = ExtReader::default()
            .parameters_only()
            .final_estimates_and_stderr_and_fixed();
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/ext");
        glob!(test_dir, "*.ext", |path| {
            let result = get_parameter_estimates(path, &reader, None).unwrap();
            assert_snapshot!(format!("{:#?}", result));
        });
    }
}
