//! Where an SCM process already stands when its plan is (re)built, and what the new plan changes about it.

use std::fmt::Display;

use super::roster::{CandidateChange, Compatibility, compatibility, diff_candidates};
use super::state::{ScmProcess, ScmState};
use super::{Lines, PLAN_FILENAME, ScmOptions, ScmPlan, none_or_list, on_off};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanChange {
    pub field: String,
    pub detail: String,
}

impl PlanChange {
    fn new(field: &str, detail: String) -> Self {
        Self {
            field: field.to_string(),
            detail,
        }
    }
}

/// What a freshly built plan meets in its out_dir
#[derive(Debug, Clone, Default)]
pub struct PlanContext {
    pub had_previous_plan: bool,
    pub changes: Vec<PlanChange>,
    pub progress: Option<ScmProcess>,
    pub compatibility: Option<Compatibility>,
}

impl PlanContext {
    pub fn read(plan: &ScmPlan) -> Self {
        let out_dir = plan.out_dir_path();

        let previous = ScmPlan::load(out_dir.join(PLAN_FILENAME)).ok();
        let state = ScmState::load(&out_dir).ok().flatten();
        // The process is read under the *new* plan
        let progress = state
            .as_ref()
            .and_then(|s| ScmProcess::of(plan.clone(), &out_dir, Some(s.clone())).ok());
        PlanContext {
            had_previous_plan: previous.is_some(),
            changes: previous
                .as_ref()
                .map(|p| diff_plans(p, plan, state.as_ref()))
                .unwrap_or_default(),
            compatibility: progress.as_ref().map(|p| compatibility(plan, &p.state)),
            progress,
        }
    }

    /// Whether the state in the out_dir cannot resume under this plan.
    pub fn state_is_stale(&self) -> bool {
        self.compatibility
            .as_ref()
            .is_some_and(Compatibility::is_incompatible)
    }

    fn is_empty(&self) -> bool {
        !self.had_previous_plan && self.progress.is_none()
    }

    fn headline(p: &ScmProcess) -> String {
        let s = &p.state;
        let rounds = match s.completed_rounds() {
            0 => "no rounds complete yet".to_string(),
            1 => "1 round complete".to_string(),
            n => format!("{n} rounds complete"),
        };
        let phase = match &s.phase {
            Some(d) => format!(", {d} phase"),
            None => String::new(),
        };
        format!("{} — {rounds}{phase} (updated {})", s.status, s.updated)
    }

    /// Append the progress and changes sections to a plan rendering.
    pub(crate) fn render_into(&self, out: &mut Lines) {
        if self.is_empty() {
            return;
        }
        out.blank();

        if let Some(p) = self.progress.as_ref() {
            out.add(format!("progress   : {}", Self::headline(p)));
            out.add(format!("selected   : {}", none_or_list(&p.state.retained)));
            let removed: Vec<String> = p
                .state
                .removed_roster()
                .map(|e| e.removal_label())
                .collect();
            if !removed.is_empty() {
                out.add(format!("removed    : {}", removed.join(", ")));
            }
            if let Some(cur) = p.state.open_round() {
                out.add(format!(
                    "in round   : {} — {}/{} concluded",
                    cur.name,
                    cur.concluded(),
                    cur.candidates.len()
                ));
            }
            if !p.models_running.is_empty() {
                out.add(format!("running    : {} model(s)", p.models_running.len()));
            }
            if let Some(f) = &p.state.final_model {
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
                    out.add(format!("  {:<14} {}", c.field, c.detail));
                }
            }
        }

        let (Some(progress), Some(verdict)) = (&self.progress, &self.compatibility) else {
            return;
        };
        let (reasons, removals, retunes) = match verdict {
            Compatibility::Identical => return,
            Compatibility::Compatible { removals, retunes } => (&[][..], removals, retunes),
            Compatibility::Incompatible {
                reasons,
                removals,
                retunes,
            } => (&reasons[..], removals, retunes),
        };

        if !removals.is_empty() {
            out.add(format!(
                "removing   : {} — never selected; takes effect from the next round, earlier rounds keep their results",
                removals.join(", ")
            ));
        }

        let open_round = progress.state.open_round();
        for r in retunes {
            let refit = open_round.filter(|o| o.candidates.iter().any(|c| c.candidate == r.name()));
            let effect = match refit {
                Some(round) => format!("refitted in {} under the new values", round.name),
                None => "takes effect from the next model written".to_string(),
            };
            out.add(format!("retuning   : {} — {effect}", r.label()));
        }
        let retained: Vec<&str> = retunes
            .iter()
            .map(|r| r.name())
            .filter(|n| progress.state.retained.iter().any(|r| r == n))
            .collect();
        if !retained.is_empty() {
            out.add(format!(
                "             {} already in the model: the rounds it was fitted in \
                 keep the values they ran under",
                retained.join(", ")
            ));
        }

        if !reasons.is_empty() {
            out.add(
                "note       : the SCM process in out_dir belongs to the previous plan; it cannot \
                 resume under this one:",
            );
            for reason in reasons {
                out.add(format!("             - {reason}"));
            }
            out.add(
                "             re-plan with overwrite to discard it and start the SCM process fresh",
            );
        }
    }
}

fn cap(n: Option<usize>) -> String {
    match n {
        Some(n) => n.to_string(),
        None => "no cap".to_string(),
    }
}

fn moved<T: PartialEq + Display>(field: &str, a: T, b: T) -> Option<PlanChange> {
    (a != b).then(|| PlanChange::new(field, format!("{a} -> {b}")))
}

/// Every way `next` differs from `prev` plan
fn diff_plans(prev: &ScmPlan, next: &ScmPlan, state: Option<&ScmState>) -> Vec<PlanChange> {
    let (p, n) = (&prev.options, &next.options);
    let ScmOptions {
        direction: _,
        forward_alpha,
        backward_alpha,
        num_rounds,
        max_retries,
        cov_step,
        final_cov_step,
    } = n;

    let mut changes: Vec<PlanChange> = [
        moved("model", &prev.model, &next.model),
        moved("direction", p.direction_label(), n.direction_label()),
    ]
    .into_iter()
    .flatten()
    .collect();

    for change in diff_candidates(&prev.candidates, &next.candidates) {
        let name = change.name();
        let load_bearing = match &change {
            CandidateChange::Removed { .. } => state.and_then(|s| s.depends_on(name)),
            _ => None,
        };
        let detail = match &load_bearing {
            Some(why) => format!("{name} {} — {why}", change.label()),
            None => format!("{name} {}", change.label()),
        };
        changes.push(PlanChange::new("candidates", detail));
    }

    changes.extend(
        [
            moved("forward_alpha", p.forward_alpha, *forward_alpha),
            moved("backward_alpha", p.backward_alpha, *backward_alpha),
            moved("max_retries", p.max_retries, *max_retries),
            moved("cov_step", on_off(p.cov_step), on_off(*cov_step)),
            moved(
                "final_cov_step",
                on_off(p.final_cov_step),
                on_off(*final_cov_step),
            ),
            moved("num_rounds", cap(p.num_rounds), cap(*num_rounds)),
        ]
        .into_iter()
        .flatten(),
    );
    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::scm::plan::build_plan;
    use crate::scm::test_support::{
        TEMPLATE, mid_scm_state, write_template, write_template_content,
    };
    use crate::scm::{Covariates, ScmOptions};
    use std::path::Path;

    /// Build a plan for the shared test model into `out_dir`, with the
    /// candidates and options given.
    fn plan_for(model: &Path, cands: &[&str], options: ScmOptions, out_dir: &Path) -> ScmPlan {
        build_plan(
            model,
            &Covariates::named(cands),
            Some(out_dir),
            options,
            "test",
        )
        .unwrap()
        .plan
    }

    #[test]
    fn a_fresh_out_dir_adds_nothing_to_the_plan() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template(dir.path());
        let out = dir.path().join("out");
        let built = build_plan(
            &model,
            &Covariates::named(&["WT_CL", "CRCL_CL"]),
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
            &Covariates::named(&["WT_CL", "CRCL_CL"]),
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
    fn a_candidates_new_initial_estimate_is_a_retune_the_process_resumes_under() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template(dir.path());
        let out = dir.path().join("out");
        let previous = plan_for(&model, &["WT_CL"], ScmOptions::default(), &out);
        previous.save().unwrap();
        mid_scm_state(&previous).save(&out).unwrap();

        // The initial model now carries a real initial guess for the effect, so
        // every model that tests it starts somewhere else.
        let edited = TEMPLATE.replace("$THETA (0 FIX)   ; WT_CL cov", "$THETA 0.4   ; WT_CL cov");
        write_template_content(dir.path(), &edited);

        let built = build_plan(
            &model,
            &Covariates::named(&["WT_CL"]),
            Some(&out),
            ScmOptions::default(),
            "test",
        )
        .unwrap();
        let text = built.render_text();
        assert!(text.contains("WT_CL initial 0.1 -> 0.4"), "got:\n{text}");
        // A retune is not SCM-defining: the process picks up where it is.
        assert!(!built.context.state_is_stale(), "got:\n{text}");
        assert!(!text.contains("cannot resume"), "got:\n{text}");
        match &built.context.compatibility {
            Some(Compatibility::Compatible { retunes, .. }) => {
                assert_eq!(retunes[0].label(), "WT_CL: initial 0.1 -> 0.4");
            }
            other => panic!("{other:?}"),
        }
        // WT_CL is in the model, not in the open round, so nothing is refit.
        assert!(
            text.contains("takes effect from the next model written"),
            "got:\n{text}"
        );
        assert!(text.contains("WT_CL already in the model"), "got:\n{text}");
    }

    /// A bound moved on a candidate the open round is still testing: the
    /// plan says that round refits it.
    #[test]
    fn retuned_bounds_on_a_candidate_in_the_open_round_are_reported_as_a_refit() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template(dir.path());
        let out = dir.path().join("out");
        let previous = plan_for(&model, &["WT_CL", "WT_V"], ScmOptions::default(), &out);
        previous.save().unwrap();
        mid_scm_state(&previous).save(&out).unwrap();

        let mut bounded = plan_for(&model, &["WT_CL", "WT_V"], ScmOptions::default(), &out);
        let wt_v = bounded
            .candidates
            .iter_mut()
            .find(|c| c.name == "WT_V")
            .unwrap();
        wt_v.lower = Some(0.0);
        wt_v.upper = Some(2.0);

        let ctx = PlanContext::read(&bounded);
        assert!(!ctx.state_is_stale());
        let mut out_lines = Lines::new();
        ctx.render_into(&mut out_lines);
        let text = out_lines.finish();
        assert!(
            text.contains("retuning   : WT_V: bounds none -> (0, 2) — refitted in forward_round3"),
            "got:\n{text}"
        );
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
            &Covariates::named(&["WT_CL", "CRCL_CL"]),
            Some(&out),
            options,
            "test",
        )
        .unwrap();
        let text = built.render_text();

        assert_eq!(built.context.changes.len(), 1);
        assert!(!built.context.state_is_stale());
        assert!(!text.contains("cannot resume"), "got:\n{text}");
    }

    #[test]
    fn removed_and_moved_candidates_are_both_reported() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template(dir.path());
        let out = dir.path().join("out");
        let mut previous = plan_for(&model, &["WT_CL", "CRCL_CL"], ScmOptions::default(), &out);
        // Pretend the initial model used to carry WT_CL one theta earlier.
        previous.candidates[0].theta = 3;
        previous.save().unwrap();

        let built = build_plan(
            &model,
            &Covariates::named(&["WT_CL"]),
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
        assert!(text.contains("CRCL_CL removed THETA(5)"), "got:\n{text}");
        assert!(!text.contains("removing   :"), "got:\n{text}");
    }

    #[test]
    fn plan_json_is_untouched_by_the_context() {
        // The context is a rendering, never part of plan.json.
        let dir = tempfile::tempdir().unwrap();
        let model = write_template(dir.path());
        let out = dir.path().join("out");
        let plan = plan_for(&model, &["WT_CL"], ScmOptions::default(), &out);
        let json = plan.to_json().unwrap();
        assert!(!json.contains("progress"));
        assert!(!json.contains("changes"));
    }
}
