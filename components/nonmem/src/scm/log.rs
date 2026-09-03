use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use fs_err as fs;
use serde::{Deserialize, Serialize};
use utils::get_utc_now;

use super::state::{CandidateRecord, CandidateStatus, ScmRunStatus, ScmState};
use super::{
    DECISION_LOG_CSV, DECISION_LOG_MD, Direction, ROUND_SUMMARY_JSON, ROUND_SUMMARY_MD, ScmPlan,
};

pub const ROUND_SUMMARY_SCHEMA_VERSION: u32 = 1;

fn fmt_opt(value: Option<f64>, decimals: usize) -> String {
    match value {
        Some(v) => format!("{v:.decimals$}"),
        None => String::new(),
    }
}

fn fmt_p(value: Option<f64>) -> String {
    match value {
        Some(p) => format!("{p:.4e}"),
        None => String::new(),
    }
}

fn csv_escape(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

/// One row of the decision log: one candidate fit within one round, with
/// every column the CSV rendering writes. Fields are plain values so other
/// front ends (e.g. an R data.frame) can be built from them directly.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionLogRow {
    pub round: String,
    pub direction: String,
    pub candidate: String,
    pub model: String,
    pub attempts: usize,
    pub status: String,
    pub reference_ofv: Option<f64>,
    pub delta_ofv: Option<f64>,
    pub df: usize,
    pub p_value: Option<f64>,
    pub significant: Option<bool>,
    pub selected: bool,
    pub heuristics: String,
    pub decision: String,
}

/// Flatten the state's rounds into decision-log rows, one per candidate fit.
pub fn decision_log_rows(state: &ScmState) -> Vec<DecisionLogRow> {
    let mut rows = Vec::new();
    for round in &state.rounds {
        for cand in &round.candidates {
            rows.push(DecisionLogRow {
                round: round.name.clone(),
                direction: round.direction.to_string(),
                candidate: cand.candidate.clone(),
                model: cand.model.clone(),
                attempts: cand.n_attempts(),
                status: cand.status.to_string(),
                // None for the reference round: the base/full model has no
                // reference — its own OFV is carried in the round's decision
                // ("base model fitted (OFV ...)"), never in this column.
                reference_ofv: round.reference_ofv,
                delta_ofv: cand.delta_ofv,
                df: cand.df,
                p_value: cand.p_value,
                significant: cand.significant,
                selected: cand.selected,
                heuristics: cand.heuristics.join("; "),
                decision: round.decision.clone(),
            });
        }
    }
    rows
}

pub fn decision_log_csv(state: &ScmState) -> String {
    let mut lines = vec![
        "round,direction,candidate,model,attempts,status,reference_ofv,delta_ofv,df,p_value,significant,selected,heuristics,decision"
            .to_string(),
    ];

    for row in decision_log_rows(state) {
        let fields = [
            row.round,
            row.direction,
            row.candidate,
            row.model,
            row.attempts.to_string(),
            row.status,
            fmt_opt(row.reference_ofv, 3),
            fmt_opt(row.delta_ofv, 3),
            row.df.to_string(),
            fmt_p(row.p_value),
            row.significant.map(|s| s.to_string()).unwrap_or_default(),
            row.selected.to_string(),
            row.heuristics,
            row.decision,
        ];
        lines.push(
            fields
                .iter()
                .map(|f| csv_escape(f))
                .collect::<Vec<_>>()
                .join(","),
        );
    }

    lines.join("\n") + "\n"
}

/// The per-candidate markdown table shared by the decision log and the
/// round summaries.
fn candidate_table_lines(candidates: &[CandidateRecord]) -> Vec<String> {
    let mut lines = vec![
        "| candidate | model | attempts | status | ΔOFV | df | p | significant | selected | heuristic checks |"
            .to_string(),
        "|---|---|---|---|---|---|---|---|---|---|".to_string(),
    ];
    for cand in candidates {
        lines.push(format!(
            "| {} | `{}` | {} | {} | {} | {} | {} | {} | {} | {} |",
            cand.candidate,
            cand.model,
            cand.n_attempts(),
            cand.status,
            fmt_opt(cand.delta_ofv, 3),
            cand.df,
            fmt_p(cand.p_value),
            cand.significant
                .map(|s| if s { "yes" } else { "no" }.to_string())
                .unwrap_or_default(),
            if cand.selected { "**yes**" } else { "" },
            if cand.heuristics.is_empty() {
                "-".to_string()
            } else {
                cand.heuristics.join("; ")
            },
        ));
    }
    lines
}

pub fn decision_log_md(plan: &ScmPlan, state: &ScmState) -> String {
    let mut out = String::new();
    let mut push = |s: String| {
        out.push_str(&s);
        out.push('\n');
    };

    push("# SCM decision log".to_string());
    push(String::new());
    push(format!("- model: `{}`", plan.model));
    push(format!("- out dir: `{}`", plan.out_dir));
    push(format!("- direction: {}", plan.options.direction_label()));
    push(format!(
        "- alphas: forward {}, backward {}",
        plan.options.forward_alpha, plan.options.backward_alpha
    ));
    push(format!(
        "- retries: up to {} per fit, starting from the previous attempt's estimates",
        plan.options.max_retries
    ));
    push(format!(
        "- covariance step: {}",
        if plan.options.cov_step { "on" } else { "off" }
    ));
    push(format!("- status: {}", state.status));
    push(format!(
        "- retained: {}",
        if state.retained.is_empty() {
            "none".to_string()
        } else {
            state.retained.join(", ")
        }
    ));
    if let Some(f) = &state.final_model {
        push(format!("- final model: `{f}` (not fitted by the search)"));
    }
    if let Some(m) = &state.message {
        push(format!("- note: {m}"));
    }
    push(String::new());

    for round in &state.rounds {
        push(format!("## {}", round.name));
        push(String::new());
        if let Some(ofv) = round.reference_ofv {
            push(format!(
                "Reference: `{}` (OFV {ofv:.3})",
                round.reference_model
            ));
            push(String::new());
        }
        for line in candidate_table_lines(&round.candidates) {
            push(line);
        }
        push(String::new());
        if !round.decision.is_empty() {
            push(format!("**Decision:** {}", round.decision));
            push(String::new());
        }
        if round
            .candidates
            .iter()
            .any(|c| c.status == CandidateStatus::Unusable)
        {
            push(
                "_Unusable candidates are reported above; they are never scored as insignificant._"
                    .to_string(),
            );
            push(String::new());
        }
    }

    out
}

/// Write both renderings into `out_dir`, returning (csv path, md path).
pub fn write_decision_log(
    out_dir: &Path,
    plan: &ScmPlan,
    state: &ScmState,
) -> Result<(PathBuf, PathBuf)> {
    let csv_path = out_dir.join(DECISION_LOG_CSV);
    let md_path = out_dir.join(DECISION_LOG_MD);
    fs::write(&csv_path, decision_log_csv(state))?;
    fs::write(&md_path, decision_log_md(plan, state))?;
    Ok((csv_path, md_path))
}

/// A self-contained record of one round, written into the round's own
/// directory when the round concludes: what was tested against which
/// reference, how every fit went, the round's decision, and where the
/// search stood when it was written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoundSummary {
    pub schema_version: u32,
    pub generated: String,
    pub plan_digest: String,
    /// The template control stream the search runs on.
    pub template_model: String,
    pub round: String,
    pub direction: Direction,
    /// Reference model relative to out_dir ("-" for the reference round).
    pub reference_model: String,
    pub reference_ofv: Option<f64>,
    /// Every candidate fit concluded with a scoreable result.
    pub all_succeeded: bool,
    /// At least one scoring attempt had heuristic checks fire.
    pub any_heuristics: bool,
    /// At least one candidate ran out of retries without a scoreable fit.
    pub any_unusable: bool,
    pub winner: Option<String>,
    pub decision: String,
    /// Covariates in the model after this round, in selection order.
    pub retained_after: Vec<String>,
    /// Search status when this summary was written.
    pub search_status: String,
    /// What the search does next.
    pub next: String,
    pub candidates: Vec<CandidateRecord>,
}

/// Build the summary of one named round from the current state.
pub fn round_summary(plan: &ScmPlan, state: &ScmState, round_name: &str) -> Result<RoundSummary> {
    let round = state
        .rounds
        .iter()
        .find(|r| r.name == round_name)
        .with_context(|| format!("no round named {round_name} in the state"))?;

    let next = if state.status == ScmRunStatus::Failed {
        match &state.message {
            Some(m) => format!("search failed: {m}"),
            None => "search failed".to_string(),
        }
    } else {
        match state.phase {
            Some(p) if round_name == "reference" => format!("start {p} selection"),
            Some(p) => format!("continue {p} selection"),
            None => match &state.final_model {
                Some(f) => format!("search complete; final model at {f}"),
                None => "search complete".to_string(),
            },
        }
    };

    Ok(RoundSummary {
        schema_version: ROUND_SUMMARY_SCHEMA_VERSION,
        generated: get_utc_now(),
        plan_digest: state.plan_digest.clone(),
        template_model: plan.model.clone(),
        round: round.name.clone(),
        direction: round.direction,
        reference_model: round.reference_model.clone(),
        reference_ofv: round.reference_ofv,
        all_succeeded: round
            .candidates
            .iter()
            .all(|c| c.status == CandidateStatus::Succeeded),
        any_heuristics: round.candidates.iter().any(|c| !c.heuristics.is_empty()),
        any_unusable: round
            .candidates
            .iter()
            .any(|c| c.status == CandidateStatus::Unusable),
        winner: round.winner.clone(),
        decision: round.decision.clone(),
        retained_after: state.retained.clone(),
        search_status: state.status.to_string(),
        next,
        candidates: round.candidates.clone(),
    })
}

/// The directory a round's models live in: the round name, except the
/// reference round, whose single "candidate" (base/full) names its directory.
fn round_dir_name(summary: &RoundSummary) -> Option<String> {
    if summary.round == "reference" {
        summary.candidates.first().map(|c| c.candidate.clone())
    } else {
        Some(summary.round.clone())
    }
}

pub fn round_summary_md(summary: &RoundSummary) -> String {
    let mut out = String::new();
    let mut push = |s: String| {
        out.push_str(&s);
        out.push('\n');
    };
    let yes_no = |b: bool| if b { "yes" } else { "no" };

    push(format!("# {}", summary.round));
    push(String::new());
    push(format!("- template: `{}`", summary.template_model));
    push(format!("- direction: {}", summary.direction));
    if summary.reference_model != "-" {
        push(format!(
            "- reference: `{}`{}",
            summary.reference_model,
            summary
                .reference_ofv
                .map(|o| format!(" (OFV {o:.3})"))
                .unwrap_or_default()
        ));
    }
    push(format!(
        "- all fits succeeded: {}",
        yes_no(summary.all_succeeded)
    ));
    push(format!(
        "- heuristic checks fired: {}",
        yes_no(summary.any_heuristics)
    ));
    push(format!(
        "- unusable candidates: {}",
        yes_no(summary.any_unusable)
    ));
    if !summary.decision.is_empty() {
        push(format!("- decision: {}", summary.decision));
    }
    push(format!(
        "- retained after this round: {}",
        if summary.retained_after.is_empty() {
            "none".to_string()
        } else {
            summary.retained_after.join(", ")
        }
    ));
    push(format!("- next: {}", summary.next));
    push(String::new());
    for line in candidate_table_lines(&summary.candidates) {
        push(line);
    }
    out
}

/// Write a round's summary (JSON + markdown) into its round directory,
/// returning (json path, md path).
pub fn write_round_summary(
    out_dir: &Path,
    plan: &ScmPlan,
    state: &ScmState,
    round_name: &str,
) -> Result<(PathBuf, PathBuf)> {
    let summary = round_summary(plan, state, round_name)?;
    let dir_name = round_dir_name(&summary)
        .with_context(|| format!("round {round_name} has no candidates to name its directory"))?;
    let dir = out_dir.join(dir_name);
    fs::create_dir_all(&dir)?;

    let json_path = dir.join(ROUND_SUMMARY_JSON);
    utils::write_json_to_file(&summary, &json_path)
        .with_context(|| format!("failed to write {}", json_path.display()))?;
    let md_path = dir.join(ROUND_SUMMARY_MD);
    fs::write(&md_path, round_summary_md(&summary))?;
    Ok((json_path, md_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scm::state::{AttemptRecord, CandidateRecord, RoundRecord};
    use crate::scm::{Candidate, Direction, PLAN_SCHEMA_VERSION, ScmOptions};

    fn sample() -> (ScmPlan, ScmState) {
        let plan = ScmPlan {
            schema_version: PLAN_SCHEMA_VERSION,
            created: "2026-08-19".into(),
            pharos_version: "test".into(),
            model: "1001.mod".into(),
            out_dir: "scm/1001".into(),
            candidates: vec![Candidate {
                name: "WT_CL".into(),
                theta: 4,
            }],
            max_models: 3,
            options: ScmOptions::default(),
        };

        let mut state = ScmState::new(plan.digest());
        let mut cand = CandidateRecord::new("WT_CL", "add WT_CL".into(), 1);
        cand.model = "forward_round1/1001_wt_cl_try2.mod".into();
        cand.attempts = vec![
            AttemptRecord {
                model: "forward_round1/1001_wt_cl.mod".into(),
                outcome: "minimization terminated".into(),
            },
            AttemptRecord {
                model: "forward_round1/1001_wt_cl_try2.mod".into(),
                outcome: "succeeded".into(),
            },
        ];
        cand.status = CandidateStatus::Succeeded;
        cand.ofv = Some(980.0);
        cand.delta_ofv = Some(-20.0);
        cand.p_value = Some(7.7e-6);
        cand.significant = Some(true);
        cand.selected = true;
        cand.heuristics = vec!["parameter near boundary".into()];

        state.rounds.push(RoundRecord {
            name: "forward_round1".into(),
            direction: Direction::Forward,
            reference_model: "base/1001_base.mod".into(),
            reference_ofv: Some(1000.0),
            candidates: vec![cand],
            winner: Some("WT_CL".into()),
            decision: "added WT_CL (p = 7.7e-6, dOFV = -20.0)".into(),
            complete: true,
        });
        state.retained = vec!["WT_CL".into()];
        (plan, state)
    }

    #[test]
    fn csv_has_header_and_rows() {
        let (_, state) = sample();
        let csv = decision_log_csv(&state);
        let lines: Vec<&str> = csv.trim().lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("round,direction,candidate"));
        assert!(lines[1].contains("forward_round1"));
        assert!(lines[1].contains("WT_CL"));
        assert!(lines[1].contains("2")); // attempts
        assert!(lines[1].contains("-20.000"));
        // decision contains a comma -> quoted
        assert!(lines[1].contains("\"added WT_CL"));
    }

    #[test]
    fn md_mentions_retries_and_heuristics() {
        let (plan, state) = sample();
        let md = decision_log_md(&plan, &state);
        assert!(md.contains("# SCM decision log"));
        assert!(md.contains("ΔOFV"));
        assert!(md.contains("parameter near boundary"));
        assert!(md.contains("**Decision:** added WT_CL"));
        assert!(md.contains("| 2 |")); // two attempts
    }

    #[test]
    fn writes_both_files() {
        let dir = tempfile::tempdir().unwrap();
        let (plan, state) = sample();
        let (csv, md) = write_decision_log(dir.path(), &plan, &state).unwrap();
        assert!(csv.exists());
        assert!(md.exists());
    }

    #[test]
    fn round_summary_captures_flags_and_writes_into_the_round_dir() {
        let dir = tempfile::tempdir().unwrap();
        let (plan, mut state) = sample();
        state.phase = Some(Direction::Forward);

        let (json, md) = write_round_summary(dir.path(), &plan, &state, "forward_round1").unwrap();
        assert!(json.starts_with(dir.path().join("forward_round1")));
        assert!(json.exists());
        assert!(md.exists());

        let summary: RoundSummary =
            serde_json::from_str(&fs::read_to_string(&json).unwrap()).unwrap();
        assert!(summary.all_succeeded);
        assert!(summary.any_heuristics); // "parameter near boundary"
        assert!(!summary.any_unusable);
        assert_eq!(summary.winner.as_deref(), Some("WT_CL"));
        assert_eq!(summary.retained_after, vec!["WT_CL".to_string()]);
        assert_eq!(summary.next, "continue forward selection");

        let md_text = fs::read_to_string(&md).unwrap();
        assert!(md_text.contains("# forward_round1"));
        assert!(md_text.contains("added WT_CL"));
        assert!(md_text.contains("retained after this round: WT_CL"));
        assert!(md_text.contains("| WT_CL |"));
    }

    #[test]
    fn reference_round_summary_lands_in_base_or_full() {
        let dir = tempfile::tempdir().unwrap();
        let (plan, mut state) = sample();
        let mut cand = CandidateRecord::new("base", "fit base model".into(), 1);
        cand.model = "base/1001_base.mod".into();
        cand.status = CandidateStatus::Succeeded;
        cand.ofv = Some(1000.0);
        state.rounds.insert(
            0,
            RoundRecord {
                name: "reference".into(),
                direction: Direction::Forward,
                reference_model: "-".into(),
                reference_ofv: None,
                candidates: vec![cand],
                winner: None,
                decision: "base model fitted (OFV 1000.000)".into(),
                complete: true,
            },
        );
        state.phase = Some(Direction::Forward);

        let (json, _) = write_round_summary(dir.path(), &plan, &state, "reference").unwrap();
        assert!(json.starts_with(dir.path().join("base")));
        let summary: RoundSummary =
            serde_json::from_str(&fs::read_to_string(&json).unwrap()).unwrap();
        assert_eq!(summary.next, "start forward selection");
    }
}
