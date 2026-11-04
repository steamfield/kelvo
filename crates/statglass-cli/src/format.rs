//! Text formatting shared by the renderers: times, values with units, and simple
//! tables. Presentation only; every value's meaning comes from the answer.

/// Formats the answer's RFC 3339 times relative to today in the local zone. The app
/// writes times in this Mac's offset, so the date and clock fields can be read as they are.
#[derive(Clone, Debug)]
pub struct Clock {
    /// `YYYY-MM-DD` today, local.
    today: String,
    /// Date every time, today's included: set for answers whose range reaches past
    /// today, where a bare `10:29` next to `Nov 2 07:02` reads as the earlier day.
    dated: bool,
}

impl Clock {
    pub fn now() -> Self {
        let now_ms = now_ms();
        let today = statglass_query::time::rfc3339_local(now_ms);
        Self::for_today(today.get(..10).unwrap_or_default())
    }

    pub fn for_today(today: &str) -> Self {
        Self {
            today: today.to_owned(),
            dated: false,
        }
    }

    /// This clock for an answer covering `from` to `to` (RFC 3339): it dates every time
    /// when the range does not lie wholly within today.
    pub fn for_range(&self, from: &str, to: &str) -> Self {
        let today = Some(self.today.as_str());
        Self {
            today: self.today.clone(),
            dated: from.get(..10) != today || to.get(..10) != today,
        }
    }

    /// `15:12` today, `Nov 2 15:12` on another day this year, `Nov 2 2025 15:12` in
    /// another year. A string that isn't RFC 3339 is shown as it is.
    pub fn short(&self, rfc3339: &str) -> String {
        self.render(rfc3339, false)
    }

    /// As [`Clock::short`] with seconds, for sub-minute buckets.
    pub fn short_secs(&self, rfc3339: &str) -> String {
        self.render(rfc3339, true)
    }

    /// A `YYYY-MM-DD` day: `Nov 2`, with the year when it is not this year. Always dated,
    /// today included.
    pub fn day(&self, date: &str) -> String {
        let (Some(year), Some(month), Some(day)) = (date.get(..4), date.get(5..7), date.get(8..10))
        else {
            return date.to_owned();
        };
        let month = month_name(month).unwrap_or(month);
        let day = day.trim_start_matches('0');
        if self.today.get(..4) == Some(year) {
            format!("{month} {day}")
        } else {
            format!("{month} {day} {year}")
        }
    }

    fn render(&self, t: &str, secs: bool) -> String {
        let (Some(date), Some(clock)) = (t.get(..10), t.get(11..if secs { 19 } else { 16 })) else {
            return t.to_owned();
        };
        if date == self.today && !self.dated {
            return clock.to_owned();
        }
        let (Some(year), Some(month), Some(day)) = (date.get(..4), date.get(5..7), date.get(8..10))
        else {
            return t.to_owned();
        };
        let month = month_name(month).unwrap_or(month);
        let day = day.trim_start_matches('0');
        if self.today.get(..4) == Some(year) {
            format!("{month} {day} {clock}")
        } else {
            format!("{month} {day} {year} {clock}")
        }
    }
}

fn month_name(mm: &str) -> Option<&'static str> {
    const NAMES: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let i: usize = mm.parse().ok()?;
    NAMES.get(i.checked_sub(1)?).copied()
}

/// Milliseconds since the epoch, now.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// A value with its catalog unit (`percent`, `bytes_per_sec`, ...). Unknown units print
/// the number and the unit text.
pub fn value(v: f64, unit: &str) -> String {
    match unit {
        "percent" => format!("{v:.1}%"),
        "bytes" => scaled(v, 1000.0, &["B", "KB", "MB", "GB", "TB", "PB"]),
        "bytes_per_sec" => scaled(v, 1000.0, &["B/s", "KB/s", "MB/s", "GB/s", "TB/s"]),
        "bits_per_sec" => scaled(v, 1000.0, &["b/s", "Kb/s", "Mb/s", "Gb/s", "Tb/s"]),
        "hz" => scaled(v, 1000.0, &["Hz", "kHz", "MHz", "GHz"]),
        "watts" => format!("{} W", number(v)),
        "watt_hours" => format!("{} Wh", number(v)),
        "celsius" => format!("{v:.1} °C"),
        "rpm" => format!("{v:.0} rpm"),
        "minutes" => format!("{v:.0} min"),
        "pages_per_sec" => format!("{} pages/s", number(v)),
        "operations_per_sec" => format!("{} ops/s", number(v)),
        "milliseconds_per_sec" => format!("{} ms/s", number(v)),
        "count" | "enum" | "bool" | "" => number(v),
        other => format!("{} {other}", number(v)),
    }
}

/// Scales up the ladder until at most three integer digits show. Base units take no
/// decimals; scaled ones take one below 100.
fn scaled(v: f64, step: f64, units: &[&str]) -> String {
    let mut scaled = v;
    let mut i = 0;
    while scaled.abs() >= 999.5 && i + 1 < units.len() {
        scaled /= step;
        i += 1;
    }
    let unit = units.get(i).copied().unwrap_or_default();
    if i == 0 || scaled.abs() >= 100.0 {
        format!("{scaled:.0} {unit}")
    } else {
        format!("{scaled:.1} {unit}")
    }
}

/// A plain number: whole numbers without decimals, otherwise up to two significant
/// decimals with trailing zeros trimmed.
pub fn number(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        return format!("{v:.0}");
    }
    let decimals = if v.abs() >= 100.0 {
        0
    } else if v.abs() >= 10.0 {
        1
    } else {
        2
    };
    let s = format!("{v:.decimals$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        s
    }
}

/// A fraction 0 to 1 as a whole percent.
pub fn percent(fraction: f64) -> String {
    format!("{:.0}%", fraction * 100.0)
}

/// A bucket width: `1 s`, `10 s`, `1 min`, `15 min`.
pub fn bucket(seconds: u32) -> String {
    if seconds >= 60 && seconds.is_multiple_of(60) {
        format!("{} min", seconds / 60)
    } else {
        format!("{seconds} s")
    }
}

/// A length of time: `40 s`, `12 min`, `3 h 20 min`, `2 d 4 h`.
pub fn duration(ms: i64) -> String {
    let s = ms.max(0) / 1_000;
    let (d, h, m) = (s / 86_400, s % 86_400 / 3_600, s % 3_600 / 60);
    match (d, h, m) {
        (0, 0, 0) => format!("{s} s"),
        (0, 0, m) => format!("{m} min"),
        (0, h, 0) => format!("{h} h"),
        (0, h, m) => format!("{h} h {m} min"),
        (d, 0, _) => format!("{d} d"),
        (d, h, _) => format!("{d} d {h} h"),
    }
}

/// Rows of cells joined by two spaces; aligned into columns for a person.
pub fn table(rows: &[Vec<String>], aligned: bool, indent: &str) -> String {
    let mut widths: Vec<usize> = Vec::new();
    if aligned {
        for row in rows {
            for (i, cell) in row.iter().enumerate() {
                let w = cell.chars().count();
                match widths.get_mut(i) {
                    Some(max) => *max = (*max).max(w),
                    None => widths.push(w),
                }
            }
        }
    }
    let mut out = String::new();
    for row in rows {
        let mut line = String::from(indent);
        let last = row.len().saturating_sub(1);
        for (i, cell) in row.iter().enumerate() {
            line.push_str(cell);
            if i < last {
                let pad = widths.get(i).map_or(0, |w| w - cell.chars().count());
                line.push_str(&" ".repeat(pad + 2));
            }
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_are_clock_today_and_dated_otherwise() {
        let c = Clock::for_today("2026-11-03");
        assert_eq!(c.short("2026-11-03T15:12:00-08:00"), "15:12");
        assert_eq!(c.short_secs("2026-11-03T15:12:07-08:00"), "15:12:07");
        assert_eq!(c.short("2026-11-02T09:05:00-08:00"), "Nov 2 09:05");
        assert_eq!(c.short("2025-12-31T23:59:00-08:00"), "Dec 31 2025 23:59");
        assert_eq!(c.short("garbage"), "garbage");
        assert_eq!(c.day("2026-11-03"), "Nov 3");
        assert_eq!(c.day("2025-12-31"), "Dec 31 2025");
    }

    #[test]
    fn a_range_reaching_past_today_dates_every_time() {
        let c = Clock::for_today("2026-11-03");
        let today = c.for_range("2026-11-03T09:00:00-06:00", "2026-11-03T10:00:00-06:00");
        assert_eq!(today.short("2026-11-03T10:29:00-06:00"), "10:29");
        let week = c.for_range("2026-10-27T22:06:00-06:00", "2026-11-03T22:06:00-06:00");
        assert_eq!(week.short("2026-11-03T10:29:00-06:00"), "Nov 3 10:29");
        assert_eq!(week.short("2026-11-02T07:02:00-06:00"), "Nov 2 07:02");
    }

    #[test]
    fn values_by_unit() {
        assert_eq!(value(18.44, "percent"), "18.4%");
        assert_eq!(value(512.0, "bytes"), "512 B");
        assert_eq!(value(1_234_567.0, "bytes"), "1.2 MB");
        assert_eq!(value(123_456_789.0, "bytes_per_sec"), "123 MB/s");
        assert_eq!(value(3.25, "watts"), "3.25 W");
        assert_eq!(value(54.04, "celsius"), "54.0 °C");
        assert_eq!(value(2400.4, "rpm"), "2400 rpm");
        assert_eq!(value(2.0, "enum"), "2");
        assert_eq!(value(1.5, "furlongs"), "1.5 furlongs");
        assert_eq!(number(12.345), "12.3");
        assert_eq!(number(0.05), "0.05");
        assert_eq!(bucket(60), "1 min");
        assert_eq!(bucket(900), "15 min");
        assert_eq!(bucket(10), "10 s");
        assert_eq!(duration(40_000), "40 s");
        assert_eq!(duration(12 * 60_000 + 5_000), "12 min");
        assert_eq!(duration(3 * 3_600_000), "3 h");
        assert_eq!(duration(3 * 3_600_000 + 20 * 60_000), "3 h 20 min");
        assert_eq!(duration(2 * 86_400_000 + 4 * 3_600_000 + 60_000), "2 d 4 h");
    }

    #[test]
    fn tables_align_only_for_people() {
        let rows = vec![
            vec!["a".to_owned(), "bb".to_owned(), "c".to_owned()],
            vec!["aaa".to_owned(), "b".to_owned(), "c".to_owned()],
        ];
        assert_eq!(table(&rows, false, ""), "a  bb  c\naaa  b  c\n");
        assert_eq!(table(&rows, true, "  "), "  a    bb  c\n  aaa  b   c\n");
    }
}
