use extendr_api::prelude::*;
use fs_err as fs;
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
    let model_path = find_output_file(search_path, "mod")?;
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

    // Convert BTreeMap to named character vector
    let keys: Vec<String> = parameter_names.keys().cloned().collect();
    let values: Vec<String> = parameter_names
        .values()
        .map(|opt_name| opt_name.clone().unwrap_or(String::new()))
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
