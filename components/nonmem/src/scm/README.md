# `nonmem::scm` — Stepwise Covariate Modeling

Technical reference for the SCM module of pharos. Written for both humans and
LLM agents working on this code: it describes every file in `src/scm/`, what it
owns, which parts of the rest of pharos it draws on, and which concepts are ours
rather than inherited.

- **Crate**: `components/nonmem`, module `scm` (`components/nonmem/src/scm/`)
- **Public surface**: re-exported from `scm/mod.rs`; consumed by the `pharos scm`
  CLI in `src/main.rs` and by `components/scheduler/src/scm_executor.rs`
  (the slurm `FitExecutor`) and `scm_driver.rs` (submitting `scm run` itself)
- **Fixtures**: `components/nonmem/test_data/scm/templates/`
- **Snapshots**: `components/nonmem/src/scm/snapshots/`

---

## 1. What the module does

An SCM process takes an **initial model** whose `$THETA` records already declare
every covariate effect to be tested (each named by its comment, conventionally
written `(0 FIX)` so the effect is switched off), and drives the standard
stepwise selection:

1. **Reference fit** — a *base* model (forward-first: no effects released) or a
   *full* model (backward-only: every effect released). Not a round.
2. **Forward rounds** — each round fits one model per untested candidate, each
   releasing the retained effects plus that one candidate. The best significant
   candidate is added to the retained set and becomes the next round's reference.
3. **Backward rounds** — each round fits one model per retained effect with that
   effect re-fixed. The least significant is dropped.
4. **Final model** — written with exactly the retained effects released,
   optionally re-fitted with `$COVARIANCE` on.

Scoring is a likelihood-ratio test on ΔOFV against the round's reference, with
per-phase alphas. Everything is **resumable**: state lives in `scm_state.json`
and is reconciled against what the fits left on disk on every read.

> Terminology note used consistently in code, docs and output: this is the **SCM
> process** / **covariate selection process**. It is never called a "search".

### Directory an SCM process owns

```
<model_dir>/scm/<stem>/            # the out_dir; everything below is SCM-owned
├── <stem>scm.toml                # the config (written by `scm init`)
├── plan.json                      # the resolved plan (written by `scm plan`)
├── scm_state.json                 # driver state; the resume record
├── scm_summary.json / .md         # whole-process record, rewritten each round
├── base/ | full/                  # the reference fit
├── forward_round1/ ... N/         # one dir per round
│   ├── <stem>_<cand>[_refitK][_tryN].mod  + its run output
│   └── round_summary.json / .md
├── backward_round1/ ... N/
└── final/<stem>_scm_final.mod
```

The out_dir name comes from `[nonmem.scm] out_dir` in `pharos.toml`, rendered
against the model dir like the run `output_dir` template (`scm/{{name}}` when
the table leaves it out). A timestamp in it is rejected at plan time: the process has to
be findable again. `[nonmem.scm]` also holds the `scm run` defaults `local`,
`max_concurrent`, `num_parallel`, `partition` and `account`, each overridable by
the matching flag.

### How `scm run` runs on the cluster

`pharos scm run` does not drive the process from the terminal. Unless `--local`
is given it submits the **driver** — one single-core slurm job named
`scm_<stem>`, running `pharos scm run --plan ... --foreground` (a hidden flag)
from the project directory — prints the job id and returns. The driver then
submits one job per fit through `ScmSlurmExecutor` and waits for them; its log
goes to the slurm log dir as `scm_<stem>_<jobid>.out`, and the process is
followed with `pharos scm status`. Before submitting, `scm run` looks in
`squeue` for a job with the same name and work dir and refuses to start a
second driver. Fits are independent jobs, not children of the driver, so a
driver killed by `scancel` or a time limit loses nothing: the same `scm run`
resumes. All of this lives in `components/scheduler/src/scm_driver.rs`.

`clear_previous_output` (overwrite) removes only `base/`, `full/`, `final/`,
`forward_roundN/`, `backward_roundN/`, the state file and the two
`scm_summary.*` files. `plan.json`, the config and anything a user put in the
directory are left alone.

---

## 2. File map

| File | Role |
|---|---|
| `mod.rs` | Module root: shared types (`ScmPlan`, `ScmOptions`, `Candidate`, `ThetaSpec`, `Covariates`, `Direction`), filename constants, plan rendering, `clear_previous_output`, small shared helpers |
| `config.rs` | The `<stem>scm.toml` dialect: parse, validate, `scm init` scaffolding, config → plan |
| `plan.rs` | `build_plan`: resolve covariate names against the initial model's `$THETA` comments, validate, emit `ScmPlan` + warnings |
| `state.rs` | `ScmState` / `RoundRecord` / `CandidateRecord` (the on-disk resume record), round scoring & ranking, and `ScmProcess` — the single read path for a live process |
| `score.rs` | Chi-squared LRT: `chi2_sf`, `chi2_isf`, `lrt`, and `Direction`'s phase-specific orientation (statistic, significance test, ranking) |
| `round.rs` | Model writing (`ModelWriter`), retry/jitter, round entry construction, reading a fit's outcome off disk, disk reconciliation |
| `roster.rs` | The candidate roster: `diff_candidates`, `compatibility` (can this plan resume this state?), and applying removals/retunes |
| `driver.rs` | `run_scm`: the orchestration loop — reference fit, rounds, waves, scoring, decisions, final model. Also `FitExecutor` / `LocalExecutor` |
| `progress.rs` | `PlanContext`: what a freshly built plan meets in its out_dir — prior progress and a field-by-field diff vs the previous plan |
| `summary.rs` | `ScmSummary` / `RoundSummary` / `CandidateSummary`: the heavy record, plus every text and markdown rendering (`scm status`, `scm summary`, `*_summary.md`) |
| `test_support.rs` | Test fixtures: templates, fabricated run output, `MockExecutor`, transcripts, insta settings |
| `snapshot_tests.rs` | All insta snapshot tests, in one module so snapshots land in `snapshots/` |

---

## 3. Pipeline, file by file

```
 scm init      config.rs::init_scm
                   ↓ writes <stem>scm.toml
 scm plan      config.rs::build_plan_from_config → plan.rs::build_plan → plan.json
                   ↓ (+ progress.rs::PlanContext for the rendering)
 scm run       scheduler::scm_driver::ScmDriver::submit   (sbatch → the job below)
   --foreground  driver.rs::run_scm
                   ├ roster.rs::compatibility      (resume / removals / retunes / refuse)
                   ├ round.rs::round_entries
                   ├ round.rs::ModelWriter::write|retry
                   ├ FitExecutor::fit              (LocalExecutor | ScmSlurmExecutor)
                   ├ round.rs::read_fit_outcome → state.rs::RoundRecord::score (score.rs::lrt)
                   └ summary.rs::build_summary → write_round_summary
 scm status    summary.rs::read_summary → render_text(brief)
 scm summary   summary.rs::read_summary → render_text(opts)
```

---

## 4. Per-file detail

The reference for each file is its own doc comments (`cargo doc -p nonmem
--no-deps --document-private-items`). The one thing they do not show as a whole
is the shape of the on-disk records, so that is kept here.

**Schema** (`scm_summary.json`; `round_summary.json` in each round directory is
one `RoundSummary` plus `generated, plan_digest, initial_model, out_dir,
scm_status, next` so it stands alone):

```
ScmSummary { generated, pharos_version, plan_digest, initial_model, out_dir,
             options, status, message, phase, updated, models_running,
             candidates, roster, retained, final_model, final_ofv,
             totals: Totals, rounds: [RoundSummary] }
  RoundSummary { round, direction, index, phase_index, complete,
                 reference_model, reference_ofv, reference_files, alpha,
                 retained_before, retained_after, removed_before, counts,
                 winner, decision, timing, candidates: [CandidateSummary] }
    CandidateSummary { candidate, action, status, model, selected, rank, theta,
                       ofv, delta_ofv, statistic, df, p_value,
                       critical_delta_ofv, significant, heuristics,
                       attempts, superseded, files, timing }
```

The record is deliberately heavy: everything a reader could want is
materialized once, by the driver, and every rendering (`scm status`, `scm
summary`, the markdown files) is a projection of it. Estimates are not in the
record; the renderers read each run's own `pharos_summary.json` (`Fits`).

---

## 5. What we draw from the rest of pharos

Nothing in `scm/` re-implements model parsing, model copying, running, or output
parsing. It composes them.

### `nonmem-parser` (`components/nonmem-parser`)

| Item | Used for |
|---|---|
| `Model::parse` | reading the initial model and every generated model |
| `Model::get_parameter_names(CommentType)` | **the** source of covariate names — same function `pharos nonmem summary` uses |
| `CommentType`, `ParsedThetaComment`, `parse_theta_param` | the project's comment dialect; stale-numbering detection |
| `ThetaParameter` | `ThetaSpec::from` |
| `Model::theta_spec_replacements`, `covariance_removal_replacements`, `render_with_replacements` | surgical, comment-preserving edits to the control stream |
| `Model::update_initial_estimates` | warm starts |
| `Transform::compute_ci` | the 95% CI in `--long` output |

### `nonmem` crate (the parent crate)

| Item | Used for |
|---|---|
| `ModelLayout` (`model_resolution.rs`) | stem/extension/model dir, `resolve_output_dir` (the project's `output_dir` template), `output_file` |
| `copy::{CopyOptions, UpdateType, copy_model, derive_model, write_model_copy}` | every generated model; jitter and estimate-carrying on retries |
| `copy::read_ext_estimates` | warm starts from a reference `.ext` |
| `check_dataset` (`dataset.rs`) | `$DATA` validation at plan time |
| `output_files::{Summary, get_summary, resolve_estimation_files}` | reading a finished run; `$EST FILE=` overrides |
| `output_files::lst::{LstSummary, RunHeuristics}` | minimization/abort status and the heuristic flags |
| `output_files::ext::ThetaEstimate` | per-effect estimate, stderr, RSE |
| `run::metadata::{RunStartFile, RunEndFile, RUN_START_FILENAME, RUN_END_FILENAME}` | run detection and timing |
| `run::signal_wrapper::TERMINATION_FILENAME` | detecting a killed run |
| `runner::run_models`, `run::RunOptions` | `LocalExecutor` |

### `config` (`components/config`)

`NonmemConfig` (the `[nonmem]` table: `output_dir` template, `comments.type`,
`[nonmem.scm]` via `ScmSettings`), `render_output_dir_template`,
`Config::load`, `find_config_dir_from`, `to_root_relative`, `to_config_relative`,
`CONFIG_FILENAME`. The plan stores project-root-relative paths, and metadata is
written only when the out_dir lives inside a pharos project.

### `utils` (`components/utils`)

`get_utc_now`, `format_duration`, `seconds_between`, `normalize_path`,
`write_json_to_file`.

### `scheduler` (`components/scheduler`)

Depends on `scm` (not the other way round): `scm_executor.rs` implements
`FitExecutor` over Slurm. It submits in windows of `max_concurrent`, detects
completion from the run end/termination files rather than from the scheduler
(submission is fire-and-forget), polls `squeue` every 30s, and declares a job
lost after 3 consecutive absences — which simply lets the SCM retry machinery
take over.

### External crates

`statrs` (chi-squared), `blake3` (plan digest), `serde`/`serde_json`, `toml`,
`fs-err`, `anyhow`, `log`, `insta` + `tempfile` (tests).

---

## 6. What is ours

Design decisions specific to this module, worth knowing before changing it:

1. **Covariates are named by `$THETA` comments, nothing else.** No `$PK` parsing,
   no theta numbers in the config. The accepted names are exactly what
   `pharos nonmem summary` prints. This makes inline/folded parameterizations
   work and keeps the config readable.
2. **The initial model declares the candidate space.** There is no code-generating
   step: every effect already exists as a `$THETA`, and the process only ever
   flips thetas between `(fixed FIX)` and a free spec. Generated models are
   therefore always valid NM-TRAN and always diffable against the original.
3. **Config / plan / state / summary are four separate artifacts.** The config is
   the human input; `plan.json` is the resolved, validated, path-normalized
   contract; `scm_state.json` is the driver's resume record; the summaries are
   the read-only heavy record. Readers never touch the state.
4. **Structural digest, not a template hash.** `ScmPlan::digest` hashes parsed,
   SCM-defining options only. Byte-level hashing of the model template was
   explicitly rejected.
5. **The roster makes the candidate list revisable without a restart.** Dropping a
   candidate that never won resumes; re-tuning an initial estimate or bounds
   resumes and refits only the affected candidate in the open round.
6. **Every stage is idempotent and resumable.** A model already on disk is not
   rewritten; a finished run is read rather than re-fitted; a round record is
   reused. `--num-rounds` is a deliberate pause, not a cap on the process.
7. **Retries carry state forward and jitter deterministically.** 5% jitter seeded
   from the destination file name (FNV-1a, stable across Rust versions), so
   retries are reproducible and snapshot-testable.
8. **Unusable is never "insignificant".** A fit that aborted, terminated, or ran
   out of retries is reported as unusable and excluded from scoring. The process
   still concludes the round, records why, and exits `2` from `scm run` so a
   pipeline notices.
9. **Readers reconcile with disk.** `scm status` mid-round reports runs that
   finished after the driver last wrote state, and scores them the same way the
   driver will.
10. **Ties break on `$THETA` order**, after p-value then ΔOFV, and the tie is
    logged.
11. **Slurm is the default executor.** Fits do not belong on a login node;
    `--local` is opt-in.

---

## 7. CLI surface (`src/main.rs`)

| Command | Does |
|---|---|
| `pharos scm init --model <model> [--overwrite]` | create `scm/<stem>/` and a starter `<stem>scm.toml` |
| `pharos scm plan --setup <config> [--num-rounds N] [--overwrite]` | validate, write `plan.json`, print the plan + warnings + out_dir progress/diff. Runs nothing |
| `pharos scm run --plan <plan.json> [--local] [--partition] [--account] [--num-parallel] [--max-concurrent 4] [--overwrite]` | run or resume; prints the brief summary at the end; exit `2` if any candidate was unusable |
| `pharos scm status <out_dir\|plan.json>` | brief rendering of where the process stands |
| `pharos scm summary <out_dir\|plan.json> [--round] [--candidate] [--long] [--timing] [--files]` | the full record |

`scm status` and `scm summary` accept either the out_dir or its `plan.json`.

---

## 8. Working on this module

- **Adding an option**: `ScmOptions` → `CONFIG_KEYS` in `config.rs` → the
  `render_init_config` template → decide whether it belongs in
  `ScmPlan::digest` (SCM-defining) or not (run control) → `diff_plans` in
  `progress.rs` → the plan and summary renderings. The digest test in `mod.rs`
  documents the intent.
- **Adding a candidate-level field**: `Candidate` → `CovariateRequest`/`Covariates`
  → resolution in `plan.rs` → `diff_candidates` + `compatibility` (retune or
  redefinition?) → `CandidateSummary`.
- **Changing an output**: the renderings are snapshot-tested. Run
  `cargo insta test --review -p nonmem` and read the diffs; they are the
  specification.
- **Testing without NONMEM**: `MockExecutor` + `write_fit_output`; use
  `transcript()` for end-to-end scenarios.
- **Consumers**: hyperion (the R package) reads `scm_summary.json` and
  `round_summary.json`. Treat those schemas as a public interface — the summary
  is intentionally heavy so R never has to recompute anything.
