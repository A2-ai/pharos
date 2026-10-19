//! The SCM configuration file: a TOML file that sets up an SCM process.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use fs_err as fs;
use serde::Deserialize;
use utils::normalize_path;

use super::plan::{BuiltPlan, build_plan};
use super::shown;
use super::{
    CovariateRequest, CovariateType, Covariates, Direction, ScmOptions, default_out_dir, round_cap,
};
use crate::ModelLayout;

pub const CONFIG_SUFFIX: &str = "scm.toml";

/// The parsed SCM config file. The options are spelled out rather than
/// flattened in so serde can refuse an unknown key.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScmConfig {
    pub model: PathBuf,
    pub covariates: Covariates,
    direction: Vec<Direction>,
    forward_alpha: Option<f64>,
    backward_alpha: Option<f64>,
    max_retries: Option<usize>,
    retry_jitter: Option<f64>,
    cov_step: Option<bool>,
    forward_final_cov_step: Option<bool>,
    final_cov_step: Option<bool>,
    /// Pause after this many rounds per submit, `0` (the default) running
    /// all of them; run control, so a submit flag overrides it and the SCM
    /// process resumes across a change to it.
    num_rounds: Option<usize>,
}

impl ScmConfig {
    pub fn load(path: &Path) -> Result<Self> {
        // std's error carries no path of its own, so the message shows the
        // config the way every other path is shown
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read SCM config {}", shown(&path)))?;
        toml::from_str(&content)
            .with_context(|| format!("failed to parse SCM config {}", shown(&path)))
    }

    /// The options the file sets, at their defaults where it says nothing.
    pub fn options(&self) -> ScmOptions {
        let d = ScmOptions::default();
        ScmOptions {
            direction: self.direction.clone(),
            forward_alpha: self.forward_alpha.unwrap_or(d.forward_alpha),
            backward_alpha: self.backward_alpha.unwrap_or(d.backward_alpha),
            num_rounds: round_cap(self.num_rounds),
            max_retries: self.max_retries.unwrap_or(d.max_retries),
            retry_jitter: self.retry_jitter.unwrap_or(d.retry_jitter),
            cov_step: self.cov_step.unwrap_or(d.cov_step),
            final_cov_step: self.final_cov_step.unwrap_or(d.final_cov_step),
            forward_final_cov_step: self
                .forward_final_cov_step
                .unwrap_or(d.forward_final_cov_step),
        }
    }
}

/// The submit flags (and `scm plan`'s, which reports what submit would do)
/// that override the config's run control for one submit. (`--overwrite` is
/// not one: it acts on the out_dir once the plan is built.)
#[derive(Debug, Clone, Default)]
pub struct ScmOverrides {
    /// `--num-rounds`, over the config's `num_rounds`; `0` runs all rounds
    pub num_rounds: Option<usize>,
}

pub fn build_plan_from_config(
    config_path: &Path,
    overrides: &ScmOverrides,
    pharos_version: &str,
) -> Result<BuiltPlan> {
    let (config, model) = load_with_model(config_path)?;
    if !model.exists() {
        bail!(
            "initial model {} does not exist; the config's `model` must be relative to the project's \
             pharos.toml",
            shown(&model)
        );
    }
    let mut options = config.options();
    if overrides.num_rounds.is_some() {
        options.num_rounds = round_cap(overrides.num_rounds);
    }
    build_plan(&model, &config.covariates, None, options, pharos_version)
}

/// The config at `config_path`, and the initial model it names, in full.
/// The config's `model` is relative to the project's pharos.toml, found
/// from the config up.
fn load_with_model(config_path: &Path) -> Result<(ScmConfig, PathBuf)> {
    let config = ScmConfig::load(config_path)?;
    let root = project_root_of(config_path.parent().unwrap_or(Path::new(".")))?;
    let model = normalize_path(&root.join(&config.model));
    Ok((config, model))
}

/// The directory of the pharos.toml that `dir` sits under: what the SCM
/// config's `model` is relative to.
fn project_root_of(dir: &Path) -> Result<PathBuf> {
    ::config::find_config_dir_from(dir)?.ok_or_else(|| {
        anyhow::anyhow!("pharos config file not found in current or parent directories")
    })
}

/// Where the SCM process a config describes writes: the project's out_dir
/// template rendered beside the config's model, as a plan on the config
/// resolves it. Needs no plan: the effects can still be empty.
pub fn out_dir_of_config(config_path: &Path) -> Result<PathBuf> {
    let (_, model) = load_with_model(config_path)?;
    let layout = ModelLayout::for_model_path(&model)?;
    let scm = super::project_config(layout.model_dir())?.scm;
    default_out_dir(&layout, &scm)
}

/// The SCM config of the process in `out_dir`: the `<stem>scm.toml` beside
/// the out_dir (where `scm init` puts it, like a model beside its run
/// directory) whose process lands there. Several processes' configs share
/// that directory, so each is read for where its process writes.
pub fn config_beside(out_dir: &Path) -> Result<PathBuf> {
    let out_dir = super::absolute(out_dir);
    let beside = out_dir
        .parent()
        .ok_or_else(|| anyhow!("{} has no parent directory", shown(&out_dir)))?;
    let mut configs: Vec<PathBuf> = fs::read_dir(beside)
        .with_context(|| format!("failed to read {}", shown(beside)))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().ends_with(CONFIG_SUFFIX))
        })
        .collect();
    configs.sort();
    if configs.is_empty() {
        bail!(
            "no SCM config (*{CONFIG_SUFFIX}) beside {}; is this an SCM output directory?",
            shown(&out_dir)
        );
    }
    let mut unreadable = Vec::new();
    let mut matching = Vec::new();
    for config in configs {
        match out_dir_of_config(&config) {
            Ok(dir) if super::absolute(&dir) == out_dir => matching.push(config),
            Ok(_) => {}
            Err(e) => unreadable.push(format!("{} ({e:#})", shown(&config))),
        }
    }
    match matching.len() {
        1 => Ok(matching.remove(0)),
        0 if unreadable.is_empty() => bail!(
            "no SCM config beside {} names the process there",
            shown(&out_dir)
        ),
        0 => bail!(
            "no SCM config beside {} names the process there; could not read: {}",
            shown(&out_dir),
            unreadable.join("; ")
        ),
        _ => bail!(
            "more than one SCM config beside {} names the process there: {}",
            shown(&out_dir),
            matching
                .iter()
                .map(|p| shown(p))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

#[derive(Debug, Clone)]
pub struct ScmInit {
    pub config_path: PathBuf,
    pub out_dir: PathBuf,
}

/// Set up an SCM process for `model`: its out_dir, and the config beside
/// the out_dir (`scm/<stem>scm.toml` next to `scm/<stem>/` by default), as
/// a model sits beside its run directory.
pub fn init_scm(model: &Path, overwrite: bool) -> Result<ScmInit> {
    if !model.exists() {
        bail!("Model file does not exist: {}", shown(&model));
    }
    let layout = ModelLayout::for_model_path(model)?;
    let scm = super::project_config(layout.model_dir())
        .unwrap_or_default()
        .scm;
    let out_dir = default_out_dir(&layout, &scm)?;
    let config_path = out_dir
        .parent()
        .ok_or_else(|| anyhow!("{} has no parent directory", shown(&out_dir)))?
        .join(format!("{}{CONFIG_SUFFIX}", layout.stem()));
    if config_path.exists() && !overwrite {
        bail!(
            "SCM config {} already exists; re-run with overwrite to replace it",
            shown(&config_path)
        );
    }
    let model_file = format!("{}.{}", layout.stem(), layout.extension());
    let dir_label = super::rel_to(&out_dir, layout.model_dir());
    // The config names the model from the project's pharos.toml.
    let root = project_root_of(layout.model_dir())?;
    let model_rel = super::path_for_plan(model, &root);

    fs::create_dir_all(&out_dir)?;
    fs::write(
        &config_path,
        render_init_config(&model_file, &model_rel, &dir_label),
    )?;
    super::write_gitignore(&out_dir, scm.track_in_git())?;

    Ok(ScmInit {
        config_path,
        out_dir,
    })
}

fn render_init_config(model_file: &str, model_rel: &str, dir_label: &str) -> String {
    let d = ScmOptions::default();
    let opt = |setting: String, comment: &str| format!("{setting:<24} # {comment}\n");

    format!(
        "\
# SCM setup for {model_file}.
#
# This file sits beside the SCM process's own directory, {dir_label}/,
# where everything the process writes lands.
#
# Fill in `effects` under [covariates] below, then submit the SCM process
# (`scm run` or `scm slurm submit` on this file); `scm plan` on this file
# shows what a submit would do without doing it.

model = \"{model_rel}\"    # relative to the project's pharos.toml

# Which phases to run: \"forward\", \"backward\", or both. Forward always
# runs before backward.
direction = [\"forward\", \"backward\"]

# The optional settings, at their defaults: change any of these as you
# need to.
{forward_alpha}{backward_alpha}{max_retries}{retry_jitter}{num_rounds}{cov_step}{forward_final_cov_step}{final_cov_step}
# Every covariate effect to be tested — insert all of them here, named the
# way the initial model's $THETA records name it.
#
# The covariates can be entered just by their name, as in Example 1. Such an
# effect is continuous, and is first tested at the initial estimate its type
# gets below, unless the initial model already gives it one. Otherwise, the
# covariates can also be entered like in Example 2. This allows you to declare
# a categorical effect, and to control an effect's initial estimate, fixed
# value, lower, or upper bounds as needed.
#
#   effects = [
#     # Example 1
#     \"WT_CL\", \"CRCL_CL\",
#     # Example 2
#     {{ name = \"SEXEFF_CL\", type = \"categorical\", fixed = 1, lower = 0, upper = 5 }},
#   ]
#
[covariates]
{fixed}continuous  = {{ initial = {continuous} }} #Default initial estimate for a continuous-type covariate
categorical = {{ initial = {categorical} }} #Default initial estimate for a categorical-type covariate

effects = []
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
            "retries per failed fit, each from the previous attempt's estimates, jittered"
        ),
        retry_jitter = opt(
            format!("retry_jitter = {}", d.retry_jitter),
            "how far a retry perturbs the estimates it starts from, as a fraction (0.05 = 5%)"
        ),
        num_rounds = opt(
            "num_rounds = 0".to_string(),
            "num_rounds = 0 means run all rounds. Use an integer for a specific number of rounds"
        ),
        cov_step = opt(
            format!("cov_step = {}", d.cov_step),
            "whether generated models run $COVARIANCE"
        ),
        forward_final_cov_step = opt(
            format!("forward_final_cov_step = {}", d.forward_final_cov_step),
            "whether the forward model is re-fitted with $COVARIANCE on"
        ),
        final_cov_step = opt(
            format!("final_cov_step = {}", d.final_cov_step),
            "whether the final model is re-fitted with $COVARIANCE on at the end"
        ),
        fixed = opt(
            format!("fixed = {}", CovariateRequest::FIXED),
            "default value a held-out effect's theta is fixed at"
        ),
        continuous = CovariateType::Continuous.default_initial(),
        categorical = CovariateType::Categorical.default_initial(),
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

    /// Fill in the `effects` of a config `init` wrote, and plan it.
    fn fill_effects(config_path: &Path, effects: &str) -> BuiltPlan {
        let filled = fs::read_to_string(config_path)
            .unwrap()
            .replace("effects = []", &format!("effects = [{effects}]"));
        fs::write(config_path, filled).unwrap();
        build_plan_from_config(config_path, &ScmOverrides::default(), "test").unwrap()
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
continuous = { initial = 0.2 }
categorical = { initial = 0.3 }
fixed = 0
effects = [
  "WT_CL",
  { name = "CRCL_CL", type = "categorical" },
  { name = "WT_V", fixed = 1, initial = 1.5 },
]
"#,
        );
        let built = build_plan_from_config(&mixed, &ScmOverrides::default(), "test").unwrap();
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
  { name = "CRCL_CL", type = "categorical", initial = 0.3, fixed = 0 },
  { name = "WT_V", initial = 1.5, fixed = 1 },
]
"#,
        );
        let long = build_plan_from_config(&long, &ScmOverrides::default(), "test").unwrap();
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
retry_jitter = 0.1
cov_step = true
[covariates]
continuous = { initial = 0.2 }
effects = ["WT_CL", "CRCL_CL", { name = "WT_V", initial = 0.7 }]
"#,
        );

        // config alone
        let built = build_plan_from_config(&config_path, &ScmOverrides::default(), "test").unwrap();
        assert_eq!(built.plan.options.forward_alpha, 0.01);
        assert_eq!(built.plan.options.max_retries, 5);
        assert_eq!(built.plan.options.retry_jitter, 0.1);
        assert!(built.plan.options.cov_step);
        assert_eq!(built.plan.candidates[0].initial, 0.2);
        assert_eq!(built.plan.candidates[2].initial, 0.7);
        assert_eq!(built.plan.options.num_rounds, None);

        // the submit flag is run control only: it paces this submit, and
        // leaves every SCM-defining value the config sets
        let overrides = ScmOverrides {
            num_rounds: Some(2),
        };
        let paced = build_plan_from_config(&config_path, &overrides, "test").unwrap();
        assert_eq!(paced.plan.candidates, built.plan.candidates);
        assert_eq!(
            paced.plan.options,
            ScmOptions {
                num_rounds: Some(2),
                ..built.plan.options.clone()
            }
        );

        // the config can pace the process itself, and the flag wins over it
        fs::write(
            &config_path,
            fs::read_to_string(&config_path)
                .unwrap()
                .replace("max_retries = 5", "max_retries = 5\nnum_rounds = 1"),
        )
        .unwrap();
        let from_config =
            build_plan_from_config(&config_path, &ScmOverrides::default(), "test").unwrap();
        assert_eq!(from_config.plan.options.num_rounds, Some(1));
        let flagged = build_plan_from_config(&config_path, &overrides, "test").unwrap();
        assert_eq!(flagged.plan.options.num_rounds, Some(2));
        // 0 means all rounds, from the flag over a paced config as well as
        // from the config itself
        let all_rounds = ScmOverrides {
            num_rounds: Some(0),
        };
        let unpaced = build_plan_from_config(&config_path, &all_rounds, "test").unwrap();
        assert_eq!(unpaced.plan.options.num_rounds, None);
        fs::write(
            &config_path,
            fs::read_to_string(&config_path)
                .unwrap()
                .replace("num_rounds = 1", "num_rounds = 0"),
        )
        .unwrap();
        let zero_in_config =
            build_plan_from_config(&config_path, &ScmOverrides::default(), "test").unwrap();
        assert_eq!(zero_in_config.plan.options.num_rounds, None);
        // and the digest, the record of what a process ran under, ignores both
        assert_eq!(flagged.plan.digest(), built.plan.digest());
    }

    #[test]
    fn model_path_resolves_relative_to_the_project_root() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("model");
        fs::create_dir_all(&sub).unwrap();
        write_template_content(dir.path(), TEMPLATE);
        fs::write(sub.join("1001.mod"), TEMPLATE).unwrap();
        fs::write(sub.join("data.csv"), "ID\n1\n").unwrap();
        // the config sits beside the out_dir, two levels under the project
        // root; its model path is read from the root, not from the config
        let out_dir = sub.join("scm").join("1001");
        fs::create_dir_all(&out_dir).unwrap();
        let config_path = sub.join("scm").join("1001scm.toml");
        fs::write(
            &config_path,
            r#"
model = "model/1001.mod"
direction = ["forward"]
[covariates]
effects = ["WT_CL"]
"#,
        )
        .unwrap();
        let built = build_plan_from_config(&config_path, &ScmOverrides::default(), "test").unwrap();
        assert!(built.plan.model_path().ends_with("model/1001.mod"));
        assert_eq!(built.plan.model, "model/1001.mod");
        // the SCM process always writes beside the model, never beside the config
        assert!(built.plan.out_dir_path().ends_with("model/scm/1001"));
        // the config and the out_dir find each other
        assert_eq!(out_dir_of_config(&config_path).unwrap(), out_dir);
        assert_eq!(config_beside(&out_dir).unwrap(), config_path);

        // no pharos.toml above the config: the model cannot be placed
        let stray = tempfile::tempdir().unwrap();
        let config_path = write_config(
            stray.path(),
            "model = \"1001.mod\"\ndirection = [\"forward\"]\n[covariates]\neffects = [\"WT_CL\"]\n",
        );
        let err =
            build_plan_from_config(&config_path, &ScmOverrides::default(), "test").unwrap_err();
        assert!(
            format!("{err:#}").contains("pharos config file not found"),
            "{err:#}"
        );
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
        // the config sits beside the out_dir, as the model beside its run dir
        assert_eq!(init.out_dir, dir.path().join("scm").join("1001"));
        assert_eq!(
            init.config_path,
            dir.path().join("scm").join("1001scm.toml")
        );
        assert!(init.out_dir.is_dir());

        let config = ScmConfig::load(&init.config_path).unwrap();
        assert_eq!(config.model, PathBuf::from("1001.mod"));
        assert!(config.covariates.effects.is_empty());
        assert_eq!(config.covariates.continuous.initial, Some(0.1));
        assert_eq!(config.covariates.categorical.initial, Some(1.0));
        assert_eq!(config.covariates.fixed, Some(0.0));
        // every optional setting spelled out at its default reads back as
        // the defaults
        assert_eq!(config.options(), ScmOptions::default());

        // empty effects is the one thing left to fill in
        let err = build_plan_from_config(&init.config_path, &ScmOverrides::default(), "test")
            .unwrap_err();
        assert!(format!("{err:#}").contains("effects"), "got: {err:#}");

        // fill them in and the config plans as written
        let built = fill_effects(&init.config_path, "\"WT_CL\", \"CRCL_CL\"");
        assert_eq!(built.plan.candidates.len(), 2);
        assert_eq!(built.plan.options, ScmOptions::default());
        // the plan lands in the directory init already created
        assert_eq!(built.plan.out_dir_path(), init.out_dir);
    }

    /// The project's `[nonmem.scm] out_dir` template places the process, and
    /// the starter config names the model from the project root wherever
    /// it lands.
    #[test]
    fn the_out_dir_template_places_the_process_and_its_config() {
        use crate::scm::test_support::{TEMPLATE_DIALECT, write_project_config_with};

        let dir = tempfile::tempdir().unwrap();
        let model = write_template_content(dir.path(), TEMPLATE);
        let mut scm = ::config::ScmSettings::default();
        scm.set_out_dir(Some("cov/{{name}}/process".to_string()));
        write_project_config_with(dir.path(), TEMPLATE_DIALECT, scm);

        let init = init_scm(&model, false).unwrap();
        assert!(init.out_dir.ends_with("cov/1001/process"));
        assert!(init.config_path.ends_with("cov/1001/1001scm.toml"));
        assert_eq!(
            ScmConfig::load(&init.config_path).unwrap().model,
            PathBuf::from("1001.mod")
        );

        let built = fill_effects(&init.config_path, "\"WT_CL\"");
        assert_eq!(built.plan.out_dir_path(), init.out_dir);
        assert_eq!(out_dir_of_config(&init.config_path).unwrap(), init.out_dir);
        assert_eq!(config_beside(&init.out_dir).unwrap(), init.config_path);
    }

    /// Every process's config shares the directory beside the out_dirs, so
    /// the out_dir's config is the one whose process lands there.
    #[test]
    fn the_config_beside_an_out_dir_is_the_one_naming_its_process() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template_content(dir.path(), TEMPLATE);
        let other = dir.path().join("1002.mod");
        fs::copy(&model, &other).unwrap();
        let first = init_scm(&model, false).unwrap();
        let second = init_scm(&other, false).unwrap();
        assert_eq!(first.config_path.parent(), second.config_path.parent());
        assert_eq!(config_beside(&first.out_dir).unwrap(), first.config_path);
        assert_eq!(config_beside(&second.out_dir).unwrap(), second.config_path);

        // a directory no config names
        let stray = dir.path().join("scm").join("1003");
        fs::create_dir_all(&stray).unwrap();
        let err = config_beside(&stray).unwrap_err();
        assert!(
            format!("{err:#}").contains("names the process there"),
            "{err:#}"
        );
        // a directory with no configs beside it at all
        let lone = dir.path().join("elsewhere").join("1001");
        fs::create_dir_all(&lone).unwrap();
        let err = config_beside(&lone).unwrap_err();
        assert!(format!("{err:#}").contains("no SCM config"), "{err:#}");
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
