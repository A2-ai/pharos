use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result};
use config::NonmemConfig;
use nonmem::RunOptions;
use nonmem::scm::FitExecutor;
use nonmem::scm::round::run_finished;

use crate::{SchedulerType, slurm};

/// Seconds between checks for finished slurm jobs.
const POLL_INTERVAL: Duration = Duration::from_secs(30);

/// Consecutive polls a job may be absent from squeue before it is declared
/// lost. A job that just completed can leave the queue while its output
/// files are still landing (NFS lag), and squeue itself can flicker — so a
/// single missed poll is never enough.
const MISSING_POLLS_BEFORE_LOST: u32 = 3;

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

/// One submitted, not-yet-finished job.
struct InFlight {
    model: PathBuf,
    job_id: usize,
    missing_polls: u32,
}

impl ScmSlurmExecutor {
    fn submit_batch(&self, models: &[PathBuf]) -> Result<Vec<(PathBuf, usize)>> {
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
        Ok(submitted)
    }
}

/// Job ids slurm currently knows about (pending, running, or completing).
/// `None` when squeue is unavailable or fails — callers must treat that as
/// "no information", never as "every job vanished".
fn squeue_job_ids() -> Option<HashSet<usize>> {
    let output = Command::new("squeue").args(["-h", "-o", "%i"]).output().ok()?;
    if !output.status.success() {
        log::debug!(
            "squeue failed ({}); skipping lost-job detection this poll",
            output.status
        );
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            // Array tasks print as "123_4"; the base id is what sbatch returned.
            .filter_map(|line| line.trim().split(['_', '.']).next()?.parse().ok())
            .collect(),
    )
}

/// Apply one squeue observation: jobs present in `alive` reset their miss
/// count, absent ones accumulate misses, and jobs missing for
/// [`MISSING_POLLS_BEFORE_LOST`] consecutive polls are removed and returned
/// as lost.
fn mark_lost(in_flight: &mut Vec<InFlight>, alive: &HashSet<usize>) -> Vec<InFlight> {
    for job in in_flight.iter_mut() {
        if alive.contains(&job.job_id) {
            job.missing_polls = 0;
        } else {
            job.missing_polls += 1;
        }
    }
    let mut lost = Vec::new();
    in_flight.retain_mut(|job| {
        if job.missing_polls >= MISSING_POLLS_BEFORE_LOST {
            lost.push(InFlight {
                model: std::mem::take(&mut job.model),
                job_id: job.job_id,
                missing_polls: job.missing_polls,
            });
            false
        } else {
            true
        }
    });
    lost
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
        let mut in_flight: Vec<InFlight> = Vec::new();

        // Sliding window: keep at most `window` jobs on the cluster, topping
        // up as earlier ones finish.
        loop {
            if !queued.is_empty() && in_flight.len() < window {
                let take = (window - in_flight.len()).min(queued.len());
                let batch: Vec<PathBuf> = queued.drain(..take).collect();
                for (model, job_id) in self.submit_batch(&batch)? {
                    in_flight.push(InFlight {
                        model,
                        job_id,
                        missing_polls: 0,
                    });
                }
            }

            // Submission is fire-and-forget, so completion is detected by
            // the end/termination files a run leaves behind. State is saved
            // per round, so killing this process leaves the search resumable
            // with `pharos nonmem scm run`.
            in_flight.retain(|job| !run_finished(&job.model));

            // A job slurm no longer knows about that never wrote its end
            // file is lost (node failure, scancel) — waiting longer is
            // pointless, and the run is never killed by us. Give up on the
            // attempt; the driver retries it if retries remain.
            if !in_flight.is_empty()
                && let Some(alive) = squeue_job_ids()
            {
                for job in mark_lost(&mut in_flight, &alive) {
                    log::warn!(
                        "slurm job {} for {} disappeared without finishing (node failure? \
                         scancel?); giving up waiting — the attempt is retried if retries remain",
                        job.job_id,
                        job.model.display()
                    );
                }
            }

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

#[cfg(test)]
mod tests {
    use super::*;

    fn job(id: usize) -> InFlight {
        InFlight {
            model: PathBuf::from(format!("m{id}.mod")),
            job_id: id,
            missing_polls: 0,
        }
    }

    #[test]
    fn lost_jobs_need_consecutive_misses() {
        let mut in_flight = vec![job(1), job(2)];
        let alive: HashSet<usize> = [1].into_iter().collect();

        // Two misses: job 2 is still given the benefit of the doubt
        for _ in 0..(MISSING_POLLS_BEFORE_LOST - 1) {
            assert!(mark_lost(&mut in_flight, &alive).is_empty());
        }
        assert_eq!(in_flight.len(), 2);

        // Third consecutive miss: job 2 is lost, job 1 stays
        let lost = mark_lost(&mut in_flight, &alive);
        assert_eq!(lost.len(), 1);
        assert_eq!(lost[0].job_id, 2);
        assert_eq!(in_flight.len(), 1);
        assert_eq!(in_flight[0].job_id, 1);
    }

    #[test]
    fn reappearing_job_resets_the_miss_count() {
        let mut in_flight = vec![job(7)];
        let empty = HashSet::new();
        let alive: HashSet<usize> = [7].into_iter().collect();

        assert!(mark_lost(&mut in_flight, &empty).is_empty());
        assert!(mark_lost(&mut in_flight, &empty).is_empty());
        // Reappears (e.g. squeue flicker): counter resets
        assert!(mark_lost(&mut in_flight, &alive).is_empty());
        assert_eq!(in_flight[0].missing_polls, 0);
        // Needs the full run of misses again
        for _ in 0..(MISSING_POLLS_BEFORE_LOST - 1) {
            assert!(mark_lost(&mut in_flight, &empty).is_empty());
        }
        assert_eq!(mark_lost(&mut in_flight, &empty).len(), 1);
        assert!(in_flight.is_empty());
    }
}
