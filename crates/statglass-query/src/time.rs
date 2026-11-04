//! Times in and out of the contract. Output is RFC 3339 in the controller Mac's local
//! offset, because agents reason about "3pm" in local time and misread epoch
//! milliseconds. Input accepts RFC 3339, a bare local time today, or a duration ago.
//! Bare times use this Mac's timezone even for remote hosts, whose zone is unknown.

use jiff::{
    Timestamp,
    civil::Time,
    tz::{Disambiguation, TimeZone},
};

/// Why a time argument was refused. The CLI reports each as `BadRange`.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TimeError {
    #[error("empty time argument")]
    Empty,
    #[error("{0:?} is not a duration like 90s, 15m, 2h or 3d")]
    BadDuration(String),
    #[error("{0:?} is not an RFC 3339 time, a time today like 15:00, or a duration ago")]
    BadInstant(String),
    #[error("{0:?} is outside the supported time range")]
    OutOfRange(String),
}

/// `ms` as RFC 3339 in the system timezone, with fractional seconds only when non-zero:
/// `2026-11-03T15:12:00-08:00`.
pub fn rfc3339_local(ms: i64) -> String {
    rfc3339_in(ms, &TimeZone::system())
}

pub(crate) fn rfc3339_in(ms: i64, tz: &TimeZone) -> String {
    let ts = Timestamp::from_millisecond(ms).unwrap_or(if ms < 0 {
        Timestamp::MIN
    } else {
        Timestamp::MAX
    });
    ts.to_zoned(tz.clone())
        .strftime("%Y-%m-%dT%H:%M:%S%.f%:z")
        .to_string()
}

/// The system timezone's IANA name (`America/Los_Angeles`), or `unknown` when the
/// system zone has none (a POSIX `TZ` string).
pub fn timezone_name() -> String {
    TimeZone::system()
        .iana_name()
        .unwrap_or("unknown")
        .to_owned()
}

/// A positive duration in milliseconds: digits then `s`, `m`, `h` or `d`.
pub fn parse_duration(arg: &str) -> Result<i64, TimeError> {
    let bad = || TimeError::BadDuration(arg.to_owned());
    let arg_t = arg.trim();
    let split = arg_t.find(|c: char| !c.is_ascii_digit()).ok_or_else(bad)?;
    let (digits, unit) = arg_t.split_at(split);
    let per_unit: i64 = match unit {
        "s" => 1_000,
        "m" => 60_000,
        "h" => 3_600_000,
        "d" => 86_400_000,
        _ => return Err(bad()),
    };
    let n: i64 = digits.parse().map_err(|_| bad())?;
    if n == 0 {
        return Err(bad());
    }
    n.checked_mul(per_unit)
        .ok_or_else(|| TimeError::OutOfRange(arg.to_owned()))
}

/// A `--from`/`--to` argument as a millisecond epoch: RFC 3339 with an offset or `Z`, a
/// bare time today in the system zone (`15:00`, `15:00:30`), or a duration meaning that
/// long before `now_ms` (`3h`). A bare time in a DST gap or overlap resolves to the
/// earlier of its two possible instants.
pub fn parse_instant(arg: &str, now_ms: i64) -> Result<i64, TimeError> {
    parse_instant_in(arg, now_ms, &TimeZone::system())
}

pub(crate) fn parse_instant_in(arg: &str, now_ms: i64, tz: &TimeZone) -> Result<i64, TimeError> {
    let arg_t = arg.trim();
    if arg_t.is_empty() {
        return Err(TimeError::Empty);
    }
    let out_of_range = || TimeError::OutOfRange(arg.to_owned());
    if let Ok(ago) = parse_duration(arg_t) {
        return now_ms.checked_sub(ago).ok_or_else(out_of_range);
    }
    if let Some(time) = bare_time(arg_t) {
        let now = Timestamp::from_millisecond(now_ms).map_err(|_| out_of_range())?;
        let today = now.to_zoned(tz.clone()).date();
        let zoned = tz
            .to_ambiguous_zoned(today.to_datetime(time))
            .disambiguate(Disambiguation::Earlier)
            .map_err(|_| out_of_range())?;
        return Ok(zoned.timestamp().as_millisecond());
    }
    arg_t
        .parse::<Timestamp>()
        .map(|ts| ts.as_millisecond())
        .map_err(|_| TimeError::BadInstant(arg.to_owned()))
}

/// `HH:MM` or `HH:MM:SS`, nothing else.
fn bare_time(arg: &str) -> Option<Time> {
    ["%H:%M:%S", "%H:%M"]
        .into_iter()
        .find_map(|fmt| Time::strptime(fmt, arg).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn la() -> TimeZone {
        TimeZone::get("America/Los_Angeles").unwrap()
    }

    fn ms(rfc3339: &str) -> i64 {
        rfc3339.parse::<Timestamp>().unwrap().as_millisecond()
    }

    #[test]
    fn durations() {
        assert_eq!(parse_duration("90s"), Ok(90_000));
        assert_eq!(parse_duration("15m"), Ok(900_000));
        assert_eq!(parse_duration("2h"), Ok(7_200_000));
        assert_eq!(parse_duration("3d"), Ok(259_200_000));
        for bad in ["", "h", "0m", "1.5h", "2w", "-3h", "3 h", "3hh", "h3"] {
            assert!(
                matches!(parse_duration(bad), Err(TimeError::BadDuration(_))),
                "{bad}"
            );
        }
        assert!(matches!(
            parse_duration("99999999999999999d"),
            Err(TimeError::BadDuration(_) | TimeError::OutOfRange(_))
        ));
        assert!(matches!(
            parse_duration("9999999999999d"),
            Err(TimeError::OutOfRange(_))
        ));
    }

    #[test]
    fn rfc3339_with_offset_or_z() {
        let now = ms("2026-11-03T23:00:00Z");
        let tz = la();
        assert_eq!(
            parse_instant_in("2026-11-03T15:12:00-08:00", now, &tz),
            Ok(ms("2026-11-03T23:12:00Z"))
        );
        assert_eq!(
            parse_instant_in("2026-11-03T23:12:00Z", now, &tz),
            Ok(ms("2026-11-03T23:12:00Z"))
        );
        assert!(matches!(
            parse_instant_in("2026-11-03T15:12:00", now, &tz),
            Err(TimeError::BadInstant(_))
        ));
        assert!(matches!(
            parse_instant_in("yesterday", now, &tz),
            Err(TimeError::BadInstant(_))
        ));
        assert_eq!(parse_instant_in("  ", now, &tz), Err(TimeError::Empty));
    }

    #[test]
    fn duration_ago_and_bare_time_today() {
        let tz = la();
        // 15:00 PST on 2026-11-03.
        let now = ms("2026-11-03T23:00:00Z");
        assert_eq!(parse_instant_in("3h", now, &tz), Ok(now - 3 * 3_600_000));
        assert_eq!(
            parse_instant_in("09:30", now, &tz),
            Ok(ms("2026-11-03T17:30:00Z"))
        );
        assert_eq!(
            parse_instant_in("09:30:15", now, &tz),
            Ok(ms("2026-11-03T17:30:15Z"))
        );
        // "Today" is the zone's date, not UTC's: 01:00 UTC on the 4th is the 3rd in LA.
        let late = ms("2026-11-04T01:00:00Z");
        assert_eq!(
            parse_instant_in("09:30", late, &tz),
            Ok(ms("2026-11-03T17:30:00Z"))
        );
        assert!(matches!(
            parse_instant_in("25:00", now, &tz),
            Err(TimeError::BadInstant(_))
        ));
    }

    #[test]
    fn dst_gap_and_overlap_resolve_to_the_earlier_instant() {
        let tz = la();
        // 2026-03-08: clocks jump 02:00 PST -> 03:00 PDT, so 02:30 does not exist. Its
        // two readings are 02:30-08:00 (10:30Z) and 02:30-07:00 (09:30Z).
        let spring = ms("2026-03-08T20:00:00Z");
        // The earlier is 09:30Z, which reads as 01:30 PST.
        assert_eq!(
            parse_instant_in("02:30", spring, &tz),
            Ok(ms("2026-03-08T09:30:00Z"))
        );
        // 2026-11-01: clocks fall back 02:00 PDT -> 01:00 PST, so 01:30 happens twice:
        // 08:30Z (PDT) and 09:30Z (PST). The earlier is PDT.
        let fall = ms("2026-11-01T20:00:00Z");
        assert_eq!(
            parse_instant_in("01:30", fall, &tz),
            Ok(ms("2026-11-01T08:30:00Z"))
        );
    }

    #[test]
    fn formats_in_the_zone_with_fraction_only_when_present() {
        let tz = la();
        assert_eq!(
            rfc3339_in(ms("2026-11-03T23:12:00Z"), &tz),
            "2026-11-03T15:12:00-08:00"
        );
        assert_eq!(
            rfc3339_in(ms("2026-11-03T23:12:00.250Z"), &tz),
            "2026-11-03T15:12:00.25-08:00"
        );
        assert_eq!(
            rfc3339_in(ms("2026-07-01T19:00:00Z"), &tz),
            "2026-07-01T12:00:00-07:00"
        );
        let out = rfc3339_local(0);
        assert_eq!(
            out.parse::<Timestamp>().unwrap().as_millisecond(),
            0,
            "{out}"
        );
        assert!(!timezone_name().is_empty());
    }
}
