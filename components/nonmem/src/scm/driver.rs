use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use config::NonmemConfig;
use fs_err as fs;

use super::roster::{Compatibility, apply_removals, apply_retunes, compatibility};
use super::round::{
    FitOutcome, ModelWriter, RoundEntry, backward_entries, ext_path_for, forward_entries,
    read_fit_outcome, record_attempt, run_finished, scm_model_name, write_run_summary,
};
use super::state::{CandidateRecord, CandidateStatus, RoundRecord, ScmRunStatus, ScmState};
use super::{
    Direction, NO_REFERENCE, REFERENCE_ROUND, SCM_SUMMARY_FILENAME, SCM_SUMMARY_MD, ScmPlan,
    none_or_list, on_off,
};
use crate::run::RunOptions;
use crate::runner::run_models;

/// Fits a batch of models to completion.
pub trait FitExecutor {
    fn fit(&self, models: &[PathBuf]) -> Result<()>;
    fn describe(&self) -> String;
    /// The project settings the fits run under (where output lands, which
    /// comment dialect names parameters)
    fn settings(&self) -> Result<NonmemConfig> {
        Ok(NonmemConfig::default())
    }
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

    fn settings(&self) -> Result<NonmemConfig> {
        Ok(self.nonmem_config.clone())
    }
}

/// Metadata files require a pharos project root that contains the output directory
fn metadata_enabled(out_dir: &Path) -> bool {
    fs::canonicalize(out_dir).is_ok_and(|dir| config::to_config_relative(dir).is_ok())
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

/// Remove previous SCM output - only known SCM subdirectories are touched; plan.json stays.
pub(crate) fn clear_previous_output(out_dir: &Path) -> Result<()> {
    if !out_dir.exists() {
        return Ok(());
    }
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

    for name in [SCM_SUMMARY_MD, SCM_SUMMARY_FILENAME] {
        let path = out_dir.join(name);
        if path.exists() {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

/// Refresh the on-disk record of the SCM process.
fn write_records(
    out_dir: &Path,
    plan: &ScmPlan,
    state: &ScmState,
    round_name: &str,
    settings: &NonmemConfig,
) -> Result<()> {
    let (summary, fits) = super::summary::build_summary(plan, state, out_dir, settings);
    super::summary::write_round_summary(out_dir, &summary, round_name, &fits)?;
    Ok(())
}

/// Run (or resume) the SCM process described by `plan`.
pub fn run_scm(plan: &ScmPlan, executor: &dyn FitExecutor, overwrite: bool) -> Result<ScmState> {
    if plan.candidates.is_empty() {
        bail!("plan has no candidates");
    }
    let out_dir = plan.out_dir_path();
    fs::create_dir_all(&out_dir)?;

    if overwrite {
        log::info!(
            "starting the SCM process over: discarding previous output in {}",
            out_dir.display()
        );
        clear_previous_output(&out_dir)?;
    }

    // Whether the state on disk is this plan's to resume.
    let mut state = match ScmState::load(&out_dir)? {
        Some(mut s) => match compatibility(plan, &s) {
            Compatibility::Identical => {
                log::info!("resuming SCM process in {}", out_dir.display());
                s
            }
            Compatibility::Compatible { removals, retunes } => {
                log::info!("resuming SCM process in {}", out_dir.display());
                for line in apply_removals(&mut s, &removals) {
                    log::info!("{line}");
                }
                for line in apply_retunes(&mut s, &retunes) {
                    log::info!("retuned {line}");
                }
                s
            }
            Compatibility::Incompatible { reasons, .. } => bail!(
                "{} contains SCM state from a different plan:\n  {}\nrun with --overwrite to discard it, or use a fresh out_dir",
                out_dir.display(),
                reasons.join("\n  ")
            ),
        },
        None => ScmState::new(plan),
    };

    // Keep the plan on disk next to the state for the record.
    plan.save()?;

    state.status = ScmRunStatus::Running;
    state.message = None;
    state.save(&out_dir)?;

    let settings = executor.settings()?;
    match drive(plan, executor, &mut state, &out_dir, &settings) {
        Ok(status) => {
            state.status = status;
            state.save(&out_dir)?;
            if status == ScmRunStatus::Completed
                && let Some(round) = state.rounds.last()
            {
                write_records(&out_dir, plan, &state, &round.name.clone(), &settings)?;
            }
            Ok(state)
        }
        Err(e) => {
            state.status = ScmRunStatus::Failed;
            state.message = Some(format!("{e:#}"));
            state.save(&out_dir)?;
            // Best-effort record of the failing round in its own directory.
            if let Some(round) = state.rounds.last() {
                let _ = write_records(&out_dir, plan, &state, &round.name.clone(), &settings);
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
    settings: &NonmemConfig,
) -> Result<ScmRunStatus> {
    let template = plan.model_path();
    if !template.exists() {
        bail!(
            "initial model {} does not exist (scm commands run from the pharos project root)",
            template.display()
        );
    }
    let stem = crate::ModelLayout::for_model_path(&template)?
        .stem()
        .to_string();
    let with_metadata = metadata_enabled(out_dir);
    log::info!(
        "SCM process on {} via {} executor (metadata: {})",
        template.display(),
        executor.describe(),
        with_metadata
    );

    let phases = plan.options.phases();

    let ctx = DriveContext {
        plan,
        out_dir,
        writer: ModelWriter {
            template: &template,
            candidates: &plan.candidates,
            with_metadata,
        },
        stem: &stem,
        settings,
    };

    // ---- Reference fit (not an SCM round) ----
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

        // A reference fit that ran out of retries is terminal: `Unusable` counts
        // as concluded, so a plain resume would skip every attempt and replay the
        // same verdict without fitting anything. A reference fit left pending or running is
        // a different case: that one is resumed, since its fits may still be on their way.
        let gave_up = state
            .rounds
            .iter()
            .find(|r| r.name == REFERENCE_ROUND)
            .is_some_and(|r| {
                r.candidates
                    .iter()
                    .any(|c| c.status == CandidateStatus::Unusable)
            });
        if gave_up {
            log::info!("the previous {ref_name} fit was given up on; starting it over");
            let stale = out_dir.join(ref_name);
            if stale.exists() {
                fs::remove_dir_all(&stale)?;
            }
            state.rounds.retain(|r| r.name != REFERENCE_ROUND);
            state.save(out_dir)?;
        }

        let entries = vec![RoundEntry {
            candidate: ref_name.to_string(),
            action,
            released: plan.thetas_for(&released_names),
            // Reference fits are never LRT-scored against anything.
            df: 0,
        }];

        let record = run_round_fits(
            &ctx,
            executor,
            state,
            REFERENCE_ROUND,
            first,
            NO_REFERENCE,
            entries,
        )?;
        let cand = record.candidates[0].clone();
        if cand.status != CandidateStatus::Succeeded {
            let message = format!(
                "reference model ({ref_name}) failed after {} attempt(s); the SCM process cannot start",
                cand.n_attempts()
            );

            if let Some(round) = state.find_round_mut(REFERENCE_ROUND) {
                round.complete = true;
                round.decision = format!(
                    "{ref_name} model failed after {} attempt(s)",
                    cand.n_attempts()
                );
            }
            state.save(out_dir)?;
            bail!(message);
        }
        if let Some(round) = state.find_round_mut(REFERENCE_ROUND) {
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
        write_records(out_dir, plan, state, REFERENCE_ROUND, settings)?;
    }

    let mut rounds_this_invocation = 0usize;

    // ---- SCM rounds ----
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
            .filter(|r| r.direction == phase && !r.is_reference() && r.complete)
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
            phase,
            &reference_model,
            entries,
        )?;

        // ---- Score the round ----
        for cand in &record.candidates {
            if cand.status != CandidateStatus::Succeeded {
                continue;
            }
            cand.ofv.context("succeeded candidate without OFV")?;
            if cand.df == 0 {
                bail!(
                    "internal error: candidate {} in {round_name} has 0 degrees of freedom",
                    cand.candidate
                );
            }
        }
        let any_unusable = record.candidates.iter().any(|c| {
            !matches!(
                c.status,
                CandidateStatus::Succeeded | CandidateStatus::Withdrawn
            )
        });

        let round = state
            .find_round_mut(&round_name)
            .context("internal error: round record missing")?;
        round.reference_ofv = Some(reference_ofv);
        let scored = round.score(&plan.options);
        let alpha = round
            .alpha(&plan.options)
            .expect("an SCM round has an alpha");
        let record = round.clone();
        if any_unusable {
            state.had_unusable = true;
        }

        // The best contender wins
        let contenders = record.contenders(&plan.options);
        let winner_idx = contenders.first().map(|s| s.index);
        if let [best, next, ..] = contenders.as_slice()
            && best.key() == next.key()
        {
            log::info!(
                "{round_name}: {} and {} score identically (p = {:.3e}, dOFV = {:+.3}); {} wins as the earlier $THETA",
                record.candidates[best.index].candidate,
                record.candidates[next.index].candidate,
                best.p_value,
                best.delta_ofv,
                record.candidates[best.index].candidate
            );
        }

        {
            let round = state
                .find_round_mut(&round_name)
                .context("internal error: round record missing")?;
            if let Some(w) = winner_idx {
                round.candidates[w].selected = true;
                round.winner = Some(round.candidates[w].candidate.clone());
            }
            round.complete = true;
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
                let n_unusable = record.unusable();
                let stopped = match phase {
                    Direction::Forward => "forward selection stopped",
                    Direction::Backward => "backward elimination stopped",
                };

                let decision = if scored.is_empty() {
                    format!("no candidate could be scored ({n_unusable} unusable); {stopped}")
                } else {
                    let verdict = match phase {
                        Direction::Forward => {
                            format!("no candidate significant at alpha {alpha}")
                        }
                        Direction::Backward => {
                            format!("every covariate significant at alpha {alpha}")
                        }
                    };
                    if n_unusable > 0 {
                        format!("{verdict} ({n_unusable} unusable, not scored); {stopped}")
                    } else {
                        format!("{verdict}; {stopped}")
                    }
                };
                state.find_round_mut(&round_name).unwrap().decision = decision;
                advance_phase(state, &phases);
            }
        }
        rounds_this_invocation += 1;
        state.save(out_dir)?;
        write_records(out_dir, plan, state, &round_name, settings)?;

        if state.phase.is_none() {
            break;
        }
    }

    write_final_model(&ctx, executor, state)?;
    state.save(out_dir)?;

    if state.had_unusable {
        state.message = Some(
            "SCM process completed, but some candidates were unusable (see scm_summary.md); they were reported, never scored as insignificant"
                .to_string(),
        );
    }
    Ok(ScmRunStatus::Completed)
}

struct DriveContext<'a> {
    plan: &'a ScmPlan,
    out_dir: &'a Path,
    writer: ModelWriter<'a>,
    stem: &'a str,
    settings: &'a NonmemConfig,
}

/// Move to the next phase (or finish).
fn advance_phase(state: &mut ScmState, phases: &[Direction]) {
    let current = state.phase.expect("advance_phase requires a phase");
    let next = phases.iter().skip_while(|p| **p != current).nth(1).copied();
    state.phase = match next {
        Some(Direction::Backward) if state.retained.is_empty() => None,
        other => other,
    };
}

/// Fit every entry of a round to a conclusion.
#[allow(clippy::too_many_arguments)]
fn run_round_fits(
    ctx: &DriveContext<'_>,
    executor: &dyn FitExecutor,
    state: &mut ScmState,
    round_name: &str,
    direction: Direction,
    reference_model: &str,
    entries: Vec<RoundEntry>,
) -> Result<RoundRecord> {
    let dir_name = if round_name == REFERENCE_ROUND {
        entries
            .first()
            .map(|e| e.candidate.clone())
            .context("reference round has no entry to name its directory")?
    } else {
        round_name.to_string()
    };
    let round_dir = ctx.out_dir.join(&dir_name);
    fs::create_dir_all(&round_dir)?;

    let reference_ext = if reference_model == NO_REFERENCE {
        None
    } else {
        Some(ext_path_for(
            &ctx.out_dir.join(reference_model),
            ctx.settings,
        )?)
    };

    // Reuse an existing (incomplete) record on resume, or start a new one.
    let existing = state
        .rounds
        .iter()
        .position(|r| r.name == round_name && (!r.complete || round_name == REFERENCE_ROUND));
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
                    .map(|e| CandidateRecord::new(&e.candidate, e.action.clone(), e.df))
                    .collect(),
                winner: None,
                decision: String::new(),
                complete: false,
            });
            state.rounds.len() - 1
        }
    };

    let max_attempts = ctx.plan.options.max_retries + 1;

    // A resumed round may have lost a candidate to a removal since it was recorded
    let mut record_index: Vec<usize> = Vec::with_capacity(entries.len());
    for entry in &entries {
        let round = &mut state.rounds[round_idx];
        let idx = match round
            .candidates
            .iter()
            .position(|c| c.candidate == entry.candidate)
        {
            Some(idx) => idx,
            None => {
                round.candidates.push(CandidateRecord::new(
                    &entry.candidate,
                    entry.action.clone(),
                    entry.df,
                ));
                round.candidates.len() - 1
            }
        };
        record_index.push(idx);
    }

    // Wave loop: each wave gives every unconcluded candidate one attempt.
    for _wave in 0..max_attempts {
        let mut to_fit: Vec<PathBuf> = Vec::new();
        let mut fitted_candidates: Vec<usize> = Vec::new();

        for (entry, &idx) in entries.iter().zip(&record_index) {
            let cand = &state.rounds[round_idx].candidates[idx];
            if cand.status.is_concluded() {
                continue;
            }

            let attempt = cand.n_attempts() + 1;
            if attempt > max_attempts {
                continue; // concluded below
            }

            let model_name = scm_model_name(ctx.stem, &entry.candidate, attempt, cand.refit);
            let model_path = round_dir.join(format!("{model_name}.mod"));

            if !model_path.exists() {
                let description = format!("SCM {round_name}: {} (attempt {attempt})", entry.action);
                let based_on = if reference_model == NO_REFERENCE {
                    None
                } else {
                    Some(format!("../{reference_model}"))
                };
                if attempt == 1 {
                    ctx.writer.write(
                        &model_path,
                        &entry.released,
                        reference_ext.as_deref(),
                        ctx.plan.options.cov_step,
                        &description,
                        based_on.as_deref(),
                    )?;
                } else {
                    let prev_name =
                        scm_model_name(ctx.stem, &entry.candidate, attempt - 1, cand.refit);
                    let prev_path = round_dir.join(format!("{prev_name}.mod"));
                    ctx.writer.retry(
                        &prev_path,
                        &model_path,
                        &description,
                        based_on.as_deref(),
                        ctx.settings,
                    )?;
                }
            }

            // Resume: a usable outcome may already be on disk.
            let outcome = conclude_or_read(ctx, &model_path)?;
            let cand = &mut state.rounds[round_idx].candidates[idx];

            if outcome.finished || outcome.terminated {
                record_attempt(cand, rel_to(&model_path, ctx.out_dir), &outcome);
            } else {
                cand.status = CandidateStatus::Running;
                cand.model = rel_to(&model_path, ctx.out_dir);
                to_fit.push(model_path);
                fitted_candidates.push(idx);
            }
        }

        state.save(ctx.out_dir)?;

        if !to_fit.is_empty() {
            if let Err(e) = executor.fit(&to_fit) {
                // Nothing was fitted: the candidates go back to pending
                for idx in &fitted_candidates {
                    state.rounds[round_idx].candidates[*idx].status = CandidateStatus::Pending;
                }
                state.save(ctx.out_dir)?;
                return Err(e);
            }

            for (list_pos, idx) in fitted_candidates.iter().enumerate() {
                let outcome = conclude_or_read(ctx, &to_fit[list_pos])?;
                let cand = &mut state.rounds[round_idx].candidates[*idx];
                record_attempt(cand, rel_to(&to_fit[list_pos], ctx.out_dir), &outcome);
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

/// Read how a model's run went
fn conclude_or_read(ctx: &DriveContext<'_>, model_path: &Path) -> Result<FitOutcome> {
    if run_finished(model_path, ctx.settings) {
        write_run_summary(model_path, ctx.settings);
    }
    read_fit_outcome(model_path, ctx.settings)
}

/// Build the final model
fn write_final_model(
    ctx: &DriveContext<'_>,
    executor: &dyn FitExecutor,
    state: &mut ScmState,
) -> Result<()> {
    let final_dir = ctx.out_dir.join("final");
    let final_path = final_dir.join(format!("{}_scm_final.mod", ctx.stem));

    let released = ctx.plan.thetas_for(&state.retained);
    let cov_step = ctx.plan.options.final_cov_step || ctx.plan.options.cov_step;
    let description = format!(
        "SCM final model: retained {}; cov step {}",
        none_or_list(&state.retained),
        on_off(cov_step)
    );
    let based_on = state.reference_model.as_ref().map(|m| format!("../{m}"));
    let reference_ext = state
        .reference_model
        .as_ref()
        .map(|m| ext_path_for(&ctx.out_dir.join(m), ctx.settings))
        .transpose()?;

    ctx.writer.write(
        &final_path,
        &released,
        reference_ext.as_deref(),
        cov_step,
        &description,
        based_on.as_deref(),
    )?;

    state.final_model = Some(rel_to(&final_path, ctx.out_dir));
    state.final_ofv = None;
    if !ctx.plan.options.final_cov_step {
        return Ok(());
    }

    // Resumable like every other fit: a final fit already finished on disk is
    // read rather than run again.
    let mut outcome = conclude_or_read(ctx, &final_path)?;
    if !outcome.finished && !outcome.terminated {
        executor.fit(std::slice::from_ref(&final_path))?;
        outcome = conclude_or_read(ctx, &final_path)?;
    }
    if outcome.finished && !outcome.terminated {
        state.final_ofv = outcome.ofv;
    } else {
        log::warn!(
            "final model {} did not minimize; it carries no covariance step results",
            final_path.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scm::ScmOptions;
    use crate::scm::round::RETRY_JITTER;
    use crate::scm::test_support::{Fit, MockExecutor, full_scm_executor, make_plan};

    /// What `scm status` prints.
    fn brief() -> crate::scm::SummaryOptions {
        crate::scm::SummaryOptions {
            brief: true,
            ..Default::default()
        }
    }

    #[test]
    fn full_forward_backward_scm() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        let executor = full_scm_executor();

        let outcome = run_scm(&plan, &executor, false).unwrap();
        assert_eq!(outcome.status, ScmRunStatus::Completed);
        let state = &outcome;

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
        // resetting to the initial model's own estimates.
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
        // attempt's last iteration (THETA6 = 0.666), not from 0.1, and lands
        // within the retry jitter of it rather than exactly on it.
        let retry_path = plan.out_dir_path().join(&wt_v.model);
        let retry_content = fs::read_to_string(&retry_path).unwrap();
        let theta6 = nonmem_parser::Model::parse(&retry_path, &retry_content)
            .unwrap()
            .thetas[5]
            .init;
        assert!(
            theta6 != 0.666 && (theta6 - 0.666).abs() <= 0.666 * RETRY_JITTER,
            "THETA6 = {theta6}, expected 0.666 jittered by at most {}%: {retry_content}",
            RETRY_JITTER * 100.0
        );

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

        // The process summary's markdown written on completion
        assert!(plan.out_dir_path().join(SCM_SUMMARY_MD).exists());

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
        assert_eq!(last_summary.scm_status, "completed");
        assert!(
            last_summary.next.contains("final model"),
            "{last_summary:?}"
        );

        // Status reads back coherently
        let status = crate::scm::read_summary(&plan.out_dir_path()).unwrap();
        assert_eq!(status.status, "completed");
        assert_eq!(status.totals.rounds_complete, 5);
        assert_eq!(status.retained, vec!["WT_CL".to_string()]);
        let text = status.render_text(&brief()).unwrap();
        assert!(text.contains("forward_round1"));
        assert!(text.contains("added WT_CL"));
    }

    /// The `$THETA` line a comment labels, for asserting on the spec a
    /// generated model gives one candidate.
    fn theta_spec(model: &str, label: &str) -> String {
        model
            .lines()
            .find(|l| l.starts_with("$THETA") && l.contains(label))
            .unwrap_or_else(|| panic!("no $THETA for {label} in\n{model}"))
            .split(';')
            .next()
            .unwrap()
            .trim()
            .to_string()
    }

    /// A round left open by a fit that never ran, whose candidate the user
    /// then re-bounds in the config: the SCM process resumes, refits only
    /// that candidate under the new bounds, and keeps everything else.
    #[test]
    fn retuning_bounds_mid_round_resumes_and_refits_only_that_candidate() {
        use crate::scm::{Compatibility, CovariateRequest, Covariates, build_plan, compatibility};

        let dir = tempfile::tempdir().unwrap();
        let options = ScmOptions {
            num_rounds: Some(1),
            ..Default::default()
        };
        let plan = make_plan(dir.path(), options.clone());
        let executor = full_scm_executor();

        // reference + round 1 (WT_CL wins), then pause
        let outcome = run_scm(&plan, &executor, false).unwrap();
        assert_eq!(outcome.status, ScmRunStatus::Paused);

        // Round 2's models are written, then nothing can be submitted: the
        // round stays open with its candidates un-run.
        let dead = full_scm_executor().failing_with("sbatch: error: submission failed");
        let unrun = ScmOptions {
            num_rounds: None,
            ..options.clone()
        };
        let mut running = plan.clone();
        running.options = unrun.clone();
        assert!(run_scm(&running, &dead, false).is_err());
        let out_dir = plan.out_dir_path();
        let wt_v_round2 = out_dir.join("forward_round2/1001_wt_v.mod");
        assert!(wt_v_round2.exists());
        let before = fs::read_to_string(&wt_v_round2).unwrap();

        // The user bounds WT_V and re-plans: still this SCM process.
        let bounded = build_plan(
            &plan.model_path(),
            &Covariates {
                effects: vec![
                    CovariateRequest::named("WT_CL"),
                    CovariateRequest::named("CRCL_CL"),
                    CovariateRequest {
                        name: "WT_V".to_string(),
                        lower: Some(0.0),
                        upper: Some(3.0),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
            Some(&out_dir),
            unrun,
            "test",
        )
        .unwrap()
        .plan;
        let state = ScmState::load(&out_dir).unwrap().unwrap();
        match compatibility(&bounded, &state) {
            Compatibility::Compatible { removals, retunes } => {
                assert!(removals.is_empty());
                assert_eq!(retunes[0].label(), "WT_V: bounds none -> (0, 3)");
            }
            other => panic!("{other:?}"),
        }

        // Resuming refits WT_V under the new bounds, in a model of its own.
        let executor = full_scm_executor().with(
            "forward_round2/1001_wt_v_refit2",
            vec![Fit::Succeeded(978.5)],
        );
        let outcome = run_scm(&bounded, &executor, false).unwrap();
        assert_eq!(outcome.status, ScmRunStatus::Completed);

        let refit = out_dir.join("forward_round2/1001_wt_v_refit2.mod");
        assert!(refit.exists(), "{:?}", executor.fits());
        // The refit is estimated under the new bounds.
        let text = fs::read_to_string(&refit).unwrap();
        assert!(theta_spec(&text, "WT_V cov").ends_with(", 3)"), "{text}");
        // The model written before them is left exactly as it was — its
        // theta unbounded, as the initial model has it — and was never fitted.
        assert_eq!(fs::read_to_string(&wt_v_round2).unwrap(), before);
        assert!(!theta_spec(&before, "WT_V cov").contains('('), "{before}");
        let fits = executor.fits();
        assert!(
            !fits.iter().any(|f| f == "forward_round2/1001_wt_v"),
            "{fits:?}"
        );
        assert_eq!(
            executor.fit_count("forward_round2/1001_wt_v_refit2"),
            1,
            "{fits:?}"
        );
        // Nothing in a concluded round was refitted, and CRCL_CL — which the
        // retune does not touch — fitted the model already written for it.
        assert!(
            !fits.iter().any(|f| f.starts_with("forward_round1/")),
            "{fits:?}"
        );
        assert!(!fits.iter().any(|f| f.starts_with("base/")), "{fits:?}");
        assert_eq!(executor.fit_count("forward_round2/1001_crcl_cl"), 1);

        // The retune is on record, dated to the round it followed.
        let entry = outcome.roster_entry("WT_V").unwrap();
        assert_eq!(entry.candidate.upper, Some(3.0));
        assert_eq!(
            entry.retunes[0].after_round.as_deref(),
            Some("forward_round1")
        );
        let round2 = outcome
            .rounds
            .iter()
            .find(|r| r.name == "forward_round2")
            .unwrap();
        let wt_v = round2
            .candidates
            .iter()
            .find(|c| c.candidate == "WT_V")
            .unwrap();
        assert_eq!(wt_v.refit, 1);
        assert_eq!(wt_v.model, "forward_round2/1001_wt_v_refit2.mod");
    }

    #[test]
    fn num_rounds_pauses_and_resume_completes_without_refitting() {
        let dir = tempfile::tempdir().unwrap();
        let options = ScmOptions {
            num_rounds: Some(1),
            ..Default::default()
        };
        let plan = make_plan(dir.path(), options);
        let executor = full_scm_executor();

        // First invocation: reference + one round, then pause
        let outcome = run_scm(&plan, &executor, false).unwrap();
        assert_eq!(outcome.status, ScmRunStatus::Paused);
        assert_eq!(outcome.completed_rounds(), 1);
        assert_eq!(outcome.retained, vec!["WT_CL".to_string()]);

        // Status shows the pause
        let status = crate::scm::read_summary(&plan.out_dir_path()).unwrap();
        assert_eq!(status.status, "paused");

        // The summary markdown is on disk after the pause, not just at the end
        let md_path = plan.out_dir_path().join(SCM_SUMMARY_MD);
        assert!(md_path.exists());
        let md = fs::read_to_string(&md_path).unwrap();
        assert!(md.contains("forward_round1"), "{md}");

        // Resume until done
        let mut last = None;
        for _ in 0..10 {
            let outcome = run_scm(&plan, &executor, false).unwrap();
            let done = outcome.status == ScmRunStatus::Completed;
            last = Some(outcome);
            if done {
                break;
            }
        }
        let outcome = last.unwrap();
        assert_eq!(outcome.status, ScmRunStatus::Completed);
        assert_eq!(outcome.retained, vec!["WT_CL".to_string()]);
        assert_eq!(outcome.completed_rounds(), 5);

        // Nothing was fitted twice: each round-1 model exactly once
        assert_eq!(executor.fit_count("forward_round1/1001_wt_cl"), 1);
        assert_eq!(executor.fit_count("forward_round1/1001_crcl_cl"), 1);
        assert_eq!(executor.fit_count("base/1001_base"), 1);
    }

    /// Removing a candidate that has lost every round so far is not a new
    /// plan: the SCM process resumes, stops testing it, keeps the rounds it
    /// took part in, and refits nothing.
    #[test]
    fn removing_a_never_selected_candidate_resumes_without_refitting() {
        use crate::scm::Covariates;
        use crate::scm::{Compatibility, build_plan, compatibility};

        let dir = tempfile::tempdir().unwrap();
        let options = ScmOptions {
            num_rounds: Some(1),
            ..Default::default()
        };
        let plan = make_plan(dir.path(), options.clone());
        let executor = full_scm_executor();

        // reference + round 1 (WT_CL wins, WT_V loses), then pause
        let outcome = run_scm(&plan, &executor, false).unwrap();
        assert_eq!(outcome.status, ScmRunStatus::Paused);
        let wt_v_round1 = plan.out_dir_path().join("forward_round1/1001_wt_v.mod");
        assert!(wt_v_round1.exists());

        // re-plan without WT_V; the paused state is still this plan's
        let fewer = build_plan(
            &plan.model_path(),
            &Covariates::named(&["WT_CL", "CRCL_CL"]),
            None,
            ScmOptions {
                num_rounds: None,
                ..options
            },
            "test",
        )
        .unwrap();
        assert_eq!(
            compatibility(&fewer.plan, &outcome),
            Compatibility::Compatible {
                removals: vec!["WT_V".to_string()],
                retunes: vec![]
            }
        );
        let outcome = run_scm(&fewer.plan, &executor, false).unwrap();
        assert_eq!(outcome.status, ScmRunStatus::Completed);
        let state = &outcome;

        // the removal is on record, dated to the round it followed
        let entry = state.roster_entry("WT_V").unwrap();
        assert_eq!(
            entry.removed.as_ref().unwrap().after_round.as_deref(),
            Some("forward_round1")
        );
        // round 1 kept WT_V's record and files; round 2 never tested it
        assert!(wt_v_round1.exists());
        assert!(
            state.rounds[1]
                .candidates
                .iter()
                .any(|c| c.candidate == "WT_V")
        );
        assert!(
            !state.rounds[2]
                .candidates
                .iter()
                .any(|c| c.candidate == "WT_V")
        );
        assert_eq!(executor.fit_count("forward_round1/1001_wt_v"), 1);
        assert_eq!(executor.fit_count("forward_round2/1001_wt_v"), 0);
        assert_eq!(executor.fit_count("forward_round1/1001_wt_cl"), 1);
        // the same decisions fall out: WT_CL and CRCL_CL added, CRCL_CL
        // dropped again in backward elimination
        assert_eq!(state.retained, vec!["WT_CL".to_string()]);

        // removing the winner, on the other hand, is a different SCM process
        let no_winner = build_plan(
            &plan.model_path(),
            &Covariates::named(&["CRCL_CL"]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap();
        let err = run_scm(&no_winner.plan, &executor, false).unwrap_err();
        assert!(
            err.to_string()
                .contains("WT_CL was selected in forward_round1"),
            "got: {err}"
        );
    }

    /// A candidate removed while its round is open is withdrawn from that
    /// round: whatever it fitted is recorded but never scored, and the round
    /// decides itself among the rest on resume.
    #[test]
    fn removing_a_candidate_from_an_open_round_withdraws_it() {
        use crate::scm::Covariates;
        use crate::scm::snapshot_tests::fabricate_running_scm;
        use crate::scm::{ScmPlan, build_plan};

        let dir = tempfile::tempdir().unwrap();
        // Round 1 open: WT_CL scored, CRCL_CL still running, WT_V pending.
        let out_dir = fabricate_running_scm(dir.path());
        let plan = ScmPlan::load(out_dir.join(crate::scm::PLAN_FILENAME)).unwrap();

        // drop CRCL_CL: WT_CL wins the round on resume, nothing is refit
        let fewer = build_plan(
            &plan.model_path(),
            &Covariates::named(&["WT_CL", "WT_V"]),
            None,
            plan.options.clone(),
            "test",
        )
        .unwrap();
        let executor = full_scm_executor();
        let outcome = run_scm(&fewer.plan, &executor, false).unwrap();
        assert_eq!(outcome.status, ScmRunStatus::Completed);
        let r1 = &outcome.rounds[1];
        let crcl = r1
            .candidates
            .iter()
            .find(|c| c.candidate == "CRCL_CL")
            .unwrap();
        assert_eq!(crcl.status, CandidateStatus::Withdrawn);
        assert_eq!(crcl.significant, None);
        assert!(!crcl.selected);
        assert_eq!(r1.winner.as_deref(), Some("WT_CL"));
        assert!(r1.decision.starts_with("added WT_CL"), "{}", r1.decision);
        assert_eq!(executor.fit_count("forward_round1/1001_wt_cl"), 0);
        assert_eq!(executor.fit_count("forward_round1/1001_crcl_cl"), 0);
        assert_eq!(executor.fit_count("forward_round2/1001_crcl_cl"), 0);
        assert_eq!(outcome.retained, vec!["WT_CL".to_string()]);
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
            .with("full/1001_full", vec![Fit::Succeeded(900.0)])
            // dropping WT_CL is free; the others are needed
            .with("backward_round1/1001_wt_cl", vec![Fit::Succeeded(900.5)])
            .with("backward_round1/1001_crcl_cl", vec![Fit::Succeeded(950.0)])
            .with("backward_round1/1001_wt_v", vec![Fit::Succeeded(930.0)])
            .with("backward_round2/1001_crcl_cl", vec![Fit::Succeeded(951.0)])
            .with("backward_round2/1001_wt_v", vec![Fit::Succeeded(931.0)]);

        let outcome = run_scm(&plan, &executor, false).unwrap();
        assert_eq!(outcome.status, ScmRunStatus::Completed);
        let state = &outcome;

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

    /// A `PROGRAM TERMINATED BY OBJ` abort still writes a plausible OFV, so
    /// nothing downstream can tell it apart from a real fit by the numbers
    /// alone. It must never be scored: the candidate fails, burns its
    /// retries, and concludes unusable with the abort named — which is how a
    /// scientist learns the covariate needs a lower bound rather than
    /// silently getting a bogus ΔOFV win.
    #[test]
    fn an_aborted_estimation_is_never_scored_and_burns_its_retries() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(
            dir.path(),
            ScmOptions {
                direction: vec![Direction::Forward],
                max_retries: 3,
                ..Default::default()
            },
        );

        // WT_CL aborts every time, and its OFV would have won the round
        // outright had it been believed. The first attempt leaves a readable
        // .ext; every retry dies at the initial OBJ evaluation the way a
        // warm start from a diverged point does, leaving a headerless one.
        let executor = MockExecutor::new(1234.0)
            .with("base/1001_base", vec![Fit::Succeeded(1000.0)])
            .with(
                "forward_round1/1001_wt_cl",
                vec![
                    Fit::Aborted(700.0),
                    Fit::AbortedHeaderless,
                    Fit::AbortedHeaderless,
                    Fit::AbortedHeaderless,
                ],
            )
            .with("forward_round1/1001_crcl_cl", vec![Fit::Succeeded(990.0)])
            .with("forward_round1/1001_wt_v", vec![Fit::Succeeded(999.0)])
            .with("forward_round2/1001_wt_v", vec![Fit::Succeeded(989.5)]);

        // A headerless .ext must not take the SCM process down with it.
        let outcome = run_scm(&plan, &executor, false).unwrap();
        assert_eq!(outcome.status, ScmRunStatus::Completed);
        let state = &outcome;

        let r1 = &state.rounds[1];
        let wt_cl = r1
            .candidates
            .iter()
            .find(|c| c.candidate == "WT_CL")
            .unwrap();

        // Failed, not succeeded — despite a readable OFV on attempt 1.
        assert_eq!(wt_cl.status, CandidateStatus::Unusable);
        assert_eq!(wt_cl.ofv, None);
        assert_eq!(wt_cl.p_value, None);
        assert_eq!(wt_cl.significant, None);

        // One attempt plus max_retries, every one of them retried.
        assert_eq!(wt_cl.n_attempts(), 4);
        assert_eq!(executor.fit_count("forward_round1/1001_wt_cl"), 4);

        // The reason reaches the record, on the attempt and as a heuristic.
        assert!(
            wt_cl
                .attempts
                .iter()
                .all(|a| a.outcome == "program aborted"),
            "{:?}",
            wt_cl.attempts
        );
        assert!(
            wt_cl.heuristics.contains(&"program aborted".to_string()),
            "{:?}",
            wt_cl.heuristics
        );

        // CRCL_CL wins on its own merits; the abort never competed.
        assert_eq!(r1.winner.as_deref(), Some("CRCL_CL"));
        assert!(state.had_unusable);
        assert!(state.message.as_ref().unwrap().contains("unusable"));

        // And it is legible in every record a scientist actually reads.
        let md = fs::read_to_string(plan.out_dir_path().join(SCM_SUMMARY_MD)).unwrap();
        let row = md.lines().find(|l| l.starts_with("| WT_CL ")).unwrap();
        assert!(row.contains("unusable"), "{row}");
        assert!(row.contains("program aborted"), "{row}");

        // `scm summary --round 1` — the rendered text, as printed.
        let summary = crate::scm::read_summary(&plan.out_dir_path()).unwrap();
        let text = summary
            .render_text(&crate::scm::SummaryOptions {
                round: Some("forward_round1".into()),
                ..Default::default()
            })
            .unwrap();
        let wt_cl_block: Vec<&str> = text
            .lines()
            .skip_while(|l| !l.starts_with("  WT_CL"))
            .take(6)
            .collect();
        assert!(
            wt_cl_block.iter().any(|l| l.contains("unusable")),
            "no unusable status in:\n{}",
            wt_cl_block.join("\n")
        );
        assert!(
            wt_cl_block
                .iter()
                .any(|l| l.contains("heuristics: program aborted")),
            "no heuristic line in:\n{}",
            wt_cl_block.join("\n")
        );

        // `round_summary.md` — the per-round record the round detail points at.
        let md = fs::read_to_string(plan.out_dir_path().join("forward_round1/round_summary.md"))
            .unwrap();
        let md_row = md
            .lines()
            .find(|l| l.starts_with("| WT_CL "))
            .unwrap_or_else(|| panic!("no WT_CL row in:\n{md}"));
        assert!(md_row.contains("unusable"), "{md_row}");
        assert!(md_row.contains("program aborted"), "{md_row}");
    }

    /// A reference fit that exhausted its retries leaves the process with no
    /// reference model and a concluded reference round. Resuming it has to
    /// fit again — the reason it failed may well have been fixed since.
    #[test]
    fn a_reference_fit_given_up_on_is_refitted_on_resume() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(
            dir.path(),
            ScmOptions {
                max_retries: 1,
                ..Default::default()
            },
        );

        let failing = MockExecutor::new(1234.0)
            .with("base/1001_base", vec![Fit::NoFinalRow, Fit::NoFinalRow]);
        let err = run_scm(&plan, &failing, false).unwrap_err();
        assert!(
            format!("{err:#}").contains("the SCM process cannot start"),
            "got: {err:#}"
        );
        assert_eq!(failing.fit_count("base/1001_base"), 2);

        // Same plan, same out_dir, a reference fit that now works.
        let executor = full_scm_executor();
        let outcome = run_scm(&plan, &executor, false).unwrap();
        assert_eq!(outcome.status, ScmRunStatus::Completed);

        // It started the reference over from attempt 1 rather than picking up
        // the abandoned attempts, and recorded the round once.
        assert_eq!(executor.fit_count("base/1001_base"), 1);
        let reference: Vec<_> = outcome.rounds.iter().filter(|r| r.is_reference()).collect();
        assert_eq!(reference.len(), 1);
        assert_eq!(reference[0].candidates[0].ofv, Some(1000.0));
        assert_eq!(reference[0].candidates[0].n_attempts(), 1);
        let base_dir = plan.out_dir_path().join("base");
        assert!(!base_dir.join("1001_base_try2.mod").exists());
    }

    /// `scm run --overwrite`: the same plan and out_dir, run from scratch.
    #[test]
    fn run_overwrite_discards_the_previous_output() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        let first = full_scm_executor();
        run_scm(&plan, &first, false).unwrap();

        // A plain resume of a finished process fits nothing: it reads what is
        // already on disk.
        let resumed = full_scm_executor();
        assert_eq!(
            run_scm(&plan, &resumed, false).unwrap().status,
            ScmRunStatus::Completed
        );
        assert!(resumed.fits().is_empty());

        // With overwrite, every fit runs again.
        let again = full_scm_executor();
        let outcome = run_scm(&plan, &again, true).unwrap();
        assert_eq!(outcome.status, ScmRunStatus::Completed);
        assert_eq!(again.fits().len(), first.fits().len());
        assert_eq!(again.fit_count("base/1001_base"), 1);
    }
}
