use crate::utils::find_output_file;

use super::{OMEGA, ParameterRow, ParameterRowBuilder, ParameterTable, SIGMA, THETA};
use extendr_api::{Robj, prelude::*};
use nonmem::estimation;
use nonmem::output_files::ext::{EstimationTable, ExtReader, get_parameter_estimates};
use nonmem::output_files::shk::ShkReader;
//use rayon::prelude::*;
use std::path::Path;

fn create_ext_reader(
    line_prefixes: Option<Vec<String>>,
    parameters_only: Option<bool>,
    only_method: Option<&str>,
    only_last: Option<bool>,
) -> Result<ExtReader> {
    let mut reader = ExtReader::default();

    if let Some(prefixes) = line_prefixes {
        reader = reader.filter_by_prefix(prefixes);
    }

    let parameters_only = parameters_only.unwrap_or(false);
    if parameters_only {
        reader = reader.parameters_only();
    }

    // Add estimation method filter
    if let Some(method) = only_method {
        // Handle common aliases
        let normalized_method = match method.to_lowercase().as_str() {
            "importance" => "imp",
            "focei" => "foce",
            _ => method,
        };
        let m: estimation::EstimationMethod = normalized_method
            .parse()
            .map_err(|e: String| Error::Other(e))?;
        reader = reader.only_method(m);
    }

    let only_last = only_last.unwrap_or(true);
    // Take all tables or only last
    if !only_last {
        reader = reader.keep_all_tables();
    }
    Ok(reader)
}

/// Gets parameter estimates from model run
///
/// @param path path to model file, model output directory, ext file or metadata json file.
/// @param hide_off_diagonal_params boolean, if TRUE will not display the unfixed off-diagonal
/// estimated parameters
/// @param only_method character, filter for getting estimates from specified method only.
/// Available methods are Fo, Foce, Saems, Bayes, Imp, ImpMap, Its, Nuts
/// @param only_last boolean, for grabbing only last estimation method parameters
/// @param columns character vector of columns to include in resulting dataframe. Default: c("kind", "name", "value", "stderr", "fixed").
/// Available columns: "kind", "name", "value", "stderr", "rse", "shrinkage", "fixed", "table_idx", "method"
///
/// @return data.frame of parameter estimates
/// @export
///
/// @examples \dontrun{
/// get_parameter_estimates("model/nonmem/run001/run001.ext")
/// }
#[extendr(r_name = "get_parameter_estimates")]
pub fn get_parameter_estimates_wrap(
    path: &str,
    #[default = "FALSE"] hide_off_diagonal_params: bool,
    #[default = "NULL"] only_method: Option<&str>,
    #[default = "TRUE"] only_last: Option<bool>,
    #[default = r#"c("kind", "name", "value", "stderr", "shrinkage", "fixed")"#] columns: Vec<
        String,
    >,
) -> Result<Robj> {
    let ext_reader = create_ext_reader(None, None, only_method, only_last)?;

    let shk_data = match find_output_file(path, "shk") {
        Ok(p) => match ShkReader::default().parse_file(p) {
            Ok(s) => s,
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    };

    let path = find_output_file(path, "ext")?;

    let tables =
        get_parameter_estimates(path, &ext_reader, Some(shk_data), hide_off_diagonal_params)
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
                    .with_table_idx(table_idx)
                    .with_method(method.clone())
                    .build()
            }));

            all_params.into_iter()
        })
        .collect();

    ParameterTable::new(rows, columns).build_df()
}

/// Fix parameter values in the list: set fixed parameters and NaNs to NA for iteration -1000000001
fn fix_parameter_values(list: List, param_names: &[String]) -> Result<List> {
    // Get iteration column
    let iterations = list
        .dollar("iteration")?
        .as_integers()
        .ok_or_else(|| Error::Other("Failed to get iterations as integers".to_string()))?;

    // Find which parameters are fixed (iteration -1000000006 has value 1)
    let fixed_row_idx = iterations
        .iter()
        .position(|iter| iter.inner() == -1000000006);
    let estimates_row_idx = iterations
        .iter()
        .position(|iter| iter.inner() == -1000000001);

    if let (Some(fixed_idx), Some(est_idx)) = (fixed_row_idx, estimates_row_idx) {
        // Build new pairs with corrected values
        let mut new_pairs = Vec::new();

        // Keep iteration and method as-is
        new_pairs.push(("iteration", list.dollar("iteration")?));
        new_pairs.push(("method", list.dollar("method")?));

        for param_name in param_names {
            let param_col = list.dollar(param_name)?.as_real_vector().ok_or_else(|| {
                Error::Other(format!("Failed to get {} as real vector", param_name))
            })?;

            // Check if parameter is fixed
            let is_fixed = param_col.get(fixed_idx).is_some_and(|&val| val == 1.0);

            let mut new_col: Vec<Rfloat> = param_col.iter().map(|&val| Rfloat::from(val)).collect();

            if is_fixed {
                new_col[est_idx] = Rfloat::na();
            }

            // Also convert any NaN to NA
            for val in &mut new_col {
                if val.is_nan() {
                    *val = Rfloat::na();
                }
            }

            new_pairs.push((param_name.as_str(), new_col.into_robj()));
        }

        Ok(List::from_pairs(new_pairs))
    } else {
        Ok(list)
    }
}

/// Helper function to convert EstimationTable vector to R dataframe
fn estimation_tables_to_dataframe(tables: Vec<EstimationTable>) -> Result<Robj> {
    if tables.is_empty() {
        return Err(Error::Other("No tables found in ext file".to_string()));
    }

    // Get parameter names from the first table
    let param_names = tables[0].parameters.clone();

    let flat_data: Vec<(i32, String, Vec<f64>)> = tables
        .into_iter()
        .flat_map(|table| {
            let method_name = table.method.unwrap().to_string();
            table
                .rows
                .into_iter()
                .map(move |row| (row.iteration as i32, method_name.clone(), row.values))
        })
        .collect();

    // Extract columns
    let iterations: Vec<i32> = flat_data.iter().map(|(iter, _, _)| *iter).collect();
    let methods: Vec<String> = flat_data
        .iter()
        .map(|(_, method, _)| method.clone())
        .collect();

    // Build column pairs
    let mut pairs = vec![
        ("iteration", iterations.into_robj()),
        ("method", methods.into_robj()),
    ];

    // Add parameter columns dynamically
    for (param_idx, param_name) in param_names.iter().enumerate() {
        let values: Vec<f64> = flat_data
            .iter()
            .map(|(_, _, row_vals)| row_vals.get(param_idx).copied().unwrap_or(f64::NAN))
            .collect();
        pairs.push((param_name.as_str(), values.into_robj()));
    }

    let list = List::from_pairs(pairs);

    // Post-process: fix parameter values for fixed parameters and NaNs
    let fixed_list = fix_parameter_values(list, &param_names)?;

    let df = data_frame!(fixed_list);

    Ok(df)
}

/// Reads ext file
///
/// @param path path to model file, model output directory, ext file or metadata json file.
/// @param line_prefixes character vector for lines to filter for
/// @param parameters_only bool if true removes ITERATION and OBJ column, default false
/// @param only_method character, filter for getting estimates from specified method only
/// @param only_last boolean, for grabbing only last estimation method parameters
///
/// @return data.frame of ext file
/// @export
///
/// @examples \dontrun{
/// read_ext_file("model/nonmem/run001/run001.ext")
/// }
#[extendr]
pub fn read_ext_file(
    path: &str,
    #[default = "NULL"] line_prefixes: Option<Vec<String>>,
    #[default = "FALSE"] parameters_only: Option<bool>,
    #[default = "NULL"] only_method: Option<&str>,
    #[default = "TRUE"] only_last: Option<bool>,
) -> Result<Robj> {
    let ext_reader = create_ext_reader(line_prefixes, parameters_only, only_method, only_last)?;
    let path = find_output_file(path, "ext")?;

    let tables = ext_reader
        .parse_file(path)
        .map_err(|e| Error::Other(e.to_string()))?;

    estimation_tables_to_dataframe(tables)
}

// /// A single row of parameter estimates
// #[derive(Debug, Clone)]
// pub struct EstimationRow {
//     pub iteration: isize,
//     pub values: Vec<f64>,
// }
//
// /// Represents a single estimation table from a NONMEM .ext file
// #[derive(Debug, Clone)]
// pub struct EstimationTable {
//     /// Estimation method (e.g., "First Order Conditional Estimation", "Iterative Two Stage")
//     pub method: Option<EstimationMethod>,
//     /// Parameter names from the ITER header line
//     pub parameters: Vec<String>,
//     /// Rows of parameter values
//     /// The size of parameters and rows should match
//     pub rows: Vec<EstimationRow>,
// }
//
// impl EstimationTable {
//     pub fn to_csv(&self) -> String {
//         let mut lines = Vec::new();
//         lines.push(parsing::format_csv_header(&self.parameters));
//
//         for row in &self.rows {
//             let values: Vec<String> = row.values.iter().map(|v| v.to_string()).collect();
//             lines.push(values.join(","));
//         }
//
//         lines.join("\n")
//     }
// }
/// Gets all parameters from a batch of ext files
///
/// @param dir directory containing ext files
/// @param path path to model file, model output directory, ext file or metadata json file.
/// @param line_prefixes character vector for lines to filter for
/// @param parameters_only bool if true removes ITERATION and OBJ column, default false
/// @param only_method character, filter for getting estimates from specified method only
/// @param only_last boolean, for grabbing only last estimation method parameters
///
/// @return list of data.frame of ext file
/// @export
///
/// @examples \dontrun{
/// read_ext_file("model/nonmem/run001/run001.ext")
/// }
#[extendr]
pub fn get_parameters_batch(
    dir: &str,
    #[default = "NULL"] line_prefixes: Option<Vec<String>>,
    #[default = "FALSE"] parameters_only: Option<bool>,
    #[default = "NULL"] only_method: Option<&str>,
    #[default = "TRUE"] only_last: Option<bool>,
) -> Result<Robj> {
    let ext_reader = create_ext_reader(line_prefixes, parameters_only, only_method, only_last)?;

    // Find all .ext files in the directory
    let dir_path = Path::new(dir);
    let ext_files: Vec<_> = std::fs::read_dir(dir_path)
        .map_err(|e| Error::Other(format!("Failed to read directory {}: {}", dir, e)))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()? == "ext" {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    if ext_files.is_empty() {
        return Err(Error::Other(format!(
            "No .ext files found in directory: {}",
            dir
        )));
    }

    let results = ext_reader
        .parse_file_batch(ext_files)
        .map_err(|e| Error::Other(e.to_string()))?;

    if results.is_empty() {
        return Err(Error::Other("No tables found in ext file".to_string()));
    }

    // Map each file's tables to a dataframe using the helper function (in parallel)
    let pairs: Vec<(String, Robj)> = results
        .into_iter()
        .map(|(path, tables)| {
            let file_stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            let df = estimation_tables_to_dataframe(tables)?;
            Ok((file_stem, df))
        })
        .collect::<Result<Vec<_>>>()?;

    // Return as a named list
    let result_list = List::from_pairs(pairs);
    Ok(result_list.into_robj())
}

extendr_module! {
    mod ext;
    fn get_parameter_estimates_wrap;
    fn read_ext_file;
    fn get_parameters_batch;
}
