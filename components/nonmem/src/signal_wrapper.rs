use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[cfg(unix)]
use std::path::Path;
#[cfg(unix)]
use std::process::Command;

#[cfg(unix)]
use anyhow::Result;
#[cfg(unix)]
use fs_err as fs;
#[cfg(unix)]
use jiff::Timestamp;
#[cfg(unix)]
use jiff::tz::TimeZone;
#[cfg(unix)]
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
#[cfg(unix)]
use std::sync::Arc;
#[cfg(unix)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(unix)]
use std::thread;
#[cfg(unix)]
use std::time::Duration;

pub const TERMINATION_FILENAME: &str = "pharos_terminated.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct Termination {
    pub signal: String,
    pub timestamp: String,
    pub reason: String,
}

impl Display for Termination {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            format!(
                "Terminated by {} ({}) at {}",
                self.signal, self.reason, self.timestamp
            )
            .as_str(),
        )
    }
}

/// Execute command with signal handling - writes termination file if killed
#[cfg(unix)]
pub fn execute_with_termination_handling(
    mut command: Command,
    output_dir: &Path,
) -> Result<std::process::ExitStatus> {
    // Set up safe flag-based signal handling
    let sigint_received = Arc::new(AtomicBool::new(false));
    let sigterm_received = Arc::new(AtomicBool::new(false));
    let sighup_received = Arc::new(AtomicBool::new(false));

    signal_hook::flag::register(SIGINT, Arc::clone(&sigint_received))?;
    signal_hook::flag::register(SIGTERM, Arc::clone(&sigterm_received))?;
    signal_hook::flag::register(SIGHUP, Arc::clone(&sighup_received))?;

    log::info!("Signal handlers registered for SIGINT, SIGTERM, SIGHUP");

    let mut child = command.spawn()?;

    // Simple main loop: check for signals and child completion
    loop {
        // Check for signals first (this makes Ctrl+C responsive)
        if sigint_received.load(Ordering::Relaxed) {
            log::info!("SIGINT detected, terminating process");
            let _ = child.kill(); // Kill child process first
            write_termination_file(output_dir, "SIGINT", "User interruption (Ctrl+C)")?;
            std::process::exit(130);
        }

        if sigterm_received.load(Ordering::Relaxed) {
            log::info!("SIGTERM detected, terminating process");
            let _ = child.kill(); // Kill child process first
            write_termination_file(output_dir, "SIGTERM", "Process termination")?;
            std::process::exit(143);
        }

        if sighup_received.load(Ordering::Relaxed) {
            log::info!("SIGHUP detected, terminating process");
            let _ = child.kill(); // Kill child process first
            write_termination_file(
                output_dir,
                "SIGHUP",
                "Terminal disconnected or SSH session lost",
            )?;
            std::process::exit(129);
        }

        // Check if child process finished naturally
        match child.try_wait()? {
            Some(status) => {
                // Child finished
                return Ok(status);
            }
            None => {
                // Child still running, sleep briefly before checking again
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

#[cfg(unix)]
fn write_termination_file(output_dir: &Path, signal: &str, reason: &str) -> Result<()> {
    let now_utc = Timestamp::now().to_zoned(TimeZone::UTC);
    let timestamp = now_utc.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string();

    let record = Termination {
        signal: signal.to_string(),
        timestamp,
        reason: reason.to_string(),
    };

    let termination_file = output_dir.join(TERMINATION_FILENAME);
    let content = serde_json::to_string_pretty(&record)?;
    fs::write(&termination_file, content)?;

    log::info!("Wrote termination record to {}", termination_file.display());
    Ok(())
}
