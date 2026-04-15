# https://just.systems
#
# Quick reference:
#   rpkg (hyperion R package):
#     just configure          - Configure R package build (dev mode)
#     just vendor             - Vendor deps for CRAN release prep
#     just rcmdinstall        - Build and install R package
#     just devtools-document  - Run roxygen2 (NAMESPACE + man pages)
#     just devtools-test      - Run R package tests
#     just devtools-check     - Run devtools::check
#
#   Rust (rpkg):
#     just check              - Run cargo check on rpkg
#     just fmt                - Format rpkg Rust code
#     just clippy             - Run clippy on rpkg
#
set shell := ["bash", "-euo", "pipefail", "-c"]

# Directory for devtools::check output (preserved for investigation)
check_output_dir := justfile_directory() / "rpkg-check-output"

[default]
default:
    @just --list

# ---- rpkg (R package) ----

# Configure rpkg for development (dev mode, no vendoring)
configure:
    cd rpkg && \
    if command -v autoconf >/dev/null 2>&1; then autoconf; else echo "autoconf not found; using existing configure"; fi && \
    NOT_CRAN=true bash ./configure

# Configure rpkg in CRAN/offline mode (run `just vendor` first)
configure-cran:
    cd rpkg && \
    if command -v autoconf >/dev/null 2>&1; then autoconf; else echo "autoconf not found; using existing configure"; fi && \
    NOT_CRAN=false bash ./configure

# Vendor dependencies for CRAN release preparation
# Requires cargo-revendor: cargo install --git https://github.com/CGMossa/miniextendr cargo-revendor
vendor:
    cargo revendor \
      --manifest-path rpkg/src/rust/Cargo.toml \
      --output rpkg/vendor \
      --strip-all \
      --freeze \
      --compress rpkg/inst/vendor.tar.xz \
      --blank-md \
      --source-marker \
      -v

# Install rpkg with R CMD INSTALL
alias rcmdinstall := r-cmd-install
r-cmd-install *args: configure
    R CMD INSTALL {{args}} rpkg

# Build R package tarball
alias rcmdbuild := r-cmd-build
r-cmd-build *args: configure
    R CMD build {{args}} --no-manual --log rpkg

# Run R CMD check on rpkg (depends on vendor for tarball builds)
alias rcmdcheck := r-cmd-check
r-cmd-check *args: vendor
    just r-cmd-build
    R CMD check {{args}} --no-manual rpkg_*.tar.gz

# Document rpkg with devtools::document (roxygen2 → NAMESPACE + man pages)
# R wrappers are generated automatically by Makevars during R CMD INSTALL.
devtools-document: configure
    Rscript -e 'devtools::document("rpkg")'

# Load and test rpkg with devtools
devtools-test FILTER="": devtools-document
    if [ -z "{{FILTER}}" ]; then \
      Rscript -e 'testthat::set_max_fails(Inf); devtools::test("rpkg")'; \
    else \
      Rscript -e 'testthat::set_max_fails(Inf); devtools::test("rpkg", filter = "{{FILTER}}")'; \
    fi

# Load rpkg with devtools::load_all
devtools-load: devtools-document
    Rscript -e 'devtools::load_all("rpkg")'

# Install rpkg with devtools::install
devtools-install: devtools-document
    Rscript -e 'devtools::install("rpkg")'

# Check rpkg with devtools::check
devtools-check: devtools-document
    Rscript -e 'devtools::check("rpkg", error_on = "error", check_dir = "{{check_output_dir}}")'

# Install R dependencies
install-deps:
    Rscript -e 'install.packages(c("devtools","roxygen2","rcmdcheck","pkgbuild","processx","testthat","S7","knitr","rmarkdown","cli","fs","lifecycle","rlang","tomledit"), repos = "https://cloud.r-project.org")'

# ---- Rust (rpkg crate) ----

# Run cargo check on rpkg
check:
    root="$(pwd)" && tmp="$(mktemp -d)" && \
    (cd "$tmp" && CARGO_TARGET_DIR="$root/rpkg/rust-target" \
     cargo check --manifest-path="$root/rpkg/src/rust/Cargo.toml")

# Format rpkg Rust code
fmt:
    root="$(pwd)" && tmp="$(mktemp -d)" && \
    (cd "$tmp" && cargo fmt --manifest-path="$root/rpkg/src/rust/Cargo.toml")

# Run clippy on rpkg
clippy:
    root="$(pwd)" && tmp="$(mktemp -d)" && \
    (cd "$tmp" && CARGO_TARGET_DIR="$root/rpkg/rust-target" \
     cargo clippy --manifest-path="$root/rpkg/src/rust/Cargo.toml")

# ---- Cleanup ----

# Clean rpkg build artifacts
clean:
    cd rpkg && NOT_CRAN=false ./cleanup 2>/dev/null || true
    rm -rf rpkg/rust-target rpkg/src/rust/target
