use config::{Config, NonmemConfig, find_config_dir};
use extendr_api::prelude::*;
use nonmem::check_model;
use std::path::{Path, PathBuf};

fn load_nonmem_config(
    config_path: PathBuf,
    run_nonmem_version: Option<&str>,
) -> Result<NonmemConfig> {
    let config = Config::load(config_path)
        .map_err(|e| Error::Other(format!("Failed to load config: {e}")))?;

    let nonmem_config = config.nonmem.ok_or(Error::Other(
        "pharos config file does not contain nonmem configuration".to_string(),
    ))?;

    if let Some(version) = run_nonmem_version
        && !nonmem_config.versions.contains_key(version)
    {
        return Err(Error::Other(format!(
            "nonmem version {version} not found in config file"
        )));
    }

    Ok(nonmem_config)
}

/// Checks mod file for nmtran errors
///
/// @param model_path path to nonmem model file
/// @param config_path path to pharos.toml config file, attempts to find automatically.
///
/// @return NULL
/// @export
///
/// @examples \dontrun{
/// check_model("model/nonmem/1001.mod")
/// }
#[extendr(r_name = "check_model")]
pub fn check_model_wrap(
    model_path: &str,
    #[default = "NULL"] config_path: Option<&str>,
) -> Result<String> {
    let config_path = match config_path {
        Some(c) => c.into(),
        None => find_config_dir()
            .map_err(|e| Error::Other(format!("Failed to find config dir: {e}")))?
            .ok_or_else(|| Error::Other("Could not find pharos config directory".to_string()))?
            .join("pharos.toml"),
    };

    let nonmem_config = load_nonmem_config(config_path, None)
        .map_err(|e| Error::Other(format!("Failed to create NonmemConfig: {e}")))?;

    let res = match check_model(&nonmem_config, Path::new(&model_path)) {
        Ok(r) => r,
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("NMTRAN.exe not found") {
                // Return this specific error as a successful result
                return Ok(error_msg);
            } else {
                // All other errors remain as actual errors
                return Err(Error::Other(format!("Failed to run NMTRAN.exe: {e}")));
            }
        }
    };

    if res.success {
        Ok(format!("{}", res.stdout))
    } else {
        Ok(format!(
            "{}\nnmtran failed with exit code {:?}",
            res.stdout, res.exit_code
        ))
    }
}

extendr_module! {
    mod check;

    fn check_model_wrap;
}
