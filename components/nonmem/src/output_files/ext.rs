use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use super::column_is_kept;
use super::parsing::{self, ParseContext};
use crate::estimation::{EstimationMethod, extract_estimation_method};
use crate::output_files::shk::ShkTable;
use nonmem_parser::ParameterInfo;

use anyhow::{Context, Result, bail};
use fs_err as fs;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

fn fmt_sig_n_digits(num: f64, significant_digits: usize) -> String {
    if !num.is_finite() {
        return "NA".to_string();
    }
    if num == 0.0 {
        return "0".to_string();
    }
    let abs = num.abs();
    // Determine digits before decimal
    let digits = abs.log10().floor() as i32 + 1;
    // total significant digits = 4 -> decimal places = 4 - digits (min 0, max say 8)
    let mut dp = significant_digits as i32 - digits;
    dp = dp.clamp(0, 8);
    format!("{:.*}", dp as usize, num)
}

// NONMEM iteration numbers for different row types
const FINAL_ESTIMATES_ITERATION: isize = -1000000000;
const STDERR_ITERATION: isize = -1000000001;
const CONDITION_NUMBER_ITERATION: isize = -1000000003;
const SD_CORR_ITERATION: isize = -1000000004;
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
    /// The columns a reader reports, in table order: parameter columns that pass
    /// `column_is_kept`, minus off-diagonal OMEGA/SIGMA cells that are fixed or, when
    /// `hide_off_diagonals` is set, all of them. This is the one place that decides it.
    pub(crate) fn reported_columns(
        &self,
        parameters: Option<&BTreeMap<String, ParameterInfo>>,
        hide_off_diagonals: bool,
    ) -> Result<Vec<ReportedColumn<'_>>> {
        let row_values = |iteration| {
            self.rows
                .iter()
                .find(|row| row.iteration == iteration)
                .map(|row| &row.values)
        };
        let values = row_values(FINAL_ESTIMATES_ITERATION);
        let fixed_flags = row_values(FIXED_FLAGS_ITERATION);

        let mut columns = Vec::new();
        for (index, name) in self.parameters.iter().enumerate() {
            // NONMEM writes OMEGA(1,1)/SIGMA(1,1) even when the model declares
            // neither. Skip those columns before anything is computed for them.
            if !column_is_kept(parameters, name) {
                continue;
            }
            let (param_type, coordinate) = if name.starts_with("THETA") {
                (ParameterType::Theta, None)
            } else if name.starts_with("OMEGA") {
                (ParameterType::Omega, Some(parse_parameter_indices(name)?))
            } else if name.starts_with("SIGMA") {
                (ParameterType::Sigma, Some(parse_parameter_indices(name)?))
            } else {
                continue;
            };
            let value = values
                .and_then(|v| v.get(index).copied())
                .unwrap_or(f64::NAN);
            let fixed = fixed_flags
                .and_then(|v| v.get(index).copied())
                .is_some_and(is_fixed_flag);
            if let Some((row, col)) = coordinate
                && row != col
                && (fixed || hide_off_diagonals)
            {
                continue;
            }
            columns.push(ReportedColumn {
                index,
                name,
                param_type,
                coordinate,
                value,
                fixed,
            });
        }
        Ok(columns)
    }

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

    /// Add SD/correlation iteration to the line prefixes
    pub fn with_sd_corr(mut self) -> Self {
        let prefix = SD_CORR_ITERATION.to_string();
        if !self.line_prefixes.contains(&prefix) {
            self.line_prefixes.push(prefix);
        }
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
        let path = path.as_ref();
        let f = fs::File::open(path)?;
        let buf = BufReader::new(f);
        self.parse(buf)
            .with_context(|| format!("failed to parse {}", path.display()))
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
                let Some(&first) = values.first() else {
                    bail!("malformed .ext data row (no numeric values): {trimmed:?}");
                };
                let iteration = first.round() as isize;

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

pub use nonmem_parser::ParameterType;

/// Generate appropriate label for OMEGA/SIGMA parameters from the (i, j) coordinate.
/// For diagonal parameters: use ETA/EPS numbering (ETA1, ETA2, etc.)
/// For off-diagonal parameters: use ETAj:ETAi or EPSj:EPSi format
fn get_random_effect_label(param_type: ParameterType, i: u32, j: u32) -> String {
    let prefix = param_type.prefix();
    if i == j {
        format!("{prefix}{i}")
    } else {
        format!("{prefix}{j}:{prefix}{i}")
    }
}

/// Get shrinkage data for the diagonal OMEGA(i,i)/SIGMA(i,i) parameter.
/// Returns None for off-diagonal parameters or when shrinkage data is unavailable
fn get_shrinkage_data(
    i: u32,
    j: u32,
    param_type: ParameterType,
    fixed: bool,
    value: f64,
    shk_table: Option<&ShkTable>,
) -> Option<f64> {
    // Off-diagonal parameters don't have shrinkage data
    if i != j {
        return None;
    }

    // Fixed parameters with zero value don't have meaningful shrinkage
    if fixed && value == 0.0 {
        return None;
    }

    // Get shrinkage from table if available
    let shk_table = shk_table?;

    // Shrinkage vectors are indexed by the zero-based ETA/EPS number
    let index = i as usize - 1;

    if param_type == ParameterType::Omega {
        shk_table
            .eta_shrinkage_sd
            .as_ref()
            .and_then(|v| v.get(index))
            .copied()
    } else {
        shk_table
            .eps_shrinkage_sd
            .as_ref()
            .and_then(|v| v.get(index))
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
    pub fn as_string_pieces(&self, sig_digits: usize) -> Vec<String> {
        let mut out = vec![self.name.clone()];
        out.push(fmt_sig_n_digits(self.estimate, sig_digits));

        if let Some(se) = self.stderr {
            let s = if let Some(rse) = self.rse {
                format!(
                    "{} ({}%)",
                    fmt_sig_n_digits(se, sig_digits),
                    fmt_sig_n_digits(rse, sig_digits)
                )
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
    pub sd: Option<f64>,
    pub corr: Option<f64>,
    pub stderr: Option<f64>,
    pub rse: Option<f64>,
    pub shrinkage: Option<f64>,
    pub fixed: bool,
    pub diagonal: bool,
}

impl RandomEffectEstimate {
    // [name, random_effect, estimate, stderr+rse, shrinkage, fixed]
    pub fn as_string_pieces(&self, sig_digits: usize) -> Vec<String> {
        let mut out = vec![self.name.clone(), self.random_effect.clone()];
        out.push(fmt_sig_n_digits(self.estimate, sig_digits));

        if let Some(se) = self.stderr {
            let s = if let Some(rse) = self.rse {
                format!(
                    "{} ({}%)",
                    fmt_sig_n_digits(se, sig_digits),
                    fmt_sig_n_digits(rse, sig_digits)
                )
            } else {
                se.to_string()
            };
            out.push(s);
        } else {
            out.push("N/A".to_string());
        }

        if let Some(sd) = self.shrinkage {
            out.push(fmt_sig_n_digits(sd, sig_digits));
        } else {
            out.push("N/A".to_string());
        }

        if self.fixed {
            out.push("yes".to_string());
        } else {
            out.push("no".to_string());
        }

        if self.diagonal {
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
/// Returns (i, j) if valid (both 1-based), errors on malformed names
pub(crate) fn parse_parameter_indices(name: &str) -> Result<(u32, u32)> {
    let name = name.trim();

    // Check if it's OMEGA or SIGMA format
    if (!name.starts_with("OMEGA(") && !name.starts_with("SIGMA(")) || !name.ends_with(')') {
        bail!("malformed OMEGA/SIGMA parameter name in .ext file: {name:?}");
    }

    // Find the opening parenthesis and extract inner content
    let paren_pos = name.find('(').unwrap();
    let inner = &name[paren_pos + 1..name.len() - 1];
    let parts: Vec<&str> = inner.split(',').collect();

    if parts.len() == 2
        && let (Ok(i), Ok(j)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>())
        && i >= 1
        && j >= 1
    {
        Ok((i, j))
    } else {
        bail!("malformed OMEGA/SIGMA parameter name in .ext file: {name:?}");
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TableParameters {
    pub method: Option<EstimationMethod>,
    pub theta: Vec<ThetaEstimate>,
    pub random_effects: Vec<RandomEffectEstimate>,
}

/// A parameter column of an `EstimationTable` that a reader reports.
pub(crate) struct ReportedColumn<'a> {
    /// Position in `EstimationTable::parameters` and in every row.
    pub index: usize,
    pub name: &'a str,
    pub param_type: ParameterType,
    /// The 1-based (row, col) of an OMEGA/SIGMA cell; `None` for THETA.
    pub coordinate: Option<(u32, u32)>,
    /// NaN when there is no final-estimates row.
    pub value: f64,
    pub fixed: bool,
}

/// Whether a value in the fixed-flags row marks a fixed parameter.
fn is_fixed_flag(value: f64) -> bool {
    value == 1.0 || (value.is_finite() && value.abs() == 1e10)
}

/// Raw results for a single estimation method: the unmodified ext table (columns
/// keyed by NONMEM name), the shrinkage block matched to it by method and occurrence,
/// and the minimization outcomes.
#[derive(Debug, Clone)]
pub struct EstimationResults {
    pub table: EstimationTable,
    pub shrinkage: Option<ShkTable>,
    pub minimization_results: MinimizationResults,
}

/// Annotate the raw results of one estimation table into `TableParameters`.
///
/// Applies the membership rule (`column_is_kept`), labels, RSE, shrinkage and
/// off-diagonal hiding. `name` holds the label when `parameters` gives one, and the
/// NONMEM column name otherwise. A THETA missing from `parameters`, or `None`
/// `parameters`, uses the default transform.
pub fn annotate_table(
    results: &EstimationResults,
    hide_off_diagonals: bool,
    parameters: Option<&BTreeMap<String, ParameterInfo>>,
) -> Result<TableParameters> {
    let table = &results.table;
    let shrinkage = results.shrinkage.as_ref();
    let stderr_row = table
        .rows
        .iter()
        .find(|row| row.iteration == STDERR_ITERATION);
    let sd_corr_row = table
        .rows
        .iter()
        .find(|row| row.iteration == SD_CORR_ITERATION);

    let mut annotated = TableParameters {
        method: table.method,
        ..Default::default()
    };

    for column in table.reported_columns(parameters, hide_off_diagonals)? {
        let ReportedColumn {
            index,
            name,
            param_type,
            coordinate,
            value,
            fixed,
        } = column;

        let stderr = stderr_row
            .and_then(|row| row.values.get(index).copied())
            .filter(|v| v.is_finite() && *v != 0.0 && *v != 1e10);
        let info = parameters.and_then(|p| p.get(name));
        let transform = info
            .and_then(|info| info.transform.clone())
            .unwrap_or_default();
        let label = info
            .and_then(|info| info.label.clone())
            .unwrap_or_else(|| name.to_string());
        let rse = stderr.map(|se| transform.compute_rse(value, se, &param_type));

        let Some((row, col)) = coordinate else {
            annotated.theta.push(ThetaEstimate {
                name: label,
                estimate: value,
                stderr,
                rse,
                fixed,
            });
            continue;
        };

        let is_diagonal = row == col;
        let sd_corr_value = sd_corr_row
            .and_then(|r| r.values.get(index).copied())
            .filter(|v| v.is_finite());
        let shrinkage_data = get_shrinkage_data(row, col, param_type, fixed, value, shrinkage)
            .filter(|&v| v != 1e-10);

        annotated.random_effects.push(RandomEffectEstimate {
            name: label,
            param_type,
            random_effect: get_random_effect_label(param_type, row, col),
            estimate: value,
            sd: if is_diagonal && !fixed {
                sd_corr_value
            } else {
                None
            },
            corr: if !is_diagonal && !fixed {
                sd_corr_value
            } else {
                None
            },
            stderr,
            rse,
            shrinkage: shrinkage_data,
            fixed,
            diagonal: is_diagonal,
        });
    }

    Ok(annotated)
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
        .and_then(|value| {
            if value == 0.0 {
                None
            } else {
                Some(value as i32)
            }
        });

    MinimizationResults {
        ofv,
        condition_number,
        termination_code,
    }
}

/// Read the tables of an .ext file as raw results, pairing each with its shrinkage
/// block and minimization outcomes. Nothing is labeled, filtered or annotated; see
/// `annotate_table`.
pub fn get_estimation_results(
    path: impl AsRef<Path>,
    ext_reader: &ExtReader,
    shk_tables: Option<Vec<Vec<ShkTable>>>,
) -> Result<Vec<EstimationResults>> {
    let file = fs::File::open(path.as_ref())?;
    let buf_reader = BufReader::new(file);

    let tables = ext_reader.parse(buf_reader)?;
    let shk_tables = shk_tables.unwrap_or_default();

    let mut results = Vec::new();
    // Estimation methods can repeat (e.g. ITS -> IMP -> IMP), so pair each table
    // with the shrinkage block of the same method AND occurrence order rather than
    // always the first match — otherwise every IMP table would bind to the first
    // IMP shrinkage block.
    let mut method_seen: Vec<Option<EstimationMethod>> = Vec::new();

    for table in tables.into_iter() {
        if table.parameters.is_empty() {
            continue;
        }

        let occurrence = method_seen.iter().filter(|m| **m == table.method).count();
        method_seen.push(table.method);

        // NOTE: Using .first() to get the main subpopulation (subpop 1).
        // This ignores any additional subpopulations that may exist.
        // TODO: Consider making subpopulation selection configurable if needed.
        let shrinkage = shk_tables
            .iter()
            .filter(|x| x.first().is_some_and(|t| t.method == table.method))
            .nth(occurrence)
            .and_then(|s| s.first())
            .cloned();

        // Extract minimization results from EstimationTable
        let minimization_results =
            extract_minimization_from_table(&table, ext_reader.parameters_only);

        results.push(EstimationResults {
            table,
            shrinkage,
            minimization_results,
        });
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use insta::{assert_debug_snapshot, assert_snapshot, glob};
    use nonmem_parser::{CommentType, Model};

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
        assert_debug_snapshot!(result[0]);
        assert_debug_snapshot!(result[1]);
    }

    fn parse_model(control_stream: &str) -> Model {
        Model::parse(Path::new("run.mod"), control_stream).unwrap()
    }

    /// Raw results with final estimate, stderr and fixed rows for the given columns.
    fn raw_results(columns: &[&str], estimates: &[f64], stderrs: &[f64]) -> EstimationResults {
        let table = EstimationTable {
            method: None,
            parameters: columns.iter().map(|c| c.to_string()).collect(),
            rows: vec![
                EstimationRow {
                    iteration: FINAL_ESTIMATES_ITERATION,
                    values: estimates.to_vec(),
                },
                EstimationRow {
                    iteration: STDERR_ITERATION,
                    values: stderrs.to_vec(),
                },
                EstimationRow {
                    iteration: FIXED_FLAGS_ITERATION,
                    values: vec![0.0; columns.len()],
                },
            ],
        };
        EstimationResults {
            table,
            shrinkage: None,
            minimization_results: MinimizationResults::default(),
        }
    }

    fn annotate_file(
        path: &Path,
        reader: &ExtReader,
        hide_off_diagonals: bool,
        parameters: Option<&BTreeMap<String, ParameterInfo>>,
    ) -> Vec<TableParameters> {
        get_estimation_results(path, reader, None)
            .unwrap()
            .iter()
            .map(|r| annotate_table(r, hide_off_diagonals, parameters).unwrap())
            .collect()
    }

    #[test]
    fn can_extract_parameter_estimates() {
        let reader = ExtReader::default()
            .parameters_only()
            .final_estimates_and_stderr_and_fixed();
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/ext/bql.ext");
        let result = annotate_file(&path, &reader, false, None);
        assert_debug_snapshot!(result);
    }

    #[test]
    fn can_extract_parameter_estimates_hiding_off_diags() {
        let reader = ExtReader::default()
            .parameters_only()
            .with_sd_corr()
            .final_estimates_and_stderr_and_fixed();
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/ext/bql.ext");
        let result = annotate_file(&path, &reader, true, None);
        assert_snapshot!(format!("{:#?}", result));
    }

    #[test]
    fn can_extract_estimation_results() {
        let reader = ExtReader::default()
            .final_estimates_and_stderr_and_fixed()
            .with_condition_number()
            .with_termination_codes()
            .with_sd_corr()
            .keep_all_tables();
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/ext/bql.ext");
        // Each table annotated, next to its minimization results.
        let result: Vec<_> = get_estimation_results(&path, &reader, None)
            .unwrap()
            .into_iter()
            .map(|r| {
                (
                    annotate_table(&r, false, None).unwrap(),
                    r.minimization_results,
                )
            })
            .collect();
        assert_snapshot!(format!("{:#?}", result));
    }

    #[test]
    fn malformed_param_names_error_instead_of_panicking() {
        // A header name cut off mid-token previously hit `.expect()` and panicked.
        assert!(parse_parameter_indices("OMEGA(1,").is_err());
        assert!(parse_parameter_indices("OMEGA").is_err());
        assert!(parse_parameter_indices("SIGMA").is_err());
        assert!(parse_parameter_indices("OMEGA(0,0)").is_err());
        assert!(parse_parameter_indices("SIGMA(2,0)").is_err());
        let table = raw_results(&["OMEGA(0,0)"], &[0.1], &[0.01]);
        assert!(annotate_table(&table, false, None).is_err());
    }

    #[test]
    fn non_numeric_data_row_errors_instead_of_panicking() {
        use std::io::Cursor;
        // A data row with no parseable numbers used to panic on `values[0]`.
        let truncated = "TABLE NO. 1\nITERATION THETA1 SIGMA(1,1)\nnot a number\n";
        let reader = ExtReader::default();
        assert!(
            reader.parse(Cursor::new(truncated)).is_err(),
            "expected error, not panic, for a non-numeric data row"
        );
    }

    #[test]
    fn shrinkage_pairs_by_method_occurrence_not_first_match() {
        use std::io::Write;

        // Each block has a distinct THETA1 (1/2/3) so a result can be tied to its
        // source table independently of the shrinkage.
        let ext = "\
TABLE NO.     1: Iterative Two Stage: Problem=1 Subproblem=0
 ITERATION    THETA1       SIGMA(1,1)
  -1000000000  1.00000E+00  5.00000E-01
  -1000000001  1.00000E-01  5.00000E-02
  -1000000006  0.00000E+00  0.00000E+00
TABLE NO.     2: Importance Sampling: Problem=1 Subproblem=0
 ITERATION    THETA1       SIGMA(1,1)
  -1000000000  2.00000E+00  6.00000E-01
  -1000000001  1.00000E-01  5.00000E-02
  -1000000006  0.00000E+00  0.00000E+00
TABLE NO.     3: Importance Sampling: Problem=1 Subproblem=0
 ITERATION    THETA1       SIGMA(1,1)
  -1000000000  3.00000E+00  7.00000E-01
  -1000000001  1.00000E-01  5.00000E-02
  -1000000006  0.00000E+00  0.00000E+00
";
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(ext.as_bytes()).unwrap();

        // Distinct EPS (SIGMA) shrinkage per step; the two IMP blocks differ.
        let shk = |method: EstimationMethod, eps: f64| {
            vec![ShkTable {
                method: Some(method),
                eps_shrinkage_sd: Some(vec![eps]),
                ..Default::default()
            }]
        };
        let shk_tables = vec![
            shk(EstimationMethod::Its, 10.0),
            shk(EstimationMethod::Imp, 20.0),
            shk(EstimationMethod::Imp, 30.0),
        ];

        let reader = ExtReader::default()
            .final_estimates_and_stderr_and_fixed()
            .keep_all_tables();
        let results = get_estimation_results(file.path(), &reader, Some(shk_tables)).unwrap();

        assert_eq!(results.len(), 3);
        // THETA1 identifies the source table; SIGMA(1,1) is the sole random effect.
        let annotated: Vec<_> = results
            .iter()
            .map(|r| annotate_table(r, false, None).unwrap())
            .collect();
        let theta1 = |t: &TableParameters| t.theta[0].estimate;
        let eps_shrinkage = |t: &TableParameters| t.random_effects[0].shrinkage;

        assert_eq!(theta1(&annotated[0]), 1.0);
        assert_eq!(eps_shrinkage(&annotated[0]), Some(10.0));

        assert_eq!(theta1(&annotated[1]), 2.0);
        assert_eq!(eps_shrinkage(&annotated[1]), Some(20.0));

        assert_eq!(theta1(&annotated[2]), 3.0);
        assert_eq!(eps_shrinkage(&annotated[2]), Some(30.0));
    }

    #[test]
    fn undeclared_sigma_columns_are_skipped() {
        let reader = ExtReader::default()
            .parameters_only()
            .final_estimates_and_stderr_and_fixed();
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/ext/bql.ext");
        let with_sigma = annotate_file(&path, &reader, false, None);

        // Declare every OMEGA bql.ext reports and no SIGMA.
        let omega_only: BTreeMap<String, ParameterInfo> = with_sigma[0]
            .random_effects
            .iter()
            .filter(|r| r.is_omega())
            .map(|r| (r.name.clone(), ParameterInfo::default()))
            .collect();
        let without_sigma = annotate_file(&path, &reader, false, Some(&omega_only));

        assert!(with_sigma[0].random_effects.iter().any(|r| r.is_sigma()));
        // Omega rows are untouched, labels and shrinkage included.
        let expected: Vec<_> = with_sigma[0]
            .random_effects
            .iter()
            .filter(|r| r.is_omega())
            .cloned()
            .collect();
        assert_eq!(without_sigma[0].random_effects, expected);
        assert_eq!(without_sigma[0].theta, with_sigma[0].theta);
    }

    fn rse_of_theta(control_stream: &str, comment_type: Option<CommentType>, estimate: f64) -> f64 {
        let params = parse_model(control_stream)
            .parameters(comment_type)
            .unwrap();
        let table = raw_results(&["THETA1"], &[estimate], &[0.2]);
        let annotated = annotate_table(&table, false, params.as_ref()).unwrap();
        annotated.theta[0].rse.unwrap()
    }

    #[test]
    fn log_theta_rse_uses_log_scale_se() {
        let expected = ((0.2f64.powi(2)).exp() - 1.0).sqrt() * 100.0;
        for control_stream in [
            "$PROBLEM p\n$THETA (0, 1) ; TVCL (L/h) :LOG\n",
            "$PROBLEM p\n$THETA (0, 1) ; CL :LOG\n",
        ] {
            let rse = rse_of_theta(control_stream, Some(CommentType::Type1), 1.5);
            assert!((rse - expected).abs() < 1e-9, "{control_stream}: {rse}");
            // A log-scale estimate of 0 (a typical value of 1) must stay finite.
            let rse = rse_of_theta(control_stream, Some(CommentType::Type1), 0.0);
            assert!((rse - expected).abs() < 1e-9, "{control_stream}: {rse}");
        }
    }

    #[test]
    fn logit_theta_rse_uses_back_transformed_estimate() {
        let rse = rse_of_theta(
            "$PROBLEM p\n$THETA (0, 1) ; F1 (frac) :logit\n",
            Some(CommentType::Type1),
            2.0,
        );
        // p = 1 / (1 + exp(-2)) = 0.8808, so (1 - p) * se * 100 = 2.384, not p * se * 100
        let p = 1.0 / (1.0 + (-2.0f64).exp());
        assert!((rse - (1.0 - p) * 0.2 * 100.0).abs() < 1e-9, "{rse}");
        assert!((rse - 2.384).abs() < 1e-3, "{rse}");
    }

    #[test]
    fn type2_tagged_theta_rse_uses_transform() {
        let rse = rse_of_theta(
            "$PROBLEM p\n$THETA (0, 1) ; CL ;log\n",
            Some(CommentType::Type2),
            1.5,
        );
        let expected = ((0.2f64.powi(2)).exp() - 1.0).sqrt() * 100.0;
        assert!((rse - expected).abs() < 1e-9, "{rse}");
    }

    #[test]
    fn untagged_parameters_keep_the_default_rse() {
        let control_stream = "$PROBLEM p\n$THETA (0, 1) ; TVCL (L/h)\n$THETA (0, 1) ; WT cov\n$OMEGA 0.1 ; OM1 TVCL :LOG\n$SIGMA 0.1 ; SIG1 :add\n";
        let params = parse_model(control_stream)
            .parameters(Some(CommentType::Type1))
            .unwrap();
        let table = raw_results(
            &["THETA1", "THETA2", "OMEGA(1,1)", "SIGMA(1,1)"],
            &[2.0, -4.0, 0.25, 0.5],
            &[0.2, 0.2, 0.05, 0.05],
        );
        let annotated = annotate_table(&table, false, params.as_ref()).unwrap();
        assert!((annotated.theta[0].rse.unwrap() - 10.0).abs() < 1e-9);
        assert!((annotated.theta[1].rse.unwrap() - 5.0).abs() < 1e-9);
        // OMEGA/SIGMA fall through to se / |estimate| even when tagged.
        assert!((annotated.random_effects[0].rse.unwrap() - 20.0).abs() < 1e-9);
        assert!((annotated.random_effects[1].rse.unwrap() - 10.0).abs() < 1e-9);
    }

    #[test]
    fn rse_needs_a_stderr() {
        let params = parse_model("$PROBLEM p\n$THETA (0, 1) ; CL :LOG\n")
            .parameters(Some(CommentType::Type1))
            .unwrap();
        let table = raw_results(&["THETA1"], &[1.0], &[0.0]);
        let annotated = annotate_table(&table, false, params.as_ref()).unwrap();
        assert_eq!(annotated.theta[0].stderr, None);
        assert_eq!(annotated.theta[0].rse, None);
    }

    #[test]
    fn membership_does_not_depend_on_comment_type_or_tags() {
        let control_stream = "$PROBLEM p\n$THETA (0, 1) ; TVCL (L/h) :LGO\n$THETA 1 ; plain\n$OMEGA 0.1 ; OM1 TVCL :LGO\n$OMEGA 0.1\n";
        let table = raw_results(
            &["THETA1", "THETA2", "OMEGA(1,1)", "OMEGA(2,2)", "SIGMA(1,1)"],
            &[1.0, 2.0, 0.1, 0.2, 1.0],
            &[0.1; 5],
        );
        let model = parse_model(control_stream);
        let columns = |comment_type: Option<CommentType>| -> Vec<String> {
            let params = model.parameters(comment_type).unwrap();
            let annotated = annotate_table(&table, false, params.as_ref()).unwrap();
            let mut keys: Vec<String> = annotated
                .random_effects
                .iter()
                .map(|r| r.random_effect.clone())
                .collect();
            keys.push(annotated.theta.len().to_string());
            keys
        };
        let none = columns(None);
        assert_eq!(none, vec!["ETA1", "ETA2", "2"]);
        assert_eq!(columns(Some(CommentType::Type1)), none);
        assert_eq!(columns(Some(CommentType::Type2)), none);
    }

    #[test]
    fn labels_replace_names_and_unlabeled_parameters_keep_theirs() {
        let params = parse_model(
            "$PROBLEM p\n$THETA (0, 1) ; TVCL (L/h)\n$THETA 1\n$OMEGA 0.1 ; OM1 TVCL :LOG\n",
        )
        .parameters(Some(CommentType::Type1))
        .unwrap();
        let table = raw_results(
            &["THETA1", "THETA2", "OMEGA(1,1)"],
            &[1.0, 2.0, 0.1],
            &[0.1; 3],
        );
        let annotated = annotate_table(&table, false, params.as_ref()).unwrap();
        assert_eq!(annotated.theta[0].name, "TVCL");
        assert_eq!(annotated.theta[1].name, "THETA2");
        assert_eq!(annotated.random_effects[0].name, "OM1 (TVCL)");
    }

    #[test]
    fn undeclared_theta_is_kept_unlabeled_and_undeclared_sigma_is_skipped() {
        let params = parse_model("$PROBLEM p\n$THETA (0, 1) ; TVCL (L/h) :LOG\n$OMEGA 0.1\n")
            .parameters(Some(CommentType::Type1))
            .unwrap();
        let table = raw_results(
            &["THETA1", "THETA2", "OMEGA(1,1)", "SIGMA(1,1)"],
            &[1.0, 3.0, 0.1, 1.0],
            &[0.1, 0.3, 0.01, 0.1],
        );
        let annotated = annotate_table(&table, false, params.as_ref()).unwrap();
        assert_eq!(annotated.theta.len(), 2);
        assert_eq!(annotated.theta[1].name, "THETA2");
        assert!((annotated.theta[1].rse.unwrap() - 10.0).abs() < 1e-9);
        assert_eq!(annotated.random_effects.len(), 1);
        assert!(annotated.random_effects[0].is_omega());
    }

    #[test]
    fn msfi_keeps_every_column_without_labels() {
        let model = parse_model(
            "$PROBLEM msfi\n$INPUT ID DV\n$DATA data.csv\n$MSFI run001.msf\n$PRED\nY = THETA(1) + ETA(1)\n$EST METHOD=1\n",
        );
        let params = model.parameters(Some(CommentType::Type1)).unwrap();
        assert!(params.is_none());
        let table = raw_results(
            &["THETA1", "OMEGA(1,1)", "SIGMA(1,1)"],
            &[1.0, 0.1, 0.2],
            &[0.1; 3],
        );
        let annotated = annotate_table(&table, false, params.as_ref()).unwrap();
        assert_eq!(annotated.theta[0].name, "THETA1");
        let names: Vec<_> = annotated
            .random_effects
            .iter()
            .map(|r| r.name.as_str())
            .collect();
        assert_eq!(names, vec!["OMEGA(1,1)", "SIGMA(1,1)"]);
    }

    #[test]
    fn eta_numbering_and_shrinkage_follow_the_coordinate() {
        // OMEGA(2,2) is not in the map, so it is skipped; OMEGA(3,3) keeps ETA3 and
        // the third shrinkage entry rather than sliding down to ETA2 / the second.
        let mut params = BTreeMap::new();
        for key in ["OMEGA(1,1)", "OMEGA(3,3)", "SIGMA(2,2)"] {
            params.insert(key.to_string(), ParameterInfo::default());
        }
        let mut table = raw_results(
            &["OMEGA(1,1)", "OMEGA(2,2)", "OMEGA(3,3)", "SIGMA(2,2)"],
            &[0.1, 0.2, 0.3, 0.4],
            &[0.01; 4],
        );
        table.shrinkage = Some(ShkTable {
            eta_shrinkage_sd: Some(vec![11.0, 22.0, 33.0]),
            eps_shrinkage_sd: Some(vec![44.0, 55.0]),
            ..Default::default()
        });
        let annotated = annotate_table(&table, false, Some(&params)).unwrap();
        let rows: Vec<_> = annotated
            .random_effects
            .iter()
            .map(|r| (r.random_effect.as_str(), r.shrinkage))
            .collect();
        assert_eq!(
            rows,
            vec![
                ("ETA1", Some(11.0)),
                ("ETA3", Some(33.0)),
                ("EPS2", Some(55.0)),
            ]
        );
    }

    #[test]
    fn off_diagonals_are_hidden_on_request() {
        let params = parse_model("$PROBLEM p\n$OMEGA BLOCK(2)\n0.1\n0.01 0.1\n")
            .parameters(None)
            .unwrap();
        let table = raw_results(
            &["OMEGA(1,1)", "OMEGA(2,1)", "OMEGA(2,2)"],
            &[0.1, 0.01, 0.1],
            &[0.01; 3],
        );
        let shown = annotate_table(&table, false, params.as_ref()).unwrap();
        assert_eq!(shown.random_effects.len(), 3);
        assert_eq!(shown.random_effects[1].random_effect, "ETA1:ETA2");
        let hidden = annotate_table(&table, true, params.as_ref()).unwrap();
        assert_eq!(hidden.random_effects.len(), 2);
    }
}
