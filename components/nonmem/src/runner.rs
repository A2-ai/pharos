use std::path::{Path, PathBuf};

use anyhow::Result;
use config::NonmemConfig;
use rayon::{ThreadPoolBuilder, prelude::*};
use serde::{Deserialize, Serialize};

use crate::NonmemRunner;

#[cfg(feature = "cli")]
use clap::Parser;

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
#[cfg_attr(feature = "cli", derive(Parser))]
pub struct RunOptions {
    /// Whether we will run nonmem in the output dir directly.
    /// By default, it will run in temp dir or /dev/shm on Linux instead to speed it up
    /// and this will be false.
    #[cfg_attr(feature = "cli", clap(long))]
    pub run_in_output_dir: bool,

    /// Whether to overwrite an existing output folder. Defaults to false
    /// If it's false and the output folder already exist, pharos will exit.
    #[cfg_attr(feature = "cli", clap(long))]
    pub overwrite: bool,

    /// A path to a script that will be ran by pharos after the nonmem run is over. It will
    /// not be run if pharos is killed in any way (directly or by cancelling a run on slurm/sge etc).
    /// The script will have its working directory set to the output folder
    #[cfg_attr(feature = "cli", clap(long))]
    pub post_run_script: Option<PathBuf>,

    /// If you're not using the default version from the config file, you can specify which
    /// one you want to use there.
    #[cfg_attr(feature = "cli", clap(long))]
    pub nonmem_version: Option<String>,

    /// Template for output directory name. Supports {{name}}, {{unix_timestamp}}, {{timestamp}}
    /// Overrides the output_dir setting in config file.
    /// If none are set, it will be output in a subfolder named after the model.
    #[cfg_attr(feature = "cli", clap(long))]
    pub output_dir: Option<String>,

    /// To set a different clean_level compared to the one in pharos.toml
    #[cfg_attr(feature = "cli", clap(long))]
    pub clean_level: Option<u8>,

    /// Whether to run nonmem in parallel using mpi. Defaults to false.
    #[cfg_attr(feature = "cli", clap(long))]
    pub parallel: bool,

    /// How many models to run in parallel. Defaults to the number of CPUs
    #[cfg_attr(feature = "cli", clap(long))]
    pub num_parallel: Option<usize>,

    /// Extra files to copy before the run starts. Only use it if the run errors because of the
    /// missing file, it should be able to find everything except files read from FORTRAN.
    /// Better to add it to the pharos.toml files_to_copy field when possible.
    #[cfg_attr(feature = "cli", clap(long))]
    pub extra_files: Vec<String>,

    /// Number of MPI CPUs to use for parallel execution for default parafile (overrides config).
    /// If you use your own parafile, it's a no-op
    #[cfg_attr(feature = "cli", clap(long))]
    pub num_mpi_cpus: Option<u8>,

    /// Timeout for the MPI default parafile (overrides config)
    /// If you use your own parafile, it's a no-op
    #[cfg_attr(feature = "cli", clap(long))]
    pub mpi_timeout: Option<usize>,

    /// Timeout for the MPI default parafile (overrides config)
    #[cfg_attr(feature = "cli", clap(long))]
    pub parafile: Option<PathBuf>,
}

impl RunOptions {
    /// Get back some of the flags that have been used to call via the CLI.
    /// The parallel stuff will update the config so we only need to get run specific flags
    pub fn run_flags(&self) -> Vec<String> {
        let mut out = Vec::new();

        if self.run_in_output_dir {
            out.push("--run-in-output-dir".to_string());
        }

        if self.overwrite {
            out.push("--overwrite".to_string());
        }

        if let Some(v) = self.nonmem_version.as_ref() {
            out.push("--nonmem-version".to_string());
            out.push(v.to_string());
        }

        if let Some(o) = self.output_dir.as_ref() {
            out.push("--output-dir".to_string());
            out.push(o.to_string());
        }

        if let Some(o) = self.clean_level.as_ref() {
            out.push("--clean-level".to_string());
            out.push(o.to_string());
        }

        if let Some(o) = self.post_run_script.as_ref() {
            out.push("--post-run-script".to_string());
            out.push(o.display().to_string());
        }

        out
    }

    pub fn update_config_from_options(&self, config: &mut NonmemConfig) {
        config.parallel.enabled = self.parallel;
        if let Some(cli_threads) = self.num_mpi_cpus {
            config.parallel.num_cpus = cli_threads;
            if cli_threads > 1 {
                config.parallel.enabled = true;
            }
        }
        if let Some(timeout) = self.mpi_timeout {
            config.parallel.timeout = timeout;
        }
        config.parallel.set_parafile(self.parafile.clone());
        if let Some(cl) = self.clean_level {
            config.clean_level = cl;
        }
    }
}

pub fn run_models(
    nonmem_config: &NonmemConfig,
    model_files: &[PathBuf],
    options: &RunOptions,
    config_dir: &Path,
) -> Result<()> {
    let max_threads = options.num_parallel.unwrap_or_else(num_cpus::get);
    let pool = ThreadPoolBuilder::new().num_threads(max_threads).build()?;

    if model_files.len() > 1 {
        println!(
            "Running {} model(s) using {} threads...",
            model_files.len(),
            max_threads
        );
    }

    pool.install(|| {
        model_files
            .par_iter()
            .map(|model_file| {
                let mut nonmem_config = nonmem_config.clone();
                options.update_config_from_options(&mut nonmem_config);
                let nonmem_version_clone = options.nonmem_version.clone();

                // Then we figure out the output dir based on cli flag + config
                let output_dir_final = options
                    .output_dir
                    .clone()
                    .or_else(|| nonmem_config.output_dir.clone());

                println!("Running model: {model_file:?}");
                let mut runner = NonmemRunner::new(
                    model_file,
                    options.overwrite,
                    nonmem_config,
                    nonmem_version_clone,
                    output_dir_final,
                    options.extra_files.clone(),
                    config_dir,
                );
                if options.run_in_output_dir {
                    runner.run_in_output_dir();
                }
                match runner.run() {
                    Ok(()) => {
                        println!("Model completed successfully: {model_file:?}");
                        Ok(())
                    }
                    Err(e) => {
                        eprintln!("Model failed: {model_file:?}: {e}");
                        Err(e)
                    }
                }
            })
            .collect::<Vec<_>>()
    });

    Ok(())
}
