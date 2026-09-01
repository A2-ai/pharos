use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use fs_err as fs;
use nonmem_parser::Model;

use super::{ScmPlan, sanitize_name};
use crate::copy::{CopyOptions, UpdateType, copy_model};
use crate::output_files::ext::{ExtReader, get_estimation_results};
use crate::output_files::lst::LstSummary;
use crate::output_files::resolve_estimation_files;
use crate::run::metadata::{RUN_END_FILENAME, RUN_START_FILENAME};
use crate::run::signal_wrapper::TERMINATION_FILENAME;
use crate::update;

/// Model file name (no extension) for a candidate attempt.
pub fn scm_model_name(stem: &str, candidate: &str, attempt: usize) -> String {
    let base = format!("{stem}_{}", sanitize_name(candidate));
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
    model_path
        .parent()
        .unwrap_or(Path::new("."))
        .join(stem_of(model_path))
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

/// Write one SCM model: a copy of the template with `released` covariate
/// thetas (1-based) turned from `(0 FIX)` into free thetas, and the
/// `$COVARIANCE` record added or removed per `cov_step`.
///
/// With a `reference_ext`, the model warm-starts from the reference fit:
/// every free parameter (base thetas, omegas, sigmas) takes the reference
/// estimate, a released theta that was already free in the reference
/// continues from its estimate, and a newly released theta — whose reference
/// value is exactly 0, from `(0 FIX)` — starts at `release_init`. Without one
/// (the reference fit itself), everything starts from the template's initial
/// estimates.
#[allow(clippy::too_many_arguments)]
pub fn write_scm_model(
    template: &Path,
    dest: &Path,
    released: &[usize],
    release_init: f64,
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
    // thetas (the unreleased candidates) are left untouched by the updater.
    // A missing or unreadable reference output degrades to a cold start with
    // a warning — a worse initial point must not kill the search.
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

    let specs: BTreeMap<usize, String> = released
        .iter()
        .map(|&theta_num| {
            if theta_num == 0 || theta_num > model.thetas.len() {
                bail!(
                    "THETA({theta_num}) out of range: model has {} thetas",
                    model.thetas.len()
                );
            }
            // Free in the reference fit -> continue from its estimate;
            // `(0 FIX)` there reports exactly 0 -> start fresh at release_init.
            let init = match reference_estimates.get(&format!("THETA{theta_num}")) {
                Some(&est) if est.is_finite() && est != 0.0 => est,
                _ => release_init,
            };
            Ok((theta_num - 1, init.to_string()))
        })
        .collect::<Result<_>>()?;

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
        update::update_model_estimates(dest, &ext, &[UpdateType::All], true).with_context(
            || {
                format!(
                    "failed to carry estimates from {} into retry model",
                    ext.display()
                )
            },
        )?;
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
    /// Human labels of the heuristic checks that fired.
    pub heuristics: Vec<String>,
}

impl FitOutcome {
    /// A fit is usable for scoring when it ran to completion, was not killed,
    /// produced an OFV, and did not terminate minimization. Anything else is
    /// reported, never silently treated as insignificant.
    pub fn usable(&self) -> bool {
        self.finished
            && !self.terminated
            && self.ofv.is_some()
            && self.minimization_terminated != Some(true)
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
        } else if self.minimization_terminated == Some(true) {
            "minimization terminated".to_string()
        } else if self.ofv.is_none() {
            "no ofv".to_string()
        } else {
            "succeeded".to_string()
        }
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

    let (minimization_terminated, heuristics) = read_lst_heuristics(model_path, &run_dir);

    Ok(FitOutcome {
        started,
        finished,
        terminated,
        ofv,
        minimization_terminated,
        heuristics,
    })
}

fn read_lst_heuristics(model_path: &Path, run_dir: &Path) -> (Option<bool>, Vec<String>) {
    let stem = stem_of(model_path);
    let lst_path = run_dir.join(format!("{stem}.lst"));
    if !lst_path.exists() {
        return (None, vec![]);
    }
    match LstSummary::from_run(&lst_path) {
        Ok(summary) => {
            let h = &summary.run_heuristics;
            let mut fired = Vec::new();
            if h.minimization_terminated == Some(true) {
                fired.push("minimization terminated".to_string());
            }
            if h.parameter_near_boundary == Some(true) {
                fired.push("parameter near boundary".to_string());
            }
            if h.hessian_reset == Some(true) {
                fired.push("hessian reset".to_string());
            }
            if h.covariance_step_aborted == Some(true) {
                fired.push("covariance step aborted".to_string());
            }
            if h.eigenvalue_issues == Some(true) {
                fired.push("eigenvalue issues".to_string());
            }
            (h.minimization_terminated, fired)
        }
        Err(e) => {
            log::warn!("failed to parse {}: {e}", lst_path.display());
            (None, vec![])
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
    use crate::scm::plan::tests::write_template;
    use crate::scm::{ScmOptions, build_plan};

    #[test]
    fn model_names_carry_attempt_suffix() {
        assert_eq!(scm_model_name("1001", "WT_CL", 1), "1001_wt_cl");
        assert_eq!(scm_model_name("1001", "WT_CL", 2), "1001_wt_cl_try2");
    }

    #[test]
    fn write_scm_model_releases_and_rebases() {
        let dir = tempfile::tempdir().unwrap();
        let template = write_template(dir.path());

        let dest = dir.path().join("scm/1001/forward_round1/1001_wt_cl.mod");
        write_scm_model(&template, &dest, &[4], 0.1, None, true, "SCM test", None, false).unwrap();

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
        write_scm_model(&template, &dest, &[4], 0.1, None, false, "SCM test", None, false).unwrap();
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
        write_scm_model(&template, &dest, &[4], 0.1, None, true, "SCM test", None, false).unwrap();
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
            &[4, 5],
            0.1,
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
    fn missing_reference_ext_degrades_to_cold_start() {
        let dir = tempfile::tempdir().unwrap();
        let template = write_template(dir.path());
        let dest = dir.path().join("scm/1001/forward_round1/1001_wt_cl.mod");
        write_scm_model(
            &template,
            &dest,
            &[4],
            0.1,
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
        let plan = build_plan(&template, &[4, 5, 6], None, ScmOptions::default(), "test")
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
