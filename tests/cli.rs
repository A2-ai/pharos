//! Pins the library entry point (`pharos::run`) and the thin `pharos` binary
//! wrapper around it. Downstream crates (hyperion) build their own `pharos`
//! binary from `pharos::run`, so the binary's contract lives here.
//!
//! Only one test in this file may call `pharos::run` in-process: it initialises
//! the global env_logger, which can only happen once per process. Everything
//! else goes through the built binary.

use std::path::PathBuf;
use std::process::Command;

use clap::Parser;

const BIN: &str = env!("CARGO_BIN_EXE_pharos");

/// A config file path that does not exist, so commands that need a config
/// fail deterministically before touching anything.
fn missing_config() -> PathBuf {
    std::env::temp_dir()
        .join("pharos-cli-test-no-such-dir")
        .join("pharos.toml")
}

#[test]
fn cli_types_are_public_and_parse() {
    let cli = pharos::Cli::try_parse_from(["pharos", "nonmem", "sitrep"]).unwrap();
    assert!(matches!(cli.command, pharos::Commands::Nonmem { .. }));

    let err = pharos::Cli::try_parse_from(["pharos", "--version"])
        .err()
        .expect("--version is handled by clap");
    assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
}

#[test]
fn run_returns_err_instead_of_exiting() {
    let config = missing_config();
    let res = pharos::run([
        "pharos".as_ref(),
        "--config-file".as_ref(),
        config.as_os_str(),
        "nonmem".as_ref(),
        "summary".as_ref(),
        "x".as_ref(),
    ] as [&std::ffi::OsStr; 6]);
    let err = res.expect_err("missing config must be an error, not an exit");
    assert!(
        err.to_string().contains("pharos config file not found"),
        "unexpected error: {err:?}"
    );
}

#[test]
fn binary_version_matches_package_version() {
    let out = Command::new(BIN).arg("--version").output().unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("pharos {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn binary_help_exits_zero() {
    let out = Command::new(BIN).arg("--help").output().unwrap();
    assert!(out.status.success());
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .contains("Usage: pharos")
    );
}

#[test]
fn binary_error_exits_one_with_message_on_stderr() {
    let out = Command::new(BIN)
        .arg("--config-file")
        .arg(missing_config())
        .args(["nonmem", "summary", "x"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.starts_with("pharos config file not found"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn binary_usage_error_exits_two() {
    let out = Command::new(BIN).arg("--bogus").output().unwrap();
    assert_eq!(out.status.code(), Some(2));
}
