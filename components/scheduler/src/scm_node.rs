//! Running an SCM process's fits on one node (`--shared-node`).
//!
//! Each fit's script is rendered from the slurm template exactly as `pharos
//! nonmem slurm submit` renders it, so whatever a site puts in its template
//! (module loads, environment) still runs. Instead of `sbatch`, the script is
//! run with bash on the node: directly when the driver is on that node (a
//! `scm slurm submit --shared-node` driver job), or as an `srun` step into a
//! node allocated up front when the driver is on the login node (`scm run
//! --shared-node`). Either way the `#SBATCH` lines are comments to bash: the
//! node, not the template, sets the resources.

use std::cell::RefCell;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use config::NonmemConfig;
use nonmem::scm::report::{Mark, report_fit, report_in, report_record};
use nonmem::scm::round::{forget_dead_run, run_completed};
use nonmem::scm::{FitExecutor, Interrupted, interrupted, live, report};

use crate::scm_executor::{
    InFlight, adopt, fit_run_options, fit_scheduler, poll_in_flight, poll_interval,
    recorded_job_alive, registered_job,
};
use crate::{PreparedJob, slurm};
use nonmem::scm::shown;

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
    /// Fits started by `submit` and not yet waited for: they hold their
    /// CPUs, so a round's window shrinks by them while they run.
    detached: RefCell<Vec<Running>>,
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
            detached: RefCell::new(Vec::new()),
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
        report(format!(
            "requesting a whole node on partition {partition_name}..."
        ));
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
        report(format!("got slurm allocation {job_id} ({node_cpus} CPUs)"));

        Ok(Self {
            config_path,
            nonmem_config,
            pharos_exe,
            partition: Some(partition_name),
            account,
            max_concurrent,
            launcher: NodeLauncher::Allocation { job_id },
            node_cpus,
            detached: RefCell::new(Vec::new()),
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
        fit_scheduler(self.partition.clone(), self.account.clone()).prepare(
            &self.config_path,
            models.to_vec(),
            fit_run_options(),
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
        report_record(format!(
            "starting {} (log {})",
            shown(&job.model),
            shown(&log_path)
        ));
        let child = command
            .spawn()
            .with_context(|| format!("failed to start the fit of {:?}", job.model))?;
        live::fit_running(&job.model, Some(self.place()));
        Ok(Running {
            model: job.model,
            child,
        })
    }

    /// Where a fit runs, for the live view: `this node` / `alloc 91`
    fn place(&self) -> String {
        match &self.launcher {
            NodeLauncher::Local => "this node".to_string(),
            NodeLauncher::Allocation { job_id } => format!("alloc {job_id}"),
        }
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
    /// attempts that failed. What a fit has left on disk is kept as it is:
    /// one that completed anyway is used on resume, and one that did not is
    /// fitted again as the same attempt (the fit overwrites what the run
    /// left). Nothing is deleted: the fit may well still be running — the
    /// kill reaches the launcher, and `srun` cannot forward SIGKILL to its
    /// step — and its copier only carries changed files, so a cleared run
    /// directory would come back without its start record and model.
    fn abandon(&self, running: &mut Vec<Running>) {
        for mut fit in running.drain(..) {
            let _ = fit.child.kill();
            let _ = fit.child.wait();
            if !run_completed(&fit.model, &self.nonmem_config) {
                report_in(format!(
                    "stopped {}; it is fitted again on resume",
                    shown(&fit.model)
                ));
            }
        }
    }

    /// Stop every fit, the detached ones included: they die with the
    /// driver anyway, and are fitted again on resume.
    fn abandon_all(&self, running: &mut Vec<Running>) {
        self.abandon(running);
        self.abandon(&mut self.detached.borrow_mut());
    }

    /// Hand the detached fits among `models` over to a `fit` that waits for
    /// them, and reap the others: a detached fit that has ended leaves its
    /// outcome on disk for the driver, so only its CPUs matter here. Returns
    /// the adopted fits and how many detached ones still run.
    fn adopt_detached(&self, models: &[PathBuf]) -> (Vec<Running>, usize) {
        let mut detached = self.detached.borrow_mut();
        let adopted: Vec<Running> = detached
            .extract_if(.., |fit| models.contains(&fit.model))
            .collect();
        detached.retain_mut(|fit| matches!(fit.child.try_wait(), Ok(None)));
        (adopted, detached.len())
    }

    /// [`adopt`] for the models with a slurm job on record (from an earlier
    /// submit without `--shared-node`), asking squeue only when there is
    /// one. A model whose recorded job is gone without completing its run
    /// has its start marker and termination record dropped, as the slurm
    /// executor does, so until the fit here starts nothing reads it as a run
    /// still going. Returns the jobs to wait for, the models whose run
    /// completed, and the models to fit here.
    fn adopt_recorded(
        &self,
        models: &[PathBuf],
    ) -> Result<(Vec<InFlight>, Vec<PathBuf>, Vec<PathBuf>)> {
        let mut recorded = Vec::new();
        for model in models {
            if registered_job(model)?.is_some() {
                recorded.push(model.clone());
            }
        }
        if recorded.is_empty() {
            return Ok((vec![], vec![], models.to_vec()));
        }
        let (in_slurm, finished, dead) =
            adopt(&recorded, slurm::alive_jobs().as_ref(), &self.nonmem_config)?;
        for model in &dead {
            forget_dead_run(model, &self.nonmem_config)?;
        }
        let queued = models
            .iter()
            .filter(|m| !in_slurm.iter().any(|j| &j.model == *m) && !finished.contains(m))
            .cloned()
            .collect();
        Ok((in_slurm, finished, queued))
    }

    fn release(&mut self) {
        if let NodeLauncher::Allocation { job_id } = self.launcher {
            self.launcher = NodeLauncher::Local;
            if slurm::job_in_queue(job_id) == Some(false) {
                return;
            }
            match slurm::scancel(job_id) {
                Ok(()) => report(format!("released slurm allocation {job_id}")),
                Err(e) => report_fit(
                    "",
                    Mark::Warn,
                    format!("WARNING: could not release slurm allocation {job_id}: {e}"),
                    None,
                ),
            }
        }
    }
}

impl Drop for ScmNodeExecutor {
    /// The fits this executor started end with it, the detached ones
    /// included: a driver that pauses after its round cap (or fails) must
    /// not leave a child writing into a run directory the next driver reads
    /// as dead and fits again. Inside a slurm job the job's end would kill
    /// them anyway; this makes it so wherever the driver runs.
    fn drop(&mut self) {
        let mut detached = std::mem::take(&mut *self.detached.borrow_mut());
        self.abandon(&mut detached);
        self.release();
    }
}

impl FitExecutor for ScmNodeExecutor {
    fn fit(&self, models: &[PathBuf], done: &dyn Fn(&Path)) -> Result<()> {
        if models.is_empty() {
            return Ok(());
        }
        let window = self.window();
        let (mut running, _) = self.adopt_detached(models);
        for fit in &running {
            report_in(format!(
                "{} is already running on the node; waiting for it",
                shown(&fit.model)
            ));
        }
        // A model with a slurm job on record was submitted by an earlier
        // driver without --shared-node. One still in the queue is waited
        // for, beside the fits here, not started again over its live run.
        let (mut in_slurm, finished, queued) = self.adopt_recorded(models)?;
        for model in &finished {
            report_in(format!(
                "{} finished as a slurm job while this driver was starting; using it",
                shown(&model)
            ));
            done(model);
        }
        for job in &in_slurm {
            report_in(format!(
                "{} is still running as slurm job {} (submitted without --shared-node); \
                 waiting for it instead of fitting it here",
                shown(&job.model),
                job.job_id
            ));
            live::fit_queued(&job.model, format!("job {}", job.job_id));
        }
        let mut queued: Vec<PathBuf> = queued
            .into_iter()
            .filter(|m| !running.iter().any(|fit| fit.model == *m))
            .collect();
        let poll_interval = poll_interval(&self.nonmem_config);
        let mut last_poll: Option<Instant> = None;

        loop {
            if interrupted() {
                self.abandon_all(&mut running);
                if !in_slurm.is_empty() {
                    report(format!(
                        "stopping; {} fit(s) still in slurm are waited for on resume",
                        in_slurm.len()
                    ));
                }
                return Err(Interrupted.into());
            }

            // Jobs in slurm are looked at at the slurm executor's pace:
            // squeue is not free, while the children here are reaped every
            // second.
            if !in_slurm.is_empty() && last_poll.is_none_or(|t| t.elapsed() >= poll_interval) {
                poll_in_flight(&mut in_slurm, &self.nonmem_config, done);
                last_poll = Some(Instant::now());
            }

            // Detached fits still running hold CPUs the wave cannot use
            let (_, detached) = self.adopt_detached(&[]);
            let busy = running.len() + detached;
            if !queued.is_empty() && busy < window {
                let take = (window - busy).min(queued.len());
                let batch: Vec<PathBuf> = queued.drain(..take).collect();
                // A stop signals the whole process group, the NM-TRAN check
                // inside `prepare` included: a check that dies with it is
                // the stop, not a failed model.
                let jobs = match self.prepare(&batch) {
                    Ok(jobs) => jobs,
                    Err(_) if interrupted() => {
                        self.abandon_all(&mut running);
                        return Err(Interrupted.into());
                    }
                    Err(e) => return Err(e),
                };
                for job in jobs {
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
                    Some(_) => done(&fit.model),
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
                self.abandon_all(&mut lost);
                bail!(
                    "slurm allocation {job_id} ended (cancelled, or its node failed); \
                     its fits are fitted again on the next submit"
                );
            }
            for (fit, status) in failed {
                report_fit(
                    "",
                    Mark::Warn,
                    format!(
                        "WARNING: the fit of {} exited with {status}; the attempt is retried if retries remain",
                        shown(&fit.model)
                    ),
                    None,
                );
                done(&fit.model);
            }

            if running.is_empty() && queued.is_empty() && in_slurm.is_empty() {
                return Ok(());
            }
            std::thread::sleep(REAP_INTERVAL);
            live::tick();
        }
    }

    /// Start the fits now and come back to them: they run beside whatever
    /// `fit` runs next, and a later `fit` of the same models waits for them.
    fn submit(&self, models: &[PathBuf]) -> Result<()> {
        let jobs = self.prepare(models)?;
        let mut detached = self.detached.borrow_mut();
        for job in jobs {
            detached.push(self.launch(job)?);
        }
        Ok(())
    }

    /// A child of this executor still running the fit, or the slurm job an
    /// earlier driver without `--shared-node` submitted it as, while that is
    /// in the queue. A fit an earlier shared-node driver started died with
    /// that driver (see the `Drop` impl), so a new driver never finds one
    /// of those running.
    fn still_running(&self, model: &Path) -> Result<Option<String>> {
        let own = self
            .detached
            .borrow_mut()
            .iter_mut()
            .any(|fit| fit.model == model && matches!(fit.child.try_wait(), Ok(None)));
        if own {
            return Ok(Some(self.place()));
        }
        Ok(
            recorded_job_alive(model, poll_interval(&self.nonmem_config))?
                .map(|job_id| format!("slurm job {job_id}")),
        )
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

    /// A detached fit (the forward model's) ends with the executor, so a
    /// driver that pauses after its round cap leaves no child writing where
    /// the next driver fits again.
    #[cfg(unix)]
    #[test]
    fn detached_fits_die_with_the_executor() {
        let dir = tempfile::tempdir().unwrap();
        let executor = ScmNodeExecutor::on_this_node(
            dir.path().join("pharos.toml"),
            NonmemConfig::default(),
            PathBuf::from("pharos"),
            None,
        );
        let child = Command::new("sleep").arg("30").spawn().unwrap();
        let pid = child.id();
        executor.detached.borrow_mut().push(Running {
            model: dir.path().join("run_a.mod"),
            child,
        });
        drop(executor);
        // Killed and reaped: no process by that id is left
        assert!(!Path::new(&format!("/proc/{pid}")).exists());
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
