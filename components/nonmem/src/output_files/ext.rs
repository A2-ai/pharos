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
const CONDITION_NUMBER_ITERATION: isize = -1000000003;
const FIXED_FLAGS_ITERATION: isize = -1000000006;
const TERMINATION_ITERATION: isize = -1000000007;

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

/// Minimization results for a single estimation method extracted from .ext files
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MinimizationResults {
    pub ofv: Option<f64>,
    pub condition_number: Option<f64>,
    pub termination_code: Option<i32>,
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

    /// Add condition number iteration to the line prefixes
    pub fn with_condition_number(mut self) -> Self {
        let prefix = CONDITION_NUMBER_ITERATION.to_string();
        if !self.line_prefixes.contains(&prefix) {
            self.line_prefixes.push(prefix);
        }
        self
    }

    /// Add termination code iteration to the line prefixes
    pub fn with_termination_codes(mut self) -> Self {
        let prefix = TERMINATION_ITERATION.to_string();
        if !self.line_prefixes.contains(&prefix) {
            self.line_prefixes.push(prefix);
        }
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


/// Generate appropriate label for OMEGA/SIGMA parameters
/// For diagonal parameters: use ETA/EPS numbering (ETA1, ETA2, etc.)
/// For off-diagonal parameters: use ETAj:ETAi or EPSj:EPSi format
fn get_random_effect_label(
    name: &str,
    param_type: ParameterType,
    existing_parameters: &[RandomEffectEstimate],
) -> String {
    if is_diagonal_parameter(name) {
        // Count existing diagonal parameters of this type for proper ETA/EPS numbering
        let existing_count = existing_parameters
            .iter()
            .filter(|p| p.param_type == param_type && is_diagonal_parameter(&p.name))
            .count();

        if param_type == ParameterType::Omega {
            format!("ETA{}", existing_count + 1)
        } else {
            format!("EPS{}", existing_count + 1)
        }
    } else {
        // Off-diagonal parameter: create ETAj:ETAi or EPSj:EPSi label
        let (i, j) = parse_parameter_indices(name)
            .expect("Failed to parse parameter indices from well-formed NONMEM parameter name. Expected format: OMEGA(i,j) or SIGMA(i,j)");
        let prefix = if param_type == ParameterType::Omega {
            "ETA"
        } else {
            "EPS"
        };
        format!("{prefix}{j}:{prefix}{i}")
    }
}

/// Get shrinkage data for OMEGA/SIGMA parameters
/// Returns None for off-diagonal parameters or when shrinkage data is unavailable
fn get_shrinkage_data(
    name: &str,
    param_type: ParameterType,
    fixed: bool,
    value: f64,
    existing_parameters: &[RandomEffectEstimate],
    shk_table: Option<&ShkTable>,
) -> Option<f64> {
    // Off-diagonal parameters don't have shrinkage data
    if !is_diagonal_parameter(name) {
        return None;
    }

    // Fixed parameters with zero value don't have meaningful shrinkage
    if fixed && value == 0.0 {
        return None;
    }

    // Get shrinkage from table if available
    let shk_table = shk_table?;

    // Count existing diagonal parameters of this type to get the correct index
    let existing_count = existing_parameters
        .iter()
        .filter(|p| p.param_type == param_type && is_diagonal_parameter(&p.name))
        .count();

    if param_type == ParameterType::Omega {
        shk_table
            .eta_shrinkage_sd
            .as_ref()
            .and_then(|v| v.get(existing_count))
            .copied()
    } else {
        shk_table
            .eps_shrinkage_sd
            .as_ref()
            .and_then(|v| v.get(existing_count))
            .copied()
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
pub struct RandomEffectEstimate {
    pub name: String,
    pub param_type: ParameterType,
    pub random_effect: String,
    pub estimate: f64,
    pub stderr: Option<f64>,
    pub rse: Option<f64>,
    pub shrinkage: Option<f64>,
    pub fixed: bool,
}

impl RandomEffectEstimate {
    // [name, random_effect, estimate, stderr+rse, shrinkage, fixed]
    pub fn as_string_pieces(&self) -> Vec<String> {
        let mut out = vec![self.name.clone(), self.random_effect.clone()];
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

    /// Returns true if this is an OMEGA parameter
    pub fn is_omega(&self) -> bool {
        matches!(self.param_type, ParameterType::Omega)
    }

    /// Returns true if this is a SIGMA parameter
    pub fn is_sigma(&self) -> bool {
        matches!(self.param_type, ParameterType::Sigma)
    }
}

/// Parse OMEGA/SIGMA parameter indices from parameter name
/// Returns Some((i, j)) if valid, None otherwise
fn parse_parameter_indices(name: &str) -> Option<(u32, u32)> {
    let name = name.trim();

    // Check if it's OMEGA or SIGMA format
    if (!name.starts_with("OMEGA(") && !name.starts_with("SIGMA(")) || !name.ends_with(')') {
        return None;
    }

    // Find the opening parenthesis and extract inner content
    let paren_pos = name.find('(').unwrap();
    let inner = &name[paren_pos + 1..name.len() - 1];
    let parts: Vec<&str> = inner.split(',').collect();

    if parts.len() == 2
        && let (Ok(i), Ok(j)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>())
    {
        Some((i, j))
    } else {
        None
    }
}

fn is_diagonal_parameter(name: &str) -> bool {
    if let Some((i, j)) = parse_parameter_indices(name) {
        i == j
    } else {
        false
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TableParameters {
    pub method: Option<EstimationMethod>,
    pub theta: Vec<ThetaEstimate>,
    pub random_effects: Vec<RandomEffectEstimate>,
}

/// Complete estimation results including parameters and minimization outcomes for a single method
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EstimationResults {
    pub parameters: TableParameters,
    pub minimization_results: MinimizationResults,
}

pub fn get_parameter_estimates(
    path: impl AsRef<Path>,
    ext_reader: &ExtReader,
    shk_tables: Option<Vec<Vec<ShkTable>>>,
    hide_off_diagonals: bool,
) -> Result<Vec<TableParameters>> {
    let estimation_results = get_estimation_results(path, ext_reader, shk_tables, hide_off_diagonals)?;
    Ok(estimation_results
        .into_iter()
        .map(|r| r.parameters)
        .collect())
}

/// Extract TableParameters from a single EstimationTable
fn extract_parameters_from_table(
    table: &EstimationTable,
    shk_table: Option<&ShkTable>,
    hide_off_diagonals: bool,
) -> Result<TableParameters> {
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
        // TODO add comment parsing for transformations
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
        } else if name.starts_with("OMEGA") || name.starts_with("SIGMA") {
            let is_diagonal = is_diagonal_parameter(name);

            // Include if: diagonal OR (off-diagonal AND not fixed AND not hide_off_diagonals)
            if is_diagonal || (!fixed && !hide_off_diagonals) {
                let param_type = if name.starts_with("OMEGA") {
                    ParameterType::Omega
                } else {
                    ParameterType::Sigma
                };

                let random_effect_label = get_random_effect_label(
                    name,
                    param_type,
                    &parameters.random_effects,
                );
                let shrinkage_data = get_shrinkage_data(
                    name,
                    param_type,
                    fixed,
                    value,
                    &parameters.random_effects,
                    shk_table,
                );

                parameters.random_effects.push(RandomEffectEstimate {
                    name: name.clone(),
                    param_type,
                    random_effect: random_effect_label,
                    estimate: value,
                    stderr,
                    rse,
                    shrinkage: shrinkage_data,
                    fixed,
                });
            }
        }
    }

    Ok(parameters)
}

/// Extract MinimizationResults from a single EstimationTable
fn extract_minimization_from_table(
    table: &EstimationTable,
    parameters_only: bool,
) -> MinimizationResults {
    // For condition number and termination code rows, the value is in the first parameter column
    // - If parameters_only() was used: first parameter is at index 0
    // - If parameters_only() was NOT used: first parameter is at index 1 (after ITERATION column)
    let param_value_index = if parameters_only { 0 } else { 1 };

    // Extract OFV from final estimates row (skip for SAEM methods)
    let ofv = if table.method == Some(EstimationMethod::Saem) {
        None
    } else {
        table
            .rows
            .iter()
            .find(|row| row.iteration == FINAL_ESTIMATES_ITERATION)
            .and_then(|row| {
                table
                    .parameters
                    .iter()
                    .position(|p| p == "OBJ")
                    .and_then(|obj_idx| row.values.get(obj_idx).copied())
                    .filter(|v| v.is_finite())
            })
    };

    // Extract condition number from condition number row
    let condition_number = table
        .rows
        .iter()
        .find(|row| row.iteration == CONDITION_NUMBER_ITERATION)
        .and_then(|row| row.values.get(param_value_index).copied());

    // Extract termination code from termination row
    let termination_code = table
        .rows
        .iter()
        .find(|row| row.iteration == TERMINATION_ITERATION)
        .and_then(|row| row.values.get(param_value_index).copied())
        .map(|value| {
            if value == 0.0 {
                None
            } else {
                Some(value as i32)
            }
        })
        .flatten();

    MinimizationResults {
        ofv,
        condition_number,
        termination_code,
    }
}

/// Unified function to get both parameters and minimization results
pub fn get_estimation_results(
    path: impl AsRef<Path>,
    ext_reader: &ExtReader,
    shk_tables: Option<Vec<Vec<ShkTable>>>,
    hide_off_diagonals: bool,
) -> Result<Vec<EstimationResults>> {
    let file = fs::File::open(path.as_ref())?;
    let buf_reader = BufReader::new(file);

    let tables = ext_reader.parse(buf_reader)?;
    let shk_tables = shk_tables.unwrap_or_default();

    let mut results = Vec::new();

    for (table_idx, table) in tables.into_iter().enumerate() {
        if table.parameters.is_empty() {
            continue;
        }

        // Extract parameters from EstimationTable and add shrinkage data
        // NOTE: Using .first() to get the main subpopulation (subpop 1).
        // This ignores any additional subpopulations that may exist.
        // TODO: Consider making subpopulation selection configurable if needed.
        let shk_table = shk_tables.get(table_idx).and_then(|s| s.first());
        let parameters = extract_parameters_from_table(&table, shk_table, hide_off_diagonals)?;

        // Extract minimization results from EstimationTable
        let minimization_results =
            extract_minimization_from_table(&table, ext_reader.parameters_only);

        results.push(EstimationResults {
            parameters,
            minimization_results,
        });
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
            let result = get_parameter_estimates(path, &reader, None, false).unwrap();
            assert_snapshot!(format!("{:#?}", result));
        });
    }

    #[test]
    fn can_extract_parameter_estimates_hiding_off_diags() {
        let reader = ExtReader::default()
            .parameters_only()
            .final_estimates_and_stderr_and_fixed();
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/ext");
        glob!(test_dir, "*.ext", |path| {
            let result = get_parameter_estimates(path, &reader, None, true).unwrap();
            assert_snapshot!(format!("{:#?}", result));
        });
    }

    #[test]
    fn can_extract_estimation_results() {
        let reader = ExtReader::default()
            .final_estimates_and_stderr_and_fixed()
            .with_condition_number()
            .with_termination_codes()
            .keep_all_tables();
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/ext");
        glob!(test_dir, "*.ext", |path| {
            let result = get_estimation_results(path, &reader, None, false).unwrap();
            assert_snapshot!(format!("{:#?}", result));
        });
    }
}
