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
) -> Result<()> {
    let config_path = match config_path {
        Some(c) => c.into(),
        None => find_config_dir()
            .map_err(|e| Error::Other(format!("Failed to find config dir: {e}")))?
            .ok_or_else(|| Error::Other("Could not find pharos config directory".to_string()))?
            .join("pharos.toml"),
    };

    let nonmem_config = load_nonmem_config(config_path, None)
        .map_err(|e| Error::Other(format!("Failed to create NonmemConfig: {e}")))?;

    let model_path = Path::new(&model_path);

    match check_model(&nonmem_config, model_path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("NMTRAN.exe not found") {
                // Display NMTRAN not found as message instead of error
                rprintln!("{}", error_msg);
                Ok(())
            } else {
                // Other errors should still cause function to fail
                Err(Error::Other(format!("Failed to check model: {e}")))
            }
        }
    }
}

extendr_module! {
    mod check;

    fn check_model_wrap;
}
