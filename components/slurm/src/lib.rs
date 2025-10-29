use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

use anyhow::{Context as AnyhowContext, Result, anyhow, bail};
use config::NonmemConfig;
use fs_err as fs;
use nonmem::RunOptions;
use tera::{Context, Tera};

#[cfg(feature = "cli")]
use clap::Parser;

mod partitions;

const DEFAULT_TEMPLATE: &str = r#"#!/bin/bash
#SBATCH --job-name="{{job_name}}"
#SBATCH --nodes=1
#SBATCH --ntasks=1
{% if parallel -%}#SBATCH --cpus-per-task={{num_mpi_cpus}}{% endif %}
#SBATCH --partition={{partition}}
{% if account -%}#SBATCH --account={{account}}{% endif %}
#SBATCH --output={{log_location}}

{% if parallel -%}
{{pharos_exe_path}} nonmem run {{model_path}} {{run_flags | join(sep=" ") }} --parallel --num-mpi-cpus {{num_mpi_cpus}}
{%- else -%}
{{pharos_exe_path}} nonmem run {{model_path}} {{run_flags | join(sep=" ") }}
{%- endif -%}
"#;

const SLURM_LOGS_DIR: &str = ".slurm-logs";
const SUBMISSIONS_DIR: &str = "submission-log";

fn get_or_create_slurm_logs_dir(
    parent: impl AsRef<Path>,
    slurm_logs_dir: Option<PathBuf>,
) -> Result<PathBuf> {
    let dir = if let Some(d) = slurm_logs_dir {
        d
    } else {
        parent.as_ref().join(SLURM_LOGS_DIR)
    };

    if dir.exists() {
        return Ok(dir);
    }

    fs::create_dir_all(&dir)?;
    let gitignore = dir.join(".gitignore");
    let mut f = fs::File::create(dir.join(&gitignore))?;
    f.write_all(b"*\n!.gitignore")?;

    Ok(dir)
}

fn get_or_create_submissions_dir(parent: impl AsRef<Path>) -> Result<PathBuf> {
    let dir = parent.as_ref().join(SUBMISSIONS_DIR);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

// Static Tera instance initialized automatically on first access
pub static TEMPLATE: LazyLock<Tera> = LazyLock::new(|| {
    let mut tera = Tera::default();
    tera.add_raw_template("slurm_job", DEFAULT_TEMPLATE)
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

pub fn submit(
    config_dir: &Path,
    models: Vec<PathBuf>,
    submit_options: SubmitOptions,
    run_options: RunOptions,
    mut config: NonmemConfig,
    pharos_exe_path: PathBuf,
) -> Result<Vec<(PathBuf, usize)>> {
    let partition = submit_options
        .partition
        .or_else(|| config.slurm.partition.clone());
    let partition_info = partitions::get_partitions_info()
        .with_context(|| "failed to retrieve SLURM partition information for job submission")?;

    let actual_partition = if let Some(p) = partition.or_else(|| config.slurm.partition.clone()) {
        if !partition_info.exists(&p) {
            bail!("Partition {p} does not exist in SLURM config");
        }
        p
    } else {
        partition_info.default_partition().partition.clone()
    };

    log::debug!("Will use SLURM partition '{actual_partition}'");

    run_options.update_config_from_options(&mut config);
    let num_cpus = run_options.num_mpi_cpus.unwrap_or_else(|| 1);

    let log_dir = get_or_create_slurm_logs_dir(config_dir, config.slurm.log_folder.clone())?;
    let submission_dir = get_or_create_submissions_dir(config_dir)?;

    let mut out = vec![];
    let run_flags = run_options.run_flags();
    for m in models {
        let m = m.canonicalize()?;
        let job_name = if let Some(job_name) = &submit_options.job_name {
            job_name.clone()
        } else {
            m.file_stem().unwrap().to_str().unwrap().to_string()
        };

        let mut context = Context::new();
        context.insert("job_name", &job_name);
        context.insert("account", &submit_options.account);
        context.insert("model_path", &m);
        context.insert("partition", &actual_partition);
        context.insert("num_mpi_cpus", &num_cpus);
        context.insert("pharos_exe_path", &pharos_exe_path);
        context.insert("parallel", &config.parallel.enabled);
        context.insert("run_flags", &run_flags);
        context.insert("log_location", log_dir.join("%x_%j.out").to_str().unwrap());

        let script = if let Some(tpl) = submit_options.template.as_ref() {
            let tpl_content = fs::read_to_string(tpl).with_context(|| {
                format!("failed to read SLURM template file '{}'", tpl.display())
            })?;
            Tera::one_off(&tpl_content, &context, false).with_context(|| {
                format!("failed to render custom SLURM template '{}'", tpl.display())
            })?
        } else {
            TEMPLATE
                .render("slurm_job", &context)
                .with_context(|| "failed to render built-in SLURM template with provided context")?
        };

        let script_path = submission_dir.join(&format!("{job_name}.sh"));
        fs::write(&script_path, &script)
            .with_context(|| format!("failed to write SLURM script to {script_path:?}",))?;

        // If it's a dry run, we print the script and stop here
        if submit_options.dry_run {
            println!("===");
            println!("Model: {m:?}");
            println!("Generated SLURM script:");
            println!("```");
            println!("{script}");
            println!("```");
            return Ok(vec![]);
        }

        log::debug!("Running sbatch for {m:?}");
        let output = Command::new("sbatch")
            .arg(script_path)
            .output()
            .with_context(|| format!("failed to execute sbatch command for model {m:?}",))?;

        if !output.status.success() {
            bail!("sbatch failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        log::debug!("Job for {m:?} successfully submitted");

        // Then get the slurm job id
        let stdout = String::from_utf8_lossy(&output.stdout);
        let num = stdout.trim().replace("Submitted batch job ", "");
        let job_id = num
            .parse()
            .map_err(|e| anyhow!("Failed to parse job ID '{stdout}': {e}"))?;

        out.push((m, job_id));
    }

    Ok(out)
}
