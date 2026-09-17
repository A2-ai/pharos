//! Stepwise covariate modeling (SCM).

pub mod config;
pub mod driver;
pub mod plan;
pub mod progress;
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

use ::config::NonmemConfig;
use anyhow::{Context, Result, bail};
use fs_err as fs;
use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::{Deserialize, Serialize};

use crate::ModelLayout;

pub use config::{
    CONFIG_SUFFIX, ScmConfig, ScmInit, ScmPlanOverrides, build_plan_from_config, config_path_for,
    init_scm,
};
pub use driver::{FitExecutor, LocalExecutor, run_scm};
pub use plan::{BuiltPlan, build_plan};
pub use progress::{PlanChange, PlanContext};
pub use roster::{
    CandidateChange, Compatibility, Removal, Retune, Retuning, RosterEntry, compatibility,
    diff_candidates,
};
pub use round::{reconcile_round_with_disk, reconcile_state_with_disk};
pub use state::{CandidateRecord, CandidateStatus, RoundRecord, ScmRunStatus, ScmState};
pub use summary::{
    CandidateSummary, RoundSummary, ScmSummary, SummaryOptions, read_summary, write_round_summary,
};

pub const PLAN_FILENAME: &str = "plan.json";
pub const STATE_FILENAME: &str = "scm_state.json";
pub const ROUND_SUMMARY_JSON: &str = "round_summary.json";
pub const ROUND_SUMMARY_MD: &str = "round_summary.md";
pub const RUN_SUMMARY_FILENAME: &str = "pharos_summary.json";
pub const SCM_SUMMARY_FILENAME: &str = "scm_summary.json";
pub const SCM_SUMMARY_MD: &str = "scm_summary.md";
pub const REFERENCE_ROUND: &str = "reference";
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScmOptions {
    pub direction: Vec<Direction>,
    pub forward_alpha: f64,
    pub backward_alpha: f64,
    pub num_rounds: Option<usize>,
    pub max_retries: usize,
    pub cov_step: bool,
    pub final_cov_step: bool,
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
        }
    }
}

impl ScmOptions {
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

/// A theta bound as NM-TRAN spells it
fn nmtran_number(value: f64) -> String {
    if value == f64::INFINITY {
        "INF".to_string()
    } else if value == f64::NEG_INFINITY {
        "-INF".to_string()
    } else {
        value.to_string()
    }
}

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
    pub fn fixed_at(value: f64) -> Self {
        Self {
            init: value,
            fixed: true,
            ..Default::default()
        }
    }

    pub fn bounded(lower: Option<f64>, init: f64, upper: Option<f64>) -> Self {
        Self {
            lower,
            init,
            upper,
            fixed: false,
        }
    }

    pub fn contains(&self, v: f64) -> bool {
        self.lower.is_none_or(|l| v > l) && self.upper.is_none_or(|u| v < u)
    }

    /// Check the spec against NM-TRAN's rules
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

impl From<&nonmem_parser::ThetaParameter> for ThetaSpec {
    /// The spec a parsed `$THETA` record carries, as authored.
    fn from(t: &nonmem_parser::ThetaParameter) -> Self {
        Self {
            lower: t.lower,
            init: t.init,
            upper: t.upper,
            fixed: t.fixed,
        }
    }
}

impl fmt::Display for ThetaSpec {
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

/// A covariate effect candidate
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    pub name: String,
    /// 1-based THETA number in the initial model.
    pub theta: usize,
    pub initial: f64,
    #[serde(default)]
    pub fixed: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lower: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upper: Option<f64>,
}

impl Candidate {
    /// The `$THETA` spec the effect is estimated under when it is in the model
    pub fn released_spec(&self) -> ThetaSpec {
        ThetaSpec::bounded(self.lower, self.initial, self.upper)
    }

    pub fn held_out_spec(&self) -> ThetaSpec {
        ThetaSpec::fixed_at(self.fixed)
    }

    pub fn bounds_label(&self) -> Option<String> {
        self.released_spec().bounds_label()
    }
}

/// One entry of the config's `[covariates] effects` array
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CovariateRequest {
    pub name: String,
    pub initial: Option<f64>,
    pub fixed: Option<f64>,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
}

impl CovariateRequest {
    pub const INITIAL: f64 = 0.1;
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
        }

        deserializer.deserialize_any(EntryVisitor)
    }
}

/// The config's `[covariates]` table, and the request `build_plan` takes:
/// section-wide defaults and the effects to test.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Covariates {
    pub initial: Option<f64>,
    pub fixed: Option<f64>,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
    #[serde(default)]
    pub effects: Vec<CovariateRequest>,
}

impl Covariates {
    pub fn named(names: &[&str]) -> Self {
        Self {
            effects: names.iter().map(|n| CovariateRequest::named(n)).collect(),
            ..Default::default()
        }
    }

    pub fn default_initial(&self) -> f64 {
        self.initial.unwrap_or(CovariateRequest::INITIAL)
    }

    pub fn default_fixed(&self) -> f64 {
        self.fixed.unwrap_or(CovariateRequest::FIXED)
    }
}

/// The plan.json: everything needed to run the SCM process.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScmPlan {
    pub created: String,
    pub pharos_version: String,
    /// Path to the initial model, relative to the pharos project root
    pub model: String,
    /// Directory SCM process writes into; plan.json lives here. Relative to project root
    pub out_dir: String,
    pub candidates: Vec<Candidate>,
    pub options: ScmOptions,
    /// The project root `model` and `out_dir` are relative to.
    #[serde(skip)]
    pub root: PathBuf,
}

/// How a path is written into plan.json: relative to the project root
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
    /// on-disk state belongs to a different plan.
    pub fn digest(&self) -> String {
        let mut options = serde_json::json!({
            "direction": self.options.direction,
            "forward_alpha": self.options.forward_alpha,
            "backward_alpha": self.options.backward_alpha,
            "max_retries": self.options.max_retries,
            "cov_step": self.options.cov_step,
        });
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
        let plan: ScmPlan = serde_json::from_str(json).context("failed to parse SCM plan JSON")?;
        plan.options.validate()?;
        Ok(plan)
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read plan file {}", path.display()))?;
        let mut plan = Self::from_json(&content)?;
        plan.root = ::config::find_config_dir_from(path.parent().unwrap_or(Path::new(".")))?
            .unwrap_or_default();
        Ok(plan)
    }

    /// Human-readable rendering of the plan.
    pub fn render_text(&self) -> String {
        self.render_text_with(&PlanContext::default())
    }

    /// [`ScmPlan::render_text`] with the out_dir's own history appended:
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
            "on failure : retry up to {}x from the previous attempt's estimates, jittered {:.0}%",
            o.max_retries,
            round::RETRY_JITTER * 100.0
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
            max_models_for(self.candidates.len(), o.phases().len())
        ));
        ctx.render_into(&mut out);
        out.finish()
    }
}

/// Accumulates the lines of a rendered report
#[derive(Default)]
pub(crate) struct Lines(String);

impl Lines {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Append one line
    pub(crate) fn add(&mut self, line: impl AsRef<str>) {
        self.0.push_str(line.as_ref().trim_end());
        self.0.push('\n');
    }
    pub(crate) fn blank(&mut self) {
        self.0.push('\n');
    }
    pub(crate) fn finish(self) -> String {
        self.0
    }
}

/// Worst case number of models an SCM process fits, excluding retries
pub fn max_models_for(n_candidates: usize, n_phases: usize) -> usize {
    1 + n_phases * n_candidates * (n_candidates + 1) / 2
}

/// The `[nonmem]` settings of the pharos project `dir` sits in
pub fn project_config(dir: impl AsRef<Path>) -> Result<NonmemConfig> {
    let dir = dir.as_ref();
    let config_dir = ::config::find_config_dir_from(dir)?.ok_or_else(|| {
        anyhow::anyhow!(
            "no {} found in '{}' or any directory above it",
            ::config::CONFIG_FILENAME,
            dir.display()
        )
    })?;
    let config = ::config::Config::load(config_dir.join(::config::CONFIG_FILENAME))?;
    Ok(config.nonmem.unwrap_or_default())
}

/// Where an SCM process on `model` writes: `scm/<stem>/` beside the model.
pub fn default_out_dir(layout: &ModelLayout) -> PathBuf {
    layout.model_dir().join("scm").join(layout.stem())
}

pub(crate) fn ofv_suffix(ofv: Option<f64>) -> String {
    ofv.map(|o| format!(" (OFV {o:.3})")).unwrap_or_default()
}

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
    fn plan_json_round_trip_and_digest_stability() {
        let plan = ScmPlan {
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

    #[test]
    fn sanitize_names() {
        assert_eq!(sanitize_name("WT_CL"), "wt_cl");
        assert_eq!(sanitize_name("CRCL/CL"), "crcl_cl");
    }
}
