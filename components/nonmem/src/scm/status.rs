//! `scm status`: the SCM process as the driver's terminal shows it, read off
//! disk now rather than followed as it happens. Every decided round gets its
//! decision line and the round at a glance, exactly as the record printed
//! them; the open round gets a line per fit as the live view draws it: what
//! it has done, or where it is, how long it has run, and the latest
//! iteration and OFV read off its `.ext` file. Once the process is paused
//! or has failed, the clocks stop at the state's last save: a round, and a
//! fit whose job is gone, show how long they ran before the stop rather
//! than a time that keeps counting. A round's clock is the process's: the
//! pauses it has been through are not time run. A fit's is its job's, which
//! runs on through any pause it outlives.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::Result;
use utils::{format_duration, get_utc_now, seconds_between};

use super::live::{self, latest_iteration};
use super::state::{CandidateRecord, CandidateStatus, RoundRecord, ScmProcess, ScmRunStatus};
use super::summary::{
    CandidateSummary, DIGITS, RoundSummary, ScmSummary, build_summary, fmt_num, round_table,
};
use super::{Lines, max_models_for, none_or_list, plural, round_label};

/// Where a fit is, for its line in the open round: `job 1234`, say. Looked
/// up by the model's path; `None` when nothing on disk says. A place ending
/// in [`NOT_IN_QUEUE`] names a job that is no longer running.
pub type PlaceOf<'a> = &'a dyn Fn(&Path) -> Option<String>;

/// What a place lookup appends when the queue no longer has the job.
pub const NOT_IN_QUEUE: &str = "(not in queue)";

/// Read the SCM process in `out_dir` and build its summary.
pub fn read_summary(out_dir: &Path) -> Result<ScmSummary> {
    Ok(ScmStatus::read(out_dir)?.summary)
}

/// The SCM process in an out_dir, read now, with its summary.
#[derive(Debug, Clone)]
pub struct ScmStatus {
    pub process: ScmProcess,
    pub summary: ScmSummary,
    out_dir: PathBuf,
}

impl ScmStatus {
    /// Read the SCM process in `out_dir` and build its summary.
    pub fn read(out_dir: &Path) -> Result<Self> {
        let settings = super::project_config(out_dir)?;
        let process = ScmProcess::read(out_dir, &settings)?;
        let mut summary = build_summary(&process.plan, &process.state, out_dir, &settings);
        if !process.started {
            summary.updated = None;
            summary.message =
                Some("the SCM process has not started; submit its config to start it".into());
        }
        summary.models_running = process.models_running.clone();
        summary.models_queued = process.models_queued.clone();
        Ok(Self {
            process,
            summary,
            out_dir: out_dir.to_path_buf(),
        })
    }

    /// The process facts, with the covariates' path when `path`: the top of
    /// `scm status`, and all the end of `scm submit` prints.
    fn facts(&self, path: bool) -> Lines {
        let mut out = Lines::new();
        out.add(format!("<scm status> {}", self.summary.out_dir));
        for (label, value) in self.summary.facts(false, path, true) {
            out.add(format!("{label:<13}: {value}"));
        }
        out
    }

    /// The process at a glance, as the end of `scm submit` prints it.
    pub fn render_brief(&self) -> String {
        self.facts(true).finish()
    }

    /// The text `scm status` prints: the process facts, then round by round,
    /// which spell out the path the facts leave out.
    pub fn render(&self, place_of: PlaceOf<'_>) -> String {
        let s = &self.summary;
        let state = &self.process.state;
        let mut out = self.facts(false);
        if !self.process.started {
            return out.finish();
        }

        // The process bar, while there is one: fits so far over the worst
        // case (retries can carry it past that), and the open round.
        if state.status == ScmRunStatus::Running {
            let done = state.fits_so_far();
            let total = max_models_for(s.candidates.len(), s.options.phases().len()).max(done);
            let mut progress = format!("{done}/{total} fits");
            if let Some(open) = state.rounds.iter().find(|r| !r.complete) {
                progress.push_str(&format!(" · {}", round_label(&open.name)));
            }
            out.add(format!("{:<13}: {progress}", "progress"));
        }

        // The summary's rounds are the state's, in order
        for (record, round) in state.rounds.iter().zip(&s.rounds) {
            out.blank();
            if record.complete {
                render_decided(&mut out, round);
            } else {
                self.render_open(&mut out, record, round, place_of);
            }
        }
        out.finish()
    }

    /// The open round as the live view draws it: a line for the round, then
    /// one per fit.
    fn render_open(
        &self,
        out: &mut Lines,
        record: &RoundRecord,
        round: &RoundSummary,
        place_of: PlaceOf<'_>,
    ) {
        let state = &self.process.state;
        // Live clocks while the process runs; once it is paused or has
        // failed, they stop at the state's last save, which dated the stop.
        let stopped = (state.status != ScmRunStatus::Running).then(|| state.updated.clone());
        let now = get_utc_now();
        let end = stopped.as_deref().unwrap_or(&now);
        let age = |since: &str| format_duration(seconds_between(Some(since), Some(&now)));
        let ran = |since: &str| format_duration(seconds_between(Some(since), Some(end)));
        let width = record
            .candidates
            .iter()
            .map(|c| c.candidate.len())
            .max()
            .unwrap_or(0);
        let fits: Vec<(FitState, String)> = record
            .candidates
            .iter()
            .zip(&round.candidates)
            .map(|(c, cs)| self.fit_line(c, cs, width, place_of, &age, &ran))
            .collect();
        let count = |state: FitState| fits.iter().filter(|f| f.0 == state).count();
        let mut parts = vec![format!("{}/{} done", count(FitState::Done), fits.len())];
        for (state, word) in [
            (FitState::Running, "running"),
            (FitState::Finished, "finished"),
            (FitState::Stopped, "stopped"),
            (FitState::Queued, "queued"),
            (FitState::Pending, "pending"),
        ] {
            let n = count(state);
            if n > 0 {
                parts.push(format!("{n} {word}"));
            }
        }
        if let Some(started) = &round.timing.started {
            let ran = format_duration(state.ran_between(Some(started), Some(end)));
            parts.push(match stopped {
                None => format!("running {ran}"),
                Some(_) => format!("{} after {ran}", state.status),
            });
        }
        out.add(format!("{}: {}", record.name, parts.join(" · ")));
        for (_, line) in &fits {
            out.add(format!("  {line}"));
        }
        // Stopped with every fit ended: the round is one resume away from
        // its decision.
        if stopped.is_some() && count(FitState::Done) + count(FitState::Finished) == fits.len() {
            out.add(
                "  every fit has ended; submit the config again to compute the p values and decide the round",
            );
        }
    }

    /// One fit's line: what it did, or where it is as the live view draws it.
    fn fit_line(
        &self,
        c: &CandidateRecord,
        cs: &CandidateSummary,
        width: usize,
        place_of: PlaceOf<'_>,
        age: &dyn Fn(&str) -> String,
        ran: &dyn Fn(&str) -> String,
    ) -> (FitState, String) {
        let running = self.process.state.status == ScmRunStatus::Running;
        let line = |state, mark: &str, detail: String| {
            (state, format!("{mark} {:<width$}  {detail}", c.candidate))
        };
        match c.status {
            CandidateStatus::Succeeded => {
                let mut detail = format!(
                    "fitted on attempt {}, OFV {}",
                    c.attempts.len(),
                    fmt_num(c.ofv, DIGITS)
                );
                if let Some(wall) = cs.attempts.last().and_then(|a| a.timing.wall_seconds) {
                    write!(detail, "  {}", format_duration(Some(wall))).unwrap();
                }
                line(FitState::Done, "✓", detail)
            }
            CandidateStatus::Unusable => {
                let why = c
                    .attempts
                    .last()
                    .map_or("not fitted", |a| a.outcome.as_str());
                let after = plural(c.attempts.len(), "attempt");
                line(
                    FitState::Done,
                    "✗",
                    format!("unusable ({why}) after {after}"),
                )
            }
            CandidateStatus::Withdrawn => line(FitState::Done, "-", "withdrawn".to_string()),
            CandidateStatus::Running => {
                let place = place_of(&self.out_dir.join(&c.model)).unwrap_or_default();
                let fit = |state, mark, doing: &str, latest| {
                    (
                        state,
                        live::fit_line(mark, &c.candidate, width, &place, doing, latest),
                    )
                };
                // Its clock runs while the process does, or while its job
                // is still in the queue after a stop; a job gone (or none
                // known) stopped with the process.
                let alive = running || (!place.is_empty() && !place.ends_with(NOT_IN_QUEUE));
                // Started once its run directory has the start file, which
                // dates it; queued until then. This attempt's own run: the
                // candidate's timing spans the attempts before it too.
                let run = &cs.run_timing;
                match (run.started.as_deref(), run.ended.as_deref()) {
                    // Ended, and not yet read in by the driver (it reads
                    // them promptly while running; a paused one on resume).
                    (Some(_), Some(_)) => {
                        let doing = format!("finished {}", format_duration(run.wall_seconds));
                        fit(FitState::Finished, "●", &doing, None)
                    }
                    (Some(started), None) => {
                        let latest = cs
                            .files
                            .ext
                            .as_deref()
                            .and_then(|ext| latest_iteration(&self.out_dir.join(ext)));
                        if alive {
                            fit(
                                FitState::Running,
                                "●",
                                &format!("running {}", age(started)),
                                latest,
                            )
                        } else {
                            fit(
                                FitState::Stopped,
                                "●",
                                &format!("stopped after {}", ran(started)),
                                latest,
                            )
                        }
                    }
                    // Never started: in the queue, or about to be submitted
                    // by the running driver; a stop before its submission
                    // left it for the next driver.
                    (None, _) if alive => fit(FitState::Queued, "○", "queued", None),
                    (None, _) => line(FitState::Pending, "○", "not submitted".to_string()),
                }
            }
            CandidateStatus::Pending => {
                let n = c.attempts.len();
                let what = match n {
                    0 => "pending".to_string(),
                    n => format!("retry {} pending", n + 1),
                };
                line(FitState::Pending, "○", what)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FitState {
    Done,
    Running,
    /// Its run has ended; the driver has not read it in yet
    Finished,
    /// Started, and its job is gone with the stopped process: rerun on resume
    Stopped,
    Queued,
    Pending,
}

/// A decided round as the record printed it: the decision line, and under
/// it the round at a glance. The reference fit has no table.
fn render_decided(out: &mut Lines, round: &RoundSummary) {
    if !round.has_reference() {
        out.add(format!("{} complete: {}", round.round, round.decision));
        return;
    }
    out.add(format!(
        "{} complete: {}; retained: {}",
        round.round,
        round.decision,
        none_or_list(&round.retained_after)
    ));
    for (_, row) in round_table(round, "  ") {
        out.add(row);
    }
}
