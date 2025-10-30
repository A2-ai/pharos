use extendr_api::{Robj, prelude::*};
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use tempfile::NamedTempFile;
use zip;

use crate::output_files::{OMEGA, ParameterRow, ParameterRowBuilder, ParameterTable, SIGMA, THETA};
use crate::utils::{find_output_file, get_comment_type};
use nonmem::estimation;
use nonmem::output_files::ext::{EstimationTable, ExtReader, get_parameter_estimates};
use nonmem::output_files::get_parameter_names;
use nonmem::output_files::shk::ShkReader;
use nonmem::Model;
//use rayon::prelude::*;

/// Extract .ext files from path (single file, directory, or zip)
fn extract_ext_files_from_path(path: &str) -> Result<Vec<std::path::PathBuf>> {
    let path_obj = Path::new(path);

    // Case 1: Single .ext file
    if path_obj.is_file() {
        if path_obj.extension() == Some(OsStr::new("ext")) {
            return Ok(vec![path_obj.to_path_buf()]);
        } else if path_obj.extension() == Some(OsStr::new("zip")) {
            // Case 2: Zip file - extract ONLY .ext files to temp locations
            let file = std::fs::File::open(path_obj)
                .map_err(|e| Error::Other(format!("Failed to open zip file: {}", e)))?;
            let mut archive = zip::ZipArchive::new(file)
                .map_err(|e| Error::Other(format!("Failed to read zip archive: {}", e)))?;

            let mut temp_files = Vec::new();
            for i in 0..archive.len() {
                let mut zip_file = archive
                    .by_index(i)
                    .map_err(|e| Error::Other(format!("Failed to read zip entry: {}", e)))?;

                // Check if zip entry has .ext extension
                if Path::new(zip_file.name()).extension() == Some(OsStr::new("ext")) {
                    let original_stem = Path::new(zip_file.name())
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown");
                    let mut temp_file = NamedTempFile::with_prefix(original_stem)
                        .map_err(|e| Error::Other(format!("Failed to create temp file: {}", e)))?;

                    std::io::copy(&mut zip_file, &mut temp_file.as_file_mut())
                        .map_err(|e| Error::Other(format!("Failed to extract file: {}", e)))?;

                    let temp_path = temp_file.path().to_path_buf();
                    temp_files.push(temp_path);
                    std::mem::forget(temp_file); // Prevent auto-deletion during processing
                }
            }

            if temp_files.is_empty() {
                return Err(Error::Other(format!(
                    "No .ext files found in zip: {}",
                    path
                )));
            }
            return Ok(temp_files);
        } else {
            return Err(Error::Other(format!("File must be .ext or .zip: {}", path)));
        }
    }

    // Case 3: Directory - scan for .ext files
    if path_obj.is_dir() {
        let ext_files: Vec<std::path::PathBuf> = std::fs::read_dir(path_obj)
            .map_err(|e| Error::Other(format!("Failed to read directory {}: {}", path, e)))?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension() == Some(OsStr::new("ext")) {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();

        if ext_files.is_empty() {
            return Err(Error::Other(format!(
                "No .ext files found in directory: {}",
                path
            )));
        }
        return Ok(ext_files);
    }

    // Case 4: Invalid input
    Err(Error::Other(format!(
        "Path must be .ext file, directory, or .zip file: {}",
        path
    )))
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

/// Helper function to build ExtReader
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
/// get_parameter_estimates("model/nonmem/run001/run001.ext")
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

    let shk_data = match find_output_file(path, "shk") {
        Ok(p) => match ShkReader::default().parse_file(p) {
            Ok(s) => s,
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    };

    let ext_path = find_output_file(path, "ext")?;
    let model_path = find_output_file(path, "mod")?;
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

/// Gets all final estimates from a batch of ext files
///
/// @param dir path to directory containing ext files, zip file, or single ext file
/// @param parameters_only bool if true removes ITERATION and OBJ column, default false
/// @param only_method character, filter for getting estimates from specified method only
/// @param only_last boolean, for grabbing only last estimation method parameters
///
/// @return data.frame of final estimates with model names
/// @export
///
/// @examples \dontrun{
/// get_final_estimates_batch("model/nonmem/")
/// get_final_estimates_batch("model/archive.zip")
/// }
#[extendr]
pub fn get_final_estimates_batch(
    dir: &str,
    #[default = "TRUE"] parameters_only: Option<bool>,
    #[default = "NULL"] only_method: Option<&str>,
    #[default = "TRUE"] only_last: Option<bool>,
) -> Result<Robj> {
    let ext_reader = create_ext_reader(
        Some(vec!["-1000000000".to_string()]),
        parameters_only,
        only_method,
        only_last,
    )?;

    // Extract .ext files from directory, zip, or single file
    let ext_files = extract_ext_files_from_path(dir)?;
    let length = ext_files.len();

    let results = ext_reader
        .parse_file_batch(ext_files)
        .map_err(|e| Error::Other(e.to_string()))?;

    if results.is_empty() {
        return Err(Error::Other("No tables found in ext file".to_string()));
    }

    // Get parameter names from first table (all should be the same)
    let param_names = if let Some((_, first_tables)) = results.first() {
        if let Some(first_table) = first_tables.first() {
            first_table.parameters.clone()
        } else {
            return Err(Error::Other(
                "No tables found in first ext file".to_string(),
            ));
        }
    } else {
        return Err(Error::Other("No results found".to_string()));
    };

    // build parameter columns directly (column-first approach)
    let mut model_names = Vec::with_capacity(length);
    let mut param_columns: Vec<Vec<Rfloat>> = (0..param_names.len())
        .map(|_| Vec::with_capacity(length))
        .collect();

    for (path, tables) in results {
        let file_stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        model_names.push(file_stem);

        // Extract parameter values and populate columns directly
        if let Some(table) = tables.first() {
            if let Some(row) = table.rows.first() {
                for (param_idx, &value) in row.values.iter().enumerate() {
                    let rfloat_val = if value.is_nan() {
                        Rfloat::na()
                    } else {
                        Rfloat::from(value)
                    };

                    // Safety check to avoid bounds issues
                    if param_idx < param_columns.len() {
                        param_columns[param_idx].push(rfloat_val);
                    }
                }
            } else {
                return Err(Error::Other("No rows found in table".to_string()));
            }
        } else {
            return Err(Error::Other("No tables found".to_string()));
        }
    }

    // Build dataframe
    let mut pairs = vec![("model", model_names.into_robj())];
    for (param_name, param_column) in param_names.iter().zip(param_columns.into_iter()) {
        pairs.push((param_name.as_str(), param_column.into_robj()));
    }

    let list = List::from_pairs(pairs);
    let df = data_frame!(list);

    Ok(df)
}

extendr_module! {
    mod ext;
    fn get_parameters;
    fn read_ext_file;
    fn get_final_estimates_batch;
}
