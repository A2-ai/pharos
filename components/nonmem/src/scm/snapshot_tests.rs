//! Snapshot tests for the SCM module.
//!
//! Every SCM snapshot lives here, in one module, so the files all land in
//! `src/scm/snapshots/` — apart from the output-file and parser snapshots
//! elsewhere in the workspace. Each test binds
//! [`test_support::snapshot_settings`], which redacts timestamps, the temp
//! dir and the plan digest, so the snapshots are stable across machines.

use std::path::Path;

use fs_err as fs;
use insta::assert_snapshot;

use super::config::{ScmOverrides, build_plan_from_config, init_scm};
use super::round::{ModelWriter, RETRY_JITTER, read_fit_outcome};
use super::state::{AttemptRecord, CandidateStatus, Pause, ScmRunStatus, ScmState};
use super::test_support::*;
use super::{
    self as scm, CovariateRequest, Covariates, Direction, SCM_SUMMARY_FILENAME, SCM_SUMMARY_MD,
    STATE_FILENAME, ScmOptions, ScmStatus, SummaryOptions, TypeDefaults, read_summary,
    render_gitignore, run_scm, sanitize_name,
};
use crate::run::metadata::{RUN_START_FILENAME, RunStartFile};
use ::config::{GitTracking, ScmSettings};

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

/// One round-1 model per template variant under
/// `test_data/scm/templates/`. A template that cannot be planned snapshots
/// its planning error instead.
#[test]
fn generated_models_for_every_template_variant() {
    insta::glob!("../../test_data/scm/templates", "*.mod", |path| {
        let dir = tempfile::tempdir().unwrap();
        let content = read(path);
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();
        let template = write_named_template(dir.path(), &file_name, &content);
        let candidates = declared_candidates(&content)
            .unwrap_or_else(|| Covariates::named(&["WT_CL", "CRCL_CL", "WT_V"]));

        snapshot_settings(dir.path()).bind(|| {
            let built = match try_plan(&template, &candidates, None, opts_cov_on()) {
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
                &plan.candidates,
                &round1,
                &[first.theta],
                None,
                true,
            );
            assert_snapshot!("round1", model_or_error(result, &round1));
        });
    });
}

/// A round-2 model warm-started from the round-1 winner's fit: the
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
    write_scm_model(&template, &plan.candidates, &winner, &[4], None, true).unwrap();
    write_fit_output(&winner, Fit::Succeeded(980.0)).unwrap();

    let dest = plan
        .out_dir_path()
        .join("forward_round2")
        .join("1001_crcl_cl.mod");
    write_scm_model(
        &template,
        &plan.candidates,
        &dest,
        &[4, 5],
        Some(&super::round::ext_path_for(&winner, &Default::default()).unwrap()),
        true,
    )
    .unwrap();

    snapshot_settings(dir.path()).bind(|| assert_snapshot!(read(&dest)));
}

/// A retry model continues from wherever the failed attempt stopped:
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
    write_scm_model(&template, &plan.candidates, &first, &[4], None, true).unwrap();
    write_fit_output(&first, Fit::NoFinalRow).unwrap();

    let retry = first.with_file_name("1001_wt_cl_try2.mod");
    ModelWriter {
        template: &first,
        candidates: &[],
        with_metadata: false,
        retry_jitter: RETRY_JITTER,
    }
    .retry(
        &first,
        &retry,
        "SCM forward_round1: add WT_CL (attempt 2)",
        None,
        &Default::default(),
    )
    .unwrap();

    snapshot_settings(dir.path()).bind(|| assert_snapshot!(read(&retry)));
}

// ---------------------------------------------------------------------------
// Config and plan
// ---------------------------------------------------------------------------

/// The starter config `scm init` writes beside a model.
#[test]
fn init_writes_the_starter_config() {
    let dir = tempfile::tempdir().unwrap();
    let model = write_template(dir.path());
    let init = init_scm(&model, false).unwrap();

    snapshot_settings(dir.path()).bind(|| assert_snapshot!(read(&init.config_path)));
}

/// `scm init` writes the out_dir's `.gitignore` at the project's level
/// (`milestones` by default); every submit and the driver rewrite it, so a
/// changed `pharos.toml` takes effect at the next command.
#[test]
fn init_and_submit_write_the_gitignore_for_the_projects_level() {
    let dir = tempfile::tempdir().unwrap();
    let model = write_template(dir.path());
    let init = init_scm(&model, false).unwrap();
    let gitignore = init.out_dir.join(".gitignore");
    assert_eq!(
        read(&gitignore),
        render_gitignore("1001", GitTracking::Milestones)
    );

    let mut scm = ScmSettings::default();
    scm.set_track_in_git(Some(GitTracking::Final));
    write_project_config_with(dir.path(), TEMPLATE_DIALECT, scm);
    let filled = read(&init.config_path).replace("effects = []", "effects = [\"WT_CL\"]");
    fs::write(&init.config_path, filled).unwrap();
    let built =
        build_plan_from_config(&init.config_path, &ScmOverrides::default(), "0.0.0").unwrap();
    built.write_gitignore().unwrap();
    assert_eq!(
        read(&gitignore),
        render_gitignore("1001", GitTracking::Final)
    );
}

/// The out_dir's `.gitignore` at each `[nonmem.scm] track_in_git` level.
#[test]
fn gitignore_for_each_tracking_level() {
    for level in [
        GitTracking::Final,
        GitTracking::Milestones,
        GitTracking::All,
    ] {
        insta::with_settings!({ snapshot_suffix => level.as_str() }, {
            assert_snapshot!(render_gitignore("1001", level));
        });
    }
}

/// The plan rendering (with its warnings) for the defaults, a bounded
/// request, and a fold-change effect with per-row values. (The plan as
/// `scm_state.json` records it is in the full-run transcript.)
#[test]
fn plan_text_for_the_main_option_sets() {
    // WT_V written as a fold-change effect, held out at 1 and released at 1.5;
    // CRCL_CL given its own initial by its row.
    let fold_change = TEMPLATE
        .replace("WT_V = (WT/70)**THETA(6)", "WT_V = THETA(6)**(WT/70)")
        .replace("$THETA (0 FIX)   ; WT_V cov", "$THETA (1 FIX)   ; WT_V cov");
    let std = Covariates::named(&["WT_CL", "CRCL_CL", "WT_V"]);
    let rows = Covariates {
        continuous: TypeDefaults { initial: Some(0.2) },
        categorical: TypeDefaults { initial: Some(1.5) },
        fixed: Some(0.0),
        effects: vec![
            req("WT_CL"),
            req("CRCL_CL").initial(0.3),
            // categorical, so it takes the categorical table's initial
            req("WT_V").categorical().fixed(1.0),
        ],
    };
    // Bounds from a row, and left off entirely.
    let bounded = covs(vec![
        req("WT_CL").bounds(Some(0.0), None),
        req("CRCL_CL")
            .initial(1.2)
            .fixed(1.0)
            .bounds(Some(0.01), Some(10.0)),
        req("WT_V").bounds(None, Some(2.0)),
    ]);
    let cases: Vec<(&str, ScmOptions, &str, &Covariates)> = vec![
        ("defaults", ScmOptions::default(), TEMPLATE, &std),
        ("bounded", ScmOptions::default(), TEMPLATE, &bounded),
        ("fold_change", opts_cov_on(), &fold_change, &rows),
    ];

    for (label, options, template, covariates) in cases {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template_content(dir.path(), template);
        let built = try_plan(&model, covariates, None, options).unwrap();

        let mut text = built.render_text();
        for w in &built.warnings {
            text.push_str(&format!("warning: {w}\n"));
        }

        snapshot_settings(dir.path()).bind(|| assert_snapshot!(format!("plan_text_{label}"), text));
    }
}

/// Every way `build_plan` refuses a request, including the option
/// validation it runs first. One snapshot, one labelled message per case.
#[test]
fn every_build_plan_error_message() {
    let dir = tempfile::tempdir().unwrap();
    let std_names = Covariates::named(&["WT_CL", "CRCL_CL", "WT_V"]);
    let no_est = TEMPLATE.replace("$ESTIMATION METHOD=1 INTER MAXEVAL=9999 NOABORT\n", "");
    // The same name reachable on two different thetas.
    let ambiguous = TEMPLATE.replace("; TVKA (1/h)", "; WT_V cov");
    // One comment covering an xN repeat names every theta it expands to.
    let repeated = TEMPLATE.replace(
        "$THETA (0 FIX)   ; WT_CL cov\n$THETA (0 FIX)   ; CRCL_CL cov\n$THETA (0 FIX)   ; WT_V cov",
        "$THETA (0 FIX)x3   ; WT_CL cov",
    );
    // Spellings that name no theta: no comment at all, a Type1 comment
    // leading with a number, a comment no dialect parses, and NM-TRAN's own
    // label syntax, which no pharos command reads as a parameter name.
    let bare = TEMPLATE.replace("$THETA (0 FIX)   ; WT_V cov", "$THETA (0 FIX)");
    let numbered = TEMPLATE.replace("; WT_V cov", "; 6 WT_V cov");
    let unparsed = TEMPLATE.replace("; WT_V cov", "; WT_V covv");
    let labelled = TEMPLATE.replace("$THETA (0 FIX)   ; WT_V cov", "$THETA WT_V=(0 FIX)");
    let names_record =
        TEMPLATE.replace("$THETA (0 FIX)   ; WT_V cov", "$THETA NAMES(WT_V) (0 FIX)");
    // Not one `$THETA` record carries a comment naming it.
    let unnamed = TEMPLATE
        .replace("   ; TVCL (L/h)", "")
        .replace("   ; TVV (L)", "")
        .replace("  ; TVKA (1/h)", "")
        .replace("   ; WT_CL cov", "")
        .replace("   ; CRCL_CL cov", "")
        .replace("   ; WT_V cov", "");
    let opts = |f: fn(&mut ScmOptions)| {
        let mut o = ScmOptions::default();
        f(&mut o);
        o
    };

    // WT_CL alone, its row carrying the values given.
    let row = |r: CovariateRequest| covs(vec![r]);
    let defaults = |initial: f64, fixed: f64| Covariates {
        continuous: TypeDefaults {
            initial: Some(initial),
        },
        fixed: Some(fixed),
        effects: vec![req("WT_CL")],
        ..Default::default()
    };

    // (label, template content, covariates, options)
    let d = ScmOptions::default;
    let n = Covariates::named;
    #[rustfmt::skip]
    let cases: Vec<(&str, &str, Covariates, ScmOptions)> = vec![
        ("unknown_name", TEMPLATE, n(&["AGE_CL"]), d()),
        ("ambiguous_name", &ambiguous, n(&["WT_V"]), d()),
        ("model_names_no_theta", &unnamed, n(&["WT_CL"]), d()),
        ("repeat_spec_shares_one_comment", &repeated, n(&["WT_CL"]), d()),
        ("no_comment", &bare, n(&["WT_V"]), d()),
        ("type1_comment_leading_with_a_number", &numbered, n(&["WT_V"]), d()),
        ("comment_no_dialect_parses", &unparsed, n(&["WT_V"]), d()),
        ("nmtran_label", &labelled, n(&["WT_V"]), d()),
        ("nmtran_names_record", &names_record, n(&["WT_V"]), d()),
        ("duplicate_names", TEMPLATE, n(&["WT_CL", "wt_cl"]), d()),
        ("empty_name", TEMPLATE, n(&["  "]), d()),
        ("empty_list", TEMPLATE, Covariates::default(), d()),
        ("no_estimation_record", &no_est, std_names.clone(), d()),
        ("direction_empty", TEMPLATE, std_names.clone(), opts(|o| o.direction = vec![])),
        ("direction_duplicated", TEMPLATE, std_names.clone(), opts(|o| o.direction = vec![Direction::Forward; 2])),
        ("forward_alpha_zero", TEMPLATE, std_names.clone(), opts(|o| o.forward_alpha = 0.0)),
        ("backward_alpha_one", TEMPLATE, std_names.clone(), opts(|o| o.backward_alpha = 1.0)),
        ("retry_jitter_one", TEMPLATE, std_names.clone(), opts(|o| o.retry_jitter = 1.0)),
        ("initial_nan", TEMPLATE, row(req("WT_CL").initial(f64::NAN)), d()),
        ("off_infinite", TEMPLATE, defaults(0.1, f64::INFINITY), d()),
        ("lower_above_upper_defaults", TEMPLATE, row(req("WT_CL").bounds(Some(1.0), Some(0.0))), d()),
        ("initial_at_lower", TEMPLATE, row(req("WT_CL").initial(0.1).bounds(Some(0.1), None)), d()),
        ("initial_above_upper", TEMPLATE, row(req("WT_CL").initial(0.5).bounds(None, Some(0.2))), d()),
    ];

    let mut out = String::new();
    for (label, template, covariates, options) in cases {
        let model = write_template_content(&dir.path().join(label), template);
        let err = try_plan(&model, &covariates, None, options).unwrap_err();
        out.push_str(&format!("{label}\n  {err:#}\n"));
    }

    // Cases about the files rather than their content.
    let missing_data_dir = dir.path().join("missing_dataset");
    fs::create_dir_all(&missing_data_dir).unwrap();
    let model = missing_data_dir.join("1001.mod");
    fs::write(&model, TEMPLATE).unwrap();
    let err = try_plan(&model, &std_names, None, ScmOptions::default()).unwrap_err();
    out.push_str(&format!("missing_dataset\n  {err:#}\n"));

    let err = try_plan(
        &dir.path().join("nope.mod"),
        &std_names,
        None,
        ScmOptions::default(),
    )
    .unwrap_err();
    out.push_str(&format!("missing_model\n  {err:#}\n"));

    let txt = write_named_template(&dir.path().join("bad_extension"), "1001.txt", TEMPLATE);
    let err = try_plan(&txt, &std_names, None, ScmOptions::default()).unwrap_err();
    out.push_str(&format!("bad_extension\n  {err:#}\n"));

    snapshot_settings(dir.path()).bind(|| assert_snapshot!(out));
}

/// Every way the config loader refuses a `<stem>scm.toml`.
#[test]
fn every_config_error_message() {
    let dir = tempfile::tempdir().unwrap();
    write_template(dir.path());

    // A config with the model and direction lines in place, then `body`.
    let cov = |body: &str| format!("model = \"1001.mod\"\ndirection = [\"forward\"]\n{body}");
    const EFFECTS: &str = "[covariates]\neffects = [\"WT_CL\"]\n";
    #[rustfmt::skip]
    let cases: Vec<(&str, String)> = vec![
        ("unknown_key", cov(&format!("foward_alpha = 0.01\n{EFFECTS}"))),
        ("theta_numbers", cov("[covariates]\neffects = [4, 5, 6]\n")),
        ("row_without_name", cov("[covariates]\neffects = [{ initial = 0.2 }]\n")),
        ("row_with_unknown_key", cov("[covariates]\neffects = [{ name = \"WT_CL\", init = 0.2 }]\n")),
        ("section_with_unknown_key", cov("[covariates]\ninital = 0.2\neffects = [\"WT_CL\"]\n")),
        ("missing_covariates", cov("")),
        ("missing_direction", format!("model = \"1001.mod\"\n{EFFECTS}")),
        ("missing_model", format!("direction = [\"forward\"]\n{EFFECTS}")),
        ("malformed_toml", "model = \ndirection = [\"forward\"]\n".to_string()),
        ("misspelled_direction", format!("model = \"1001.mod\"\ndirection = [\"foward\"]\n{EFFECTS}")),
        ("unresolvable_model", format!("model = \"nope/1001.mod\"\ndirection = [\"forward\"]\n{EFFECTS}")),
        ("empty_effects", cov("[covariates]\neffects = []\n")),
    ];

    let mut out = String::new();
    for (label, body) in cases {
        let config = dir.path().join(format!("{label}scm.toml"));
        fs::write(&config, body).unwrap();
        let err = build_plan_from_config(&config, &ScmOverrides::default(), "test").unwrap_err();
        out.push_str(&format!("{label}\n  {err:#}\n"));
    }
    let err = build_plan_from_config(
        &dir.path().join("absentscm.toml"),
        &ScmOverrides::default(),
        "test",
    )
    .unwrap_err();
    out.push_str(&format!("missing_config_file\n  {err:#}\n"));

    snapshot_settings(dir.path()).bind(|| assert_snapshot!(out));
}

/// Re-planning over a paused SCM process with SCM-defining changes: the
/// rendering shows where the process got to, what changed, and that the
/// state cannot resume under the new plan.
#[test]
fn replan_over_a_paused_process_with_scm_defining_changes() {
    let dir = tempfile::tempdir().unwrap();
    let model = write_template(dir.path());
    let previous = plan_of(&model, &["WT_CL", "CRCL_CL"], None, ScmOptions::default());
    mid_scm_state(&previous)
        .save(&previous.out_dir_path())
        .unwrap();

    let built = plan_named(
        &model,
        &["WT_CL", "CRCL_CL", "WT_V"],
        None,
        ScmOptions {
            forward_alpha: 0.01,
            num_rounds: Some(2),
            ..Default::default()
        },
    )
    .unwrap();

    snapshot_settings(dir.path()).bind(|| assert_snapshot!(built.render_text()));
}

/// Re-planning over a paused SCM process without a candidate
/// that never won: the rendering says the SCM process carries on without it.
/// Then the same without a candidate that did win, which it cannot.
#[test]
fn replan_removing_candidates_over_a_paused_process() {
    let dir = tempfile::tempdir().unwrap();
    let model = write_template(dir.path());
    let previous = plan_of(
        &model,
        &["WT_CL", "CRCL_CL", "WT_V"],
        None,
        ScmOptions::default(),
    );
    mid_scm_state(&previous)
        .save(&previous.out_dir_path())
        .unwrap();

    let loser_gone =
        plan_named(&model, &["WT_CL", "CRCL_CL"], None, ScmOptions::default()).unwrap();
    let winner_gone =
        plan_named(&model, &["CRCL_CL", "WT_V"], None, ScmOptions::default()).unwrap();

    snapshot_settings(dir.path()).bind(|| {
        assert_snapshot!("replan_removing_a_loser", loser_gone.render_text());
        assert_snapshot!("replan_removing_a_winner", winner_gone.render_text());
    });
}

// ---------------------------------------------------------------------------
// Failure shapes
// ---------------------------------------------------------------------------

/// How the driver classifies every kind of run a fit can leave behind.
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
    for (i, fit) in fits.iter().map(Some).chain([None]).enumerate() {
        let model =
            write_named_template(&dir.path().join(format!("case{i}")), "1001.mod", TEMPLATE);
        if let Some(fit) = fit {
            write_fit_output(&model, *fit).unwrap();
        }
        let outcome = read_fit_outcome(&model, &Default::default(), false).unwrap();
        out.push_str(&format!(
            "## {}\nlabel {} · usable {} · ofv {:?} · heuristics {:?}\n\n",
            fit.map_or("NeverStarted".to_string(), |f| format!("{f:?}")),
            outcome.label(),
            outcome.usable(),
            outcome.ofv,
            outcome.heuristics
        ));
    }

    snapshot_settings(dir.path()).bind(|| assert_snapshot!(out));
}

// ---------------------------------------------------------------------------
// Records and renderings
// ---------------------------------------------------------------------------

/// What `scm status` prints. The place lookup stands in for the slurm job
/// registry: the fixtures' running model is `job 4242`, in the queue
/// unless `job_gone`.
fn brief_with(out_dir: &Path, job_gone: bool) -> String {
    let place_of = |model: &Path| {
        model
            .ends_with("forward_round1/1001_crcl_cl.mod")
            .then(|| match job_gone {
                true => format!("job 4242 {}", scm::status::NOT_IN_QUEUE),
                false => "job 4242".to_string(),
            })
    };
    ScmStatus::read(out_dir).unwrap().render(&place_of)
}

fn brief(out_dir: &Path) -> String {
    brief_with(out_dir, false)
}

/// Change the state in `out_dir` as a driver would: loaded, edited, saved.
fn edit_state(out_dir: &Path, edit: impl FnOnce(&mut ScmState)) {
    let mut state = ScmState::load(out_dir).unwrap().unwrap();
    edit(&mut state);
    state.save(out_dir).unwrap();
}

/// The state as `scm stop` leaves it.
fn pause(state: &mut ScmState) {
    state.status = ScmRunStatus::Paused;
    state.message = Some(scm::INTERRUPTED_NOTE.to_string());
}

/// `scm status` across the states an SCM process can be found
/// in: not started, mid-round, and paused mid-round (the completed state
/// ends the mid-run removal transcript, the failed state the reference-fit
/// one).
#[test]
fn status_rendering_across_states() {
    // not started: a filled-in config in the out_dir, no state
    {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template(dir.path());
        let init = init_scm(&model, false).unwrap();
        let filled = fs::read_to_string(&init.config_path).unwrap().replace(
            "effects = []",
            "effects = [\"WT_CL\", \"CRCL_CL\", \"WT_V\"]",
        );
        fs::write(&init.config_path, filled).unwrap();
        let text = brief(&init.out_dir);
        snapshot_settings(dir.path()).bind(|| assert_snapshot!("status_planned", text));
    }
    // running: mid-round, one retry behind it, one model still running and
    // part-way through its iterations
    {
        let dir = tempfile::tempdir().unwrap();
        let out_dir = fabricate_running_scm(dir.path());
        fs::write(
            out_dir.join("forward_round1/1001_crcl_cl/1001_crcl_cl.ext"),
            format!("{EXT_HEADER}{EXT_ITERATIONS}"),
        )
        .unwrap();
        let text = brief(&out_dir);
        snapshot_settings(dir.path()).bind(|| assert_snapshot!("status_running_mid_round", text));
    }
    // resumed this morning after a stop last night: the round's clock
    // counts the time the process ran, not the night; the running fit's
    // job sat in the queue the whole time, so its own clock counts it.
    {
        let dir = tempfile::tempdir().unwrap();
        let out_dir = fabricate_running_scm(dir.path());
        let hours_ago = |h: i64| {
            (jiff::Timestamp::now() - jiff::SignedDuration::from_hours(h))
                .to_zoned(jiff::tz::TimeZone::UTC)
                .strftime("%Y-%m-%dT%H:%M:%S%:z")
                .to_string()
        };
        let run_dir = out_dir.join("forward_round1/1001_crcl_cl");
        let mut start = RunStartFile::load(run_dir.join(RUN_START_FILENAME)).unwrap();
        start.start = hours_ago(20);
        start.save(&run_dir).unwrap();
        edit_state(&out_dir, |state| {
            state.pauses.push(Pause {
                stopped: hours_ago(19),
                resumed: hours_ago(1),
            })
        });
        let text = brief(&out_dir);
        assert!(
            text.contains("1 running · 1 pending · running 2h 00m"),
            "{text}"
        );
        assert!(text.contains("job 4242   running 20h 00m"), "{text}");
    }
    // paused mid-round by `scm stop`: the clocks stop at the pause. The
    // running fit's job is still in the queue (a stop without `--fits`), so
    // its own clock runs on; then it is gone, and it stopped with the
    // process.
    {
        let dir = tempfile::tempdir().unwrap();
        let out_dir = fabricate_running_scm(dir.path());
        edit_state(&out_dir, pause);
        let text = brief(&out_dir);
        snapshot_settings(dir.path()).bind(|| assert_snapshot!("status_paused_fit_in_queue", text));
        let text = brief_with(&out_dir, true);
        assert!(
            text.contains("1 stopped · 1 pending · paused after"),
            "{text}"
        );
        assert!(
            text.contains("CRCL_CL  job 4242 (not in queue) stopped after"),
            "{text}"
        );
        snapshot_settings(dir.path()).bind(|| assert_snapshot!("status_paused_fit_gone", text));
    }
    // paused with the fits ended but not read in by the driver: the running
    // fit's run ended (status reads it in as fitted), and a second was
    // dispatched but the stop came before its submission; then that one has
    // ended too, and the round waits on a resume for its decision.
    {
        let dir = tempfile::tempdir().unwrap();
        let out_dir = fabricate_running_scm(dir.path());
        let crcl = out_dir.join("forward_round1/1001_crcl_cl.mod");
        write_fit_output(&crcl, Fit::Succeeded(990.0)).unwrap();
        let wt_v = out_dir.join("forward_round1/1001_wt_v.mod");
        fs::write(&wt_v, TEMPLATE).unwrap();
        edit_state(&out_dir, |state| {
            pause(state);
            let round = state.rounds.last_mut().unwrap();
            let cand = round
                .candidates
                .iter_mut()
                .find(|c| c.candidate == "WT_V")
                .unwrap();
            cand.status = CandidateStatus::Running;
            cand.model = "forward_round1/1001_wt_v.mod".into();
        });
        let text = brief_with(&out_dir, true);
        assert!(
            text.contains("2/3 done · 1 pending · paused after"),
            "{text}"
        );
        assert!(text.contains("○ WT_V     not submitted"), "{text}");
        assert!(!text.contains("every fit has ended"), "{text}");

        write_fit_output(&wt_v, Fit::Succeeded(995.0)).unwrap();
        let text = brief_with(&out_dir, true);
        snapshot_settings(dir.path())
            .bind(|| assert_snapshot!("status_paused_every_fit_ended", text));
    }
}

/// `scm summary` of a single round, in progress and then
/// complete in the long view with files and timing — a retried winner, a
/// not-significant candidate and an unusable one (p-values on both sides
/// of the 0.001 formatting switch), and a running candidate that has no
/// attempt recorded yet.
#[test]
fn summary_rendering_of_a_single_round() {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = fabricate_running_scm(dir.path());
    let one = |round: &str| SummaryOptions {
        round: Some(round.to_string()),
        ..Default::default()
    };

    // in progress, as fabricated
    let open = read_summary(&out_dir).unwrap();
    let in_progress = open.render_text(&one("1")).unwrap();
    // The finished candidate is scored against the round's reference as soon
    // as its fit lands, but nothing is ranked while the round can still
    // overturn a placing.
    {
        let round = open.rounds.iter().find(|r| r.index == 1).unwrap();
        let wt_cl = &round.candidates[0];
        assert_eq!(wt_cl.status, "succeeded");
        assert_eq!(wt_cl.delta_ofv, Some(-20.0));
        assert_eq!(wt_cl.significant, Some(true));
        assert!(round.candidates.iter().all(|c| c.rank.is_none()));
    }

    // now conclude it: WT_CL wins, CRCL_CL not significant, WT_V unusable
    let mut state = ScmState::load(&out_dir).unwrap().unwrap();
    {
        let round = state.round_mut("forward_round1").unwrap();
        let wt_cl = &mut round.candidates[0];
        wt_cl.delta_ofv = Some(-20.0);
        wt_cl.p_value = Some(7.744e-6);
        wt_cl.significant = Some(true);
        wt_cl.selected = true;

        let crcl = &mut round.candidates[1];
        succeeded(crcl, "forward_round1/1001_crcl_cl.mod", 996.0);
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
    let complete_long = summary
        .render_text(&SummaryOptions {
            long: true,
            files: true,
            timing: true,
            ..one("forward_round1")
        })
        .unwrap();

    snapshot_settings(dir.path()).bind(|| {
        assert_snapshot!("summary_round_in_progress", in_progress);
        assert_snapshot!("summary_round_complete_long_files_timing", complete_long);
    });
}

// ---------------------------------------------------------------------------
// Driver transcripts
// ---------------------------------------------------------------------------

/// A full forward-then-backward run: fits dispatched, files
/// written, final state and summary markdown, then the `scm summary --long`
/// view and the records the run wrote: `scm_summary.json` with its rounds
/// left out, and the `round_summary.json` of the round with a retry, which
/// pins the round schema once.
#[test]
fn transcript_full_forward_backward_run() {
    let dir = tempfile::tempdir().unwrap();
    let plan = make_plan(dir.path(), ScmOptions::default());
    let executor = full_scm_executor();
    run_scm(&plan, &executor, false).unwrap();
    let out_dir = plan.out_dir_path();
    let long = SummaryOptions {
        long: true,
        ..Default::default()
    };
    let summary_long = read_summary(&out_dir).unwrap().render_text(&long).unwrap();
    // `rounds` is the record's last field: everything from it on is cut.
    let summary = read(&out_dir.join(SCM_SUMMARY_FILENAME));
    let rounds_at = summary.find("  \"rounds\": [").unwrap();
    let summary = format!(
        "{}  \"rounds\": \"[one RoundSummary per round: see round_summary_json]\"\n}}\n",
        &summary[..rounds_at]
    );

    snapshot_settings(dir.path()).bind(|| {
        assert_snapshot!(
            "transcript_full_forward_backward_run",
            transcript(&plan, &executor, &[STATE_FILENAME, SCM_SUMMARY_MD])
        );
        assert_snapshot!("summary_long", summary_long);
        assert_snapshot!("scm_summary_json", summary);
        assert_snapshot!(
            "round_summary_json",
            read(&out_dir.join("forward_round2").join(scm::ROUND_SUMMARY_JSON))
        );
    });
}

/// A pause takes the forward model's fit down with it (a shared node's step
/// dies with the allocation the pausing driver releases), leaving a run
/// directory that says started. The resumed driver says so, fits it again
/// as the same attempt before its first round, and reads the refit at the
/// next look-in: what it reports, the fits it dispatches, and the state.
#[test]
fn transcript_forward_model_fit_dead_on_resume() {
    let dir = tempfile::tempdir().unwrap();
    let plan = make_plan(
        dir.path(),
        ScmOptions {
            num_rounds: Some(3),
            ..Default::default()
        },
    );
    let paused = full_scm_executor().submitted_fits_wait();
    let state = run_scm(&plan, &paused, false).unwrap();
    assert_eq!(state.status, ScmRunStatus::Paused);
    let model = plan
        .out_dir_path()
        .join(&state.forward_final.as_ref().unwrap().model);
    write_fit_output(&model, Fit::StillRunning).unwrap();

    let mut resumed = plan.clone();
    resumed.options.num_rounds = None;
    let executor = full_scm_executor();
    let (state, lines) = scm::report::capture(|| run_scm(&resumed, &executor, false).unwrap());
    assert_eq!(state.status, ScmRunStatus::Completed);

    snapshot_settings(dir.path()).bind(|| {
        assert_snapshot!(format!(
            "# resumed driver\n{}\n{}",
            listing(&lines),
            transcript(&resumed, &executor, &[STATE_FILENAME])
        ));
    });
}

/// Every candidate in the round is unusable: nothing was scored, yet
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
    // Every candidate fails both attempts, each in a different way.
    let executor = MockExecutor::new(1234.0)
        .with("base/1001_base", vec![Fit::Succeeded(1000.0)])
        .with(
            "forward_round1/1001_wt_cl",
            vec![Fit::NoFinalRow, Fit::MinimizationTerminated(990.0)],
        )
        .with(
            "forward_round1/1001_crcl_cl",
            vec![Fit::Aborted(700.0), Fit::AbortedHeaderless],
        )
        .with(
            "forward_round1/1001_wt_v",
            vec![Fit::Terminated, Fit::NoFinalRow],
        );
    let result = run_scm(&plan, &executor, false);
    let header = match &result {
        Ok(outcome) => format!("run_scm: Ok, status {}\n\n", outcome.status),
        Err(e) => format!("run_scm: Err: {e:#}\n\n"),
    };

    snapshot_settings(dir.path()).bind(|| {
        let record = transcript(&plan, &executor, &[SCM_SUMMARY_MD]);
        assert_snapshot!(format!("{header}{record}"))
    });
}

/// What the driver says as it goes — to the `scm run` terminal and the
/// driver job's `.out` file: every fit as it ends, every round as it is
/// decided, and the process's end. The full run retries WT_V in round 2; the
/// second run is a reference fit that never succeeds.
#[test]
fn driver_reports_each_fit_and_round_as_it_ends() {
    let dir = tempfile::tempdir().unwrap();
    let plan = make_plan(dir.path(), ScmOptions::default());
    let ((), full) = super::report::capture(|| {
        run_scm(&plan, &full_scm_executor(), false).unwrap();
    });

    let failing_dir = tempfile::tempdir().unwrap();
    let failing = make_plan(
        failing_dir.path(),
        ScmOptions {
            max_retries: 1,
            ..Default::default()
        },
    );
    let executor =
        MockExecutor::new(1234.0).with("base/1001_base", vec![Fit::NoFinalRow, Fit::NoFinalRow]);
    let ((), no_reference) = super::report::capture(|| {
        run_scm(&failing, &executor, false).unwrap_err();
    });

    let mut settings = snapshot_settings(dir.path());
    settings.add_filter(
        &regex::escape(&failing_dir.path().display().to_string()),
        "[TMP2]",
    );
    forward_slashes(&mut settings);
    settings.bind(|| {
        assert_snapshot!(format!(
            "# full run\n{}\n# reference fit fails\n{}",
            listing(&full),
            listing(&no_reference)
        ))
    });
}

/// The reference fit fails every attempt, so the SCM process cannot
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
    let executor = MockExecutor::new(1234.0).with(
        "base/1001_base",
        vec![Fit::MinimizationTerminated(1000.0), Fit::NoFinalRow],
    );
    let err = run_scm(&plan, &executor, false).unwrap_err();

    snapshot_settings(dir.path()).bind(|| {
        assert_snapshot!(format!(
            "error: {err:#}\n\n{}",
            transcript(&plan, &executor, &[])
        ))
    });
}

/// A candidate removed between rounds: the resumed run refits
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
    run_scm(&plan, &executor, false).unwrap();

    let fewer = plan_of(
        &plan.model_path(),
        &["WT_CL", "CRCL_CL"],
        None,
        ScmOptions::default(),
    );
    run_scm(&fewer, &executor, false).unwrap();
    let status = brief(&plan.out_dir_path());

    snapshot_settings(dir.path()).bind(|| {
        assert_snapshot!(format!(
            "{}\n# scm status\n{status}",
            transcript(&fewer, &executor, &[]),
        ))
    });
}

/// Running a changed plan into an out_dir holding another plan's state
/// is refused; with overwrite the SCM-owned output is cleared and nothing
/// else is touched. The second run fails at its first fit so the tree
/// shows the cleared out_dir rather than a fresh run's output. (The tree
/// before the overwrite is the full run's, in its own transcript.)
#[test]
fn transcript_mismatched_plan_refused_then_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let plan = make_plan(dir.path(), ScmOptions::default());
    run_scm(&plan, &full_scm_executor(), false).unwrap();
    let out_dir = plan.out_dir_path();

    // Files the SCM does not own, planted where the clearing runs: they go
    // with everything else, only the config stays.
    fs::create_dir_all(out_dir.join("notes")).unwrap();
    fs::write(out_dir.join("notes/readme.txt"), "mine\n").unwrap();
    fs::create_dir_all(out_dir.join("forward_roundX")).unwrap();
    fs::write(out_dir.join("forward_roundX/keep.txt"), "not a round\n").unwrap();
    let before = file_tree(&out_dir);

    let mut changed = plan.clone();
    changed.options.forward_alpha = 0.01;
    let refused = run_scm(&changed, &full_scm_executor(), false).unwrap_err();

    let failing = MockExecutor::new(0.0).failing_with("sbatch: error: Batch job submission failed");
    let failed = run_scm(&changed, &failing, true).unwrap_err();
    let after = file_tree(&out_dir);
    assert!(before.len() > after.len(), "{before:?}");
    // The state is the overwrite run's, under the changed plan, failed at
    // its reference fit.
    let state = ScmState::load(&out_dir).unwrap().unwrap();
    assert_eq!(state.status, ScmRunStatus::Failed);
    assert_eq!(state.message.as_deref(), Some(&*format!("{failed:#}")));
    assert_eq!(state.plan.options.forward_alpha, 0.01);
    assert_eq!(state.rounds.len(), 1);
    assert!(state.rounds[0].is_reference() && !state.rounds[0].complete);

    let out = format!(
        "# refused without overwrite\n{refused:#}\n\n# overwrite run (fails at its first fit)\n\
         {failed:#}\n\n# out_dir after overwrite\n{}",
        listing(&after),
    );

    snapshot_settings(dir.path()).bind(|| assert_snapshot!(out));
}
