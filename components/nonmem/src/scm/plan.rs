use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use fs_err as fs;
use nonmem_parser::{
    CodeBlock, CommentType, Model, NmtranExpr, NmtranStatement, ParsedThetaComment, Type1Theta,
    parse_theta_param,
};
use utils::get_utc_now;

use super::{
    Candidate, PLAN_SCHEMA_VERSION, PlanContext, ScmOptions, ScmPlan, max_models_for, parent_or_dot,
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

/// The name a theta's comment gives it. Comments name nothing the SCM process
/// acts on — the `$PK` term name does that — so no particular annotation
/// form is required: `; WT_CL`, `; WT_CL cov`, `; WT_CL (L/h) :LOG`, and the
/// numbered style `; 6 WT_CL WT on clearance` all name `WT_CL`. `None` when
/// there is no usable comment. Used only to describe thetas in warnings.
fn comment_name(model: &Model, idx0: usize) -> Option<String> {
    let comment = strip_leading_index(model.thetas[idx0].comment.as_deref()?);
    match parse_theta_param(comment, CommentType::Type1) {
        Some(ParsedThetaComment::Type1(
            Type1Theta::Covariate { parameter } | Type1Theta::WithUnit { parameter, .. },
        )) => Some(parameter),
        _ => first_name_token(comment),
    }
}

/// What to call a theta in a warning: what its comment calls it, or
/// `THETA<n>` (1-based) when the theta has no usable comment.
fn candidate_name(model: &Model, idx0: usize) -> String {
    comment_name(model, idx0).unwrap_or_else(|| format!("THETA{}", idx0 + 1))
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

/// The first whitespace-separated word of a comment, when it is safe to use
/// as a model-file name component.
fn first_name_token(comment: &str) -> Option<String> {
    let token = comment.split_whitespace().next()?;
    let safe = !token.is_empty()
        && token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'));
    safe.then(|| token.to_string())
}

/// Whether THETA(`theta_num`) (1-based) is written the way a covariate effect
/// conventionally is when it is held out of the model: fixed at zero. Nothing
/// requires the shape — the config alone names the candidates — but a theta
/// carrying it that no one requested is worth pointing at.
fn is_candidate_theta(model: &Model, theta_num: usize) -> bool {
    model
        .thetas
        .get(theta_num - 1)
        .is_some_and(|t| t.fixed && t.init == 0.0)
}

/// Every 1-based THETA number referenced anywhere in `expr`.
fn thetas_in_expr(expr: &NmtranExpr, out: &mut BTreeSet<usize>) {
    match expr {
        NmtranExpr::FunctionCall { name, args } => {
            if name.eq_ignore_ascii_case("THETA")
                && let [NmtranExpr::Number(n)] = args.as_slice()
                && n.fract() == 0.0
                && *n >= 1.0
            {
                out.insert(*n as usize);
            } else {
                for a in args {
                    thetas_in_expr(a, out);
                }
            }
        }
        NmtranExpr::BinaryExpr { lhs, rhs, .. } => {
            thetas_in_expr(lhs, out);
            thetas_in_expr(rhs, out);
        }
        NmtranExpr::UnaryExpr { operand, .. } => thetas_in_expr(operand, out),
        NmtranExpr::Paren(inner) => thetas_in_expr(inner, out),
        NmtranExpr::Number(_) | NmtranExpr::Ident(_) => {}
    }
}

/// An assignment target in the abbreviated code and the THETAs its
/// expressions reference, accumulated over every assignment to that target
/// (IF/ELSE branches included). The spelling is the author's first.
struct PkTerm {
    name: String,
    thetas: BTreeSet<usize>,
}

fn collect_pk_terms(stmts: &[NmtranStatement], terms: &mut Vec<PkTerm>) {
    for stmt in stmts {
        match stmt {
            NmtranStatement::Assignment {
                target,
                indices,
                expr,
            } if indices.is_empty() => {
                let mut thetas = BTreeSet::new();
                thetas_in_expr(expr, &mut thetas);
                match terms
                    .iter_mut()
                    .find(|t| t.name.eq_ignore_ascii_case(target))
                {
                    Some(term) => term.thetas.extend(thetas),
                    None => terms.push(PkTerm {
                        name: target.clone(),
                        thetas,
                    }),
                }
            }
            NmtranStatement::If {
                body,
                elseif_branches,
                else_body,
                ..
            } => {
                collect_pk_terms(body, terms);
                for (_, branch) in elseif_branches {
                    collect_pk_terms(branch, terms);
                }
                if let Some(branch) = else_body {
                    collect_pk_terms(branch, terms);
                }
            }
            NmtranStatement::DoWhile { body, .. } => collect_pk_terms(body, terms),
            _ => {}
        }
    }
}

fn pk_terms(block: &CodeBlock) -> Vec<PkTerm> {
    let mut terms = Vec::new();
    collect_pk_terms(&block.statements, &mut terms);
    terms
}

/// The `$PK` term naming each theta a covariates request could ask for: the
/// assignments referencing exactly one THETA, keyed by that 1-based number.
fn pk_names_by_theta(model: &Model) -> BTreeMap<usize, String> {
    let Some(block) = model.pk.as_ref().or(model.pred.as_ref()) else {
        return BTreeMap::new();
    };
    pk_terms(block)
        .into_iter()
        .filter_map(|t| {
            let [theta] = t.thetas.iter().copied().collect::<Vec<_>>()[..] else {
                return None;
            };
            Some((theta, t.name))
        })
        .collect()
}

/// Resolve requested `$PK` term names to `(theta number, name as authored)`.
/// A name resolves when the template's `$PK` (or `$PRED`) assigns it an
/// expression referencing exactly one THETA; matching is case-insensitive so
/// the request doesn't have to reproduce the author's capitalization.
fn resolve_pk_names(model: &Model, names: &[String]) -> Result<Vec<(usize, String)>> {
    let Some(block) = model.pk.as_ref().or(model.pred.as_ref()) else {
        bail!("cannot resolve covariate names: the template has no $PK or $PRED block");
    };
    let terms = pk_terms(block);

    let mut resolved: Vec<(usize, String)> = Vec::new();
    for requested in names {
        let requested = requested.trim();
        if requested.is_empty() {
            bail!("covariates contains an empty name");
        }
        if resolved
            .iter()
            .any(|(_, n)| n.eq_ignore_ascii_case(requested))
        {
            bail!("covariate name {requested} is requested more than once");
        }
        let Some(term) = terms
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(requested))
        else {
            let eligible: Vec<&str> = terms
                .iter()
                .filter(|t| t.thetas.len() == 1)
                .map(|t| t.name.as_str())
                .collect();
            bail!(
                "no $PK term named {requested}; terms in this template referencing a \
                 single THETA: {}",
                if eligible.is_empty() {
                    "(none)".to_string()
                } else {
                    eligible.join(", ")
                }
            );
        };
        match term.thetas.len() {
            1 => {
                let theta = *term.thetas.first().unwrap();
                if let Some((_, other)) = resolved.iter().find(|(t, _)| *t == theta) {
                    bail!(
                        "covariate names {other} and {} both resolve to THETA({theta})",
                        term.name
                    );
                }
                resolved.push((theta, term.name.clone()));
            }
            0 => bail!(
                "$PK term {} does not reference any THETA, so it cannot name a covariate effect",
                term.name
            ),
            _ => bail!(
                "$PK term {} references THETAs {:?}; a covariate term must reference exactly one",
                term.name,
                term.thetas.iter().collect::<Vec<_>>()
            ),
        }
    }
    Ok(resolved)
}

/// Build and validate an SCM plan.
///
/// `covariates` names the candidate effects by their `$PK` term name (see
/// [`resolve_pk_names`]): the term name IS the candidate name, and a theta
/// comment that disagrees with it only warns. `pharos_version` is recorded
/// in the plan for provenance (the binary's `CARGO_PKG_VERSION`).
pub fn build_plan(
    model_path: &Path,
    covariates: &[String],
    out_dir: Option<&Path>,
    options: ScmOptions,
    pharos_version: &str,
) -> Result<BuiltPlan> {
    options.validate()?;

    if covariates.is_empty() {
        bail!("covariates must name at least one $PK term, e.g. [\"WT_CL\", \"CRCL_CL\"]");
    }

    if !model_path.exists() {
        bail!("Model file does not exist: {}", model_path.display());
    }
    validate_model_extension(model_path)?;

    let content = fs::read_to_string(model_path)?;
    let model = Model::parse(model_path, &content)
        .with_context(|| format!("failed to parse template model {}", model_path.display()))?;

    if model.estimations.is_empty() {
        bail!(
            "template model {} has no $ESTIMATION record",
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

    // Resolve the request to `(theta number, name as authored)`, in theta
    // order — the order the plan lists its candidates in.
    let mut selected = resolve_pk_names(&model, covariates)?;
    selected.sort_unstable_by_key(|(n, _)| *n);
    let requested: Vec<usize> = selected.iter().map(|(n, _)| *n).collect();

    let mut warnings = Vec::new();
    let mut candidates = Vec::new();

    for (theta_num, name) in &selected {
        let theta_num = *theta_num;
        let idx0 = theta_num - 1;
        let Some(theta) = model.thetas.get(idx0) else {
            bail!(
                "$PK term {name} references THETA({theta_num}) but the model only has {} thetas",
                model.thetas.len()
            );
        };

        // The $PK term name IS the candidate name. The theta's comment names
        // nothing, but a comment that disagrees is worth saying out loud —
        // one of the two is usually a leftover from an earlier edit.
        if let Some(cn) = comment_name(&model, idx0)
            && !cn.eq_ignore_ascii_case(name)
        {
            warnings.push(format!(
                "THETA({theta_num}) is named {name} by its $PK term but {cn} by its \
                 comment; the $PK name wins"
            ));
        }

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

        // Where the effect starts the first time it is tested: the guess the
        // template already carries for it, or the plan's release_init when
        // the theta is written as an absent effect (`(0 FIX)`, or plain 0).
        let init = if theta.init != 0.0 {
            theta.init
        } else {
            options.release_init
        };

        candidates.push(Candidate {
            name: name.clone(),
            theta: theta_num,
            init,
        });
    }

    // Surface thetas the caller did NOT request that look like covariate
    // effects held out of the model — fixed at 0. Plenty of thetas are
    // `(0 FIX)` for their own reasons, so this is a nudge, not a rule: only
    // `covariates` decides what gets tested.
    let requestable = pk_names_by_theta(&model);
    for i in 0..model.thetas.len() {
        let theta_num = i + 1;
        if requested.contains(&theta_num) || !is_candidate_theta(&model, theta_num) {
            continue;
        }
        // Name it the way a request would have to — by its $PK term — so the
        // warning doubles as the line to add to `covariates`. A theta no
        // single-THETA term names cannot be requested at all; fall back to
        // its comment so the warning still points somewhere.
        let name = requestable
            .get(&theta_num)
            .cloned()
            .unwrap_or_else(|| candidate_name(&model, i));
        warnings.push(format!(
            "THETA({theta_num}) [{name}] is fixed at 0 like a covariate effect but is not in \
             `covariates`; it will NOT be tested"
        ));
    }

    if options.cov_step && model.covariance.is_none() {
        warnings.push(
            "template has no $COVARIANCE record; cov_step is on, so one will be appended to generated models"
                .to_string(),
        );
    }
    if !options.cov_step && model.covariance.is_some() {
        warnings.push(
            "cov_step is off: the template's $COVARIANCE record will be removed from generated models"
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
    use std::path::PathBuf;

    /// Shorthand for the covariates argument: the `$PK` term names naming
    /// the candidate effects.
    pub(crate) fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    /// The test templates carry a `$COVARIANCE` record, so tests that expect a
    /// warning-free plan opt the covariance step back on (the default is off).
    pub(crate) fn opts_cov_on() -> ScmOptions {
        ScmOptions {
            cov_step: true,
            ..Default::default()
        }
    }

    /// The template style the SCM process requires: each candidate effect is
    /// its own named `$PK` assignment referencing exactly one theta, so the
    /// term name can key the request. Writing those thetas `(0 FIX)` is the
    /// convention, not a rule.
    pub(crate) const TEMPLATE: &str = "\
$PROBLEM scm template
$INPUT ID TIME AMT DV WT CRCL AGE
$DATA data.csv IGNORE=@
$SUBROUTINES ADVAN2 TRANS2
$PK
WT_CL = (WT/70)**THETA(4)
CRCL_CL = (CRCL/100)**THETA(5)
WT_V = (WT/70)**THETA(6)
CL = THETA(1) * WT_CL * CRCL_CL * EXP(ETA(1))
V  = THETA(2) * WT_V * EXP(ETA(2))
KA = THETA(3)
S2 = V
$ERROR
Y = F * (1 + EPS(1))
$THETA (0, 3)    ; TVCL (L/h)
$THETA (0, 20)   ; TVV (L)
$THETA (0, 1.2)  ; TVKA (1/h)
$THETA (0 FIX)   ; WT_CL cov
$THETA (0 FIX)   ; CRCL_CL cov
$THETA (0 FIX)   ; WT_V cov
$OMEGA 0.1
$OMEGA 0.1
$SIGMA 0.02
$ESTIMATION METHOD=1 INTER MAXEVAL=9999 NOABORT
$COVARIANCE
";

    /// The same model written inline — the covariate effects folded into the
    /// `TVCL` / `V` expressions instead of standing on their own. No term
    /// names a single candidate theta, so nothing in it can be requested.
    pub(crate) const INLINE_TEMPLATE: &str = "\
$PROBLEM scm template (inline covariate effects)
$INPUT ID TIME AMT DV WT CRCL AGE
$DATA data.csv IGNORE=@
$SUBROUTINES ADVAN2 TRANS2
$PK
TVCL = THETA(1) * (WT/70)**THETA(4) * (CRCL/100)**THETA(5)
CL = TVCL * EXP(ETA(1))
V  = THETA(2) * (WT/70)**THETA(6) * EXP(ETA(2))
KA = THETA(3)
S2 = V
$ERROR
Y = F * (1 + EPS(1))
$THETA (0, 3)    ; TVCL (L/h)
$THETA (0, 20)   ; TVV (L)
$THETA (0, 1.2)  ; TVKA (1/h)
$THETA (0 FIX)   ; WT_CL cov
$THETA (0 FIX)   ; CRCL_CL cov
$THETA (0 FIX)   ; WT_V cov
$OMEGA 0.1
$OMEGA 0.1
$SIGMA 0.02
$ESTIMATION METHOD=1 INTER MAXEVAL=9999 NOABORT
$COVARIANCE
";

    /// Write the template + a dummy dataset into `dir`, returning the model path.
    pub(crate) fn write_template(dir: &Path) -> PathBuf {
        write_template_content(dir, TEMPLATE)
    }

    pub(crate) fn write_template_content(dir: &Path, content: &str) -> PathBuf {
        let model_path = dir.join("1001.mod");
        fs::write(&model_path, content).unwrap();
        fs::write(
            dir.join("data.csv"),
            "ID,TIME,AMT,DV,WT,CRCL,AGE\n1,0,100,0,70,100,40\n",
        )
        .unwrap();
        model_path
    }

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
    fn pk_name_matching_is_case_insensitive_but_keeps_the_authored_spelling() {
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
    fn pk_name_wins_over_a_disagreeing_comment() {
        let dir = tempfile::tempdir().unwrap();
        let renamed = TEMPLATE.replace("; WT_V cov", "; WTONV cov");
        let model_path = write_template_content(dir.path(), &renamed);
        let built = build_plan(
            &model_path,
            &names(&["WT_V"]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap();
        assert_eq!(built.plan.candidates[0].name, "WT_V");
        assert!(
            built
                .warnings
                .iter()
                .any(|w| w.contains("WTONV") && w.contains("$PK name wins")),
            "warnings: {:?}",
            built.warnings
        );
    }

    #[test]
    fn unknown_pk_name_lists_the_eligible_terms() {
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
        let msg = err.to_string();
        assert!(msg.contains("no $PK term named AGE_CL"), "got: {msg}");
        assert!(
            msg.contains("WT_CL") && msg.contains("CRCL_CL") && msg.contains("WT_V"),
            "got: {msg}"
        );
    }

    #[test]
    fn pk_name_referencing_multiple_thetas_errors() {
        let dir = tempfile::tempdir().unwrap();
        // In the inline-style template, TVCL references THETA(1), (4) and (5).
        let model_path = write_template_content(dir.path(), INLINE_TEMPLATE);
        let err = build_plan(
            &model_path,
            &names(&["TVCL"]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("exactly one"),
            "got: {}",
            err.to_string()
        );
    }

    #[test]
    fn a_template_with_no_named_effects_cannot_be_planned() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template_content(dir.path(), INLINE_TEMPLATE);
        // The candidate thetas are buried in TVCL / V, so nothing names them.
        let err = build_plan(
            &model_path,
            &names(&["WT_CL"]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("no $PK term named WT_CL"), "got: {msg}");
        // The covariate effects are folded into TVCL / V, so no term offers
        // one — whatever other single-theta terms the template happens to
        // have.
        assert!(!msg.contains("TVCL") && !msg.contains("WT_V"), "got: {msg}");
    }

    #[test]
    fn duplicate_pk_names_error() {
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
        let err = build_plan(&model_path, &[], None, ScmOptions::default(), "test").unwrap_err();
        assert!(err.to_string().contains("$PK term"), "got: {err}");
    }

    #[test]
    fn a_name_pointing_past_the_last_theta_errors() {
        let dir = tempfile::tempdir().unwrap();
        // $PK names an effect on a theta $THETA never declares.
        let short = TEMPLATE.replace("WT_V = (WT/70)**THETA(6)", "WT_V = (WT/70)**THETA(9)");
        let model_path = write_template_content(dir.path(), &short);
        let err = build_plan(
            &model_path,
            &names(&["WT_V"]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("THETA(9)") && msg.contains("only has 6"),
            "got: {msg}"
        );
    }

    #[test]
    fn unrequested_candidate_warns() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template(dir.path());
        let built = build_plan(
            &model_path,
            &names(&["WT_CL", "CRCL_CL"]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap();
        assert_eq!(built.plan.candidates.len(), 2);
        // Named the way the request would have to name it.
        assert!(
            built
                .warnings
                .iter()
                .any(|w| w.contains("[WT_V]") && w.contains("not in `covariates`")),
            "warnings: {:?}",
            built.warnings
        );
    }

    #[test]
    fn unrequested_zero_fixed_theta_warns_without_any_annotation() {
        let dir = tempfile::tempdir().unwrap();
        // The `(0 FIX)` shape alone earns the nudge: no comment
        // at all on one theta, a non-cov comment on another, and a fourth
        // theta no $PK term names.
        let bare = TEMPLATE
            .replace("$THETA (0 FIX)   ; WT_V cov", "$THETA (0 FIX)")
            .replace("; CRCL_CL cov", "; CRCL_CL some note")
            .replace(
                "$OMEGA 0.1\n$OMEGA 0.1",
                "$THETA (0 FIX)\n$OMEGA 0.1\n$OMEGA 0.1",
            );
        let model_path = write_template_content(dir.path(), &bare);
        let built = build_plan(
            &model_path,
            &names(&["WT_CL"]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap();
        assert!(
            built
                .warnings
                .iter()
                .any(|w| w.contains("THETA(5)") && w.contains("not in `covariates`")),
            "warnings: {:?}",
            built.warnings
        );
        assert!(
            built
                .warnings
                .iter()
                .any(|w| w.contains("THETA(6)") && w.contains("not in `covariates`")),
            "warnings: {:?}",
            built.warnings
        );
        // Nothing in $PK names THETA(7), so it falls back to its position —
        // it could not be requested at all as the template stands.
        assert!(
            built
                .warnings
                .iter()
                .any(|w| w.contains("THETA(7) [THETA7]") && w.contains("not in `covariates`")),
            "warnings: {:?}",
            built.warnings
        );
        // a structural theta (not fixed at 0) never triggers it
        assert!(
            !built.warnings.iter().any(|w| w.contains("THETA(1)")),
            "warnings: {:?}",
            built.warnings
        );
    }

    #[test]
    fn comment_forms_that_agree_with_the_pk_name_never_warn() {
        let dir = tempfile::tempdir().unwrap();
        // No `cov` suffix, a unit-style comment, and the numbered house
        // style: every one of them agrees with the $PK term that names the
        // candidate, so none of them has anything to say.
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
    fn a_theta_with_no_usable_comment_is_still_named_by_its_pk_term() {
        let dir = tempfile::tempdir().unwrap();
        // No comment at all, and a number-only comment: neither names the
        // theta, and neither needs to.
        let bare = TEMPLATE
            .replace("$THETA (0 FIX)   ; WT_V cov", "$THETA (0 FIX)")
            .replace("; CRCL_CL cov", "; 5");
        let model_path = write_template_content(dir.path(), &bare);
        let built = build_plan(
            &model_path,
            &names(&["WT_CL", "CRCL_CL", "WT_V"]),
            None,
            opts_cov_on(),
            "test",
        )
        .unwrap();
        assert_eq!(built.plan.candidates[1].name, "CRCL_CL");
        assert_eq!(built.plan.candidates[2].name, "WT_V");
        assert!(built.warnings.is_empty(), "warnings: {:?}", built.warnings);
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
        // A template that already carries an initial guess for the effect —
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
        // `covv` is not the `cov` annotation, and no longer needs to be: the
        // $PK term names the candidate, whatever the comment says.
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

        // no $COVARIANCE in template + cov_step on -> warn about appending
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
        assert!(text.contains("retry up to 3x"));
        // 3 candidates, both phases: 1 + 2 * 3(3+1)/2 = 13
        assert!(text.contains("max models : 13"), "got:\n{text}");
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
