mod check;
pub mod copy;
pub mod estimation;
mod lineage;
mod model_metadata;
mod model_name_pattern;
pub mod output_files;
mod parsing;
mod run;
pub mod runner;
pub mod transforms;

use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use config::{CONFIG_FILENAME, NonmemConfig};
use fs_err as fs;
use serde::Serialize;
use tempfile::{TempDir, tempdir, tempdir_in};
use utils::{get_utc_now, write_json_to_file};

pub use run::signal_wrapper::{TERMINATION_FILENAME, Termination};

pub use run::RunOptions;
use run::metadata::RUN_CONFIG_FILENAME;
use run::setup::ModelSetup;

#[cfg(unix)]
use run::signal_wrapper::execute_with_termination_handling;

pub use crate::model_name_pattern::expand_model_pattern;
use crate::run::files::calculate_output_file_hashes;
use crate::run::post_run;
pub use check::check_model;
pub use copy::{CopyOptions, copy_model};
pub use lineage::LineageTree;
pub use model_metadata::{
    ModelMetadata, clear_metadata_file, update_metadata_file, validate_model_path,
};
pub use parsing::{Dataset, Model};
pub use run::metadata::{OutputFileHash, RunEndFile, RunStartFile};
pub use runner::run_models;

/// Check if a directory supports script execution by running a minimal test script.
#[cfg(target_os = "linux")]
fn can_execute_in_dir(dir: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    let test_script = dir.join(".exec_test.sh");

    let result = (|| -> Result<bool> {
        let mut f = fs::File::create(&test_script)?;
        f.write_all(b"#!/bin/sh\nexit 0\n")?;
        drop(f);

        // Make executable
        let mut perms = fs::metadata(&test_script)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&test_script, perms)?;

        // Try to execute
        let status = Command::new(&test_script)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()?;

        Ok(status.success())
    })();

    // Cleanup
    let _ = fs::remove_file(&test_script);

    result.unwrap_or(false)
}

#[derive(Debug, Serialize)]
pub struct NonmemRunner {
    model: PathBuf,
    overwrite: bool,
    run_in_output_dir: bool,
    config: NonmemConfig,
    nonmem_version: Option<String>,
    output_dir: Option<String>,
    #[serde(skip)]
    tempdir: Option<TempDir>,
    extra_files: Vec<String>,
    config_dir: PathBuf,
}

impl NonmemRunner {
    pub fn new(
        model: impl AsRef<Path>,
        overwrite: bool,
        config: NonmemConfig,
        nonmem_version: Option<String>,
        output_dir: Option<String>,
        extra_files: Vec<String>,
        config_dir: impl AsRef<Path>,
    ) -> NonmemRunner {
        Self {
            model: model.as_ref().to_owned(),
            overwrite,
            run_in_output_dir: false,
            config,
            nonmem_version,
            output_dir,
            extra_files,
            tempdir: None,
            config_dir: config_dir.as_ref().to_owned(),
        }
    }

    pub fn run_in_output_dir(&mut self) {
        self.run_in_output_dir = true;
    }

    fn generate_script(&self, model_name: &str) -> Result<String> {
        let nonmem_path = self
            .config
            .get_nonmem_executable_path(self.nonmem_version.as_deref())?;
        let mut elems = vec![nonmem_path.to_string_lossy().into_owned()];
        elems.push(format!("{model_name}.mod"));
        elems.push(format!("{model_name}.lst"));

        // Add parafile flag if parallel execution is enabled
        if self.config.parallel.enabled {
            elems.push(format!("-parafile={model_name}.pnm"));
        }

        elems.extend(self.config.options.as_flags());

        Ok(elems.join(" "))
    }

    fn validate_and_prepare(&self) -> Result<(ModelSetup, Option<PathBuf>)> {
        // 0. Validate parallel configuration if enabled
        self.config
            .parallel
            .validate(&self.config_dir)
            .with_context(|| {
                format!(
                    "Failed to validate parallel configuration in file: {:?}",
                    self.config_dir.join(CONFIG_FILENAME)
                )
            })?;

        // 1. We ensure the model and dataset exist, hashing it as well.
        let model_setup = run::setup::prepare_model(
            &self.model,
            self.overwrite,
            self.output_dir.clone(),
            &self.config,
        )
        .with_context(|| format!("Failed to prepare model: {}", self.model.display()))?;

        log::debug!("Model output dir will be {:?}", model_setup.output_dir);

        let post_run_script =
            if let Some(script_path) = self.config.post_run_script(&self.config_dir) {
                if !script_path.exists() {
                    bail!("Post-run script {script_path:?} was not found.");
                }

                Some(script_path.canonicalize()?)
            } else {
                None
            };

        Ok((model_setup, post_run_script))
    }

    fn setup_execution_environment(&mut self, model_setup: &ModelSetup) -> Result<PathBuf> {
        // 2. We get started, finding where we will run things
        let running_dir = if self.run_in_output_dir {
            model_setup.output_dir.clone()
        } else {
            #[cfg(target_os = "linux")]
            let tmp = {
                // Try /dev/shm first if it exists and supports execution
                let shm_path = Path::new("/dev/shm");
                if shm_path.exists() && can_execute_in_dir(shm_path) {
                    tempdir_in("/dev/shm").context("Failed to create temp directory in /dev/shm")?
                } else {
                    log::debug!(
                        "/dev/shm not available or mounted with noexec, using system temp directory"
                    );
                    tempdir().context("Failed to create temp directory")?
                }
            };
            #[cfg(not(target_os = "linux"))]
            let tmp = tempdir().context("Failed to create temp directory")?;
            let p = tmp.path().to_path_buf();
            self.tempdir = Some(tmp);
            p
        };
        log::debug!("Model {:?} will be running in {running_dir:?}", self.model);

        // Purely used for debugging purposes, we don't use the env vars anywhere
        let env_vars = utils::get_masked_env_vars();
        log::debug!("Env vars: {:#?}", env_vars);

        fs::create_dir_all(&model_setup.output_dir).with_context(|| {
            format!(
                "Failed to create output directory: {}",
                model_setup.output_dir.display()
            )
        })?;
        // a no-op if we are running in the output dir
        fs::create_dir_all(&running_dir).with_context(|| {
            format!(
                "Failed to create running directory: {}",
                running_dir.display()
            )
        })?;

        Ok(running_dir)
    }

    fn prepare_execution_files(
        &self,
        model_setup: &ModelSetup,
        running_dir: &Path,
    ) -> Result<String> {
        // Create the model file
        let mut f = fs::File::create(running_dir.join(format!("{}.mod", model_setup.name)))
            .context("Failed to create model file")?;
        f.write_all(model_setup.model_content.as_bytes())
            .context("Failed to write model content")?;

        // Create a .gitignore that ignores everything
        let mut f = fs::File::create(running_dir.join(".gitignore"))
            .context("Failed to create .gitignore file")?;
        f.write_all(run::gitignore::INITIAL_GITIGNORE.as_bytes())
            .context("Failed to write .gitignore content")?;

        // Add all extra files
        for extra in &self.extra_files {
            fs::copy(extra, running_dir.join(extra))
                .with_context(|| format!("Failed to copy extra file: {}", extra))?;
        }

        // Create the config snapshot
        write_json_to_file(&self, running_dir.join(RUN_CONFIG_FILENAME))
            .context("Failed to write config snapshot")?;

        // Create the run start dump
        let model_canonical_path = self.model.canonicalize()?;
        let start_file = RunStartFile::new(model_setup, &model_canonical_path);
        let start_time = start_file.start.clone();
        start_file
            .save(running_dir)
            .context("Failed to save run start file")?;

        // Generate parafile if parallel execution is enabled
        let parallel = &self.config.parallel;
        if parallel.enabled {
            let parafile_path = running_dir.join(format!("{}.pnm", model_setup.name));
            if let Some(existing_path) = parallel.parafile(&self.config_dir) {
                fs::copy(
                    existing_path.canonicalize().with_context(|| {
                        format!("Failed to canonicalize parafile path: {:?}", existing_path)
                    })?,
                    parafile_path,
                )
                .context("Failed to copy existing parafile")?;
            } else {
                let mut f =
                    fs::File::create(&parafile_path).context("Failed to create parafile")?;
                f.write_all(parallel.generate_parafile().as_bytes())
                    .context("Failed to write parafile content")?;
            }
        }

        Ok(start_time)
    }

    fn execute_nonmem_script(
        &self,
        model_setup: &ModelSetup,
        running_dir: &Path,
        mut copier_coordinator: Option<run::files::FileCopyCoordinator>,
    ) -> Result<(std::process::ExitStatus, HashSet<String>, Duration)> {
        // Generate and write the script to run nonmem
        let script = self.generate_script(&model_setup.name)?;
        let script_path = running_dir.join(format!("{}.sh", model_setup.name));
        let mut f = fs::File::create(&script_path)?;
        f.write_all(script.as_bytes())
            .context("Failed to write NONMEM script content")?;

        let script_start = Instant::now();

        let mut command = Command::new("sh");
        command.arg(script_path.file_name().unwrap());
        command.stdout(std::process::Stdio::inherit());
        command.stderr(std::process::Stdio::inherit());
        command.current_dir(running_dir);

        let status = {
            #[cfg(unix)]
            {
                log::debug!("Starting script with signal handling");
                execute_with_termination_handling(command, &model_setup.output_dir)
                    .context("Failed to execute NONMEM script with signal handling")?
            }
            #[cfg(not(unix))]
            {
                log::debug!("Starting script without signal handling");
                // On non-Unix systems, just run normally
                let mut command = command
                    .spawn()
                    .context("Failed to spawn NONMEM script process")?;
                command
                    .wait()
                    .context("Failed to wait for NONMEM script completion")?
            }
        };

        if status.success() {
            log::debug!(
                "Nonmem run successfully finished with status {:?}",
                status.code().unwrap_or(0)
            );
        } else {
            log::warn!(
                "NONMEM run failed with exit code: {}",
                status.code().unwrap_or_default()
            );
        }

        // 7. Stop background file copying and do final copy
        let files_copied = if let Some(ref mut copier) = copier_coordinator {
            copier
                .stop_and_finalize()
                .context("Failed to finalize file copying")?
        } else {
            HashSet::new()
        };

        let script_duration = script_start.elapsed();
        Ok((status, files_copied, script_duration))
    }

    pub fn run(&mut self) -> Result<i32> {
        let (model_setup, post_run_script) = self.validate_and_prepare()?;
        let running_dir = self.setup_execution_environment(&model_setup)?;
        let start_time = self.prepare_execution_files(&model_setup, &running_dir)?;

        // Setup file copying if running in a different directory
        let (copier_coordinator, custom_patterns) = if running_dir != model_setup.output_dir {
            let mut c = run::files::FileCopyCoordinator::new(
                &self.config,
                &model_setup,
                &running_dir,
                &model_setup.output_dir,
            );
            let patterns = c.copier.patterns.clone();
            c.start_background_copying()
                .context("Failed to start background file copying")?;
            (Some(c), patterns)
        } else {
            (None, Vec::new())
        };

        let (nonmem_exit_status, files_copied, script_duration) =
            self.execute_nonmem_script(&model_setup, &running_dir, copier_coordinator)?;

        // Calculate Blake3 hashes for output files
        let output_files_hashes = calculate_output_file_hashes(
            &model_setup.output_dir,
            &model_setup.name,
            &model_setup.output_files,
        );

        // We do that first so if a cleanup or something after fails we still get the end file
        let end_dump = RunEndFile {
            start: start_time,
            end: get_utc_now(),
            exit_code: nonmem_exit_status.code().unwrap_or_default(),
            runtime_ms: script_duration.as_millis(),
            files_copied,
            output_files_rewrites: model_setup.output_files.clone(),
            output_files_hashes,
        };
        let exit_code = end_dump.exit_code;
        end_dump
            .save(&model_setup.output_dir)
            .context("Failed to save run end metadata")?;

        run::files::cleanup_unwanted_files(
            &model_setup.output_dir,
            self.config.clean_level,
            &custom_patterns,
            &model_setup.name,
        )
        .with_context(|| {
            format!(
                "Failed to cleanup unwanted files in directory: {}",
                model_setup.output_dir.display()
            )
        })?;

        let mut f = fs::File::create(model_setup.output_dir.join(".gitignore"))
            .context("Failed to create final .gitignore file")?;
        f.write_all(run::gitignore::get_final_gitignore(&model_setup.name).as_bytes())
            .context("Failed to write final .gitignore content")?;

        if let Some(script_path) = post_run_script {
            post_run::execute_post_run_script(&script_path, end_dump.exit_code, &model_setup)
                .context("Failed to execute post-run script")?;
        }

        Ok(exit_code)
    }
}
