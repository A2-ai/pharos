//! Submitting the SCM driver to slurm.
//! [`ScmSlurmExecutor`]: crate::ScmSlurmExecutor

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use config::NonmemConfig;
use fs_err as fs;
use serde_json::json;

use crate::slurm;

pub(crate) const DRIVER_TEMPLATE: &str = r#"#!/bin/bash
#SBATCH --job-name="{{job_name}}"
#SBATCH --nodes=1
#SBATCH --ntasks=1
#SBATCH --cpus-per-task=1
#SBATCH --partition={{partition}}
{% if account -%}
#SBATCH --account={{account}}
{% endif -%}
#SBATCH --chdir={{config_dir}}
#SBATCH --output={{log_path}}

# The SCM driver: submits one slurm job per fit and waits for them. 
exec {{pharos_exe | shquote}} --config-file={{config_path | shquote}} {% if verbose %}--verbose {% endif %}scm run --plan {{plan_path | shquote}} --foreground {{run_flags | shquote}}
"#;

/// Everything needed to submit the driver for one SCM process.
pub struct ScmDriver {
    pub config_path: PathBuf,
    pub nonmem_config: NonmemConfig,
    pub pharos_exe: PathBuf,
    pub plan_path: PathBuf,
    pub model_stem: String,
    pub partition: Option<String>,
    pub account: Option<String>,
    pub run_flags: Vec<String>,
    pub verbose: bool,
}

/// A queued driver job.
pub struct SubmittedDriver {
    pub job_id: usize,
    pub job_name: String,
    pub log_path: PathBuf,
}

impl ScmDriver {
    pub fn job_name(&self) -> String {
        format!("scm_{}", self.model_stem)
    }

    fn config_dir(&self) -> &Path {
        self.config_path
            .parent()
            .expect("config file to have a parent dir")
    }

    /// The driver already queued or running for this process, if any
    pub fn running_driver(&self) -> Option<usize> {
        find_driver(
            &slurm::squeue("%i|%j|%Z")?,
            &self.job_name(),
            self.config_dir(),
        )
    }

    /// The driver's submission script, logging to `log_path`.
    fn render_script(&self, partition: &str, log_path: &Path) -> Result<String> {
        let context = tera::Context::from_serialize(&json!({
            "job_name": self.job_name(),
            "partition": partition,
            "account": self.account,
            "config_dir": self.config_dir(),
            "config_path": self.config_path,
            "log_path": log_path,
            "pharos_exe": self.pharos_exe,
            "plan_path": self.plan_path,
            "run_flags": self.run_flags,
            "verbose": self.verbose,
        }))
        .context("failed to build the SCM driver template context")?;
        slurm::TERA
            .render("scm_driver", &context)
            .context("failed to render the SCM driver script")
    }

    /// Write the driver's submission script and `sbatch` it.
    pub fn submit(&self) -> Result<SubmittedDriver> {
        let config_dir = self.config_dir();
        let job_name = self.job_name();
        let log_dir = crate::get_or_create_logs_dir(
            config_dir,
            self.nonmem_config.slurm.log_folder(config_dir),
            slurm::SLURM_LOGS_DIR,
        )?;
        let submission_dir = crate::get_or_create_submissions_dir(config_dir)?;
        let partition = slurm::resolve_partition(
            self.partition.as_deref(),
            self.nonmem_config.slurm.partition.as_deref(),
        )?;
        let script = self.render_script(&partition, &log_dir.join("%x_%j.out"))?;

        let script_path = submission_dir.join(format!("slurm_{job_name}_driver.sh"));
        fs::write(&script_path, &script)
            .with_context(|| format!("failed to write script to {script_path:?}"))?;

        log::debug!("Running sbatch for the SCM driver {script_path:?}");
        let output = Command::new("sbatch")
            .arg(&script_path)
            .output()
            .context("failed to execute sbatch for the SCM driver")?;
        if !output.status.success() {
            bail!("sbatch failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        let job_id = slurm::sbatch_job_id(&String::from_utf8_lossy(&output.stdout))?;

        Ok(SubmittedDriver {
            job_id,
            log_path: log_dir.join(format!("{job_name}_{job_id}.out")),
            job_name,
        })
    }
}

fn find_driver(squeue: &str, job_name: &str, config_dir: &Path) -> Option<usize> {
    squeue.lines().find_map(|line| {
        let mut fields = line.trim().split('|');
        let id = fields.next()?;
        let name = fields.next()?;
        let work_dir = fields.next()?;
        (name == job_name && Path::new(work_dir) == config_dir)
            .then(|| slurm::squeue_job_id(id))
            .flatten()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = "/proj/.slurm-logs/%x_%j.out";

    fn driver(account: Option<&str>, run_flags: &[&str]) -> ScmDriver {
        ScmDriver {
            config_path: PathBuf::from("/proj/pharos.toml"),
            nonmem_config: NonmemConfig::default(),
            pharos_exe: PathBuf::from("/opt/bin/pharos"),
            plan_path: PathBuf::from("/proj/model/scm/run001/plan.json"),
            model_stem: "run001".to_string(),
            partition: Some("cpu".to_string()),
            account: account.map(str::to_string),
            run_flags: run_flags.iter().map(|f| f.to_string()).collect(),
            verbose: false,
        }
    }

    #[test]
    fn driver_script_runs_scm_run_in_the_foreground() {
        let d = driver(None, &["--partition", "cpu", "--max-concurrent", "4"]);
        let script = d.render_script("cpu", Path::new(LOG)).unwrap();
        let expected = "\
#!/bin/bash
#SBATCH --job-name=\"scm_run001\"
#SBATCH --nodes=1
#SBATCH --ntasks=1
#SBATCH --cpus-per-task=1
#SBATCH --partition=cpu
#SBATCH --chdir=/proj
#SBATCH --output=/proj/.slurm-logs/%x_%j.out

# The SCM driver: submits one slurm job per fit and waits for them. 
exec /opt/bin/pharos --config-file=/proj/pharos.toml scm run --plan /proj/model/scm/run001/plan.json --foreground --partition cpu --max-concurrent 4
";
        assert_eq!(script, expected);
    }

    #[test]
    fn driver_script_passes_account_verbose_and_quotes_paths() {
        let mut d = driver(Some("a b"), &["--account", "a b"]);
        d.verbose = true;
        d.plan_path = PathBuf::from("/proj/my model/plan.json");
        let script = d.render_script("cpu", Path::new(LOG)).unwrap();
        assert!(script.contains("#SBATCH --account=a b\n"), "got:\n{script}");
        assert!(
            script.contains(
                "--verbose scm run --plan '/proj/my model/plan.json' --foreground --account 'a b'\n"
            ),
            "got:\n{script}"
        );
    }

    #[test]
    fn find_driver_matches_name_and_project_dir() {
        let squeue = "\
12|scm_run001|/other/proj
13|run001|/proj
14_2|scm_run001|/proj
15|scm_run001|/proj
";
        assert_eq!(
            find_driver(squeue, "scm_run001", Path::new("/proj")),
            Some(14)
        );
        assert_eq!(find_driver(squeue, "scm_run002", Path::new("/proj")), None);
        assert_eq!(find_driver("", "scm_run001", Path::new("/proj")), None);
    }
}
