use extendr_api::prelude::*;
use std::path::Path;

// pharos config and nonmem crates
use nonmem::check_model;

use crate::utils::load_nonmem_config;

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
pub fn check_model_wrap(model_path: &str) -> Result<String> {
    let (_config_path, nonmem_config) = load_nonmem_config(None)
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
