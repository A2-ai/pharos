use config::{Config, NonmemConfig};
use extendr_api::prelude::*;
use nonmem::check_model;
use std::path::Path;

fn load_nonmem_config(config_path: &str, run_nonmem_version: Option<&str>) -> Result<NonmemConfig> {
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
/// @param config_path path to pharos.toml config file
///
/// @return NULL
/// @export
///
/// @examples \dontrun{
/// check_model("model/nonmem/1001.mod")
/// }
#[extendr(r_name = "check_model")]
pub fn check_model_wrap(model_path: &str, config_path: &str) -> Result<()> {
    let nonmem_config = load_nonmem_config(config_path, None)
        .map_err(|e| Error::Other(format!("Failed to create NonmemConfig: {e}")))?;

    check_model(&nonmem_config, Path::new(&model_path))
        .map_err(|e| Error::Other(format!("Failed to check model: {e}")))?;

    Ok(())
}

extendr_module! {
    mod check;

    fn check_model_wrap;
}
