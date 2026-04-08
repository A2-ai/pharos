use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::parsing::{self, ParseContext};
use crate::estimation::{EstimationMethod, extract_estimation_method};
use anyhow::Result;
use config::CommentType;
use fs_err as fs;
use nonmem_parser::{Model, ParameterOrdering};

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

    pub fn parse_file(
        &self,
        path: impl AsRef<Path>,
        model: Option<&Model>,
        comment_type: Option<CommentType>,
    ) -> Result<Vec<GradientTable>> {
        let file = fs::File::open(path.as_ref())?;
        let reader = BufReader::new(file);
        self.parse(reader, model, comment_type)
    }

    pub fn parse<R: BufRead>(
        &self,
        mut reader: R,
        model: Option<&Model>,
        comment_type: Option<CommentType>,
    ) -> Result<Vec<GradientTable>> {
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

        // Apply model-based parameter naming if model is provided
        if let Some(model) = model {
            let parameter_names = model.get_parameter_names(comment_type)?;

            let grd_names = build_gradient_names(model, &parameter_names)?;
            update_gradient_table_names(&mut tables, &grd_names);
        }

        Ok(tables)
    }
}

/// Build mapping from GRD(n) to gradient names for non-fixed parameters
fn build_gradient_names(
    model: &Model,
    parameter_names: &BTreeMap<String, Option<String>>,
) -> Result<HashMap<String, String>> {
    let mut grd_names = HashMap::new();
    let mut grd_counter = 1;

    // Add THETAs
    for (i, theta) in model.thetas.iter().enumerate() {
        if !theta.fixed {
            let key = format!("THETA{}", i + 1);
            let name = if let Some(Some(friendly_name)) = parameter_names.get(&key) {
                format!("GRD({friendly_name})")
            } else {
                format!("GRD({key})")
            };
            grd_names.insert(format!("GRD({grd_counter})"), name);
            grd_counter += 1;
        }
    }

    // Add OMEGAs (ColumnMajor for GRD files)
    let omega_params = model.get_omega_parameters(ParameterOrdering::ColumnMajor)?;
    for (param_name, eta_label, _, block_fixed) in omega_params {
        if !block_fixed {
            let name = if let Some(Some(friendly_name)) = parameter_names.get(&param_name) {
                format!("GRD({friendly_name})")
            } else {
                format!("GRD({eta_label})")
            };
            grd_names.insert(format!("GRD({grd_counter})"), name);
            grd_counter += 1;
        }
    }

    // Add SIGMAs (ColumnMajor for GRD files)
    let sigma_params = model.get_sigma_parameters(ParameterOrdering::ColumnMajor)?;
    for (param_name, eps_label, _, block_fixed) in sigma_params {
        if !block_fixed {
            let name = if let Some(Some(friendly_name)) = parameter_names.get(&param_name) {
                format!("GRD({friendly_name})")
            } else {
                format!("GRD({eps_label})")
            };
            grd_names.insert(format!("GRD({grd_counter})"), name);
            grd_counter += 1;
        }
    }

    Ok(grd_names)
}

/// Update gradient table parameter names using the mapping
fn update_gradient_table_names(tables: &mut [GradientTable], grd_names: &HashMap<String, String>) {
    tables.iter_mut().for_each(|table| {
        table
            .parameters
            .iter_mut()
            .filter(|name| *name != "ITERATION")
            .for_each(|param_name| {
                if let Some(new_name) = grd_names.get(param_name) {
                    *param_name = new_name.clone();
                }
            });
    });
}

#[cfg(test)]
mod tests {
    use insta::{assert_snapshot, glob};
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_grd_parsing_scenarios() {
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data");
        glob!(test_dir.join("grd"), "*.grd", |path| {
            let run_name = path.file_stem().unwrap().to_string_lossy();
            let model_path = test_dir
                .join("model_paths")
                .join(format!("{}.mod", run_name));
            let model = if model_path.exists() {
                let model_content = fs::read_to_string(model_path).unwrap();
                Some(Model::parse(&model_content).unwrap())
            } else {
                None
            };

            let test_scenarios = vec![
                ("no_comments", None),
                ("type1_comments", Some(CommentType::Type1)),
            ];

            for (scenario_name, comment_type) in test_scenarios {
                let reader = GrdReader::default();
                let result = reader
                    .parse_file(path, model.as_ref(), comment_type)
                    .unwrap();

                let snapshot_name = format!("{run_name}_{scenario_name}");
                assert_snapshot!(snapshot_name, result[0].to_csv());
            }
        });
    }
}
