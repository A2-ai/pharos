use std::path::PathBuf;

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

    /// If you're not using the default version from the config file, you can specify which
    /// one you want to use there.
    #[cfg_attr(feature = "cli", clap(long))]
    pub nonmem_version: Option<String>,

    /// Template for output directory name. Supports {{name}}, {{unix_timestamp}}, {{timestamp}}
    /// Overrides the output_dir setting in config file.
    /// If none are set, it will be output in a subfolder named after the model.
    #[cfg_attr(feature = "cli", clap(long))]
    pub output_dir: Option<String>,

    /// How many models to run in parallel. Defaults to the number of CPUs
    #[cfg_attr(feature = "cli", clap(long))]
    pub num_parallel: Option<usize>,

    /// To set a different clean_level compared to the one in voodoo.toml
    #[cfg_attr(feature = "cli", clap(long))]
    pub clean_level: Option<u8>,

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

pub fn run_models(
    nonmem_config: &NonmemConfig,
    model_files: &[PathBuf],
    options: &RunOptions,
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
                let nonmem_version_clone = options.nonmem_version.clone();

                if let Some(cli_threads) = options.num_mpi_cpus {
                    nonmem_config.parallel.num_cpus = cli_threads;
                }
                if let Some(timeout) = options.mpi_timeout {
                    nonmem_config.parallel.timeout = timeout;
                }
                nonmem_config.parallel.parafile = options.parafile.clone();
                if let Some(cl) = options.clean_level {
                    nonmem_config.clean_level = cl;
                }

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
