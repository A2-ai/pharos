//! The SCM configuration file: a TOML file that sets up an SCM process.
//!
//! The config carries what *defines* the SCM process — the template model,
//! candidates, direction, alphas, retries, cov step, release init. The
//! `scm plan` call itself carries only per-invocation control:
//! `num_rounds`, overrides for retries / cov step / release init, and
//! `overwrite`. The plan.json written out is the merge of the two; running
//! and resuming are unchanged.
//!
//! [`init_scm`] writes the file below beside a model as
//! `<stem>-scm.toml` and creates the `scm/<stem>` directory the SCM process
//! writes into — the covariates left empty for the user to fill in, the
//! optional settings spelled out at their defaults:
//!
//! ```toml
//! model = "scm-demo.mod"
//! covariates = ["WT_CL", "CRCL_CL", "AGE_CL"]  # $PK term names
//! direction = ["forward", "backward"]
//!
//! # optional, shown at their defaults:
//! forward_alpha = 0.05
//! backward_alpha = 0.001
//! max_retries = 3
//! cov_step = false
//! release_init = 0.1
//! ```
//!
//! Relative paths in the config resolve against the config file's own
//! directory, so the file can live beside the model and be run from anywhere.
//! The SCM process always writes into `scm/<model stem>` beside the model; that
//! location is not configurable.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use fs_err as fs;
use serde::Deserialize;
use serde::de::{self, Deserializer, Visitor};

use super::plan::{BuiltPlan, build_plan};
use super::round::file_stem_of;
use super::{Direction, PLAN_FILENAME, ScmOptions, parent_or_dot};
use crate::validate_model_extension;

/// The suffix an SCM config file carries: `<model stem>-scm.toml`, beside
/// the model it plans an SCM process for.
pub const CONFIG_SUFFIX: &str = "-scm.toml";

/// The parsed SCM config file. Unknown keys are rejected so a typo'd option
/// fails loudly instead of silently falling back to a default.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScmConfig {
    /// Path to the template control stream, relative to this file.
    pub model: PathBuf,
    /// Candidate covariate effects, named by their `$PK` term:
    /// `["WT_CL", "CRCL_CL"]`. Each name must be an assignment in `$PK` (or
    /// `$PRED`) whose expression references exactly one THETA, e.g.
    /// `WT_CL = ((WT/70)**THETA(6))` names THETA(6).
    #[serde(deserialize_with = "covariate_names")]
    pub covariates: Vec<String>,
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

/// One entry of the `covariates` array. Names only: THETA numbers used to
/// select candidates, so an integer here is a config written against an
/// older pharos and gets a message saying what to write instead of serde's
/// bare "invalid type" complaint.
struct CovariateName(String);

impl<'de> Deserialize<'de> for CovariateName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct NameVisitor;

        impl Visitor<'_> for NameVisitor {
            type Value = CovariateName;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a $PK term name, e.g. \"WT_CL\"")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(CovariateName(v.to_string()))
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Err(E::custom(theta_number_message(&v.to_string())))
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Err(E::custom(theta_number_message(&v.to_string())))
            }
        }

        deserializer.deserialize_any(NameVisitor)
    }
}

fn theta_number_message(found: &str) -> String {
    format!(
        "covariates are named by their $PK term, not by THETA number (found {found}); \
         write e.g. covariates = [\"WT_CL\", \"CRCL_CL\"], naming the $PK assignment \
         that references each candidate theta"
    )
}

fn covariate_names<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    let names = Vec::<CovariateName>::deserialize(deserializer)?;
    Ok(names.into_iter().map(|n| n.0).collect())
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
/// `num_rounds` paces this run of the SCM process; the rest override the config.
#[derive(Debug, Clone, Default)]
pub struct ScmPlanOverrides {
    pub num_rounds: Option<usize>,
    pub max_retries: Option<usize>,
    pub cov_step: Option<bool>,
    pub release_init: Option<f64>,
    pub overwrite: bool,
}

fn resolve(base: &Path, p: &Path) -> PathBuf {
    if p.is_relative() {
        base.join(p)
    } else {
        p.to_path_buf()
    }
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
        cov_step: overrides
            .cov_step
            .or(config.cov_step)
            .unwrap_or(defaults.cov_step),
        overwrite: overrides.overwrite,
    };

    build_plan(&model, &config.covariates, None, options, pharos_version)
}

/// Where a model's SCM config belongs: `<stem>-scm.toml` beside the model.
pub fn config_path_for(model: &Path) -> Result<PathBuf> {
    let stem = file_stem_of(model)
        .with_context(|| format!("model file {} has no file stem", model.display()))?;
    Ok(parent_or_dot(model).join(format!("{stem}{CONFIG_SUFFIX}")))
}

/// Where a model's SCM process writes: `scm/<stem>` beside the model. The same
/// directory [`build_plan`] defaults the plan's out_dir to.
pub fn out_dir_for(model: &Path) -> Result<PathBuf> {
    let stem = file_stem_of(model)
        .with_context(|| format!("model file {} has no file stem", model.display()))?;
    Ok(parent_or_dot(model).join("scm").join(stem))
}

/// What [`init_scm`] put on disk.
#[derive(Debug, Clone)]
pub struct ScmInit {
    /// The config file written, ready for the user to fill in.
    pub config_path: PathBuf,
    /// The SCM process directory created beside the model.
    pub out_dir: PathBuf,
}

/// Set up an SCM process for `model`: write `<stem>-scm.toml` beside it and
/// create the `scm/<stem>` directory the SCM process will write into. Fills the
/// config's optional settings in at their defaults and leaves `covariates`
/// empty for the user. Fits nothing, and plans nothing — the config is not
/// yet valid, because the candidates are the user's to name.
pub fn init_scm(model: &Path, overwrite: bool) -> Result<ScmInit> {
    if !model.exists() {
        bail!("Model file does not exist: {}", model.display());
    }
    validate_model_extension(model)?;

    let config_path = config_path_for(model)?;
    if config_path.exists() && !overwrite {
        bail!(
            // Both the CLI (`--overwrite`) and hyperion (`overwrite = TRUE`)
            // surface this, so name the option, not either spelling.
            "SCM config {} already exists; re-run with overwrite to replace it",
            config_path.display()
        );
    }
    let out_dir = out_dir_for(model)?;

    // The config names the model by file name: the two sit side by side, and
    // relative paths in the config resolve against the config's own directory.
    let model_file = model
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .with_context(|| format!("model path {} has no file name", model.display()))?;
    let stem = file_stem_of(model).expect("a path with a file name to have a stem");

    fs::create_dir_all(&out_dir)?;
    fs::write(&config_path, render_init_config(&model_file, &stem))?;

    Ok(ScmInit {
        config_path,
        out_dir,
    })
}

/// The starter config [`init_scm`] writes: every required key present —
/// `covariates` deliberately empty — and every optional key spelled out at
/// its default, annotated so the file explains itself.
fn render_init_config(model_file: &str, stem: &str) -> String {
    let d = ScmOptions::default();
    // Optional settings carry their meaning in a trailing comment, aligned.
    let opt = |setting: String, comment: &str| format!("{setting:<24} # {comment}\n");

    format!(
        "\
# SCM setup for {model_file}, written by SCM init.
#
# Fill in `covariates` below, then plan and run the SCM process:
#
#   pharos:   pharos scm plan {stem}{CONFIG_SUFFIX}
#             pharos scm run --plan scm/{stem}/{PLAN_FILENAME}
#
#   hyperion: plan <- scm_plan(\"{stem}{CONFIG_SUFFIX}\")
#             scm_run(plan)
#
# Paths resolve against this file's own directory, so the SCM process runs from
# anywhere. It writes into scm/{stem}/ beside the model.

model = \"{model_file}\"

# Every covariate effect to be tested — insert all of them here, named by
# their $PK term (\"WT_CL\", \"CRCL_CL\"). Each name must be a $PK assignment
# referencing exactly one THETA. This list alone decides what is tested: a
# theta named here is fixed at 0 in every model that holds its effect out,
# whatever the template writes it as.
covariates = []

# Which phases to run: \"forward\", \"backward\", or both. Forward always
# runs before backward.
direction = [\"forward\", \"backward\"]

# The optional settings, at their defaults — change any of these as you
# need to.
{forward_alpha}{backward_alpha}{max_retries}{cov_step}{release_init}",
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
            "retries per failed fit, each from the previous attempt's estimates"
        ),
        cov_step = opt(
            format!("cov_step = {}", d.cov_step),
            "whether generated models run $COVARIANCE"
        ),
        release_init = opt(
            format!("release_init = {}", d.release_init),
            "initial estimate a released covariate theta starts at, unless the template gives it one"
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scm::plan::tests::{TEMPLATE, write_template_content};

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
        write_template_content(dir.path(), TEMPLATE);
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
        write_template_content(dir.path(), TEMPLATE);
        let config_path = write_config(
            dir.path(),
            r#"
model = "1001.mod"
covariates = ["WT_CL", "CRCL_CL", "WT_V"]
direction = ["forward"]
forward_alpha = 0.01
max_retries = 5
cov_step = true
release_init = 0.2
"#,
        );

        // config alone
        let built =
            build_plan_from_config(&config_path, &ScmPlanOverrides::default(), "test").unwrap();
        assert_eq!(built.plan.options.forward_alpha, 0.01);
        assert_eq!(built.plan.options.max_retries, 5);
        assert!(built.plan.options.cov_step);
        assert_eq!(built.plan.options.release_init, 0.2);
        assert_eq!(built.plan.options.num_rounds, None);

        // call-site overrides beat the config
        let overrides = ScmPlanOverrides {
            num_rounds: Some(2),
            max_retries: Some(1),
            cov_step: Some(false),
            release_init: Some(0.05),
            overwrite: true,
        };
        let built = build_plan_from_config(&config_path, &overrides, "test").unwrap();
        assert_eq!(built.plan.options.num_rounds, Some(2));
        assert_eq!(built.plan.options.max_retries, 1);
        assert!(!built.plan.options.cov_step);
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
        write_template_content(&sub, TEMPLATE);
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
        // the SCM process always writes beside the model, never beside the config
        assert!(built.plan.out_dir.contains("model/scm/1001"));
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        write_template_content(dir.path(), TEMPLATE);
        let config_path = write_config(
            dir.path(),
            r#"
model = "1001.mod"
covariates = ["WT_CL"]
direction = ["forward"]
foward_alpha = 0.01
"#,
        );
        let err =
            build_plan_from_config(&config_path, &ScmPlanOverrides::default(), "test").unwrap_err();
        assert!(format!("{err:#}").contains("foward_alpha"), "got: {err:#}");
    }

    /// out_dir is no longer a config key: the SCM process writes beside the
    /// model. A config carrying one is a stale file, and says so.
    #[test]
    fn out_dir_is_no_longer_a_config_key() {
        let dir = tempfile::tempdir().unwrap();
        write_template_content(dir.path(), TEMPLATE);
        let config_path = write_config(
            dir.path(),
            r#"
model = "1001.mod"
out_dir = "scm-out"
covariates = ["WT_CL"]
direction = ["forward"]
"#,
        );
        let err =
            build_plan_from_config(&config_path, &ScmPlanOverrides::default(), "test").unwrap_err();
        assert!(format!("{err:#}").contains("out_dir"), "got: {err:#}");
    }

    /// THETA numbers used to be a second spelling of the covariates array.
    /// They are not accepted any more, and the error has to say what to
    /// write instead — a config from an older pharos lands here.
    #[test]
    fn theta_numbers_in_the_covariates_array_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        write_template_content(dir.path(), TEMPLATE);

        for array in [r#"[4, 5, 6]"#, r#"[6, "WT_CL"]"#] {
            let config_path = write_config(
                dir.path(),
                &format!(
                    r#"
model = "1001.mod"
covariates = {array}
direction = ["forward"]
"#
                ),
            );
            let err = build_plan_from_config(&config_path, &ScmPlanOverrides::default(), "test")
                .unwrap_err();
            let msg = format!("{err:#}");
            assert!(msg.contains("not by THETA number"), "got: {msg}");
            assert!(msg.contains("$PK"), "got: {msg}");
        }
    }

    #[test]
    fn missing_required_keys_error() {
        let dir = tempfile::tempdir().unwrap();
        write_template_content(dir.path(), TEMPLATE);
        let config_path = write_config(dir.path(), "model = \"1001.mod\"\n");
        let err =
            build_plan_from_config(&config_path, &ScmPlanOverrides::default(), "test").unwrap_err();
        assert!(format!("{err:#}").contains("covariates"), "got: {err:#}");
    }

    // init ---------------------------------------------------------------

    #[test]
    fn init_writes_the_config_beside_the_model_and_makes_the_scm_dir() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template_content(dir.path(), TEMPLATE);

        let init = init_scm(&model, false).unwrap();
        assert_eq!(init.config_path, dir.path().join("1001-scm.toml"));
        assert_eq!(init.out_dir, dir.path().join("scm").join("1001"));
        assert!(init.out_dir.is_dir());

        let body = fs::read_to_string(&init.config_path).unwrap();
        // Both front ends' next steps are spelled out.
        assert!(body.contains("pharos scm plan 1001-scm.toml"), "{body}");
        assert!(body.contains("scm_plan(\"1001-scm.toml\")"), "{body}");
        assert!(body.contains("model = \"1001.mod\""));
        assert!(body.contains("covariates = []"));
        assert!(body.contains("direction = [\"forward\", \"backward\"]"));
        assert!(!body.contains("out_dir"));
        // every optional setting present at its default
        for expected in [
            "forward_alpha = 0.05",
            "backward_alpha = 0.001",
            "max_retries = 3",
            "cov_step = false",
            "release_init = 0.1",
        ] {
            assert!(body.contains(expected), "missing {expected} in:\n{body}");
        }
    }

    /// The written file must be the config loader's own dialect: it parses,
    /// and the only thing standing between it and a plan is the covariates
    /// the user has yet to name.
    #[test]
    fn the_initialized_config_parses_and_only_needs_covariates() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template_content(dir.path(), TEMPLATE);
        let init = init_scm(&model, false).unwrap();

        let config = ScmConfig::load(&init.config_path).unwrap();
        assert_eq!(config.model, PathBuf::from("1001.mod"));
        assert!(config.covariates.is_empty());
        assert_eq!(
            config.direction,
            vec![Direction::Forward, Direction::Backward]
        );
        assert_eq!(config.forward_alpha, Some(0.05));
        assert_eq!(config.cov_step, Some(false));

        // empty candidates is the one thing left to fill in
        let err = build_plan_from_config(&init.config_path, &ScmPlanOverrides::default(), "test")
            .unwrap_err();
        assert!(format!("{err:#}").contains("covariates"), "got: {err:#}");

        // fill them in and the config plans as written
        let filled = fs::read_to_string(&init.config_path)
            .unwrap()
            .replace("covariates = []", "covariates = [\"WT_CL\", \"CRCL_CL\"]");
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
                .contains("covariates = []")
        );
    }

    #[test]
    fn init_rejects_a_missing_model_or_a_bad_extension() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope.mod");
        let err = init_scm(&missing, false).unwrap_err();
        assert!(
            format!("{err:#}").contains("does not exist"),
            "got: {err:#}"
        );

        let bad = dir.path().join("1001.txt");
        fs::write(&bad, "x").unwrap();
        let err = init_scm(&bad, false).unwrap_err();
        assert!(
            format!("{err:#}").contains("unsupported extension"),
            "got: {err:#}"
        );
    }
}
