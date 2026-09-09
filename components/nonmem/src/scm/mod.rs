//! Stepwise covariate modeling (SCM).
//!
//! The SCM process is driven by a `plan.json`: [`plan::build_plan`]
//! validates the candidates the caller names by `$PK` term against a
//! user-authored template control stream (each candidate effect its own `$PK`
//! term over a single theta), [`driver::run_scm`] executes the SCM process
//! round by round with resumable state in `scm_state.json`, and [`status::read_status`]
//! reports on an SCM process wherever it currently stands.
//!
//! A round the numbers cannot decide — two candidates with an identical
//! p-value AND an identical ΔOFV — is not resolved by a tie-break rule: the
//! SCM process records both scores, pauses, and waits for the user to name the
//! winner (`scm run --choose <candidate>`, see [`state::PendingTie`]).
//!
//! Each round leaves a record behind as it concludes: a `round_summary.json`
//! / `.md` in its own round directory, a `pharos_summary.json` in every
//! finished run's directory, and freshly rewritten `scm_summary.json` and
//! decision-log files in the SCM process's out_dir — so the on-disk record
//! always matches the state, not just at the end of the SCM process.
//! [`summary::read_summary`] builds the same record on demand for
//! `scm summary`, so the files and the screen never disagree.
//!
//! Candidates are tracked by the state's roster ([`roster`]): a candidate
//! that has never won a round can be dropped from the plan and the SCM
//! process carries on without it, keeping every earlier round as it was.

pub mod config;
pub mod driver;
pub mod log;
pub mod plan;
pub mod progress;
pub mod roster;
pub mod round;
pub mod score;
#[cfg(test)]
mod snapshot_tests;
pub mod state;
pub mod status;
pub mod summary;
#[cfg(test)]
pub(crate) mod test_support;

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result, bail};
use fs_err as fs;
use serde::{Deserialize, Serialize};

pub use config::{
    CONFIG_SUFFIX, ScmConfig, ScmInit, ScmPlanOverrides, build_plan_from_config, config_path_for,
    init_scm, out_dir_for,
};
pub use driver::{FitExecutor, LocalExecutor, ScmOutcome, run_scm};
pub use log::{DecisionLogRow, decision_log_rows};
pub use plan::{BuiltPlan, build_plan};
pub use progress::{CurrentRound, PlanChange, PlanContext, PlanProgress};
pub use roster::{Compatibility, Removal, RosterEntry, compatibility};
pub use round::{reconcile_round_with_disk, reconcile_state_with_disk};
pub use state::{
    CandidateRecord, CandidateStatus, PendingTie, RoundRecord, ScmRunStatus, ScmState,
};
pub use status::{ScmStatus, read_status};
pub use summary::{
    CandidateSummary, MatrixValue, RoundSummary, ScmSummary, SortKey, SummaryFormat,
    SummaryOptions, read_summary, write_round_summary,
};

pub const PLAN_FILENAME: &str = "plan.json";
pub const STATE_FILENAME: &str = "scm_state.json";
pub const DECISION_LOG_CSV: &str = "scm_decision_log.csv";
pub const DECISION_LOG_MD: &str = "scm_decision_log.md";
/// Written into each round directory when the round concludes.
pub const ROUND_SUMMARY_JSON: &str = "round_summary.json";
pub const ROUND_SUMMARY_MD: &str = "round_summary.md";
/// Per-run `pharos nonmem summary` output written into each run directory.
pub const RUN_SUMMARY_FILENAME: &str = "pharos_summary.json";
/// The process-level summary rewritten in the out_dir after every round.
pub const SCM_SUMMARY_FILENAME: &str = "scm_summary.json";
/// Schema 2: candidates carry `initial` (was `init`) and `off`; the plan
/// digest no longer covers the candidate list (the state's roster does).
pub const PLAN_SCHEMA_VERSION: u32 = 2;
/// Name of the pseudo-round holding the reference fit (not an SCM round).
pub const REFERENCE_ROUND: &str = "reference";
/// Stands in for a round's reference model when there isn't one (the
/// reference round itself).
pub const NO_REFERENCE: &str = "-";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
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

/// SCM process options carried in the plan — everything that defines the SCM process
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
    /// Pause the SCM process after this many rounds per invocation (resumable).
    pub num_rounds: Option<usize>,
    /// Retries per failed fit; each retry starts from the previous attempt's
    /// estimates (never jittered).
    pub max_retries: usize,
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
            cov_step: false,
            overwrite: false,
        }
    }
}

impl ScmOptions {
    /// The phases this SCM process runs, in run order: forward always precedes
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
        Ok(())
    }
}

/// A covariate effect candidate: one `$PK` term over one theta, named in
/// the config's `[covariates]` section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    /// The name of the `$PK` term, e.g. `WT_CL`.
    pub name: String,
    /// 1-based THETA number in the template.
    pub theta: usize,
    /// Initial estimate the effect is released at the first time it is
    /// tested. Resolved at plan time: the config row's own `initial`, else
    /// the template's initial estimate when it differs from `off`, else the
    /// section default. A schema-1 plan.json spells this `init`.
    #[serde(alias = "init")]
    pub initial: f64,
    /// The value the theta is fixed at in every model that holds the effect
    /// out: 0 for the usual additive-in-theta forms (power, proportional,
    /// exponential), 1 for a fold-change form such as `THETA(n)**SEX`.
    #[serde(default)]
    pub off: f64,
}

impl Candidate {
    /// Whether the template's theta, as authored, is already the held-out
    /// spelling of this effect (`(off FIX)`).
    pub fn is_held_out_spec(&self, fixed: bool, init: f64) -> bool {
        fixed && init == self.off
    }
}

/// One entry of the config's `[covariates] effects` array: a candidate by
/// `$PK` term name, with the values the row gives it explicitly. Missing
/// values fall back to the section's [`CovariateDefaults`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CovariateRequest {
    pub name: String,
    pub initial: Option<f64>,
    pub off: Option<f64>,
}

impl CovariateRequest {
    pub fn named(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }
}

/// The `initial` / `off` defaults of the config's `[covariates]` section.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CovariateDefaults {
    /// Where an effect is released the first time it is tested, unless its
    /// row or the template says otherwise.
    pub initial: f64,
    /// What a held-out effect's theta is fixed at, unless its row says
    /// otherwise.
    pub off: f64,
}

impl Default for CovariateDefaults {
    fn default() -> Self {
        Self {
            initial: 0.1,
            off: 0.0,
        }
    }
}

/// The covariates request `build_plan` takes: the section defaults and the
/// effects to test.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Covariates {
    pub defaults: CovariateDefaults,
    pub effects: Vec<CovariateRequest>,
}

impl Covariates {
    /// Effects by name alone, every value at the section default.
    pub fn named(names: &[&str]) -> Self {
        Self {
            defaults: CovariateDefaults::default(),
            effects: names.iter().map(|n| CovariateRequest::named(n)).collect(),
        }
    }
}

/// The plan.json: everything needed to run the SCM process.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScmPlan {
    pub schema_version: u32,
    pub created: String,
    pub pharos_version: String,
    /// Path to the template control stream, as given (typically relative to
    /// the pharos project root, which is where scm commands run from).
    pub model: String,
    /// Directory the SCM process writes into; plan.json lives here.
    pub out_dir: String,
    pub candidates: Vec<Candidate>,
    /// Maximum possible number of models the SCM process can fit — the reference
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

    /// Stable digest of the SCM-defining options, used to detect that
    /// on-disk state belongs to a different plan. The candidate list is
    /// deliberately not part of it: the state's roster tracks candidates on
    /// their own, so a candidate that never won a round can be removed
    /// without the whole SCM process reading as a different plan (see
    /// [`roster::compatibility`]).
    pub fn digest(&self) -> String {
        let payload = serde_json::json!({
            "model": self.model,
            "out_dir": self.out_dir,
            "options": {
                // overwrite/num_rounds are run-control, not SCM-defining
                "direction": self.options.direction,
                "forward_alpha": self.options.forward_alpha,
                "backward_alpha": self.options.backward_alpha,
                "max_retries": self.options.max_retries,
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
        // An older plan loads into the current schema (`init` -> `initial`,
        // `off` defaulted) and is written back as such.
        plan.schema_version = PLAN_SCHEMA_VERSION;
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
        self.render_text_with(&PlanContext::default())
    }

    /// [`ScmPlan::render_text`] with the out_dir's own history appended:
    /// where the SCM process already living there stands, and what this plan
    /// changed about the one it replaces. An empty context renders exactly
    /// the plan.
    pub fn render_text_with(&self, ctx: &PlanContext) -> String {
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
        out.add(format!(
            "  {:<12} {:<9} {:>8}  {:>5}",
            "name", "theta", "initial", "off"
        ));
        for c in &self.candidates {
            out.add(format!(
                "  {:<12} {:<9} {:>8}  {:>5}",
                c.name,
                format!("THETA({})", c.theta),
                c.initial,
                c.off
            ));
        }
        out.add("             (initial: where the effect is released when first tested; off: what it is fixed at when held out)");
        out.add(format!(
            "max models : {} (incl. reference fit, excl. retries)",
            self.max_models
        ));
        ctx.render_into(&mut out);
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

    /// Append one line. Trailing whitespace is dropped, so a column that
    /// happens to be empty at the end of a line leaves nothing behind.
    pub(crate) fn add(&mut self, line: impl AsRef<str>) {
        self.0.push_str(line.as_ref().trim_end());
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

/// Worst case number of models an SCM process fits: the single reference fit plus,
/// for each phase, one model per candidate in the first round, one fewer in
/// the next, and so on down to one — n(n+1)/2 per phase. Excludes retries.
/// (Forward starts from the base model, backward-only from the full model,
/// and a forward -> backward SCM process re-uses the forward winner as the
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
                    initial: 0.1,
                    off: 0.0,
                },
                Candidate {
                    name: "CRCL_CL".into(),
                    theta: 7,
                    initial: 0.1,
                    off: 1.0,
                },
            ],
            max_models: 7,
            options: ScmOptions::default(),
        };

        let json = plan.to_json().unwrap();
        let back = ScmPlan::from_json(&json).unwrap();
        assert_eq!(back, plan);
        assert_eq!(back.digest(), plan.digest());

        // num_rounds is run control, not SCM-defining
        let mut capped = plan.clone();
        capped.options.num_rounds = Some(2);
        assert_eq!(capped.digest(), plan.digest());

        // but alphas are SCM-defining
        let mut changed = plan.clone();
        changed.options.forward_alpha = 0.01;
        assert_ne!(changed.digest(), plan.digest());

        // the candidate list is tracked by the state's roster, not the digest
        let mut fewer = plan.clone();
        fewer.candidates.pop();
        assert_eq!(fewer.digest(), plan.digest());
    }

    /// A schema-1 plan.json spells the release value `init` and has no
    /// `off`; it loads with `off = 0` and is otherwise unchanged.
    #[test]
    fn schema_one_plan_json_loads() {
        let json = r#"{
  "schema_version": 1, "created": "x", "pharos_version": "0.5.1",
  "model": "1001.mod", "out_dir": "scm/1001",
  "candidates": [{"name": "WT_CL", "theta": 4, "init": 0.4}],
  "max_models": 3,
  "options": {"direction": ["forward"], "forward_alpha": 0.05, "backward_alpha": 0.001,
              "num_rounds": null, "max_retries": 3, "release_init": 0.1, "cov_step": false, "overwrite": false}
}"#;
        let plan = ScmPlan::from_json(json).unwrap();
        assert_eq!(plan.candidates[0].initial, 0.4);
        assert_eq!(plan.candidates[0].off, 0.0);
        assert_eq!(plan.options.max_retries, 3);
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
