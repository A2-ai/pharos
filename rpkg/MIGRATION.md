# Extendr to Miniextendr Migration Guide

This document captures the patterns needed to port hyperion's Rust code from
extendr to miniextendr. It is a living reference — update it as new patterns
are discovered during the incremental port.

## Overview

| extendr | miniextendr |
|---------|-------------|
| `extendr_api::prelude::*` | `miniextendr_api::miniextendr` (the attribute macro) + specific imports |
| `Robj` (wrapper around SEXP) | `SEXP` directly, with `SexpExt` trait methods |
| `extendr_module! { ... }` | removed — functions self-register via `linkme` |
| `#[extendr]` | `#[miniextendr]` |
| `miniextendr_init!()` in lib.rs | replaces all module declarations |

## Entry Point (lib.rs)

**extendr:**
```rust
use extendr_api::prelude::*;

pub mod init;

extendr_module! {
    mod hyperion;
    use init;
    use hyperion_core;
    use hyperion_nonmem;
    use hyperion_scheduler;
}
```

**miniextendr:**
```rust
use miniextendr_api::miniextendr;

miniextendr_api::miniextendr_init!();

pub mod init;
// pub mod core;    // all submodules — no module registration needed
// pub mod nonmem;
// ...
```

Functions annotated with `#[miniextendr]` self-register via linkme's
`#[distributed_slice]`. No `extendr_module!` declarations anywhere.

## Function Attributes

### Basic export

**extendr:**
```rust
#[extendr]
pub fn read_model(path: &str) -> Result<Robj> { ... }
```

**miniextendr:**
```rust
#[miniextendr]
pub fn read_model(path: &str) -> Result<SEXP, anyhow::Error> { ... }
```

### Default parameters

**extendr:**
```rust
#[extendr]
pub fn submit(
    model: Robj,
    #[extendr(default = "FALSE")] overwrite: bool,
    #[extendr(default = "NULL")] partition: Option<String>,
    #[extendr(default = "1")] ncpu: Option<u8>,
) -> Result<()> { ... }
```

**miniextendr:**
```rust
#[miniextendr(defaults(overwrite = "FALSE", partition = "NULL", ncpu = "1"))]
pub fn submit(
    model: SEXP,
    overwrite: bool,
    partition: Option<String>,
    ncpu: Option<u8>,
) -> Result<(), anyhow::Error> { ... }
```

Defaults are gathered into a single `defaults(...)` attribute on the function,
not scattered across parameters.

### Renaming in R

**extendr:**
```rust
#[extendr(r_name = "get_comment_type")]
pub fn get_comment_type_wrap() -> Result<Robj> { ... }
```

**miniextendr:**
```rust
#[miniextendr(r_name = "get_comment_type")]
pub fn get_comment_type_wrap() -> Result<SEXP, anyhow::Error> { ... }
```

### Internal (non-exported) functions

**extendr:** uses roxygen `@keywords internal` in doc comments.

**miniextendr:**
```rust
#[miniextendr(internal)]   // adds @keywords internal, suppresses @export
pub fn validate_model_path_wrap(path: &str) -> Result<String, anyhow::Error> { ... }
```

Or `#[miniextendr(noexport)]` to just suppress `@export` without adding
`@keywords internal`.

## Error Handling

### The error trait approach

**extendr:** uses its own `extendr_api::Error` enum with `Error::Other(String)`.

```rust
use extendr_api::Result;  // = Result<T, extendr_api::Error>

fn foo() -> Result<Robj> {
    Err(extendr_api::Error::Other("something went wrong".into()))
}
```

**miniextendr:** uses standard `Result<T, E>` where `E: Display`. The
`#[miniextendr]` wrapper catches `Err` and converts it to an R error via the
error's `Display` impl.

```rust
use anyhow::{bail, Context};

#[miniextendr]
fn foo() -> Result<SEXP, anyhow::Error> {
    bail!("something went wrong");
}
```

### Mapping the ResultExt / OptionExt helpers

The hyperion `ResultExt` trait maps errors to `extendr_api::Error::Other`.
With miniextendr, use `anyhow::Context` instead:

**extendr (hyperion_core):**
```rust
pub trait ResultExt<T> {
    fn map_to_extendr_err(self, message: impl Into<String>) -> Result<T>;
}

// Usage:
fs::read_to_string(&path).map_to_extendr_err("")?;
Config::load(&p).map_to_extendr_err("Failed to load config")?;
```

**miniextendr:**
```rust
use anyhow::Context;

// Usage:
fs::read_to_string(&path).context("Failed to read file")?;
Config::load(&p).context("Failed to load config")?;
```

For the `extendr_err!` macro:
```rust
// extendr:
Err(extendr_err!("pharos config file not found"))

// miniextendr:
anyhow::bail!("pharos config file not found")
```

For `OptionExt`:
```rust
// extendr:
config.nonmem.ok_or_extendr_err("missing nonmem config")?;

// miniextendr:
config.nonmem.context("missing nonmem config")?;
// or:
config.nonmem.ok_or_else(|| anyhow::anyhow!("missing nonmem config"))?;
```

## R Object Manipulation (Robj vs SEXP)

### Creating R objects

| extendr | miniextendr |
|---------|-------------|
| `"hello".into_robj()` | `"hello".into_sexp()` (via `IntoR` trait) |
| `42.into_robj()` | `42i32.into_sexp()` |
| `true.into_robj()` | `true.into_sexp()` |
| `vec![1,2,3].into_robj()` | `vec![1i32,2,3].into_sexp()` |

### Named lists

**extendr:**
```rust
let result = list!(
    high_correlation_threshold = correlation_threshold,
    high_condition_threshold = condition_threshold
);
result.into_robj()
```

**miniextendr:**
```rust
use miniextendr_api::list;

let result = list!(
    high_correlation_threshold = correlation_threshold,
    high_condition_threshold = condition_threshold,
);
result.into_sexp()
```

The `list!` macro exists in both frameworks with similar syntax.

### Attributes

**extendr:**
```rust
model_robj.set_attrib("filename", name.into_robj())?;
model_robj.set_class(["hyperion_nonmem_model"])?;
let source = model.get_attrib("model_source");
```

**miniextendr:**
```rust
use miniextendr_api::ffi::{SexpExt, SEXP};

// set_attr takes (symbol_sexp, value_sexp)
sexp.set_attr(SEXP::symbol("filename"), name.into_sexp());

// set_class takes a SEXP character vector
sexp.set_class(vec!["hyperion_nonmem_model"].into_sexp());

// get_attr returns SEXP (R_NilValue if missing)
let source = sexp.get_attr(SEXP::symbol("model_source"));
// or get_attr_opt for Option<SEXP>
let source = sexp.get_attr_opt(SEXP::symbol("model_source"));
```

For `List`, there are convenience methods:
```rust
list.set_class_str(&["hyperion_nonmem_model"]);
list.set_names_str(&["col1", "col2"]);
```

### Inspecting SEXP values

| extendr | miniextendr |
|---------|-------------|
| `robj.inherits("class")` | `sexp.inherits_class(c"class")` |
| `robj.as_str()` | `String::try_from_sexp(sexp)?` or typed param |
| `robj.as_str_vector()` | `Vec<String>::try_from_sexp(sexp)?` |
| `robj.as_list()` | `List::from_raw(sexp)` then `.get()`, `.get_index::<T>()` |
| `robj.rtype()` | `sexp.type_of()` returns `SEXPTYPE` enum |
| `robj.is_null()` | `sexp.type_of() == SEXPTYPE::NILSXP` |

### Calling R functions

**extendr:**
```rust
call!("stop", advice)?;
call!("warning", advice)?;
```

**miniextendr:**
```rust
// For stop — just panic or return Err
panic!("{}", advice);
// or: anyhow::bail!("{}", advice);

// For warning
miniextendr_api::error::r_warning(&advice);
```

### Printing to R console

**extendr:**
```rust
rprintln!("Model {p:?} -> job ID {job_id}");
```

**miniextendr:**
```rust
use miniextendr_api::r_println;
r_println!("Model {p:?} -> job ID {job_id}");
```

## Serde (Serialization / Deserialization)

The hyperion code heavily uses extendr's serde support to convert Rust structs
to R lists and back.

**extendr:**
```rust
use extendr_api::serializer::to_robj;
use extendr_api::deserializer::from_robj;

let robj = to_robj(&model)?;
let model: Model = from_robj(&robj)?;
```

**miniextendr:** Enable the `serde` feature in Cargo.toml:
```toml
miniextendr-api = { ..., features = ["serde"] }
```

Then:
```rust
use miniextendr_api::serde::{to_r, from_r};

let sexp = to_r(&model)?;
let model: Model = from_r(sexp)?;
```

## Cargo.toml Changes

**extendr:**
```toml
[dependencies]
extendr-api = { git = "https://github.com/extendr/extendr", branch = "main", features = ["serde"] }
```

**miniextendr:**
```toml
[dependencies]
miniextendr-api = { git = "https://github.com/CGMossa/miniextendr", features = ["serde_r"] }

[build-dependencies]
miniextendr-lint = { git = "https://github.com/CGMossa/miniextendr" }
```

The `miniextendr-lint` build dependency provides compile-time source checks.

## Module Structure

**extendr** requires explicit module registration in every crate:

```rust
// In each sub-crate's lib.rs:
extendr_module! {
    mod hyperion_nonmem;
    use model;
    use output_files;
    use utils;
}

// In each submodule:
extendr_module! {
    mod model;
    use copy;
    use summary;
    fn read_model;
    fn check_dataset;
}
```

**miniextendr** — just delete all `extendr_module!` blocks. Functions annotated
with `#[miniextendr]` are automatically registered. The only requirement is that
the module is reachable via `mod` declarations from `lib.rs`.

## Workspace vs Flat Crate

The original hyperion uses a Cargo workspace with sub-crates (`core`, `nonmem`,
`scheduler`). With miniextendr, `#[miniextendr]` functions must all live in the
**same crate** that calls `miniextendr_init!()` — the linkme distributed slices
are per-crate.

**Options:**
1. **Flatten into one crate** — move all code into the `rpkg/src/rust/` crate
   as modules. This is simpler and what miniextendr expects.
2. **Keep sub-crates for pure logic** — sub-crates contain business logic
   (no `#[miniextendr]`), and the top-level crate re-exports with
   `#[miniextendr]` wrappers. This maintains separation but adds thin wrappers.

For hyperion, option 1 (flatten) is recommended since the sub-crates exist
primarily to organize extendr module registrations.

## Checklist Per Module

When porting a module:

- [ ] Remove `extendr_module! { ... }` block
- [ ] Replace `use extendr_api::prelude::*` with specific miniextendr imports
- [ ] Replace `#[extendr]` with `#[miniextendr]`
- [ ] Replace `#[extendr(default = "X")]` params with `#[miniextendr(defaults(param = "X"))]`
- [ ] Replace `#[extendr(r_name = "X")]` with `#[miniextendr(r_name = "X")]`
- [ ] Replace `Result<Robj>` with `Result<SEXP, anyhow::Error>`
- [ ] Replace `Robj` params with `SEXP` or typed params
- [ ] Replace `.into_robj()` with `.into_sexp()`
- [ ] Replace `rprintln!` with `r_println!`
- [ ] Replace `extendr_err!` with `anyhow::bail!`
- [ ] Replace `map_to_extendr_err` with `.context()`
- [ ] Replace `ok_or_extendr_err` with `.context()` or `.ok_or_else(|| anyhow!(...))`
- [ ] Replace `to_robj(&val)` / `from_robj(&val)` with `to_r(&val)` / `from_r(sexp)`
- [ ] Replace `call!("stop", ...)` with `panic!` or `bail!`
- [ ] Replace `call!("warning", ...)` with `r_warning()`
- [ ] Replace attribute access (`get_attrib`, `set_attrib`, `set_class`) with `SexpExt` methods
- [ ] Ensure the module file is reachable via `mod` from `lib.rs`
- [ ] Test that the function appears in generated R wrappers after build

## Port Order

Recommended incremental porting order (simplest first):

1. **core** — `set_panic_message`, `find_pharos_config_file` (2 functions, minimal deps)
2. **init** — `init` (1 function, uses config crate)
3. **nonmem/utils** — utility functions + `get_pharos_config` (5 exported functions)
4. **nonmem/model/mod** — `read_model`, `check_dataset`, `read_model_from_lst`
5. **nonmem/model/\*** — individual submodules (copy, summary, check, lineage, etc.)
6. **nonmem/output_files** — ext, grd, shk, transforms
7. **scheduler** — `submit_model_to_slurm`, `submit_model_to_sge` (complex, many params)

Port one module at a time, rebuild, and verify the R wrappers are generated.

## R-Side Changes

The R wrapper file changes from `R/extendr-wrappers.R` to
`R/hyperion-wrappers.R` (or whatever name miniextendr generates). The R-side
code in `R/*.R` should not need changes — the wrapper function signatures
remain the same from R's perspective.

NAMESPACE changes from:
```
useDynLib(hyperion, .registration = TRUE)
```
to the same (miniextendr still uses `.registration = TRUE`).
