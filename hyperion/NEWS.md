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
