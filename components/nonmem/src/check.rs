use anyhow::{Result, anyhow, bail};
use fs_err as fs;
use std::collections::HashSet;
use std::path::Path;
use std::process::{Command, Stdio};

use config::NonmemConfig;

pub fn check_model(nonmem_config: &NonmemConfig, model_file: &Path) -> Result<()> {
    let nmtrans_exec = nonmem_config.get_nmtrans_executable_path(None)?;

    let model_dir = model_file
        .parent()
        .ok_or_else(|| anyhow!("Could not determine model file directory"))?;

    // Collect files before running NMTRANS
    let files_before: HashSet<_> = fs::read_dir(model_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name())
        .collect();

    let file = fs::File::open(model_file)?;
    let status = Command::new(nmtrans_exec)
        .stdin(Stdio::from(file.into_file()))
        .current_dir(model_dir)
        .status()?;

    // Clean up new files created by NMTRANS
    let files_after: HashSet<_> = fs::read_dir(model_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name())
        .collect();

    for new_file in files_after.difference(&files_before) {
        let file_path = model_dir.join(new_file);
        let _ = fs::remove_file(file_path);
    }

    if !status.success() {
        bail!(
            "nmtrans failed with exit code: {}",
            status.code().unwrap_or(-1)
        );
    }

    Ok(())
}
