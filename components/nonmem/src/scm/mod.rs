//! Stepwise covariate modeling (SCM).

pub mod config;
pub mod driver;
pub mod gitignore;
pub mod interrupt;
pub mod live;
pub mod plan;
pub mod progress;
pub mod report;
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

use ::config::{NonmemConfig, ScmSettings};
use anyhow::{Result, bail};
use fs_err as fs;
use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::{Deserialize, Serialize};

use crate::ModelLayout;

pub use config::{
    CONFIG_SUFFIX, ScmConfig, ScmInit, ScmOverrides, build_plan_from_config, config_beside,
    init_scm, out_dir_of_config,
};
pub use driver::{FitExecutor, run_scm};
pub use gitignore::{DRIVER_LOG_FILENAME, render_gitignore, write_gitignore};
pub use interrupt::{
    INTERRUPTED_NOTE, Interrupted, install_interrupt_handler, interrupted, is_interrupted,
};
pub use plan::{BuiltPlan, build_plan};
pub use progress::{PlanChange, PlanContext};
pub use report::{Mark, report, report_dated, report_fit, report_in, report_start};
pub use roster::{
    CandidateChange, ChangeKind, Compatibility, Removal, Retune, Retuning, RosterEntry,
    compatibility, diff_candidates,
};
pub use round::reconcile_state_with_disk;
pub use state::{CandidateRecord, CandidateStatus, RoundRecord, ScmRunStatus, ScmState};
pub use status::{PlaceOf, ScmStatus, read_summary};
pub use summary::{CandidateSummary, RoundSummary, ScmSummary, SummaryOptions, write_records};

pub const STATE_FILENAME: &str = "scm_state.json";
pub const ROUND_SUMMARY_JSON: &str = "round_summary.json";
pub const ROUND_SUMMARY_MD: &str = "round_summary.md";
pub const RUN_SUMMARY_FILENAME: &str = "pharos_summary.json";
pub const SCM_SUMMARY_FILENAME: &str = "scm_summary.json";
pub const SCM_SUMMARY_MD: &str = "scm_summary.md";
pub const REFERENCE_ROUND: &str = "reference";
/// Where the forward model's covariance-step re-fit lives
pub const FORWARD_FINAL_DIR: &str = "forward_final";
pub const NO_REFERENCE: &str = "-";

/// `Display` for a fieldless enum serialized `rename_all = "lowercase"`: the
/// text is the variant name lowercased, so renderings and JSON never disagree.
macro_rules! lowercase_display {
    ($($ty:ty),* $(,)?) => {$(
        impl ::std::fmt::Display for $ty {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(&format!("{self:?}").to_ascii_lowercase())
            }
        }
    )*};
}
pub(crate) use lowercase_display;

lowercase_display!(Direction, CovariateType);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Forward,
    Backward,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScmOptions {
    pub direction: Vec<Direction>,
    pub forward_alpha: f64,
    pub backward_alpha: f64,
    pub num_rounds: Option<usize>,
    pub max_retries: usize,
    /// How far a retry perturbs the estimates it starts from, as a
    /// fraction of each (0.05 = 5%).
    pub retry_jitter: f64,
    pub cov_step: bool,
    pub final_cov_step: bool,
    /// Re-fit the forward phase's model with `$COVARIANCE` on as soon as
    /// forward selection ends. Only applies when both phases run; moot when
    /// `cov_step` already runs the step in every model.
    #[serde(default = "default_true")]
    pub forward_final_cov_step: bool,
}

fn default_true() -> bool {
    true
}

impl Default for ScmOptions {
    fn default() -> Self {
        Self {
            direction: vec![Direction::Forward, Direction::Backward],
            forward_alpha: 0.05,
            backward_alpha: 0.001,
            num_rounds: None,
            max_retries: 3,
            retry_jitter: round::RETRY_JITTER,
            cov_step: false,
            final_cov_step: true,
            forward_final_cov_step: true,
        }
    }
}

impl ScmOptions {
    /// Whether the forward model gets its own covariance-step fit: both
    /// phases run and the option is on. With `cov_step` on, the forward
    /// model has already run the step and is used as it is.
    pub fn fits_forward_final(&self) -> bool {
        self.forward_final_cov_step && self.runs_forward() && self.runs_backward()
    }

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
        if !(self.retry_jitter >= 0.0 && self.retry_jitter < 1.0) {
            bail!("retry_jitter must be in [0, 1), got {}", self.retry_jitter);
        }
        Ok(())
    }

    /// The same options with `num_rounds = 0` read as "run all rounds"
    /// (`None`), so the driver never sees a zero cap.
    pub fn normalized(mut self) -> Self {
        self.num_rounds = round_cap(self.num_rounds);
        self
    }
}

/// A `num_rounds` as the config or the `--num-rounds` flag spells it, as the
/// plan holds it: `0` means run all rounds, so it becomes `None`.
pub fn round_cap(n: Option<usize>) -> Option<usize> {
    n.filter(|&n| n > 0)
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
            bail!(
                "{who}: initial ({}) must lie strictly inside its bounds {}; NM-TRAN rejects an \
                 initial estimate at or outside its bounds",
                self.init,
                self.bounds_label().unwrap_or_default()
            );
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
        let n = nmtran_number;
        let init = n(self.init);
        match (self.lower, self.upper) {
            (None, None) if self.fixed => return write!(f, "({init} FIX)"),
            (None, None) => return f.write_str(&init),
            (Some(lower), None) => write!(f, "({}, {init})", n(lower))?,
            (lower, Some(upper)) => {
                let lower = lower.unwrap_or(f64::NEG_INFINITY);
                write!(f, "({}, {init}, {})", n(lower), n(upper))?
            }
        }
        if self.fixed {
            f.write_str(" FIX")?;
        }
        Ok(())
    }
}

/// A covariate effect candidate
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    pub name: String,
    /// 1-based THETA number in the initial model.
    pub theta: usize,
    /// What the effect measures; it picked the default `initial` below.
    pub kind: CovariateType,
    pub initial: f64,
    pub fixed: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lower: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upper: Option<f64>,
}

impl Candidate {
    /// The `$THETA` spec the effect is estimated under when it is in the model
    pub fn released_spec(&self) -> ThetaSpec {
        ThetaSpec {
            lower: self.lower,
            init: self.initial,
            upper: self.upper,
            fixed: false,
        }
    }

    pub fn held_out_spec(&self) -> ThetaSpec {
        ThetaSpec {
            init: self.fixed,
            fixed: true,
            ..Default::default()
        }
    }

    pub fn bounds_label(&self) -> Option<String> {
        self.released_spec().bounds_label()
    }
}

/// What a covariate effect measures, written `type = "continuous"` on an
/// effect's row. It picks which `[covariates]` table supplies the effect's
/// default initial estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CovariateType {
    #[default]
    Continuous,
    Categorical,
}

impl CovariateType {
    /// The built-in initial estimate for effects of this type, used when the
    /// config's own table for the type leaves it out.
    pub const fn default_initial(self) -> f64 {
        match self {
            Self::Continuous => 0.1,
            Self::Categorical => 1.0,
        }
    }
}

/// One of the config's per-type tables: `categorical = { initial = 1 }`.
#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeDefaults {
    pub initial: Option<f64>,
}

/// One entry of the config's `[covariates] effects` array
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CovariateRequest {
    pub name: String,
    /// The TOML key is `type`; effects that leave it out are continuous.
    pub kind: CovariateType,
    pub initial: Option<f64>,
    pub fixed: Option<f64>,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
}

impl CovariateRequest {
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
            #[serde(rename = "type")]
            kind: Option<CovariateType>,
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
                    "a theta name (\"WT_CL\") or a row ({ name = \"WT_CL\", type = \"continuous\", fixed = 0 })",
                )
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(CovariateRequest::named(v))
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                let row = Row::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(CovariateRequest {
                    name: row.name,
                    kind: row.kind.unwrap_or_default(),
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
    pub fixed: Option<f64>,
    #[serde(default)]
    pub continuous: TypeDefaults,
    #[serde(default)]
    pub categorical: TypeDefaults,
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

    /// The initial estimate effects of `kind` are first tested at: the
    /// config's table for that type, else the type's built-in default.
    pub fn default_initial(&self, kind: CovariateType) -> f64 {
        self.table(kind).initial.unwrap_or(kind.default_initial())
    }

    pub fn table(&self, kind: CovariateType) -> TypeDefaults {
        match kind {
            CovariateType::Continuous => self.continuous,
            CovariateType::Categorical => self.categorical,
        }
    }

    pub fn default_fixed(&self) -> f64 {
        self.fixed.unwrap_or(CovariateRequest::FIXED)
    }
}

/// The resolved plan: everything needed to run the SCM process, built from
/// the config and the initial model. It is never a file of its own: the
/// driver records it in `scm_state.json` (`ScmState::plan`) when the process
/// starts, and keeps it current on every resume.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScmPlan {
    pub created: String,
    pub pharos_version: String,
    /// Path to the initial model, relative to the pharos project root
    pub model: String,
    /// Directory SCM process writes into, relative to the project root
    pub out_dir: String,
    pub candidates: Vec<Candidate>,
    pub options: ScmOptions,
    /// The project root `model` and `out_dir` are relative to.
    #[serde(skip)]
    pub root: PathBuf,
}

/// How a path is written into the plan: relative to the project root
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

    /// 1-based theta numbers for a set of candidate names.
    pub fn thetas_for(&self, names: &[String]) -> Vec<usize> {
        self.candidates
            .iter()
            .filter(|c| names.contains(&c.name))
            .map(|c| c.theta)
            .collect()
    }

    /// Stable digest of the SCM-defining options: the record, in a state
    /// and its summary, of what the process runs under. A process resumes
    /// across a change to any of them, and the record moves with it.
    pub fn digest(&self) -> String {
        // num_rounds and the forward model's side fit are run control, not
        // SCM-defining: a state resumes across a change to either.
        let options = ScmOptions {
            num_rounds: None,
            forward_final_cov_step: ScmOptions::default().forward_final_cov_step,
            ..self.options.clone()
        };
        let payload = serde_json::json!({
            "model": self.model,
            "out_dir": self.out_dir,
            "options": options,
        });
        blake3::hash(payload.to_string().as_bytes())
            .to_hex()
            .to_string()
    }

    /// Give a plan read back off disk (where `root` is not stored) the
    /// project root `dir` sits in.
    pub(crate) fn restore_root(&mut self, dir: &Path) -> Result<()> {
        self.options.validate()?;
        self.root = ::config::find_config_dir_from(dir)?.unwrap_or_default();
        Ok(())
    }

    /// Human-readable rendering of the plan, with the out_dir's own history
    /// appended
    pub fn render_text(&self, ctx: &PlanContext) -> String {
        let mut out = Lines::new();
        let o = &self.options;

        out.add(format!("<scm plan> {}", self.out_dir));
        out.add(format!("model      : {}", self.model));
        out.add(format!("direction  : {}", o.direction_label()));
        if o.runs_forward() {
            out.add(format!("forward    : alpha = {}", o.forward_alpha));
        }
        if o.runs_backward() {
            out.add(format!("backward   : alpha = {}", o.backward_alpha));
        }
        out.add(format!(
            "on failure : retry up to {}x from the previous attempt's estimates, jittered {}%",
            o.max_retries,
            percent(o.retry_jitter)
        ));
        out.add(format!("cov step   : {}", on_off(o.cov_step)));
        if o.fits_forward_final() {
            out.add(format!(
                "forward fit: {}",
                if o.cov_step {
                    "the forward model already runs the cov step; it is used as it is"
                } else {
                    "re-fit the forward model with the cov step on"
                }
            ));
        }
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
        let row = |name: &str,
                   theta: String,
                   kind: &str,
                   initial: String,
                   fixed: String,
                   bounds: String| {
            let mut line = format!("  {name:<12} {theta:<9} {kind:<12} {initial:>8}  {fixed:>5}");
            if bounded {
                line.push_str(&format!("  {bounds:>14}"));
            }
            line
        };
        out.add(row(
            "name",
            "theta".to_string(),
            "type",
            "initial".to_string(),
            "FIXED".to_string(),
            "bounds".to_string(),
        ));
        for c in &self.candidates {
            out.add(row(
                &c.name,
                format!("THETA({})", c.theta),
                &c.kind.to_string(),
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

/// The driver's record in the out_dir (the scheduler writes it): the
/// driver clearing the out_dir on `--overwrite` keeps its own.
const DRIVER_RECORD_FILENAME: &str = "scm_driver.json";

/// Remove previous SCM output: everything in the out_dir but the
/// `.gitignore` and the running driver's own record and log, which the
/// submit wrote just before the driver started. (The SCM config sits
/// beside the out_dir, not in it.)
pub(crate) fn clear_previous_output(out_dir: &Path) -> Result<()> {
    if !out_dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(out_dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let kept =
            name == ".gitignore" || name == DRIVER_RECORD_FILENAME || name == DRIVER_LOG_FILENAME;
        if kept {
            continue;
        }
        if path.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}

/// Worst case number of models an SCM process fits, excluding retries
pub fn max_models_for(n_candidates: usize, n_phases: usize) -> usize {
    1 + n_phases * n_candidates * (n_candidates + 1) / 2
}

/// The `[nonmem]` settings of the pharos project `dir` sits in
pub fn project_config(dir: impl AsRef<Path>) -> Result<NonmemConfig> {
    let dir = dir.as_ref();
    let config_dir = ::config::find_config_dir_from(dir)?.ok_or_else(|| {
        anyhow::anyhow!("pharos config file not found in current or parent directories")
    })?;
    let config = ::config::Config::load(config_dir.join(::config::CONFIG_FILENAME))?;
    Ok(config.nonmem.unwrap_or_default())
}

/// Where an SCM process on `model` writes: the project's `[nonmem.scm]
/// out_dir` template rendered beside the model (`scm/<stem>/` by default).
pub fn default_out_dir(layout: &ModelLayout, scm: &ScmSettings) -> Result<PathBuf> {
    let name = ::config::render_output_dir_template(scm.out_dir(), layout.stem())?;
    Ok(layout.model_dir().join(name))
}

/// The SCM out_dir a `scm status` / `scm stop` / `scm summary` argument
/// names: the directory itself, its `<stem>scm.toml` (beside it; the
/// out_dir is where the config's process writes), or another file in it.
pub fn out_dir_named(path: &Path) -> Result<PathBuf> {
    if !path.is_file() {
        return Ok(path.to_path_buf());
    }
    let is_config = path
        .file_name()
        .is_some_and(|n| n.to_string_lossy().ends_with(config::CONFIG_SUFFIX));
    match is_config {
        true => config::out_dir_of_config(path),
        false => Ok(path
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf)),
    }
}

/// A fraction as a percentage, with no floating-point tail: `0.05` -> `5`.
pub(crate) fn percent(fraction: f64) -> f64 {
    (fraction * 100.0 * 1e6).round() / 1e6
}

/// `forward_round1` as the bars and `scm status` name it: `forward round 1`
pub(crate) fn round_label(round_name: &str) -> String {
    round_name.replace("_round", " round ")
}

/// A path as every SCM message shows it: relative to the project's
/// pharos.toml when it sits under one, as it is otherwise, `/`-separated.
/// Never absolute for a path in a project, wherever the command runs from.
pub fn shown(path: &Path) -> String {
    let abs = absolute(path);
    match project_root_near(&abs) {
        Some(root) => path_for_plan(&abs, &root).replace('\\', "/"),
        None => abs.to_string_lossy().replace('\\', "/"),
    }
}

/// An error's chain as the messages show it: `{e:#}` with every path under
/// the project `near` sits in made relative, the way [`shown`] shows one.
/// Code outside the SCM module (reading a run, the file system) names
/// files in full; the record does not.
pub fn shown_error(e: &anyhow::Error, near: &Path) -> String {
    let mut text = format!("{e:#}");
    if let Some(root) = project_root_near(&absolute(near)) {
        // As found, and as the file system spells it (a canonicalized path
        // in a message)
        let mut roots = vec![root.clone()];
        if let Ok(canonical) = std::fs::canonicalize(&root)
            && canonical != root
        {
            roots.push(canonical);
        }
        roots.sort_by_key(|r| std::cmp::Reverse(r.as_os_str().len()));
        for root in roots {
            let prefix = format!("{}{}", root.display(), std::path::MAIN_SEPARATOR);
            text = text.replace(&prefix, "");
        }
        // What is left of a path is `/`-separated too; a backslash in an
        // SCM error is never anything else.
        #[cfg(windows)]
        {
            text = text.replace("\\\\", "/").replace('\\', "/");
        }
    }
    text
}

/// `path` absolute and lexically normalized; as given when that fails.
fn absolute(path: &Path) -> PathBuf {
    std::path::absolute(path)
        .map(|p| utils::normalize_path(&p))
        .unwrap_or_else(|_| path.to_path_buf())
}

/// The directory of the pharos.toml `path` (a file or directory, existing
/// or not) sits under, if any.
fn project_root_near(path: &Path) -> Option<PathBuf> {
    let start = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or(path)
    };
    ::config::find_config_dir_from(start).ok().flatten()
}

/// `path` relative to `base` for the on-disk records; the full path when it
/// does not sit under `base`. Always `/`-separated, whatever the platform,
/// so the records and messages read the same everywhere.
pub(crate) fn rel_to(path: &Path, base: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// A round's pick as its decision spells it:
/// `added WT_CL (p = 7.744e-6, dOFV = -20.000)`.
pub(crate) fn pick_label(verb: &str, name: &str, p: f64, delta_ofv: f64) -> String {
    format!("{verb} {name} (p = {p:.3e}, dOFV = {delta_ofv:+.3})")
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

/// `1 fit` / `3 fits` / `2 retries`.
pub(crate) fn plural(n: usize, noun: &str) -> String {
    match (n, noun.strip_suffix('y')) {
        (1, _) => format!("1 {noun}"),
        (n, Some(stem)) => format!("{n} {stem}ies"),
        (n, None) => format!("{n} {noun}s"),
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

    /// Paths in an error from outside the module come out relative to the
    /// project, the way `shown` shows them.
    #[test]
    fn shown_error_makes_project_paths_relative() {
        let dir = tempfile::tempdir().unwrap();
        test_support::write_project_config(dir.path(), test_support::TEMPLATE_DIALECT);
        let file = dir.path().join("scm").join("1001").join("x.json");
        let e = anyhow::anyhow!("failed to open file `{}`: gone", file.display())
            .context(format!("could not read {}", file.display()));
        assert_eq!(
            shown_error(&e, dir.path()),
            "could not read scm/1001/x.json: failed to open file `scm/1001/x.json`: gone"
        );
        // outside any project: as it is
        let stray = tempfile::tempdir().unwrap();
        let e = anyhow::anyhow!("gone: {}", stray.path().display());
        assert_eq!(shown_error(&e, stray.path()), format!("{e:#}"));
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

        let json = serde_json::to_string_pretty(&plan).unwrap();
        let back: ScmPlan = serde_json::from_str(&json).unwrap();
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
}
