//! Stepwise covariate modeling (SCM).
//!
//! The SCM process is driven by a `plan.json`: [`plan::build_plan`]
//! resolves the candidates the caller names against the `$THETA` records of
//! a user-authored initial model (each name from a `$THETA` comment
//! naming exactly one theta), [`driver::run_scm`] executes the SCM process
//! round by round with resumable state in `scm_state.json`, and
//! [`summary::read_summary`] reports on an SCM process wherever it currently
//! stands (`scm status` is its brief rendering, `scm summary` the full one).
//!
//! A round the numbers cannot decide — two candidates with an identical
//! p-value AND an identical ΔOFV — is not resolved by a tie-break rule: the
//! SCM process records both scores, pauses, and waits for the user to name the
//! winner (`scm run --choose <candidate>`, see [`state::PendingTie`]).
//!
//! Each round leaves a record behind as it concludes: a `round_summary.json`
//! / `.md` in its own round directory, a `pharos_summary.json` in every
//! finished run's directory, and a freshly rewritten `scm_summary.{json,md}`
//! in the SCM process's out_dir — so the on-disk record always matches the
//! state, not just at the end of the SCM process. `scm summary` builds the
//! same record on demand, so the files and the screen never disagree.
//!
//! Candidates are tracked by the state's roster ([`roster`]): a candidate
//! that has never won a round can be dropped from the plan and the SCM
//! process carries on without it, keeping every earlier round as it was, and
//! a candidate's initial estimate or bounds can be retuned mid-process —
//! the usual fix when a round fails on them — with the candidate refitted in
//! the round that is open and concluded rounds left as they are.

pub mod config;
pub mod driver;
pub mod plan;
pub mod progress;
pub mod project;
pub mod roster;
pub mod round;
pub mod score;
#[cfg(test)]
mod snapshot_tests;
pub mod state;
pub mod summary;
#[cfg(test)]
pub(crate) mod test_support;

use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use fs_err as fs;
use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::{Deserialize, Serialize};

pub use config::{
    CONFIG_SUFFIX, ScmConfig, ScmInit, ScmPlanOverrides, build_plan_from_config, config_path_for,
    init_scm, out_dir_for,
};
pub use driver::{FitExecutor, LocalExecutor, RunControls, run_scm, run_scm_with};
pub use plan::{BuiltPlan, build_plan};
pub use progress::{PlanChange, PlanContext};
pub use project::RunSettings;
pub use roster::{
    CandidateChange, Compatibility, Removal, Retune, Retuning, RosterEntry, compatibility,
    diff_candidates,
};
pub use round::{reconcile_round_with_disk, reconcile_state_with_disk};
pub use state::{
    CandidateRecord, CandidateStatus, PendingTie, RoundRecord, ScmRunStatus, ScmState,
};
pub use summary::{
    CandidateSummary, RoundSummary, ScmSummary, SummaryOptions, read_summary, write_round_summary,
};

pub const PLAN_FILENAME: &str = "plan.json";
pub const STATE_FILENAME: &str = "scm_state.json";
/// Written into each round directory when the round concludes.
pub const ROUND_SUMMARY_JSON: &str = "round_summary.json";
pub const ROUND_SUMMARY_MD: &str = "round_summary.md";
/// Per-run `pharos nonmem summary` output written into each run directory.
pub const RUN_SUMMARY_FILENAME: &str = "pharos_summary.json";
/// The process-level summary rewritten in the out_dir after every round, as
/// the record itself and as the markdown `scm summary` render.
pub const SCM_SUMMARY_FILENAME: &str = "scm_summary.json";
pub const SCM_SUMMARY_MD: &str = "scm_summary.md";
/// Schema 2: candidates carry `initial` (was `init`) and `fixed` (was
/// `off`); the plan
/// digest no longer covers the candidate list (the state's roster does).
pub const PLAN_SCHEMA_VERSION: u32 = 2;
/// Name of the pseudo-round holding the reference fit (not an SCM round).
pub const REFERENCE_ROUND: &str = "reference";
/// Stands in for a round's reference model when there isn't one (the
/// reference round itself).
pub const NO_REFERENCE: &str = "-";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
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
    /// Whether the final model is re-fitted with the covariance step on
    /// once the SCM process finishes. On by default: the SCM process picks
    /// the covariates, and the final fit is what reports their estimates
    /// with standard errors.
    pub final_cov_step: bool,
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
            final_cov_step: true,
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

/// A theta bound as NM-TRAN spells it: a number, or `INF` / `-INF` for the
/// infinite bounds the parser reads `-INF` and `INF` into (Rust would print
/// those as `-inf` / `inf`).
fn nmtran_number(value: f64) -> String {
    if value == f64::INFINITY {
        "INF".to_string()
    } else if value == f64::NEG_INFINITY {
        "-INF".to_string()
    } else {
        value.to_string()
    }
}

/// A `$THETA` value spec: bounds, initial estimate and FIX flag, detached
/// from any model so it can be built, validated and rendered on its own.
/// `Display` spells it the way NM-TRAN reads it.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ThetaSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lower: Option<f64>,
    pub init: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upper: Option<f64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fixed: bool,
}

impl ThetaSpec {
    /// A theta pinned at `value`: `(value FIX)`.
    pub fn fixed_at(value: f64) -> Self {
        Self {
            init: value,
            fixed: true,
            ..Default::default()
        }
    }

    /// A free theta under the given bounds.
    pub fn bounded(lower: Option<f64>, init: f64, upper: Option<f64>) -> Self {
        Self {
            lower,
            init,
            upper,
            fixed: false,
        }
    }

    /// Whether `v` sits strictly inside the bounds (an unbounded side always
    /// passes).
    pub fn contains(&self, v: f64) -> bool {
        self.lower.is_none_or(|l| v > l) && self.upper.is_none_or(|u| v < u)
    }

    /// Check the spec against NM-TRAN's rules: every value finite (bounds may
    /// be infinite), lower below upper, and a free theta's initial estimate
    /// strictly inside its bounds. `who` names the theta in the message.
    pub fn validate(&self, who: &str) -> Result<()> {
        if !self.init.is_finite() {
            bail!(
                "{who}: initial estimate must be a finite number, got {}",
                self.init
            );
        }
        for (label, value) in [("lower", self.lower), ("upper", self.upper)] {
            if let Some(v) = value
                && v.is_nan()
            {
                bail!("{who}: {label} bound must be a number, got {v}");
            }
        }
        if let (Some(lower), Some(upper)) = (self.lower, self.upper)
            && lower >= upper
        {
            bail!("{who}: lower ({lower}) must be below upper ({upper})");
        }
        if !self.fixed && !self.contains(self.init) {
            if let Some(lower) = self.lower
                && self.init <= lower
            {
                bail!(
                    "{who}: initial ({}) must be above lower ({lower}); NM-TRAN rejects an \
                     initial estimate at or outside its bounds",
                    self.init
                );
            }
            if let Some(upper) = self.upper {
                bail!(
                    "{who}: initial ({}) must be below upper ({upper}); NM-TRAN rejects an \
                     initial estimate at or outside its bounds",
                    self.init
                );
            }
        }
        Ok(())
    }

    /// The bounds alone, as `(0, INF)`, `(-INF, 2)`, `(0, 2)`; `None` when
    /// unbounded.
    pub fn bounds_label(&self) -> Option<String> {
        match (self.lower, self.upper) {
            (None, None) => None,
            (lower, upper) => Some(format!(
                "({}, {})",
                nmtran_number(lower.unwrap_or(f64::NEG_INFINITY)),
                nmtran_number(upper.unwrap_or(f64::INFINITY))
            )),
        }
    }
}

impl fmt::Display for ThetaSpec {
    /// `0.1` | `(0, 0.1)` | `(-INF, 0.1, 5)` | `(1 FIX)` | `(0, 1, 5) FIX`.
    /// An upper bound cannot be spelled without a lower one, so a spec with
    /// only an upper bound writes `-INF` for the lower.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let init = nmtran_number(self.init);
        match (self.lower, self.upper, self.fixed) {
            (None, None, false) => f.write_str(&init),
            (None, None, true) => write!(f, "({init} FIX)"),
            (Some(lower), None, fixed) => {
                write!(f, "({}, {init})", nmtran_number(lower))?;
                if fixed {
                    f.write_str(" FIX")?;
                }
                Ok(())
            }
            (lower, Some(upper), fixed) => {
                let lower = lower.unwrap_or(f64::NEG_INFINITY);
                write!(
                    f,
                    "({}, {init}, {})",
                    nmtran_number(lower),
                    nmtran_number(upper)
                )?;
                if fixed {
                    f.write_str(" FIX")?;
                }
                Ok(())
            }
        }
    }
}

/// A covariate effect candidate: one theta, named in the config's
/// `[covariates]` section and resolved against the initial model's
/// `$THETA` records.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    /// The name the initial model's `$THETA` record gives the theta, e.g.
    /// `WT_CL`. Taken from the model rather than from the config's own
    /// spelling, so the several names one theta may answer to all resolve
    /// to one candidate — this is the roster's identity key.
    pub name: String,
    /// 1-based THETA number in the initial model.
    pub theta: usize,
    /// Initial estimate the effect takes the first time it is tested.
    /// Resolved at plan time: the config row's own `initial`, else the
    /// initial model's own estimate when it differs from `fixed`, else the
    /// section default. A schema-1 plan.json spells this `init`.
    #[serde(alias = "init")]
    pub initial: f64,
    /// The value the theta is fixed at in every model that holds the effect
    /// out: 0 for the usual additive-in-theta forms (power, proportional,
    /// exponential), 1 for a fold-change form such as `THETA(n)**SEX`. A
    /// plan.json written before the rename spells this `off`.
    #[serde(default, alias = "off")]
    pub fixed: f64,
    /// Lower bound the theta is estimated under whenever the effect is in
    /// the model; `None` leaves it unbounded. Resolved at plan time: the
    /// config row's own `lower`, else the section default, else the bound
    /// the initial model's own `$THETA` spec carries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lower: Option<f64>,
    /// Upper bound, resolved the same way as [`Candidate::lower`]. NM-TRAN
    /// cannot spell an upper bound without a lower one, so a candidate with
    /// only an upper bound is written `(-INF, init, upper)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upper: Option<f64>,
}

impl Candidate {
    /// The `$THETA` spec the effect is estimated under when it is in the
    /// model: free at [`Candidate::initial`] within the candidate's bounds.
    pub fn released_spec(&self) -> ThetaSpec {
        ThetaSpec::bounded(self.lower, self.initial, self.upper)
    }

    /// The `$THETA` spec pinning the effect when it is held out: `(fixed FIX)`.
    pub fn held_out_spec(&self) -> ThetaSpec {
        ThetaSpec::fixed_at(self.fixed)
    }

    /// The bounds as the plan renderings show them: `(0, INF)`,
    /// `(-INF, 2)`, `(0, 2)`; `None` when the theta is unbounded.
    pub fn bounds_label(&self) -> Option<String> {
        self.released_spec().bounds_label()
    }
}

/// One entry of the config's `[covariates] effects` array: a candidate by
/// theta name, with the values the row gives it explicitly. A missing value
/// falls back to the section's own default ([`Covariates`]), then to the
/// built-in one.
///
/// Deserializes from either spelling an `effects` array allows: a bare
/// name (`"WT_CL"`), or a row (`{ name = "WT_CL", initial = 0.2, fixed = 0 }`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CovariateRequest {
    pub name: String,
    pub initial: Option<f64>,
    pub fixed: Option<f64>,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
}

impl CovariateRequest {
    /// The initial estimate an effect takes the first time it is tested,
    /// unless its row, the section or the initial model says otherwise.
    pub const INITIAL: f64 = 0.1;
    /// What a held-out effect's theta is fixed at, unless its row or the
    /// section says otherwise.
    pub const FIXED: f64 = 0.0;

    pub fn named(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }
}

impl<'de> Deserialize<'de> for CovariateRequest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        /// The long form of an entry, with serde's own field checking.
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Row {
            name: String,
            initial: Option<f64>,
            /// Spelled `off` before the rename; that spelling is still accepted.
            #[serde(alias = "off")]
            fixed: Option<f64>,
            lower: Option<f64>,
            upper: Option<f64>,
        }

        struct EntryVisitor;

        impl<'de> Visitor<'de> for EntryVisitor {
            type Value = CovariateRequest;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(
                    "a theta name (\"WT_CL\") or a row ({ name = \"WT_CL\", initial = 0.1, fixed = 0 })",
                )
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(CovariateRequest::named(v))
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                let row = Row::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(CovariateRequest {
                    name: row.name,
                    initial: row.initial,
                    fixed: row.fixed,
                    lower: row.lower,
                    upper: row.upper,
                })
            }

            // THETA numbers used to select candidates; a config written
            // against an older pharos gets a message saying what to write.
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Err(E::custom(theta_number_message(&v.to_string())))
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Err(E::custom(theta_number_message(&v.to_string())))
            }
        }

        deserializer.deserialize_any(EntryVisitor)
    }
}

fn theta_number_message(found: &str) -> String {
    format!(
        "covariates are named, not numbered (found {found}); \
         write e.g. effects = [\"WT_CL\", \"CRCL_CL\"] under [covariates], naming each \
         candidate theta the way its $THETA record names it"
    )
}

/// The config's `[covariates]` table, and the request `build_plan` takes:
/// section-wide defaults and the effects to test.
///
/// Each entry of `effects` is either a bare theta name (`"WT_CL"`), which
/// takes every default, or a row (`{ name = "SEXEFF_CL", initial = 1.2,
/// fixed = 1 }`) that overrides whichever of them it spells out. The two
/// forms mix freely in one array. A default the section leaves unset falls
/// back to [`CovariateRequest::INITIAL`] / [`CovariateRequest::FIXED`]; an
/// unset bound leaves each candidate the bound its `$THETA` spec in the
/// initial model carries.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Covariates {
    /// Default initial estimate for an effect the first time it is tested.
    pub initial: Option<f64>,
    /// Default value a held-out effect's theta is fixed at. Spelled `off`
    /// before the rename; that spelling is still accepted.
    #[serde(alias = "off")]
    pub fixed: Option<f64>,
    /// Default lower bound for an effect while it is in the model.
    pub lower: Option<f64>,
    /// Default upper bound; see [`Covariates::lower`].
    pub upper: Option<f64>,
    /// The candidate effects, as names or rows.
    #[serde(default)]
    pub effects: Vec<CovariateRequest>,
}

impl Covariates {
    /// Effects by name alone, every value at the section default.
    pub fn named(names: &[&str]) -> Self {
        Self {
            effects: names.iter().map(|n| CovariateRequest::named(n)).collect(),
            ..Default::default()
        }
    }

    /// The section's `initial`, or the built-in default.
    pub fn default_initial(&self) -> f64 {
        self.initial.unwrap_or(CovariateRequest::INITIAL)
    }

    /// The section's `fixed`, or the built-in default.
    pub fn default_fixed(&self) -> f64 {
        self.fixed.unwrap_or(CovariateRequest::FIXED)
    }
}

/// The plan.json: everything needed to run the SCM process.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScmPlan {
    pub schema_version: u32,
    pub created: String,
    pub pharos_version: String,
    /// Path to the initial model, relative to the pharos project root — so a
    /// plan reads the same whichever directory the command ran in. A model
    /// outside any project is stored as given. Resolve it with
    /// [`ScmPlan::model_path`] rather than reading this field as a path.
    pub model: String,
    /// Directory the SCM process writes into; plan.json lives here. Relative
    /// to the project root, like [`ScmPlan::model`]; resolve it with
    /// [`ScmPlan::out_dir_path`].
    pub out_dir: String,
    pub candidates: Vec<Candidate>,
    /// Maximum possible number of models the SCM process can fit — the reference
    /// fit plus the worst case of every phase, excluding retries. Derived
    /// from the candidates and direction (see [`ScmPlan::computed_max_models`]);
    /// a plan.json written before this field existed loads with it filled in.
    #[serde(default)]
    pub max_models: usize,
    pub options: ScmOptions,
    /// The project root `model` and `out_dir` are relative to. Filled in when
    /// the plan is built, and when it is loaded from the project it lives in;
    /// never serialized, because it is a property of where the project sits,
    /// not of the SCM process. Empty when there is no project root, in which
    /// case the stored paths stand on their own.
    #[serde(skip)]
    pub root: PathBuf,
}

/// How a path is written into plan.json: relative to the project root, so
/// that neither the plan nor its digest depends on the directory a command
/// ran in. A path outside the project — or one with no root to measure from —
/// is stored as given.
pub fn path_for_plan(path: impl AsRef<Path>, root: &Path) -> String {
    let path = path.as_ref();
    let as_given = || path.to_string_lossy().into_owned();
    if root.as_os_str().is_empty() {
        return as_given();
    }
    std::path::absolute(path)
        .ok()
        .and_then(|abs| ::config::to_root_relative(utils::normalize_path(&abs), root).ok())
        .unwrap_or_else(as_given)
}

impl ScmPlan {
    pub fn model_path(&self) -> PathBuf {
        self.root.join(&self.model)
    }

    /// This plan's worst-case model count; see [`max_models_for`].
    pub fn computed_max_models(&self) -> usize {
        max_models_for(self.candidates.len(), self.options.phases().len())
    }

    pub fn out_dir_path(&self) -> PathBuf {
        self.root.join(&self.out_dir)
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
        let mut options = serde_json::json!({
            // overwrite/num_rounds are run-control, not SCM-defining
            "direction": self.options.direction,
            "forward_alpha": self.options.forward_alpha,
            "backward_alpha": self.options.backward_alpha,
            "max_retries": self.options.max_retries,
            "cov_step": self.options.cov_step,
        });
        // The final re-fit only defines the SCM process when it is on: with
        // it off nothing is fitted past the rounds, so the digest stays the
        // one a plan without the re-fit has always hashed to.
        if self.options.final_cov_step {
            options["final_cov_step"] = serde_json::Value::Bool(true);
        }
        let payload = serde_json::json!({
            "model": self.model,
            "out_dir": self.out_dir,
            "options": options,
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
        let mut plan = Self::from_json(&content)?;
        // The paths in the file are relative to the project root, so it is
        // found from where the file itself lives — the plan resolves the same
        // whatever directory the command was run in.
        plan.root = ::config::find_config_dir_from(path.parent().unwrap_or(Path::new(".")))?
            .unwrap_or_default();
        Ok(plan)
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
        out.add(format!(
            "final fit  : {}",
            if o.final_cov_step {
                "re-fit the final model with the cov step on"
            } else {
                "final model written, not fitted"
            }
        ));
        if let Some(n) = o.num_rounds {
            out.add(format!("num rounds : pause after {n} (resumable)"));
        }
        out.add("candidates :");
        // The bounds column only earns its width when something is bounded.
        let bounded = self.candidates.iter().any(|c| c.bounds_label().is_some());
        let row = |name: &str, theta: String, initial: String, fixed: String, bounds: String| {
            let mut line = format!("  {name:<12} {theta:<9} {initial:>8}  {fixed:>5}");
            if bounded {
                line.push_str(&format!("  {bounds:>14}"));
            }
            line
        };
        out.add(row(
            "name",
            "theta".to_string(),
            "initial".to_string(),
            "FIXED".to_string(),
            "bounds".to_string(),
        ));
        for c in &self.candidates {
            out.add(row(
                &c.name,
                format!("THETA({})", c.theta),
                c.initial.to_string(),
                c.fixed.to_string(),
                c.bounds_label().unwrap_or_else(|| "-".to_string()),
            ));
        }
        out.add("             (initial: the effect's initial estimate the first time it is tested; FIXED: what it is fixed at when held out)");
        if bounded {
            out.add(
                "             (bounds: the $THETA bounds the effect is estimated under while it is in the model)",
            );
        }
        out.add(format!(
            "max models : {} (incl. reference fit, excl. retries)",
            self.max_models
        ));
        ctx.render_into(&mut out);
        out.finish()
    }
}

/// Accumulates the lines of a rendered report. Every SCM rendering — the
/// plan, the summary, a round, a round summary — builds its
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
        assert!(o.final_cov_step);
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
                    fixed: 0.0,
                    ..Default::default()
                },
                Candidate {
                    name: "CRCL_CL".into(),
                    theta: 7,
                    initial: 0.1,
                    fixed: 1.0,
                    ..Default::default()
                },
            ],
            max_models: 7,
            options: ScmOptions::default(),
            root: PathBuf::new(),
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

        // the final re-fit is SCM-defining, and turning it off leaves the
        // digest a plan without it has always hashed to
        let mut no_final = plan.clone();
        no_final.options.final_cov_step = false;
        assert_ne!(no_final.digest(), plan.digest());

        // the candidate list is tracked by the state's roster, not the digest
        let mut fewer = plan.clone();
        fewer.candidates.pop();
        assert_eq!(fewer.digest(), plan.digest());
    }

    /// A schema-1 plan.json spells the initial estimate `init` and has no
    /// `fixed`; it loads with `fixed = 0` and is otherwise unchanged.
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
        assert_eq!(plan.candidates[0].fixed, 0.0);
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
            root: PathBuf::new(),
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
