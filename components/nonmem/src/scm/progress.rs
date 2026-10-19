//! Where an SCM process already stands when its plan is (re)built, and what the new plan changes about it.

use std::fmt::Display;

use super::roster::{ChangeKind, Compatibility, compatibility, diff_candidates};
use super::state::{ScmProcess, ScmState};
use super::{Lines, ScmOptions, ScmPlan, none_or_list, on_off, plural};

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

/// What a freshly built plan meets in its out_dir: the SCM process there,
/// if one has started, read under the new plan, and how the new plan differs
/// from the one that process ran under (its state's `plan`).
#[derive(Debug, Clone, Default)]
pub struct PlanContext {
    pub changes: Vec<PlanChange>,
    pub progress: Option<ScmProcess>,
    pub compatibility: Option<Compatibility>,
}

impl PlanContext {
    pub fn read(plan: &ScmPlan) -> Self {
        let out_dir = plan.out_dir_path();

        let state = ScmState::load(&out_dir).ok().flatten();
        let previous = state.as_ref().map(|s| &s.plan);
        // The process is read under the *new* plan
        let progress = state.as_ref().and_then(|s| {
            let settings = super::project_config(&out_dir).ok()?;
            Some(ScmProcess::of(
                plan.clone(),
                &out_dir,
                Some(s.clone()),
                &settings,
            ))
        });
        PlanContext {
            changes: previous
                .map(|p| diff_plans(p, plan, state.as_ref()))
                .unwrap_or_default(),
            compatibility: progress.as_ref().map(|p| compatibility(plan, &p.state)),
            progress,
        }
    }

    /// Whether the plan has anything to say besides itself: with no process
    /// started in its out_dir there is nothing to compare it with.
    pub fn is_empty(&self) -> bool {
        self.progress.is_none()
    }

    fn headline(p: &ScmProcess) -> String {
        let s = &p.state;
        let rounds = match s.completed_rounds() {
            0 => "no rounds complete yet".to_string(),
            n => format!("{} complete", plural(n, "round")),
        };
        let phase = match &s.phase {
            Some(d) => format!(", {d} phase"),
            None => String::new(),
        };
        format!("{} — {rounds}{phase} (updated {})", s.status, s.updated)
    }

    /// The progress and changes sections on their own: what a refused
    /// submit shows.
    pub fn render_text(&self) -> String {
        let mut out = Lines::new();
        self.render_into(&mut out);
        out.finish()
    }

    /// Append the progress and changes sections to a plan rendering.
    pub(crate) fn render_into(&self, out: &mut Lines) {
        if self.is_empty() {
            return;
        }
        out.blank();

        if let Some(p) = self.progress.as_ref() {
            out.add(format!("progress     : {}", Self::headline(p)));
            out.add(format!(
                "selected     : {}",
                none_or_list(&p.state.retained)
            ));
            let removed: Vec<String> = p
                .state
                .removed_roster()
                .map(|e| e.removal_label())
                .collect();
            if !removed.is_empty() {
                out.add(format!("removed      : {}", removed.join(", ")));
            }
            if let Some(cur) = p.state.open_round() {
                out.add(format!(
                    "in round     : {} — {}/{} concluded",
                    cur.name,
                    cur.concluded(),
                    cur.candidates.len()
                ));
            }
            if !p.models_running.is_empty() {
                out.add(format!(
                    "running      : {} model(s)",
                    p.models_running.len()
                ));
            }
            if !p.models_queued.is_empty() {
                out.add(format!("queued       : {} model(s)", p.models_queued.len()));
            }
            if let Some(f) = &p.state.final_model {
                out.add(format!("final model  : {f}"));
            }
        }

        if self.changes.is_empty() {
            out.add(
                "changes      : none — identical to the config the SCM process was submitted under",
            );
        } else {
            out.add(format!(
                "changes      : {} vs the config the SCM process was submitted under",
                plural(self.changes.len(), "change")
            ));
        }

        // The one thing said past the changes line: a process that cannot
        // resume under the plan (a candidate added mid-process, above all)
        // needs --overwrite. Removals and retunes are applied silently
        // when the process resumes.
        let (Some(progress), Some(verdict)) = (&self.progress, &self.compatibility) else {
            return;
        };
        if verdict.reasons.is_empty() {
            return;
        }
        out.add(format!(
            "note         : the SCM process in {} belongs to the config it was submitted under; it \
             cannot resume under this one:",
            progress.plan.out_dir
        ));
        for reason in &verdict.reasons {
            out.add(format!("               - {reason}"));
        }
        out.add(
            "               so a submit is refused; submit with --overwrite to discard that \
             process and start fresh",
        );
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

/// The SCM-defining fields `next` changes from `prev`: the ones
/// [`ScmPlan::digest`] covers. A process resumes across all of them, under
/// the new values from where it stands.
pub(crate) fn digest_changes(prev: &ScmPlan, next: &ScmPlan) -> Vec<PlanChange> {
    let (p, n) = (&prev.options, &next.options);
    let ScmOptions {
        direction: _,
        forward_alpha,
        backward_alpha,
        num_rounds: _,
        max_retries,
        retry_jitter,
        cov_step,
        final_cov_step,
        forward_final_cov_step: _,
    } = n;
    [
        moved("model", &prev.model, &next.model),
        moved("direction", p.direction_label(), n.direction_label()),
        moved("forward_alpha", p.forward_alpha, *forward_alpha),
        moved("backward_alpha", p.backward_alpha, *backward_alpha),
        moved("max_retries", p.max_retries, *max_retries),
        moved("retry_jitter", p.retry_jitter, *retry_jitter),
        moved("cov_step", on_off(p.cov_step), on_off(*cov_step)),
        moved(
            "final_cov_step",
            on_off(p.final_cov_step),
            on_off(*final_cov_step),
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Every way `next` differs from `prev` plan: the SCM-defining fields, then
/// the candidates, then run control.
fn diff_plans(prev: &ScmPlan, next: &ScmPlan, state: Option<&ScmState>) -> Vec<PlanChange> {
    let mut changes = digest_changes(prev, next);
    for change in diff_candidates(&prev.candidates, &next.candidates) {
        let name = &change.name;
        let load_bearing = match &change.kind {
            ChangeKind::Removed => state.and_then(|s| s.depends_on(name)),
            _ => None,
        };
        let detail = match &load_bearing {
            Some(why) => format!("{name} {} — it was {why}", change.label()),
            None => format!("{name} {}", change.label()),
        };
        changes.push(PlanChange::new("candidates", detail));
    }
    changes.extend(moved(
        "num_rounds",
        cap(prev.options.num_rounds),
        cap(next.options.num_rounds),
    ));
    changes.extend(moved(
        "forward_final_cov_step",
        on_off(prev.options.forward_final_cov_step),
        on_off(next.options.forward_final_cov_step),
    ));
    changes
}

#[cfg(test)]
mod tests {
    use crate::scm::ScmOptions;
    use crate::scm::state::ScmState;
    use crate::scm::test_support::{
        TEMPLATE, mid_scm_state, plan_named, plan_of, write_template, write_template_content,
    };

    #[test]
    fn replanning_an_unchanged_plan_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template(dir.path());
        let out = dir.path().join("out");
        let plan = plan_of(
            &model,
            &["WT_CL", "CRCL_CL"],
            Some(&out),
            ScmOptions::default(),
        );
        ScmState::new(&plan).save(&out).unwrap();

        let built = plan_named(
            &model,
            &["WT_CL", "CRCL_CL"],
            Some(&out),
            ScmOptions::default(),
        )
        .unwrap();
        let text = built.render_text();

        assert!(built.context.changes.is_empty());
        assert!(
            text.contains(
                "changes      : none — identical to the config the SCM process was submitted under"
            ),
            "got:\n{text}"
        );
        // A process submitted but not yet fitting anything has no rounds to report.
        assert!(
            text.contains("progress     : planned — no rounds complete yet"),
            "got:\n{text}"
        );
    }

    #[test]
    fn a_candidates_new_initial_estimate_is_a_retune_the_process_resumes_under() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template(dir.path());
        let out = dir.path().join("out");
        let previous = plan_of(&model, &["WT_CL"], Some(&out), ScmOptions::default());
        mid_scm_state(&previous).save(&out).unwrap();

        // The initial model now carries a real initial guess for the effect, so
        // every model that tests it starts somewhere else.
        let edited = TEMPLATE.replace("$THETA (0 FIX)   ; WT_CL cov", "$THETA 0.4   ; WT_CL cov");
        write_template_content(dir.path(), &edited);

        let built = plan_named(&model, &["WT_CL"], Some(&out), ScmOptions::default()).unwrap();
        let text = built.render_text();
        // The change is counted, and nothing past the changes line details
        // it: a retune is not SCM-defining, the process picks up where it is.
        assert_eq!(built.context.changes.len(), 1);
        assert_eq!(built.context.changes[0].detail, "WT_CL initial 0.1 -> 0.4");
        assert!(text.contains("changes      : 1 change vs"), "got:\n{text}");
        assert!(!text.contains("initial 0.1 -> 0.4"), "got:\n{text}");
        assert!(!text.contains("cannot resume"), "got:\n{text}");
        assert!(text.trim_end().ends_with("submitted under"), "got:\n{text}");
        let verdict = built.context.compatibility.as_ref().unwrap();
        assert!(!verdict.is_incompatible(), "{verdict:?}");
        assert_eq!(verdict.retunes[0].label(), "WT_CL: initial 0.1 -> 0.4");
    }

    /// Changing the config's settings (alphas, retries, cov steps,
    /// direction) over a paused process: counted as changes, and the process
    /// resumes under them. Only a candidate added mid-process needs
    /// --overwrite, and that is the one thing said past the changes line.
    #[test]
    fn settings_changes_resume_and_only_an_added_candidate_needs_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template(dir.path());
        let out = dir.path().join("out");
        let previous = plan_of(
            &model,
            &["WT_CL", "CRCL_CL"],
            Some(&out),
            ScmOptions::default(),
        );
        mid_scm_state(&previous).save(&out).unwrap();

        let options = ScmOptions {
            forward_alpha: 0.01,
            max_retries: 9,
            cov_step: true,
            ..Default::default()
        };
        let built = plan_named(&model, &["WT_CL", "CRCL_CL"], Some(&out), options.clone()).unwrap();
        let text = built.render_text();
        assert_eq!(built.context.changes.len(), 3);
        assert!(text.contains("changes      : 3 changes vs"), "got:\n{text}");
        assert!(!text.contains("forward_alpha"), "got:\n{text}");
        assert!(!text.contains("overwrite"), "got:\n{text}");
        assert!(built.is_resumable());

        let added = plan_named(&model, &["WT_CL", "CRCL_CL", "WT_V"], Some(&out), options).unwrap();
        let text = added.render_text();
        assert!(!added.is_resumable());
        assert!(
            text.contains(
                "- WT_V is not part of this SCM process; adding a candidate needs overwrite"
            ),
            "got:\n{text}"
        );
        assert!(!text.contains("forward_alpha"), "got:\n{text}");
        assert!(
            text.contains("submit with --overwrite to discard that process"),
            "got:\n{text}"
        );
    }

    #[test]
    fn a_run_control_change_alone_leaves_the_state_resumable() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template(dir.path());
        let out = dir.path().join("out");
        let previous = plan_of(
            &model,
            &["WT_CL", "CRCL_CL"],
            Some(&out),
            ScmOptions::default(),
        );
        mid_scm_state(&previous).save(&out).unwrap();

        let options = ScmOptions {
            num_rounds: Some(1),
            ..Default::default()
        };
        let built = plan_named(&model, &["WT_CL", "CRCL_CL"], Some(&out), options).unwrap();
        let text = built.render_text();

        assert_eq!(built.context.changes.len(), 1);
        assert!(!text.contains("cannot resume"), "got:\n{text}");
    }

    #[test]
    fn removed_and_moved_candidates_are_both_reported() {
        let dir = tempfile::tempdir().unwrap();
        let model = write_template(dir.path());
        let out = dir.path().join("out");
        let mut previous = plan_of(
            &model,
            &["WT_CL", "CRCL_CL"],
            Some(&out),
            ScmOptions::default(),
        );
        // Pretend the initial model used to carry WT_CL one theta earlier.
        previous.candidates[0].theta = 3;
        ScmState::new(&previous).save(&out).unwrap();

        let built = plan_named(&model, &["WT_CL"], Some(&out), ScmOptions::default()).unwrap();
        let text = built.render_text();

        let details: Vec<&str> = built
            .context
            .changes
            .iter()
            .map(|c| c.detail.as_str())
            .collect();
        assert_eq!(
            details,
            [
                "WT_CL moved THETA(3) -> THETA(4)",
                "CRCL_CL removed from effects"
            ]
        );
        assert!(text.contains("changes      : 2 changes vs"), "got:\n{text}");
        // the process never selected CRCL_CL, so it carries on without it
        // (nothing past the changes line says so)...
        assert!(!text.contains("removing"), "got:\n{text}");
        // ...but a moved theta means the initial model changed under it
        assert!(text.contains("cannot resume"), "got:\n{text}");
        assert!(
            text.contains("- WT_CL moved THETA(3) -> THETA(4); the initial model changed"),
            "got:\n{text}"
        );
    }
}
