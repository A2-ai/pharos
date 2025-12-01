use fs_err as fs;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use crate::run::setup::ModelSetup;
use anyhow::{Result, bail};
use tera::{Context, Tera};

pub fn execute_post_run_script(
    script_path: &Path,
    nonmem_exit_code: i32,
    model_setup: &ModelSetup,
) -> Result<()> {
    log::debug!("Executing post-run script {script_path:?}");
    let mut env_vars = HashMap::new();
    let model_dir = model_setup.model_dir.canonicalize()?;
    env_vars.insert(
        "PHAROS_NONMEM_EXIT_CODE".to_owned(),
        nonmem_exit_code.to_string(),
    );
    env_vars.insert(
        "PHAROS_MODEL_DIR".to_owned(),
        model_dir.to_string_lossy().to_string(),
    );
    env_vars.insert("PHAROS_MODEL_NAME".to_owned(), model_setup.name.clone());
    env_vars.insert(
        "PHAROS_OUTPUT_DIR".to_owned(),
        model_setup.output_dir.to_string_lossy().to_string(),
    );
    let mut context = Context::new();
    context.insert("exit_code", &nonmem_exit_code);
    context.insert("model_dir", &model_dir);
    context.insert("output_dir", &model_setup.output_dir);
    context.insert("model_name", &model_setup.name);
    let rendered = Tera::one_off(&fs::read_to_string(script_path)?, &context, false)?;

    let rendered_path = model_setup.output_dir.join("post_run_script");
    let mut out = fs::File::create(&rendered_path)?;
    out.write_all(rendered.as_bytes())?;
    out.flush()?;

    match Command::new("sh")
        .arg(rendered_path)
        .current_dir(&model_setup.output_dir)
        .envs(env_vars)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
    {
        Ok(post_run_status) => {
            if !post_run_status.success() {
                bail!(
                    "Error executing post_run script, exit code: {}",
                    post_run_status.code().unwrap_or(0)
                );
            }
        }
        Err(e) => {
            bail!("Error executing post_run script: {e}");
        }
    }

    log::debug!("Post-run script finished successfully");
    Ok(())
}
