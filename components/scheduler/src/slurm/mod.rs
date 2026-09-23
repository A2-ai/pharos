use std::path::PathBuf;
use std::process::Command;
use std::sync::LazyLock;

use anyhow::{Result, anyhow};
use tera::Tera;

#[cfg(feature = "cli")]
use clap::Parser;

pub(crate) mod partitions;
pub use partitions::{PartitionInfo, get_partitions_info, resolve_partition};

const DEFAULT_TEMPLATE: &str = r#"#!/bin/bash
#SBATCH --job-name="{{job_name}}"
#SBATCH --nodes=1
{% if parallel -%}
#SBATCH --ntasks={{num_mpi_cpus}}
#SBATCH --cpus-per-task=1
{% else -%}
#SBATCH --ntasks=1
#SBATCH --cpus-per-task=1
{% endif %}#SBATCH --partition={{partition}}
{% if account -%}#SBATCH --account={{account}}{% endif %}
#SBATCH --output={{log_path}}

# Replace bash process with pharos directly - SLURM signals go directly to pharos
{% if parallel -%}
exec {{pharos_exe_path | shquote}} nonmem --config-file={{config_path | shquote}} run {{model_path | shquote}} {{run_flags | shquote}} --parallel --num-mpi-cpus {{num_mpi_cpus}}
{%- else -%}
exec {{pharos_exe_path | shquote}} nonmem --config-file={{config_path | shquote}} run {{model_path | shquote}} {{run_flags | shquote}}
{%- endif -%}
"#;

pub const SLURM_LOGS_DIR: &str = ".slurm-logs";

/// Resource variables slurm sets in a job's environment that a nested `sbatch`
/// would read back as defaults for the job it submits.
const INHERITED_SLURM_VARS: &[&str] = &[
    "SLURM_CPUS_PER_TASK",
    "SLURM_DISTRIBUTION",
    "SLURM_JOB_NUM_NODES",
    "SLURM_MEM_PER_CPU",
    "SLURM_MEM_PER_GPU",
    "SLURM_MEM_PER_NODE",
    "SLURM_NNODES",
    "SLURM_NPROCS",
    "SLURM_NTASKS",
    "SLURM_NTASKS_PER_NODE",
];

/// Keep a job submitted from inside another job from inheriting its resources.
pub(crate) fn strip_inherited_slurm_env(command: &mut Command) {
    for var in INHERITED_SLURM_VARS {
        command.env_remove(var);
    }
}

/// The job id in sbatch's `Submitted batch job <id>` output.
pub(crate) fn sbatch_job_id(stdout: &str) -> Result<usize> {
    let num = stdout.trim().replace("Submitted batch job ", "");
    num.parse()
        .map_err(|e| anyhow!("Failed to parse job ID '{stdout}': {e}"))
}

/// `squeue -h -o <format>`, or `None` (logged) when squeue is missing or fails.
pub(crate) fn squeue(format: &str) -> Option<String> {
    let output = Command::new("squeue")
        .args(["-h", "-o", format])
        .output()
        .ok()?;
    if !output.status.success() {
        log::debug!("squeue failed ({}); skipping this check", output.status);
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The base job id of an squeue `%i` field: array tasks print as `123_4`.
pub(crate) fn squeue_job_id(field: &str) -> Option<usize> {
    field.trim().split(['_', '.']).next()?.parse().ok()
}

pub static TERA: LazyLock<Tera> = LazyLock::new(|| {
    let mut tera = Tera::default();
    tera.register_filter("shquote", crate::shquote_filter);
    tera.add_raw_template("job", DEFAULT_TEMPLATE)
        .expect("Failed to compile SLURM template");
    tera.add_raw_template("scm_driver", crate::scm_driver::DRIVER_TEMPLATE)
        .expect("Failed to compile the SCM driver template");
    tera
});

#[derive(Debug, Default, PartialEq)]
#[cfg_attr(feature = "cli", derive(Parser))]
pub struct SubmitOptions {
    /// The model to run
    /// It can be a path to .mod file or a pattern like `run[001:003].mod` where pharos will
    /// submit the models in parallel to slurm
    pub model: String,
    /// The name of the job. Defaults to the model name
    #[cfg_attr(feature = "cli", clap(long))]
    pub job_name: Option<String>,
    /// The partition to use. Defaults to the default partition.
    /// You can also set it in the pharos.toml config file
    #[cfg_attr(feature = "cli", clap(long))]
    pub partition: Option<String>,
    #[cfg_attr(feature = "cli", clap(long))]
    pub account: Option<String>,
    /// The template to use. Defaults to a built-in template
    /// You can also set it in the pharos.toml config file.
    /// The `shquote` filter is available to shell-quote interpolated values,
    /// e.g. `{{ model_path | shquote }}` and `{{ run_flags | shquote }}`.
    #[cfg_attr(feature = "cli", clap(long))]
    pub template: Option<PathBuf>,
    /// Whether to actually submit the job or not.
    #[cfg_attr(feature = "cli", clap(long))]
    pub dry_run: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sbatch_job_id_cases() {
        assert_eq!(sbatch_job_id("Submitted batch job 4242\n").unwrap(), 4242);
        assert_eq!(sbatch_job_id("4242\n").unwrap(), 4242);
        assert!(sbatch_job_id("sbatch: error: invalid partition\n").is_err());
    }
}
