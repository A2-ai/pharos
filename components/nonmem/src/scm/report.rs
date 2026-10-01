//! What the driver says as an SCM process goes: each fit as it ends, each round
//! as it is decided. It goes to stdout whatever the log level, timestamped,
//! so a `scm submit` terminal and a driver job's slurm `.out` file follow the
//! process line by line — the record to check when something looks stuck.
//! A login-node driver also keeps the lines in a file (see [`mirror_to`]),
//! for when its terminal is gone.

use std::fmt::Display;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use fs_err as fs;

/// The file every reported line is also appended to, if any
static MIRROR: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Append every line reported from now on to `path` as well.
pub fn mirror_to(path: PathBuf) {
    *MIRROR.lock().unwrap_or_else(|e| e.into_inner()) = Some(path);
}

#[cfg(test)]
thread_local! {
    static CAPTURED: std::cell::RefCell<Option<Vec<String>>> = const { std::cell::RefCell::new(None) };
}

/// Say `line`. Best effort: a terminal that has gone away (a closed ssh
/// session) must not take the driver down with it.
pub fn report(line: impl Display) {
    #[cfg(test)]
    if CAPTURED.with_borrow_mut(|c| c.as_mut().map(|c| c.push(line.to_string())).is_some()) {
        return;
    }
    let line = format!("[{}] {line}", utils::get_utc_now());
    // Tests go through `println!`, which the test harness captures, so a
    // passing test's driver lines stay out of `cargo test` output.
    #[cfg(test)]
    println!("{line}");
    #[cfg(not(test))]
    {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    }
    mirror(&line);
}

/// Append `line` to the [`mirror_to`] file, if one is set.
fn mirror(line: &str) {
    if let Some(path) = MIRROR.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        let written = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut f| writeln!(f, "{line}"));
        if let Err(e) = written {
            log::warn!("could not write {}: {e}", path.display());
        }
    }
}

/// The lines [`report`] says while `f` runs on this thread, untimestamped.
#[cfg(test)]
pub(crate) fn capture<T>(f: impl FnOnce() -> T) -> (T, Vec<String>) {
    CAPTURED.with_borrow_mut(|c| *c = Some(Vec::new()));
    let value = f();
    let lines = CAPTURED.with_borrow_mut(|c| c.take()).unwrap_or_default();
    (value, lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reported_lines_are_appended_to_the_mirror_file() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("scm_driver.log");
        fs::write(&log, "earlier line\n").unwrap();
        mirror_to(log.clone());
        report("forward_round1 complete: added WT_CL");
        *MIRROR.lock().unwrap() = None;

        let text = fs::read_to_string(&log).unwrap();
        assert!(text.starts_with("earlier line\n["), "{text}");
        assert!(
            text.contains("] forward_round1 complete: added WT_CL\n"),
            "{text}"
        );
    }
}
