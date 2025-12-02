use extendr_api::prelude::*;
use fs_err as fs;
use std::path::Path;

// Pharos nonmem crate
use nonmem::output_files::{
    cor::CorrelationMatrix,
    ext::{MinimizationResults, TableParameters},
    get_summary,
    lst::{RunDetails, RunHeuristics, parse_lst},
};

use crate::{
    output_files::{OMEGA, ParameterRowBuilder, ParameterTable, SIGMA, THETA},
    utils::{find_output_file, get_comment_type},
};
use hyperion_core::{ResultExt, extendr_err};

#[derive(Debug, IntoDataFrameRow)]
pub struct MinimizationResultsRow {
    pub ofv: Rfloat,
    pub condition_number: Rfloat,
    pub termination_status: Rint,
}

/// Row for RunDetails - one row per estimation method
#[derive(Debug, IntoDataFrameRow)]
pub struct RunDetailsRow {
    pub problem: String,
    pub number_data_records: i32,
    pub number_subjects: i32,
    pub number_obs: i32,
    pub postprocess_time: f64,
    pub function_evaluations: i32,
    pub significant_digits: i32,
    pub only_sim: bool,
    pub estimation_method: String,
    pub estimation_time: f64,
    pub covariance_time: f64,
}

/// Row for RunHeuristics - tidy format with one row per heuristic
#[derive(Debug, IntoDataFrameRow)]
pub struct RunHeuristicsRow {
    pub heuristic_name: String,
    pub value: bool,
}

/// Row for CorrelationMatrix - tidy format with one row per parameter pair
#[derive(Debug, IntoDataFrameRow)]
pub struct CorrelationMatrixRow {
    pub param1: String,
    pub param2: String,
    pub correlation: Rfloat,
    pub method: String,
}

pub fn build_run_minimization_results_df(minimizations: &[MinimizationResults]) -> Result<Robj> {
    let rows: Vec<MinimizationResultsRow> = minimizations
        .iter()
        .map(|min_result| MinimizationResultsRow {
            ofv: min_result.ofv.map_or(Rfloat::na(), Rfloat::from),
            condition_number: min_result
                .condition_number
                .map_or(Rfloat::na(), Rfloat::from),
            termination_status: min_result.termination_code.map_or(Rint::na(), Rint::from),
        })
        .collect();

    let df = rows
        .into_dataframe()
        .map_to_extendr_err("Failed to build minimization results df")?;

    Ok(df.into_robj())
}

/// Parse parameter name into (type_order, numeric_parts) for custom sorting
/// THETA < SIGMA < OMEGA ordering with numeric sorting within each type
fn parse_parameter_for_ordering(param: &str) -> (u8, Vec<u32>) {
    if let Some(p) = param.strip_prefix("THETA") {
        // THETA1 -> (0, [1])
        let num = p.parse().unwrap_or(0);
        (0, vec![num])
    } else if param.starts_with("SIGMA(") {
        // SIGMA(1,1) -> (1, [1, 1])
        let nums = parse_matrix_indices(param);
        (1, nums)
    } else if param.starts_with("OMEGA(") {
        // OMEGA(1,1) -> (2, [1, 1])
        let nums = parse_matrix_indices(param);
        (2, nums)
    } else {
        // Unknown parameter type
        (255, vec![])
    }
}

/// Extract numeric indices from matrix parameter names like "SIGMA(1,1)" or "OMEGA(2,1)"
fn parse_matrix_indices(param: &str) -> Vec<u32> {
    if let Some(start) = param.find('(')
        && let Some(end) = param.find(')')
    {
        let indices_str = &param[start + 1..end];
        return indices_str
            .split(',')
            .map(|s| s.trim().parse().unwrap_or(0))
            .collect();
    }
    vec![]
}

/// Custom comparison function for parameter ordering: THETA < SIGMA < OMEGA
fn compare_parameters(a: &str, b: &str) -> std::cmp::Ordering {
    let (a_type, a_nums) = parse_parameter_for_ordering(a);
    let (b_type, b_nums) = parse_parameter_for_ordering(b);

    // First compare by type (THETA < SIGMA < OMEGA)
    match a_type.cmp(&b_type) {
        std::cmp::Ordering::Equal => {
            // Same type, compare by numeric parts
            a_nums.cmp(&b_nums)
        }
        other => other,
    }
}

pub fn build_correlation_matrix_df(correlations: CorrelationMatrix) -> Result<Robj> {
    let method_string = correlations
        .method
        .as_ref()
        .map(|m| m.to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let mut rows: Vec<CorrelationMatrixRow> = correlations
        .correlations
        .into_iter()
        .map(|ce| CorrelationMatrixRow {
            param1: ce.param1,
            param2: ce.param2,
            correlation: Rfloat::from(ce.value),
            method: method_string.clone(),
        })
        .collect();

    // Sort by custom parameter ordering (param1 first, then param2)
    rows.sort_by(|a, b| {
        match compare_parameters(&a.param1, &b.param1) {
            std::cmp::Ordering::Equal => {
                // If param1 is the same, sort by param2
                compare_parameters(&a.param2, &b.param2)
            }
            other => other,
        }
    });

    let df = rows
        .into_dataframe()
        .map_to_extendr_err("Failed to build correlation matrix df")?;

    Ok(df.into_robj())
}

/// Convert RunDetails to dataframe with one row per estimation method
pub fn build_run_details_df(details: RunDetails) -> Result<Robj> {
    let rows: Vec<RunDetailsRow> = details
        .estimation_methods
        .into_iter()
        .enumerate()
        .map(|(i, method)| RunDetailsRow {
            problem: details.problem.clone(),
            number_data_records: details.number_data_records as i32,
            number_subjects: details.number_subjects as i32,
            number_obs: details.number_obs as i32,
            postprocess_time: details.postprocess_time,
            function_evaluations: details.function_evaluations as i32,
            significant_digits: details.significant_digits as i32,
            only_sim: details.only_sim,
            estimation_method: method,
            estimation_time: details.estimation_time.get(i).copied().unwrap_or(0.0),
            covariance_time: details.covariance_time.get(i).copied().unwrap_or(0.0),
        })
        .collect();

    let df = rows
        .into_dataframe()
        .map_to_extendr_err("Failed to build run_details dataframe")?;

    Ok(df.into_robj())
}

/// Convert RunHeuristics to a tidy dataframe
pub fn build_run_heuristics_df(heuristics: &RunHeuristics) -> Result<Robj> {
    let rows = vec![
        RunHeuristicsRow {
            heuristic_name: "covariance_step_aborted".to_string(),
            value: heuristics.covariance_step_aborted.unwrap_or(false),
        },
        RunHeuristicsRow {
            heuristic_name: "eigenvalue_issues".to_string(),
            value: heuristics.eigenvalue_issues.unwrap_or(false),
        },
        RunHeuristicsRow {
            heuristic_name: "parameter_near_boundary".to_string(),
            value: heuristics.parameter_near_boundary.unwrap_or(false),
        },
        RunHeuristicsRow {
            heuristic_name: "hessian_reset".to_string(),
            value: heuristics.hessian_reset.unwrap_or(false),
        },
        RunHeuristicsRow {
            heuristic_name: "minimization_terminated".to_string(),
            value: heuristics.minimization_terminated.unwrap_or(false),
        },
    ];

    let df = rows
        .into_dataframe()
        .map_to_extendr_err("Failed to build run_heuristics dataframe")?;

    Ok(df.into_robj())
}

/// Build parameters dataframe from summary parameters
pub fn build_parameters_df(parameters: TableParameters, columns: Vec<String>) -> Result<Robj> {
    let thetas = parameters.theta;
    let (omegas, sigmas): (Vec<_>, Vec<_>) = parameters
        .random_effects
        .into_iter()
        .partition(|re| re.is_omega());

    let mut parameter_rows = Vec::with_capacity(thetas.len() + omegas.len() + sigmas.len());

    // Add theta parameters
    parameter_rows.extend(thetas.into_iter().map(|p| {
        ParameterRowBuilder::new(THETA, p.name, p.estimate)
            .with_stderr_rse(p.stderr, p.rse, p.fixed)
            .build()
    }));

    // Add omega parameters (use ETA name)
    parameter_rows.extend(omegas.into_iter().map(|p| {
        ParameterRowBuilder::new(OMEGA, p.name, p.estimate)
            .with_stderr_rse(p.stderr, p.rse, p.fixed)
            .with_shrinkage(p.shrinkage, p.fixed)
            .with_random_effect(p.random_effect)
            .build()
    }));

    // Add sigma parameters (use EPS name)
    parameter_rows.extend(sigmas.into_iter().map(|p| {
        ParameterRowBuilder::new(SIGMA, p.name, p.estimate)
            .with_stderr_rse(p.stderr, p.rse, p.fixed)
            .with_shrinkage(p.shrinkage, p.fixed)
            .with_random_effect(p.random_effect)
            .build()
    }));

    // Build dataframe
    let parameters_df = ParameterTable::new(parameter_rows, columns)
        .build_df()
        .map_to_extendr_err("Failed to build parameters")?;

    Ok(parameters_df)
}

/// Gets model run summary
///
/// @param directory path to model run output directory containing .ext, .lst files
/// @param hide_off_diagonal_params boolean, if TRUE will not display the unfixed off-diagonal
/// estimated parameters
/// @param columns character vector of columns to include in resulting dataframe. Default: c("name", "value", "stderr", "rse", "shrinkage", "kind").
/// Available columns: "kind", "name", "value", "stderr", "rse", "shrinkage", "fixed", "table_idx", "method", random_effect
///
/// @return hyperion_nonmem_summary S3 object
/// @export
///
/// @examples \dontrun{
/// get_model_summary("model/nonmem/run001")
/// }
#[extendr]
pub fn get_model_summary(
    directory: &str,
    #[default = "FALSE"] hide_off_diagonal_params: bool,
    #[default = r#"c("name", "random_effect", "value", "stderr", "rse", "shrinkage", "kind")"#]
    columns: Vec<String>,
) -> Result<Robj> {
    // Load config and extract comment type
    let comment_type = get_comment_type();

    if Path::new(&directory).is_file() {
        return Err(extendr_err!(
            "Please input path to model run output directory."
        ));
    };

    let summary = get_summary(directory, comment_type, hide_off_diagonal_params)
        .map_to_extendr_err("Failed to get summary")?;

    let run_details_df = build_run_details_df(summary.lst.run_details)?;
    let run_heuristics_df = build_run_heuristics_df(&summary.lst.run_heuristics)?;
    let parameters_df = build_parameters_df(summary.parameters, columns)?;
    let run_minimization_results_df =
        build_run_minimization_results_df(&summary.minimization_results)?;

    // for None correlation_matrix Robj::from(()) gives NULL
    let correlation_matrix_df = match summary.correlation_matrix {
        Some(cm) => build_correlation_matrix_df(cm)?,
        None => Robj::from(()),
    };

    // Return as named list
    let mut result = list!(
        run_name = summary.run_name,
        run_details = run_details_df,
        run_heuristics = run_heuristics_df,
        minimization_results = run_minimization_results_df,
        parameters = parameters_df,
        correlation_matrix = correlation_matrix_df
    )
    .into_robj();

    let result = result
        .set_class(["hyperion_nonmem_summary"])
        .map_to_extendr_err("Failed to set class")?;

    Ok(result.to_owned())
}

/// Parses lst file for run details and heuristics
///
/// @param path path to model file, model output directory, lst file or metadata json file.
///
/// @return list of data.frames of run details and run heuristics
/// @export
///
/// @examples \dontrun{
/// get_run_info("model/nonmem/run001/run001.lst")
/// }
#[extendr]
pub fn get_run_info(path: &str) -> Result<Robj> {
    let path = find_output_file(path, "lst")?;

    let content = fs::read_to_string(path).map_to_extendr_err("")?;
    let summary = parse_lst(&content);

    let run_details_df = build_run_details_df(summary.run_details)
        .map_to_extendr_err("Failed to build run details")?;

    let run_heuristics_df = build_run_heuristics_df(&summary.run_heuristics)
        .map_to_extendr_err("Failed to build heuristics details")?;

    let result = list!(
        run_details = run_details_df,
        run_heuristics = run_heuristics_df
    );

    Ok(result.into_robj())
}

extendr_module! {
    mod summary;
    fn get_model_summary;
    fn get_run_info;
}
