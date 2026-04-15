# mini_hyperion

Monorepo containing the pharos CLI and the hyperion R package (miniextendr-based).

## Project Structure

```
mini_hyperion/
├── src/                  # pharos CLI binary
├── components/           # Shared Rust crates (used by both pharos CLI and hyperion R pkg)
│   ├── config/           # pharos.toml config parsing
│   ├── nonmem/           # NONMEM model parsing, copying, output files
│   ├── scheduler/        # SLURM/SGE job submission
│   └── utils/            # Shared utilities
├── hyperion/             # Original R package (extendr-based, being replaced)
├── rpkg/                 # New R package (miniextendr-based, port in progress)
│   ├── R/                # R source files
│   ├── src/rust/         # Rust code for R bindings (standalone workspace)
│   ├── tests/            # R test suite
│   ├── configure.ac      # Autoconf source (generates configure)
│   ├── MIGRATION.md      # Extendr → miniextendr migration reference
│   └── NAMESPACE         # R exports
├── Cargo.toml            # Root workspace (pharos CLI + components)
└── justfile              # Build recipes for rpkg development
```

## Two Workspaces

This repo has two independent Cargo workspaces:

1. **Root workspace** (`Cargo.toml`): pharos CLI + `components/*`
2. **rpkg workspace** (`rpkg/src/rust/Cargo.toml`): hyperion R package (standalone, required by miniextendr)

rpkg depends on `components/*` via `[patch.crates-io]` path overrides. The `version = "*"` in `[dependencies]` + local paths in `[patch.crates-io]` pattern allows `cargo-revendor` to vendor everything for CRAN builds.

## Build Commands

```bash
# rpkg development
just configure          # Configure R package build (dev mode)
just rcmdinstall        # Build and install hyperion R package
just devtools-document  # Run roxygen2 (NAMESPACE + man pages)
just devtools-test      # Run R tests

# Vendoring for release
just vendor             # Vendor all deps (requires cargo-revendor)

# Rust development (rpkg)
just check              # cargo check on rpkg
just fmt                # Format rpkg Rust code

# Pharos CLI (root workspace)
cargo build             # Build pharos CLI
cargo test              # Run pharos + component tests
```

## Capturing Command Output

**Always redirect long-running R/Cargo command output to a log file**, then read the log:

```bash
just devtools-document 2>&1 > /tmp/devtools-doc.log
just rcmdinstall 2>&1 > /tmp/rcmdinstall.log
just devtools-test 2>&1 > /tmp/devtools-test.log
```

Use the Read tool to read the log file — do NOT use `tail` or `head`.

## Sandbox Restrictions

Commands that compile code require `dangerouslyDisableSandbox: true`:

```bash
just configure          # Generates Makevars
just rcmdinstall        # R CMD INSTALL compiles Rust + R
just devtools-document  # Compiles via devtools::document()
just devtools-test      # May need to recompile
cargo check             # Rust compilation
```

## Critical: Configure Before R CMD Operations

**Always run `just configure` before any R CMD operation.** The configure script:

1. Generates `src/Makevars` from `src/Makevars.in`
2. In dev mode: removes `.cargo/config.toml` so cargo uses normal resolution
3. In CRAN mode: sets up source replacements pointing to `vendor/`

```bash
# CORRECT
just configure && just rcmdinstall

# WRONG — will fail or use stale code
R CMD INSTALL rpkg
```

## Edit `.in` Templates, Not Generated Files

- `rpkg/src/rust/.cargo/config.toml` → edit `rpkg/src/rust/cargo-config.toml.in`
- `rpkg/src/Makevars` → edit `rpkg/src/Makevars.in`
- `rpkg/src/hyperion-win.def` → edit `rpkg/src/win.def.in`
- `rpkg/configure` → edit `rpkg/configure.ac` (then run `autoconf`)

## Dependency Management (rpkg)

rpkg's `Cargo.toml` uses `version = "*"` for all non-crates.io deps:

```toml
[dependencies]
miniextendr-api = { version = "*", features = ["serde"] }
nonmem = "*"
config = "*"
scheduler = "*"

[patch.crates-io]
# miniextendr — from git
miniextendr-api = { git = "https://github.com/CGMossa/miniextendr" }
# pharos components — from local monorepo
nonmem = { path = "../../../components/nonmem" }
config = { path = "../../../components/config" }
scheduler = { path = "../../../components/scheduler" }
```

In dev mode, `[patch.crates-io]` resolves everything locally. For CRAN builds, `cargo-revendor --freeze` rewrites deps to resolve from `vendor/`.

## Adding New Rust Functions (rpkg)

1. Add `#[miniextendr]` function to a `.rs` file under `rpkg/src/rust/`
2. Ensure the file is reachable via `mod` declarations from `lib.rs`
3. Rebuild: `just configure && just rcmdinstall && just devtools-document`
4. R wrappers are auto-generated in `rpkg/R/hyperion-wrappers.R` during build

No `extendr_module!` or manual registration needed — functions self-register via linkme.

## Miniextendr Patterns (vs extendr)

See `rpkg/MIGRATION.md` for the full reference. Key differences:

| extendr | miniextendr |
|---------|-------------|
| `#[extendr]` | `#[miniextendr]` |
| `Robj` | `SEXP` |
| `extendr_module! { ... }` | removed (auto-registration) |
| `extendr_api::Result<T>` | `Result<T, anyhow::Error>` |
| `.into_robj()` | `.into_sexp()` |
| `rprintln!()` | `r_println!()` |
| `IntoDataFrameRow` derive | serde `#[derive(Serialize)]` + `to_r()` |
| `#[extendr(default = "X")]` | `#[miniextendr(defaults(param = "X"))]` |

## Port Status

Tracking issues: #112–#118 on a2-ai/pharos.

Ported modules live in `rpkg/src/rust/`. The original extendr code in `hyperion/src/rust/` is the reference for porting.
