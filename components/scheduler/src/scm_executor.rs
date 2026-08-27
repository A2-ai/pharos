use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use config::NonmemConfig;
use nonmem::RunOptions;
use nonmem::scm::FitExecutor;
use nonmem::scm::round::run_finished;

use crate::{SchedulerType, slurm};

pub struct ScmSlurmExecutor {
    pub config_path: PathBuf,
    pub nonmem_config: NonmemConfig,
    pub pharos_exe: PathBuf,
    pub partition: Option<String>,
    pub account: Option<String>,
    pub nonmem_version: Option<String>,
    pub poll_interval: Duration,
    /// How long to wait for one round's jobs before giving up. The state file
    /// makes the search resumable, so a timeout is recoverable.
    pub timeout: Duration,
    /// Cap on jobs in flight at once; further models are submitted as earlier
    /// ones finish (sliding window). 0 means no cap.
    pub max_concurrent: usize,
}

impl ScmSlurmExecutor {
    fn submit_batch(&self, models: &[PathBuf]) -> Result<()> {
        let submit_options = slurm::SubmitOptions {
            model: String::new(),
            job_name: None,
            partition: self.partition.clone(),
            account: self.account.clone(),
            template: None,
            dry_run: false,
        };

        let scheduler = SchedulerType::new_slurm(submit_options);
        let submitted = scheduler
            .submit(
                &self.config_path,
                models.to_vec(),
                RunOptions {
                    overwrite: true,
                    nonmem_version: self.nonmem_version.clone(),
                    ..Default::default()
                },
                self.nonmem_config.clone(),
                self.pharos_exe.clone(),
            )
            .context("failed to submit SCM round to slurm")?;

        for (model, job_id) in &submitted {
            log::info!("submitted {} as slurm job {job_id}", model.display());
        }
        Ok(())
    }
}

impl FitExecutor for ScmSlurmExecutor {
    fn fit(&self, models: &[PathBuf]) -> Result<()> {
        if models.is_empty() {
            return Ok(());
        }

        let window = if self.max_concurrent == 0 {
            models.len()
        } else {
            self.max_concurrent
        };

        let deadline = Instant::now() + self.timeout;
        let mut queued: Vec<PathBuf> = models.to_vec();
        let mut in_flight: Vec<PathBuf> = Vec::new();

        // Sliding window: keep at most `window` jobs on the cluster, topping
        // up as earlier ones finish.
        loop {
            if !queued.is_empty() && in_flight.len() < window {
                let take = (window - in_flight.len()).min(queued.len());
                let batch: Vec<PathBuf> = queued.drain(..take).collect();
                self.submit_batch(&batch)?;
                in_flight.extend(batch);
            }

            // Submission is fire-and-forget (no job-state API), so completion
            // is detected by the end/termination files a run leaves behind. A
            // job cancelled before it starts is indistinguishable from one
            // still queued — the timeout catches it.
            in_flight.retain(|m| !run_finished(m));

            if in_flight.is_empty() && queued.is_empty() {
                return Ok(());
            }

            if Instant::now() >= deadline {
                let list = in_flight
                    .iter()
                    .chain(queued.iter())
                    .map(|m| m.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                bail!(
                    "timed out after {:?} waiting for {} slurm job(s): {list}. \
                     The search state was saved; run `pharos nonmem scm run` again to resume.",
                    self.timeout,
                    in_flight.len() + queued.len()
                );
            }

            std::thread::sleep(self.poll_interval);
        }
    }

    fn describe(&self) -> String {
        let base = match &self.partition {
            Some(p) => format!("slurm (partition {p}"),
            None => "slurm (default partition".to_string(),
        };
        if self.max_concurrent > 0 {
            format!("{base}, max {} concurrent)", self.max_concurrent)
        } else {
            format!("{base})")
        }
    }
}
