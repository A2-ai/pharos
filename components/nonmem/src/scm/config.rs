//! The SCM configuration file: a TOML file that sets up an SCM process.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use fs_err as fs;
use serde::Deserialize;
use utils::normalize_path;

use super::plan::{BuiltPlan, build_plan};
use super::{CovariateRequest, Covariates, ScmOptions, default_out_dir};
use crate::ModelLayout;

pub const CONFIG_SUFFIX: &str = "-scm.toml";

/// The parsed SCM config file.
#[derive(Debug, Clone, Deserialize)]
pub struct ScmConfig {
    pub model: PathBuf,
    pub covariates: Covariates,
    #[serde(flatten)]
    pub options: ScmOptions,
}

const CONFIG_KEYS: &[&str] = &[
    "model",
    "covariates",
    "direction",
    "forward_alpha",
    "backward_alpha",
    "max_retries",
    "cov_step",
    "final_cov_step",
];

impl ScmConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read SCM config {}", path.display()))?;
        Self::parse(&content)
            .with_context(|| format!("failed to parse SCM config {}", path.display()))
    }

    pub fn parse(content: &str) -> Result<Self> {
        let table: toml::Table = toml::from_str(content)?;
        if let Some(unknown) = table.keys().find(|k| !CONFIG_KEYS.contains(&k.as_str())) {
            bail!(
                "unknown field `{unknown}`, expected one of {}",
                CONFIG_KEYS
                    .iter()
                    .map(|k| format!("`{k}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        let config: ScmConfig = toml::from_str(content)?;
        if !table.contains_key("direction") {
            bail!("missing field `direction`");
        }
        Ok(config)
    }
}

/// The `scm plan` flags that are run control rather than part of the config.
#[derive(Debug, Clone, Default)]
pub struct ScmPlanOverrides {
    pub num_rounds: Option<usize>,
    pub overwrite: bool,
}

/// A config path resolved against the config's own directory.
fn resolve(base: &Path, p: &Path) -> PathBuf {
    if p.is_relative() {
        normalize_path(&base.join(p))
    } else {
        p.to_path_buf()
    }
}

pub fn build_plan_from_config(
    config_path: &Path,
    overrides: &ScmPlanOverrides,
    pharos_version: &str,
) -> Result<BuiltPlan> {
    let config = ScmConfig::load(config_path)?;
    let base = config_path.parent().unwrap_or(Path::new("."));
    let model = resolve(base, &config.model);

    // Re-planning with overwrite discards the SCM process already in the
    // out_dir, so the plan is built over a clean one.
    if overrides.overwrite {
        let layout = ModelLayout::for_model_path(&model)?;
        super::clear_previous_output(&default_out_dir(&layout))?;
    }
    let options = ScmOptions {
        num_rounds: overrides.num_rounds,
        ..config.options
    };
    build_plan(&model, &config.covariates, None, options, pharos_version)
}

#[derive(Debug, Clone)]
pub struct ScmInit {
    pub config_path: PathBuf,
    pub out_dir: PathBuf,
}

/// Set up an SCM process for `model`
pub fn init_scm(model: &Path, overwrite: bool) -> Result<ScmInit> {
    if !model.exists() {
        bail!("Model file does not exist: {}", model.display());
    }
    let layout = ModelLayout::for_model_path(model)?;
    let out_dir = default_out_dir(&layout);
    let config_path = out_dir.join(format!("{}{CONFIG_SUFFIX}", layout.stem()));
    if config_path.exists() && !overwrite {
        bail!(
            "SCM config {} already exists; re-run with overwrite to replace it",
            config_path.display()
        );
    }
    let model_file = format!("{}.{}", layout.stem(), layout.extension());

    fs::create_dir_all(&out_dir)?;
    fs::write(&config_path, render_init_config(&model_file, layout.stem()))?;

    Ok(ScmInit {
        config_path,
        out_dir,
    })
}

fn render_init_config(model_file: &str, stem: &str) -> String {
    let d = ScmOptions::default();
    let opt = |setting: String, comment: &str| format!("{setting:<24} # {comment}\n");

    format!(
        "\
# SCM setup for {model_file}, written by SCM init.
#
# This file lives in the SCM process's own directory, scm/{stem}/, beside
# everything the process writes.
#
# Fill in `effects` under [covariates] below, then plan and run the SCM process.

model = \"../../{model_file}\"

# Which phases to run: \"forward\", \"backward\", or both. Forward always
# runs before backward.
direction = [\"forward\", \"backward\"]

# The optional settings, at their defaults: change any of these as you
# need to.
{forward_alpha}{backward_alpha}{max_retries}{cov_step}{final_cov_step}
# Every covariate effect to be tested — insert all of them here, named the
# way the initial model's $THETA records name it.
#
# The covariates can be entered just by their name, as in Example 1, and will
# use the defaults for 'initial' and 'fixed' listed below, unless otherwise
# specified in your initial model. Otherwise, the covariates can also be entered
# like in Example 2. This allows you to control their initial estimates, fixed
# values, lower, or upper bounds as needed.
#
#   effects = [
#     # Example 1
#     \"WT_CL\", \"CRCL_CL\",
#     # Example 2
#     {{ name = \"SEXEFF_CL\", initial = 1.2, fixed = 1, lower = 0, upper = 5 }},
#   ]
#
[covariates]
{initial}{fixed}effects = []
",
        forward_alpha = opt(
            format!("forward_alpha = {}", d.forward_alpha),
            "p-value to add a covariate in forward selection"
        ),
        backward_alpha = opt(
            format!("backward_alpha = {}", d.backward_alpha),
            "p-value to keep a covariate in backward elimination"
        ),
        max_retries = opt(
            format!("max_retries = {}", d.max_retries),
            "retries per failed fit, each from the previous attempt's estimates, jittered 5%"
        ),
        cov_step = opt(
            format!("cov_step = {}", d.cov_step),
            "whether generated models run $COVARIANCE"
        ),
        final_cov_step = opt(
            format!("final_cov_step = {}", d.final_cov_step),
            "whether the final model is re-fitted with $COVARIANCE on at the end"
        ),
        initial = opt(
            format!("initial = {}", CovariateRequest::INITIAL),
            "default initial estimate for an effect the first time it is tested, unless the initial model gives it one"
        ),
        fixed = opt(
            format!("fixed = {}", CovariateRequest::FIXED),
            "default value a held-out effect's theta is fixed at"
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scm::test_support::{TEMPLATE, write_template_content};

    fn write_config(dir: &Path, body: &str) -> PathBuf {
        let path = dir.join("scm.toml");
        fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn short_long_and_mixed_forms_resolve_to_the_same_plan() {
        let dir = tempfile::tempdir().unwrap();
        write_template_content(dir.path(), TEMPLATE);

        let mixed = write_config(
            dir.path(),
            r#"
model = "1001.mod"
direction = ["forward"]
[covariates]
initial = 0.2
fixed = 0
effects = [
  "WT_CL",
  { name = "CRCL_CL", initial = 0.3 },
  { name = "WT_V", fixed = 1, initial = 1.5 },
]
"#,
        );
        let built = build_plan_from_config(&mixed, &ScmPlanOverrides::default(), "test").unwrap();
        let c = &built.plan.candidates;
        assert_eq!((c[0].initial, c[0].fixed), (0.2, 0.0));
        assert_eq!((c[1].initial, c[1].fixed), (0.3, 0.0));
        assert_eq!((c[2].initial, c[2].fixed), (1.5, 1.0));

        // the same values written entirely as rows
        let long = write_config(
            dir.path(),
            r#"
model = "1001.mod"
direction = ["forward"]
[covariates]
effects = [
  { name = "WT_CL", initial = 0.2, fixed = 0 },
  { name = "CRCL_CL", initial = 0.3, fixed = 0 },
  { name = "WT_V", initial = 1.5, fixed = 1 },
]
"#,
        );
        let long = build_plan_from_config(&long, &ScmPlanOverrides::default(), "test").unwrap();
        assert_eq!(long.plan.candidates, built.plan.candidates);
    }

    #[test]
    fn config_values_and_overrides_layer_correctly() {
        let dir = tempfile::tempdir().unwrap();
        write_template_content(dir.path(), TEMPLATE);
        let config_path = write_config(
            dir.path(),
            r#"
model = "1001.mod"
direction = ["forward"]
forward_alpha = 0.01
max_retries = 5
cov_step = true
[covariates]
initial = 0.2
effects = ["WT_CL", "CRCL_CL", { name = "WT_V", initial = 0.7 }]
"#,
        );

        // config alone
        let built =
            build_plan_from_config(&config_path, &ScmPlanOverrides::default(), "test").unwrap();
        assert_eq!(built.plan.options.forward_alpha, 0.01);
        assert_eq!(built.plan.options.max_retries, 5);
        assert!(built.plan.options.cov_step);
        assert_eq!(built.plan.candidates[0].initial, 0.2);
        assert_eq!(built.plan.candidates[2].initial, 0.7);
        assert_eq!(built.plan.options.num_rounds, None);

        // the call-site knobs are run control only: they pace and overwrite
        // this run, and leave every SCM-defining value the config sets
        let overrides = ScmPlanOverrides {
            num_rounds: Some(2),
            overwrite: true,
        };
        let built = build_plan_from_config(&config_path, &overrides, "test").unwrap();
        assert_eq!(built.plan.options.num_rounds, Some(2));
        assert_eq!(built.plan.options.max_retries, 5);
        assert!(built.plan.options.cov_step);
        assert_eq!(built.plan.candidates[0].initial, 0.2);
        assert_eq!(built.plan.candidates[2].initial, 0.7);
        assert_eq!(built.plan.options.forward_alpha, 0.01);
    }

    #[test]
    fn paths_resolve_relative_to_the_config_file() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("model");
        fs::create_dir_all(&sub).unwrap();
        write_template_content(&sub, TEMPLATE);
        // config sits above the model directory
        let config_path = write_config(
            dir.path(),
            r#"
model = "model/1001.mod"
direction = ["forward"]
[covariates]
effects = ["WT_CL"]
"#,
        );
        let built =
            build_plan_from_config(&config_path, &ScmPlanOverrides::default(), "test").unwrap();
        // the stored paths are relative to the project root, so the resolved
        // ones are what show the config's `model/` prefix was honoured
        assert!(built.plan.model_path().ends_with("model/1001.mod"));
        // the SCM process always writes beside the model, never beside the config
        assert!(built.plan.out_dir_path().ends_with("model/scm/1001"));
    }

    // init ---------------------------------------------------------------

    /// The written file must be the config loader's own dialect: it parses,
    /// and the only thing standing between it and a plan is the effects
    /// the user has yet to name.
    #[test]
    fn the_initialized_config_parses_and_only_needs_effects() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template_content(dir.path(), TEMPLATE);
        let init = init_scm(&model, false).unwrap();

        let config = ScmConfig::load(&init.config_path).unwrap();
        assert_eq!(config.model, PathBuf::from("../../1001.mod"));
        assert!(config.covariates.effects.is_empty());
        assert_eq!(config.covariates.initial, Some(0.1));
        assert_eq!(config.covariates.fixed, Some(0.0));
        // every optional setting spelled out at its default reads back as
        // the defaults
        assert_eq!(config.options, ScmOptions::default());

        // empty effects is the one thing left to fill in
        let err = build_plan_from_config(&init.config_path, &ScmPlanOverrides::default(), "test")
            .unwrap_err();
        assert!(format!("{err:#}").contains("effects"), "got: {err:#}");

        // fill them in and the config plans as written
        let filled = fs::read_to_string(&init.config_path)
            .unwrap()
            .replace("effects = []", "effects = [\"WT_CL\", \"CRCL_CL\"]");
        fs::write(&init.config_path, filled).unwrap();
        let built = build_plan_from_config(&init.config_path, &ScmPlanOverrides::default(), "test")
            .unwrap();
        assert_eq!(built.plan.candidates.len(), 2);
        assert_eq!(built.plan.options, ScmOptions::default());
        // the plan lands in the directory init already created
        assert_eq!(built.plan.out_dir_path(), init.out_dir);
    }

    #[test]
    fn init_refuses_to_clobber_an_existing_config_without_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template_content(dir.path(), TEMPLATE);
        let init = init_scm(&model, false).unwrap();
        fs::write(&init.config_path, "# mine\n").unwrap();

        let err = init_scm(&model, false).unwrap_err();
        assert!(
            format!("{err:#}").contains("already exists"),
            "got: {err:#}"
        );
        assert_eq!(fs::read_to_string(&init.config_path).unwrap(), "# mine\n");

        init_scm(&model, true).unwrap();
        assert!(
            fs::read_to_string(&init.config_path)
                .unwrap()
                .contains("effects = []")
        );
    }
}
