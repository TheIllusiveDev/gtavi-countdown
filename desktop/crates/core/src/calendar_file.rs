//! Validation for calendar files the app writes. Kept from the web-view
//! versions, where the name and contents came from page script: saving must
//! not be usable to write an arbitrary file to an arbitrary place.

/// Largest calendar file the app will write. A single event is under 1 KiB.
pub const MAX_BYTES: usize = 64 * 1024;

/// Same whitespace set as .NET's `string.Trim` for ASCII input.
fn is_space(c: char) -> bool {
    c == ' ' || ('\t'..='\r').contains(&c)
}

/// Checks a calendar export and returns the file name to save it under. The
/// name must be a bare .ics file name with no directory components, and the
/// contents an iCalendar object of a sane size.
pub fn validate<'a>(file_name: &'a str, contents: &str) -> Result<&'a str, &'static str> {
    let name = file_name.trim_matches(is_space);
    let allowed = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.');
    let ics = name.len() >= 4 && name.is_char_boundary(name.len() - 4) && name[name.len() - 4..].eq_ignore_ascii_case(".ics");
    if name.is_empty() || name.len() > 64 || name.starts_with('.') || !name.chars().all(allowed) || !ics {
        return Err("The calendar file name is not valid.");
    }
    if contents.len() > MAX_BYTES {
        return Err("The calendar file is too large.");
    }
    let body = contents.trim_end_matches(is_space);
    if !body.starts_with("BEGIN:VCALENDAR") || !body.ends_with("END:VCALENDAR") {
        return Err("The data is not a calendar file.");
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CALENDAR: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nEND:VCALENDAR\r\n";

    #[test]
    fn accepts_a_plain_calendar_file() {
        assert_eq!(validate("gta-vi-release.ics", CALENDAR), Ok("gta-vi-release.ics"));
        assert_eq!(validate("  Event_1.ICS ", CALENDAR), Ok("Event_1.ICS"));
    }

    #[test]
    fn rejects_paths_and_other_extensions() {
        for name in [
            "",
            "../evil.ics",
            "/etc/cron.d/evil.ics",
            "..\\evil.ics",
            "sub/dir.ics",
            ".hidden.ics",
            "notes.txt",
            "run.ics.sh",
            "nul\0.ics",
            "café.ics",
        ] {
            assert!(validate(name, CALENDAR).is_err(), "{name:?} was accepted");
        }
    }

    #[test]
    fn rejects_overlong_names() {
        assert!(validate(&format!("{}.ics", "a".repeat(80)), CALENDAR).is_err());
    }

    #[test]
    fn rejects_content_that_is_not_a_calendar() {
        assert!(validate("a.ics", "#!/bin/sh\nrm -rf ~\n").is_err());
        assert!(validate("a.ics", "BEGIN:VCALENDAR\r\n").is_err());
        let huge = format!("BEGIN:VCALENDAR\r\n{}END:VCALENDAR", "X".repeat(MAX_BYTES));
        assert!(validate("a.ics", &huge).is_err());
    }
}
