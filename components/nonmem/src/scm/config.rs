//! The SCM configuration file: a TOML file that sets up an SCM process.

use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use fs_err as fs;
use serde::Deserialize;
use serde::de::{self, Deserializer, MapAccess, Visitor};

use super::plan::{BuiltPlan, build_plan};
use super::round::file_stem_of;
use super::{
    CovariateDefaults, CovariateRequest, Covariates, Direction, ScmOptions, parent_or_dot,
};
use crate::validate_model_extension;

/// The suffix an SCM config file carries: `<model stem>-scm.toml`, inside
/// the `scm/<model stem>` directory it plans an SCM process for.
pub const CONFIG_SUFFIX: &str = "-scm.toml";

/// The parsed SCM config file. Unknown keys are rejected so a typo'd option
/// fails loudly instead of silently falling back to a default.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScmConfig {
    /// Path to the initial model, relative to this file.
    pub model: PathBuf,
    /// The candidate covariate effects and their defaults; see
    /// [`CovariatesSection`].
    pub covariates: CovariatesSection,
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
    /// Whether the final model is re-fitted with the covariance step on
    /// once the SCM process finishes (default: true).
    pub final_cov_step: Option<bool>,
}

/// The `[covariates]` table: section-wide defaults and the effects to test.
///
/// Each entry of `effects` is either a bare theta name (`"WT_CL"`),
/// which takes every default, or a row (`{ name = "SEXEFF_CL", initial =
/// 1.2, fixed = 1 }`) that overrides whichever of them it spells out. The
/// two forms mix freely in one array.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct CovariatesSection {
    /// Default initial estimate for an effect the first time it is tested.
    pub initial: Option<f64>,
    /// Default value a held-out effect's theta is fixed at. Spelled `off`
    /// before the rename; that spelling is still accepted.
    #[serde(alias = "off")]
    pub fixed: Option<f64>,
    /// Default lower bound for an effect while it is in the model. Left
    /// unset, each candidate keeps whatever bound its `$THETA` spec in the initial model
    /// spec carries.
    pub lower: Option<f64>,
    /// Default upper bound; see [`CovariatesSection::lower`].
    pub upper: Option<f64>,
    /// The candidate effects, as names or rows.
    #[serde(default)]
    pub effects: Vec<CovariateEntry>,
}

impl CovariatesSection {
    /// The request `build_plan` takes, with `initial_override` (the
    /// `scm plan` call's `--initial`) beating the section's own default.
    pub fn to_request(&self, initial_override: Option<f64>) -> Covariates {
        let builtin = CovariateDefaults::default();
        Covariates {
            defaults: CovariateDefaults {
                initial: initial_override.or(self.initial).unwrap_or(builtin.initial),
                fixed: self.fixed.unwrap_or(builtin.fixed),
                lower: self.lower,
                upper: self.upper,
            },
            effects: self
                .effects
                .iter()
                .map(|e| match e {
                    CovariateEntry::Name(name) => CovariateRequest::named(name),
                    CovariateEntry::Row(row) => CovariateRequest {
                        name: row.name.clone(),
                        initial: row.initial,
                        fixed: row.fixed,
                        lower: row.lower,
                        upper: row.upper,
                    },
                })
                .collect(),
        }
    }
}

/// One entry of `[covariates] effects`.
#[derive(Debug, Clone, PartialEq)]
pub enum CovariateEntry {
    /// `"WT_CL"`: the name alone, both values at the section default.
    Name(String),
    /// `{ name = "WT_CL", initial = 0.2, fixed = 0 }`.
    Row(CovariateRow),
}

/// The long form of an effect entry.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CovariateRow {
    /// The name the initial model's `$THETA` record gives this theta:
    /// its `$THETA` label, or the name its comment carries.
    pub name: String,
    /// This effect's initial estimate the first time it is tested.
    pub initial: Option<f64>,
    /// What this effect's theta is fixed at when held out. Spelled `off`
    /// before the rename; that spelling is still accepted.
    #[serde(alias = "off")]
    pub fixed: Option<f64>,
    /// Lower bound this effect is estimated under while it is in the model.
    pub lower: Option<f64>,
    /// Upper bound this effect is estimated under while it is in the model.
    pub upper: Option<f64>,
}

impl<'de> Deserialize<'de> for CovariateEntry {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EntryVisitor;

        impl<'de> Visitor<'de> for EntryVisitor {
            type Value = CovariateEntry;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str(
                    "a theta name (\"WT_CL\") or a row ({ name = \"WT_CL\", initial = 0.1, fixed = 0 })",
                )
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(CovariateEntry::Name(v.to_string()))
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                let row = CovariateRow::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(CovariateEntry::Row(row))
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

/// The section a config has to spell out now, shown when an older spelling
/// is met.
const SECTION_EXAMPLE: &str = "\
[covariates]
initial = 0.1
fixed = 0
effects = [\"WT_CL\", \"CRCL_CL\"]";

impl ScmConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read SCM config {}", path.display()))?;
        Self::parse(&content)
            .with_context(|| format!("failed to parse SCM config {}", path.display()))
    }

    /// Parse the file's text. Spellings from before the `[covariates]`
    /// section existed — a top-level `covariates` array, a top-level
    /// `release_init` — are refused with the section to write instead,
    /// rather than falling through to serde's bare unknown-field error.
    pub fn parse(content: &str) -> Result<Self> {
        let table: toml::Table = toml::from_str(content)?;
        if let Some(covariates) = table.get("covariates")
            && covariates.is_array()
        {
            bail!(
                "`covariates` is now a section, not a top-level array; write\n\n{SECTION_EXAMPLE}\n\n\
                 (bare names take the section's `initial` / `fixed` defaults; a row \
                 {{ name = \"SEX_CL\", initial = 1.2, fixed = 1 }} overrides them)"
            );
        }
        if table.contains_key("release_init") {
            bail!("`release_init` is now `initial` under [covariates]; write\n\n{SECTION_EXAMPLE}");
        }
        Ok(toml::from_str(content)?)
    }
}

/// Per-invocation knobs on `scm plan` that are not part of the config file.
/// `num_rounds` paces this run of the SCM process; the rest override the config.
#[derive(Debug, Clone, Default)]
pub struct ScmPlanOverrides {
    pub num_rounds: Option<usize>,
    pub max_retries: Option<usize>,
    pub cov_step: Option<bool>,
    /// Overrides the `[covariates]` section's default `initial`.
    pub initial: Option<f64>,
    pub overwrite: bool,
}

fn resolve(base: &Path, p: &Path) -> PathBuf {
    if p.is_relative() {
        normalize(&base.join(p))
    } else {
        p.to_path_buf()
    }
}

/// Collapse `a/b/../c` into `a/c` lexically. The config lives two levels
/// below the model it plans for, so every model path it carries starts
/// `../../`; without this the plan's model and out_dir would both be spelled
/// `scm/1001/../../1001.mod`, which is the right file by the wrong name.
/// Purely textual — no filesystem access, so it never resolves symlinks —
/// and a leading `..` that has nothing to pop is kept.
fn normalize(path: &Path) -> PathBuf {
    let mut out: Vec<Component> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir if matches!(out.last(), Some(Component::Normal(_))) => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    if out.is_empty() {
        return PathBuf::from(".");
    }
    out.iter().collect()
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
        cov_step: overrides
            .cov_step
            .or(config.cov_step)
            .unwrap_or(defaults.cov_step),
        final_cov_step: config.final_cov_step.unwrap_or(defaults.final_cov_step),
        overwrite: overrides.overwrite,
    };

    let covariates = config.covariates.to_request(overrides.initial);
    build_plan(&model, &covariates, None, options, pharos_version)
}

/// Where a model's SCM config belongs: `<stem>-scm.toml` inside the SCM
/// process's own directory, `scm/<stem>` beside the model.
pub fn config_path_for(model: &Path) -> Result<PathBuf> {
    let stem = file_stem_of(model)
        .with_context(|| format!("model file {} has no file stem", model.display()))?;
    Ok(out_dir_for(model)?.join(format!("{stem}{CONFIG_SUFFIX}")))
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
    /// The config file written, ready for the user to fill in. It lives in
    /// [`ScmInit::out_dir`].
    pub config_path: PathBuf,
    /// The SCM process directory created beside the model.
    pub out_dir: PathBuf,
}

/// Set up an SCM process for `model`: create the `scm/<stem>` directory the
/// SCM process will write into and write `<stem>-scm.toml` inside it. Fills
/// the config's optional settings in at their defaults and leaves the effects
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

    // Relative paths in the config resolve against the config's own
    // directory, and the config sits two levels below the model in
    // scm/<stem>/, so the model is named `../../<file>`.
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
/// `effects` deliberately empty — and every optional key spelled out at
/// its default, annotated so the file explains itself.
fn render_init_config(model_file: &str, stem: &str) -> String {
    let d = ScmOptions::default();
    let c = CovariateDefaults::default();
    // Optional settings carry their meaning in a trailing comment, aligned.
    let opt = |setting: String, comment: &str| format!("{setting:<24} # {comment}\n");

    format!(
        "\
# SCM setup for {model_file}, written by SCM init.
#
# This file lives in the SCM process's own directory, scm/{stem}/, beside
# everything the process writes.
#
# Fill in `effects` under [covariates] below, then plan and run the SCM process.
#
# Paths resolve against this file's own directory, so the SCM process runs
# from anywhere — which is why the model two levels up is named ../../.

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
            "retries per failed fit, each from the previous attempt's estimates"
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
            format!("initial = {}", c.initial),
            "default initial estimate for an effect the first time it is tested, unless the initial model gives it one"
        ),
        fixed = opt(
            format!("fixed = {}", c.fixed),
            "default value a held-out effect's theta is fixed at"
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
direction = ["forward", "backward"]
[covariates]
effects = ["WT_CL", "CRCL_CL", "WT_V"]
"#;

    /// `lower` / `upper` in the section and on a row reach the plan's
    /// candidates; an effect with neither stays unbounded.
    #[test]
    fn bounds_from_the_section_and_from_a_row() {
        let dir = tempfile::tempdir().unwrap();
        write_template_content(dir.path(), TEMPLATE);
        let config_path = write_config(
            dir.path(),
            r#"
model = "1001.mod"
direction = ["forward"]
[covariates]
lower = 0
effects = ["WT_CL", { name = "CRCL_CL", initial = 1.2, fixed = 1, lower = 0.01, upper = 10 }]
"#,
        );
        let built =
            build_plan_from_config(&config_path, &ScmPlanOverrides::default(), "test").unwrap();
        let c = &built.plan.candidates;
        assert_eq!((c[0].lower, c[0].upper), (Some(0.0), None));
        assert_eq!((c[1].lower, c[1].upper), (Some(0.01), Some(10.0)));
        assert_eq!(c[1].bounds_label().as_deref(), Some("(0.01, 10)"));

        // the same config without any bounds leaves the candidates unbounded
        let plain = write_config(
            dir.path(),
            "model = \"1001.mod\"\ndirection = [\"forward\"]\n[covariates]\neffects = [\"WT_CL\"]\n",
        );
        let built = build_plan_from_config(&plain, &ScmPlanOverrides::default(), "test").unwrap();
        assert_eq!(built.plan.candidates[0].bounds_label(), None);
    }

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
        assert_eq!(plan.candidates[0].initial, 0.1);
        assert_eq!(plan.candidates[0].fixed, 0.0);
        assert_eq!(plan.options, ScmOptions::default());
        assert!(plan.out_dir.ends_with("scm/1001"));
    }

    /// Bare names and rows mix in one array; a row overrides only what it
    /// spells out, and the section defaults fill the rest.
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

        // call-site overrides beat the config; the `initial` override moves
        // the section default, never a row's own value
        let overrides = ScmPlanOverrides {
            num_rounds: Some(2),
            max_retries: Some(1),
            cov_step: Some(false),
            initial: Some(0.05),
            overwrite: true,
        };
        let built = build_plan_from_config(&config_path, &overrides, "test").unwrap();
        assert_eq!(built.plan.options.num_rounds, Some(2));
        assert_eq!(built.plan.options.max_retries, 1);
        assert!(!built.plan.options.cov_step);
        assert_eq!(built.plan.candidates[0].initial, 0.05);
        assert_eq!(built.plan.candidates[2].initial, 0.7);
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
direction = ["forward"]
[covariates]
effects = ["WT_CL"]
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
        for (key, body) in [
            (
                "foward_alpha",
                "model = \"1001.mod\"\ndirection = [\"forward\"]\nfoward_alpha = 0.01\n[covariates]\neffects = [\"WT_CL\"]\n",
            ),
            (
                "inital",
                "model = \"1001.mod\"\ndirection = [\"forward\"]\n[covariates]\ninital = 0.2\neffects = [\"WT_CL\"]\n",
            ),
            (
                "iniital",
                "model = \"1001.mod\"\ndirection = [\"forward\"]\n[covariates]\neffects = [{ name = \"WT_CL\", iniital = 0.2 }]\n",
            ),
        ] {
            let config_path = write_config(dir.path(), body);
            let err = build_plan_from_config(&config_path, &ScmPlanOverrides::default(), "test")
                .unwrap_err();
            assert!(format!("{err:#}").contains(key), "got: {err:#}");
        }
    }

    /// The flat spellings from before the section existed are refused with
    /// the section to write instead.
    #[test]
    fn old_flat_spellings_are_rejected_with_guidance() {
        let dir = tempfile::tempdir().unwrap();
        write_template_content(dir.path(), TEMPLATE);

        let flat = write_config(
            dir.path(),
            "model = \"1001.mod\"\ncovariates = [\"WT_CL\"]\ndirection = [\"forward\"]\n",
        );
        let err = build_plan_from_config(&flat, &ScmPlanOverrides::default(), "test").unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("now a section"), "got: {msg}");
        assert!(msg.contains("[covariates]"), "got: {msg}");

        let release = write_config(
            dir.path(),
            "model = \"1001.mod\"\ndirection = [\"forward\"]\nrelease_init = 0.2\n[covariates]\neffects = [\"WT_CL\"]\n",
        );
        let err =
            build_plan_from_config(&release, &ScmPlanOverrides::default(), "test").unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("`initial` under [covariates]"), "got: {msg}");

        // out_dir was dropped earlier and still fails as an unknown key
        let out_dir = write_config(
            dir.path(),
            "model = \"1001.mod\"\nout_dir = \"x\"\ndirection = [\"forward\"]\n[covariates]\neffects = [\"WT_CL\"]\n",
        );
        let err =
            build_plan_from_config(&out_dir, &ScmPlanOverrides::default(), "test").unwrap_err();
        assert!(format!("{err:#}").contains("out_dir"), "got: {err:#}");
    }

    /// THETA numbers used to be a second spelling of the candidates.
    /// They are not accepted any more, and the error has to say what to
    /// write instead — a config from an older pharos lands here.
    #[test]
    fn theta_numbers_in_the_effects_array_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        write_template_content(dir.path(), TEMPLATE);

        for array in [r#"[4, 5, 6]"#, r#"[6, "WT_CL"]"#] {
            let config_path = write_config(
                dir.path(),
                &format!(
                    "model = \"1001.mod\"\ndirection = [\"forward\"]\n[covariates]\neffects = {array}\n"
                ),
            );
            let err = build_plan_from_config(&config_path, &ScmPlanOverrides::default(), "test")
                .unwrap_err();
            let msg = format!("{err:#}");
            assert!(msg.contains("named, not numbered"), "got: {msg}");
            assert!(msg.contains("$THETA"), "got: {msg}");
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

        // a row without a name is not a candidate
        let config_path = write_config(
            dir.path(),
            "model = \"1001.mod\"\ndirection = [\"forward\"]\n[covariates]\neffects = [{ initial = 0.2 }]\n",
        );
        let err =
            build_plan_from_config(&config_path, &ScmPlanOverrides::default(), "test").unwrap_err();
        assert!(format!("{err:#}").contains("name"), "got: {err:#}");
    }

    // init ---------------------------------------------------------------

    #[test]
    fn init_writes_the_config_into_the_scm_dir_it_makes() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template_content(dir.path(), TEMPLATE);

        let init = init_scm(&model, false).unwrap();
        assert_eq!(init.out_dir, dir.path().join("scm").join("1001"));
        assert_eq!(init.config_path, init.out_dir.join("1001-scm.toml"));
        assert!(init.out_dir.is_dir());

        let body = fs::read_to_string(&init.config_path).unwrap();
        // The config sits two levels below the model it plans for.
        assert!(body.contains("model = \"../../1001.mod\""));
        assert!(body.contains("[covariates]"));
        assert!(body.contains("effects = []"));
        assert!(body.contains("direction = [\"forward\", \"backward\"]"));
        assert!(!body.contains("out_dir"));
        assert!(!body.contains("release_init"));
        // every optional setting present at its default
        for expected in [
            "forward_alpha = 0.05",
            "backward_alpha = 0.001",
            "max_retries = 3",
            "cov_step = false",
            "final_cov_step = true",
            "initial = 0.1",
            "fixed = 0",
        ] {
            assert!(body.contains(expected), "missing {expected} in:\n{body}");
        }
    }

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
        assert_eq!(
            config.direction,
            vec![Direction::Forward, Direction::Backward]
        );
        assert_eq!(config.forward_alpha, Some(0.05));
        assert_eq!(config.cov_step, Some(false));
        assert_eq!(config.final_cov_step, Some(true));

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
