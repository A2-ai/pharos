use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use config::NonmemConfig;
use fs_err as fs;
use serde::{Deserialize, Serialize};
use utils::{get_utc_now, seconds_between};

use super::roster::RosterEntry;
use super::score::lrt;
use super::{Direction, REFERENCE_ROUND, STATE_FILENAME, ScmOptions, ScmPlan, lowercase_display};

lowercase_display!(ScmRunStatus, CandidateStatus, CheckpointStatus);

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

/// Where a checkpoint fit stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckpointStatus {
    /// Submitted, and not yet concluded
    Running,
    Succeeded,
    /// Ran out of retries without a usable fit
    Unusable,
    /// The selected model had already run the covariance step: no fit of its own
    Reused,
}

impl CheckpointStatus {
    pub fn is_concluded(&self) -> bool {
        *self != CheckpointStatus::Running
    }
}

/// A selected model re-fitted with the covariance step on, outside the
/// rounds: the forward model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckpointFit {
    /// The selected model this re-fits, relative to out_dir
    pub source: String,
    /// The effects released in it
    pub retained: Vec<String>,
    /// The latest attempt (the source itself when reused), relative to out_dir
    pub model: String,
    pub attempts: Vec<AttemptRecord>,
    pub status: CheckpointStatus,
    pub ofv: Option<f64>,
    pub heuristics: Vec<String>,
}

impl CheckpointFit {
    /// A usable fit of exactly the `retained` effects, if this is one.
    pub fn usable_for(&self, retained: &[String]) -> bool {
        matches!(
            self.status,
            CheckpointStatus::Succeeded | CheckpointStatus::Reused
        ) && same_set(&self.retained, retained)
    }
}

fn same_set(a: &[String], b: &[String]) -> bool {
    let mut a: Vec<&String> = a.iter().collect();
    let mut b: Vec<&String> = b.iter().collect();
    a.sort();
    b.sort();
    a == b
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
    /// The plan the process runs under: what the config resolved to when it
    /// was last submitted. Readers (`scm status`, `scm summary`, a re-plan's
    /// diff) take the process from here, never from the config, which may
    /// have been edited since.
    pub plan: ScmPlan,
    pub plan_digest: String,
    pub roster: Vec<RosterEntry>,
    pub status: ScmRunStatus,
    pub message: Option<String>,
    pub retained: Vec<String>,
    pub reference_model: Option<String>,
    pub reference_ofv: Option<f64>,
    pub phase: Option<Direction>,
    pub rounds: Vec<RoundRecord>,
    /// The forward model's covariance-step fit, once forward selection ends
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forward_final: Option<CheckpointFit>,
    pub final_model: Option<String>,
    pub final_ofv: Option<f64>,
    /// Heuristics the final re-fit fired, e.g. its covariance step aborted
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub final_heuristics: Vec<String>,
    pub had_unusable: bool,
    /// Every pause so far: when a driver stopped (its state's last save)
    /// and when the next took the process up again. Readers subtract them
    /// from the clocks they show, so a round paused overnight does not
    /// claim the night as time run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pauses: Vec<Pause>,
    pub updated: String,
}

/// A stretch the process spent stopped: from a driver's last save to the
/// next driver's start.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pause {
    pub stopped: String,
    pub resumed: String,
}

impl ScmState {
    /// A fresh state for `plan`: its digest, and its candidates as the roster.
    pub fn new(plan: &ScmPlan) -> Self {
        Self {
            plan: plan.clone(),
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
        let mut state: ScmState = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        // The plan's paths are project-relative; the root is where the out_dir sits.
        state.plan.restore_root(out_dir)?;
        Ok(Some(state))
    }

    pub fn save(&mut self, out_dir: &Path) -> Result<()> {
        self.updated = get_utc_now();
        fs::create_dir_all(out_dir)?;
        utils::write_json_to_file(self, Self::state_path(out_dir))
            .with_context(|| format!("failed to write state in {}", out_dir.display()))?;
        Ok(())
    }

    /// A driver takes up a process the last one left: the stretch since
    /// its last save was a pause.
    pub fn resume(&mut self) {
        self.pauses.push(Pause {
            stopped: self.updated.clone(),
            resumed: get_utc_now(),
        });
    }

    /// Seconds the process spent paused between two timestamps.
    pub fn paused_between(&self, start: &str, end: &str) -> f64 {
        let Some(span) = seconds_between(Some(start), Some(end)) else {
            return 0.0;
        };
        self.pauses
            .iter()
            .filter_map(|p| {
                let from = seconds_between(Some(start), Some(&p.stopped))?.max(0.0);
                let to = seconds_between(Some(start), Some(&p.resumed))?.min(span);
                Some((to - from).max(0.0))
            })
            .sum()
    }

    /// How long the process ran between two timestamps: the span less the
    /// pauses within it. `None` when either timestamp is missing or
    /// unreadable.
    pub fn ran_between(&self, start: Option<&str>, end: Option<&str>) -> Option<f64> {
        let (start, end) = (start?, end?);
        let span = seconds_between(Some(start), Some(end))?;
        Some((span - self.paused_between(start, end)).max(0.0))
    }

    /// Every fit attempted so far, retries and superseded attempts included
    pub fn fits_so_far(&self) -> usize {
        let rounds: usize = self
            .rounds
            .iter()
            .flat_map(|r| &r.candidates)
            .map(|c| c.attempts.len() + c.superseded.len())
            .sum();
        rounds + self.forward_final.as_ref().map_or(0, |f| f.attempts.len())
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
    /// Read the SCM process living in `out_dir`, its fits' output under the
    /// project `settings`. A started process is read under the plan its
    /// state records; one not started yet is read under what its config in
    /// the out_dir resolves to today.
    pub fn read(out_dir: &Path, settings: &NonmemConfig) -> Result<Self> {
        match ScmState::load(out_dir)? {
            Some(state) => Ok(Self::of(state.plan.clone(), out_dir, Some(state), settings)),
            None => {
                let config = super::config::config_in(out_dir)?;
                let built = super::build_plan_from_config(
                    &config,
                    &super::ScmOverrides::default(),
                    env!("CARGO_PKG_VERSION"),
                )
                .with_context(|| {
                    format!(
                        "the SCM process in {} has not started, and its config does not plan",
                        out_dir.display()
                    )
                })?;
                Ok(Self::of(built.plan, out_dir, None, settings))
            }
        }
    }

    /// [`ScmProcess::read`] for a plan already in hand and a state already
    /// loaded — what a re-plan has
    pub fn of(
        plan: ScmPlan,
        out_dir: &Path,
        state: Option<ScmState>,
        settings: &NonmemConfig,
    ) -> Self {
        let started = state.is_some();
        let mut state = state.unwrap_or_else(|| ScmState::new(&plan));
        let models_running =
            super::round::reconcile_state_with_disk(&mut state, out_dir, &plan.options, settings);
        Self {
            plan,
            state,
            started,
            models_running,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pause(stopped: &str, resumed: &str) -> Pause {
        Pause {
            stopped: stopped.to_string(),
            resumed: resumed.to_string(),
        }
    }

    /// A round started one afternoon, stopped an hour in and resumed the
    /// next morning, has run for the hour and the time since the resume.
    #[test]
    fn time_run_leaves_out_the_pauses_within_the_span() {
        let mut state = ScmState::default();
        let (start, end) = ("2026-10-13T12:00:00+00:00", "2026-10-14T08:00:00+00:00");
        assert_eq!(state.paused_between(start, end), 0.0);
        assert_eq!(
            state.ran_between(Some(start), Some(end)),
            Some(20.0 * 3600.0)
        );

        state.pauses.push(pause(
            "2026-10-13T13:00:00+00:00",
            "2026-10-14T07:00:00+00:00",
        ));
        assert_eq!(state.paused_between(start, end), 18.0 * 3600.0);
        assert_eq!(
            state.ran_between(Some(start), Some(end)),
            Some(2.0 * 3600.0)
        );

        // Pauses outside the span do not count; ones straddling it count
        // only for the part within.
        state.pauses.push(pause(
            "2026-10-12T00:00:00+00:00",
            "2026-10-13T11:00:00+00:00",
        ));
        state.pauses.push(pause(
            "2026-10-14T09:00:00+00:00",
            "2026-10-14T10:00:00+00:00",
        ));
        assert_eq!(
            state.ran_between(Some(start), Some(end)),
            Some(2.0 * 3600.0)
        );
        state.pauses.push(pause(
            "2026-10-13T11:30:00+00:00",
            "2026-10-13T12:30:00+00:00",
        ));
        state.pauses.push(pause(
            "2026-10-14T07:45:00+00:00",
            "2026-10-14T09:00:00+00:00",
        ));
        assert_eq!(
            state.ran_between(Some(start), Some(end)),
            Some(1.25 * 3600.0)
        );

        assert_eq!(state.ran_between(None, Some(end)), None);
        assert_eq!(state.ran_between(Some(start), None), None);
        assert_eq!(state.ran_between(Some("yesterday"), Some(end)), None);
    }

    /// Resuming records the stretch since the state's last save as a pause.
    #[test]
    fn resume_records_the_pause_since_the_last_save() {
        let mut state = ScmState {
            updated: "2026-10-13T13:00:00+00:00".to_string(),
            ..Default::default()
        };
        state.resume();
        assert_eq!(state.pauses.len(), 1);
        assert_eq!(state.pauses[0].stopped, "2026-10-13T13:00:00+00:00");
        assert!(
            seconds_between(Some(&state.pauses[0].resumed), Some(&get_utc_now())).unwrap() < 5.0
        );
    }
}
