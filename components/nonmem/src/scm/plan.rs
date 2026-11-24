use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use fs_err as fs;
use nonmem_parser::{CommentType, Model, ParsedThetaComment};
use utils::get_utc_now;

use super::shown;
use super::{
    Candidate, CovariateType, Covariates, PlanContext, ScmOptions, ScmPlan, path_for_plan,
    project_config, sanitize_name,
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
        self.plan.render_text(&self.context)
    }

    /// Discard the SCM process in the plan's out_dir (the config and anything
    /// else not SCM-owned stay) and read the out_dir again. Only a plan that
    /// built — so validated — gets this far.
    pub fn clear_previous_output(&mut self) -> Result<()> {
        super::clear_previous_output(&self.plan.out_dir_path())?;
        self.context = PlanContext::read(&self.plan);
        Ok(())
    }

    /// Whether the SCM process already in the out_dir can resume under this
    /// plan: what a submit checks before it drives, and what `scm plan`
    /// reports.
    pub fn is_resumable(&self) -> bool {
        self.context.unreadable.is_none()
            && !self
                .context
                .compatibility
                .as_ref()
                .is_some_and(|c| c.is_incompatible())
    }

    /// Write the out_dir's .gitignore for the project's current
    /// `track_in_git` setting, creating the out_dir when the process has
    /// not been started there yet (an out_dir `scm init` did not make).
    pub fn write_gitignore(&self) -> Result<()> {
        let layout = ModelLayout::for_model_path(self.plan.model_path())?;
        let scm = project_config(layout.model_dir())?.scm;
        let out_dir = self.plan.out_dir_path();
        fs::create_dir_all(&out_dir)?;
        super::write_gitignore(&out_dir, scm.track_in_git())
    }
}

/// Every name the initial model's `$THETA` records give a theta, keyed by the
/// name uppercased: `(1-based THETA number, the name as the model spells
/// it)`. The comments must have been parsed under the project's dialect
/// ([`Model::parse_comments`]); the names are the ones `pharos nonmem
/// summary` prints
fn theta_name_index(model: &Model) -> BTreeMap<String, Vec<(usize, String)>> {
    let mut index: BTreeMap<String, Vec<(usize, String)>> = BTreeMap::new();
    for (idx0, t) in model.thetas.iter().enumerate() {
        if let Some(name) = t.parsed_comment.as_ref().and_then(|p| p.name()) {
            index
                .entry(name.to_ascii_uppercase())
                .or_default()
                .push((idx0 + 1, name));
        }
    }
    index
}

/// Thetas whose comment claims a position that is not the one they sit at.
fn stale_theta_comment_numbers(model: &Model) -> Vec<(usize, usize)> {
    model
        .thetas
        .iter()
        .enumerate()
        .filter_map(|(idx0, t)| {
            let ParsedThetaComment::Type2(parsed) = t.parsed_comment.as_ref()? else {
                return None;
            };
            let n: usize = parsed
                .prefix
                .as_deref()?
                .trim_start_matches(|c: char| !c.is_ascii_digit())
                .trim_end_matches(|c: char| !c.is_ascii_digit())
                .parse()
                .ok()?;
            (n != idx0 + 1).then_some((idx0 + 1, n))
        })
        .collect()
}

/// Resolve the names the config requested to `(1-based THETA number, the
/// name as the model spells it)`, matching case-insensitively so a request
/// need not reproduce the author's capitalization.
fn resolve_theta_names(
    model: &Model,
    comment_type: CommentType,
    names: &[String],
) -> Result<Vec<(usize, String)>> {
    let index = theta_name_index(model);

    let mut resolved: Vec<(usize, String)> = Vec::new();
    for requested in names {
        let requested = requested.trim();
        if requested.is_empty() {
            bail!("covariates contains an empty name");
        }

        let Some(hits) = index.get(&requested.to_ascii_uppercase()) else {
            bail!(
                "{}",
                not_found_message(requested, model, comment_type, &index)
            );
        };

        // The index lists each theta once, in $THETA order.
        if hits.len() > 1 {
            let named: Vec<String> = hits.iter().map(|(t, _)| format!("THETA({t})")).collect();
            bail!(
                "covariate name {requested} is ambiguous in the initial model: \
                 it names {}\nrename one of them, or request the name that identifies \
                 only the one you mean",
                named.join(" and ")
            );
        }
        let theta = hits[0].0;

        // Each theta is named once, so a theta already resolved can only
        // have been reached by the same name a second time.
        if resolved.iter().any(|(t, _)| *t == theta) {
            bail!("covariate name {requested} is requested more than once");
        }
        resolved.push((theta, hits[0].1.clone()));
    }
    Ok(resolved)
}

/// How `[nonmem.comments] type` spells `comment_type`
fn comment_type_name(comment_type: CommentType) -> &'static str {
    match comment_type {
        CommentType::Type1 => "type1",
        CommentType::Type2 => "type2",
    }
}

/// Why `requested` names no theta, with what it takes to see the cause: the
/// comment type the comments were read as, the names that reading gave, and
/// the comments it could not read at all, as written. A type that does not
/// match the comments reads none of them, and the names alone would not
/// say so.
fn not_found_message(
    requested: &str,
    model: &Model,
    comment_type: CommentType,
    index: &BTreeMap<String, Vec<(usize, String)>>,
) -> String {
    let mut available: Vec<&str> = Vec::new();
    for (_, name) in index.values().flatten() {
        if !available.contains(&name.as_str()) {
            available.push(name);
        }
    }
    available.sort_unstable();

    let type_name = comment_type_name(comment_type);
    let mut msg = format!(
        "no theta named {requested} in the initial model\n  comment type: {type_name}\n  {type_name} comment parameter names: {}",
        if available.is_empty() {
            "none".to_string()
        } else {
            available.join(", ")
        }
    );

    // The comments this type could not read, as written
    let unread: Vec<(String, &str)> = model
        .thetas
        .iter()
        .enumerate()
        .filter_map(|(idx0, t)| {
            let comment = t.comment.as_deref()?.trim();
            (t.parsed_comment.is_none() && !comment.is_empty())
                .then(|| (format!("THETA({})", idx0 + 1), comment))
        })
        .collect();
    if !unread.is_empty() {
        let width = unread.iter().map(|(t, _)| t.len()).max().unwrap_or(0);
        msg.push_str(&format!("\n\n  comments not read as {type_name}:"));
        for (theta, comment) in &unread {
            msg.push_str(&format!("\n    {theta:<width$}  {comment}"));
        }
    }

    // No comment at all: say what one looks like
    if available.is_empty() && unread.is_empty() {
        let example = match comment_type {
            CommentType::Type1 => "$THETA (0 FIX)   ; WT_CL cov",
            CommentType::Type2 => "$THETA (0 FIX)   ; WT_CL",
        };
        msg.push_str(&format!(
            "\n  no $THETA record carries a comment; give each candidate theta its own $THETA \
             record with a comment naming it, e.g. `{example}`"
        ));
    }

    msg
}

/// An effect tested from `initial` while held out at `fixed`: the estimate
/// must be a number. It may equal the held-out value: a released theta is
/// estimated from wherever it starts.
fn check_initial(who: &str, initial: f64, _fixed: f64) -> Result<()> {
    if !initial.is_finite() {
        bail!("{who} initial must be a finite number, got {initial}");
    }
    Ok(())
}

/// The names a candidate's file name `name` could be a retry or refit of:
/// what is left once a `_try<n>` and then a `_refit<n>` that
/// `scm_model_name` would add are taken off its end. Only the numbers it
/// gives are counted (2 and up), so `wt_cl_try1` is no retry of `wt_cl`.
fn retry_bases(name: &str) -> Vec<&str> {
    fn without(name: &str, tag: &str) -> Option<String> {
        let (base, n) = name.rsplit_once(tag)?;
        let given = !n.is_empty()
            && !n.starts_with('0')
            && n.bytes().all(|b| b.is_ascii_digit())
            && n.parse::<u64>().is_ok_and(|n| n >= 2);
        (given && !base.is_empty()).then(|| base.to_string())
    }
    let mut bases = Vec::new();
    let mut rest = name;
    if let Some(base) = without(rest, "_try") {
        bases.push(&name[..base.len()]);
        rest = &name[..base.len()];
    }
    if let Some(base) = without(rest, "_refit") {
        bases.push(&name[..base.len()]);
    }
    bases
}

/// Build and validate an SCM plan.
pub fn build_plan(
    model_path: &Path,
    covariates: &Covariates,
    out_dir: Option<&Path>,
    options: ScmOptions,
    pharos_version: &str,
) -> Result<BuiltPlan> {
    let options = options.normalized();
    options.validate()?;

    if covariates.effects.is_empty() {
        bail!(
            "[covariates] effects must name at least one theta, e.g. effects = [\"WT_CL\", \"CRCL_CL\"]"
        );
    }
    let fixed = covariates.default_fixed();
    if !fixed.is_finite() {
        bail!("[covariates] fixed must be a finite number, got {fixed}");
    }
    // Each per-type table on its own: a type nothing is tested under never
    // has to make sense.
    for kind in [CovariateType::Continuous, CovariateType::Categorical] {
        if let Some(initial) = covariates.table(kind).initial {
            check_initial(&format!("[covariates] {kind}"), initial, fixed)?;
        }
    }

    // The initial model
    if !model_path.exists() {
        bail!("Model file does not exist: {}", shown(&model_path));
    }
    let layout = ModelLayout::for_model_path(model_path)?;
    let mut model = Model::parse(model_path, &fs::read_to_string(model_path)?)
        .with_context(|| format!("failed to parse initial model {}", shown(&model_path)))?;

    if model.estimations.is_empty() {
        bail!(
            "initial model {} has no $ESTIMATION record",
            shown(&model_path)
        );
    }
    check_dataset(&model, layout.model_dir()).with_context(|| {
        format!(
            "dataset {} referenced by $DATA of {} does not exist",
            model.data.path,
            shown(&model_path)
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
    if project.scm.out_dir().contains("timestamp") {
        bail!(
            "this project's `[nonmem.scm] out_dir` template ({}) carries a timestamp, so the SCM \
             process directory cannot be found again once it is written",
            project.scm.out_dir()
        );
    }
    let Some(comment_type) = project.comments.r#type else {
        bail!(
            "this project sets no comment dialect, so no $THETA comment names a theta; \
             set `type` under [nonmem.comments] in pharos.toml"
        );
    };

    // Resolve the request to `(theta number, canonical name)
    model.parse_comments(comment_type);
    let names: Vec<String> = covariates.effects.iter().map(|e| e.name.clone()).collect();
    let mut selected = resolve_theta_names(&model, comment_type, &names)?;
    selected.sort_unstable_by_key(|(theta, _)| *theta);

    let mut warnings = Vec::new();
    let mut candidates = Vec::new();

    let stale = stale_theta_comment_numbers(&model);

    for (theta_num, name) in &selected {
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
            None => covariates.default_initial(request.kind),
        };
        if !fixed.is_finite() {
            bail!("{name}: fixed must be a finite number, got {fixed}");
        }
        check_initial(&format!("{name}:"), initial, fixed)?;

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
            kind: request.kind,
            initial,
            fixed,
            lower: request.lower.or(theta.lower),
            upper: request.upper.or(theta.upper),
        };

        candidate.released_spec().validate(name)?;
        candidates.push(candidate);
    }

    // A candidate's models are named from it, in letters and digits alone
    // (`sanitize_name`). Two names that differ only in what a file name
    // does not keep (`CRCL/CL` and `CRCL_CL`) would be written to the same
    // model: one fit, read as both candidates' result.
    let mut file_names: BTreeMap<String, &str> = BTreeMap::new();
    for candidate in &candidates {
        let file_name = sanitize_name(&candidate.name);
        if let Some(other) = file_names.insert(file_name.clone(), &candidate.name) {
            bail!(
                "covariates {other} and {} would both be fitted as `{file_name}`: their names \
                 differ only in characters a model file name does not keep; rename one of \
                 them in the initial model's $THETA comment",
                candidate.name
            );
        }
    }
    // A candidate's retries and refits add `_try<n>` and `_refit<n>` to its
    // name (`scm_model_name`), in the same round directory as every other
    // candidate's models. A candidate whose own name already ends that way
    // (`WT_CL_TRY2` beside `WT_CL`) would be fitted under the name of the
    // other's retry: the round would read one fit as both.
    for candidate in &candidates {
        let file_name = sanitize_name(&candidate.name);
        for base in retry_bases(&file_name) {
            if let Some(other) = file_names.get(base) {
                bail!(
                    "covariates {other} and {} would collide: {} is fitted as `{file_name}`, \
                     the name a retry or refit of {other} is given; rename one of them in \
                     the initial model's $THETA comment",
                    candidate.name,
                    candidate.name
                );
            }
        }
    }

    // What becomes of `$COVARIANCE`: (cov_step, final_cov_step, the initial model has one)
    let covariance = match (options.cov_step, options.final_cov_step, model.covariance.is_some()) {
        (true, _, false) => Some(
            "initial model has no $COVARIANCE record; cov_step is on, so one will be appended to generated models".to_string(),
        ),
        (false, final_cov_step, true) => Some(format!(
            "cov_step is off: the initial model's $COVARIANCE record will be removed from {}",
            if final_cov_step { "the round models" } else { "generated models" }
        )),
        (false, true, false) => Some(
            "initial model has no $COVARIANCE record; final_cov_step is on, so one will be appended to the final model".to_string(),
        ),
        _ => None,
    };
    warnings.extend(covariance);

    let out_dir = match out_dir {
        Some(d) => d.to_path_buf(),
        None => super::default_out_dir(&layout, &project.scm)?,
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
    use crate::scm::state::ScmState;
    use crate::scm::test_support::{
        TEMPLATE, covs, mid_scm_state, opts_cov_on, plan_named, plan_of, req, try_plan,
        write_project_config, write_template, write_template_content,
    };
    use nonmem_parser::CommentType;

    /// Names match case-insensitively and keep the authored spelling.
    #[test]
    fn candidates_are_listed_in_theta_order_however_they_were_requested() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template(dir.path());
        let built = plan_named(
            &model_path,
            &["WT_V", "wt_cl", "CRCL_CL"],
            None,
            opts_cov_on(),
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

        // the plan survives the round trip through the state that records it
        let out_dir = built.plan.out_dir_path();
        ScmState::new(&built.plan).save(&out_dir).unwrap();
        let loaded = ScmState::load(&out_dir).unwrap().unwrap().plan;
        assert_eq!(loaded, built.plan);
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
            let built = plan_named(&model_path, &[shown], None, opts_cov_on())
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
            let built = plan_named(&model_path, &["WT_CL"], None, ScmOptions::default())
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

    /// A plan the SCM process in its out_dir cannot resume under is refused,
    /// and the process keeps the plan its state records; once the out_dir is
    /// cleared (what an overwrite does), it resumes.
    #[test]
    fn an_incompatible_plan_is_refused_until_the_out_dir_is_cleared() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template(dir.path());
        let previous = plan_of(&model, &["WT_CL", "CRCL_CL"], None, ScmOptions::default());
        let out_dir = previous.out_dir_path();
        mid_scm_state(&previous).save(&out_dir).unwrap();
        let recorded_alpha = || {
            ScmState::load(&out_dir)
                .unwrap()
                .unwrap()
                .plan
                .options
                .forward_alpha
        };

        // A run-control change resumes, and so does a settings change.
        let paced = ScmOptions {
            num_rounds: Some(1),
            forward_alpha: 0.01,
            ..Default::default()
        };
        assert!(
            plan_named(&model, &["WT_CL", "CRCL_CL"], None, paced)
                .unwrap()
                .is_resumable()
        );

        // A candidate the process never knew does not.
        let mut changed = plan_named(
            &model,
            &["WT_CL", "CRCL_CL", "WT_V"],
            None,
            ScmOptions::default(),
        )
        .unwrap();
        assert!(!changed.is_resumable());
        assert!(changed.render_text().contains("cannot resume"));
        assert_eq!(recorded_alpha(), 0.05);
        assert!(ScmState::load(&out_dir).unwrap().is_some());

        changed.clear_previous_output().unwrap();
        assert!(ScmState::load(&out_dir).unwrap().is_none());
        assert!(!changed.render_text().contains("cannot resume"));
        assert!(changed.is_resumable());
    }

    /// No `$COVARIANCE` in the initial model with `cov_step` on warns that one
    /// is appended (the removal warning is in the plan text snapshots).
    #[test]
    fn cov_step_on_without_a_covariance_record_warns() {
        let dir = tempfile::tempdir().unwrap();
        let no_cov = TEMPLATE.replace("$COVARIANCE\n", "");
        let model_path = write_template_content(dir.path(), &no_cov);
        let built = plan_named(&model_path, &["WT_CL"], None, opts_cov_on()).unwrap();
        assert!(built.warnings.iter().any(|w| w.contains("appended")));
    }

    /// Bounds per effect: the row's own value, else the bound the initial
    /// model's own `$THETA` spec carries.
    #[test]
    fn bounds_resolve_row_then_template() {
        let dir = tempfile::tempdir().unwrap();
        // WT_CL is authored bounded in the initial model; the others are `(0 FIX)`.
        let content = TEMPLATE.replace(
            "$THETA (0 FIX)   ; WT_CL cov",
            "$THETA (-2, 0.4, 2)   ; WT_CL cov",
        );
        let model_path = write_template_content(dir.path(), &content);
        let covariates = covs(vec![
            // says nothing, and its theta is unbounded
            req("CRCL_CL"),
            // its own bounds, against an unbounded theta
            req("WT_V").bounds(Some(0.01), Some(10.0)),
            // the row's lower wins over the initial model's -2, and the
            // initial model still supplies the upper the row leaves out
            req("WT_CL").bounds(Some(0.0), None),
        ]);
        let built = try_plan(&model_path, &covariates, None, opts_cov_on()).unwrap();
        let c = &built.plan.candidates;
        assert_eq!((c[0].lower, c[0].upper), (Some(0.0), Some(2.0))); // WT_CL
        assert_eq!((c[1].lower, c[1].upper), (None, None)); // CRCL_CL
        assert_eq!((c[2].lower, c[2].upper), (Some(0.01), Some(10.0))); // WT_V
        assert_eq!(c[0].bounds_label().as_deref(), Some("(0, 2)"));

        // A config that says nothing about bounds keeps the initial model's, and
        // an unbounded theta stays unbounded.
        let built = plan_named(&model_path, &["WT_CL", "CRCL_CL"], None, opts_cov_on()).unwrap();
        let c = &built.plan.candidates;
        assert_eq!((c[0].lower, c[0].upper), (Some(-2.0), Some(2.0)));
        assert_eq!((c[1].lower, c[1].upper), (None, None));
        assert_eq!(c[1].bounds_label(), None);
    }

    /// `initial` and `fixed` per effect: a row's own value, else the initial
    /// model's estimate when it is not the fixed value, else the section default.
    #[test]
    fn initial_and_fixed_resolve_row_then_template_then_default() {
        use crate::scm::TypeDefaults;
        let dir = tempfile::tempdir().unwrap();
        // WT_CL carries a guess of its own (0.4); CRCL_CL is `(0 FIX)`; WT_V is
        // written as a fold-change effect fixed at 1.
        let content = TEMPLATE
            .replace("$THETA (0 FIX)   ; WT_CL cov", "$THETA 0.4   ; WT_CL cov")
            .replace("$THETA (0 FIX)   ; WT_V cov", "$THETA (1 FIX)   ; WT_V cov");
        let model_path = write_template_content(dir.path(), &content);
        let covariates = Covariates {
            continuous: TypeDefaults { initial: Some(0.2) },
            fixed: Some(0.0),
            effects: vec![
                req("WT_CL"),
                req("CRCL_CL").initial(0.9),
                req("WT_V").fixed(1.0),
            ],
            ..Default::default()
        };
        let built = try_plan(&model_path, &covariates, None, opts_cov_on()).unwrap();
        let c = &built.plan.candidates;
        // the initial model's guess wins over the section default
        assert_eq!((c[0].initial, c[0].fixed), (0.4, 0.0));
        // the row's own initial wins over everything
        assert_eq!((c[1].initial, c[1].fixed), (0.9, 0.0));
        // `(1 FIX)` is the held-out spelling for fixed = 1, so the default applies
        assert_eq!((c[2].initial, c[2].fixed), (0.2, 1.0));
        assert!(built.warnings.is_empty(), "warnings: {:?}", built.warnings);
    }

    /// Two candidates whose names differ only in characters a file name
    /// does not keep would share one model file, and so one fit: the plan
    /// refuses them, naming both, before anything is written.
    #[test]
    fn candidates_that_would_share_a_model_file_are_refused() {
        for (theta_comment, other) in [("CRCL/CL", "CRCL_CL"), ("WT-V", "WT_V")] {
            let dir = tempfile::tempdir().unwrap();
            // WT_CL's theta takes the colliding name; `other` keeps its own
            let content = TEMPLATE.replace("; WT_CL cov", &format!("; {theta_comment} cov"));
            assert_ne!(content, TEMPLATE);
            let model = write_template_content(dir.path(), &content);
            let err = plan_named(&model, &[theta_comment, other], None, ScmOptions::default())
                .unwrap_err();
            let text = format!("{err:#}");
            assert!(
                text.contains(theta_comment) && text.contains(other) && text.contains("rename"),
                "got: {text}"
            );
            // Either one alone is a plan
            plan_named(&model, &[other], None, ScmOptions::default()).unwrap();
        }
    }

    /// A candidate named like another's retry or refit would share that
    /// model file in the round's directory: the plan refuses the pair,
    /// naming both. A name no retry or refit is ever given is a plan.
    #[test]
    fn a_candidate_named_like_another_s_retry_is_refused() {
        use crate::scm::round::scm_model_name;

        // the collision itself: WT_CL's second attempt is WT_CL_TRY2's first
        assert_eq!(
            scm_model_name("1001", "WT_CL", 2, 0),
            scm_model_name("1001", "WT_CL_TRY2", 1, 0)
        );
        assert_eq!(
            scm_model_name("1001", "WT_CL", 1, 1),
            scm_model_name("1001", "WT_CL_REFIT2", 1, 0)
        );

        let plan_with = |name: &str| {
            let dir = tempfile::tempdir().unwrap();
            let content = TEMPLATE.replace("; CRCL_CL cov", &format!("; {name} cov"));
            assert_ne!(content, TEMPLATE);
            let model = write_template_content(dir.path(), &content);
            plan_named(&model, &["WT_CL", name], None, ScmOptions::default())
                .map(|_| ())
                .map_err(|e| format!("{e:#}"))
        };
        for name in [
            "WT_CL_TRY2",
            "WT_CL_TRY12",
            "WT_CL_REFIT2",
            "WT_CL_REFIT3_TRY2",
        ] {
            let err = plan_with(name).unwrap_err();
            assert!(
                err.contains("WT_CL") && err.contains(name) && err.contains("retry or refit"),
                "{name}: {err}"
            );
        }
        // never given to a retry or refit, so no collision
        for name in [
            "WT_CL_TRY1",
            "WT_CL_TRY02",
            "WT_CL_TRY",
            "WT_CL_TRYOUT",
            "AGE_TRY2",
        ] {
            assert_eq!(plan_with(name), Ok(()), "{name}");
        }
        assert_eq!(retry_bases("wt_cl_refit3_try2"), ["wt_cl_refit3", "wt_cl"]);
        assert!(retry_bases("_try2").is_empty());
    }

    /// A name missing under the project's comment type says what the
    /// comments were read as and what that gave, and the comments it could
    /// not read, as written: a type that does not match the comments reads
    /// none of them.
    #[test]
    fn a_missing_name_shows_how_the_comments_were_read() {
        let thetas = "$THETA
 0.343013    ;KA [1/hr]  ;lognormal
 2.08667    ;V  [L]     ;lognormal
 -2.01197    ;CL [L/hr]  ;lognormal
 -0.168077 ;ALAG1 [hr]  ;lognormal
 0.898 ;V~WT [-]
 0.6055 ;CL~WT [-]
 -0.1 ;CL~AGE [-]";
        let at = TEMPLATE.find("$THETA").unwrap();
        let end = TEMPLATE.find("$OMEGA").unwrap();
        let content = format!("{}{thetas}\n{}", &TEMPLATE[..at], &TEMPLATE[end..]);
        let error = |comment_type, name: &str| {
            let dir = tempfile::tempdir().unwrap();
            let model = write_template_content(dir.path(), &content);
            write_project_config(dir.path(), comment_type);
            let err = plan_named(&model, &[name], None, ScmOptions::default()).unwrap_err();
            format!("{err:#}")
        };

        // type2 comments read as type1: none read, each shown as written
        let msg = error(CommentType::Type1, "V~WT");
        assert!(
            msg.starts_with("no theta named V~WT in the initial model"),
            "{msg}"
        );
        assert!(msg.contains("\n  comment type: type1\n"), "{msg}");
        assert!(
            msg.contains(
                "\n  type1 comment parameter names: none\n\n  comments not read as type1:\n"
            ),
            "{msg}"
        );
        assert!(!msg.contains("when the comments are read as"), "{msg}");
        assert!(
            msg.contains("\n    THETA(1)  KA [1/hr]  ;lognormal"),
            "{msg}"
        );
        assert!(msg.contains("\n    THETA(7)  CL~AGE [-]"), "{msg}");

        // the right type and a wrong name: every name, nothing unread
        let msg = error(CommentType::Type2, "WT_V");
        assert!(msg.contains("\n  comment type: type2\n"), "{msg}");
        assert!(
            msg.contains("type2 comment parameter names: ALAG1, CL, CL~AGE, CL~WT, KA, V, V~WT"),
            "{msg}"
        );
        assert!(!msg.contains("comments not read"), "{msg}");
    }
}
