use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use config::NonmemConfig;
use nonmem::RunOptions;
use nonmem::scm::FitExecutor;
use nonmem::scm::round::run_finished;

use crate::{SchedulerType, slurm};

/// Seconds between checks for finished slurm jobs.
const POLL_INTERVAL: Duration = Duration::from_secs(30);

pub struct ScmSlurmExecutor {
    pub config_path: PathBuf,
    pub nonmem_config: NonmemConfig,
    pub pharos_exe: PathBuf,
    pub partition: Option<String>,
    pub account: Option<String>,
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
            // is detected by the end/termination files a run leaves behind.
            // There is no deadline: the loop waits as long as the jobs take.
            // State is saved per round, so killing this process leaves the
            // search resumable with `pharos nonmem scm run`.
            in_flight.retain(|m| !run_finished(m));

            if in_flight.is_empty() && queued.is_empty() {
                return Ok(());
            }

            std::thread::sleep(POLL_INTERVAL);
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
