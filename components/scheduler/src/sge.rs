use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

use crate::{get_or_create_logs_dir, get_or_create_submissions_dir};
use anyhow::{Context as AnyhowContext, Result, anyhow, bail};
#[cfg(feature = "cli")]
use clap::Parser;
use config::NonmemConfig;
use fs_err as fs;
use nonmem::RunOptions;
use tera::{Context, Tera};

const DEFAULT_TEMPLATE: &str = r#"#!/bin/bash
#$ -N {{job_name}}
#$ -V
#$ -o {{log_path}}
#$ -e {{log_path}}
{% if parallel -%}#$ -pe orte {{num_mpi_cpus}}{% endif %}

{% if parallel -%}
{{pharos_exe_path}} nonmem run {{model_path}} {{run_flags | join(sep=" ") }} --parallel --num-mpi-cpus {{num_mpi_cpus}}
{%- else -%}
{{pharos_exe_path}} nonmem run {{model_path}} {{run_flags | join(sep=" ") }}
{%- endif -%}
"#;

const SGE_LOGS_DIR: &str = ".sge-logs";

pub static TEMPLATE: LazyLock<Tera> = LazyLock::new(|| {
    let mut tera = Tera::default();
    tera.add_raw_template("sge_job", DEFAULT_TEMPLATE)
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

pub fn submit(
    config_dir: &Path,
    models: Vec<PathBuf>,
    submit_options: SubmitOptions,
    run_options: RunOptions,
    mut config: NonmemConfig,
    pharos_exe_path: PathBuf,
) -> Result<Vec<(PathBuf, usize)>> {
    run_options.update_config_from_options(&mut config);
    let submission_dir = get_or_create_submissions_dir(config_dir)?;
    let log_dir = get_or_create_logs_dir(config_dir, config.sge.log_folder.clone(), SGE_LOGS_DIR)?;
    let num_cpus = run_options.num_mpi_cpus.unwrap_or_else(|| 1);
    let run_flags = run_options.run_flags();

    let mut out = vec![];
    for m in models {
        let m = m.canonicalize()?;
        let job_name = if let Some(job_name) = &submit_options.job_name {
            job_name.clone()
        } else {
            m.file_stem().unwrap().to_str().unwrap().to_string()
        };
        let mut context = Context::new();
        context.insert("job_name", &job_name);
        context.insert("model_path", &m);
        context.insert("num_mpi_cpus", &num_cpus);
        context.insert("pharos_exe_path", &pharos_exe_path);
        context.insert("parallel", &config.parallel.enabled);
        context.insert("run_flags", &run_flags);
        context.insert("log_path", &log_dir.join("test.log"));

        let script = if let Some(tpl) = submit_options
            .template
            .as_ref()
            .or_else(|| config.sge.template.as_ref())
        {
            let tpl_content = fs::read_to_string(tpl)
                .with_context(|| format!("failed to read SGE template file '{}'", tpl.display()))?;
            Tera::one_off(&tpl_content, &context, false).with_context(|| {
                format!("failed to render custom SGE template '{}'", tpl.display())
            })?
        } else {
            TEMPLATE
                .render("sge_job", &context)
                .with_context(|| "failed to render built-in SGE template with provided context")?
        };

        // If it's a dry run, we print the script and stop here
        if submit_options.dry_run {
            println!("===");
            println!("Model: {m:?}");
            println!("Generated SGE script:");
            println!("```");
            println!("{script}");
            println!("```");
            return Ok(vec![]);
        }

        let script_path = submission_dir.join(&format!("sge_{job_name}.sh"));
        fs::write(&script_path, &script)
            .with_context(|| format!("failed to write SGE script to {script_path:?}",))?;

        log::debug!("Running qsub for {m:?}");
        let output = Command::new("qsub")
            .arg(script_path)
            .output()
            .with_context(|| format!("failed to execute qsub command for model {m:?}",))?;

        if !output.status.success() {
            bail!("qsub failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        // Then get the sge job id
        let stdout = String::from_utf8_lossy(&output.stdout);
        let job_id = stdout
            .trim()
            .parse()
            .map_err(|e| anyhow!("Failed to parse job ID '{stdout}': {e}"))?;
        out.push((m, job_id));
    }
    Ok(out)
}
