use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::parsing::{self, ParseContext};
use crate::estimation::{EstimationMethod, extract_estimation_method};
use crate::Model;
use crate::parsing::BlockStructure;
use anyhow::Result;
use config::CommentType;
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

    pub fn parse_file(&self, path: impl AsRef<Path>, comment_type: Option<CommentType>) -> Result<Vec<GradientTable>> {
        let path = path.as_ref();
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        let mut tables = self.parse(reader)?;

        // Try to find corresponding .mod file and update parameter names
        let mod_path = path.with_extension("mod");
        if mod_path.exists() {
            if let Ok(model_content) = fs::read_to_string(&mod_path) {
                if let Ok(mut model) = Model::parse(&model_content) {
                    if let Some(c) = comment_type {
                        model.parse_comments(c);
                    }

                    // Build parameter names map
                    let mut parameter_names = HashMap::new();
                    for (i, param) in model.theta_parameters.iter().enumerate() {
                        parameter_names.insert(format!("THETA{}", i + 1), param.name());
                    }
                    let mut num_omega = 1;
                    for block in &model.omega_blocks {
                        if block.structure != BlockStructure::Diagonal {
                            continue;
                        }
                        for param in &block.parameters {
                            parameter_names.insert(format!("OMEGA({num_omega},{num_omega})"), param.name());
                            num_omega += 1;
                        }
                    }
                    let mut num_sigma = 1;
                    for block in &model.sigma_blocks {
                        if block.structure != BlockStructure::Diagonal {
                            continue;
                        }
                        for param in &block.parameters {
                            parameter_names.insert(format!("SIGMA({num_sigma},{num_sigma})"), param.name());
                            num_sigma += 1;
                        }
                    }

                    // Count non-fixed parameters to determine GRD mapping
                    // Order: THETAs, ETAs (OMEGAs), EPSs (SIGMAs)
                    let num_theta = model.theta_parameters.iter().filter(|p| !p.is_fixed).count();

                    let mut num_omega = 0;
                    let mut omega_param_map = Vec::new(); // Track (param_index, param_key)
                    for block in &model.omega_blocks {
                        if block.structure != BlockStructure::Diagonal {
                            continue;
                        }
                        for param in &block.parameters {
                            if !param.is_fixed {
                                num_omega += 1;
                                omega_param_map.push((num_omega, format!("OMEGA({num_omega},{num_omega})")));
                            }
                        }
                    }

                    let mut num_sigma = 0;
                    let mut sigma_param_map = Vec::new(); // Track (param_index, param_key)
                    for block in &model.sigma_blocks {
                        if block.structure != BlockStructure::Diagonal {
                            continue;
                        }
                        for param in &block.parameters {
                            if !param.is_fixed {
                                num_sigma += 1;
                                sigma_param_map.push((num_sigma, format!("SIGMA({num_sigma},{num_sigma})")));
                            }
                        }
                    }

                    // Update gradient table parameter names
                    for table in &mut tables {
                        for param_name in &mut table.parameters {
                            // Skip ITERATION column, only update gradient columns
                            if param_name.starts_with("GRD(") && param_name.ends_with(')') {
                                // Extract parameter number from GRD(n) format
                                if let Some(num_str) = param_name.strip_prefix("GRD(").and_then(|s| s.strip_suffix(')')) {
                                    if let Ok(grd_num) = num_str.parse::<usize>() {
                                        let new_name = if grd_num <= num_theta {
                                            // GRD(1) to GRD(N) -> non-fixed THETA(1) to THETA(N)
                                            let param_key = format!("THETA{}", grd_num);
                                            if let Some(Some(name)) = parameter_names.get(&param_key) {
                                                format!("GRD({})", name)
                                            } else {
                                                format!("GRD(THETA{})", grd_num)
                                            }
                                        } else if grd_num <= num_theta + num_omega {
                                            // GRD(N+1) to GRD(N+M) -> non-fixed ETA(1) to ETA(M)
                                            let eta_idx = grd_num - num_theta;
                                            if let Some((_, param_key)) = omega_param_map.get(eta_idx.saturating_sub(1)) {
                                                if let Some(Some(name)) = parameter_names.get(param_key) {
                                                    format!("GRD({})", name)
                                                } else {
                                                    format!("GRD(ETA{})", eta_idx)
                                                }
                                            } else {
                                                format!("GRD(ETA{})", eta_idx)
                                            }
                                        } else if grd_num <= num_theta + num_omega + num_sigma {
                                            // GRD(N+M+1) to GRD(N+M+K) -> non-fixed EPS(1) to EPS(K)
                                            let eps_idx = grd_num - num_theta - num_omega;
                                            if let Some((_, param_key)) = sigma_param_map.get(eps_idx.saturating_sub(1)) {
                                                if let Some(Some(name)) = parameter_names.get(param_key) {
                                                    format!("GRD({})", name)
                                                } else {
                                                    format!("GRD(EPS{})", eps_idx)
                                                }
                                            } else {
                                                format!("GRD(EPS{})", eps_idx)
                                            }
                                        } else {
                                            param_name.clone()
                                        };
                                        *param_name = new_name;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(tables)
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
            let result = reader.parse_file(path, None).unwrap();
            assert_snapshot!(result[0].to_csv());
        });
    }
}
