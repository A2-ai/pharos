use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::round::{reconcile_round_with_disk, reconcile_state_with_disk};
use super::state::{PendingTie, RoundRecord, ScmState};
use super::{Lines, PLAN_FILENAME, ROUND_SUMMARY_MD, ScmPlan, none_or_list, ofv_suffix, round_dir};

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
    /// Set when the search is paused waiting for the user to break a tie.
    pub pending_tie: Option<PendingTie>,
    pub updated: Option<String>,
    /// Models with a started but unfinished run right now (relative to out_dir).
    pub models_running: Vec<String>,
}

/// Load the plan of the search living in `out_dir`, insisting the directory
/// actually is one (both readers start here).
fn load_plan_in(out_dir: &Path) -> Result<ScmPlan> {
    let plan_path = out_dir.join(PLAN_FILENAME);
    if !plan_path.exists() {
        bail!(
            "{} has no {PLAN_FILENAME}; is this an SCM output directory?",
            out_dir.display()
        );
    }
    ScmPlan::load(&plan_path).with_context(|| format!("failed to load {}", plan_path.display()))
}

/// Read the status of the search living in `out_dir` (the directory holding
/// plan.json / scm_state.json).
pub fn read_status(out_dir: &Path) -> Result<ScmStatus> {
    let plan = load_plan_in(out_dir)?;
    let mut state = ScmState::load(out_dir)?;

    let models_running = match &mut state {
        Some(state) => reconcile_state_with_disk(state, out_dir),
        None => vec![],
    };

    let mut status = ScmStatus {
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
        pending_tie: None,
        updated: None,
        models_running,
    };

    if let Some(state) = state {
        status.rounds_complete = state.completed_search_rounds();
        status.status = state.status.to_string();
        status.message = state.message;
        status.phase = state.phase.map(|p| p.to_string());
        status.retained = state.retained;
        status.reference_model = state.reference_model;
        status.reference_ofv = state.reference_ofv;
        status.final_model = state.final_model;
        status.had_unusable = state.had_unusable;
        status.pending_tie = state.pending_tie;
        status.updated = Some(state.updated);
        status.rounds = state.rounds;
    }

    Ok(status)
}

impl ScmStatus {
    /// Human-readable rendering for the CLI.
    pub fn render_text(&self) -> String {
        let mut out = Lines::new();

        out.add(format!("<scm status> {}", self.out_dir));
        out.add(format!("model      : {}", self.plan.model));
        out.add(format!(
            "candidates : {}",
            self.plan
                .candidates
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
        match &self.updated {
            Some(u) => out.add(format!("status     : {} (updated {u})", self.status)),
            None => out.add(format!("status     : {}", self.status)),
        }
        if let Some(m) = &self.message {
            out.add(format!("note       : {m}"));
        }
        if let Some(p) = &self.phase {
            out.add(format!("phase      : {p}"));
        }
        // The one state that needs the user to do something, so it gets its
        // own line rather than hiding in the note.
        if let Some(tie) = &self.pending_tie {
            out.add(format!(
                "awaiting   : your decision on {} in {} (p = {:.3e}, dOFV = {:+.3}) — re-run with --choose <candidate>",
                tie.candidates.join(" / "),
                tie.round,
                tie.p_value,
                tie.delta_ofv
            ));
        }

        if !self.rounds.is_empty() {
            out.add("rounds     :");
            for round in &self.rounds {
                let (total, done) = (round.candidates.len(), round.concluded());
                let (retries, unusable) = (round.retries(), round.unusable());

                let mut extra = format!("{total} model(s)");
                if retries > 0 {
                    let plural = if retries == 1 { "y" } else { "ies" };
                    write!(extra, ", {retries} retr{plural}").unwrap();
                }
                if unusable > 0 {
                    write!(extra, ", {unusable} unusable").unwrap();
                }

                if round.complete {
                    out.add(format!("  {:<18} {} [{extra}]", round.name, round.decision));
                } else {
                    out.add(format!(
                        "  {:<18} in progress — {done}/{total} concluded [{extra}]",
                        round.name
                    ));
                }
            }
        }

        if !self.models_running.is_empty() {
            out.add(format!("running    : {}", self.models_running.join(", ")));
        }
        if !self.rounds.is_empty() {
            out.add(
                "records    : round_summary.{json,md} in each round dir; \
                 scm_decision_log.{csv,md} in the out dir",
            );
        }
        // What the search added, shown only once the whole search is done —
        // while it runs, the per-round decision lines above tell the story.
        if self.status == "completed" {
            out.add(format!("retained   : {}", none_or_list(&self.retained)));
        }
        if let Some(f) = &self.final_model {
            // The final model is generated warm-started from the search's
            // last reference fit, so that fit's OFV is its OFV.
            out.add(format!(
                "final model: {f}{}",
                ofv_suffix(self.reference_ofv)
            ));
        }
        out.finish()
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
        && let Some(round) = state.rounds.iter().filter(|r| !r.is_reference()).nth(n - 1)
    {
        return Ok(round);
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
    load_plan_in(out_dir)?;
    let state = ScmState::load(out_dir)?
        .ok_or_else(|| anyhow::anyhow!("the search has not started; no rounds to summarize"))?;
    let mut round = find_round(&state, selector)?.clone();
    if !round.complete {
        reconcile_round_with_disk(&mut round, out_dir, &mut Vec::new());
    }

    let summary_md = round_dir(&round.name, &round.candidates)
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
        let mut out = Lines::new();
        let round = &self.round;

        out.add(format!("<scm round> {} — {}", round.name, self.out_dir));
        out.add(format!("direction  : {}", round.direction));
        if round.complete {
            out.add(format!("progress   : complete — {}", round.decision));
        } else {
            let (total, done) = (round.candidates.len(), round.concluded());
            out.add(format!(
                "progress   : in progress — {done}/{total} concluded"
            ));
            // An open round carries a decision only when it is waiting on
            // one (a tie the search could not break).
            if !round.decision.is_empty() {
                out.add(format!("note       : {}", round.decision));
            }
        }
        if round.has_reference() {
            out.add(format!(
                "reference  : {}{}",
                round.reference_model,
                ofv_suffix(round.reference_ofv)
            ));
        }

        out.add("candidates :");
        for cand in &round.candidates {
            let mut line = format!(
                "  {:<12} {:<16} {}",
                cand.candidate, cand.action, cand.status
            );
            if let Some(ofv) = cand.ofv {
                write!(line, "  OFV {ofv:.3}").unwrap();
            }
            if let Some(d) = cand.delta_ofv {
                write!(line, "  dOFV {d:.3}").unwrap();
            }
            if let Some(p) = cand.p_value {
                if p >= 0.001 {
                    write!(line, "  p {p:.4}").unwrap();
                } else {
                    write!(line, "  p {p:.3e}").unwrap();
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
            out.add(line);
            for attempt in &cand.attempts {
                out.add(format!("      {:<44} {}", attempt.model, attempt.outcome));
            }
            // The attempts list is empty until a model is dispatched; the
            // model field still points at the run when one exists.
            if cand.attempts.is_empty() && !cand.model.is_empty() {
                out.add(format!("      {:<44} {}", cand.model, cand.status));
            }
            if !cand.heuristics.is_empty() {
                out.add(format!("      heuristics: {}", cand.heuristics.join(", ")));
            }
        }

        match &self.summary_md {
            Some(md) => out.add(format!(
                "round file : {md} (full per-run record incl. heuristics)"
            )),
            None => out.add("round file : round_summary.md not written yet"),
        }
        out.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scm::plan::tests::{thetas, write_template};
    use crate::scm::state::{AttemptRecord, CandidateRecord, CandidateStatus};
    use crate::run::metadata::{RUN_END_FILENAME, RUN_START_FILENAME};
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

    /// A run's output directory as pharos leaves it: started, and finished
    /// with an OFV when one is given.
    fn write_run(model: &Path, ofv: Option<f64>) {
        let stem = model.file_stem().unwrap().to_string_lossy().to_string();
        let run_dir = model.parent().unwrap().join(&stem);
        fs_err::create_dir_all(&run_dir).unwrap();
        fs_err::write(run_dir.join(RUN_START_FILENAME), "{}").unwrap();
        if let Some(ofv) = ofv {
            let ext = format!(
                "TABLE NO.     1: First Order Conditional Estimation with Interaction\n\
                 \x20ITERATION    THETA1       THETA2       THETA3       THETA4       THETA5       THETA6       OMEGA(1,1)   OMEGA(2,2)   SIGMA(1,1)   OBJ\n\
                 \x20 -1000000000  3.10000E+00  2.10000E+01  1.30000E+00  2.50000E-01  1.50000E-01  5.00000E-02  8.00000E-02  8.50000E-02  1.80000E-02  {ofv}\n"
            );
            fs_err::write(run_dir.join(format!("{stem}.ext")), ext).unwrap();
            fs_err::write(run_dir.join(RUN_END_FILENAME), "{}").unwrap();
        }
    }

    /// The driver only writes a wave's outcomes back to the state once the
    /// whole batch returns, so the reader has to see finished runs itself.
    #[test]
    fn an_open_round_counts_runs_that_finished_since_the_state_was_written() {
        let dir = tempfile::tempdir().unwrap();
        let out_dir = fabricate_search(dir.path());
        let model = out_dir.join("forward_round1/1001_crcl_cl.mod");

        // Dispatched and still running: reported as running, not concluded.
        write_run(&model, None);
        let status = read_status(&out_dir).unwrap();
        assert!(
            status.render_text().contains("in progress — 1/2 concluded"),
            "got:\n{}",
            status.render_text()
        );
        assert_eq!(
            status.models_running,
            vec!["forward_round1/1001_crcl_cl.mod".to_string()]
        );

        // Finished with an OFV while the driver still waits on its batch.
        write_run(&model, Some(990.0));
        let status = read_status(&out_dir).unwrap();
        let text = status.render_text();
        assert!(text.contains("in progress — 2/2 concluded"), "got:\n{text}");
        assert!(status.models_running.is_empty(), "got:\n{text}");

        // The round view picks up the same fit, OFV and all.
        let text = read_round_detail(&out_dir, "1").unwrap().render_text();
        assert!(text.contains("in progress — 2/2 concluded"), "got:\n{text}");
        assert!(text.contains("OFV 990.000"), "got:\n{text}");
        assert!(
            text.contains("forward_round1/1001_crcl_cl.mod"),
            "got:\n{text}"
        );

        // The decision log reads the same search through the same helper,
        // so it reports the fit rather than a candidate still running.
        let mut state = ScmState::load(&out_dir).unwrap().unwrap();
        let running = reconcile_state_with_disk(&mut state, &out_dir);
        assert!(running.is_empty(), "got: {running:?}");
        let row = crate::scm::decision_log_rows(&state)
            .into_iter()
            .find(|r| r.candidate == "CRCL_CL")
            .expect("CRCL_CL row");
        assert_eq!(row.status, "succeeded");
        assert_eq!(row.attempts, 1);

        // Reading never writes: the state stays the driver's to update.
        let state = ScmState::load(&out_dir).unwrap().unwrap();
        assert_eq!(
            state.rounds[1].candidates[1].status,
            CandidateStatus::Running
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

        // the brace-carrying records pointer survives formatting intact
        assert!(
            text.contains(
                "records    : round_summary.{json,md} in each round dir; \
                 scm_decision_log.{csv,md} in the out dir"
            ),
            "got:\n{text}"
        );

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
