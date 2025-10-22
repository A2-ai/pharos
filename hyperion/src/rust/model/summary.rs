use crate::output_files::{OMEGA, ParameterRowBuilder, ParameterTable, SIGMA, THETA};
use crate::utils::find_output_file;
use config::CommentType;
use extendr_api::prelude::*;
use fs_err as fs;
use nonmem::output_files::ext::MinimizationResults;
use std::path::Path;
use nonmem::output_files::get_summary;
use nonmem::output_files::lst::parse_lst;
use nonmem::output_files::lst::{RunDetails, RunHeuristics};

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

pub fn build_run_minimization_results_df(minimizations: &Vec<MinimizationResults>) -> Result<Robj> {
    let rows: Vec<MinimizationResultsRow> = minimizations
        .iter()
        .map(|min_result| MinimizationResultsRow {
            ofv: min_result.ofv.map_or(Rfloat::na(), Rfloat::from),
            condition_number: min_result.condition_number.map_or(Rfloat::na(), Rfloat::from),
            termination_status: min_result.termination_code.map_or(Rint::na(), Rint::from),
        })
        .collect();

    let df = rows
        .into_dataframe()
        .map_err(|e| Error::Other(format!("Failed to build minimization results df: {e}")))?;

    Ok(df.into_robj())
}

/// Convert RunDetails to dataframe with one row per estimation method
pub fn build_run_details_df(details: &RunDetails) -> Result<Robj> {
    let rows: Vec<RunDetailsRow> = details
        .estimation_methods
        .iter()
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
            estimation_method: method.clone(),
            estimation_time: details.estimation_time.get(i).copied().unwrap_or(0.0),
            covariance_time: details.covariance_time.get(i).copied().unwrap_or(0.0),
        })
        .collect();

    let df = rows
        .into_dataframe()
        .map_err(|e| Error::Other(format!("Failed to build run_details dataframe: {e}")))?;

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
        .map_err(|e| Error::Other(format!("Failed to build run_heuristics dataframe: {e}")))?;

    Ok(df.into_robj())
}

/// Gets model run summary
///
/// @param directory path to model run output directory containing .ext, .lst files
/// @param comment_type string of control stream comments types. Type1 or NULL
/// @param columns character vector of columns to include in resulting dataframe. Default: c("name", "value", "stderr", "rse", "shrinkage", "kind").
/// Available columns: "kind", "name", "value", "stderr", "rse", "shrinkage", "fixed", "table_idx", "method"
///
/// @return list of data.frames of run details, run heuristics, and parameter estimates
/// @export
///
/// @examples \dontrun{
/// get_model_summary("model/nonmem/run001")
/// }
#[extendr]
pub fn get_model_summary(
    directory: &str,
    #[default = "FALSE"] hide_off_diagonal_params: bool,
    #[default = "NULL"] comment_type: Option<String>,
    #[default = r#"c("name", "value", "stderr", "rse", "shrinkage", "kind")"#] columns: Vec<String>,
) -> Result<Robj> {
    // need to think about comment_type from config file?
    let comment_type: Option<CommentType> =
        comment_type.and_then(|s| match s.trim().to_uppercase().as_ref() {
            "TYPE1" => Some(CommentType::Type1),
            _ => None,
        });
    
    if Path::new(&directory).is_file() {
        return Err(Error::Other("Please input path to model run output directory.".to_string()))
    };

    let summary = get_summary(directory, comment_type, hide_off_diagonal_params)
        .map_err(|e| Error::Other(format!("Failed to get summary: {e}")))?;

    let run_details_df = build_run_details_df(&summary.lst.run_details)?;
    let run_heuristics_df = build_run_heuristics_df(&summary.lst.run_heuristics)?;
    let run_minimization_results_df = build_run_minimization_results_df(&summary.minimization_results)?;

    // Build parameter rows using the builder pattern (no optional columns for summary)
    let mut parameter_rows = Vec::new();

    // Add theta parameters
    parameter_rows.extend(summary.parameters.theta.iter().map(|p| {
        ParameterRowBuilder::new(THETA, p.name.clone(), p.estimate)
            .with_stderr_rse(p.stderr, p.rse, p.fixed)
            .build()
    }));

    // Add omega parameters (use ETA name)
    parameter_rows.extend(summary.parameters.random_effects.iter().filter(|r| r.is_omega()).map(|p| {
        ParameterRowBuilder::new(OMEGA, p.name.clone(), p.estimate)
            .with_stderr_rse(p.stderr, p.rse, p.fixed)
            .with_shrinkage(p.shrinkage, p.fixed)
            .build()
    }));
    // Add sigma parameters (use EPS name)
    parameter_rows.extend(summary.parameters.random_effects.iter().filter(|r| r.is_sigma()).map(|p| {
        ParameterRowBuilder::new(SIGMA, p.name.clone(), p.estimate)
            .with_stderr_rse(p.stderr, p.rse, p.fixed)
            .with_shrinkage(p.shrinkage, p.fixed)
            .build()
    }));

    // For summary: name, value, stderr, rse, shrinkage
    let parameters_df = ParameterTable::new(parameter_rows, columns)
        .build_df()
        .map_err(|e| Error::Other(format!("Failed to build parameters: {e}")))?;

    // Return as named list
    let mut result = list!(
        run_name = summary.run_name,
        run_details = run_details_df,
        run_heuristics = run_heuristics_df,
        minimization_results = run_minimization_results_df,
        parameters = parameters_df
    )
    .into_robj();

    let result = result
        .set_class(["hyperion_summary"])
        .map_err(|e| Error::Other(format!("Failed to set class: {e}")))?;

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

    let content = fs::read_to_string(path).map_err(|e| Error::Other(format!("{e}")))?;
    let summary = parse_lst(&content);

    let run_details_df = build_run_details_df(&summary.run_details)
        .map_err(|e| Error::Other(format!("Failed to build run details: {e}")))?;

    let run_heuristics_df = build_run_heuristics_df(&summary.run_heuristics)
        .map_err(|e| Error::Other(format!("Failed to build heuristics details: {e}")))?;

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
