use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Result, anyhow, bail};
use fs_err as fs;

use crate::Model;
use config::NonmemConfig;

pub fn check_model(nonmem_config: &NonmemConfig, model_file: &Path) -> Result<()> {
    let nmtrans_exec = nonmem_config.get_nmtrans_executable_path(None)?;

    let model_dir = model_file
        .parent()
        .ok_or_else(|| anyhow!("Could not determine model file directory"))?;

    let model = Model::parse(&fs::read_to_string(model_file)?)?;
    let dataset = model.check_dataset(model_dir)?;
    let model_content = model.with_modified_paths(&dataset.canonical_path);

    let tmp_dir = tempfile::tempdir()?;
    let model_tmp_path = tmp_dir.path().join("model.mod");
    fs::write(&model_tmp_path, model_content)?;
    log::debug!("Model written to {}", model_tmp_path.display());
    let file = fs::File::open(model_tmp_path)?;
    let status = Command::new(nmtrans_exec)
        .stdin(Stdio::from(file.into_file()))
        .current_dir(tmp_dir.path())
        .status()?;

    if !status.success() {
        bail!(
            "nmtrans failed with exit code: {}",
            status.code().unwrap_or(-1)
        );
    }

    Ok(())
}
