use std::cmp::max;
use std::collections::BTreeMap;
use std::path::Path;

use crate::output_files::cor::{CorReader, CorrelationMatrix};
use crate::output_files::ext::{
    ExtReader, MinimizationResults, ParameterType, TableParameters, get_estimation_results,
};
use crate::output_files::lst::{LstSummary, parse_lst};
use crate::output_files::shk::ShkReader;
use crate::{Model, TERMINATION_FILENAME, Termination};
use anyhow::{Result, anyhow, bail};
use config::CommentType;
use fs_err as fs;
use serde::{Deserialize, Serialize};

pub mod cor;
pub mod ext;
pub mod grd;
pub mod lst;
mod parsing;
pub mod shk;

/// Can be a bit lossy but probably ok for display
fn count_significant_digits(num: f64) -> usize {
    let s = format!("{}", num);
    if let Some(pos) = s.find('.') {
        s.len() - pos - 1
    } else {
        0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Summary {
    pub run_name: String,
    pub lst: LstSummary,
    pub minimization_results: Vec<MinimizationResults>,
    pub parameters: TableParameters,
    pub parameter_names: BTreeMap<String, Option<String>>,
    pub correlation_matrix: Option<CorrelationMatrix>,
}

impl Summary {
    pub fn get_num_significant_digits(&self, param_type: ParameterType) -> usize {
        let mut significant_digits = 0;

        if param_type == ParameterType::Theta {
            for t in &self.parameters.theta {
                significant_digits = max(significant_digits, count_significant_digits(t.estimate))
            }
        }

        for r in &self.parameters.random_effects {
            if param_type == r.param_type {
                significant_digits = max(significant_digits, count_significant_digits(r.estimate))
            }
        }

        significant_digits
    }
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

    let terminated_path = directory.join(TERMINATION_FILENAME);
    if terminated_path.exists() {
        let termination: Termination = serde_json::from_reader(fs::File::open(terminated_path)?)?;
        bail!("Run did not finish.\n{termination}");
    }
    let run_name = directory.file_name().and_then(|n| n.to_str()).unwrap();
    let model_path = ["mod", "ctl"]
        .iter()
        .map(|ext| directory.join(format!("{run_name}.{ext}")))
        .find(|p| p.exists())
        .ok_or_else(|| anyhow!("Failed to find model file (either .mod or .ctl)"))?;

    let lst_path = directory.join(format!("{run_name}.lst"));
    let ext_path = directory.join(format!("{run_name}.ext"));
    let shk_path = directory.join(format!("{run_name}.shk"));
    let cor_path = directory.join(format!("{run_name}.cor"));

    let mut model = Model::parse(&fs::read_to_string(model_path)?)?;
    let parameter_names = model.get_parameter_names(comment_type)?;

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

    let estimation_results = get_estimation_results(
        &ext_path,
        &ext_reader,
        Some(shk_data),
        hide_off_diagonals,
        Some(&parameter_names),
    )?;

    if estimation_results.is_empty() {
        bail!("Could not find any tables in {} file", ext_path.display());
    }

    // Extract minimization results from ALL methods
    let minimization_results: Vec<MinimizationResults> = estimation_results
        .iter()
        .map(|r| r.minimization_results.clone())
        .collect();

    // Extract parameters from LAST method only
    let last_table = estimation_results.last().unwrap().parameters.clone();

    // .cor file is not guaranteed to exist.
    let correlation_matrix = if cor_path.exists() {
        let cor_reader = CorReader::default().keep_all_tables();
        cor_reader.parse_file(cor_path)?.pop()
    } else {
        None
    };

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
    fn test_summary_scenarios() {
        glob!("../../test_data/run_output", "**/*.mod", |mod_path| {
            let run_directory = mod_path.parent().unwrap();
            let run_name = run_directory.file_name().unwrap().to_string_lossy();

            let test_scenarios = vec![
                ("baseline_no_comments", (None, false)),
                ("type1_comments", (Some(CommentType::Type1), false)),
                ("hide_off_diagonals", (None, true)),
                (
                    "type1_comments_hide_off_diags",
                    (Some(CommentType::Type1), true),
                ),
            ];

            for (scenario_name, (comment_type, hide_off_diagonals)) in test_scenarios {
                let summary = get_summary(run_directory, comment_type, hide_off_diagonals).unwrap();

                let snapshot_name = format!("{run_name}_{scenario_name}");
                assert_debug_snapshot!(snapshot_name, summary);
            }
        });
    }
}
