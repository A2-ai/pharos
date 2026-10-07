//! The SCM summary: one heavy record per round, and one for the whole SCM
//! process, built from the state and what every fit left on disk.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result, bail};
use config::NonmemConfig;
use fs_err as fs;
use nonmem_parser::Transform;
use serde::{Deserialize, Serialize};
use utils::{format_duration as fmt_duration, get_utc_now, seconds_between};

use super::roster::RosterEntry;
use super::round::{ext_path_in, run_dir_for, run_summary};
use super::score::chi2_isf;
use super::state::{
    CandidateRecord, CandidateStatus, CheckpointFit, CheckpointStatus, RoundRecord, ScmState,
};
use super::{
    Direction, Lines, NO_REFERENCE, ROUND_SUMMARY_JSON, ROUND_SUMMARY_MD, RUN_SUMMARY_FILENAME,
    SCM_SUMMARY_FILENAME, SCM_SUMMARY_MD, ScmOptions, ScmPlan, none_or_list, ofv_suffix, on_off,
    pick_label, plural, rel_to, yes_no,
};
use crate::output_files::ext::ThetaEstimate;
use crate::run::metadata::{RUN_END_FILENAME, RUN_START_FILENAME, RunEndFile, RunStartFile};
use crate::{ModelLayout, output_files::Summary};

/// The whole SCM process: plan, roster, every round, totals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScmSummary {
    pub generated: String,
    pub pharos_version: String,
    pub plan_digest: String,
    pub initial_model: String,
    pub out_dir: String,
    pub options: ScmOptions,
    pub status: String,
    pub message: Option<String>,
    pub phase: Option<String>,
    pub updated: Option<String>,
    pub models_running: Vec<String>,
    pub candidates: Vec<String>,
    pub roster: Vec<RosterEntry>,
    pub retained: Vec<String>,
    /// The forward model's covariance-step fit, when the process has one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forward_final: Option<CheckpointFit>,
    pub final_model: Option<String>,
    pub final_ofv: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub final_heuristics: Vec<String>,
    pub totals: Totals,
    pub rounds: Vec<RoundSummary>,
    /// The `pharos nonmem summary` of every run the rounds name, read once
    /// while the summary was built.
    #[serde(skip)]
    pub fits: Fits,
}

/// Process-wide counts and timing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Totals {
    pub rounds_complete: usize,
    pub models_fitted: usize,
    pub retries: usize,
    pub unusable: usize,
    pub withdrawn: usize,
    pub timing: Timing,
}

/// One round, reference fit included.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoundSummary {
    pub round: String,
    pub direction: Direction,
    /// The round's position among SCM rounds (1-based); 0 for the reference.
    pub index: usize,
    /// Its position within its phase (1-based); 0 for the reference.
    pub phase_index: usize,
    pub complete: bool,
    pub reference_model: String,
    pub reference_ofv: Option<f64>,
    pub reference_files: RunFiles,
    pub alpha: Option<f64>,
    pub retained_before: Vec<String>,
    pub retained_after: Vec<String>,
    pub removed_before: Vec<String>,
    pub counts: RoundCounts,
    pub winner: Option<String>,
    pub decision: String,
    pub timing: Timing,
    pub candidates: Vec<CandidateSummary>,
}

/// `round_summary.json`: one round, with the process facts it stands alone with.
#[derive(Serialize)]
struct RoundFile<'a> {
    generated: &'a str,
    plan_digest: &'a str,
    initial_model: &'a str,
    out_dir: &'a str,
    scm_status: &'a str,
    next: String,
    #[serde(flatten)]
    round: &'a RoundSummary,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RoundCounts {
    pub candidates: usize,
    pub succeeded: usize,
    pub unusable: usize,
    pub withdrawn: usize,
    pub retries: usize,
}

/// One candidate's fit within one round.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateSummary {
    pub candidate: String,
    pub action: String,
    pub status: String,
    pub model: String,
    pub selected: bool,
    pub rank: Option<usize>,
    /// The effect's theta (the roster carries its initial and fixed values);
    /// none for a reference fit.
    pub theta: Option<usize>,
    pub ofv: Option<f64>,
    pub delta_ofv: Option<f64>,
    pub statistic: Option<f64>,
    pub df: usize,
    pub p_value: Option<f64>,
    pub critical_delta_ofv: Option<f64>,
    pub significant: Option<bool>,
    pub heuristics: Vec<String>,
    pub attempts: Vec<AttemptSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub superseded: Vec<AttemptSummary>,
    pub files: RunFiles,
    pub timing: Timing,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttemptSummary {
    pub model: String,
    pub outcome: String,
    pub timing: Timing,
}

/// Where a candidate's scoring run left its files, relative to out_dir.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RunFiles {
    pub run_dir: Option<String>,
    pub lst: Option<String>,
    pub ext: Option<String>,
    pub summary_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Timing {
    pub started: Option<String>,
    pub ended: Option<String>,
    pub wall_seconds: Option<f64>,
    pub estimation_seconds: Option<f64>,
}

impl Timing {
    /// Widen this span to cover `other`, and add its estimation time.
    fn absorb(&mut self, other: &Timing) {
        if let Some(s) = &other.started
            && self.started.as_ref().is_none_or(|mine| s < mine)
        {
            self.started = Some(s.clone());
        }
        if let Some(e) = &other.ended
            && self.ended.as_ref().is_none_or(|mine| e > mine)
        {
            self.ended = Some(e.clone());
        }
        if let Some(est) = other.estimation_seconds {
            self.estimation_seconds = Some(self.estimation_seconds.unwrap_or(0.0) + est);
        }
        self.wall_seconds = seconds_between(self.started.as_deref(), self.ended.as_deref());
    }
}

/// Where a run left its files and how long it took.
#[derive(Default, Clone)]
struct RunReading {
    timing: Timing,
    files: RunFiles,
}

/// What a summary is built against, and every run read so far.
struct Build<'a> {
    plan: &'a ScmPlan,
    state: &'a ScmState,
    out_dir: &'a Path,
    settings: &'a NonmemConfig,
    runs: BTreeMap<String, RunReading>,
    fits: Fits,
}

impl Build<'_> {
    /// One `AttemptSummary` per attempt, each timed by its own run.
    fn attempt_summaries(
        &mut self,
        records: &[super::state::AttemptRecord],
    ) -> Vec<AttemptSummary> {
        records
            .iter()
            .map(|a| AttemptSummary {
                model: a.model.clone(),
                outcome: a.outcome.clone(),
                timing: self.run(&a.model).timing.clone(),
            })
            .collect()
    }

    /// What `model_rel`'s run left behind, read at most once.
    fn run(&mut self, model_rel: &str) -> RunReading {
        if let Some(reading) = self.runs.get(model_rel) {
            return reading.clone();
        }
        let reading = self.read_run(model_rel);
        self.runs.insert(model_rel.to_string(), reading.clone());
        reading
    }

    fn read_run(&mut self, model_rel: &str) -> RunReading {
        let out_dir = self.out_dir;
        let mut reading = RunReading::default();
        if model_rel.is_empty() {
            return reading;
        }
        let model_path = out_dir.join(model_rel);
        let Ok(layout) = ModelLayout::for_model_path(&model_path) else {
            return reading;
        };
        let Ok(run_dir) = run_dir_for(&model_path, self.settings) else {
            return reading;
        };
        let rel = |p: &Path| p.exists().then(|| rel_to(p, out_dir));
        reading.files.run_dir = rel(&run_dir);
        if reading.files.run_dir.is_none() {
            return reading;
        }

        reading.files.ext = ext_path_in(&model_path, &run_dir)
            .ok()
            .and_then(|p| rel(&p));
        reading.files.lst = rel(&layout.output_file(&run_dir, "lst"));
        reading.files.summary_json = rel(&run_dir.join(RUN_SUMMARY_FILENAME));
        let summary = if run_dir.join(RUN_END_FILENAME).exists() {
            run_summary(&run_dir, self.settings, false).ok()
        } else {
            None
        };

        let start = RunStartFile::load(run_dir.join(RUN_START_FILENAME)).ok();
        let end = RunEndFile::load(run_dir.join(RUN_END_FILENAME)).ok();
        reading.timing.started = start.map(|s| s.start);
        reading.timing.wall_seconds = end.as_ref().map(|e| e.runtime_ms as f64 / 1000.0);
        reading.timing.ended = end.map(|e| e.end);
        reading.timing.estimation_seconds = summary
            .as_ref()
            .map(|s| s.lst.run_details.estimation_time.iter().sum())
            .filter(|t: &f64| *t > 0.0);
        if let (Some(path), Some(summary)) = (&reading.files.summary_json, summary) {
            self.fits.insert(path.clone(), summary);
        }
        reading
    }
}

/// The fits an [`ScmSummary`] describes, keyed by their summary JSON's path
/// (`RunFiles::summary_json`).
pub type Fits = BTreeMap<String, Summary>;

impl RoundSummary {
    /// The effect's own estimate, taken from the model where it is set as free
    pub fn effect_of<'a>(
        &self,
        cand: &CandidateSummary,
        fits: &'a Fits,
    ) -> Option<&'a ThetaEstimate> {
        let free_in = match self.direction {
            Direction::Forward => &cand.files,
            Direction::Backward => &self.reference_files,
        };
        // A run's summary names a theta by its `$THETA` comment under a
        // comment dialect (which every SCM process has), `THETA<n>` otherwise.
        let numbered = format!("THETA{}", cand.theta?);
        fits.get(free_in.summary_json.as_deref()?)?
            .parameters
            .theta
            .iter()
            .find(|t| t.name == numbered || t.name.eq_ignore_ascii_case(&cand.candidate))
    }
}

/// Read the SCM process in `out_dir` and build its summary.
pub fn read_summary(out_dir: &Path) -> Result<ScmSummary> {
    Ok(super::status::ScmStatus::read(out_dir)?.summary)
}

/// Build the summary of `state` against `plan`
pub fn build_summary(
    plan: &ScmPlan,
    state: &ScmState,
    out_dir: &Path,
    settings: &NonmemConfig,
) -> ScmSummary {
    let mut build = Build {
        plan,
        state,
        out_dir,
        settings,
        runs: BTreeMap::new(),
        fits: Fits::default(),
    };
    let mut rounds = Vec::new();
    let mut totals = Totals::default();
    let mut index = 0usize;
    let mut phase_counts: BTreeMap<Direction, usize> = BTreeMap::new();

    // retained_before is reconstructed from each round's decision.
    let mut retained: Vec<String> = Vec::new();
    // In a backward-only process the full model retains every candidate.
    if let Some(first) = state.rounds.first()
        && first.is_reference()
        && first.direction == Direction::Backward
    {
        retained = plan.candidates.iter().map(|c| c.name.clone()).collect();
    }

    for round in &state.rounds {
        let (round_index, phase_index) = if round.is_reference() {
            (0, 0)
        } else {
            index += 1;
            let p = phase_counts.entry(round.direction).or_default();
            *p += 1;
            (index, *p)
        };
        let before = retained.clone();
        if round.complete
            && let Some(w) = &round.winner
        {
            match round.direction {
                Direction::Forward => retained.push(w.clone()),
                Direction::Backward => retained.retain(|n| n != w),
            }
        }
        let summary = build_round(
            &mut build,
            round,
            round_index,
            phase_index,
            &before,
            &retained,
        );
        if round.complete && !round.is_reference() {
            totals.rounds_complete += 1;
        }
        totals.models_fitted += summary
            .candidates
            .iter()
            .map(|c| c.attempts.len() + c.superseded.len())
            .sum::<usize>();
        totals.retries += summary.counts.retries;
        totals.unusable += summary.counts.unusable;
        totals.withdrawn += summary.counts.withdrawn;
        totals.timing.absorb(&summary.timing);
        rounds.push(summary);
    }

    ScmSummary {
        generated: get_utc_now(),
        pharos_version: plan.pharos_version.clone(),
        plan_digest: state.plan_digest.clone(),
        initial_model: plan.model.clone(),
        out_dir: plan.out_dir.clone(),
        options: plan.options.clone(),
        status: state.status.to_string(),
        message: state.message.clone(),
        phase: state.phase.map(|p| p.to_string()),
        updated: Some(state.updated.clone()),
        models_running: Vec::new(),
        candidates: plan.candidates.iter().map(|c| c.name.clone()).collect(),
        roster: state.roster.clone(),
        retained: state.retained.clone(),
        forward_final: state.forward_final.clone(),
        final_model: state.final_model.clone(),
        // Without a final re-fit the final model is the selected one, OFV and all;
        // a re-fit that did not minimize leaves no OFV to report.
        final_ofv: state.final_ofv.or_else(|| {
            let refit = plan.options.final_cov_step;
            state
                .final_model
                .as_ref()
                .filter(|_| !refit)
                .and(state.reference_ofv)
        }),
        final_heuristics: state.final_heuristics.clone(),
        totals,
        rounds,
        fits: build.fits,
    }
}

/// One round's summary; the ranks come from [`RoundRecord::ranks`].
fn build_round(
    build: &mut Build<'_>,
    round: &RoundRecord,
    index: usize,
    phase_index: usize,
    retained_before: &[String],
    retained_after: &[String],
) -> RoundSummary {
    let (plan, state) = (build.plan, build.state);
    let alpha = round.alpha(&plan.options);

    let reference_files = if round.has_reference() {
        build.run(&round.reference_model).files
    } else {
        RunFiles::default()
    };

    let ranks = round.ranks();

    let mut candidates = Vec::new();
    let mut timing = Timing::default();
    let mut counts = RoundCounts {
        candidates: round.candidates.len(),
        ..Default::default()
    };
    for (i, cand) in round.candidates.iter().enumerate() {
        let summary = build_candidate(build, round, cand, alpha, ranks.get(&i).copied());
        match cand.status {
            CandidateStatus::Succeeded => counts.succeeded += 1,
            CandidateStatus::Unusable => counts.unusable += 1,
            CandidateStatus::Withdrawn => counts.withdrawn += 1,
            _ => {}
        }
        counts.retries += cand.n_attempts().saturating_sub(1);
        timing.absorb(&summary.timing);
        candidates.push(summary);
    }

    let round_position = |name: &str| state.rounds.iter().position(|r| r.name == name);
    let this_position = round_position(&round.name).unwrap_or(usize::MAX);
    let removed_before = state
        .removed_roster()
        .filter(|e| match &e.removed.as_ref().unwrap().after_round {
            None => true,
            Some(after) => round_position(after).is_some_and(|p| p < this_position),
        })
        .map(|e| e.candidate.name.clone())
        .collect();

    RoundSummary {
        round: round.name.clone(),
        direction: round.direction,
        index,
        phase_index,
        complete: round.complete,
        reference_model: round.reference_model.clone(),
        reference_ofv: round.reference_ofv,
        reference_files,
        alpha,
        retained_before: retained_before.to_vec(),
        retained_after: retained_after.to_vec(),
        removed_before,
        counts,
        winner: round.winner.clone(),
        decision: round.decision.clone(),
        timing,
        candidates,
    }
}

fn build_candidate(
    build: &mut Build<'_>,
    round: &RoundRecord,
    cand: &CandidateRecord,
    alpha: Option<f64>,
    rank: Option<usize>,
) -> CandidateSummary {
    let theta = build
        .state
        .roster_entry(&cand.candidate)
        .map(|e| e.candidate.theta);

    let RunReading {
        files,
        timing: run_timing,
    } = build.run(&cand.model);

    let statistic = cand.delta_ofv.map(|d| round.direction.statistic(d));
    let critical_delta_ofv = match (alpha, cand.df) {
        (Some(a), df) if df > 0 => Some(chi2_isf(a, df)).filter(|v| v.is_finite()),
        _ => None,
    };

    let attempts = build.attempt_summaries(&cand.attempts);
    let superseded = build.attempt_summaries(&cand.superseded);
    // The candidate's span covers every attempt it has had, the ones made
    // under retuned-away values included.
    let mut timing = Timing::default();
    for a in superseded.iter().chain(&attempts) {
        timing.absorb(&a.timing);
    }
    if attempts.is_empty() && superseded.is_empty() {
        timing = run_timing;
    }

    CandidateSummary {
        candidate: cand.candidate.clone(),
        action: cand.action.clone(),
        status: cand.status.to_string(),
        model: cand.model.clone(),
        selected: cand.selected,
        rank,
        theta,
        ofv: cand.ofv,
        delta_ofv: cand.delta_ofv,
        statistic,
        df: cand.df,
        p_value: cand.p_value,
        critical_delta_ofv,
        significant: cand.significant,
        heuristics: cand.heuristics.clone(),
        attempts,
        superseded,
        files,
        timing,
    }
}

/// Refresh the on-disk record of the SCM process: `round_summary.{json,md}`
/// in `record`'s round directory, and `scm_summary.{json,md}` in `out_dir`.
pub fn write_records(
    out_dir: &Path,
    plan: &ScmPlan,
    state: &ScmState,
    record: &RoundRecord,
    settings: &NonmemConfig,
) -> Result<()> {
    let summary = &build_summary(plan, state, out_dir, settings);
    let round_name = &record.name;
    let round = summary
        .rounds
        .iter()
        .find(|r| &r.round == round_name)
        .with_context(|| format!("no round named {round_name} in the state"))?;
    let dir_name = record
        .dir_name()
        .with_context(|| format!("round {round_name} has no candidates to name its directory"))?;
    let dir = out_dir.join(dir_name);
    fs::create_dir_all(&dir)?;

    let file = RoundFile {
        generated: &summary.generated,
        plan_digest: &summary.plan_digest,
        initial_model: &summary.initial_model,
        out_dir: &summary.out_dir,
        scm_status: &summary.status,
        next: summary.next_step(round),
        round,
    };
    let json_path = dir.join(ROUND_SUMMARY_JSON);
    utils::write_json_to_file(&file, &json_path)
        .with_context(|| format!("failed to write {}", json_path.display()))?;
    let mut md = Lines::new();
    round_markdown(&mut md, round, &summary.fits, Some(&file));
    fs::write(dir.join(ROUND_SUMMARY_MD), md.finish())?;

    let process_path = out_dir.join(SCM_SUMMARY_FILENAME);
    utils::write_json_to_file(summary, &process_path)
        .with_context(|| format!("failed to write {}", process_path.display()))?;
    fs::write(out_dir.join(SCM_SUMMARY_MD), summary.markdown())?;
    Ok(())
}

/// What `scm summary` shows. Every flag adds a layer to the default view
#[derive(Debug, Clone, Default)]
pub struct SummaryOptions {
    /// Restrict to one round: the Nth SCM round ("2" / "round 2"), a round
    /// name ("forward_round1"), or "reference".
    pub round: Option<String>,
    /// Only this candidate: the rounds it was tested in, and its row alone.
    pub candidate: Option<String>,
    /// The header and one line per round, no candidate rows — what the end
    /// of `scm submit` prints (`scm status` has its own rendering, in
    /// `status.rs`). Not a `scm summary` flag.
    pub brief: bool,
    /// `--long`: absolute OFV, the effect's estimate with RSE and CI, df,
    /// attempts, condition number and heuristics on every candidate line
    pub long: bool,
    /// `--timing`: estimation time per candidate and per attempt, and
    /// wall time per round and for the process as a whole.
    pub timing: bool,
    /// `--files`: run directory, .lst, .ext and summary JSON per candidate.
    pub files: bool,
    /// Lines shown right after the process facts. Not a flag.
    pub extra: Vec<String>,
}

impl SummaryOptions {
    /// What the end of `scm submit` prints.
    pub fn brief() -> Self {
        Self {
            brief: true,
            ..Default::default()
        }
    }

    /// Whether `cand` is one the options show.
    fn shows(&self, cand: &CandidateSummary) -> bool {
        self.candidate
            .as_deref()
            .is_none_or(|n| cand.candidate.eq_ignore_ascii_case(n))
    }
}

/// Find the round `selector` names: an exact round name ("forward_round1",
/// "reference"), or the Nth SCM round chronologically ("2" / "round 2")
fn find_round<'a>(rounds: &'a [RoundSummary], selector: &str) -> Result<&'a RoundSummary> {
    let sel = selector.trim();
    if let Some(round) = rounds.iter().find(|r| r.round.eq_ignore_ascii_case(sel)) {
        return Ok(round);
    }
    let lowered = sel.to_ascii_lowercase();
    let num_part = lowered
        .strip_prefix("round")
        .map(str::trim)
        .unwrap_or(lowered.as_str());
    if let Ok(n) = num_part.parse::<usize>()
        && n >= 1
        && let Some(round) = rounds.iter().find(|r| r.index == n)
    {
        return Ok(round);
    }
    let available: Vec<&str> = rounds.iter().map(|r| r.round.as_str()).collect();
    bail!(
        "no round matching '{selector}'; rounds so far: {}",
        if available.is_empty() {
            "(none yet)".to_string()
        } else {
            available.join(", ")
        }
    );
}

impl ScmSummary {
    /// What the SCM process does after `round`, for the round's own record.
    fn next_step(&self, round: &RoundSummary) -> String {
        if self.status == "failed" {
            return match &self.message {
                Some(m) => format!("SCM process failed: {m}"),
                None => "SCM process failed".to_string(),
            };
        }
        match (&self.phase, &self.final_model) {
            (Some(p), _) if !round.has_reference() => format!("start {p} selection"),
            (Some(p), _) => format!("continue {p} selection"),
            (None, Some(f)) => format!("SCM process complete; final model at {f}"),
            (None, None) => "SCM process complete".to_string(),
        }
    }

    pub fn select_rounds(&self, opts: &SummaryOptions) -> Result<Vec<&RoundSummary>> {
        if self.rounds.is_empty() && opts.round.is_some() {
            bail!("the SCM process has not started; no rounds to summarize");
        }
        let mut rounds: Vec<&RoundSummary> = match &opts.round {
            Some(sel) => vec![find_round(&self.rounds, sel)?],
            None => self.rounds.iter().collect(),
        };
        if let Some(name) = &opts.candidate {
            if !self
                .roster
                .iter()
                .any(|e| e.candidate.name.eq_ignore_ascii_case(name))
            {
                bail!(
                    "no candidate named {name}; this SCM process knows: {}",
                    self.roster
                        .iter()
                        .map(|e| e.candidate.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            rounds.retain(|r| {
                r.candidates
                    .iter()
                    .any(|c| c.candidate.eq_ignore_ascii_case(name))
            });
        }
        Ok(rounds)
    }
}

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

/// A number to `digits` places, or `-` when there is none.
pub(crate) fn fmt_num(v: Option<f64>, digits: usize) -> String {
    match v {
        Some(v) => format!("{v:.digits$}"),
        None => "-".to_string(),
    }
}

pub(crate) fn fmt_signed(v: Option<f64>, digits: usize) -> String {
    match v {
        Some(v) => format!("{v:+.digits$}"),
        None => "-".to_string(),
    }
}

pub(crate) fn fmt_p(p: Option<f64>) -> String {
    match p {
        Some(p) if p >= 0.001 => format!("{p:.3}"),
        Some(p) => format!("{p:.1e}"),
        None => "-".to_string(),
    }
}

/// `0.412 (14.2%)`, or `0.412 (N/A)` when the fit carries no standard error
/// to make an RSE from — same as `pharos nonmem summary` with covariance step off
fn fmt_estimate(e: &ThetaEstimate) -> String {
    match e.rse {
        Some(rse) => format!("{:.DIGITS$} ({rse:.1}%)", e.estimate),
        None => format!("{:.DIGITS$} (N/A)", e.estimate),
    }
}

/// The 95% CI
fn fmt_ci(e: &ThetaEstimate) -> String {
    let ci = e
        .stderr
        .and_then(|se| Transform::Identity.compute_ci(e.estimate, se, 0.95).ok());
    match ci {
        Some((lo, hi)) => format!("{lo:.DIGITS$}, {hi:.DIGITS$}"),
        None => "N/A".to_string(),
    }
}

/// A round's candidates winner-first: ranked ones in rank order
fn winner_first(round: &RoundSummary) -> Vec<&CandidateSummary> {
    let mut cands: Vec<&CandidateSummary> = round.candidates.iter().collect();
    let key = |c: &CandidateSummary| c.rank.map(|r| r as f64).or(c.p_value);
    cands.sort_by(|a, b| match (key(a), key(b)) {
        (Some(x), Some(y)) => x.total_cmp(&y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    cands
}

/// Every rendering gives its numbers three decimals.
pub(crate) const DIGITS: usize = 3;

/// What a candidate row reads from: one candidate, in its round
#[derive(Clone, Copy)]
struct Row<'a> {
    round: &'a RoundSummary,
    cand: &'a CandidateSummary,
    fits: &'a Fits,
}

impl<'a> Row<'a> {
    /// The effect's own estimate, from whichever model leaves it free.
    fn effect(&self) -> Option<&'a ThetaEstimate> {
        self.round.effect_of(self.cand, self.fits)
    }

    /// The condition number of the fit's last `$EST`.
    fn condition_number(&self) -> Option<f64> {
        self.fits
            .get(self.cand.files.summary_json.as_deref()?)?
            .minimization_results
            .last()?
            .condition_number
            .filter(|v| v.is_finite())
    }
}

/// One line of the padded text table
fn text_line([name, ofv, dofv, p, star]: [String; 5]) -> String {
    format!("  {name:<12} {ofv:>12} {dofv:>10} {p:>9} {star:<2}")
}

/// The heading of the padded text table; with `timing`, `flags` sits past
/// the est column the rows carry before theirs.
fn text_header(flags: bool, timing: bool) -> String {
    let mut line = text_line(["candidate", "OFV", "dOFV", "p", ""].map(str::to_string));
    if flags {
        if timing {
            write!(line, "   {:EST_WIDTH$}", "").unwrap();
        }
        line.push_str("  flags");
    }
    line
}

/// One candidate's line of the text table: a missing value is `-`
fn text_row(c: &CandidateSummary) -> String {
    let star = match c.significant {
        Some(true) => "*",
        _ => " ",
    };
    text_line([
        c.candidate.clone(),
        fmt_num(c.ofv, DIGITS),
        fmt_signed(c.delta_ofv, DIGITS),
        fmt_p(c.p_value),
        star.to_string(),
    ])
}

/// The `--long` line under a candidate: the effect's estimate (RSE in
/// parentheses) and CI, df, attempts, condition number and the heuristics
/// that fired. A value the run has no reading for is left out.
fn detail_text(r: &Row<'_>) -> String {
    let c = r.cand;
    let e = r.effect();
    let items = [
        ("est", e.map(fmt_estimate)),
        ("CI95", e.map(fmt_ci)),
        ("df", Some(c.df.to_string())),
        ("tries", Some(c.attempts.len().to_string())),
        ("cond#", r.condition_number().map(|v| fmt_num(Some(v), 0))),
        (
            "heuristics",
            (!c.heuristics.is_empty()).then(|| c.heuristics.join(", ")),
        ),
    ];
    let items: Vec<String> = items
        .into_iter()
        .filter_map(|(l, v)| Some(format!("{l} {}", v?)))
        .collect();
    format!("      {}", items.join(" · "))
}

/// The trailing `flags` area of a candidate's line: its status when that is
/// not the plain success the numbers imply, and the round's verdict on it.
/// Empty when a fit succeeded unremarkably and the round did not act on it.
fn flags_text(round: &RoundSummary, c: &CandidateSummary) -> String {
    let mut out = String::new();
    // "running" is left to the model line below the candidate, which names the
    // model actually running; repeating it here only crowds the candidate row.
    if c.status != "succeeded" && c.status != "running" {
        write!(out, "  {}", c.status).unwrap();
    }
    if c.selected {
        let verb = match round.direction {
            Direction::Forward => "selected",
            Direction::Backward => "dropped",
        };
        write!(out, "  <- {verb}").unwrap();
    } else if round.direction == Direction::Backward
        && round.complete
        && c.significant == Some(true)
    {
        out.push_str("  kept");
    }
    out
}

/// A markdown table of one round's candidates, shared by the round summary.
/// The attempts, cond# and heuristic checks columns appear only when some
/// candidate in the round was retried, has a condition number or tripped a
/// check.
fn add_candidate_table(out: &mut Lines, round: &RoundSummary, fits: &Fits) {
    let shown = [
        ("candidate", true),
        ("model", true),
        (
            "attempts",
            round.candidates.iter().any(|c| c.attempts.len() > 1),
        ),
        ("status", true),
        ("OFV", true),
        ("\u{394}OFV", true),
        ("p", true),
        ("significant", true),
        ("selected", true),
        ("estimate (RSE%)", true),
        (
            "cond#",
            round
                .candidates
                .iter()
                .any(|cand| Row { round, cand, fits }.condition_number().is_some()),
        ),
        (
            "heuristic checks",
            round.candidates.iter().any(|c| !c.heuristics.is_empty()),
        ),
    ];
    let pick = |cells: Vec<String>| {
        let kept: Vec<String> = cells
            .into_iter()
            .zip(&shown)
            .filter_map(|(cell, (_, on))| on.then_some(cell))
            .collect();
        format!("| {} |", kept.join(" | "))
    };
    out.add(pick(shown.iter().map(|(h, _)| h.to_string()).collect()));
    out.add(format!(
        "|{}",
        "---|".repeat(shown.iter().filter(|(_, on)| *on).count())
    ));
    let num =
        |v: Option<f64>, digits: usize| v.map(|v| format!("{v:.digits$}")).unwrap_or_default();
    for cand in &round.candidates {
        let r = Row { round, cand, fits };
        let c = cand;
        out.add(pick(vec![
            c.candidate.clone(),
            if c.model.is_empty() {
                String::new()
            } else {
                format!("`{}`", c.model)
            },
            c.attempts.len().to_string(),
            c.status.to_string(),
            num(c.ofv, DIGITS),
            c.delta_ofv.map(|v| format!("{v:+.3}")).unwrap_or_default(),
            c.p_value.map(|p| format!("{p:.4e}")).unwrap_or_default(),
            c.significant.map(yes_no).unwrap_or("").to_string(),
            if c.selected { "**yes**" } else { "" }.to_string(),
            r.effect().map(fmt_estimate).unwrap_or_default(),
            num(r.condition_number(), 0),
            c.heuristics.join("; "),
        ]));
    }
}

// ---------------------------------------------------------------------------
// Text rendering
// ---------------------------------------------------------------------------

impl ScmSummary {
    fn retuned_labels(&self) -> Vec<String> {
        self.roster
            .iter()
            .filter_map(|e| e.retune_label())
            .collect()
    }

    /// One label per candidate the SCM process has dropped from its roster.
    fn removal_labels(&self) -> Vec<String> {
        self.roster
            .iter()
            .filter(|e| e.removed.is_some())
            .map(|e| e.removal_label())
            .collect()
    }

    /// Every covariate the rounds so far added or dropped, then what the
    /// model holds now:
    /// `+WT_CL (forward 1) -> +CRCL_CL (forward 2) -> -CRCL_CL (backward 1) => WT_CL`.
    fn path(&self) -> String {
        let steps: Vec<String> = self
            .rounds
            .iter()
            .filter(|r| r.complete && r.has_reference())
            .filter_map(|r| {
                let sign = match r.direction {
                    Direction::Forward => '+',
                    Direction::Backward => '-',
                };
                let winner = r.winner.as_ref()?;
                Some(format!(
                    "{sign}{winner} ({} {})",
                    r.direction, r.phase_index
                ))
            })
            .collect();
        let now = none_or_list(&self.retained);
        if steps.is_empty() {
            now
        } else {
            format!("{} => {now}", steps.join(" -> "))
        }
    }

    /// The process facts as label and value pairs, in display order: the
    /// text header and the top of `scm_summary.md` print the same list.
    /// `brief` (and `scm status`) leaves out the path, which its round lines
    /// already spell out. `live` (the text renderings, read now) gives the
    /// state's age instead of its timestamp, which the written record keeps.
    pub(crate) fn facts(
        &self,
        timing: bool,
        brief: bool,
        live: bool,
    ) -> Vec<(&'static str, String)> {
        let o = &self.options;
        let mut alphas = Vec::new();
        if o.runs_forward() {
            alphas.push(format!("forward {}", o.forward_alpha));
        }
        if o.runs_backward() {
            alphas.push(format!("backward {}", o.backward_alpha));
        }
        let updated = self.updated.as_ref().map(|u| {
            let age = live
                .then(|| seconds_between(Some(u), Some(&get_utc_now())))
                .flatten();
            match age {
                Some(age) => format!(" (updated {} ago)", fmt_duration(Some(age))),
                None => format!(" (updated {u})"),
            }
        });
        let rounds = match self.totals.rounds_complete {
            0 => "no rounds complete".to_string(),
            n => format!("{} complete", plural(n, "round")),
        };
        let status = format!("{}{} · {rounds}", self.status, updated.unwrap_or_default());
        let list = |items: &[String]| (!items.is_empty()).then(|| items.join(", "));
        let final_model = self
            .final_model
            .as_ref()
            .map(|f| format!("{f}{}", ofv_suffix(self.final_ofv)));
        let forward_model = self.forward_final.as_ref().map(|f| match f.status {
            CheckpointStatus::Running => format!("{} (running)", f.model),
            CheckpointStatus::Succeeded => format!("{}{}", f.model, ofv_suffix(f.ofv)),
            CheckpointStatus::Reused => {
                format!("{}{} (cov step already on)", f.model, ofv_suffix(f.ofv))
            }
            CheckpointStatus::Unusable => format!("{} (unusable)", f.model),
        });
        let time = timing.then(|| {
            format!(
                "{} ({}) · {}",
                plural(self.totals.models_fitted, "fit"),
                plural(self.totals.retries, "retry"),
                timing_span(&self.totals.timing)
            )
        });
        let facts = [
            ("model", Some(self.initial_model.clone())),
            ("status", Some(status)),
            ("direction", Some(o.direction_label())),
            ("alphas", Some(alphas.join(", "))),
            ("cov step", Some(on_off(o.cov_step).to_string())),
            ("candidates", Some(self.candidates.join(", "))),
            ("phase", self.phase.clone()),
            ("note", self.message.clone()),
            ("running", list(&self.models_running)),
            ("path", (!brief).then(|| self.path())),
            ("removed", list(&self.removal_labels())),
            ("retuned", list(&self.retuned_labels())),
            ("forward model", forward_model),
            (
                "forward fit",
                self.forward_final
                    .as_ref()
                    .and_then(|f| list(&f.heuristics)),
            ),
            ("final model", final_model),
            ("final fit", list(&self.final_heuristics)),
            ("time", time),
        ];
        facts
            .into_iter()
            .filter_map(|(l, v)| Some((l, v?)))
            .collect()
    }

    /// The text rendering.
    pub fn render_text(&self, opts: &SummaryOptions) -> Result<String> {
        let fits = &self.fits;
        let mut out = Lines::new();
        let rounds = self.select_rounds(opts)?;
        let command = if opts.brief { "status" } else { "summary" };
        out.add(format!("<scm {command}> {}", self.out_dir));
        for (label, value) in self.facts(opts.timing, opts.brief, true) {
            out.add(format!("{label:<11}: {value}"));
        }
        for line in &opts.extra {
            out.add(line);
        }

        // A planned process has no rounds and no records yet; its `note` says so.
        if opts.brief {
            if !self.rounds.is_empty() {
                out.add("rounds     :");
            }
            for round in &rounds {
                out.add(format!("  {:<18} {}", round.round, round.status_label()));
            }
        } else {
            for round in &rounds {
                out.blank();
                self.render_round(&mut out, round, opts, fits);
            }
            if !self.rounds.is_empty() {
                out.blank();
                out.add(
                    "records    : scm_summary.{json,md} · round_summary.{json,md} in each round dir",
                );
            }
        }
        Ok(out.finish())
    }

    fn render_round(
        &self,
        out: &mut Lines,
        round: &RoundSummary,
        opts: &SummaryOptions,
        fits: &Fits,
    ) {
        let d = DIGITS;
        let single = opts.round.is_some();
        let detail = opts.long || single;

        // Headline: round and where it got to, then reference, alpha, critical value
        out.add(format!("{:<16} {}", round.round, round.progress_label()));
        if round.has_reference() {
            let crit = round
                .candidates
                .iter()
                .find(|c| c.df == 1)
                .and_then(|c| c.critical_delta_ofv)
                .or_else(|| round.candidates.iter().find_map(|c| c.critical_delta_ofv));
            out.add(format!(
                "                 ref OFV {} · alpha {} · crit dOFV {}",
                fmt_num(round.reference_ofv, d),
                round.alpha.map(|a| a.to_string()).unwrap_or_default(),
                fmt_num(crit, d)
            ));
        }
        if detail && round.has_reference() {
            out.add(format!(
                "                 reference {}",
                round.reference_model
            ));
            out.add(format!(
                "                 retained before this round: {}",
                none_or_list(&round.retained_before)
            ));
            // An open round has changed nothing yet
            if round.complete {
                out.add(format!("                 {}", round.change_label()));
            }
            if !round.removed_before.is_empty() {
                out.add(format!(
                    "                 removed before this round: {}",
                    round.removed_before.join(", ")
                ));
            }
        }
        if opts.timing {
            let n: usize = round.candidates.iter().map(|c| c.attempts.len()).sum();
            out.add(format!(
                "                 {} · {}",
                plural(n, "fit"),
                timing_span(&round.timing)
            ));
        }

        // The reference fit is one model and nothing to rank: its attempts,
        // and whatever fired, say all there is.
        if !round.has_reference() {
            for c in round.candidates.iter().filter(|c| opts.shows(c)) {
                self.render_attempts(out, c, opts);
                if !c.heuristics.is_empty() {
                    out.add(format!("      heuristics: {}", c.heuristics.join(", ")));
                }
                if opts.files {
                    render_files(out, &c.files);
                }
            }
            return;
        }

        let shown: Vec<&CandidateSummary> = winner_first(round)
            .into_iter()
            .filter(|c| opts.shows(c))
            .collect();
        let flags: Vec<String> = shown.iter().map(|c| flags_text(round, c)).collect();
        out.add(text_header(
            flags.iter().any(|f| !f.is_empty()),
            opts.timing,
        ));
        for (c, flags) in shown.into_iter().zip(flags) {
            let mut line = text_row(c);
            // est before the flags, padded, so every row's time lines up
            if opts.timing {
                write!(line, "   {:<EST_WIDTH$}", timing_suffix(&c.timing)).unwrap();
            }
            line.push_str(&flags);
            out.add(line);
            if opts.long {
                out.add(detail_text(&Row {
                    round,
                    cand: c,
                    fits,
                }));
            }

            if detail || (c.status != "succeeded" && c.status != "pending") {
                self.render_attempts(out, c, opts);
            }
            if !opts.long && single && !c.heuristics.is_empty() {
                out.add(format!("      heuristics: {}", c.heuristics.join(", ")));
            }
            if opts.files {
                render_files(out, &c.files);
            }
        }
    }

    fn render_attempts(&self, out: &mut Lines, c: &CandidateSummary, opts: &SummaryOptions) {
        let superseded = c.superseded.iter().map(|a| (a, " (superseded)"));
        for (a, tag) in superseded.chain(c.attempts.iter().map(|a| (a, ""))) {
            // With timing, est takes the candidate rows' est column and the
            // outcome their flags column.
            out.add(if opts.timing {
                let est = timing_suffix(&a.timing);
                format!(
                    "      {:<44}    {est:<EST_WIDTH$}  {}{tag}",
                    a.model, a.outcome
                )
            } else {
                format!("      {:<44} {}{tag}", a.model, a.outcome)
            });
        }
        // A dispatched model only joins the attempts list once it finishes, so
        // a model still running is named here.
        if !c.model.is_empty() && !c.attempts.iter().any(|a| a.model == c.model) {
            out.add(format!("      {:<44} {}", c.model, c.status));
        }
    }
}

/// The `--files` lines under a candidate: the paths its run left.
fn render_files(out: &mut Lines, f: &RunFiles) {
    let files = [
        ("run dir", &f.run_dir),
        ("lst", &f.lst),
        ("ext", &f.ext),
        ("summary", &f.summary_json),
    ];
    for (label, path) in files
        .into_iter()
        .filter_map(|(l, p)| Some((l, p.as_ref()?)))
    {
        out.add(format!("      {label:<8} {path}"));
    }
}

/// How every rendering that totals a span spells it.
fn timing_span(t: &Timing) -> String {
    format!("wall {}", fmt_duration(t.wall_seconds))
}

/// Width of the est column in the candidate table: `est ` and the widest
/// duration `fmt_duration` spells under 100h (`59m 59s`, `12h 03m`).
const EST_WIDTH: usize = 11;

/// The per-fit suffix: estimation time alone, no clock times.
fn timing_suffix(t: &Timing) -> String {
    format!("est {}", fmt_duration(t.estimation_seconds))
}

/// The markdown record of one round: its facts, then its candidate table.
/// With `file`, the standalone `round_summary.md` (a top-level heading, and
/// the process facts the file carries).
fn round_markdown(out: &mut Lines, round: &RoundSummary, fits: &Fits, file: Option<&RoundFile>) {
    out.add(format!(
        "{} {}",
        if file.is_some() { "#" } else { "##" },
        round.round
    ));
    out.blank();
    if let Some(f) = file {
        out.add(format!("- initial model: `{}`", f.initial_model));
        out.add(format!("- direction: {}", round.direction));
    }
    if round.has_reference() {
        out.add(format!(
            "- reference: `{}`{}",
            round.reference_model,
            ofv_suffix(round.reference_ofv)
        ));
    }
    if !round.has_reference() {
        reference_markdown(out, round, file);
        return;
    }
    if let Some(a) = round.alpha {
        out.add(format!("- alpha: {a}"));
    }
    let c = &round.counts;
    out.add(format!(
        "- all models minimized: {}",
        yes_no(c.succeeded + c.withdrawn == c.candidates)
    ));
    out.add(format!(
        "- heuristic checks fired: {}",
        yes_no(round.candidates.iter().any(|c| !c.heuristics.is_empty()))
    ));
    if round.counts.withdrawn > 0 {
        out.add(format!(
            "- withdrawn candidates: {}",
            round.counts.withdrawn
        ));
    }
    if !round.removed_before.is_empty() {
        out.add(format!(
            "- removed before this round: {}",
            round.removed_before.join(", ")
        ));
    }
    if !round.decision.is_empty() {
        out.add(format!("- decision: {}", round.decision));
    }
    out.add(format!(
        "- retained before this round: {}",
        none_or_list(&round.retained_before)
    ));
    // An open round has changed nothing yet
    if round.complete {
        out.add(format!("- {}", round.change_label()));
    }
    round_closing_facts(out, round, file);
    out.blank();
    add_candidate_table(out, round, fits);
    if round.counts.unusable > 0 {
        out.blank();
        out.add(
            "_Unusable candidates are reported above; they are never scored as insignificant._",
        );
    }
}

/// The last facts of a round's markdown: how long it took and, in its own
/// file, what comes next.
fn round_closing_facts(out: &mut Lines, round: &RoundSummary, file: Option<&RoundFile>) {
    if let Some(w) = round.timing.wall_seconds {
        out.add(format!("- wall time: {}", fmt_duration(Some(w))));
    }
    if let Some(f) = file {
        out.add(format!("- next: {}", f.next));
    }
}

/// The reference fit's markdown: one model, so its facts and no table.
fn reference_markdown(out: &mut Lines, round: &RoundSummary, file: Option<&RoundFile>) {
    for c in &round.candidates {
        out.add(format!("- model: `{}` ({})", c.model, c.status));
        if c.attempts.len() > 1 {
            let tries: Vec<String> = c
                .attempts
                .iter()
                .map(|a| format!("`{}` {}", a.model, a.outcome))
                .collect();
            out.add(format!("- attempts: {}", tries.join("; ")));
        }
        out.add(format!(
            "- heuristic checks fired: {}",
            none_or_list(&c.heuristics)
        ));
    }
    if !round.decision.is_empty() {
        out.add(format!("- decision: {}", round.decision));
    }
    round_closing_facts(out, round, file);
}

impl ScmSummary {
    /// `scm_summary.md`
    fn markdown(&self) -> String {
        let fits = &self.fits;
        let mut out = Lines::new();
        out.add("# SCM summary");
        out.blank();
        out.add(format!("- out dir: {}", self.out_dir));
        for (label, value) in self.facts(false, false, false) {
            out.add(format!("- {label}: {value}"));
        }
        out.blank();
        for round in &self.rounds {
            round_markdown(&mut out, round, fits, None);
            out.blank();
        }
        out.finish()
    }
}

impl RoundSummary {
    pub fn has_reference(&self) -> bool {
        self.reference_model != NO_REFERENCE
    }

    pub fn candidate(&self, name: &str) -> Option<&CandidateSummary> {
        self.candidates.iter().find(|c| c.candidate == name)
    }

    /// Where the round got to, in one phrase and its decision once complete
    pub fn progress_label(&self) -> String {
        self.label_with(&self.decision)
    }

    /// The round's line in `scm status`: [`RoundSummary::progress_label`],
    /// except that a forward round's pick reads as the covariate it retained.
    pub fn status_label(&self) -> String {
        let pick = self
            .candidates
            .iter()
            .find(|c| c.selected)
            .filter(|_| self.complete && self.direction == Direction::Forward);
        match pick.and_then(|c| Some((c, c.p_value?, c.delta_ofv?))) {
            Some((c, p, delta)) => self.label_with(&pick_label("retained", &c.candidate, p, delta)),
            None => self.progress_label(),
        }
    }

    /// [`RoundSummary::progress_label`] with `decision` for the decision.
    fn label_with(&self, decision: &str) -> String {
        let c = &self.counts;
        let mut label = if self.complete {
            decision.to_string()
        } else {
            let concluded = c.succeeded + c.unusable + c.withdrawn;
            let mut l = format!("in progress — {concluded}/{} concluded", c.candidates);
            if !decision.is_empty() {
                write!(l, " ({decision})").unwrap();
            }
            l
        };
        let mut extra = Vec::new();
        if c.retries > 0 {
            extra.push(plural(c.retries, "retry"));
        }
        if c.withdrawn > 0 {
            extra.push(format!("{} withdrawn", c.withdrawn));
        }
        if !extra.is_empty() {
            write!(label, " [{}]", extra.join(", ")).unwrap();
        }
        label
    }

    /// What this round left, in its phase's own terms: everything a forward
    /// round retained (beside `retained before this round`), the covariate a
    /// backward round dropped
    pub fn change_label(&self) -> String {
        let (verb, changed) = match self.direction {
            Direction::Forward => ("retained", self.retained_after.clone()),
            Direction::Backward => (
                "dropped",
                difference(&self.retained_before, &self.retained_after),
            ),
        };
        format!("{verb} after this round: {}", none_or_list(&changed))
    }
}

/// The names in `from` that `other` does not carry, in `from`'s order.
fn difference(from: &[String], other: &[String]) -> Vec<String> {
    from.iter()
        .filter(|n| !other.contains(n))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scm::test_support::{
        Fit, TEMPLATE, fabricate_running_scm, full_scm_executor, make_plan, write_fit_output,
    };
    use crate::scm::{ScmOptions, run_scm};

    /// The driver only writes a wave's outcomes back to the state once the
    /// whole batch returns, so the reader has to see finished runs itself.
    #[test]
    fn an_open_round_counts_runs_that_finished_since_the_state_was_written() {
        let dir = tempfile::tempdir().unwrap();
        let out_dir = fabricate_running_scm(dir.path());
        let brief = SummaryOptions::brief();

        // Dispatched and still running: reported as running, not concluded.
        let summary = read_summary(&out_dir).unwrap();
        let text = summary.render_text(&brief).unwrap();
        assert!(text.contains("in progress — 1/3 concluded"), "got:\n{text}");
        assert_eq!(
            summary.models_running,
            vec!["forward_round1/1001_crcl_cl.mod".to_string()]
        );

        // Finished with an OFV while the driver still waits on its batch.
        let model = out_dir.join("forward_round1/1001_crcl_cl.mod");
        fs::write(&model, TEMPLATE).unwrap();
        write_fit_output(&model, Fit::Succeeded(990.0)).unwrap();
        let summary = read_summary(&out_dir).unwrap();
        let text = summary.render_text(&brief).unwrap();
        assert!(text.contains("in progress — 2/3 concluded"), "got:\n{text}");
        assert!(summary.models_running.is_empty(), "got:\n{text}");
        let row = summary.rounds[1]
            .candidate("CRCL_CL")
            .expect("CRCL_CL candidate");
        assert_eq!(row.status, "succeeded");
        // Scored against the round's reference the moment its fit lands,
        // rather than waiting for the driver to score the whole round.
        assert_eq!(row.delta_ofv, Some(-10.0));
        assert_eq!(row.significant, Some(true));

        // Reading never writes: the state stays the driver's to update.
        let state = ScmState::load(&out_dir).unwrap().unwrap();
        assert_eq!(
            state.rounds[1].candidates[1].status,
            CandidateStatus::Running
        );
    }

    #[test]
    fn selection_follows_the_options() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        run_scm(&plan, &full_scm_executor(), false).unwrap();
        let summary = read_summary(&plan.out_dir_path()).unwrap();

        for sel in ["1", "round 1", "Round 1", "forward_round1"] {
            let opts = SummaryOptions {
                round: Some(sel.to_string()),
                ..Default::default()
            };
            let rounds = summary.select_rounds(&opts).unwrap();
            assert_eq!(rounds.len(), 1, "{sel}");
            assert_eq!(rounds[0].round, "forward_round1", "{sel}");
        }
        let err = summary
            .select_rounds(&SummaryOptions {
                round: Some("9".into()),
                ..Default::default()
            })
            .unwrap_err();
        assert!(err.to_string().contains("forward_round1"), "{err}");

        // WT_V was tested in the three forward rounds only
        let traced = SummaryOptions {
            candidate: Some("wt_v".into()),
            ..Default::default()
        };
        assert_eq!(summary.select_rounds(&traced).unwrap().len(), 3);
        // and its rows alone are shown there
        let text = summary.render_text(&traced).unwrap();
        assert!(text.contains("WT_V"), "{text}");
        assert!(!text.contains("CRCL_CL   "), "{text}");

        // winner first
        let by_p: Vec<&str> = winner_first(&summary.rounds[1])
            .iter()
            .map(|c| c.candidate.as_str())
            .collect();
        assert_eq!(by_p, vec!["WT_CL", "CRCL_CL", "WT_V"]);
    }
}
