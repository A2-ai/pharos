use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use config::NonmemConfig;
use fs_err as fs;
use nonmem::RunOptions;
use nonmem::scm::report::{Mark, report_fit, report_in, report_record};
use nonmem::scm::round::{forget_dead_run, run_completed, run_finished};
use nonmem::scm::{FitExecutor, Interrupted, interrupted, live, report};

use crate::{SchedulerType, slurm};
use nonmem::scm::shown;

/// Consecutive polls a job may be absent from squeue before it is declared lost.
const MISSING_POLLS_BEFORE_LOST: u32 = 3;

/// The slurm job each fit in a directory was submitted as, by model file name.
/// A driver that stops leaves its fits running; the next one waits for those
/// instead of submitting them again over their live run directories.
const JOB_REGISTRY: &str = ".scm_slurm_jobs.json";

fn registry_path(model: &Path) -> Option<PathBuf> {
    Some(model.parent()?.join(JOB_REGISTRY))
}

/// The registry at `path`, empty when there is none. One that cannot be read
/// is an error, never an empty map: a driver that took an unreadable registry
/// for "no jobs on record" would fit every model again over the live run of
/// the job it lost sight of.
fn read_registry(path: &Path) -> Result<BTreeMap<String, usize>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(e) => return Err(e.into()),
    };
    serde_json::from_str(&text).with_context(|| {
        format!(
            "failed to parse the SCM job registry {}: it records the slurm job each fit in \
             that directory was submitted as, so a driver can wait for the job instead of \
             fitting the model again; restore it, or remove it once those jobs have ended",
            shown(&path)
        )
    })
}

/// Write the registry whole and rename it into place, so a driver stopped
/// mid-write (its job scancelled) never leaves a half-written file behind.
fn write_registry(path: &Path, registry: &BTreeMap<String, usize>) -> Result<()> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_string_pretty(registry)?)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn model_key(model: &Path) -> Option<String> {
    Some(model.file_name()?.to_string_lossy().into_owned())
}

/// The job `model` was last submitted as, if any.
pub(crate) fn registered_job(model: &Path) -> Result<Option<usize>> {
    let (Some(path), Some(key)) = (registry_path(model), model_key(model)) else {
        return Ok(None);
    };
    Ok(read_registry(&path)?.get(&key).copied())
}

/// The pause between checks for finished jobs: `[nonmem.scm] poll_interval`.
pub(crate) fn poll_interval(settings: &NonmemConfig) -> Duration {
    Duration::from_secs(settings.scm.poll_interval().max(1))
}

/// Sleep `interval` in short steps, returning early on a stop request, so the
/// stop is acted on promptly; the live view ticks meanwhile.
pub(crate) fn pause_for(interval: Duration) {
    let deadline = Instant::now() + interval;
    while Instant::now() < deadline && !interrupted() {
        std::thread::sleep(Duration::from_millis(200));
        live::tick();
    }
}

/// The slurm job `model` was last submitted as, while it is in the queue.
/// One look at the queue that misses the job is not proof it is gone: a job
/// that has just ended may not have its end record visible yet on a shared
/// filesystem, and a driver that fitted the model again on that would
/// overwrite a finished run. So an absent job gets the benefit of the doubt
/// `fit` gives one in flight, [`MISSING_POLLS_BEFORE_LOST`] looks a poll
/// interval apart, and a job squeue cannot report on is taken to be running.
pub(crate) fn recorded_job_alive(model: &Path, poll_interval: Duration) -> Result<Option<usize>> {
    let Some(job_id) = registered_job(model)? else {
        return Ok(None);
    };
    let gone = confirmed_gone(job_id, slurm::alive_jobs, || {
        pause_for(poll_interval);
        if interrupted() {
            return Err(Interrupted.into());
        }
        Ok(())
    })?;
    Ok((!gone).then_some(job_id))
}

/// Whether `job_id` is missing from [`MISSING_POLLS_BEFORE_LOST`] consecutive
/// `observe`s of the queue, `wait`ing between them: false as soon as one look
/// finds it, or cannot say.
fn confirmed_gone(
    job_id: usize,
    mut observe: impl FnMut() -> Option<HashSet<usize>>,
    mut wait: impl FnMut() -> Result<()>,
) -> Result<bool> {
    for miss in 1..=MISSING_POLLS_BEFORE_LOST {
        match observe() {
            Some(alive) if !alive.contains(&job_id) => {
                if miss < MISSING_POLLS_BEFORE_LOST {
                    wait()?;
                }
            }
            _ => return Ok(false),
        }
    }
    Ok(true)
}

/// Where `scm status` says a fit is: the slurm job it was last submitted
/// as, flagged when that job has left the queue. squeue is asked once, on
/// the first fit that needs it; a fit never submitted here has no place.
pub fn fit_place_lookup() -> impl Fn(&Path) -> Option<String> {
    let alive: std::cell::OnceCell<Option<HashSet<usize>>> = std::cell::OnceCell::new();
    move |model: &Path| {
        // Status is a reading, not a decision: an unreadable registry shows no place.
        let job_id = registered_job(model).unwrap_or_else(|e| {
            log::warn!("{e:#}");
            None
        })?;
        Some(match alive.get_or_init(slurm::alive_jobs) {
            Some(alive) if !alive.contains(&job_id) => {
                format!("job {job_id} {}", nonmem::scm::status::NOT_IN_QUEUE)
            }
            _ => format!("job {job_id}"),
        })
    }
}

/// Cancel the fits still in the slurm queue among those recorded in
/// `out_dir`'s round directories (the fits a driver submitted), so a stop
/// takes them down too; they are otherwise left to finish and used on
/// resume. Returns the jobs cancelled.
pub fn cancel_recorded_fits(out_dir: &Path) -> Result<Vec<usize>> {
    let alive = slurm::alive_jobs().context("squeue is not available")?;
    let mut cancelled = Vec::new();
    for entry in fs::read_dir(out_dir)? {
        let dir = entry?.path();
        if !dir.is_dir() {
            continue;
        }
        for job_id in read_registry(&dir.join(JOB_REGISTRY))?.into_values() {
            if alive.contains(&job_id) && !cancelled.contains(&job_id) {
                slurm::scancel(job_id)?;
                cancelled.push(job_id);
            }
        }
    }
    cancelled.sort_unstable();
    Ok(cancelled)
}

fn record_jobs(submitted: &[(PathBuf, usize)]) -> Result<()> {
    let mut by_dir: BTreeMap<PathBuf, Vec<(String, usize)>> = BTreeMap::new();
    for (model, job_id) in submitted {
        if let (Some(path), Some(key)) = (registry_path(model), model_key(model)) {
            by_dir.entry(path).or_default().push((key, *job_id));
        }
    }
    for (path, jobs) in by_dir {
        let mut registry = read_registry(&path)?;
        registry.extend(jobs);
        write_registry(&path, &registry)?;
    }
    Ok(())
}

/// Split `models` three ways: those whose recorded job left a completed run
/// behind (a killed run is not one: it is fitted again as the same attempt),
/// those whose last job is still in the queue (`alive`; every recorded job
/// when squeue cannot say), and those that need submitting.
pub(crate) fn adopt(
    models: &[PathBuf],
    alive: Option<&HashSet<usize>>,
    settings: &NonmemConfig,
) -> Result<(Vec<InFlight>, Vec<PathBuf>, Vec<PathBuf>)> {
    let (mut adopted, mut finished, mut queued) = (Vec::new(), Vec::new(), Vec::new());
    for model in models {
        match registered_job(model)? {
            Some(_) if run_completed(model, settings) => finished.push(model.clone()),
            Some(job_id) if alive.is_none_or(|alive| alive.contains(&job_id)) => {
                adopted.push(InFlight::new(model.clone(), job_id))
            }
            _ => queued.push(model.clone()),
        }
    }
    Ok((adopted, finished, queued))
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
    /// Fits in flight at once; 0 means every ready fit is submitted at once.
    pub max_concurrent: usize,
}

impl ScmSlurmExecutor {
    fn poll_interval(&self) -> Duration {
        poll_interval(&self.nonmem_config)
    }
}

/// One submitted, not-yet-finished job.
pub(crate) struct InFlight {
    pub(crate) model: PathBuf,
    pub(crate) job_id: usize,
    missing_polls: u32,
}

impl InFlight {
    pub(crate) fn new(model: PathBuf, job_id: usize) -> Self {
        Self {
            model,
            job_id,
            missing_polls: 0,
        }
    }
}

/// One look at the jobs in flight: a job whose run has ended is done; one
/// gone from the queue for [`MISSING_POLLS_BEFORE_LOST`] consecutive looks
/// is given up on (reported, and done); the live view learns which jobs are
/// running. Nothing changes when squeue cannot say.
pub(crate) fn poll_in_flight(
    in_flight: &mut Vec<InFlight>,
    settings: &NonmemConfig,
    done: &dyn Fn(&Path),
) {
    // Submission is fire-and-forget, so completion is detected by the
    // end/termination files a run leaves behind.
    for job in in_flight.extract_if(.., |job| run_finished(&job.model, settings)) {
        done(&job.model);
    }

    if !in_flight.is_empty()
        && let Some(queue) = slurm::squeue("%i|%T")
    {
        let (alive, running) = queue_states(&queue);
        for job in in_flight.iter().filter(|j| running.contains(&j.job_id)) {
            live::fit_running(&job.model, None);
        }
        for job in mark_lost(in_flight, &alive) {
            report_fit(
                "",
                Mark::Warn,
                format!(
                    "WARNING: slurm job {} for {} disappeared without finishing (node failure? \
                     scancel?); giving up waiting — the attempt is retried if retries remain",
                    job.job_id,
                    shown(&job.model)
                ),
                None,
            );
            done(&job.model);
        }
    }
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
                report_record(format!("submitted {} as slurm job {job_id}", shown(&model)));
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
        let alive = slurm::alive_jobs();
        // A recorded job gone from the queue may have finished since the
        // driver last read its run: one that has is used as it is, not
        // submitted again over its finished run.
        let (mut in_flight, finished, mut queued) = adopt(models, alive.as_ref(), &settings)?;
        // A recorded job gone from the queue without completing its run was
        // killed: its run is overwritten by the next, and until then must
        // not read as still going.
        for model in &queued {
            if registered_job(model)?.is_some() {
                forget_dead_run(model, &settings)?;
            }
        }
        for model in &finished {
            report_in(format!(
                "{} finished while this driver was starting; using it",
                shown(&model)
            ));
            done(model);
        }
        if alive.is_none() && !in_flight.is_empty() {
            report_in(format!(
                "squeue is not available: the {} fit(s) already submitted are taken to be running",
                in_flight.len()
            ));
        }
        for job in &in_flight {
            report_in(format!(
                "{} is still running as slurm job {}; waiting for it instead of submitting it again",
                shown(&job.model),
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
                    in_flight.push(InFlight::new(model, job_id));
                }
            }

            poll_in_flight(&mut in_flight, &settings, done);

            if in_flight.is_empty() && queued.is_empty() {
                return Ok(());
            }

            // The live view notices fits starting meanwhile on its own, off
            // their run directories.
            pause_for(self.poll_interval());
        }
    }

    /// One `sbatch` per model, recorded like any submission so a later
    /// `fit` adopts the job. Outside `max_concurrent`: that caps a round's
    /// wave, and this runs beside the waves.
    fn submit(&self, models: &[PathBuf]) -> Result<()> {
        self.submit_batch(models).map(|_| ())
    }

    /// The job `model` was last submitted as, while it is in the queue: see
    /// [`recorded_job_alive`] for the benefit of the doubt an absent job gets.
    fn still_running(&self, model: &Path) -> Result<Option<String>> {
        Ok(recorded_job_alive(model, self.poll_interval())?
            .map(|job_id| format!("slurm job {job_id}")))
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
        let models = [running.clone(), gone.clone(), never.clone()];
        let settings = NonmemConfig::default();
        record_jobs(&[(running.clone(), 11), (gone.clone(), 12)]).unwrap();

        let alive: HashSet<usize> = [11, 99].into_iter().collect();
        let (adopted, finished, queued) = adopt(&models, Some(&alive), &settings).unwrap();
        assert_eq!(adopted.len(), 1);
        assert_eq!((adopted[0].model.clone(), adopted[0].job_id), (running, 11));
        assert!(finished.is_empty());
        assert_eq!(queued, vec![gone.clone(), never.clone()]);

        // Without squeue, every recorded job is waited for
        let (adopted, _, queued) = adopt(&models, None, &settings).unwrap();
        assert_eq!(adopted.len(), 2);
        assert_eq!(queued, vec![never]);

        // A job gone from the queue leaving a killed run is submitted again
        // as the same attempt; one leaving a completed run is used as it is.
        let run_dir = dir.path().join("run_b");
        fs::create_dir_all(&run_dir).unwrap();
        fs::write(run_dir.join(nonmem::TERMINATION_FILENAME), "{}").unwrap();
        let (_, finished, queued) =
            adopt(std::slice::from_ref(&gone), Some(&alive), &settings).unwrap();
        assert!(finished.is_empty());
        assert_eq!(queued, vec![gone.clone()]);
        fs::write(run_dir.join(nonmem::RUN_END_FILENAME), "{}").unwrap();
        let (_, finished, _) = adopt(std::slice::from_ref(&gone), Some(&alive), &settings).unwrap();
        assert_eq!(finished, vec![gone.clone()]);

        // A resubmission replaces the old job id
        record_jobs(&[(gone.clone(), 13)]).unwrap();
        assert_eq!(registered_job(&gone).unwrap(), Some(13));
    }

    #[test]
    fn a_registry_that_cannot_be_read_is_an_error_not_an_empty_one() {
        let dir = tempfile::tempdir().unwrap();
        let model = dir.path().join("run_a.mod");
        // Written whole and renamed into place: nothing half-written is left
        record_jobs(&[(model.clone(), 11)]).unwrap();
        assert_eq!(registered_job(&model).unwrap(), Some(11));
        assert!(!dir.path().join(".scm_slurm_jobs.json.tmp").exists());

        fs::write(dir.path().join(JOB_REGISTRY), "{\"run_a.mod\": 1").unwrap();
        let err = registered_job(&model).unwrap_err().to_string();
        assert!(
            err.contains("failed to parse the SCM job registry"),
            "{err}"
        );
        assert!(record_jobs(&[(model.clone(), 12)]).is_err());
        assert!(adopt(std::slice::from_ref(&model), None, &NonmemConfig::default()).is_err());
    }

    #[test]
    fn a_job_is_gone_only_after_consecutive_misses() {
        use std::cell::Cell;
        let waits = Cell::new(0u32);
        let wait = || {
            waits.set(waits.get() + 1);
            Ok(())
        };
        let absent = || Some(HashSet::new());

        // Missing from every look: gone, after the waits between the looks
        assert!(confirmed_gone(5, absent, wait).unwrap());
        assert_eq!(waits.get(), MISSING_POLLS_BEFORE_LOST - 1);

        // Seen on the second look (an end record not yet visible, say): not
        // gone, and no further waiting
        waits.set(0);
        let mut looks = 0;
        let flicker = || {
            looks += 1;
            Some(if looks == 2 {
                [5].into_iter().collect()
            } else {
                HashSet::new()
            })
        };
        assert!(!confirmed_gone(5, flicker, wait).unwrap());
        assert_eq!(waits.get(), 1);

        // squeue cannot say: taken to be running, at once
        waits.set(0);
        assert!(!confirmed_gone(5, || None, wait).unwrap());
        assert_eq!(waits.get(), 0);

        // A stop request during a wait is a stop
        let stop = || Err(Interrupted.into());
        assert!(confirmed_gone(5, absent, stop).is_err());
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
