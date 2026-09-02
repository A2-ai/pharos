//! The SCM configuration file: a TOML file that sets up a search.
//!
//! The config carries what *defines* the search — the template model, output
//! directory, candidates, direction, alphas, retries, cov step, release
//! init. The `scm plan` call itself carries only per-invocation control:
//! `num_rounds`, overrides for retries / cov step / release init, and
//! `overwrite`. The plan.json written out is the merge of the two; running
//! and resuming are unchanged.
//!
//! ```toml
//! model = "model/nonmem/PK/scm-demo.mod"
//! # out_dir = "model/nonmem/PK/scm/scm-demo"   # default: scm/<stem> beside the model
//! covariates = ["WT_CL", "CRCL_CL", "AGE_CL"]  # or THETA numbers: [6, 7, 8]
//! direction = ["forward", "backward"]
//!
//! # optional, with the usual defaults:
//! # forward_alpha = 0.05
//! # backward_alpha = 0.001
//! # max_retries = 3
//! # cov_step = true
//! # release_init = 0.1
//! ```
//!
//! Relative paths in the config resolve against the config file's own
//! directory, so the file can live beside the model and be run from anywhere.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use fs_err as fs;
use serde::Deserialize;

use super::plan::{BuiltPlan, CovariateSpec, build_plan};
use super::{Direction, ScmOptions};

/// The parsed SCM config file. Unknown keys are rejected so a typo'd option
/// fails loudly instead of silently falling back to a default.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScmConfig {
    /// Path to the template control stream, relative to this file.
    pub model: PathBuf,
    /// Output directory for the search, relative to this file.
    /// Omitted: `scm/<model stem>` beside the model.
    pub out_dir: Option<PathBuf>,
    /// Candidate covariate effects: `$PK` term names (`["WT_CL", "CRCL_CL"]`)
    /// or 1-based THETA numbers (`[6, 7]`) — one form or the other.
    pub covariates: CovariateSpec,
    /// Which phases to run: `["forward"]`, `["backward"]`, or both.
    pub direction: Vec<Direction>,
    /// Significance level for adding a covariate in forward selection.
    pub forward_alpha: Option<f64>,
    /// Significance level for keeping a covariate in backward elimination.
    pub backward_alpha: Option<f64>,
    /// Retries per failed fit.
    pub max_retries: Option<usize>,
    /// Whether generated models run the covariance step.
    pub cov_step: Option<bool>,
    /// Initial estimate a newly released covariate theta starts at.
    pub release_init: Option<f64>,
}

impl ScmConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read SCM config {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("failed to parse SCM config {}", path.display()))
    }
}

/// Per-invocation knobs on `scm plan` that are not part of the config file.
/// `num_rounds` paces this run of the search; the rest override the config.
#[derive(Debug, Clone, Default)]
pub struct ScmPlanOverrides {
    pub num_rounds: Option<usize>,
    pub max_retries: Option<usize>,
    pub cov_step: Option<bool>,
    pub release_init: Option<f64>,
    pub overwrite: bool,
}

fn resolve(base: &Path, p: &Path) -> PathBuf {
    if p.is_relative() { base.join(p) } else { p.to_path_buf() }
}

/// Load the config at `config_path`, apply the call's overrides, and build
/// and validate the plan (runs nothing).
pub fn build_plan_from_config(
    config_path: &Path,
    overrides: &ScmPlanOverrides,
    pharos_version: &str,
) -> Result<BuiltPlan> {
    let config = ScmConfig::load(config_path)?;
    let base = config_path.parent().unwrap_or(Path::new("."));
    let model = resolve(base, &config.model);
    let out_dir = config.out_dir.as_ref().map(|p| resolve(base, p));

    let defaults = ScmOptions::default();
    let options = ScmOptions {
        direction: config.direction.clone(),
        forward_alpha: config.forward_alpha.unwrap_or(defaults.forward_alpha),
        backward_alpha: config.backward_alpha.unwrap_or(defaults.backward_alpha),
        num_rounds: overrides.num_rounds,
        max_retries: overrides
            .max_retries
            .or(config.max_retries)
            .unwrap_or(defaults.max_retries),
        release_init: overrides
            .release_init
            .or(config.release_init)
            .unwrap_or(defaults.release_init),
        cov_step: overrides.cov_step.or(config.cov_step).unwrap_or(defaults.cov_step),
        overwrite: overrides.overwrite,
    };

    build_plan(
        &model,
        &config.covariates,
        out_dir.as_deref(),
        options,
        pharos_version,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scm::plan::tests::{NAMED_TEMPLATE, write_template_content};

    fn write_config(dir: &Path, body: &str) -> PathBuf {
        let path = dir.join("scm.toml");
        fs::write(&path, body).unwrap();
        path
    }

    const MINIMAL: &str = r#"
model = "1001.mod"
covariates = ["WT_CL", "CRCL_CL", "WT_V"]
direction = ["forward", "backward"]
"#;

    #[test]
    fn minimal_config_uses_the_defaults() {
        let dir = tempfile::tempdir().unwrap();
        write_template_content(dir.path(), NAMED_TEMPLATE);
        let config_path = write_config(dir.path(), MINIMAL);

        let built =
            build_plan_from_config(&config_path, &ScmPlanOverrides::default(), "test").unwrap();
        let plan = &built.plan;
        assert_eq!(plan.candidates.len(), 3);
        assert_eq!(plan.candidates[0].name, "WT_CL");
        assert_eq!(plan.options, ScmOptions::default());
        assert!(plan.out_dir.ends_with("scm/1001"));
    }

    #[test]
    fn config_values_and_overrides_layer_correctly() {
        let dir = tempfile::tempdir().unwrap();
        write_template_content(dir.path(), NAMED_TEMPLATE);
        let config_path = write_config(
            dir.path(),
            r#"
model = "1001.mod"
out_dir = "scm-out"
covariates = [4, 5, 6]
direction = ["forward"]
forward_alpha = 0.01
max_retries = 5
cov_step = false
release_init = 0.2
"#,
        );

        // config alone
        let built =
            build_plan_from_config(&config_path, &ScmPlanOverrides::default(), "test").unwrap();
        assert_eq!(built.plan.options.forward_alpha, 0.01);
        assert_eq!(built.plan.options.max_retries, 5);
        assert!(!built.plan.options.cov_step);
        assert_eq!(built.plan.options.release_init, 0.2);
        assert_eq!(built.plan.options.num_rounds, None);
        assert!(built.plan.out_dir.ends_with("scm-out"));

        // call-site overrides beat the config
        let overrides = ScmPlanOverrides {
            num_rounds: Some(2),
            max_retries: Some(1),
            cov_step: Some(true),
            release_init: Some(0.05),
            overwrite: true,
        };
        let built = build_plan_from_config(&config_path, &overrides, "test").unwrap();
        assert_eq!(built.plan.options.num_rounds, Some(2));
        assert_eq!(built.plan.options.max_retries, 1);
        assert!(built.plan.options.cov_step);
        assert_eq!(built.plan.options.release_init, 0.05);
        assert!(built.plan.options.overwrite);
        // untouched config values survive the overrides
        assert_eq!(built.plan.options.forward_alpha, 0.01);
    }

    #[test]
    fn paths_resolve_relative_to_the_config_file() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("model");
        fs::create_dir_all(&sub).unwrap();
        write_template_content(&sub, NAMED_TEMPLATE);
        // config sits above the model directory
        let config_path = write_config(
            dir.path(),
            r#"
model = "model/1001.mod"
covariates = ["WT_CL"]
direction = ["forward"]
"#,
        );
        let built =
            build_plan_from_config(&config_path, &ScmPlanOverrides::default(), "test").unwrap();
        assert!(built.plan.model.contains("model/"));
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        write_template_content(dir.path(), NAMED_TEMPLATE);
        let config_path = write_config(
            dir.path(),
            r#"
model = "1001.mod"
covariates = ["WT_CL"]
direction = ["forward"]
foward_alpha = 0.01
"#,
        );
        let err = build_plan_from_config(&config_path, &ScmPlanOverrides::default(), "test")
            .unwrap_err();
        assert!(format!("{err:#}").contains("foward_alpha"), "got: {err:#}");
    }

    #[test]
    fn mixed_covariates_array_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        write_template_content(dir.path(), NAMED_TEMPLATE);
        let config_path = write_config(
            dir.path(),
            r#"
model = "1001.mod"
covariates = [6, "WT_CL"]
direction = ["forward"]
"#,
        );
        assert!(
            build_plan_from_config(&config_path, &ScmPlanOverrides::default(), "test").is_err()
        );
    }

    #[test]
    fn missing_required_keys_error() {
        let dir = tempfile::tempdir().unwrap();
        write_template_content(dir.path(), NAMED_TEMPLATE);
        let config_path = write_config(dir.path(), "model = \"1001.mod\"\n");
        let err = build_plan_from_config(&config_path, &ScmPlanOverrides::default(), "test")
            .unwrap_err();
        assert!(format!("{err:#}").contains("covariates"), "got: {err:#}");
    }
}
