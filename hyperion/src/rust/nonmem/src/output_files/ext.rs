use extendr_api::{Robj, prelude::*};
use std::ffi::OsStr;
use std::path::Path;

//pharos nonmem crate
use nonmem::estimation;
use nonmem::output_files::ext::{EstimationTable, ExtReader};

use crate::utils::find_output_file;

/// Extract .ext files from path (single file or directory)
/// Returns Vec<(PathBuf, String)> where String is the model name (file stem)
fn extract_ext_files_from_path(path: &str) -> Result<Vec<(std::path::PathBuf, String)>> {
    let path_obj = Path::new(path);

    // Case 1: Single .ext file
    if path_obj.is_file() {
        if path_obj.extension() == Some(OsStr::new("ext")) {
            let model_name = path_obj
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            return Ok(vec![(path_obj.to_path_buf(), model_name)]);
        } else {
            return Err(Error::Other(format!("File must be .ext: {}", path)));
        }
    }

    // Case 2: Directory - recursively scan for .ext files
    if path_obj.is_dir() {
        fn scan_directory_recursive(dir: &Path) -> Result<Vec<(std::path::PathBuf, String)>> {
            let mut ext_files = Vec::new();

            for entry in std::fs::read_dir(dir).map_err(|e| {
                Error::Other(format!("Failed to read directory {}: {}", dir.display(), e))
            })? {
                let entry = entry
                    .map_err(|e| Error::Other(format!("Failed to read directory entry: {}", e)))?;
                let path = entry.path();

                if path.is_file() && path.extension() == Some(OsStr::new("ext")) {
                    let model_name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    ext_files.push((path, model_name));
                } else if path.is_dir() {
                    // Recursively scan subdirectories
                    let mut sub_files = scan_directory_recursive(&path)?;
                    ext_files.append(&mut sub_files);
                }
            }

            Ok(ext_files)
        }

        let ext_files = scan_directory_recursive(path_obj)?;

        if ext_files.is_empty() {
            return Err(Error::Other(format!(
                "No .ext files found in directory (including subdirectories): {}",
                path
            )));
        }
        return Ok(ext_files);
    }

    // Case 3: Invalid input
    Err(Error::Other(format!(
        "Path must be .ext file or directory: {}",
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
pub fn create_ext_reader(
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

/// Gets all final estimates from an ext file or vector of ext files
///
/// @param paths path to directory containing ext files (including subdirectories), single ext file, or vector of ext file paths
/// @param parameters_only bool if true removes ITERATION and OBJ column, default false
/// @param only_method character, filter for getting estimates from specified method only
/// @param only_last boolean, for grabbing only last estimation method parameters
///
/// @return data.frame of final estimates with model names
/// @export
///
/// @examples \dontrun{
/// get_final_estimates("model/nonmem/")
/// get_final_estimates("bootstrap/")  # Searches subdirectories recursively
/// get_final_estimates(c("run001.ext", "run002.ext", "run003.ext"))
/// }
#[extendr]
pub fn get_final_estimates(
    paths: Robj,
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

    // Handle different input types: single string or vector of strings
    let ext_files_with_names = if let Some(path_str) = paths.as_str() {
        // Single string input - use existing helper
        extract_ext_files_from_path(path_str)?
    } else if let Some(path_vec) = paths.as_str_vector() {
        // Vector of strings input - process each path individually
        let mut all_files = Vec::new();
        for path_str in path_vec {
            if Path::new(&path_str).extension() == Some(OsStr::new("ext")) {
                // Single .ext file
                let model_name = Path::new(&path_str)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                all_files.push((Path::new(&path_str).to_path_buf(), model_name));
            } else {
                return Err(Error::Other(format!(
                    "All paths must be .ext files: {}",
                    path_str
                )));
            }
        }
        if all_files.is_empty() {
            return Err(Error::Other("No .ext files provided in vector".to_string()));
        }
        all_files
    } else {
        return Err(Error::Other(
            "Input must be a string or vector of strings".to_string(),
        ));
    };
    let length = ext_files_with_names.len();

    // Split into paths and names without cloning
    let (ext_files, model_names_ordered): (Vec<std::path::PathBuf>, Vec<String>) =
        ext_files_with_names.into_iter().unzip();

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
    let mut param_columns: Vec<Vec<Rfloat>> = (0..param_names.len())
        .map(|_| Vec::with_capacity(length))
        .collect();

    for (_, tables) in results {
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
    let mut pairs = vec![("model", model_names_ordered.into_robj())];
    for (param_name, param_column) in param_names.iter().zip(param_columns.into_iter()) {
        pairs.push((param_name.as_str(), param_column.into_robj()));
    }

    let list = List::from_pairs(pairs);
    let df = data_frame!(list);

    Ok(df)
}

extendr_module! {
    mod ext;

    fn read_ext_file;
    fn get_final_estimates;
}
