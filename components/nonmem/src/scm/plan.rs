use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use fs_err as fs;
use nonmem_parser::{CommentType, Model, ParsedThetaComment, parse_theta_param};
use utils::get_utc_now;

use super::project::RunSettings;
use super::{
    Candidate, Covariates, PLAN_SCHEMA_VERSION, PlanContext, ScmOptions, ScmPlan, max_models_for,
    path_for_plan,
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
///
/// The names are [`Model::get_parameter_names`]', the ones `pharos nonmem
/// summary` prints, so a covariate is requested by exactly the name its theta
/// is shown under everywhere else in pharos: one naming rule, not two.
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
///
/// Only Type2 comments carry a position: it is the `PREFIX` the parser
/// already reads (`THETA4`, `THETA(4)`, `4.`), so the number comes from
/// there. A Type1 comment has no such form.
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

/// When a requested name names no theta: prints every name that otherwise would
/// have worked
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
             comment naming it, e.g. `$THETA (0 FIX)   ; WT_CL cov`",
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

    // Names resolve under the dialect the model's project declares, the
    // same one `pharos nonmem summary` names its parameters with.
    let Some(comment_type) = RunSettings::discover_from(layout.model_dir())?.comment_type else {
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

        // What the effect is fixed at when held out: the row's own value,
        // else the section default.
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
        None => layout.model_dir().join("scm").join(layout.stem()),
    };

    // Paths go into the plan relative to the project root, so planning from
    // a subdirectory writes the same plan as planning from the root.
    let root = config::find_config_dir_from(layout.model_dir())?.unwrap_or_default();

    let plan = ScmPlan {
        schema_version: PLAN_SCHEMA_VERSION,
        created: get_utc_now(),
        pharos_version: pharos_version.to_string(),
        model: path_for_plan(model_path, &root),
        out_dir: path_for_plan(&out_dir, &root),
        root,
        max_models: max_models_for(candidates.len(), options.phases().len()),
        candidates,
        options,
    };

    // Read the out_dir before anything writes to it: what is there now is
    // the SCM process this plan is about to be laid over.
    let context = PlanContext::read(&plan);

    Ok(BuiltPlan {
        plan,
        warnings,
        context,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    // The fixtures every SCM test shares live in `scm::test_support`;
    // re-exported here so the other modules' `plan::tests::...` imports
    // keep working.
    pub(crate) use crate::scm::test_support::{
        INLINE_TEMPLATE, TEMPLATE, names, opts_cov_on, write_project_config, write_template,
        write_template_content,
    };

    #[test]
    fn builds_a_valid_plan() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template(dir.path());

        let built = build_plan(
            &model_path,
            &names(&["WT_CL", "CRCL_CL", "WT_V"]),
            None,
            opts_cov_on(),
            "test",
        )
        .unwrap();
        let plan = &built.plan;

        assert_eq!(plan.candidates.len(), 3);
        assert_eq!(plan.candidates[0].name, "WT_CL");
        assert_eq!(plan.candidates[0].theta, 4);
        assert_eq!(plan.candidates[1].name, "CRCL_CL");
        assert_eq!(plan.candidates[1].theta, 5);
        assert_eq!(plan.candidates[2].name, "WT_V");
        assert_eq!(plan.candidates[2].theta, 6);
        assert!(plan.out_dir.ends_with("scm/1001"));
        assert!(built.warnings.is_empty(), "warnings: {:?}", built.warnings);

        // save + load round trip
        let path = plan.save().unwrap();
        let loaded = ScmPlan::load(&path).unwrap();
        assert_eq!(&loaded, plan);
    }

    #[test]
    fn candidates_are_listed_in_theta_order_however_they_were_requested() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template(dir.path());
        let built = build_plan(
            &model_path,
            &names(&["WT_V", "WT_CL", "CRCL_CL"]),
            None,
            opts_cov_on(),
            "test",
        )
        .unwrap();
        let order: Vec<&str> = built
            .plan
            .candidates
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(order, vec!["WT_CL", "CRCL_CL", "WT_V"]);
    }

    #[test]
    fn name_matching_is_case_insensitive_but_keeps_the_authored_spelling() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template(dir.path());
        let built = build_plan(
            &model_path,
            &names(&["wt_cl"]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap();
        assert_eq!(built.plan.candidates[0].name, "WT_CL");
        assert_eq!(built.plan.candidates[0].theta, 4);
    }

    #[test]
    fn resolves_a_name_from_every_theta_spelling() {
        // The comment forms that name a theta, each under the dialect that
        // spells it that way. Each variant renames the WT_V candidate on
        // THETA(6) and requests it by that name; `$PK` is untouched and
        // never consulted.
        let cases = [
            (
                "bare comment",
                CommentType::Type2,
                "$THETA (0 FIX)   ; WT_V",
            ),
            (
                "prefixed comment",
                CommentType::Type2,
                "$THETA (0 FIX)   ; THETA6: WT_V",
            ),
            (
                "numbered comment",
                CommentType::Type2,
                "$THETA (0 FIX)   ; 6 WT_V",
            ),
            (
                "unit comment",
                CommentType::Type1,
                "$THETA (0 FIX)   ; WT_V (-) :LOG",
            ),
        ];
        for (label, dialect, spelling) in cases {
            let dir = tempfile::tempdir().unwrap();
            let content = TEMPLATE.replace("$THETA (0 FIX)   ; WT_V cov", spelling);
            let model_path = write_template_content(dir.path(), &content);
            // the project this spelling belongs to
            write_project_config(dir.path(), dialect);
            let built = build_plan(&model_path, &names(&["WT_V"]), None, opts_cov_on(), "test")
                .unwrap_or_else(|e| panic!("{label}: {e:#}"));
            assert_eq!(built.plan.candidates.len(), 1, "{label}");
            assert_eq!(built.plan.candidates[0].name, "WT_V", "{label}");
            assert_eq!(built.plan.candidates[0].theta, 6, "{label}");
        }
    }

    #[test]
    fn an_unknown_name_lists_the_names_that_would_have_worked() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template(dir.path());
        let err = build_plan(
            &model_path,
            &names(&["AGE_CL"]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("no theta named AGE_CL"), "got: {msg}");
        assert!(
            msg.contains("named by a comment:") && msg.contains("WT_CL") && msg.contains("CRCL_CL"),
            "got: {msg}"
        );
    }

    #[test]
    fn an_inline_model_plans_from_its_theta_comments() {
        let dir = tempfile::tempdir().unwrap();
        // The effects are folded into TVCL / V, so no `$PK` term names any
        // candidate theta. The `$THETA` comments do, which is all the SCM
        // process needs.
        let model_path = write_template_content(dir.path(), INLINE_TEMPLATE);
        let built = build_plan(
            &model_path,
            &names(&["WT_CL", "CRCL_CL", "WT_V"]),
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
    }

    #[test]
    fn an_ambiguous_name_errors_and_names_every_theta_it_could_mean() {
        let dir = tempfile::tempdir().unwrap();
        // THETA(3) and THETA(6) are both named WT_V by their comments.
        // Nothing in the model can break the tie.
        let clash = TEMPLATE.replace("; TVKA (1/h)", "; WT_V cov");
        let model_path = write_template_content(dir.path(), &clash);
        let err = build_plan(
            &model_path,
            &names(&["WT_V"]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("WT_V is ambiguous"), "got: {msg}");
        assert!(
            msg.contains("THETA(3)") && msg.contains("THETA(6)"),
            "got: {msg}"
        );
    }

    #[test]
    fn a_shared_comment_over_a_repeat_spec_is_ambiguous() {
        let dir = tempfile::tempdir().unwrap();
        // One comment covering an xN repeat names every theta it expands
        // to, so the name identifies none of them.
        let repeated = TEMPLATE.replace(
            "$THETA (0 FIX)   ; WT_CL cov\n$THETA (0 FIX)   ; CRCL_CL cov\n$THETA (0 FIX)   ; WT_V cov",
            "$THETA (0 FIX)x3   ; WT_CL cov",
        );
        let model_path = write_template_content(dir.path(), &repeated);
        let err = build_plan(
            &model_path,
            &names(&["WT_CL"]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("WT_CL is ambiguous"), "got: {msg}");
    }

    #[test]
    fn requesting_one_name_twice_errors() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template(dir.path());
        let err = build_plan(
            &model_path,
            &names(&["WT_CL", "wt_cl"]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("more than once"),
            "got: {}",
            err.to_string()
        );
    }

    #[test]
    fn rejects_an_empty_covariate_list() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template(dir.path());
        let err = build_plan(
            &model_path,
            &Covariates::default(),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("must name at least one theta"),
            "got: {err}"
        );
    }

    #[test]
    fn every_comment_form_names_its_theta() {
        let dir = tempfile::tempdir().unwrap();
        // The Type1 spellings of a candidate: the covariate form and the
        // two unit-style forms. Each names its candidate under the project's
        // dialect, and none of them warns.
        let varied = TEMPLATE
            .replace("; CRCL_CL cov", "; CRCL_CL (-) :LOG")
            .replace("; WT_V cov", "; WT_V (-)");
        let model_path = write_template_content(dir.path(), &varied);
        let built = build_plan(
            &model_path,
            &names(&["WT_CL", "CRCL_CL", "WT_V"]),
            None,
            opts_cov_on(),
            "test",
        )
        .unwrap();
        assert_eq!(built.plan.candidates[0].name, "WT_CL");
        assert_eq!(built.plan.candidates[2].name, "WT_V");
        assert!(built.warnings.is_empty(), "warnings: {:?}", built.warnings);
    }

    #[test]
    fn a_theta_with_no_name_at_all_cannot_be_requested() {
        let dir = tempfile::tempdir().unwrap();
        // No label, no comment: nothing in `$THETA` names THETA(6), so it
        // cannot be a candidate however `$PK` is written.
        let bare = TEMPLATE.replace("$THETA (0 FIX)   ; WT_V cov", "$THETA (0 FIX)");
        let model_path = write_template_content(dir.path(), &bare);
        let err = build_plan(
            &model_path,
            &names(&["WT_CL", "CRCL_CL", "WT_V"]),
            None,
            opts_cov_on(),
            "test",
        )
        .unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("no theta named WT_V"), "got: {msg}");
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
            let built = build_plan(&model_path, &names(&[shown]), None, opts_cov_on(), "test")
                .unwrap_or_else(|e| panic!("summary shows THETA{theta_num} as {shown}: {e:#}"));
            assert_eq!(built.plan.candidates[0].name, *shown);
            assert_eq!(built.plan.candidates[0].theta, theta_num);
        }
    }

    #[test]
    fn a_type1_comment_leading_with_a_number_names_nothing() {
        // A leading position is Type2's `PREFIX`; Type1 has no such form, so
        // `; 6 WT_V cov` names no theta here for the same reason it names
        // none in `pharos nonmem summary`.
        let dir = tempfile::tempdir().unwrap();
        let numbered = TEMPLATE.replace("; WT_V cov", "; 6 WT_V cov");
        let model_path = write_template_content(dir.path(), &numbered);
        let err =
            build_plan(&model_path, &names(&["WT_V"]), None, opts_cov_on(), "test").unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("no theta named WT_V"), "got: {msg}");
    }

    #[test]
    fn stale_comment_numbering_warns() {
        let dir = tempfile::tempdir().unwrap();
        // Type2 reads a leading number as the theta's position, so a comment
        // can name its theta and misnumber it at the same time; Type1 cannot,
        // since a leading number leaves the comment naming nothing.
        for prefix in ["9", "THETA9", "THETA(9)", "9."] {
            let stale = TEMPLATE.replace("; WT_CL cov", &format!("; {prefix} WT_CL"));
            let model_path = write_template_content(dir.path(), &stale);
            write_project_config(dir.path(), CommentType::Type2);
            let built = build_plan(
                &model_path,
                &names(&["WT_CL"]),
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
    fn a_candidate_written_as_a_free_theta_is_accepted() {
        let dir = tempfile::tempdir().unwrap();
        // An initial model that already carries an initial guess for the effect —
        // the shape of a model that has been fitted with the covariate in.
        let free = TEMPLATE.replace("$THETA (0 FIX)   ; WT_CL cov", "$THETA 0.1   ; WT_CL cov");
        let model_path = write_template_content(dir.path(), &free);
        let built = build_plan(
            &model_path,
            &names(&["WT_CL"]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap();
        assert_eq!(built.plan.candidates[0].name, "WT_CL");
        assert_eq!(built.plan.candidates[0].theta, 4);
        // Nothing about the theta's own shape is worth a warning.
        assert!(
            !built.warnings.iter().any(|w| w.contains("[WT_CL]")),
            "warnings: {:?}",
            built.warnings
        );
    }

    #[test]
    fn rejects_missing_dataset() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = dir.path().join("1001.mod");
        fs::write(&model_path, TEMPLATE).unwrap();
        let err = build_plan(
            &model_path,
            &names(&["WT_CL"]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap_err();
        assert!(err.to_string().contains("does not exist"), "got: {err}");
    }

    #[test]
    fn a_comment_no_dialect_parses_names_nothing() {
        let dir = tempfile::tempdir().unwrap();
        // `covv` is not the `cov` annotation, and no dialect parses the
        // comment. A theta is named the way the rest of pharos names one or
        // not at all, so THETA(6) cannot be requested.
        let odd = TEMPLATE.replace("; WT_V cov", "; WT_V covv");
        let model_path = write_template_content(dir.path(), &odd);
        let err = build_plan(
            &model_path,
            &names(&["WT_CL", "CRCL_CL", "WT_V"]),
            None,
            opts_cov_on(),
            "test",
        )
        .unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("no theta named WT_V"), "got: {msg}");
    }

    #[test]
    fn an_nmtran_label_does_not_name_a_theta() {
        let dir = tempfile::tempdir().unwrap();
        // `$THETA NAMES(...)` and `$THETA WT_V=(...)` are NM-TRAN naming
        // syntax, not comments. No other pharos command reads them as
        // parameter names, so neither does the SCM process: a model whose
        // candidate thetas carry only a label cannot be planned.
        for spelling in ["$THETA WT_V=(0 FIX)", "$THETA NAMES(WT_V) (0 FIX)"] {
            let content = TEMPLATE.replace("$THETA (0 FIX)   ; WT_V cov", spelling);
            let model_path = write_template_content(dir.path(), &content);
            let err = build_plan(&model_path, &names(&["WT_V"]), None, opts_cov_on(), "test")
                .unwrap_err();
            let msg = format!("{err:#}");
            assert!(msg.contains("no theta named WT_V"), "{spelling}: {msg}");
        }
    }

    #[test]
    fn cov_step_warnings() {
        let dir = tempfile::tempdir().unwrap();

        // no $COVARIANCE in initial model + cov_step on -> warn about appending
        let no_cov = TEMPLATE.replace("$COVARIANCE\n", "");
        let model_path = write_template_content(dir.path(), &no_cov);
        let built =
            build_plan(&model_path, &names(&["WT_CL"]), None, opts_cov_on(), "test").unwrap();
        assert!(built.warnings.iter().any(|w| w.contains("appended")));

        // $COVARIANCE present + cov_step off -> warn about removal
        let model_path = write_template(dir.path());
        let opts = ScmOptions {
            cov_step: false,
            ..Default::default()
        };
        let built = build_plan(&model_path, &names(&["WT_CL"]), None, opts, "test").unwrap();
        assert!(built.warnings.iter().any(|w| w.contains("removed")));
    }

    #[test]
    fn plan_render_text_mentions_the_essentials() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template(dir.path());
        let built = build_plan(
            &model_path,
            &names(&["WT_CL", "CRCL_CL", "WT_V"]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap();
        let text = built.plan.render_text();
        assert!(text.contains("<scm plan>"));
        assert!(text.contains("forward    : alpha 0.05"));
        assert!(text.contains("backward   : alpha 0.001"));
        assert!(text.contains("WT_CL"));
        assert!(text.contains("THETA(4)"));
        assert!(text.contains("initial"), "got:\n{text}");
        assert!(text.contains("off"), "got:\n{text}");
        assert!(text.contains("retry up to 3x"));
        // 3 candidates, both phases: 1 + 2 * 3(3+1)/2 = 13
        assert!(text.contains("max models : 13"), "got:\n{text}");
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
            &names(&["WT_CL", "CRCL_CL"]),
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

    #[test]
    fn an_initial_equal_to_its_fixed_value_is_rejected() {
        use crate::scm::CovariateRequest;
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template(dir.path());

        // per row
        let covariates = Covariates {
            effects: vec![CovariateRequest {
                name: "WT_CL".into(),
                initial: Some(1.0),
                fixed: Some(1.0),
                ..Default::default()
            }],
            ..Default::default()
        };
        let err = build_plan(
            &model_path,
            &covariates,
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("WT_CL: initial (1) equals fixed (1)"),
            "got: {err}"
        );

        // as section defaults
        let covariates = Covariates {
            initial: Some(0.0),
            fixed: Some(0.0),
            effects: vec![CovariateRequest::named("WT_CL")],
            ..Default::default()
        };
        let err = build_plan(
            &model_path,
            &covariates,
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("initial (0) equals fixed (0)"),
            "got: {err}"
        );
    }

    #[test]
    fn max_models_depends_on_direction() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template(dir.path());
        let mk = |direction: Vec<crate::scm::Direction>| {
            let opts = ScmOptions {
                direction,
                ..Default::default()
            };
            build_plan(
                &model_path,
                &names(&["WT_CL", "CRCL_CL", "WT_V"]),
                None,
                opts,
                "test",
            )
            .unwrap()
            .plan
        };
        use crate::scm::Direction::{Backward, Forward};
        // one phase: reference + 3+2+1; both phases: reference + 2 * (3+2+1)
        assert_eq!(mk(vec![Forward]).max_models, 7);
        assert_eq!(mk(vec![Backward]).max_models, 7);
        let both = mk(vec![Forward, Backward]);
        assert_eq!(both.max_models, 13);

        // stored in plan.json, and backfilled when an old plan lacks it
        let path = both.save().unwrap();
        let loaded = ScmPlan::load(&path).unwrap();
        assert_eq!(loaded.max_models, 13);
        let stripped = fs::read_to_string(&path)
            .unwrap()
            .replace("\"max_models\": 13,", "");
        assert!(stripped.len() < fs::read_to_string(&path).unwrap().len());
        let old = ScmPlan::from_json(&stripped).unwrap();
        assert_eq!(old.max_models, 13);
    }
}
