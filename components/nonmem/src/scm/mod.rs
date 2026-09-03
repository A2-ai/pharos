//! Stepwise covariate modeling (SCM).
//!
//! The search is driven by a `plan.json`: [`plan::build_plan`]
//! validates the candidates the caller names by THETA number against a
//! user-authored template control stream (candidate effects written into
//! `$PK` and `(0 FIX)`'d in `$THETA`), [`driver::run_scm`] executes the search round by
//! round with resumable state in `scm_state.json`, and [`status::read_status`]
//! reports on a search wherever it currently stands.
//!
//! A round the numbers cannot decide — two candidates with an identical
//! p-value AND an identical ΔOFV — is not resolved by a tie-break rule: the
//! search records both scores, pauses, and waits for the user to name the
//! winner (`scm run --choose <candidate>`, see [`state::PendingTie`]).
//!
//! Each round leaves a record behind as it concludes: a `round_summary.json`
//! / `.md` in its own round directory, a `pharos_summary.json` in every
//! finished run's directory, and freshly rewritten decision-log files in the
//! search's out_dir — so the on-disk record always matches the state, not
//! just at the end of the search.

pub mod config;
pub mod driver;
pub mod log;
pub mod plan;
pub mod round;
pub mod score;
pub mod state;
pub mod status;

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result, bail};
use fs_err as fs;
use serde::{Deserialize, Serialize};

pub use config::{ScmConfig, ScmPlanOverrides, build_plan_from_config};
pub use driver::{FitExecutor, LocalExecutor, ScmOutcome, run_scm};
pub use log::{DecisionLogRow, RoundSummary, decision_log_rows, write_round_summary};
pub use plan::{CovariateSpec, build_plan};
pub use state::{
    CandidateRecord, CandidateStatus, PendingTie, RoundRecord, ScmRunStatus, ScmState,
};
pub use status::{ScmRoundDetail, ScmStatus, read_round_detail, read_status};

pub const PLAN_FILENAME: &str = "plan.json";
pub const STATE_FILENAME: &str = "scm_state.json";
pub const DECISION_LOG_CSV: &str = "scm_decision_log.csv";
pub const DECISION_LOG_MD: &str = "scm_decision_log.md";
/// Written into each round directory when the round concludes.
pub const ROUND_SUMMARY_JSON: &str = "round_summary.json";
pub const ROUND_SUMMARY_MD: &str = "round_summary.md";
/// Per-run `pharos nonmem summary` output written into each run directory.
pub const RUN_SUMMARY_FILENAME: &str = "pharos_summary.json";
pub const PLAN_SCHEMA_VERSION: u32 = 1;
/// Name of the pseudo-round holding the reference fit (not a search round).
pub const REFERENCE_ROUND: &str = "reference";
/// Stands in for a round's reference model when there isn't one (the
/// reference round itself).
pub const NO_REFERENCE: &str = "-";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Forward,
    Backward,
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Direction::Forward => "forward",
            Direction::Backward => "backward",
        })
    }
}

impl FromStr for Direction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "forward" => Ok(Direction::Forward),
            "backward" => Ok(Direction::Backward),
            _ => Err(format!(
                "Unknown direction '{s}': expected 'forward' or 'backward'"
            )),
        }
    }
}

/// Search options carried in the plan — everything that defines the search
/// itself. Execution concerns (slurm, partition, polling) live with `scm run`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScmOptions {
    /// Which phases to run, e.g. `["forward", "backward"]`. Forward always
    /// runs before backward when both are present.
    pub direction: Vec<Direction>,
    /// Significance level for adding a covariate in forward selection.
    pub forward_alpha: f64,
    /// Significance level for keeping a covariate in backward elimination.
    pub backward_alpha: f64,
    /// Pause the search after this many rounds per invocation (resumable).
    pub num_rounds: Option<usize>,
    /// Retries per failed fit; each retry starts from the previous attempt's
    /// estimates (never jittered).
    pub max_retries: usize,
    /// Initial estimate a newly released covariate theta starts at on a
    /// first attempt. Thetas already free in the round's reference fit
    /// (retained covariates and base parameters) continue from its estimates.
    pub release_init: f64,
    /// Whether generated models run the covariance step ($COVARIANCE).
    pub cov_step: bool,
    /// Replace existing SCM output from a different plan in out_dir.
    pub overwrite: bool,
}

impl Default for ScmOptions {
    fn default() -> Self {
        Self {
            direction: vec![Direction::Forward, Direction::Backward],
            forward_alpha: 0.05,
            backward_alpha: 0.001,
            num_rounds: None,
            max_retries: 3,
            release_init: 0.1,
            cov_step: false,
            overwrite: false,
        }
    }
}

impl ScmOptions {
    /// The phases this search runs, in run order: forward always precedes
    /// backward, however the plan happens to list them.
    pub fn phases(&self) -> Vec<Direction> {
        [Direction::Forward, Direction::Backward]
            .into_iter()
            .filter(|d| self.direction.contains(d))
            .collect()
    }

    pub fn runs_forward(&self) -> bool {
        self.direction.contains(&Direction::Forward)
    }

    pub fn runs_backward(&self) -> bool {
        self.direction.contains(&Direction::Backward)
    }

    /// The phases in run order, e.g. `forward -> backward`.
    pub fn direction_label(&self) -> String {
        self.phases()
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join(" -> ")
    }

    pub fn validate(&self) -> Result<()> {
        if self.direction.is_empty() {
            bail!("direction must contain 'forward', 'backward', or both");
        }
        let mut seen = std::collections::HashSet::new();
        for d in &self.direction {
            if !seen.insert(*d) {
                bail!("direction contains '{d}' more than once");
            }
        }
        for (name, alpha) in [
            ("forward_alpha", self.forward_alpha),
            ("backward_alpha", self.backward_alpha),
        ] {
            if !(alpha > 0.0 && alpha < 1.0) {
                bail!("{name} must be in (0, 1), got {alpha}");
            }
        }
        if let Some(n) = self.num_rounds
            && n < 1
        {
            bail!("num_rounds must be at least 1");
        }
        if !(self.release_init.is_finite() && self.release_init != 0.0) {
            bail!(
                "release_init must be a non-zero finite number, got {}",
                self.release_init
            );
        }
        Ok(())
    }
}

/// A covariate effect candidate: one `(0 FIX)` theta in the template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    /// The name from the theta's comment, e.g. `WT_CL`; `THETA<n>` when the
    /// theta has no comment.
    pub name: String,
    /// 1-based THETA number in the template.
    pub theta: usize,
}

/// The plan.json: everything needed to run the search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScmPlan {
    pub schema_version: u32,
    pub created: String,
    pub pharos_version: String,
    /// Path to the template control stream, as given (typically relative to
    /// the pharos project root, which is where scm commands run from).
    pub model: String,
    /// Directory the search writes into; plan.json lives here.
    pub out_dir: String,
    pub candidates: Vec<Candidate>,
    /// Maximum possible number of models the search can fit — the reference
    /// fit plus the worst case of every phase, excluding retries. Derived
    /// from the candidates and direction (see [`ScmPlan::computed_max_models`]);
    /// a plan.json written before this field existed loads with it filled in.
    #[serde(default)]
    pub max_models: usize,
    pub options: ScmOptions,
}

impl ScmPlan {
    pub fn model_path(&self) -> PathBuf {
        PathBuf::from(&self.model)
    }

    /// This plan's worst-case model count; see [`max_models_for`].
    pub fn computed_max_models(&self) -> usize {
        max_models_for(self.candidates.len(), self.options.phases().len())
    }

    pub fn out_dir_path(&self) -> PathBuf {
        PathBuf::from(&self.out_dir)
    }

    pub fn plan_path(&self) -> PathBuf {
        self.out_dir_path().join(PLAN_FILENAME)
    }

    /// 1-based theta numbers for a set of candidate names.
    pub fn thetas_for(&self, names: &[String]) -> Vec<usize> {
        self.candidates
            .iter()
            .filter(|c| names.contains(&c.name))
            .map(|c| c.theta)
            .collect()
    }

    /// Stable digest of the search-defining fields, used to detect that
    /// on-disk state belongs to a different plan.
    pub fn digest(&self) -> String {
        let payload = serde_json::json!({
            "model": self.model,
            "out_dir": self.out_dir,
            "candidates": self.candidates,
            "options": {
                // overwrite/num_rounds are run-control, not search-defining
                "direction": self.options.direction,
                "forward_alpha": self.options.forward_alpha,
                "backward_alpha": self.options.backward_alpha,
                "max_retries": self.options.max_retries,
                "release_init": self.options.release_init,
                "cov_step": self.options.cov_step,
            },
        });
        blake3::hash(payload.to_string().as_bytes())
            .to_hex()
            .to_string()
    }

    pub fn save(&self) -> Result<PathBuf> {
        let path = self.plan_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        utils::write_json_to_file(self, &path)
            .with_context(|| format!("failed to write {}", path.display()))?;
        Ok(path)
    }

    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(json: &str) -> Result<Self> {
        let mut plan: ScmPlan =
            serde_json::from_str(json).context("failed to parse SCM plan JSON")?;
        if plan.schema_version > PLAN_SCHEMA_VERSION {
            bail!(
                "plan schema version {} is newer than this pharos supports ({})",
                plan.schema_version,
                PLAN_SCHEMA_VERSION
            );
        }
        // A plan written before max_models existed carries the default 0.
        if plan.max_models == 0 {
            plan.max_models = plan.computed_max_models();
        }
        plan.options.validate()?;
        Ok(plan)
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read plan file {}", path.display()))?;
        Self::from_json(&content)
    }

    /// Human-readable rendering of the plan.
    pub fn render_text(&self) -> String {
        let mut out = Lines::new();
        let o = &self.options;

        out.add(format!("<scm plan> {}", self.plan_path().display()));
        out.add(format!("model      : {}", self.model));
        out.add(format!("out dir    : {}", self.out_dir));
        out.add(format!("direction  : {}", o.direction_label()));
        if o.runs_forward() {
            out.add(format!("forward    : alpha {}", o.forward_alpha));
        }
        if o.runs_backward() {
            out.add(format!("backward   : alpha {}", o.backward_alpha));
        }
        out.add(format!(
            "on failure : retry up to {}x from the previous attempt's estimates",
            o.max_retries
        ));
        out.add(format!("cov step   : {}", on_off(o.cov_step)));
        if let Some(n) = o.num_rounds {
            out.add(format!("num rounds : pause after {n} (resumable)"));
        }
        out.add("candidates :");
        for c in &self.candidates {
            out.add(format!(
                "  {:<12} THETA({}) -> released at {} when first tested",
                c.name, c.theta, o.release_init
            ));
        }
        out.add(format!(
            "max models : {} (incl. reference fit, excl. retries)",
            self.max_models
        ));
        out.finish()
    }
}

/// Accumulates the lines of a rendered report. Every SCM rendering — the
/// plan, the status, a round, the decision log, a round summary — builds its
/// text through one of these.
#[derive(Default)]
pub(crate) struct Lines(String);

impl Lines {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn add(&mut self, line: impl AsRef<str>) {
        self.0.push_str(line.as_ref());
        self.0.push('\n');
    }

    /// A blank separator line (markdown renderings lean on these).
    pub(crate) fn blank(&mut self) {
        self.0.push('\n');
    }

    pub(crate) fn finish(self) -> String {
        self.0
    }
}

/// Worst case number of models a search fits: the single reference fit plus,
/// for each phase, one model per candidate in the first round, one fewer in
/// the next, and so on down to one — n(n+1)/2 per phase. Excludes retries.
/// (Forward starts from the base model, backward-only from the full model,
/// and a forward -> backward search re-uses the forward winner as the
/// backward reference, so there is only ever one reference fit.)
pub fn max_models_for(n_candidates: usize, n_phases: usize) -> usize {
    1 + n_phases * n_candidates * (n_candidates + 1) / 2
}

/// The directory a round's models and records live in: the round name,
/// except the reference round, whose single "candidate" (base/full) names
/// its directory. `None` when a reference round has no candidate to name it.
pub(crate) fn round_dir(round_name: &str, candidates: &[CandidateRecord]) -> Option<String> {
    if round_name == REFERENCE_ROUND {
        candidates.first().map(|c| c.candidate.clone())
    } else {
        Some(round_name.to_string())
    }
}

/// A path's parent, falling back to the current directory.
pub(crate) fn parent_or_dot(path: &Path) -> &Path {
    path.parent().unwrap_or(Path::new("."))
}

/// `" (OFV 1234.567)"` for a known OFV, empty otherwise — the parenthetical
/// every rendering appends after a model name.
pub(crate) fn ofv_suffix(ofv: Option<f64>) -> String {
    ofv.map(|o| format!(" (OFV {o:.3})")).unwrap_or_default()
}

/// A comma-separated list, or "none" when there is nothing in it.
pub(crate) fn none_or_list(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_string()
    } else {
        items.join(", ")
    }
}

pub(crate) fn on_off(flag: bool) -> &'static str {
    if flag { "on" } else { "off" }
}

pub(crate) fn yes_no(flag: bool) -> &'static str {
    if flag { "yes" } else { "no" }
}

/// Sanitize a candidate name into a filename-safe, lowercase fragment.
pub(crate) fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_defaults_are_the_documented_ones() {
        let o = ScmOptions::default();
        assert_eq!(o.direction, vec![Direction::Forward, Direction::Backward]);
        assert_eq!(o.forward_alpha, 0.05);
        assert_eq!(o.backward_alpha, 0.001);
        assert_eq!(o.max_retries, 3);
        assert_eq!(o.release_init, 0.1);
        assert!(!o.cov_step);
        assert!(!o.overwrite);
        assert!(o.num_rounds.is_none());
        o.validate().unwrap();
    }

    #[test]
    fn phases_run_forward_first_however_the_plan_lists_them() {
        let reversed = ScmOptions {
            direction: vec![Direction::Backward, Direction::Forward],
            ..Default::default()
        };
        assert_eq!(
            reversed.phases(),
            vec![Direction::Forward, Direction::Backward]
        );
        assert_eq!(reversed.direction_label(), "forward -> backward");

        let backward_only = ScmOptions {
            direction: vec![Direction::Backward],
            ..Default::default()
        };
        assert_eq!(backward_only.phases(), vec![Direction::Backward]);
        assert_eq!(backward_only.direction_label(), "backward");
    }

    #[test]
    fn max_models_counts_the_reference_fit_and_every_shrinking_round() {
        // 3 candidates, one phase: 3 + 2 + 1 fits, plus the reference
        assert_eq!(max_models_for(3, 1), 7);
        assert_eq!(max_models_for(3, 2), 13);
        assert_eq!(max_models_for(0, 2), 1);
    }

    #[test]
    fn options_validation_rejects_bad_inputs() {
        let mut o = ScmOptions {
            direction: vec![],
            ..Default::default()
        };
        assert!(o.validate().is_err());

        o.direction = vec![Direction::Forward, Direction::Forward];
        assert!(o.validate().is_err());

        o.direction = vec![Direction::Forward];
        o.forward_alpha = 0.0;
        assert!(o.validate().is_err());

        o.forward_alpha = 0.05;
        o.num_rounds = Some(0);
        assert!(o.validate().is_err());
    }

    #[test]
    fn direction_serde_round_trip() {
        let opts = ScmOptions::default();
        let json = serde_json::to_string(&opts).unwrap();
        assert!(json.contains("\"forward\""));
        assert!(json.contains("\"backward\""));
        let back: ScmOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(back, opts);
    }

    #[test]
    fn plan_json_round_trip_and_digest_stability() {
        let plan = ScmPlan {
            schema_version: PLAN_SCHEMA_VERSION,
            created: "2026-08-19T00:00:00Z".into(),
            pharos_version: "0.5.1".into(),
            model: "model/nonmem/1001.mod".into(),
            out_dir: "model/nonmem/scm/1001".into(),
            candidates: vec![
                Candidate {
                    name: "WT_CL".into(),
                    theta: 6,
                },
                Candidate {
                    name: "CRCL_CL".into(),
                    theta: 7,
                },
            ],
            max_models: 7,
            options: ScmOptions::default(),
        };

        let json = plan.to_json().unwrap();
        let back = ScmPlan::from_json(&json).unwrap();
        assert_eq!(back, plan);
        assert_eq!(back.digest(), plan.digest());

        // num_rounds is run control, not search-defining
        let mut capped = plan.clone();
        capped.options.num_rounds = Some(2);
        assert_eq!(capped.digest(), plan.digest());

        // but alphas are search-defining
        let mut changed = plan.clone();
        changed.options.forward_alpha = 0.01;
        assert_ne!(changed.digest(), plan.digest());
    }

    #[test]
    fn newer_schema_version_is_rejected() {
        let plan = ScmPlan {
            schema_version: PLAN_SCHEMA_VERSION + 1,
            created: String::new(),
            pharos_version: String::new(),
            model: "m.mod".into(),
            out_dir: "scm/m".into(),
            candidates: vec![],
            max_models: 0,
            options: ScmOptions::default(),
        };
        let json = serde_json::to_string(&plan).unwrap();
        assert!(ScmPlan::from_json(&json).is_err());
    }

    #[test]
    fn sanitize_names() {
        assert_eq!(sanitize_name("WT_CL"), "wt_cl");
        assert_eq!(sanitize_name("CRCL/CL"), "crcl_cl");
    }
}
