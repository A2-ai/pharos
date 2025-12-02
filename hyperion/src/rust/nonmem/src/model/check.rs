use extendr_api::prelude::*;
use hyperion_core::ResultExt;
use std::path::Path;

// pharos config and nonmem crates
use nonmem::check_model;

use crate::utils::load_nonmem_config;
use hyperion_core::extendr_err;

/// Checks mod file for nmtran errors
///
/// @param model_path path to nonmem model file
///
/// @return NULL
/// @export
///
/// @examples \dontrun{
/// check_model("model/nonmem/1001.mod")
/// }
#[extendr(r_name = "check_model")]
pub fn check_model_wrap(model_path: &str) -> Result<String> {
    let (_config_path, nonmem_config) =
        load_nonmem_config(None).map_to_extendr_err("Failed to create NonmemConfig")?;

    let res = match check_model(&nonmem_config, Path::new(&model_path)) {
        Ok(r) => r,
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("NMTRAN.exe not found") {
                // Return this specific error as a successful result
                return Ok(error_msg);
            } else {
                // All other errors remain as actual errors
                return Err(extendr_err!("Failed to run NMTRAN.exe: {e}"));
            }
        }
    };

    if res.success {
        Ok(res.stdout.to_string())
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
