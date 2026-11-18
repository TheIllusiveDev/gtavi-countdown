//! The human-readable dates and times shown around the countdown. Names are
//! English and formatting does not depend on the host's locale.

use crate::zones::{self, UnknownTimeZone, WallTime};

const WEEKDAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

fn weekday_name(w: &WallTime) -> &'static str {
    WEEKDAYS[zones::weekday(zones::days_from_civil(w.year, w.month, w.day)) as usize]
}

fn month_name(w: &WallTime) -> &'static str {
    MONTHS[w.month as usize - 1]
}

/// "Thursday, November 19, 2026"
pub fn long_date(utc_ms: i64, zone: &str) -> Result<String, UnknownTimeZone> {
    let w = zones::wall_time_at(utc_ms, zone)?;
    Ok(format!("{}, {} {}, {}", weekday_name(&w), month_name(&w), w.day, w.year))
}

/// "00:00"
pub fn time(utc_ms: i64, zone: &str) -> Result<String, UnknownTimeZone> {
    let w = zones::wall_time_at(utc_ms, zone)?;
    Ok(format!("{:02}:{:02}", w.hour, w.minute))
}

/// "Wed, Nov 18, 18:00"
pub fn short_date_time(utc_ms: i64, zone: &str) -> Result<String, UnknownTimeZone> {
    let w = zones::wall_time_at(utc_ms, zone)?;
    Ok(format!(
        "{}, {} {}, {:02}:{:02}",
        &weekday_name(&w)[..3],
        &month_name(&w)[..3],
        w.day,
        w.hour,
        w.minute
    ))
}

/// "Dec 4, 2023", read in UTC.
pub fn short_date(utc_ms: i64) -> String {
    let (year, month, day) = zones::civil_from_days(utc_ms.div_euclid(86_400_000));
    format!("{} {day}, {year}", &MONTHS[month as usize - 1][..3])
}

/// "Zagreb time, UTC+01:00"
pub fn zone_text(zone: &str, at: i64) -> Result<String, UnknownTimeZone> {
    Ok(format!("{} time, {}", zones::zone_city(zone), zones::format_offset(zones::offset_at(at, zone)?)))
}

/// "12,345"
pub fn integer(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    if value < 0 {
        out.push('-');
    }
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}
