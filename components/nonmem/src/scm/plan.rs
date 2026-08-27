use std::path::Path;

use anyhow::{Context, Result, bail};
use fs_err as fs;
use nonmem_parser::{CommentType, Model, ParsedThetaComment, Type1Theta, parse_theta_param};
use utils::get_utc_now;

use super::{Candidate, PLAN_SCHEMA_VERSION, ScmOptions, ScmPlan};
use crate::validate_model_extension;

/// A built plan plus non-fatal findings worth surfacing to the user.
#[derive(Debug, Clone)]
pub struct BuiltPlan {
    pub plan: ScmPlan,
    pub warnings: Vec<String>,
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

/// True when the theta carries an explicit `; NAME cov` annotation
/// (optionally behind a leading position number).
fn is_cov_annotated(model: &Model, idx0: usize) -> bool {
    let Some(comment) = model.thetas[idx0].comment.as_deref() else {
        return false;
    };
    matches!(
        parse_theta_param(strip_leading_index(comment), CommentType::Type1),
        Some(ParsedThetaComment::Type1(Type1Theta::Covariate { .. }))
    )
}

/// Build and validate an SCM plan.
///
/// `covariates` are 1-based THETA numbers in the template. Candidate names
/// are read from each theta's comment when it has one, and fall back to
/// `THETA<n>` when it does not. `pharos_version` is recorded in the plan for
/// provenance (the binary's `CARGO_PKG_VERSION`).
pub fn build_plan(
    model_path: &Path,
    covariates: &[usize],
    out_dir: Option<&Path>,
    options: ScmOptions,
    pharos_version: &str,
) -> Result<BuiltPlan> {
    options.validate()?;

    if covariates.is_empty() {
        bail!("covariates must contain at least one THETA number");
    }
    let mut sorted = covariates.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    if sorted.len() != covariates.len() {
        bail!("covariates contains duplicate THETA numbers");
    }
    if sorted[0] == 0 {
        bail!("covariates are 1-based THETA numbers; 0 is not a valid THETA");
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

    let mut warnings = Vec::new();
    let mut candidates = Vec::new();

    for &theta_num in &sorted {
        let idx0 = theta_num - 1;
        let Some(theta) = model.thetas.get(idx0) else {
            bail!(
                "THETA({theta_num}) requested as a covariate but the model only has {} thetas",
                model.thetas.len()
            );
        };

        let name = candidate_name(&model, idx0);
        if comment_name(&model, idx0).is_none() {
            warnings.push(format!(
                "THETA({theta_num}) has no usable comment; the candidate is named {name}"
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

    // Surface `cov`-annotated thetas the caller did NOT request. The
    // annotation no longer selects anything, but a template that marks a
    // covariate the request leaves out is the shape of an oversight.
    for i in 0..model.thetas.len() {
        let theta_num = i + 1;
        if sorted.contains(&theta_num) || !is_cov_annotated(&model, i) {
            continue;
        }
        warnings.push(format!(
            "THETA({theta_num}) is annotated `; {} cov` but was not requested; it will NOT be tested",
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
            build_plan(&model_path, &[4, 5, 6], None, ScmOptions::default(), "test").unwrap();
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

    #[test]
    fn unrequested_candidate_warns() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template(dir.path());
        let built = build_plan(&model_path, &[4, 5], None, ScmOptions::default(), "test").unwrap();
        assert_eq!(built.plan.candidates.len(), 2);
        assert!(
            built.warnings.iter().any(|w| w.contains("WT_V")),
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
            build_plan(&model_path, &[4, 5, 6], None, ScmOptions::default(), "test").unwrap();
        assert_eq!(built.plan.candidates[0].name, "WT_CL");
        assert_eq!(built.plan.candidates[1].name, "CRCL_CL");
        assert!(built.warnings.is_empty(), "warnings: {:?}", built.warnings);
    }

    #[test]
    fn uncommented_theta_is_named_for_its_position() {
        let dir = tempfile::tempdir().unwrap();
        let bare = TEMPLATE.replace("$THETA (0 FIX)   ; WT_V cov", "$THETA (0 FIX)");
        let model_path = write_template_content(dir.path(), &bare);
        let built = build_plan(&model_path, &[6], None, ScmOptions::default(), "test").unwrap();
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
            build_plan(&model_path, &[4, 5, 6], None, ScmOptions::default(), "test").unwrap();
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
        let built = build_plan(&model_path, &[6], None, ScmOptions::default(), "test").unwrap();
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
        let built = build_plan(&model_path, &[4], None, ScmOptions::default(), "test").unwrap();
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
        assert!(build_plan(&model_path, &[42], None, ScmOptions::default(), "test").is_err());
        assert!(build_plan(&model_path, &[4, 4], None, ScmOptions::default(), "test").is_err());
        assert!(build_plan(&model_path, &[], None, ScmOptions::default(), "test").is_err());
        assert!(build_plan(&model_path, &[0], None, ScmOptions::default(), "test").is_err());
    }

    #[test]
    fn rejects_released_candidate_theta() {
        let dir = tempfile::tempdir().unwrap();
        let bad = TEMPLATE.replace("$THETA (0 FIX)   ; WT_CL cov", "$THETA 0.1   ; WT_CL cov");
        let model_path = write_template_content(dir.path(), &bad);
        let err = build_plan(&model_path, &[4], None, ScmOptions::default(), "test").unwrap_err();
        assert!(err.to_string().contains("must be fixed"), "got: {err}");
    }

    #[test]
    fn rejects_missing_dataset() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = dir.path().join("1001.mod");
        fs::write(&model_path, TEMPLATE).unwrap();
        let err = build_plan(&model_path, &[4], None, ScmOptions::default(), "test").unwrap_err();
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
            build_plan(&model_path, &[4, 5, 6], None, ScmOptions::default(), "test").unwrap();
        assert_eq!(built.plan.candidates[2].name, "WT_V");
        assert!(built.warnings.is_empty(), "warnings: {:?}", built.warnings);
    }

    #[test]
    fn cov_step_warnings() {
        let dir = tempfile::tempdir().unwrap();

        // no $COVARIANCE in template + cov_step on -> warn about appending
        let no_cov = TEMPLATE.replace("$COVARIANCE\n", "");
        let model_path = write_template_content(dir.path(), &no_cov);
        let built = build_plan(&model_path, &[4], None, ScmOptions::default(), "test").unwrap();
        assert!(built.warnings.iter().any(|w| w.contains("appended")));

        // $COVARIANCE present + cov_step off -> warn about removal
        let model_path = write_template(dir.path());
        let opts = ScmOptions {
            cov_step: false,
            ..Default::default()
        };
        let built = build_plan(&model_path, &[4], None, opts, "test").unwrap();
        assert!(built.warnings.iter().any(|w| w.contains("removed")));
    }

    #[test]
    fn plan_render_text_mentions_the_essentials() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = write_template(dir.path());
        let built =
            build_plan(&model_path, &[4, 5, 6], None, ScmOptions::default(), "test").unwrap();
        let text = built.plan.render_text();
        assert!(text.contains("<scm plan>"));
        assert!(text.contains("forward    : alpha 0.05"));
        assert!(text.contains("backward   : alpha 0.001"));
        assert!(text.contains("WT_CL"));
        assert!(text.contains("THETA(4)"));
        assert!(text.contains("retry up to 3x"));
    }
}
