//! `scm status`: the SCM process as the driver's terminal shows it, read off
//! disk now rather than followed as it happens. Every decided round gets its
//! decision line and the round at a glance, exactly as the record printed
//! them; the open round gets a line per fit as the live view draws it: what
//! it has done, or where it is, how long it has run, and the latest
//! iteration and OFV read off its `.ext` file.

use std::path::{Path, PathBuf};

use anyhow::Result;
use utils::{format_duration, get_utc_now, seconds_between};

use super::live::latest_iteration;
use super::report::Tone;
use super::state::{CandidateStatus, RoundRecord, ScmProcess, ScmRunStatus};
use super::summary::{
    CandidateSummary, DIGITS, RoundSummary, ScmSummary, build_summary, fmt_num, fmt_p, fmt_signed,
};
use super::{Direction, Lines, max_models_for, none_or_list};

/// Where a fit is, for its line in the open round: `job 1234`, say. Looked
/// up by the model's path; `None` when nothing on disk says.
pub type PlaceOf<'a> = &'a dyn Fn(&Path) -> Option<String>;

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
        let process = ScmProcess::read(out_dir)?;
        let settings = super::project_config(out_dir)?;
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

    /// The text `scm status` prints: the process facts, `extra` lines after
    /// them (what the caller knows about the driver), then round by round.
    pub fn render(&self, extra: &[String], place_of: PlaceOf<'_>) -> String {
        let s = &self.summary;
        let state = &self.process.state;
        let mut out = Lines::new();
        out.add(format!("<scm status> {}", s.out_dir));
        for (label, value) in s.facts(false, true, true) {
            out.add(format!("{label:<11}: {value}"));
        }
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
                render_decided(&mut out, record, round);
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

        let fits: Vec<FitLine> = record
            .candidates
            .iter()
            .zip(&round.candidates)
            .map(|(c, cs)| self.fit_line(c, cs, place_of, &age))
            .collect();
        let count = |state: FitState| fits.iter().filter(|f| f.state == state).count();
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

        let width = fits.iter().map(|f| f.name.len()).max().unwrap_or(0);
        for fit in fits {
            let mut line = format!("  {} {:<width$}", fit.mark, fit.name);
            for part in fit.parts {
                line.push_str("  ");
                line.push_str(&part);
            }
            out.add(line);
        }
    }

    /// One fit's line: its mark and name, then what it did or where it is.
    fn fit_line(
        &self,
        c: &super::state::CandidateRecord,
        cs: &CandidateSummary,
        place_of: PlaceOf<'_>,
        age: &dyn Fn(&str) -> String,
    ) -> FitLine {
        let mut line = FitLine {
            mark: '○',
            name: c.candidate.clone(),
            state: FitState::Pending,
            parts: Vec::new(),
        };
        match c.status {
            CandidateStatus::Succeeded => {
                line.mark = '✓';
                line.state = FitState::Done;
                line.parts.push(format!(
                    "fitted on attempt {}, OFV {}",
                    c.attempts.len(),
                    fmt_num(c.ofv, DIGITS)
                ));
                if let Some(wall) = cs.attempts.last().and_then(|a| a.timing.wall_seconds) {
                    line.parts.push(format_duration(Some(wall)));
                }
            }
            CandidateStatus::Unusable => {
                line.mark = '✗';
                line.state = FitState::Done;
                let why = c
                    .attempts
                    .last()
                    .map(|a| a.outcome.as_str())
                    .unwrap_or("not fitted");
                line.parts.push(format!(
                    "unusable ({why}) after {}",
                    super::plural(c.attempts.len(), "attempt")
                ));
            }
            CandidateStatus::Withdrawn => {
                line.mark = '-';
                line.state = FitState::Done;
                line.parts.push("withdrawn".to_string());
            }
            CandidateStatus::Running => {
                if let Some(place) = place_of(&self.out_dir.join(&c.model)) {
                    line.parts.push(place);
                }
                // Started once its run directory has the start file, which
                // dates it; queued until then.
                match cs
                    .timing
                    .started
                    .as_deref()
                    .filter(|_| cs.timing.ended.is_none())
                {
                    Some(started) => {
                        line.mark = '●';
                        line.state = FitState::Running;
                        line.parts.push(format!("running {}", age(started)));
                        if let Some(it) = cs
                            .files
                            .ext
                            .as_deref()
                            .and_then(|ext| latest_iteration(&self.out_dir.join(ext)))
                        {
                            line.parts
                                .push(format!("iter {:<4} OFV {:.DIGITS$}", it.iteration, it.ofv));
                        }
                    }
                    None => {
                        line.state = FitState::Queued;
                        line.parts.push("queued".to_string());
                    }
                }
            }
            CandidateStatus::Pending => {
                let n = c.attempts.len();
                line.parts.push(if n > 0 {
                    format!("retry {} pending", n + 1)
                } else {
                    "pending".to_string()
                });
            }
        }
        line
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FitState {
    Done,
    Running,
    Queued,
    Pending,
}

struct FitLine {
    mark: char,
    name: String,
    state: FitState,
    parts: Vec<String>,
}

/// A decided round as the record printed it: the decision line, and under
/// it the round at a glance. The reference fit has no table.
fn render_decided(out: &mut Lines, record: &RoundRecord, round: &RoundSummary) {
    if record.is_reference() {
        out.add(format!("{} complete: {}", record.name, record.decision));
        return;
    }
    out.add(format!(
        "{} complete: {}; retained: {}",
        record.name,
        record.decision,
        none_or_list(&round.retained_after)
    ));
    for (_, row) in round_table(record, "  ") {
        out.add(row);
    }
}

/// `forward_round1` as the live view names it: `forward round 1`
fn round_label(round_name: &str) -> String {
    round_name.replace("_round", " round ")
}

/// The round at a glance, under its decision line: every candidate best
/// first, with what the round made of it. Each row starts with `indent`.
pub(crate) fn round_table(round: &RoundRecord, indent: &str) -> Vec<(Tone, String)> {
    let row = |name: &str, ofv: &str, dofv: &str, p: &str, star: &str, flags: &str| {
        format!("{indent}{name:<12} {ofv:>12} {dofv:>10} {p:>9} {star:<2}{flags}")
            .trim_end()
            .to_string()
    };
    let verb = match round.direction {
        Direction::Forward => "added",
        Direction::Backward => "dropped",
    };
    let ranks = round.ranks();
    let mut order: Vec<usize> = (0..round.candidates.len()).collect();
    order.sort_by_key(|i| (ranks.get(i).copied().unwrap_or(usize::MAX), *i));

    let mut rows = vec![(Tone::Dim, row("candidate", "OFV", "dOFV", "p", "", ""))];
    for i in order {
        let c = &round.candidates[i];
        let mut flags = Vec::new();
        let tone = if c.selected {
            flags.push(format!("<- {verb}"));
            Tone::Plain
        } else {
            match c.status {
                CandidateStatus::Unusable => {
                    let why = c
                        .attempts
                        .last()
                        .map(|a| a.outcome.as_str())
                        .unwrap_or("not fitted");
                    flags.push(format!("unusable ({why})"));
                    Tone::Bad
                }
                CandidateStatus::Withdrawn => {
                    flags.push("withdrawn".to_string());
                    Tone::Dim
                }
                _ if c.significant == Some(true) => {
                    if round.direction == Direction::Backward {
                        flags.push("kept".to_string());
                    }
                    Tone::Plain
                }
                _ => Tone::Dim,
            }
        };
        if c.attempts.len() > 1 {
            flags.push(format!("[{} attempts]", c.attempts.len()));
        }
        let star = if c.significant == Some(true) { "*" } else { "" };
        let flags = flags.iter().map(|f| format!("  {f}")).collect::<String>();
        rows.push((
            tone,
            row(
                &c.candidate,
                &fmt_num(c.ofv, DIGITS),
                &fmt_signed(c.delta_ofv, DIGITS),
                &fmt_p(c.p_value),
                star,
                &flags,
            ),
        ));
    }
    rows
}
