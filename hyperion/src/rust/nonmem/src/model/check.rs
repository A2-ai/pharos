use extendr_api::Result;
use extendr_api::prelude::*;
use hyperion_core::ResultExt;

// pharos config and nonmem crates
use nonmem::check_model;

use crate::utils::{load_nonmem_config, path_from_robj};
use hyperion_core::extendr_err;

/// Checks mod file for nmtran errors
///
/// @param model_path path to nonmem model file, or a hyperion_nonmem_model object
///
/// @return exit code of NMTRAN
/// @export
///
/// @examples \dontrun{
/// check_model("model/nonmem/1001.mod")
/// model <- read_model("model/nonmem/1001.mod")
/// check_model(model)
/// }
#[extendr(r_name = "check_model")]
pub fn check_model_wrap(model_path: Robj) -> Result<i32> {
    let model_path = path_from_robj(&model_path, true)?;

    let (_config_path, nonmem_config) =
        load_nonmem_config(None).map_to_extendr_err("Failed to create NonmemConfig")?;

    let res = match check_model(&nonmem_config, &model_path) {
        Ok(r) => r,
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("NMTRAN.exe not found") {
                println!("{}", error_msg.trim());
                return Ok(-1);
            } else {
                return Err(extendr_err!("Failed to run NMTRAN.exe: {e}"));
            }
        }
    };

    rprintln!("{}", res.stdout.trim());

    Ok(res.exit_code)
}

extendr_module! {
    mod check;

    fn check_model_wrap;
}
