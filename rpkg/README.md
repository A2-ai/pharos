# hyperion (miniextendr)

Hyperion is an R interface to pharos for pharmaceutical model development workflows. This is the miniextendr-based version, replacing the previous extendr-based package in `hyperion/`.

## Prerequisites

- R >= 4.2
- Rust >= 1.85 (`rustup update stable`)
- autoconf (`brew install autoconf` on macOS)
- [just](https://github.com/casey/just) (optional, for build recipes)

## Quick Start

From the repo root:

```bash
just configure      # Configure the build (dev mode)
just rcmdinstall    # Compile Rust + install R package
```

Or without just:

```bash
cd rpkg
NOT_CRAN=true bash ./configure
cd ..
R CMD INSTALL rpkg
```

Then in R:

```r
library(hyperion)
```

## Development Setup

### Install R Dependencies

```bash
just install-deps
```

Or manually:

```r
install.packages(c(
  "devtools", "roxygen2", "testthat", "S7", "knitr",
  "cli", "fs", "lifecycle", "rlang", "tomledit"
))
```

### Install minirextendr

minirextendr is the scaffolding and tooling helper for miniextendr R packages:

```r
remotes::install_github("CGMossa/miniextendr", subdir = "minirextendr")
```

### Set Up Git Hooks (Recommended)

Pre-commit and post-merge hooks catch common issues before they reach CI:

```r
minirextendr::use_miniextendr_git_hooks("rpkg")
```

The **pre-commit** hook:
- Checks `cargo fmt` — blocks commit if Rust code isn't formatted
- Checks stale `configure` — blocks if `configure.ac` was edited without running `autoconf`
- Checks stale `NAMESPACE` — blocks if R wrappers changed without `devtools::document()`
- Warns about stale vendor tarball (non-blocking)

The **post-merge** hook:
- Reminds you to reconfigure + rebuild when build files changed after `git pull`

### Set Up rv (Optional)

[rv](https://github.com/a2-ai/rv) manages R version, repositories, and dependencies per-project.

Add minirextendr and other dev deps to `rproject.toml`:

```toml
use_lockfile = false

[project]
name = "hyperion"
r_version = "4.5"

repositories = [
  { alias = "ppm", url = "https://packagemanager.posit.co/cran/latest" },
  { alias = "CRAN", url = "https://cran.r-project.org" },
]

dependencies = [
  "devtools",
  { name = "minirextendr", git = "https://github.com/CGMossa/miniextendr", subdir = "minirextendr" },
  { name = "covr", install_suggestions = true },
  { name = "hyperion", path = ".", dependencies_only = false, install_suggestions = true },
]
```

Then:

```bash
rv sync    # Install all dependencies into rv/library/
```

## Build Recipes

| Command | Description |
|---------|-------------|
| `just configure` | Configure build (dev mode, no vendoring) |
| `just rcmdinstall` | Build and install R package |
| `just devtools-document` | Run roxygen2 (regenerate NAMESPACE + man pages) |
| `just devtools-test` | Run R test suite |
| `just devtools-test "pattern"` | Run tests matching a filter |
| `just check` | `cargo check` on the Rust crate |
| `just fmt` | `cargo fmt` on the Rust crate |
| `just clippy` | `cargo clippy` on the Rust crate |
| `just vendor` | Vendor all deps for CRAN release (requires cargo-revendor) |
| `just devtools-check` | Full `devtools::check()` |

## Adding Rust Functions

1. Write your function in a `.rs` file under `src/rust/`:

```rust
use miniextendr_api::miniextendr;

/// My function description
///
/// @param x Input value
/// @return Result
/// @export
#[miniextendr]
pub fn my_function(x: f64) -> f64 {
    x * 2.0
}
```

2. Make sure the file is reachable via `mod` from `src/rust/lib.rs`
3. Rebuild: `just configure && just rcmdinstall && just devtools-document`
4. R wrappers are auto-generated in `R/hyperion-wrappers.R`

No manual registration needed — `#[miniextendr]` functions self-register via linkme.

## Dependency Management

Pharos components (`nonmem`, `config`, `scheduler`) live in `components/` in the monorepo and are referenced via `[patch.crates-io]`:

```toml
[dependencies]
nonmem = "*"
config = "*"
scheduler = "*"

[patch.crates-io]
nonmem = { path = "../../../components/nonmem" }
config = { path = "../../../components/config" }
scheduler = { path = "../../../components/scheduler" }
```

For CRAN builds, `cargo-revendor --freeze` rewrites these to resolve from `vendor/`. Install cargo-revendor:

```bash
cargo install --git https://github.com/CGMossa/miniextendr cargo-revendor
```

## Migration from extendr

See [MIGRATION.md](MIGRATION.md) for the full extendr-to-miniextendr reference.

## Port Status

This package is an incremental port from the extendr-based `hyperion/` directory. Tracking issues: a2-ai/pharos#112-#118.
