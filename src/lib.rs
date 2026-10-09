//! The pharos CLI as a library, so other crates (e.g. hyperion's bundled
//! `pharos` binary) can build the exact same CLI from a pinned tag. The
//! `pharos` binary in this crate is a thin wrapper around [`run`].

mod cli;

pub use cli::{Cli, Commands};

/// Runs the pharos CLI with the given arguments. The first item is the
/// program name, as with `std::env::args_os()`.
///
/// `--help`, `--version` and argument parse errors are handled by clap, which
/// prints and exits the process directly, as the standalone binary always has.
pub fn run<I, T>(args: I) -> anyhow::Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    cli::run(args)
}
