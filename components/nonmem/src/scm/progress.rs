//! Where an SCM process already stands when its plan is (re)built, and what the
//! new plan changes about it.
//!
//! `scm plan` is re-run all the time — after a fit fails mid-round, after a
//! SCM process pauses, or just to move an alpha. Rendered on its own the plan
//! reads the same every time, which says nothing about the SCM process sitting
//! in its out_dir already half run. [`PlanContext`] is read from the out_dir
//! *before* the new plan.json replaces the old one, so the plan rendering
//! can say how far the SCM process got, which covariates it has selected, and
//! what this plan changed. It is a sketch, not a report: `scm status` and
//! `scm summary` remain the detailed views.

use serde::{Deserialize, Serialize};

use super::roster::{Compatibility, compatibility};
use super::round::reconcile_state_with_disk;
use super::state::{PendingTie, ScmState};
use super::{Candidate, Lines, PLAN_FILENAME, ScmPlan, none_or_list, on_off};

/// One difference between the new plan and the plan.json it replaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanChange {
    /// The plan field that changed, e.g. `candidates`, `forward_alpha`.
    pub field: String,
    /// What changed about it, e.g. `0.05 -> 0.01`, `added AGE_CL THETA(8)`.
    pub detail: String,
    /// Whether the change makes state already in the out_dir belong to a
    /// different plan, so the SCM process cannot resume it. Options and
    /// added or altered candidates are; a removed candidate is only when it
    /// has won a round (see [`compatibility`]).
    pub scm_defining: bool,
}

impl PlanChange {
    fn new(field: &str, detail: String, scm_defining: bool) -> Self {
        Self {
            field: field.to_string(),
            detail,
            scm_defining,
        }
    }
}

/// The round in flight when the plan was rebuilt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentRound {
    pub name: String,
    /// Candidates that reached a terminal state in it.
    pub concluded: usize,
    pub total: usize,
}

/// How far the SCM process in the plan's out_dir got, read from its state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanProgress {
    /// planned | running | paused | completed | failed
    pub status: String,
    /// The phase the SCM process is in, when it has started one.
    pub phase: Option<String>,
    /// Completed SCM rounds (the reference fit is not a round).
    pub rounds_complete: usize,
    /// The round that had started but not finished, if any.
    pub current_round: Option<CurrentRound>,
    /// Covariates selected so far, in selection order.
    pub retained: Vec<String>,
    /// Candidates removed from the SCM process so far, as `NAME (after round)`.
    #[serde(default)]
    pub removed: Vec<String>,
    pub final_model: Option<String>,
    /// Set when the SCM process paused for the user to break a tie.
    pub pending_tie: Option<PendingTie>,
    /// Models with a started but unfinished run right now.
    pub models_running: usize,
    /// When the state was last written.
    pub updated: String,
}

impl PlanProgress {
    /// The one-line summary: status, rounds run, phase.
    fn headline(&self) -> String {
        let rounds = match self.rounds_complete {
            0 => "no rounds complete yet".to_string(),
            1 => "1 round complete".to_string(),
            n => format!("{n} rounds complete"),
        };
        let phase = match &self.phase {
            Some(p) => format!(", {p} phase"),
            None => String::new(),
        };
        format!(
            "{} — {rounds}{phase} (updated {})",
            self.status, self.updated
        )
    }
}

/// What a freshly built plan meets in its out_dir: the SCM process already run
/// there, and the plan.json this one replaces.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PlanContext {
    /// Whether the out_dir already held a plan.json this plan replaces.
    pub had_previous_plan: bool,
    /// Differences from that plan; empty when there was none, or when the
    /// new plan is identical to it.
    pub changes: Vec<PlanChange>,
    /// The state found in the out_dir, when the SCM process has started.
    pub progress: Option<PlanProgress>,
    /// True when that state belongs to a *different* plan than this one —
    /// the SCM process cannot resume it without overwrite.
    pub state_is_stale: bool,
    /// Why, one line per reason, when it is.
    #[serde(default)]
    pub stale_reasons: Vec<String>,
    /// Candidates this plan drops that never won a round: the SCM process
    /// resumes without them (whether or not it is stale for other reasons).
    #[serde(default)]
    pub removals: Vec<String>,
}

impl PlanContext {
    /// Read the out_dir `plan` is about to be written into. Call this before
    /// [`ScmPlan::save`], which replaces the plan.json being compared
    /// against. An unreadable plan.json or state file is treated as absent:
    /// this is a courtesy rendering, never a reason to fail planning.
    pub fn read(plan: &ScmPlan) -> Self {
        let out_dir = plan.out_dir_path();

        let previous = ScmPlan::load(out_dir.join(PLAN_FILENAME)).ok();
        let state = ScmState::load(&out_dir).ok().flatten();
        let mut ctx = PlanContext {
            had_previous_plan: previous.is_some(),
            changes: previous
                .as_ref()
                .map(|p| diff_plans(p, plan, state.as_ref()))
                .unwrap_or_default(),
            ..Default::default()
        };

        if let Some(mut state) = state {
            // The driver writes a wave's outcomes back only once the whole
            // batch returns, so mid-round the state still calls finished
            // runs `running`. Read them off disk the way `scm status` does,
            // so both views describe the same SCM process.
            let running = reconcile_state_with_disk(&mut state, &out_dir);
            // The same verdict `scm run` will reach.
            match compatibility(plan, &state) {
                Compatibility::Identical => {}
                Compatibility::Compatible { removals } => ctx.removals = removals,
                Compatibility::Incompatible { reasons, removals } => {
                    ctx.state_is_stale = true;
                    ctx.stale_reasons = reasons;
                    ctx.removals = removals;
                }
            }
            ctx.progress = Some(PlanProgress {
                status: state.status.to_string(),
                phase: state.phase.map(|p| p.to_string()),
                rounds_complete: state.completed_rounds(),
                current_round: state
                    .rounds
                    .iter()
                    .find(|r| !r.complete && !r.is_reference())
                    .map(|r| CurrentRound {
                        name: r.name.clone(),
                        concluded: r.concluded(),
                        total: r.candidates.len(),
                    }),
                removed: state.removed_roster().map(|e| e.removal_label()).collect(),
                retained: state.retained,
                final_model: state.final_model,
                pending_tie: state.pending_tie,
                models_running: running.len(),
                updated: state.updated,
            });
        }

        ctx
    }

    /// Whether there is anything to render at all: a fresh out_dir has
    /// neither an SCM process behind it nor a plan to differ from.
    pub fn is_empty(&self) -> bool {
        !self.had_previous_plan && self.progress.is_none()
    }

    /// Append the progress and changes sections to a plan rendering.
    pub(crate) fn render_into(&self, out: &mut Lines) {
        if self.is_empty() {
            return;
        }
        out.blank();

        if let Some(p) = &self.progress {
            out.add(format!("progress   : {}", p.headline()));
            out.add(format!("selected   : {}", none_or_list(&p.retained)));
            if !p.removed.is_empty() {
                out.add(format!("removed    : {}", p.removed.join(", ")));
            }
            if let Some(cur) = &p.current_round {
                out.add(format!(
                    "in round   : {} — {}/{} concluded",
                    cur.name, cur.concluded, cur.total
                ));
            }
            if p.models_running > 0 {
                out.add(format!("running    : {} model(s)", p.models_running));
            }
            if let Some(tie) = &p.pending_tie {
                out.add(format!(
                    "awaiting   : your decision on {} in {} — re-run with --choose <candidate>",
                    tie.candidates.join(" / "),
                    tie.round
                ));
            }
            if let Some(f) = &p.final_model {
                out.add(format!("final model: {f}"));
            }
        } else if self.had_previous_plan {
            out.add("progress   : not started — the out_dir holds a plan but no SCM process state");
        }

        if self.had_previous_plan {
            if self.changes.is_empty() {
                out.add("changes    : none — identical to the previous plan");
            } else {
                let n = self.changes.len();
                let plural = if n == 1 { "" } else { "s" };
                out.add(format!(
                    "changes    : {n} change{plural} vs the previous plan"
                ));
                for c in &self.changes {
                    let flag = if c.scm_defining {
                        "  (SCM-defining)"
                    } else {
                        ""
                    };
                    out.add(format!("  {:<14} {}{flag}", c.field, c.detail));
                }
            }
        }

        // Removals of never-selected candidates take effect on the next run
        // without disturbing anything already fitted.
        if !self.removals.is_empty() && self.progress.is_some() {
            out.add(format!(
                "removing   : {} — never selected; takes effect from the next round, earlier rounds keep their results",
                self.removals.join(", ")
            ));
        }

        // The one consequence the user has to act on: a changed SCM process
        // cannot pick up where the old one left off.
        if self.state_is_stale {
            out.add(
                "note       : the SCM process in out_dir belongs to the previous plan; it cannot \
                 resume under this one:",
            );
            for reason in &self.stale_reasons {
                out.add(format!("             - {reason}"));
            }
            out.add(
                "             re-plan with overwrite to discard it and start the SCM process fresh",
            );
        }
    }
}

/// The candidate of that name, if the plan has one — candidates are matched
/// by name, so a name that moved thetas is a change, not a new candidate.
fn find_candidate<'a>(candidates: &'a [Candidate], name: &str) -> Option<&'a Candidate> {
    candidates.iter().find(|c| c.name == name)
}

/// Every way the new plan differs from the one it replaces, in the order a
/// plan rendering lists the fields. `state` decides whether a removed
/// candidate costs the SCM process its state (it does when the candidate
/// has won a round); without a state nothing is at stake.
fn diff_plans(prev: &ScmPlan, next: &ScmPlan, state: Option<&ScmState>) -> Vec<PlanChange> {
    let mut changes = Vec::new();
    let (po, no) = (&prev.options, &next.options);

    if prev.model != next.model {
        changes.push(PlanChange::new(
            "model",
            format!("{} -> {}", prev.model, next.model),
            true,
        ));
    }
    if po.direction != no.direction {
        changes.push(PlanChange::new(
            "direction",
            format!("{} -> {}", po.direction_label(), no.direction_label()),
            true,
        ));
    }

    // Candidates are compared by name: an added or dropped effect is what
    // the user needs to see, and a name that moved thetas is a template
    // edit worth flagging on its own.
    for c in &next.candidates {
        match find_candidate(&prev.candidates, &c.name) {
            None => changes.push(PlanChange::new(
                "candidates",
                format!("added {} THETA({})", c.name, c.theta),
                true,
            )),
            Some(old) if old.theta != c.theta => changes.push(PlanChange::new(
                "candidates",
                format!(
                    "{} moved THETA({}) -> THETA({})",
                    c.name, old.theta, c.theta
                ),
                true,
            )),
            Some(old) => {
                if old.initial != c.initial {
                    changes.push(PlanChange::new(
                        "candidates",
                        format!(
                            "{} starts at {} -> {} when first tested",
                            c.name, old.initial, c.initial
                        ),
                        true,
                    ));
                }
                if old.off != c.off {
                    changes.push(PlanChange::new(
                        "candidates",
                        format!("{} is held out at {} -> {}", c.name, old.off, c.off),
                        true,
                    ));
                }
            }
        }
    }
    for c in &prev.candidates {
        if find_candidate(&next.candidates, &c.name).is_none() {
            // A candidate that won a round is load-bearing; one that never
            // did can go without disturbing the state.
            let load_bearing = state.and_then(|s| s.depends_on(&c.name));
            let detail = match &load_bearing {
                Some(why) => format!("removed {} THETA({}) — {why}", c.name, c.theta),
                None => format!("removed {} THETA({})", c.name, c.theta),
            };
            changes.push(PlanChange::new(
                "candidates",
                detail,
                load_bearing.is_some(),
            ));
        }
    }

    if po.forward_alpha != no.forward_alpha {
        changes.push(PlanChange::new(
            "forward_alpha",
            format!("{} -> {}", po.forward_alpha, no.forward_alpha),
            true,
        ));
    }
    if po.backward_alpha != no.backward_alpha {
        changes.push(PlanChange::new(
            "backward_alpha",
            format!("{} -> {}", po.backward_alpha, no.backward_alpha),
            true,
        ));
    }
    if po.max_retries != no.max_retries {
        changes.push(PlanChange::new(
            "max_retries",
            format!("{} -> {}", po.max_retries, no.max_retries),
            true,
        ));
    }
    if po.cov_step != no.cov_step {
        changes.push(PlanChange::new(
            "cov_step",
            format!("{} -> {}", on_off(po.cov_step), on_off(no.cov_step)),
            true,
        ));
    }

    // num_rounds paces this run of the SCM process rather than defining it, so a
    // change to it never invalidates state.
    if po.num_rounds != no.num_rounds {
        let label = |n: Option<usize>| match n {
            Some(n) => n.to_string(),
            None => "no cap".to_string(),
        };
        changes.push(PlanChange::new(
            "num_rounds",
            format!("{} -> {}", label(po.num_rounds), label(no.num_rounds)),
            false,
        ));
    }

    changes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scm::Direction;
    use crate::scm::plan::build_plan;
    use crate::scm::plan::tests::{names, write_template};
    use crate::scm::state::{CandidateRecord, CandidateStatus, RoundRecord, ScmRunStatus};
    use crate::scm::{PLAN_SCHEMA_VERSION, ScmOptions};
    use std::path::Path;

    /// Build a plan for the shared test template into `out_dir`, with the
    /// candidates and options given.
    fn plan_for(model: &Path, cands: &[&str], options: ScmOptions, out_dir: &Path) -> ScmPlan {
        build_plan(model, &names(cands), Some(out_dir), options, "test")
            .unwrap()
            .plan
    }

    /// A state two forward rounds in, with a third under way.
    fn mid_scm_state(plan: &ScmPlan) -> ScmState {
        let mut state = ScmState::new(plan);
        state.status = ScmRunStatus::Paused;
        state.phase = Some(Direction::Forward);
        state.retained = vec!["WT_CL".to_string(), "CRCL_CL".to_string()];
        state.rounds = vec![
            RoundRecord {
                name: "forward_round1".to_string(),
                direction: Direction::Forward,
                reference_model: "base/base.mod".to_string(),
                reference_ofv: Some(100.0),
                candidates: vec![],
                winner: Some("WT_CL".to_string()),
                decision: "added WT_CL".to_string(),
                complete: true,
            },
            RoundRecord {
                name: "forward_round2".to_string(),
                direction: Direction::Forward,
                reference_model: "forward_round1/wt_cl/wt_cl.mod".to_string(),
                reference_ofv: Some(90.0),
                candidates: vec![],
                winner: Some("CRCL_CL".to_string()),
                decision: "added CRCL_CL".to_string(),
                complete: true,
            },
            RoundRecord {
                name: "forward_round3".to_string(),
                direction: Direction::Forward,
                reference_model: "forward_round2/crcl_cl/crcl_cl.mod".to_string(),
                reference_ofv: Some(80.0),
                candidates: vec![
                    {
                        let mut c = CandidateRecord::new("WT_V", "add WT_V".to_string(), 1);
                        c.status = CandidateStatus::Succeeded;
                        c
                    },
                    CandidateRecord::new("AGE_CL", "add AGE_CL".to_string(), 1),
                ],
                winner: None,
                decision: String::new(),
                complete: false,
            },
        ];
        state
    }

    #[test]
    fn a_fresh_out_dir_adds_nothing_to_the_plan() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template(dir.path());
        let out = dir.path().join("out");
        let built = build_plan(
            &model,
            &names(&["WT_CL", "CRCL_CL"]),
            Some(&out),
            ScmOptions::default(),
            "test",
        )
        .unwrap();

        assert!(built.context.is_empty());
        assert_eq!(built.render_text(), built.plan.render_text());
        assert!(!built.render_text().contains("progress"));
        assert!(!built.render_text().contains("changes"));
    }

    #[test]
    fn replanning_an_unchanged_plan_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template(dir.path());
        let out = dir.path().join("out");
        plan_for(&model, &["WT_CL", "CRCL_CL"], ScmOptions::default(), &out)
            .save()
            .unwrap();

        let built = build_plan(
            &model,
            &names(&["WT_CL", "CRCL_CL"]),
            Some(&out),
            ScmOptions::default(),
            "test",
        )
        .unwrap();
        let text = built.render_text();

        assert!(built.context.changes.is_empty());
        assert!(
            text.contains("changes    : none — identical to the previous plan"),
            "got:\n{text}"
        );
        // A plan written but never run has no progress to report.
        assert!(text.contains("progress   : not started"), "got:\n{text}");
    }

    #[test]
    fn a_paused_scm_shows_where_it_got_to() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template(dir.path());
        let out = dir.path().join("out");
        let previous = plan_for(
            &model,
            &["WT_CL", "CRCL_CL", "WT_V"],
            ScmOptions::default(),
            &out,
        );
        previous.save().unwrap();
        mid_scm_state(&previous).save(&out).unwrap();

        let built = build_plan(
            &model,
            &names(&["WT_CL", "CRCL_CL", "WT_V"]),
            Some(&out),
            ScmOptions::default(),
            "test",
        )
        .unwrap();
        let text = built.render_text();

        let progress = built.context.progress.as_ref().unwrap();
        assert_eq!(progress.rounds_complete, 2);
        assert_eq!(progress.retained, vec!["WT_CL", "CRCL_CL"]);
        assert!(
            text.contains("progress   : paused — 2 rounds complete, forward phase"),
            "got:\n{text}"
        );
        assert!(text.contains("selected   : WT_CL, CRCL_CL"), "got:\n{text}");
        assert!(
            text.contains("in round   : forward_round3 — 1/2 concluded"),
            "got:\n{text}"
        );
        // Nothing about the SCM process changed, so nothing threatens the state.
        assert!(!built.context.state_is_stale);
        assert!(!text.contains("cannot resume"), "got:\n{text}");
    }

    #[test]
    fn a_scm_defining_change_lists_it_and_warns_the_state_is_stale() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template(dir.path());
        let out = dir.path().join("out");
        let previous = plan_for(&model, &["WT_CL", "CRCL_CL"], ScmOptions::default(), &out);
        previous.save().unwrap();
        mid_scm_state(&previous).save(&out).unwrap();

        // Add a candidate, tighten the forward alpha, and drop the round cap
        // in — three changes, only the first two SCM-defining.
        let options = ScmOptions {
            forward_alpha: 0.01,
            num_rounds: Some(2),
            ..Default::default()
        };
        let built = build_plan(
            &model,
            &names(&["WT_CL", "CRCL_CL", "WT_V"]),
            Some(&out),
            options,
            "test",
        )
        .unwrap();
        let text = built.render_text();

        let fields: Vec<&str> = built
            .context
            .changes
            .iter()
            .map(|c| c.field.as_str())
            .collect();
        assert_eq!(fields, vec!["candidates", "forward_alpha", "num_rounds"]);
        assert!(text.contains("added WT_V THETA(6)"), "got:\n{text}");
        assert!(text.contains("0.05 -> 0.01"), "got:\n{text}");
        assert!(text.contains("no cap -> 2"), "got:\n{text}");
        assert!(text.contains("(SCM-defining)"), "got:\n{text}");

        assert!(built.context.state_is_stale);
        assert!(text.contains("cannot resume"), "got:\n{text}");
        assert!(
            text.contains("WT_V is not part of this SCM process"),
            "got:\n{text}"
        );
        assert!(text.contains("re-plan with overwrite"), "got:\n{text}");
    }

    /// Dropping a candidate that never won is not SCM-defining: the plan
    /// says the SCM process carries on without it. Dropping a winner is.
    #[test]
    fn removing_a_loser_keeps_the_state_and_removing_a_winner_does_not() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template(dir.path());
        let out = dir.path().join("out");
        let previous = plan_for(
            &model,
            &["WT_CL", "CRCL_CL", "WT_V"],
            ScmOptions::default(),
            &out,
        );
        previous.save().unwrap();
        // WT_CL and CRCL_CL won rounds 1 and 2; WT_V is still being tested
        mid_scm_state(&previous).save(&out).unwrap();

        let built = build_plan(
            &model,
            &names(&["WT_CL", "CRCL_CL"]),
            Some(&out),
            ScmOptions::default(),
            "test",
        )
        .unwrap();
        let text = built.render_text();
        assert_eq!(built.context.changes.len(), 1);
        assert!(!built.context.changes[0].scm_defining);
        assert!(!built.context.state_is_stale);
        assert_eq!(built.context.removals, vec!["WT_V".to_string()]);
        assert!(text.contains("removed WT_V THETA(6)"), "got:\n{text}");
        assert!(!text.contains("(SCM-defining)"), "got:\n{text}");
        assert!(
            text.contains("removing   : WT_V — never selected; takes effect from the next round"),
            "got:\n{text}"
        );
        assert!(!text.contains("cannot resume"), "got:\n{text}");

        let built = build_plan(
            &model,
            &names(&["CRCL_CL", "WT_V"]),
            Some(&out),
            ScmOptions::default(),
            "test",
        )
        .unwrap();
        let text = built.render_text();
        assert!(built.context.state_is_stale);
        assert!(
            text.contains("removed WT_CL THETA(4) — selected in forward_round1  (SCM-defining)"),
            "got:\n{text}"
        );
        assert!(text.contains("cannot resume"), "got:\n{text}");
    }

    #[test]
    fn a_candidates_new_initial_estimate_is_listed_as_a_change() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template(dir.path());
        let out = dir.path().join("out");
        let previous = plan_for(&model, &["WT_CL"], ScmOptions::default(), &out);
        previous.save().unwrap();
        mid_scm_state(&previous).save(&out).unwrap();

        // The template now carries a real initial guess for the effect, so
        // every model that tests it starts somewhere else.
        let edited = crate::scm::plan::tests::TEMPLATE
            .replace("$THETA (0 FIX)   ; WT_CL cov", "$THETA 0.4   ; WT_CL cov");
        crate::scm::plan::tests::write_template_content(dir.path(), &edited);

        let built = build_plan(
            &model,
            &names(&["WT_CL"]),
            Some(&out),
            ScmOptions::default(),
            "test",
        )
        .unwrap();
        let text = built.render_text();
        assert!(
            text.contains("WT_CL starts at 0.1 -> 0.4 when first tested"),
            "got:\n{text}"
        );
        assert!(built.context.state_is_stale);
    }

    #[test]
    fn a_run_control_change_alone_leaves_the_state_resumable() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template(dir.path());
        let out = dir.path().join("out");
        let previous = plan_for(&model, &["WT_CL", "CRCL_CL"], ScmOptions::default(), &out);
        previous.save().unwrap();
        mid_scm_state(&previous).save(&out).unwrap();

        let options = ScmOptions {
            num_rounds: Some(1),
            ..Default::default()
        };
        let built = build_plan(
            &model,
            &names(&["WT_CL", "CRCL_CL"]),
            Some(&out),
            options,
            "test",
        )
        .unwrap();
        let text = built.render_text();

        assert_eq!(built.context.changes.len(), 1);
        assert!(!built.context.changes[0].scm_defining);
        assert!(!built.context.state_is_stale);
        assert!(!text.contains("(SCM-defining)"), "got:\n{text}");
        assert!(!text.contains("cannot resume"), "got:\n{text}");
    }

    #[test]
    fn removed_and_moved_candidates_are_both_reported() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template(dir.path());
        let out = dir.path().join("out");
        let mut previous = plan_for(&model, &["WT_CL", "CRCL_CL"], ScmOptions::default(), &out);
        // Pretend the template used to carry WT_CL one theta earlier.
        previous.candidates[0].theta = 3;
        previous.save().unwrap();

        let built = build_plan(
            &model,
            &names(&["WT_CL"]),
            Some(&out),
            ScmOptions::default(),
            "test",
        )
        .unwrap();
        let text = built.render_text();

        assert!(
            text.contains("WT_CL moved THETA(3) -> THETA(4)"),
            "got:\n{text}"
        );
        // no state behind the plan: a removal is a plain change
        assert!(text.contains("removed CRCL_CL THETA(5)"), "got:\n{text}");
        assert!(!text.contains("removing   :"), "got:\n{text}");
    }

    #[test]
    fn plan_schema_version_is_untouched_by_the_context() {
        // The context is a rendering, never part of plan.json.
        let dir = tempfile::tempdir().unwrap();
        let model = write_template(dir.path());
        let out = dir.path().join("out");
        let plan = plan_for(&model, &["WT_CL"], ScmOptions::default(), &out);
        assert_eq!(plan.schema_version, PLAN_SCHEMA_VERSION);
        let json = plan.to_json().unwrap();
        assert!(!json.contains("progress"));
        assert!(!json.contains("changes"));
    }
}
