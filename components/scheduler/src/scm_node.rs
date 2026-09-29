//! Running an SCM process's fits on one node (`--shared-node`).
//!
//! Each fit's script is rendered from the slurm template exactly as `pharos
//! nonmem slurm submit` renders it, so whatever a site puts in its template
//! (module loads, environment) still runs. Instead of `sbatch`, the script is
//! run with bash on the node: directly when the driver is on that node (a
//! `scm slurm submit --shared-node` driver job), or as an `srun` step into a
//! node allocated up front when the driver is on the login node (`scm submit
//! --shared-node`). Either way the `#SBATCH` lines are comments to bash: the
//! node, not the template, sets the resources.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use config::NonmemConfig;
use fs_err as fs;
use nonmem::scm::round::run_dir_for;
use nonmem::scm::{FitExecutor, Interrupted, interrupted};
use nonmem::{RUN_END_FILENAME, RunOptions};

use crate::{PreparedJob, SchedulerType, slurm};

/// How often finished fits are reaped: they are children, so checking is free.
const REAP_INTERVAL: Duration = Duration::from_secs(1);

/// Where the fits run.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeLauncher {
    /// On this node: the driver is a slurm job holding the whole node
    Local,
    /// In the node allocation `job_id`, from the login node
    Allocation { job_id: usize },
}

pub struct ScmNodeExecutor {
    pub config_path: PathBuf,
    pub nonmem_config: NonmemConfig,
    pub pharos_exe: PathBuf,
    pub partition: Option<String>,
    pub account: Option<String>,
    /// Fits at once; unset, as many as the node's CPUs hold
    pub max_concurrent: Option<usize>,
    launcher: NodeLauncher,
    node_cpus: usize,
}

/// One running fit.
struct Running {
    model: PathBuf,
    child: Child,
}

impl ScmNodeExecutor {
    /// Fits run on this node, the one the driver job holds.
    pub fn on_this_node(
        config_path: PathBuf,
        nonmem_config: NonmemConfig,
        pharos_exe: PathBuf,
        max_concurrent: Option<usize>,
    ) -> Self {
        Self {
            config_path,
            nonmem_config,
            pharos_exe,
            partition: None,
            account: None,
            max_concurrent,
            launcher: NodeLauncher::Local,
            node_cpus: this_node_cpus(),
        }
    }

    /// Allocate a whole node on `partition` and run the fits in it. Blocks
    /// until slurm grants the node. The allocation is released when the
    /// executor is dropped, which a stop request (see [`nonmem::scm::interrupt`])
    /// leads to within a second.
    pub fn allocate(
        config_path: PathBuf,
        nonmem_config: NonmemConfig,
        pharos_exe: PathBuf,
        partition: Option<String>,
        account: Option<String>,
        max_concurrent: Option<usize>,
        job_name: &str,
    ) -> Result<Self> {
        let partition_name = slurm::resolve_partition(
            partition.as_deref(),
            nonmem_config.slurm.partition.as_deref(),
        )?;
        let args = salloc_args(&partition_name, account.as_deref(), job_name);
        println!("requesting a whole node on partition {partition_name}...");
        log::debug!("salloc {}", args.join(" "));
        let output = Command::new("salloc")
            .args(&args)
            .stdout(Stdio::null())
            .output()
            .context("failed to execute salloc")?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() {
            bail!("salloc failed: {stderr}");
        }
        let job_id = slurm::salloc_job_id(&stderr)?;
        let node_cpus = slurm::job_cpus(job_id)
            .or_else(|| {
                slurm::get_partitions_info()
                    .ok()?
                    .partition_table
                    .iter()
                    .find(|p| p.partition == partition_name)
                    .map(|p| p.cpus as usize)
            })
            .unwrap_or(1);
        println!("got slurm allocation {job_id} ({node_cpus} CPUs)");

        Ok(Self {
            config_path,
            nonmem_config,
            pharos_exe,
            partition: Some(partition_name),
            account,
            max_concurrent,
            launcher: NodeLauncher::Allocation { job_id },
            node_cpus,
        })
    }

    /// The allocation the fits run in, if the executor holds one.
    pub fn allocation(&self) -> Option<usize> {
        match self.launcher {
            NodeLauncher::Allocation { job_id } => Some(job_id),
            NodeLauncher::Local => None,
        }
    }

    fn config_dir(&self) -> &Path {
        self.config_path
            .parent()
            .expect("config file to have a parent dir")
    }

    /// CPUs one fit takes: its MPI workers when NONMEM runs in parallel
    fn cpus_per_fit(&self) -> usize {
        (self
            .nonmem_config
            .parallel
            .resolve_num_cpus(self.config_dir()) as usize)
            .max(1)
    }

    fn window(&self) -> usize {
        concurrency(self.max_concurrent, self.node_cpus, self.cpus_per_fit())
    }

    fn prepare(&self, models: &[PathBuf]) -> Result<Vec<PreparedJob>> {
        let scheduler = SchedulerType::new_slurm(slurm::SubmitOptions {
            model: String::new(),
            job_name: None,
            partition: self.partition.clone(),
            account: self.account.clone(),
            template: None,
            dry_run: false,
        });
        scheduler.prepare(
            &self.config_path,
            models.to_vec(),
            RunOptions {
                overwrite: true,
                ..Default::default()
            },
            self.nonmem_config.clone(),
            self.pharos_exe.clone(),
        )
    }

    fn launch(&self, job: PreparedJob) -> Result<Running> {
        job.write_script()?;
        let log_path = job
            .log_dir
            .join(format!("{}_{}.out", job.job_name, self.log_tag()));
        let log = File::create(&log_path)
            .with_context(|| format!("failed to create fit log {log_path:?}"))?;
        let mut command = match &self.launcher {
            NodeLauncher::Local => {
                let mut c = Command::new("bash");
                c.arg(&job.script_path);
                c
            }
            NodeLauncher::Allocation { job_id } => {
                let mut c = Command::new("srun");
                c.args(srun_args(*job_id, self.cpus_per_fit(), &job.job_name))
                    .arg("bash")
                    .arg(&job.script_path);
                c
            }
        };
        command
            .current_dir(self.config_dir())
            .stdin(Stdio::null())
            .stdout(log.try_clone()?)
            .stderr(log);
        log::info!(
            "starting {} (log {})",
            job.model.display(),
            log_path.display()
        );
        let child = command
            .spawn()
            .with_context(|| format!("failed to start the fit of {:?}", job.model))?;
        Ok(Running {
            model: job.model,
            child,
        })
    }

    /// The job id in a fit's log name, like slurm's `%x_%j.out`
    fn log_tag(&self) -> String {
        match &self.launcher {
            NodeLauncher::Allocation { job_id } => job_id.to_string(),
            NodeLauncher::Local => {
                std::env::var("SLURM_JOB_ID").unwrap_or_else(|_| "local".to_string())
            }
        }
    }

    /// Stop the fits in progress: the driver is going away, so they are not
    /// attempts that failed. Their incomplete output is cleared, and resuming
    /// fits the same attempt again instead of charging a retry.
    fn abandon(&self, running: &mut Vec<Running>) {
        for mut fit in running.drain(..) {
            let _ = fit.child.kill();
            let _ = fit.child.wait();
            let Ok(run_dir) = run_dir_for(&fit.model, &self.nonmem_config) else {
                continue;
            };
            // A fit that completed as the stop came in is kept.
            if run_dir.join(RUN_END_FILENAME).exists() || !run_dir.exists() {
                continue;
            }
            match fs::remove_dir_all(&run_dir) {
                Ok(()) => log::info!(
                    "stopped {}; it is fitted again on resume",
                    fit.model.display()
                ),
                Err(e) => log::warn!("could not clear {}: {e}", run_dir.display()),
            }
        }
    }

    fn release(&mut self) {
        if let NodeLauncher::Allocation { job_id } = self.launcher {
            self.launcher = NodeLauncher::Local;
            if slurm::job_in_queue(job_id) == Some(false) {
                return;
            }
            let released = Command::new("scancel")
                .arg(job_id.to_string())
                .status()
                .is_ok_and(|s| s.success());
            if released {
                println!("released slurm allocation {job_id}");
            } else {
                eprintln!("warning: could not release slurm allocation {job_id}: scancel {job_id}");
            }
        }
    }
}

impl Drop for ScmNodeExecutor {
    fn drop(&mut self) {
        self.release();
    }
}

impl FitExecutor for ScmNodeExecutor {
    fn fit(&self, models: &[PathBuf]) -> Result<()> {
        if models.is_empty() {
            return Ok(());
        }
        let window = self.window();
        let mut queued: Vec<PathBuf> = models.to_vec();
        let mut running: Vec<Running> = Vec::new();

        loop {
            if interrupted() {
                self.abandon(&mut running);
                return Err(Interrupted.into());
            }

            if !queued.is_empty() && running.len() < window {
                let take = (window - running.len()).min(queued.len());
                let batch: Vec<PathBuf> = queued.drain(..take).collect();
                for job in self.prepare(&batch)? {
                    running.push(self.launch(job)?);
                }
            }

            // A fit's outcome is read off disk by the driver; the exit status
            // only says whether the script itself got that far.
            let mut still_running = Vec::with_capacity(running.len());
            let mut failed = Vec::new();
            for mut fit in running {
                match fit.child.try_wait()? {
                    Some(status) if !status.success() => failed.push((fit, status)),
                    Some(_) => {}
                    None => still_running.push(fit),
                }
            }
            running = still_running;

            // Fits failing in an allocation that has ended died with the node,
            // not on their own: none of them is charged an attempt.
            if !failed.is_empty()
                && let Some(job_id) = self.allocation()
                && slurm::job_in_queue(job_id) == Some(false)
            {
                let mut lost: Vec<Running> = failed.into_iter().map(|(fit, _)| fit).collect();
                lost.append(&mut running);
                self.abandon(&mut lost);
                bail!(
                    "slurm allocation {job_id} ended (cancelled, or its node failed); \
                     its fits are fitted again when the plan is submitted again"
                );
            }
            for (fit, status) in failed {
                log::warn!(
                    "the fit of {} exited with {status}; the attempt is retried if retries remain",
                    fit.model.display()
                );
            }

            if running.is_empty() && queued.is_empty() {
                return Ok(());
            }
            std::thread::sleep(REAP_INTERVAL);
        }
    }

    fn settings(&self) -> Result<NonmemConfig> {
        Ok(self.nonmem_config.clone())
    }

    fn describe(&self) -> String {
        let place = match &self.launcher {
            NodeLauncher::Local => "this node".to_string(),
            NodeLauncher::Allocation { job_id } => format!("slurm allocation {job_id}"),
        };
        let n =
            |count: usize, one: &str| format!("{count} {one}{}", if count == 1 { "" } else { "s" });
        format!(
            "shared node ({place}, {}, {} at once)",
            n(self.node_cpus, "CPU"),
            n(self.window(), "fit")
        )
    }
}

/// The CPUs this process may use: slurm's count inside a job, else what the
/// OS allows it (which honours the job's cgroup/affinity).
fn this_node_cpus() -> usize {
    std::env::var("SLURM_CPUS_ON_NODE")
        .ok()
        .and_then(|n| n.trim().parse().ok())
        .or_else(|| std::thread::available_parallelism().ok().map(|n| n.get()))
        .unwrap_or(1)
}

/// Fits at once on a node of `node_cpus`: `max_concurrent` if given, else as
/// many as fit whole.
fn concurrency(max_concurrent: Option<usize>, node_cpus: usize, cpus_per_fit: usize) -> usize {
    match max_concurrent {
        Some(n) if n > 0 => n,
        _ => {
            if cpus_per_fit > node_cpus {
                log::warn!(
                    "one fit wants {cpus_per_fit} CPUs but the node has {node_cpus}; running one at a time"
                );
            }
            (node_cpus / cpus_per_fit).max(1)
        }
    }
}

fn salloc_args(partition: &str, account: Option<&str>, job_name: &str) -> Vec<String> {
    let mut args = vec![
        "--no-shell".to_string(),
        "--nodes=1".to_string(),
        "--exclusive".to_string(),
        "--mem=0".to_string(),
        format!("--partition={partition}"),
        format!("--job-name={job_name}"),
    ];
    if let Some(account) = account {
        args.push(format!("--account={account}"));
    }
    args
}

fn srun_args(job_id: usize, cpus_per_fit: usize, job_name: &str) -> Vec<String> {
    vec![
        format!("--jobid={job_id}"),
        "--ntasks=1".to_string(),
        format!("--cpus-per-task={cpus_per_fit}"),
        "--exact".to_string(),
        format!("--job-name={job_name}"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrency_fills_the_node_unless_capped() {
        assert_eq!(concurrency(None, 8, 1), 8);
        assert_eq!(concurrency(None, 8, 4), 2);
        assert_eq!(concurrency(None, 8, 3), 2);
        assert_eq!(concurrency(None, 2, 4), 1);
        assert_eq!(concurrency(Some(3), 8, 1), 3);
        assert_eq!(concurrency(Some(0), 8, 1), 8);
    }

    #[test]
    fn salloc_claims_a_whole_node() {
        assert_eq!(
            salloc_args("cpu8", Some("lab"), "scm_run001_node"),
            [
                "--no-shell",
                "--nodes=1",
                "--exclusive",
                "--mem=0",
                "--partition=cpu8",
                "--job-name=scm_run001_node",
                "--account=lab"
            ]
        );
    }

    #[test]
    fn srun_steps_take_one_fits_cpus() {
        assert_eq!(
            srun_args(91, 4, "run001_WT_CL"),
            [
                "--jobid=91",
                "--ntasks=1",
                "--cpus-per-task=4",
                "--exact",
                "--job-name=run001_WT_CL"
            ]
        );
    }
}
