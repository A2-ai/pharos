use std::path::PathBuf;
use std::sync::LazyLock;

use tera::Tera;

#[cfg(feature = "cli")]
use clap::Parser;

pub(crate) mod partitions;
pub use partitions::get_partitions_info;

const DEFAULT_TEMPLATE: &str = r#"#!/bin/bash
#SBATCH --job-name="{{job_name}}"
#SBATCH --nodes=1
#SBATCH --ntasks=1
{% if parallel -%}#SBATCH --cpus-per-task={{num_mpi_cpus}}{% endif %}
#SBATCH --partition={{partition}}
{% if account -%}#SBATCH --account={{account}}{% endif %}
#SBATCH --output={{log_path}}

# Replace bash process with pharos directly - SLURM signals go directly to pharos
{% if parallel -%}
exec {{pharos_exe_path}} nonmem --config-file={{config_path}} run {{model_path}} {{run_flags | join(sep=" ") }} --parallel --num-mpi-cpus {{num_mpi_cpus}} --verbose
{%- else -%}
exec {{pharos_exe_path}} nonmem --config-file={{config_path}} run {{model_path}} {{run_flags | join(sep=" ") }} --verbose
{%- endif -%}
"#;

pub const SLURM_LOGS_DIR: &str = ".slurm-logs";

pub static TERA: LazyLock<Tera> = LazyLock::new(|| {
    let mut tera = Tera::default();
    tera.add_raw_template("job", DEFAULT_TEMPLATE)
        .expect("Failed to compile SLURM template");
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
    /// You can also set it in the pharos.toml config file
    #[cfg_attr(feature = "cli", clap(long))]
    pub template: Option<PathBuf>,
    /// Whether to actually submit the job or not.
    #[cfg_attr(feature = "cli", clap(long))]
    pub dry_run: bool,
}
