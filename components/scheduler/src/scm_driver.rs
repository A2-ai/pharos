//! Submitting the SCM driver to slurm.
//! [`ScmSlurmExecutor`]: crate::ScmSlurmExecutor

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use config::NonmemConfig;
use fs_err as fs;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::slurm;

pub const DRIVER_RECORD_FILENAME: &str = "scm_driver.json";

pub(crate) const DRIVER_TEMPLATE: &str = r#"#!/bin/bash
#SBATCH --job-name="{{job_name}}"
#SBATCH --nodes=1
{% if shared_node -%}
#SBATCH --exclusive
#SBATCH --mem=0
{% else -%}
#SBATCH --ntasks=1
#SBATCH --cpus-per-task=1
{% endif -%}
#SBATCH --partition={{partition}}
{% if account -%}
#SBATCH --account={{account}}
{% endif -%}
#SBATCH --chdir={{config_dir}}
#SBATCH --output={{log_path}}

{% if shared_node -%}
# The SCM driver on a whole node: runs every fit on this node, alongside itself.
{% else -%}
# The SCM driver: submits one slurm job per fit and waits for them.
{% endif -%}
exec {{pharos_exe | shquote}} --config-file={{config_path | shquote}} {% if verbose %}--verbose {% endif %}nonmem scm drive {{plan_path | shquote}} {{run_flags | shquote}}
"#;

/// Everything needed to submit the driver for one SCM process.
pub struct ScmDriver {
    pub config_path: PathBuf,
    pub nonmem_config: NonmemConfig,
    pub pharos_exe: PathBuf,
    pub plan_path: PathBuf,
    pub model_stem: String,
    /// The driver job's partition (`--driver-partition`). With `shared_node`
    /// the fits share the driver's node, so this is the fit partition.
    pub partition: Option<String>,
    pub account: Option<String>,
    /// Claim a whole node and run the fits on it instead of one job per fit
    pub shared_node: bool,
    /// Flags for `scm drive`
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
            "shared_node": self.shared_node,
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
        // A driver alone gets the cluster default, not the fits' partition: it
        // only waits. On a shared node it is where the fits run.
        let partition = if self.shared_node {
            slurm::resolve_partition(
                self.partition.as_deref(),
                self.nonmem_config.slurm.partition.as_deref(),
            )?
        } else {
            slurm::resolve_partition(self.partition.as_deref(), None)?
        };
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

/// Where an SCM process's driver runs, written to its out_dir when the driver
/// starts so `scm status` can tell a driver that is gone from one still working.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriverRecord {
    /// The submit command's shape: `slurm`, `slurm --shared-node`, `login`,
    /// `login --shared-node`
    pub mode: String,
    /// The driver's slurm job (`scm slurm submit`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<usize>,
    /// The driver process on the login node (`scm submit`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    /// The node allocation the fits run in (`scm submit --shared-node`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allocation: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log: Option<PathBuf>,
    pub started: String,
}

/// Whether the recorded driver is still running.
#[derive(Debug, Clone, PartialEq)]
pub enum Liveness {
    Alive,
    Gone,
    /// Cannot be checked from here, and why
    Unknown(String),
}

impl DriverRecord {
    /// A driver submitted as slurm job `job_id`.
    pub fn slurm(job_id: usize, shared_node: bool, log: PathBuf) -> Self {
        Self {
            mode: mode("slurm", shared_node),
            job_id: Some(job_id),
            host: None,
            pid: None,
            allocation: None,
            log: Some(log),
            started: utils::get_utc_now(),
        }
    }

    /// This process, driving from the login node.
    pub fn login(shared_node: bool, log: PathBuf) -> Self {
        Self {
            mode: mode("login", shared_node),
            job_id: None,
            host: Some(hostname()),
            pid: Some(std::process::id()),
            allocation: None,
            log: Some(log),
            started: utils::get_utc_now(),
        }
    }

    pub fn path(out_dir: &Path) -> PathBuf {
        out_dir.join(DRIVER_RECORD_FILENAME)
    }

    /// The record in `out_dir`, if a driver was ever started there.
    pub fn read(out_dir: &Path) -> Option<Self> {
        let text = fs::read_to_string(Self::path(out_dir)).ok()?;
        serde_json::from_str(&text)
            .inspect_err(|e| log::warn!("unreadable {DRIVER_RECORD_FILENAME}: {e}"))
            .ok()
    }

    pub fn save(&self, out_dir: &Path) -> Result<()> {
        fs::create_dir_all(out_dir)?;
        fs::write(Self::path(out_dir), serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn liveness(&self) -> Liveness {
        if let Some(job_id) = self.job_id {
            return match slurm::alive_jobs() {
                Some(alive) if alive.contains(&job_id) => Liveness::Alive,
                Some(_) => Liveness::Gone,
                None => Liveness::Unknown("squeue is not available".into()),
            };
        }
        let (Some(host), Some(pid)) = (&self.host, self.pid) else {
            return Liveness::Unknown("the record names no driver".into());
        };
        if *host != hostname() {
            return Liveness::Unknown(format!("the driver runs on {host}"));
        }
        let alive = Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        if alive {
            Liveness::Alive
        } else {
            Liveness::Gone
        }
    }

    /// How the driver is described to the user: `slurm job 12` / `pid 34 on login1`
    pub fn describe(&self) -> String {
        match (self.job_id, &self.host, self.pid) {
            (Some(id), _, _) => format!("slurm job {id}"),
            (None, Some(host), Some(pid)) => format!("pid {pid} on {host}"),
            _ => self.mode.clone(),
        }
    }
}

fn mode(base: &str, shared_node: bool) -> String {
    if shared_node {
        format!("{base} --shared-node")
    } else {
        base.to_string()
    }
}

fn hostname() -> String {
    Command::new("hostname")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
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
            shared_node: false,
            run_flags: run_flags.iter().map(|f| f.to_string()).collect(),
            verbose: false,
        }
    }

    #[test]
    fn driver_script_runs_scm_drive_on_one_cpu() {
        let d = driver(None, &["--partition", "big", "--max-concurrent", "4"]);
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
exec /opt/bin/pharos --config-file=/proj/pharos.toml nonmem scm drive /proj/model/scm/run001/plan.json --partition big --max-concurrent 4
";
        assert_eq!(script, expected);
    }

    #[test]
    fn shared_node_driver_claims_the_whole_node() {
        let mut d = driver(None, &["--shared-node"]);
        d.shared_node = true;
        let script = d.render_script("cpu8", Path::new(LOG)).unwrap();
        let expected = "\
#!/bin/bash
#SBATCH --job-name=\"scm_run001\"
#SBATCH --nodes=1
#SBATCH --exclusive
#SBATCH --mem=0
#SBATCH --partition=cpu8
#SBATCH --chdir=/proj
#SBATCH --output=/proj/.slurm-logs/%x_%j.out

# The SCM driver on a whole node: runs every fit on this node, alongside itself.
exec /opt/bin/pharos --config-file=/proj/pharos.toml nonmem scm drive /proj/model/scm/run001/plan.json --shared-node
";
        assert_eq!(script, expected);
    }

    #[test]
    fn driver_record_round_trips_and_names_the_driver() {
        let dir = tempfile::tempdir().unwrap();
        let record = DriverRecord::slurm(42, false, PathBuf::from("/proj/.slurm-logs/x.out"));
        record.save(dir.path()).unwrap();
        let read = DriverRecord::read(dir.path()).unwrap();
        assert_eq!(read, record);
        assert_eq!(read.describe(), "slurm job 42");

        let mut login = DriverRecord::login(true, PathBuf::from("driver.log"));
        assert_eq!(login.mode, "login --shared-node");
        assert_eq!(login.liveness(), Liveness::Alive, "this very process");
        login.host = Some("elsewhere-node".into());
        assert_eq!(
            login.liveness(),
            Liveness::Unknown("the driver runs on elsewhere-node".into())
        );
        assert!(DriverRecord::read(&dir.path().join("missing")).is_none());
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
                "--verbose nonmem scm drive '/proj/my model/plan.json' --account 'a b'\n"
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
