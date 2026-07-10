use std::path::PathBuf;
use std::sync::LazyLock;

use anyhow::{Result, anyhow};
#[cfg(feature = "cli")]
use clap::Parser;
use tera::Tera;

/// Parse a job ID from qsub output.
///
/// SGE output varies by version and job state:
/// - Some clusters put a bare job ID number in stdout
/// - Others put "Your job <N> ("<name>") has been submitted" in stdout or stderr
pub fn parse_job_id(stdout: &str, stderr: &str) -> Result<usize> {
    let combined = format!("{}\n{}", stdout, stderr);

    let job_id_str = combined
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            if let Ok(id) = trimmed.parse::<usize>() {
                return Some(id.to_string());
            }
            trimmed
                .strip_prefix("Your job ")
                .and_then(|rest| rest.split_whitespace().next())
                .map(|s| s.to_string())
        })
        .ok_or_else(|| {
            anyhow!("Failed to find job ID in SGE output.\nstdout: {stdout}\nstderr: {stderr}")
        })?;

    job_id_str
        .parse()
        .map_err(|e| anyhow!("Failed to parse job ID '{job_id_str}': {e}"))
}

const DEFAULT_TEMPLATE: &str = r#"#!/bin/bash
#$ -N {{job_name}}
#$ -cwd
#$ -V
#$ -j y
#$ -o {{log_path}}
{% if parallel -%}#$ -pe orte {{num_mpi_cpus}}{% endif %}

{% if parallel -%}
exec {{pharos_exe_path | shquote}} nonmem --config-file={{config_path | shquote}} run {{model_path | shquote}} {{run_flags | shquote}} --parallel --num-mpi-cpus {{num_mpi_cpus}}
{%- else -%}
exec {{pharos_exe_path | shquote}} nonmem --config-file={{config_path | shquote}} run {{model_path | shquote}} {{run_flags | shquote}}
{%- endif -%}
"#;

pub const SGE_LOGS_DIR: &str = ".sge-logs";

pub static TERA: LazyLock<Tera> = LazyLock::new(|| {
    let mut tera = Tera::default();
    tera.register_filter("shquote", crate::shquote_filter);
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
    fn parse_job_id_cases() {
        let cases: Vec<(&str, &str, &str, Option<usize>)> = vec![
            ("bare number in stdout", "1391\n", "", Some(1391)),
            ("bare number in stderr", "", "1391\n", Some(1391)),
            (
                "full message in stdout",
                "Your job 3 (\"run002\") has been submitted\n",
                "",
                Some(3),
            ),
            (
                "full message in stderr",
                "",
                "Your job 42 (\"run001\") has been submitted\n",
                Some(42),
            ),
            (
                "queuing warning with job in stderr",
                "",
                "Unable to run job: warning: user's job is not allowed to run in any queue\n\
                 Your job 7 (\"run003\") has been submitted\nExiting.\n",
                Some(7),
            ),
            ("empty output", "", "", None),
            ("no job id", "some garbage", "more garbage", None),
        ];

        for (name, stdout, stderr, expected) in cases {
            let result = parse_job_id(stdout, stderr);
            match expected {
                Some(id) => assert_eq!(result.unwrap(), id, "case: {name}"),
                None => assert!(result.is_err(), "case: {name} should have failed"),
            }
        }
    }
}
