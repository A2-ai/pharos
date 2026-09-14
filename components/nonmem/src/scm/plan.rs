use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use fs_err as fs;
use nonmem_parser::{CommentType, Model, ParsedThetaComment, Type1Theta, parse_theta_param};
use utils::get_utc_now;

use super::{
    Candidate, Covariates, PLAN_SCHEMA_VERSION, PlanContext, ScmOptions, ScmPlan, max_models_for,
    parent_or_dot,
};
use crate::validate_model_extension;

/// A built plan plus non-fatal findings worth surfacing to the user, and
/// what the plan met in its out_dir: the SCM process already run there, and the
/// plan.json this one replaces.
#[derive(Debug, Clone)]
pub struct BuiltPlan {
    pub plan: ScmPlan,
    pub warnings: Vec<String>,
    /// Read while building, i.e. before [`ScmPlan::save`] overwrites the
    /// plan.json it compares against.
    pub context: PlanContext,
}

impl BuiltPlan {
    /// The plan rendered with its out_dir's history — what every caller
    /// showing a freshly built plan should print.
    pub fn render_text(&self) -> String {
        self.plan.render_text_with(&self.context)
    }
}

/// A comment in the numbered style (`6 WT_CL WT on clearance`) labels the
/// theta with its position before naming it; the label is not a name, so
/// drop it. Returns the leading integer alongside the rest of the comment.
fn split_leading_index(comment: &str) -> (Option<usize>, &str) {
    let trimmed = comment.trim_start();
    let Some(token) = trimmed.split_whitespace().next() else {
        return (None, trimmed);
    };
    match token.parse::<usize>() {
        Ok(n) => (Some(n), trimmed[token.len()..].trim_start()),
        Err(_) => (None, trimmed),
    }
}

fn strip_leading_index(comment: &str) -> &str {
    split_leading_index(comment).1
}

/// Whether a name is safe to use as a candidate name: it becomes part of a
/// generated model's file name, and it has to survive a round trip through
/// the config and the plan.
fn is_usable_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// The first whitespace-separated word of a comment, when it is safe to use
/// as a name.
fn first_name_token(comment: &str) -> Option<String> {
    let token = comment.split_whitespace().next()?;
    is_usable_name(token).then(|| token.to_string())
}

/// Which part of a `$THETA` record gave a theta a name.
///
/// The order of the variants is the precedence used to pick a candidate's
/// canonical name (see [`canonical_name`]): the `$THETA` label is the
/// author's own NM-TRAN naming syntax and cannot drift from the parameter
/// it is attached to, a parsed comment is a recognized annotation form,
/// and a bare comment token is whatever the comment happens to start with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum NameSource {
    /// `$THETA CL=(0, 1.5, 10)` or `$THETA NAMES(KA, V2, Q) ...`.
    Label,
    /// A comment a [`CommentType`] dialect parses, e.g. `; WT_CL cov`.
    ParsedComment,
    /// The first word of a comment neither dialect parses, e.g.
    /// `; 6 WT_CL WT on clearance`.
    CommentToken,
}

impl NameSource {
    /// How a message describes where a name came from.
    fn label(self) -> &'static str {
        match self {
            NameSource::Label => "$THETA label",
            NameSource::ParsedComment | NameSource::CommentToken => "comment",
        }
    }
}

/// One theta named one way.
#[derive(Debug, Clone)]
struct NameHit {
    /// 1-based THETA number.
    theta: usize,
    source: NameSource,
    /// The name as the model spells it, for messages and for the candidate's
    /// canonical name.
    as_written: String,
}

/// Every name the initial model's `$THETA` records give a theta, keyed by
/// the name uppercased.
///
/// The SCM process reads nothing but `$THETA`: an effect is a covariate
/// effect because the config says so, and the only thing the model has to
/// supply is which theta each requested name means. Three spellings name a
/// theta, and a theta can carry more than one of them:
///
/// - its `$THETA` naming syntax, `$THETA WT_CL=(0, 0.4)` or
///   `$THETA NAMES(WT_CL, CRCL_CL) ...`;
/// - a comment either comment dialect parses — `; WT_CL cov` and
///   `; WT_CL (L/h) :LOG` under Type1, `; WT_CL` and `; 6 WT_CL` under
///   Type2. Both dialects are tried, so no configured comment type is
///   needed;
/// - the first word of any other comment, which is what makes the common
///   numbered style `; 6 WT_CL WT on clearance` name `WT_CL`.
///
/// A name that lands on more than one theta is kept as several hits rather
/// than resolved here: [`resolve_theta_names`] is what reports it.
fn theta_name_index(model: &Model) -> BTreeMap<String, Vec<NameHit>> {
    let mut index: BTreeMap<String, Vec<NameHit>> = BTreeMap::new();
    let mut add = |name: &str, theta: usize, source: NameSource| {
        let entry = index.entry(name.to_ascii_uppercase()).or_default();
        // One theta named the same way twice adds nothing; keep the
        // strongest source so the canonical name is picked from it.
        if !entry.iter().any(|h| h.theta == theta && h.source == source) {
            entry.push(NameHit {
                theta,
                source,
                as_written: name.to_string(),
            });
        }
    };

    for (idx0, theta) in model.thetas.iter().enumerate() {
        let theta_num = idx0 + 1;

        if let Some(label) = theta.name.as_deref().filter(|l| !l.trim().is_empty()) {
            add(label.trim(), theta_num, NameSource::Label);
        }

        let Some(comment) = theta.comment.as_deref() else {
            continue;
        };
        let comment = strip_leading_index(comment);

        let parsed = [CommentType::Type1, CommentType::Type2]
            .into_iter()
            .filter_map(|ct| parse_theta_param(comment, ct))
            .filter_map(|p| match p {
                // A Type1 `Type` comment (`; RES ERR :stdev`) names a kind
                // of parameter, not one parameter, and can be several
                // words; it is no use as a name.
                ParsedThetaComment::Type1(Type1Theta::Type { .. }) => None,
                other => other.name(),
            });
        let mut named = false;
        for name in parsed {
            if is_usable_name(&name) {
                add(&name, theta_num, NameSource::ParsedComment);
                named = true;
            }
        }
        // The fallback only applies to comments no dialect could name, so a
        // recognized annotation never also registers its first word.
        if !named && let Some(token) = first_name_token(comment) {
            add(&token, theta_num, NameSource::CommentToken);
        }
    }

    index
}

/// The name the plan records for a theta, and the roster's identity key.
///
/// Taken from the model rather than from what the config typed, so the
/// several names one theta may answer to all collapse to one candidate:
/// re-wording a config from `WT` to `WT_CL` for the same theta is not a
/// different candidate. Precedence is [`NameSource`]'s own order.
fn canonical_name(index: &BTreeMap<String, Vec<NameHit>>, theta: usize) -> Option<String> {
    index
        .values()
        .flatten()
        .filter(|h| h.theta == theta)
        .min_by(|a, b| {
            a.source
                .cmp(&b.source)
                .then_with(|| a.as_written.cmp(&b.as_written))
        })
        .map(|h| h.as_written.clone())
}

/// One requested name, resolved.
struct Resolved {
    /// 1-based THETA number the name landed on.
    theta: usize,
    /// The name the plan records and the roster keys on: the model's own,
    /// by [`NameSource`] precedence, not the spelling the config used.
    name: String,
    /// The spelling the config asked for, so the request's own row can be
    /// found again once the resolution is in theta order.
    requested: String,
}

/// Resolve the names the config requested, matching case-insensitively so a
/// request need not reproduce the author's capitalization.
///
/// A name resolves when the `$THETA` records name exactly one theta by it.
/// Nothing in a model marks a theta as a covariate effect — the scientist's
/// naming is the specification and the SCM process is what fixes the theta
/// when the effect is held out — so a name landing on two thetas is an
/// error rather than a precedence pick: there is no evidence available to
/// break the tie, and guessing would silently test the wrong theta.
fn resolve_theta_names(model: &Model, names: &[String]) -> Result<Vec<Resolved>> {
    let index = theta_name_index(model);

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
            let mut lines =
                format!("covariate name {requested} is ambiguous in the initial model:");
            for theta in &thetas {
                let by: Vec<String> = hits
                    .iter()
                    .filter(|h| h.theta == *theta)
                    .map(|h| format!("{} {}", h.source.label(), h.as_written))
                    .collect();
                lines.push_str(&format!("\n  THETA({theta}) — {}", by.join(", ")));
            }
            lines.push_str(
                "\nrename one of them, or request the name that identifies only the one you mean",
            );
            bail!("{lines}");
        }
        let theta = thetas[0];

        // Two requests can land on one theta two ways: the same name twice,
        // or two of the names that theta answers to. They are different
        // mistakes, so they get different messages.
        if let Some(other) = resolved.iter().find(|r| r.theta == theta) {
            if other.requested.eq_ignore_ascii_case(requested) {
                bail!("covariate name {requested} is requested more than once");
            }
            bail!(
                "covariate names {} and {requested} both resolve to THETA({theta})",
                other.requested
            );
        }
        resolved.push(Resolved {
            theta,
            name: canonical_name(&index, theta).unwrap_or_else(|| hits[0].as_written.clone()),
            requested: requested.to_string(),
        });
    }
    Ok(resolved)
}

/// What to say when a requested name names no theta: every name that would
/// have worked, grouped by where it came from, so the message doubles as
/// the list of spellings available in this model.
fn not_found_message(requested: &str, index: &BTreeMap<String, Vec<NameHit>>) -> String {
    let mut by_label: Vec<&str> = Vec::new();
    let mut by_comment: Vec<&str> = Vec::new();
    for hit in index.values().flatten() {
        let bucket = match hit.source {
            NameSource::Label => &mut by_label,
            NameSource::ParsedComment | NameSource::CommentToken => &mut by_comment,
        };
        if !bucket.contains(&hit.as_written.as_str()) {
            bucket.push(&hit.as_written);
        }
    }
    by_label.sort_unstable();
    by_comment.sort_unstable();

    let mut msg = format!("no theta named {requested} in the initial model");
    for (label, names) in [
        ("named by $THETA", by_label),
        ("named by a comment", by_comment),
    ] {
        if !names.is_empty() {
            msg.push_str(&format!("\n  {label}: {}", names.join(", ")));
        }
    }
    if index.is_empty() {
        msg.push_str(
            "\n  no $THETA record in this model carries a name or a comment, so no covariate \
             effect can be requested",
        );
    }
    msg
}

/// Validate a bound pair, and the initial estimate against it when there is one
/// (NM-TRAN rejects an initial estimate that is not strictly inside its
/// bounds). `who` names what is being checked in the message: the section or
/// one candidate.
fn check_bounds(
    who: &str,
    lower: Option<f64>,
    upper: Option<f64>,
    initial: Option<f64>,
) -> Result<()> {
    for (label, value) in [("lower", lower), ("upper", upper)] {
        if let Some(v) = value
            && v.is_nan()
        {
            bail!("{who} {label} must be a number, got {v}");
        }
    }
    if let (Some(lower), Some(upper)) = (lower, upper)
        && lower >= upper
    {
        bail!("{who} lower ({lower}) must be below upper ({upper})");
    }
    // The held-out spelling is a bare `(fixed FIX)`, so `fixed` never has to
    // sit inside the bounds — only the initial estimate does.
    if let Some(initial) = initial {
        if let Some(lower) = lower
            && initial <= lower
        {
            bail!(
                "{who}: initial ({initial}) must be above lower ({lower}); NM-TRAN rejects an \
                 initial estimate at or outside its bounds"
            );
        }
        if let Some(upper) = upper
            && initial >= upper
        {
            bail!(
                "{who}: initial ({initial}) must be below upper ({upper}); NM-TRAN rejects an \
                 initial estimate at or outside its bounds"
            );
        }
    }
    Ok(())
}

/// Build and validate an SCM plan.
///
/// `covariates` names the candidate effects by any name the initial model's
/// `$THETA` records give them (see [`resolve_theta_names`]): a `$THETA`
/// label, a parsed comment, or a comment's first word. Each effect's `initial` and
/// `fixed` come from its own row, else the section defaults; a theta in the initial model
/// that carries an initial estimate other than `fixed` supplies `initial`
/// when the row does not. Each bound comes from the row, else the section
/// default, else the `$THETA` spec's own bound in the initial model. `pharos_version`
/// is recorded in the plan for provenance (the binary's
/// `CARGO_PKG_VERSION`).
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
        ("initial", covariates.defaults.initial),
        ("fixed", covariates.defaults.fixed),
    ] {
        if !value.is_finite() {
            bail!("[covariates] {label} must be a finite number, got {value}");
        }
    }
    if covariates.defaults.initial == covariates.defaults.fixed {
        bail!(
            "[covariates] initial ({}) equals fixed ({}): an effect whose initial estimate is its held-out value is not tested at all",
            covariates.defaults.initial,
            covariates.defaults.fixed
        );
    }
    check_bounds(
        "[covariates]",
        covariates.defaults.lower,
        covariates.defaults.upper,
        None,
    )?;

    if !model_path.exists() {
        bail!("Model file does not exist: {}", model_path.display());
    }
    validate_model_extension(model_path)?;

    let content = fs::read_to_string(model_path)?;
    let model = Model::parse(model_path, &content)
        .with_context(|| format!("failed to parse initial model {}", model_path.display()))?;

    if model.estimations.is_empty() {
        bail!(
            "initial model {} has no $ESTIMATION record",
            model_path.display()
        );
    }

    // The dataset must exist: a relative $DATA path resolves against the
    // model's own directory.
    let data_path = Path::new(&model.data.path);
    let resolved_data = if data_path.is_relative() {
        parent_or_dot(model_path).join(data_path)
    } else {
        data_path.to_path_buf()
    };
    if !resolved_data.exists() {
        bail!(
            "dataset {} referenced by $DATA does not exist (resolved to {})",
            model.data.path,
            resolved_data.display()
        );
    }

    // Resolve the request to `(theta number, canonical name)`, in theta
    // order — the order the plan lists its candidates in.
    let names: Vec<String> = covariates.effects.iter().map(|e| e.name.clone()).collect();
    let mut selected = resolve_theta_names(&model, &names)?;
    selected.sort_unstable_by_key(|r| r.theta);

    let mut warnings = Vec::new();
    let mut candidates = Vec::new();

    for Resolved {
        theta: theta_num,
        name,
        requested,
    } in &selected
    {
        let theta_num = *theta_num;
        let idx0 = theta_num - 1;
        // The row that asked for this effect. Matched on the spelling the
        // config used, which is not necessarily the candidate's name.
        let request = covariates
            .effects
            .iter()
            .find(|e| e.name.trim().eq_ignore_ascii_case(requested))
            .expect("every resolved name came from a request");
        // The name index is built from `model.thetas`, so a resolved theta
        // number always indexes it.
        let theta = &model.thetas[idx0];

        // A numbered comment (`; 7 CRCL_CL ...`) that disagrees with the
        // theta's actual position usually means the comments went stale
        // after thetas were added or reordered.
        if let Some(comment) = theta.comment.as_deref()
            && let (Some(label), _) = split_leading_index(comment)
            && label != theta_num
        {
            warnings.push(format!(
                "THETA({theta_num}) [{name}] has a comment numbered {label}; \
                 the comment numbering looks stale"
            ));
        }

        // What the effect is fixed at when held out: the row's own value,
        // else the section default.
        let fixed = request.fixed.unwrap_or(covariates.defaults.fixed);
        // The effect's initial estimate the first time it is tested: the
        // row's own value; else the guess the initial model already carries
        // for it (anything other than its held-out value); else the section
        // default.
        let initial = match request.initial {
            Some(v) => v,
            None if theta.init != fixed => theta.init,
            None => covariates.defaults.initial,
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
        // An initial model that already pins the theta at some other value is
        // usually a leftover: the config decides, so say which value wins.
        if theta.fixed && theta.init != fixed {
            warnings.push(format!(
                "THETA({theta_num}) [{name}] is fixed at {} in the initial model but fixed = {fixed} \
                 in the config; generated models hold the effect out at {fixed}",
                theta.init
            ));
        }

        // Each bound: the row's own value, else the section default, else
        // whatever the initial model's own `$THETA` spec carries — so a config
        // that says nothing about bounds keeps the initial model's.
        let lower = request.lower.or(covariates.defaults.lower).or(theta.lower);
        let upper = request.upper.or(covariates.defaults.upper).or(theta.upper);
        check_bounds(name, lower, upper, Some(initial))?;

        candidates.push(Candidate {
            name: name.clone(),
            theta: theta_num,
            initial,
            fixed,
            lower,
            upper,
        });
    }

    if options.cov_step && model.covariance.is_none() {
        warnings.push(
            "initial model has no $COVARIANCE record; cov_step is on, so one will be appended to generated models"
                .to_string(),
        );
    }
    if !options.cov_step && model.covariance.is_some() {
        // The final model is the exception when final_cov_step is on: it
        // keeps the record, because it is what reports the retained
        // covariates' estimates with standard errors.
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

    let stem = super::round::file_stem_of(model_path).context("model file has no file stem")?;
    let out_dir = match out_dir {
        Some(d) => d.to_path_buf(),
        None => parent_or_dot(model_path).join("scm").join(&stem),
    };

    let plan = ScmPlan {
        schema_version: PLAN_SCHEMA_VERSION,
        created: get_utc_now(),
        pharos_version: pharos_version.to_string(),
        model: model_path.to_string_lossy().to_string(),
        out_dir: out_dir.to_string_lossy().to_string(),
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
        INLINE_TEMPLATE, TEMPLATE, names, opts_cov_on, write_template, write_template_content,
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
        // The four ways a `$THETA` record can name a theta. Each variant
        // renames the WT_V candidate on THETA(6) and requests it by that
        // name; `$PK` is untouched and never consulted.
        let cases = [
            ("$THETA label", "$THETA WT_V=(0 FIX)", "WT_V"),
            ("NAMES list", "$THETA NAMES(WT_V) (0 FIX)", "WT_V"),
            ("bare comment", "$THETA (0 FIX)   ; WT_V", "WT_V"),
            (
                "prefixed comment",
                "$THETA (0 FIX)   ; THETA6: WT_V",
                "WT_V",
            ),
            (
                "numbered comment",
                "$THETA (0 FIX)   ; 6 WT_V WT on volume",
                "WT_V",
            ),
        ];
        for (label, spelling, requested) in cases {
            let dir = tempfile::tempdir().unwrap();
            let content = TEMPLATE.replace("$THETA (0 FIX)   ; WT_V cov", spelling);
            let model_path = write_template_content(dir.path(), &content);
            let built = build_plan(
                &model_path,
                &names(&[requested]),
                None,
                opts_cov_on(),
                "test",
            )
            .unwrap_or_else(|e| panic!("{label}: {e:#}"));
            assert_eq!(built.plan.candidates.len(), 1, "{label}");
            assert_eq!(built.plan.candidates[0].name, "WT_V", "{label}");
            assert_eq!(built.plan.candidates[0].theta, 6, "{label}");
        }
    }

    #[test]
    fn an_unknown_name_lists_the_names_that_would_have_worked() {
        let dir = tempfile::tempdir().unwrap();
        // One theta named by its `$THETA` label, the rest by their comments,
        // so the message has both groups to report.
        let content = TEMPLATE.replace("$THETA (0 FIX)   ; WT_V cov", "$THETA WT_V=(0 FIX)");
        let model_path = write_template_content(dir.path(), &content);
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
        assert!(msg.contains("named by $THETA: WT_V"), "got: {msg}");
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
        // THETA(6) is labelled WT_V by `$THETA`; THETA(3) is now named WT_V
        // by its comment. Nothing in the model can break the tie.
        let clash = TEMPLATE
            .replace("$THETA (0 FIX)   ; WT_V cov", "$THETA WT_V=(0 FIX)")
            .replace("; TVKA (1/h)", "; WT_V");
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
        assert!(
            msg.contains("$THETA label") && msg.contains("comment"),
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
    fn two_names_for_one_theta_error() {
        let dir = tempfile::tempdir().unwrap();
        // THETA(6) answers to both its label and its comment; asking for
        // both is asking for one theta twice.
        let both = TEMPLATE.replace(
            "$THETA (0 FIX)   ; WT_V cov",
            "$THETA WTV=(0 FIX)   ; WT_V cov",
        );
        let model_path = write_template_content(dir.path(), &both);
        let err = build_plan(
            &model_path,
            &names(&["WTV", "WT_V"]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("both resolve to THETA(6)"), "got: {msg}");
    }

    #[test]
    fn aliases_for_one_theta_resolve_to_the_same_candidate_name() {
        let dir = tempfile::tempdir().unwrap();
        // THETA(6) is named WTV by its label and WT_V by its comment. Either
        // request must produce the same candidate, so re-wording a config
        // does not read as a different candidate to the roster.
        let both = TEMPLATE.replace(
            "$THETA (0 FIX)   ; WT_V cov",
            "$THETA WTV=(0 FIX)   ; WT_V cov",
        );
        let model_path = write_template_content(dir.path(), &both);
        let mut got = Vec::new();
        for requested in ["WTV", "WT_V"] {
            let built = build_plan(
                &model_path,
                &names(&[requested]),
                None,
                opts_cov_on(),
                "test",
            )
            .unwrap();
            got.push((
                built.plan.candidates[0].name.clone(),
                built.plan.candidates[0].theta,
            ));
        }
        assert_eq!(got[0], got[1], "aliases produced different candidates");
        // The `$THETA` label outranks the comment as the canonical name.
        assert_eq!(got[0], ("WTV".to_string(), 6));
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
        // No `cov` suffix (Type2), a unit-style comment (Type1), and the
        // numbered house style (neither dialect, so the first word): all
        // three name their candidate, and none of them warns.
        let varied = TEMPLATE
            .replace("; WT_CL cov", "; WT_CL")
            .replace("; CRCL_CL cov", "; CRCL_CL (-) :LOG")
            .replace("; WT_V cov", "; 6 WT_V weight on volume");
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
    fn stale_comment_numbering_warns() {
        let dir = tempfile::tempdir().unwrap();
        let stale = TEMPLATE.replace("; WT_CL cov", "; 9 WT_CL WT on clearance");
        let model_path = write_template_content(dir.path(), &stale);
        let built = build_plan(
            &model_path,
            &names(&["WT_CL"]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap();
        assert_eq!(built.plan.candidates[0].name, "WT_CL");
        assert!(
            built
                .warnings
                .iter()
                .any(|w| w.contains("numbered 9") && w.contains("stale")),
            "warnings: {:?}",
            built.warnings
        );
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
    fn annotation_wording_is_not_policed() {
        let dir = tempfile::tempdir().unwrap();
        // `covv` is not the `cov` annotation and does not have to be: no
        // dialect parses the comment, so its first word names the theta.
        let odd = TEMPLATE.replace("; WT_V cov", "; WT_V covv");
        let model_path = write_template_content(dir.path(), &odd);
        let built = build_plan(
            &model_path,
            &names(&["WT_CL", "CRCL_CL", "WT_V"]),
            None,
            opts_cov_on(),
            "test",
        )
        .unwrap();
        assert_eq!(built.plan.candidates[2].name, "WT_V");
        assert!(built.warnings.is_empty(), "warnings: {:?}", built.warnings);
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
        use crate::scm::{CovariateDefaults, CovariateRequest};
        let dir = tempfile::tempdir().unwrap();
        // WT_CL is authored bounded in the initial model; the others are `(0 FIX)`.
        let content = TEMPLATE.replace(
            "$THETA (0 FIX)   ; WT_CL cov",
            "$THETA (-2, 0.4, 2)   ; WT_CL cov",
        );
        let model_path = write_template_content(dir.path(), &content);
        let covariates = Covariates {
            defaults: CovariateDefaults {
                lower: Some(0.0),
                ..Default::default()
            },
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
        use crate::scm::{CovariateDefaults, CovariateRequest};
        let dir = tempfile::tempdir().unwrap();
        // WT_CL carries a guess of its own (0.4); CRCL_CL is `(0 FIX)`; WT_V is
        // written as a fold-change effect fixed at 1.
        let content = TEMPLATE
            .replace("$THETA (0 FIX)   ; WT_CL cov", "$THETA 0.4   ; WT_CL cov")
            .replace("$THETA (0 FIX)   ; WT_V cov", "$THETA (1 FIX)   ; WT_V cov");
        let model_path = write_template_content(dir.path(), &content);
        let covariates = Covariates {
            defaults: CovariateDefaults {
                initial: 0.2,
                fixed: 0.0,
                ..Default::default()
            },
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
        use crate::scm::{CovariateDefaults, CovariateRequest};
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template(dir.path());
        // The initial model says (0 FIX), the config holds the effect out at 1.
        let covariates = Covariates {
            defaults: CovariateDefaults::default(),
            effects: vec![CovariateRequest {
                name: "WT_CL".into(),
                initial: Some(1.2),
                fixed: Some(1.0),
                ..Default::default()
            }],
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
        use crate::scm::{CovariateDefaults, CovariateRequest};
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template(dir.path());

        // per row
        let covariates = Covariates {
            defaults: CovariateDefaults::default(),
            effects: vec![CovariateRequest {
                name: "WT_CL".into(),
                initial: Some(1.0),
                fixed: Some(1.0),
                ..Default::default()
            }],
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
            defaults: CovariateDefaults {
                initial: 0.0,
                fixed: 0.0,
                ..Default::default()
            },
            effects: vec![CovariateRequest::named("WT_CL")],
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
