use std::path::{Path, PathBuf};

use anyhow::Result;
use fs_err as fs;

use super::state::{CandidateStatus, ScmState};
use super::{DECISION_LOG_CSV, DECISION_LOG_MD, ScmPlan};

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
                reference_ofv: round.reference_ofv.or(cand.ofv),
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
        push(
            "| candidate | model | attempts | status | ΔOFV | df | p | significant | selected | heuristic checks |"
                .to_string(),
        );
        push("|---|---|---|---|---|---|---|---|---|---|".to_string());
        for cand in &round.candidates {
            push(format!(
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
}
