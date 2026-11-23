//! Submitting the SCM driver to slurm.
//! [`ScmSlurmExecutor`]: crate::ScmSlurmExecutor

use std::collections::HashSet;
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
# Never run twice: a requeued driver would start over with the flags it was
# submitted with, `--overwrite` among them, and discard the rounds it fitted.
#SBATCH --no-requeue

{% if shared_node -%}
# The SCM driver on a whole node: runs every fit on this node, alongside itself.
{% else -%}
# The SCM driver: submits one slurm job per fit and waits for them.
{% endif -%}
exec {{pharos_exe | shquote}} --config-file={{config_path | shquote}} {% if verbose %}--verbose {% endif %}nonmem scm run {{scm_config | shquote}} {{run_flags | shquote}}
"#;

/// Everything needed to submit the driver for one SCM process.
pub struct ScmDriver {
    pub config_path: PathBuf,
    pub nonmem_config: NonmemConfig,
    pub pharos_exe: PathBuf,
    /// The SCM config (`<stem>scm.toml`) the driver resolves and runs
    pub scm_config: PathBuf,
    pub model_stem: String,
    /// The driver job's partition (`--driver-partition`). With `shared_node`
    /// the fits share the driver's node, so this is the fit partition.
    pub partition: Option<String>,
    pub account: Option<String>,
    /// Claim a whole node and run the fits on it instead of one job per fit
    pub shared_node: bool,
    /// Flags for the `scm run` the job starts
    pub run_flags: Vec<String>,
    pub verbose: bool,
}

/// A queued driver job, held until [`SubmittedDriver::release`].
pub struct SubmittedDriver {
    pub job_id: usize,
    pub job_name: String,
    pub log_path: PathBuf,
}

impl SubmittedDriver {
    /// Let the held job start. Called once the driver record naming it is
    /// written, so the `scm run` it starts finds that record.
    pub fn release(&self) -> Result<()> {
        slurm::scontrol_release(self.job_id)
    }

    /// Take the job back out of the queue (it was never released).
    pub fn cancel(&self) -> Result<()> {
        slurm::scancel(self.job_id)
    }
}

/// The driver job's name for the model `stem`.
pub fn driver_job_name(stem: &str) -> String {
    format!("scm_{stem}")
}

impl ScmDriver {
    pub fn job_name(&self) -> String {
        driver_job_name(&self.model_stem)
    }

    fn config_dir(&self) -> &Path {
        self.config_path
            .parent()
            .expect("config file to have a parent dir")
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
            "scm_config": self.scm_config,
            "run_flags": self.run_flags,
            "shared_node": self.shared_node,
            "verbose": self.verbose,
        }))
        .context("failed to build the SCM driver template context")?;
        slurm::TERA
            .render("scm_driver", &context)
            .context("failed to render the SCM driver script")
    }

    /// Write the driver's submission script and `sbatch --hold` it: the job
    /// waits in the queue until [`SubmittedDriver::release`], so the record
    /// naming it is on disk before the `scm run` inside it looks.
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
            .arg("--hold")
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
    /// The driver process on the login node (`scm run` at a terminal)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    /// When the process `pid` started, as its host tells it
    /// ([`process_start`]). A pid is reused once its process is gone, so
    /// the driver is the process with this pid *and* this start: a later
    /// process that happens to get the pid is not taken for it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid_started: Option<String>,
    /// The node allocation the fits run in (`scm run --shared-node` on the
    /// login node)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allocation: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log: Option<PathBuf>,
    pub started: String,
    /// When the driver ended, once it has and could say so (a killed one
    /// cannot). A login driver's record that says so is never taken for a
    /// running driver, whatever process has its pid by then; a slurm
    /// driver is told by its job, which outlives the end recorded here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended: Option<String>,
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
            pid_started: None,
            allocation: None,
            log: Some(log),
            started: utils::get_utc_now(),
            ended: None,
        }
    }

    /// This process, driving from the login node.
    pub fn login(shared_node: bool, log: PathBuf) -> Self {
        Self {
            mode: mode("login", shared_node),
            job_id: None,
            host: Some(hostname()),
            pid: Some(std::process::id()),
            pid_started: process_start(std::process::id()).ok().flatten(),
            allocation: None,
            log: Some(log),
            started: utils::get_utc_now(),
            ended: None,
        }
    }

    pub fn path(out_dir: &Path) -> PathBuf {
        out_dir.join(DRIVER_RECORD_FILENAME)
    }

    /// Whether this process runs inside the driver job the record names:
    /// `scm run` is then the driver `scm slurm submit` queued, not a new one.
    pub fn is_this_job(&self) -> bool {
        let here = std::env::var("SLURM_JOB_ID")
            .ok()
            .and_then(|id| id.parse::<usize>().ok());
        self.job_id.is_some() && self.job_id == here
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
        utils::write_json_to_file(self, Self::path(out_dir))
    }

    /// Record that this driver has ended, however it did.
    pub fn mark_ended(&mut self, out_dir: &Path) -> Result<()> {
        self.ended = Some(utils::get_utc_now());
        self.save(out_dir)
    }

    pub fn liveness(&self) -> Liveness {
        self.liveness_in(slurm::alive_jobs)
    }

    /// [`Self::liveness`], with `alive_jobs` the slurm queue's job ids.
    ///
    /// A slurm driver is alive while its job is in the queue, whatever its
    /// record says: the job outlives the `scm run` inside it, and holds
    /// whatever that run left behind (a shared node's fits are processes
    /// in the job, with no job of their own to cancel) until slurm has
    /// cleaned it up and let it go. Only a login driver's record is taken
    /// at its word when it says the driver ended: its pid is all there is
    /// to go on, and may have gone to another process since.
    fn liveness_in(&self, alive_jobs: impl FnOnce() -> Option<HashSet<usize>>) -> Liveness {
        if let Some(job_id) = self.job_id {
            return match alive_jobs() {
                Some(alive) if alive.contains(&job_id) => Liveness::Alive,
                Some(_) => Liveness::Gone,
                None => Liveness::Unknown("squeue is not available".into()),
            };
        }
        if self.ended.is_some() {
            return Liveness::Gone;
        }
        let (Some(host), Some(pid)) = (&self.host, self.pid) else {
            return Liveness::Unknown("the record names no driver".into());
        };
        if *host != hostname() {
            return Liveness::Unknown(format!("the driver runs on {host}"));
        }
        // The driver is the process with the recorded pid and the recorded
        // start. A driver that was killed left no end on its record, and
        // its pid may since have gone to a process that is not it.
        if let Some(started) = &self.pid_started {
            return match process_start(pid) {
                Ok(Some(now)) if now == *started => Liveness::Alive,
                // not there, or another process that has the pid now
                Ok(_) => Liveness::Gone,
                // a lookup that failed is not a driver that is gone
                Err(why) => Liveness::Unknown(format!("pid {pid} could not be looked up: {why}")),
            };
        }
        // A record with no start (this host could not tell it): the pid
        // alone has to do.
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

    /// Tell the driver to stop: `scancel` its slurm job, or SIGTERM its
    /// login-node process (which has to be on this host). The driver catches
    /// the signal, records the SCM process as paused and exits once its
    /// executor hands the running fits back; fits that are slurm jobs of
    /// their own keep running. Says what was done.
    pub fn stop(&self) -> Result<String> {
        match self.liveness() {
            Liveness::Gone => Ok(format!("{} has already exited", self.describe())),
            Liveness::Unknown(why) => bail!("cannot stop {}: {why}", self.describe()),
            Liveness::Alive => {
                match (self.job_id, self.pid) {
                    (Some(job_id), _) => slurm::scancel(job_id)?,
                    (None, Some(pid)) => {
                        let status = Command::new("kill")
                            .args(["-TERM", &pid.to_string()])
                            .status()
                            .context("failed to execute kill")?;
                        if !status.success() {
                            bail!("kill -TERM {pid} failed");
                        }
                    }
                    (None, None) => bail!("the record names no driver to stop"),
                }
                Ok(format!(
                    "{} told to stop; it records the SCM process as paused and exits once its \
                     running fits are handed back",
                    self.describe()
                ))
            }
        }
    }

    /// Release the node allocation the record names, once its driver is
    /// gone and the allocation is still held (a driver releases it itself
    /// as it stops). What became of it, when there is one.
    pub fn release_allocation(&self) -> Result<Option<String>> {
        let Some(job_id) = self.allocation else {
            return Ok(None);
        };
        let what = match (self.liveness(), slurm::job_in_queue(job_id)) {
            (Liveness::Alive, _) => {
                format!(
                    "slurm allocation {job_id} is held by the driver, which releases it as it stops"
                )
            }
            (Liveness::Unknown(why), _) => format!(
                "slurm allocation {job_id} is left alone: cannot tell whether its driver still runs ({why})"
            ),
            (Liveness::Gone, Some(true)) => {
                slurm::scancel(job_id)?;
                format!("released slurm allocation {job_id}")
            }
            (Liveness::Gone, Some(false)) => format!("slurm allocation {job_id} is no longer held"),
            (Liveness::Gone, None) => format!(
                "cannot tell whether slurm allocation {job_id} is still held: squeue is not available"
            ),
        };
        Ok(Some(what))
    }

    /// Refuse to discard the SCM process this driver drives while it may
    /// still be running: its fits would land in an out_dir that no longer
    /// records them. `out_dir` is the directory as the caller shows it,
    /// `stop` how its user stops the process (`` `pharos nonmem scm stop
    /// <out_dir>` ``) and `overwrite` the option that discards it
    /// (`--overwrite`).
    pub fn refuse_if_alive(&self, out_dir: &str, stop: &str, overwrite: &str) -> Result<()> {
        match self.liveness() {
            Liveness::Gone => Ok(()),
            Liveness::Alive => bail!(
                "the SCM process in {out_dir} is still running; stop it with {stop} \
                 before discarding it with {overwrite}"
            ),
            Liveness::Unknown(why) => bail!(
                "cannot tell whether the SCM process in {out_dir} is still running: {why}; \
                 once it has stopped, delete {out_dir}/{DRIVER_RECORD_FILENAME} and submit \
                 again with {overwrite}"
            ),
        }
    }

    /// Refuse to start a driver while this one is queued or running, or
    /// may be. `out_dir` is the directory as the caller shows it, `status`
    /// and `stop` how its user follows and stops the process.
    pub fn refuse_second_driver(&self, out_dir: &str, status: &str, stop: &str) -> Result<()> {
        match self.liveness() {
            Liveness::Gone => Ok(()),
            Liveness::Alive => bail!(
                "the SCM process in {out_dir} is already running ({}); follow it with {status}, \
                 or stop it with {stop} before resubmitting",
                self.describe()
            ),
            Liveness::Unknown(why) => bail!(
                "cannot tell whether the SCM process in {out_dir} is still running ({}): {why}; \
                 once it has stopped, delete {out_dir}/{DRIVER_RECORD_FILENAME} and submit again",
                self.describe()
            ),
        }
    }

    /// What `scm status` says about this driver: whether it is still there
    /// and, when it is gone while the process says it is running, where to
    /// look. `shown` is how the caller shows a path.
    pub fn status_lines(&self, scm_status: &str, shown: &dyn Fn(&Path) -> String) -> Vec<String> {
        let mut lines = Vec::new();
        let log = self
            .log
            .as_ref()
            .map(|l| format!("; see {}", shown(l)))
            .unwrap_or_default();
        let liveness = self.liveness();
        match &liveness {
            Liveness::Alive => lines.push(format!("driver       : running ({})", self.describe())),
            Liveness::Gone if scm_status == "running" || scm_status == "planned" => {
                lines.push(format!(
                    "driver       : NOT RUNNING ({}, {}) — the SCM process stopped before finishing{log}",
                    self.describe(),
                    self.mode
                ));
                lines.push(
                    "               submit its config again to resume; fits still in the queue are waited for, not rerun"
                        .to_string(),
                );
            }
            Liveness::Gone => {
                lines.push(format!("driver       : exited ({}){log}", self.describe()))
            }
            Liveness::Unknown(why) => lines.push(format!(
                "driver       : {} — cannot check it from here ({why})",
                self.describe()
            )),
        }
        if let Some(allocation) = self.allocation
            && liveness != Liveness::Alive
            && slurm::job_is_queued(allocation)
        {
            lines.push(format!(
                "allocation   : slurm allocation {allocation} is still held; release it with `scancel {allocation}`"
            ));
        }
        lines
    }
}

fn mode(base: &str, shared_node: bool) -> String {
    if shared_node {
        format!("{base} --shared-node")
    } else {
        base.to_string()
    }
}

/// When the process `pid` started, in a form that is the same every time
/// it is asked for as long as that process lives, and differs for any other
/// process that later gets the pid. `Ok(None)` is a process known not to
/// exist; `Err` is a lookup that failed, which says nothing either way and
/// must never be taken for the process being gone.
///
/// On Linux: the boot's id and the start in clock ticks since that boot,
/// from `/proc`, which is exact. Elsewhere: what `ps` prints as the start,
/// under a fixed locale and time zone so that two shells agree on it.
fn process_start(pid: u32) -> Result<Option<String>, String> {
    // Linux, where /proc lists every process: one that is not there is gone
    if Path::new("/proc/self/stat").exists() {
        return match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
            Ok(stat) => {
                let boot = std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
                    .map(|id| id.trim().to_string())
                    .unwrap_or_default();
                proc_stat_start(&stat, &boot)
                    .map(Some)
                    .ok_or_else(|| format!("/proc/{pid}/stat is not in the expected form"))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("could not read /proc/{pid}/stat: {e}")),
        };
    }
    let output = Command::new("ps")
        .args(["-o", "lstart=", "-p", &pid.to_string()])
        .env("LC_ALL", "C")
        .env("TZ", "UTC")
        .output()
        .map_err(|e| format!("could not run ps: {e}"))?;
    ps_start(
        output.status.success(),
        &String::from_utf8_lossy(&output.stdout),
        &String::from_utf8_lossy(&output.stderr),
    )
}

/// The start in a `/proc/<pid>/stat` line, with the boot it counts from.
/// The command name, in parentheses, may itself hold spaces and
/// parentheses: the fields are counted from after its last `)`.
/// `starttime` is field 22, the 20th after the name.
fn proc_stat_start(stat: &str, boot: &str) -> Option<String> {
    let ticks = stat.rsplit_once(')')?.1.split_whitespace().nth(19)?;
    Some(format!("{boot} {ticks}").trim().to_string())
}

/// What `ps -o lstart= -p <pid>` said. It prints the start and succeeds for
/// a process that exists. For one that does not it fails and prints nothing
/// at all, and only that is taken as "no such process": a failure that
/// says something, or a success that prints nothing, is a lookup that did
/// not work.
fn ps_start(succeeded: bool, stdout: &str, stderr: &str) -> Result<Option<String>, String> {
    let (start, complaint) = (stdout.trim(), stderr.trim());
    match (succeeded, start.is_empty(), complaint.is_empty()) {
        (true, false, _) => Ok(Some(start.to_string())),
        (false, true, true) => Ok(None),
        (_, _, false) => Err(format!("ps failed: {complaint}")),
        _ => Err("ps gave no start time".to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = "/proj/.slurm-logs/%x_%j.out";

    fn driver(account: Option<&str>, run_flags: &[&str]) -> ScmDriver {
        ScmDriver {
            config_path: PathBuf::from("/proj/pharos.toml"),
            nonmem_config: NonmemConfig::default(),
            pharos_exe: PathBuf::from("/opt/bin/pharos"),
            scm_config: PathBuf::from("/proj/model/scm/run001scm.toml"),
            model_stem: "run001".to_string(),
            partition: Some("cpu".to_string()),
            account: account.map(str::to_string),
            shared_node: false,
            run_flags: run_flags.iter().map(|f| f.to_string()).collect(),
            verbose: false,
        }
    }

    #[test]
    fn driver_script_runs_scm_run_on_one_cpu() {
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
# Never run twice: a requeued driver would start over with the flags it was
# submitted with, `--overwrite` among them, and discard the rounds it fitted.
#SBATCH --no-requeue

# The SCM driver: submits one slurm job per fit and waits for them.
exec /opt/bin/pharos --config-file=/proj/pharos.toml nonmem scm run /proj/model/scm/run001scm.toml --partition big --max-concurrent 4
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
# Never run twice: a requeued driver would start over with the flags it was
# submitted with, `--overwrite` among them, and discard the rounds it fitted.
#SBATCH --no-requeue

# The SCM driver on a whole node: runs every fit on this node, alongside itself.
exec /opt/bin/pharos --config-file=/proj/pharos.toml nonmem scm run /proj/model/scm/run001scm.toml --shared-node
";
        assert_eq!(script, expected);
    }

    /// `liveness` asks `/proc`, `ps` or `kill -0`, which only exist where
    /// the driver runs.
    #[cfg(unix)]
    #[test]
    fn driver_record_round_trips_and_names_the_driver() {
        let dir = tempfile::tempdir().unwrap();
        let record = DriverRecord::slurm(42, false, PathBuf::from("/proj/.slurm-logs/x.out"));
        record.save(dir.path()).unwrap();
        let read = DriverRecord::read(dir.path()).unwrap();
        assert_eq!(read, record);
        assert_eq!(read.describe(), "slurm job 42");
        // SAFETY: the test's own process environment, read by nothing else here
        unsafe { std::env::set_var("SLURM_JOB_ID", "42") };
        assert!(read.is_this_job(), "the record names the job this runs in");
        unsafe { std::env::set_var("SLURM_JOB_ID", "43") };
        assert!(!read.is_this_job());
        unsafe { std::env::remove_var("SLURM_JOB_ID") };
        assert!(!read.is_this_job());

        let mut login = DriverRecord::login(true, PathBuf::from("driver.log"));
        assert_eq!(login.mode, "login --shared-node");
        assert_eq!(login.liveness(), Liveness::Alive, "this very process");
        // The pid alone is not the driver: a process that got the pid after
        // the driver was killed started at another time, and is never
        // signalled as if it were the driver.
        assert_eq!(
            login.pid_started,
            process_start(std::process::id()).unwrap()
        );
        assert!(login.pid_started.is_some());
        let started = login.pid_started.replace("some earlier start".into());
        assert_eq!(login.liveness(), Liveness::Gone, "the pid was reused");
        assert!(login.stop().unwrap().ends_with("has already exited"));
        login.pid_started = started;
        assert_eq!(login.liveness(), Liveness::Alive);
        login.host = Some("elsewhere-node".into());
        assert_eq!(
            login.liveness(),
            Liveness::Unknown("the driver runs on elsewhere-node".into())
        );
        assert!(login.stop().is_err());
        // A driver that has exited is not stopped again
        login.host = Some(hostname());
        // The pid of a process that has just exited: one that exists nowhere
        let mut exited = Command::new("true").spawn().unwrap();
        login.pid = Some(exited.id());
        exited.wait().unwrap();
        assert_eq!(login.liveness(), Liveness::Gone);
        assert!(login.stop().unwrap().ends_with("has already exited"));
        assert_eq!(login.release_allocation().unwrap(), None);
        assert!(DriverRecord::read(&dir.path().join("missing")).is_none());
    }

    #[test]
    fn driver_script_passes_account_verbose_and_quotes_paths() {
        let mut d = driver(Some("a b"), &["--account", "a b"]);
        d.verbose = true;
        d.scm_config = PathBuf::from("/proj/my model/my modelscm.toml");
        let script = d.render_script("cpu", Path::new(LOG)).unwrap();
        assert!(script.contains("#SBATCH --account=a b\n"), "got:\n{script}");
        assert!(
            script.contains(
                "--verbose nonmem scm run '/proj/my model/my modelscm.toml' --account 'a b'\n"
            ),
            "got:\n{script}"
        );
    }

    /// One driver per process: a second is refused while the first runs,
    /// and while nothing here can tell whether it does. Only a driver known
    /// to be gone makes way.
    #[cfg(unix)]
    #[test]
    fn a_second_driver_is_refused_unless_the_first_is_known_gone() {
        let refusal = |record: &DriverRecord| {
            record
                .refuse_second_driver("scm/run001", "`status`", "`stop`")
                .map_err(|e| format!("{e:#}"))
        };
        let mut login = DriverRecord::login(false, PathBuf::from("driver.log"));
        let alive = refusal(&login).unwrap_err();
        assert!(alive.contains("already running"), "got: {alive}");
        assert!(alive.contains("`stop`"), "got: {alive}");

        login.host = Some("elsewhere-node".into());
        let unknown = refusal(&login).unwrap_err();
        assert!(
            unknown.contains("cannot tell")
                && unknown.contains("the driver runs on elsewhere-node")
                && unknown.contains("delete scm/run001/scm_driver.json"),
            "got: {unknown}"
        );

        // however it is asked: a driver that recorded its end is gone
        login.ended = Some(utils::get_utc_now());
        assert_eq!(refusal(&login), Ok(()));
    }

    /// A lookup that failed says nothing about the process: it is never
    /// read as the process being gone, which would let a second driver
    /// start beside one still running.
    #[test]
    fn a_failed_lookup_is_not_an_absent_process() {
        // ps: found, absent, and three ways of not having an answer
        assert_eq!(
            ps_start(true, "Tue Nov 17 14:22:21 2026\n", ""),
            Ok(Some("Tue Nov 17 14:22:21 2026".to_string()))
        );
        assert_eq!(ps_start(false, "", ""), Ok(None));
        assert!(ps_start(false, "", "ps: illegal option -- o\n").is_err());
        assert!(ps_start(true, "", "").is_err());
        assert!(ps_start(false, "Tue Nov 17", "ps: warning\n").is_err());

        // /proc: the start is field 22, the 20th after the command name, which
        // may itself hold spaces and parentheses (each field here holds its
        // own number)
        let stat = format!(
            "4242 (a (odd) name) S {}",
            (4..=24)
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        );
        assert_eq!(
            proc_stat_start(&stat, "boot-1").as_deref(),
            Some("boot-1 22")
        );
        assert_eq!(proc_stat_start("4242 (short) S 1 2", "boot-1"), None);
    }

    /// A slurm driver is alive while its job is in the queue, even once
    /// its record says it ended: the job still holds what the run left (a
    /// shared node's fits), and a new driver or an overwrite waits for slurm
    /// to let it go. A login driver's recorded end is taken as it is.
    #[test]
    fn a_slurm_driver_lives_as_long_as_its_job() {
        let queue = |ids: &[usize]| Some(ids.iter().copied().collect::<HashSet<usize>>());
        let mut job = DriverRecord::slurm(42, true, PathBuf::from("/proj/.slurm-logs/x.out"));
        job.ended = Some(utils::get_utc_now());
        assert_eq!(job.liveness_in(|| queue(&[42, 7])), Liveness::Alive);
        assert_eq!(job.liveness_in(|| queue(&[7])), Liveness::Gone);
        assert!(matches!(job.liveness_in(|| None), Liveness::Unknown(_)));

        let mut login = DriverRecord::login(true, PathBuf::from("driver.log"));
        login.ended = Some(utils::get_utc_now());
        assert_eq!(
            login.liveness_in(|| panic!("a login driver is not looked for in slurm")),
            Liveness::Gone
        );
    }

    /// A driver that recorded its end is gone, whatever has its pid by then
    /// (here the pid is this very process, still alive).
    #[test]
    fn a_driver_that_recorded_its_end_is_gone() {
        let dir = tempfile::tempdir().unwrap();
        let mut login = DriverRecord::login(false, dir.path().join("scm_driver.log"));
        login.mark_ended(dir.path()).unwrap();
        let read = DriverRecord::read(dir.path()).unwrap();
        assert!(read.ended.is_some());
        assert_eq!(read.liveness(), Liveness::Gone);
    }
}
