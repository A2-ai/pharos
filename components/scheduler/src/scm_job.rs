//! An SCM config resolved to its plan, and the sequence every submit runs
//! before it drives. Every front end goes through [`ScmJob`], the `pharos
//! nonmem scm` commands first among them, so what a submit resolves, checks,
//! refuses and writes is decided once, here; a refusal carries the steps
//! ahead as [`Step`]s, which a front end words its own way.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use config::{CONFIG_FILENAME, NonmemConfig};
use nonmem::scm::{self, BuiltPlan, ScmPlan, ScmRunStatus, ScmState, shown};
use nonmem::{Refusal, Step, check_model};

use crate::{DriverRecord, ScmDriver, ScmSlurmExecutor, SubmittedDriver, slurm};

/// What a submit takes. With the `cli` feature these are the CLI's flags,
/// flattened into `run` and `slurm submit`.
#[cfg_attr(feature = "cli", derive(clap::Args))]
#[derive(Debug, Clone, Default)]
pub struct ScmSubmitArgs {
    /// The SCM config (`<stem>scm.toml`) written by `scm init`, its
    /// covariates filled in
    pub config: PathBuf,
    /// Pause after this many rounds (the SCM process is resumable); overrides
    /// the config's `num_rounds`. 0 runs all rounds
    #[cfg_attr(feature = "cli", clap(long))]
    pub num_rounds: Option<usize>,
    /// Partition for the model fits; with --shared-node, the partition of the
    /// one node they all run on
    #[cfg_attr(feature = "cli", clap(long))]
    pub partition: Option<String>,
    #[cfg_attr(feature = "cli", clap(long))]
    pub account: Option<String>,
    /// Cap on fits running at once (default: every ready fit; with
    /// --shared-node, as many as the node's CPUs hold)
    #[cfg_attr(feature = "cli", clap(long))]
    pub max_concurrent: Option<usize>,
    /// Run every fit on one whole node instead of one slurm job per fit
    #[cfg_attr(feature = "cli", clap(long))]
    pub shared_node: bool,
    /// Discard the SCM process already in the out_dir and start this one fresh
    #[cfg_attr(feature = "cli", clap(long))]
    pub overwrite: bool,
}

/// What `scm plan` takes: a submit's flags without `--overwrite`. A plan
/// only reads the out_dir, so it has nothing to discard; it reports what a
/// submit would do with the SCM process already there.
#[cfg_attr(feature = "cli", derive(clap::Args))]
#[derive(Debug, Clone, Default)]
pub struct ScmPlanArgs {
    /// The SCM config (`<stem>scm.toml`) written by `scm init`, its
    /// covariates filled in
    pub config: PathBuf,
    /// Pause after this many rounds (the SCM process is resumable); overrides
    /// the config's `num_rounds`. 0 runs all rounds
    #[cfg_attr(feature = "cli", clap(long))]
    pub num_rounds: Option<usize>,
    /// Partition for the model fits; with --shared-node, the partition of the
    /// one node they all run on
    #[cfg_attr(feature = "cli", clap(long))]
    pub partition: Option<String>,
    #[cfg_attr(feature = "cli", clap(long))]
    pub account: Option<String>,
    /// Cap on fits running at once (default: every ready fit; with
    /// --shared-node, as many as the node's CPUs hold)
    #[cfg_attr(feature = "cli", clap(long))]
    pub max_concurrent: Option<usize>,
    /// Run every fit on one whole node instead of one slurm job per fit
    #[cfg_attr(feature = "cli", clap(long))]
    pub shared_node: bool,
}

impl From<ScmPlanArgs> for ScmSubmitArgs {
    fn from(args: ScmPlanArgs) -> Self {
        Self {
            config: args.config,
            num_rounds: args.num_rounds,
            partition: args.partition,
            account: args.account,
            max_concurrent: args.max_concurrent,
            shared_node: args.shared_node,
            overwrite: false,
        }
    }
}

/// An SCM config resolved to its plan, and the settings it runs under:
/// the submit's arguments over `[nonmem.scm]`.
pub struct ScmJob {
    /// The SCM config, as submitted (canonical)
    pub scm_config: PathBuf,
    pub built: BuiltPlan,
    /// The project's pharos.toml (canonical)
    pub config_path: PathBuf,
    pub nonmem_config: NonmemConfig,
    pub out_dir: PathBuf,
    /// The fits' partition
    pub partition: Option<String>,
    pub account: Option<String>,
    /// Unset: no cap (a shared node fills its CPUs)
    pub max_concurrent: Option<usize>,
}

impl ScmJob {
    /// Resolve the config to its plan — so validate it, and read where the
    /// SCM process in its out_dir stands — before anything is touched.
    /// `version` is the pharos version the plan records;
    /// `load_nonmem_config` finds the pharos.toml the fits run under.
    pub fn load(
        args: &ScmSubmitArgs,
        version: &str,
        load_nonmem_config: impl FnOnce() -> Result<(PathBuf, NonmemConfig)>,
    ) -> Result<Self> {
        let scm_config = args
            .config
            .canonicalize()
            .with_context(|| format!("no SCM config at {}", shown(&args.config)))?;
        let overrides = scm::ScmOverrides {
            num_rounds: args.num_rounds,
        };
        let built = scm::build_plan_from_config(&scm_config, &overrides, version)?;
        let (config_path, nonmem_config) = load_nonmem_config()?;
        // The plan resolved the comment dialect, `output_dir` and
        // `[nonmem.scm]` from the project the model sits in; the fits run
        // under the pharos.toml found from here. They have to be the same
        // file, or the process is planned under one project and run under
        // another.
        let project_root = config_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default()
            .canonicalize()
            .unwrap_or_default();
        let plan_root = built.plan.root.canonicalize().unwrap_or_default();
        if project_root != plan_root {
            bail!(Refusal {
                problem: format!(
                    "the pharos.toml in use ({}) is not the project the model belongs to ({})",
                    config_path.display(),
                    plan_root.join(CONFIG_FILENAME).display()
                ),
                steps: vec![Step::UseProject { root: plan_root }],
            });
        }
        let scm_settings = &nonmem_config.scm;
        let max_concurrent = args.max_concurrent.or(scm_settings.max_concurrent());
        Ok(Self {
            partition: args
                .partition
                .clone()
                .or_else(|| scm_settings.partition.clone()),
            account: args
                .account
                .clone()
                .or_else(|| scm_settings.account.clone()),
            max_concurrent,
            out_dir: built.plan.out_dir_path(),
            config_path: config_path.canonicalize()?,
            nonmem_config,
            scm_config,
            built,
        })
    }

    pub fn plan(&self) -> &ScmPlan {
        &self.built.plan
    }

    pub fn model_stem(&self) -> String {
        self.plan()
            .model_path()
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "scm".to_string())
    }

    /// The fits' partition, settled: the submit's, else the project's
    /// `[nonmem.scm] partition`, else the `[nonmem.slurm]` one.
    pub fn fits_partition(&self) -> Result<String> {
        slurm::resolve_partition(
            self.partition.as_deref(),
            self.nonmem_config.slurm.partition.as_deref(),
        )
    }

    /// Whether the SCM process in the out_dir has already run to completion:
    /// a submit without `overwrite` has nothing to do and is refused.
    fn completed(&self) -> bool {
        self.built
            .context
            .progress
            .as_ref()
            .is_some_and(|p| p.state.status == ScmRunStatus::Completed)
    }

    /// The note `overwrite` earns: the SCM process in the out_dir that
    /// `who` ("the driver") discards, as `<status>, N rounds complete`;
    /// `None` when none has started.
    pub fn discard_note(&self, who: &str) -> Option<String> {
        self.built.context.progress.as_ref().map(|p| {
            let n = p.state.completed_rounds();
            let s = if n == 1 { "" } else { "s" };
            format!(
                "{who} discards the SCM process in {} ({}, {n} round{s} complete) and starts fresh",
                shown(&self.out_dir),
                p.state.status
            )
        })
    }

    /// The initial model through NM-TRAN, as every fit's script runs it
    /// before `sbatch`: nothing when the project has no NM-TRAN to check
    /// with, the failure's output as the error.
    fn nmtran_check(&self) -> Result<()> {
        if self
            .nonmem_config
            .get_nmtrans_executable_path(None)
            .is_err()
        {
            return Ok(());
        }
        let model = self.plan().model_path();
        let result = check_model(&self.nonmem_config, &model)?;
        if !result.success {
            bail!(
                "NM-TRAN rejects the initial model {}:\n{}",
                shown(&model),
                result.stdout.trim()
            );
        }
        Ok(())
    }

    /// The screening every submit does before it drives, and `scm plan`
    /// only reports: whether the SCM process already in the out_dir resumes
    /// under this plan (or is discarded, with `overwrite`), and NM-TRAN's
    /// view of the initial model. Then the out_dir's `.gitignore`, which
    /// follows the project's current setting. Returns the note `overwrite`
    /// earns. The plan's warnings are the caller's to show
    /// (`built.warnings`). Whether `overwrite` is safe is the submit's own
    /// check ([`Self::refuse_overwrite_under_driver`]): a driver job runs
    /// this too, and its own record would otherwise stop it.
    pub fn screen(&self, overwrite: bool) -> Result<Option<String>> {
        let out_dir = shown(&self.out_dir);
        if !overwrite && self.completed() {
            bail!(Refusal {
                problem: format!("the SCM process in {out_dir} is already complete"),
                steps: vec![Step::Overwrite],
            });
        }
        if !overwrite && !self.built.is_resumable() {
            // The plan's own note says why.
            bail!(Refusal {
                problem: format!(
                    "the SCM process in {out_dir} cannot resume under this plan\n{}",
                    self.built.context.render_text().trim_end()
                ),
                steps: vec![Step::Overwrite],
            });
        }
        self.nmtran_check()?;
        self.built.write_gitignore()?;
        Ok(overwrite.then(|| self.discard_note("the driver")).flatten())
    }

    /// Everything a submit refuses on, then [`Self::screen`]: no driver may
    /// already be queued or running for the process, and `overwrite` may
    /// not discard a process whose driver may still be. Returns the note
    /// `overwrite` earns.
    pub fn screen_for_submit(&self, overwrite: bool) -> Result<Option<String>> {
        self.refuse_second_driver()?;
        if overwrite {
            self.refuse_overwrite_under_driver()?;
        }
        self.screen(overwrite)
    }

    /// The driver job's `scm run` flags: the settings already resolved
    /// here, so the job runs with exactly what was submitted.
    pub fn driver_job_flags(&self, args: &ScmSubmitArgs) -> Vec<String> {
        let mut flags = Vec::new();
        // As given, not as the plan holds it: `0` (run all rounds) overrides
        // a cap the config sets, and the driver reads the config again.
        if let Some(n) = args.num_rounds {
            flags.extend(["--num-rounds".to_string(), n.to_string()]);
        }
        if let Some(n) = self.max_concurrent {
            flags.extend(["--max-concurrent".to_string(), n.to_string()]);
        }
        for (flag, value) in [
            ("--partition", &self.partition),
            ("--account", &self.account),
        ] {
            if let Some(v) = value {
                flags.extend([flag.to_string(), v.clone()]);
            }
        }
        if args.shared_node {
            flags.push("--shared-node".to_string());
        }
        if args.overwrite {
            flags.push("--overwrite".to_string());
        }
        flags
    }

    /// One driver per SCM process: refuse while another is queued or
    /// running, or may be: alive, or not checkable from here (a login
    /// driver on another host, a driver job with no `squeue`). Two drivers
    /// on one process would each fit and save over the other. A driver that
    /// ended said so on its record, so "cannot tell" is left for one that
    /// was killed or still runs. The record, not the driver's job name,
    /// identifies it: two models with the same stem in one project share a
    /// job name.
    pub fn refuse_second_driver(&self) -> Result<()> {
        match DriverRecord::read(&self.out_dir) {
            Some(record) => record.refuse_second_driver(&self.out_dir, &self.scm_config),
            None => Ok(()),
        }
    }

    /// `overwrite` discards the process, so it is refused while its last
    /// driver may still be running: alive, or not checkable from here (a
    /// login driver on another host, a driver job with no `squeue`).
    pub fn refuse_overwrite_under_driver(&self) -> Result<()> {
        match DriverRecord::read(&self.out_dir) {
            Some(record) => record.refuse_if_alive(&self.out_dir, &self.scm_config),
            None => Ok(()),
        }
    }

    /// The driver record, when it names the slurm job this process runs in:
    /// this `scm run` is then the driver `scm slurm submit` queued.
    pub fn this_driver_job(&self) -> Option<DriverRecord> {
        DriverRecord::read(&self.out_dir).filter(|r| r.is_this_job())
    }

    /// The executor that runs each fit as a slurm job of its own, with
    /// `pharos_exe` the pharos the jobs call.
    pub fn slurm_executor(&self, pharos_exe: PathBuf) -> ScmSlurmExecutor {
        ScmSlurmExecutor {
            config_path: self.config_path.clone(),
            nonmem_config: self.nonmem_config.clone(),
            pharos_exe,
            partition: self.partition.clone(),
            account: self.account.clone(),
            max_concurrent: self.max_concurrent.unwrap_or(0),
        }
    }

    /// Drive the process to its end (or pause). The executor is dropped
    /// before returning, so a node allocation it holds is released even when
    /// the caller exits the process right after.
    pub fn run(&self, executor: Box<dyn scm::FitExecutor>, overwrite: bool) -> Result<ScmState> {
        if overwrite {
            self.cancel_discarded_fits()?;
        }
        let outcome = scm::run_scm(self.plan(), executor.as_ref(), overwrite);
        drop(executor);
        outcome
    }

    /// `overwrite` discards the process in the out_dir, the fits it left in
    /// the slurm queue with it: a driver that died without a stop leaves
    /// them running, and they would write into the directories the new
    /// process is about to use. They are cancelled and waited out of the
    /// queue first. If that cannot be confirmed, nothing is discarded: an
    /// overwrite that went ahead on a cancel that may not have worked
    /// would protect nothing.
    fn cancel_discarded_fits(&self) -> Result<()> {
        if !self.out_dir.is_dir() {
            return Ok(());
        }
        let cancelled = crate::scm_executor::cancel_recorded_fits_and_wait(
            &self.out_dir,
            crate::scm_executor::poll_interval(&self.nonmem_config),
        )
        .map_err(|e| Refusal {
            problem: format!(
                "the SCM process in {} was not discarded: its fits could not be confirmed out \
                 of the slurm queue ({e:#}); once they have ended",
                shown(&self.out_dir)
            ),
            steps: vec![Step::Overwrite],
        })?;
        if !cancelled.is_empty() {
            let jobs: Vec<String> = cancelled.iter().map(usize::to_string).collect();
            scm::report(format!(
                "cancelled the discarded SCM process's fits still in the slurm queue: job(s) {}",
                jobs.join(", ")
            ));
        }
        Ok(())
    }

    /// Queue the driver as a slurm job and return it: held until the record
    /// naming it is on disk, released then, cancelled if that fails. Both
    /// partitions are settled here, not in a job that fails later: the
    /// driver's is `driver_partition`, else the project's `[nonmem.scm]
    /// driver_partition`, else the cluster's default (it only waits); on a
    /// shared node it is the fits' partition, where they all run.
    /// `pharos_exe` is the pharos the driver job runs.
    pub fn submit_to_slurm(
        &self,
        args: &ScmSubmitArgs,
        driver_partition: Option<String>,
        pharos_exe: PathBuf,
        verbose: bool,
    ) -> Result<SubmittedDriver> {
        let driver_partition = if args.shared_node {
            self.partition.clone()
        } else {
            driver_partition.or_else(|| self.nonmem_config.scm.driver_partition.clone())
        };
        self.fits_partition()?;

        let driver = ScmDriver {
            config_path: self.config_path.clone(),
            nonmem_config: self.nonmem_config.clone(),
            pharos_exe,
            scm_config: self.scm_config.clone(),
            model_stem: self.model_stem(),
            partition: driver_partition,
            account: self.account.clone(),
            shared_node: args.shared_node,
            run_flags: self.driver_job_flags(args),
            verbose,
        };
        // The job is queued held, and released once the record names it:
        // the `scm run` inside it reads the record to know it is this
        // driver, not a second one.
        let submitted = driver.submit()?;
        let record = DriverRecord::slurm(
            submitted.job_id,
            args.shared_node,
            submitted.log_path.clone(),
        );
        if let Err(e) = record.save(&self.out_dir).and_then(|_| submitted.release()) {
            let _ = submitted.cancel();
            return Err(e.context(format!(
                "the SCM driver job {} was cancelled before it started",
                submitted.job_id
            )));
        }
        Ok(submitted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_refuses_a_missing_config() {
        let args = ScmSubmitArgs {
            config: PathBuf::from("/nowhere/run001scm.toml"),
            ..Default::default()
        };
        let err = match ScmJob::load(&args, "0.0.0", || {
            unreachable!("the config is resolved first")
        }) {
            Ok(_) => panic!("a missing config loaded"),
            Err(e) => e,
        };
        // The path as `shown` spells it: with the current drive on Windows.
        let expected = format!("no SCM config at {}", shown(&args.config));
        assert!(expected.ends_with("/nowhere/run001scm.toml"), "{expected}");
        assert!(format!("{err:#}").starts_with(&expected), "got: {err:#}");
    }

    /// A project holding the standard template and an SCM config over it,
    /// loaded as a job.
    fn fixture(dir: &Path) -> ScmJob {
        let template = include_str!("../../nonmem/test_data/scm/templates/standard.mod");
        let dataset = "ID,TIME,AMT,DV,WT,CRCL,AGE\n1,0,100,0,70,100,40\n";
        let config_path = dir.join(CONFIG_FILENAME);
        let project = r#"
            [nonmem]
            clean_level = 1
            default_version = "nm751"

            [nonmem.options]
            prsame = false
            prcompile = false
            prdefault = false
            tprdefault = false
            background = false
            nobuild = false
            maxlim = 2

            [nonmem.versions]
            nm751 = "/opt/nonmem/nm751"

            [nonmem.comments]
            type = "type1"
        "#;
        fs_err::write(&config_path, project).unwrap();
        fs_err::write(dir.join("1001.mod"), template).unwrap();
        fs_err::write(dir.join("data.csv"), dataset).unwrap();
        let scm_config = dir.join("1001scm.toml");
        fs_err::write(
            &scm_config,
            "model = \"1001.mod\"\ndirection = [\"forward\"]\n[covariates]\neffects = [\"WT_CL\"]\n",
        )
        .unwrap();
        let args = ScmSubmitArgs {
            config: scm_config,
            ..Default::default()
        };
        ScmJob::load(&args, "test", || Ok((config_path, NonmemConfig::default()))).unwrap()
    }

    /// A submit over a finished process is refused unless it overwrites it,
    /// and the refusal names the step that does.
    #[test]
    fn a_completed_process_is_refused_with_the_overwrite_step() {
        let dir = tempfile::tempdir().unwrap();
        let dir = dir.path().canonicalize().unwrap();
        let job = fixture(&dir);
        let mut state = ScmState::new(job.plan());
        state.status = ScmRunStatus::Completed;
        state.save(&job.out_dir).unwrap();

        let job = fixture(&dir);
        let err = job.screen_for_submit(false).unwrap_err();
        assert!(
            format!("{err:#}").contains("is already complete"),
            "got: {err:#}"
        );
        assert_eq!(
            err.downcast_ref::<Refusal>().map(|r| r.steps.clone()),
            Some(vec![Step::Overwrite])
        );
    }
}
