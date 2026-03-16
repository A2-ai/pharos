use std::path::PathBuf;
use std::sync::LazyLock;

#[cfg(feature = "cli")]
use clap::Parser;
use tera::Tera;

const DEFAULT_TEMPLATE: &str = r#"#!/bin/bash
#$ -N {{job_name}}
#$ -V
#$ -j y
#$ -o {{log_path}}
{% if parallel -%}#$ -pe orte {{num_mpi_cpus}}{% endif %}

{% if parallel -%}
exec {{pharos_exe_path}} nonmem --config-file={{config_path}} run {{model_path}} {{run_flags | join(sep=" ") }} --parallel --num-mpi-cpus {{num_mpi_cpus}}
{%- else -%}
exec {{pharos_exe_path}} nonmem --config-file={{config_path}} run {{model_path}} {{run_flags | join(sep=" ") }}
{%- endif -%}
"#;

pub const SGE_LOGS_DIR: &str = ".sge-logs";

pub static TERA: LazyLock<Tera> = LazyLock::new(|| {
    let mut tera = Tera::default();
    tera.add_raw_template("job", DEFAULT_TEMPLATE)
        .expect("Failed to compile SGE template");
    tera
});

#[derive(Debug, Default, PartialEq)]
#[cfg_attr(feature = "cli", derive(Parser))]
pub struct SubmitOptions {
    /// The model to run
    /// It can be a path to .mod file or a pattern like `run[001:003].mod` where pharos will
    /// submit the models in parallel to sge
    pub model: String,
    /// The name of the job. Defaults to the model name
    #[cfg_attr(feature = "cli", clap(long))]
    pub job_name: Option<String>,
    /// The template to use. Defaults to a built-in template
    /// You can also set it in the pharos.toml config file
    #[cfg_attr(feature = "cli", clap(long))]
    pub template: Option<PathBuf>,
    /// Whether to actually submit the job or not.
    #[cfg_attr(feature = "cli", clap(long))]
    pub dry_run: bool,
}
