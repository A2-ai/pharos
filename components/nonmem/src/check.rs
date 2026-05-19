use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Result, anyhow};
use fs_err as fs;
use nonmem_parser::Model;

use crate::dataset::check_dataset;
use config::NonmemConfig;

#[derive(Debug, Serialize, Deserialize)]
pub struct NmtranResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
}

pub fn check_model(
    nonmem_config: &NonmemConfig,
    model_file: &Path,
    no_parse: bool,
) -> Result<NmtranResult> {
    let nmtrans_exec = nonmem_config.get_nmtrans_executable_path(None)?;

    let model_dir = model_file
        .parent()
        .ok_or_else(|| anyhow!("Could not determine model file directory"))?;

    let (working_dir, stdin_path, _tmp_dir) = if no_parse {
        (model_dir.to_path_buf(), model_file.to_path_buf(), None)
    } else {
        let model = Model::parse(model_file, &fs::read_to_string(model_file)?)?;
        let dataset = check_dataset(&model, model_dir)?;
        let model_content = model.with_modified_paths(&dataset.canonical_path);

        let tmp_dir = tempfile::tempdir()?;
        let model_tmp_path = tmp_dir.path().join("model.mod");
        fs::write(&model_tmp_path, model_content)?;
        log::debug!("Model written to {}", model_tmp_path.display());
        (tmp_dir.path().to_path_buf(), model_tmp_path, Some(tmp_dir))
    };

    let file = fs::File::open(stdin_path)?;
    let output = Command::new(nmtrans_exec)
        .stdin(Stdio::from(file.into_file()))
        .current_dir(working_dir)
        .output()?;

    Ok(NmtranResult {
        success: output.status.success(),
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
    })
}
