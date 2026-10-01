//! What the driver says as an SCM process goes: each fit as it ends, each round
//! as it is decided. It goes to stdout whatever the log level, timestamped,
//! so a `scm submit` terminal and a driver job's slurm `.out` file follow the
//! process line by line — the record to check when something looks stuck.
//! A login-node driver also keeps the lines in a file (see [`mirror_to`]),
//! for when its terminal is gone.
//!
//! Most lines carry only the UTC clock time (`[14:52:11]`), which keeps the
//! feed narrow. The full timestamp (`[2026-10-01T14:52:11+00:00]`) goes on
//! the first line a driver says, on the first line of each new UTC day, and
//! on the lines a caller marks with [`report_dated`] — the ones that close a
//! process — so the date is never far from any line and the file says what
//! zone it keeps.

use std::fmt::Display;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use fs_err as fs;

/// The file every reported line is also appended to, if any
static MIRROR: Mutex<Option<PathBuf>> = Mutex::new(None);

/// The UTC date (`YYYY-MM-DD`) of the last line said, if any: a line on
/// another date gets the full timestamp so the day change is on record.
static LAST_DATE: Mutex<Option<String>> = Mutex::new(None);

/// Append every line reported from now on to `path` as well.
pub fn mirror_to(path: PathBuf) {
    *MIRROR.lock().unwrap_or_else(|e| e.into_inner()) = Some(path);
}

#[cfg(test)]
thread_local! {
    static CAPTURED: std::cell::RefCell<Option<Vec<String>>> = const { std::cell::RefCell::new(None) };
}

/// Say `line`, stamped with the clock time (or the full timestamp when it is
/// the first line, or the first of a new day). Best effort: a terminal that
/// has gone away (a closed ssh session) must not take the driver down with it.
pub fn report(line: impl Display) {
    say(line, false);
}

/// Say `line` stamped with the full timestamp whatever came before: for the
/// lines that close a process, so its record ends with a date as it began.
pub fn report_dated(line: impl Display) {
    say(line, true);
}

fn say(line: impl Display, dated: bool) {
    #[cfg(test)]
    if CAPTURED.with_borrow_mut(|c| c.as_mut().map(|c| c.push(line.to_string())).is_some()) {
        return;
    }
    let now = utils::get_utc_now();
    let stamp = {
        let mut last = LAST_DATE.lock().unwrap_or_else(|e| e.into_inner());
        stamp(&now, dated, &mut last)
    };
    let line = format!("[{stamp}] {line}");
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

/// The stamp for a line said at `now` (an RFC-3339 timestamp as
/// [`utils::get_utc_now`] writes it): the whole of `now` when `dated`, when
/// nothing has been said yet, or when `now` falls on a different date from
/// the last line; otherwise its clock time alone. Records `now`'s date as
/// the last one said.
fn stamp(now: &str, dated: bool, last_date: &mut Option<String>) -> String {
    let (Some(date), Some(clock)) = (now.get(..10), now.get(11..19)) else {
        return now.to_string();
    };
    let new_day = last_date.as_deref() != Some(date);
    if new_day {
        *last_date = Some(date.to_string());
    }
    if dated || new_day {
        now.to_string()
    } else {
        clock.to_string()
    }
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

    #[test]
    fn the_first_line_and_each_new_day_carry_the_date() {
        let mut last = None;
        let full = "2026-10-01T14:52:11+00:00";
        assert_eq!(stamp(full, false, &mut last), full);
        assert_eq!(
            stamp("2026-10-01T15:06:02+00:00", false, &mut last),
            "15:06:02"
        );
        // Midnight passes: the first line of the new day is dated.
        let next_day = "2026-10-02T00:04:17+00:00";
        assert_eq!(stamp(next_day, false, &mut last), next_day);
        assert_eq!(
            stamp("2026-10-02T00:04:51+00:00", false, &mut last),
            "00:04:51"
        );
        // A closing line is dated whatever came before.
        let close = "2026-10-02T00:19:33+00:00";
        assert_eq!(stamp(close, true, &mut last), close);
        assert_eq!(
            stamp("2026-10-02T00:19:34+00:00", false, &mut last),
            "00:19:34"
        );
    }

    #[test]
    fn a_timestamp_of_another_shape_is_kept_whole() {
        let mut last = None;
        assert_eq!(stamp("nonsense", false, &mut last), "nonsense");
    }
}
