//! Time zone arithmetic on IANA identifiers, built on the time zone database
//! that `jiff` finds on the system (or bundles, on Windows). Instants are Unix
//! milliseconds throughout.

use std::collections::BTreeSet;
use std::fmt;

use jiff::Timestamp;
use jiff::tz::TimeZone;

/// A wall-clock reading in some time zone, to the second. Month is 1-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WallTime {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

/// An identifier the time zone database does not know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownTimeZone(pub String);

impl fmt::Display for UnknownTimeZone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unknown time zone \"{}\".", self.0)
    }
}

impl std::error::Error for UnknownTimeZone {}

/// Continents and oceans that prefix geographic IANA zones. Anything else
/// (the inverted "Etc/GMT+N" family, legacy aliases such as "US/Eastern" or
/// bare abbreviations such as "EST") would confuse more than it helps in a picker.
const GEOGRAPHIC_AREAS: [&str; 10] = [
    "Africa/",
    "America/",
    "Antarctica/",
    "Arctic/",
    "Asia/",
    "Atlantic/",
    "Australia/",
    "Europe/",
    "Indian/",
    "Pacific/",
];

const CURATED_ZONES: [&str; 17] = [
    "UTC",
    "Pacific/Auckland",
    "Australia/Sydney",
    "Asia/Tokyo",
    "Asia/Seoul",
    "Asia/Shanghai",
    "Asia/Kolkata",
    "Asia/Dubai",
    "Europe/Moscow",
    "Europe/Berlin",
    "Europe/London",
    "America/Sao_Paulo",
    "America/New_York",
    "America/Chicago",
    "America/Denver",
    "America/Los_Angeles",
    "Pacific/Honolulu",
];

const DAY_MS: i64 = 86_400_000;

/// The database lookup is case-insensitive; the web version's is not, so a
/// name is accepted only in its exact spelling.
fn lookup(zone: &str) -> Option<TimeZone> {
    if zone.is_empty() {
        return None;
    }
    let tz = TimeZone::get(zone).ok()?;
    (tz.iana_name() == Some(zone)).then_some(tz)
}

fn resolve(zone: &str) -> Result<TimeZone, UnknownTimeZone> {
    lookup(zone).ok_or_else(|| UnknownTimeZone(zone.to_owned()))
}

fn timestamp(utc_ms: i64) -> Timestamp {
    // Every instant the app handles is within jiff's range of ±9999 years.
    Timestamp::from_millisecond(utc_ms).unwrap_or(Timestamp::UNIX_EPOCH)
}

/// Days since 1970-01-01 of a proleptic Gregorian date (Howard Hinnant's
/// `days_from_civil`).
pub fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let y = i64::from(year) - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = i64::from(month);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The date of a day number from [`days_from_civil`].
pub fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
    (year, month, day)
}

pub fn days_in_month(year: i32, month: u32) -> u32 {
    let next = if month == 12 { days_from_civil(year + 1, 1, 1) } else { days_from_civil(year, month + 1, 1) };
    (next - days_from_civil(year, month, 1)) as u32
}

/// Day of the week of a day number: 0 is Sunday, 6 is Saturday.
pub fn weekday(days: i64) -> u32 {
    // 1970-01-01 was a Thursday.
    (days + 4).rem_euclid(7) as u32
}

/// The wall-clock reading in `zone` at the instant `utc_ms`.
pub fn wall_time_at(utc_ms: i64, zone: &str) -> Result<WallTime, UnknownTimeZone> {
    let local = utc_ms + offset_at(utc_ms, zone)?;
    let (year, month, day) = civil_from_days(local.div_euclid(DAY_MS));
    let seconds = local.rem_euclid(DAY_MS) / 1000;
    Ok(WallTime {
        year,
        month,
        day,
        hour: (seconds / 3600) as u32,
        minute: (seconds % 3600 / 60) as u32,
        second: (seconds % 60) as u32,
    })
}

/// Offset of `zone` from UTC at `utc_ms`, in milliseconds. Positive east of Greenwich.
pub fn offset_at(utc_ms: i64, zone: &str) -> Result<i64, UnknownTimeZone> {
    let tz = resolve(zone)?;
    Ok(i64::from(tz.to_offset(timestamp(utc_ms)).seconds()) * 1000)
}

/// Converts a wall-clock time in `zone` to the UTC instant it denotes. The
/// offset is resolved in two passes so the result is correct when the first
/// guess lands on the other side of a daylight-saving transition.
pub fn zoned_time_to_utc(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    zone: &str,
) -> Result<i64, UnknownTimeZone> {
    let wall = days_from_civil(year, month, day) * DAY_MS + i64::from(hour) * 3_600_000 + i64::from(minute) * 60_000;
    let first_guess = wall - offset_at(wall, zone)?;
    Ok(wall - offset_at(first_guess, zone)?)
}

/// "UTC+05:30", "UTC-08:00" or "UTC".
pub fn format_offset(offset_ms: i64) -> String {
    if offset_ms == 0 {
        return "UTC".to_owned();
    }
    let sign = if offset_ms < 0 { '-' } else { '+' };
    // Round half away from zero, as the web version does.
    let total_minutes = (offset_ms.unsigned_abs() as f64 / 60_000.0).round() as u64;
    format!("UTC{sign}{:02}:{:02}", total_minutes / 60, total_minutes % 60)
}

/// The IANA zone the host system is configured for, or "UTC".
pub fn system_time_zone() -> String {
    TimeZone::system()
        .iana_name()
        .filter(|name| is_valid_time_zone(name))
        .map_or_else(|| "UTC".to_owned(), str::to_owned)
}

pub fn is_valid_time_zone(zone: &str) -> bool {
    lookup(zone).is_some()
}

/// Every geographic IANA zone the database knows, plus UTC first, falling
/// back to a short curated list when the database cannot be enumerated.
pub fn list_time_zones() -> Vec<String> {
    let found: BTreeSet<String> = jiff::tz::db()
        .available()
        .map(|name| name.as_str().to_owned())
        .filter(|name| GEOGRAPHIC_AREAS.iter().any(|area| name.starts_with(area)))
        .filter(|name| is_valid_time_zone(name))
        .collect();
    if found.len() >= 10 {
        return std::iter::once("UTC".to_owned()).chain(found).collect();
    }
    CURATED_ZONES
        .iter()
        .filter(|zone| is_valid_time_zone(zone))
        .map(|zone| (*zone).to_owned())
        .collect()
}

/// "America/Argentina/Buenos_Aires" becomes "Buenos Aires".
pub fn zone_city(zone: &str) -> String {
    zone.rsplit('/').next().unwrap_or(zone).replace('_', " ")
}

/// "America/Argentina/Buenos_Aires" becomes "America / Argentina".
pub fn zone_region(zone: &str) -> String {
    match zone.rfind('/') {
        Some(cut) => zone[..cut].replace('/', " / ").replace('_', " "),
        None => String::new(),
    }
}

fn day_number(utc_ms: i64, zone: &str) -> Result<i64, UnknownTimeZone> {
    let w = wall_time_at(utc_ms, zone)?;
    Ok(days_from_civil(w.year, w.month, w.day))
}

/// Whole calendar days from the date of `from_ms` to the date of `to_ms`, both
/// read in `zone`: the number of midnights in between ("sleeps").
pub fn calendar_days_between(from_ms: i64, to_ms: i64, zone: &str) -> Result<i64, UnknownTimeZone> {
    Ok(day_number(to_ms, zone)? - day_number(from_ms, zone)?)
}

/// Saturdays strictly after the date of `from_ms` and strictly before the date
/// of `to_ms` in `zone`: the full weekends still to get through.
pub fn weekends_between(from_ms: i64, to_ms: i64, zone: &str) -> Result<i64, UnknownTimeZone> {
    let days_between = calendar_days_between(from_ms, to_ms, zone)?;
    if days_between <= 1 {
        return Ok(0);
    }
    let start_dow = i64::from(weekday(day_number(from_ms, zone)?));
    // Days until the first Saturday after the starting date (1 to 7).
    let until_saturday = match (6 - start_dow + 7) % 7 {
        0 => 7,
        n => n,
    };
    if until_saturday >= days_between {
        return Ok(0);
    }
    Ok(1 + (days_between - 1 - until_saturday) / 7)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{iso, utc};

    const HOUR: i64 = 3_600_000;

    fn midnight(zone: &str) -> i64 {
        zoned_time_to_utc(2026, 11, 19, 0, 0, zone).unwrap()
    }

    #[test]
    fn civil_day_numbers_round_trip() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
        for days in [-800_000, -1, 0, 1, 20_000, 2_000_000] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2026, 12), 31);
        // 19 November 2026 is a Thursday.
        assert_eq!(weekday(days_from_civil(2026, 11, 19)), 4);
    }

    #[test]
    fn offset_at_returns_zero_for_utc() {
        assert_eq!(offset_at(utc(2026, 11, 19, 0, 0, 0, 0), "UTC"), Ok(0));
    }

    #[test]
    fn offset_at_follows_daylight_saving_in_europe() {
        assert_eq!(offset_at(utc(2026, 11, 19, 0, 0, 0, 0), "Europe/Zagreb"), Ok(HOUR));
        assert_eq!(offset_at(utc(2026, 7, 1, 0, 0, 0, 0), "Europe/Zagreb"), Ok(2 * HOUR));
    }

    #[test]
    fn offset_at_handles_zones_west_of_greenwich_and_fractional_offsets() {
        let t = utc(2026, 11, 19, 0, 0, 0, 0);
        assert_eq!(offset_at(t, "America/New_York"), Ok(-5 * HOUR));
        assert_eq!(offset_at(t, "Asia/Kolkata"), Ok(HOUR * 11 / 2));
        assert_eq!(offset_at(t, "Pacific/Chatham"), Ok(HOUR * 55 / 4));
    }

    #[test]
    fn offset_at_ignores_the_sub_second_part_of_the_instant() {
        assert_eq!(offset_at(utc(2026, 11, 19, 0, 0, 0, 999), "Asia/Tokyo"), Ok(9 * HOUR));
    }

    #[test]
    fn unknown_zones_are_errors() {
        assert_eq!(offset_at(0, "Mars/Olympus_Mons"), Err(UnknownTimeZone("Mars/Olympus_Mons".into())));
        assert!(wall_time_at(0, "").is_err());
    }

    #[test]
    fn release_midnight_resolves_around_the_world() {
        for (zone, expected) in [
            ("UTC", "2026-11-19T00:00:00.000Z"),
            ("Europe/London", "2026-11-19T00:00:00.000Z"),
            ("Europe/Zagreb", "2026-11-18T23:00:00.000Z"),
            ("Pacific/Auckland", "2026-11-18T11:00:00.000Z"),
            ("Asia/Tokyo", "2026-11-18T15:00:00.000Z"),
            ("Asia/Kolkata", "2026-11-18T18:30:00.000Z"),
            ("America/New_York", "2026-11-19T05:00:00.000Z"),
            ("America/Los_Angeles", "2026-11-19T08:00:00.000Z"),
        ] {
            assert_eq!(iso(midnight(zone)), expected, "{zone}");
        }
    }

    #[test]
    fn zoned_time_to_utc_is_correct_on_both_sides_of_a_daylight_saving_change() {
        // Europe leaves summer time on 25 October 2026 at 01:00 UTC.
        assert_eq!(
            iso(zoned_time_to_utc(2026, 10, 25, 0, 30, "Europe/Berlin").unwrap()),
            "2026-10-24T22:30:00.000Z"
        );
        assert_eq!(
            iso(zoned_time_to_utc(2026, 10, 25, 12, 0, "Europe/Berlin").unwrap()),
            "2026-10-25T11:00:00.000Z"
        );
    }

    #[test]
    fn round_trips_through_wall_time_at() {
        for zone in ["Australia/Sydney", "America/Sao_Paulo", "Asia/Kathmandu", "Pacific/Honolulu"] {
            let wall = wall_time_at(midnight(zone), zone).unwrap();
            assert_eq!(wall, WallTime { year: 2026, month: 11, day: 19, hour: 0, minute: 0, second: 0 }, "{zone}");
        }
    }

    #[test]
    fn format_offset_formats_positive_negative_fractional_and_zero_offsets() {
        assert_eq!(format_offset(0), "UTC");
        assert_eq!(format_offset(HOUR), "UTC+01:00");
        assert_eq!(format_offset(-8 * HOUR), "UTC-08:00");
        assert_eq!(format_offset(HOUR * 11 / 2), "UTC+05:30");
        assert_eq!(format_offset(-HOUR * 7 / 2), "UTC-03:30");
        assert_eq!(format_offset(14 * HOUR), "UTC+14:00");
    }

    #[test]
    fn zone_names_are_validated() {
        assert!(is_valid_time_zone("Europe/Zagreb"));
        assert!(!is_valid_time_zone("Mars/Olympus_Mons"));
        assert!(!is_valid_time_zone(""));
        // The web version matches names exactly.
        assert!(!is_valid_time_zone("europe/zagreb"));
    }

    #[test]
    fn system_time_zone_is_valid() {
        assert!(is_valid_time_zone(&system_time_zone()));
    }

    #[test]
    fn list_time_zones_includes_utc() {
        let zones = list_time_zones();
        assert!(zones.len() > 10);
        assert_eq!(zones[0], "UTC");
        for zone in &zones {
            assert!(is_valid_time_zone(zone), "{zone}");
        }
    }

    #[test]
    fn list_time_zones_lists_geographic_zones_only_without_duplicates() {
        let zones = list_time_zones();
        for zone in &zones {
            assert!(!zone.starts_with("Etc/"), "{zone}");
            assert!(zone == "UTC" || zone.contains('/'), "{zone}");
        }
        let unique: BTreeSet<_> = zones.iter().collect();
        assert_eq!(unique.len(), zones.len());
        for expected in ["Pacific/Auckland", "America/Los_Angeles", "Europe/Zagreb"] {
            assert!(zones.iter().any(|zone| zone == expected), "{expected}");
        }
    }

    #[test]
    fn zone_names_split_into_city_and_region() {
        assert_eq!(zone_city("America/Argentina/Buenos_Aires"), "Buenos Aires");
        assert_eq!(zone_region("America/Argentina/Buenos_Aires"), "America / Argentina");
        assert_eq!(zone_city("UTC"), "UTC");
        assert_eq!(zone_region("UTC"), "");
    }

    const ZONE: &str = "Europe/Zagreb";

    fn at(month: u32, day: u32, hour: u32, minute: u32) -> i64 {
        zoned_time_to_utc(2026, month, day, hour, minute, ZONE).unwrap()
    }

    #[test]
    fn calendar_days_between_counts_sleeps_as_midnights_in_the_target_zone() {
        let release = midnight(ZONE);
        // Thursday evening, a week before release: seven nights to go.
        assert_eq!(calendar_days_between(at(11, 12, 20, 30), release, ZONE), Ok(7));
        // One minute past midnight on the 18th: still one night.
        assert_eq!(calendar_days_between(at(11, 18, 0, 1), release, ZONE), Ok(1));
        assert_eq!(calendar_days_between(at(11, 18, 23, 59), release, ZONE), Ok(1));
    }

    #[test]
    fn calendar_days_between_counts_across_a_daylight_saving_change_without_drifting() {
        assert_eq!(calendar_days_between(at(10, 20, 23, 0), midnight(ZONE), ZONE), Ok(30));
    }

    #[test]
    fn weekends_between_counts_the_saturdays_strictly_between_two_dates() {
        let release = midnight(ZONE);
        // 12 November 2026 is a Thursday; the 14th is the only Saturday before the 19th.
        assert_eq!(weekends_between(at(11, 12, 12, 0), release, ZONE), Ok(1));
        assert_eq!(weekends_between(at(11, 13, 12, 0), release, ZONE), Ok(1));
        // On the Saturday itself the weekend has begun and no longer counts.
        assert_eq!(weekends_between(at(11, 14, 12, 0), release, ZONE), Ok(0));
        assert_eq!(weekends_between(at(11, 18, 12, 0), release, ZONE), Ok(0));
        // From Sunday 1 November: the 7th and the 14th.
        assert_eq!(weekends_between(at(11, 1, 12, 0), release, ZONE), Ok(2));
        // From Saturday 31 October: the 7th and the 14th.
        assert_eq!(weekends_between(at(10, 31, 12, 0), release, ZONE), Ok(2));
    }
}
