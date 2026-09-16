use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use fs_err as fs;
use nonmem_parser::Model;

use super::project::RunSettings;
use super::state::{AttemptRecord, CandidateRecord, CandidateStatus, RoundRecord, ScmState};
use super::{Candidate, ScmOptions, ScmPlan, ThetaSpec, sanitize_name};
use crate::copy::{CopyOptions, UpdateType, copy_model, derive_model, write_model_copy};
use crate::output_files::lst::{LstSummary, RunHeuristics};
use crate::output_files::{Summary, get_summary, resolve_estimation_files};
use crate::run::metadata::{RUN_END_FILENAME, RUN_START_FILENAME};
use crate::run::signal_wrapper::TERMINATION_FILENAME;
use crate::{ModelLayout, update};

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

/// Where a model's run output lands: the project's `output_dir` template
/// rendered for the model, or pharos' default layout (a subfolder next to
/// the model, named after it) when the project sets none. Exactly where the
/// runner puts it, since both go through [`ModelLayout::resolve_output_dir`].
///
/// A template carrying a timestamp renders a fresh name on every call, so
/// for those the name resolved now is not the one an earlier run was written
/// under. The run-start files say where that run actually landed; the freshly
/// rendered name is the fallback, and is the right answer for a model that
/// has not run yet.
pub fn run_dir_for(model_path: &Path, settings: &RunSettings) -> Result<PathBuf> {
    run_dir_in(model_path, settings, config::find_config_dir()?.as_deref())
}

/// [`run_dir_for`] against an explicit project root, which anchors the
/// run-start lookup. `None` means there is no pharos.toml to anchor it, so
/// the rendered name is all there is to go on.
pub(crate) fn run_dir_in(
    model_path: &Path,
    settings: &RunSettings,
    project_root: Option<&Path>,
) -> Result<PathBuf> {
    let layout = ModelLayout::for_model_path(model_path)?;
    let resolved = layout.resolve_output_dir(settings.output_dir.as_deref())?;
    if resolved.exists() || !renders_a_new_name_each_time(settings.output_dir.as_deref()) {
        return Ok(resolved);
    }
    Ok(discovered_run_dir(model_path, project_root)?.unwrap_or(resolved))
}

/// Whether an `output_dir` template renders a different name on every call.
/// `render_output_dir_template` offers `{{timestamp}}` and
/// `{{unix_timestamp}}`; every other value it exposes is stable for a model.
fn renders_a_new_name_each_time(template: Option<&str>) -> bool {
    template.is_some_and(|t| t.contains("timestamp"))
}

/// Where a run of `model_path` was actually written, per the run-start file
/// the runner left in it. `None` when the model has not run, or when it sits
/// outside the project root the run-start paths are recorded against. Errors
/// when the model has run more than once under distinct timestamps, since
/// nothing here can say which of them is meant.
fn discovered_run_dir(model_path: &Path, project_root: Option<&Path>) -> Result<Option<PathBuf>> {
    let (Some(root), Ok(model_path)) = (project_root, fs::canonicalize(model_path)) else {
        return Ok(None);
    };
    let Ok(root) = fs::canonicalize(root) else {
        return Ok(None);
    };
    if !model_path.starts_with(&root) {
        return Ok(None);
    }
    ModelLayout::for_model_path(model_path)?.discover_output_dir(&root)
}

/// Whether a model's run has finished, one way or another: pharos wrote its
/// RUN_END marker, or the signal wrapper recorded a termination. A model
/// whose run directory cannot even be named has not finished.
pub fn run_finished(model: &Path, settings: &RunSettings) -> bool {
    run_dir_for(model, settings)
        .map(|run_dir| {
            run_dir.join(RUN_END_FILENAME).exists() || run_dir.join(TERMINATION_FILENAME).exists()
        })
        .unwrap_or(false)
}

/// The `.ext` file a run produced, honoring `$EST FILE=` overrides.
pub fn ext_path_for(model_path: &Path, settings: &RunSettings) -> Result<PathBuf> {
    ext_path_in(model_path, &run_dir_for(model_path, settings)?)
}

/// [`ext_path_for`] for a run directory already in hand, which is what a
/// caller that has just placed the run has. Parsing the control stream is
/// the expensive part, so the run directory is never resolved twice.
pub(crate) fn ext_path_in(model_path: &Path, run_dir: &Path) -> Result<PathBuf> {
    let layout = ModelLayout::for_model_path(model_path)?;
    let default = layout.output_file(run_dir, "ext");
    let parsed = fs::read_to_string(model_path)
        .ok()
        .and_then(|s| Model::parse(model_path, &s).ok());
    Ok(match parsed {
        Some(model) => resolve_estimation_files(&model, run_dir, &default)
            .last()
            .cloned()
            .unwrap_or(default),
        None => default,
    })
}

/// The `pharos nonmem summary` of a finished run: the `pharos_summary.json`
/// the driver wrote into the run directory when there is one, else built
/// from the run's output the way `pharos nonmem summary` builds it.
pub fn run_summary(run_dir: &Path, settings: &RunSettings) -> Result<Summary> {
    let path = run_dir.join(super::RUN_SUMMARY_FILENAME);
    if path.exists() {
        let content = fs::read_to_string(&path)?;
        return serde_json::from_str(&content)
            .with_context(|| format!("failed to parse {}", path.display()));
    }
    get_summary(run_dir, settings.comment_type, false)
}

/// The copy options every SCM-generated model is written with: no estimate
/// updates unless the caller adds them, metadata (description, based_on,
/// tags) only when `with_metadata` is set.
fn scm_copy_options(
    description: &str,
    based_on: Option<&str>,
    with_metadata: bool,
    tags: &[&str],
) -> CopyOptions {
    CopyOptions {
        update: vec![UpdateType::None],
        description: description.to_string(),
        based_on: match (with_metadata, based_on) {
            (true, Some(b)) => vec![b.to_string()],
            _ => vec![],
        },
        tags: tags.iter().map(|t| t.to_string()).collect(),
        no_metadata: !with_metadata,
        ..Default::default()
    }
}

/// The file names `copy_model` renames output paths between.
fn file_names(from: &Path, dest: &Path) -> Result<(String, String)> {
    let name = |p: &Path, what: &str| -> Result<String> {
        Ok(p.file_name()
            .with_context(|| format!("{what} model has no file name"))?
            .to_string_lossy()
            .to_string())
    };
    Ok((name(from, "source")?, name(dest, "destination")?))
}

/// The `$THETA` spec a parsed record carries, as authored. Taken field by
/// field because the parser's theta type is not nameable from here.
fn authored_spec(lower: Option<f64>, init: f64, upper: Option<f64>, fixed: bool) -> ThetaSpec {
    ThetaSpec {
        lower,
        init,
        upper,
        fixed,
    }
}

/// Write one SCM model: a copy of the initial model in which the `released`
/// covariate thetas (1-based) are free and every other `candidates` theta is
/// pinned at `(fixed FIX)` — the effect held out of the model, `fixed` being 0
/// for the usual forms and 1 for a fold-change form — with the
/// `$COVARIANCE` record added or removed per `cov_step`.
///
/// Pinning is what lets the initial model carry a candidate as an ordinary free
/// theta with a real initial estimate: the config names the candidates, and
/// every generated model fixes the ones it is not testing.
///
/// A released theta starts from its estimate in `reference_ext` when it was
/// free there too (a held-out theta reports exactly its held-out value), and
/// otherwise from the candidate's own [`Candidate::initial`], which the plan
/// resolved from the config and the initial model. Bounds the initial model gave the
/// theta are kept.
///
/// With a `reference_ext`, the model also warm-starts every other free
/// parameter (base thetas, omegas, sigmas) from the reference fit. Without
/// one (the reference fit itself), everything starts from the initial model's
/// initial estimates.
///
/// The whole derivation happens on one parsed model — copy, warm start,
/// spec rewrite — and the result is written once.
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
    let template_model = Model::parse(template, &fs::read_to_string(template)?)?;
    let options = scm_copy_options(description, based_on, with_metadata, &["scm"]);
    let (from_name, dest_name) = file_names(template, dest)?;
    let mut model = derive_model(
        &template_model,
        template,
        dest,
        &from_name,
        &dest_name,
        &options,
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
                    model.update_initial_estimates(&estimates, None, None, &[]);
                    reference_estimates = estimates;
                }
                Err(e) => log::warn!(
                    "could not read reference estimates from {}: {e:#}",
                    ext.display()
                ),
            }
        } else {
            log::warn!(
                "reference output {} not found; {} starts from the initial model's own estimates",
                ext.display(),
                dest.display()
            );
        }
    }

    let released_set: BTreeSet<usize> = released.iter().copied().collect();
    if let Some(unknown) = released_set
        .iter()
        .find(|t| !candidates.iter().any(|c| c.theta == **t))
    {
        bail!("THETA({unknown}) was released but is not a candidate in the plan");
    }

    // The candidate theta specs. The initial model's own specs decide the
    // `(fixed FIX)` shape and supply the bounds a plan left out: how the
    // effect was authored, not whatever the reference fit left behind.
    let mut specs: BTreeMap<usize, String> = BTreeMap::new();
    for candidate in candidates {
        let theta_num = candidate.theta;
        let Some(template_theta) = template_model.thetas.get(theta_num.wrapping_sub(1)) else {
            bail!(
                "THETA({theta_num}) out of range: the initial model has {} thetas",
                template_model.thetas.len()
            );
        };

        if !released_set.contains(&theta_num) {
            // Held out of this model. A candidate the initial model already writes
            // `(fixed FIX)` is left exactly as authored; anything else is pinned.
            let held_out = candidate.held_out_spec();
            if authored_spec(
                template_theta.lower,
                template_theta.init,
                template_theta.upper,
                template_theta.fixed,
            ) != held_out
            {
                specs.insert(theta_num - 1, held_out.to_string());
            }
            continue;
        }

        // Free in the reference fit -> continue from its estimate; a theta
        // held out there reports exactly its held-out value, so start it where
        // the plan says. An estimate that does not sit strictly inside the
        // candidate's bounds is no use as a warm start — NM-TRAN would
        // reject it — so fall back to the plan's initial estimate, which plan
        // time already checked against the bounds.
        // A plan built by this version already carries the initial model's own
        // bounds wherever the config gave none; the fallback to the initial model
        // keeps a plan.json written before candidates had bounds behaving
        // exactly as it did.
        let mut spec = candidate.released_spec();
        let template_spec = authored_spec(
            template_theta.lower,
            template_theta.init,
            template_theta.upper,
            template_theta.fixed,
        );
        spec.lower = spec.lower.or(template_spec.lower);
        spec.upper = spec.upper.or(template_spec.upper);
        if let Some(&est) = reference_estimates.get(&format!("THETA{theta_num}"))
            && est.is_finite()
            && est != candidate.fixed
            && spec.contains(est)
        {
            spec.init = est;
        }
        specs.insert(theta_num - 1, spec.to_string());
    }

    let mut replacements = model.theta_spec_replacements(&specs)?;
    if !cov_step {
        replacements.extend(model.covariance_removal_replacements());
    }
    let mut content = model.render_with_replacements(&replacements);
    // The token renderer can only rewrite tokens that exist, so a model with
    // no $COVARIANCE record gets one appended to the rendered text.
    if cov_step && model.covariance.is_none() {
        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str("$COVARIANCE\n");
    }
    write_model_copy(template, dest, &content, &options)
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
    settings: &RunSettings,
) -> Result<()> {
    let (from_name, dest_name) = file_names(prev_model, dest)?;
    let mut options = scm_copy_options(description, based_on, with_metadata, &["scm", "retry"]);

    let ext = ext_path_for(prev_model, settings)?;
    if ext.exists() {
        // A run that died before NONMEM wrote the .ext header leaves a file
        // that parses to no tables at all. That is a worse starting point,
        // never a reason to abandon the whole SCM process: fall back to the
        // model's own initial estimates and let the candidate conclude on
        // its own merits once its retries run out.
        let carry = CopyOptions {
            update: vec![UpdateType::All],
            ext_path: Some(ext.clone()),
            allow_partial: true,
            ..options.clone()
        };
        match copy_model(prev_model, dest, &from_name, &dest_name, &carry) {
            Ok(()) => return Ok(()),
            Err(e) => log::warn!(
                "could not carry estimates from {} into {}: {e:#}; \
                 retrying with unchanged initial estimates",
                ext.display(),
                dest.display()
            ),
        }
    } else {
        log::warn!(
            "no .ext output found for {}; retrying with unchanged initial estimates",
            prev_model.display()
        );
    }
    options.update = vec![UpdateType::None];
    copy_model(prev_model, dest, &from_name, &dest_name, &options)
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
/// and the written summary all describe the same SCM process.
pub fn reconcile_state_with_disk(
    state: &mut ScmState,
    out_dir: &Path,
    options: &ScmOptions,
    settings: &RunSettings,
) -> Vec<String> {
    let mut running = Vec::new();
    for round in &mut state.rounds {
        if round.complete {
            continue;
        }
        reconcile_round_with_disk(round, out_dir, options, settings, &mut running);
    }
    running
}

/// [`reconcile_state_with_disk`] for a single open round, appending the
/// models still running to `running`.
pub fn reconcile_round_with_disk(
    round: &mut RoundRecord,
    out_dir: &Path,
    options: &ScmOptions,
    settings: &RunSettings,
    running: &mut Vec<String>,
) {
    for cand in &mut round.candidates {
        // Only a dispatched candidate has a run to look at; a concluded one
        // already carries the driver's own reading of it.
        if cand.status != CandidateStatus::Running || cand.model.is_empty() {
            continue;
        }
        let model_path = out_dir.join(&cand.model);
        if !run_finished(&model_path, settings) {
            if run_dir_for(&model_path, settings).is_ok_and(|d| d.join(RUN_START_FILENAME).exists())
            {
                running.push(cand.model.clone());
            }
            continue;
        }
        match read_fit_outcome(&model_path, settings) {
            Ok(outcome) => {
                let rel = cand.model.clone();
                record_attempt(cand, rel, &outcome);
            }
            // Output we cannot read is no evidence either way; leave the
            // candidate as the driver last wrote it.
            Err(e) => log::warn!("failed to read outcome of {}: {e}", model_path.display()),
        }
    }
    // Score whatever just concluded, so a reader that beats the driver to a
    // finished run reports the same numbers the driver will write.
    round.score(options);
}

/// Read the outcome of a model's run from its output directory.
///
/// A finished run is read through its `pharos nonmem summary` (see
/// [`run_summary`]), the same record `pharos nonmem summary` prints; a run
/// whose output the summary cannot make sense of (killed before the .ext
/// header, say) falls back to the listing alone, so its verdict is still
/// reported rather than lost.
pub fn read_fit_outcome(model_path: &Path, settings: &RunSettings) -> Result<FitOutcome> {
    let layout = ModelLayout::for_model_path(model_path)?;
    let run_dir = layout.resolve_output_dir(settings.output_dir.as_deref())?;

    let started = run_dir.join(RUN_START_FILENAME).exists();
    let finished = run_dir.join(RUN_END_FILENAME).exists();
    let terminated = run_dir.join(TERMINATION_FILENAME).exists();

    let mut outcome = FitOutcome {
        started,
        finished,
        terminated,
        ofv: None,
        minimization_terminated: None,
        program_aborted: None,
        heuristics: vec![],
    };

    if finished && !terminated {
        match run_summary(&run_dir, settings) {
            Ok(summary) => {
                outcome.ofv = summary
                    .minimization_results
                    .last()
                    .and_then(|m| m.ofv)
                    .filter(|v| v.is_finite());
                outcome.apply_lst(&summary.lst);
                return Ok(outcome);
            }
            Err(e) => log::warn!("could not summarize {}: {e:#}", run_dir.display()),
        }
    }
    let lst_path = layout.output_file(&run_dir, "lst");
    if lst_path.exists() {
        match LstSummary::from_run(&lst_path) {
            Ok(lst) => outcome.apply_lst(&lst),
            Err(e) => log::warn!("failed to parse {}: {e}", lst_path.display()),
        }
    }
    Ok(outcome)
}

/// Human labels of every heuristic check that fired, in the order an SCM
/// report lists them. The two that decide whether a fit is usable at all
/// come first.
fn fired_labels(h: &RunHeuristics) -> Vec<String> {
    [
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
    .collect()
}

impl FitOutcome {
    /// Take the listing's verdicts: the two that decide usability, and the
    /// labels of every heuristic check that fired.
    fn apply_lst(&mut self, lst: &LstSummary) {
        let h = &lst.run_heuristics;
        self.minimization_terminated = h.minimization_terminated;
        self.program_aborted = h.program_aborted;
        self.heuristics = fired_labels(h);
    }
}

/// Best-effort `pharos nonmem summary` of a finished run, written as
/// `pharos_summary.json` into the run directory (the same JSON `pharos
/// nonmem summary --json` prints, parameters named per the project's
/// comment type). Left alone when it is already there. Failures are logged,
/// never fatal: the summary is a record, not a scoring input.
pub fn write_run_summary(model_path: &Path, settings: &RunSettings) {
    let run_dir = match run_dir_for(model_path, settings) {
        Ok(dir) => dir,
        Err(e) => {
            log::warn!("could not place the run of {}: {e:#}", model_path.display());
            return;
        }
    };
    let path = run_dir.join(super::RUN_SUMMARY_FILENAME);
    if path.exists() {
        return;
    }
    let summary = match get_summary(&run_dir, settings.comment_type, false) {
        Ok(summary) => summary,
        Err(e) => {
            log::warn!("could not summarize run {}: {e:#}", run_dir.display());
            return;
        }
    };
    if let Err(e) = utils::write_json_to_file(&summary.finite(), &path) {
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

    /// The initial model's three candidate effects, released at 0.1 unless the
    /// test's initial model gives the theta an initial estimate of its own, and
    /// held out at 0.
    fn cands(inits: &[(usize, f64)]) -> Vec<Candidate> {
        inits
            .iter()
            .map(|&(theta, initial)| Candidate {
                name: format!("THETA{theta}"),
                theta,
                initial,
                fixed: 0.0,
                ..Default::default()
            })
            .collect()
    }

    /// A fold-change effect on THETA(4): FIXED at 1, initial estimate 1.3.
    fn fold_change_cands() -> Vec<Candidate> {
        let mut c = cands(&[(4, 1.3), (5, 0.1), (6, 0.1)]);
        c[0].fixed = 1.0;
        c
    }

    /// The shape a scientist actually hands over: the candidate thetas carry
    /// ordinary initial estimates and nothing is written `(0 FIX)`. The
    /// naming in `$THETA` is the whole specification — planning takes each
    /// effect's initial estimate from the model's own estimate, and the SCM
    /// process is what pins the effect when it is held out.
    #[test]
    fn candidates_authored_at_real_estimates_plan_and_write_end_to_end() {
        let dir = tempfile::tempdir().unwrap();
        let authored = crate::scm::plan::tests::TEMPLATE
            .replace(
                "$THETA (0 FIX)   ; WT_CL cov",
                "$THETA (0, 0.35)   ; WT_CL cov",
            )
            .replace(
                "$THETA (0 FIX)   ; CRCL_CL cov",
                "$THETA (0, 0.42)   ; CRCL_CL cov",
            )
            .replace(
                "$THETA (0 FIX)   ; WT_V cov",
                "$THETA (0, 0.8)   ; WT_V cov",
            );
        let template = crate::scm::plan::tests::write_template_content(dir.path(), &authored);

        let built = build_plan(
            &template,
            &names(&["WT_CL", "CRCL_CL", "WT_V"]),
            None,
            crate::scm::plan::tests::opts_cov_on(),
            "test",
        )
        .unwrap();
        assert!(built.warnings.is_empty(), "warnings: {:?}", built.warnings);

        // Each effect's initial estimate is the estimate the model carries.
        let got: Vec<(&str, usize, f64)> = built
            .plan
            .candidates
            .iter()
            .map(|c| (c.name.as_str(), c.theta, c.initial))
            .collect();
        assert_eq!(
            got,
            vec![("WT_CL", 4, 0.35), ("CRCL_CL", 5, 0.42), ("WT_V", 6, 0.8)]
        );

        // Round 1 releases WT_CL and holds the other two out at `(0 FIX)`,
        // which the initial model never wrote for them.
        let round1 = dir.path().join("scm/1001/forward_round1/1001_wt_cl.mod");
        write_scm_model(
            &template,
            &round1,
            &built.plan.candidates,
            &[4],
            None,
            true,
            "SCM test",
            None,
            false,
        )
        .unwrap();
        let content = fs::read_to_string(&round1).unwrap();
        let model = Model::parse(&round1, &content).unwrap();
        assert!(!model.thetas[3].fixed, "{content}");
        assert!((model.thetas[3].init - 0.35).abs() < 1e-12, "{content}");
        for idx in [4, 5] {
            assert!(model.thetas[idx].fixed, "theta {idx}: {content}");
            assert!(
                model.thetas[idx].init.abs() < 1e-12,
                "theta {idx}: {content}"
            );
        }
    }

    /// A held-out fold-change effect is pinned at `(1 FIX)`, not `(0 FIX)`
    /// (which would zero the parameter for every SEX = 1 subject); released,
    /// it starts at its own initial. Warm-starting reads an estimate equal
    /// to the held-out value as "held out in the reference".
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
        // $DATA rebased to still point at the initial model's dataset
        assert!(content.contains("../../../data.csv"), "{content}");
        // $COVARIANCE retained (initial model has one, cov_step on)
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
        // The newly freed covariate starts fresh at the plan's initial estimate
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

        // Released: starts where the plan resolved it from the initial model.
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
    /// initial model's own, and a reference estimate that falls outside them is
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
        // The config's bounds replace the initial model's (-2, 2) ...
        assert_eq!(model.thetas[3].lower, Some(0.0), "{content}");
        assert_eq!(model.thetas[3].upper, Some(5.0), "{content}");
        // ... and the out-of-bounds reference estimate is dropped for the
        // plan's initial estimate rather than written as an illegal init.
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
        let settings = RunSettings::default();
        write_run_summary(&template, &settings);
        assert!(
            !run_dir_for(&template, &settings)
                .unwrap()
                .join("pharos_summary.json")
                .exists()
        );
    }

    /// The run directory follows the project's `output_dir` template, so a
    /// project that files its runs elsewhere is read where they actually
    /// are.
    #[test]
    fn run_dir_follows_the_projects_output_dir_template() {
        let dir = tempfile::tempdir().unwrap();
        let template = write_template(dir.path());
        let settings = RunSettings {
            output_dir: Some("runs/{{name}}_fit".to_string()),
            ..Default::default()
        };
        assert_eq!(
            run_dir_for(&template, &settings).unwrap(),
            dir.path().join("runs").join("1001_fit")
        );
        assert_eq!(
            run_dir_for(&template, &RunSettings::default()).unwrap(),
            dir.path().join("1001")
        );
        assert!(!run_finished(&template, &settings));
    }

    /// A timestamped `output_dir` renders a new name every call, so a run
    /// that already happened must be found by its run-start file rather than
    /// by re-rendering the template.
    #[test]
    fn run_dir_finds_a_timestamped_run_where_it_was_actually_written() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let template = write_template(root);
        let settings = RunSettings {
            output_dir: Some("{{name}}-{{timestamp}}".to_string()),
            ..Default::default()
        };

        // Nothing has run yet: the freshly rendered name is the answer.
        let planned = run_dir_in(&template, &settings, Some(root)).unwrap();
        assert!(
            planned
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("1001-"),
            "{planned:?}"
        );

        // The run lands in a directory stamped at its own start time.
        let actual = root.join("1001-2020-01-01T00_00_00+0000");
        fs::create_dir_all(&actual).unwrap();
        fs::write(
            actual.join(RUN_START_FILENAME),
            serde_json::json!({
                "start": "2020-01-01T00:00:00Z",
                "model_name": "1001",
                "model_path": "1001.mod",
                "dataset_path": "data.csv",
                "dataset_canonical_path": root.join("data.csv"),
                "dataset_hashes": {"blake3": ""},
                "model_hashes": {"blake3": ""},
            })
            .to_string(),
        )
        .unwrap();

        assert_eq!(
            run_dir_in(&template, &settings, Some(root)).unwrap(),
            actual
        );

        // Without a project root to anchor the lookup there is nothing to go
        // on but the rendered name, which is not where the run is.
        assert_ne!(run_dir_in(&template, &settings, None).unwrap(), actual);
    }

    /// A template with no timestamp resolves the same name every call, so it
    /// never pays for the run-start scan.
    #[test]
    fn a_stable_template_needs_no_discovery() {
        let dir = tempfile::tempdir().unwrap();
        let template = write_template(dir.path());
        let settings = RunSettings {
            output_dir: Some("runs/{{name}}_fit".to_string()),
            ..Default::default()
        };
        assert_eq!(
            run_dir_in(&template, &settings, None).unwrap(),
            dir.path().join("runs").join("1001_fit")
        );
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
        let outcome = read_fit_outcome(&template, &RunSettings::default()).unwrap();
        assert!(!outcome.usable());
        assert_eq!(outcome.label(), "never started");
    }
}
