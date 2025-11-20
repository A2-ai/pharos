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

use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use anyhow::{Result, bail};
use config::NonmemConfig;
use fs_err as fs;
use serde::Serialize;
use tempfile::{TempDir, tempdir, tempdir_in};
use utils::{get_utc_now, write_json_to_file};

pub use run::signal_wrapper::{TERMINATION_FILENAME, Termination};

pub use run::RunOptions;
use run::metadata::RUN_CONFIG_FILENAME;

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

    pub fn run(&mut self) -> Result<i32> {
        // 0. Validate parallel configuration if enabled
        self.config.parallel.validate(&self.config_dir)?;

        // 1. We ensure the model and dataset exist, hashing it as well.
        let model_setup = run::setup::prepare_model(
            &self.model,
            self.overwrite,
            self.output_dir.clone(),
            &self.config,
        )?;
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

        // 2. We get started, finding where we will run things
        let running_dir = if self.run_in_output_dir {
            model_setup.output_dir.clone()
        } else {
            let tmp = if cfg!(target_os = "linux") {
                tempdir_in("/dev/shm")?
            } else {
                tempdir()?
            };
            let p = tmp.path().to_path_buf();
            self.tempdir = Some(tmp);
            p
        };
        log::debug!("Model {:?} will be running in {running_dir:?}", self.model);

        let env_vars = utils::get_masked_env_vars();
        log::debug!("Env vars: {:#?}", env_vars);

        fs::create_dir_all(&model_setup.output_dir)?;
        fs::create_dir_all(&running_dir)?;
        // This will contain the canonicalized path to the dataset
        let mut f = fs::File::create(running_dir.join(format!("{}.mod", model_setup.name)))?;
        f.write_all(model_setup.model_content.as_bytes())?;
        // Create a .gitignore that ignores everything
        let mut f = fs::File::create(running_dir.join(".gitignore"))?;
        f.write_all(run::gitignore::INITIAL_GITIGNORE.as_bytes())?;
        // Add all extra files in it
        for extra in &self.extra_files {
            fs::copy(extra, running_dir.join(extra))?;
        }
        // Create the config snapshot
        write_json_to_file(&self, running_dir.join(RUN_CONFIG_FILENAME))?;

        // Create the run start dump
        let model_canonical_path = self.model.canonicalize()?;
        let start_file = RunStartFile::new(&model_setup, &model_canonical_path);
        let start = start_file.start.clone();
        start_file.save(&running_dir)?;

        // 3. Generate parafile if parallel execution is enabled
        let parallel = &self.config.parallel;
        if parallel.enabled {
            let parafile_path = running_dir.join(format!("{}.pnm", model_setup.name));
            if let Some(existing_path) = parallel.parafile(&self.config_dir) {
                fs::copy(existing_path.canonicalize()?, parafile_path)?;
            } else {
                let mut f = fs::File::create(&parafile_path)?;
                f.write_all(parallel.generate_parafile().as_bytes())?;
            }
        }

        // 4. We generate the script to run nonmem
        let script = self.generate_script(&model_setup.name)?;
        let script_path = running_dir.join(format!("{}.sh", model_setup.name));
        let mut f = fs::File::create(&script_path)?;
        f.write_all(script.as_bytes())?;

        // 5. Setup file copying if running in a different directory
        let mut copier_coordinator = if running_dir != model_setup.output_dir {
            let mut c = run::files::FileCopyCoordinator::new(
                &self.config,
                &model_setup,
                &running_dir,
                &model_setup.output_dir,
            );
            c.start_background_copying()?;
            Some(c)
        } else {
            None
        };

        // 6. Execute the script with signal handling
        let script_start = Instant::now();

        let mut command = Command::new("sh");
        command.arg(script_path.file_name().unwrap());
        command.stdout(std::process::Stdio::inherit());
        command.stderr(std::process::Stdio::inherit());
        command.current_dir(&running_dir);

        let status = {
            #[cfg(unix)]
            {
                log::debug!("Starting script finished with signal handling");
                execute_with_termination_handling(command, &model_setup.output_dir)?
            }
            #[cfg(not(unix))]
            {
                log::debug!("Starting script finished without signal handling");
                // On non-Unix systems, just run normally
                let mut command = command.spawn()?;
                command.wait()?
            }
        };
        log::debug!(
            "Script finished with status {:?}",
            status.code().unwrap_or(0)
        );

        // 7. Stop background file copying and do final copy
        let files_copied = if let Some(ref mut copier) = copier_coordinator {
            copier.stop_and_finalize()?
        } else {
            HashSet::new()
        };

        let script_end = Instant::now();

        // Calculate Blake3 hashes for output files
        let output_files_hashes = calculate_output_file_hashes(
            &model_setup.output_dir,
            &model_setup.name,
            &model_setup.output_files,
        );

        // Create the run end dump
        let end_dump = RunEndFile {
            start,
            end: get_utc_now(),
            exit_code: status.code().unwrap_or_default(),
            runtime_ms: script_end.duration_since(script_start).as_millis(),
            files_copied,
            output_files_rewrites: model_setup.output_files.clone(),
            output_files_hashes,
        };
        end_dump.save(&model_setup.output_dir)?;

        // 8. Clean up unwanted files from output directory and update .gitignore
        if let Err(e) = run::files::cleanup_unwanted_files(
            &model_setup.output_dir,
            self.config.clean_level,
            &copier_coordinator
                .map(|x| x.copier.patterns)
                .unwrap_or_default(),
            &model_setup.name,
        ) {
            eprintln!("Error during cleanup: {e}");
        }

        let mut f = fs::File::create(model_setup.output_dir.join(".gitignore"))?;
        f.write_all(run::gitignore::get_final_gitignore(&model_setup.name).as_bytes())?;

        // 9. Execute the post run script if there is one
        if let Some(script_path) = post_run_script {
            post_run::execute_post_run_script(&script_path, end_dump.exit_code, &model_setup)?;
        }

        let exit_code = end_dump.exit_code;

        if !status.success() {
            log::warn!("NONMEM script execution failed with exit code: {exit_code}");
        }

        Ok(exit_code)
    }
}
