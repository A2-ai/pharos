//! Snapshot tests for the SCM module.
//!
//! Every SCM snapshot lives here, in one module, so the files all land in
//! `src/scm/snapshots/` — apart from the output-file and parser snapshots
//! elsewhere in the workspace. Each test binds
//! [`test_support::snapshot_settings`], which redacts timestamps, the temp
//! dir and the plan digest, so the snapshots are stable across machines.
//!
//! What is snapshotted is what people and hyperion read verbatim: generated
//! control streams, the config and plan files, error messages, the state
//! and record files, and the CLI renderings. Behavior that a plain
//! assertion pins down well already lives in each module's own tests.

use std::path::{Path, PathBuf};

use fs_err as fs;
use insta::assert_snapshot;

use super::config::{ScmPlanOverrides, build_plan_from_config, init_scm};
use super::round::{read_fit_outcome, write_retry_model, write_scm_model};
use super::score::{chi2_sf, lrt};
use super::state::{AttemptRecord, CandidateRecord, CandidateStatus, RoundRecord, ScmState};
use super::test_support::*;
use super::{
    CovariateDefaults, CovariateRequest, Covariates, Direction, MatrixValue, ROUND_SUMMARY_JSON,
    ROUND_SUMMARY_MD, SCM_SUMMARY_FILENAME, ScmOptions, SummaryOptions, build_plan, read_status,
    read_summary, run_scm, sanitize_name,
};

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

/// The candidates a template declares for itself in a `; candidates: A B`
/// comment, when the standard three do not fit it.
fn declared_candidates(content: &str) -> Option<Covariates> {
    content.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("; candidates:")?;
        let declared: Vec<&str> = rest.split_whitespace().collect();
        Some(Covariates::named(&declared))
    })
}

fn model_or_error(result: anyhow::Result<()>, path: &Path) -> String {
    match result {
        Ok(()) => read(path),
        Err(e) => format!("error: {e:#}\n"),
    }
}

// ---------------------------------------------------------------------------
// Generated control streams
// ---------------------------------------------------------------------------

/// Snapshot 1: One round-1 model and one full model per template variant under
/// `test_data/scm/templates/`. A template that cannot be planned snapshots
/// its planning error instead.
#[test]
fn generated_models_for_every_template_variant() {
    insta::glob!("../../test_data/scm/templates", "*.{mod,ctl}", |path| {
        let dir = tempfile::tempdir().unwrap();
        let content = read(path);
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();
        let template = write_named_template(dir.path(), &file_name, &content);
        let candidates =
            declared_candidates(&content).unwrap_or_else(|| names(&["WT_CL", "CRCL_CL", "WT_V"]));

        snapshot_settings(dir.path()).bind(|| {
            let built = match build_plan(&template, &candidates, None, opts_cov_on(), "test") {
                Ok(built) => built,
                Err(e) => {
                    assert_snapshot!("plan_error", format!("{e:#}\n"));
                    return;
                }
            };
            let plan = built.plan;
            let stem = template.file_stem().unwrap().to_string_lossy().to_string();
            let out_dir = plan.out_dir_path();

            let first = &plan.candidates[0];
            let round1 = out_dir
                .join("forward_round1")
                .join(format!("{stem}_{}.mod", sanitize_name(&first.name)));
            let result = write_scm_model(
                &template,
                &round1,
                &plan.candidates,
                &[first.theta],
                None,
                true,
                &format!("SCM forward_round1: add {} (attempt 1)", first.name),
                None,
                false,
            );
            assert_snapshot!("round1", model_or_error(result, &round1));

            let all: Vec<usize> = plan.candidates.iter().map(|c| c.theta).collect();
            let full = out_dir.join("full").join(format!("{stem}_full.mod"));
            let result = write_scm_model(
                &template,
                &full,
                &plan.candidates,
                &all,
                None,
                true,
                "SCM reference: fit full model (attempt 1)",
                None,
                false,
            );
            assert_snapshot!("full", model_or_error(result, &full));
        });
    });
}

/// Snapshot 2: A round-2 model warm-started from the round-1 winner's fit: the
/// retained theta continues from its estimate, the base parameters from
/// theirs, and the newly released candidate starts at its initial.
#[test]
fn round_two_model_warm_starts_from_the_reference_fit() {
    let dir = tempfile::tempdir().unwrap();
    let plan = make_plan(dir.path(), opts_cov_on());
    let template = plan.model_path();

    // The round-1 winner (WT_CL released) and its fit.
    let winner = plan
        .out_dir_path()
        .join("forward_round1")
        .join("1001_wt_cl.mod");
    write_scm_model(
        &template,
        &winner,
        &plan.candidates,
        &[4],
        None,
        true,
        "SCM forward_round1: add WT_CL (attempt 1)",
        None,
        false,
    )
    .unwrap();
    write_fit_output(&winner, Fit::Succeeded(980.0)).unwrap();

    let dest = plan
        .out_dir_path()
        .join("forward_round2")
        .join("1001_crcl_cl.mod");
    write_scm_model(
        &template,
        &dest,
        &plan.candidates,
        &[4, 5],
        Some(&super::round::ext_path_for(&winner)),
        true,
        "SCM forward_round2: add CRCL_CL (attempt 1)",
        Some("../forward_round1/1001_wt_cl.mod"),
        false,
    )
    .unwrap();

    snapshot_settings(dir.path()).bind(|| assert_snapshot!(read(&dest)));
}

/// Snapshot 3: A retry model continues from wherever the failed attempt stopped:
/// the last iteration row of a fit that never reached final estimates.
#[test]
fn retry_model_continues_from_the_failed_attempts_last_iteration() {
    let dir = tempfile::tempdir().unwrap();
    let plan = make_plan(dir.path(), opts_cov_on());
    let template = plan.model_path();

    let first = plan
        .out_dir_path()
        .join("forward_round1")
        .join("1001_wt_cl.mod");
    write_scm_model(
        &template,
        &first,
        &plan.candidates,
        &[4],
        None,
        true,
        "SCM forward_round1: add WT_CL (attempt 1)",
        None,
        false,
    )
    .unwrap();
    write_fit_output(&first, Fit::NoFinalRow).unwrap();

    let retry = first.with_file_name("1001_wt_cl_try2.mod");
    write_retry_model(
        &first,
        &retry,
        "SCM forward_round1: add WT_CL (attempt 2)",
        None,
        false,
    )
    .unwrap();

    snapshot_settings(dir.path()).bind(|| assert_snapshot!(read(&retry)));
}

/// Snapshot 4: The final model a completed forward-then-backward run leaves behind:
/// the retained covariate released and warm-started from the last
/// reference fit, the others documented as `(0 FIX)`.
#[test]
fn final_model_of_a_completed_run() {
    let dir = tempfile::tempdir().unwrap();
    let plan = make_plan(dir.path(), ScmOptions::default());
    let outcome = run_scm(&plan, &full_scm_executor(), None).unwrap();
    let final_model = plan
        .out_dir_path()
        .join(outcome.state.final_model.as_ref().unwrap());

    snapshot_settings(dir.path()).bind(|| assert_snapshot!(read(&final_model)));
}

// ---------------------------------------------------------------------------
// Config and plan
// ---------------------------------------------------------------------------

/// Snapshot 5: The starter config `scm init` writes beside a model.
#[test]
fn init_writes_the_starter_config() {
    let dir = tempfile::tempdir().unwrap();
    let model = write_template(dir.path());
    let init = init_scm(&model, false).unwrap();

    snapshot_settings(dir.path()).bind(|| assert_snapshot!(read(&init.config_path)));
}

/// Snapshot 6: plan.json and the plan rendering (with its warnings) for the option
/// sets a user actually reaches for.
#[test]
fn plan_json_and_text_for_the_main_option_sets() {
    let free_theta = TEMPLATE.replace("$THETA (0 FIX)   ; WT_CL cov", "$THETA 0.4   ; WT_CL cov");
    // WT_V written as a fold-change effect, held out at 1 and released at 1.5;
    // CRCL_CL given its own initial by its row.
    let fold_change = TEMPLATE
        .replace("WT_V = (WT/70)**THETA(6)", "WT_V = THETA(6)**(WT/70)")
        .replace("$THETA (0 FIX)   ; WT_V cov", "$THETA (1 FIX)   ; WT_V cov");
    let std = names(&["WT_CL", "CRCL_CL", "WT_V"]);
    let rows = Covariates {
        defaults: CovariateDefaults {
            initial: 0.2,
            off: 0.0,
            ..Default::default()
        },
        effects: vec![
            CovariateRequest::named("WT_CL"),
            CovariateRequest {
                name: "CRCL_CL".into(),
                initial: Some(0.3),
                off: None,
                ..Default::default()
            },
            CovariateRequest {
                name: "WT_V".into(),
                initial: Some(1.5),
                off: Some(1.0),
                ..Default::default()
            },
        ],
    };
    // Bounds from the section, from a row, and left off entirely.
    let bounded = Covariates {
        defaults: CovariateDefaults {
            lower: Some(0.0),
            ..Default::default()
        },
        effects: vec![
            CovariateRequest::named("WT_CL"),
            CovariateRequest {
                name: "CRCL_CL".into(),
                initial: Some(1.2),
                off: Some(1.0),
                lower: Some(0.01),
                upper: Some(10.0),
            },
            CovariateRequest {
                name: "WT_V".into(),
                upper: Some(2.0),
                ..Default::default()
            },
        ],
    };
    let cases: Vec<(&str, ScmOptions, &str, &Covariates)> = vec![
        ("defaults", ScmOptions::default(), TEMPLATE, &std),
        ("forward_only", opts_forward_only(), TEMPLATE, &std),
        (
            "backward_only",
            ScmOptions {
                direction: vec![Direction::Backward],
                ..Default::default()
            },
            TEMPLATE,
            &std,
        ),
        (
            "num_rounds",
            ScmOptions {
                num_rounds: Some(2),
                ..Default::default()
            },
            TEMPLATE,
            &std,
        ),
        ("cov_on", opts_cov_on(), TEMPLATE, &std),
        ("bounded", ScmOptions::default(), TEMPLATE, &bounded),
        ("template_init", ScmOptions::default(), &free_theta, &std),
        ("fold_change", opts_cov_on(), &fold_change, &rows),
    ];

    for (label, options, template, covariates) in cases {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template_content(dir.path(), template);
        let built = build_plan(&model, covariates, None, options, "test").unwrap();

        let mut text = built.render_text();
        for w in &built.warnings {
            text.push_str(&format!("warning: {w}\n"));
        }

        snapshot_settings(dir.path()).bind(|| {
            assert_snapshot!(format!("plan_json_{label}"), built.plan.to_json().unwrap());
            assert_snapshot!(format!("plan_text_{label}"), text);
        });
    }
}

/// Snapshot 7: Every way `build_plan` refuses a request, including the option
/// validation it runs first. One snapshot, one labelled message per case.
#[test]
fn every_build_plan_error_message() {
    let dir = tempfile::tempdir().unwrap();
    let std_names = names(&["WT_CL", "CRCL_CL", "WT_V"]);
    let alias = TEMPLATE.replace(
        "WT_CL = (WT/70)**THETA(4)\n",
        "WT_CL = (WT/70)**THETA(4)\nWT_CL_ALIAS = (WT/70)**THETA(4)\n",
    );
    let no_est = TEMPLATE.replace("$ESTIMATION METHOD=1 INTER MAXEVAL=9999 NOABORT\n", "");
    let no_pk = TEMPLATE.replace(
        "$PK\nWT_CL = (WT/70)**THETA(4)\nCRCL_CL = (CRCL/100)**THETA(5)\nWT_V = (WT/70)**THETA(6)\nCL = THETA(1) * WT_CL * CRCL_CL * EXP(ETA(1))\nV  = THETA(2) * WT_V * EXP(ETA(2))\nKA = THETA(3)\nS2 = V\n",
        "",
    );
    let past_last = TEMPLATE.replace("WT_V = (WT/70)**THETA(6)", "WT_V = (WT/70)**THETA(9)");
    let opts = |f: fn(&mut ScmOptions)| {
        let mut o = ScmOptions::default();
        f(&mut o);
        o
    };

    let with_values = |name: &str, initial: Option<f64>, off: Option<f64>| Covariates {
        defaults: CovariateDefaults::default(),
        effects: vec![CovariateRequest {
            name: name.into(),
            initial,
            off,
            ..Default::default()
        }],
    };
    // One candidate whose row carries `initial` plus bounds; the bounds go
    // on the section when there is no initial to place them against.
    let with_bounds = |initial: Option<f64>, lower: Option<f64>, upper: Option<f64>| Covariates {
        defaults: CovariateDefaults {
            lower: if initial.is_none() { lower } else { None },
            upper: if initial.is_none() { upper } else { None },
            ..Default::default()
        },
        effects: vec![CovariateRequest {
            name: "WT_CL".into(),
            initial,
            lower: initial.and(lower),
            upper: initial.and(upper),
            ..Default::default()
        }],
    };
    let defaults = |initial: f64, off: f64| Covariates {
        defaults: CovariateDefaults {
            initial,
            off,
            ..Default::default()
        },
        effects: vec![CovariateRequest::named("WT_CL")],
    };

    // (label, template content, covariates, options)
    let cases: Vec<(&str, &str, Covariates, ScmOptions)> = vec![
        (
            "unknown_name",
            TEMPLATE,
            names(&["AGE_CL"]),
            ScmOptions::default(),
        ),
        (
            "unknown_name_no_eligible_terms",
            INLINE_TEMPLATE,
            names(&["WT_CL"]),
            ScmOptions::default(),
        ),
        (
            "term_references_several_thetas",
            INLINE_TEMPLATE,
            names(&["TVCL"]),
            ScmOptions::default(),
        ),
        (
            "term_references_no_theta",
            TEMPLATE,
            names(&["S2"]),
            ScmOptions::default(),
        ),
        (
            "duplicate_names",
            TEMPLATE,
            names(&["WT_CL", "wt_cl"]),
            ScmOptions::default(),
        ),
        (
            "two_names_one_theta",
            &alias,
            names(&["WT_CL", "WT_CL_ALIAS"]),
            ScmOptions::default(),
        ),
        (
            "empty_name",
            TEMPLATE,
            names(&["  "]),
            ScmOptions::default(),
        ),
        (
            "empty_list",
            TEMPLATE,
            Covariates::default(),
            ScmOptions::default(),
        ),
        (
            "no_estimation_record",
            &no_est,
            std_names.clone(),
            ScmOptions::default(),
        ),
        (
            "no_pk_block",
            &no_pk,
            std_names.clone(),
            ScmOptions::default(),
        ),
        (
            "term_past_last_theta",
            &past_last,
            names(&["WT_V"]),
            ScmOptions::default(),
        ),
        (
            "direction_empty",
            TEMPLATE,
            std_names.clone(),
            opts(|o| o.direction = vec![]),
        ),
        (
            "direction_duplicated",
            TEMPLATE,
            std_names.clone(),
            opts(|o| o.direction = vec![Direction::Forward, Direction::Forward]),
        ),
        (
            "forward_alpha_zero",
            TEMPLATE,
            std_names.clone(),
            opts(|o| o.forward_alpha = 0.0),
        ),
        (
            "backward_alpha_one",
            TEMPLATE,
            std_names.clone(),
            opts(|o| o.backward_alpha = 1.0),
        ),
        (
            "num_rounds_zero",
            TEMPLATE,
            std_names.clone(),
            opts(|o| o.num_rounds = Some(0)),
        ),
        (
            "initial_equals_off_row",
            TEMPLATE,
            with_values("WT_CL", Some(1.0), Some(1.0)),
            ScmOptions::default(),
        ),
        (
            "initial_equals_off_defaults",
            TEMPLATE,
            defaults(0.0, 0.0),
            ScmOptions::default(),
        ),
        (
            "initial_nan",
            TEMPLATE,
            with_values("WT_CL", Some(f64::NAN), None),
            ScmOptions::default(),
        ),
        (
            "off_infinite",
            TEMPLATE,
            defaults(0.1, f64::INFINITY),
            ScmOptions::default(),
        ),
        (
            "lower_above_upper_defaults",
            TEMPLATE,
            with_bounds(None, Some(1.0), Some(0.0)),
            ScmOptions::default(),
        ),
        (
            "initial_at_lower",
            TEMPLATE,
            with_bounds(Some(0.1), Some(0.1), None),
            ScmOptions::default(),
        ),
        (
            "initial_above_upper",
            TEMPLATE,
            with_bounds(Some(0.5), None, Some(0.2)),
            ScmOptions::default(),
        ),
    ];

    let mut out = String::new();
    for (label, template, covariates, options) in cases {
        let model = write_template_content(&dir.path().join(label), template);
        let err = build_plan(&model, &covariates, None, options, "test").unwrap_err();
        out.push_str(&format!("{label}\n  {err:#}\n"));
    }

    // Cases about the files rather than their content.
    let missing_data_dir = dir.path().join("missing_dataset");
    fs::create_dir_all(&missing_data_dir).unwrap();
    let model = missing_data_dir.join("1001.mod");
    fs::write(&model, TEMPLATE).unwrap();
    let err = build_plan(&model, &std_names, None, ScmOptions::default(), "test").unwrap_err();
    out.push_str(&format!("missing_dataset\n  {err:#}\n"));

    let err = build_plan(
        &dir.path().join("nope.mod"),
        &std_names,
        None,
        ScmOptions::default(),
        "test",
    )
    .unwrap_err();
    out.push_str(&format!("missing_model\n  {err:#}\n"));

    let txt = write_named_template(&dir.path().join("bad_extension"), "1001.txt", TEMPLATE);
    let err = build_plan(&txt, &std_names, None, ScmOptions::default(), "test").unwrap_err();
    out.push_str(&format!("bad_extension\n  {err:#}\n"));

    snapshot_settings(dir.path()).bind(|| assert_snapshot!(out));
}

/// Snapshot 8: Every way the config loader refuses a `<stem>-scm.toml`.
#[test]
fn every_config_error_message() {
    let dir = tempfile::tempdir().unwrap();
    write_template(dir.path());

    const SECTION: &str = "[covariates]\neffects = [\"WT_CL\"]\n";
    let with_section = |head: &str| format!("{head}{SECTION}");
    let cases: Vec<(&str, String)> = vec![
        (
            "unknown_key",
            with_section("model = \"1001.mod\"\ndirection = [\"forward\"]\nfoward_alpha = 0.01\n"),
        ),
        (
            "stale_out_dir_key",
            with_section("model = \"1001.mod\"\nout_dir = \"scm-out\"\ndirection = [\"forward\"]\n"),
        ),
        (
            "flat_covariates_array",
            "model = \"1001.mod\"\ncovariates = [\"WT_CL\"]\ndirection = [\"forward\"]\n".to_string(),
        ),
        (
            "stale_release_init_key",
            with_section("model = \"1001.mod\"\ndirection = [\"forward\"]\nrelease_init = 0.2\n"),
        ),
        (
            "theta_numbers",
            "model = \"1001.mod\"\ndirection = [\"forward\"]\n[covariates]\neffects = [4, 5, 6]\n".to_string(),
        ),
        (
            "theta_number_mixed_with_names",
            "model = \"1001.mod\"\ndirection = [\"forward\"]\n[covariates]\neffects = [6, \"WT_CL\"]\n".to_string(),
        ),
        (
            "row_without_name",
            "model = \"1001.mod\"\ndirection = [\"forward\"]\n[covariates]\neffects = [{ initial = 0.2 }]\n".to_string(),
        ),
        (
            "row_with_unknown_key",
            "model = \"1001.mod\"\ndirection = [\"forward\"]\n[covariates]\neffects = [{ name = \"WT_CL\", init = 0.2 }]\n".to_string(),
        ),
        (
            "section_with_unknown_key",
            "model = \"1001.mod\"\ndirection = [\"forward\"]\n[covariates]\ninital = 0.2\neffects = [\"WT_CL\"]\n".to_string(),
        ),
        ("missing_covariates", "model = \"1001.mod\"\ndirection = [\"forward\"]\n".to_string()),
        ("missing_direction", with_section("model = \"1001.mod\"\n")),
        ("missing_model", with_section("direction = [\"forward\"]\n")),
        ("malformed_toml", "model = \ndirection = [\"forward\"]\n".to_string()),
        (
            "misspelled_direction",
            with_section("model = \"1001.mod\"\ndirection = [\"foward\"]\n"),
        ),
        (
            "unresolvable_model",
            with_section("model = \"nope/1001.mod\"\ndirection = [\"forward\"]\n"),
        ),
        (
            "empty_effects",
            "model = \"1001.mod\"\ndirection = [\"forward\"]\n[covariates]\neffects = []\n".to_string(),
        ),
        (
            "initial_equals_off",
            "model = \"1001.mod\"\ndirection = [\"forward\"]\n[covariates]\ninitial = 0\neffects = [\"WT_CL\"]\n".to_string(),
        ),
    ];

    let mut out = String::new();
    for (label, body) in cases {
        let config = dir.path().join(format!("{label}-scm.toml"));
        fs::write(&config, body).unwrap();
        let err =
            build_plan_from_config(&config, &ScmPlanOverrides::default(), "test").unwrap_err();
        out.push_str(&format!("{label}\n  {err:#}\n"));
    }
    let err = build_plan_from_config(
        &dir.path().join("absent-scm.toml"),
        &ScmPlanOverrides::default(),
        "test",
    )
    .unwrap_err();
    out.push_str(&format!("missing_config_file\n  {err:#}\n"));

    snapshot_settings(dir.path()).bind(|| assert_snapshot!(out));
}

/// Snapshot 9: Re-planning over a paused SCM process with SCM-defining changes: the
/// rendering shows where the process got to, what changed, and that the
/// state cannot resume under the new plan.
#[test]
fn replan_over_a_paused_process_with_scm_defining_changes() {
    let dir = tempfile::tempdir().unwrap();
    let model = write_template(dir.path());
    let previous = build_plan(
        &model,
        &names(&["WT_CL", "CRCL_CL"]),
        None,
        ScmOptions::default(),
        "test",
    )
    .unwrap()
    .plan;
    previous.save().unwrap();
    mid_scm_state(&previous)
        .save(&previous.out_dir_path())
        .unwrap();

    let built = build_plan(
        &model,
        &names(&["WT_CL", "CRCL_CL", "WT_V"]),
        None,
        ScmOptions {
            forward_alpha: 0.01,
            num_rounds: Some(2),
            ..Default::default()
        },
        "test",
    )
    .unwrap();

    snapshot_settings(dir.path()).bind(|| assert_snapshot!(built.render_text()));
}

/// Snapshot 9b: Re-planning over a paused SCM process without a candidate
/// that never won: the rendering says the SCM process carries on without it.
/// Then the same without a candidate that did win, which it cannot.
#[test]
fn replan_removing_candidates_over_a_paused_process() {
    let dir = tempfile::tempdir().unwrap();
    let model = write_template(dir.path());
    let previous = build_plan(
        &model,
        &names(&["WT_CL", "CRCL_CL", "WT_V"]),
        None,
        ScmOptions::default(),
        "test",
    )
    .unwrap()
    .plan;
    previous.save().unwrap();
    mid_scm_state(&previous)
        .save(&previous.out_dir_path())
        .unwrap();

    let loser_gone = build_plan(
        &model,
        &names(&["WT_CL", "CRCL_CL"]),
        None,
        ScmOptions::default(),
        "test",
    )
    .unwrap();
    let winner_gone = build_plan(
        &model,
        &names(&["CRCL_CL", "WT_V"]),
        None,
        ScmOptions::default(),
        "test",
    )
    .unwrap();

    snapshot_settings(dir.path()).bind(|| {
        assert_snapshot!("replan_removing_a_loser", loser_gone.render_text());
        assert_snapshot!("replan_removing_a_winner", winner_gone.render_text());
    });
}

// ---------------------------------------------------------------------------
// Failure shapes
// ---------------------------------------------------------------------------

/// Snapshot 10: How the driver classifies every kind of run a fit can leave behind.
#[test]
fn fit_outcome_for_every_kind_of_run() {
    let dir = tempfile::tempdir().unwrap();
    let fits = [
        Fit::Succeeded(990.0),
        Fit::SucceededWithWarnings(990.0),
        Fit::MinimizationTerminated(990.0),
        Fit::Aborted(700.0),
        Fit::AbortedHeaderless,
        Fit::NoFinalRow,
        Fit::Terminated,
        Fit::StillRunning,
    ];

    let mut out = String::new();
    for (i, fit) in fits.into_iter().enumerate() {
        let model =
            write_named_template(&dir.path().join(format!("case{i}")), "1001.mod", TEMPLATE);
        write_fit_output(&model, fit).unwrap();
        let outcome = read_fit_outcome(&model).unwrap();
        out.push_str(&format!(
            "## {fit:?}\nlabel: {}\nusable: {}\n{outcome:#?}\n\n",
            outcome.label(),
            outcome.usable()
        ));
    }
    let model = write_named_template(&dir.path().join("never_started"), "1001.mod", TEMPLATE);
    let outcome = read_fit_outcome(&model).unwrap();
    out.push_str(&format!(
        "## NeverStarted\nlabel: {}\nusable: {}\n{outcome:#?}\n",
        outcome.label(),
        outcome.usable()
    ));

    snapshot_settings(dir.path()).bind(|| assert_snapshot!(out));
}

// ---------------------------------------------------------------------------
// Records and renderings
// ---------------------------------------------------------------------------

/// A plan on disk plus a fabricated state with a reference fit and one
/// forward round in flight: WT_CL scored after a retry, CRCL_CL still
/// running, WT_V not yet dispatched.
fn fabricate_running_scm(dir: &Path) -> PathBuf {
    let plan = make_plan(dir, ScmOptions::default());
    plan.save().unwrap();
    let out_dir = plan.out_dir_path();

    let mut state = ScmState::new(&plan);
    state.status = super::ScmRunStatus::Running;
    state.phase = Some(Direction::Forward);
    state.reference_model = Some("base/1001_base.mod".into());
    state.reference_ofv = Some(1000.0);

    let mut base = CandidateRecord::new("base", "fit base model".into(), 0);
    base.model = "base/1001_base.mod".into();
    base.attempts.push(AttemptRecord {
        model: "base/1001_base.mod".into(),
        outcome: "succeeded".into(),
    });
    base.status = CandidateStatus::Succeeded;
    base.ofv = Some(1000.0);
    state.rounds.push(RoundRecord {
        name: "reference".into(),
        direction: Direction::Forward,
        reference_model: "-".into(),
        reference_ofv: None,
        candidates: vec![base],
        winner: None,
        decision: "base model fitted (OFV 1000.000)".into(),
        complete: true,
    });

    let mut wt_cl = CandidateRecord::new("WT_CL", "add WT_CL".into(), 1);
    wt_cl.model = "forward_round1/1001_wt_cl_try2.mod".into();
    wt_cl.attempts = vec![
        AttemptRecord {
            model: "forward_round1/1001_wt_cl.mod".into(),
            outcome: "no ofv".into(),
        },
        AttemptRecord {
            model: "forward_round1/1001_wt_cl_try2.mod".into(),
            outcome: "succeeded".into(),
        },
    ];
    wt_cl.status = CandidateStatus::Succeeded;
    wt_cl.ofv = Some(980.0);
    wt_cl.heuristics = vec!["parameter near boundary".into()];

    let mut crcl = CandidateRecord::new("CRCL_CL", "add CRCL_CL".into(), 1);
    crcl.model = "forward_round1/1001_crcl_cl.mod".into();
    crcl.status = CandidateStatus::Running;
    let running_model = out_dir.join(&crcl.model);
    fs::create_dir_all(running_model.parent().unwrap()).unwrap();
    fs::write(&running_model, TEMPLATE).unwrap();
    write_fit_output(&running_model, Fit::StillRunning).unwrap();

    let wt_v = CandidateRecord::new("WT_V", "add WT_V".into(), 1);

    state.rounds.push(RoundRecord {
        name: "forward_round1".into(),
        direction: Direction::Forward,
        reference_model: "base/1001_base.mod".into(),
        reference_ofv: Some(1000.0),
        candidates: vec![wt_cl, crcl, wt_v],
        winner: None,
        decision: String::new(),
        complete: false,
    });
    state.save(&out_dir).unwrap();
    out_dir
}

/// Snapshot 11: `scm status` across every state an SCM process can be found in.
#[test]
fn status_rendering_across_states() {
    // planned: a plan on disk, no state
    {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        plan.save().unwrap();
        let status = read_status(&plan.out_dir_path()).unwrap();
        snapshot_settings(dir.path())
            .bind(|| assert_snapshot!("status_planned", status.render_text()));
    }
    // running: mid-round, one retry behind it, one model still running
    {
        let dir = tempfile::tempdir().unwrap();
        let out_dir = fabricate_running_scm(dir.path());
        let status = read_status(&out_dir).unwrap();
        snapshot_settings(dir.path())
            .bind(|| assert_snapshot!("status_running_mid_round", status.render_text()));
    }
    // paused by num_rounds
    {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(
            dir.path(),
            ScmOptions {
                num_rounds: Some(1),
                ..Default::default()
            },
        );
        run_scm(&plan, &full_scm_executor(), None).unwrap();
        let status = read_status(&plan.out_dir_path()).unwrap();
        snapshot_settings(dir.path())
            .bind(|| assert_snapshot!("status_paused_num_rounds", status.render_text()));
    }
    // completed with a covariate retained
    {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        run_scm(&plan, &full_scm_executor(), None).unwrap();
        let status = read_status(&plan.out_dir_path()).unwrap();
        snapshot_settings(dir.path())
            .bind(|| assert_snapshot!("status_completed_retained", status.render_text()));
    }
    // completed with nothing retained
    {
        let dir = tempfile::tempdir().unwrap();
        let plan = forward_only_plan(dir.path());
        run_scm(&plan, &nothing_significant_executor(), None).unwrap();
        let status = read_status(&plan.out_dir_path()).unwrap();
        snapshot_settings(dir.path())
            .bind(|| assert_snapshot!("status_completed_nothing_retained", status.render_text()));
    }
    // completed with an unusable candidate
    {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(
            dir.path(),
            ScmOptions {
                direction: vec![Direction::Forward],
                max_retries: 1,
                ..Default::default()
            },
        );
        run_scm(&plan, &unusable_candidate_executor(), None).unwrap();
        let status = read_status(&plan.out_dir_path()).unwrap();
        snapshot_settings(dir.path())
            .bind(|| assert_snapshot!("status_completed_with_unusable", status.render_text()));
    }
    // failed: the reference fit never succeeded
    {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(
            dir.path(),
            ScmOptions {
                direction: vec![Direction::Forward],
                max_retries: 1,
                ..Default::default()
            },
        );
        run_scm(&plan, &failing_reference_executor(), None).unwrap_err();
        let status = read_status(&plan.out_dir_path()).unwrap();
        snapshot_settings(dir.path())
            .bind(|| assert_snapshot!("status_failed", status.render_text()));
    }
}

/// Snapshot 12: `scm summary` renderings. A single round in progress and
/// then complete — with a retried winner, a not-significant candidate and
/// an unusable one (p-values on both sides of the 0.001 formatting switch),
/// and a running candidate that has no attempt recorded yet — plus the
/// reference round on its own.
#[test]
fn summary_rendering_of_a_single_round() {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = fabricate_running_scm(dir.path());
    let one = |round: &str| SummaryOptions {
        round: Some(round.to_string()),
        ..Default::default()
    };

    // in progress, as fabricated
    let in_progress = read_summary(&out_dir)
        .unwrap()
        .render_text(&one("1"))
        .unwrap();

    // now conclude it: WT_CL wins, CRCL_CL not significant, WT_V unusable
    let mut state = ScmState::load(&out_dir).unwrap().unwrap();
    {
        let round = state.find_round_mut("forward_round1").unwrap();
        let wt_cl = &mut round.candidates[0];
        wt_cl.delta_ofv = Some(-20.0);
        wt_cl.p_value = Some(7.744e-6);
        wt_cl.significant = Some(true);
        wt_cl.selected = true;

        let crcl = &mut round.candidates[1];
        crcl.attempts.push(AttemptRecord {
            model: "forward_round1/1001_crcl_cl.mod".into(),
            outcome: "succeeded".into(),
        });
        crcl.status = CandidateStatus::Succeeded;
        crcl.ofv = Some(996.0);
        crcl.delta_ofv = Some(-4.0);
        crcl.p_value = Some(0.0455);
        crcl.significant = Some(false);

        let wt_v = &mut round.candidates[2];
        wt_v.model = "forward_round1/1001_wt_v_try4.mod".into();
        wt_v.attempts = ["", "_try2", "_try3", "_try4"]
            .iter()
            .map(|suffix| AttemptRecord {
                model: format!("forward_round1/1001_wt_v{suffix}.mod"),
                outcome: "program aborted".into(),
            })
            .collect();
        wt_v.status = CandidateStatus::Unusable;
        wt_v.heuristics = vec!["program aborted".into()];

        round.winner = Some("WT_CL".into());
        round.decision = "added WT_CL (p = 7.744e-6, dOFV = -20.000)".into();
        round.complete = true;
    }
    state.retained = vec!["WT_CL".into()];
    state.save(&out_dir).unwrap();
    let summary = read_summary(&out_dir).unwrap();
    let complete = summary.render_text(&one("forward_round1")).unwrap();
    let complete_long = summary
        .render_text(&SummaryOptions {
            long: true,
            files: true,
            ..one("forward_round1")
        })
        .unwrap();
    let reference = summary.render_text(&one("reference")).unwrap();

    snapshot_settings(dir.path()).bind(|| {
        assert_snapshot!("summary_round_in_progress", in_progress);
        assert_snapshot!("summary_round_complete", complete);
        assert_snapshot!("summary_round_complete_long_files", complete_long);
        assert_snapshot!("summary_round_reference", reference);
    });
}

/// Snapshot 12b: `scm summary` over a completed forward -> backward run: the
/// default all-rounds view, the long view with every attempt, the timing
/// view (blank clocks: mocked runs carry no timestamps), the parameter
/// drift view, the candidate × round matrix in both values, one candidate
/// traced, a phase alone, and the markdown and CSV formats.
#[test]
fn summary_rendering_of_a_completed_run() {
    let dir = tempfile::tempdir().unwrap();
    let plan = make_plan(dir.path(), ScmOptions::default());
    run_scm(&plan, &full_scm_executor(), None).unwrap();
    let summary = read_summary(&plan.out_dir_path()).unwrap();
    let render = |opts: SummaryOptions| summary.render(&opts).unwrap();

    snapshot_settings(dir.path()).bind(|| {
        assert_snapshot!("summary_default", render(SummaryOptions::default()));
        assert_snapshot!(
            "summary_long_all",
            render(SummaryOptions {
                long: true,
                all: true,
                ..Default::default()
            })
        );
        assert_snapshot!(
            "summary_time",
            render(SummaryOptions {
                time: true,
                ..Default::default()
            })
        );
        assert_snapshot!(
            "summary_parameters",
            render(SummaryOptions {
                parameters: true,
                round: Some("forward_round1".into()),
                ..Default::default()
            })
        );
        assert_snapshot!(
            "summary_matrix_p",
            render(SummaryOptions {
                matrix: Some(MatrixValue::P),
                ..Default::default()
            })
        );
        assert_snapshot!(
            "summary_matrix_dofv",
            render(SummaryOptions {
                matrix: Some(MatrixValue::Dofv),
                ..Default::default()
            })
        );
        assert_snapshot!(
            "summary_candidate_trace",
            render(SummaryOptions {
                candidate: Some("WT_V".into()),
                ..Default::default()
            })
        );
        assert_snapshot!(
            "summary_backward_phase_by_name",
            render(SummaryOptions {
                phase: Some(Direction::Backward),
                sort: super::SortKey::Name,
                ..Default::default()
            })
        );
        assert_snapshot!(
            "summary_markdown",
            render(SummaryOptions {
                format: super::SummaryFormat::Markdown,
                ..Default::default()
            })
        );
        assert_snapshot!(
            "summary_csv",
            render(SummaryOptions {
                format: super::SummaryFormat::Csv,
                ..Default::default()
            })
        );
        assert_snapshot!(
            "scm_summary_json",
            read(&plan.out_dir_path().join(SCM_SUMMARY_FILENAME))
        );
    });
}

/// Snapshot 13: The round summary (JSON and markdown) a round leaves in its own
/// directory while the SCM process is still under way.
#[test]
fn round_summary_files_for_a_mid_run_round() {
    let dir = tempfile::tempdir().unwrap();
    let plan = make_plan(
        dir.path(),
        ScmOptions {
            num_rounds: Some(1),
            ..Default::default()
        },
    );
    run_scm(&plan, &full_scm_executor(), None).unwrap();
    let round_dir = plan.out_dir_path().join("forward_round1");

    snapshot_settings(dir.path()).bind(|| {
        assert_snapshot!(
            "round_summary_json",
            read(&round_dir.join(ROUND_SUMMARY_JSON))
        );
        assert_snapshot!("round_summary_md", read(&round_dir.join(ROUND_SUMMARY_MD)));
    });
}

/// Snapshot 14: The chi-square survival function on a grid, and the LRT in both
/// directions — the numbers every p-value in the SCM comes from.
#[test]
fn chi_square_and_lrt_reference_values() {
    let mut out = String::from("statistic  df  p\n");
    for x in [0.5, 1.0, 2.0, 3.841, 5.0, 6.635, 10.0, 10.828, 20.0, 50.0] {
        for df in 1..=4 {
            out.push_str(&format!("{x:>9.3}  {df}   {:.6e}\n", chi2_sf(x, df)));
        }
    }
    out.push_str("\nlrt (reference, candidate, df, direction)\n");
    for (r, c, df, dir) in [
        (1000.0, 990.0, 1, Direction::Forward),
        (1000.0, 1005.0, 1, Direction::Forward),
        (1000.0, 990.0, 2, Direction::Forward),
        (1000.0, 1015.0, 1, Direction::Backward),
        (1000.0, 1000.5, 1, Direction::Backward),
        (1000.0, 995.0, 1, Direction::Backward),
    ] {
        out.push_str(&format!("{r} {c} {df} {dir}: {:?}\n", lrt(r, c, df, dir)));
    }
    assert_snapshot!(out);
}

// ---------------------------------------------------------------------------
// Driver transcripts
// ---------------------------------------------------------------------------

/// Snapshot 15: A full forward-then-backward run: fits dispatched, files written,
/// final state and decision log.
#[test]
fn transcript_full_forward_backward_run() {
    let dir = tempfile::tempdir().unwrap();
    let plan = make_plan(dir.path(), ScmOptions::default());
    let executor = full_scm_executor();
    run_scm(&plan, &executor, None).unwrap();

    snapshot_settings(dir.path()).bind(|| assert_snapshot!(transcript(&plan, &executor)));
}

/// Snapshot 16: A candidate that never produces a usable fit burns its retries,
/// concludes unusable, and is reported rather than scored.
#[test]
fn transcript_unusable_candidate_after_retries() {
    let dir = tempfile::tempdir().unwrap();
    let plan = make_plan(
        dir.path(),
        ScmOptions {
            direction: vec![Direction::Forward],
            max_retries: 1,
            ..Default::default()
        },
    );
    let executor = unusable_candidate_executor();
    run_scm(&plan, &executor, None).unwrap();

    snapshot_settings(dir.path()).bind(|| assert_snapshot!(transcript(&plan, &executor)));
}

/// Snapshot 17: Every candidate in the round is unusable: nothing was scored, yet
/// the round has to conclude somehow. The decision it records is what this
/// snapshot pins down.
#[test]
fn transcript_every_candidate_unusable() {
    let dir = tempfile::tempdir().unwrap();
    let plan = make_plan(
        dir.path(),
        ScmOptions {
            direction: vec![Direction::Forward],
            max_retries: 1,
            ..Default::default()
        },
    );
    let executor = everything_unusable_executor();
    let result = run_scm(&plan, &executor, None);
    let header = match &result {
        Ok(outcome) => format!("run_scm: Ok, status {}\n\n", outcome.state.status),
        Err(e) => format!("run_scm: Err: {e:#}\n\n"),
    };

    snapshot_settings(dir.path())
        .bind(|| assert_snapshot!(format!("{header}{}", transcript(&plan, &executor))));
}

/// Snapshot 18: The reference fit fails every attempt, so the SCM process cannot
/// start: the error, the failed state, and the records left for the
/// reference round.
#[test]
fn transcript_reference_fit_fails_every_attempt() {
    let dir = tempfile::tempdir().unwrap();
    let plan = make_plan(
        dir.path(),
        ScmOptions {
            direction: vec![Direction::Forward],
            max_retries: 1,
            ..Default::default()
        },
    );
    let executor = failing_reference_executor();
    let err = run_scm(&plan, &executor, None).unwrap_err();

    snapshot_settings(dir.path()).bind(|| {
        assert_snapshot!(format!(
            "error: {err:#}\n\n{}",
            transcript(&plan, &executor)
        ))
    });
}

/// Snapshot 20: A candidate removed between rounds: the resumed run refits
/// nothing, the roster records the removal, and the removed candidate's
/// round-1 files stay where they were.
#[test]
fn transcript_candidate_removed_between_rounds() {
    let dir = tempfile::tempdir().unwrap();
    let plan = make_plan(
        dir.path(),
        ScmOptions {
            num_rounds: Some(1),
            ..Default::default()
        },
    );
    let executor = full_scm_executor();
    run_scm(&plan, &executor, None).unwrap();

    let fewer = build_plan(
        &plan.model_path(),
        &names(&["WT_CL", "CRCL_CL"]),
        None,
        ScmOptions::default(),
        "test",
    )
    .unwrap()
    .plan;
    run_scm(&fewer, &executor, None).unwrap();
    let status = read_status(&plan.out_dir_path()).unwrap();

    snapshot_settings(dir.path()).bind(|| {
        assert_snapshot!(format!(
            "{}\n# scm status\n{}",
            transcript(&fewer, &executor),
            status.render_text()
        ))
    });
}

/// Snapshot 19: Running a changed plan into an out_dir holding another plan's state
/// is refused; with overwrite the SCM-owned output is cleared and nothing
/// else is touched. The second run fails at its first fit so the tree
/// shows the cleared out_dir rather than a fresh run's output.
#[test]
fn transcript_mismatched_plan_refused_then_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let plan = make_plan(dir.path(), ScmOptions::default());
    run_scm(&plan, &full_scm_executor(), None).unwrap();
    let out_dir = plan.out_dir_path();

    // Files the SCM does not own, planted where the clearing runs.
    fs::create_dir_all(out_dir.join("notes")).unwrap();
    fs::write(out_dir.join("notes/readme.txt"), "mine\n").unwrap();
    fs::create_dir_all(out_dir.join("forward_roundX")).unwrap();
    fs::write(out_dir.join("forward_roundX/keep.txt"), "not a round\n").unwrap();
    let before = file_tree(&out_dir);

    let mut changed = plan.clone();
    changed.options.forward_alpha = 0.01;
    let refused = run_scm(&changed, &full_scm_executor(), None).unwrap_err();

    changed.options.overwrite = true;
    let failing = MockExecutor::new(0.0).failing_with("sbatch: error: Batch job submission failed");
    let failed = run_scm(&changed, &failing, None).unwrap_err();
    let after = file_tree(&out_dir);

    let mut out =
        format!("# refused without overwrite\n{refused:#}\n\n# out_dir before overwrite\n");
    for f in before {
        out.push_str(&f);
        out.push('\n');
    }
    out.push_str(&format!(
        "\n# overwrite run (fails at its first fit)\n{failed:#}\n\n# out_dir after overwrite\n"
    ));
    for f in after {
        out.push_str(&f);
        out.push('\n');
    }
    out.push_str(&format!(
        "\n# scm_state.json after overwrite\n{}",
        read(&ScmState::state_path(&out_dir))
    ));

    snapshot_settings(dir.path()).bind(|| assert_snapshot!(out));
}
