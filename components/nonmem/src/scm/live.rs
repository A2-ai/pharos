//! The live view of an SCM process: what a terminal shows under the record
//! lines while fits run. A bar for the open round with a line per fit
//! (where it runs, how long, and the latest iteration and OFV read off its
//! `.ext` file), and a bar for the whole process.
//!
//! It exists only when [`enable`] found a terminal on both stdout and
//! stderr. Otherwise every call here is a no-op and the driver's output is
//! the plain record alone, which is what a slurm `.out` file or a redirected
//! `scm submit` gets. The bars are drawn on stderr; the record stays on
//! stdout, written through [`println`] so a line never lands in the middle
//! of a bar.
//!
//! The driver tells it about rounds and fits ([`round_begin`],
//! [`fit_done`], [`round_end`]); the executors tell it where each fit is
//! ([`fit_queued`], [`fit_running`]) and call [`tick`] while they wait.
//! Whether a fit has started is read off disk rather than off the
//! scheduler: `pharos nonmem run` writes `pharos_start.json` into the run
//! directory as it begins, and a queued fit is shown running from the
//! moment that file is there (the file's start time dates it).

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use console::{Term, style};
use fs_err as fs;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use serde::Deserialize;

use crate::run::metadata::RUN_START_FILENAME;

static LIVE: Mutex<Option<Live>> = Mutex::new(None);

/// How often each fit's run directory is read: a queued fit's for its start
/// file, a running fit's `.ext` for its latest iteration
const DISK_READ_INTERVAL: Duration = Duration::from_secs(3);
/// How often the elapsed times on screen are refreshed
const REFRESH_INTERVAL: Duration = Duration::from_secs(1);

/// Turn the live view on, if stdout and stderr are both terminals. Returns
/// whether it is on.
pub fn enable() -> bool {
    if !(Term::stdout().is_term() && Term::stderr().is_term()) {
        return false;
    }
    let mp = MultiProgress::with_draw_target(ProgressDrawTarget::stderr());
    let process = mp.add(ProgressBar::new(1));
    process.set_style(bar_style());
    process.set_prefix("process");
    *lock() = Some(Live {
        mp,
        process,
        started: Instant::now(),
        fits_done: 0,
        round: None,
        round_label: String::new(),
        last_refresh: Instant::now(),
    });
    true
}

pub fn is_live() -> bool {
    lock().is_some()
}

fn lock() -> std::sync::MutexGuard<'static, Option<Live>> {
    LIVE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Print a record line on stdout, above the bars when they are up.
pub(crate) fn println(line: &str) {
    match lock().as_ref() {
        Some(live) => live.mp.suspend(|| {
            println!("{line}");
        }),
        None => println!("{line}"),
    }
}

/// Print a line only the live view shows: nothing in a log, nothing in
/// the mirror file.
pub fn note(line: impl AsRef<str>) {
    if let Some(live) = lock().as_ref() {
        let line = line.as_ref();
        live.mp.suspend(|| println!("{line}"));
    }
}

/// How long the process has been running in this driver, when live.
pub fn process_elapsed() -> Option<Duration> {
    lock().as_ref().map(|l| l.started.elapsed())
}

/// Size the process bar: `total` fits the process may need, `done` of them
/// already fitted (on resume).
pub fn process_begin(total: usize, done: usize) {
    if let Some(live) = lock().as_mut() {
        live.process.set_length(total.max(1) as u64);
        live.process.set_position(done as u64);
        live.fits_done = done;
        live.refresh_process();
    }
}

/// One fit the driver is about to dispatch
pub struct FitEntry {
    pub model: PathBuf,
    /// The candidate's name, as the line shows it
    pub name: String,
    /// The fit's run directory, where its start file lands
    pub run_dir: Option<PathBuf>,
    /// The `.ext` file the fit writes, for its latest iteration
    pub ext: Option<PathBuf>,
}

/// A wave of fits starts in `round`: a bar for the wave and a line per fit.
pub fn round_begin(round: &str, label: &str, fits: Vec<FitEntry>) {
    let mut guard = lock();
    let Some(live) = guard.as_mut() else { return };
    live.close_round();
    live.round_label = round.to_string();
    let bar = live
        .mp
        .insert_before(&live.process, ProgressBar::new(fits.len() as u64));
    bar.set_style(bar_style());
    bar.set_prefix(label.to_string());
    let name_width = fits.iter().map(|f| f.name.len()).max().unwrap_or(8).max(8);
    let fits = fits
        .into_iter()
        .map(|f| {
            let line = live
                .mp
                .insert_before(&live.process, ProgressBar::new_spinner());
            line.set_style(ProgressStyle::with_template("    {msg}").expect("a fixed template"));
            FitView {
                model: f.model,
                name: f.name,
                run_dir: f.run_dir,
                ext: f.ext,
                line,
                state: FitState::Queued,
                place: String::new(),
                since: Instant::now(),
                disk_read: None,
                latest: None,
            }
        })
        .collect();
    live.round = Some(RoundView {
        bar,
        started: Instant::now(),
        fits,
        name_width,
    });
    live.refresh(true);
}

/// The executor has handed `model` to the scheduler as `place` (`job 1234`).
pub fn fit_queued(model: &Path, place: impl Into<String>) {
    if let Some(live) = lock().as_mut()
        && let Some(fit) = live.fit_mut(model)
    {
        fit.place = place.into();
        fit.state = FitState::Queued;
        fit.since = Instant::now();
        live.refresh(true);
    }
}

/// `model` is now running (`place` names where, when it changed).
pub fn fit_running(model: &Path, place: Option<String>) {
    if let Some(live) = lock().as_mut()
        && let Some(fit) = live.fit_mut(model)
        && fit.state != FitState::Running
    {
        if let Some(place) = place {
            fit.place = place;
        }
        fit.state = FitState::Running;
        fit.since = Instant::now();
        live.refresh(true);
    }
}

/// `model`'s run ended: its line goes, the bars advance. Returns how long
/// it ran (or waited and ran) for the record line, when live.
pub fn fit_done(model: &Path) -> Option<Duration> {
    let mut guard = lock();
    let live = guard.as_mut()?;
    let round = live.round.as_mut()?;
    let pos = round.fits.iter().position(|f| f.model == model)?;
    let fit = round.fits.remove(pos);
    fit.line.finish_and_clear();
    live.mp.remove(&fit.line);
    round.bar.inc(1);
    live.fits_done += 1;
    live.process.set_position(live.fits_done as u64);
    live.refresh(true);
    Some(fit.since.elapsed())
}

/// The wave is over: its bar and lines go.
pub fn round_end() {
    if let Some(live) = lock().as_mut() {
        live.close_round();
        live.refresh_process();
    }
}

/// Called by executors as they wait: refreshes elapsed times and, every
/// [`DISK_READ_INTERVAL`], reads the fits' run directories: a queued fit
/// whose start file is there is shown running, a running fit's latest
/// iteration is read off its `.ext`.
pub fn tick() {
    if let Some(live) = lock().as_mut() {
        live.refresh(false);
    }
}

/// The process is over (or stopped): everything is cleared.
pub fn finish() {
    if let Some(live) = lock().as_mut() {
        live.close_round();
        live.process.finish_and_clear();
        live.mp.remove(&live.process);
    }
}

struct Live {
    mp: MultiProgress,
    process: ProgressBar,
    started: Instant,
    fits_done: usize,
    round: Option<RoundView>,
    round_label: String,
    last_refresh: Instant,
}

struct RoundView {
    bar: ProgressBar,
    started: Instant,
    fits: Vec<FitView>,
    name_width: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FitState {
    Queued,
    Running,
}

struct FitView {
    model: PathBuf,
    name: String,
    run_dir: Option<PathBuf>,
    ext: Option<PathBuf>,
    line: ProgressBar,
    state: FitState,
    /// Where the fit is: `job 1234`, `this node`, ...
    place: String,
    /// When it was queued, or started running
    since: Instant,
    /// When its run directory was last read
    disk_read: Option<Instant>,
    latest: Option<Iteration>,
}

impl Live {
    fn fit_mut(&mut self, model: &Path) -> Option<&mut FitView> {
        self.round
            .as_mut()?
            .fits
            .iter_mut()
            .find(|f| f.model == model)
    }

    fn close_round(&mut self) {
        if let Some(round) = self.round.take() {
            for fit in round.fits {
                fit.line.finish_and_clear();
                self.mp.remove(&fit.line);
            }
            round.bar.finish_and_clear();
            self.mp.remove(&round.bar);
        }
    }

    fn refresh_process(&self) {
        let total = self.process.length().unwrap_or(0);
        let mut msg = format!("{}/{total} fits", self.fits_done);
        if !self.round_label.is_empty() {
            msg.push_str(&format!(" · {}", self.round_label));
        }
        msg.push_str(&format!(" · {}", elapsed(self.started)));
        self.process.set_message(msg);
    }

    /// Redraw the messages; `force` skips the refresh interval.
    fn refresh(&mut self, force: bool) {
        if !force && self.last_refresh.elapsed() < REFRESH_INTERVAL {
            return;
        }
        self.last_refresh = Instant::now();
        self.refresh_process();
        let Some(round) = self.round.as_mut() else {
            return;
        };
        let (mut running, mut queued) = (0, 0);
        for fit in &mut round.fits {
            if fit
                .disk_read
                .is_none_or(|t| t.elapsed() >= DISK_READ_INTERVAL)
            {
                fit.disk_read = Some(Instant::now());
                fit.read_disk();
            }
            match fit.state {
                FitState::Running => running += 1,
                FitState::Queued => queued += 1,
            }
            fit.line.set_message(fit.render(round.name_width));
        }
        let done = round.bar.position();
        let total = round.bar.length().unwrap_or(0);
        let mut parts = vec![format!("{done}/{total} done")];
        if running > 0 {
            parts.push(format!("{running} running"));
        }
        if queued > 0 {
            parts.push(format!("{queued} queued"));
        }
        parts.push(elapsed(round.started));
        round.bar.set_message(parts.join(" · "));
    }
}

impl FitView {
    /// Read what the run directory says: a queued fit whose start file is
    /// there has started (as of the file's start time), and a running
    /// fit's `.ext` has its latest iteration.
    fn read_disk(&mut self) {
        match self.state {
            FitState::Queued => {
                if let Some(run_dir) = &self.run_dir
                    && let Some(started) = started_at(run_dir)
                {
                    self.state = FitState::Running;
                    self.since = started;
                }
            }
            FitState::Running => {
                if let Some(ext) = &self.ext
                    && let Some(latest) = latest_iteration(ext)
                {
                    self.latest = Some(latest);
                }
            }
        }
    }

    fn render(&self, name_width: usize) -> String {
        let (mark, doing) = match self.state {
            FitState::Queued => (style("○").dim(), format!("queued {}", elapsed(self.since))),
            FitState::Running => (
                style("●").cyan(),
                format!("running {}", elapsed(self.since)),
            ),
        };
        let mark = mark.to_string();
        fit_line(
            &mark,
            &self.name,
            name_width,
            &self.place,
            &doing,
            self.latest,
        )
    }
}

/// One fit's line under a round, as the live view and `scm status` draw it:
/// `● WT_CL    job 1234   running 4m 12s   iter 8    OFV 1050.000`.
pub(crate) fn fit_line(
    mark: &str,
    name: &str,
    name_width: usize,
    place: &str,
    doing: &str,
    latest: Option<Iteration>,
) -> String {
    let mut line = format!("{mark} {name:<name_width$}  {place:<10} {doing:<15}");
    if let Some(it) = latest {
        line.push_str(&format!("  iter {:<4} OFV {:.3}", it.iteration, it.ofv));
    }
    line.trim_end().to_string()
}

/// When the fit in `run_dir` started, if its start file is there: the
/// file's own start time, so the clock on the line is the fit's age, not
/// the time since the file was noticed. Now, when the time is unreadable.
fn started_at(run_dir: &Path) -> Option<Instant> {
    #[derive(Deserialize)]
    struct Started {
        start: String,
    }
    let text = fs::read_to_string(run_dir.join(RUN_START_FILENAME)).ok()?;
    let now = Instant::now();
    let Ok(Started { start }) = serde_json::from_str::<Started>(&text) else {
        return Some(now);
    };
    let age = utils::seconds_between(Some(&start), Some(&utils::get_utc_now()))
        .filter(|s| *s > 0.0)
        .map(Duration::from_secs_f64);
    Some(age.and_then(|a| now.checked_sub(a)).unwrap_or(now))
}

/// The latest row of a fit's `.ext` file
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Iteration {
    pub(crate) iteration: u64,
    pub(crate) ofv: f64,
}

/// The last iteration row (a non-negative iteration number, its OBJ in the
/// last column) of the `.ext` file at `path`, if it has one yet. Reads the
/// whole file: `.ext` files are small, and this runs every
/// [`DISK_READ_INTERVAL`] per running fit.
pub(crate) fn latest_iteration(path: &Path) -> Option<Iteration> {
    let text = fs::read_to_string(path).ok()?;
    parse_latest_iteration(&text)
}

fn parse_latest_iteration(text: &str) -> Option<Iteration> {
    text.lines().rev().find_map(|line| {
        let mut fields = line.split_whitespace();
        let iteration: u64 = fields.next()?.parse().ok()?;
        let ofv: f64 = fields.last()?.parse().ok()?;
        Some(Iteration { iteration, ofv })
    })
}

fn elapsed(since: Instant) -> String {
    utils::format_duration(Some(since.elapsed().as_secs_f64()))
}

fn bar_style() -> ProgressStyle {
    ProgressStyle::with_template("  {prefix:<16.bold} {bar:28.cyan/238} {msg}")
        .expect("a fixed template")
        .progress_chars("━╸ ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_latest_iteration_is_the_last_non_negative_row() {
        let ext = "\
TABLE NO.     1: First Order Conditional Estimation
 ITERATION    THETA1       THETA2       OBJ
            0  1.24000E+00  4.08600E+01   -103.29831730419750
            5  1.24987E+00  4.07145E+01   -103.43319829850387
           10  1.24679E+00  4.08038E+01   -103.46776642487734
  -1000000000  1.24679E+00  4.08038E+01   -103.46776642487734
  -1000000006  0.00000E+00  0.00000E+00
";
        assert_eq!(
            parse_latest_iteration(ext),
            Some(Iteration {
                iteration: 10,
                ofv: -103.46776642487734
            })
        );
        assert_eq!(
            parse_latest_iteration("TABLE NO. 1\n ITERATION OBJ\n"),
            None
        );
        assert_eq!(parse_latest_iteration(""), None);
    }

    #[test]
    fn a_start_file_dates_the_fit_from_its_own_start_time() {
        let dir = tempfile::tempdir().unwrap();
        assert!(started_at(dir.path()).is_none());

        let then = jiff::Timestamp::now() - jiff::SignedDuration::from_secs(90);
        let then = then
            .to_zoned(jiff::tz::TimeZone::UTC)
            .strftime("%Y-%m-%dT%H:%M:%S%:z")
            .to_string();
        fs::write(
            dir.path().join(RUN_START_FILENAME),
            format!(r#"{{"start": "{then}", "model_name": "run1"}}"#),
        )
        .unwrap();
        let age = started_at(dir.path()).unwrap().elapsed().as_secs_f64();
        assert!((85.0..95.0).contains(&age), "age {age}");

        // An unreadable start file still means the fit has started: now.
        fs::write(dir.path().join(RUN_START_FILENAME), "not json").unwrap();
        assert!(started_at(dir.path()).unwrap().elapsed().as_secs() < 1);
    }

    #[test]
    fn nothing_happens_without_a_terminal() {
        // The test harness is not a terminal on both streams, so the live
        // view stays off and every call is a no-op.
        assert!(!is_live());
        process_begin(10, 2);
        round_begin("forward_round1", "forward round 1", vec![]);
        fit_queued(Path::new("x.mod"), "job 1");
        fit_running(Path::new("x.mod"), None);
        assert_eq!(fit_done(Path::new("x.mod")), None);
        assert_eq!(process_elapsed(), None);
        tick();
        round_end();
        finish();
    }
}
