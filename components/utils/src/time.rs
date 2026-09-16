use jiff::Timestamp;
use jiff::tz::TimeZone;

pub fn get_utc_now() -> String {
    let now_utc = Timestamp::now().to_zoned(TimeZone::UTC);
    now_utc.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string()
}

/// Seconds between two RFC-3339 timestamps, `None` when either is missing or
/// unparseable. The timestamps are the ones [`get_utc_now`] writes.
pub fn seconds_between(start: Option<&str>, end: Option<&str>) -> Option<f64> {
    let s: Timestamp = start?.parse().ok()?;
    let e: Timestamp = end?.parse().ok()?;
    Some(e.duration_since(s).as_secs_f64())
}

/// A duration for a report column: `1h 02m`, `2m 03s`, `1.5s`, or `-` when
/// there is none.
pub fn format_duration(seconds: Option<f64>) -> String {
    let Some(s) = seconds else {
        return "-".to_string();
    };
    let total = s.round() as u64;
    let (h, m, sec) = (total / 3600, (total % 3600) / 60, total % 60);
    if h > 0 {
        format!("{h}h {m:02}m")
    } else if m > 0 {
        format!("{m}m {sec:02}s")
    } else {
        format!("{s:.1}s")
    }
}

/// The wall-clock time out of a timestamp: `2026-09-08T16:02:15+00:00` ->
/// `16:02:15`, `-` when there is none.
pub fn clock(ts: Option<&str>) -> String {
    ts.and_then(|t| t.get(11..19))
        .map(str::to_string)
        .unwrap_or_else(|| "-".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_and_clocks_format() {
        assert_eq!(format_duration(None), "-");
        assert_eq!(format_duration(Some(1.53)), "1.5s");
        assert_eq!(format_duration(Some(123.0)), "2m 03s");
        assert_eq!(format_duration(Some(3723.0)), "1h 02m");
        assert_eq!(clock(Some("2026-09-08T16:02:15+00:00")), "16:02:15");
        assert_eq!(clock(None), "-");
        assert_eq!(clock(Some("nonsense")), "-");
    }

    #[test]
    fn spans_measure_in_seconds() {
        let span = seconds_between(
            Some("2026-09-08T16:00:00+00:00"),
            Some("2026-09-08T16:01:30+00:00"),
        );
        assert_eq!(span, Some(90.0));
        assert_eq!(
            seconds_between(None, Some("2026-09-08T16:01:30+00:00")),
            None
        );
        assert_eq!(seconds_between(Some("nope"), Some("nope")), None);
    }
}
