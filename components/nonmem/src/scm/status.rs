use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::round::{run_dir_for, run_finished};
use super::state::{CandidateStatus, RoundRecord, ScmState};
use super::{PLAN_FILENAME, ROUND_SUMMARY_MD, ScmPlan};
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
        push(format!(
            "candidates : {}",
            self.plan
                .candidates
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
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
        // What the search added, shown only once the whole search is done —
        // while it runs, the per-round decision lines above tell the story.
        if self.status == "completed" {
            push(format!(
                "retained   : {}",
                if self.retained.is_empty() {
                    "none".to_string()
                } else {
                    self.retained.join(", ")
                }
            ));
        }
        if let Some(f) = &self.final_model {
            // The final model is generated warm-started from the search's
            // last reference fit, so that fit's OFV is its OFV.
            let ofv = self
                .reference_ofv
                .map(|o| format!(" (OFV {o:.3})"))
                .unwrap_or_default();
            push(format!("final model: {f}{ofv}"));
        }
        out
    }
}

/// Detailed view of one round: every model run in it with its outcome, plus
/// where the round's own record files live. `scm status` shows the whole
/// search one line per round; this drills into a single round.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScmRoundDetail {
    pub out_dir: String,
    pub round: RoundRecord,
    /// `<round dir>/round_summary.md`, relative to out_dir, when it exists —
    /// the full per-run record including every heuristic that fired.
    pub summary_md: Option<String>,
}

/// The directory a round's outputs live in: the round name, except the
/// reference round whose models live under the reference fit's own name
/// ("base" / "full").
fn round_dir_of(round: &RoundRecord) -> Option<String> {
    if round.name == "reference" {
        round.candidates.first().map(|c| c.candidate.clone())
    } else {
        Some(round.name.clone())
    }
}

/// Find the round `selector` names: an exact round name ("forward_round1",
/// "reference"), or the Nth search round chronologically ("2" / "round 2" —
/// the reference fit is not a round).
fn find_round<'a>(state: &'a ScmState, selector: &str) -> Result<&'a RoundRecord> {
    let sel = selector.trim();
    if let Some(round) = state
        .rounds
        .iter()
        .find(|r| r.name.eq_ignore_ascii_case(sel))
    {
        return Ok(round);
    }

    let lowered = sel.to_ascii_lowercase();
    let num_part = lowered
        .strip_prefix("round")
        .map(str::trim)
        .unwrap_or(lowered.as_str());
    if let Ok(n) = num_part.parse::<usize>()
        && n >= 1
    {
        if let Some(round) = state
            .rounds
            .iter()
            .filter(|r| r.name != "reference")
            .nth(n - 1)
        {
            return Ok(round);
        }
    }

    let available: Vec<&str> = state.rounds.iter().map(|r| r.name.as_str()).collect();
    bail!(
        "no round matching '{selector}'; rounds so far: {}",
        if available.is_empty() {
            "(none yet)".to_string()
        } else {
            available.join(", ")
        }
    );
}

/// Read the detailed record of one round of the search in `out_dir`.
pub fn read_round_detail(out_dir: &Path, selector: &str) -> Result<ScmRoundDetail> {
    let plan_path = out_dir.join(PLAN_FILENAME);
    if !plan_path.exists() {
        bail!(
            "{} has no {PLAN_FILENAME}; is this an SCM output directory?",
            out_dir.display()
        );
    }
    let state = ScmState::load(out_dir)?
        .ok_or_else(|| anyhow::anyhow!("the search has not started; no rounds to summarize"))?;
    let round = find_round(&state, selector)?.clone();

    let summary_md = round_dir_of(&round)
        .map(|dir| format!("{dir}/{ROUND_SUMMARY_MD}"))
        .filter(|rel| out_dir.join(rel).exists());

    Ok(ScmRoundDetail {
        out_dir: out_dir.to_string_lossy().to_string(),
        round,
        summary_md,
    })
}

impl ScmRoundDetail {
    /// Human-readable rendering for the CLI.
    pub fn render_text(&self) -> String {
        let mut out = String::new();
        let mut push = |s: String| {
            out.push_str(&s);
            out.push('\n');
        };

        let round = &self.round;
        push(format!("<scm round> {} — {}", round.name, self.out_dir));
        push(format!("direction  : {}", round.direction));
        if round.complete {
            push(format!("progress   : complete — {}", round.decision));
        } else {
            let total = round.candidates.len();
            let done = round
                .candidates
                .iter()
                .filter(|c| c.status.is_concluded())
                .count();
            push(format!(
                "progress   : in progress — {done}/{total} concluded"
            ));
        }
        if round.reference_model != "-" {
            let ofv = round
                .reference_ofv
                .map(|o| format!(" (OFV {o:.3})"))
                .unwrap_or_default();
            push(format!("reference  : {}{ofv}", round.reference_model));
        }

        push("candidates :".to_string());
        for cand in &round.candidates {
            let mut line = format!(
                "  {:<12} {:<16} {}",
                cand.candidate, cand.action, cand.status
            );
            if let Some(ofv) = cand.ofv {
                line.push_str(&format!("  OFV {ofv:.3}"));
            }
            if let Some(d) = cand.delta_ofv {
                line.push_str(&format!("  dOFV {d:.3}"));
            }
            if let Some(p) = cand.p_value {
                if p >= 0.001 {
                    line.push_str(&format!("  p {p:.4}"));
                } else {
                    line.push_str(&format!("  p {p:.3e}"));
                }
                match cand.significant {
                    Some(true) => line.push_str(" (significant)"),
                    Some(false) => line.push_str(" (not significant)"),
                    None => {}
                }
            }
            if cand.selected {
                line.push_str("  <- selected");
            }
            push(line);
            for attempt in &cand.attempts {
                push(format!("      {:<44} {}", attempt.model, attempt.outcome));
            }
            // The attempts list is empty until a model is dispatched; the
            // model field still points at the run when one exists.
            if cand.attempts.is_empty() && !cand.model.is_empty() {
                push(format!("      {:<44} {}", cand.model, cand.status));
            }
            if !cand.heuristics.is_empty() {
                push(format!("      heuristics: {}", cand.heuristics.join(", ")));
            }
        }

        match &self.summary_md {
            Some(md) => push(format!(
                "round file : {md} (full per-run record incl. heuristics)"
            )),
            None => push("round file : round_summary.md not written yet".to_string()),
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scm::plan::tests::{thetas, write_template};
    use crate::scm::state::{AttemptRecord, CandidateRecord};
    use crate::scm::{Direction, ScmOptions, build_plan};
    use std::path::PathBuf;

    /// A plan on disk plus a fabricated two-round state: a reference fit and
    /// one in-progress forward round with a retry and a selection.
    fn fabricate_search(dir: &Path) -> PathBuf {
        let model_path = write_template(dir);
        let built = build_plan(
            &model_path,
            &thetas(&[4, 5, 6]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap();
        built.plan.save().unwrap();
        let out_dir = built.plan.out_dir_path();

        let mut state = ScmState::new(built.plan.digest());
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
        wt_cl.attempts.push(AttemptRecord {
            model: "forward_round1/1001_wt_cl.mod".into(),
            outcome: "no ofv".into(),
        });
        wt_cl.attempts.push(AttemptRecord {
            model: "forward_round1/1001_wt_cl_try2.mod".into(),
            outcome: "succeeded".into(),
        });
        wt_cl.status = CandidateStatus::Succeeded;
        wt_cl.ofv = Some(980.0);
        wt_cl.delta_ofv = Some(-20.0);
        wt_cl.p_value = Some(7.7e-6);
        wt_cl.significant = Some(true);
        wt_cl.selected = true;
        wt_cl.heuristics = vec!["parameter near boundary".into()];
        let mut crcl = CandidateRecord::new("CRCL_CL", "add CRCL_CL".into(), 1);
        crcl.model = "forward_round1/1001_crcl_cl.mod".into();
        crcl.status = CandidateStatus::Running;
        state.rounds.push(RoundRecord {
            name: "forward_round1".into(),
            direction: Direction::Forward,
            reference_model: "base/1001_base.mod".into(),
            reference_ofv: Some(1000.0),
            candidates: vec![wt_cl, crcl],
            winner: None,
            decision: String::new(),
            complete: false,
        });
        state.save(&out_dir).unwrap();
        out_dir
    }

    #[test]
    fn round_detail_selects_by_number_name_and_reference() {
        let dir = tempfile::tempdir().unwrap();
        let out_dir = fabricate_search(dir.path());

        // "1", "round 1", and the full name all land on forward_round1
        for sel in ["1", "round 1", "Round 1", "forward_round1"] {
            let detail = read_round_detail(&out_dir, sel).unwrap();
            assert_eq!(detail.round.name, "forward_round1", "selector {sel}");
        }
        let reference = read_round_detail(&out_dir, "reference").unwrap();
        assert_eq!(reference.round.name, "reference");

        let err = read_round_detail(&out_dir, "7").unwrap_err();
        assert!(err.to_string().contains("forward_round1"), "got: {err}");
    }

    #[test]
    fn round_detail_lists_every_model_run_with_its_outcome() {
        let dir = tempfile::tempdir().unwrap();
        let out_dir = fabricate_search(dir.path());

        let detail = read_round_detail(&out_dir, "1").unwrap();
        let text = detail.render_text();
        assert!(text.contains("in progress — 1/2 concluded"), "got:\n{text}");
        // every attempt, including the failed first try
        assert!(
            text.contains("forward_round1/1001_wt_cl.mod"),
            "got:\n{text}"
        );
        assert!(text.contains("no ofv"), "got:\n{text}");
        assert!(
            text.contains("forward_round1/1001_wt_cl_try2.mod"),
            "got:\n{text}"
        );
        assert!(text.contains("<- selected"), "got:\n{text}");
        // the still-running candidate shows its model even with no attempt yet
        assert!(
            text.contains("forward_round1/1001_crcl_cl.mod"),
            "got:\n{text}"
        );
        assert!(text.contains("running"), "got:\n{text}");
        assert!(
            text.contains("heuristics: parameter near boundary"),
            "got:\n{text}"
        );
        // no round_summary.md written in this fabricated search
        assert!(text.contains("not written yet"), "got:\n{text}");

        // once the md exists, the pointer names it
        fs_err::create_dir_all(out_dir.join("forward_round1")).unwrap();
        fs_err::write(out_dir.join("forward_round1/round_summary.md"), "x").unwrap();
        let detail = read_round_detail(&out_dir, "1").unwrap();
        assert_eq!(
            detail.summary_md.as_deref(),
            Some("forward_round1/round_summary.md")
        );
    }

    #[test]
    fn status_render_lists_candidates_and_holds_retained_until_completed() {
        let dir = tempfile::tempdir().unwrap();
        let out_dir = fabricate_search(dir.path());

        let mut status = read_status(&out_dir).unwrap();
        status.retained = vec!["WT_CL".into()];
        status.reference_model = Some("base/1001_base.mod".into());
        status.reference_ofv = Some(980.0);
        let text = status.render_text();
        assert!(
            text.contains("candidates : WT_CL, CRCL_CL, WT_V"),
            "got:\n{text}"
        );
        // mid-search: the rounds tell the story; no retained line yet, and
        // the reference line is gone entirely
        assert!(!text.contains("retained"), "got:\n{text}");
        assert!(!text.contains("reference  :"), "got:\n{text}");

        status.status = "completed".to_string();
        status.final_model = Some("final/1001_scm_final.mod".into());
        let text = status.render_text();
        assert!(text.contains("retained   : WT_CL"), "got:\n{text}");
        // the final model carries the last reference fit's OFV
        assert!(
            text.contains("final model: final/1001_scm_final.mod (OFV 980.000)"),
            "got:\n{text}"
        );
    }
}
