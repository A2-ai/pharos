//! `scm status`: the SCM process as the driver's terminal shows it, read off
//! disk now rather than followed as it happens. Every decided round gets its
//! decision line and the round at a glance, exactly as the record printed
//! them; the open round gets a line per fit as the live view draws it: what
//! it has done, or where it is, how long it has run, and the latest
//! iteration and OFV read off its `.ext` file.

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
/// up by the model's path; `None` when nothing on disk says.
pub type PlaceOf<'a> = &'a dyn Fn(&Path) -> Option<String>;

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
            summary.message = Some("plan written; the SCM process has not started".into());
        }
        summary.models_running = process.models_running.clone();
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
            out.add(format!("{label:<11}: {value}"));
        }
        out
    }

    /// The process at a glance, as the end of `scm submit` prints it.
    pub fn render_brief(&self) -> String {
        self.facts(true).finish()
    }

    /// The text `scm status` prints: the process facts, `extra` lines after
    /// them (what the caller knows about the driver), then round by round,
    /// which spell out the path the facts leave out.
    pub fn render(&self, extra: &[String], place_of: PlaceOf<'_>) -> String {
        let s = &self.summary;
        let state = &self.process.state;
        let mut out = self.facts(false);
        for line in extra {
            out.add(line);
        }
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
            out.add(format!("{:<11}: {progress}", "progress"));
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
        let now = get_utc_now();
        let age = |since: &str| format_duration(seconds_between(Some(since), Some(&now)));
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
            .map(|(c, cs)| self.fit_line(c, cs, width, place_of, &age))
            .collect();
        let count = |state: FitState| fits.iter().filter(|f| f.0 == state).count();
        let mut parts = vec![format!("{}/{} done", count(FitState::Done), fits.len())];
        for (state, word) in [
            (FitState::Running, "running"),
            (FitState::Queued, "queued"),
            (FitState::Pending, "pending"),
        ] {
            let n = count(state);
            if n > 0 {
                parts.push(format!("{n} {word}"));
            }
        }
        if let Some(started) = &round.timing.started {
            parts.push(format!("running {}", age(started)));
        }
        out.add(format!("{}: {}", record.name, parts.join(" · ")));
        for (_, line) in fits {
            out.add(format!("  {line}"));
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
    ) -> (FitState, String) {
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
                // Started once its run directory has the start file, which
                // dates it; queued until then.
                let started = cs.timing.started.as_deref();
                match started.filter(|_| cs.timing.ended.is_none()) {
                    Some(started) => {
                        let latest = cs
                            .files
                            .ext
                            .as_deref()
                            .and_then(|ext| latest_iteration(&self.out_dir.join(ext)));
                        let doing = format!("running {}", age(started));
                        let text = live::fit_line("●", &c.candidate, width, &place, &doing, latest);
                        (FitState::Running, text)
                    }
                    None => {
                        let text = live::fit_line("○", &c.candidate, width, &place, "queued", None);
                        (FitState::Queued, text)
                    }
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
