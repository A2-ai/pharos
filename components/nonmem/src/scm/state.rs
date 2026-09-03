use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use fs_err as fs;
use serde::{Deserialize, Serialize};
use utils::get_utc_now;

use super::{Direction, NO_REFERENCE, REFERENCE_ROUND, STATE_FILENAME};

pub const STATE_SCHEMA_VERSION: u32 = 1;

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
}

impl CandidateStatus {
    /// Whether the candidate has reached a terminal state for its round
    /// (scored, or given up on after exhausting retries).
    pub fn is_concluded(&self) -> bool {
        matches!(self, CandidateStatus::Succeeded | CandidateStatus::Unusable)
    }
}

impl fmt::Display for CandidateStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CandidateStatus::Pending => "pending",
            CandidateStatus::Running => "running",
            CandidateStatus::Succeeded => "succeeded",
            CandidateStatus::Unusable => "unusable",
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

    pub fn all_succeeded(&self) -> bool {
        self.candidates
            .iter()
            .all(|c| c.status == CandidateStatus::Succeeded)
    }

    pub fn any_heuristics(&self) -> bool {
        self.candidates.iter().any(|c| !c.heuristics.is_empty())
    }
}

/// A round the search cannot decide on its own: two or more candidates whose
/// p-value AND ΔOFV are identical, so no tie-break on the numbers can
/// separate them. The search pauses and the user picks the winner.
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
    pub plan_digest: String,
    pub status: ScmRunStatus,
    pub message: Option<String>,
    /// Covariates currently in the model, in selection order.
    pub retained: Vec<String>,
    /// Current reference model path, relative to out_dir.
    pub reference_model: Option<String>,
    pub reference_ofv: Option<f64>,
    /// Phase the search is currently in.
    pub phase: Option<Direction>,
    pub rounds: Vec<RoundRecord>,
    /// Final model path relative to out_dir, once the search completes.
    pub final_model: Option<String>,
    /// True if any round contained an unusable candidate.
    pub had_unusable: bool,
    /// Set when the search paused for the user to break a tie; cleared when
    /// their choice is applied. A state written before this field existed
    /// loads without one.
    #[serde(default)]
    pub pending_tie: Option<PendingTie>,
    pub updated: String,
}

impl ScmState {
    pub fn new(plan_digest: String) -> Self {
        Self {
            schema_version: STATE_SCHEMA_VERSION,
            plan_digest,
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

    pub fn load(out_dir: &Path) -> Result<Option<Self>> {
        let path = Self::state_path(out_dir);
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let state: ScmState = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(Some(state))
    }

    pub fn save(&mut self, out_dir: &Path) -> Result<()> {
        self.updated = get_utc_now();
        fs::create_dir_all(out_dir)?;
        utils::write_json_to_file(self, Self::state_path(out_dir))
            .with_context(|| format!("failed to write state in {}", out_dir.display()))?;
        Ok(())
    }

    /// Number of completed search rounds (the reference fit is not a round).
    pub fn completed_search_rounds(&self) -> usize {
        self.rounds
            .iter()
            .filter(|r| r.complete && !r.is_reference())
            .count()
    }

    pub fn find_round_mut(&mut self, name: &str) -> Option<&mut RoundRecord> {
        self.rounds.iter_mut().find(|r| r.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_round_trips_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = ScmState::new("digest123".into());
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
        assert_eq!(loaded.completed_search_rounds(), 1);
        assert_eq!(loaded.retained, vec!["WT_CL".to_string()]);
    }

    #[test]
    fn missing_state_loads_as_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(ScmState::load(dir.path()).unwrap().is_none());
    }
}
