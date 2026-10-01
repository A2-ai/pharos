use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use config::NonmemConfig;
use fs_err as fs;
use nonmem::RunOptions;
use nonmem::scm::report::{Mark, report_fit, report_in};
use nonmem::scm::round::run_finished;
use nonmem::scm::{FitExecutor, Interrupted, interrupted, live, report};

use crate::{SchedulerType, slurm};

/// Consecutive polls a job may be absent from squeue before it is declared lost.
const MISSING_POLLS_BEFORE_LOST: u32 = 3;

/// The slurm job each fit in a directory was submitted as, by model file name.
/// A driver that stops leaves its fits running; the next one waits for those
/// instead of submitting them again over their live run directories.
const JOB_REGISTRY: &str = ".scm_slurm_jobs.json";

fn registry_path(model: &Path) -> Option<PathBuf> {
    Some(model.parent()?.join(JOB_REGISTRY))
}

fn read_registry(path: &Path) -> BTreeMap<String, usize> {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn model_key(model: &Path) -> Option<String> {
    Some(model.file_name()?.to_string_lossy().into_owned())
}

/// The job `model` was last submitted as, if any.
fn registered_job(model: &Path) -> Option<usize> {
    read_registry(&registry_path(model)?)
        .get(&model_key(model)?)
        .copied()
}

fn record_jobs(submitted: &[(PathBuf, usize)]) -> Result<()> {
    let mut by_dir: BTreeMap<PathBuf, Vec<(String, usize)>> = BTreeMap::new();
    for (model, job_id) in submitted {
        if let (Some(path), Some(key)) = (registry_path(model), model_key(model)) {
            by_dir.entry(path).or_default().push((key, *job_id));
        }
    }
    for (path, jobs) in by_dir {
        let mut registry = read_registry(&path);
        registry.extend(jobs);
        fs::write(&path, serde_json::to_string_pretty(&registry)?)?;
    }
    Ok(())
}

/// Split `models` three ways: those whose last job is still in the queue
/// (`alive`), those whose recorded job left the queue with a finished run
/// behind it, and those that need submitting.
fn adopt(
    models: &[PathBuf],
    alive: &HashSet<usize>,
    settings: &NonmemConfig,
) -> (Vec<InFlight>, Vec<PathBuf>, Vec<PathBuf>) {
    let (mut adopted, mut finished, mut queued) = (Vec::new(), Vec::new(), Vec::new());
    for model in models {
        match registered_job(model) {
            Some(job_id) if alive.contains(&job_id) => adopted.push(InFlight {
                model: model.clone(),
                job_id,
                missing_polls: 0,
            }),
            Some(_) if run_finished(model, settings) => finished.push(model.clone()),
            _ => queued.push(model.clone()),
        }
    }
    (adopted, finished, queued)
}

/// The scheduler every SCM fit is prepared with: the project's slurm
/// template, on the fits' partition.
pub(crate) fn fit_scheduler(partition: Option<String>, account: Option<String>) -> SchedulerType {
    SchedulerType::new_slurm(slurm::SubmitOptions {
        model: String::new(),
        job_name: None,
        partition,
        account,
        template: None,
        dry_run: false,
    })
}

/// Every SCM fit overwrites whatever its run directory holds.
pub(crate) fn fit_run_options() -> RunOptions {
    RunOptions {
        overwrite: true,
        ..Default::default()
    }
}

pub struct ScmSlurmExecutor {
    pub config_path: PathBuf,
    pub nonmem_config: NonmemConfig,
    pub pharos_exe: PathBuf,
    pub partition: Option<String>,
    pub account: Option<String>,
    pub max_concurrent: usize,
}

impl ScmSlurmExecutor {
    /// The pause between checks for finished jobs: `[nonmem.scm] poll_interval`.
    fn poll_interval(&self) -> Duration {
        Duration::from_secs(self.nonmem_config.scm.poll_interval().max(1))
    }
}

/// One submitted, not-yet-finished job.
struct InFlight {
    model: PathBuf,
    job_id: usize,
    missing_polls: u32,
}

impl ScmSlurmExecutor {
    /// Submit `models` one `sbatch` at a time, recording each job as soon as
    /// it is queued, so a failure or a stop part-way through never leaves a
    /// submitted job unrecorded. Once a stop is requested nothing more is
    /// submitted: the jobs already queued are returned, to be waited for now
    /// or on resume.
    fn submit_batch(&self, models: &[PathBuf]) -> Result<Vec<(PathBuf, usize)>> {
        let scheduler = fit_scheduler(self.partition.clone(), self.account.clone());
        let mut submitted = Vec::new();
        for model in models {
            if interrupted() {
                break;
            }
            let job = match scheduler.submit(
                &self.config_path,
                vec![model.clone()],
                fit_run_options(),
                self.nonmem_config.clone(),
                self.pharos_exe.clone(),
            ) {
                Ok(job) => job,
                // A stop signals the driver's whole job, the sbatch it is
                // waiting on included: a submission that dies with it is the
                // stop, not a failure.
                Err(_) if interrupted() => break,
                Err(e) => return Err(e.context("failed to submit SCM round to slurm")),
            };
            for (model, job_id) in &job {
                report_in(format!(
                    "submitted {} as slurm job {job_id}",
                    model.display()
                ));
                live::fit_queued(model, format!("job {job_id}"));
            }
            record_jobs(&job)?;
            submitted.extend(job);
        }
        Ok(submitted)
    }
}

/// The jobs in an `squeue -o "%i|%T"` listing: every one, and those running.
fn queue_states(queue: &str) -> (HashSet<usize>, HashSet<usize>) {
    let mut alive = HashSet::new();
    let mut running = HashSet::new();
    for line in queue.lines() {
        let mut fields = line.trim().split('|');
        let Some(id) = fields.next().and_then(slurm::squeue_job_id) else {
            continue;
        };
        alive.insert(id);
        if fields.next().is_some_and(|state| state.trim() == "RUNNING") {
            running.insert(id);
        }
    }
    (alive, running)
}

/// Apply one squeue observation: jobs present in `alive` reset their miss
/// count, absent ones accumulate misses, and jobs missing for [`MISSING_POLLS_BEFORE_LOST`]
/// consecutive polls are removed and returned as lost.
fn mark_lost(in_flight: &mut Vec<InFlight>, alive: &HashSet<usize>) -> Vec<InFlight> {
    for job in in_flight.iter_mut() {
        if alive.contains(&job.job_id) {
            job.missing_polls = 0;
        } else {
            job.missing_polls += 1;
        }
    }
    in_flight
        .extract_if(.., |job| job.missing_polls >= MISSING_POLLS_BEFORE_LOST)
        .collect()
}

impl FitExecutor for ScmSlurmExecutor {
    fn fit(&self, models: &[PathBuf], done: &dyn Fn(&Path)) -> Result<()> {
        if models.is_empty() {
            return Ok(());
        }

        let window = if self.max_concurrent == 0 {
            models.len()
        } else {
            self.max_concurrent
        };

        let settings = self.settings()?;
        let alive = slurm::alive_jobs().unwrap_or_default();
        // A recorded job gone from the queue may have finished since the
        // driver last read its run: one that has is used as it is, not
        // submitted again over its finished run.
        let (mut in_flight, finished, mut queued) = adopt(models, &alive, &settings);
        for model in &finished {
            report_in(format!(
                "{} finished while this driver was starting; using it",
                model.display()
            ));
            done(model);
        }
        for job in &in_flight {
            report_in(format!(
                "{} is still running as slurm job {}; waiting for it instead of submitting it again",
                job.model.display(),
                job.job_id
            ));
            live::fit_queued(&job.model, format!("job {}", job.job_id));
        }

        loop {
            // Fits already submitted are independent jobs: they keep running,
            // and the next driver waits for them.
            if interrupted() {
                if !in_flight.is_empty() {
                    report(format!(
                        "stopping; {} fit(s) still in slurm are waited for on resume",
                        in_flight.len()
                    ));
                }
                return Err(Interrupted.into());
            }

            if !queued.is_empty() && in_flight.len() < window {
                let take = (window - in_flight.len()).min(queued.len());
                let batch: Vec<PathBuf> = queued.drain(..take).collect();
                let submitted = self.submit_batch(&batch)?;
                // A stop part-way through leaves the rest of the batch
                // unsubmitted: still queued, so the stop is seen as one.
                let unsubmitted = batch
                    .into_iter()
                    .filter(|m| !submitted.iter().any(|(s, _)| s == m));
                queued.splice(0..0, unsubmitted);
                for (model, job_id) in submitted {
                    in_flight.push(InFlight {
                        model,
                        job_id,
                        missing_polls: 0,
                    });
                }
            }

            // Submission is fire-and-forget, so completion is detected by
            // the end/termination files a run leaves behind.
            for job in in_flight.extract_if(.., |job| run_finished(&job.model, &settings)) {
                done(&job.model);
            }

            if !in_flight.is_empty()
                && let Some(queue) = slurm::squeue("%i|%T")
            {
                let (alive, running) = queue_states(&queue);
                for job in in_flight.iter().filter(|j| running.contains(&j.job_id)) {
                    live::fit_running(&job.model, None);
                }
                for job in mark_lost(&mut in_flight, &alive) {
                    report_fit(
                        "",
                        Mark::Warn,
                        format!(
                            "WARNING: slurm job {} for {} disappeared without finishing (node failure? \
                             scancel?); giving up waiting — the attempt is retried if retries remain",
                            job.job_id,
                            job.model.display()
                        ),
                        None,
                    );
                    done(&job.model);
                }
            }

            if in_flight.is_empty() && queued.is_empty() {
                return Ok(());
            }

            // In short steps, so a stop request is acted on promptly.
            let deadline = std::time::Instant::now() + self.poll_interval();
            while std::time::Instant::now() < deadline && !interrupted() {
                std::thread::sleep(Duration::from_millis(200));
                live::tick();
            }
        }
    }

    fn settings(&self) -> Result<NonmemConfig> {
        Ok(self.nonmem_config.clone())
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
    fn fits_still_in_the_queue_are_adopted_not_resubmitted() {
        let dir = tempfile::tempdir().unwrap();
        let running = dir.path().join("run_a.mod");
        let gone = dir.path().join("run_b.mod");
        let never = dir.path().join("run_c.mod");
        record_jobs(&[(running.clone(), 11), (gone.clone(), 12)]).unwrap();

        let alive: HashSet<usize> = [11, 99].into_iter().collect();
        let (adopted, finished, queued) = adopt(
            &[running.clone(), gone.clone(), never.clone()],
            &alive,
            &NonmemConfig::default(),
        );
        assert_eq!(adopted.len(), 1);
        assert_eq!((adopted[0].model.clone(), adopted[0].job_id), (running, 11));
        assert!(finished.is_empty());
        assert_eq!(queued, vec![gone.clone(), never]);

        // A resubmission replaces the old job id
        record_jobs(&[(gone.clone(), 13)]).unwrap();
        assert_eq!(registered_job(&gone), Some(13));
    }

    #[test]
    fn queue_states_tell_running_jobs_from_queued_ones() {
        let (alive, running) = queue_states("11|RUNNING\n12|PENDING\n13_2|RUNNING\n\nbad\n");
        assert_eq!(alive, [11, 12, 13].into_iter().collect());
        assert_eq!(running, [11, 13].into_iter().collect());
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
