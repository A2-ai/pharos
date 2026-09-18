use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use config::NonmemConfig;
use fs_err as fs;
use nonmem_parser::Model;

use super::state::{AttemptRecord, CandidateRecord, CandidateStatus, RoundRecord, ScmState};
use super::{Candidate, ScmOptions, ScmPlan, ThetaSpec, sanitize_name};
use crate::copy::{CopyOptions, UpdateType, copy_model, derive_model, write_model_copy};
use crate::output_files::lst::{LstSummary, RunHeuristics};
use crate::output_files::{Summary, get_summary, resolve_estimation_files};
use crate::run::metadata::{RUN_END_FILENAME, RUN_START_FILENAME};
use crate::run::signal_wrapper::TERMINATION_FILENAME;
use crate::{ModelLayout, update};

/// How far a retry perturbs the estimates it starts from.
pub const RETRY_JITTER: f64 = 0.05;

/// A jitter seed derived from the retry model's own file name, which already
/// encodes the candidate and the attempt number: successive attempts jitter
/// differently, and re-running or resuming a process reproduces them exactly.
///
/// FNV-1a rather than `DefaultHasher`, whose output is not stable across Rust
/// releases — the written models are snapshotted.
fn jitter_seed(dest_name: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in dest_name.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Model file name (no extension) for a candidate attempt
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

/// Where a model's run output lands
pub fn run_dir_for(model_path: &Path, settings: &NonmemConfig) -> Result<PathBuf> {
    ModelLayout::for_model_path(model_path)?.resolve_output_dir(settings.output_dir.as_deref())
}

/// Whether a model's run has finished, one way or another
pub fn run_finished(model: &Path, settings: &NonmemConfig) -> bool {
    run_dir_for(model, settings)
        .map(|run_dir| {
            run_dir.join(RUN_END_FILENAME).exists() || run_dir.join(TERMINATION_FILENAME).exists()
        })
        .unwrap_or(false)
}

/// The `.ext` file a run produced, honoring `$EST FILE=` overrides.
pub fn ext_path_for(model_path: &Path, settings: &NonmemConfig) -> Result<PathBuf> {
    ext_path_in(model_path, &run_dir_for(model_path, settings)?)
}

/// [`ext_path_for`] for a run directory already in hand
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
/// the driver wrote into the run directory when there is one
pub fn run_summary(run_dir: &Path, settings: &NonmemConfig) -> Result<Summary> {
    let path = run_dir.join(super::RUN_SUMMARY_FILENAME);
    if path.exists() {
        let content = fs::read_to_string(&path)?;
        return serde_json::from_str(&content)
            .with_context(|| format!("failed to parse {}", path.display()));
    }
    get_summary(run_dir, settings.comments.r#type, false)
}

/// The copy options every SCM-generated model is written with
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

/// Writes the models of one SCM process
pub struct ModelWriter<'a> {
    pub template: &'a Path,
    pub candidates: &'a [Candidate],
    pub with_metadata: bool,
}

impl ModelWriter<'_> {
    /// Write one SCM model: a copy of the initial model in which the `released`
    /// covariate thetas (1-based) are free and other candidate thetas are pinned at `(fixed FIX)`
    ///
    /// The whole derivation happens on one parsed model and the result is written once.
    pub fn write(
        &self,
        dest: &Path,
        released: &[usize],
        reference_ext: Option<&Path>,
        cov_step: bool,
        description: &str,
        based_on: Option<&str>,
    ) -> Result<()> {
        let (template, candidates, with_metadata) =
            (self.template, self.candidates, self.with_metadata);
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
        // thetas are left untouched by the updater
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

        // The candidate theta specs
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
                let held_out = candidate.held_out_spec();
                if ThetaSpec::from(template_theta) != held_out {
                    specs.insert(theta_num - 1, held_out.to_string());
                }
                continue;
            }

            // Free in the reference fit -> continue from its estimate; a theta
            // held out there reports exactly its held-out value, so start it where
            // the plan says
            let mut spec = candidate.released_spec();
            let template_spec = ThetaSpec::from(template_theta);
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
    /// it finished, the last iteration otherwise), jittered by
    /// [`RETRY_JITTER`] so an attempt that settled on a boundary starts the
    /// next one off it. Thetas only; fixed thetas and bounds are respected by
    /// the jitterer.
    pub fn retry(
        &self,
        prev_model: &Path,
        dest: &Path,
        description: &str,
        based_on: Option<&str>,
        settings: &NonmemConfig,
    ) -> Result<()> {
        let (from_name, dest_name) = file_names(prev_model, dest)?;
        let mut options =
            scm_copy_options(description, based_on, self.with_metadata, &["scm", "retry"]);

        // Same jitter on both paths below: whether or not the estimates can be
        // carried over, re-running byte-identical initials is never worth an attempt.
        options.jitter = Some(RETRY_JITTER);
        options.seed = Some(jitter_seed(&dest_name));

        let ext = ext_path_for(prev_model, settings)?;
        if ext.exists() {
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
                 retrying from jittered initial estimates",
                    ext.display(),
                    dest.display()
                ),
            }
        } else {
            log::warn!(
                "no .ext output found for {}; retrying from jittered initial estimates",
                prev_model.display()
            );
        }
        options.update = vec![UpdateType::None];
        copy_model(prev_model, dest, &from_name, &dest_name, &options)
    }
}

/// Everything the driver needs to know about how a fit went.
#[derive(Debug, Clone, PartialEq)]
pub struct FitOutcome {
    pub started: bool,
    pub finished: bool,
    pub terminated: bool,
    pub ofv: Option<f64>,
    pub minimization_terminated: Option<bool>,
    pub program_aborted: Option<bool>,
    pub heuristics: Vec<String>,
}

impl FitOutcome {
    /// A fit is usable for scoring when it ran to completion, was not killed,
    /// produced an OFV, and neither terminated minimization nor aborted the estimation.
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
/// Every reader of a live SCM process goes through this, so status, a round view
/// and the written summary all describe the same SCM process.
pub fn reconcile_state_with_disk(
    state: &mut ScmState,
    out_dir: &Path,
    options: &ScmOptions,
    settings: &NonmemConfig,
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
    settings: &NonmemConfig,
    running: &mut Vec<String>,
) {
    for cand in &mut round.candidates {
        // Only a dispatched candidate has a run to look at
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
            Err(e) => log::warn!("failed to read outcome of {}: {e}", model_path.display()),
        }
    }
    // Score whatever just concluded, so a reader that beats the driver to a
    // finished run reports the same numbers the driver will write.
    round.score(options);
}

/// Read the outcome of a model's run from its output directory.
pub fn read_fit_outcome(model_path: &Path, settings: &NonmemConfig) -> Result<FitOutcome> {
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
    fn apply_lst(&mut self, lst: &LstSummary) {
        let h = &lst.run_heuristics;
        self.minimization_terminated = h.minimization_terminated;
        self.program_aborted = h.program_aborted;
        self.heuristics = fired_labels(h);
    }
}

/// Best-effort `pharos nonmem summary` of a finished run, written as
/// `pharos_summary.json` into the run directory
pub fn write_run_summary(model_path: &Path, settings: &NonmemConfig) {
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
    let summary = match get_summary(&run_dir, settings.comments.r#type, false) {
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
    pub candidate: String,
    /// "add X" / "drop X" / "fit base model" / "fit full model".
    pub action: String,
    /// 1-based theta numbers released in this model.
    pub released: Vec<usize>,
    /// LRT degrees of freedom: how many thetas this model releases (forward)
    /// or re-fixes (backward) relative to the round's reference
    pub df: usize,
}

/// Build the entries for a forward round
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

/// Build the entries for a backward round
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
    use crate::scm::test_support::{TEMPLATE, write_template, write_template_content};
    use crate::scm::{Covariates, ScmOptions, build_plan};

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

    /// A held-out fold-change effect is pinned at `(1 FIX)`, not `(0 FIX)`
    /// (which would zero the parameter for every SEX = 1 subject); released,
    /// it starts at its own initial. Warm-starting reads an estimate equal
    /// to the held-out value as "held out in the reference".
    #[test]
    fn a_fold_change_candidate_is_held_out_at_one() {
        let dir = tempfile::tempdir().unwrap();
        let fold = TEMPLATE
            .replace("WT_CL = (WT/70)**THETA(4)", "WT_CL = THETA(4)**(WT/70)")
            .replace("$THETA (0 FIX)   ; WT_CL cov", "$THETA 1.3   ; WT_CL cov");
        let template = write_template_content(dir.path(), &fold);

        // held out: pinned at 1
        let held = dir.path().join("scm/1001/forward_round1/1001_crcl_cl.mod");
        ModelWriter {
            template: &template,
            candidates: &fold_change_cands(),
            with_metadata: false,
        }
        .write(&held, &[5], None, true, "SCM test", None)
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
        ModelWriter {
            template: &template,
            candidates: &fold_change_cands(),
            with_metadata: false,
        }
        .write(&released, &[4, 5], Some(&ext_path), true, "SCM test", None)
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
        assert_eq!(scm_model_name("1001", "CRCL/CL", 1, 0), "1001_crcl_cl");
        assert_eq!(scm_model_name("1001", "WT_CL", 2, 0), "1001_wt_cl_try2");
        assert_eq!(scm_model_name("1001", "WT_CL", 1, 1), "1001_wt_cl_refit2");
        assert_eq!(
            scm_model_name("1001", "WT_CL", 2, 1),
            "1001_wt_cl_refit2_try2"
        );
    }

    /// Bounds the config set reach the generated model, override the
    /// initial model's own, and a reference estimate that falls outside them is
    /// not used as a warm start.
    #[test]
    fn plan_bounds_override_the_templates_and_gate_the_warm_start() {
        let dir = tempfile::tempdir().unwrap();
        let bounded = TEMPLATE.replace(
            "$THETA (0 FIX)   ; WT_CL cov",
            "$THETA (-2, 0.4, 2)   ; WT_CL cov",
        );
        let template = write_template_content(dir.path(), &bounded);

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
        ModelWriter {
            template: &template,
            candidates: &candidates,
            with_metadata: false,
        }
        .write(&dest, &[4, 5], Some(&ext_path), true, "SCM test", None)
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

        // A reference .ext that does not exist degrades to a cold start.
        let cold = dir.path().join("scm/1001/forward_round3/1001_wt_cl.mod");
        ModelWriter {
            template: &template,
            candidates: &candidates,
            with_metadata: false,
        }
        .write(
            &cold,
            &[4],
            Some(&dir.path().join("nope.ext")),
            true,
            "SCM test",
            None,
        )
        .unwrap();
        let model = Model::parse(&cold, &fs::read_to_string(&cold).unwrap()).unwrap();
        assert!((model.thetas[3].init - 0.4).abs() < 1e-12);
    }

    /// `$COVARIANCE` follows `cov_step`: stripped from a template that has
    /// one when it is off, appended to a template without one when it is on.
    #[test]
    fn write_scm_model_matches_covariance_to_cov_step() {
        let dir = tempfile::tempdir().unwrap();
        let candidates = cands(&[(4, 0.1), (5, 0.1), (6, 0.1)]);

        let template = write_template(dir.path());
        let dest = dir.path().join("scm/1001/forward_round1/1001_wt_cl.mod");
        ModelWriter {
            template: &template,
            candidates: &candidates,
            with_metadata: false,
        }
        .write(&dest, &[4], None, false, "SCM test", None)
        .unwrap();
        let content = fs::read_to_string(&dest).unwrap();
        assert!(!content.contains("$COVARIANCE"), "{content}");
        Model::parse(&dest, &content).unwrap();

        let template = write_template_content(dir.path(), &TEMPLATE.replace("$COVARIANCE\n", ""));
        let dest = dir.path().join("scm/1001/forward_round1/1001_crcl_cl.mod");
        ModelWriter {
            template: &template,
            candidates: &candidates,
            with_metadata: false,
        }
        .write(&dest, &[5], None, true, "SCM test", None)
        .unwrap();
        let content = fs::read_to_string(&dest).unwrap();
        assert!(content.trim_end().ends_with("$COVARIANCE"), "{content}");
        Model::parse(&dest, &content).unwrap();
    }

    #[test]
    fn round_entries_cover_the_right_sets() {
        let dir = tempfile::tempdir().unwrap();
        let template = write_template(dir.path());
        let plan = build_plan(
            &template,
            &Covariates::named(&["WT_CL", "CRCL_CL", "WT_V"]),
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
}
