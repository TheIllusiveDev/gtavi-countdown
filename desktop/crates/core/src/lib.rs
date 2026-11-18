//! The parts of the countdown that can be tested without a user interface:
//! the games and their dates, time zone arithmetic, countdown maths, English
//! wording, the iCalendar writer, settings validation, theme palettes and
//! calendar-file validation. Instants are Unix milliseconds throughout, as
//! in the web version.
//!
//! The SNTP client is the separate `timesync` crate in this workspace.

pub mod calendar_file;
pub mod countdown;
pub mod games;
pub mod ics;
pub mod settings;
pub mod themes;
pub mod wording;
pub mod zones;

#[cfg(test)]
pub(crate) mod test_support {
    /// A UTC instant in Unix milliseconds.
    pub fn utc(year: i32, month: u32, day: u32, hour: i64, minute: i64, second: i64, ms: i64) -> i64 {
        crate::zones::days_from_civil(year, month, day) * 86_400_000 + hour * 3_600_000 + minute * 60_000 + second * 1000 + ms
    }

    /// The instant as an ISO 8601 string in UTC, to the millisecond.
    pub fn iso(ms: i64) -> String {
        let w = crate::zones::wall_time_at(ms, "UTC").unwrap();
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
            w.year,
            w.month,
            w.day,
            w.hour,
            w.minute,
            w.second,
            ms.rem_euclid(1000)
        )
    }
}
