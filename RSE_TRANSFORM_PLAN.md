# Plan: transform-aware RSE in the run summary

Status (2026-10-13): the first implementation (uncommitted, pharos + hyperion) passed
`Option<&BTreeMap<String, ParameterInfo>>` through every reader. Matt rejected that design: it replaced a
type that stated its intent (`DeclaredRandomEffects`) with a nested raw collection. It is being reworked
to the `DeclaredParameters` type below. The output should match the 9 pending `.snap.new` exactly.
Still pending after the rework: Matt's snapshot acceptance, commit placement (changes sit on
`update-mod`), hyperion tag bump.

## The bug

`get_summary` (`components/nonmem/src/output_files/mod.rs`) reports RSE as `|se / est| * 100` for
every parameter. The computation lives in `extract_parameters_from_table`
(`components/nonmem/src/output_files/ext.rs`, under `// TODO add comment parsing for transformations`).

For a THETA estimated on the log scale this gives `se / log(θ)`, which is meaningless and blows up
as θ → 1. The correct formulas already exist in `Transform::compute_rse`
(`components/nonmem-parser/src/comments/transforms.rs`) but are only called from tests:

- THETA + LogNormal: `sqrt(exp(se²) - 1) * 100`
- THETA + Logit: `(1 - p) * se * 100`
- otherwise: `se / |est| * 100`

The summary never sees transforms because it only pulls labels from the model
(`Model::get_parameter_names`), and threads `DeclaredRandomEffects` separately.

## Agreed

### Transforms from comments

- Type2 comments already carry `parameterization: Option<Transform>`.
- Type1 comments keep their raw strings (no conversion at parse time). Add
  `fn transform(&self) -> Option<Transform>` on `Type1Theta`, `Type1Omega`, `Type1Sigma`, implemented
  as `Transform::from_str(tag).ok()`. `:LOG` → `LogNormal`; `:stdev` (a scale, not a transform) → `None`.
  `Type1Theta::Covariate` has no tag → `None`.
- Add `transform()` on `ParsedThetaComment`, `ParsedOmegaComment`, `ParsedSigmaComment`, dispatching
  to Type1/Type2 like `name()` does.
- Type1 is more lenient than Type2: an unknown Type1 tag (`:LGO`) silently becomes `None` and falls back
  to the default RSE, while Type2 rejects the comment. Accepted.

### One declared type: `DeclaredParameters`

`DeclaredParameters` replaces `DeclaredRandomEffects` with the same intent ("what the control stream
declares"). It now goes down to the cell level and also carries what the comments say about each
parameter. It mirrors the model's structure, takes typed IDs, and owns its rules.

```rust
// components/nonmem-parser/src/types.rs, next to ParameterType
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParameterId { Theta(u32), Omega(u32, u32), Sigma(u32, u32) }
// FromStr parses NONMEM column headers ("THETA3", "OMEGA(2,1)"); Display writes them back.
// param_type() -> ParameterType. FromStr replaces ext.rs `parse_parameter_indices`.

// components/nonmem-parser/src/model/parameters.rs
pub struct ParameterInfo {
    pub label: Option<String>,        // Parsed*Comment::name()
    pub transform: Option<Transform>, // Parsed*Comment::transform(); None = no tag / unknown tag
}

pub enum DeclaredParameters {
    /// $THETA/$OMEGA/$SIGMA in the control stream define the parameters.
    ControlStream {
        thetas: Vec<ParameterInfo>, // THETA(n) -> thetas[n-1]
        omegas: Vec<DeclaredBlock>,
        sigmas: Vec<DeclaredBlock>,
    },
    /// $MSFI: the MSF file defines them; the control stream declares none.
    Msf,
}

pub struct DeclaredBlock {
    first: u32,                // first ETA/EPS index, 1-based
    size: u32,                 // BLOCK SAME already expanded
    cells: Vec<ParameterInfo>, // lower-triangular, row-major
}

impl DeclaredParameters {
    pub fn includes(&self, id: ParameterId) -> bool;
    pub fn info(&self, id: ParameterId) -> Option<&ParameterInfo>;
    pub fn iter(&self) -> impl Iterator<Item = (ParameterId, &ParameterInfo)>;
}

impl Model {
    pub fn declared_parameters(
        &self,
        comment_type: Option<CommentType>,
    ) -> anyhow::Result<DeclaredParameters>;
}
```

- **`includes`:** `Msf` → `true`. `Theta(_)` → always `true` (resolved decision 2). `Omega(i,j)` /
  `Sigma(i,j)` → `true` iff some block has `first <= j <= i < first + size`. With no `$SIGMA` there
  are no sigma blocks, so NONMEM's placeholder `SIGMA(1,1)` is dropped, same as before. Separate diagonal
  values (`$OMEGA 0.1 0.2`) are blocks of size 1, so cells between them are not declared.
- **`info`:** `None` under `Msf`, for a THETA past `thetas.len()`, and for an undeclared cell.
- **`iter`:** declaration order (thetas, then OMEGA, then SIGMA, each row-major within a block);
  empty under `Msf`. Feeds `Summary.parameter_names` and hyperion's name listing.
- **Comments never decide membership.** The blocks come from the AST; `comment_type` only fills
  `label`/`transform`. With `None`, every `ParameterInfo` is default.
- **Keep `ParameterInfo` minimal.** Param type and diagonal-ness follow from the ID; `fixed` comes from
  the ext fixed row. Copying them in creates a second source of truth.
- **Strings only at the edges.** Readers parse each header into a `ParameterId` once. The external
  contract stays string-keyed: `Summary.parameter_names` (built from `iter()` + `Display`, no JSON
  change), `update_initial_estimates`, user `excluded` lists.
- **RSE:** `info.and_then(|i| i.transform.clone()).unwrap_or_default().compute_rse(value, se, &param_type)`.
  The default is applied where it's used: Identity reproduces the old formula, and OMEGA/SIGMA fall
  through to it.

### Call sites

All take `declared: &DeclaredParameters`. No `Option` wraps the type anywhere.

| Site | Change |
|---|---|
| `ext.rs` `EstimationTable::reported_columns` | `declared.includes(id)`. `ReportedColumn` carries `id: ParameterId` in place of `param_type` + `coordinate`. |
| `ext.rs` `annotate_table` | Label/transform via `declared.info(id)`. RSE via `compute_rse`. |
| `cor.rs` `CorReader::parse` / `parse_file` | Parse each header into a `ParameterId` (error on a malformed one, as ext does), keep iff `declared.includes(id)`. |
| `output_files/mod.rs` `get_summary` | `let declared = model.declared_parameters(comment_type)?;` pass `&declared` to ext and cor. `parameter_names` from `declared.iter()`. |
| `grd.rs` (~174) | Label via `declared.info(id)`. Drop the `unwrap_or_default()`. |
| `copy.rs` `read_estimates` | `model.declared_parameters(None)?`. No labels, so `THETA1` keys stay intact for `update_initial_estimates`, and filtering stops the NaN check firing on phantom columns. |

Tests build `DeclaredParameters` by parsing inline models through `Model::declared_parameters`, not by
hand-assembling maps.

### Deletions

- `Model::get_parameter_names`
- `DeclaredRandomEffects`, `Model::declared_random_effects()`, re-export in `nonmem-parser/src/lib.rs`,
  and its test in `parameters.rs`
- From the first implementation: `Model::parameters`, `column_is_kept` (`output_files/mod.rs`),
  `parse_parameter_indices` (`ext.rs`), `ReportedColumn.param_type`/`.coordinate`
- `Model::parse_comments`, the `parsed_comment` fields on `ThetaParameter`/`OmegaSigmaParam`, and
  `ParsedRaneffComment` (hyperion migrated; see "Hyperion").

Tests to rewrite: the `undeclared_sigma_columns_are_skipped` tests, cor tests, and `copy.rs` tests that
build `DeclaredRandomEffects` by hand → build a map without SIGMA keys instead.

## Resolved decisions (2026-10-11)

1. **(b)**: `ext` returns raw estimates keyed by NONMEM name; a join step in `output_files` applies
   membership, labels, RSE, and shrinkage to the final table only. The shrinkage block matched to that
   table (by method + occurrence, as `get_estimation_results` does today) must be carried to the join.
2. **THETAs are never filtered** (today's behaviour). A THETA column missing from the map is kept,
   unlabeled, default RSE. Membership filtering applies to OMEGA/SIGMA only. Rationale: the summary
   reads the `.mod` copy inside the run dir, so a mismatch needs that copy edited; an error would only
   catch count mismatches, not reorders.
3. **Keep `Summary.parameter_names`** as a label-only projection; no JSON change.
4. **`$MSFI` → `DeclaredParameters::Msf`** (was `None`; reworked 2026-10-13). NONMEM docs: "When $MSFI is used in a problem specification, $THETA, $OMEGA
   and $SIGMA records should not be used" (nmusers.github.io/docs/reference-manual/control-records/msfi/).
5. **ETA/EPS numbering and shrinkage index derive from the `(i,i)` coordinate**, not from counting
   earlier diagonals.

Findings from code review of the plan:
- `Type1Theta::Type { typ, parameterization }` (e.g. `CL :LOG`) carries a tag and must map in
  `transform()`. Field spelled `parametrization` on `WithUnit`, `parameterization` elsewhere.
- RSE is computed before param-type detection in `extract_parameters_from_table`; the join must know
  the type.
- `GrdReader` has no production caller (test only).
- No run fixture has a log/logit THETA; RSE coverage needs new unit tests on the join (inline models,
  no new `.mod` fixtures). All 8 summary snapshots change via `correlation_matrix.parameters` only.
- `validate_comments` (`parameters.rs:152`) still parses comments separately.

### Hyperion

Policy (Matt): pharos API is designed on its merits; hyperion (`~/Documents/hyperion`) changes in
lockstep. It depends on pharos via git deps pinned to tag `v0.6.0` (`src/rust/Cargo.toml:41-44`).
Verify hyperion against the local pharos without editing its manifest:
`cargo check --config 'patch."https://github.com/a2-ai/pharos".nonmem.path="/Users/mattsmith/Documents/pharos/components/nonmem"' --config 'patch."https://github.com/a2-ai/pharos".nonmem-parser.path="..."'`
(plus `config`/`scheduler` if needed). Bumping the tag is Matt's step.

Hyperion uses (all under `src/rust/nonmem/src/`):
- `model.parse_comments(ct)` (`model/mod.rs:28`) and `parsed_comment` reads
  (`model/comment_info.rs:122,134,156`). **Delete all three anyway** (`parse_comments`,
  `parsed_comment` fields, `ParsedRaneffComment`): caching derived data in the AST via a mutating call
  is the bad design. Hyperion's `comment_info.rs` calls the public `parse_theta_param` /
  `parse_omega_param` / `parse_sigma_param` on `param.comment` directly.
- Comment field names matched by name (`comment_info.rs:28-104`). **Unify the spelling**:
  `Type1Theta::WithUnit.parametrization` → `parameterization` (matches every other comment struct);
  update hyperion. The AST's block-level `Parametrization`/`block.parametrization` (Cholesky etc.) is a
  different concept read by R by name (`R/model-methods.R:419,518`); leave it.
- `get_parameter_names`, `declared_random_effects`, 6-arg `get_parameter_estimates`, reading
  `theta[].rse` etc. (`model/parameters.rs:138-200`). Its displayed RSE has the same bug. The join from
  decision 1 is public; hyperion switches to `Model::declared_parameters()` + the join.
  - `get_parameters` (`model/parameters.rs:~142,151`): pass `&declared` to `annotate_table`.
  - `get_model_parameter_names` (`model/parameters.rs:~229`): build from `declared.iter()`. It
    currently re-sorts string keys with `compare_param_names` to undo BTreeMap's alphabetical
    `OMEGA(10,10)` order. Check what order R receives today and keep it. If declaration order
    matches, drop the re-sort.
- `get_summary` and `Summary`/`TableParameters`/`ThetaEstimate`/`RandomEffectEstimate` fields
  (`model/summary.rs:229,299-316`); update hyperion for any field change.
- `Transform::{from_str, compute_rse, compute_cv, compute_ci, back_transform}`
  (`output_files/transforms.rs`). Hyperion's `compute_rse` doc comment (:217-221) wrongly claims
  LogNormal OMEGA/SIGMA use `sqrt(exp(se²)-1)`; fix the comment.

## Original open decisions (superseded by the above)

1. **Where labels and RSE are applied.**
   - (a) Thread the map into `ext` — smaller diff.
   - (b) `ext` returns raw estimates (`name, estimate, stderr, sd_corr, fixed`); a join step in
     `output_files` (e.g. `join_parameters(ext_table, params, shk, hide_off_diagonals) -> TableParameters`)
     applies membership, labels, RSE, shrinkage. Parser stays model-agnostic; only the last table is
     annotated (today every table is renamed/RSE'd and all but the last are discarded).
   - Either way, fix the dual meaning of `name`: `ext` overwrites `ThetaEstimate.name` /
     `RandomEffectEstimate.name` with the label, and `copy` only works because it passes no labels.
     Options: move renaming out of `ext`, or add `label: Option<String>` beside a stable `name`
     (JSON change).
2. **ext THETA column not in the map.** Today THETAs are never filtered; membership would silently drop
   them (e.g. `.mod` edited after the run, `ModelLayout` picking a stale `.mod`). Proposal: error on an
   unknown THETA column; skip unknown OMEGA/SIGMA cells.
3. **`Summary.parameter_names` (serialized).** Keep a label-only projection (`name -> info.label`, empty
   under `None`; no JSON change), or replace with `parameters: Option<BTreeMap<String, ParameterInfo>>`
   (JSON change, transforms visible downstream, `ParameterInfo` needs `Serialize`).
4. **Do `$MSFI` models carry commented `$THETA` in practice?** If yes, `None` discards those labels and
   we'd want `enum { Declared(map), FromMsf(..) }` instead of `Option`.

## Expected behaviour changes

- RSE changes for THETAs tagged log/logit (intended).
- Off-diagonals between separate `$OMEGA` records (e.g. `OMEGA(2,1)` with two diagonal `$OMEGA`) are no
  longer in `CorrelationMatrix.parameters` (`--include-all-correlations` output shrinks). Ext output is
  unchanged — those are already dropped by the fixed-flag filter.
- Type1 tags like `:add` / `:prop` now resolve to `AddErr` / `Proportional`; no effect on THETA RSE, but
  visible if transforms are serialized (decision 3).

Snapshots (`components/nonmem/src/output_files/snapshots/`) will change. Generate diffs for review; do
not run `cargo insta accept`.

## Brittle code to watch (not necessarily in scope)

- `get_random_effect_label` / `get_shrinkage_data` in `ext.rs` number ETA/EPS by counting diagonals
  pushed so far. Correct only because diagonals are never filtered. Under membership filtering, add a
  test or derive from the `(i,i)` index directly.
- `ParameterType::prefix()` returns `"THETA"` but `"ETA"`/`"EPS"` for the others — mixes two meanings.
- No NWPRI (`$THETAP/$OMEGAP/$PRIOR`) or `$MSFI` run fixtures exist; confirm ext THETA count matches
  `model.thetas` for prior models.
