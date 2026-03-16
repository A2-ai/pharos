use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[cfg(feature = "cli")]
use clap::Parser;
use config::NonmemConfig;

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

    /// A path to a script that will be run by pharos after the nonmem run is over. It will
    /// not be run if pharos is killed in any way (directly or by cancelling a run on slurm/sge etc).
    /// The script will be executed from the output folder and can be templated. It also
    /// receives the following environment variables:
    /// - PHAROS_NONMEM_EXIT_CODE: exit code of nonmem run
    /// - PHAROS_MODEL_DIR: directory where the original model is
    /// - PHAROS_MODEL_NAME: name of the model, without the extension
    /// - PHAROS_OUTPUT_DIR: the path to the output directory
    ///
    /// If the post-run script fails, the entire run will be considered failed.
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

    /// Custom MPI parafile (overrides config)
    #[cfg_attr(feature = "cli", clap(long))]
    pub parafile: Option<PathBuf>,

    /// Whether to enable logging for the run
    #[cfg_attr(feature = "cli", clap(skip))]
    pub verbose: bool,
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

        if self.verbose {
            out.push("--verbose".to_string());
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
            out.push(o.to_string_lossy().to_string());
        }

        if let Some(o) = self.parafile.as_ref() {
            out.push("--parafile".to_string());
            out.push(o.to_string_lossy().to_string());
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
        if let Some(p) = self.parafile.clone() {
            config.parallel.set_parafile(Some(p));
        }
        if let Some(cl) = self.clean_level {
            config.clean_level = cl;
        }
        if let Some(p) = self.post_run_script.clone() {
            config.set_post_run_script(Some(p));
        }
    }
}
