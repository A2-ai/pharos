use extendr_api::prelude::*;
use fs_err as fs;
use std::cmp::Ordering;
use std::ffi::OsStr;
use std::path::Path;

// pharos nonmem crate
use nonmem::{
    Model,
    output_files::{ext::get_parameter_estimates, shk::ShkReader},
};

use crate::{
    model::robj_to_model,
    output_files::ext::create_ext_reader,
    output_files::{OMEGA, ParameterRow, ParameterRowBuilder, SIGMA, THETA, build_parameters_df},
    utils::{find_output_file, get_comment_type},
};
use hyperion_core::ResultExt;

/// Extract numeric indices from a parameter name for sorting.
///
/// Handles formats like:
/// - "THETA1", "THETA10" -> (1, 0, 0) or (10, 0, 0)
/// - "OMEGA(1,1)", "OMEGA(10,10)" -> (1, 1, 0) or (10, 10, 0)
/// - "SIGMA(1,1)", "SIGMA(2,2)" -> (1, 1, 0) or (2, 2, 0)
///
/// Returns a tuple of (first_num, second_num, param_type_order) for sorting.
fn extract_param_sort_key(name: &str) -> (u32, u32, u8) {
    // Determine parameter type order: THETA=0, OMEGA=1, SIGMA=2
    let type_order = if name.starts_with("THETA") {
        0
    } else if name.starts_with("OMEGA") {
        1
    } else if name.starts_with("SIGMA") {
        2
    } else {
        3
    };

    // Try to extract numbers from THETA format (e.g., "THETA1", "THETA10")
    if name.starts_with("THETA") {
        if let Ok(num) = name[5..].parse::<u32>() {
            return (num, 0, type_order);
        }
    }

    // Try to extract numbers from matrix format (e.g., "OMEGA(1,1)", "SIGMA(10,10)")
    if let Some(start) = name.find('(') {
        if let Some(end) = name.find(')') {
            let inner = &name[start + 1..end];
            let parts: Vec<&str> = inner.split(',').collect();
            if parts.len() == 2 {
                if let (Ok(row), Ok(col)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                    return (row, col, type_order);
                }
            }
        }
    }

    // Fallback: return high values to sort unknown formats to the end
    (u32::MAX, u32::MAX, type_order)
}

/// Compare two parameter names for numeric sorting.
fn compare_param_names(a: &str, b: &str) -> Ordering {
    let key_a = extract_param_sort_key(a);
    let key_b = extract_param_sort_key(b);

    // First sort by parameter type (THETA, OMEGA, SIGMA)
    match key_a.2.cmp(&key_b.2) {
        Ordering::Equal => {}
        other => return other,
    }

    // Then sort by first number (row for matrices, index for THETA)
    match key_a.0.cmp(&key_b.0) {
        Ordering::Equal => {}
        other => return other,
    }

    // Then sort by second number (column for matrices)
    key_a.1.cmp(&key_b.1)
}

/// Gets parameter estimates from model run
///
/// @param path path to model file, model output directory, ext file or metadata json file.
/// @param hide_off_diagonal_params boolean, if TRUE will not display the unfixed off-diagonal
/// estimated parameters
/// @param only_method character, filter for getting estimates from specified method only.
/// Available methods are Fo, Foce, Saems, Bayes, Imp, ImpMap, Its, Nuts
/// @param only_last boolean, for grabbing only last estimation method parameters
/// @param show_table_idx boolean, if TRUE include table_idx column in output
/// @param show_method boolean, if TRUE include method column in output
///
/// @return data.frame of parameter estimates
/// @export
///
/// @examples \dontrun{
/// get_parameters("model/nonmem/run001/run001.ext")
/// }
#[extendr]
pub fn get_parameters(
    path: &str,
    #[extendr(default = "FALSE")] hide_off_diagonal_params: bool,
    #[extendr(default = "NULL")] only_method: Option<&str>,
    #[extendr(default = "TRUE")] only_last: Option<bool>,
    #[extendr(default = "FALSE")] show_table_idx: bool,
    #[extendr(default = "FALSE")] show_method: bool,
) -> Result<Robj> {
    let ext_reader = create_ext_reader(None, None, only_method, only_last)?;

    let search_path = if Path::new(path).extension() == Some(OsStr::new("ext")) {
        Path::new(path).parent().unwrap().to_str().unwrap()
    } else {
        path
    };

    let shk_data = match find_output_file(search_path, "shk") {
        Ok(p) => ShkReader.parse_file(p).unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    let ext_path = find_output_file(path, "ext")?;
    let model_path =
        find_output_file(search_path, "mod").or_else(|_| find_output_file(path, "ctl"))?;
    let content = fs::read_to_string(&model_path).map_to_extendr_err("")?;

    let mut model = Model::parse(&content).map_to_extendr_err("Failed to read model file")?;

    let comment_type = get_comment_type();
    let parameter_names = model
        .get_parameter_names(comment_type)
        .map_to_extendr_err("Failed to get model parameter names")?;

    let tables = get_parameter_estimates(
        ext_path,
        &ext_reader,
        Some(shk_data),
        hide_off_diagonal_params,
        Some(&parameter_names),
    )
    .map_to_extendr_err("")?;

    // Build rows using the builder pattern
    let rows: Vec<ParameterRow> = tables
        .iter()
        .enumerate()
        .flat_map(|(i, tp)| {
            let table_idx = (i as i32) + 1;
            let method = tp
                .method
                .as_ref()
                .map(|m| m.to_string())
                .unwrap_or_default();

            // Collect parameters from theta, omega, and sigma
            let mut all_params = Vec::new();

            // Add theta parameters
            all_params.extend(tp.theta.iter().map(|p| {
                ParameterRowBuilder::new(THETA, p.name.clone(), p.estimate)
                    .with_stderr_rse(p.stderr, p.rse, p.fixed)
                    .with_table_idx(table_idx)
                    .with_method(method.clone())
                    .build()
            }));

            // Add omega parameters
            all_params.extend(tp.random_effects.iter().filter(|r| r.is_omega()).map(|p| {
                ParameterRowBuilder::new(OMEGA, p.name.clone(), p.estimate)
                    .with_stderr_rse(p.stderr, p.rse, p.fixed)
                    .with_sd(p.sd)
                    .with_corr(p.corr)
                    .with_shrinkage(p.shrinkage, p.fixed)
                    .with_random_effect(p.random_effect.clone())
                    .with_diagonal(p.diagonal)
                    .with_table_idx(table_idx)
                    .with_method(method.clone())
                    .build()
            }));
            // Add sigma parameters
            all_params.extend(tp.random_effects.iter().filter(|r| r.is_sigma()).map(|p| {
                ParameterRowBuilder::new(SIGMA, p.name.clone(), p.estimate)
                    .with_stderr_rse(p.stderr, p.rse, p.fixed)
                    .with_sd(p.sd)
                    .with_corr(p.corr)
                    .with_shrinkage(p.shrinkage, p.fixed)
                    .with_random_effect(p.random_effect.clone())
                    .with_diagonal(p.diagonal)
                    .with_table_idx(table_idx)
                    .with_method(method.clone())
                    .build()
            }));

            all_params.into_iter()
        })
        .collect();

    build_parameters_df(rows, show_table_idx, show_method)
}

/// Gets parameter names from model for display purposes
///
/// @param model hyperion_nonmem_model object from read_model()
///
/// @return Named character vector with NONMEM names as names and user-friendly names as values
/// @export
///
/// @examples \dontrun{
/// model <- read_model("run001.mod")
/// param_names <- get_model_parameter_names(model)
/// omega_names <- param_names[grepl("^OMEGA", names(param_names))]
/// }
#[extendr]
pub fn get_model_parameter_names(model: Robj) -> Result<Robj> {
    let mut model = robj_to_model(&model)?;

    let comment_type = get_comment_type();
    let parameter_names = model
        .get_parameter_names(comment_type)
        .map_to_extendr_err("Failed to get model parameter names")?;

    // Convert BTreeMap to named character vector, sorting keys numerically
    // BTreeMap sorts keys alphabetically, but we need numeric order
    // (e.g., OMEGA(1,1), OMEGA(2,2), ..., OMEGA(10,10) instead of
    //  OMEGA(1,1), OMEGA(10,10), OMEGA(2,2), ...)
    let mut keys: Vec<String> = parameter_names.keys().cloned().collect();
    keys.sort_by(|a, b| compare_param_names(a, b));

    // Collect values in the same sorted order
    let values: Vec<String> = keys
        .iter()
        .map(|k| {
            parameter_names
                .get(k)
                .and_then(|v| v.clone())
                .unwrap_or_default()
        })
        .collect();

    // Create named character vector
    let result = List::from_names_and_values(keys, values).into_robj();

    Ok(result)
}

extendr_module! {
    mod parameters;

    fn get_parameters;
    fn get_model_parameter_names;
}
