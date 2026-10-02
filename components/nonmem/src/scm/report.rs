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
//!
//! On a terminal (see [`live`](super::live)) the same lines are shown with
//! less on them: a stamp only on the lines that open and close the process
//! and on each round's first line ([`report_start`]), the lines within a
//! round indented under it without the round's name repeated
//! ([`report_fit`], [`report_in`]), a mark on each fit as it ends, and no
//! line for what the fit's live line already shows ([`report_record`]). The
//! record the mirror file and a log get is the same in both cases.

use std::fmt::Display;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use console::style;
use fs_err as fs;

use super::live;

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

/// How a fit ended, shown as a mark on a terminal: `✓`, `↻`, `✗`, `!`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    None,
    Ok,
    Retry,
    Failed,
    Warn,
}

/// The tone of a table row on a terminal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Plain,
    Dim,
    Bad,
}

/// What kind of line, which decides its stamp and its shape on a terminal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A line of the process itself: stamped in the record, bare on a terminal
    Top,
    /// Opens or closes the process: the full timestamp everywhere
    Dated,
    /// A round's first line: the clock time everywhere
    Start,
    /// Within a round: stamped in the record, indented on a terminal
    In,
    /// Within a round, for the record only: a terminal shows it on the
    /// fit's live line instead
    Record,
    /// Never stamped: table rows under a decision
    Raw,
}

/// Say `line`, stamped with the clock time (or the full timestamp when it is
/// the first line, or the first of a new day). Best effort: a terminal that
/// has gone away (a closed ssh session) must not take the driver down with it.
pub fn report(line: impl Display) {
    say(
        Kind::Top,
        "",
        Mark::None,
        Tone::Plain,
        &line.to_string(),
        None,
    );
}

/// Say `line` stamped with the full timestamp whatever came before: for the
/// lines that close a process, so its record ends with a date as it began.
pub fn report_dated(line: impl Display) {
    say(
        Kind::Dated,
        "",
        Mark::None,
        Tone::Plain,
        &line.to_string(),
        None,
    );
}

/// A round's (or wave's) first line: stamped with the clock time, in the
/// record and on a terminal alike.
pub fn report_start(line: impl Display) {
    say(
        Kind::Start,
        "",
        Mark::None,
        Tone::Plain,
        &line.to_string(),
        None,
    );
}

/// A fit as it ends, within `round`: `round: line` in the record; on a
/// terminal, indented under the round's first line with `mark` in front and
/// how long it `took` after it.
pub fn report_fit(round: &str, mark: Mark, line: impl Display, took: Option<Duration>) {
    say(Kind::In, round, mark, Tone::Plain, &line.to_string(), took);
}

/// A line within the open round that the record needs but a terminal
/// already shows on the fit's live line: which slurm job a model was
/// submitted as, where a shared-node fit started.
pub fn report_record(line: impl Display) {
    say(
        Kind::Record,
        "",
        Mark::None,
        Tone::Plain,
        &line.to_string(),
        None,
    );
}

/// A line within the open round that is not a fit's end (a job adopted on
/// resume, say): as it is in the record, indented on a terminal.
pub fn report_in(line: impl Display) {
    say(
        Kind::In,
        "",
        Mark::None,
        Tone::Plain,
        &line.to_string(),
        None,
    );
}

/// Rows under a decision line: never stamped, in `tone` on a terminal.
pub fn report_table(rows: &[(Tone, String)]) {
    for (tone, row) in rows {
        say(Kind::Raw, "", Mark::None, *tone, row, None);
    }
}

fn say(kind: Kind, round: &str, mark: Mark, tone: Tone, text: &str, took: Option<Duration>) {
    let record = if round.is_empty() {
        text.to_string()
    } else {
        format!("{round}: {text}")
    };
    #[cfg(test)]
    if CAPTURED.with_borrow_mut(|c| c.as_mut().map(|c| c.push(record.clone())).is_some()) {
        return;
    }
    let now = utils::get_utc_now();
    let stamp = {
        let mut last = LAST_DATE.lock().unwrap_or_else(|e| e.into_inner());
        stamp(&now, kind == Kind::Dated, &mut last)
    };
    let plain = if kind == Kind::Raw {
        record
    } else {
        format!("[{stamp}] {record}")
    };
    // Through `println!`, which the test harness captures, so a passing
    // test's driver lines stay out of `cargo test` output.
    if live::is_live() {
        if kind != Kind::Record {
            live::println(&terminal_line(kind, mark, tone, text, took, &stamp));
        }
    } else {
        live::println(&plain);
    }
    mirror(&plain);
}

/// The line as a terminal shows it: see the module doc. `stamp` is the
/// record's stamp for the line: the full timestamp on the first line and
/// the first of a new day, which the terminal shows on any kind of line, so
/// its feed too opens with a date and marks midnight.
fn terminal_line(
    kind: Kind,
    mark: Mark,
    tone: Tone,
    text: &str,
    took: Option<Duration>,
    stamp: &str,
) -> String {
    let dated = stamp.len() > 8;
    match kind {
        Kind::Top if dated => format!("{} {text}", style(format!("[{stamp}]")).dim()),
        Kind::Top => text.to_string(),
        Kind::Dated | Kind::Start => {
            format!(
                "{} {}",
                style(format!("[{stamp}]")).dim(),
                style(text).bold()
            )
        }
        Kind::In | Kind::Record => {
            let mut line = match mark {
                Mark::None => format!("  {text}"),
                Mark::Ok => format!("  {} {text}", style("✓").green().bold()),
                Mark::Retry => format!("  {} {text}", style("↻").yellow().bold()),
                Mark::Failed => format!("  {} {text}", style("✗").red().bold()),
                Mark::Warn => format!("  {} {text}", style("!").yellow().bold()),
            };
            if let Some(took) = took {
                let took = utils::format_duration(Some(took.as_secs_f64()));
                line.push_str(&format!("   {}", style(took).dim()));
            }
            line
        }
        Kind::Raw => match tone {
            Tone::Plain => text.to_string(),
            Tone::Dim => style(text).dim().to_string(),
            Tone::Bad => style(text).red().to_string(),
        },
    }
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
        report_fit("forward_round1", Mark::Ok, "WT_CL fitted", None);
        report_record("submitted x.mod as slurm job 12");
        report_table(&[(Tone::Plain, "  WT_CL  980.000".to_string())]);
        *MIRROR.lock().unwrap() = None;

        let text = fs::read_to_string(&log).unwrap();
        assert!(text.starts_with("earlier line\n["), "{text}");
        assert!(
            text.contains("] forward_round1 complete: added WT_CL\n"),
            "{text}"
        );
        // The record carries the round's name on a fit line, and no mark
        assert!(text.contains("] forward_round1: WT_CL fitted\n"), "{text}");
        // A record-only line is stamped in the record like any other
        assert!(
            text.contains("] submitted x.mod as slurm job 12\n"),
            "{text}"
        );
        // Table rows are not stamped
        assert!(text.ends_with("\n  WT_CL  980.000\n"), "{text}");
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

    #[test]
    fn terminal_lines_stamp_only_openings_and_round_starts() {
        console::set_colors_enabled(false);
        let full = "2026-10-01T14:52:11+00:00";
        let took = Some(Duration::from_secs(440));
        let line =
            |kind, mark, text, took| terminal_line(kind, mark, Tone::Plain, text, took, "14:52:11");
        assert_eq!(
            line(Kind::Top, Mark::None, "reference complete", None),
            "reference complete"
        );
        // The first line (or the first of a day) is dated whatever it is
        assert_eq!(
            terminal_line(
                Kind::Top,
                Mark::None,
                Tone::Plain,
                "driver started",
                None,
                full
            ),
            "[2026-10-01T14:52:11+00:00] driver started"
        );
        assert_eq!(
            terminal_line(Kind::Dated, Mark::None, Tone::Plain, "starting", None, full),
            "[2026-10-01T14:52:11+00:00] starting"
        );
        assert_eq!(
            line(
                Kind::Start,
                Mark::None,
                "forward_round1: fitting 3 models",
                None
            ),
            "[14:52:11] forward_round1: fitting 3 models"
        );
        assert_eq!(
            line(Kind::In, Mark::Ok, "WT_CL fitted", took),
            "  ✓ WT_CL fitted   7m 20s"
        );
        assert_eq!(
            line(Kind::In, Mark::Retry, "WT_V FAILED", None),
            "  ↻ WT_V FAILED"
        );
        assert_eq!(
            line(Kind::In, Mark::None, "submitted WT_CL", None),
            "  submitted WT_CL"
        );
        assert_eq!(line(Kind::Raw, Mark::None, "  row", None), "  row");
    }
}
