use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use fs_err as fs;
use serde::{Deserialize, Serialize};
use utils::get_utc_now;

use super::roster::RosterEntry;
use super::{
    Candidate, Direction, NO_REFERENCE, PLAN_FILENAME, REFERENCE_ROUND, STATE_FILENAME, ScmPlan,
};

/// Schema 2: the state carries the candidate roster (see [`super::roster`])
/// and `plan_digest` covers the options only.
pub const STATE_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScmRunStatus {
    Planned,
    Running,
    Paused,
    Completed,
    Failed,
}

impl fmt::Display for ScmRunStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ScmRunStatus::Planned => "planned",
            ScmRunStatus::Running => "running",
            ScmRunStatus::Paused => "paused",
            ScmRunStatus::Completed => "completed",
            ScmRunStatus::Failed => "failed",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CandidateStatus {
    Pending,
    Running,
    Succeeded,
    /// Ran out of retries without a scoreable fit. Reported, never treated as
    /// evidence the covariate is insignificant.
    Unusable,
    /// Removed from the plan while its round was open. Whatever its fit
    /// produced is recorded, never scored or ranked.
    Withdrawn,
}

impl CandidateStatus {
    /// Whether the candidate has reached a terminal state for its round
    /// (scored, given up on after exhausting retries, or withdrawn).
    pub fn is_concluded(&self) -> bool {
        matches!(
            self,
            CandidateStatus::Succeeded | CandidateStatus::Unusable | CandidateStatus::Withdrawn
        )
    }
}

impl fmt::Display for CandidateStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CandidateStatus::Pending => "pending",
            CandidateStatus::Running => "running",
            CandidateStatus::Succeeded => "succeeded",
            CandidateStatus::Unusable => "unusable",
            CandidateStatus::Withdrawn => "withdrawn",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttemptRecord {
    /// Model path, relative to out_dir.
    pub model: String,
    /// What happened: "succeeded", "terminated", "no ofv",
    /// "minimization terminated", "did not finish".
    pub outcome: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateRecord {
    /// Candidate name (e.g. WT_CL); for reference fits, "base" or "full".
    pub candidate: String,
    /// "add WT_CL", "drop WT_CL", "fit base model", "fit full model".
    pub action: String,
    /// Model of the scoring attempt (last attempt), relative to out_dir.
    pub model: String,
    pub attempts: Vec<AttemptRecord>,
    /// Attempts made under an initial estimate or bounds the plan has since
    /// changed (see [`super::roster::Retune`]). Kept on record — the models
    /// and their output stay where they are — but never scored or ranked.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub superseded: Vec<AttemptRecord>,
    /// How many times the candidate has been refitted after a retune; 0 for
    /// the usual case. It names the refit's models apart from the attempts
    /// that ran under the old values (`1001_wt_cl_refit2`).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub refit: usize,
    pub status: CandidateStatus,
    pub ofv: Option<f64>,
    /// candidate OFV − reference OFV (negative = candidate improves).
    pub delta_ofv: Option<f64>,
    pub df: usize,
    pub p_value: Option<f64>,
    pub significant: Option<bool>,
    /// Heuristic checks that fired for the scoring attempt.
    pub heuristics: Vec<String>,
    /// Whether this candidate won its round.
    pub selected: bool,
}

impl CandidateRecord {
    pub fn new(candidate: &str, action: String, df: usize) -> Self {
        Self {
            candidate: candidate.to_string(),
            action,
            model: String::new(),
            attempts: vec![],
            superseded: vec![],
            refit: 0,
            status: CandidateStatus::Pending,
            ofv: None,
            delta_ofv: None,
            df,
            p_value: None,
            significant: None,
            heuristics: vec![],
            selected: false,
        }
    }

    pub fn n_attempts(&self) -> usize {
        self.attempts.len()
    }

    /// Every attempt the candidate has ever had, superseded ones included.
    pub fn total_attempts(&self) -> usize {
        self.attempts.len() + self.superseded.len()
    }

    /// Start the candidate over under retuned values: the attempts it made
    /// under the old ones move to [`CandidateRecord::superseded`], its score
    /// is cleared, and its next fit is a fresh first attempt written from
    /// the template under a name of its own.
    pub fn refit_under_new_values(&mut self) {
        self.superseded.append(&mut self.attempts);
        self.refit += 1;
        self.status = CandidateStatus::Pending;
        self.model = String::new();
        self.ofv = None;
        self.delta_ofv = None;
        self.p_value = None;
        self.significant = None;
        self.heuristics.clear();
        self.selected = false;
    }
}

/// `skip_serializing_if` for counters that are almost always 0.
fn is_zero(n: &usize) -> bool {
    *n == 0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoundRecord {
    /// e.g. "forward_round1", "backward_round1", "reference".
    pub name: String,
    pub direction: Direction,
    /// Reference model path relative to out_dir ("-" for the reference round).
    pub reference_model: String,
    pub reference_ofv: Option<f64>,
    pub candidates: Vec<CandidateRecord>,
    /// Winning candidate name, if the round selected one.
    pub winner: Option<String>,
    /// Human summary of the round's decision.
    pub decision: String,
    pub complete: bool,
}

impl RoundRecord {
    /// The reference fit's pseudo-round, which is never LRT-scored.
    pub fn is_reference(&self) -> bool {
        self.name == REFERENCE_ROUND
    }

    /// Whether this round was fitted against a reference model.
    pub fn has_reference(&self) -> bool {
        self.reference_model != NO_REFERENCE
    }

    /// Candidates that reached a terminal state (scored, or given up on).
    pub fn concluded(&self) -> usize {
        self.candidates
            .iter()
            .filter(|c| c.status.is_concluded())
            .count()
    }

    /// Retries used across the round: every attempt after each candidate's
    /// first.
    pub fn retries(&self) -> usize {
        self.candidates
            .iter()
            .map(|c| c.n_attempts().saturating_sub(1))
            .sum()
    }

    pub fn unusable(&self) -> usize {
        self.candidates
            .iter()
            .filter(|c| c.status == CandidateStatus::Unusable)
            .count()
    }

    pub fn withdrawn(&self) -> usize {
        self.candidates
            .iter()
            .filter(|c| c.status == CandidateStatus::Withdrawn)
            .count()
    }

    /// Every candidate still in the round (not withdrawn) fitted usably.
    pub fn all_succeeded(&self) -> bool {
        self.candidates
            .iter()
            .filter(|c| c.status != CandidateStatus::Withdrawn)
            .all(|c| c.status == CandidateStatus::Succeeded)
    }

    pub fn any_heuristics(&self) -> bool {
        self.candidates.iter().any(|c| !c.heuristics.is_empty())
    }
}

/// A round the SCM process cannot decide on its own: two or more candidates whose
/// p-value AND ΔOFV are identical, so no tie-break on the numbers can
/// separate them. The SCM process pauses and the user picks the winner.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingTie {
    /// Round whose decision is outstanding.
    pub round: String,
    pub direction: Direction,
    /// The tied candidates, in candidate order — one of these is the choice.
    pub candidates: Vec<String>,
    pub p_value: f64,
    pub delta_ofv: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScmState {
    pub schema_version: u32,
    /// Digest of the plan's SCM-defining options ([`ScmPlan::digest`]).
    pub plan_digest: String,
    /// Every candidate this SCM process has known, removed ones included.
    /// Empty only in a schema-1 state before [`ScmState::load`] migrates it.
    #[serde(default)]
    pub roster: Vec<RosterEntry>,
    pub status: ScmRunStatus,
    pub message: Option<String>,
    /// Covariates currently in the model, in selection order.
    pub retained: Vec<String>,
    /// Current reference model path, relative to out_dir.
    pub reference_model: Option<String>,
    pub reference_ofv: Option<f64>,
    /// Phase the SCM process is currently in.
    pub phase: Option<Direction>,
    pub rounds: Vec<RoundRecord>,
    /// Final model path relative to out_dir, once the SCM process completes.
    pub final_model: Option<String>,
    /// True if any round contained an unusable candidate.
    pub had_unusable: bool,
    /// Set when the SCM process paused for the user to break a tie; cleared when
    /// their choice is applied. A state written before this field existed
    /// loads without one.
    #[serde(default)]
    pub pending_tie: Option<PendingTie>,
    pub updated: String,
}

impl ScmState {
    /// A fresh state for `plan`: its digest, and its candidates as the
    /// roster.
    pub fn new(plan: &ScmPlan) -> Self {
        Self::with_roster(
            plan.digest(),
            plan.candidates.iter().map(RosterEntry::active).collect(),
        )
    }

    pub fn with_roster(plan_digest: String, roster: Vec<RosterEntry>) -> Self {
        Self {
            schema_version: STATE_SCHEMA_VERSION,
            plan_digest,
            roster,
            status: ScmRunStatus::Planned,
            message: None,
            retained: vec![],
            reference_model: None,
            reference_ofv: None,
            phase: None,
            rounds: vec![],
            final_model: None,
            had_unusable: false,
            pending_tie: None,
            updated: get_utc_now(),
        }
    }

    pub fn state_path(out_dir: &Path) -> PathBuf {
        out_dir.join(STATE_FILENAME)
    }

    /// Load the state in `out_dir`, if any. A schema-1 state (no roster, a
    /// digest that still covered the candidates) is migrated in memory: the
    /// roster is seeded from the plan.json beside it — the plan the state
    /// ran under, since `run` saves the plan before it fits anything — and
    /// the digest recomputed over the options alone. The next save writes
    /// it back as schema 2.
    pub fn load(out_dir: &Path) -> Result<Option<Self>> {
        let path = Self::state_path(out_dir);
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let mut state: ScmState = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        if state.schema_version < 2 {
            let plan_path = out_dir.join(PLAN_FILENAME);
            let plan = ScmPlan::load(&plan_path).with_context(|| {
                format!(
                    "{} is a schema-1 state file and needs the plan.json beside it to migrate",
                    path.display()
                )
            })?;
            state.roster = plan.candidates.iter().map(RosterEntry::active).collect();
            state.plan_digest = plan.digest();
            state.schema_version = STATE_SCHEMA_VERSION;
        }
        Ok(Some(state))
    }

    pub fn save(&mut self, out_dir: &Path) -> Result<()> {
        self.updated = get_utc_now();
        fs::create_dir_all(out_dir)?;
        utils::write_json_to_file(self, Self::state_path(out_dir))
            .with_context(|| format!("failed to write state in {}", out_dir.display()))?;
        Ok(())
    }

    /// Number of completed SCM rounds (the reference fit is not a round).
    pub fn completed_rounds(&self) -> usize {
        self.rounds
            .iter()
            .filter(|r| r.complete && !r.is_reference())
            .count()
    }

    pub fn find_round_mut(&mut self, name: &str) -> Option<&mut RoundRecord> {
        self.rounds.iter_mut().find(|r| r.name == name)
    }

    /// The round that has started but not concluded, if any (never the
    /// reference round).
    pub fn open_round(&self) -> Option<&RoundRecord> {
        self.rounds
            .iter()
            .find(|r| !r.complete && !r.is_reference())
    }

    /// Roster entries still in the SCM process.
    pub fn active_roster(&self) -> impl Iterator<Item = &RosterEntry> {
        self.roster.iter().filter(|e| e.removed.is_none())
    }

    /// Roster entries removed from the SCM process, in roster order.
    pub fn removed_roster(&self) -> impl Iterator<Item = &RosterEntry> {
        self.roster.iter().filter(|e| e.removed.is_some())
    }

    /// The roster entry for a candidate, by name.
    pub fn roster_entry(&self, name: &str) -> Option<&RosterEntry> {
        self.roster.iter().find(|e| e.candidate.name == name)
    }

    /// Whether `name` has ever won a round, or sits in the current model
    /// (the full model of a backward-only SCM process retains every
    /// candidate without any of them winning).
    pub fn depends_on(&self, name: &str) -> Option<String> {
        if let Some(round) = self
            .rounds
            .iter()
            .find(|r| r.winner.as_deref() == Some(name))
        {
            return Some(format!("selected in {}", round.name));
        }
        if self.retained.iter().any(|n| n == name) {
            return Some("in the current model".to_string());
        }
        None
    }

    /// The candidates the roster knows, as plan candidates, for callers that
    /// need the values a removed candidate ran with.
    pub fn roster_candidates(&self) -> Vec<Candidate> {
        self.roster.iter().map(|e| e.candidate.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_round_trips_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = ScmState::with_roster("digest123".into(), vec![]);
        state.retained.push("WT_CL".into());
        state.rounds.push(RoundRecord {
            name: "forward_round1".into(),
            direction: Direction::Forward,
            reference_model: "base/1001_base.mod".into(),
            reference_ofv: Some(1000.0),
            candidates: vec![CandidateRecord::new("WT_CL", "add WT_CL".into(), 1)],
            winner: Some("WT_CL".into()),
            decision: "added WT_CL".into(),
            complete: true,
        });
        state.save(dir.path()).unwrap();

        let loaded = ScmState::load(dir.path()).unwrap().unwrap();
        assert_eq!(loaded.plan_digest, "digest123");
        assert_eq!(loaded.rounds.len(), 1);
        assert_eq!(loaded.completed_rounds(), 1);
        assert_eq!(loaded.retained, vec!["WT_CL".to_string()]);
    }

    #[test]
    fn missing_state_loads_as_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(ScmState::load(dir.path()).unwrap().is_none());
    }

    /// A schema-1 state file loads with its roster seeded from the plan.json
    /// beside it and its digest recomputed, so an SCM process started before
    /// the roster existed resumes under the new rule.
    #[test]
    fn schema_one_state_migrates_from_the_plan_beside_it() {
        use crate::scm::ScmOptions;
        use crate::scm::test_support::make_plan;

        let dir = tempfile::tempdir().unwrap();
        let plan = make_plan(dir.path(), ScmOptions::default());
        plan.save().unwrap();
        let out_dir = plan.out_dir_path();
        fs::write(
            ScmState::state_path(&out_dir),
            r#"{"schema_version": 1, "plan_digest": "old-digest-over-candidates",
                "status": "paused", "message": null, "retained": ["WT_CL"],
                "reference_model": "forward_round1/1001_wt_cl.mod", "reference_ofv": 980.0,
                "phase": "forward", "rounds": [], "final_model": null, "had_unusable": false,
                "updated": "2026-08-19T12:00:00+00:00"}"#,
        )
        .unwrap();

        let state = ScmState::load(&out_dir).unwrap().unwrap();
        assert_eq!(state.schema_version, STATE_SCHEMA_VERSION);
        assert_eq!(state.plan_digest, plan.digest());
        let names: Vec<&str> = state
            .roster
            .iter()
            .map(|e| e.candidate.name.as_str())
            .collect();
        assert_eq!(names, vec!["WT_CL", "CRCL_CL", "WT_V"]);
        assert!(state.roster.iter().all(|e| e.removed.is_none()));
        assert_eq!(state.retained, vec!["WT_CL".to_string()]);

        // without the plan.json there is nothing to migrate from
        fs::remove_file(out_dir.join(PLAN_FILENAME)).unwrap();
        let err = ScmState::load(&out_dir).unwrap_err();
        assert!(format!("{err:#}").contains("schema-1"), "got: {err:#}");
    }
}
