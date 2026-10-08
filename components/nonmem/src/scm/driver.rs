use std::cell::Cell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use config::NonmemConfig;
use fs_err as fs;

use crate::run::metadata::{RUN_START_FILENAME, RunStartFile};

use super::live;
use super::report::{Mark, report, report_dated, report_fit, report_start, report_table};
use super::roster::{apply_removals, apply_retunes, compatibility};
use super::round::{
    FitOutcome, ModelWriter, RoundEntry, ext_path_for, read_fit_outcome, record_attempt,
    round_entries, run_dir_for, scm_model_name,
};
use super::state::{
    AttemptRecord, CandidateRecord, CandidateStatus, CheckpointFit, CheckpointStatus, RoundRecord,
    ScmRunStatus, ScmState,
};
use super::summary::{round_table, write_records};
use super::{
    Direction, FORWARD_FINAL_DIR, NO_REFERENCE, REFERENCE_ROUND, ScmPlan, clear_previous_output,
    max_models_for, none_or_list, ofv_suffix, on_off, rel_to,
};

/// Fits a batch of models to completion.
pub trait FitExecutor {
    /// Fit `models`, calling `done` with each one as soon as its run ends
    /// (however it ended), so it is reported while the others still run.
    fn fit(&self, models: &[PathBuf], done: &dyn Fn(&Path)) -> Result<()>;
    /// Start fitting `models` without waiting for them. A later [`fit`](Self::fit)
    /// of the same models picks up what this started — waits for a fit still
    /// running, uses one that finished — rather than starting it again. The
    /// default starts nothing early, leaving that later `fit` all the work.
    fn submit(&self, _models: &[PathBuf]) -> Result<()> {
        Ok(())
    }
    fn describe(&self) -> String;
    /// The project settings the fits run under (where output lands, which
    /// comment dialect names parameters)
    fn settings(&self) -> Result<NonmemConfig> {
        Ok(NonmemConfig::default())
    }
}

/// Metadata files require a pharos project root that contains the output directory
fn metadata_enabled(out_dir: &Path) -> bool {
    fs::canonicalize(out_dir).is_ok_and(|dir| config::to_config_relative(dir).is_ok())
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

    let settings = executor.settings()?;
    // Whether the state on disk is this plan's to resume.
    let mut state = match ScmState::load(&out_dir)? {
        Some(mut s) => {
            let previous = ScmPlan::load(plan.plan_path()).ok();
            let verdict = compatibility(plan, &s, previous.as_ref());
            if verdict.is_incompatible() {
                bail!(
                    "{} contains SCM state from a different plan:\n  {}\nrun with --overwrite to discard it, or use a fresh out_dir",
                    out_dir.display(),
                    verdict.reasons.join("\n  ")
                );
            }
            if s.status == ScmRunStatus::Completed {
                report(format!(
                    "the SCM process in {} is already complete; re-plan with --overwrite to start it over",
                    out_dir.display()
                ));
                return Ok(s);
            }
            report(format!("resuming the SCM process in {}", out_dir.display()));
            for line in apply_removals(&mut s, &verdict.removals) {
                report(line);
            }
            for line in apply_retunes(&mut s, &verdict.retunes, &out_dir, &settings) {
                report(format!("retuned {line}"));
            }
            s
        }
        None => {
            report(format!("starting the SCM process in {}", out_dir.display()));
            ScmState::new(plan)
        }
    };

    // Keep the plan on disk next to the state for the record.
    plan.save()?;

    state.status = ScmRunStatus::Running;
    state.message = None;
    state.save(&out_dir)?;

    live::process_begin(
        max_models_for(plan.candidates.len(), plan.options.phases().len()),
        state.fits_so_far(),
    );
    let outcome = drive(plan, executor, &mut state, &out_dir, &settings);
    live::finish();
    // On a terminal the closing line says how long this driver took.
    let took = live::process_elapsed()
        .map(|d| format!(" in {}", utils::format_duration(Some(d.as_secs_f64()))))
        .unwrap_or_default();
    match outcome {
        Ok(status) => {
            state.status = status;
            state.save(&out_dir)?;
            report_dated(match &state.message {
                Some(note) => format!("SCM process {status}{took}: {note}"),
                None => format!("SCM process {status}{took}"),
            });
            if status == ScmRunStatus::Completed
                && let Some(round) = state.rounds.last()
            {
                write_records(&out_dir, plan, &state, round, &settings)?;
            }
            Ok(state)
        }
        // Stopped on request: nothing is wrong with the process, and it resumes.
        Err(e) if super::interrupt::is_interrupted(&e) => {
            state.status = ScmRunStatus::Paused;
            state.message = Some(super::interrupt::INTERRUPTED_NOTE.to_string());
            state.save(&out_dir)?;
            report_dated(format!(
                "SCM process paused{took}: {}",
                super::interrupt::INTERRUPTED_NOTE
            ));
            Ok(state)
        }
        Err(e) => {
            state.status = ScmRunStatus::Failed;
            state.message = Some(format!("{e:#}"));
            state.save(&out_dir)?;
            report_dated(format!("SCM process FAILED{took}: {e:#}"));
            // Best-effort record of the failing round in its own directory.
            if let Some(round) = state.rounds.last() {
                let _ = write_records(&out_dir, plan, &state, round, &settings);
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
    super::write_gitignore(out_dir, &stem, settings.scm.track_in_git())?;
    let with_metadata = metadata_enabled(out_dir);
    report(format!(
        "SCM process on {} via {} executor (metadata: {})",
        template.display(),
        executor.describe(),
        with_metadata
    ));

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
        let gave_up = state.round(REFERENCE_ROUND).is_some_and(|r| {
            r.candidates
                .iter()
                .any(|c| c.status == CandidateStatus::Unusable)
        });
        if gave_up {
            report(format!(
                "the previous {ref_name} fit was given up on; starting it over"
            ));
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

        let idx = run_round_fits(
            &ctx,
            executor,
            state,
            REFERENCE_ROUND,
            first,
            NO_REFERENCE,
            entries,
        )?;
        let round = &mut state.rounds[idx];
        let cand = round.candidates[0].clone();
        round.complete = true;
        if cand.status != CandidateStatus::Succeeded {
            let failed = format!(
                "{ref_name} model failed after {} attempt(s)",
                cand.attempts.len()
            );
            round.decision = failed.clone();
            state.save(out_dir)?;
            bail!("reference {failed}; the SCM process cannot start");
        }
        round.decision = format!("{ref_name} model fitted{}", ofv_suffix(cand.ofv));
        state.reference_model = Some(cand.model);
        state.reference_ofv = cand.ofv;
        state.retained = released_names;
        state.phase = Some(first);
        state.save(out_dir)?;
        write_records(out_dir, plan, state, &state.rounds[idx], settings)?;
        report(format!(
            "{REFERENCE_ROUND} complete: {}",
            state.rounds[idx].decision
        ));
    }

    let mut rounds_this_invocation = 0usize;

    // ---- SCM rounds ----
    while let Some(phase) = state.phase {
        if !phases.contains(&phase) {
            bail!("state phase {phase} is not part of this plan's direction");
        }

        // The forward model's own covariance-step fit starts the moment the
        // phase turns, and is looked in on between rounds, never waited for.
        if phase == Direction::Backward {
            start_forward_final(&ctx, executor, state)?;
        }
        poll_forward_final(&ctx, executor, state)?;

        let entries = round_entries(plan, &state.retained, phase);

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
                "paused after {rounds_this_invocation} round(s) (num_rounds = {cap}); submit the plan again to continue"
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

        let idx = run_round_fits(
            &ctx,
            executor,
            state,
            &round_name,
            phase,
            &reference_model,
            entries,
        )?;

        // ---- Score the round: the best contender wins ----
        let round = &mut state.rounds[idx];
        round.reference_ofv = Some(reference_ofv);
        let scored = round.score(&plan.options);
        let alpha = round
            .alpha(&plan.options)
            .expect("an SCM round has an alpha");
        // Every candidate is concluded once the fits are done, so anything
        // not succeeded or withdrawn ran out of retries.
        let n_unusable = round.unusable();
        let contenders = round.contenders(&plan.options);
        if let [best, next, ..] = contenders.as_slice()
            && best.key() == next.key()
        {
            log::info!(
                "{round_name}: {} and {} score identically (p = {:.3e}, dOFV = {:+.3}); {} wins as the earlier $THETA",
                round.candidates[best.index].candidate,
                round.candidates[next.index].candidate,
                best.p_value,
                best.delta_ofv,
                round.candidates[best.index].candidate
            );
        }
        round.complete = true;

        // ---- Decide ----
        match contenders.first().map(|s| s.index) {
            Some(w) => {
                let cand = &mut round.candidates[w];
                cand.selected = true;
                let name = cand.candidate.clone();
                let model = cand.model.clone();
                let ofv = cand.ofv;
                let p = cand.p_value.unwrap_or(f64::NAN);
                let delta = cand.delta_ofv.unwrap_or(f64::NAN);
                let verb = match phase {
                    Direction::Forward => "added",
                    Direction::Backward => "dropped",
                };
                round.decision = super::pick_label(verb, &name, p, delta);
                round.winner = Some(name.clone());
                match phase {
                    Direction::Forward => state.retained.push(name),
                    Direction::Backward => state.retained.retain(|n| *n != name),
                }
                state.reference_model = Some(model);
                state.reference_ofv = ofv;
            }
            None => {
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
                round.decision = decision;
                advance_phase(state, &phases);
            }
        }
        if n_unusable > 0 {
            state.had_unusable = true;
        }
        rounds_this_invocation += 1;
        state.save(out_dir)?;
        let round = write_records(out_dir, plan, state, &state.rounds[idx], settings)?;
        report(format!(
            "{round_name} complete: {}; retained: {}",
            round.decision,
            none_or_list(&state.retained)
        ));
        report_table(&round_table(&round, "             "));

        if state.phase.is_none() {
            break;
        }
    }

    if state.had_unusable {
        state.message = Some(
            "SCM process completed, but some candidates were unusable (see scm_summary.md); they were reported, never scored as insignificant"
                .to_string(),
        );
    }
    collect_forward_final(&ctx, executor, state)?;
    if let Some(fit) = &state.forward_final
        && fit.status == CheckpointStatus::Unusable
    {
        completion_note(
            state,
            format!(
                "the forward model re-fit did not minimize in {} attempt(s), so it has no OFV or covariance step results",
                fit.attempts.len()
            ),
        );
    }
    write_final_model(&ctx, executor, state)?;
    state.save(out_dir)?;
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

/// Fit every entry of a round to a conclusion, returning the index of the
/// round's record in `state.rounds`.
#[allow(clippy::too_many_arguments)]
fn run_round_fits(
    ctx: &DriveContext<'_>,
    executor: &dyn FitExecutor,
    state: &mut ScmState,
    round_name: &str,
    direction: Direction,
    reference_model: &str,
    entries: Vec<RoundEntry>,
) -> Result<usize> {
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
    let round_dir = ctx.out_dir.join(
        state.rounds[round_idx]
            .dir_name()
            .context("reference round has no entry to name its directory")?,
    );
    fs::create_dir_all(&round_dir)?;

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
    for wave in 0..max_attempts {
        let mut to_fit: Vec<PathBuf> = Vec::new();
        let mut fitted_candidates: Vec<usize> = Vec::new();
        // What each dispatched model is, for reporting it as it ends
        let mut dispatched: HashMap<PathBuf, (String, usize)> = HashMap::new();

        for (entry, &idx) in entries.iter().zip(&record_index) {
            let cand = &state.rounds[round_idx].candidates[idx];
            if cand.status.is_concluded() {
                continue;
            }

            let attempt = cand.attempts.len() + 1;
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

            // Resume: a usable outcome may already be on disk. A run that was
            // only terminated — a termination record but no end record — died
            // with its driver or its node (a stop, a crash, a cancelled
            // allocation), not on its own, so it is not an attempt that
            // failed: the same attempt is fitted again, overwriting what the
            // run left. A fit cancelled while the driver runs is recorded as
            // it ends (below), so it never reaches this branch.
            let outcome = read_fit_outcome(&model_path, ctx.settings, true)?;
            let cand = &mut state.rounds[round_idx].candidates[idx];

            if outcome.finished {
                record_attempt(cand, rel_to(&model_path, ctx.out_dir), &outcome);
            } else {
                cand.status = CandidateStatus::Running;
                cand.model = rel_to(&model_path, ctx.out_dir);
                dispatched.insert(model_path.clone(), (entry.candidate.clone(), attempt));
                to_fit.push(model_path);
                fitted_candidates.push(idx);
            }
        }

        state.save(ctx.out_dir)?;

        if !to_fit.is_empty() {
            let names: Vec<&str> = to_fit
                .iter()
                .filter_map(|m| dispatched.get(m).map(|(c, _)| c.as_str()))
                .collect();
            report_start(format!(
                "{round_name}: {} {}: {}",
                if wave == 0 { "fitting" } else { "retrying" },
                super::plural(to_fit.len(), "model"),
                names.join(", ")
            ));
            let label = super::round_label(round_name);
            live::round_begin(
                round_name,
                &if wave == 0 {
                    label
                } else {
                    format!("{label} retry")
                },
                to_fit
                    .iter()
                    .map(|model| live::FitEntry {
                        model: model.clone(),
                        name: dispatched
                            .get(model)
                            .map(|(c, _)| c.clone())
                            .unwrap_or_default(),
                        run_dir: run_dir_for(model, ctx.settings).ok(),
                        ext: ext_path_for(model, ctx.settings).ok(),
                    })
                    .collect(),
            );
            let finished = Cell::new(0usize);
            let done = |model: &Path| {
                finished.set(finished.get() + 1);
                let took = live::fit_done(model);
                let Some((candidate, attempt)) = dispatched.get(model) else {
                    return;
                };
                let outcome = read_fit_outcome(model, ctx.settings, true).unwrap_or_default();
                let (mark, text) = describe_fit(candidate, *attempt, max_attempts, &outcome);
                report_fit(
                    round_name,
                    mark,
                    format!("{text} ({} of {} done)", finished.get(), to_fit.len()),
                    took,
                );
            };
            let fitted = executor.fit(&to_fit, &done);
            live::round_end();
            if let Err(e) = fitted {
                // Nothing was fitted: the candidates go back to pending
                for idx in &fitted_candidates {
                    state.rounds[round_idx].candidates[*idx].status = CandidateStatus::Pending;
                }
                state.save(ctx.out_dir)?;
                return Err(e);
            }

            for (list_pos, idx) in fitted_candidates.iter().enumerate() {
                let outcome = read_fit_outcome(&to_fit[list_pos], ctx.settings, true)?;
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

    Ok(round_idx)
}

/// One ended fit, as reported: `WT_CL fitted on attempt 1, OFV 990.123`, or
/// what went wrong and whether it is retried.
fn describe_fit(
    candidate: &str,
    attempt: usize,
    max_attempts: usize,
    outcome: &FitOutcome,
) -> (Mark, String) {
    let (mark, mut line) = if let Some(ofv) = outcome.ofv.filter(|_| outcome.usable()) {
        (
            Mark::Ok,
            format!("{candidate} fitted on attempt {attempt}, OFV {ofv:.3}"),
        )
    } else if attempt < max_attempts {
        (
            Mark::Retry,
            format!(
                "{candidate} attempt {attempt} FAILED ({}); retrying as attempt {} of {max_attempts}",
                outcome.label(),
                attempt + 1
            ),
        )
    } else {
        (
            Mark::Failed,
            format!(
                "{candidate} attempt {attempt} FAILED ({}); out of retries, unusable",
                outcome.label()
            ),
        )
    };
    let warnings: Vec<&str> = outcome
        .heuristics
        .iter()
        .map(String::as_str)
        .filter(|h| *h != outcome.label())
        .collect();
    if !warnings.is_empty() {
        line.push_str(&format!(" [{}]", warnings.join(", ")));
    }
    (mark, line)
}

/// Build the final model
fn write_final_model(
    ctx: &DriveContext<'_>,
    executor: &dyn FitExecutor,
    state: &mut ScmState,
) -> Result<()> {
    // A fit of the final model with the covariance step on may already be
    // in hand: the last reference fit, when every model ran the step (it
    // releases exactly the retained effects); or the forward model's
    // re-fit, when backward elimination dropped nothing. That fit is copied
    // into `final/`, model and run directory, rather than run again.
    let in_hand = if ctx.plan.options.cov_step {
        let model = state
            .reference_model
            .clone()
            .context("internal error: no reference model")?;
        let outcome = read_fit_outcome(&ctx.out_dir.join(&model), ctx.settings, true)?;
        let why = "every model ran the cov step, so the last reference fit";
        Some((model, state.reference_ofv, outcome.heuristics, why))
    } else if ctx.plan.options.final_cov_step
        && let Some(fit) = state
            .forward_final
            .as_ref()
            .filter(|f| f.usable_for(&state.retained))
    {
        let why = "backward elimination dropped nothing, so the forward model's fit";
        Some((fit.model.clone(), fit.ofv, fit.heuristics.clone(), why))
    } else {
        None
    };
    if let Some((source, ofv, heuristics, why)) = in_hand {
        let model = copy_fit_into_final(ctx, &source)?;
        state.final_model = Some(model.clone());
        state.final_ofv = ofv;
        state.final_heuristics = heuristics;
        report(format!(
            "final: {why} is the final model; copied {source} and its run to {model}{}",
            ofv_suffix(ofv)
        ));
        return Ok(());
    }

    let final_dir = ctx.out_dir.join("final");
    let final_path = final_dir.join(format!("{}_scm_final.mod", ctx.stem));

    let released = ctx.plan.thetas_for(&state.retained);
    let cov_step = ctx.plan.options.final_cov_step;
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
    state.final_heuristics.clear();
    if !ctx.plan.options.final_cov_step {
        report(format!(
            "final model written to {}",
            rel_to(&final_path, ctx.out_dir)
        ));
        return Ok(());
    }

    // Resumable and retried like every other fit: an attempt already finished on
    // disk is read rather than run again, a failed one is retried from its estimates.
    let max_attempts = ctx.plan.options.max_retries + 1;
    let mut path = final_path;
    for attempt in 1..=max_attempts {
        if attempt > 1 {
            let next = final_dir.join(format!(
                "{}.mod",
                scm_model_name(ctx.stem, "scm_final", attempt, 0)
            ));
            if !next.exists() {
                let description = format!("{description} (attempt {attempt})");
                ctx.writer.retry(
                    &path,
                    &next,
                    &description,
                    based_on.as_deref(),
                    ctx.settings,
                )?;
            }
            path = next;
        }
        // As in a round: a run only terminated (no end record) died with its
        // driver, so this attempt is fitted again rather than charged.
        let mut outcome = read_fit_outcome(&path, ctx.settings, true)?;
        let took = Cell::new(None);
        if !outcome.finished {
            report_start(format!("final: fitting {}", rel_to(&path, ctx.out_dir)));
            live::round_begin(
                "final",
                "final",
                vec![live::FitEntry {
                    model: path.clone(),
                    name: "final".to_string(),
                    run_dir: run_dir_for(&path, ctx.settings).ok(),
                    ext: ext_path_for(&path, ctx.settings).ok(),
                }],
            );
            let fitted = executor.fit(std::slice::from_ref(&path), &|model| {
                took.set(live::fit_done(model));
            });
            live::round_end();
            fitted?;
            outcome = read_fit_outcome(&path, ctx.settings, true)?;
        }
        let (mark, text) = describe_fit("final model", attempt, max_attempts, &outcome);
        report_fit("final", mark, text, took.get());
        state.final_model = Some(rel_to(&path, ctx.out_dir));
        state.final_heuristics = outcome.heuristics.clone();
        if outcome.usable() {
            state.final_ofv = outcome.ofv;
            return Ok(());
        }
    }
    completion_note(
        state,
        format!(
            "the final model re-fit did not minimize in {max_attempts} attempt(s), so it has no OFV or covariance step results"
        ),
    );
    Ok(())
}

/// Copy a fitted model and its run directory into `final/`, names kept, so
/// `final/` holds the final model whichever fit it came from. The copy's
/// start record names the copy, so `pharos nonmem summary` on it stands on
/// its own. An earlier copy (a resume) is overwritten. Returns the copied
/// model's path relative to out_dir.
fn copy_fit_into_final(ctx: &DriveContext<'_>, model: &str) -> Result<String> {
    let src = ctx.out_dir.join(model);
    let final_dir = ctx.out_dir.join("final");
    fs::create_dir_all(&final_dir)?;
    let dest = final_dir.join(
        src.file_name()
            .with_context(|| format!("{model} has no file name"))?,
    );
    fs::copy(&src, &dest)?;
    let src_run = run_dir_for(&src, ctx.settings)?;
    let dest_run = run_dir_for(&dest, ctx.settings)?;
    copy_dir_all(&src_run, &dest_run)?;

    let start_path = dest_run.join(RUN_START_FILENAME);
    if let Ok(mut start) = RunStartFile::load(&start_path) {
        let from = rel_to(&src, ctx.out_dir);
        let to = rel_to(&dest, ctx.out_dir);
        if let Some(prefix) = start.model_path.strip_suffix(&from) {
            start.model_path = format!("{prefix}{to}");
            start.save(&dest_run)?;
        }
    }
    Ok(rel_to(&dest, ctx.out_dir))
}

/// Copy `src` into `dest` recursively, overwriting files already there.
fn copy_dir_all(src: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Add `note` to what the completed process says of itself.
fn completion_note(state: &mut ScmState, note: String) {
    state.message = Some(match state.message.take() {
        Some(m) => format!("{m}; {note}"),
        None => format!("SCM process completed, but {note}"),
    });
}

// ---------------------------------------------------------------------------
// The forward model's covariance-step fit
// ---------------------------------------------------------------------------

/// The model name the forward model's re-fit is written under (`<stem>_` first)
const FORWARD_FINAL_NAME: &str = "scm_forward_final";

fn forward_final_description(retained: &[String]) -> String {
    format!(
        "SCM forward model: retained {}; cov step on",
        none_or_list(retained)
    )
}

fn forward_final_path(ctx: &DriveContext<'_>, attempt: usize) -> PathBuf {
    ctx.out_dir.join(FORWARD_FINAL_DIR).join(format!(
        "{}.mod",
        scm_model_name(ctx.stem, FORWARD_FINAL_NAME, attempt, 0)
    ))
}

/// Start the forward model's fit with the covariance step on, as a fit the
/// executor does not wait for, so backward elimination runs beside it. Done
/// once, the moment the phase turns; nothing when the plan does not ask for
/// it. With `cov_step` on, the forward model has already run the step, and
/// is recorded as it is.
fn start_forward_final(
    ctx: &DriveContext<'_>,
    executor: &dyn FitExecutor,
    state: &mut ScmState,
) -> Result<()> {
    if state.forward_final.is_some() || !ctx.plan.options.fits_forward_final() {
        return Ok(());
    }
    let source = state
        .reference_model
        .clone()
        .context("internal error: no forward model to re-fit")?;
    let retained = state.retained.clone();

    if ctx.plan.options.cov_step {
        let outcome = read_fit_outcome(&ctx.out_dir.join(&source), ctx.settings, true)?;
        state.forward_final = Some(CheckpointFit {
            model: source.clone(),
            source: source.clone(),
            retained,
            attempts: vec![],
            status: CheckpointStatus::Reused,
            ofv: state.reference_ofv,
            heuristics: outcome.heuristics,
        });
        state.save(ctx.out_dir)?;
        report(format!(
            "{FORWARD_FINAL_DIR}: the forward model {source} already ran the cov step; using it as it is"
        ));
        return Ok(());
    }

    let path = forward_final_path(ctx, 1);
    fs::create_dir_all(path.parent().expect("the forward_final dir"))?;
    if !path.exists() {
        let reference_ext = ext_path_for(&ctx.out_dir.join(&source), ctx.settings)?;
        ctx.writer.write(
            &path,
            &ctx.plan.thetas_for(&retained),
            Some(&reference_ext),
            true,
            &forward_final_description(&retained),
            Some(&format!("../{source}")),
        )?;
    }
    let model = rel_to(&path, ctx.out_dir);
    state.forward_final = Some(CheckpointFit {
        source,
        retained,
        model: model.clone(),
        attempts: vec![],
        status: CheckpointStatus::Running,
        ofv: None,
        heuristics: vec![],
    });
    state.save(ctx.out_dir)?;
    report_start(format!(
        "{FORWARD_FINAL_DIR}: fitting {model} with the cov step on"
    ));
    executor.submit(std::slice::from_ref(&path))
}

/// Look in on the forward model's fit between rounds: one that has ended is
/// concluded (or retried); one still running is left to run.
fn poll_forward_final(
    ctx: &DriveContext<'_>,
    executor: &dyn FitExecutor,
    state: &mut ScmState,
) -> Result<()> {
    let Some(fit) = state
        .forward_final
        .as_ref()
        .filter(|f| !f.status.is_concluded())
    else {
        return Ok(());
    };
    let outcome = read_fit_outcome(&ctx.out_dir.join(&fit.model), ctx.settings, true)?;
    if !outcome.finished {
        return Ok(());
    }
    conclude_forward_final(ctx, executor, state, &outcome, None)
}

/// Wait for the forward model's fit, once the rounds are over: the one time
/// it is waited for. A retry it needs now is fitted here and now.
fn collect_forward_final(
    ctx: &DriveContext<'_>,
    executor: &dyn FitExecutor,
    state: &mut ScmState,
) -> Result<()> {
    loop {
        let Some(fit) = state
            .forward_final
            .as_ref()
            .filter(|f| !f.status.is_concluded())
        else {
            return Ok(());
        };
        let path = ctx.out_dir.join(&fit.model);
        let mut outcome = read_fit_outcome(&path, ctx.settings, true)?;
        let took = Cell::new(None);
        if !outcome.finished {
            report_start(format!("{FORWARD_FINAL_DIR}: waiting for {}", fit.model));
            live::round_begin(
                FORWARD_FINAL_DIR,
                "forward final",
                vec![live::FitEntry {
                    model: path.clone(),
                    name: "forward model".to_string(),
                    run_dir: run_dir_for(&path, ctx.settings).ok(),
                    ext: ext_path_for(&path, ctx.settings).ok(),
                }],
            );
            let fitted = executor.fit(std::slice::from_ref(&path), &|model| {
                took.set(live::fit_done(model));
            });
            live::round_end();
            fitted?;
            outcome = read_fit_outcome(&path, ctx.settings, true)?;
        }
        conclude_forward_final(ctx, executor, state, &outcome, took.get())?;
    }
}

/// Record how the forward model's current attempt ended and move on: a
/// usable fit concludes it; a failed one starts the next attempt from its
/// estimates — again without waiting — while retries remain; the last
/// failure gives up.
fn conclude_forward_final(
    ctx: &DriveContext<'_>,
    executor: &dyn FitExecutor,
    state: &mut ScmState,
    outcome: &FitOutcome,
    took: Option<Duration>,
) -> Result<()> {
    let max_attempts = ctx.plan.options.max_retries + 1;
    let fit = state
        .forward_final
        .as_mut()
        .expect("a forward model fit to conclude");
    fit.attempts.push(AttemptRecord {
        model: fit.model.clone(),
        outcome: outcome.label(),
    });
    fit.heuristics = outcome.heuristics.clone();
    let attempt = fit.attempts.len();
    let (mark, text) = describe_fit("forward model", attempt, max_attempts, outcome);
    report_fit(FORWARD_FINAL_DIR, mark, text, took);

    let mut next = None;
    if outcome.usable() {
        fit.status = CheckpointStatus::Succeeded;
        fit.ofv = outcome.ofv;
    } else if attempt < max_attempts {
        let prev = ctx.out_dir.join(&fit.model);
        let path = forward_final_path(ctx, attempt + 1);
        if !path.exists() {
            let description = format!(
                "{} (attempt {})",
                forward_final_description(&fit.retained),
                attempt + 1
            );
            ctx.writer.retry(
                &prev,
                &path,
                &description,
                Some(&format!("../{}", fit.source)),
                ctx.settings,
            )?;
        }
        fit.model = rel_to(&path, ctx.out_dir);
        next = Some(path);
    } else {
        fit.status = CheckpointStatus::Unusable;
    }
    state.save(ctx.out_dir)?;
    match next {
        Some(path) => executor.submit(std::slice::from_ref(&path)),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scm::round::RETRY_JITTER;
    use crate::scm::test_support::{
        Fit, MockExecutor, covs, full_scm_executor, make_plan, plan_of, req, try_plan,
        write_fit_output,
    };
    use crate::scm::{SCM_SUMMARY_MD, ScmOptions};

    /// The full fixture end to end. The fits dispatched, the files, the
    /// final state and the summary are pinned by the transcript snapshot;
    /// this checks what a transcript cannot show: the generated models.
    #[test]
    fn full_forward_backward_scm() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        let state = run_scm(&plan, &full_scm_executor(), false).unwrap();
        assert_eq!(state.status, ScmRunStatus::Completed);
        assert_eq!(state.retained, vec!["WT_CL".to_string()]);
        assert!(!state.had_unusable);

        // Round-2 models warm-start from the round-1 winner's fit: the
        // retained WT_CL theta carries its estimate (THETA4 = 0.25) and the
        // base thetas continue from the reference (THETA1 = 3.1) instead of
        // resetting to the initial model's own estimates.
        let r2_model = plan.out_dir_path().join("forward_round2/1001_crcl_cl.mod");
        let r2_content = fs::read_to_string(&r2_model).unwrap();
        assert!(r2_content.contains("0.25"), "{r2_content}");
        assert!(r2_content.contains("3.1"), "{r2_content}");

        // WT_V needed a retry in round 2. The retry model's released theta
        // continues from the failed attempt's last iteration (THETA6 =
        // 0.666), not from 0.1, and lands within the retry jitter of it
        // rather than exactly on it.
        let wt_v = state.rounds[2].candidate("WT_V").unwrap();
        assert!(wt_v.model.ends_with("_try2.mod"));
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

        // The final model releases WT_CL at the estimate from the last
        // reference fit (THETA4 = 0.25) and holds the others out.
        let final_model = plan
            .out_dir_path()
            .join(state.final_model.as_ref().unwrap());
        let content = fs::read_to_string(&final_model).unwrap();
        assert!(content.contains("0.25"), "{content}");
        assert!(content.contains("(0 FIX)   ; CRCL_CL cov"), "{content}");
        assert!(content.contains("(0 FIX)   ; WT_V cov"), "{content}");
    }

    const FORWARD_FINAL_KEY: &str = "forward_final/1001_scm_forward_final";

    /// The forward model's covariance-step fit starts the moment forward
    /// selection ends and runs beside backward elimination: no round waits
    /// for it, and it is on record when it ends.
    #[test]
    fn the_forward_model_is_refitted_with_the_cov_step_beside_backward_elimination() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        let executor = full_scm_executor();
        let state = run_scm(&plan, &executor, false).unwrap();

        // Started before the first backward round's fits, and never waited for
        let fits = executor.fits();
        let started = fits.iter().position(|f| f == FORWARD_FINAL_KEY).unwrap();
        let backward = fits
            .iter()
            .position(|f| f.starts_with("backward_round1/"))
            .unwrap();
        assert!(started < backward, "{fits:?}");
        assert_eq!(executor.submitted(), [FORWARD_FINAL_KEY]);

        let fit = state.forward_final.as_ref().unwrap();
        assert_eq!(fit.status, CheckpointStatus::Succeeded);
        assert_eq!(fit.ofv, Some(973.8));
        assert_eq!(fit.source, "forward_round2/1001_crcl_cl.mod");
        assert_eq!(fit.retained, ["WT_CL", "CRCL_CL"]);
        assert_eq!(fit.model, "forward_final/1001_scm_forward_final.mod");
        assert_eq!(fit.attempts.len(), 1);
        // The forward model with the cov step on: both effects released at
        // its estimates, the untested one still held out.
        let content = fs::read_to_string(plan.out_dir_path().join(&fit.model)).unwrap();
        assert!(content.contains("$COVARIANCE"), "{content}");
        assert!(content.contains("0.25"), "{content}");
        assert!(content.contains("(0 FIX)   ; WT_V cov"), "{content}");

        // Backward dropped CRCL_CL, so the final model is a fit of its own
        assert_eq!(
            state.final_model.as_deref(),
            Some("final/1001_scm_final.mod")
        );
        assert_eq!(state.final_ofv, Some(979.5));

        // Still running when the rounds are over, it is waited for then:
        // the one time it holds the process up.
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        let executor = full_scm_executor().submitted_fits_wait();
        let state = run_scm(&plan, &executor, false).unwrap();
        let fits = executor.fits();
        assert_eq!(
            &fits[fits.len() - 2..],
            [FORWARD_FINAL_KEY, "final/1001_scm_final"],
            "{fits:?}"
        );
        assert_eq!(state.forward_final.as_ref().unwrap().ofv, Some(973.8));
    }

    /// A failed attempt is retried from its estimates between rounds, again
    /// without waiting; the last failure gives up, and the process says so.
    #[test]
    fn a_failed_forward_model_fit_is_retried_between_rounds_and_given_up_on() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        let executor = full_scm_executor().with(
            FORWARD_FINAL_KEY,
            vec![Fit::NoFinalRow, Fit::SucceededWithWarnings(973.9)],
        );
        let state = run_scm(&plan, &executor, false).unwrap();
        let fit = state.forward_final.as_ref().unwrap();
        assert_eq!(fit.status, CheckpointStatus::Succeeded);
        assert_eq!(fit.model, "forward_final/1001_scm_forward_final_try2.mod");
        assert_eq!(fit.attempts.len(), 2);
        assert_eq!(fit.ofv, Some(973.9));
        assert!(
            fit.heuristics
                .contains(&"parameter near boundary".to_string())
        );
        assert_eq!(executor.submitted(), [FORWARD_FINAL_KEY, FORWARD_FINAL_KEY]);
        // The retry went out between rounds, before the last one
        let fits = executor.fits();
        let retry = fits.iter().rposition(|f| f == FORWARD_FINAL_KEY).unwrap();
        let last_round = fits
            .iter()
            .position(|f| f.starts_with("backward_round2/"))
            .unwrap();
        assert!(retry < last_round, "{fits:?}");

        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        let executor = full_scm_executor().with(FORWARD_FINAL_KEY, vec![Fit::NoFinalRow; 4]);
        let state = run_scm(&plan, &executor, false).unwrap();
        assert_eq!(state.status, ScmRunStatus::Completed);
        let fit = state.forward_final.as_ref().unwrap();
        assert_eq!(fit.status, CheckpointStatus::Unusable);
        assert_eq!(fit.attempts.len(), 4);
        assert_eq!(fit.ofv, None);
        assert_eq!(executor.fit_count(FORWARD_FINAL_KEY), 4);
        assert!(
            state
                .message
                .as_ref()
                .unwrap()
                .contains("forward model re-fit did not minimize")
        );
        // The final model is still fitted on its own
        assert_eq!(state.final_ofv, Some(979.5));
    }

    /// With the cov step in every model, nothing is re-fitted: the forward
    /// model is used as it is, and the last reference fit is copied into
    /// final/ as the final model, whatever backward elimination dropped.
    #[test]
    fn with_the_cov_step_on_everywhere_nothing_is_refitted() {
        let options = ScmOptions {
            cov_step: true,
            ..ScmOptions::default()
        };
        // Backward drops CRCL_CL: the final model is its last reference fit
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), options.clone());
        let executor = full_scm_executor();
        let state = run_scm(&plan, &executor, false).unwrap();
        let fit = state.forward_final.as_ref().unwrap();
        assert_eq!(fit.status, CheckpointStatus::Reused);
        assert_eq!(fit.model, "forward_round2/1001_crcl_cl.mod");
        assert_eq!(fit.ofv, Some(974.0));
        assert!(executor.submitted().is_empty());
        assert_eq!(executor.fit_count("forward_final"), 0);
        assert_eq!(executor.fit_count("final/1001_scm_final"), 0);
        assert!(!plan.out_dir_path().join("forward_final").exists());
        assert_eq!(state.retained, ["WT_CL"]);
        assert_eq!(state.final_model.as_deref(), Some("final/1001_crcl_cl.mod"));
        assert_eq!(state.final_ofv, Some(980.0));
        let out_dir = plan.out_dir_path();
        assert_eq!(
            fs::read_to_string(out_dir.join("final/1001_crcl_cl.mod")).unwrap(),
            fs::read_to_string(out_dir.join("backward_round1/1001_crcl_cl.mod")).unwrap()
        );
        assert!(out_dir.join("final/1001_crcl_cl/1001_crcl_cl.ext").exists());

        // Nothing dropped: the forward model's round fit is the final model
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), options);
        let executor =
            full_scm_executor().with("backward_round1/1001_crcl_cl", vec![Fit::Succeeded(1000.0)]);
        let state = run_scm(&plan, &executor, false).unwrap();
        assert_eq!(state.retained, ["WT_CL", "CRCL_CL"]);
        assert_eq!(executor.fit_count("final/1001_scm_final"), 0);
        assert_eq!(state.final_model.as_deref(), Some("final/1001_crcl_cl.mod"));
        assert_eq!(state.final_ofv, Some(974.0));
    }

    /// Backward elimination that drops nothing leaves the forward model as
    /// the final model, whose cov-step fit is then already in hand.
    #[test]
    fn backward_elimination_dropping_nothing_makes_the_forward_model_fit_the_final() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        // Dropping CRCL_CL now hurts as much as dropping WT_CL: both kept
        let executor =
            full_scm_executor().with("backward_round1/1001_crcl_cl", vec![Fit::Succeeded(1000.0)]);
        let state = run_scm(&plan, &executor, false).unwrap();
        assert_eq!(state.status, ScmRunStatus::Completed);
        assert_eq!(state.retained, ["WT_CL", "CRCL_CL"]);
        // No fit of its own: the forward model's fit is copied into final/,
        // model and run directory, and the copy's start record names the copy
        assert_eq!(executor.fit_count("final/1001_scm_final"), 0);
        assert_eq!(
            state.final_model.as_deref(),
            Some("final/1001_scm_forward_final.mod")
        );
        assert_eq!(state.final_ofv, Some(973.8));
        let out_dir = plan.out_dir_path();
        let copied = out_dir.join("final/1001_scm_forward_final.mod");
        assert_eq!(
            fs::read_to_string(&copied).unwrap(),
            fs::read_to_string(out_dir.join("forward_final/1001_scm_forward_final.mod")).unwrap()
        );
        let run_dir = out_dir.join("final/1001_scm_forward_final");
        for file in [
            "1001_scm_forward_final.ext",
            "1001_scm_forward_final.lst",
            "1001_scm_forward_final.mod",
            RUN_START_FILENAME,
        ] {
            assert!(run_dir.join(file).exists(), "{file}");
        }
        let start = RunStartFile::load(run_dir.join(RUN_START_FILENAME)).unwrap();
        assert!(
            start
                .model_path
                .ends_with("final/1001_scm_forward_final.mod")
                && !start.model_path.contains("forward_final/"),
            "{}",
            start.model_path
        );
        // The summary reads the copy's OFV off its own run directory
        let outcome = read_fit_outcome(&copied, &NonmemConfig::default(), false).unwrap();
        assert_eq!(outcome.ofv, Some(973.8));
    }

    /// The forward model's fit needs both phases and the option: a plan
    /// without either has no such fit on record.
    #[test]
    fn no_forward_model_fit_without_both_phases_or_with_the_option_off() {
        let variants = [
            ScmOptions {
                forward_final_cov_step: false,
                ..ScmOptions::default()
            },
            ScmOptions {
                direction: vec![Direction::Forward],
                ..ScmOptions::default()
            },
            ScmOptions {
                direction: vec![Direction::Backward],
                ..ScmOptions::default()
            },
        ];
        for options in variants {
            let dir = tempfile::tempdir().unwrap();
            let plan = make_plan(dir.path(), options.clone());
            let executor = full_scm_executor();
            let state = run_scm(&plan, &executor, false).unwrap();
            assert_eq!(state.status, ScmRunStatus::Completed, "{options:?}");
            assert!(state.forward_final.is_none(), "{options:?}");
            assert!(executor.submitted().is_empty(), "{options:?}");
            assert!(!plan.out_dir_path().join("forward_final").exists());
        }
    }

    /// A pause right after forward selection leaves the forward model's fit
    /// running; the resumed driver reads what it left rather than fitting
    /// it again.
    #[test]
    fn a_forward_model_fit_outlives_a_pause_and_is_read_on_resume() {
        let dir = tempfile::tempdir().unwrap();
        let options = ScmOptions {
            num_rounds: Some(3),
            ..ScmOptions::default()
        };
        let plan = make_plan(dir.path(), options);
        let executor = full_scm_executor().submitted_fits_wait();
        let state = run_scm(&plan, &executor, false).unwrap();
        assert_eq!(state.status, ScmRunStatus::Paused);
        assert_eq!(state.phase, Some(Direction::Backward));
        assert_eq!(executor.submitted(), [FORWARD_FINAL_KEY]);
        let fit = state.forward_final.as_ref().unwrap();
        assert_eq!(fit.status, CheckpointStatus::Running);
        assert_eq!(fit.attempts.len(), 0);

        // The fit ends while the process is paused
        let model = plan.out_dir_path().join(&fit.model);
        write_fit_output(&model, Fit::Succeeded(973.8)).unwrap();

        let executor = full_scm_executor();
        let state = run_scm(&plan, &executor, false).unwrap();
        assert_eq!(state.status, ScmRunStatus::Completed);
        let fit = state.forward_final.as_ref().unwrap();
        assert_eq!(fit.status, CheckpointStatus::Succeeded);
        assert_eq!(fit.ofv, Some(973.8));
        assert_eq!(executor.fit_count("forward_final"), 0);
        assert!(executor.submitted().is_empty());
    }

    /// A round left open by a fit that never ran, whose candidate the user
    /// then re-bounds in the config: the SCM process resumes, refits only
    /// that candidate under the new bounds, and keeps everything else.
    #[test]
    fn retuning_bounds_mid_round_resumes_and_refits_only_that_candidate() {
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
        let effects = vec![
            req("WT_CL"),
            req("CRCL_CL"),
            req("WT_V").bounds(Some(0.0), Some(3.0)),
        ];
        let bounded = try_plan(&plan.model_path(), &covs(effects), Some(&out_dir), unrun)
            .unwrap()
            .plan;

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
        // The bounds a written model estimates WT_V (THETA6) under.
        let bounds = |text: &str| {
            let t = &nonmem_parser::Model::parse(&refit, text).unwrap().thetas[5];
            (t.lower, t.upper)
        };
        assert_eq!(bounds(&text), (Some(0.0), Some(3.0)), "{text}");
        // The model written before them is left exactly as it was — its
        // theta unbounded, as the initial model has it — and was never fitted.
        assert_eq!(fs::read_to_string(&wt_v_round2).unwrap(), before);
        assert_eq!(bounds(&before), (None, None), "{before}");
        // Only round 2 was fitted: the refit once, and CRCL_CL — which the
        // retune does not touch — in the model already written for it.
        assert_eq!(
            executor.fits(),
            [
                "forward_round2/1001_crcl_cl",
                "forward_round2/1001_wt_v_refit2",
                "forward_round3/1001_wt_v",
                "forward_final/1001_scm_forward_final",
                "backward_round1/1001_wt_cl",
                "backward_round1/1001_crcl_cl",
                "backward_round2/1001_wt_cl",
                "final/1001_scm_final",
            ]
        );

        // The retune is on record, dated to the round it followed.
        let entry = outcome.roster_entry("WT_V").unwrap();
        assert_eq!(entry.candidate.upper, Some(3.0));
        assert_eq!(
            entry.retunes[0].after_round.as_deref(),
            Some("forward_round1")
        );
        let wt_v = outcome
            .round("forward_round2")
            .and_then(|r| r.candidate("WT_V"))
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

        // Resume, one round per invocation, until done
        let mut outcome = outcome;
        while outcome.status == ScmRunStatus::Paused {
            outcome = run_scm(&plan, &executor, false).unwrap();
        }
        assert_eq!(outcome.status, ScmRunStatus::Completed);
        assert_eq!(outcome.retained, vec!["WT_CL".to_string()]);
        assert_eq!(outcome.completed_rounds(), 5);

        // Nothing was fitted twice: each round-1 model exactly once
        assert_eq!(executor.fit_count("forward_round1/1001_wt_cl"), 1);
        assert_eq!(executor.fit_count("forward_round1/1001_crcl_cl"), 1);
        assert_eq!(executor.fit_count("base/1001_base"), 1);

        // Submitting the finished process again fits nothing and leaves it
        // complete; `--overwrite` starts over: the reference and round 1
        // are fitted again, then the cap pauses it.
        let again = full_scm_executor();
        assert_eq!(
            run_scm(&plan, &again, false).unwrap().status,
            ScmRunStatus::Completed
        );
        assert!(again.fits().is_empty());
        assert_eq!(
            run_scm(&plan, &again, true).unwrap().status,
            ScmRunStatus::Paused
        );
        assert_eq!(again.fits().len(), 4);
        assert_eq!(again.fit_count("base/1001_base"), 1);
    }

    /// A candidate removed while its round is open is withdrawn from that
    /// round: whatever it fitted is recorded but never scored, and the round
    /// decides itself among the rest on resume.
    #[test]
    fn removing_a_candidate_from_an_open_round_withdraws_it() {
        use crate::scm::ScmPlan;
        use crate::scm::test_support::fabricate_running_scm;

        let dir = tempfile::tempdir().unwrap();
        // Round 1 open: WT_CL scored, CRCL_CL still running, WT_V pending.
        let out_dir = fabricate_running_scm(dir.path());
        let plan = ScmPlan::load(out_dir.join(crate::scm::PLAN_FILENAME)).unwrap();

        // drop CRCL_CL: WT_CL wins the round on resume, nothing is refit
        let fewer = plan_of(
            &plan.model_path(),
            &["WT_CL", "WT_V"],
            None,
            plan.options.clone(),
        );
        let executor = full_scm_executor();
        let outcome = run_scm(&fewer, &executor, false).unwrap();
        assert_eq!(outcome.status, ScmRunStatus::Completed);
        let r1 = &outcome.rounds[1];
        let crcl = r1.candidate("CRCL_CL").unwrap();
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
        let wt_cl = r1.candidate("WT_CL").unwrap();

        // Failed, not succeeded — despite a readable OFV on attempt 1.
        assert_eq!(wt_cl.status, CandidateStatus::Unusable);
        assert_eq!(wt_cl.ofv, None);
        assert_eq!(wt_cl.p_value, None);
        assert_eq!(wt_cl.significant, None);

        // One attempt plus max_retries, every one of them retried.
        assert_eq!(wt_cl.attempts.len(), 4);
        assert_eq!(executor.fit_count("forward_round1/1001_wt_cl"), 4);

        // The reason reaches the record, on the attempt and as a heuristic.
        let outcomes: Vec<&str> = wt_cl.attempts.iter().map(|a| a.outcome.as_str()).collect();
        assert_eq!(outcomes, ["program aborted"; 4]);
        assert_eq!(wt_cl.heuristics, ["program aborted"]);

        // CRCL_CL wins on its own merits; the abort never competed.
        assert_eq!(r1.winner.as_deref(), Some("CRCL_CL"));
        assert!(state.had_unusable);
        assert!(state.message.as_ref().unwrap().contains("unusable"));
    }

    /// A stop request mid-round pauses the process with a note instead of
    /// failing it, and submitting again completes it with no attempt charged
    /// for the round that was interrupted.
    #[test]
    fn an_interrupted_process_pauses_and_resumes_without_a_charged_attempt() {
        struct StopAt<'a> {
            inner: &'a MockExecutor,
            stop_at: &'a str,
        }
        impl FitExecutor for StopAt<'_> {
            fn fit(&self, models: &[PathBuf], done: &dyn Fn(&Path)) -> Result<()> {
                if models
                    .iter()
                    .any(|m| m.to_string_lossy().contains(self.stop_at))
                {
                    return Err(crate::scm::Interrupted.into());
                }
                self.inner.fit(models, done)
            }
            fn describe(&self) -> String {
                "stops".to_string()
            }
        }

        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        let executor = full_scm_executor();
        let stopping = StopAt {
            inner: &executor,
            stop_at: "forward_round2",
        };
        let state = run_scm(&plan, &stopping, false).unwrap();
        assert_eq!(state.status, ScmRunStatus::Paused);
        assert_eq!(state.message.as_deref(), Some(crate::scm::INTERRUPTED_NOTE));

        let state = run_scm(&plan, &executor, false).unwrap();
        assert_eq!(state.status, ScmRunStatus::Completed);
        let round2 = state.round("forward_round2").unwrap();
        let crcl = round2
            .candidates
            .iter()
            .find(|c| c.candidate == "CRCL_CL")
            .unwrap();
        assert_eq!(
            crcl.attempts.len(),
            1,
            "the interrupted fit is not an attempt"
        );
    }

    /// The final re-fit is retried like any fit and records its heuristics; one
    /// that never minimizes has no OFV (the summary must not borrow the last
    /// reference fit's) and says so.
    #[test]
    fn the_final_refit_is_retried_and_a_failed_one_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        let retried = [Fit::NoFinalRow, Fit::SucceededWithWarnings(979.5)];
        let executor = full_scm_executor().with("final/1001_scm_final", retried.to_vec());
        let state = run_scm(&plan, &executor, false).unwrap();
        assert_eq!(
            state.final_model.as_deref(),
            Some("final/1001_scm_final_try2.mod")
        );
        assert_eq!(state.final_ofv, Some(979.5));
        assert!(
            state
                .final_heuristics
                .contains(&"parameter near boundary".to_string())
        );

        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        let executor = full_scm_executor().with("final/1001_scm_final", vec![Fit::NoFinalRow; 4]);
        let state = run_scm(&plan, &executor, false).unwrap();
        assert_eq!(state.status, ScmRunStatus::Completed);
        assert_eq!(executor.fit_count("final/1001_scm_final"), 4);
        assert_eq!(state.final_ofv, None);
        assert!(state.message.as_ref().unwrap().contains("did not minimize"));
        let summary = fs::read_to_string(plan.out_dir_path().join("scm_summary.json")).unwrap();
        let summary: serde_json::Value = serde_json::from_str(&summary).unwrap();
        assert!(summary["final_ofv"].is_null());
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
        assert_eq!(reference[0].candidates[0].attempts.len(), 1);
        let base_dir = plan.out_dir_path().join("base");
        assert!(!base_dir.join("1001_base_try2.mod").exists());
    }
}
