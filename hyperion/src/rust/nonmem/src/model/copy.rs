use extendr_api::prelude::*;

use nonmem::copy::{JitterSpec, ParamType, UpdateType};
use nonmem::{CopyOptions, copy_model};
use std::path::{Path, PathBuf};

use hyperion_core::{OptionExt, ResultExt};

fn parse_jitter_robj(jitter: Option<Robj>) -> Result<Vec<JitterSpec>> {
    match jitter {
        Some(robj) => {
            if robj.is_null() {
                // Handle NULL case
                Ok(Vec::new())
            } else if robj.is_real() && robj.len() == 1 && robj.names().is_none() {
                // Scalar: jitter = 0.2 -> all parameters
                let percentage = robj.as_real().unwrap();
                Ok(vec![JitterSpec {
                    param_type: ParamType::All,
                    percentage,
                }])
            } else if robj.is_real() && robj.names().is_some() {
                // Named vector: c("theta" = 0.1, "omega" = 0.2)
                let values = robj.as_real_vector().unwrap();
                let names = robj.names().unwrap();
                let mut specs = Vec::new();

                for (i, name) in names.enumerate() {
                    let param_type = match name.to_lowercase().as_str() {
                        "all" => ParamType::All,
                        "theta" => ParamType::Theta,
                        "omega" => ParamType::Omega,
                        "sigma" => ParamType::Sigma,
                        _ => {
                            return Err(Error::Other(format!(
                                "Unknown jitter parameter type: {}",
                                name
                            )));
                        }
                    };
                    specs.push(JitterSpec {
                        param_type,
                        percentage: values[i],
                    });
                }
                Ok(specs)
            } else {
                Err(Error::Other(
                    "Invalid jitter format - must be scalar or named numeric vector".to_string(),
                ))
            }
        }
        None => Ok(Vec::new()), // No jitter
    }
}

fn parse_jitter_excluded_robj(jitter_excluded: Option<Robj>) -> Result<Option<String>> {
    match jitter_excluded {
        Some(robj) => {
            if robj.is_null() {
                Ok(None)
            } else if robj.is_string() {
                if robj.len() == 1 {
                    // Scalar string: jitter_excluded = "THETA1"
                    Ok(Some(robj.as_str().unwrap().to_string()))
                } else {
                    // Character vector: jitter_excluded = c("THETA1", "OMEGA(1,1)")
                    // Join with commas for the underlying implementation
                    let excluded_params: Vec<String> = robj
                        .as_str_vector()
                        .unwrap()
                        .into_iter()
                        .map(|s| s.to_string())
                        .collect();
                    Ok(Some(excluded_params.join(",")))
                }
            } else {
                Err(Error::Other(
                    "jitter_excluded parameter must be a string or character vector".to_string(),
                ))
            }
        }
        None => Ok(None),
    }
}

fn parse_update_robj(update: Robj) -> Result<Vec<UpdateType>> {
    let strings = if update.is_string() {
        if update.len() == 1 {
            // Scalar string: update = "all" or update = "theta"
            vec![update.as_str().unwrap().to_string()]
        } else {
            // Character vector: update = c("theta", "omega")
            update
                .as_str_vector()
                .unwrap()
                .into_iter()
                .map(|s| s.to_string())
                .collect()
        }
    } else {
        return Err(Error::Other(
            "Update parameter must be a string or character vector".to_string(),
        ));
    };

    let mut update_types = Vec::new();
    for s in strings {
        match s.to_lowercase().as_str() {
            "all" => update_types.push(UpdateType::All),
            "none" => update_types.push(UpdateType::None),
            "theta" => update_types.push(UpdateType::Theta),
            "omega" => update_types.push(UpdateType::Omega),
            "sigma" => update_types.push(UpdateType::Sigma),
            _ => return Err(Error::Other(format!("Unknown update type: {}", s))),
        }
    }
    if update_types.is_empty() {
        update_types.push(UpdateType::None);
    }
    Ok(update_types)
}

/// Copies model file to new model file
///
/// @param from path to model file to copy
/// @param to path to model file to write to
/// @param overwrite boolean, wheter to overwrite existing model. Default FALSE
/// @param ext_file path to ext file to use for parameter estimates
/// @param update character or character vector specifying which parameters to update from ext file.
/// Options: "all", "none", "theta", "omega", "sigma". Examples: "all" or c("theta", "omega")
/// @param jitter numeric value or named numeric vector for parameter jittering using uniform distribution.
/// Each parameter value is multiplied by a random factor between (1 - jitter%) and (1 + jitter%) with boundary enforcement.
/// Examples: 0.1 (10% jitter on all params) or c("theta" = 0.05, "omega" = 0.1)
/// @param jitter_excluded character or character vector of parameter names to exclude from jittering.
/// Examples: "THETA1" or c("THETA1", "OMEGA(1,1)")
/// @param seed integer for random number generator seed to ensure reproducible jittering
/// @param description Description of model in metadata file
/// @param no_metadata boolean, if true, does not create metadatafile, default FALSE
///
/// @return path to new model file (invisible) todo
/// @export
///
/// @examples \dontrun{
/// copy_model(from = "model/nonmem/run001.mod", to = "model/nonmem/run002.mod")
/// }
#[extendr(r_name = "copy_model")]
pub fn copy_model_wrap(
    from: &str,
    to: &str,
    #[default = "FALSE"] overwrite: bool,
    #[default = "NULL"] ext_file: Option<&str>,
    #[default = "'none'"] update: Robj,
    #[default = "NULL"] jitter: Option<Robj>,
    #[default = "NULL"] jitter_excluded: Option<Robj>,
    #[default = "NULL"] seed: Option<u64>,
    #[default = "NULL"] description: String,
    #[default = "FALSE"] no_metadata: bool,
) -> Result<()> {
    // Parse input parameters
    let update_types = parse_update_robj(update)?;
    let jitter_specs = parse_jitter_robj(jitter)?;
    let jitter_excluded_parsed = parse_jitter_excluded_robj(jitter_excluded)?;

    let mut options = CopyOptions {
        update: update_types,
        ext_path: ext_file.map(PathBuf::from),
        jitter: jitter_specs,
        seed,
        jitter_excluded: jitter_excluded_parsed,
        description,
        no_metadata,
    };

    let from = Path::new(&from);
    if !from.exists() {
        return Err(Error::Other(format!(
            "Model file does not exist: {}",
            from.display()
        )));
    }
    let original_filename = match from.file_name() {
        Some(filename) => filename.to_string_lossy().to_string(),
        None => Err(Error::Other(
            "`from` model file does not have a file name".to_string(),
        ))?,
    };

    // Validate to file doesn't exist or overwrite is allowed
    let to = Path::new(&to);
    if to.exists() && !overwrite {
        return Err(Error::Other(format!(
            "Model file {} already exists and the --overwrite flag was not passed",
            to.display()
        )));
    }

    let new_filename = match to.file_name() {
        Some(filename) => filename.to_string_lossy().to_string(),
        None => Err(Error::Other(
            "`to` model file does not have a file name".to_string(),
        ))?,
    };

    // Validate ext file if parameter updates are requested
    if options.is_updating_params() {
        let ext_path = match &ext_file {
            Some(path) => PathBuf::from(path),
            None => {
                // Default: run001.mod -> run001/run001.ext
                let model_stem = from
                    .file_stem()
                    .ok_or_extendr_err("Could not determine model file stem")?
                    .to_string_lossy();

                from.parent()
                    .ok_or_extendr_err("Could not determine parent directory")?
                    .join(&*model_stem)
                    .join(format!("{}.ext", model_stem))
            }
        };

        if !ext_path.exists() {
            if ext_file.is_none() {
                Error::Other(format!(
                    "Could not find .ext file at expected location: {}\n\
                                 Use ext_file to specify the correct path to the parameter estimates file",
                    ext_path.display()
                ));
            } else {
                Error::Other(format!("Ext file not found: {}", ext_path.display()));
            }
        }
        options.ext_path = Some(ext_path);
    }

    copy_model(from, to, &original_filename, &new_filename, &options)
        .map_to_extendr_err("Failed to copy model")?;

    Ok(())
}

extendr_module! {
    mod copy;
    fn copy_model_wrap;
}
