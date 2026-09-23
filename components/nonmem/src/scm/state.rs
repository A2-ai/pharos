use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use fs_err as fs;
use serde::{Deserialize, Serialize};
use utils::get_utc_now;

use super::roster::RosterEntry;
use super::score::lrt;
use super::{
    Direction, NO_REFERENCE, PLAN_FILENAME, REFERENCE_ROUND, STATE_FILENAME, ScmOptions, ScmPlan,
    lowercase_display,
};

lowercase_display!(ScmRunStatus, CandidateStatus);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScmRunStatus {
    #[default]
    Planned,
    Running,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CandidateStatus {
    #[default]
    Pending,
    Running,
    Succeeded,
    /// Ran out of retries without a scoreable fit
    Unusable,
    /// Removed from the plan while its round was open
    Withdrawn,
}

impl CandidateStatus {
    /// Whether the candidate has reached a terminal state for its round
    pub fn is_concluded(&self) -> bool {
        matches!(
            self,
            CandidateStatus::Succeeded | CandidateStatus::Unusable | CandidateStatus::Withdrawn
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttemptRecord {
    /// Model path, relative to out_dir.
    pub model: String,
    pub outcome: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CandidateRecord {
    pub candidate: String,
    pub action: String,
    pub model: String,
    pub attempts: Vec<AttemptRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub superseded: Vec<AttemptRecord>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub refit: usize,
    pub status: CandidateStatus,
    pub ofv: Option<f64>,
    pub delta_ofv: Option<f64>,
    pub df: usize,
    pub p_value: Option<f64>,
    pub significant: Option<bool>,
    pub heuristics: Vec<String>,
    pub selected: bool,
}

impl CandidateRecord {
    pub fn new(candidate: &str, action: String, df: usize) -> Self {
        Self {
            candidate: candidate.to_string(),
            action,
            df,
            ..Default::default()
        }
    }

    pub fn n_attempts(&self) -> usize {
        self.attempts.len()
    }

    /// Start the candidate anew under retuned values; the attempts so far
    /// stay on record as superseded.
    pub fn refit_under_new_values(&mut self) {
        let mut superseded = std::mem::take(&mut self.superseded);
        superseded.append(&mut self.attempts);
        *self = Self {
            superseded,
            refit: self.refit + 1,
            ..Self::new(&self.candidate, std::mem::take(&mut self.action), self.df)
        };
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
    pub reference_model: String,
    pub reference_ofv: Option<f64>,
    pub candidates: Vec<CandidateRecord>,
    pub winner: Option<String>,
    pub decision: String,
    pub complete: bool,
}

impl RoundRecord {
    pub fn is_reference(&self) -> bool {
        self.name == REFERENCE_ROUND
    }

    /// Whether this round was fitted against a reference model.
    pub fn has_reference(&self) -> bool {
        self.reference_model != NO_REFERENCE
    }

    /// The directory this round's models and records live in: the reference
    /// fit's own name (`base` / `full`), or the round name.
    pub fn dir_name(&self) -> Option<String> {
        if self.is_reference() {
            self.candidates.first().map(|c| c.candidate.clone())
        } else {
            Some(self.name.clone())
        }
    }

    /// Candidates that reached a terminal state (scored, or given up on).
    pub fn concluded(&self) -> usize {
        self.candidates
            .iter()
            .filter(|c| c.status.is_concluded())
            .count()
    }

    pub fn unusable(&self) -> usize {
        self.candidates
            .iter()
            .filter(|c| c.status == CandidateStatus::Unusable)
            .count()
    }

    pub fn candidate(&self, name: &str) -> Option<&CandidateRecord> {
        self.candidates.iter().find(|c| c.candidate == name)
    }

    /// The significance level this round's candidates are tested against
    pub fn alpha(&self, options: &ScmOptions) -> Option<f64> {
        if self.is_reference() {
            return None;
        }
        Some(match self.direction {
            Direction::Forward => options.forward_alpha,
            Direction::Backward => options.backward_alpha,
        })
    }

    /// Score every candidate that fitted usably but carries no score yet,
    /// writing `delta_ofv`, `p_value` and `significant` onto its record.
    pub fn score(&mut self, options: &ScmOptions) -> Vec<Scored> {
        let (Some(alpha), Some(reference_ofv)) = (self.alpha(options), self.reference_ofv) else {
            return vec![];
        };
        let direction = self.direction;
        for cand in &mut self.candidates {
            if cand.status != CandidateStatus::Succeeded || cand.p_value.is_some() || cand.df == 0 {
                continue;
            }
            let Some(ofv) = cand.ofv else { continue };
            let (delta_ofv, p_value) = lrt(reference_ofv, ofv, cand.df, direction);
            cand.delta_ofv = Some(delta_ofv);
            cand.p_value = Some(p_value);
            cand.significant = Some(p_value < alpha);
        }
        self.scored()
    }

    /// Every candidate that fitted usably and carries a score
    pub fn scored(&self) -> Vec<Scored> {
        self.candidates
            .iter()
            .enumerate()
            .filter_map(|(index, c)| match (c.status, c.p_value, c.delta_ofv) {
                (CandidateStatus::Succeeded, Some(p_value), Some(delta_ofv)) => Some(Scored {
                    index,
                    p_value,
                    delta_ofv,
                }),
                _ => None,
            })
            .collect()
    }

    /// The scored candidates, sorted best to worst for the phase
    pub fn ranking(&self) -> Vec<Scored> {
        if !self.candidates.iter().all(|c| c.status.is_concluded()) {
            return vec![];
        }
        let mut scored = self.scored();
        scored.sort_by(|a, b| self.direction.rank(a.key(), b.key()));
        scored
    }

    /// 1-based rank by candidate index, from [`RoundRecord::ranking`].
    pub fn ranks(&self) -> BTreeMap<usize, usize> {
        self.ranking()
            .iter()
            .enumerate()
            .map(|(rank, s)| (s.index, rank + 1))
            .collect()
    }

    /// The ranked candidates that meet the phase's alpha
    pub fn contenders(&self, options: &ScmOptions) -> Vec<Scored> {
        let Some(alpha) = self.alpha(options) else {
            return vec![];
        };
        let direction = self.direction;
        self.ranking()
            .into_iter()
            .filter(|s| direction.meets(s.p_value, alpha))
            .collect()
    }
}

/// One candidate of a round, scored: where it sits in the round and what it scored
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scored {
    pub index: usize,
    pub p_value: f64,
    pub delta_ofv: f64,
}

impl Scored {
    /// The `(p, ΔOFV)` pair [`Direction::rank`] orders on.
    pub fn key(&self) -> (f64, f64) {
        (self.p_value, self.delta_ofv)
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScmState {
    pub plan_digest: String,
    pub roster: Vec<RosterEntry>,
    pub status: ScmRunStatus,
    pub message: Option<String>,
    pub retained: Vec<String>,
    pub reference_model: Option<String>,
    pub reference_ofv: Option<f64>,
    pub phase: Option<Direction>,
    pub rounds: Vec<RoundRecord>,
    pub final_model: Option<String>,
    pub final_ofv: Option<f64>,
    pub had_unusable: bool,
    pub updated: String,
}

impl ScmState {
    /// A fresh state for `plan`: its digest, and its candidates as the roster.
    pub fn new(plan: &ScmPlan) -> Self {
        Self {
            plan_digest: plan.digest(),
            roster: plan.candidates.iter().map(RosterEntry::active).collect(),
            updated: get_utc_now(),
            ..Default::default()
        }
    }

    pub fn state_path(out_dir: &Path) -> PathBuf {
        out_dir.join(STATE_FILENAME)
    }

    /// Load the state in `out_dir`, if any.
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

    pub fn completed_rounds(&self) -> usize {
        self.rounds
            .iter()
            .filter(|r| r.complete && !r.is_reference())
            .count()
    }

    pub fn round(&self, name: &str) -> Option<&RoundRecord> {
        self.rounds.iter().find(|r| r.name == name)
    }

    pub fn round_mut(&mut self, name: &str) -> Option<&mut RoundRecord> {
        self.rounds.iter_mut().find(|r| r.name == name)
    }

    pub fn open_round(&self) -> Option<&RoundRecord> {
        self.rounds
            .iter()
            .find(|r| !r.complete && !r.is_reference())
    }

    pub fn open_round_mut(&mut self) -> Option<&mut RoundRecord> {
        self.rounds
            .iter_mut()
            .find(|r| !r.complete && !r.is_reference())
    }

    pub fn active_roster(&self) -> impl Iterator<Item = &RosterEntry> {
        self.roster.iter().filter(|e| e.removed.is_none())
    }

    pub fn removed_roster(&self) -> impl Iterator<Item = &RosterEntry> {
        self.roster.iter().filter(|e| e.removed.is_some())
    }

    pub fn roster_entry(&self, name: &str) -> Option<&RosterEntry> {
        self.roster.iter().find(|e| e.candidate.name == name)
    }

    /// The roster entry of a candidate still in the SCM process
    pub fn active_entry_mut(&mut self, name: &str) -> Option<&mut RosterEntry> {
        self.roster
            .iter_mut()
            .find(|e| e.candidate.name == name && e.removed.is_none())
    }

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
}

// ---------------------------------------------------------------------------
// Reading a process off disk
// ---------------------------------------------------------------------------

/// An SCM process as it stands in its output directory: the plan it runs
/// under, its state brought up to date with what the fits have left behind,
/// and the models still running.
#[derive(Debug, Clone)]
pub struct ScmProcess {
    pub plan: ScmPlan,
    pub state: ScmState,
    pub started: bool,
    pub models_running: Vec<String>,
}

impl ScmProcess {
    /// Read the SCM process living in `out_dir`
    pub fn read(out_dir: &Path) -> Result<Self> {
        let plan_path = out_dir.join(PLAN_FILENAME);
        if !plan_path.exists() {
            anyhow::bail!(
                "{} has no {PLAN_FILENAME}; is this an SCM output directory?",
                out_dir.display()
            );
        }
        let plan = ScmPlan::load(&plan_path)
            .with_context(|| format!("failed to load {}", plan_path.display()))?;
        Self::of(plan, out_dir, ScmState::load(out_dir)?)
    }

    /// [`ScmProcess::read`] for a plan already in hand and a state already
    /// loaded — what a re-plan has
    pub fn of(plan: ScmPlan, out_dir: &Path, state: Option<ScmState>) -> Result<Self> {
        let started = state.is_some();
        let mut state = state.unwrap_or_else(|| ScmState::new(&plan));
        let settings = super::project_config(out_dir)?;
        let models_running =
            super::round::reconcile_state_with_disk(&mut state, out_dir, &plan.options, &settings);
        Ok(Self {
            plan,
            state,
            started,
            models_running,
        })
    }
}
