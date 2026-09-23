//! The candidate roster: every candidate an SCM process has known, and
//! whether a re-planned candidate list can still resume the state on disk.

use serde::{Deserialize, Serialize};
use utils::get_utc_now;

use super::state::{CandidateStatus, ScmState};
use super::{Candidate, ScmPlan};

/// When a removal or a retune happened
pub fn when_label(after_round: &Option<String>) -> String {
    match after_round {
        Some(r) => format!("after {r}"),
        None => "before the first round".to_string(),
    }
}

/// One candidate as the SCM process knows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RosterEntry {
    #[serde(flatten)]
    pub candidate: Candidate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub removed: Option<Removal>,
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

    pub fn removal_label(&self) -> String {
        match &self.removed {
            Some(r) => format!("{} ({})", self.candidate.name, when_label(&r.after_round)),
            None => self.candidate.name.clone(),
        }
    }

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
    pub after_round: Option<String>,
    /// Timestamp of the removal.
    pub at: String,
}

/// A change the plan made to a candidate's initial estimate or bounds while
/// the SCM process was under way
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Retune {
    pub after_round: Option<String>,
    /// Timestamp of the change.
    pub at: String,
    pub changes: Vec<String>,
}

/// A candidate the plan retunes: the values it now carries, and what moved.
#[derive(Debug, Clone, PartialEq)]
pub struct Retuning {
    pub candidate: Candidate,
    pub changes: Vec<String>,
}

impl Retuning {
    pub fn name(&self) -> &str {
        &self.candidate.name
    }

    pub fn label(&self) -> String {
        format!("{}: {}", self.candidate.name, self.changes.join("; "))
    }
}

/// One way a candidate list differs from the one before it
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateChange {
    pub name: String,
    pub kind: ChangeKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeKind {
    Added {
        theta: usize,
    },
    Removed {
        theta: usize,
    },
    MovedTheta {
        from: usize,
        to: usize,
    },
    HeldOutAt {
        from: f64,
        to: f64,
    },
    Initial {
        from: f64,
        to: f64,
    },
    Bounds {
        from: Option<String>,
        to: Option<String>,
    },
}

impl CandidateChange {
    pub fn label(&self) -> String {
        let none = || "none".to_string();
        match &self.kind {
            ChangeKind::Added { theta } => format!("added THETA({theta})"),
            ChangeKind::Removed { theta } => format!("removed THETA({theta})"),
            ChangeKind::MovedTheta { from, to } => format!("moved THETA({from}) -> THETA({to})"),
            ChangeKind::HeldOutAt { from, to } => format!("held out at {from} -> {to}"),
            ChangeKind::Initial { from, to } => format!("initial {from} -> {to}"),
            ChangeKind::Bounds { from, to } => format!(
                "bounds {} -> {}",
                from.clone().unwrap_or_else(none),
                to.clone().unwrap_or_else(none)
            ),
        }
    }
}

/// Every way `next` differs from `prev` plan
pub fn diff_candidates(prev: &[Candidate], next: &[Candidate]) -> Vec<CandidateChange> {
    let find = |list: &[Candidate], name: &str| list.iter().find(|c| c.name == name).cloned();
    let mut changes = Vec::new();
    for c in next {
        let mut push = |kind| {
            changes.push(CandidateChange {
                name: c.name.clone(),
                kind,
            })
        };
        let Some(old) = find(prev, &c.name) else {
            push(ChangeKind::Added { theta: c.theta });
            continue;
        };
        if old.theta != c.theta {
            push(ChangeKind::MovedTheta {
                from: old.theta,
                to: c.theta,
            });
        }
        if old.fixed != c.fixed {
            push(ChangeKind::HeldOutAt {
                from: old.fixed,
                to: c.fixed,
            });
        }
        if old.initial != c.initial {
            push(ChangeKind::Initial {
                from: old.initial,
                to: c.initial,
            });
        }
        if (old.lower, old.upper) != (c.lower, c.upper) {
            push(ChangeKind::Bounds {
                from: old.bounds_label(),
                to: c.bounds_label(),
            });
        }
    }
    for c in prev {
        if find(next, &c.name).is_none() {
            changes.push(CandidateChange {
                name: c.name.clone(),
                kind: ChangeKind::Removed { theta: c.theta },
            });
        }
    }
    changes
}

/// Whether a plan can pick up the SCM process a state describes: it resumes
/// as-is when nothing below is set, after recording the removals and retunes
/// when only those are, and not at all (without `overwrite`) when there are
/// reasons.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Compatibility {
    /// Why the state cannot resume under this plan.
    pub reasons: Vec<String>,
    /// Candidates the plan dropped that never won a round.
    pub removals: Vec<String>,
    /// Candidates whose initial estimate or bounds the plan retunes.
    pub retunes: Vec<Retuning>,
}

impl Compatibility {
    pub fn is_incompatible(&self) -> bool {
        !self.reasons.is_empty()
    }

    /// Same options, same candidates, same values.
    pub fn is_identical(&self) -> bool {
        self.reasons.is_empty() && self.removals.is_empty() && self.retunes.is_empty()
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
        let name = change.name.clone();
        match &change.kind {
            ChangeKind::Removed { .. } => match state.depends_on(&name) {
                Some(why) => reasons.push(format!(
                    "{name} was {why}; the rounds after it were built on it, so removing it needs overwrite"
                )),
                None => removals.push(name),
            },
            ChangeKind::Added { .. } => match state.roster_entry(&name) {
                Some(entry) if entry.removed.is_some() => reasons.push(format!(
                    "{name} was removed {}; adding it back needs overwrite",
                    when_label(&entry.removed.as_ref().unwrap().after_round)
                )),
                _ => reasons.push(format!(
                    "{name} is not part of this SCM process; adding a candidate needs overwrite"
                )),
            },
            ChangeKind::MovedTheta { .. } => reasons.push(format!(
                "{name} {}; the initial model changed under the SCM process",
                change.label()
            )),
            ChangeKind::HeldOutAt { from, to } => reasons.push(format!(
                "{name} is held out at {from} -> {to}; changing a candidate's FIXED value needs overwrite"
            )),
            ChangeKind::Initial { .. } | ChangeKind::Bounds { .. } => {
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

    Compatibility {
        reasons,
        removals,
        retunes,
    }
}

pub fn apply_removals(state: &mut ScmState, removals: &[String]) -> Vec<String> {
    let after_round = last_concluded_round(state);
    let at = get_utc_now();
    let mut lines = Vec::new();

    for name in removals {
        if let Some(entry) = state.active_entry_mut(name) {
            entry.removed = Some(Removal {
                after_round: after_round.clone(),
                at: at.clone(),
            });
        }

        let mut line = format!("removed {name}");
        if let Some(round) = state.open_round_mut()
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
        if let Some(entry) = state.active_entry_mut(&name) {
            entry.candidate = retuning.candidate.clone();
            entry.retunes.push(Retune {
                after_round: after_round.clone(),
                at: at.clone(),
                changes: retuning.changes.clone(),
            });
        }

        let mut line = retuning.label();
        if let Some(round) = state.open_round_mut() {
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
        if let Some(round) = state.round_mut(&round_name) {
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
    use crate::scm::state::RoundRecord;
    use crate::scm::test_support::{make_plan, plan_of};
    use crate::scm::{Direction, ScmOptions};

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

    fn replan(plan: &ScmPlan, cands: &[&str]) -> ScmPlan {
        let (out_dir, options) = (plan.out_dir_path(), plan.options.clone());
        plan_of(&plan.model_path(), cands, Some(&out_dir), options)
    }

    #[test]
    fn dropping_a_loser_is_compatible_and_dropping_the_winner_is_not() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        let mut state = state_after_round_one(&plan);
        assert!(compatibility(&plan, &state).is_identical());

        let fewer = replan(&plan, &["WT_CL", "CRCL_CL"]);
        assert_eq!(
            compatibility(&fewer, &state),
            Compatibility {
                removals: vec!["WT_V".to_string()],
                ..Default::default()
            }
        );

        let no_winner = replan(&plan, &["CRCL_CL", "WT_V"]);
        let verdict = compatibility(&no_winner, &state);
        assert_eq!(verdict.reasons.len(), 1, "{verdict:?}");
        assert!(
            verdict.reasons[0].contains("WT_CL was selected in forward_round1"),
            "{verdict:?}"
        );
        assert!(verdict.removals.is_empty());

        // Applying the removal dates it to the round it followed, and the
        // plan without the candidate is then identical to the state
        apply_removals(&mut state, &["WT_V".to_string()]);
        assert_eq!(
            state.roster_entry("WT_V").unwrap().removal_label(),
            "WT_V (after forward_round1)"
        );
        assert_eq!(state.active_roster().count(), 2);
        assert!(compatibility(&fewer, &state).is_identical());
        assert!(compatibility(&plan, &state).is_incompatible());

        let mut fresh = ScmState::new(&plan);
        apply_removals(&mut fresh, &["WT_V".to_string()]);
        assert_eq!(
            fresh.roster_entry("WT_V").unwrap().removal_label(),
            "WT_V (before the first round)"
        );
    }

    #[test]
    fn additions_option_changes_and_a_new_off_value_are_incompatible() {
        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        // The state only ever knew WT_CL and CRCL_CL.
        let mut two = plan.clone();
        two.candidates.retain(|c| c.name != "WT_V");
        let state = state_after_round_one(&two);

        let reasons = compatibility(&plan, &state).reasons;
        assert!(
            reasons[0].contains("WT_V is not part of this SCM process"),
            "{reasons:?}"
        );

        let mut alpha = two.clone();
        alpha.options.forward_alpha = 0.01;
        assert!(compatibility(&alpha, &state).is_incompatible());

        let mut off = two.clone();
        off.candidates[0].fixed = 1.0;
        let reasons = compatibility(&off, &state).reasons;
        assert!(reasons[0].contains("held out at 0 -> 1"), "{reasons:?}");

        // a removal that would be fine on its own is still listed alongside
        let mut mixed = two.clone();
        mixed.candidates.retain(|c| c.name != "CRCL_CL");
        mixed.options.max_retries = 9;
        let verdict = compatibility(&mixed, &state);
        assert!(verdict.is_incompatible());
        assert_eq!(verdict.removals, vec!["CRCL_CL".to_string()]);
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
        let verdict = compatibility(&retuned, &state);
        assert!(!verdict.is_incompatible(), "{verdict:?}");
        assert!(verdict.removals.is_empty());
        assert_eq!(verdict.retunes.len(), 1);
        assert_eq!(verdict.retunes[0].name(), "CRCL_CL");
        assert_eq!(
            verdict.retunes[0].changes,
            vec![
                "initial 0.1 -> 0.5".to_string(),
                "bounds none -> (0, 2)".to_string()
            ]
        );

        // Even for the candidate that won a round: its earlier rounds stand,
        // and the new values bear only on models still to be written.
        let mut winner = plan.clone();
        winner.candidates[0].upper = Some(3.0);
        let verdict = compatibility(&winner, &state);
        assert!(!verdict.is_incompatible(), "{verdict:?}");
        assert_eq!(verdict.retunes[0].name(), "WT_CL");

        // A retune alongside a change that does need overwrite is reported
        // with it, so the rendering can still name it.
        let mut with_alpha = retuned.clone();
        with_alpha.options.forward_alpha = 0.01;
        let verdict = compatibility(&with_alpha, &state);
        assert!(verdict.is_incompatible());
        assert_eq!(verdict.retunes[0].name(), "CRCL_CL");

        // Applying a retune no open round holds records the new values and
        // touches nothing else
        let mut state = state;
        let retunes = compatibility(&retuned, &state).retunes;
        let lines = apply_retunes(&mut state, &retunes);
        assert_eq!(
            lines,
            vec!["CRCL_CL: initial 0.1 -> 0.5; bounds none -> (0, 2)".to_string()]
        );
        let entry = state.roster_entry("CRCL_CL").unwrap();
        assert_eq!(
            (entry.candidate.initial, entry.candidate.upper),
            (0.5, Some(2.0))
        );
        assert_eq!(
            entry.retunes[0].after_round.as_deref(),
            Some("forward_round1")
        );
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
        let fewer = replan(&plan, &["WT_CL", "CRCL_CL"]);
        let reasons = compatibility(&fewer, &state).reasons;
        assert!(
            reasons[0].contains("WT_V was in the current model"),
            "{reasons:?}"
        );
    }
}
