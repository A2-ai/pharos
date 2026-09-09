//! The SCM summary: one heavy record per round, and one for the whole SCM
//! process, built from the state and what every fit left on disk.
//!
//! The same struct serves two purposes. The driver serialises it into each
//! round's `round_summary.json` / `.md` as the round concludes and into
//! `scm_summary.json` in the out_dir, and `scm summary` rebuilds it on
//! demand and renders it — so the files and the screen can never disagree.
//!
//! Per candidate it carries everything a scientist tends to go looking for
//! after the fact: the scoring (ΔOFV against the critical value at alpha,
//! rank, p), the effect's own estimate with its standard error and interval,
//! the fit's quality (termination, condition number, heuristics), the change
//! in every IIV term against the reference, the full parameter table, every
//! attempt with its timing, and where the run's files live.
//!
//! [`SummaryOptions`] pick what `scm summary` renders, in the spirit of
//! `ls -la -t`: the default is a compact view of every round to date, and
//! the flags stack detail onto it.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result, bail};
use fs_err as fs;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use utils::get_utc_now;

use super::roster::RosterEntry;
use super::round::{ext_path_for, reconcile_state_with_disk, run_dir_for, stem_of};
use super::score::chi2_isf;
use super::state::{
    CandidateRecord, CandidateStatus, PendingTie, RoundRecord, ScmRunStatus, ScmState,
};
use super::{
    Direction, Lines, NO_REFERENCE, PLAN_FILENAME, ROUND_SUMMARY_JSON, ROUND_SUMMARY_MD,
    RUN_SUMMARY_FILENAME, SCM_SUMMARY_FILENAME, ScmOptions, ScmPlan, none_or_list, ofv_suffix,
    on_off, round_dir, yes_no,
};
use crate::output_files::ext::{
    ExtReader, MinimizationResults, ParameterType, TableParameters, get_estimation_results,
};
use crate::output_files::lst::LstSummary;
use crate::run::metadata::{RUN_END_FILENAME, RUN_START_FILENAME};

/// Schema 2: the heavy record (scoring, estimates, fit quality, IIV, timing,
/// files) replaces the schema-1 round summary that carried the candidate
/// records alone.
pub const SUMMARY_SCHEMA_VERSION: u32 = 2;

// ---------------------------------------------------------------------------
// The record
// ---------------------------------------------------------------------------

/// The whole SCM process: plan, roster, every round, totals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScmSummary {
    pub schema_version: u32,
    pub generated: String,
    pub pharos_version: String,
    pub plan_digest: String,
    pub template_model: String,
    pub out_dir: String,
    pub options: ScmOptions,
    /// planned | running | paused | completed | failed
    pub status: String,
    pub message: Option<String>,
    pub phase: Option<String>,
    /// Every candidate the SCM process has known, removed ones included.
    pub roster: Vec<RosterEntry>,
    /// Covariates in the model now, in selection order.
    pub retained: Vec<String>,
    pub final_model: Option<String>,
    /// The final model is assembled from the last reference fit, so that
    /// fit's OFV is its OFV.
    pub final_ofv: Option<f64>,
    pub pending_tie: Option<PendingTie>,
    pub totals: Totals,
    pub rounds: Vec<RoundSummary>,
}

/// Process-wide counts and timing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Totals {
    /// Completed SCM rounds (the reference fit is not a round).
    pub rounds_complete: usize,
    /// Every attempt dispatched, retries included.
    pub models_fitted: usize,
    pub retries: usize,
    pub unusable: usize,
    pub withdrawn: usize,
    pub timing: Timing,
}

/// One round, reference fit included.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoundSummary {
    pub schema_version: u32,
    pub generated: String,
    pub plan_digest: String,
    pub template_model: String,
    pub round: String,
    pub direction: Direction,
    /// The round's position among SCM rounds (1-based); 0 for the reference.
    pub index: usize,
    /// Its position within its phase (1-based); 0 for the reference.
    pub phase_index: usize,
    pub complete: bool,
    /// Reference model relative to out_dir ("-" for the reference round).
    pub reference_model: String,
    pub reference_ofv: Option<f64>,
    /// The reference fit's estimates, for the parameter drift view.
    pub reference_parameters: Option<ParameterTable>,
    /// The alpha this round's candidates are scored against.
    pub alpha: Option<f64>,
    /// Covariates in the model when the round started, and after its decision.
    pub retained_before: Vec<String>,
    pub retained_after: Vec<String>,
    /// Candidates removed from the SCM process before this round.
    pub removed_before: Vec<String>,
    pub counts: RoundCounts,
    /// Every candidate still in the round fitted usably.
    pub all_succeeded: bool,
    /// At least one scoring attempt had heuristic checks fire.
    pub any_heuristics: bool,
    /// At least one candidate ran out of retries without a scoreable fit.
    pub any_unusable: bool,
    pub winner: Option<String>,
    pub decision: String,
    /// SCM status when this summary was built.
    pub scm_status: String,
    /// What the SCM process does next.
    pub next: String,
    pub timing: Timing,
    pub candidates: Vec<CandidateSummary>,
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
    /// Candidate name (e.g. WT_CL); for reference fits, "base" or "full".
    pub candidate: String,
    /// "add WT_CL", "drop WT_CL", "fit base model", "fit full model".
    pub action: String,
    /// pending | running | succeeded | unusable | withdrawn
    pub status: String,
    /// Model of the scoring attempt, relative to out_dir.
    pub model: String,
    pub selected: bool,
    /// Position in the round's ranking among scored candidates (1 = best
    /// for the phase); `None` when not scored.
    pub rank: Option<usize>,
    /// The theta(s) this candidate's effect lives on; empty for reference fits.
    pub thetas: Vec<usize>,
    /// Where the effect was released when first tested, and what it is
    /// fixed at when held out (from the roster; `None` for reference fits).
    pub initial: Option<f64>,
    pub off: Option<f64>,
    pub ofv: Option<f64>,
    pub reference_ofv: Option<f64>,
    /// candidate OFV − reference OFV (negative = candidate improves).
    pub delta_ofv: Option<f64>,
    /// The tested statistic (never negative; a "wrong-way" delta clamps to 0).
    pub statistic: Option<f64>,
    pub df: usize,
    pub p_value: Option<f64>,
    pub alpha: Option<f64>,
    /// The |ΔOFV| the candidate needs to reach alpha at its df.
    pub critical_delta_ofv: Option<f64>,
    pub significant: Option<bool>,
    /// Heuristic checks that fired for the scoring attempt.
    pub heuristics: Vec<String>,
    pub attempts: Vec<AttemptSummary>,
    /// How the scoring attempt's fit went, read from its .ext and .lst.
    pub fit: Option<FitSummary>,
    /// The effect's own estimate in the model where it is free: the
    /// candidate fit in forward selection, the reference in backward.
    pub effect_estimates: Vec<EffectEstimate>,
    /// Every IIV (diagonal OMEGA) term, reference versus candidate.
    pub iiv: Vec<IivChange>,
    /// The scoring attempt's full parameter table.
    pub parameters: Option<ParameterTable>,
    pub files: RunFiles,
    pub timing: Timing,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttemptSummary {
    pub model: String,
    pub outcome: String,
    pub timing: Timing,
}

/// What a fit's output says about how it went.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FitSummary {
    pub termination_code: Option<i32>,
    pub condition_number: Option<f64>,
    pub minimization_terminated: Option<bool>,
    pub program_aborted: Option<bool>,
    pub covariance_step_aborted: Option<bool>,
    pub eigenvalue_issues: Option<bool>,
    pub parameter_near_boundary: Option<bool>,
    pub hessian_reset: Option<bool>,
    pub significant_digits: Option<f64>,
    pub function_evaluations: Option<usize>,
    pub estimation_seconds: Option<f64>,
    pub covariance_seconds: Option<f64>,
    pub number_subjects: Option<usize>,
    pub number_obs: Option<usize>,
    pub estimation_methods: Vec<String>,
}

/// Estimates of every parameter of one fit.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ParameterTable {
    pub thetas: Vec<ParameterEstimate>,
    pub omegas: Vec<ParameterEstimate>,
    pub sigmas: Vec<ParameterEstimate>,
}

impl ParameterTable {
    /// A run that never reached final estimates parses to NaN estimates;
    /// those are left out, so every estimate here is a real number (and
    /// JSON, which has no NaN, round-trips).
    fn from_table(t: &TableParameters) -> Self {
        let mut table = ParameterTable::default();
        for th in t.theta.iter().filter(|th| th.estimate.is_finite()) {
            table.thetas.push(ParameterEstimate::new(
                &th.name,
                th.estimate,
                th.stderr,
                th.rse,
                th.fixed,
                true,
            ));
        }
        for r in t.random_effects.iter().filter(|r| r.estimate.is_finite()) {
            let est =
                ParameterEstimate::new(&r.name, r.estimate, r.stderr, r.rse, r.fixed, r.diagonal);
            match r.param_type {
                ParameterType::Omega => table.omegas.push(est),
                ParameterType::Sigma => table.sigmas.push(est),
                ParameterType::Theta => table.thetas.push(est),
            }
        }
        table
    }

    fn find(&self, name: &str) -> Option<&ParameterEstimate> {
        self.thetas
            .iter()
            .chain(&self.omegas)
            .chain(&self.sigmas)
            .find(|p| p.name == name)
    }

    fn all(&self) -> impl Iterator<Item = &ParameterEstimate> {
        self.thetas.iter().chain(&self.omegas).chain(&self.sigmas)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterEstimate {
    pub name: String,
    pub estimate: f64,
    pub stderr: Option<f64>,
    /// Relative standard error, in percent.
    pub rse: Option<f64>,
    pub fixed: bool,
    /// For OMEGA / SIGMA: whether this is a variance (diagonal) term.
    pub diagonal: bool,
    pub ci95_lower: Option<f64>,
    pub ci95_upper: Option<f64>,
}

impl ParameterEstimate {
    fn new(
        name: &str,
        estimate: f64,
        stderr: Option<f64>,
        rse: Option<f64>,
        fixed: bool,
        diagonal: bool,
    ) -> Self {
        let stderr = stderr.filter(|v| v.is_finite());
        let rse = rse.filter(|v| v.is_finite());
        let (ci95_lower, ci95_upper) = match stderr {
            Some(se) => (Some(estimate - 1.96 * se), Some(estimate + 1.96 * se)),
            None => (None, None),
        };
        Self {
            name: name.to_string(),
            estimate,
            stderr,
            rse,
            fixed,
            diagonal,
            ci95_lower,
            ci95_upper,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectEstimate {
    pub theta: usize,
    pub estimate: f64,
    pub stderr: Option<f64>,
    pub rse: Option<f64>,
    pub ci95_lower: Option<f64>,
    pub ci95_upper: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IivChange {
    /// e.g. `OMEGA(1,1)`
    pub name: String,
    pub reference: f64,
    pub candidate: f64,
    /// (candidate − reference) / reference, in percent.
    pub percent_change: Option<f64>,
}

/// Where a candidate's scoring run left its files, relative to out_dir.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RunFiles {
    pub run_dir: Option<String>,
    pub lst: Option<String>,
    pub ext: Option<String>,
    /// The `pharos nonmem summary` JSON, when the run wrote one.
    pub summary_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Timing {
    pub started: Option<String>,
    pub ended: Option<String>,
    pub wall_seconds: Option<f64>,
    /// NONMEM's own estimation time, summed over $EST records.
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

fn seconds_between(start: Option<&str>, end: Option<&str>) -> Option<f64> {
    let s = Timestamp::from_str(start?).ok()?;
    let e = Timestamp::from_str(end?).ok()?;
    Some(e.duration_since(s).as_secs_f64())
}

// ---------------------------------------------------------------------------
// Reading a run
// ---------------------------------------------------------------------------

/// The pharos start marker, read leniently: the fields the summary needs
/// and nothing else, so a marker from another pharos version still reads.
#[derive(Deserialize, Default)]
struct StartStamp {
    start: Option<String>,
}

#[derive(Deserialize, Default)]
struct EndStamp {
    end: Option<String>,
    runtime_ms: Option<u128>,
}

/// Everything a run's output directory says about it.
#[derive(Default)]
struct RunReading {
    parameters: Option<TableParameters>,
    minimization: Option<MinimizationResults>,
    lst: Option<LstSummary>,
    timing: Timing,
    files: RunFiles,
}

fn read_run(out_dir: &Path, model_rel: &str) -> RunReading {
    let mut reading = RunReading::default();
    if model_rel.is_empty() {
        return reading;
    }
    let model_path = out_dir.join(model_rel);
    let run_dir = run_dir_for(&model_path);
    let rel = |p: &Path| -> Option<String> {
        p.exists().then(|| {
            p.strip_prefix(out_dir)
                .unwrap_or(p)
                .to_string_lossy()
                .to_string()
        })
    };
    reading.files.run_dir = rel(&run_dir);
    if reading.files.run_dir.is_none() {
        return reading;
    }

    let ext = ext_path_for(&model_path);
    reading.files.ext = rel(&ext);
    if ext.exists() {
        let reader = ExtReader::default().final_estimates_and_stderr_and_fixed();
        if let Ok(results) = get_estimation_results(&ext, &reader, None, false, None)
            && let Some(last) = results.into_iter().last()
        {
            reading.parameters = Some(last.parameters);
            reading.minimization = Some(last.minimization_results);
        }
    }

    let lst = run_dir.join(format!("{}.lst", stem_of(&model_path)));
    reading.files.lst = rel(&lst);
    if lst.exists() {
        reading.lst = LstSummary::from_run(&lst).ok();
    }
    reading.files.summary_json = rel(&run_dir.join(RUN_SUMMARY_FILENAME));

    let start: StartStamp = fs::read_to_string(run_dir.join(RUN_START_FILENAME))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let end: EndStamp = fs::read_to_string(run_dir.join(RUN_END_FILENAME))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    reading.timing.started = start.start;
    reading.timing.ended = end.end;
    reading.timing.wall_seconds = end.runtime_ms.map(|ms| ms as f64 / 1000.0).or_else(|| {
        seconds_between(
            reading.timing.started.as_deref(),
            reading.timing.ended.as_deref(),
        )
    });
    reading.timing.estimation_seconds = reading
        .lst
        .as_ref()
        .map(|l| l.run_details.estimation_time.iter().sum())
        .filter(|t: &f64| *t > 0.0);
    reading
}

fn fit_summary(reading: &RunReading) -> Option<FitSummary> {
    if reading.minimization.is_none() && reading.lst.is_none() {
        return None;
    }
    let mut fit = FitSummary::default();
    if let Some(m) = &reading.minimization {
        fit.termination_code = m.termination_code;
        fit.condition_number = m.condition_number.filter(|v| v.is_finite());
    }
    if let Some(l) = &reading.lst {
        let h = &l.run_heuristics;
        fit.minimization_terminated = h.minimization_terminated;
        fit.program_aborted = h.program_aborted;
        fit.covariance_step_aborted = h.covariance_step_aborted;
        fit.eigenvalue_issues = h.eigenvalue_issues;
        fit.parameter_near_boundary = h.parameter_near_boundary;
        fit.hessian_reset = h.hessian_reset;
        let d = &l.run_details;
        fit.significant_digits = (d.significant_digits > 0.0).then_some(d.significant_digits);
        fit.function_evaluations = (d.function_evaluations > 0).then_some(d.function_evaluations);
        fit.estimation_seconds = reading.timing.estimation_seconds;
        let cov: f64 = d.covariance_time.iter().sum();
        fit.covariance_seconds = (cov > 0.0).then_some(cov);
        fit.number_subjects = (d.number_subjects > 0).then_some(d.number_subjects);
        fit.number_obs = (d.number_obs > 0).then_some(d.number_obs);
        fit.estimation_methods = d.estimation_methods.clone();
    }
    Some(fit)
}

// ---------------------------------------------------------------------------
// Building the record
// ---------------------------------------------------------------------------

/// Read the SCM process in `out_dir` and build its summary. A planned but
/// unstarted process summarises to its plan and no rounds.
pub fn read_summary(out_dir: &Path) -> Result<ScmSummary> {
    let plan_path = out_dir.join(PLAN_FILENAME);
    if !plan_path.exists() {
        bail!(
            "{} has no {PLAN_FILENAME}; is this an SCM output directory?",
            out_dir.display()
        );
    }
    let plan = ScmPlan::load(&plan_path)
        .with_context(|| format!("failed to load {}", plan_path.display()))?;
    let mut state = match ScmState::load(out_dir)? {
        Some(state) => state,
        None => ScmState::new(&plan),
    };
    // The driver writes a wave's outcomes back only once the whole batch
    // returns; read finished runs off disk the way every other reader does.
    reconcile_state_with_disk(&mut state, out_dir);
    Ok(build_summary(&plan, &state, out_dir))
}

/// Build the summary of `state` against `plan`, reading fits under
/// `out_dir`. Never fails: a run whose output cannot be read simply carries
/// less.
pub fn build_summary(plan: &ScmPlan, state: &ScmState, out_dir: &Path) -> ScmSummary {
    let generated = get_utc_now();
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
            plan,
            state,
            out_dir,
            round,
            round_index,
            phase_index,
            &before,
            &retained,
            &generated,
        );
        if round.complete && !round.is_reference() {
            totals.rounds_complete += 1;
        }
        totals.models_fitted += summary
            .candidates
            .iter()
            .map(|c| c.attempts.len())
            .sum::<usize>();
        totals.retries += summary.counts.retries;
        totals.unusable += summary.counts.unusable;
        totals.withdrawn += summary.counts.withdrawn;
        totals.timing.absorb(&summary.timing);
        rounds.push(summary);
    }

    ScmSummary {
        schema_version: SUMMARY_SCHEMA_VERSION,
        generated,
        pharos_version: plan.pharos_version.clone(),
        plan_digest: state.plan_digest.clone(),
        template_model: plan.model.clone(),
        out_dir: plan.out_dir.clone(),
        options: plan.options.clone(),
        status: state.status.to_string(),
        message: state.message.clone(),
        phase: state.phase.map(|p| p.to_string()),
        roster: state.roster.clone(),
        retained: state.retained.clone(),
        final_model: state.final_model.clone(),
        final_ofv: state.final_model.as_ref().and(state.reference_ofv),
        pending_tie: state.pending_tie.clone(),
        totals,
        rounds,
    }
}

/// The comparator the driver ranks a round with: forward, smallest p then
/// largest drop; backward, largest p then smallest rise.
fn rank_order(direction: Direction, a: (f64, f64), b: (f64, f64)) -> std::cmp::Ordering {
    match direction {
        Direction::Forward => a.0.total_cmp(&b.0),
        Direction::Backward => b.0.total_cmp(&a.0),
    }
    .then(a.1.total_cmp(&b.1))
}

#[allow(clippy::too_many_arguments)]
fn build_round(
    plan: &ScmPlan,
    state: &ScmState,
    out_dir: &Path,
    round: &RoundRecord,
    index: usize,
    phase_index: usize,
    retained_before: &[String],
    retained_after: &[String],
    generated: &str,
) -> RoundSummary {
    let alpha = if round.is_reference() {
        None
    } else {
        Some(match round.direction {
            Direction::Forward => plan.options.forward_alpha,
            Direction::Backward => plan.options.backward_alpha,
        })
    };

    let reference_reading = if round.has_reference() {
        Some(read_run(out_dir, &round.reference_model))
    } else {
        None
    };
    let reference_parameters = reference_reading
        .as_ref()
        .and_then(|r| r.parameters.as_ref())
        .map(ParameterTable::from_table);

    // Rank the scored candidates the way the driver does.
    let mut scored: Vec<(usize, f64, f64)> = round
        .candidates
        .iter()
        .enumerate()
        .filter_map(|(i, c)| match (c.status, c.p_value, c.delta_ofv) {
            (CandidateStatus::Succeeded, Some(p), Some(d)) => Some((i, p, d)),
            _ => None,
        })
        .collect();
    scored.sort_by(|a, b| rank_order(round.direction, (a.1, a.2), (b.1, b.2)));
    let ranks: BTreeMap<usize, usize> = scored
        .iter()
        .enumerate()
        .map(|(rank, (i, _, _))| (*i, rank + 1))
        .collect();

    let mut candidates = Vec::new();
    let mut timing = Timing::default();
    let mut counts = RoundCounts {
        candidates: round.candidates.len(),
        ..Default::default()
    };
    for (i, cand) in round.candidates.iter().enumerate() {
        let summary = build_candidate(
            state,
            out_dir,
            round,
            cand,
            alpha,
            ranks.get(&i).copied(),
            reference_parameters.as_ref(),
        );
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

    let next = if state.status == ScmRunStatus::Failed {
        match &state.message {
            Some(m) => format!("SCM process failed: {m}"),
            None => "SCM process failed".to_string(),
        }
    } else {
        match state.phase {
            Some(p) if round.is_reference() => format!("start {p} selection"),
            Some(p) => format!("continue {p} selection"),
            None => match &state.final_model {
                Some(f) => format!("SCM process complete; final model at {f}"),
                None => "SCM process complete".to_string(),
            },
        }
    };

    // Removals dated to a round before this one (or to before any round).
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
        schema_version: SUMMARY_SCHEMA_VERSION,
        generated: generated.to_string(),
        plan_digest: state.plan_digest.clone(),
        template_model: plan.model.clone(),
        round: round.name.clone(),
        direction: round.direction,
        index,
        phase_index,
        complete: round.complete,
        reference_model: round.reference_model.clone(),
        reference_ofv: round.reference_ofv,
        reference_parameters,
        alpha,
        retained_before: retained_before.to_vec(),
        retained_after: retained_after.to_vec(),
        removed_before,
        counts,
        all_succeeded: round.all_succeeded(),
        any_heuristics: round.any_heuristics(),
        any_unusable: round.unusable() > 0,
        winner: round.winner.clone(),
        decision: round.decision.clone(),
        scm_status: state.status.to_string(),
        next,
        timing,
        candidates,
    }
}

fn build_candidate(
    state: &ScmState,
    out_dir: &Path,
    round: &RoundRecord,
    cand: &CandidateRecord,
    alpha: Option<f64>,
    rank: Option<usize>,
    reference_parameters: Option<&ParameterTable>,
) -> CandidateSummary {
    let entry = state.roster_entry(&cand.candidate);
    let thetas: Vec<usize> = entry.map(|e| vec![e.candidate.theta]).unwrap_or_default();

    let reading = read_run(out_dir, &cand.model);
    let parameters = reading.parameters.as_ref().map(ParameterTable::from_table);

    // The effect's estimate in the model where it is free.
    let free_in = match round.direction {
        Direction::Forward => parameters.as_ref(),
        Direction::Backward => reference_parameters,
    };
    let effect_estimates = thetas
        .iter()
        .filter_map(|t| {
            let p = free_in?.find(&format!("THETA{t}"))?;
            Some(EffectEstimate {
                theta: *t,
                estimate: p.estimate,
                stderr: p.stderr,
                rse: p.rse,
                ci95_lower: p.ci95_lower,
                ci95_upper: p.ci95_upper,
            })
        })
        .collect();

    let iiv = match (&parameters, reference_parameters) {
        (Some(c), Some(r)) => c
            .omegas
            .iter()
            .filter(|o| o.diagonal)
            .filter_map(|o| {
                let reference = r.find(&o.name)?.estimate;
                Some(IivChange {
                    name: o.name.clone(),
                    reference,
                    candidate: o.estimate,
                    percent_change: (reference != 0.0)
                        .then(|| (o.estimate - reference) / reference * 100.0)
                        .filter(|v| v.is_finite()),
                })
            })
            .collect(),
        _ => vec![],
    };

    let statistic = cand.delta_ofv.map(|d| match round.direction {
        Direction::Forward => (-d).max(0.0),
        Direction::Backward => d.max(0.0),
    });
    let critical_delta_ofv = match (alpha, cand.df) {
        (Some(a), df) if df > 0 => Some(chi2_isf(a, df)).filter(|v| v.is_finite()),
        _ => None,
    };

    let attempts = cand
        .attempts
        .iter()
        .map(|a| AttemptSummary {
            model: a.model.clone(),
            outcome: a.outcome.clone(),
            timing: read_run(out_dir, &a.model).timing,
        })
        .collect::<Vec<_>>();
    // The candidate's span covers every attempt.
    let mut timing = Timing::default();
    for a in &attempts {
        timing.absorb(&a.timing);
    }
    if attempts.is_empty() {
        timing = reading.timing.clone();
    }

    CandidateSummary {
        candidate: cand.candidate.clone(),
        action: cand.action.clone(),
        status: cand.status.to_string(),
        model: cand.model.clone(),
        selected: cand.selected,
        rank,
        thetas,
        initial: entry.map(|e| e.candidate.initial),
        off: entry.map(|e| e.candidate.off),
        ofv: cand.ofv,
        reference_ofv: round.reference_ofv,
        delta_ofv: cand.delta_ofv,
        statistic,
        df: cand.df,
        p_value: cand.p_value,
        alpha,
        critical_delta_ofv,
        significant: cand.significant,
        heuristics: cand.heuristics.clone(),
        attempts,
        fit: fit_summary(&reading),
        effect_estimates,
        iiv,
        parameters,
        files: reading.files,
        timing,
    }
}

// ---------------------------------------------------------------------------
// Writing the record
// ---------------------------------------------------------------------------

/// Write the named round's summary (JSON + markdown) into its round
/// directory and refresh `scm_summary.json` in the out_dir. Returns the
/// round's (json path, md path).
pub fn write_round_summary(
    out_dir: &Path,
    plan: &ScmPlan,
    state: &ScmState,
    round_name: &str,
) -> Result<(PathBuf, PathBuf)> {
    let summary = build_summary(plan, state, out_dir);
    let round = summary
        .rounds
        .iter()
        .find(|r| r.round == round_name)
        .with_context(|| format!("no round named {round_name} in the state"))?;
    let record = state
        .rounds
        .iter()
        .find(|r| r.name == round_name)
        .expect("the summary was built from this state");
    let dir_name = round_dir(&round.round, &record.candidates)
        .with_context(|| format!("round {round_name} has no candidates to name its directory"))?;
    let dir = out_dir.join(dir_name);
    fs::create_dir_all(&dir)?;

    let json_path = dir.join(ROUND_SUMMARY_JSON);
    utils::write_json_to_file(round, &json_path)
        .with_context(|| format!("failed to write {}", json_path.display()))?;
    let md_path = dir.join(ROUND_SUMMARY_MD);
    fs::write(&md_path, round_summary_md(round))?;

    let process_path = out_dir.join(SCM_SUMMARY_FILENAME);
    utils::write_json_to_file(&summary, &process_path)
        .with_context(|| format!("failed to write {}", process_path.display()))?;
    Ok((json_path, md_path))
}

// ---------------------------------------------------------------------------
// Rendering options
// ---------------------------------------------------------------------------

/// How to order candidates within a round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortKey {
    /// Winner-first for the phase: smallest p in forward selection, largest
    /// p in backward elimination (the default).
    #[default]
    P,
    /// By ΔOFV, most negative first.
    Dofv,
    /// The plan's order (by theta).
    Name,
}

impl FromStr for SortKey {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "p" | "p_value" | "pvalue" => Ok(SortKey::P),
            "dofv" | "delta" | "delta_ofv" => Ok(SortKey::Dofv),
            "name" | "plan" | "theta" => Ok(SortKey::Name),
            _ => Err(format!("unknown sort key '{s}': expected p, dofv or name")),
        }
    }
}

/// What the candidate × round matrix shows in each cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatrixValue {
    #[default]
    P,
    Dofv,
}

impl FromStr for MatrixValue {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "p" | "p_value" | "pvalue" => Ok(MatrixValue::P),
            "dofv" | "delta" | "delta_ofv" => Ok(MatrixValue::Dofv),
            _ => Err(format!("unknown matrix value '{s}': expected p or dofv")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SummaryFormat {
    #[default]
    Text,
    Json,
    Markdown,
    Csv,
}

impl FromStr for SummaryFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "text" => Ok(SummaryFormat::Text),
            "json" => Ok(SummaryFormat::Json),
            "md" | "markdown" => Ok(SummaryFormat::Markdown),
            "csv" => Ok(SummaryFormat::Csv),
            _ => Err(format!(
                "unknown format '{s}': expected text, json, md or csv"
            )),
        }
    }
}

/// What `scm summary` shows. Every flag adds a stratum to the default view;
/// they stack.
#[derive(Debug, Clone, Default)]
pub struct SummaryOptions {
    /// Restrict to one round: the Nth SCM round ("2" / "round 2"), a round
    /// name ("forward_round1"), or "reference". A single round always lists
    /// its attempts.
    pub round: Option<String>,
    /// Restrict to one phase.
    pub phase: Option<Direction>,
    /// Trace one candidate through every round it was tested in.
    pub candidate: Option<String>,
    /// `-l`: absolute OFV, the LRT statistic against its critical value,
    /// the effect's estimate with RSE and CI, df, attempts, condition
    /// number and heuristics on every candidate line.
    pub long: bool,
    /// `-a`: what the default hides — the reference fit's line, withdrawn
    /// and unusable candidates' attempts, every attempt with its model path.
    pub all: bool,
    /// `-t`: start, end and wall time per fit and per round, estimation
    /// time and function evaluations, and totals.
    pub time: bool,
    /// `-p`: each round's winner's parameter table beside its reference,
    /// and the IIV change on every diagonal OMEGA.
    pub parameters: bool,
    /// `--matrix`: a candidates × rounds grid.
    pub matrix: Option<MatrixValue>,
    /// `--files`: run directory, .lst, .ext and summary JSON per candidate.
    pub files: bool,
    pub sort: SortKey,
    pub reverse: bool,
    /// Decimals for ΔOFV, OFV and estimates.
    pub digits: usize,
    pub format: SummaryFormat,
}

impl SummaryOptions {
    fn digits(&self) -> usize {
        if self.digits == 0 { 3 } else { self.digits }
    }
}

// ---------------------------------------------------------------------------
// Selecting
// ---------------------------------------------------------------------------

/// Find the round `selector` names: an exact round name ("forward_round1",
/// "reference"), or the Nth SCM round chronologically ("2" / "round 2" —
/// the reference fit is not a round).
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
    /// The rounds the options select, in order.
    pub fn select_rounds(&self, opts: &SummaryOptions) -> Result<Vec<&RoundSummary>> {
        if self.rounds.is_empty() && opts.round.is_some() {
            bail!("the SCM process has not started; no rounds to summarize");
        }
        let mut rounds: Vec<&RoundSummary> = match &opts.round {
            Some(sel) => vec![find_round(&self.rounds, sel)?],
            None => self.rounds.iter().collect(),
        };
        if let Some(phase) = opts.phase {
            rounds.retain(|r| r.direction == phase && r.index > 0);
        }
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

    /// A copy holding only the rounds the options select — what `--json`
    /// prints.
    pub fn filtered(&self, opts: &SummaryOptions) -> Result<ScmSummary> {
        let rounds = self.select_rounds(opts)?.into_iter().cloned().collect();
        Ok(ScmSummary {
            rounds,
            ..self.clone()
        })
    }

    /// Render per `opts.format`.
    pub fn render(&self, opts: &SummaryOptions) -> Result<String> {
        match opts.format {
            SummaryFormat::Text => self.render_text(opts),
            SummaryFormat::Json => Ok(serde_json::to_string_pretty(&self.filtered(opts)?)? + "\n"),
            SummaryFormat::Markdown => self.render_markdown(opts),
            SummaryFormat::Csv => self.render_csv(opts),
        }
    }
}

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

fn fmt_num(v: Option<f64>, digits: usize) -> String {
    match v {
        Some(v) => format!("{v:.digits$}"),
        None => "-".to_string(),
    }
}

fn fmt_signed(v: Option<f64>, digits: usize) -> String {
    match v {
        Some(v) => format!("{v:+.digits$}"),
        None => "-".to_string(),
    }
}

/// p-values: fixed below the 0.001 switch, scientific above it.
fn fmt_p(p: Option<f64>) -> String {
    match p {
        Some(p) if p >= 0.001 => format!("{p:.3}"),
        Some(p) => format!("{p:.1e}"),
        None => "-".to_string(),
    }
}

/// `0.412 (14.2%)`, `0.412 (-)`, or `-`.
fn fmt_estimate(e: Option<&EffectEstimate>, digits: usize) -> String {
    match e {
        Some(e) => match e.rse {
            Some(rse) => format!("{:.digits$} ({rse:.1}%)", e.estimate),
            None => format!("{:.digits$}", e.estimate),
        },
        None => "-".to_string(),
    }
}

fn fmt_ci(e: Option<&EffectEstimate>, digits: usize) -> String {
    match e.and_then(|e| e.ci95_lower.zip(e.ci95_upper)) {
        Some((lo, hi)) => format!("{lo:.digits$}, {hi:.digits$}"),
        None => "-".to_string(),
    }
}

fn fmt_duration(seconds: Option<f64>) -> String {
    let Some(s) = seconds else {
        return "-".to_string();
    };
    let total = s.round() as u64;
    let (h, m, sec) = (total / 3600, (total % 3600) / 60, total % 60);
    if h > 0 {
        format!("{h}h {m:02}m")
    } else if m > 0 {
        format!("{m}m {sec:02}s")
    } else {
        format!("{s:.1}s")
    }
}

/// `2026-09-08T16:02:15+00:00` -> `16:02:15`.
fn clock(ts: Option<&str>) -> String {
    ts.and_then(|t| t.get(11..19))
        .map(str::to_string)
        .unwrap_or_else(|| "-".to_string())
}

fn significance_mark(c: &CandidateSummary) -> &'static str {
    match c.significant {
        Some(true) => "*",
        _ => " ",
    }
}

/// Candidates of a round in the order the options ask for.
fn ordered<'a>(round: &'a RoundSummary, opts: &SummaryOptions) -> Vec<&'a CandidateSummary> {
    let mut cands: Vec<&CandidateSummary> = round.candidates.iter().collect();
    match opts.sort {
        SortKey::Name => {}
        SortKey::P | SortKey::Dofv => {
            // Scored first (in rank order / by ΔOFV), then the rest in plan order.
            cands.sort_by(|a, b| {
                let key = |c: &CandidateSummary| match opts.sort {
                    SortKey::Dofv => c.delta_ofv,
                    _ => c.rank.map(|r| r as f64),
                };
                match (key(a), key(b)) {
                    (Some(x), Some(y)) => x.total_cmp(&y),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            });
        }
    }
    if opts.reverse {
        cands.reverse();
    }
    cands
}

// ---------------------------------------------------------------------------
// Text rendering
// ---------------------------------------------------------------------------

impl ScmSummary {
    fn header_lines(&self, out: &mut Lines, opts: &SummaryOptions) {
        let n = self.totals.rounds_complete;
        let rounds = match n {
            0 => "no rounds complete".to_string(),
            1 => "1 round complete".to_string(),
            n => format!("{n} rounds complete"),
        };
        out.add(format!(
            "<scm summary> {}   {} · {} · {rounds}",
            self.out_dir,
            self.status,
            self.options.direction_label()
        ));
        let o = &self.options;
        let mut alphas = Vec::new();
        if o.runs_forward() {
            alphas.push(format!("forward {}", o.forward_alpha));
        }
        if o.runs_backward() {
            alphas.push(format!("backward {}", o.backward_alpha));
        }
        out.add(format!(
            "model      : {}   alphas: {}   cov step: {}",
            self.template_model,
            alphas.join(", "),
            on_off(o.cov_step)
        ));
        if let Some(p) = &self.phase {
            out.add(format!("phase      : {p}"));
        }
        if let Some(m) = &self.message {
            out.add(format!("note       : {m}"));
        }
        if let Some(tie) = &self.pending_tie {
            out.add(format!(
                "awaiting   : your decision on {} in {} — re-run with --choose <candidate>",
                tie.candidates.join(" / "),
                tie.round
            ));
        }
        out.add(format!("retained   : {}", none_or_list(&self.retained)));
        let removed: Vec<String> = self
            .roster
            .iter()
            .filter(|e| e.removed.is_some())
            .map(|e| e.removal_label())
            .collect();
        if !removed.is_empty() {
            out.add(format!("removed    : {}", removed.join(", ")));
        }
        if let Some(f) = &self.final_model {
            out.add(format!("final model: {f}{}", ofv_suffix(self.final_ofv)));
        }
        if opts.time {
            let t = &self.totals.timing;
            out.add(format!(
                "time       : {} fit{} ({} retr{}) · wall {} ({} → {}) · est time {}",
                self.totals.models_fitted,
                if self.totals.models_fitted == 1 {
                    ""
                } else {
                    "s"
                },
                self.totals.retries,
                if self.totals.retries == 1 { "y" } else { "ies" },
                fmt_duration(t.wall_seconds),
                clock(t.started.as_deref()),
                clock(t.ended.as_deref()),
                fmt_duration(t.estimation_seconds)
            ));
        }
    }

    pub fn render_text(&self, opts: &SummaryOptions) -> Result<String> {
        let mut out = Lines::new();
        let rounds = self.select_rounds(opts)?;
        self.header_lines(&mut out, opts);

        if let Some(name) = &opts.candidate {
            out.blank();
            self.render_candidate_trace(&mut out, name, &rounds, opts);
            return Ok(out.finish());
        }
        if let Some(value) = opts.matrix {
            out.blank();
            self.render_matrix(&mut out, value, opts);
            if !opts.long && !opts.all && !opts.time && !opts.parameters {
                out.add("records    : scm_summary.json · scm_decision_log.{csv,md} · round_summary.{json,md} in each round dir");
                return Ok(out.finish());
            }
        }

        let single = opts.round.is_some();
        if self.rounds.is_empty() {
            out.blank();
            out.add("no rounds yet: the plan is written and the SCM process has not started");
            return Ok(out.finish());
        }
        for round in &rounds {
            out.blank();
            if round.index == 0 {
                self.render_reference(&mut out, round, opts, single);
            } else {
                self.render_round(&mut out, round, opts, single);
            }
        }
        out.blank();
        out.add("records    : scm_summary.json · scm_decision_log.{csv,md} · round_summary.{json,md} in each round dir");
        Ok(out.finish())
    }

    fn render_reference(
        &self,
        out: &mut Lines,
        round: &RoundSummary,
        opts: &SummaryOptions,
        single: bool,
    ) {
        let Some(c) = round.candidates.first() else {
            return;
        };
        let mut line = format!(
            "reference  {:<44} {:<10} OFV {}",
            c.model,
            c.status,
            fmt_num(c.ofv, opts.digits())
        );
        if opts.time {
            write!(line, "   {}", timing_suffix(&c.timing)).unwrap();
        }
        out.add(line);
        if !round.complete {
            out.add("           in progress");
        }
        if opts.all || single || opts.long {
            self.render_attempts(out, c, opts);
        }
        if opts.parameters
            && let Some(p) = &c.parameters
        {
            out.add("           parameters:");
            for est in p.all() {
                out.add(format!(
                    "             {:<12} {:>12}{}",
                    est.name,
                    fmt_num(Some(est.estimate), opts.digits()),
                    est.rse
                        .map(|r| format!("  (RSE {r:.1}%)"))
                        .unwrap_or_default()
                ));
            }
        }
    }

    fn render_round(
        &self,
        out: &mut Lines,
        round: &RoundSummary,
        opts: &SummaryOptions,
        single: bool,
    ) {
        let d = opts.digits();
        // Headline: round, reference, alpha, critical value, decision.
        let crit = round
            .candidates
            .iter()
            .find(|c| c.df == 1)
            .and_then(|c| c.critical_delta_ofv)
            .or_else(|| round.candidates.iter().find_map(|c| c.critical_delta_ofv));
        let mut head = format!(
            "{:<16} ref OFV {} · alpha {} · crit dOFV {}",
            round.round,
            fmt_num(round.reference_ofv, d),
            round.alpha.map(|a| a.to_string()).unwrap_or_default(),
            fmt_num(crit, d)
        );
        if round.complete {
            write!(head, " · {}", round.decision).unwrap();
        } else {
            let concluded = round.counts.succeeded + round.counts.unusable + round.counts.withdrawn;
            write!(
                head,
                " · in progress — {concluded}/{} concluded",
                round.counts.candidates
            )
            .unwrap();
            if !round.decision.is_empty() {
                write!(head, " ({})", round.decision).unwrap();
            }
        }
        out.add(head);
        if opts.long || single {
            out.add(format!(
                "                 reference {} · retained before: {} · after: {}",
                round.reference_model,
                none_or_list(&round.retained_before),
                none_or_list(&round.retained_after)
            ));
        }
        if !round.removed_before.is_empty() && (opts.all || single) {
            out.add(format!(
                "                 removed before this round: {}",
                round.removed_before.join(", ")
            ));
        }
        if opts.time {
            let t = &round.timing;
            let fits: usize = round.candidates.iter().map(|c| c.attempts.len()).sum();
            out.add(format!(
                "                 {fits} fit{} · wall {} ({} → {}) · est time {}",
                if fits == 1 { "" } else { "s" },
                fmt_duration(t.wall_seconds),
                clock(t.started.as_deref()),
                clock(t.ended.as_deref()),
                fmt_duration(t.estimation_seconds)
            ));
        }

        if opts.long {
            out.add(format!(
                "  {:<12} {:>12} {:>10} {:>8} {:>9}    {:<22} {:<22} {:>2} {:>5} {:>7}  flags",
                "candidate",
                "OFV",
                "dOFV",
                "LRT",
                "p",
                "est (RSE%)",
                "CI95",
                "df",
                "tries",
                "cond#"
            ));
        }
        for c in ordered(round, opts) {
            let mut line = if opts.long {
                format!(
                    "  {:<12} {:>12} {:>10} {:>8} {:>9} {}  {:<22} {:<22} {:>2} {:>5} {:>7}",
                    c.candidate,
                    fmt_num(c.ofv, d),
                    fmt_signed(c.delta_ofv, d),
                    fmt_num(c.statistic, 2),
                    fmt_p(c.p_value),
                    significance_mark(c),
                    fmt_estimate(c.effect_estimates.first(), d),
                    fmt_ci(c.effect_estimates.first(), d),
                    c.df,
                    c.attempts.len(),
                    c.fit
                        .as_ref()
                        .and_then(|f| f.condition_number)
                        .map(|v| format!("{v:.0}"))
                        .unwrap_or_else(|| "-".to_string()),
                )
            } else {
                format!(
                    "  {:<12} dOFV {:>10}   p {:<9} {}",
                    c.candidate,
                    fmt_signed(c.delta_ofv, d),
                    fmt_p(c.p_value),
                    significance_mark(c)
                )
            };
            // status, when it is not the plain success the numbers imply
            match c.status.as_str() {
                "succeeded" => {}
                other => write!(line, "  {other}").unwrap(),
            }
            if c.selected {
                let verb = match round.direction {
                    Direction::Forward => "selected",
                    Direction::Backward => "dropped",
                };
                write!(line, "  <- {verb}").unwrap();
            } else if round.direction == Direction::Backward
                && round.complete
                && c.significant == Some(true)
            {
                line.push_str("  kept");
            }
            if opts.long && !c.heuristics.is_empty() {
                write!(line, "  {}", c.heuristics.join(", ")).unwrap();
            }
            if opts.time {
                write!(line, "   {}", timing_suffix(&c.timing)).unwrap();
            }
            out.add(line);

            let show_attempts = opts.all
                || single
                || (c.status != "succeeded" && c.status != "pending" && opts.long);
            if show_attempts {
                self.render_attempts(out, c, opts);
            }
            if !opts.long && !c.heuristics.is_empty() && (opts.all || single) {
                out.add(format!("      heuristics: {}", c.heuristics.join(", ")));
            }
            if opts.files {
                let f = &c.files;
                out.add(format!(
                    "      files: run dir {} · lst {} · ext {} · summary {}",
                    f.run_dir.as_deref().unwrap_or("-"),
                    f.lst.as_deref().unwrap_or("-"),
                    f.ext.as_deref().unwrap_or("-"),
                    f.summary_json.as_deref().unwrap_or("-")
                ));
            }
        }

        if opts.parameters {
            self.render_parameters(out, round, opts);
        }
    }

    fn render_attempts(&self, out: &mut Lines, c: &CandidateSummary, opts: &SummaryOptions) {
        for a in &c.attempts {
            let mut line = format!("      {:<44} {}", a.model, a.outcome);
            if opts.time {
                write!(line, "   {}", timing_suffix(&a.timing)).unwrap();
            }
            out.add(line);
        }
        // The attempts list is empty until a model is dispatched; the model
        // field still points at the run when one exists.
        if c.attempts.is_empty() && !c.model.is_empty() {
            out.add(format!("      {:<44} {}", c.model, c.status));
        }
    }

    /// The winner's parameters beside the reference's, and the IIV change.
    fn render_parameters(&self, out: &mut Lines, round: &RoundSummary, opts: &SummaryOptions) {
        let d = opts.digits();
        let Some(winner) = round
            .candidates
            .iter()
            .find(|c| c.selected)
            .or_else(|| round.candidates.iter().find(|c| c.rank == Some(1)))
        else {
            return;
        };
        let (Some(params), Some(reference)) = (&winner.parameters, &round.reference_parameters)
        else {
            out.add("                 parameters: not readable for this round");
            return;
        };
        out.add(format!(
            "                 parameters — reference vs {} ({}):",
            winner.candidate,
            if winner.selected { "winner" } else { "best" }
        ));
        out.add(format!(
            "                   {:<12} {:>12} {:>12} {:>9}",
            "parameter", "reference", winner.candidate, "change"
        ));
        for p in params.all() {
            let r = reference.find(&p.name);
            let change = match r {
                Some(r) if r.estimate != 0.0 && !p.fixed => {
                    format!("{:+.1}%", (p.estimate - r.estimate) / r.estimate * 100.0)
                }
                _ => "-".to_string(),
            };
            out.add(format!(
                "                   {:<12} {:>12} {:>12} {:>9}",
                p.name,
                r.map(|r| fmt_num(Some(r.estimate), d))
                    .unwrap_or_else(|| "-".to_string()),
                fmt_num(Some(p.estimate), d),
                change
            ));
        }
        if !winner.iiv.is_empty() {
            let iiv: Vec<String> = winner
                .iiv
                .iter()
                .map(|i| {
                    format!(
                        "{} {}",
                        i.name,
                        i.percent_change
                            .map(|c| format!("{c:+.1}%"))
                            .unwrap_or_else(|| "-".to_string())
                    )
                })
                .collect();
            out.add(format!("                   IIV change: {}", iiv.join(", ")));
        }
    }

    /// One candidate through every round it was tested in.
    fn render_candidate_trace(
        &self,
        out: &mut Lines,
        name: &str,
        rounds: &[&RoundSummary],
        opts: &SummaryOptions,
    ) {
        let d = opts.digits();
        let entry = self
            .roster
            .iter()
            .find(|e| e.candidate.name.eq_ignore_ascii_case(name));
        let Some(entry) = entry else {
            return;
        };
        let c = &entry.candidate;
        let tested = rounds.iter().filter(|r| r.index > 0).count();
        let bounds = match c.bounds_label() {
            Some(b) => format!(", bounds {b}"),
            None => String::new(),
        };
        let mut head = format!(
            "{}  THETA({})  initial {}, off {}{bounds}  ·  tested {tested}×",
            c.name, c.theta, c.initial, c.off
        );
        if let Some(won) = rounds
            .iter()
            .find(|r| r.winner.as_deref() == Some(c.name.as_str()))
        {
            write!(
                head,
                ", {} in {}",
                match won.direction {
                    Direction::Forward => "selected",
                    Direction::Backward => "dropped",
                },
                won.round
            )
            .unwrap();
        } else if let Some(removal) = &entry.removed {
            write!(head, ", never selected, removed {}", removal.when_label()).unwrap();
        } else if tested > 0 {
            head.push_str(", never selected");
        }
        out.add(head);

        let mut best: Option<(f64, &str, Option<f64>)> = None;
        for round in rounds.iter().filter(|r| r.index > 0) {
            let Some(cand) = round
                .candidates
                .iter()
                .find(|x| x.candidate.eq_ignore_ascii_case(name))
            else {
                continue;
            };
            let mut line = format!(
                "  {:<16} dOFV {:>10}   p {:<9} {}  est {:<18} {}",
                round.round,
                fmt_signed(cand.delta_ofv, d),
                fmt_p(cand.p_value),
                significance_mark(cand),
                fmt_estimate(cand.effect_estimates.first(), d),
                cand.status
            );
            if cand.selected {
                line.push_str("  <- selected");
            }
            if opts.time {
                write!(line, "   {}", timing_suffix(&cand.timing)).unwrap();
            }
            out.add(line);
            if opts.all {
                self.render_attempts(out, cand, opts);
            }
            if round.direction == Direction::Forward
                && let Some(p) = cand.p_value
                && best.is_none_or(|(bp, _, _)| p < bp)
            {
                best = Some((p, &round.round, cand.critical_delta_ofv));
            }
        }
        if let Some((p, round, crit)) = best
            && !rounds
                .iter()
                .any(|r| r.winner.as_deref() == Some(c.name.as_str()))
        {
            let mut line = format!("  smallest p across rounds: {} ({round})", fmt_p(Some(p)));
            if let Some(crit) = crit {
                write!(
                    line,
                    "; would have needed dOFV ≤ {}",
                    fmt_signed(Some(-crit), d)
                )
                .unwrap();
            }
            out.add(line);
        }
    }

    /// Candidates × rounds.
    fn render_matrix(&self, out: &mut Lines, value: MatrixValue, opts: &SummaryOptions) {
        let d = opts.digits();
        let rounds: Vec<&RoundSummary> = self.rounds.iter().filter(|r| r.index > 0).collect();
        if rounds.is_empty() {
            out.add("matrix     : no SCM rounds yet");
            return;
        }
        let width = 10usize;
        let short = |r: &RoundSummary| {
            let prefix = match r.direction {
                Direction::Forward => "fwd",
                Direction::Backward => "bwd",
            };
            format!("{prefix}{}", r.phase_index)
        };
        let label = match value {
            MatrixValue::P => "p-value",
            MatrixValue::Dofv => "dOFV",
        };
        let mut head = format!("{label:<14}");
        for r in &rounds {
            write!(head, "{:>width$}", short(r)).unwrap();
        }
        out.add(head);

        for entry in &self.roster {
            let name = &entry.candidate.name;
            let mut line = format!("{name:<14}");
            let mut removed_shown = false;
            for r in &rounds {
                let cell = match r.candidates.iter().find(|c| &c.candidate == name) {
                    Some(c) => {
                        let v = match value {
                            MatrixValue::P => fmt_p(c.p_value),
                            MatrixValue::Dofv => fmt_signed(c.delta_ofv, d),
                        };
                        let v = match c.status.as_str() {
                            "succeeded" => v,
                            "withdrawn" => "withdrawn".to_string(),
                            "unusable" => "unusable".to_string(),
                            other => other.to_string(),
                        };
                        if c.selected { format!("[{v}]") } else { v }
                    }
                    None if r.removed_before.contains(name) => {
                        if removed_shown {
                            String::new()
                        } else {
                            removed_shown = true;
                            "removed".to_string()
                        }
                    }
                    None => "·".to_string(),
                };
                write!(line, "{cell:>width$}").unwrap();
            }
            out.add(line.trim_end());
        }
        let mut footer = format!("{:<14}", "retained");
        for r in &rounds {
            let cell = match (&r.winner, r.direction) {
                (Some(w), Direction::Forward) => format!("+{w}"),
                (Some(w), Direction::Backward) => format!("-{w}"),
                (None, _) if r.complete => "(none)".to_string(),
                (None, _) => "…".to_string(),
            };
            write!(footer, "{cell:>width$}").unwrap();
        }
        out.add(footer);
        out.add(
            "[ ] winner · dashes: not tested in that round · retained row: what each round changed",
        );
    }
}

fn timing_suffix(t: &Timing) -> String {
    format!(
        "{} → {}  {}",
        clock(t.started.as_deref()),
        clock(t.ended.as_deref()),
        fmt_duration(t.wall_seconds)
    )
}

// ---------------------------------------------------------------------------
// Markdown and CSV
// ---------------------------------------------------------------------------

/// The markdown record of one round, written beside its JSON.
pub fn round_summary_md(round: &RoundSummary) -> String {
    let mut out = Lines::new();
    out.add(format!("# {}", round.round));
    out.blank();
    out.add(format!("- template: `{}`", round.template_model));
    out.add(format!("- direction: {}", round.direction));
    if round.reference_model != NO_REFERENCE {
        out.add(format!(
            "- reference: `{}`{}",
            round.reference_model,
            ofv_suffix(round.reference_ofv)
        ));
    }
    if let Some(a) = round.alpha {
        out.add(format!("- alpha: {a}"));
    }
    out.add(format!(
        "- all fits succeeded: {}",
        yes_no(round.all_succeeded)
    ));
    out.add(format!(
        "- heuristic checks fired: {}",
        yes_no(round.any_heuristics)
    ));
    out.add(format!(
        "- unusable candidates: {}",
        yes_no(round.any_unusable)
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
    out.add(format!(
        "- retained after this round: {}",
        none_or_list(&round.retained_after)
    ));
    if let Some(w) = round.timing.wall_seconds {
        out.add(format!("- wall time: {}", fmt_duration(Some(w))));
    }
    out.add(format!("- next: {}", round.next));
    out.blank();
    add_candidate_table(&mut out, round);
    out.finish()
}

fn add_candidate_table(out: &mut Lines, round: &RoundSummary) {
    out.add(
        "| candidate | model | attempts | status | OFV | ΔOFV | crit ΔOFV | df | p | significant | selected | estimate (RSE%) | cond# | heuristic checks |",
    );
    out.add("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for c in &round.candidates {
        out.add(format!(
            "| {} | `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            c.candidate,
            c.model,
            c.attempts.len(),
            c.status,
            fmt_num(c.ofv, 3),
            c.delta_ofv.map(|v| format!("{v:.3}")).unwrap_or_default(),
            c.critical_delta_ofv
                .map(|v| format!("{v:.3}"))
                .unwrap_or_default(),
            c.df,
            c.p_value.map(|p| format!("{p:.4e}")).unwrap_or_default(),
            c.significant.map(yes_no).unwrap_or_default(),
            if c.selected { "**yes**" } else { "" },
            match c.effect_estimates.first() {
                Some(e) => fmt_estimate(Some(e), 3),
                None => String::new(),
            },
            c.fit
                .as_ref()
                .and_then(|f| f.condition_number)
                .map(|v| format!("{v:.0}"))
                .unwrap_or_default(),
            if c.heuristics.is_empty() {
                "-".to_string()
            } else {
                c.heuristics.join("; ")
            },
        ));
    }
}

impl ScmSummary {
    fn render_markdown(&self, opts: &SummaryOptions) -> Result<String> {
        let rounds = self.select_rounds(opts)?;
        let mut out = Lines::new();
        out.add("# SCM summary");
        out.blank();
        out.add(format!("- model: `{}`", self.template_model));
        out.add(format!("- out dir: `{}`", self.out_dir));
        out.add(format!("- status: {}", self.status));
        out.add(format!("- direction: {}", self.options.direction_label()));
        out.add(format!(
            "- alphas: forward {}, backward {}",
            self.options.forward_alpha, self.options.backward_alpha
        ));
        out.add(format!("- retained: {}", none_or_list(&self.retained)));
        let removed: Vec<String> = self
            .roster
            .iter()
            .filter(|e| e.removed.is_some())
            .map(|e| e.removal_label())
            .collect();
        if !removed.is_empty() {
            out.add(format!("- removed: {}", removed.join(", ")));
        }
        if let Some(f) = &self.final_model {
            out.add(format!(
                "- final model: `{f}`{}",
                ofv_suffix(self.final_ofv)
            ));
        }
        out.blank();
        for round in rounds {
            out.add(format!("## {}", round.round));
            out.blank();
            if round.has_reference() {
                out.add(format!(
                    "Reference: `{}`{} · alpha {}",
                    round.reference_model,
                    ofv_suffix(round.reference_ofv),
                    round.alpha.map(|a| a.to_string()).unwrap_or_default()
                ));
                out.blank();
            }
            add_candidate_table(&mut out, round);
            out.blank();
            if !round.decision.is_empty() {
                out.add(format!("**Decision:** {}", round.decision));
                out.blank();
            }
        }
        Ok(out.finish())
    }

    fn render_csv(&self, opts: &SummaryOptions) -> Result<String> {
        let rounds = self.select_rounds(opts)?;
        let mut lines = vec![
            "round,direction,candidate,status,model,attempts,ofv,reference_ofv,delta_ofv,statistic,df,p_value,alpha,critical_delta_ofv,significant,selected,rank,theta,initial,off,estimate,stderr,rse,condition_number,heuristics,wall_seconds"
                .to_string(),
        ];
        let f = |v: Option<f64>| v.map(|v| v.to_string()).unwrap_or_default();
        for r in rounds {
            for c in &r.candidates {
                let e = c.effect_estimates.first();
                let fields = [
                    r.round.clone(),
                    r.direction.to_string(),
                    c.candidate.clone(),
                    c.status.clone(),
                    c.model.clone(),
                    c.attempts.len().to_string(),
                    f(c.ofv),
                    f(c.reference_ofv),
                    f(c.delta_ofv),
                    f(c.statistic),
                    c.df.to_string(),
                    f(c.p_value),
                    f(c.alpha),
                    f(c.critical_delta_ofv),
                    c.significant.map(|s| s.to_string()).unwrap_or_default(),
                    c.selected.to_string(),
                    c.rank.map(|r| r.to_string()).unwrap_or_default(),
                    c.thetas
                        .iter()
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>()
                        .join(";"),
                    f(c.initial),
                    f(c.off),
                    f(e.map(|e| e.estimate)),
                    f(e.and_then(|e| e.stderr)),
                    f(e.and_then(|e| e.rse)),
                    f(c.fit.as_ref().and_then(|x| x.condition_number)),
                    c.heuristics.join("; "),
                    f(c.timing.wall_seconds),
                ];
                lines.push(
                    fields
                        .iter()
                        .map(|x| csv_escape(x))
                        .collect::<Vec<_>>()
                        .join(","),
                );
            }
        }
        Ok(lines.join("\n") + "\n")
    }
}

impl RoundSummary {
    pub fn has_reference(&self) -> bool {
        self.reference_model != NO_REFERENCE
    }
}

fn csv_escape(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scm::test_support::{full_scm_executor, make_plan};
    use crate::scm::{ScmOptions, run_scm};

    fn completed(dir: &Path) -> (ScmPlan, ScmSummary) {
        let plan = make_plan(dir, ScmOptions::default());
        run_scm(&plan, &full_scm_executor(), None).unwrap();
        let summary = read_summary(&plan.out_dir_path()).unwrap();
        (plan, summary)
    }

    #[test]
    fn the_summary_carries_scoring_estimates_and_files_for_every_round() {
        let dir = tempfile::tempdir().unwrap();
        let (plan, summary) = completed(dir.path());

        assert_eq!(summary.status, "completed");
        assert_eq!(summary.totals.rounds_complete, 5);
        assert_eq!(summary.totals.models_fitted, 11);
        assert_eq!(summary.totals.retries, 1);
        assert_eq!(summary.retained, vec!["WT_CL".to_string()]);
        assert_eq!(summary.final_ofv, Some(980.0));
        assert_eq!(summary.roster.len(), 3);

        let r1 = &summary.rounds[1];
        assert_eq!((r1.index, r1.phase_index), (1, 1));
        assert_eq!(r1.alpha, Some(0.05));
        assert!(r1.retained_before.is_empty());
        assert_eq!(r1.retained_after, vec!["WT_CL".to_string()]);
        let wt_cl = r1
            .candidates
            .iter()
            .find(|c| c.candidate == "WT_CL")
            .unwrap();
        assert_eq!(wt_cl.rank, Some(1));
        assert_eq!(wt_cl.thetas, vec![4]);
        assert_eq!((wt_cl.initial, wt_cl.off), (Some(0.1), Some(0.0)));
        assert_eq!(wt_cl.statistic, Some(20.0));
        assert!((wt_cl.critical_delta_ofv.unwrap() - 3.841).abs() < 1e-3);
        // the mocked .ext reports THETA4 = 0.25 in the final row
        assert_eq!(wt_cl.effect_estimates[0].theta, 4);
        assert!((wt_cl.effect_estimates[0].estimate - 0.25).abs() < 1e-9);
        assert!(wt_cl.fit.is_some());
        assert!(
            wt_cl
                .files
                .ext
                .as_deref()
                .unwrap()
                .ends_with("1001_wt_cl.ext")
        );
        assert!(wt_cl.parameters.as_ref().unwrap().omegas.len() >= 2);
        assert_eq!(wt_cl.iiv.len(), 2);

        // backward: the dropped candidate's estimate comes from the reference
        let b1 = &summary.rounds[4];
        assert_eq!(b1.direction, Direction::Backward);
        let crcl = b1
            .candidates
            .iter()
            .find(|c| c.candidate == "CRCL_CL")
            .unwrap();
        assert!(crcl.selected);
        assert_eq!(crcl.effect_estimates[0].theta, 5);
        assert_eq!(crcl.rank, Some(1));

        // the files on disk are the same record
        let on_disk: RoundSummary = serde_json::from_str(
            &fs::read_to_string(
                plan.out_dir_path()
                    .join("forward_round1/round_summary.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(on_disk.candidates.len(), r1.candidates.len());
        assert_eq!(on_disk.winner, r1.winner);
        let process: ScmSummary = serde_json::from_str(
            &fs::read_to_string(plan.out_dir_path().join(SCM_SUMMARY_FILENAME)).unwrap(),
        )
        .unwrap();
        assert_eq!(process.rounds.len(), summary.rounds.len());
        assert_eq!(process.status, "completed");
    }

    #[test]
    fn selection_and_sorting_follow_the_options() {
        let dir = tempfile::tempdir().unwrap();
        let (_, summary) = completed(dir.path());

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

        let backward = summary
            .select_rounds(&SummaryOptions {
                phase: Some(Direction::Backward),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(backward.len(), 2);

        let traced = summary
            .select_rounds(&SummaryOptions {
                candidate: Some("wt_v".into()),
                ..Default::default()
            })
            .unwrap();
        // WT_V was tested in the three forward rounds only
        assert_eq!(traced.len(), 3);

        // default order: winner first; name order: plan order
        let r1 = &summary.rounds[1];
        let by_p: Vec<&str> = ordered(r1, &SummaryOptions::default())
            .iter()
            .map(|c| c.candidate.as_str())
            .collect();
        assert_eq!(by_p, vec!["WT_CL", "CRCL_CL", "WT_V"]);
        let by_name: Vec<&str> = ordered(
            r1,
            &SummaryOptions {
                sort: SortKey::Name,
                reverse: true,
                ..Default::default()
            },
        )
        .iter()
        .map(|c| c.candidate.as_str())
        .collect();
        assert_eq!(by_name, vec!["WT_V", "CRCL_CL", "WT_CL"]);
    }

    #[test]
    fn every_format_renders() {
        let dir = tempfile::tempdir().unwrap();
        let (_, summary) = completed(dir.path());
        for format in [
            SummaryFormat::Text,
            SummaryFormat::Json,
            SummaryFormat::Markdown,
            SummaryFormat::Csv,
        ] {
            let text = summary
                .render(&SummaryOptions {
                    format,
                    long: true,
                    all: true,
                    time: true,
                    parameters: true,
                    files: true,
                    ..Default::default()
                })
                .unwrap();
            assert!(text.contains("forward_round1"), "{format:?}:\n{text}");
        }
        let json = summary
            .render(&SummaryOptions {
                format: SummaryFormat::Json,
                round: Some("2".into()),
                ..Default::default()
            })
            .unwrap();
        let filtered: ScmSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(filtered.rounds.len(), 1);
        assert_eq!(filtered.rounds[0].round, "forward_round2");
    }

    #[test]
    fn a_planned_process_summarises_to_its_plan() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        plan.save().unwrap();
        let summary = read_summary(&plan.out_dir_path()).unwrap();
        assert_eq!(summary.status, "planned");
        assert!(summary.rounds.is_empty());
        let text = summary.render_text(&SummaryOptions::default()).unwrap();
        assert!(text.contains("has not started"), "{text}");
        let err = summary
            .select_rounds(&SummaryOptions {
                round: Some("1".into()),
                ..Default::default()
            })
            .unwrap_err();
        assert!(err.to_string().contains("not started"), "{err}");
    }

    #[test]
    fn durations_and_clocks_format() {
        assert_eq!(fmt_duration(None), "-");
        assert_eq!(fmt_duration(Some(12.34)), "12.3s");
        assert_eq!(fmt_duration(Some(125.0)), "2m 05s");
        assert_eq!(fmt_duration(Some(3725.0)), "1h 02m");
        assert_eq!(clock(Some("2026-09-08T16:02:15+00:00")), "16:02:15");
        assert_eq!(clock(None), "-");
        assert_eq!(
            seconds_between(
                Some("2026-09-08T16:00:00+00:00"),
                Some("2026-09-08T16:01:30+00:00")
            ),
            Some(90.0)
        );
        assert_eq!(fmt_p(Some(0.0456)), "0.046");
        assert_eq!(fmt_p(Some(7.744e-6)), "7.7e-6");
    }
}
