//! Text formatting shared by the renderers: times, values with units, and simple
//! tables. Presentation only; every value's meaning comes from the answer.

/// Formats the answer's RFC 3339 times relative to today in the local zone. The app
/// writes times in this Mac's offset, so the date and clock fields can be read as they are.
#[derive(Clone, Debug)]
pub struct Clock {
    /// `YYYY-MM-DD` today, local.
    today: String,
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

    fn render(&self, t: &str, secs: bool) -> String {
        let (Some(date), Some(clock)) = (t.get(..10), t.get(11..if secs { 19 } else { 16 })) else {
            return t.to_owned();
        };
        if date == self.today {
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
