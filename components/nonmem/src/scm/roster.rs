//! The candidate roster: every candidate an SCM process has known, and
//! whether a re-planned candidate list can still resume the state on disk.

use serde::{Deserialize, Serialize};
use utils::get_utc_now;

use super::state::{CandidateStatus, ScmState};
use super::{Candidate, ScmPlan};

/// When a removal or a retune happened, as both report it: `after
/// forward_round2`, or `before the first round` when no round had concluded.
pub fn when_label(after_round: &Option<String>) -> String {
    match after_round {
        Some(r) => format!("after {r}"),
        None => "before the first round".to_string(),
    }
}

/// One candidate as the SCM process knows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RosterEntry {
    /// The candidate with the values in force now (`theta`, `initial`,
    /// `fixed`, bounds); a retune replaces them and is recorded below.
    #[serde(flatten)]
    pub candidate: Candidate,
    /// Set once the candidate has been removed from the plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub removed: Option<Removal>,
    /// Every time the plan moved this candidate's initial estimate or
    /// bounds, oldest first. `candidate` above always carries the values in
    /// force now.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retunes: Vec<Retune>,
}

impl RosterEntry {
    pub fn active(candidate: &Candidate) -> Self {
        Self {
            candidate: candidate.clone(),
            removed: None,
            retunes: vec![],
        }
    }

    /// `AGE_CL (after forward_round2)` / `AGE_CL (before the first round)`.
    pub fn removal_label(&self) -> String {
        match &self.removed {
            Some(r) => format!("{} ({})", self.candidate.name, when_label(&r.after_round)),
            None => self.candidate.name.clone(),
        }
    }

    /// `WT_CL (bounds (0, INF) -> (0, 2), after forward_round1)`, listing
    /// the most recent retune; `None` when the candidate never had one.
    pub fn retune_label(&self) -> Option<String> {
        let last = self.retunes.last()?;
        Some(format!(
            "{} ({}, {})",
            self.candidate.name,
            last.changes.join("; "),
            when_label(&last.after_round)
        ))
    }
}

/// When a candidate left the SCM process.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Removal {
    /// The last round that had concluded when the candidate was removed;
    /// `None` when no SCM round had concluded yet.
    pub after_round: Option<String>,
    /// Timestamp of the removal.
    pub at: String,
}

/// A change the plan made to a candidate's initial estimate or bounds while
/// the SCM process was under way. The values themselves live on the roster
/// entry's candidate; this is the record of what moved and when.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Retune {
    /// The last round that had concluded when the values changed; `None`
    /// when no SCM round had concluded yet.
    pub after_round: Option<String>,
    /// Timestamp of the change.
    pub at: String,
    /// One line per value that moved, e.g.
    /// `bounds (0, INF) -> (0, 2)`, `initial 0.1 -> 0.5`.
    pub changes: Vec<String>,
}

/// A candidate the plan retunes: the values it now carries, and what moved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Retuning {
    /// The candidate as the plan now gives it.
    pub candidate: Candidate,
    /// One line per value that moved, in the order the plan lists them.
    pub changes: Vec<String>,
}

impl Retuning {
    pub fn name(&self) -> &str {
        &self.candidate.name
    }

    /// `WT_CL: bounds (0, INF) -> (0, 2)`.
    pub fn label(&self) -> String {
        format!("{}: {}", self.candidate.name, self.changes.join("; "))
    }
}

/// One way a candidate list differs from the one before it. The plan
/// rendering and the resume check both walk the same diff and only word it
/// differently.
#[derive(Debug, Clone, PartialEq)]
pub enum CandidateChange {
    Added {
        name: String,
        theta: usize,
    },
    Removed {
        name: String,
        theta: usize,
    },
    MovedTheta {
        name: String,
        from: usize,
        to: usize,
    },
    HeldOutAt {
        name: String,
        from: f64,
        to: f64,
    },
    Initial {
        name: String,
        from: f64,
        to: f64,
    },
    Bounds {
        name: String,
        from: Option<String>,
        to: Option<String>,
    },
}

impl CandidateChange {
    pub fn name(&self) -> &str {
        match self {
            CandidateChange::Added { name, .. }
            | CandidateChange::Removed { name, .. }
            | CandidateChange::MovedTheta { name, .. }
            | CandidateChange::HeldOutAt { name, .. }
            | CandidateChange::Initial { name, .. }
            | CandidateChange::Bounds { name, .. } => name,
        }
    }

    /// Whether the change only retunes a candidate (its initial estimate or
    /// bounds), which an SCM process resumes under, rather than redefining
    /// it.
    pub fn is_retune(&self) -> bool {
        matches!(
            self,
            CandidateChange::Initial { .. } | CandidateChange::Bounds { .. }
        )
    }

    /// The change without the candidate's name, as the roster records a
    /// retune: `initial 0.1 -> 0.5`, `bounds none -> (0, 2)`.
    pub fn label(&self) -> String {
        let none = || "none".to_string();
        match self {
            CandidateChange::Added { theta, .. } => format!("added THETA({theta})"),
            CandidateChange::Removed { theta, .. } => format!("removed THETA({theta})"),
            CandidateChange::MovedTheta { from, to, .. } => {
                format!("moved THETA({from}) -> THETA({to})")
            }
            CandidateChange::HeldOutAt { from, to, .. } => format!("held out at {from} -> {to}"),
            CandidateChange::Initial { from, to, .. } => format!("initial {from} -> {to}"),
            CandidateChange::Bounds { from, to, .. } => format!(
                "bounds {} -> {}",
                from.clone().unwrap_or_else(none),
                to.clone().unwrap_or_else(none)
            ),
        }
    }
}

/// Every way `next` differs from `prev`, candidates matched by name: the
/// changes to `next`'s candidates in its order, then `prev`'s candidates it
/// no longer lists. A name that moved thetas is a change, not a new
/// candidate.
pub fn diff_candidates(prev: &[Candidate], next: &[Candidate]) -> Vec<CandidateChange> {
    let find = |list: &[Candidate], name: &str| list.iter().find(|c| c.name == name).cloned();
    let mut changes = Vec::new();
    for c in next {
        let name = c.name.clone();
        let Some(old) = find(prev, &c.name) else {
            changes.push(CandidateChange::Added {
                name,
                theta: c.theta,
            });
            continue;
        };
        if old.theta != c.theta {
            changes.push(CandidateChange::MovedTheta {
                name: name.clone(),
                from: old.theta,
                to: c.theta,
            });
        }
        if old.fixed != c.fixed {
            changes.push(CandidateChange::HeldOutAt {
                name: name.clone(),
                from: old.fixed,
                to: c.fixed,
            });
        }
        if old.initial != c.initial {
            changes.push(CandidateChange::Initial {
                name: name.clone(),
                from: old.initial,
                to: c.initial,
            });
        }
        if (old.lower, old.upper) != (c.lower, c.upper) {
            changes.push(CandidateChange::Bounds {
                name,
                from: old.bounds_label(),
                to: c.bounds_label(),
            });
        }
    }
    for c in prev {
        if find(next, &c.name).is_none() {
            changes.push(CandidateChange::Removed {
                name: c.name.clone(),
                theta: c.theta,
            });
        }
    }
    changes
}

/// Whether a plan can pick up the SCM process a state describes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Compatibility {
    /// Same options, same candidates, same values: resume as-is.
    Identical,
    /// Same options; the plan dropped candidates that never won a round,
    /// retuned candidates' initial estimates or bounds, or both. Resume
    /// after recording them.
    Compatible {
        removals: Vec<String>,
        retunes: Vec<Retuning>,
    },
    /// The state cannot resume under this plan without `overwrite`.
    Incompatible {
        /// One line per reason, in candidate order.
        reasons: Vec<String>,
        /// Removals that would have been fine on their own, listed so the
        /// rendering can still call them out.
        removals: Vec<String>,
        /// Retunes that would have been fine on their own, likewise.
        retunes: Vec<Retuning>,
    },
}

impl Compatibility {
    pub fn is_incompatible(&self) -> bool {
        matches!(self, Compatibility::Incompatible { .. })
    }
}

/// Compare `plan` with the SCM process `state` describes.
pub fn compatibility(plan: &ScmPlan, state: &ScmState) -> Compatibility {
    let mut reasons = Vec::new();
    let mut removals = Vec::new();
    let mut retunes = Vec::new();

    if state.plan_digest != plan.digest() {
        reasons.push(
            "the plan's model, direction, alphas, retries, cov step or final re-fit differ from \
             the ones this SCM process ran under"
                .to_string(),
        );
    }

    // The candidates the SCM process still tracks, against the plan's.
    let known: Vec<Candidate> = state.active_roster().map(|e| e.candidate.clone()).collect();
    for change in diff_candidates(&known, &plan.candidates) {
        let name = change.name().to_string();
        match &change {
            CandidateChange::Removed { .. } => match state.depends_on(&name) {
                Some(why) => reasons.push(format!(
                    "{name} was {why}; the rounds after it were built on it, so removing it needs overwrite"
                )),
                None => removals.push(name),
            },
            CandidateChange::Added { .. } => match state.roster_entry(&name) {
                Some(entry) if entry.removed.is_some() => reasons.push(format!(
                    "{name} was removed {}; adding it back needs overwrite",
                    when_label(&entry.removed.as_ref().unwrap().after_round)
                )),
                _ => reasons.push(format!(
                    "{name} is not part of this SCM process; adding a candidate needs overwrite"
                )),
            },
            CandidateChange::MovedTheta { .. } => reasons.push(format!(
                "{name} {}; the initial model changed under the SCM process",
                change.label()
            )),
            CandidateChange::HeldOutAt { from, to, .. } => reasons.push(format!(
                "{name} is held out at {from} -> {to}; changing a candidate's FIXED value needs overwrite"
            )),
            CandidateChange::Initial { .. } | CandidateChange::Bounds { .. } => {
                let candidate = plan
                    .candidates
                    .iter()
                    .find(|c| c.name == name)
                    .expect("a retuned candidate is in the plan")
                    .clone();
                match retunes.iter_mut().find(|r: &&mut Retuning| r.name() == name) {
                    Some(r) => r.changes.push(change.label()),
                    None => retunes.push(Retuning {
                        candidate,
                        changes: vec![change.label()],
                    }),
                }
            }
        }
    }

    if !reasons.is_empty() {
        Compatibility::Incompatible {
            reasons,
            removals,
            retunes,
        }
    } else if removals.is_empty() && retunes.is_empty() {
        Compatibility::Identical
    } else {
        Compatibility::Compatible { removals, retunes }
    }
}

pub fn apply_removals(state: &mut ScmState, removals: &[String]) -> Vec<String> {
    let after_round = last_concluded_round(state);
    let at = get_utc_now();
    let mut lines = Vec::new();

    for name in removals {
        if let Some(entry) = state
            .roster
            .iter_mut()
            .find(|e| &e.candidate.name == name && e.removed.is_none())
        {
            entry.removed = Some(Removal {
                after_round: after_round.clone(),
                at: at.clone(),
            });
        }

        let mut line = format!("removed {name}");
        if let Some(round) = state
            .rounds
            .iter_mut()
            .find(|r| !r.complete && !r.is_reference())
            && let Some(cand) = round.candidates.iter_mut().find(|c| &c.candidate == name)
        {
            cand.status = CandidateStatus::Withdrawn;
            cand.significant = None;
            cand.selected = false;
            line.push_str(&format!(
                "; withdrawn from {} (recorded, not scored)",
                round.name
            ));
        }
        lines.push(line);
    }

    if let Some(tie) = &state.pending_tie
        && tie.candidates.iter().any(|c| removals.contains(c))
    {
        let round_name = tie.round.clone();
        state.pending_tie = None;
        if let Some(round) = state.find_round_mut(&round_name) {
            round.decision.clear();
        }
        state.message = None;
        lines.push(format!(
            "the tie in {round_name} dissolved with the removal"
        ));
    }

    lines
}

fn last_concluded_round(state: &ScmState) -> Option<String> {
    state
        .rounds
        .iter()
        .rev()
        .find(|r| r.complete && !r.is_reference())
        .map(|r| r.name.clone())
}

pub fn apply_retunes(state: &mut ScmState, retunes: &[Retuning]) -> Vec<String> {
    let after_round = last_concluded_round(state);
    let at = get_utc_now();
    let mut lines = Vec::new();
    let mut refitting_in: Option<String> = None;

    for retuning in retunes {
        let name = retuning.name().to_string();
        if let Some(entry) = state
            .roster
            .iter_mut()
            .find(|e| e.candidate.name == name && e.removed.is_none())
        {
            entry.candidate = retuning.candidate.clone();
            entry.retunes.push(Retune {
                after_round: after_round.clone(),
                at: at.clone(),
                changes: retuning.changes.clone(),
            });
        }

        let mut line = retuning.label();
        if let Some(round) = state
            .rounds
            .iter_mut()
            .find(|r| !r.complete && !r.is_reference())
        {
            let round_name = round.name.clone();
            if let Some(cand) = round.candidates.iter_mut().find(|c| c.candidate == name)
                && cand.status != CandidateStatus::Withdrawn
            {
                let so_far = cand.n_attempts();
                cand.refit_under_new_values();
                line.push_str(&format!("; refitting in {round_name} under the new values"));
                if so_far > 0 {
                    line.push_str(&format!(
                        " ({so_far} attempt(s) so far kept on record, superseded)"
                    ));
                }
                refitting_in = Some(round_name);
            }
        }
        lines.push(line);
    }

    if let Some(round_name) = refitting_in {
        if let Some(tie) = &state.pending_tie
            && tie.round == round_name
        {
            state.pending_tie = None;
            lines.push(format!(
                "the decision awaited in {round_name} is deferred until the refit is scored"
            ));
        }
        if let Some(round) = state.find_round_mut(&round_name) {
            round.decision.clear();
            round.winner = None;
        }
        state.message = None;
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scm::state::{CandidateRecord, PendingTie, RoundRecord};
    use crate::scm::test_support::{make_plan, names, write_template};
    use crate::scm::{Direction, ScmOptions, build_plan};

    /// A state after one forward round: WT_CL won, the other two lost.
    fn state_after_round_one(plan: &ScmPlan) -> ScmState {
        let mut state = ScmState::new(plan);
        state.retained = vec!["WT_CL".to_string()];
        state.phase = Some(Direction::Forward);
        state.rounds.push(RoundRecord {
            name: "forward_round1".into(),
            direction: Direction::Forward,
            reference_model: "base/1001_base.mod".into(),
            reference_ofv: Some(1000.0),
            candidates: vec![],
            winner: Some("WT_CL".into()),
            decision: "added WT_CL".into(),
            complete: true,
        });
        state
    }

    fn replan(dir: &std::path::Path, plan: &ScmPlan, cands: &[&str]) -> ScmPlan {
        build_plan(
            &plan.model_path(),
            &names(cands),
            Some(&plan.out_dir_path()),
            plan.options.clone(),
            "test",
        )
        .unwrap()
        .plan
        .tap(|_| {
            let _ = dir;
        })
    }

    trait Tap: Sized {
        fn tap(self, f: impl FnOnce(&Self)) -> Self {
            f(&self);
            self
        }
    }
    impl<T> Tap for T {}

    #[test]
    fn the_same_plan_is_identical() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        let state = state_after_round_one(&plan);
        assert_eq!(compatibility(&plan, &state), Compatibility::Identical);
    }

    #[test]
    fn dropping_a_loser_is_compatible_and_dropping_the_winner_is_not() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        let state = state_after_round_one(&plan);

        let fewer = replan(dir.path(), &plan, &["WT_CL", "CRCL_CL"]);
        assert_eq!(
            compatibility(&fewer, &state),
            Compatibility::Compatible {
                removals: vec!["WT_V".to_string()],
                retunes: vec![]
            }
        );

        let no_winner = replan(dir.path(), &plan, &["CRCL_CL", "WT_V"]);
        match compatibility(&no_winner, &state) {
            Compatibility::Incompatible {
                reasons, removals, ..
            } => {
                assert_eq!(reasons.len(), 1);
                assert!(
                    reasons[0].contains("WT_CL was selected in forward_round1"),
                    "{reasons:?}"
                );
                assert!(removals.is_empty());
            }
            other => panic!("expected incompatible, got {other:?}"),
        }
    }

    #[test]
    fn additions_option_changes_and_a_new_off_value_are_incompatible() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        // The state only ever knew WT_CL and CRCL_CL.
        let mut two = plan.clone();
        two.candidates.retain(|c| c.name != "WT_V");
        let state = state_after_round_one(&two);

        match compatibility(&plan, &state) {
            Compatibility::Incompatible { reasons, .. } => {
                assert!(
                    reasons[0].contains("WT_V is not part of this SCM process"),
                    "{reasons:?}"
                );
            }
            other => panic!("{other:?}"),
        }

        let mut alpha = two.clone();
        alpha.options.forward_alpha = 0.01;
        assert!(compatibility(&alpha, &state).is_incompatible());

        let mut off = two.clone();
        off.candidates[0].fixed = 1.0;
        match compatibility(&off, &state) {
            Compatibility::Incompatible { reasons, .. } => {
                assert!(reasons[0].contains("held out at 0 -> 1"), "{reasons:?}");
            }
            other => panic!("{other:?}"),
        }

        // a removal that would be fine on its own is still listed alongside
        let mut mixed = two.clone();
        mixed.candidates.retain(|c| c.name != "CRCL_CL");
        mixed.options.max_retries = 9;
        match compatibility(&mixed, &state) {
            Compatibility::Incompatible { removals, .. } => {
                assert_eq!(removals, vec!["CRCL_CL".to_string()]);
            }
            other => panic!("{other:?}"),
        }
    }

    /// A candidate whose initial estimate or bounds moved is a retune, not a
    /// different SCM process: a round that failed on either can be fixed in
    /// the config and resumed.
    #[test]
    fn retuning_an_initial_estimate_or_bounds_is_compatible() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        let state = state_after_round_one(&plan);

        let mut retuned = plan.clone();
        retuned.candidates[1].initial = 0.5;
        retuned.candidates[1].lower = Some(0.0);
        retuned.candidates[1].upper = Some(2.0);
        match compatibility(&retuned, &state) {
            Compatibility::Compatible { removals, retunes } => {
                assert!(removals.is_empty());
                assert_eq!(retunes.len(), 1);
                assert_eq!(retunes[0].name(), "CRCL_CL");
                assert_eq!(
                    retunes[0].changes,
                    vec![
                        "initial 0.1 -> 0.5".to_string(),
                        "bounds none -> (0, 2)".to_string()
                    ]
                );
            }
            other => panic!("{other:?}"),
        }

        // Even for the candidate that won a round: its earlier rounds stand,
        // and the new values bear only on models still to be written.
        let mut winner = plan.clone();
        winner.candidates[0].upper = Some(3.0);
        match compatibility(&winner, &state) {
            Compatibility::Compatible { retunes, .. } => {
                assert_eq!(retunes[0].name(), "WT_CL");
            }
            other => panic!("{other:?}"),
        }

        // A retune alongside a change that does need overwrite is reported
        // with it, so the rendering can still name it.
        let mut with_alpha = retuned.clone();
        with_alpha.options.forward_alpha = 0.01;
        match compatibility(&with_alpha, &state) {
            Compatibility::Incompatible { retunes, .. } => {
                assert_eq!(retunes[0].name(), "CRCL_CL");
            }
            other => panic!("{other:?}"),
        }
    }

    /// Applying a retune: the roster takes the new values and keeps the
    /// change on record, and the candidate the open round is still testing
    /// is set up to refit under them without losing what it already ran.
    #[test]
    fn applying_a_retune_records_it_and_refits_the_open_round_only() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        let mut state = state_after_round_one(&plan);
        state.rounds.push(RoundRecord {
            name: "forward_round2".into(),
            direction: Direction::Forward,
            reference_model: "forward_round1/1001_wt_cl.mod".into(),
            reference_ofv: Some(990.0),
            candidates: vec![
                {
                    let mut c = CandidateRecord::new("CRCL_CL", "add CRCL_CL".into(), 1);
                    c.status = CandidateStatus::Unusable;
                    c.model = "forward_round2/1001_crcl_cl.mod".into();
                    c.attempts = vec![crate::scm::state::AttemptRecord {
                        model: "forward_round2/1001_crcl_cl.mod".into(),
                        outcome: "minimization terminated".into(),
                    }];
                    c
                },
                {
                    let mut c = CandidateRecord::new("WT_V", "add WT_V".into(), 1);
                    c.status = CandidateStatus::Succeeded;
                    c.ofv = Some(980.0);
                    c
                },
            ],
            winner: None,
            decision: String::new(),
            complete: false,
        });

        let retuning = Retuning {
            candidate: Candidate {
                lower: Some(0.0),
                upper: Some(2.0),
                ..plan.candidates[1].clone()
            },
            changes: vec!["bounds none -> (0, 2)".to_string()],
        };
        let lines = apply_retunes(&mut state, &[retuning]);
        assert_eq!(
            lines,
            vec![
                "CRCL_CL: bounds none -> (0, 2); refitting in forward_round2 under the new \
                 values (1 attempt(s) so far kept on record, superseded)"
                    .to_string()
            ]
        );

        // the roster carries the new values, with the change dated
        let entry = state.roster_entry("CRCL_CL").unwrap();
        assert_eq!(entry.candidate.upper, Some(2.0));
        assert_eq!(entry.retunes.len(), 1);
        assert_eq!(
            entry.retunes[0].after_round.as_deref(),
            Some("forward_round1")
        );
        assert_eq!(
            entry.retune_label().unwrap(),
            "CRCL_CL (bounds none -> (0, 2), after forward_round1)"
        );

        // the open round refits it, keeping the attempt it already made
        let round = state.open_round().unwrap();
        let cand = &round.candidates[0];
        assert_eq!(cand.status, CandidateStatus::Pending);
        assert_eq!(cand.refit, 1);
        assert_eq!(cand.n_attempts(), 0);
        assert_eq!(cand.superseded.len(), 1);
        // nothing else in the round is disturbed
        assert_eq!(round.candidates[1].status, CandidateStatus::Succeeded);
        // and the concluded round is untouched
        assert!(state.rounds[0].complete);
        assert_eq!(state.rounds[0].winner.as_deref(), Some("WT_CL"));
    }

    /// A retune of a candidate no open round holds changes the values for
    /// models still to be written and nothing else.
    #[test]
    fn a_retune_with_no_open_round_only_records_the_new_values() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        let mut state = state_after_round_one(&plan);

        let retuning = Retuning {
            candidate: Candidate {
                initial: 0.5,
                ..plan.candidates[0].clone()
            },
            changes: vec!["initial 0.1 -> 0.5".to_string()],
        };
        let lines = apply_retunes(&mut state, &[retuning]);
        assert_eq!(lines, vec!["WT_CL: initial 0.1 -> 0.5".to_string()]);
        assert_eq!(state.roster_entry("WT_CL").unwrap().candidate.initial, 0.5);
        assert!(state.rounds.iter().all(|r| r.complete));
    }

    #[test]
    fn a_retained_candidate_of_a_backward_only_process_cannot_be_removed() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(
            dir.path(),
            ScmOptions {
                direction: vec![Direction::Backward],
                ..Default::default()
            },
        );
        let mut state = ScmState::new(&plan);
        // the full model released everything; nothing has "won"
        state.retained = plan.candidates.iter().map(|c| c.name.clone()).collect();
        let fewer = replan(dir.path(), &plan, &["WT_CL", "CRCL_CL"]);
        match compatibility(&fewer, &state) {
            Compatibility::Incompatible { reasons, .. } => {
                assert!(
                    reasons[0].contains("WT_V was in the current model"),
                    "{reasons:?}"
                );
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn applying_a_removal_withdraws_from_the_open_round_and_dissolves_a_tie() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        let mut state = state_after_round_one(&plan);
        // round 2 is open and paused on a tie between CRCL_CL and WT_V
        let mut crcl = CandidateRecord::new("CRCL_CL", "add CRCL_CL".into(), 1);
        crcl.status = CandidateStatus::Succeeded;
        crcl.ofv = Some(970.0);
        crcl.significant = Some(true);
        let mut wt_v = CandidateRecord::new("WT_V", "add WT_V".into(), 1);
        wt_v.status = CandidateStatus::Succeeded;
        wt_v.ofv = Some(970.0);
        wt_v.significant = Some(true);
        state.rounds.push(RoundRecord {
            name: "forward_round2".into(),
            direction: Direction::Forward,
            reference_model: "forward_round1/1001_wt_cl.mod".into(),
            reference_ofv: Some(980.0),
            candidates: vec![crcl, wt_v],
            winner: None,
            decision: "tie between CRCL_CL, WT_V".into(),
            complete: false,
        });
        state.pending_tie = Some(PendingTie {
            round: "forward_round2".into(),
            direction: Direction::Forward,
            candidates: vec!["CRCL_CL".into(), "WT_V".into()],
            p_value: 0.001,
            delta_ofv: -10.0,
        });

        let lines = apply_removals(&mut state, &["WT_V".to_string()]);
        assert!(
            lines[0].contains("withdrawn from forward_round2"),
            "{lines:?}"
        );
        assert!(
            lines[1].contains("tie in forward_round2 dissolved"),
            "{lines:?}"
        );

        let entry = state.roster_entry("WT_V").unwrap();
        let removal = entry.removed.as_ref().unwrap();
        assert_eq!(removal.after_round.as_deref(), Some("forward_round1"));
        assert_eq!(entry.removal_label(), "WT_V (after forward_round1)");

        let round = state.open_round().unwrap();
        let wt_v = round
            .candidates
            .iter()
            .find(|c| c.candidate == "WT_V")
            .unwrap();
        assert_eq!(wt_v.status, CandidateStatus::Withdrawn);
        assert_eq!(wt_v.ofv, Some(970.0)); // the fit is kept for the record
        assert!(state.pending_tie.is_none());
        assert!(round.decision.is_empty());

        // once removed, the plan without it is identical to the state
        let fewer = replan(dir.path(), &plan, &["WT_CL", "CRCL_CL"]);
        assert_eq!(compatibility(&fewer, &state), Compatibility::Identical);
        // and adding it back is an addition
        assert!(compatibility(&plan, &state).is_incompatible());
    }

    #[test]
    fn a_removal_before_any_round_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template(dir.path());
        let plan = build_plan(
            &model,
            &names(&["WT_CL", "WT_V"]),
            None,
            ScmOptions::default(),
            "test",
        )
        .unwrap()
        .plan;
        let mut state = ScmState::new(&plan);
        apply_removals(&mut state, &["WT_V".to_string()]);
        let entry = state.roster_entry("WT_V").unwrap();
        assert_eq!(entry.removal_label(), "WT_V (before the first round)");
        assert_eq!(state.active_roster().count(), 1);
        assert_eq!(state.removed_roster().count(), 1);
    }
}
