use std::collections::HashMap;
use std::path::Path;

use crate::Model;
use crate::output_files::cor::{CorReader, CorrelationMatrix};
use crate::output_files::ext::{
    ExtReader, MinimizationResults, TableParameters, get_estimation_results,
};
use crate::output_files::lst::{LstSummary, parse_lst};
use crate::output_files::shk::ShkReader;
use crate::parsing::ParameterOrdering;
use anyhow::{Result, bail};
use config::CommentType;
use fs_err as fs;
use serde::{Deserialize, Serialize};

pub mod cor;
pub mod ext;
pub mod grd;
pub mod lst;
mod parsing;
pub mod shk;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Summary {
    pub run_name: String,
    pub lst: LstSummary,
    pub minimization_results: Vec<MinimizationResults>,
    pub parameters: TableParameters,
    pub parameter_names: HashMap<String, Option<String>>,
    pub correlation_matrix: CorrelationMatrix,
}

pub fn get_summary(
    directory: impl AsRef<Path>,
    comment_type: Option<CommentType>,
    hide_off_diagonals: bool,
) -> Result<Summary> {
    let directory = directory.as_ref();
    if !directory.is_dir() {
        bail!("Directory does not exist: {}", directory.display());
    }
    let run_name = directory.file_name().and_then(|n| n.to_str()).unwrap();
    let model_path = directory.join(format!("{run_name}.mod"));

    let lst_path = directory.join(format!("{run_name}.lst"));
    let ext_path = directory.join(format!("{run_name}.ext"));
    let shk_path = directory.join(format!("{run_name}.shk"));
    let cor_path = directory.join(format!("{run_name}.cor"));

    let mut model = Model::parse(&fs::read_to_string(model_path)?)?;
    if let Some(c) = comment_type {
        model.parse_comments(c);
    }

    let mut parameter_names = HashMap::new();

    // Add THETA parameter names
    for (i, param) in model.theta_parameters.iter().enumerate() {
        parameter_names.insert(format!("THETA{}", i + 1), param.name());
    }

    // Add OMEGA parameter names using shared iterator (RowMajor to match EXT file order)
    for (ext_name, _eta_label, param) in model.iter_omega_parameters(ParameterOrdering::RowMajor) {
        parameter_names.insert(ext_name, param.name());
    }

    // Add SIGMA parameter names using shared iterator (RowMajor to match EXT file order)
    for (ext_name, _eps_label, param) in model.iter_sigma_parameters(ParameterOrdering::RowMajor) {
        parameter_names.insert(ext_name, param.name());
    }

    let lst_summary = parse_lst(&fs::read_to_string(&lst_path)?);
    let shk_data = if shk_path.exists() {
        ShkReader.parse_file(shk_path)?
    } else {
        Vec::new()
    };

    // Create ExtReader with configuration for both parameters and minimization data
    let ext_reader = ExtReader::default()
        .final_estimates_and_stderr_and_fixed() // for parameters
        .with_condition_number() // for minimization metadata
        .with_termination_codes() // for minimization metadata
        .keep_all_tables();

    let estimation_results =
        get_estimation_results(&ext_path, &ext_reader, Some(shk_data), hide_off_diagonals)?;

    if estimation_results.is_empty() {
        bail!("Could not find any tables in {} file", ext_path.display());
    }

    // Extract minimization results from ALL methods
    let minimization_results: Vec<MinimizationResults> = estimation_results
        .iter()
        .map(|r| r.minimization_results.clone())
        .collect();

    // Extract parameters from LAST method only
    let last_result = estimation_results.last().unwrap();
    let mut last_table = last_result.parameters.clone();

    for param in last_table.theta.iter_mut() {
        if let Some(Some(n)) = parameter_names.get(&param.name) {
            param.name = n.to_string();
        }
    }
    for param in last_table.random_effects.iter_mut() {
        if let Some(Some(n)) = parameter_names.get(&param.name) {
            param.name = n.to_string();
        }
    }

    let cor_reader = CorReader::default().keep_all_tables();
    let correlation_matrix = cor_reader.parse_file(cor_path)?.pop().unwrap();

    Ok(Summary {
        run_name: run_name.to_string(),
        lst: lst_summary,
        minimization_results,
        parameters: last_table,
        parameter_names,
        correlation_matrix,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::CommentType;
    use insta::{assert_debug_snapshot, glob};

    #[test]
    fn test_parameter_comment_alignment() {
        glob!("../../test_data/run_output", "**/*.mod", |mod_path| {
            let run_directory = mod_path.parent().unwrap();
            let run_name = run_directory.file_name().unwrap().to_string_lossy();

            // Test with Type1 comments to verify comment alignment
            let summary = get_summary(run_directory, Some(CommentType::Type1), false).unwrap();

            // Verify key parameter mappings work correctly
            let key_mappings: Vec<(String, Option<String>)> = summary
                .parameter_names
                .iter()
                .filter(|(name, _)| {
                    name.starts_with("OMEGA(")
                        || name.starts_with("SIGMA(")
                        || name.starts_with("THETA")
                })
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            // Sort for deterministic comparison
            let mut sorted_mappings = key_mappings;
            sorted_mappings.sort_by(|a, b| a.0.cmp(&b.0));

            // Snapshot name: run_parameter_mappings
            let snapshot_name = format!("{}_parameter_mappings", run_name);
            assert_debug_snapshot!(snapshot_name, sorted_mappings);
        });
    }
}
