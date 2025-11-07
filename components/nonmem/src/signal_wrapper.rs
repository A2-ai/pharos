use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[cfg(unix)]
use std::io::Read;
#[cfg(unix)]
use std::path::Path;
#[cfg(unix)]
use std::process::Command;
#[cfg(unix)]
use std::sync::Arc;
#[cfg(unix)]
use std::sync::atomic::{AtomicBool, Ordering};

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
    recv: impl Read + Send + 'static,
    output_dir: &Path,
) -> Result<(std::process::ExitStatus, Vec<u8>)> {
    use std::sync::Mutex;
    use std::thread;

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

    // Set up concurrent output reading
    let output_buffer = Arc::new(Mutex::new(Vec::new()));
    let output_buffer_clone = Arc::clone(&output_buffer);

    // Spawn background thread to read output
    let reader_thread = thread::spawn(move || {
        let mut recv = recv;
        let mut buffer = Vec::new();
        match recv.read_to_end(&mut buffer) {
            Ok(_) => {
                if let Ok(mut output) = output_buffer_clone.lock() {
                    *output = buffer;
                }
                log::debug!("Output reading completed successfully");
            }
            Err(e) => {
                log::warn!("Error reading from pipe: {}", e);
            }
        }
    });

    // Main loop: check for signals and child completion
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
                // Child finished, wait for output reading to complete
                log::debug!("Child process finished, waiting for output reading to complete");
                let _ = reader_thread.join();

                // Extract the collected output
                let output = match output_buffer.lock() {
                    Ok(buffer) => buffer.clone(),
                    Err(_) => {
                        log::warn!("Failed to acquire output buffer lock, returning empty output");
                        Vec::new()
                    }
                };

                return Ok((status, output));
            }
            None => {
                // Child still running, sleep briefly before checking again
                std::thread::sleep(std::time::Duration::from_millis(100));
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
