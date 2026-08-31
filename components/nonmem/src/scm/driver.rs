use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use config::NonmemConfig;
use fs_err as fs;

use super::round::{
    RoundEntry, backward_entries, ext_path_for, file_stem_of, forward_entries, read_fit_outcome,
    scm_model_name, write_retry_model, write_run_summary, write_scm_model,
};
use super::score::lrt;
use super::state::{
    AttemptRecord, CandidateRecord, CandidateStatus, RoundRecord, ScmRunStatus, ScmState,
};
use super::{Direction, ScmPlan};
use crate::run::RunOptions;
use crate::runner::run_models;

/// Fits a batch of models to completion (blocking). Implementations decide
/// where the fits run; outcomes are read from the filesystem afterwards.
pub trait FitExecutor {
    fn fit(&self, models: &[PathBuf]) -> Result<()>;
    fn describe(&self) -> String;
}

/// Runs fits in-process via the standard pharos runner.
pub struct LocalExecutor {
    pub nonmem_config: NonmemConfig,
    pub config_dir: PathBuf,
    pub num_parallel: Option<usize>,
}

impl LocalExecutor {
    fn run_options(&self) -> RunOptions {
        RunOptions {
            overwrite: true,
            num_parallel: self.num_parallel,
            ..Default::default()
        }
    }
}

impl FitExecutor for LocalExecutor {
    fn fit(&self, models: &[PathBuf]) -> Result<()> {
        if models.is_empty() {
            return Ok(());
        }
        // Individual fit failures surface through the outcome files; only a
        // pharos-level error is fatal here.
        let _ = run_models(
            &self.nonmem_config,
            models,
            &self.run_options(),
            &self.config_dir,
        )?;
        Ok(())
    }

    fn describe(&self) -> String {
        "local".to_string()
    }
}

/// Outcome of a completed (or paused/failed) `run_scm` invocation.
#[derive(Debug, Clone)]
pub struct ScmOutcome {
    pub state: ScmState,
}

/// Metadata files require a pharos project root that contains the output
/// directory; outside one (e.g. tests), models are written without metadata.
fn metadata_enabled(out_dir: &Path) -> bool {
    let Ok(Some(root)) = config::find_config_dir() else {
        return false;
    };
    let Ok(root) = fs::canonicalize(root) else {
        return false;
    };
    fs::canonicalize(out_dir)
        .map(|d| d.starts_with(&root))
        .unwrap_or(false)
}

/// Path relative to out_dir for state records; falls back to the full path.
fn rel_to(path: &Path, out_dir: &Path) -> String {
    path.strip_prefix(out_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

const SCM_ROUND_DIR_PREFIXES: &[&str] = &["forward_round", "backward_round"];
const SCM_FIXED_DIRS: &[&str] = &["base", "full", "final"];

/// Remove previous SCM output (round dirs and the state file) from out_dir.
/// Only known SCM subdirectories are touched.
fn clear_previous_output(out_dir: &Path) -> Result<()> {
    for entry in fs::read_dir(out_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let is_scm_dir = SCM_FIXED_DIRS.contains(&name.as_str())
            || SCM_ROUND_DIR_PREFIXES.iter().any(|p| {
                name.starts_with(p) && name[p.len()..].chars().all(|c| c.is_ascii_digit())
            });
        if is_scm_dir {
            fs::remove_dir_all(&path)?;
        }
    }
    let state_path = ScmState::state_path(out_dir);
    if state_path.exists() {
        fs::remove_file(state_path)?;
    }
    Ok(())
}

/// Run (or resume) the SCM search described by `plan`.
pub fn run_scm(plan: &ScmPlan, executor: &dyn FitExecutor) -> Result<ScmOutcome> {
    if plan.candidates.is_empty() {
        bail!("plan has no candidates");
    }
    let out_dir = plan.out_dir_path();
    fs::create_dir_all(&out_dir)?;
    let digest = plan.digest();

    let mut state = match ScmState::load(&out_dir)? {
        Some(s) if s.plan_digest == digest => {
            log::info!("resuming SCM search in {}", out_dir.display());
            s
        }
        Some(_) => {
            if !plan.options.overwrite {
                bail!(
                    "{} contains SCM state from a different plan; set overwrite to replace it or use a fresh out_dir",
                    out_dir.display()
                );
            }
            clear_previous_output(&out_dir)?;
            ScmState::new(digest)
        }
        None => ScmState::new(digest),
    };

    // Keep the plan on disk next to the state for the record.
    plan.save()?;

    state.status = ScmRunStatus::Running;
    state.message = None;
    state.save(&out_dir)?;

    match drive(plan, executor, &mut state, &out_dir) {
        Ok(status) => {
            state.status = status;
            state.save(&out_dir)?;
            // Completed searches get a final refresh of the decision log and
            // the last round's summary, so both carry the terminal status
            // and the final model.
            if status == ScmRunStatus::Completed {
                super::log::write_decision_log(&out_dir, plan, &state)?;
                if let Some(round) = state.rounds.last() {
                    super::log::write_round_summary(&out_dir, plan, &state, &round.name)?;
                }
            }
            Ok(ScmOutcome {
                state: state.clone(),
            })
        }
        Err(e) => {
            state.status = ScmRunStatus::Failed;
            state.message = Some(format!("{e:#}"));
            state.save(&out_dir)?;
            // Best-effort record of the failing round in its own directory.
            if let Some(round) = state.rounds.last() {
                let _ = super::log::write_round_summary(&out_dir, plan, &state, &round.name);
                let _ = super::log::write_decision_log(&out_dir, plan, &state);
            }
            Err(e)
        }
    }
}

fn drive(
    plan: &ScmPlan,
    executor: &dyn FitExecutor,
    state: &mut ScmState,
    out_dir: &Path,
) -> Result<ScmRunStatus> {
    let template = plan.model_path();
    if !template.exists() {
        bail!(
            "template model {} does not exist (scm commands run from the pharos project root)",
            template.display()
        );
    }
    let stem = file_stem_of(&template).context("template model has no file stem")?;
    let with_metadata = metadata_enabled(out_dir);
    log::info!(
        "SCM search on {} via {} executor (metadata: {})",
        template.display(),
        executor.describe(),
        with_metadata
    );

    let mut phases: Vec<Direction> = Vec::new();
    if plan.options.runs_forward() {
        phases.push(Direction::Forward);
    }
    if plan.options.runs_backward() {
        phases.push(Direction::Backward);
    }

    let ctx = DriveContext {
        plan,
        out_dir,
        template: &template,
        stem: &stem,
        with_metadata,
    };

    // ---- Reference fit (not a search round) ----
    if state.reference_model.is_none() {
        let first = phases[0];
        let (ref_name, action, released_names): (&str, String, Vec<String>) = match first {
            Direction::Forward => ("base", "fit base model".into(), vec![]),
            Direction::Backward => (
                "full",
                "fit full model".into(),
                plan.candidates.iter().map(|c| c.name.clone()).collect(),
            ),
        };

        let entries = vec![RoundEntry {
            candidate: ref_name.to_string(),
            action,
            released: plan.thetas_for(&released_names),
        }];

        let record = run_round_fits(
            &ctx,
            executor,
            state,
            "reference",
            ref_name,
            first,
            "-",
            entries,
        )?;
        let cand = record.candidates[0].clone();
        if cand.status != CandidateStatus::Succeeded {
            bail!(
                "reference model ({ref_name}) failed after {} attempt(s); the search cannot start",
                cand.n_attempts()
            );
        }
        if let Some(round) = state.find_round_mut("reference") {
            round.complete = true;
            round.decision = format!(
                "{ref_name} model fitted (OFV {})",
                cand.ofv.map(|o| format!("{o:.3}")).unwrap_or_default()
            );
        }
        state.reference_model = Some(cand.model.clone());
        state.reference_ofv = cand.ofv;
        state.retained = released_names;
        state.phase = Some(first);
        state.save(out_dir)?;
        super::log::write_round_summary(out_dir, plan, state, "reference")?;
        super::log::write_decision_log(out_dir, plan, state)?;
    }

    let mut rounds_this_invocation = 0usize;

    // ---- Search rounds ----
    while let Some(phase) = state.phase {
        if !phases.contains(&phase) {
            bail!("state phase {phase} is not part of this plan's direction");
        }

        let entries = match phase {
            Direction::Forward => forward_entries(plan, &state.retained),
            Direction::Backward => backward_entries(plan, &state.retained),
        };

        if entries.is_empty() {
            advance_phase(state, &phases);
            state.save(out_dir)?;
            if state.phase.is_none() {
                break;
            }
            continue;
        }

        if let Some(cap) = plan.options.num_rounds
            && rounds_this_invocation >= cap
        {
            state.message = Some(format!(
                "paused after {rounds_this_invocation} round(s) (num_rounds = {cap}); run `scm run` again to continue"
            ));
            return Ok(ScmRunStatus::Paused);
        }

        let round_number = state
            .rounds
            .iter()
            .filter(|r| r.direction == phase && r.name != "reference" && r.complete)
            .count()
            + 1;
        let round_name = format!("{phase}_round{round_number}");

        let reference_model = state
            .reference_model
            .clone()
            .context("internal error: no reference model")?;
        let reference_ofv = state
            .reference_ofv
            .context("internal error: no reference OFV")?;

        let record = run_round_fits(
            &ctx,
            executor,
            state,
            &round_name,
            &round_name,
            phase,
            &reference_model,
            entries,
        )?;

        // ---- Score the round ----
        let mut any_unusable = false;
        let mut scored: Vec<(usize, f64, f64)> = Vec::new(); // (idx, p, delta)
        for (idx, cand) in record.candidates.iter().enumerate() {
            match cand.status {
                CandidateStatus::Succeeded => {
                    let ofv = cand.ofv.context("succeeded candidate without OFV")?;
                    let r = lrt(reference_ofv, ofv, cand.df, phase);
                    scored.push((idx, r.p_value, r.delta_ofv));
                }
                _ => any_unusable = true,
            }
        }

        let alpha = match phase {
            Direction::Forward => plan.options.forward_alpha,
            Direction::Backward => plan.options.backward_alpha,
        };

        let winner_idx: Option<usize> = match phase {
            // Forward: most significant improvement below alpha wins.
            Direction::Forward => scored
                .iter()
                .filter(|(_, p, _)| *p < alpha)
                .min_by(|a, b| {
                    a.1.partial_cmp(&b.1)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then(a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
                })
                .map(|(idx, _, _)| *idx),
            // Backward: the least-needed covariate (largest p above alpha) is dropped.
            Direction::Backward => scored
                .iter()
                .filter(|(_, p, _)| *p > alpha)
                .max_by(|a, b| {
                    a.1.partial_cmp(&b.1)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then(b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal))
                })
                .map(|(idx, _, _)| *idx),
        };

        // Write scoring results into the state's round record.
        {
            let round = state
                .find_round_mut(&round_name)
                .context("internal error: round record missing")?;
            round.reference_ofv = Some(reference_ofv);
            for (idx, p, delta) in &scored {
                let cand = &mut round.candidates[*idx];
                cand.delta_ofv = Some(*delta);
                cand.p_value = Some(*p);
                cand.significant = Some(*p < alpha);
            }
            if let Some(w) = winner_idx {
                round.candidates[w].selected = true;
                round.winner = Some(round.candidates[w].candidate.clone());
            }
            round.complete = true;
        }
        if any_unusable {
            state.had_unusable = true;
        }

        // ---- Decide ----
        match winner_idx {
            Some(w) => {
                let round = state.find_round_mut(&round_name).unwrap();
                let name = round.candidates[w].candidate.clone();
                let model = round.candidates[w].model.clone();
                let ofv = round.candidates[w].ofv;
                let p = round.candidates[w].p_value.unwrap_or(f64::NAN);
                let delta = round.candidates[w].delta_ofv.unwrap_or(f64::NAN);
                match phase {
                    Direction::Forward => {
                        round.decision = format!("added {name} (p = {p:.3e}, dOFV = {delta:+.3})");
                        state.retained.push(name);
                    }
                    Direction::Backward => {
                        round.decision =
                            format!("dropped {name} (p = {p:.3e}, dOFV = {delta:+.3})");
                        state.retained.retain(|n| *n != name);
                    }
                }
                state.reference_model = Some(model);
                state.reference_ofv = ofv;
            }
            None => {
                let decision = match phase {
                    Direction::Forward => format!(
                        "no candidate significant at alpha {alpha}; forward selection stopped"
                    ),
                    Direction::Backward => format!(
                        "every covariate significant at alpha {alpha}; backward elimination stopped"
                    ),
                };
                state.find_round_mut(&round_name).unwrap().decision = decision;
                advance_phase(state, &phases);
            }
        }

        rounds_this_invocation += 1;
        state.save(out_dir)?;
        // The round's record lives in its own directory, and the decision
        // log on disk always matches the state — not just at the end.
        super::log::write_round_summary(out_dir, plan, state, &round_name)?;
        super::log::write_decision_log(out_dir, plan, state)?;

        if state.phase.is_none() {
            break;
        }
    }

    // ---- Final model ----
    write_final_model(&ctx, state)?;
    state.save(out_dir)?;

    if state.had_unusable {
        state.message = Some(
            "search completed, but some candidates were unusable (see the decision log); they were reported, never scored as insignificant"
                .to_string(),
        );
    }

    Ok(ScmRunStatus::Completed)
}

struct DriveContext<'a> {
    plan: &'a ScmPlan,
    out_dir: &'a Path,
    template: &'a Path,
    stem: &'a str,
    with_metadata: bool,
}

/// Move to the next phase (or finish). Forward -> Backward only makes sense
/// when something was retained.
fn advance_phase(state: &mut ScmState, phases: &[Direction]) {
    let current = state.phase.expect("advance_phase requires a phase");
    let next = phases.iter().skip_while(|p| **p != current).nth(1).copied();
    state.phase = match next {
        Some(Direction::Backward) if state.retained.is_empty() => None,
        other => other,
    };
}

/// Fit every entry of a round to a conclusion (success or retries exhausted),
/// resuming from whatever already exists on disk and in the state.
#[allow(clippy::too_many_arguments)]
fn run_round_fits(
    ctx: &DriveContext<'_>,
    executor: &dyn FitExecutor,
    state: &mut ScmState,
    round_name: &str,
    dir_name: &str,
    direction: Direction,
    reference_model: &str,
    entries: Vec<RoundEntry>,
) -> Result<RoundRecord> {
    let round_dir = ctx.out_dir.join(dir_name);
    fs::create_dir_all(&round_dir)?;

    // First attempts warm-start from the current reference fit's estimates;
    // the reference fit itself starts from the template.
    let reference_ext = if reference_model == "-" {
        None
    } else {
        Some(ext_path_for(&ctx.out_dir.join(reference_model)))
    };

    // Reuse an existing (incomplete) record on resume, or start a new one.
    let existing = state
        .rounds
        .iter()
        .position(|r| r.name == round_name && !r.complete);
    let round_idx = match existing {
        Some(idx) => idx,
        None => {
            state.rounds.push(RoundRecord {
                name: round_name.to_string(),
                direction,
                reference_model: reference_model.to_string(),
                reference_ofv: state.reference_ofv,
                candidates: entries
                    .iter()
                    .map(|e| CandidateRecord::new(&e.candidate, e.action.clone(), 1))
                    .collect(),
                winner: None,
                decision: String::new(),
                complete: false,
            });
            state.rounds.len() - 1
        }
    };

    let max_attempts = ctx.plan.options.max_retries + 1;

    // Wave loop: each wave gives every unconcluded candidate one attempt.
    for _wave in 0..max_attempts {
        let mut to_fit: Vec<PathBuf> = Vec::new();
        let mut fitted_candidates: Vec<usize> = Vec::new();

        for (idx, entry) in entries.iter().enumerate() {
            let cand = &state.rounds[round_idx].candidates[idx];
            if cand.status.is_concluded() {
                continue;
            }

            let attempt = cand.n_attempts() + 1;
            if attempt > max_attempts {
                continue; // concluded below
            }

            let model_name = scm_model_name(ctx.stem, &entry.candidate, attempt);
            let model_path = round_dir.join(format!("{model_name}.mod"));

            if !model_path.exists() {
                let description = format!("SCM {round_name}: {} (attempt {attempt})", entry.action);
                let based_on = if reference_model == "-" {
                    None
                } else {
                    Some(format!("../{reference_model}"))
                };
                if attempt == 1 {
                    write_scm_model(
                        ctx.template,
                        &model_path,
                        &entry.released,
                        ctx.plan.options.release_init,
                        reference_ext.as_deref(),
                        ctx.plan.options.cov_step,
                        &description,
                        based_on.as_deref(),
                        ctx.with_metadata,
                    )?;
                } else {
                    let prev_name = scm_model_name(ctx.stem, &entry.candidate, attempt - 1);
                    let prev_path = round_dir.join(format!("{prev_name}.mod"));
                    write_retry_model(
                        &prev_path,
                        &model_path,
                        &description,
                        based_on.as_deref(),
                        ctx.with_metadata,
                    )?;
                }
            }

            // Resume: a usable outcome may already be on disk.
            let outcome = read_fit_outcome(&model_path)?;
            let cand = &mut state.rounds[round_idx].candidates[idx];
            // A finished fit (usable or not) from a previous invocation is
            // recorded as-is; a failure then gets its retry in the next wave.
            if outcome.finished || outcome.terminated {
                conclude_attempt(cand, &model_path, ctx.out_dir, &outcome);
            } else {
                cand.status = CandidateStatus::Running;
                cand.model = rel_to(&model_path, ctx.out_dir);
                to_fit.push(model_path);
                fitted_candidates.push(idx);
            }
        }

        state.save(ctx.out_dir)?;

        if !to_fit.is_empty() {
            executor.fit(&to_fit)?;

            for (list_pos, idx) in fitted_candidates.iter().enumerate() {
                let outcome = read_fit_outcome(&to_fit[list_pos])?;
                let cand = &mut state.rounds[round_idx].candidates[*idx];
                conclude_attempt(cand, &to_fit[list_pos], ctx.out_dir, &outcome);
            }
            state.save(ctx.out_dir)?;
        }

        let all_concluded = state.rounds[round_idx]
            .candidates
            .iter()
            .all(|c| c.status.is_concluded());
        if all_concluded {
            break;
        }
    }

    // Anything still unconcluded is out of retries.
    for cand in &mut state.rounds[round_idx].candidates {
        if !cand.status.is_concluded() {
            cand.status = CandidateStatus::Unusable;
        }
    }
    state.save(ctx.out_dir)?;

    Ok(state.rounds[round_idx].clone())
}

fn conclude_attempt(
    cand: &mut CandidateRecord,
    model_path: &Path,
    out_dir: &Path,
    outcome: &super::round::FitOutcome,
) {
    let rel = rel_to(model_path, out_dir);
    cand.attempts.push(AttemptRecord {
        model: rel.clone(),
        outcome: outcome.label(),
    });
    cand.model = rel;
    cand.heuristics = outcome.heuristics.clone();
    if outcome.usable() {
        cand.status = CandidateStatus::Succeeded;
        cand.ofv = outcome.ofv;
    } else {
        cand.status = CandidateStatus::Pending;
    }
    // Every finished run gets its `pharos nonmem summary` written beside its
    // outputs (best effort; terminated runs have nothing to summarize).
    if outcome.finished && !outcome.terminated {
        write_run_summary(model_path);
    }
}

/// Build (but do not fit) the final model: the template with the retained
/// covariates released, warm-started from the final reference fit.
/// Unselected candidates stay `(0 FIX)`, documenting what was tested.
fn write_final_model(ctx: &DriveContext<'_>, state: &mut ScmState) -> Result<()> {
    let final_dir = ctx.out_dir.join("final");
    let final_path = final_dir.join(format!("{}_scm_final.mod", ctx.stem));

    let released = ctx.plan.thetas_for(&state.retained);
    let description = if state.retained.is_empty() {
        "SCM final model: no covariates retained".to_string()
    } else {
        format!("SCM final model: retained {}", state.retained.join(", "))
    };
    let based_on = state.reference_model.as_ref().map(|m| format!("../{m}"));
    let reference_ext = state
        .reference_model
        .as_ref()
        .map(|m| ext_path_for(&ctx.out_dir.join(m)));

    write_scm_model(
        ctx.template,
        &final_path,
        &released,
        ctx.plan.options.release_init,
        reference_ext.as_deref(),
        ctx.plan.options.cov_step,
        &description,
        based_on.as_deref(),
        ctx.with_metadata,
    )?;

    state.final_model = Some(rel_to(&final_path, ctx.out_dir));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scm::plan::tests::write_template;
    use crate::scm::{ScmOptions, build_plan};
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Fabricates pharos run outputs instead of running NONMEM. Behavior is
    /// keyed by `"{round_dir}/{model_stem_without_try_suffix}"`; the Vec gives
    /// the OFV per attempt, `None` meaning "ran but never reached final
    /// estimates" (which is retryable).
    struct MockExecutor {
        behaviors: HashMap<String, Vec<Option<f64>>>,
        default_ofv: f64,
        fits: Mutex<Vec<String>>,
    }

    impl MockExecutor {
        fn new(default_ofv: f64) -> Self {
            Self {
                behaviors: HashMap::new(),
                default_ofv,
                fits: Mutex::new(vec![]),
            }
        }

        fn with(mut self, key: &str, attempts: Vec<Option<f64>>) -> Self {
            self.behaviors.insert(key.to_string(), attempts);
            self
        }

        fn key_and_attempt(model: &Path) -> (String, usize) {
            let stem = model.file_stem().unwrap().to_string_lossy().to_string();
            let dir = model
                .parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let (base, attempt) = match stem.rfind("_try") {
                Some(pos) if stem[pos + 4..].chars().all(|c| c.is_ascii_digit()) => {
                    let n: usize = stem[pos + 4..].parse().unwrap();
                    (stem[..pos].to_string(), n)
                }
                _ => (stem, 1),
            };
            (format!("{dir}/{base}"), attempt)
        }

        fn fit_count(&self, needle: &str) -> usize {
            self.fits
                .lock()
                .unwrap()
                .iter()
                .filter(|f| f.contains(needle))
                .count()
        }
    }

    impl FitExecutor for MockExecutor {
        fn fit(&self, models: &[PathBuf]) -> Result<()> {
            for model in models {
                let (key, attempt) = Self::key_and_attempt(model);
                self.fits.lock().unwrap().push(key.clone());

                let ofv = match self.behaviors.get(&key) {
                    Some(attempts) => attempts
                        .get(attempt - 1)
                        .copied()
                        .unwrap_or(Some(self.default_ofv)),
                    None => Some(self.default_ofv),
                };

                let stem = model.file_stem().unwrap().to_string_lossy().to_string();
                let run_dir = model.parent().unwrap().join(&stem);
                fs::create_dir_all(&run_dir)?;
                fs::write(run_dir.join("pharos_start.json"), "{}")?;
                fs::write(run_dir.join("pharos_end.json"), "{}")?;

                let mut ext = String::from(
                    "TABLE NO.     1: First Order Conditional Estimation with Interaction\n",
                );
                ext.push_str(" ITERATION    THETA1       THETA2       THETA3       THETA4       THETA5       THETA6       OMEGA(1,1)   OMEGA(2,2)   SIGMA(1,1)   OBJ\n");
                ext.push_str("            0  3.00000E+00  2.00000E+01  1.20000E+00  1.00000E-01  1.00000E-01  1.00000E-01  1.00000E-01  1.00000E-01  2.00000E-02  1100\n");
                ext.push_str("            8  1.11000E-01  2.22000E-01  3.33000E-01  4.44000E-01  5.55000E-01  6.66000E-01  9.00000E-02  9.00000E-02  1.90000E-02  1050\n");
                if let Some(ofv) = ofv {
                    ext.push_str(&format!(
                        "  -1000000000  3.10000E+00  2.10000E+01  1.30000E+00  2.50000E-01  1.50000E-01  5.00000E-02  8.00000E-02  8.50000E-02  1.80000E-02  {ofv}\n"
                    ));
                }
                fs::write(run_dir.join(format!("{stem}.ext")), ext)?;
            }
            Ok(())
        }

        fn describe(&self) -> String {
            "mock".to_string()
        }
    }

    fn make_plan(dir: &Path, options: ScmOptions) -> ScmPlan {
        let template = write_template(dir);
        build_plan(&template, &[4, 5, 6], None, options, "test")
            .unwrap()
            .plan
    }

    /// The full fixture: forward picks WT_CL then CRCL_CL (with a retry on
    /// WT_V in round 2), forward stops in round 3, backward drops CRCL_CL at
    /// the stricter alpha, then keeps WT_CL and stops.
    fn full_search_executor() -> MockExecutor {
        MockExecutor::new(1234.0)
            .with("base/1001_base", vec![Some(1000.0)])
            // forward round 1: WT_CL wins big
            .with("forward_round1/1001_wt_cl", vec![Some(980.0)])
            .with("forward_round1/1001_crcl_cl", vec![Some(996.0)])
            .with("forward_round1/1001_wt_v", vec![Some(999.0)])
            // forward round 2 (ref 980): CRCL_CL wins; WT_V fails once, then succeeds
            .with("forward_round2/1001_crcl_cl", vec![Some(974.0)])
            .with("forward_round2/1001_wt_v", vec![None, Some(978.5)])
            // forward round 3 (ref 974): WT_V not significant -> forward stops
            .with("forward_round3/1001_wt_v", vec![Some(973.0)])
            // backward round 1 (ref 974): dropping WT_CL hurts a lot (keep),
            // dropping CRCL_CL costs 6 points (p ~ 0.014 > 0.001 -> drop)
            .with("backward_round1/1001_wt_cl", vec![Some(995.0)])
            .with("backward_round1/1001_crcl_cl", vec![Some(980.0)])
            // backward round 2 (ref 980): dropping WT_CL still hurts -> stop
            .with("backward_round2/1001_wt_cl", vec![Some(1000.0)])
    }

    #[test]
    fn full_forward_backward_search() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        let executor = full_search_executor();

        let outcome = run_scm(&plan, &executor).unwrap();
        assert_eq!(outcome.state.status, ScmRunStatus::Completed);
        let state = &outcome.state;

        assert_eq!(state.retained, vec!["WT_CL".to_string()]);
        assert!(!state.had_unusable);

        let round_names: Vec<&str> = state.rounds.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(
            round_names,
            vec![
                "reference",
                "forward_round1",
                "forward_round2",
                "forward_round3",
                "backward_round1",
                "backward_round2",
            ]
        );

        // The reference fit is recorded complete with its OFV in the decision
        let r0 = &state.rounds[0];
        assert!(r0.complete);
        assert!(r0.decision.contains("base model fitted"), "{}", r0.decision);

        // Round 1: WT_CL selected, all three tested
        let r1 = &state.rounds[1];
        assert_eq!(r1.winner.as_deref(), Some("WT_CL"));
        assert_eq!(r1.candidates.len(), 3);
        assert!(r1.decision.starts_with("added WT_CL"));
        let wt_cl = r1
            .candidates
            .iter()
            .find(|c| c.candidate == "WT_CL")
            .unwrap();
        assert_eq!(wt_cl.delta_ofv, Some(-20.0));
        assert_eq!(wt_cl.significant, Some(true));
        assert!(wt_cl.selected);

        // Round-2 models warm-start from the round-1 winner's fit: the
        // retained WT_CL theta carries its estimate (THETA4 = 0.25) and the
        // base thetas continue from the reference (THETA1 = 3.1) instead of
        // resetting to the template's initial estimates.
        let r2_model = plan.out_dir_path().join("forward_round2/1001_crcl_cl.mod");
        let r2_content = fs::read_to_string(&r2_model).unwrap();
        assert!(r2_content.contains("0.25"), "{r2_content}");
        assert!(r2_content.contains("3.1"), "{r2_content}");

        // Round 2: WT_V needed a retry that started from the previous attempt
        let r2 = &state.rounds[2];
        let wt_v = r2
            .candidates
            .iter()
            .find(|c| c.candidate == "WT_V")
            .unwrap();
        assert_eq!(wt_v.n_attempts(), 2);
        assert_eq!(wt_v.attempts[0].outcome, "no ofv");
        assert_eq!(wt_v.attempts[1].outcome, "succeeded");
        assert!(wt_v.model.ends_with("_try2.mod"));

        // The retry model's released theta continues from the failed
        // attempt's last iteration (THETA6 = 0.666), not from 0.1.
        let retry_path = plan.out_dir_path().join(&wt_v.model);
        let retry_content = fs::read_to_string(&retry_path).unwrap();
        assert!(retry_content.contains("0.666"), "{retry_content}");

        // Backward: CRCL_CL dropped at the stricter alpha, WT_CL kept
        let b1 = &state.rounds[4];
        assert_eq!(b1.winner.as_deref(), Some("CRCL_CL"));
        assert!(b1.decision.starts_with("dropped CRCL_CL"));
        let b2 = &state.rounds[5];
        assert!(b2.winner.is_none());
        assert!(b2.decision.contains("backward elimination stopped"));

        // Final model exists, WT_CL released with estimates from the final
        // reference fit (THETA4 final estimate 0.25), others still fixed.
        let final_model = plan
            .out_dir_path()
            .join(state.final_model.as_ref().unwrap());
        assert!(final_model.exists());
        let content = fs::read_to_string(&final_model).unwrap();
        assert!(content.contains("0.25"), "{content}");
        assert!(content.contains("(0 FIX)   ; CRCL_CL cov"), "{content}");
        assert!(content.contains("(0 FIX)   ; WT_V cov"), "{content}");

        // Decision log written on completion
        assert!(
            plan.out_dir_path()
                .join(super::super::DECISION_LOG_CSV)
                .exists()
        );
        assert!(
            plan.out_dir_path()
                .join(super::super::DECISION_LOG_MD)
                .exists()
        );

        // Every concluded round left its summary in its own directory
        for round_dir in [
            "base",
            "forward_round1",
            "forward_round2",
            "forward_round3",
            "backward_round1",
            "backward_round2",
        ] {
            let dir = plan.out_dir_path().join(round_dir);
            assert!(dir.join("round_summary.json").exists(), "{round_dir}");
            assert!(dir.join("round_summary.md").exists(), "{round_dir}");
        }
        let r1_summary: crate::scm::RoundSummary = serde_json::from_str(
            &fs::read_to_string(
                plan.out_dir_path()
                    .join("forward_round1/round_summary.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(r1_summary.all_succeeded);
        assert!(!r1_summary.any_unusable);
        assert_eq!(r1_summary.winner.as_deref(), Some("WT_CL"));
        assert_eq!(r1_summary.retained_after, vec!["WT_CL".to_string()]);
        assert_eq!(r1_summary.next, "continue forward selection");

        // The last round's summary was refreshed with the terminal status
        let last_summary: crate::scm::RoundSummary = serde_json::from_str(
            &fs::read_to_string(
                plan.out_dir_path()
                    .join("backward_round2/round_summary.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(last_summary.search_status, "completed");
        assert!(last_summary.next.contains("final model"), "{last_summary:?}");

        // Status reads back coherently
        let status = crate::scm::read_status(&plan.out_dir_path()).unwrap();
        assert_eq!(status.status, "completed");
        assert_eq!(status.rounds_complete, 5);
        assert_eq!(status.retained, vec!["WT_CL".to_string()]);
        let text = status.render_text();
        assert!(text.contains("forward_round1"));
        assert!(text.contains("added WT_CL"));
    }

    #[test]
    fn num_rounds_pauses_and_resume_completes_without_refitting() {
        let dir = tempfile::tempdir().unwrap();
        let options = ScmOptions {
            num_rounds: Some(1),
            ..Default::default()
        };
        let plan = make_plan(dir.path(), options);
        let executor = full_search_executor();

        // First invocation: reference + one round, then pause
        let outcome = run_scm(&plan, &executor).unwrap();
        assert_eq!(outcome.state.status, ScmRunStatus::Paused);
        assert_eq!(outcome.state.completed_search_rounds(), 1);
        assert_eq!(outcome.state.retained, vec!["WT_CL".to_string()]);

        // Status shows the pause
        let status = crate::scm::read_status(&plan.out_dir_path()).unwrap();
        assert_eq!(status.status, "paused");

        // The decision log is on disk after the pause, not just at the end
        let csv_path = plan.out_dir_path().join(super::super::DECISION_LOG_CSV);
        assert!(csv_path.exists());
        let csv = fs::read_to_string(&csv_path).unwrap();
        assert!(csv.contains("forward_round1"), "{csv}");

        // Resume until done
        let mut last = None;
        for _ in 0..10 {
            let outcome = run_scm(&plan, &executor).unwrap();
            let done = outcome.state.status == ScmRunStatus::Completed;
            last = Some(outcome);
            if done {
                break;
            }
        }
        let outcome = last.unwrap();
        assert_eq!(outcome.state.status, ScmRunStatus::Completed);
        assert_eq!(outcome.state.retained, vec!["WT_CL".to_string()]);
        assert_eq!(outcome.state.completed_search_rounds(), 5);

        // Nothing was fitted twice: each round-1 model exactly once
        assert_eq!(executor.fit_count("forward_round1/1001_wt_cl"), 1);
        assert_eq!(executor.fit_count("forward_round1/1001_crcl_cl"), 1);
        assert_eq!(executor.fit_count("base/1001_base"), 1);
    }

    #[test]
    fn unusable_candidate_is_reported_not_scored() {
        let dir = tempfile::tempdir().unwrap();
        let options = ScmOptions {
            direction: vec![Direction::Forward],
            max_retries: 1,
            ..Default::default()
        };
        let plan = make_plan(dir.path(), options);

        // WT_V never produces an OFV; WT_CL is barely significant, CRCL_CL not
        let executor = MockExecutor::new(1234.0)
            .with("base/1001_base", vec![Some(1000.0)])
            .with("forward_round1/1001_wt_cl", vec![Some(995.0)])
            .with("forward_round1/1001_crcl_cl", vec![Some(999.5)])
            .with("forward_round1/1001_wt_v", vec![None, None])
            .with("forward_round2/1001_crcl_cl", vec![Some(994.0)])
            .with("forward_round2/1001_wt_v", vec![None, None]);

        let outcome = run_scm(&plan, &executor).unwrap();
        assert_eq!(outcome.state.status, ScmRunStatus::Completed);
        let state = &outcome.state;
        assert!(state.had_unusable);
        assert!(state.message.as_ref().unwrap().contains("unusable"));

        let r1 = &state.rounds[1];
        let wt_v = r1
            .candidates
            .iter()
            .find(|c| c.candidate == "WT_V")
            .unwrap();
        assert_eq!(wt_v.status, CandidateStatus::Unusable);
        assert_eq!(wt_v.n_attempts(), 2); // 1 + max_retries
        assert_eq!(wt_v.p_value, None); // never scored
        assert_eq!(wt_v.significant, None);

        // WT_CL still won the round despite the unusable sibling
        assert_eq!(r1.winner.as_deref(), Some("WT_CL"));

        // The round summary reports the unusable candidate
        let r1_summary: crate::scm::RoundSummary = serde_json::from_str(
            &fs::read_to_string(
                plan.out_dir_path()
                    .join("forward_round1/round_summary.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(r1_summary.any_unusable);
        assert!(!r1_summary.all_succeeded);
    }

    #[test]
    fn backward_only_starts_from_the_full_model() {
        let dir = tempfile::tempdir().unwrap();
        let options = ScmOptions {
            direction: vec![Direction::Backward],
            ..Default::default()
        };
        let plan = make_plan(dir.path(), options);

        let executor = MockExecutor::new(1234.0)
            .with("full/1001_full", vec![Some(900.0)])
            // dropping WT_CL is free; the others are needed
            .with("backward_round1/1001_wt_cl", vec![Some(900.5)])
            .with("backward_round1/1001_crcl_cl", vec![Some(950.0)])
            .with("backward_round1/1001_wt_v", vec![Some(930.0)])
            .with("backward_round2/1001_crcl_cl", vec![Some(951.0)])
            .with("backward_round2/1001_wt_v", vec![Some(931.0)]);

        let outcome = run_scm(&plan, &executor).unwrap();
        assert_eq!(outcome.state.status, ScmRunStatus::Completed);
        let state = &outcome.state;

        // Full model was the reference and released everything
        let full_model = plan.out_dir_path().join("full/1001_full.mod");
        let content = fs::read_to_string(&full_model).unwrap();
        assert!(!content.contains("(0 FIX)"), "{content}");

        assert_eq!(
            state.retained,
            vec!["CRCL_CL".to_string(), "WT_V".to_string()]
        );
        assert!(state.rounds[1].decision.starts_with("dropped WT_CL"));
    }

    #[test]
    fn mismatched_state_requires_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        let executor = full_search_executor();
        run_scm(&plan, &executor).unwrap();

        // Same out_dir, different alphas -> refuses without overwrite
        let mut changed = plan.clone();
        changed.options.forward_alpha = 0.01;
        let err = run_scm(&changed, &executor).unwrap_err();
        assert!(err.to_string().contains("different plan"), "got: {err}");

        // With overwrite it restarts cleanly
        changed.options.overwrite = true;
        let outcome = run_scm(&changed, &executor).unwrap();
        assert_eq!(outcome.state.status, ScmRunStatus::Completed);
    }
}
