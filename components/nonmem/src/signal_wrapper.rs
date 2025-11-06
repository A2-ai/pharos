use std::fmt::{Display, Formatter};
use std::io::Read;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use fs_err as fs;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde::{Deserialize, Serialize};
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};

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
pub fn execute_with_termination_handling(
    mut command: Command,
    mut recv: impl Read,
    output_dir: &Path,
) -> Result<(std::process::ExitStatus, Vec<u8>)> {
    // Set up signal handling
    let sigint_received = Arc::new(AtomicBool::new(false));
    let sigterm_received = Arc::new(AtomicBool::new(false));
    let sighup_received = Arc::new(AtomicBool::new(false));

    signal_hook::flag::register(SIGINT, Arc::clone(&sigint_received))?;
    signal_hook::flag::register(SIGTERM, Arc::clone(&sigterm_received))?;
    signal_hook::flag::register(SIGHUP, Arc::clone(&sighup_received))?;

    log::debug!("Signal handlers registered for SIGINT, SIGTERM, SIGHUP");

    // Spawn child process
    let mut child = command.spawn()?;

    // Simple loop: check for signals and child completion
    loop {
        if sigint_received.load(Ordering::Relaxed) {
            log::info!("SIGINT detected, terminating process");
            write_termination_file(output_dir, "SIGINT", "User interruption (Ctrl+C)")?;
            let _ = child.kill(); // Kill child process
            std::process::exit(130); // Exit immediately with SIGINT exit code
        }

        if sigterm_received.load(Ordering::Relaxed) {
            log::info!("SIGTERM detected, terminating process");
            write_termination_file(output_dir, "SIGTERM", "Process termination")?;
            let _ = child.kill(); // Kill child process
            std::process::exit(143); // Exit immediately with SIGTERM exit code
        }

        if sighup_received.load(Ordering::Relaxed) {
            log::info!("SIGHUP detected, terminating process");
            write_termination_file(
                output_dir,
                "SIGHUP",
                "Terminal disconnected or SSH session lost",
            )?;
            let _ = child.kill(); // Kill child process
            std::process::exit(129); // Exit immediately with SIGHUP exit code
        }

        // Check if child process finished naturally
        match child.try_wait()? {
            Some(status) => {
                // Child finished normally, collect output and return
                let mut output = Vec::new();
                recv.read_to_end(&mut output)?;
                return Ok((status, output));
            }
            None => {
                // Child still running, sleep briefly before checking again
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    }
}

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
