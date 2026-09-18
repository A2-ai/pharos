use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use fs_err as fs;
use nonmem_parser::{CommentType, Model, ParsedThetaComment, parse_theta_param};
use utils::get_utc_now;

use super::{
    Candidate, Covariates, PlanContext, ScmOptions, ScmPlan, path_for_plan, project_config,
};
use crate::{ModelLayout, check_dataset};

/// A built plan
#[derive(Debug, Clone)]
pub struct BuiltPlan {
    pub plan: ScmPlan,
    pub warnings: Vec<String>,
    pub context: PlanContext,
}

impl BuiltPlan {
    /// The plan rendered with its out_dir's history
    pub fn render_text(&self) -> String {
        self.plan.render_text_with(&self.context)
    }
}

/// One theta named one way.
#[derive(Debug, Clone)]
struct NameHit {
    /// 1-based THETA number.
    theta: usize,
    /// The name as the model spells it
    as_written: String,
}

/// Every name the initial model's `$THETA` records give a theta under the
/// project's comment dialect, keyed by the name uppercased.
/// The names are [`Model::get_parameter_names`]', the ones `pharos nonmem summary` prints
fn theta_name_index(
    model: &Model,
    comment_type: CommentType,
) -> Result<BTreeMap<String, Vec<NameHit>>> {
    let mut index: BTreeMap<String, Vec<NameHit>> = BTreeMap::new();
    for (param, name) in model.get_parameter_names(Some(comment_type))? {
        // Thetas only; the map carries the omegas and sigmas too.
        let (Some(theta), Some(name)) = (
            param.strip_prefix("THETA").and_then(|n| n.parse().ok()),
            name,
        ) else {
            continue;
        };
        index
            .entry(name.to_ascii_uppercase())
            .or_default()
            .push(NameHit {
                theta,
                as_written: name,
            });
    }
    Ok(index)
}

/// Thetas whose comment claims a position that is not the one they sit at.
fn stale_theta_comment_numbers(model: &Model, comment_type: CommentType) -> Vec<(usize, usize)> {
    model
        .thetas
        .iter()
        .enumerate()
        .filter_map(|(idx0, t)| {
            let ParsedThetaComment::Type2(parsed) =
                parse_theta_param(t.comment.as_deref()?, comment_type)?
            else {
                return None;
            };
            let digits = parsed.prefix?;
            let n: usize = digits
                .trim_start_matches(|c: char| !c.is_ascii_digit())
                .trim_end_matches(|c: char| !c.is_ascii_digit())
                .parse()
                .ok()?;
            (n != idx0 + 1).then_some((idx0 + 1, n))
        })
        .collect()
}

/// One requested name, resolved.
struct Resolved {
    /// 1-based THETA number the name landed on.
    theta: usize,
    name: String,
}

/// Resolve the names the config requested, matching case-insensitively so a
/// request need not reproduce the author's capitalization.
fn resolve_theta_names(
    model: &Model,
    names: &[String],
    comment_type: CommentType,
) -> Result<Vec<Resolved>> {
    let index = theta_name_index(model, comment_type)?;

    let mut resolved: Vec<Resolved> = Vec::new();
    for requested in names {
        let requested = requested.trim();
        if requested.is_empty() {
            bail!("covariates contains an empty name");
        }

        let hits = index.get(&requested.to_ascii_uppercase());
        let Some(hits) = hits.filter(|h| !h.is_empty()) else {
            bail!("{}", not_found_message(requested, &index));
        };

        let mut thetas: Vec<usize> = hits.iter().map(|h| h.theta).collect();
        thetas.sort_unstable();
        thetas.dedup();
        if thetas.len() > 1 {
            let named: Vec<String> = thetas.iter().map(|t| format!("THETA({t})")).collect();
            bail!(
                "covariate name {requested} is ambiguous in the initial model: \
                 it names {}\nrename one of them, or request the name that identifies \
                 only the one you mean",
                named.join(" and ")
            );
        }
        let theta = thetas[0];

        // Each theta is named once, so a theta already resolved can only
        // have been reached by the same name a second time.
        if resolved.iter().any(|r| r.theta == theta) {
            bail!("covariate name {requested} is requested more than once");
        }
        resolved.push(Resolved {
            theta,
            name: hits[0].as_written.clone(),
        });
    }
    Ok(resolved)
}

fn not_found_message(requested: &str, index: &BTreeMap<String, Vec<NameHit>>) -> String {
    let mut available: Vec<&str> = Vec::new();
    for hit in index.values().flatten() {
        if !available.contains(&hit.as_written.as_str()) {
            available.push(&hit.as_written);
        }
    }
    available.sort_unstable();

    let mut msg = format!("no theta named {requested} in the initial model");
    if available.is_empty() {
        msg.push_str(
            "\n  no $THETA record in this model carries a comment naming it, so no covariate \
             effect can be requested; give each candidate theta its own $THETA record with a \
             comment naming it, e.g. `$THETA (0 FIX)   ; WT_CL`",
        );
    } else {
        msg.push_str(&format!("\n  named by a comment: {}", available.join(", ")));
    }

    msg
}

/// Build and validate an SCM plan.
pub fn build_plan(
    model_path: &Path,
    covariates: &Covariates,
    out_dir: Option<&Path>,
    options: ScmOptions,
    pharos_version: &str,
) -> Result<BuiltPlan> {
    options.validate()?;

    if covariates.effects.is_empty() {
        bail!(
            "[covariates] effects must name at least one theta, e.g. effects = [\"WT_CL\", \"CRCL_CL\"]"
        );
    }
    for (label, value) in [
        ("initial", covariates.default_initial()),
        ("fixed", covariates.default_fixed()),
    ] {
        if !value.is_finite() {
            bail!("[covariates] {label} must be a finite number, got {value}");
        }
    }
    if covariates.default_initial() == covariates.default_fixed() {
        bail!(
            "[covariates] initial ({}) equals fixed ({}): an effect whose initial estimate is its held-out value is not tested at all",
            covariates.default_initial(),
            covariates.default_fixed()
        );
    }
    if let (Some(lower), Some(upper)) = (covariates.lower, covariates.upper)
        && lower >= upper
    {
        bail!("[covariates] lower ({lower}) must be below upper ({upper})");
    }

    // The initial model
    if !model_path.exists() {
        bail!("Model file does not exist: {}", model_path.display());
    }
    let layout = ModelLayout::for_model_path(model_path)?;
    let model = Model::parse(model_path, &fs::read_to_string(model_path)?)
        .with_context(|| format!("failed to parse initial model {}", model_path.display()))?;

    if model.estimations.is_empty() {
        bail!(
            "initial model {} has no $ESTIMATION record",
            model_path.display()
        );
    }
    check_dataset(&model, layout.model_dir()).with_context(|| {
        format!(
            "dataset {} referenced by $DATA of {} does not exist",
            model.data.path,
            model_path.display()
        )
    })?;

    // Names resolve under the dialect the model's project declares
    let project = project_config(layout.model_dir())?;
    // The SCM process reads every run back by re-rendering the project's
    // `output_dir` for the model, so the template has to render the same
    // name every time.
    if project
        .output_dir
        .as_deref()
        .is_some_and(|t| t.contains("timestamp"))
    {
        bail!(
            "this project's `output_dir` template ({}) carries a timestamp, so a run's directory \
             cannot be found again once it is written; the SCM process needs a stable template",
            project.output_dir.as_deref().unwrap_or_default()
        );
    }
    let Some(comment_type) = project.comments.r#type else {
        bail!(
            "this project sets no comment dialect, so no $THETA comment names a theta; \
             set `type` under [nonmem.comments] in pharos.toml"
        );
    };

    // Resolve the request to `(theta number, canonical name)
    let names: Vec<String> = covariates.effects.iter().map(|e| e.name.clone()).collect();
    let mut selected = resolve_theta_names(&model, &names, comment_type)?;
    selected.sort_unstable_by_key(|r| r.theta);

    let mut warnings = Vec::new();
    let mut candidates = Vec::new();

    let stale = stale_theta_comment_numbers(&model, comment_type);

    for Resolved {
        theta: theta_num,
        name,
    } in &selected
    {
        let theta_num = *theta_num;
        let idx0 = theta_num - 1;
        let request = covariates
            .effects
            .iter()
            .find(|e| e.name.trim().eq_ignore_ascii_case(name))
            .expect("every resolved name came from a request");
        // The name index is built from `model.thetas`
        let theta = &model.thetas[idx0];

        if let Some((_, label)) = stale.iter().find(|(t, _)| *t == theta_num) {
            warnings.push(format!(
                "THETA({theta_num}) [{name}] has a comment numbered {label}; \
                 the comment numbering looks stale"
            ));
        }

        let fixed = request.fixed.unwrap_or(covariates.default_fixed());

        let initial = match request.initial {
            Some(v) => v,
            None if theta.init != fixed => theta.init,
            None => covariates.default_initial(),
        };
        for (label, value) in [("initial", initial), ("fixed", fixed)] {
            if !value.is_finite() {
                bail!("{name}: {label} must be a finite number, got {value}");
            }
        }
        if initial == fixed {
            bail!(
                "{name}: initial ({initial}) equals fixed ({fixed}): an effect whose initial estimate is its held-out value is not tested at all"
            );
        }

        if theta.fixed && theta.init != fixed {
            warnings.push(format!(
                "THETA({theta_num}) [{name}] is fixed at {} in the initial model but fixed = {fixed} \
                 in the config; generated models hold the effect out at {fixed}",
                theta.init
            ));
        }

        let candidate = Candidate {
            name: name.clone(),
            theta: theta_num,
            initial,
            fixed,
            lower: request.lower.or(covariates.lower).or(theta.lower),
            upper: request.upper.or(covariates.upper).or(theta.upper),
        };

        candidate.released_spec().validate(name)?;
        candidates.push(candidate);
    }

    if options.cov_step && model.covariance.is_none() {
        warnings.push(
            "initial model has no $COVARIANCE record; cov_step is on, so one will be appended to generated models"
                .to_string(),
        );
    }
    if !options.cov_step && model.covariance.is_some() {
        let round_models = if options.final_cov_step {
            "the round models"
        } else {
            "generated models"
        };
        warnings.push(format!(
            "cov_step is off: the initial model's $COVARIANCE record will be removed from {round_models}"
        ));
    }
    if options.final_cov_step && !options.cov_step && model.covariance.is_none() {
        warnings.push(
            "initial model has no $COVARIANCE record; final_cov_step is on, so one will be appended to the final model"
                .to_string(),
        );
    }

    let out_dir = match out_dir {
        Some(d) => d.to_path_buf(),
        None => super::default_out_dir(&layout),
    };

    // Paths go into the plan relative to the project root
    let root = config::find_config_dir_from(layout.model_dir())?.unwrap_or_default();

    let plan = ScmPlan {
        created: get_utc_now(),
        pharos_version: pharos_version.to_string(),
        model: path_for_plan(model_path, &root),
        out_dir: path_for_plan(&out_dir, &root),
        root,
        candidates,
        options,
    };

    // Read the out_dir before anything writes to it
    let context = PlanContext::read(&plan);

    Ok(BuiltPlan {
        plan,
        warnings,
        context,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scm::test_support::{
        TEMPLATE, opts_cov_on, write_project_config, write_template, write_template_content,
    };

    #[test]
    fn candidates_are_listed_in_theta_order_however_they_were_requested() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template(dir.path());
        let built = build_plan(
            &model_path,
            &Covariates::named(&["WT_V", "WT_CL", "CRCL_CL"]),
            None,
            opts_cov_on(),
            "test",
        )
        .unwrap();
        let got: Vec<(&str, usize)> = built
            .plan
            .candidates
            .iter()
            .map(|c| (c.name.as_str(), c.theta))
            .collect();
        assert_eq!(got, vec![("WT_CL", 4), ("CRCL_CL", 5), ("WT_V", 6)]);
        assert!(built.plan.out_dir.ends_with("scm/1001"));
        assert!(built.warnings.is_empty(), "warnings: {:?}", built.warnings);

        // save + load round trip
        let loaded = ScmPlan::load(built.plan.save().unwrap()).unwrap();
        assert_eq!(loaded, built.plan);
    }

    #[test]
    fn name_matching_is_case_insensitive_but_keeps_the_authored_spelling() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template(dir.path());
        let built = build_plan(
            &model_path,
            &Covariates::named(&["wt_cl"]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap();
        assert_eq!(built.plan.candidates[0].name, "WT_CL");
        assert_eq!(built.plan.candidates[0].theta, 4);
    }

    #[test]
    fn a_covariate_is_requested_by_the_name_the_summary_shows() {
        // The guarantee: the names `scm plan` accepts are exactly the names
        // `Model::get_parameter_names` gives the thetas, which is what
        // `pharos nonmem summary` prints. `V2/F` stands in for the names a
        // rule of our own would be tempted to reject.
        let dir = tempfile::tempdir().unwrap();
        let varied = TEMPLATE
            .replace("; CRCL_CL cov", "; V2/F cov")
            .replace("; WT_V cov", "; WT_V (-) :LOG");
        let model_path = write_template_content(dir.path(), &varied);

        let model = Model::parse(&model_path, &fs::read_to_string(&model_path).unwrap()).unwrap();
        let summary_names = model.get_parameter_names(Some(CommentType::Type1)).unwrap();

        for idx0 in 0..model.thetas.len() {
            let theta_num = idx0 + 1;
            let shown = summary_names
                .get(&format!("THETA{theta_num}"))
                .expect("every theta is in the map");
            let Some(shown) = shown else {
                continue;
            };
            let built = build_plan(
                &model_path,
                &Covariates::named(&[shown]),
                None,
                opts_cov_on(),
                "test",
            )
            .unwrap_or_else(|e| panic!("summary shows THETA{theta_num} as {shown}: {e:#}"));
            assert_eq!(built.plan.candidates[0].name, *shown);
            assert_eq!(built.plan.candidates[0].theta, theta_num);
        }
    }

    #[test]
    fn stale_comment_numbering_warns() {
        let dir = tempfile::tempdir().unwrap();
        // Type2 reads a leading number as the theta's position, so a comment
        // can name its theta and misnumber it at the same time; Type1 cannot,
        // since a leading number leaves the comment naming nothing.
        for prefix in ["9", "THETA(9)"] {
            let stale = TEMPLATE.replace("; WT_CL cov", &format!("; {prefix} WT_CL"));
            let model_path = write_template_content(dir.path(), &stale);
            write_project_config(dir.path(), CommentType::Type2);
            let built = build_plan(
                &model_path,
                &Covariates::named(&["WT_CL"]),
                None,
                ScmOptions::default(),
                "test",
            )
            .unwrap_or_else(|e| panic!("{prefix}: {e:#}"));
            assert_eq!(built.plan.candidates[0].name, "WT_CL");
            assert!(
                built
                    .warnings
                    .iter()
                    .any(|w| w.contains("numbered 9") && w.contains("stale")),
                "{prefix} warnings: {:?}",
                built.warnings
            );
        }
    }

    #[test]
    fn cov_step_warnings() {
        let dir = tempfile::tempdir().unwrap();

        // no $COVARIANCE in initial model + cov_step on -> warn about appending
        let no_cov = TEMPLATE.replace("$COVARIANCE\n", "");
        let model_path = write_template_content(dir.path(), &no_cov);
        let built = build_plan(
            &model_path,
            &Covariates::named(&["WT_CL"]),
            None,
            opts_cov_on(),
            "test",
        )
        .unwrap();
        assert!(built.warnings.iter().any(|w| w.contains("appended")));

        // $COVARIANCE present + cov_step off -> warn about removal
        let model_path = write_template(dir.path());
        let opts = ScmOptions {
            cov_step: false,
            ..Default::default()
        };
        let built = build_plan(
            &model_path,
            &Covariates::named(&["WT_CL"]),
            None,
            opts,
            "test",
        )
        .unwrap();
        assert!(built.warnings.iter().any(|w| w.contains("removed")));
    }

    /// Bounds per effect: the row's own value, else the section default, else
    /// the bound the initial model's own `$THETA` spec carries.
    #[test]
    fn bounds_resolve_row_then_section_then_template() {
        use crate::scm::CovariateRequest;
        let dir = tempfile::tempdir().unwrap();
        // WT_CL is authored bounded in the initial model; the others are `(0 FIX)`.
        let content = TEMPLATE.replace(
            "$THETA (0 FIX)   ; WT_CL cov",
            "$THETA (-2, 0.4, 2)   ; WT_CL cov",
        );
        let model_path = write_template_content(dir.path(), &content);
        let covariates = Covariates {
            lower: Some(0.0),
            effects: vec![
                // takes the section's lower; nothing supplies an upper
                CovariateRequest::named("CRCL_CL"),
                // its own bounds beat the section's
                CovariateRequest {
                    name: "WT_V".into(),
                    lower: Some(0.01),
                    upper: Some(10.0),
                    ..Default::default()
                },
                // the section's lower wins over the initial model's -2, and the
                // the initial model still supplies the upper the config leaves out
                CovariateRequest::named("WT_CL"),
            ],
            ..Default::default()
        };
        let built = build_plan(&model_path, &covariates, None, opts_cov_on(), "test").unwrap();
        let c = &built.plan.candidates;
        assert_eq!((c[0].lower, c[0].upper), (Some(0.0), Some(2.0))); // WT_CL
        assert_eq!((c[1].lower, c[1].upper), (Some(0.0), None)); // CRCL_CL
        assert_eq!((c[2].lower, c[2].upper), (Some(0.01), Some(10.0))); // WT_V
        assert_eq!(c[0].bounds_label().as_deref(), Some("(0, 2)"));

        // A config that says nothing about bounds keeps the initial model's, and
        // an unbounded theta stays unbounded.
        let built = build_plan(
            &model_path,
            &Covariates::named(&["WT_CL", "CRCL_CL"]),
            None,
            opts_cov_on(),
            "test",
        )
        .unwrap();
        let c = &built.plan.candidates;
        assert_eq!((c[0].lower, c[0].upper), (Some(-2.0), Some(2.0)));
        assert_eq!((c[1].lower, c[1].upper), (None, None));
        assert_eq!(c[1].bounds_label(), None);
    }

    /// `initial` and `off` per effect: a row's own value, else the initial model's
    /// estimate when it is not the off value, else the section default.
    #[test]
    fn initial_and_off_resolve_row_then_template_then_default() {
        use crate::scm::CovariateRequest;
        let dir = tempfile::tempdir().unwrap();
        // WT_CL carries a guess of its own (0.4); CRCL_CL is `(0 FIX)`; WT_V is
        // written as a fold-change effect fixed at 1.
        let content = TEMPLATE
            .replace("$THETA (0 FIX)   ; WT_CL cov", "$THETA 0.4   ; WT_CL cov")
            .replace("$THETA (0 FIX)   ; WT_V cov", "$THETA (1 FIX)   ; WT_V cov");
        let model_path = write_template_content(dir.path(), &content);
        let covariates = Covariates {
            initial: Some(0.2),
            fixed: Some(0.0),
            effects: vec![
                CovariateRequest::named("WT_CL"),
                CovariateRequest {
                    name: "CRCL_CL".into(),
                    initial: Some(0.9),
                    fixed: None,
                    ..Default::default()
                },
                CovariateRequest {
                    name: "WT_V".into(),
                    initial: None,
                    fixed: Some(1.0),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let built = build_plan(&model_path, &covariates, None, opts_cov_on(), "test").unwrap();
        let c = &built.plan.candidates;
        // the initial model's guess wins over the section default
        assert_eq!((c[0].initial, c[0].fixed), (0.4, 0.0));
        // the row's own initial wins over everything
        assert_eq!((c[1].initial, c[1].fixed), (0.9, 0.0));
        // `(1 FIX)` is the held-out spelling for off = 1, so the default applies
        assert_eq!((c[2].initial, c[2].fixed), (0.2, 1.0));
        assert!(built.warnings.is_empty(), "warnings: {:?}", built.warnings);
    }

    #[test]
    fn a_template_pinned_at_another_value_than_its_fixed_value_warns() {
        use crate::scm::CovariateRequest;
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template(dir.path());
        // The initial model says (0 FIX), the config holds the effect out at 1.
        let covariates = Covariates {
            effects: vec![CovariateRequest {
                name: "WT_CL".into(),
                initial: Some(1.2),
                fixed: Some(1.0),
                ..Default::default()
            }],
            ..Default::default()
        };
        let built = build_plan(
            &model_path,
            &covariates,
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap();
        assert_eq!(built.plan.candidates[0].fixed, 1.0);
        assert!(
            built
                .warnings
                .iter()
                .any(|w| w.contains("fixed at 0 in the initial model but fixed = 1")),
            "warnings: {:?}",
            built.warnings
        );
    }
}
