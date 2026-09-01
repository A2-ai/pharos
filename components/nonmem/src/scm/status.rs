use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::round::{run_dir_for, run_finished};
use super::state::{CandidateStatus, RoundRecord, ScmState};
use super::{PLAN_FILENAME, ScmPlan};
use crate::run::metadata::RUN_START_FILENAME;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScmStatus {
    pub out_dir: String,
    pub plan: ScmPlan,
    /// planned | running | paused | completed | failed
    pub status: String,
    pub message: Option<String>,
    pub phase: Option<String>,
    pub retained: Vec<String>,
    pub reference_model: Option<String>,
    pub reference_ofv: Option<f64>,
    pub rounds_complete: usize,
    pub rounds: Vec<RoundRecord>,
    pub final_model: Option<String>,
    pub had_unusable: bool,
    pub updated: Option<String>,
    /// Models with a started but unfinished run right now (relative to out_dir).
    pub models_running: Vec<String>,
}

/// Read the status of the search living in `out_dir` (the directory holding
/// plan.json / scm_state.json).
pub fn read_status(out_dir: &Path) -> Result<ScmStatus> {
    let plan_path = out_dir.join(PLAN_FILENAME);
    if !plan_path.exists() {
        bail!(
            "{} has no {PLAN_FILENAME}; is this an SCM output directory?",
            out_dir.display()
        );
    }
    let plan = ScmPlan::load(&plan_path)
        .with_context(|| format!("failed to load {}", plan_path.display()))?;

    let state = ScmState::load(out_dir)?;

    let mut models_running = Vec::new();
    if let Some(state) = &state {
        for round in &state.rounds {
            if round.complete {
                continue;
            }
            for cand in &round.candidates {
                if cand.status != CandidateStatus::Running || cand.model.is_empty() {
                    continue;
                }
                let model_path = out_dir.join(&cand.model);
                let run_dir = run_dir_for(&model_path);
                let started = run_dir.join(RUN_START_FILENAME).exists();
                if started && !run_finished(&model_path) {
                    models_running.push(cand.model.clone());
                }
            }
        }
    }

    Ok(match state {
        Some(state) => ScmStatus {
            out_dir: out_dir.to_string_lossy().to_string(),
            plan,
            status: state.status.to_string(),
            message: state.message.clone(),
            phase: state.phase.map(|p| p.to_string()),
            retained: state.retained.clone(),
            reference_model: state.reference_model.clone(),
            reference_ofv: state.reference_ofv,
            rounds_complete: state.completed_search_rounds(),
            rounds: state.rounds.clone(),
            final_model: state.final_model.clone(),
            had_unusable: state.had_unusable,
            updated: Some(state.updated),
            models_running,
        },
        None => ScmStatus {
            out_dir: out_dir.to_string_lossy().to_string(),
            plan,
            status: "planned".to_string(),
            message: Some("plan written; the search has not started".to_string()),
            phase: None,
            retained: vec![],
            reference_model: None,
            reference_ofv: None,
            rounds_complete: 0,
            rounds: vec![],
            final_model: None,
            had_unusable: false,
            updated: None,
            models_running: vec![],
        },
    })
}

impl ScmStatus {
    /// Human-readable rendering for the CLI.
    pub fn render_text(&self) -> String {
        let mut out = String::new();
        let mut push = |s: String| {
            out.push_str(&s);
            out.push('\n');
        };

        push(format!("<scm status> {}", self.out_dir));
        push(format!("model      : {}", self.plan.model));
        match &self.updated {
            Some(u) => push(format!("status     : {} (updated {u})", self.status)),
            None => push(format!("status     : {}", self.status)),
        }
        if let Some(m) = &self.message {
            push(format!("note       : {m}"));
        }
        if let Some(p) = &self.phase {
            push(format!("phase      : {p}"));
        }
        push(format!(
            "retained   : {}",
            if self.retained.is_empty() {
                "none".to_string()
            } else {
                self.retained.join(", ")
            }
        ));
        if let Some(r) = &self.reference_model {
            let ofv = self
                .reference_ofv
                .map(|o| format!(" (OFV {o:.3})"))
                .unwrap_or_default();
            push(format!("reference  : {r}{ofv}"));
        }

        if !self.rounds.is_empty() {
            push("rounds     :".to_string());
            for round in &self.rounds {
                let total = round.candidates.len();
                let done = round
                    .candidates
                    .iter()
                    .filter(|c| c.status.is_concluded())
                    .count();
                let retries: usize = round
                    .candidates
                    .iter()
                    .map(|c| c.n_attempts().saturating_sub(1))
                    .sum();
                let unusable = round
                    .candidates
                    .iter()
                    .filter(|c| c.status == CandidateStatus::Unusable)
                    .count();

                let mut extra = format!("{total} model(s)");
                if retries > 0 {
                    extra.push_str(&format!(
                        ", {retries} retr{}",
                        if retries == 1 { "y" } else { "ies" }
                    ));
                }
                if unusable > 0 {
                    extra.push_str(&format!(", {unusable} unusable"));
                }

                if round.complete {
                    push(format!("  {:<18} {} [{extra}]", round.name, round.decision));
                } else {
                    push(format!(
                        "  {:<18} in progress — {done}/{total} concluded [{extra}]",
                        round.name
                    ));
                }
            }
        }

        if !self.models_running.is_empty() {
            push(format!("running    : {}", self.models_running.join(", ")));
        }
        if !self.rounds.is_empty() {
            push(
                "records    : round_summary.{json,md} in each round dir; \
                 scm_decision_log.{csv,md} in the out dir"
                    .to_string(),
            );
        }
        if let Some(f) = &self.final_model {
            push(format!("final model: {f}"));
        }
        out
    }
}
