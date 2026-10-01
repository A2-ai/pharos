//! Stopping an SCM process on request (Ctrl-C, `scancel`, a closed terminal).
//!
//! The driver catches SIGINT/SIGTERM/SIGHUP and only raises a flag; executors
//! check it while they wait on fits and stop with [`Interrupted`], and
//! [`run_scm`](super::run_scm) records the process as paused rather than
//! failed: nothing went wrong with the covariate selection, and submitting the
//! plan again resumes it. A second signal while the first is being handled
//! exits at once.

use std::fmt;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;

static FLAG: LazyLock<Arc<AtomicBool>> = LazyLock::new(|| Arc::new(AtomicBool::new(false)));

/// The note a process stopped this way carries.
pub const INTERRUPTED_NOTE: &str = "interrupted; submit the plan again to resume";

/// An SCM process stopped on request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interrupted;

impl fmt::Display for Interrupted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("interrupted")
    }
}

impl std::error::Error for Interrupted {}

/// Catch SIGINT, SIGTERM and SIGHUP for the rest of the process: the first
/// raises the flag, a second exits.
pub fn install_interrupt_handler() -> Result<()> {
    #[cfg(unix)]
    {
        use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
        for sig in [SIGINT, SIGTERM, SIGHUP] {
            // Registered first, so it sees the flag as it was before this signal.
            signal_hook::flag::register_conditional_shutdown(sig, 130, Arc::clone(&FLAG))?;
            signal_hook::flag::register(sig, Arc::clone(&FLAG))?;
        }
    }
    Ok(())
}

/// Whether a stop has been requested.
pub fn interrupted() -> bool {
    FLAG.load(Ordering::Relaxed)
}

/// Whether `error` is (or wraps) an [`Interrupted`].
pub fn is_interrupted(error: &anyhow::Error) -> bool {
    error.chain().any(|e| e.is::<Interrupted>())
}
