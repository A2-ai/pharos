use std::collections::HashMap;
use std::path::Path;

use crate::Model;
use crate::output_files::ext::{
    ExtReader, MinimizationResults, TableParameters, get_estimation_results,
};
use crate::output_files::lst::{LstSummary, parse_lst};
use crate::output_files::shk::ShkReader;
use crate::parsing::BlockStructure;
use anyhow::{Result, bail};
use config::CommentType;
use fs_err as fs;
use serde::{Deserialize, Serialize};

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

    let mut model = Model::parse(&fs::read_to_string(model_path)?)?;
    if let Some(c) = comment_type {
        model.parse_comments(c);
    }

    let mut parameter_names = HashMap::new();
    for (i, param) in model.theta_parameters.iter().enumerate() {
        parameter_names.insert(format!("THETA{}", i + 1), param.name());
    }
    let mut num_omega = 1;
    for block in model.omega_blocks {
        if block.structure != BlockStructure::Diagonal {
            continue;
        }
        for param in block.parameters {
            parameter_names.insert(format!("OMEGA({num_omega},{num_omega})"), param.name());
            num_omega += 1;
        }
    }
    let mut num_sigma = 1;
    for block in model.sigma_blocks {
        if block.structure != BlockStructure::Diagonal {
            continue;
        }
        for param in block.parameters {
            parameter_names.insert(format!("SIGMA({num_sigma},{num_sigma})"), param.name());
            num_sigma += 1;
        }
    }

    let lst_summary = parse_lst(&fs::read_to_string(&lst_path)?);
    let shk_data = if shk_path.exists() {
        ShkReader.parse_file(shk_path)?
    } else {
        Vec::new()
    };
    let parameters =
        get_parameter_estimates(&ext_path, &ext_tables, Some(shk_data), hide_off_diagonals)?;

    // Create ExtReader with configuration for both parameters and minimization data
    let ext_reader = ExtReader::default()
        .final_estimates_and_stderr_and_fixed() // for parameters
        .with_condition_number() // for minimization metadata
        .with_termination_codes() // for minimization metadata
        .keep_all_tables();

    let estimation_results = get_estimation_results(&ext_path, &ext_reader, Some(shk_data))?;

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

    Ok(Summary {
        run_name: run_name.to_string(),
        lst: lst_summary,
        minimization_results,
        parameters: last_table,
        parameter_names,
    })
}
