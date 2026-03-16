# hyperion 0.4.2

# hyperion 0.4.0

## Breaking changes

- `copy_model()`: the `jitter` parameter now accepts a single numeric value only.
  Named vector input (e.g., `c("theta" = 0.05, "omega" = 0.1)`) is no longer
  supported. Jittering of omega matrices could produce non-positive definite
  matrices causing nonmem to fail. Jittering for these parameters was removed
  to prevent this issue.
- `get_model_summary()` has been fully removed. Use `summary(model)` instead.

## New features

- `get_model_parameter_info()` now supports model objects that are not yet run.
  - For completed runs, metadata is still sourced from `.lst`.
  - For `not_run`/`running` model objects, metadata is parsed directly from the model.
- `get_partition_info()` gives a data frame of available slurm partitions
- submit_model_to_slurm() now validates requested CPU counts against live 
  SLURM partition information before submission.
- SLURM submissions now warn when the chosen ncpu/partition combination 
  would leave the final node less than 50% utilized.


## Bug fixes

- Run heuristics now correctly report `NA` when a heuristic result is unavailable
  (e.g., covariance step not run), instead of silently defaulting to `FALSE`.
  Summary output shows a warning indicator for these cases.
- `summary()` now gracefully warns and falls back to NONMEM parameter names when
  comment parsing fails, instead of aborting.
- `get_parameter_names()` now returns an empty data frame with a warning instead
  of aborting when comment parsing fails.
- `estimation_time` and `covariance_time` in run details now return `NA` instead
  of `0.0` when not present in the output.
- Fixed raw unit parsing for nested delimiters in comments, including:
  - `(1/(mg*hr))`
  - `(1/[mg*hr])`
  - `[1/[mg*hr]]`
- Improved raw OMEGA off-diagonal parsing so theta-pair comments with slash names
  (for example `CL/F-V2/F`) are parsed as associated theta pairs instead of being
  split into incorrect fragments.

# hyperion 0.3.2

## New features

- Added `get_model_metadata()` to retrieve model metadata (`description`, `tags`,
  and `based_on`) directly from a model object.
- Updated documentation for model metadata retrieval and NONMEM comment parsing formats.

# hyperion 0.3.1

## New features

- updated `pharos` dependency. When submitting a previously run model, the 
  output directory will be deleted before submission to the grid.
- if NMTRAN is available on head nodes/where models are submitted to the 
  grid from, check_model will be run before submission.

## Bug fixes
- fixed issue with error message showing when ext/grd files were attempted to
  be read for summary(mod) for a running model before ext/grd files existed

# hyperion 0.3.0

## New Features

### Running Model Status Detection

- `get_run_status()` now returns `"running"` in addition to `"run"` and
  `"not_run"`. Detects active NONMEM execution by checking whether the .ext
  file contains final estimates.

### Summary Support for All Model States

- `summary()` on NONMEM models now produces distinct output for three states:
  - **Completed** (`"run"`): Full parameter table, heuristics, OFV (unchanged).
  - **Running** (`"running"`): Recent iterations and gradients from .ext/.grd
    files. New `n_iterations` parameter controls how many to show (default 10).
  - **Not run** (`"not_run"`): Model metadata (problem statement, dataset path)
    with submission hints.
- Console and R Markdown/Quarto rendering for running and not-run summaries via
  `print()` and `knit_print()` S3 methods.

### New Exported Function: `from_config_relative()`

- Resolves config-relative paths (stored relative to `pharos.toml` directory) to
  absolute paths.

### `get_model_dir()` Gains `absolute` Parameter

- `get_model_dir(mod, absolute = TRUE)` returns the absolute path instead of the
  config-relative path.

# hyperion 0.2.0

## New Features

### Parameter Comment System

- Add a structured parameter metadata system built on S7 classes:
  - `ThetaComment`, `OmegaComment`, `SigmaComment` store per-parameter fields like
    `name`, `display`, `description`, `unit`, `parameterization`, and
    `associated_theta` (OMEGA only).
  - `ModelComments` is a container for all parameters and validates cross-links
    (e.g., OMEGA `associated_theta` names).
- New entry point `get_model_parameter_info()` reads comments from a model or
  run output and returns a `ModelComments` object for inspection and reporting.
  - Lookup enrichment: `apply_lookup()` / `apply_lookup_defaults()` fill missing
    fields from a TOML lookup file and track provenance.
  - Update helpers: `update_param_info()` edits names/display/description/unit/
    parameterization with source tracking.
  - Audit helpers: `audit_parameter_info()` reports where each field came from
    (model, lookup, or default).

### Comment Parsing Modes

- Support for two comment parsing modes controlled via `pharos.toml`:
  - **type1**: Structured comment format with explicit field delimiters. Set
    `type = "type1"` in the `[nonmem.comments]` section of `pharos.toml`.
  - **raw** (default): Flexible parsing from raw comment text when no type is
    specified or type is set to any other value.
- New `use_type1_comments()` helper to configure `pharos.toml` for type1 comment
  parsing.
- Query helpers for reporting and labeling:
  - `get_parameter_names()` returns a NONMEM→user name/display mapping.
  - `get_parameter_transform()` and `get_parameter_unit()` retrieve per-parameter
    transforms/units.
  - `get_theta_names()` and `get_eta_labels()` generate labels for tables/plots.
- Add parameter transform calculations for post-processing estimates:
  - `compute_cv()` and `compute_rse()` compute CV% and RSE% with transform-aware
    formulas.
  - `compute_ci()` returns confidence intervals (with back-transform where
    appropriate).
  - `transform_value()` back-transforms estimates to the natural scale.
  - All functions support vector inputs with strict length checks and length-1
    recycling.
- Add model lineage helpers and expanded model summary/tree output.
- Expand NONMEM example data bundled with the package.
- Refresh documentation, man pages, and vignettes to cover new APIs and examples.

### Model Utilities

- New model accessor functions:
  - `get_model_name()` - Get the model name (filename without extension)
  - `get_model_dir()` - Get the model directory path (relative to pharos.toml)
  - `get_data_path()` - Get the dataset path from the model
- `check_dataset()` now automatically derives the model directory from the
  `model_source` attribute, removing the need for the `model_dir` argument.

## Breaking Changes

- `get_model_summary()` has been removed; use `summary(mod)`.
- Parameter data frame column renamed: `value` is now `estimate`.
- Test data relocated from `vignettes/test_data/` to `inst/extdata/`.
- `check_dataset()` no longer accepts a `model_dir` argument; the directory is
  now derived automatically from the model's `model_source` attribute.

## Dependencies

- Added `S7` to Imports for parameter comment classes.
- Added `fs` to Imports for path manipulation.
- Added `testthat (>= 3.3.2)` to Suggests for snapshot testing.
- Moved `tomledit` from Suggests to Imports.
