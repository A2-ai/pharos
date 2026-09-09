use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use fs_err as fs;
use nonmem_parser::Model;

use super::score::lrt;
use super::state::{AttemptRecord, CandidateRecord, CandidateStatus, RoundRecord, ScmState};
use super::{Candidate, Direction, ScmOptions, ScmPlan, nmtran_bound, parent_or_dot, sanitize_name};
use crate::copy::{CopyOptions, UpdateType, copy_model};
use crate::output_files::ext::{ExtReader, get_estimation_results};
use crate::output_files::lst::LstSummary;
use crate::output_files::resolve_estimation_files;
use crate::run::metadata::{RUN_END_FILENAME, RUN_START_FILENAME};
use crate::run::signal_wrapper::TERMINATION_FILENAME;
use crate::update;

/// Model file name (no extension) for a candidate attempt: `1001_wt_cl`,
/// its retries `1001_wt_cl_try2`, and — once the candidate's initial
/// estimate or bounds have been retuned mid-round (`refit` counts how often,
/// see [`crate::scm::roster::Retune`]) — `1001_wt_cl_refit2` and its own
/// retries `1001_wt_cl_refit2_try2`. The refit's models are named apart so
/// the attempts made under the old values keep their files untouched.
pub fn scm_model_name(stem: &str, candidate: &str, attempt: usize, refit: usize) -> String {
    let mut base = format!("{stem}_{}", sanitize_name(candidate));
    if refit > 0 {
        base.push_str(&format!("_refit{}", refit + 1));
    }
    if attempt <= 1 {
        base
    } else {
        format!("{base}_try{attempt}")
    }
}

/// A path's file stem as an owned string, `None` when it has no stem.
pub(crate) fn file_stem_of(path: &Path) -> Option<String> {
    path.file_stem().map(|s| s.to_string_lossy().to_string())
}

/// A path's file stem as an owned string, empty when it has no stem.
pub(crate) fn stem_of(path: &Path) -> String {
    file_stem_of(path).unwrap_or_default()
}

/// Where a model's run output lands (pharos' default layout: a subfolder next
/// to the model, named after it).
pub fn run_dir_for(model_path: &Path) -> PathBuf {
    parent_or_dot(model_path).join(stem_of(model_path))
}

/// Whether a model's run has finished, one way or another: pharos wrote its
/// RUN_END marker, or the signal wrapper recorded a termination.
pub fn run_finished(model: &Path) -> bool {
    let run_dir = run_dir_for(model);
    run_dir.join(RUN_END_FILENAME).exists() || run_dir.join(TERMINATION_FILENAME).exists()
}

/// The `.ext` file a run produced, honoring `$EST FILE=` overrides.
pub fn ext_path_for(model_path: &Path) -> PathBuf {
    let run_dir = run_dir_for(model_path);
    let stem = stem_of(model_path);
    let default = run_dir.join(format!("{stem}.ext"));
    match fs::read_to_string(model_path)
        .ok()
        .and_then(|s| Model::parse(model_path, &s).ok())
    {
        Some(model) => resolve_estimation_files(&model, &run_dir, &default)
            .last()
            .cloned()
            .unwrap_or(default),
        None => default,
    }
}

/// Copy `from` to `dest` as an SCM-generated model: no estimate updates,
/// metadata (description, based_on, tags) only when `with_metadata` is set.
fn copy_scm_model(
    from: &Path,
    dest: &Path,
    description: &str,
    based_on: Option<&str>,
    with_metadata: bool,
    tags: &[&str],
) -> Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    let original_filename = from
        .file_name()
        .context("source model has no file name")?
        .to_string_lossy()
        .to_string();
    let new_filename = dest
        .file_name()
        .context("destination model has no file name")?
        .to_string_lossy()
        .to_string();

    let options = CopyOptions {
        update: vec![UpdateType::None],
        description: description.to_string(),
        based_on: match (with_metadata, based_on) {
            (true, Some(b)) => vec![b.to_string()],
            _ => vec![],
        },
        tags: tags.iter().map(|t| t.to_string()).collect(),
        no_metadata: !with_metadata,
        ..Default::default()
    };
    copy_model(from, dest, &original_filename, &new_filename, &options)
}

/// The `$THETA` spec for a released candidate: `init` on its own, or wrapped
/// in the candidate's bounds. Those are resolved at plan time from the
/// config's `lower` / `upper` falling back to the template's own spec, so a
/// candidate authored as a bounded theta (`(0, 0.1)`) keeps its bounds when
/// the effect goes back in.
fn released_spec(lower: Option<f64>, upper: Option<f64>, init: f64) -> String {
    match (lower, upper) {
        (None, None) => init.to_string(),
        (Some(lower), None) => format!("({}, {init})", nmtran_bound(lower)),
        (lower, Some(upper)) => {
            // An upper bound cannot be given without a lower one.
            let lower = lower.unwrap_or(f64::NEG_INFINITY);
            format!("({}, {init}, {})", nmtran_bound(lower), nmtran_bound(upper))
        }
    }
}

/// The `$THETA` spec pinning a held-out effect: `(0 FIX)`, `(1 FIX)`, ...
fn held_out_spec(off: f64) -> String {
    format!("({} FIX)", nmtran_bound(off))
}

/// Write one SCM model: a copy of the template in which the `released`
/// covariate thetas (1-based) are free and every other `candidates` theta is
/// pinned at `(off FIX)` — the effect held out of the model, `off` being 0
/// for the usual forms and 1 for a fold-change form — with the
/// `$COVARIANCE` record added or removed per `cov_step`.
///
/// Pinning is what lets the template carry a candidate as an ordinary free
/// theta with a real initial estimate: the config names the candidates, and
/// every generated model fixes the ones it is not testing.
///
/// A released theta starts from its estimate in `reference_ext` when it was
/// free there too (a held-out theta reports exactly its off value), and
/// otherwise from the candidate's own [`Candidate::initial`], which the plan
/// resolved from the config and the template. Bounds the template gave the
/// theta are kept.
///
/// With a `reference_ext`, the model also warm-starts every other free
/// parameter (base thetas, omegas, sigmas) from the reference fit. Without
/// one (the reference fit itself), everything starts from the template's
/// initial estimates.
#[allow(clippy::too_many_arguments)]
pub fn write_scm_model(
    template: &Path,
    dest: &Path,
    candidates: &[Candidate],
    released: &[usize],
    reference_ext: Option<&Path>,
    cov_step: bool,
    description: &str,
    based_on: Option<&str>,
    with_metadata: bool,
) -> Result<()> {
    copy_scm_model(
        template,
        dest,
        description,
        based_on,
        with_metadata,
        &["scm"],
    )?;

    // Warm start: pull the reference fit's estimates into the copy. Fixed
    // thetas are left untouched by the updater; every candidate theta is
    // rewritten below anyway.
    // A missing or unreadable reference output degrades to a cold start with
    // a warning — a worse initial point must not kill the SCM process.
    let mut reference_estimates: HashMap<String, f64> = HashMap::new();
    if let Some(ext) = reference_ext {
        if ext.exists() {
            match update::read_ext_estimates(ext, &[UpdateType::All], true) {
                Ok(estimates) => {
                    match update::update_model_estimates(dest, ext, &[UpdateType::All], true) {
                        Ok(()) => reference_estimates = estimates,
                        Err(e) => log::warn!(
                            "could not warm-start {} from {}: {e:#}",
                            dest.display(),
                            ext.display()
                        ),
                    }
                }
                Err(e) => log::warn!(
                    "could not read reference estimates from {}: {e:#}",
                    ext.display()
                ),
            }
        } else {
            log::warn!(
                "reference output {} not found; {} starts from the template's initial estimates",
                ext.display(),
                dest.display()
            );
        }
    }

    // Re-open the copied model and rewrite the covariate theta specs.
    let content = fs::read_to_string(dest)?;
    let model = Model::parse(dest, &content)?;

    // The template's own theta specs, read before the warm start overwrote
    // the copy's: bounds and the `(0 FIX)` shape come from how the effect was
    // authored, not from whatever the reference fit left behind.
    let template_content = fs::read_to_string(template)?;
    let template_model = Model::parse(template, &template_content)?;

    let released_set: BTreeSet<usize> = released.iter().copied().collect();
    if let Some(unknown) = released_set
        .iter()
        .find(|t| !candidates.iter().any(|c| c.theta == **t))
    {
        bail!("THETA({unknown}) was released but is not a candidate in the plan");
    }

    let mut specs: BTreeMap<usize, String> = BTreeMap::new();
    for candidate in candidates {
        let theta_num = candidate.theta;
        if theta_num == 0 || theta_num > model.thetas.len() {
            bail!(
                "THETA({theta_num}) out of range: model has {} thetas",
                model.thetas.len()
            );
        }
        let Some(template_theta) = template_model.thetas.get(theta_num - 1) else {
            bail!(
                "THETA({theta_num}) out of range: the template has {} thetas",
                template_model.thetas.len()
            );
        };

        if !released_set.contains(&theta_num) {
            // Held out of this model. A candidate the template already writes
            // `(off FIX)` is left exactly as authored; anything else is pinned.
            if !candidate.is_held_out_spec(template_theta.fixed, template_theta.init) {
                specs.insert(theta_num - 1, held_out_spec(candidate.off));
            }
            continue;
        }

        // Free in the reference fit -> continue from its estimate; a theta
        // held out there reports exactly its off value, so start it where
        // the plan says. An estimate that does not sit strictly inside the
        // candidate's bounds is no use as a warm start — NM-TRAN would
        // reject it — so fall back to the plan's release value, which plan
        // time already checked against the bounds.
        // A plan built by this version already carries the template's own
        // bounds wherever the config gave none; the fallback to the template
        // keeps a plan.json written before candidates had bounds behaving
        // exactly as it did.
        let lower = candidate.lower.or(template_theta.lower);
        let upper = candidate.upper.or(template_theta.upper);
        let inside = |v: f64| lower.is_none_or(|l| v > l) && upper.is_none_or(|u| v < u);
        let init = match reference_estimates.get(&format!("THETA{theta_num}")) {
            Some(&est) if est.is_finite() && est != candidate.off && inside(est) => est,
            _ => candidate.initial,
        };
        specs.insert(theta_num - 1, released_spec(lower, upper, init));
    }

    let mut replacements = model.theta_spec_replacements(&specs)?;
    if !cov_step {
        replacements.extend(model.covariance_removal_replacements());
    }
    let mut new_content = model.render_with_replacements(&replacements);

    if cov_step && model.covariance.is_none() {
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        new_content.push_str("$COVARIANCE\n");
    }

    fs::write(dest, new_content)?;
    Ok(())
}

/// Write a retry model: a copy of the previous attempt whose initial
/// estimates continue from wherever that attempt stopped (final estimates if
/// it finished, the last iteration otherwise). Never jittered.
pub fn write_retry_model(
    prev_model: &Path,
    dest: &Path,
    description: &str,
    based_on: Option<&str>,
    with_metadata: bool,
) -> Result<()> {
    copy_scm_model(
        prev_model,
        dest,
        description,
        based_on,
        with_metadata,
        &["scm", "retry"],
    )?;

    let ext = ext_path_for(prev_model);
    if ext.exists() {
        // A run that died before NONMEM wrote the .ext header leaves a file
        // that parses to no tables at all. That is a worse starting point,
        // never a reason to abandon the whole SCM process: fall back to the
        // model's own initial estimates and let the candidate conclude on
        // its own merits once its retries run out.
        if let Err(e) = update::update_model_estimates(dest, &ext, &[UpdateType::All], true) {
            log::warn!(
                "could not carry estimates from {} into {}: {e:#}; \
                 retrying with unchanged initial estimates",
                ext.display(),
                dest.display()
            );
        }
    } else {
        log::warn!(
            "no .ext output found for {}; retrying with unchanged initial estimates",
            prev_model.display()
        );
    }

    Ok(())
}

/// Everything the driver needs to know about how a fit went.
#[derive(Debug, Clone, PartialEq)]
pub struct FitOutcome {
    pub started: bool,
    pub finished: bool,
    pub terminated: bool,
    pub ofv: Option<f64>,
    pub minimization_terminated: Option<bool>,
    /// NONMEM aborted the estimation itself (`PROGRAM TERMINATED BY OBJ`).
    /// The run still writes a final-estimates row holding the last diverged
    /// iteration, so an OFV is readable and means nothing.
    pub program_aborted: Option<bool>,
    /// Human labels of the heuristic checks that fired.
    pub heuristics: Vec<String>,
}

impl FitOutcome {
    /// A fit is usable for scoring when it ran to completion, was not killed,
    /// produced an OFV, and neither terminated minimization nor aborted the
    /// estimation. Anything else is reported, never silently treated as
    /// insignificant.
    pub fn usable(&self) -> bool {
        self.finished
            && !self.terminated
            && self.ofv.is_some()
            && self.minimization_terminated != Some(true)
            && self.program_aborted != Some(true)
    }

    pub fn label(&self) -> String {
        if self.terminated {
            "terminated".to_string()
        } else if !self.finished {
            if self.started {
                "did not finish".to_string()
            } else {
                "never started".to_string()
            }
        } else if self.program_aborted == Some(true) {
            "program aborted".to_string()
        } else if self.minimization_terminated == Some(true) {
            "minimization terminated".to_string()
        } else if self.ofv.is_none() {
            "no ofv".to_string()
        } else {
            "succeeded".to_string()
        }
    }
}

/// Fold one finished attempt into a candidate's record: the attempt is
/// appended, and the candidate either concludes as scored or drops back to
/// pending so the next wave retries it.
///
/// Shared by the driver (which fits the model) and the status reader (which
/// sees the same finished run on disk before the driver's batch returns), so
/// both reach the same conclusion from the same evidence.
pub fn record_attempt(cand: &mut CandidateRecord, model_rel: String, outcome: &FitOutcome) {
    cand.attempts.push(AttemptRecord {
        model: model_rel.clone(),
        outcome: outcome.label(),
    });
    cand.model = model_rel;
    cand.heuristics = outcome.heuristics.clone();
    if outcome.usable() {
        cand.status = CandidateStatus::Succeeded;
        cand.ofv = outcome.ofv;
    } else {
        cand.status = CandidateStatus::Pending;
    }
}

/// Bring the SCM process state up to date with what the fits have left on disk,
/// returning the models still running (relative to `out_dir`).
///
/// The driver dispatches a whole wave of fits at once and only writes their
/// outcomes back to `scm_state.json` after the entire batch returns, so
/// mid-round the state still calls every candidate `running` even once most
/// of their runs have finished. A reader has exactly the evidence the driver
/// will use — the run's own output — so it draws the same conclusion here
/// rather than reporting a round as untouched until its last fit lands.
/// Reading never writes: the state file stays the driver's to update.
///
/// Every reader of a live SCM process goes through this, so status, a round view
/// and the decision log all describe the same SCM process.
pub fn reconcile_state_with_disk(
    state: &mut ScmState,
    out_dir: &Path,
    options: &ScmOptions,
) -> Vec<String> {
    let mut running = Vec::new();
    for round in &mut state.rounds {
        if round.complete {
            continue;
        }
        reconcile_round_with_disk(round, out_dir, options, &mut running);
    }
    running
}

/// [`reconcile_state_with_disk`] for a single open round, appending the
/// models still running to `running`.
pub fn reconcile_round_with_disk(
    round: &mut RoundRecord,
    out_dir: &Path,
    options: &ScmOptions,
    running: &mut Vec<String>,
) {
    for cand in &mut round.candidates {
        // Only a dispatched candidate has a run to look at; a concluded one
        // already carries the driver's own reading of it.
        if cand.status != CandidateStatus::Running || cand.model.is_empty() {
            continue;
        }
        let model_path = out_dir.join(&cand.model);
        if !run_finished(&model_path) {
            if run_dir_for(&model_path).join(RUN_START_FILENAME).exists() {
                running.push(cand.model.clone());
            }
            continue;
        }
        match read_fit_outcome(&model_path) {
            Ok(outcome) => {
                let rel = cand.model.clone();
                record_attempt(cand, rel, &outcome);
            }
            // Output we cannot read is no evidence either way; leave the
            // candidate as the driver last wrote it.
            Err(e) => log::warn!("failed to read outcome of {}: {e}", model_path.display()),
        }
    }
    score_round_so_far(round, options);
}

/// Score every candidate in an open round that has already concluded as
/// succeeded but has no score yet.
///
/// The driver scores a round in one pass once its last fit lands (see
/// `driver::run_scm`), so mid-round the state carries an OFV and nothing
/// else. The test is arithmetic on evidence a reader already has — the
/// candidate's OFV, the round's reference OFV, its df and the phase alpha —
/// and the reference OFV only moves when a round concludes, so scoring a
/// finished candidate here reaches exactly the numbers the driver will
/// write. Ranking, the winner and the decision still wait for the whole
/// round: those need every candidate.
fn score_round_so_far(round: &mut RoundRecord, options: &ScmOptions) {
    if round.is_reference() {
        return;
    }
    let Some(reference_ofv) = round.reference_ofv else {
        return;
    };
    let direction = round.direction;
    let alpha = match direction {
        Direction::Forward => options.forward_alpha,
        Direction::Backward => options.backward_alpha,
    };
    for cand in &mut round.candidates {
        // A scored candidate carries the driver's own numbers; a candidate
        // with 0 df would score as never-significant, which is the driver's
        // error to report, not ours to bake in.
        if cand.status != CandidateStatus::Succeeded || cand.p_value.is_some() || cand.df == 0 {
            continue;
        }
        let Some(ofv) = cand.ofv else { continue };
        let r = lrt(reference_ofv, ofv, cand.df, direction);
        cand.delta_ofv = Some(r.delta_ofv);
        cand.p_value = Some(r.p_value);
        cand.significant = Some(r.p_value < alpha);
    }
}

/// Read the outcome of a model's run from its output directory.
pub fn read_fit_outcome(model_path: &Path) -> Result<FitOutcome> {
    let run_dir = run_dir_for(model_path);

    let started = run_dir.join(RUN_START_FILENAME).exists();
    let finished = run_dir.join(RUN_END_FILENAME).exists();
    let terminated = run_dir.join(TERMINATION_FILENAME).exists();

    let ext = ext_path_for(model_path);
    let ofv = if ext.exists() {
        let reader = ExtReader::default().final_estimates_and_stderr_and_fixed();
        match get_estimation_results(&ext, &reader, None, false, None) {
            Ok(results) => results.last().and_then(|r| r.minimization_results.ofv),
            Err(e) => {
                log::warn!("failed to parse {}: {e}", ext.display());
                None
            }
        }
    } else {
        None
    };

    let (minimization_terminated, program_aborted, heuristics) =
        read_lst_heuristics(model_path, &run_dir);

    Ok(FitOutcome {
        started,
        finished,
        terminated,
        ofv,
        minimization_terminated,
        program_aborted,
        heuristics,
    })
}

fn read_lst_heuristics(
    model_path: &Path,
    run_dir: &Path,
) -> (Option<bool>, Option<bool>, Vec<String>) {
    let stem = stem_of(model_path);
    let lst_path = run_dir.join(format!("{stem}.lst"));
    if !lst_path.exists() {
        return (None, None, vec![]);
    }
    match LstSummary::from_run(&lst_path) {
        Ok(summary) => {
            let h = &summary.run_heuristics;
            let fired = [
                (h.minimization_terminated, "minimization terminated"),
                (h.program_aborted, "program aborted"),
                (h.parameter_near_boundary, "parameter near boundary"),
                (h.hessian_reset, "hessian reset"),
                (h.covariance_step_aborted, "covariance step aborted"),
                (h.eigenvalue_issues, "eigenvalue issues"),
            ]
            .iter()
            .filter(|(flag, _)| *flag == Some(true))
            .map(|(_, label)| label.to_string())
            .collect();
            (h.minimization_terminated, h.program_aborted, fired)
        }
        Err(e) => {
            log::warn!("failed to parse {}: {e}", lst_path.display());
            (None, None, vec![])
        }
    }
}

/// Best-effort `pharos nonmem summary` of a finished run, written as
/// `pharos_summary.json` into the run directory (the same JSON `pharos
/// nonmem summary --json` prints). Failures are logged, never fatal: the
/// summary is a record, not a scoring input.
pub fn write_run_summary(model_path: &Path) {
    let run_dir = run_dir_for(model_path);
    let summary = match crate::output_files::get_summary(&run_dir, None, false) {
        Ok(summary) => summary,
        Err(e) => {
            log::warn!("could not summarize run {}: {e:#}", run_dir.display());
            return;
        }
    };
    let path = run_dir.join(super::RUN_SUMMARY_FILENAME);
    if let Err(e) = utils::write_json_to_file(&summary, &path) {
        log::warn!("could not write {}: {e:#}", path.display());
    }
}

/// Which thetas a model in a round releases, and what it is testing.
#[derive(Debug, Clone, PartialEq)]
pub struct RoundEntry {
    /// Candidate under test ("base"/"full" for reference fits).
    pub candidate: String,
    /// "add X" / "drop X" / "fit base model" / "fit full model".
    pub action: String,
    /// 1-based theta numbers released in this model.
    pub released: Vec<usize>,
    /// LRT degrees of freedom: how many thetas this model releases (forward)
    /// or re-fixes (backward) relative to the round's reference. Derived
    /// from the released-set delta so a future multi-theta candidate (e.g. a
    /// categorical covariate needing k-1 thetas) is scored correctly by
    /// construction. 0 for reference fits, which are never LRT-scored.
    pub df: usize,
}

/// Build the entries for a forward round: each not-yet-retained candidate is
/// tested by releasing it on top of the retained set.
pub fn forward_entries(plan: &ScmPlan, retained: &[String]) -> Vec<RoundEntry> {
    let n_retained_thetas = plan.thetas_for(retained).len();
    plan.candidates
        .iter()
        .filter(|c| !retained.contains(&c.name))
        .map(|c| {
            let mut names: Vec<String> = retained.to_vec();
            names.push(c.name.clone());
            let released = plan.thetas_for(&names);
            RoundEntry {
                candidate: c.name.clone(),
                action: format!("add {}", c.name),
                df: released.len().saturating_sub(n_retained_thetas),
                released,
            }
        })
        .collect()
}

/// Build the entries for a backward round: each retained candidate is tested
/// by re-fixing it while the rest stay released.
pub fn backward_entries(plan: &ScmPlan, retained: &[String]) -> Vec<RoundEntry> {
    let n_retained_thetas = plan.thetas_for(retained).len();
    retained
        .iter()
        .map(|name| {
            let names: Vec<String> = retained.iter().filter(|n| *n != name).cloned().collect();
            let released = plan.thetas_for(&names);
            RoundEntry {
                candidate: name.clone(),
                action: format!("drop {name}"),
                df: n_retained_thetas.saturating_sub(released.len()),
                released,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scm::plan::tests::names;
    use crate::scm::plan::tests::write_template;
    use crate::scm::{ScmOptions, build_plan};

    /// The template's three candidate effects, released at 0.1 unless the
    /// test's template gives the theta an initial estimate of its own, and
    /// held out at 0.
    fn cands(inits: &[(usize, f64)]) -> Vec<Candidate> {
        inits
            .iter()
            .map(|&(theta, initial)| Candidate {
                name: format!("THETA{theta}"),
                theta,
                initial,
                off: 0.0,
                ..Default::default()
            })
            .collect()
    }

    /// A fold-change effect on THETA(4): off at 1, released at 1.3.
    fn fold_change_cands() -> Vec<Candidate> {
        let mut c = cands(&[(4, 1.3), (5, 0.1), (6, 0.1)]);
        c[0].off = 1.0;
        c
    }

    /// A held-out fold-change effect is pinned at `(1 FIX)`, not `(0 FIX)`
    /// (which would zero the parameter for every SEX = 1 subject); released,
    /// it starts at its own initial. Warm-starting reads an estimate equal
    /// to the off value as "held out in the reference".
    #[test]
    fn a_fold_change_candidate_is_held_out_at_one() {
        let dir = tempfile::tempdir().unwrap();
        let fold = crate::scm::plan::tests::TEMPLATE
            .replace("WT_CL = (WT/70)**THETA(4)", "WT_CL = THETA(4)**(WT/70)")
            .replace("$THETA (0 FIX)   ; WT_CL cov", "$THETA 1.3   ; WT_CL cov");
        let template = crate::scm::plan::tests::write_template_content(dir.path(), &fold);

        // held out: pinned at 1
        let held = dir.path().join("scm/1001/forward_round1/1001_crcl_cl.mod");
        write_scm_model(
            &template,
            &held,
            &fold_change_cands(),
            &[5],
            None,
            true,
            "SCM test",
            None,
            false,
        )
        .unwrap();
        let content = fs::read_to_string(&held).unwrap();
        assert!(
            content.contains("$THETA (1 FIX)   ; WT_CL cov"),
            "{content}"
        );

        // a reference fit in which THETA4 was held out (reports exactly 1)
        let ext_path = dir.path().join("ref.ext");
        fs::write(
            &ext_path,
            "TABLE NO.     1: First Order Conditional Estimation with Interaction\n \
 ITERATION    THETA1       THETA2       THETA3       THETA4       THETA5       THETA6       OMEGA(1,1)   OMEGA(2,2)   SIGMA(1,1)   OBJ\n  \
 -1000000000  3.10000E+00  2.10000E+01  1.30000E+00  1.00000E+00  2.50000E-01  0.00000E+00  9.00000E-02  8.50000E-02  1.80000E-02  980\n",
        )
        .unwrap();
        let released = dir.path().join("scm/1001/forward_round2/1001_wt_cl.mod");
        write_scm_model(
            &template,
            &released,
            &fold_change_cands(),
            &[4, 5],
            Some(&ext_path),
            true,
            "SCM test",
            None,
            false,
        )
        .unwrap();
        let content = fs::read_to_string(&released).unwrap();
        let model = Model::parse(&released, &content).unwrap();
        // released fresh at its initial, not at the reference's 1.0
        assert!(!model.thetas[3].fixed, "{content}");
        assert!((model.thetas[3].init - 1.3).abs() < 1e-12, "{content}");
        // the retained CRCL_CL continues from its reference estimate
        assert!((model.thetas[4].init - 0.25).abs() < 1e-12, "{content}");
    }

    #[test]
    fn released_spec_spells_infinite_bounds_the_nmtran_way() {
        assert_eq!(released_spec(None, None, 0.1), "0.1");
        assert_eq!(released_spec(Some(0.0), None, 0.1), "(0, 0.1)");
        assert_eq!(released_spec(Some(-2.0), Some(2.0), 0.4), "(-2, 0.4, 2)");
        assert_eq!(
            released_spec(Some(f64::NEG_INFINITY), Some(2.0), 0.1),
            "(-INF, 0.1, 2)"
        );
        assert_eq!(released_spec(None, Some(2.0), 0.1), "(-INF, 0.1, 2)");
        assert_eq!(
            released_spec(Some(0.0), Some(f64::INFINITY), 0.1),
            "(0, 0.1, INF)"
        );
    }

    #[test]
    fn model_names_carry_attempt_suffix() {
        assert_eq!(scm_model_name("1001", "WT_CL", 1, 0), "1001_wt_cl");
        assert_eq!(scm_model_name("1001", "WT_CL", 2, 0), "1001_wt_cl_try2");
        assert_eq!(scm_model_name("1001", "WT_CL", 1, 1), "1001_wt_cl_refit2");
        assert_eq!(
            scm_model_name("1001", "WT_CL", 2, 1),
            "1001_wt_cl_refit2_try2"
        );
    }

    #[test]
    fn write_scm_model_releases_and_rebases() {
        let dir = tempfile::tempdir().unwrap();
        let template = write_template(dir.path());

        let dest = dir.path().join("scm/1001/forward_round1/1001_wt_cl.mod");
        write_scm_model(
            &template,
            &dest,
            &cands(&[(4, 0.1), (5, 0.1), (6, 0.1)]),
            &[4],
            None,
            true,
            "SCM test",
            None,
            false,
        )
        .unwrap();

        let content = fs::read_to_string(&dest).unwrap();
        // Released at 0.1, comment preserved
        assert!(content.contains("$THETA 0.1   ; WT_CL cov"), "{content}");
        // Other candidates still fixed
        assert!(content.contains("(0 FIX)   ; CRCL_CL cov"), "{content}");
        assert!(content.contains("(0 FIX)   ; WT_V cov"), "{content}");
        // $DATA rebased to still point at the template's dataset
        assert!(content.contains("../../../data.csv"), "{content}");
        // $COVARIANCE retained (template has one, cov_step on)
        assert!(content.contains("$COVARIANCE"), "{content}");

        // The released model parses and has the right free thetas
        let model = Model::parse(&dest, &content).unwrap();
        assert!(!model.thetas[3].fixed);
        assert!((model.thetas[3].init - 0.1).abs() < 1e-12);
        assert!(model.thetas[4].fixed);
    }

    #[test]
    fn write_scm_model_strips_covariance_when_cov_step_off() {
        let dir = tempfile::tempdir().unwrap();
        let template = write_template(dir.path());
        let dest = dir.path().join("scm/1001/forward_round1/1001_wt_cl.mod");
        write_scm_model(
            &template,
            &dest,
            &cands(&[(4, 0.1), (5, 0.1), (6, 0.1)]),
            &[4],
            None,
            false,
            "SCM test",
            None,
            false,
        )
        .unwrap();
        let content = fs::read_to_string(&dest).unwrap();
        assert!(!content.contains("$COVARIANCE"), "{content}");
        Model::parse(&dest, &content).unwrap();
    }

    #[test]
    fn write_scm_model_appends_covariance_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let template_content = crate::scm::plan::tests::TEMPLATE.replace("$COVARIANCE\n", "");
        let template =
            crate::scm::plan::tests::write_template_content(dir.path(), &template_content);
        let dest = dir.path().join("scm/1001/forward_round1/1001_wt_cl.mod");
        write_scm_model(
            &template,
            &dest,
            &cands(&[(4, 0.1), (5, 0.1), (6, 0.1)]),
            &[4],
            None,
            true,
            "SCM test",
            None,
            false,
        )
        .unwrap();
        let content = fs::read_to_string(&dest).unwrap();
        assert!(content.trim_end().ends_with("$COVARIANCE"), "{content}");
        Model::parse(&dest, &content).unwrap();
    }

    #[test]
    fn write_scm_model_warm_starts_from_reference_ext() {
        let dir = tempfile::tempdir().unwrap();
        let template = write_template(dir.path());

        // A reference fit in which THETA4 (WT_CL) was free (estimate 0.25)
        // and THETA5/THETA6 were still `(0 FIX)` (reported as exactly 0).
        let ext_path = dir.path().join("ref.ext");
        let ext = "\
TABLE NO.     1: First Order Conditional Estimation with Interaction
 ITERATION    THETA1       THETA2       THETA3       THETA4       THETA5       THETA6       OMEGA(1,1)   OMEGA(2,2)   SIGMA(1,1)   OBJ
  -1000000000  3.10000E+00  2.10000E+01  1.30000E+00  2.50000E-01  0.00000E+00  0.00000E+00  9.00000E-02  8.50000E-02  1.80000E-02  980
";
        fs::write(&ext_path, ext).unwrap();

        // A round-2 model: WT_CL retained, CRCL_CL under test.
        let dest = dir.path().join("scm/1001/forward_round2/1001_crcl_cl.mod");
        write_scm_model(
            &template,
            &dest,
            &cands(&[(4, 0.1), (5, 0.1), (6, 0.1)]),
            &[4, 5],
            Some(&ext_path),
            true,
            "SCM test",
            None,
            false,
        )
        .unwrap();

        let content = fs::read_to_string(&dest).unwrap();
        let model = Model::parse(&dest, &content).unwrap();
        // Base parameters continue from the reference fit
        assert!((model.thetas[0].init - 3.1).abs() < 1e-9, "{content}");
        assert!(content.contains("0.09"), "{content}"); // OMEGA(1,1)
        // The retained covariate continues from its reference estimate
        assert!(!model.thetas[3].fixed);
        assert!((model.thetas[3].init - 0.25).abs() < 1e-9, "{content}");
        // The newly released covariate starts fresh at release_init
        assert!(!model.thetas[4].fixed);
        assert!((model.thetas[4].init - 0.1).abs() < 1e-9, "{content}");
        // The untested candidate stays fixed at 0
        assert!(model.thetas[5].fixed);
        assert!(model.thetas[5].init.abs() < 1e-12, "{content}");
    }

    #[test]
    fn a_free_candidate_theta_is_pinned_when_held_out_and_starts_from_its_own_init() {
        let dir = tempfile::tempdir().unwrap();
        // WT_CL is authored as an ordinary free theta carrying an initial
        // guess; only `covariates` marks it as a candidate.
        let free = crate::scm::plan::tests::TEMPLATE
            .replace("$THETA (0 FIX)   ; WT_CL cov", "$THETA 0.4   ; WT_CL cov");
        let template = crate::scm::plan::tests::write_template_content(dir.path(), &free);

        // Held out: fixed at 0 like every other untested effect.
        let held = dir.path().join("scm/1001/forward_round1/1001_crcl_cl.mod");
        write_scm_model(
            &template,
            &held,
            &cands(&[(4, 0.4), (5, 0.1), (6, 0.1)]),
            &[5],
            None,
            true,
            "SCM test",
            None,
            false,
        )
        .unwrap();
        let content = fs::read_to_string(&held).unwrap();
        let model = Model::parse(&held, &content).unwrap();
        assert!(model.thetas[3].fixed, "{content}");
        assert!(model.thetas[3].init.abs() < 1e-12, "{content}");

        // Released: starts where the plan resolved it from the template.
        let released = dir.path().join("scm/1001/forward_round1/1001_wt_cl.mod");
        write_scm_model(
            &template,
            &released,
            &cands(&[(4, 0.4), (5, 0.1), (6, 0.1)]),
            &[4],
            None,
            true,
            "SCM test",
            None,
            false,
        )
        .unwrap();
        let content = fs::read_to_string(&released).unwrap();
        let model = Model::parse(&released, &content).unwrap();
        assert!(!model.thetas[3].fixed, "{content}");
        assert!((model.thetas[3].init - 0.4).abs() < 1e-12, "{content}");
    }

    /// Bounds the config set reach the generated model, override the
    /// template's own, and a reference estimate that falls outside them is
    /// not used as a warm start.
    #[test]
    fn plan_bounds_override_the_templates_and_gate_the_warm_start() {
        let dir = tempfile::tempdir().unwrap();
        let bounded = crate::scm::plan::tests::TEMPLATE.replace(
            "$THETA (0 FIX)   ; WT_CL cov",
            "$THETA (-2, 0.4, 2)   ; WT_CL cov",
        );
        let template = crate::scm::plan::tests::write_template_content(dir.path(), &bounded);

        // THETA4 (WT_CL) was estimated at -0.5 in the reference fit, below
        // the lower bound the config now imposes; THETA5 (CRCL_CL) was held
        // out there.
        let ext_path = dir.path().join("ref.ext");
        fs::write(
            &ext_path,
            "\
TABLE NO.     1: First Order Conditional Estimation with Interaction
 ITERATION    THETA1       THETA2       THETA3       THETA4       THETA5       THETA6       OMEGA(1,1)   OMEGA(2,2)   SIGMA(1,1)   OBJ
  -1000000000  3.10000E+00  2.10000E+01  1.30000E+00 -5.00000E-01  0.00000E+00  0.00000E+00  9.00000E-02  8.50000E-02  1.80000E-02  980
",
        )
        .unwrap();

        let mut candidates = cands(&[(4, 0.4), (5, 0.1), (6, 0.1)]);
        candidates[0].lower = Some(0.0);
        candidates[0].upper = Some(5.0);
        candidates[1].lower = Some(0.0);

        let dest = dir.path().join("scm/1001/forward_round2/1001_crcl_cl.mod");
        write_scm_model(
            &template,
            &dest,
            &candidates,
            &[4, 5],
            Some(&ext_path),
            true,
            "SCM test",
            None,
            false,
        )
        .unwrap();
        let content = fs::read_to_string(&dest).unwrap();
        let model = Model::parse(&dest, &content).unwrap();
        // The config's bounds replace the template's (-2, 2) ...
        assert_eq!(model.thetas[3].lower, Some(0.0), "{content}");
        assert_eq!(model.thetas[3].upper, Some(5.0), "{content}");
        // ... and the out-of-bounds reference estimate is dropped for the
        // plan's release value rather than written as an illegal init.
        assert!((model.thetas[3].init - 0.4).abs() < 1e-12, "{content}");
        // A lower bound alone is spelled `(0, init)`.
        assert_eq!(model.thetas[4].lower, Some(0.0), "{content}");
        assert_eq!(model.thetas[4].upper, None, "{content}");
        assert!(content.contains("(0, 0.1)"), "{content}");
    }

    #[test]
    fn a_bounded_candidate_theta_keeps_its_bounds_when_released() {
        let dir = tempfile::tempdir().unwrap();
        let bounded = crate::scm::plan::tests::TEMPLATE.replace(
            "$THETA (0 FIX)   ; WT_CL cov",
            "$THETA (-2, 0.4, 2)   ; WT_CL cov",
        );
        let template = crate::scm::plan::tests::write_template_content(dir.path(), &bounded);

        let dest = dir.path().join("scm/1001/forward_round1/1001_wt_cl.mod");
        write_scm_model(
            &template,
            &dest,
            &cands(&[(4, 0.4), (5, 0.1), (6, 0.1)]),
            &[4],
            None,
            true,
            "SCM test",
            None,
            false,
        )
        .unwrap();
        let content = fs::read_to_string(&dest).unwrap();
        let model = Model::parse(&dest, &content).unwrap();
        assert_eq!(model.thetas[3].lower, Some(-2.0), "{content}");
        assert_eq!(model.thetas[3].upper, Some(2.0), "{content}");
        assert!((model.thetas[3].init - 0.4).abs() < 1e-12, "{content}");
    }

    #[test]
    fn missing_reference_ext_degrades_to_cold_start() {
        let dir = tempfile::tempdir().unwrap();
        let template = write_template(dir.path());
        let dest = dir.path().join("scm/1001/forward_round1/1001_wt_cl.mod");
        write_scm_model(
            &template,
            &dest,
            &cands(&[(4, 0.1), (5, 0.1), (6, 0.1)]),
            &[4],
            Some(&dir.path().join("nope.ext")),
            true,
            "SCM test",
            None,
            false,
        )
        .unwrap();
        let content = fs::read_to_string(&dest).unwrap();
        let model = Model::parse(&dest, &content).unwrap();
        assert!((model.thetas[3].init - 0.1).abs() < 1e-12, "{content}");
    }

    #[test]
    fn run_summary_write_is_best_effort() {
        let dir = tempfile::tempdir().unwrap();
        let template = write_template(dir.path());
        // No run output exists at all — must warn, not panic or fail.
        write_run_summary(&template);
        assert!(!run_dir_for(&template).join("pharos_summary.json").exists());
    }

    #[test]
    fn round_entries_cover_the_right_sets() {
        let dir = tempfile::tempdir().unwrap();
        let template = write_template(dir.path());
        let plan = build_plan(
            &template,
            &names(&["WT_CL", "CRCL_CL", "WT_V"]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap()
        .plan;

        // Forward, nothing retained: 3 entries, each releasing 1 theta
        let entries = forward_entries(&plan, &[]);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].candidate, "WT_CL");
        assert_eq!(entries[0].released, vec![4]);
        assert!(entries.iter().all(|e| e.df == 1));

        // Forward with WT_CL retained: 2 entries, each releasing 2 thetas
        // but testing (df) only the 1 theta beyond the retained set
        let retained = vec!["WT_CL".to_string()];
        let entries = forward_entries(&plan, &retained);
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.released.len() == 2));
        assert!(entries.iter().all(|e| e.released.contains(&4)));
        assert!(entries.iter().all(|e| e.df == 1));

        // Backward from {WT_CL, WT_V}: 2 entries, each releasing the other
        // and re-fixing (df) exactly 1 theta
        let retained = vec!["WT_CL".to_string(), "WT_V".to_string()];
        let entries = backward_entries(&plan, &retained);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].candidate, "WT_CL");
        assert_eq!(entries[0].released, vec![6]);
        assert_eq!(entries[1].candidate, "WT_V");
        assert_eq!(entries[1].released, vec![4]);
        assert!(entries.iter().all(|e| e.df == 1));
    }

    #[test]
    fn outcome_of_missing_run_is_unusable() {
        let dir = tempfile::tempdir().unwrap();
        let template = write_template(dir.path());
        let outcome = read_fit_outcome(&template).unwrap();
        assert!(!outcome.usable());
        assert_eq!(outcome.label(), "never started");
    }
}
