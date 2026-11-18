//! The games the countdown can count down to. This is the single place to
//! edit if a date moves.
//!
//! Each game unlocks at a wall-clock time (midnight unless the publisher says
//! otherwise), so the absolute instant depends on the time zone the countdown
//! is evaluated in. The dates are the ones the publishers announced; the
//! unlock hours are as reported and may change.

use crate::zones;

/// A calendar date. Month and day are 1-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl Date {
    pub const fn new(year: i32, month: u32, day: u32) -> Date {
        Date { year, month, day }
    }

    /// Parses "YYYY-MM-DD", accepting only real dates from 1970 to 9999.
    pub fn parse(text: &str) -> Option<Date> {
        let b = text.as_bytes();
        if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
            return None;
        }
        let digits = |r: std::ops::Range<usize>| -> Option<u32> {
            text.get(r.clone()).filter(|s| s.bytes().all(|c| c.is_ascii_digit()))?.parse().ok()
        };
        let date = Date::new(digits(0..4)? as i32, digits(5..7)?, digits(8..10)?);
        date.is_valid().then_some(date)
    }

    pub fn is_valid(&self) -> bool {
        (1970..=9999).contains(&self.year)
            && (1..=12).contains(&self.month)
            && self.day >= 1
            && self.day <= zones::days_in_month(self.year, self.month)
    }

    /// Noon UTC on this date, in Unix milliseconds. Milestones are placed
    /// there so that they land on the same calendar day in every zone.
    pub fn noon_utc(&self) -> i64 {
        zones::days_from_civil(self.year, self.month, self.day) * 86_400_000 + 12 * 3_600_000
    }

    /// The date of a UTC instant.
    pub fn of_instant(utc_ms: i64) -> Date {
        let (year, month, day) = zones::civil_from_days(utc_ms.div_euclid(86_400_000));
        Date::new(year, month, day)
    }
}

impl std::fmt::Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// A public event on the road to release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Milestone {
    pub date: Date,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    /// Stable identifier used in the settings: "gta-vi", "fable" or "custom".
    pub id: String,
    pub title: String,
    /// Used in the header and the window title: "GTA VI".
    pub short_title: String,
    pub date: Date,
    pub hour: u32,
    pub minute: u32,
    pub platforms: Vec<String>,
    /// Oldest first. The first is the origin of the progress bar; the
    /// release itself is added by the timeline.
    pub milestones: Vec<Milestone>,
}

impl Game {
    /// File-name friendly form of the identifier, for the calendar export.
    pub fn slug(&self) -> String {
        if self.id != CUSTOM {
            return self.id.clone();
        }
        let slug: String = self
            .title
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .collect();
        let slug = slug.split('-').filter(|part| !part.is_empty()).collect::<Vec<_>>().join("-");
        if slug.is_empty() { "game".to_owned() } else { slug.chars().take(40).collect() }
    }

    /// The caption over the timeline: "THE WAIT SINCE TRAILER 1".
    pub fn wait_caption(&self) -> String {
        let since = self.milestones.first().map_or("ANNOUNCEMENT".to_owned(), |m| m.title.to_uppercase());
        format!("THE WAIT SINCE {since}")
    }
}

pub const DEFAULT_GAME: &str = "gta-vi";
pub const CUSTOM: &str = "custom";

fn milestone(date: Date, title: &str, detail: &str) -> Milestone {
    Milestone { date, title: title.to_owned(), detail: detail.to_owned() }
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

/// The built-in games, in release order.
pub fn built_in() -> Vec<Game> {
    vec![
        Game {
            id: "gta-vi".into(),
            title: "Grand Theft Auto VI".into(),
            short_title: "GTA VI".into(),
            date: Date::new(2026, 11, 19),
            hour: 0,
            minute: 0,
            platforms: strings(&["PlayStation 5", "Xbox Series X|S"]),
            milestones: vec![
                milestone(Date::new(2023, 12, 4), "Trailer 1", "First trailer published, with a 2025 release window."),
                milestone(Date::new(2025, 5, 2), "First delay", "Release moved to May 26, 2026. Trailer 2 followed on May 6."),
                milestone(Date::new(2025, 11, 6), "Second delay", "Release moved to November 19, 2026."),
            ],
        },
        Game {
            id: "metro-2039".into(),
            title: "Metro 2039".into(),
            short_title: "Metro 2039".into(),
            date: Date::new(2027, 2, 4),
            hour: 0,
            minute: 0,
            platforms: strings(&["PlayStation 5", "Xbox Series X|S", "PC"]),
            milestones: vec![
                milestone(
                    Date::new(2026, 6, 7),
                    "Release window",
                    "February 2027 window announced at the Xbox Games Showcase.",
                ),
                milestone(Date::new(2026, 9, 3), "Date announced", "February 4, 2027 confirmed in a State of Play."),
            ],
        },
        Game {
            id: "god-of-war-laufey".into(),
            title: "God of War Laufey".into(),
            short_title: "God of War".into(),
            date: Date::new(2027, 2, 16),
            hour: 0,
            minute: 0,
            platforms: strings(&["PlayStation 5"]),
            milestones: vec![milestone(
                Date::new(2026, 7, 24),
                "Date announced",
                "Release date set for February 16, 2027 at San Diego Comic-Con.",
            )],
        },
        Game {
            id: "fable".into(),
            title: "Fable".into(),
            short_title: "Fable".into(),
            date: Date::new(2027, 2, 23),
            hour: 0,
            minute: 0,
            platforms: strings(&["Xbox Series X|S", "PlayStation 5", "PC"]),
            milestones: vec![
                milestone(Date::new(2020, 7, 23), "Reveal", "Announced at the Xbox Games Showcase."),
                milestone(
                    Date::new(2026, 6, 7),
                    "Date announced",
                    "Release set for February 23, 2027 at the Xbox Games Showcase.",
                ),
            ],
        },
        Game {
            id: "final-fantasy-vii-revelation".into(),
            title: "Final Fantasy VII Revelation".into(),
            short_title: "FFVII Revelation".into(),
            date: Date::new(2027, 4, 8),
            hour: 0,
            minute: 0,
            platforms: strings(&["PlayStation 5", "Xbox Series X|S", "Nintendo Switch 2", "PC"]),
            milestones: vec![
                milestone(Date::new(2026, 6, 6), "Reveal", "Announced with a spring 2027 release window."),
                milestone(Date::new(2026, 9, 3), "Date announced", "April 8, 2027 confirmed in a State of Play."),
            ],
        },
    ]
}

/// A game the user entered in the settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomGame {
    pub title: String,
    pub date: Date,
    pub hour: u32,
    pub minute: u32,
    /// The day it was added, the origin of its progress bar.
    pub added: Date,
}

pub const MAX_TITLE_CHARS: usize = 60;

impl CustomGame {
    /// Trims the title and checks every field; `None` if anything is off.
    pub fn new(title: &str, date: Date, hour: u32, minute: u32, added: Date) -> Option<CustomGame> {
        let title = title.trim();
        let count = title.chars().count();
        let ok = (1..=MAX_TITLE_CHARS).contains(&count)
            && !title.chars().any(char::is_control)
            && date.is_valid()
            && added.is_valid()
            && hour < 24
            && minute < 60;
        ok.then(|| CustomGame { title: title.to_owned(), date, hour, minute, added })
    }

    pub fn to_game(&self) -> Game {
        Game {
            id: CUSTOM.into(),
            title: self.title.clone(),
            short_title: self.title.clone(),
            date: self.date,
            hour: self.hour,
            minute: self.minute,
            platforms: Vec::new(),
            milestones: vec![milestone(self.added, "Added", "Added to the countdown.")],
        }
    }
}

/// Parses "HH:MM" in 24-hour form.
pub fn parse_time(text: &str) -> Option<(u32, u32)> {
    let (h, m) = text.split_once(':')?;
    if h.len() != 2 || m.len() != 2 || !h.bytes().chain(m.bytes()).all(|c| c.is_ascii_digit()) {
        return None;
    }
    let (h, m) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some((h, m))
}

/// The game a settings object selects: a built-in by id, the custom game, or
/// the default when neither is usable.
pub fn resolve(id: &str, custom: Option<&CustomGame>) -> Game {
    if id == CUSTOM
        && let Some(custom) = custom
    {
        return custom.to_game();
    }
    let mut games = built_in();
    let index = games.iter().position(|g| g.id == id).unwrap_or(0);
    games.swap_remove(index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_games_are_well_formed() {
        let games = built_in();
        assert_eq!(games[0].id, DEFAULT_GAME);
        for pair in games.windows(2) {
            assert!(pair[0].date <= pair[1].date, "games are listed in release order");
        }
        for game in &games {
            assert!(game.date.is_valid(), "{}", game.id);
            assert!(!game.milestones.is_empty(), "{}", game.id);
            assert!(!game.platforms.is_empty(), "{}", game.id);
            for pair in game.milestones.windows(2) {
                assert!(pair[0].date < pair[1].date, "{}", game.id);
            }
            assert!(game.milestones.last().unwrap().date < game.date, "{}", game.id);
            assert_ne!(game.id, CUSTOM);
        }
    }

    #[test]
    fn dates_parse_strictly() {
        assert_eq!(Date::parse("2027-02-23"), Some(Date::new(2027, 2, 23)));
        for bad in ["", "2027-2-23", "2027-02-30", "2027-13-01", "1969-12-31", "2027/02/23", "+027-02-23", "２０２７-02-23"] {
            assert_eq!(Date::parse(bad), None, "{bad}");
        }
        assert_eq!(Date::new(2023, 12, 4).to_string(), "2023-12-04");
        assert_eq!(Date::of_instant(Date::new(2023, 12, 4).noon_utc()), Date::new(2023, 12, 4));
    }

    #[test]
    fn times_parse_strictly() {
        assert_eq!(parse_time("00:00"), Some((0, 0)));
        assert_eq!(parse_time("23:59"), Some((23, 59)));
        for bad in ["24:00", "12:60", "1:00", "12-00", "", "ab:cd"] {
            assert_eq!(parse_time(bad), None, "{bad}");
        }
    }

    #[test]
    fn custom_games_are_validated() {
        let date = Date::new(2027, 3, 1);
        let added = Date::new(2026, 11, 18);
        let game = CustomGame::new("  Hollow Knight: Silksong 2  ", date, 9, 30, added).unwrap();
        assert_eq!(game.title, "Hollow Knight: Silksong 2");
        assert!(CustomGame::new("   ", date, 0, 0, added).is_none());
        assert!(CustomGame::new(&"x".repeat(61), date, 0, 0, added).is_none());
        assert!(CustomGame::new("Tab\there", date, 0, 0, added).is_none());
        assert!(CustomGame::new("Late", date, 24, 0, added).is_none());
        let resolved = game.to_game();
        assert_eq!(resolved.id, CUSTOM);
        assert_eq!(resolved.milestones[0].date, added);
        assert_eq!(resolved.slug(), "hollow-knight-silksong-2");
    }

    #[test]
    fn resolve_falls_back_to_the_default_game() {
        assert_eq!(resolve("fable", None).id, "fable");
        assert_eq!(resolve("no-such-game", None).id, DEFAULT_GAME);
        assert_eq!(resolve(CUSTOM, None).id, DEFAULT_GAME);
        assert_eq!(built_in()[0].wait_caption(), "THE WAIT SINCE TRAILER 1");
    }
}
