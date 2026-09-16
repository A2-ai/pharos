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
//! the flags stack detail onto it. The markdown renderings are the files
//! the driver writes, not terminal output. The brief rendering (what `scm
//! status` prints) is the header and one line per round: where the SCM
//! process stands and what to do next, without the candidate rows.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use fs_err as fs;
use nonmem_parser::Transform;
use serde::{Deserialize, Serialize};
use utils::{clock, format_duration as fmt_duration, get_utc_now, seconds_between};

use super::project::RunSettings;
use super::roster::RosterEntry;
use super::round::{ext_path_in, run_dir_in, run_summary};
use super::score::chi2_isf;
use super::state::{
    CandidateRecord, CandidateStatus, PendingTie, RoundRecord, ScmProcess, ScmRunStatus, ScmState,
};
use super::{
    Direction, Lines, NO_REFERENCE, REFERENCE_ROUND, ROUND_SUMMARY_JSON, ROUND_SUMMARY_MD,
    RUN_SUMMARY_FILENAME, SCM_SUMMARY_FILENAME, SCM_SUMMARY_MD, ScmOptions, ScmPlan, none_or_list,
    ofv_suffix, on_off, yes_no,
};
use crate::output_files::ext::{TableParameters, ThetaEstimate};
use crate::run::metadata::{RUN_END_FILENAME, RUN_START_FILENAME, RunEndFile, RunStartFile};
use crate::{ModelLayout, output_files::Summary};

/// Schema 4: a candidate no longer repeats its round's `reference_ofv` and
/// `alpha`; they are written once, on the round.
///
/// Schema 3: a candidate carries where its run put its files, not a copy of
/// what is in them. The estimates, fit quality and IIV that schema 2 embedded
/// (`fit`, `parameters`, `effect_estimates`, `iiv`, `reference_parameters`)
/// are read back from each run's own `pharos_summary.json` instead — see
/// [`Fits`] — and a round now records `out_dir` and `reference_files` so it
/// can find them from wherever it is read.
///
/// Schema 2 added the heavy record (scoring, estimates, fit quality, IIV,
/// timing, files) over the schema-1 round summary, which carried the
/// candidate records alone.
pub const SUMMARY_SCHEMA_VERSION: u32 = 4;

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
    pub initial_model: String,
    pub out_dir: String,
    pub options: ScmOptions,
    /// planned | running | paused | completed | failed
    pub status: String,
    pub message: Option<String>,
    pub phase: Option<String>,
    /// When the state was last written; `None` for a planned but unstarted
    /// process, which has no state.
    pub updated: Option<String>,
    /// Models with a started but unfinished run right now (relative to
    /// out_dir).
    pub models_running: Vec<String>,
    /// The plan's candidates, in plan order.
    pub candidates: Vec<String>,
    /// Every candidate the SCM process has known, removed ones included.
    pub roster: Vec<RosterEntry>,
    /// Covariates in the model now, in selection order.
    pub retained: Vec<String>,
    pub final_model: Option<String>,
    /// The final model's own OFV once it has been fitted (`final_cov_step`);
    /// otherwise the last reference fit's, which is what the unfitted final
    /// model was assembled from.
    pub final_ofv: Option<f64>,
    pub pending_tie: Option<PendingTie>,
    pub totals: Totals,
    pub rounds: Vec<RoundSummary>,
    /// The directory this summary was read from, as the caller named it.
    /// `out_dir` is project-root-relative, so it does not resolve from an
    /// arbitrary working directory; the fits are read from here instead.
    /// Never serialized — it is where the reader stood, not part of the
    /// record.
    #[serde(skip)]
    pub base: PathBuf,
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
    pub initial_model: String,
    /// The SCM process's output directory, which every path in this record
    /// is relative to — so a round summary read out of its own round
    /// directory can still find the runs it names.
    pub out_dir: String,
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
    /// Where the reference fit's run left its files, so the round can be
    /// asked for the estimates it was scored against.
    pub reference_files: RunFiles,
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
    /// The effect's initial estimate the first time it was tested, and what
    /// it is fixed at when held out (from the roster; `None` for reference
    /// fits).
    pub initial: Option<f64>,
    pub fixed: Option<f64>,
    pub ofv: Option<f64>,
    /// candidate OFV − the round's reference OFV (negative = candidate improves).
    pub delta_ofv: Option<f64>,
    /// The tested statistic (never negative; a "wrong-way" delta clamps to 0).
    pub statistic: Option<f64>,
    pub df: usize,
    pub p_value: Option<f64>,
    /// The |ΔOFV| the candidate needs to reach the round's alpha at its df.
    pub critical_delta_ofv: Option<f64>,
    pub significant: Option<bool>,
    /// Heuristic checks that fired for the scoring attempt.
    pub heuristics: Vec<String>,
    pub attempts: Vec<AttemptSummary>,
    /// Attempts made before the candidate's initial estimate or bounds were
    /// retuned mid-round: fitted, kept on disk, never scored.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub superseded: Vec<AttemptSummary>,
    /// Where the scoring attempt's run left its files. How the fit itself
    /// went — its parameter table, condition number, termination and
    /// heuristics — is in the `pharos_summary.json` named here, and is read
    /// back through [`Fits`] rather than copied into this record.
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
///
/// `summary_json` is the whole record of how the fit went — the same JSON
/// `pharos nonmem summary --json` prints — which is why the SCM summary
/// carries the path rather than a copy of what is in it (see [`Fits`]).
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

// ---------------------------------------------------------------------------
// Reading a run
// ---------------------------------------------------------------------------

/// Everything a run's output directory says about it.
#[derive(Default)]
struct RunReading {
    /// The run's `pharos nonmem summary`, when it finished and its output
    /// could be read.
    summary: Option<Summary>,
    timing: Timing,
    files: RunFiles,
}

/// What a summary is built against, and every run read so far.
///
/// A run costs a run-directory lookup and a parse of its control stream,
/// and the same model is asked for repeatedly: a candidate's scoring model
/// is also its last attempt, and a round's reference model is the previous
/// round's winner. Each is read once here. `project_root` is resolved once
/// for the same reason — a lookup per run re-walks the tree to one
/// pharos.toml.
struct Build<'a> {
    plan: &'a ScmPlan,
    state: &'a ScmState,
    out_dir: &'a Path,
    settings: &'a RunSettings,
    project_root: Option<PathBuf>,
    generated: String,
    runs: BTreeMap<String, RunReading>,
}

impl Build<'_> {
    /// The fits read while building, as the renderings ask for them.
    fn into_fits(self) -> Fits {
        let mut fits = Fits::default();
        for reading in self.runs.into_values() {
            if let (Some(path), Some(summary)) = (reading.files.summary_json, reading.summary) {
                fits.by_path.insert(path, summary);
            }
        }
        fits
    }

    /// One `AttemptSummary` per attempt, each timed by its own run.
    fn attempt_summaries(
        &mut self,
        records: &[super::state::AttemptRecord],
    ) -> Vec<AttemptSummary> {
        let mut out = Vec::with_capacity(records.len());
        for a in records {
            let timing = self.run(&a.model).timing.clone();
            out.push(AttemptSummary {
                model: a.model.clone(),
                outcome: a.outcome.clone(),
                timing,
            });
        }
        out
    }

    /// What `model_rel`'s run left behind, read at most once.
    fn run(&mut self, model_rel: &str) -> &RunReading {
        if !self.runs.contains_key(model_rel) {
            let reading = self.read_run(model_rel);
            self.runs.insert(model_rel.to_string(), reading);
        }
        &self.runs[model_rel]
    }

    fn read_run(&self, model_rel: &str) -> RunReading {
        let out_dir = self.out_dir;
        let settings = self.settings;
        let mut reading = RunReading::default();
        if model_rel.is_empty() {
            return reading;
        }
        let model_path = out_dir.join(model_rel);
        let Ok(layout) = ModelLayout::for_model_path(&model_path) else {
            return reading;
        };
        let Ok(run_dir) = run_dir_in(&model_path, settings, self.project_root.as_deref()) else {
            return reading;
        };
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

        reading.files.ext = ext_path_in(&model_path, &run_dir)
            .ok()
            .and_then(|p| rel(&p));
        reading.files.lst = rel(&layout.output_file(&run_dir, "lst"));
        reading.files.summary_json = rel(&run_dir.join(RUN_SUMMARY_FILENAME));
        if run_dir.join(RUN_END_FILENAME).exists() {
            reading.summary = run_summary(&run_dir, settings).ok();
        }

        // The markers, read leniently: a marker from another pharos version
        // that does not parse simply leaves the timing blank.
        let start = RunStartFile::load(run_dir.join(RUN_START_FILENAME)).ok();
        let end = RunEndFile::load(run_dir.join(RUN_END_FILENAME)).ok();
        reading.timing.started = start.map(|s| s.start);
        reading.timing.wall_seconds = end.as_ref().map(|e| e.runtime_ms as f64 / 1000.0);
        reading.timing.ended = end.map(|e| e.end);
        if reading.timing.wall_seconds.is_none() {
            reading.timing.wall_seconds = seconds_between(
                reading.timing.started.as_deref(),
                reading.timing.ended.as_deref(),
            );
        }
        reading.timing.estimation_seconds = reading
            .summary
            .as_ref()
            .map(|s| s.lst.run_details.estimation_time.iter().sum())
            .filter(|t: &f64| *t > 0.0);
        reading
    }
}

// ---------------------------------------------------------------------------
// The fits behind a summary
// ---------------------------------------------------------------------------

/// The fits an [`ScmSummary`] describes, read from the run directories it
/// names.
///
/// Every SCM run already writes its own `pharos_summary.json` — the record
/// `pharos nonmem summary --json` prints, parameters named per the project's
/// comment type — so the summary carries the path to it and a rendering asks
/// for the numbers here. That keeps one schema for how a fit went (pharos's
/// own [`Summary`]) instead of an SCM copy of it, and keeps `scm_summary.json`
/// to what the SCM process itself decided.
///
/// A run whose summary cannot be read simply has no estimates: every lookup
/// is an `Option`, and the renderings already spell a missing number `-`.
#[derive(Debug, Default)]
pub struct Fits {
    by_path: BTreeMap<String, Summary>,
}

impl Fits {
    /// Read every run the summary points at, once.
    pub fn read(summary: &ScmSummary) -> Self {
        let out_dir = summary.base_dir();
        let mut fits = Fits::default();
        for round in &summary.rounds {
            fits.load(out_dir, &round.reference_files);
            for cand in &round.candidates {
                fits.load(out_dir, &cand.files);
            }
        }
        fits
    }

    fn load(&mut self, out_dir: &Path, files: &RunFiles) {
        let Some(rel) = &files.summary_json else {
            return;
        };
        if self.by_path.contains_key(rel) {
            return;
        }
        match fs::read_to_string(out_dir.join(rel)).map_err(anyhow::Error::from) {
            Ok(content) => match serde_json::from_str::<Summary>(&content) {
                Ok(summary) => {
                    self.by_path.insert(rel.clone(), summary);
                }
                Err(e) => log::warn!("could not parse {rel}: {e}"),
            },
            Err(e) => log::warn!("could not read {rel}: {e:#}"),
        }
    }

    /// The fit a run left behind, when its summary could be read.
    pub fn get(&self, files: &RunFiles) -> Option<&Summary> {
        self.by_path.get(files.summary_json.as_ref()?)
    }

    /// The estimates of a run, when its summary could be read.
    pub fn parameters(&self, files: &RunFiles) -> Option<&TableParameters> {
        Some(&self.get(files)?.parameters)
    }
}

impl RoundSummary {
    /// The effect's own estimate, taken from the model where it is free: the
    /// candidate's own fit in forward selection, the round's reference fit in
    /// backward elimination (where the candidate model is the one *without*
    /// the effect).
    pub fn effect_of<'a>(
        &self,
        cand: &CandidateSummary,
        fits: &'a Fits,
    ) -> Option<&'a ThetaEstimate> {
        let free_in = match self.direction {
            Direction::Forward => &cand.files,
            Direction::Backward => &self.reference_files,
        };
        let name = format!("THETA{}", cand.thetas.first()?);
        fits.parameters(free_in)?
            .theta
            .iter()
            .find(|t| t.name == name)
    }
}

// ---------------------------------------------------------------------------
// Building the record
// ---------------------------------------------------------------------------

/// Read the SCM process in `out_dir` and build its summary. A planned but
/// unstarted process summarises to its plan and no rounds.
pub fn read_summary(out_dir: &Path) -> Result<ScmSummary> {
    let process = ScmProcess::read(out_dir)?;
    let settings = RunSettings::discover_from(out_dir)?;
    let (mut summary, _) = build_summary(&process.plan, &process.state, out_dir, &settings);
    if !process.started {
        summary.updated = None;
        summary.message = Some("plan written; the SCM process has not started".into());
    }
    summary.models_running = process.models_running;
    Ok(summary)
}

/// Build the summary of `state` against `plan`, reading fits under
/// `out_dir`, and hand back the fits it read on the way — a caller that
/// renders or writes files needs exactly those, and reading them twice is
/// the same JSON parsed twice. Never fails: a run whose output cannot be
/// read simply carries less.
pub fn build_summary(
    plan: &ScmPlan,
    state: &ScmState,
    out_dir: &Path,
    settings: &RunSettings,
) -> (ScmSummary, Fits) {
    let mut build = Build {
        plan,
        state,
        out_dir,
        settings,
        project_root: config::find_config_dir().ok().flatten(),
        generated: get_utc_now(),
        runs: BTreeMap::new(),
    };
    let generated = build.generated.clone();
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

    let summary = ScmSummary {
        schema_version: SUMMARY_SCHEMA_VERSION,
        generated,
        pharos_version: plan.pharos_version.clone(),
        plan_digest: state.plan_digest.clone(),
        initial_model: plan.model.clone(),
        out_dir: plan.out_dir.clone(),
        base: out_dir.to_path_buf(),
        options: plan.options.clone(),
        status: state.status.to_string(),
        message: state.message.clone(),
        phase: state.phase.map(|p| p.to_string()),
        updated: Some(state.updated.clone()),
        models_running: Vec::new(),
        candidates: plan.candidates.iter().map(|c| c.name.clone()).collect(),
        roster: state.roster.clone(),
        retained: state.retained.clone(),
        final_model: state.final_model.clone(),
        final_ofv: state
            .final_ofv
            .or_else(|| state.final_model.as_ref().and(state.reference_ofv)),
        pending_tie: state.pending_tie.clone(),
        totals,
        rounds,
    };
    let fits = build.into_fits();
    (summary, fits)
}

/// The comparator the driver ranks a round with: forward, smallest p then
/// largest drop; backward, largest p then smallest rise.
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
        build.run(&round.reference_model).files.clone()
    } else {
        RunFiles::default()
    };

    // Placings are the round's own, and it withholds them until every
    // candidate has concluded (see `RoundRecord::ranking`).
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
        generated: build.generated.clone(),
        plan_digest: state.plan_digest.clone(),
        initial_model: plan.model.clone(),
        out_dir: plan.out_dir.clone(),
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
    build: &mut Build<'_>,
    round: &RoundRecord,
    cand: &CandidateRecord,
    alpha: Option<f64>,
    rank: Option<usize>,
) -> CandidateSummary {
    let entry = build.state.roster_entry(&cand.candidate);
    let thetas: Vec<usize> = entry.map(|e| vec![e.candidate.theta]).unwrap_or_default();

    let reading = build.run(&cand.model);
    let (files, run_timing) = (reading.files.clone(), reading.timing.clone());

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
        thetas,
        initial: entry.map(|e| e.candidate.initial),
        fixed: entry.map(|e| e.candidate.fixed),
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

// ---------------------------------------------------------------------------
// Writing the record
// ---------------------------------------------------------------------------

/// Write the named round's summary (JSON + markdown) into its round
/// directory and refresh `scm_summary.{json,md}` in the out_dir.
/// Returns the round's (json path, md path).
pub fn write_round_summary(
    out_dir: &Path,
    summary: &ScmSummary,
    round_name: &str,
    fits: &Fits,
) -> Result<(PathBuf, PathBuf)> {
    let round = summary
        .rounds
        .iter()
        .find(|r| r.round == round_name)
        .with_context(|| format!("no round named {round_name} in the state"))?;
    let dir_name = round
        .dir_name()
        .with_context(|| format!("round {round_name} has no candidates to name its directory"))?;
    let dir = out_dir.join(dir_name);
    fs::create_dir_all(&dir)?;

    let json_path = dir.join(ROUND_SUMMARY_JSON);
    utils::write_json_to_file(round, &json_path)
        .with_context(|| format!("failed to write {}", json_path.display()))?;
    let md_path = dir.join(ROUND_SUMMARY_MD);
    fs::write(&md_path, round_summary_md(round, fits))?;

    let process_path = out_dir.join(SCM_SUMMARY_FILENAME);
    utils::write_json_to_file(summary, &process_path)
        .with_context(|| format!("failed to write {}", process_path.display()))?;
    fs::write(out_dir.join(SCM_SUMMARY_MD), summary.markdown(fits))?;
    Ok((json_path, md_path))
}

// ---------------------------------------------------------------------------
// Rendering options
// ---------------------------------------------------------------------------

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
    /// Only this candidate: the rounds it was tested in, and its row alone.
    pub candidate: Option<String>,
    /// The header and one line per round, no candidate rows — what `scm
    /// status` prints. Not a `scm summary` flag.
    pub brief: bool,
    /// `--long`: absolute OFV, the effect's estimate with RSE and CI, df,
    /// attempts, condition number and heuristics on every candidate line,
    /// plus everything the default hides — the reference fit's line, every
    /// attempt with its model path.
    pub long: bool,
    /// `--timing`: start, end and wall time per fit and per round,
    /// estimation time and function evaluations, and totals.
    pub timing: bool,
    /// `--files`: run directory, .lst, .ext and summary JSON per candidate.
    pub files: bool,
}

impl SummaryOptions {
    /// Whether `cand` is one the options show.
    fn shows(&self, cand: &CandidateSummary) -> bool {
        self.candidate
            .as_deref()
            .is_none_or(|n| cand.candidate.eq_ignore_ascii_case(n))
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
    /// Where the fits this summary names are read from: the directory it was
    /// read from, falling back to the recorded `out_dir` for a summary built
    /// by hand.
    pub fn base_dir(&self) -> &Path {
        if self.base.as_os_str().is_empty() {
            Path::new(&self.out_dir)
        } else {
            &self.base
        }
    }

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

/// `0.412 (14.2%)`, or `0.412 (N/A)` when the fit carries no standard error
/// to make an RSE from — the same `N/A` `pharos nonmem summary` prints for a
/// run with the covariance step off. `-` when there is no estimate at all.
fn fmt_estimate(e: Option<&ThetaEstimate>, digits: usize) -> String {
    match e {
        Some(e) => match e.rse {
            Some(rse) => format!("{:.digits$} ({rse:.1}%)", e.estimate),
            None => format!("{:.digits$} (N/A)", e.estimate),
        },
        None => "-".to_string(),
    }
}

/// The 95% CI, `N/A` when the fit has no standard errors to build one from
/// (the cov step is off), `-` when there is no estimate at all.
fn fmt_ci(e: Option<&ThetaEstimate>, digits: usize) -> String {
    let Some(e) = e else {
        return "-".to_string();
    };
    let ci = e
        .stderr
        .and_then(|se| Transform::Identity.compute_ci(e.estimate, se, 0.95).ok());
    match ci {
        Some((lo, hi)) => format!("{lo:.digits$}, {hi:.digits$}"),
        None => "N/A".to_string(),
    }
}

/// `1 fit` / `3 fits`.
fn plural(n: usize, noun: &str) -> String {
    match n {
        1 => format!("1 {noun}"),
        n => format!("{n} {noun}s"),
    }
}

/// A round's candidates winner-first: ranked ones in rank order, then the
/// scored-but-unranked by p (mid-round nothing is ranked yet, since the
/// round can still overturn a placing), then the rest in plan order.
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

// ---------------------------------------------------------------------------
// The candidate columns
// ---------------------------------------------------------------------------

/// One cell of a candidate table, as a value rather than as text.
///
/// Every rendering carries the same candidate columns — [`TEXT_COLUMNS`],
/// [`TEXT_COLUMNS_LONG`] and [`MARKDOWN_COLUMNS`] — and differ only in how
/// each spells a number, a p-value, a flag and a missing value.
/// Keeping the value typed until the last moment is what lets a column be
/// defined once and still read the way each rendering's own readers expect.
#[derive(Debug, Clone)]
pub(crate) enum Cell<'a> {
    Text(String),
    /// A path or model name; markdown sets it in backticks.
    Code(String),
    /// A number, to the rendering's own precision (see [`Column::decimals`]).
    Num(Option<f64>),
    /// A number that always carries its sign — every ΔOFV.
    Signed(Option<f64>),
    P(Option<f64>),
    Int(Option<usize>),
    /// Significance: the `*` the text table marks a row with.
    Sig(Option<bool>),
    /// The round's own pick.
    Selected(bool),
    /// An effect's estimate with its RSE, and the interval around it.
    Est(Option<&'a ThetaEstimate>),
    Ci(Option<&'a ThetaEstimate>),
}

impl Cell<'_> {
    /// The padded text table's spelling: a missing value is `-`, as
    /// everywhere else on screen.
    fn text(&self, digits: usize) -> String {
        match self {
            Cell::Text(s) | Cell::Code(s) => s.clone(),
            Cell::Num(v) => fmt_num(*v, digits),
            Cell::Signed(v) => fmt_signed(*v, digits),
            Cell::P(p) => fmt_p(*p),
            Cell::Int(n) => n.map(|n| n.to_string()).unwrap_or_else(|| "-".to_string()),
            Cell::Sig(s) => if *s == Some(true) { "*" } else { " " }.to_string(),
            Cell::Selected(s) => if *s { "yes" } else { "" }.to_string(),
            Cell::Est(e) => fmt_estimate(*e, digits),
            Cell::Ci(e) => fmt_ci(*e, digits),
        }
    }

    /// A markdown cell: a missing value is blank, which reads better down a
    /// table column than a dash.
    fn markdown(&self, digits: usize) -> String {
        match self {
            Cell::Code(s) if s.is_empty() => String::new(),
            Cell::Code(s) => format!("`{s}`"),
            Cell::Num(None) | Cell::Signed(None) | Cell::P(None) => String::new(),
            Cell::P(Some(p)) => format!("{p:.4e}"),
            Cell::Int(None) | Cell::Sig(None) => String::new(),
            Cell::Sig(Some(s)) => yes_no(*s).to_string(),
            Cell::Selected(s) => if *s { "**yes**" } else { "" }.to_string(),
            Cell::Est(None) | Cell::Ci(None) => String::new(),
            other => other.text(digits),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Align {
    Left,
    Right,
}
use Align::{Left, Right};

/// One candidate column, defined once for every rendering that carries it.
pub(crate) struct Column {
    /// Heading, width and alignment in the padded text table; `None` for a
    /// column no text rendering lays out.
    text: Option<(&'static str, usize, Align)>,
    /// Heading in a markdown table; `None` when markdown does not carry it.
    md: Option<&'static str>,
    /// Decimals this column wants, when it wants something other than the
    /// renderings' default of 3.
    decimals: Option<usize>,
    value: for<'a> fn(&Row<'a>) -> Cell<'a>,
}

/// A column, defined once for every rendering that carries it: the text
/// table and markdown opt in with [`Column::text`] and [`Column::md`].
const fn col(value: for<'a> fn(&Row<'a>) -> Cell<'a>) -> Column {
    Column {
        text: None,
        md: None,
        decimals: None,
        value,
    }
}

impl Column {
    const fn text(mut self, head: &'static str, width: usize, align: Align) -> Self {
        self.text = Some((head, width, align));
        self
    }

    const fn md(mut self, head: &'static str) -> Self {
        self.md = Some(head);
        self
    }

    const fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = Some(decimals);
        self
    }
}

/// What a column reads from: one candidate, in its round, with the fits the
/// summary points at.
#[derive(Clone, Copy)]
pub(crate) struct Row<'a> {
    pub round: &'a RoundSummary,
    pub cand: &'a CandidateSummary,
    pub fits: &'a Fits,
}

impl<'a> Row<'a> {
    /// The effect's own estimate, from whichever model leaves it free.
    fn effect(&self) -> Option<&'a ThetaEstimate> {
        self.round.effect_of(self.cand, self.fits)
    }

    /// The condition number of the fit's last `$EST`.
    fn condition_number(&self) -> Option<f64> {
        self.fits
            .get(&self.cand.files)?
            .minimization_results
            .last()?
            .condition_number
            .filter(|v| v.is_finite())
    }
}

const C_CANDIDATE: Column = col(|r| Cell::Text(r.cand.candidate.clone()))
    .text("candidate", 12, Left)
    .md("candidate");
const C_MODEL: Column = col(|r| Cell::Code(r.cand.model.clone())).md("model");
const C_ATTEMPTS: Column = col(|r| Cell::Int(Some(r.cand.attempts.len())))
    .text("tries", 5, Right)
    .md("attempts");
const C_STATUS: Column = col(|r| Cell::Text(r.cand.status.clone())).md("status");
const C_OFV: Column = col(|r| Cell::Num(r.cand.ofv))
    .text("OFV", 12, Right)
    .md("OFV");
const C_DELTA_OFV: Column = col(|r| Cell::Signed(r.cand.delta_ofv))
    .text("dOFV", 10, Right)
    .md("\u{394}OFV");
const C_DF: Column = col(|r| Cell::Int(Some(r.cand.df)))
    .text("df", 2, Right)
    .md("df");
const C_P_VALUE: Column = col(|r| Cell::P(r.cand.p_value)).text("p", 9, Right).md("p");
const C_CRITICAL_DELTA_OFV: Column =
    col(|r| Cell::Num(r.cand.critical_delta_ofv)).md("crit \u{394}OFV");
/// In the text table this is the unheaded `*` beside the p-value, two
/// columns wide so the mark sits clear of the number.
const C_SIGNIFICANT: Column = col(|r| Cell::Sig(r.cand.significant))
    .text("", 2, Left)
    .md("significant");
const C_SELECTED: Column = col(|r| Cell::Selected(r.cand.selected)).md("selected");
const C_EST_RSE: Column = col(|r| Cell::Est(r.effect()))
    .text("est (RSE%)", 22, Left)
    .md("estimate (RSE%)");
const C_CI95: Column = col(|r| Cell::Ci(r.effect())).text("CI95", 22, Left);
const C_CONDITION_NUMBER: Column = col(|r| Cell::Num(r.condition_number()))
    .text("cond#", 7, Right)
    .md("cond#")
    .decimals(0);
const C_HEURISTICS: Column =
    col(|r| Cell::Text(r.cand.heuristics.join("; "))).md("heuristic checks");

/// The default text table. The status, the selection arrow, the heuristics
/// and the timing are appended after each row rather than laid out, since
/// they are annotations on a row and not columns of their own.
const TEXT_COLUMNS: &[&Column] = &[
    &C_CANDIDATE,
    &C_OFV,
    &C_DELTA_OFV,
    &C_P_VALUE,
    &C_SIGNIFICANT,
];

/// The `--long` text table: the default columns and the fit's own numbers.
const TEXT_COLUMNS_LONG: &[&Column] = &[
    &C_CANDIDATE,
    &C_OFV,
    &C_DELTA_OFV,
    &C_P_VALUE,
    &C_SIGNIFICANT,
    &C_EST_RSE,
    &C_CI95,
    &C_DF,
    &C_ATTEMPTS,
    &C_CONDITION_NUMBER,
];

/// The markdown candidate table, shared by the round summary and
/// `scm_summary.md`.
const MARKDOWN_COLUMNS: &[&Column] = &[
    &C_CANDIDATE,
    &C_MODEL,
    &C_ATTEMPTS,
    &C_STATUS,
    &C_OFV,
    &C_DELTA_OFV,
    &C_CRITICAL_DELTA_OFV,
    &C_DF,
    &C_P_VALUE,
    &C_SIGNIFICANT,
    &C_SELECTED,
    &C_EST_RSE,
    &C_CONDITION_NUMBER,
    &C_HEURISTICS,
];

/// Every rendering gives its numbers three decimals, bar the columns that
/// ask for their own.
const DIGITS: usize = 3;

fn pad(out: &mut String, cell: &str, width: usize, align: Align) {
    match align {
        Left => write!(out, "{cell:<width$}").unwrap(),
        Right => write!(out, "{cell:>width$}").unwrap(),
    }
}

/// The heading row of the padded text table, with `trailing` appended for
/// the annotations that follow each row.
fn text_header(cols: &[&Column], trailing: &str) -> String {
    let mut line = String::from("  ");
    for (i, col) in cols.iter().enumerate() {
        if i > 0 {
            line.push(' ');
        }
        let (head, width, align) = col.text.expect("a text column has a text spec");
        pad(&mut line, head, width, align);
    }
    line.push_str(trailing);
    line
}

fn text_row(cols: &[&Column], row: &Row<'_>, digits: usize) -> String {
    let mut line = String::from("  ");
    for (i, col) in cols.iter().enumerate() {
        if i > 0 {
            line.push(' ');
        }
        let (_, width, align) = col.text.expect("a text column has a text spec");
        let cell = (col.value)(row).text(col.decimals.unwrap_or(digits));
        pad(&mut line, &cell, width, align);
    }
    line
}

/// A markdown table of one round's candidates.
fn add_candidate_table(out: &mut Lines, round: &RoundSummary, fits: &Fits) {
    let cols = MARKDOWN_COLUMNS;
    let head: Vec<&str> = cols
        .iter()
        .map(|c| c.md.expect("a markdown column has a heading"))
        .collect();
    out.add(format!("| {} |", head.join(" | ")));
    out.add(format!("|{}", "---|".repeat(cols.len())));
    for cand in &round.candidates {
        let row = Row { round, cand, fits };
        let cells: Vec<String> = cols
            .iter()
            .map(|c| (c.value)(&row).markdown(c.decimals.unwrap_or(DIGITS)))
            .collect();
        out.add(format!("| {} |", cells.join(" | ")));
    }
}

// ---------------------------------------------------------------------------
// Text rendering
// ---------------------------------------------------------------------------

impl ScmSummary {
    /// Whether the SCM process has started at all; an unstarted one is the
    /// plan and nothing more.
    pub fn started(&self) -> bool {
        self.updated.is_some()
    }

    /// One label per candidate whose initial estimate or bounds moved while
    /// the SCM process was under way, most recent change first shown.
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

    fn header_lines(&self, out: &mut Lines, opts: &SummaryOptions) {
        let rounds = match self.totals.rounds_complete {
            0 => "no rounds complete".to_string(),
            n => format!("{} complete", plural(n, "round")),
        };
        let updated = match &self.updated {
            Some(u) => format!(" (updated {u})"),
            None => String::new(),
        };
        out.add(format!(
            "<scm summary> {}   {}{updated} · {} · {rounds}",
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
            self.initial_model,
            alphas.join(", "),
            on_off(o.cov_step)
        ));
        out.add(format!("candidates : {}", self.candidates.join(", ")));
        if let Some(p) = &self.phase {
            out.add(format!("phase      : {p}"));
        }
        if let Some(m) = &self.message {
            out.add(format!("note       : {m}"));
        }
        // The one state that needs the user to do something, so it gets its
        // own line rather than hiding in the note.
        if let Some(tie) = &self.pending_tie {
            out.add(format!(
                "awaiting   : your decision on {} in {} (p = {:.3e}, dOFV = {:+.3}) — re-run with --choose <candidate>",
                tie.candidates.join(" / "),
                tie.round,
                tie.p_value,
                tie.delta_ofv
            ));
        }
        if !self.models_running.is_empty() {
            out.add(format!("running    : {}", self.models_running.join(", ")));
        }
        out.add(format!("retained   : {}", none_or_list(&self.retained)));
        let removed = self.removal_labels();
        if !removed.is_empty() {
            out.add(format!("removed    : {}", removed.join(", ")));
        }
        let retuned = self.retuned_labels();
        if !retuned.is_empty() {
            out.add(format!("retuned    : {}", retuned.join(", ")));
        }
        if let Some(f) = &self.final_model {
            out.add(format!("final model: {f}{}", ofv_suffix(self.final_ofv)));
        }
        if opts.timing {
            out.add(format!(
                "time       : {} ({} retr{}) · {}",
                plural(self.totals.models_fitted, "fit"),
                self.totals.retries,
                if self.totals.retries == 1 { "y" } else { "ies" },
                timing_span(&self.totals.timing)
            ));
        }
    }

    /// The text rendering, reading the fits it quotes for itself.
    pub fn render_text(&self, opts: &SummaryOptions) -> Result<String> {
        self.render_text_with(opts, &Fits::read(self))
    }

    fn render_text_with(&self, opts: &SummaryOptions, fits: &Fits) -> Result<String> {
        let mut out = Lines::new();
        let rounds = self.select_rounds(opts)?;
        self.header_lines(&mut out, opts);

        if self.rounds.is_empty() {
            out.blank();
            out.add("no rounds yet: the plan is written and the SCM process has not started");
            return Ok(out.finish());
        }
        if opts.brief {
            out.add("rounds     :");
            for round in &rounds {
                out.add(format!("  {:<18} {}", round.round, round.progress_label()));
            }
        } else {
            for round in &rounds {
                out.blank();
                self.render_round(&mut out, round, opts, fits);
            }
        }
        out.blank();
        out.add("records    : scm_summary.{json,md} · round_summary.{json,md} in each round dir");
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
        // A single round always lists its attempts.
        let single = opts.round.is_some();
        let detail = opts.long || single;

        // Headline: round, reference, alpha, critical value, then where the
        // round got to. The reference fit has no scoring to head it.
        let mut head = format!("{:<16}", round.round);
        if round.has_reference() {
            let crit = round
                .candidates
                .iter()
                .find(|c| c.df == 1)
                .and_then(|c| c.critical_delta_ofv)
                .or_else(|| round.candidates.iter().find_map(|c| c.critical_delta_ofv));
            write!(
                head,
                " ref OFV {} · alpha {} · crit dOFV {} ·",
                fmt_num(round.reference_ofv, d),
                round.alpha.map(|a| a.to_string()).unwrap_or_default(),
                fmt_num(crit, d)
            )
            .unwrap();
        }
        out.add(format!("{head} {}", round.progress_label()));
        if detail && round.has_reference() {
            out.add(format!(
                "                 reference {} · retained before this round: {}",
                round.reference_model,
                none_or_list(&round.retained_before)
            ));
            out.add(format!("                 {}", round.change_label()));
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

        let cols = if opts.long {
            TEXT_COLUMNS_LONG
        } else {
            TEXT_COLUMNS
        };
        out.add(text_header(cols, "  flags"));
        for c in winner_first(round).into_iter().filter(|c| opts.shows(c)) {
            let mut line = text_row(
                cols,
                &Row {
                    round,
                    cand: c,
                    fits,
                },
                d,
            );
            // status, when it is not the plain success the numbers imply
            if c.status != "succeeded" {
                write!(line, "  {}", c.status).unwrap();
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
            if opts.timing {
                write!(line, "   {}", timing_suffix(&c.timing)).unwrap();
            }
            out.add(line);

            if detail || (c.status != "succeeded" && c.status != "pending") {
                self.render_attempts(out, c, opts);
            }
            if !opts.long && single && !c.heuristics.is_empty() {
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
    }

    fn render_attempts(&self, out: &mut Lines, c: &CandidateSummary, opts: &SummaryOptions) {
        for a in &c.superseded {
            let mut line = format!("      {:<44} {} (superseded)", a.model, a.outcome);
            if opts.timing {
                write!(line, "   {}", timing_suffix(&a.timing)).unwrap();
            }
            out.add(line);
        }
        for a in &c.attempts {
            let mut line = format!("      {:<44} {}", a.model, a.outcome);
            if opts.timing {
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
}

/// `wall 1m 20s (09:01 → 09:02) · est time 1m 12s` — how every rendering
/// that totals a span spells it.
fn timing_span(t: &Timing) -> String {
    format!(
        "wall {} ({} → {}) · est time {}",
        fmt_duration(t.wall_seconds),
        clock(t.started.as_deref()),
        clock(t.ended.as_deref()),
        fmt_duration(t.estimation_seconds)
    )
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
pub fn round_summary_md(round: &RoundSummary, fits: &Fits) -> String {
    let mut out = Lines::new();
    out.add(format!("# {}", round.round));
    out.blank();
    out.add(format!("- initial model: `{}`", round.initial_model));
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
        "- all models minimized: {}",
        yes_no(round.all_succeeded)
    ));
    out.add(format!(
        "- heuristic checks fired: {}",
        yes_no(round.any_heuristics)
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
    out.add(format!("- {}", round.change_label()));
    if let Some(w) = round.timing.wall_seconds {
        out.add(format!("- wall time: {}", fmt_duration(Some(w))));
    }
    out.add(format!("- next: {}", round.next));
    out.blank();
    add_candidate_table(&mut out, round, fits);
    out.finish()
}

impl ScmSummary {
    /// The facts the markdown rendering of an SCM process opens with: what
    /// it was run on, how it was configured, and where it got to.
    fn markdown_facts(&self, out: &mut Lines) {
        out.add(format!("- model: `{}`", self.initial_model));
        out.add(format!("- out dir: `{}`", self.out_dir));
        out.add(format!("- status: {}", self.status));
        if let Some(m) = &self.message {
            out.add(format!("- note: {m}"));
        }
        out.add(format!("- direction: {}", self.options.direction_label()));
        out.add(format!(
            "- alphas: forward {}, backward {}",
            self.options.forward_alpha, self.options.backward_alpha
        ));
        out.add(format!(
            "- retries: up to {} per fit, starting from the previous attempt's estimates",
            self.options.max_retries
        ));
        out.add(format!(
            "- covariance step: {}",
            on_off(self.options.cov_step)
        ));
        out.add(format!("- retained: {}", none_or_list(&self.retained)));
        let removed = self.removal_labels();
        if !removed.is_empty() {
            out.add(format!("- removed: {}", removed.join(", ")));
        }
        let retuned = self.retuned_labels();
        if !retuned.is_empty() {
            out.add(format!("- retuned: {}", retuned.join(", ")));
        }
    }

    /// `scm_summary.md`: the facts, then every round's table and decision.
    fn markdown(&self, fits: &Fits) -> String {
        let mut out = Lines::new();
        out.add("# SCM summary");
        out.blank();
        self.markdown_facts(&mut out);
        if let Some(f) = &self.final_model {
            out.add(format!(
                "- final model: `{f}`{}",
                ofv_suffix(self.final_ofv)
            ));
        }
        out.blank();
        for round in &self.rounds {
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
            add_candidate_table(&mut out, round, fits);
            out.blank();
            if !round.decision.is_empty() {
                out.add(format!("**Decision:** {}", round.decision));
                out.blank();
            }
            if round.counts.unusable > 0 {
                out.add(
                    "_Unusable candidates are reported above; they are never scored as insignificant._",
                );
                out.blank();
            }
        }
        out.finish()
    }
}

impl RoundSummary {
    /// The directory this round's models and records live in: the round
    /// name, except the reference round, whose single "candidate"
    /// (base/full) names its directory.
    pub fn dir_name(&self) -> Option<String> {
        if self.round == REFERENCE_ROUND {
            self.candidates.first().map(|c| c.candidate.clone())
        } else {
            Some(self.round.clone())
        }
    }

    pub fn has_reference(&self) -> bool {
        self.reference_model != NO_REFERENCE
    }

    /// Where the round got to, in one phrase: its decision once complete,
    /// otherwise how many candidates have concluded — plus the retries and
    /// withdrawals behind it, when there were any.
    pub fn progress_label(&self) -> String {
        let c = &self.counts;
        let mut label = if self.complete {
            self.decision.clone()
        } else {
            let concluded = c.succeeded + c.unusable + c.withdrawn;
            let mut l = format!("in progress — {concluded}/{} concluded", c.candidates);
            if !self.decision.is_empty() {
                write!(l, " ({})", self.decision).unwrap();
            }
            l
        };
        let mut extra = Vec::new();
        if c.retries > 0 {
            extra.push(format!(
                "{} retr{}",
                c.retries,
                if c.retries == 1 { "y" } else { "ies" }
            ));
        }
        if c.withdrawn > 0 {
            extra.push(format!("{} withdrawn", c.withdrawn));
        }
        if !extra.is_empty() {
            write!(label, " [{}]", extra.join(", ")).unwrap();
        }
        label
    }

    /// What this round changed, in its phase's own terms: the covariate
    /// forward selection added, or the one backward elimination dropped.
    /// "none" when the round moved nothing — a round that ended without a
    /// winner, or one still under way.
    pub fn change_label(&self) -> String {
        let (verb, changed) = match self.direction {
            Direction::Forward => (
                "added",
                difference(&self.retained_after, &self.retained_before),
            ),
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
    use crate::scm::snapshot_tests::fabricate_running_scm;
    use crate::scm::test_support::{Fit, TEMPLATE, full_scm_executor, make_plan, write_fit_output};
    use crate::scm::{ScmOptions, run_scm};

    /// The driver only writes a wave's outcomes back to the state once the
    /// whole batch returns, so the reader has to see finished runs itself.
    #[test]
    fn an_open_round_counts_runs_that_finished_since_the_state_was_written() {
        let dir = tempfile::tempdir().unwrap();
        let out_dir = fabricate_running_scm(dir.path());
        let brief = SummaryOptions {
            brief: true,
            ..Default::default()
        };

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
        let row = summary
            .rounds
            .iter()
            .flat_map(|r| &r.candidates)
            .find(|c| c.candidate == "CRCL_CL")
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
        // the final model's own fit, not the last reference fit's 980
        assert_eq!(summary.final_ofv, Some(979.5));
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
        assert_eq!((wt_cl.initial, wt_cl.fixed), (Some(0.1), Some(0.0)));
        assert_eq!(wt_cl.statistic, Some(20.0));
        assert!((wt_cl.critical_delta_ofv.unwrap() - 3.841).abs() < 1e-3);
        assert!(
            wt_cl
                .files
                .ext
                .as_deref()
                .unwrap()
                .ends_with("1001_wt_cl.ext")
        );

        // The estimates are not in the record: they are read back from the
        // `pharos_summary.json` each run wrote, which is the whole point of
        // carrying the path rather than a copy.
        let fits = Fits::read(&summary);
        let effect = r1
            .effect_of(wt_cl, &fits)
            .expect("THETA4 in the winner's fit");
        // the mocked .ext reports THETA4 = 0.25 in the final row
        assert_eq!(effect.name, "THETA4");
        assert!((effect.estimate - 0.25).abs() < 1e-9);
        assert!(fits.get(&wt_cl.files).is_some());

        // backward: the dropped candidate's estimate comes from the reference
        let b1 = &summary.rounds[4];
        assert_eq!(b1.direction, Direction::Backward);
        let crcl = b1
            .candidates
            .iter()
            .find(|c| c.candidate == "CRCL_CL")
            .unwrap();
        assert!(crcl.selected);
        assert_eq!(
            b1.effect_of(crcl, &fits)
                .expect("THETA5 in the reference fit")
                .name,
            "THETA5"
        );
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
    fn selection_follows_the_options() {
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

    #[test]
    fn a_planned_process_summarises_to_its_plan() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        plan.save().unwrap();
        let summary = read_summary(&plan.out_dir_path()).unwrap();
        assert_eq!(summary.status, "planned");
        assert!(summary.rounds.is_empty());
        assert!(!summary.started());
        let text = summary.render_text(&SummaryOptions::default()).unwrap();
        assert!(text.contains("has not started"), "{text}");
        assert!(text.contains("candidates : WT_CL, CRCL_CL, WT_V"), "{text}");
        let err = summary
            .select_rounds(&SummaryOptions {
                round: Some("1".into()),
                ..Default::default()
            })
            .unwrap_err();
        assert!(err.to_string().contains("not started"), "{err}");
    }
}
