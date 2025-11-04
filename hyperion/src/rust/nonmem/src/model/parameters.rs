use extendr_api::prelude::*;
use fs_err as fs;
use std::ffi::OsStr;
use std::path::Path;

use crate::{
    OMEGA, ParameterRow, ParameterRowBuilder, ParameterTable, SIGMA, THETA, find_output_file,
    get_comment_type,
};
use crate::model::robj_to_model;

use nonmem::Model;
use nonmem::output_files::ext::get_parameter_estimates;
use nonmem::output_files::get_parameter_names;
use crate::output_files::ext::create_ext_reader;
use nonmem::output_files::shk::ShkReader;

/// Gets parameter estimates from model run
///
/// @param path path to model file, model output directory, ext file or metadata json file.
/// @param hide_off_diagonal_params boolean, if TRUE will not display the unfixed off-diagonal
/// estimated parameters
/// @param only_method character, filter for getting estimates from specified method only.
/// Available methods are Fo, Foce, Saems, Bayes, Imp, ImpMap, Its, Nuts
/// @param only_last boolean, for grabbing only last estimation method parameters
/// @param columns character vector of columns to include in resulting dataframe. Default:c("kind", "name", "random_effect", "value", "stderr", "rse", "shrinkage", "fixed", "diagonal")
/// /// Available columns: "kind", "name", "random_effect", "value", "stderr", "rse", "shrinkage", "fixed", "diagonal", "table_idx", "method"
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
    #[default = "FALSE"] hide_off_diagonal_params: bool,
    #[default = "NULL"] only_method: Option<&str>,
    #[default = "TRUE"] only_last: Option<bool>,
    #[default = r#"c("kind", "name", "random_effect", "value", "stderr", "rse", "shrinkage", "fixed", "diagonal")"#]
    columns: Vec<String>,
) -> Result<Robj> {
    let ext_reader = create_ext_reader(None, None, only_method, only_last)?;

    let search_path = if Path::new(path).extension() == Some(OsStr::new("ext")) {
        Path::new(path).parent().unwrap().to_str().unwrap()
    } else {
        path
    };

    let shk_data = match find_output_file(search_path, "shk") {
        Ok(p) => match ShkReader::default().parse_file(p) {
            Ok(s) => s,
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    };

    let ext_path = find_output_file(path, "ext")?;
    let model_path = find_output_file(search_path, "mod")?;
    let content = fs::read_to_string(&model_path).map_err(|e| Error::Other(format!("{e}")))?;

    let mut model = Model::parse(&content)
        .map_err(|e| Error::Other(format!("Failed to read model file: {e}")))?;

    let comment_type = get_comment_type();
    let parameter_names = get_parameter_names(&mut model, comment_type);

    let tables = get_parameter_estimates(
        ext_path,
        &ext_reader,
        Some(shk_data),
        hide_off_diagonal_params,
        Some(&parameter_names),
    )
    .map_err(|e| Error::Other(e.to_string()))?;

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

    ParameterTable::new(rows, columns).build_df()
}

/// Gets parameter names from model for display purposes
///
/// @param model hyperion_nonmem_model object from read_model()
/// @return Named character vector with NONMEM names as names and user-friendly names as values
/// @keywords internal
/// @noRd
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
    let parameter_names = get_parameter_names(&mut model, comment_type);

    // Convert BTreeMap to named character vector
    let keys: Vec<String> = parameter_names.keys().cloned().collect();
    let values: Vec<String> = parameter_names.values()
        .map(|opt_name| opt_name.as_ref().unwrap_or(&String::new()).clone())
        .collect();

    // Create named character vector
    let result = List::from_names_and_values(keys, values)
        .into_robj();

    Ok(result)
}

extendr_module! {
    mod parameters;

    fn get_parameters;
    fn get_model_parameter_names;
}
