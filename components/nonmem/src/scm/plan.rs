use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result, bail};
use fs_err as fs;
use nonmem_parser::{
    CodeBlock, CommentType, Model, NmtranExpr, NmtranStatement, ParsedThetaComment, Type1Theta,
    parse_theta_param,
};
use utils::get_utc_now;

use super::{Candidate, PLAN_SCHEMA_VERSION, ScmOptions, ScmPlan};
use crate::validate_model_extension;

/// A built plan plus non-fatal findings worth surfacing to the user.
#[derive(Debug, Clone)]
pub struct BuiltPlan {
    pub plan: ScmPlan,
    pub warnings: Vec<String>,
}

/// How the caller identifies the candidate covariate effects. In a config
/// file this is untagged: an array of integers is THETA numbers, an array of
/// strings is `$PK` term names.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum CovariateSpec {
    /// 1-based THETA numbers in the template, e.g. `[6, 7, 8]`.
    ThetaNumbers(Vec<usize>),
    /// `$PK` term names, exactly as the author wrote them (matched
    /// case-insensitively): each must be an assignment in `$PK` (or `$PRED`)
    /// whose expression references exactly one THETA, e.g.
    /// `WT_CL = ((WT/70)**THETA(6))` makes `WT_CL` name THETA(6).
    PkNames(Vec<String>),
}

impl CovariateSpec {
    /// Interpret raw CLI tokens: all-numeric means THETA numbers, otherwise
    /// the whole list is `$PK` term names. Mixing the two is an error — a
    /// half-renamed list is the shape of a typo.
    pub fn from_args(args: &[String]) -> Result<Self> {
        let numeric = args
            .iter()
            .filter(|a| a.trim().parse::<usize>().is_ok())
            .count();
        if numeric == args.len() {
            Ok(Self::ThetaNumbers(
                args.iter().map(|a| a.trim().parse().unwrap()).collect(),
            ))
        } else if numeric == 0 {
            Ok(Self::PkNames(
                args.iter().map(|a| a.trim().to_string()).collect(),
            ))
        } else {
            bail!(
                "covariates mixes THETA numbers and names; give all numbers (6,7,8) \
                 or all $PK term names (WT_CL,CRCL_CL)"
            );
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Self::ThetaNumbers(v) => v.is_empty(),
            Self::PkNames(v) => v.is_empty(),
        }
    }
}

/// The name a theta's comment gives it. The caller names candidates by THETA
/// number, so no particular annotation form is required — `; WT_CL`,
/// `; WT_CL cov`, `; WT_CL (L/h) :LOG`, and the numbered style
/// `; 6 WT_CL WT on clearance` all name `WT_CL`. `None` when there is no
/// usable comment.
fn comment_name(model: &Model, idx0: usize) -> Option<String> {
    let comment = strip_leading_index(model.thetas[idx0].comment.as_deref()?);
    match parse_theta_param(comment, CommentType::Type1) {
        Some(ParsedThetaComment::Type1(
            Type1Theta::Covariate { parameter } | Type1Theta::WithUnit { parameter, .. },
        )) => Some(parameter),
        _ => first_name_token(comment),
    }
}

/// The candidate's name: what its comment calls it, or `THETA<n>` (1-based)
/// when the theta has no usable comment.
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
        let Some(term) = terms.iter().find(|t| t.name.eq_ignore_ascii_case(requested)) else {
            let eligible: Vec<&str> = terms
                .iter()
                .filter(|t| t.thetas.len() == 1)
                .filter(|t| {
                    let n = *t.thetas.first().unwrap();
                    model
                        .thetas
                        .get(n - 1)
                        .is_some_and(|th| th.fixed && th.init == 0.0)
                })
                .map(|t| t.name.as_str())
                .collect();
            bail!(
                "no $PK term named {requested}; terms in this template referencing a \
                 single `(0 FIX)` THETA: {}",
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
/// `covariates` selects the candidate effects either by 1-based THETA number
/// or by `$PK` term name (see [`CovariateSpec`]). When selected by number,
/// candidate names are read from each theta's comment when it has one, and
/// fall back to `THETA<n>` when it does not; when selected by name, the `$PK`
/// term name IS the candidate name and a disagreeing comment only warns.
/// `pharos_version` is recorded in the plan for provenance (the binary's
/// `CARGO_PKG_VERSION`).
pub fn build_plan(
    model_path: &Path,
    covariates: &CovariateSpec,
    out_dir: Option<&Path>,
    options: ScmOptions,
    pharos_version: &str,
) -> Result<BuiltPlan> {
    options.validate()?;

    if covariates.is_empty() {
        bail!("covariates must contain at least one THETA number or $PK term name");
    }
    if let CovariateSpec::ThetaNumbers(numbers) = covariates {
        let mut sorted = numbers.clone();
        sorted.sort_unstable();
        sorted.dedup();
        if sorted.len() != numbers.len() {
            bail!("covariates contains duplicate THETA numbers");
        }
        if sorted[0] == 0 {
            bail!("covariates are 1-based THETA numbers; 0 is not a valid THETA");
        }
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
        model_path
            .parent()
            .unwrap_or(Path::new("."))
            .join(data_path)
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

    // Resolve the request to `(theta number, $PK name when keyed by name)`,
    // in theta order either way.
    let selected: Vec<(usize, Option<String>)> = match covariates {
        CovariateSpec::ThetaNumbers(numbers) => {
            let mut sorted = numbers.clone();
            sorted.sort_unstable();
            sorted.into_iter().map(|n| (n, None)).collect()
        }
        CovariateSpec::PkNames(names) => {
            let mut resolved = resolve_pk_names(&model, names)?;
            resolved.sort_unstable_by_key(|(n, _)| *n);
            resolved
                .into_iter()
                .map(|(n, name)| (n, Some(name)))
                .collect()
        }
    };
    let requested: Vec<usize> = selected.iter().map(|(n, _)| *n).collect();

    let mut warnings = Vec::new();
    let mut candidates = Vec::new();

    for (theta_num, pk_name) in &selected {
        let theta_num = *theta_num;
        let idx0 = theta_num - 1;
        let Some(theta) = model.thetas.get(idx0) else {
            bail!(
                "THETA({theta_num}) requested as a covariate but the model only has {} thetas",
                model.thetas.len()
            );
        };

        // Keyed by $PK name, the name IS the candidate name; keyed by THETA
        // number, the theta's comment names it (`THETA<n>` as a last resort).
        let name = match pk_name {
            Some(n) => {
                if let Some(cn) = comment_name(&model, idx0)
                    && !cn.eq_ignore_ascii_case(n)
                {
                    warnings.push(format!(
                        "THETA({theta_num}) is named {n} by its $PK term but {cn} by its \
                         comment; the $PK name wins"
                    ));
                }
                n.clone()
            }
            None => {
                let name = candidate_name(&model, idx0);
                if comment_name(&model, idx0).is_none() {
                    warnings.push(format!(
                        "THETA({theta_num}) has no usable comment; the candidate is named {name}"
                    ));
                }
                name
            }
        };

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

        if !theta.fixed {
            bail!(
                "THETA({theta_num}) [{name}] must be fixed in the template, e.g. `(0 FIX)`; found a free theta with init {}",
                theta.init
            );
        }
        if theta.init != 0.0 {
            bail!(
                "THETA({theta_num}) [{name}] must be fixed at 0 in the template; found `{} FIX`",
                theta.init
            );
        }

        if candidates.iter().any(|c: &Candidate| c.name == name) {
            bail!(
                "candidate name {name} appears on more than one requested theta; \
                 give them distinct comments"
            );
        }

        candidates.push(Candidate {
            name,
            theta: theta_num,
        });
    }

    // Surface thetas the caller did NOT request that look like candidate
    // effects — fixed at 0, exactly the shape every candidate must have. A
    // template carrying a `(0 FIX)` theta the request leaves out is the
    // shape of an oversight, whatever its comment says.
    for (i, theta) in model.thetas.iter().enumerate() {
        let theta_num = i + 1;
        if requested.contains(&theta_num) || !(theta.fixed && theta.init == 0.0) {
            continue;
        }
        warnings.push(format!(
            "THETA({theta_num}) [{}] is fixed at 0 like a candidate effect but was not \
             requested; it will NOT be tested",
            candidate_name(&model, i)
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
        None => model_path
            .parent()
            .unwrap_or(Path::new("."))
            .join("scm")
            .join(&stem),
    };

    let plan = ScmPlan {
        schema_version: PLAN_SCHEMA_VERSION,
        created: get_utc_now(),
        pharos_version: pharos_version.to_string(),
        model: model_path.to_string_lossy().to_string(),
        out_dir: out_dir.to_string_lossy().to_string(),
        candidates,
        options,
    };

    Ok(BuiltPlan { plan, warnings })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Shorthand for the THETA-number form of the covariates argument.
    pub(crate) fn thetas(v: &[usize]) -> CovariateSpec {
        CovariateSpec::ThetaNumbers(v.to_vec())
    }

    /// Shorthand for the $PK-name form of the covariates argument.
    pub(crate) fn names(v: &[&str]) -> CovariateSpec {
        CovariateSpec::PkNames(v.iter().map(|s| s.to_string()).collect())
    }

    pub(crate) const TEMPLATE: &str = "\
$PROBLEM scm template
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

        let built =
            build_plan(&model_path, &thetas(&[4, 5, 6]), None, ScmOptions::default(), "test").unwrap();
        let plan = &built.plan;

        assert_eq!(plan.candidates.len(), 3);
        assert_eq!(plan.candidates[0].name, "WT_CL");
        assert_eq!(plan.candidates[0].theta, 4);
        assert_eq!(plan.candidates[1].name, "CRCL_CL");
        assert_eq!(plan.candidates[2].name, "WT_V");
        assert!(plan.out_dir.ends_with("scm/1001"));
        assert!(built.warnings.is_empty(), "warnings: {:?}", built.warnings);

        // save + load round trip
        let path = plan.save().unwrap();
        let loaded = ScmPlan::load(&path).unwrap();
        assert_eq!(&loaded, plan);
    }

    /// The standardized-template style: each candidate effect is its own
    /// named `$PK` assignment, so the term name can key the request.
    pub(crate) const NAMED_TEMPLATE: &str = "\
$PROBLEM scm template (named $PK terms)
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

    #[test]
    fn pk_names_resolve_to_thetas() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template_content(dir.path(), NAMED_TEMPLATE);
        let built = build_plan(
            &model_path,
            &names(&["WT_CL", "CRCL_CL", "WT_V"]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap();
        assert_eq!(built.plan.candidates.len(), 3);
        assert_eq!(built.plan.candidates[0].name, "WT_CL");
        assert_eq!(built.plan.candidates[0].theta, 4);
        assert_eq!(built.plan.candidates[1].name, "CRCL_CL");
        assert_eq!(built.plan.candidates[1].theta, 5);
        assert_eq!(built.plan.candidates[2].name, "WT_V");
        assert_eq!(built.plan.candidates[2].theta, 6);
        assert!(built.warnings.is_empty(), "warnings: {:?}", built.warnings);

        // The same plan the THETA-number form builds.
        let by_number = build_plan(
            &model_path,
            &thetas(&[4, 5, 6]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap();
        assert_eq!(built.plan.candidates, by_number.plan.candidates);
    }

    #[test]
    fn pk_name_matching_is_case_insensitive_but_keeps_the_authored_spelling() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template_content(dir.path(), NAMED_TEMPLATE);
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
        let renamed = NAMED_TEMPLATE.replace("; WT_V cov", "; WTONV cov");
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
        let model_path = write_template_content(dir.path(), NAMED_TEMPLATE);
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
        // In the inline-style TEMPLATE, TVCL references THETA(1), (4) and (5).
        let model_path = write_template(dir.path());
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
    fn duplicate_pk_names_error() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template_content(dir.path(), NAMED_TEMPLATE);
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
    fn covariate_args_split_numbers_from_names() {
        let to_args = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            CovariateSpec::from_args(&to_args(&["6", "7", "8"])).unwrap(),
            CovariateSpec::ThetaNumbers(vec![6, 7, 8])
        );
        assert_eq!(
            CovariateSpec::from_args(&to_args(&["WT_CL", "CRCL_CL"])).unwrap(),
            CovariateSpec::PkNames(vec!["WT_CL".to_string(), "CRCL_CL".to_string()])
        );
        assert!(CovariateSpec::from_args(&to_args(&["6", "WT_CL"])).is_err());
    }

    #[test]
    fn unrequested_candidate_warns() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template(dir.path());
        let built = build_plan(&model_path, &thetas(&[4, 5]), None, ScmOptions::default(), "test").unwrap();
        assert_eq!(built.plan.candidates.len(), 2);
        assert!(
            built.warnings.iter().any(|w| w.contains("WT_V")),
            "warnings: {:?}",
            built.warnings
        );
    }

    #[test]
    fn unrequested_zero_fixed_theta_warns_without_any_annotation() {
        let dir = tempfile::tempdir().unwrap();
        // The `(0 FIX)` shape alone flags a left-out candidate: no comment
        // at all on one theta, a non-cov comment on another.
        let bare = TEMPLATE
            .replace("$THETA (0 FIX)   ; WT_V cov", "$THETA (0 FIX)")
            .replace("; CRCL_CL cov", "; CRCL_CL some note");
        let model_path = write_template_content(dir.path(), &bare);
        let built = build_plan(&model_path, &thetas(&[4]), None, ScmOptions::default(), "test").unwrap();
        assert!(
            built
                .warnings
                .iter()
                .any(|w| w.contains("THETA(5)") && w.contains("not requested")),
            "warnings: {:?}",
            built.warnings
        );
        assert!(
            built
                .warnings
                .iter()
                .any(|w| w.contains("THETA(6)") && w.contains("not requested")),
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
    fn plain_comment_names_the_candidate() {
        let dir = tempfile::tempdir().unwrap();
        // No `cov` suffix, and a unit-style comment: both still name the theta.
        let plain = TEMPLATE
            .replace("; WT_CL cov", "; WT_CL")
            .replace("; CRCL_CL cov", "; CRCL_CL (-) :LOG");
        let model_path = write_template_content(dir.path(), &plain);
        let built =
            build_plan(&model_path, &thetas(&[4, 5, 6]), None, ScmOptions::default(), "test").unwrap();
        assert_eq!(built.plan.candidates[0].name, "WT_CL");
        assert_eq!(built.plan.candidates[1].name, "CRCL_CL");
        assert!(built.warnings.is_empty(), "warnings: {:?}", built.warnings);
    }

    #[test]
    fn uncommented_theta_is_named_for_its_position() {
        let dir = tempfile::tempdir().unwrap();
        let bare = TEMPLATE.replace("$THETA (0 FIX)   ; WT_V cov", "$THETA (0 FIX)");
        let model_path = write_template_content(dir.path(), &bare);
        let built = build_plan(&model_path, &thetas(&[6]), None, ScmOptions::default(), "test").unwrap();
        assert_eq!(built.plan.candidates[0].name, "THETA6");
        assert!(
            built
                .warnings
                .iter()
                .any(|w| w.contains("no usable comment")),
            "warnings: {:?}",
            built.warnings
        );
    }

    #[test]
    fn numbered_comments_name_the_candidate_not_the_number() {
        let dir = tempfile::tempdir().unwrap();
        // The `; <n> NAME description...` house style: the leading position
        // number is a label, not the name.
        let numbered = TEMPLATE
            .replace("; WT_CL cov", "; 4 WT_CL WT on clearance")
            .replace("; CRCL_CL cov", "; 5 CRCL_CL CRCL on clearance")
            .replace("; WT_V cov", "; 6 WT_V cov");
        let model_path = write_template_content(dir.path(), &numbered);
        let built =
            build_plan(&model_path, &thetas(&[4, 5, 6]), None, ScmOptions::default(), "test").unwrap();
        assert_eq!(built.plan.candidates[0].name, "WT_CL");
        assert_eq!(built.plan.candidates[1].name, "CRCL_CL");
        assert_eq!(built.plan.candidates[2].name, "WT_V");
        assert!(built.warnings.is_empty(), "warnings: {:?}", built.warnings);
    }

    #[test]
    fn number_only_comment_falls_back_to_position() {
        let dir = tempfile::tempdir().unwrap();
        let bare = TEMPLATE.replace("; WT_V cov", "; 6");
        let model_path = write_template_content(dir.path(), &bare);
        let built = build_plan(&model_path, &thetas(&[6]), None, ScmOptions::default(), "test").unwrap();
        assert_eq!(built.plan.candidates[0].name, "THETA6");
        assert!(
            built
                .warnings
                .iter()
                .any(|w| w.contains("no usable comment")),
            "warnings: {:?}",
            built.warnings
        );
    }

    #[test]
    fn stale_comment_numbering_warns() {
        let dir = tempfile::tempdir().unwrap();
        let stale = TEMPLATE.replace("; WT_CL cov", "; 9 WT_CL WT on clearance");
        let model_path = write_template_content(dir.path(), &stale);
        let built = build_plan(&model_path, &thetas(&[4]), None, ScmOptions::default(), "test").unwrap();
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
    fn rejects_out_of_range_and_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template(dir.path());
        assert!(build_plan(&model_path, &thetas(&[42]), None, ScmOptions::default(), "test").is_err());
        assert!(build_plan(&model_path, &thetas(&[4, 4]), None, ScmOptions::default(), "test").is_err());
        assert!(build_plan(&model_path, &thetas(&[]), None, ScmOptions::default(), "test").is_err());
        assert!(build_plan(&model_path, &thetas(&[0]), None, ScmOptions::default(), "test").is_err());
    }

    #[test]
    fn rejects_released_candidate_theta() {
        let dir = tempfile::tempdir().unwrap();
        let bad = TEMPLATE.replace("$THETA (0 FIX)   ; WT_CL cov", "$THETA 0.1   ; WT_CL cov");
        let model_path = write_template_content(dir.path(), &bad);
        let err = build_plan(&model_path, &thetas(&[4]), None, ScmOptions::default(), "test").unwrap_err();
        assert!(err.to_string().contains("must be fixed"), "got: {err}");
    }

    #[test]
    fn rejects_missing_dataset() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = dir.path().join("1001.mod");
        fs::write(&model_path, TEMPLATE).unwrap();
        let err = build_plan(&model_path, &thetas(&[4]), None, ScmOptions::default(), "test").unwrap_err();
        assert!(err.to_string().contains("does not exist"), "got: {err}");
    }

    #[test]
    fn annotation_wording_is_not_policed() {
        let dir = tempfile::tempdir().unwrap();
        // `covv` is not the `cov` annotation, and no longer needs to be: the
        // theta was requested by number, so it is simply named WT_V.
        let odd = TEMPLATE.replace("; WT_V cov", "; WT_V covv");
        let model_path = write_template_content(dir.path(), &odd);
        let built =
            build_plan(&model_path, &thetas(&[4, 5, 6]), None, ScmOptions::default(), "test").unwrap();
        assert_eq!(built.plan.candidates[2].name, "WT_V");
        assert!(built.warnings.is_empty(), "warnings: {:?}", built.warnings);
    }

    #[test]
    fn cov_step_warnings() {
        let dir = tempfile::tempdir().unwrap();

        // no $COVARIANCE in template + cov_step on -> warn about appending
        let no_cov = TEMPLATE.replace("$COVARIANCE\n", "");
        let model_path = write_template_content(dir.path(), &no_cov);
        let built = build_plan(&model_path, &thetas(&[4]), None, ScmOptions::default(), "test").unwrap();
        assert!(built.warnings.iter().any(|w| w.contains("appended")));

        // $COVARIANCE present + cov_step off -> warn about removal
        let model_path = write_template(dir.path());
        let opts = ScmOptions {
            cov_step: false,
            ..Default::default()
        };
        let built = build_plan(&model_path, &thetas(&[4]), None, opts, "test").unwrap();
        assert!(built.warnings.iter().any(|w| w.contains("removed")));
    }

    #[test]
    fn plan_render_text_mentions_the_essentials() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template(dir.path());
        let built =
            build_plan(&model_path, &thetas(&[4, 5, 6]), None, ScmOptions::default(), "test").unwrap();
        let text = built.plan.render_text();
        assert!(text.contains("<scm plan>"));
        assert!(text.contains("forward    : alpha 0.05"));
        assert!(text.contains("backward   : alpha 0.001"));
        assert!(text.contains("WT_CL"));
        assert!(text.contains("THETA(4)"));
        assert!(text.contains("retry up to 3x"));
    }
}
