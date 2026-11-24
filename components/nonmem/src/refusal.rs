//! A request pharos turns down, and what the user can do about it, kept
//! apart so each front end words the second part its own way.

use std::fmt;
use std::path::PathBuf;

/// One way forward from a refusal, worded by each front end: the CLI names
/// its commands and flags, another front end its own calls. Paths are as
/// pharos knows them; the front end shows them.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// Run the same command or call again, replacing what is there
    Overwrite,
    /// Name the project root the runs were recorded under
    GiveBasePath,
    /// Run under the project at `root`: the pharos.toml in use is another
    /// project's
    UseProject { root: PathBuf },
    /// Follow the SCM process in `out_dir`, whose config the user submitted
    /// is `config`
    FollowScm { out_dir: PathBuf, config: PathBuf },
    /// Stop the SCM process in `out_dir`, whose config the user submitted is
    /// `config`, before trying again
    StopScm { out_dir: PathBuf, config: PathBuf },
}

/// A request pharos turns down: what is wrong, and the ways forward (any
/// one of them will do). It is the error itself, `bail!(Refusal { .. })`,
/// with any cause worded into `problem` (as context it would hide the cause
/// under it); a front end finds it with `downcast_ref::<Refusal>()` and
/// words `steps`.
///
/// Its `Display` is `problem` alone: a record, a log, or any text made from
/// the error says what is wrong and nothing a front end words differently.
#[derive(Debug)]
pub struct Refusal {
    /// What is wrong, in words every front end shares: no command, flag or
    /// call. A condition every way forward needs ("once it has stopped,
    /// delete ...") ends it.
    pub problem: String,
    pub steps: Vec<Step>,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.problem)
    }
}

impl std::error::Error for Refusal {}
