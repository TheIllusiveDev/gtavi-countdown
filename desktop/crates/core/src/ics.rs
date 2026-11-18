//! Minimal iCalendar (RFC 5545) writer for a single event with reminders.

use crate::zones;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarEvent {
    pub uid: String,
    /// UTC instant in Unix milliseconds.
    pub start: i64,
    pub duration_minutes: i64,
    pub summary: String,
    pub description: String,
    /// Minutes before the start.
    pub reminders: Vec<i64>,
    /// Creation stamp; defaults to now.
    pub stamp: Option<i64>,
}

/// 2026-11-18T23:00:00.000Z becomes 20261118T230000Z.
pub fn date(utc_ms: i64) -> String {
    let days = utc_ms.div_euclid(86_400_000);
    let seconds = utc_ms.rem_euclid(86_400_000) / 1000;
    let (year, month, day) = zones::civil_from_days(days);
    format!(
        "{year:04}{month:02}{day:02}T{:02}{:02}{:02}Z",
        seconds / 3600,
        seconds % 3600 / 60,
        seconds % 60
    )
}

/// Escapes TEXT values as required by RFC 5545 section 3.3.11.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str(r"\;"),
            ',' => out.push_str("\\,"),
            '\r' if chars.peek() == Some(&'\n') => {
                chars.next();
                out.push_str("\\n");
            }
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out
}

/// Folds a content line so that no physical line exceeds 75 octets, never
/// splitting a UTF-8 sequence.
pub fn fold(line: &str) -> String {
    if line.len() <= 75 {
        return line.to_owned();
    }
    let mut out = String::with_capacity(line.len() + line.len() / 74 * 3);
    let mut current = 0;
    // The first line may hold 75 octets; continuation lines start with a
    // space, which leaves 74 octets for content.
    let mut limit = 75;
    for c in line.chars() {
        let size = c.len_utf8();
        if current + size > limit {
            out.push_str("\r\n ");
            current = 0;
            limit = 74;
        }
        out.push(c);
        current += size;
    }
    out
}

pub fn build_calendar(e: &CalendarEvent) -> String {
    let stamp = e.stamp.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64)
    });
    let mut lines = vec![
        "BEGIN:VCALENDAR".to_owned(),
        "VERSION:2.0".to_owned(),
        "PRODID:-//gtavi-countdown//Release Countdown//EN".to_owned(),
        "CALSCALE:GREGORIAN".to_owned(),
        "METHOD:PUBLISH".to_owned(),
        "BEGIN:VEVENT".to_owned(),
        format!("UID:{}", e.uid),
        format!("DTSTAMP:{}", date(stamp)),
        format!("DTSTART:{}", date(e.start)),
        format!("DTEND:{}", date(e.start + e.duration_minutes * 60_000)),
        format!("SUMMARY:{}", escape(&e.summary)),
        format!("DESCRIPTION:{}", escape(&e.description)),
        "TRANSP:TRANSPARENT".to_owned(),
    ];
    for minutes in &e.reminders {
        lines.push("BEGIN:VALARM".to_owned());
        lines.push("ACTION:DISPLAY".to_owned());
        lines.push(format!("DESCRIPTION:{}", escape(&e.summary)));
        lines.push(format!("TRIGGER:-PT{minutes}M"));
        lines.push("END:VALARM".to_owned());
    }
    lines.push("END:VEVENT".to_owned());
    lines.push("END:VCALENDAR".to_owned());
    lines.iter().map(|line| fold(line) + "\r\n").collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::utc;

    fn unfold(lines: &[&str]) -> String {
        lines.iter().enumerate().map(|(i, line)| if i == 0 { *line } else { &line[1..] }).collect()
    }

    #[test]
    fn date_formats_an_instant_as_a_utc_date_time() {
        assert_eq!(date(utc(2026, 11, 18, 23, 0, 0, 0)), "20261118T230000Z");
        assert_eq!(date(utc(2026, 1, 2, 3, 4, 5, 678)), "20260102T030405Z");
    }

    #[test]
    fn escape_escapes_the_characters_rfc_5545_reserves_in_text() {
        assert_eq!(escape("PS5, Xbox; both"), r"PS5\, Xbox\; both");
        assert_eq!(escape("a\\b"), "a\\\\b");
        assert_eq!(escape("line one\nline two\r\nline three"), "line one\\nline two\\nline three");
    }

    #[test]
    fn fold_leaves_short_lines_alone() {
        assert_eq!(fold("SUMMARY:Short"), "SUMMARY:Short");
        assert_eq!(fold(&"X".repeat(75)), "X".repeat(75));
    }

    #[test]
    fn fold_folds_long_lines_at_75_octets_with_a_leading_space() {
        let original = format!("DESCRIPTION:{}", "a".repeat(200));
        let folded = fold(&original);
        let lines: Vec<&str> = folded.split("\r\n").collect();
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|line| line.len() <= 75));
        assert!(lines[1..].iter().all(|line| line.starts_with(' ')));
        // Unfolding restores the original content line.
        assert_eq!(unfold(&lines), original);
    }

    #[test]
    fn fold_never_splits_a_multi_byte_character() {
        let original = format!("SUMMARY:{}", "é中\u{1F3AE}".repeat(30));
        let folded = fold(&original);
        let lines: Vec<&str> = folded.split("\r\n").collect();
        for line in &lines {
            assert!(line.len() <= 75);
            // Splitting a sequence would not even produce valid &str slices;
            // check that no line is empty after its continuation space.
            assert!(!line.trim_start_matches(' ').is_empty());
        }
        assert_eq!(unfold(&lines), original);
    }

    fn calendar() -> String {
        build_calendar(&CalendarEvent {
            uid: "release-1@gtavi-countdown".into(),
            start: utc(2026, 11, 18, 23, 0, 0, 0),
            duration_minutes: 60,
            summary: "Grand Theft Auto VI release".into(),
            description: "Platforms: PlayStation 5, Xbox Series X|S.".into(),
            reminders: vec![1440, 60],
            stamp: Some(utc(2026, 11, 12, 19, 0, 0, 0)),
        })
    }

    #[test]
    fn build_calendar_produces_a_well_formed_calendar_object() {
        let text = calendar();
        let lines: Vec<&str> = text.split("\r\n").collect();
        assert_eq!(lines[0], "BEGIN:VCALENDAR");
        assert_eq!(lines[lines.len() - 2], "END:VCALENDAR");
        assert_eq!(lines[lines.len() - 1], "");
        assert!(!text.replace("\r\n", "").contains('\n'));
        for expected in [
            "VERSION:2.0",
            "UID:release-1@gtavi-countdown",
            "DTSTAMP:20261112T190000Z",
            "DTSTART:20261118T230000Z",
            "DTEND:20261119T000000Z",
            "SUMMARY:Grand Theft Auto VI release",
            "DESCRIPTION:Platforms: PlayStation 5\\, Xbox Series X|S.",
        ] {
            assert!(lines.contains(&expected), "{expected}");
        }
    }

    #[test]
    fn build_calendar_adds_one_alarm_per_reminder() {
        let text = calendar();
        let lines: Vec<&str> = text.split("\r\n").collect();
        assert_eq!(lines.iter().filter(|l| **l == "BEGIN:VALARM").count(), 2);
        assert_eq!(lines.iter().filter(|l| **l == "END:VALARM").count(), 2);
        assert!(lines.contains(&"TRIGGER:-PT1440M"));
        assert!(lines.contains(&"TRIGGER:-PT60M"));
    }

    #[test]
    fn build_calendar_balances_every_begin_with_an_end() {
        let text = calendar();
        let lines: Vec<&str> = text.split("\r\n").collect();
        assert_eq!(
            lines.iter().filter(|l| l.starts_with("BEGIN:")).count(),
            lines.iter().filter(|l| l.starts_with("END:")).count()
        );
    }
}
