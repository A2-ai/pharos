//! Shared fixtures for the SCM test suite.
//!
//! Everything an SCM test needs that is not the thing under test lives here,
//! defined once so every reader of an SCM process — status, round detail,
//! decision log, round summary, plan context — is exercised against the same
//! runs:
//!
//! - [`snapshot_settings`]: the insta settings every SCM snapshot binds, with
//!   the filters that make timestamps and temp-dir paths deterministic.
//! - The template control streams, kept as files under
//!   `test_data/scm/templates/` so `glob!`-driven tests can iterate them, and
//!   the helpers that lay one down beside a dummy dataset.
//! - [`Fit`] and [`write_fit_output`]: what a finished pharos run of a given
//!   kind leaves on disk, fabricated without running NONMEM.
//! - [`MockExecutor`] and the named scenarios built on it.
//! - [`transcript`]: the record of a whole driver run in one string, for
//!   end-to-end scenario snapshots.

// A fixture library: scenarios and helpers are defined ahead of the tests
// that will use them, so unused ones are expected, not a bug.
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::Result;
use fs_err as fs;

use super::driver::FitExecutor;
use super::{DECISION_LOG_MD, Direction, STATE_FILENAME, ScmOptions, ScmPlan, build_plan};
use crate::run::metadata::{RUN_END_FILENAME, RUN_START_FILENAME};
use crate::run::signal_wrapper::TERMINATION_FILENAME;

// ---------------------------------------------------------------------------
// Snapshot settings
// ---------------------------------------------------------------------------

/// Placeholder a redacted timestamp renders as.
pub(crate) const TIMESTAMP_PLACEHOLDER: &str = "[TIMESTAMP]";
/// Placeholder a redacted temp-dir path renders as.
pub(crate) const TMP_PLACEHOLDER: &str = "[TMP]";
/// Placeholder a redacted plan digest renders as.
pub(crate) const DIGEST_PLACEHOLDER: &str = "[DIGEST]";

/// Where every SCM snapshot file lives: `src/scm/snapshots/`, relative to
/// the test file, and deliberately apart from the output-file and parser
/// snapshot directories elsewhere in the workspace.
pub(crate) const SNAPSHOT_DIR: &str = "snapshots";

/// The insta settings every SCM snapshot binds. Three things in SCM output
/// change from run to run and are filtered out here:
///
/// - timestamps, as `utils::get_utc_now` writes them
///   (`2026-09-08T16:02:15+00:00`; `Z` and fractional seconds tolerated) —
///   the plan's `created`, the state's `updated`, a round summary's
///   `generated`;
/// - the temp directory the test built its fixtures in, which lands in
///   plan.json, the first line of the plan and status renderings, and most
///   error messages. Its canonical form is filtered too, for platforms
///   where the temp dir is a symlink;
/// - the plan digest, a blake3 hash over fields that include those paths,
///   so it changes with the temp dir too.
///
/// Usage:
///
/// ```ignore
/// let settings = snapshot_settings(dir.path());
/// settings.bind(|| insta::assert_snapshot!(status.render_text()));
/// ```
pub(crate) fn snapshot_settings(tmp: &Path) -> insta::Settings {
    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path(SNAPSHOT_DIR);
    settings.add_filter(
        r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})?",
        TIMESTAMP_PLACEHOLDER,
    );
    settings.add_filter(r"\b[0-9a-f]{64}\b", DIGEST_PLACEHOLDER);
    let mut roots = vec![tmp.to_path_buf()];
    if let Ok(canonical) = std::fs::canonicalize(tmp)
        && canonical != tmp
    {
        roots.push(canonical);
    }
    // Longest first so a canonical prefix never leaves a tail behind.
    roots.sort_by_key(|p| std::cmp::Reverse(p.as_os_str().len()));
    for root in roots {
        settings.add_filter(&regex::escape(&root.display().to_string()), TMP_PLACEHOLDER);
    }
    settings
}

// ---------------------------------------------------------------------------
// Templates
// ---------------------------------------------------------------------------

/// The template style the SCM process requires: each candidate effect is
/// its own named `$PK` assignment referencing exactly one theta, so the
/// term name can key the request. Writing those thetas `(0 FIX)` is the
/// convention, not a rule.
pub(crate) const TEMPLATE: &str = include_str!("../../test_data/scm/templates/standard.mod");

/// The same model written inline — the covariate effects folded into the
/// `TVCL` / `V` expressions instead of standing on their own. No term
/// names a single candidate theta, so nothing in it can be requested.
pub(crate) const INLINE_TEMPLATE: &str = include_str!("../../test_data/scm/templates/inline.mod");

/// Directory the template files live in, for `glob!`-driven tests.
pub(crate) fn templates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/scm/templates")
}

/// The dummy dataset every template's `$DATA data.csv` points at.
const DATASET: &str = "ID,TIME,AMT,DV,WT,CRCL,AGE\n1,0,100,0,70,100,40\n";

/// Shorthand for the covariates argument: the `$PK` term names naming the
/// candidate effects.
pub(crate) fn names(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// The test templates carry a `$COVARIANCE` record, so tests that expect a
/// warning-free plan opt the covariance step back on (the default is off).
pub(crate) fn opts_cov_on() -> ScmOptions {
    ScmOptions {
        cov_step: true,
        ..Default::default()
    }
}

/// Forward-only options, the shape most scenario tests want.
pub(crate) fn opts_forward_only() -> ScmOptions {
    ScmOptions {
        direction: vec![Direction::Forward],
        ..Default::default()
    }
}

/// Write [`TEMPLATE`] + the dummy dataset into `dir` as `1001.mod`,
/// returning the model path.
pub(crate) fn write_template(dir: &Path) -> PathBuf {
    write_template_content(dir, TEMPLATE)
}

/// Write `content` as `1001.mod` beside the dummy dataset, returning the
/// model path.
pub(crate) fn write_template_content(dir: &Path, content: &str) -> PathBuf {
    write_named_template(dir, "1001.mod", content)
}

/// Write `content` under `file_name` beside the dummy dataset, for tests
/// that need a different stem or extension (`.ctl`).
pub(crate) fn write_named_template(dir: &Path, file_name: &str, content: &str) -> PathBuf {
    fs::create_dir_all(dir).unwrap();
    let model_path = dir.join(file_name);
    fs::write(&model_path, content).unwrap();
    fs::write(dir.join("data.csv"), DATASET).unwrap();
    model_path
}

/// A plan for the standard template's three candidates, written into
/// `dir`, with the options given.
pub(crate) fn make_plan(dir: &Path, options: ScmOptions) -> ScmPlan {
    let template = write_template(dir);
    build_plan(
        &template,
        &names(&["WT_CL", "CRCL_CL", "WT_V"]),
        None,
        options,
        "test",
    )
    .unwrap()
    .plan
}

pub(crate) fn forward_only_plan(dir: &Path) -> ScmPlan {
    make_plan(dir, opts_forward_only())
}

// ---------------------------------------------------------------------------
// Fabricated run output
// ---------------------------------------------------------------------------

/// How one mocked fit ends — each variant is what a real pharos run of that
/// kind leaves in its output directory.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Fit {
    /// Minimization successful; final estimates written.
    Succeeded(f64),
    /// Minimization successful, but the listing carries the heuristic
    /// warnings a shaky fit prints: parameter near boundary, hessian reset.
    SucceededWithWarnings(f64),
    /// `0MINIMIZATION TERMINATED`: NONMEM gave up on the search. Final
    /// estimates are written, so an OFV is readable, but the fit is not
    /// usable for scoring.
    MinimizationTerminated(f64),
    /// `PROGRAM TERMINATED BY OBJ`. NONMEM still writes a final-estimates
    /// row holding the last diverged iteration, so an OFV is there to be
    /// read — which is exactly what makes this failure mode dangerous.
    Aborted(f64),
    /// Aborted at the initial OBJ evaluation, before the .ext header was
    /// written: the file exists but parses to zero tables.
    AbortedHeaderless,
    /// Ran but never reached final estimates (retryable).
    NoFinalRow,
    /// Killed by the signal wrapper mid-estimation: a termination marker,
    /// a partial .ext and no end file.
    Terminated,
    /// Dispatched, and still running: only the start marker exists.
    StillRunning,
}

/// The pharos run directory for `model`, as the runner lays it out: a
/// subfolder beside the model, named after it.
pub(crate) fn run_dir_of(model: &Path) -> PathBuf {
    let stem = model.file_stem().unwrap().to_string_lossy().to_string();
    model.parent().unwrap().join(stem)
}

const EXT_HEADER: &str = "TABLE NO.     1: First Order Conditional Estimation with Interaction\n\
 ITERATION    THETA1       THETA2       THETA3       THETA4       THETA5       THETA6       OMEGA(1,1)   OMEGA(2,2)   SIGMA(1,1)   OBJ\n";
const EXT_ITERATIONS: &str = "            0  3.00000E+00  2.00000E+01  1.20000E+00  1.00000E-01  1.00000E-01  1.00000E-01  1.00000E-01  1.00000E-01  2.00000E-02  1100\n\
            8  1.11000E-01  2.22000E-01  3.33000E-01  4.44000E-01  5.55000E-01  6.66000E-01  9.00000E-02  9.00000E-02  1.90000E-02  1050\n";
const EXT_FINAL_ROW: &str = "  -1000000000  3.10000E+00  2.10000E+01  1.30000E+00  2.50000E-01  1.50000E-01  5.00000E-02  8.00000E-02  8.50000E-02  1.80000E-02";

/// Lay down what a pharos run of `model` ending in `fit` leaves behind:
/// the start/end/termination markers, the `.ext` and the `.lst`. The
/// listing carries the model back out (line 1 is a timestamp, then the
/// control stream up to NM-TRAN MESSAGES), so the real one is embedded.
pub(crate) fn write_fit_output(model: &Path, fit: Fit) -> Result<()> {
    let stem = model.file_stem().unwrap().to_string_lossy().to_string();
    let run_dir = run_dir_of(model);
    fs::create_dir_all(&run_dir)?;
    fs::write(run_dir.join(RUN_START_FILENAME), "{}")?;

    if fit == Fit::StillRunning {
        return Ok(());
    }

    // .ext
    let ext = match fit {
        Fit::AbortedHeaderless => format!("{EXT_FINAL_ROW}  0.00000E+00\n"),
        Fit::Succeeded(ofv)
        | Fit::SucceededWithWarnings(ofv)
        | Fit::MinimizationTerminated(ofv)
        | Fit::Aborted(ofv) => {
            format!("{EXT_HEADER}{EXT_ITERATIONS}{EXT_FINAL_ROW}  {ofv}\n")
        }
        Fit::NoFinalRow | Fit::Terminated => format!("{EXT_HEADER}{EXT_ITERATIONS}"),
        Fit::StillRunning => unreachable!(),
    };
    fs::write(run_dir.join(format!("{stem}.ext")), ext)?;

    // .lst
    let mut lst = String::from("Wed Sep  4 00:00:00 UTC 2026\n");
    lst.push_str(&fs::read_to_string(model)?);
    lst.push_str("\nNM-TRAN MESSAGES\n \n MONITORING OF SEARCH:\n \n");
    match fit {
        Fit::Succeeded(_) => {
            lst.push_str("0MINIMIZATION SUCCESSFUL\n");
            lst.push_str(" NO. OF FUNCTION EVALUATIONS USED:      123\n");
        }
        Fit::SucceededWithWarnings(_) => {
            lst.push_str("0MINIMIZATION SUCCESSFUL\n");
            lst.push_str(" HOWEVER, PROBLEMS OCCURRED WITH THE MINIMIZATION.\n");
            lst.push_str("0PARAMETER ESTIMATE IS NEAR ITS BOUNDARY\n");
            lst.push_str("0RESET HESSIAN, TYPE I\n");
            lst.push_str(" NO. OF FUNCTION EVALUATIONS USED:      456\n");
        }
        Fit::MinimizationTerminated(_) => {
            lst.push_str("0MINIMIZATION TERMINATED\n");
            lst.push_str(" DUE TO ROUNDING ERRORS (ERROR=134)\n");
            lst.push_str(" NO. OF FUNCTION EVALUATIONS USED:      789\n");
        }
        Fit::Aborted(_) | Fit::AbortedHeaderless => {
            lst.push_str("0PRED EXIT CODE = 1\n0PROGRAM TERMINATED BY OBJ\n");
            lst.push_str(" MESSAGE ISSUED FROM ESTIMATION STEP\n");
        }
        // A run killed mid-estimation prints no verdict at all.
        Fit::NoFinalRow | Fit::Terminated => {}
        Fit::StillRunning => unreachable!(),
    }
    lst.push_str(" \n #TERE:\n Elapsed estimation  time in seconds:     1.00\n");
    fs::write(run_dir.join(format!("{stem}.lst")), lst)?;

    // markers
    if fit == Fit::Terminated {
        fs::write(run_dir.join(TERMINATION_FILENAME), "{}")?;
    } else {
        fs::write(run_dir.join(RUN_END_FILENAME), "{}")?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Mock executor and scenarios
// ---------------------------------------------------------------------------

/// Fabricates pharos run outputs instead of running NONMEM. Behavior is
/// keyed by `"{round_dir}/{model_stem_without_try_suffix}"`; the Vec gives
/// one [`Fit`] per attempt. A model with no behavior (or more attempts than
/// listed) succeeds at `default_ofv`.
pub(crate) struct MockExecutor {
    behaviors: HashMap<String, Vec<Fit>>,
    default_ofv: f64,
    /// Every fit dispatched, in order, as behavior keys.
    fits: Mutex<Vec<String>>,
    /// When set, `fit` returns this error instead of writing anything —
    /// the shape of a scheduler that could not submit.
    fail_with: Option<String>,
}

impl MockExecutor {
    pub(crate) fn new(default_ofv: f64) -> Self {
        Self {
            behaviors: HashMap::new(),
            default_ofv,
            fits: Mutex::new(vec![]),
            fail_with: None,
        }
    }

    pub(crate) fn with(mut self, key: &str, attempts: Vec<Fit>) -> Self {
        self.behaviors.insert(key.to_string(), attempts);
        self
    }

    /// Make every `fit` call fail with `message` (a pharos-level error, as
    /// opposed to a fit that ran and failed).
    pub(crate) fn failing_with(mut self, message: &str) -> Self {
        self.fail_with = Some(message.to_string());
        self
    }

    pub(crate) fn key_and_attempt(model: &Path) -> (String, usize) {
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

    /// How many dispatched fits had `needle` in their key.
    pub(crate) fn fit_count(&self, needle: &str) -> usize {
        self.fits
            .lock()
            .unwrap()
            .iter()
            .filter(|f| f.contains(needle))
            .count()
    }

    /// Every fit dispatched so far, in order.
    pub(crate) fn fits(&self) -> Vec<String> {
        self.fits.lock().unwrap().clone()
    }
}

impl FitExecutor for MockExecutor {
    fn fit(&self, models: &[PathBuf]) -> Result<()> {
        if let Some(message) = &self.fail_with {
            anyhow::bail!("{message}");
        }
        for model in models {
            let (key, attempt) = Self::key_and_attempt(model);
            self.fits.lock().unwrap().push(key.clone());

            let fit = match self.behaviors.get(&key) {
                Some(attempts) => attempts
                    .get(attempt - 1)
                    .copied()
                    .unwrap_or(Fit::Succeeded(self.default_ofv)),
                None => Fit::Succeeded(self.default_ofv),
            };
            write_fit_output(model, fit)?;
        }
        Ok(())
    }

    fn describe(&self) -> String {
        "mock".to_string()
    }
}

/// The full fixture: forward picks WT_CL then CRCL_CL (with a retry on
/// WT_V in round 2), forward stops in round 3, backward drops CRCL_CL at
/// the stricter alpha, then keeps WT_CL and stops.
pub(crate) fn full_scm_executor() -> MockExecutor {
    MockExecutor::new(1234.0)
        .with("base/1001_base", vec![Fit::Succeeded(1000.0)])
        // forward round 1: WT_CL wins big
        .with("forward_round1/1001_wt_cl", vec![Fit::Succeeded(980.0)])
        .with("forward_round1/1001_crcl_cl", vec![Fit::Succeeded(996.0)])
        .with("forward_round1/1001_wt_v", vec![Fit::Succeeded(999.0)])
        // forward round 2 (ref 980): CRCL_CL wins; WT_V fails once, then succeeds
        .with("forward_round2/1001_crcl_cl", vec![Fit::Succeeded(974.0)])
        .with(
            "forward_round2/1001_wt_v",
            vec![Fit::NoFinalRow, Fit::Succeeded(978.5)],
        )
        // forward round 3 (ref 974): WT_V not significant -> forward stops
        .with("forward_round3/1001_wt_v", vec![Fit::Succeeded(973.0)])
        // backward round 1 (ref 974): dropping WT_CL hurts a lot (keep),
        // dropping CRCL_CL costs 6 points (p ~ 0.014 > 0.001 -> drop)
        .with("backward_round1/1001_wt_cl", vec![Fit::Succeeded(995.0)])
        .with("backward_round1/1001_crcl_cl", vec![Fit::Succeeded(980.0)])
        // backward round 2 (ref 980): dropping WT_CL still hurts -> stop
        .with("backward_round2/1001_wt_cl", vec![Fit::Succeeded(1000.0)])
}

/// Two candidates with the same OFV score identically — same ΔOFV, same
/// p-value — so the SCM process cannot pick between them (forward-only).
pub(crate) fn tied_executor() -> MockExecutor {
    MockExecutor::new(1234.0)
        .with("base/1001_base", vec![Fit::Succeeded(1000.0)])
        // WT_CL and CRCL_CL land on exactly the same OFV
        .with("forward_round1/1001_wt_cl", vec![Fit::Succeeded(980.0)])
        .with("forward_round1/1001_crcl_cl", vec![Fit::Succeeded(980.0)])
        .with("forward_round1/1001_wt_v", vec![Fit::Succeeded(999.0)])
        // round 2 (ref 980): nothing else is significant -> forward stops
        .with("forward_round2/1001_wt_cl", vec![Fit::Succeeded(979.9)])
        .with("forward_round2/1001_wt_v", vec![Fit::Succeeded(979.8)])
}

/// Forward-only: WT_V never produces an OFV and concludes unusable after
/// its retries; WT_CL is barely significant, CRCL_CL is not. Pair with
/// `max_retries = 1`.
pub(crate) fn unusable_candidate_executor() -> MockExecutor {
    MockExecutor::new(1234.0)
        .with("base/1001_base", vec![Fit::Succeeded(1000.0)])
        .with("forward_round1/1001_wt_cl", vec![Fit::Succeeded(995.0)])
        .with("forward_round1/1001_crcl_cl", vec![Fit::Succeeded(999.5)])
        .with(
            "forward_round1/1001_wt_v",
            vec![Fit::NoFinalRow, Fit::NoFinalRow],
        )
        .with("forward_round2/1001_crcl_cl", vec![Fit::Succeeded(994.0)])
        .with(
            "forward_round2/1001_wt_v",
            vec![Fit::NoFinalRow, Fit::NoFinalRow],
        )
}

/// The reference fit never succeeds: the SCM process cannot start. Pair
/// with `max_retries = 1` for a two-attempt failure.
pub(crate) fn failing_reference_executor() -> MockExecutor {
    MockExecutor::new(1234.0).with(
        "base/1001_base",
        vec![Fit::MinimizationTerminated(1000.0), Fit::NoFinalRow],
    )
}

/// Forward-only: WT_CL and CRCL_CL reach the same p-value bucket only
/// approximately — their ΔOFVs differ, so the ΔOFV tie-break decides and
/// nothing pauses.
pub(crate) fn near_tie_executor() -> MockExecutor {
    MockExecutor::new(1234.0)
        .with("base/1001_base", vec![Fit::Succeeded(1000.0)])
        .with("forward_round1/1001_wt_cl", vec![Fit::Succeeded(980.0)])
        .with("forward_round1/1001_crcl_cl", vec![Fit::Succeeded(980.001)])
        .with("forward_round1/1001_wt_v", vec![Fit::Succeeded(999.0)])
        .with("forward_round2/1001_crcl_cl", vec![Fit::Succeeded(979.9)])
        .with("forward_round2/1001_wt_v", vec![Fit::Succeeded(979.8)])
}

/// Forward-only: nothing is significant in round 1, so forward stops with
/// no covariate retained and the final model releases nothing.
pub(crate) fn nothing_significant_executor() -> MockExecutor {
    MockExecutor::new(1234.0)
        .with("base/1001_base", vec![Fit::Succeeded(1000.0)])
        .with("forward_round1/1001_wt_cl", vec![Fit::Succeeded(999.5)])
        .with("forward_round1/1001_crcl_cl", vec![Fit::Succeeded(999.0)])
        .with("forward_round1/1001_wt_v", vec![Fit::Succeeded(1000.2)])
}

/// Forward-only: every candidate fails every attempt in round 1, so the
/// round concludes with nothing scored at all. Pair with `max_retries = 1`.
pub(crate) fn everything_unusable_executor() -> MockExecutor {
    MockExecutor::new(1234.0)
        .with("base/1001_base", vec![Fit::Succeeded(1000.0)])
        .with(
            "forward_round1/1001_wt_cl",
            vec![Fit::NoFinalRow, Fit::MinimizationTerminated(990.0)],
        )
        .with(
            "forward_round1/1001_crcl_cl",
            vec![Fit::Aborted(700.0), Fit::AbortedHeaderless],
        )
        .with(
            "forward_round1/1001_wt_v",
            vec![Fit::Terminated, Fit::NoFinalRow],
        )
}

// ---------------------------------------------------------------------------
// Fabricated state
// ---------------------------------------------------------------------------

/// A state two forward rounds in, with a third under way: WT_CL and
/// CRCL_CL selected, WT_V scored in round 3 and AGE_CL still pending. The
/// shape a re-plan meets when the SCM process is paused mid-way.
pub(crate) fn mid_scm_state(plan: &ScmPlan) -> super::state::ScmState {
    use super::state::{CandidateRecord, CandidateStatus, RoundRecord, ScmRunStatus, ScmState};

    let mut state = ScmState::new(plan.digest());
    state.status = ScmRunStatus::Paused;
    state.phase = Some(Direction::Forward);
    state.retained = vec!["WT_CL".to_string(), "CRCL_CL".to_string()];
    state.rounds = vec![
        RoundRecord {
            name: "forward_round1".to_string(),
            direction: Direction::Forward,
            reference_model: "base/1001_base.mod".to_string(),
            reference_ofv: Some(1000.0),
            candidates: vec![],
            winner: Some("WT_CL".to_string()),
            decision: "added WT_CL (p = 7.744e-6, dOFV = -20.000)".to_string(),
            complete: true,
        },
        RoundRecord {
            name: "forward_round2".to_string(),
            direction: Direction::Forward,
            reference_model: "forward_round1/1001_wt_cl.mod".to_string(),
            reference_ofv: Some(980.0),
            candidates: vec![],
            winner: Some("CRCL_CL".to_string()),
            decision: "added CRCL_CL (p = 1.431e-2, dOFV = -6.000)".to_string(),
            complete: true,
        },
        RoundRecord {
            name: "forward_round3".to_string(),
            direction: Direction::Forward,
            reference_model: "forward_round2/1001_crcl_cl.mod".to_string(),
            reference_ofv: Some(974.0),
            candidates: vec![
                {
                    let mut c = CandidateRecord::new("WT_V", "add WT_V".to_string(), 1);
                    c.status = CandidateStatus::Succeeded;
                    c
                },
                CandidateRecord::new("AGE_CL", "add AGE_CL".to_string(), 1),
            ],
            winner: None,
            decision: String::new(),
            complete: false,
        },
    ];
    state
}

// ---------------------------------------------------------------------------
// Transcript
// ---------------------------------------------------------------------------

/// Every file under `root`, as sorted paths relative to it — the shape of
/// an SCM out_dir after a run.
pub(crate) fn file_tree(root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let rel = entry
            .path()
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        files.push(rel);
    }
    files.sort();
    files
}

/// The record of one driver run against `plan`, in a single string meant
/// for `assert_snapshot!`: the fits dispatched in order, the files the run
/// left in the out_dir, the state file, and the decision log. Bind
/// [`snapshot_settings`] around the assertion so the state's timestamp is
/// redacted.
pub(crate) fn transcript(plan: &ScmPlan, executor: &MockExecutor) -> String {
    let out_dir = plan.out_dir_path();
    let mut out = String::new();

    out.push_str("# fits dispatched\n");
    for fit in executor.fits() {
        out.push_str(&fit);
        out.push('\n');
    }

    out.push_str("\n# files in out_dir\n");
    for file in file_tree(&out_dir) {
        out.push_str(&file);
        out.push('\n');
    }

    out.push_str(&format!("\n# {STATE_FILENAME}\n"));
    match fs::read_to_string(out_dir.join(STATE_FILENAME)) {
        Ok(state) => out.push_str(&state),
        Err(_) => out.push_str("(absent)\n"),
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }

    out.push_str(&format!("\n# {DECISION_LOG_MD}\n"));
    match fs::read_to_string(out_dir.join(DECISION_LOG_MD)) {
        Ok(log) => out.push_str(&log),
        Err(_) => out.push_str("(absent)\n"),
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The filters are the one piece of this module with behavior of its
    /// own; this snapshot is the proof they redact what they claim to.
    #[test]
    fn snapshot_settings_redact_timestamps_and_tmp_paths() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template(dir.path());
        let plan = build_plan(
            &model,
            &names(&["WT_CL", "CRCL_CL"]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap()
        .plan;

        let sample = format!(
            "created: {}\nplan: {}\nmodel: {}\ncanonical: {}\nplain z: 2026-09-08T16:02:15Z\nfractional: 2026-09-08T16:02:15.123+00:00\n",
            plan.created,
            plan.plan_path().display(),
            plan.model,
            std::fs::canonicalize(dir.path()).unwrap().display(),
        );

        snapshot_settings(dir.path()).bind(|| insta::assert_snapshot!(sample));
    }

    #[test]
    fn every_fit_kind_writes_a_run_the_driver_can_read() {
        use super::super::round::read_fit_outcome;

        let dir = tempfile::tempdir().unwrap();
        let cases = [
            (Fit::Succeeded(990.0), "succeeded", true),
            (Fit::SucceededWithWarnings(990.0), "succeeded", true),
            (
                Fit::MinimizationTerminated(990.0),
                "minimization terminated",
                false,
            ),
            (Fit::Aborted(700.0), "program aborted", false),
            (Fit::AbortedHeaderless, "program aborted", false),
            (Fit::NoFinalRow, "no ofv", false),
            (Fit::Terminated, "terminated", false),
            (Fit::StillRunning, "did not finish", false),
        ];
        for (i, (fit, label, usable)) in cases.into_iter().enumerate() {
            let model = write_named_template(
                &dir.path().join(format!("case{i}")),
                "1001.mod",
                TEMPLATE,
            );
            write_fit_output(&model, fit).unwrap();
            let outcome = read_fit_outcome(&model).unwrap();
            assert_eq!(outcome.label(), label, "{fit:?}");
            assert_eq!(outcome.usable(), usable, "{fit:?}");
        }
    }
}
