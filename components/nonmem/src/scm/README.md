# `nonmem::scm` — Stepwise Covariate Modeling

Technical reference for the SCM module of pharos. Written for both humans and
LLM agents working on this code: it describes every file in `src/scm/`, what it
owns, which parts of the rest of pharos it draws on, and which concepts are ours
rather than inherited.

- **Crate**: `components/nonmem`, module `scm` (`components/nonmem/src/scm/`)
- **Public surface**: re-exported from `scm/mod.rs`; consumed by the `pharos scm`
  CLI in `src/main.rs` and by `components/scheduler/src/scm_executor.rs`
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
├── <stem>-scm.toml                # the config (written by `scm init`)
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

`clear_previous_output` (overwrite) removes only `base/`, `full/`, `final/`,
`forward_roundN/`, `backward_roundN/`, the state file and the two
`scm_summary.*` files. `plan.json`, the config and anything a user put in the
directory are left alone.

---

## 2. File map

| File | Role |
|---|---|
| `mod.rs` | Module root: shared types (`ScmPlan`, `ScmOptions`, `Candidate`, `ThetaSpec`, `Covariates`, `Direction`), filename constants, plan rendering, `clear_previous_output`, small shared helpers |
| `config.rs` | The `<stem>-scm.toml` dialect: parse, validate, `scm init` scaffolding, config → plan |
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
                   ↓ writes <stem>-scm.toml
 scm plan      config.rs::build_plan_from_config → plan.rs::build_plan → plan.json
                   ↓ (+ progress.rs::PlanContext for the rendering)
 scm run       driver.rs::run_scm
                   ├ roster.rs::compatibility      (resume / removals / retunes / refuse)
                   ├ round.rs::forward_entries|backward_entries
                   ├ round.rs::ModelWriter::write|retry
                   ├ FitExecutor::fit              (LocalExecutor | ScmSlurmExecutor)
                   ├ round.rs::read_fit_outcome → state.rs::RoundRecord::score (score.rs::lrt)
                   └ summary.rs::build_summary → write_round_summary
 scm status    summary.rs::read_summary → render_text(brief)
 scm summary   summary.rs::read_summary → render_text(opts)
```

---

## 4. Per-file detail

### `mod.rs` — shared vocabulary

Owns the types every other file speaks in, and the module's constants.

**Types**

- `Direction` — `Forward` / `Backward`. `phases()` always returns forward before
  backward regardless of how the config lists them.
- `ScmOptions` — `direction`, `forward_alpha` (0.05), `backward_alpha` (0.001),
  `num_rounds`, `max_retries` (3), `cov_step` (false), `final_cov_step` (true).
  `validate()` rejects empty/duplicated directions, alphas outside (0,1), and
  `num_rounds < 1`.
- `ThetaSpec` — a `$THETA` spec as NM-TRAN spells it (`lower`, `init`, `upper`,
  `fixed`). Its `Display` is the authoritative NM-TRAN rendering, including the
  rule that an upper bound cannot be written without a lower one (`-INF` is
  emitted). `validate()` enforces NM-TRAN's rule that an initial estimate must
  lie strictly inside its bounds. Converts `From<&nonmem_parser::ThetaParameter>`.
- `Candidate` — a resolved covariate effect: `name`, 1-based `theta`, `initial`
  (value when released), `fixed` (value it is pinned at when held out), optional
  `lower`/`upper`. `released_spec()` / `held_out_spec()` produce the two
  `ThetaSpec`s a generated model uses.
- `CovariateRequest` / `Covariates` — the `[covariates]` table as written.
  `CovariateRequest` has a hand-written `Deserialize` accepting **either** a bare
  string (`"WT_CL"`) **or** a row (`{ name = "WT_CL", initial = 0.1, fixed = 0 }`).
  Section-level `initial`/`fixed`/`lower`/`upper` act as defaults.
- `ScmPlan` — what `plan.json` holds: `created`, `pharos_version`, `model`,
  `out_dir` (both relative to the pharos project root), `candidates`, `options`,
  plus a `#[serde(skip)]` `root` recovered on load from the nearest `pharos.toml`.

**`ScmPlan::digest()`** — a blake3 hash over the *SCM-defining* fields only:
model, out_dir, direction, alphas, max_retries, cov_step, and `final_cov_step`
(only when true, so plans predating the flag keep their digest). Deliberately
**excludes** `num_rounds` (run control) and the candidate list (tracked by the
state's roster instead, which is what lets candidates be removed without a
restart). This is a structural digest over parsed values — there is no
byte-level hashing of the model template anywhere in this module.

**Rendering** — `render_text` / `render_text_with(PlanContext)` produce the
`<scm plan>` block `pharos scm plan` prints. `Lines` is the tiny line-accumulator
every rendering in the module (plan, status, summary, markdown) builds on.

**Helpers** — `max_models_for(n, phases) = 1 + phases·n(n+1)/2` (worst case,
excluding retries), `default_out_dir`, `project_config`, `path_for_plan`,
`sanitize_name`, `ofv_suffix`, `none_or_list`, `on_off`, `yes_no`.

**Constants** — `PLAN_FILENAME`, `STATE_FILENAME`, `ROUND_SUMMARY_{JSON,MD}`,
`RUN_SUMMARY_FILENAME` (`pharos_summary.json`), `SCM_SUMMARY_{FILENAME,MD}`,
`REFERENCE_ROUND` (`"reference"`), `NO_REFERENCE` (`"-"`).

---

### `config.rs` — the `<stem>-scm.toml` dialect

`ScmConfig { model, covariates, #[serde(flatten)] options }`.

- `parse` does an explicit unknown-key check against `CONFIG_KEYS` *before*
  deserializing, so a typo produces a listing of the valid keys rather than a
  serde message. `direction` is required; everything else defaults.
- `build_plan_from_config(config_path, overrides, version)` resolves `model`
  relative to the **config file's own directory**, applies `ScmPlanOverrides`
  (`num_rounds`, `overwrite` — the two run-control knobs that live on the CLI
  rather than in the config), optionally clears prior output, and calls
  `plan::build_plan`.
- `init_scm(model, overwrite)` creates `scm/<stem>/` and writes a starter config
  there with `model = "../../<file>"`. The template is generated from
  `ScmOptions::default()` and `CovariateRequest::{INITIAL, FIXED}`, so the
  comments can never drift from the real defaults; a test asserts the written
  file parses and needs only `effects` filled in.

---

### `plan.rs` — name resolution and validation

The heart of "which theta does `WT_CL` mean?".

- `theta_name_index` builds `UPPERCASE name → [(theta number, as-written name)]`
  from `Model::get_parameter_names(comment_type)` — **the same function
  `pharos nonmem summary` uses**. This is a deliberate guarantee: the names
  `scm plan` accepts are exactly the names the summary prints, and a test
  iterates the summary's own map to prove it.
- Names come **only from `$THETA` comments**. `$PK` is never parsed for this
  (dropped 2026-09-10), so inline/folded parameterizations work unchanged.
  Matching is case-insensitive; the authored spelling is what the plan stores.
- Ambiguity (one name on two thetas) and duplicate requests are hard errors, with
  messages that list what *is* available.
- `stale_theta_comment_numbers` warns when a Type2 comment claims a position that
  is not the theta's actual index.

`build_plan` also validates the surrounding project:

| Check | Why |
|---|---|
| `effects` non-empty | nothing to test otherwise |
| `initial != fixed` (section and per-row) | an effect whose initial estimate is its held-out value is never actually tested |
| `lower < upper`, finite values | NM-TRAN |
| model exists, parses, has `$ESTIMATION` | |
| `check_dataset` | `$DATA` must resolve |
| project `output_dir` template has no `timestamp` | the process re-reads each run by re-rendering the template, so it must be stable |
| `[nonmem.comments] type` is set | without a dialect no `$THETA` comment names anything |

**Value resolution order** (per effect):

- `initial` — the row's own value → the initial model's `$THETA` init (when it
  differs from the held-out value) → the `[covariates]` section default (0.1)
- `fixed` — the row's own value → the section default (0.0)
- `lower`/`upper` — the row's own → the section's → whatever the initial model's
  `$THETA` already carries

Warnings (non-fatal, printed by the CLI): stale comment numbering; the initial
model fixing a theta at a value other than the configured `fixed`; `$COVARIANCE`
present/absent vs `cov_step` and `final_cov_step`.

Returns `BuiltPlan { plan, warnings, context }` — `context` is read *before*
anything writes, so the rendering can describe the out_dir as it was found.

---

### `state.rs` — the resume record, and the one read path

**On-disk (`scm_state.json`)**

```
ScmState { plan_digest, roster, status, message, retained, reference_model,
           reference_ofv, phase, rounds, final_model, final_ofv,
           had_unusable, updated }
  RoundRecord { name, direction, reference_model, reference_ofv,
                candidates, winner, decision, complete }
    CandidateRecord { candidate, action, model, attempts, superseded, refit,
                      status, ofv, delta_ofv, df, p_value, significant,
                      heuristics, selected }
```

- `ScmRunStatus`: `planned | running | paused | completed | failed`.
- `CandidateStatus`: `pending | running | succeeded | unusable | withdrawn`.
  `unusable` = out of retries without a scoreable fit; `withdrawn` = removed from
  the plan while its round was open. Both count as *concluded* — they end the
  candidate for that round but are never scored as "insignificant".
- `CandidateRecord::refit_under_new_values()` moves the existing attempts into
  `superseded`, bumps `refit`, and resets the record to pending. This is how a
  mid-round retune is honoured without losing the record of what already ran.

**Scoring lives here, not in the driver.** `RoundRecord::score` writes
`delta_ofv` / `p_value` / `significant` onto any succeeded-but-unscored
candidate (using `score::lrt`); `scored`, `ranking`, `ranks` and `contenders`
derive from it. Ranking uses `Direction::rank`, so ties break on ΔOFV and then on
`$THETA` order (the earlier theta wins; the driver logs when this happens).
Because the *reader* calls `score` too, `scm status` run against a round the
driver hasn't finished writing back reports the same numbers the driver will.

**`ScmProcess`** — `read(out_dir)` (or `of(plan, out_dir, state)`) is the single
entry point every reader uses: `scm status`, `scm summary`, and a re-plan's
`PlanContext`. It loads the plan, loads or synthesizes the state, then calls
`round::reconcile_state_with_disk` to fold in runs that finished since the state
was last written, returning the models still running. **Reading never writes** —
the state file stays the driver's.

---

### `score.rs` — the statistics

Thin and deliberately isolated.

- `chi2_sf(x, df)` / `chi2_isf(p, df)` via `statrs`' `ChiSquared`. A test pins
  them against PsN 5.7.1's hard-coded (alpha, df) → critical-value table.
- `lrt(reference_ofv, candidate_ofv, df, direction) -> (delta_ofv, p_value)`.
- `Direction` gets its phase-specific behaviour here:
  - `statistic`: forward `max(-ΔOFV, 0)`, backward `max(ΔOFV, 0)`
  - `meets(p, alpha)`: forward `p < alpha` (add it), backward `p > alpha` (it can
    go)
  - `rank`: forward by ascending p, backward by descending p; ΔOFV breaks ties

---

### `round.rs` — writing models and reading fits

**`ModelWriter { template, candidates, with_metadata }`**

- `write(dest, released, reference_ext, cov_step, description, based_on)` —
  parses the initial model once, derives a copy, then:
  - **warm start**: pulls the reference fit's estimates from its `.ext` via
    `update::read_ext_estimates` and applies them as initial estimates
  - candidate thetas not in `released` get `(fixed FIX)`; released ones get
    `released_spec()`, inheriting any bound the template carries, and continue
    from the reference estimate **only when that estimate isn't exactly the
    held-out value** (a theta held out in the reference reports its pinned value,
    which would be a meaningless starting point)
  - `$COVARIANCE` is stripped or appended to match `cov_step`
  - the whole edit is one set of replacements rendered once
- `retry(prev_model, dest, ...)` — copies the *previous attempt*, carrying its
  estimates forward (final estimates if it finished, last iteration otherwise)
  and jittering thetas by `RETRY_JITTER` (5%) so an attempt that parked on a
  boundary starts off it. The jitter seed is FNV-1a over the destination file
  name — stable across Rust releases (so the written models can be snapshotted)
  and reproducible on resume, while successive attempts jitter differently.
- `scm_model_name(stem, candidate, attempt, refit)` →
  `1001_wt_cl`, `1001_wt_cl_try2`, `1001_wt_cl_refit2`, `1001_wt_cl_refit2_try2`.

**Reading a fit**

- `run_dir_for` / `ext_path_for` / `ext_path_in` — locate a run's output,
  honouring `$EST FILE=` overrides via `resolve_estimation_files`.
- `run_finished` — end marker or termination marker present.
- `read_fit_outcome` → `FitOutcome { started, finished, terminated, ofv,
  minimization_terminated, program_aborted, heuristics }`. `usable()` requires:
  finished, not terminated, has a finite OFV, minimization not terminated,
  program not aborted. A fit that aborted is **never scored**, not scored as
  insignificant.
- `write_run_summary` writes the standard `pharos nonmem summary` JSON
  (`pharos_summary.json`) into each run dir, so later reads are cheap and stable.
- `reconcile_state_with_disk` / `reconcile_round_with_disk` — fold finished runs
  into open rounds and re-score; return the models still running.

**Round construction**

- `forward_entries(plan, retained)` — one entry per untested candidate, releasing
  retained + that candidate; `df` = thetas gained.
- `backward_entries(plan, retained)` — one entry per retained effect, releasing
  everything else; `df` = thetas re-fixed.

---

### `roster.rs` — resumability across re-plans

The **roster** is the state's own list of every candidate the process has known,
each with an optional `Removal` and a list of `Retune`s. It is what allows the
candidate list to change without discarding the process (the plan digest
deliberately ignores candidates).

- `diff_candidates(prev, next)` → `CandidateChange::{Added, Removed, MovedTheta,
  HeldOutAt, Initial, Bounds}`. `Initial` and `Bounds` are **retunes**;
  everything else redefines the candidate.
- `compatibility(plan, state)` → 
  - `Identical` — resume as-is
  - `Compatible { removals, retunes }` — resume after recording them
  - `Incompatible { reasons, .. }` — needs `--overwrite`

| Change | Verdict |
|---|---|
| digest differs (model, direction, alphas, retries, cov steps) | incompatible |
| candidate removed that never won a round and isn't retained | **compatible** (removal) |
| candidate removed that was selected or is in the current model | incompatible |
| candidate added (or re-added after removal) | incompatible |
| theta number moved | incompatible — the initial model changed underneath |
| `fixed` (held-out) value changed | incompatible |
| `initial` or bounds changed | **compatible** (retune) |

- `apply_removals` marks the roster entry removed (stamped with the last
  concluded round) and, if the candidate sits in an open round, sets it
  `Withdrawn` — recorded, not scored.
- `apply_retunes` updates the roster entry, appends a `Retune`, and — if the
  candidate is in the open round — calls `refit_under_new_values()`, clearing the
  round's provisional decision and winner so the round is re-decided. Candidates
  already retained keep the values the rounds they ran in used.

---

### `driver.rs` — orchestration

**`FitExecutor`** is the seam between the SCM logic and how fits actually run:

```rust
trait FitExecutor {
    fn fit(&self, models: &[PathBuf]) -> Result<()>;   // blocks until the batch is done
    fn describe(&self) -> String;
    fn settings(&self) -> Result<NonmemConfig>;
}
```

- `LocalExecutor` (here) — `runner::run_models` in-process, `num_parallel`.
- `ScmSlurmExecutor` (in `components/scheduler`) — the CLI default.
- `MockExecutor` (in `test_support.rs`) — fabricates run output.

**`run_scm(plan, executor, overwrite)`**

1. optionally `clear_previous_output`
2. load state → `compatibility` → resume / apply removals+retunes / refuse with
   the reasons and a pointer to `--overwrite`
3. save `plan.json` beside the state, mark `running`
4. `drive(...)`, then persist the terminal status; on error record `failed` +
   message and still best-effort write the failing round's records

**`drive`**

- **Reference fit**: `base` (forward-first) or `full` (backward-only). A
  reference that previously ran out of retries is *restarted* (its `Unusable`
  status would otherwise make a resume replay the verdict without fitting
  anything); one left pending/running is resumed. Failure here aborts the whole
  process with a clear message.
- **Round loop**, per phase:
  - build entries, honour `num_rounds` (→ `Paused`, resumable)
  - `run_round_fits` (below)
  - score, pick the best contender, record the decision:
    forward `added X (p, dOFV)` / backward `dropped X (p, dOFV)`; the winner's
    model becomes the next reference
  - no contender → record why (`no candidate significant at alpha …`, or `no
    candidate could be scored (N unusable)`) and `advance_phase`
  - write `scm_state.json` + round/process summaries after every round
- **`advance_phase`** — forward → backward, unless nothing was retained, in which
  case the process is done.
- **`write_final_model`** — `final/<stem>_scm_final.mod` with the retained
  effects released, warm-started from the last reference; re-fitted with the
  covariance step when `final_cov_step`, and resumable like any other fit.

**`run_round_fits`** — the wave loop. Up to `max_retries + 1` waves; each wave
gives every unconcluded candidate exactly one attempt:

- write the model if it isn't already on disk (attempt 1 → `ModelWriter::write`,
  later attempts → `ModelWriter::retry` from the previous attempt)
- if a usable outcome is already on disk, record it without fitting (this is what
  makes resume free)
- otherwise mark `running`, batch it, `executor.fit(batch)`, then read every
  outcome back
- the state is saved before and after every batch
- anything still unconcluded after the last wave becomes `Unusable`

A resumed round reuses its existing `RoundRecord`, adds records for entries that
appeared, and tolerates candidates that were removed since it was written.

---

### `progress.rs` — "where does this out_dir already stand?"

`PlanContext::read(plan)` loads the previous `plan.json` and state, reads the
process **under the new plan**, and computes:

- `had_previous_plan`
- `changes: Vec<PlanChange>` — a field-by-field diff (`diff_plans`) covering
  model, direction, alphas, retries, cov steps, `num_rounds`, plus every
  `CandidateChange`. A removal that the state depends on is annotated with *why*
  (`selected in forward_round2`, `in the current model`).
- `compatibility` — the verdict from `roster.rs`

`render_into` appends the `progress:` / `changes:` / `removing:` / `retuning:` /
`note:` block to the plan rendering, so `pharos scm plan` over a live out_dir
reports where the process stands, what this plan changes, what takes effect from
the next round vs. what gets refit, and — if incompatible — exactly why it cannot
resume. None of this ever enters `plan.json`; it is purely a rendering.

---

### `summary.rs` — the record and every rendering

**Schema** (`scm_summary.json`, and `round_summary.json` per round):

```
ScmSummary { generated, pharos_version, plan_digest, initial_model, out_dir,
             options, status, message, phase, updated, models_running,
             candidates, roster, retained, final_model, final_ofv,
             totals: Totals, rounds: [RoundSummary] }
  RoundSummary { round, direction, index, phase_index, complete,
                 reference_model, reference_ofv, reference_files, alpha,
                 retained_before, retained_after, removed_before, counts,
                 all_succeeded, any_heuristics, any_unusable, winner, decision,
                 scm_status, next, timing, candidates: [CandidateSummary] }
    CandidateSummary { candidate, action, status, model, selected, rank, thetas,
                       initial, fixed, ofv, delta_ofv, statistic, df, p_value,
                       critical_delta_ofv, significant, heuristics,
                       attempts, superseded, files, timing }
```

It is deliberately **heavy**: everything a reader could want is materialized once
(critical ΔOFV from `chi2_isf`, ranks, per-attempt timing, file paths) rather
than recomputed by each consumer — including the R side (hyperion).

- `build_summary(plan, state, out_dir, settings)` is the single builder. `Build`
  caches each run directory reading (`RunReading`: timing from the run
  start/end metadata files, and the paths of `run_dir`/`.lst`/`.ext`/
  `pharos_summary.json`) so a run is read at most once per summary.
- `Fits` holds the parsed `pharos nonmem summary` of each run, keyed by relative
  path; `RoundSummary::effect_of` pulls a candidate's own `ThetaEstimate` from
  whichever model leaves that theta free (the candidate's model in a forward
  round, the *reference* in a backward round).
- `retained_before` is reconstructed by replaying each round's winner, so a
  round's summary is self-contained.
- `read_summary(out_dir)` = `ScmProcess::read` + `build_summary`, with
  `models_running` attached.
- `write_round_summary` writes `round_summary.{json,md}` into the round's own
  directory **and** rewrites `scm_summary.{json,md}` at the top.

**Renderings** — all built on `Lines`:

- `SummaryOptions { round, candidate, brief, long, timing, files }`.
  `brief` is `scm status` (header + one line per round) and is also what the end
  of `scm run` prints. `--long` adds estimate with RSE, 95% CI, df, attempts and
  condition number; `--timing` adds spans and totals; `--files` adds paths.
  `--round` accepts `2`, `round 2`, `forward_round1` or `reference`.
- `winner_first` orders a round's candidates by rank (then p).
- Markdown: `round_markdown` is shared between the standalone
  `round_summary.md` and the per-round sections of `scm_summary.md`.

---

### `test_support.rs` and `snapshot_tests.rs`

`test_support.rs` exists so every reader of an SCM process (status, round view,
summary, round summary, plan context) is exercised against the *same* fabricated
runs:

- `snapshot_settings(tmp)` — the insta settings every SCM snapshot binds,
  filtering the three things that change per run: timestamps, the temp dir (and
  its canonical form), and the plan digest (which hashes paths).
- Templates live as files under `test_data/scm/templates/` so `glob!`-driven
  tests iterate them: `standard.mod`, `inline.mod`, `bounded.mod`,
  `categorical.mod`, `free_theta.mod`, `multi_record.mod`, `pred.mod`.
  Helpers lay one down beside a dummy
  dataset and a `pharos.toml` declaring the comment dialect.
- `Fit` + `write_fit_output` fabricate what a finished pharos run of a given kind
  leaves on disk — no NONMEM required.
- `MockExecutor` (+ `full_scm_executor`, `failing_reference_executor`,
  `everything_unusable_executor`) drives whole runs.
- `transcript(plan, executor)` records an entire driver run — output, final
  state, file tree — as one string for end-to-end scenario snapshots.
- `mid_scm_state`, `fabricate_running_scm`, `make_plan`, `file_tree` for
  smaller cases.

`snapshot_tests.rs` keeps every snapshot in one module so the files land in
`src/scm/snapshots/` (34 snapshots), apart from the parser and output-file
snapshot directories elsewhere in the workspace. Coverage: generated models per
template variant, warm-started round-2 models, retry models, the starter config,
plan JSON/text for the main option sets, **every** `build_plan` and config error
message, re-plan renderings, fit-outcome classification, `scm status` across
states, `scm summary` views, round summary files, and full-run transcripts
(forward→backward, every candidate unusable, failing reference, mid-run
removal, mismatched plan refused then overwritten).

The snapshot count is intentionally trimmed — a prioritized set, not one per
permutation.

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
| `update::read_ext_estimates` | warm starts from a reference `.ext` |
| `check_dataset` (`dataset.rs`) | `$DATA` validation at plan time |
| `output_files::{Summary, get_summary, resolve_estimation_files}` | reading a finished run; `$EST FILE=` overrides |
| `output_files::lst::{LstSummary, RunHeuristics}` | minimization/abort status and the heuristic flags |
| `output_files::ext::ThetaEstimate` | per-effect estimate, stderr, RSE |
| `run::metadata::{RunStartFile, RunEndFile, RUN_START_FILENAME, RUN_END_FILENAME}` | run detection and timing |
| `run::signal_wrapper::TERMINATION_FILENAME` | detecting a killed run |
| `runner::run_models`, `run::RunOptions` | `LocalExecutor` |

### `config` (`components/config`)

`NonmemConfig` (the `[nonmem]` table: `output_dir` template, `comments.type`),
`Config::load`, `find_config_dir_from`, `to_root_relative`, `to_config_relative`,
`CONFIG_FILENAME`. The plan stores project-root-relative paths, and metadata is
written only when the out_dir lives inside a pharos project.

### `utils` (`components/utils`)

`get_utc_now`, `clock`, `format_duration`, `seconds_between`, `normalize_path`,
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
| `pharos scm init <model> [--overwrite]` | create `scm/<stem>/` and a starter `<stem>-scm.toml` |
| `pharos scm plan <config> [--num-rounds N] [--overwrite]` | validate, write `plan.json`, print the plan + warnings + out_dir progress/diff. Runs nothing |
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
