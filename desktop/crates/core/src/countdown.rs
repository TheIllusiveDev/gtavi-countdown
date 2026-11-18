//! Countdown maths. Instants and durations are milliseconds.

use crate::games::Game;
use crate::zones::{self, UnknownTimeZone};

pub const SECOND: i64 = 1000;
pub const MINUTE: i64 = 60 * SECOND;
pub const HOUR: i64 = 60 * MINUTE;
pub const DAY: i64 = 24 * HOUR;

/// The distance to the target split into display units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Breakdown {
    /// True once the target has passed; the parts then count upwards.
    pub elapsed: bool,
    pub days: i64,
    pub hours: i64,
    pub minutes: i64,
    pub seconds: i64,
    pub milliseconds: i64,
    /// Absolute distance to the target.
    pub total: i64,
}

/// Alternative single-unit readings of the remaining time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Totals {
    pub weeks: f64,
    pub hours: i64,
    pub minutes: i64,
    pub seconds: i64,
    /// Midnights between now and the target in the target zone.
    pub sleeps: i64,
    /// Full weekends (Saturdays) between now and the target.
    pub weekends: i64,
}

/// The instant `game` unlocks for someone whose clock reads `zone`.
pub fn release_instant(game: &Game, zone: &str) -> Result<i64, UnknownTimeZone> {
    let d = game.date;
    zones::zoned_time_to_utc(d.year, d.month, d.day, game.hour, game.minute, zone)
}

pub fn breakdown(now: i64, target: i64) -> Breakdown {
    let diff = target - now;
    let total = diff.abs();
    Breakdown {
        elapsed: diff <= 0,
        days: total / DAY,
        hours: total % DAY / HOUR,
        minutes: total % HOUR / MINUTE,
        seconds: total % MINUTE / SECOND,
        milliseconds: total % SECOND,
        total,
    }
}

/// Milliseconds until the display changes next. Scheduling the tick on this
/// boundary keeps the display in phase with the target instead of with the
/// moment the app happened to start. Before the target the seconds are
/// floored, so the reading drops one millisecond after each whole second of
/// remaining time; after it, the reading rises on each whole second of
/// elapsed time.
pub fn ms_until_next_second(now: i64, target: i64) -> i64 {
    let diff = target - now;
    if diff <= 0 {
        SECOND - (-diff % SECOND)
    } else if diff < SECOND {
        diff
    } else {
        diff % SECOND + 1
    }
}

pub fn totals(now: i64, target: i64, zone: &str) -> Result<Totals, UnknownTimeZone> {
    let remaining = (target - now).max(0);
    let counted = remaining > 0;
    Ok(Totals {
        weeks: remaining as f64 / (7.0 * DAY as f64),
        hours: remaining / HOUR,
        minutes: remaining / MINUTE,
        seconds: remaining / SECOND,
        sleeps: if counted { zones::calendar_days_between(now, target, zone)?.max(0) } else { 0 },
        weekends: if counted { zones::weekends_between(now, target, zone)? } else { 0 },
    })
}

/// The instant the wait began: the game's first milestone, such as the
/// publication of the first trailer.
pub fn wait_origin(game: &Game) -> i64 {
    game.milestones.first().map_or(game.date.noon_utc(), |m| m.date.noon_utc())
}

/// Fraction of the wait since the first milestone that is over, from 0 to 1.
pub fn wait_progress(game: &Game, now: i64, target: i64) -> f64 {
    timeline_position(game, now, target)
}

/// Position of an instant on the wait timeline, from 0 to 1.
pub fn timeline_position(game: &Game, instant: i64, target: i64) -> f64 {
    let origin = wait_origin(game);
    let span = target - origin;
    if span <= 0 {
        return 1.0;
    }
    ((instant - origin) as f64 / span as f64).clamp(0.0, 1.0)
}

/// Compact one-line reading such as "6d 03h 12m 08s".
pub fn format_compact(b: &Breakdown) -> String {
    format!("{}d {:02}h {:02}m {:02}s", b.days, b.hours, b.minutes, b.seconds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::iso;
    use crate::games::{self, Date};
    use crate::wording;

    fn gta() -> Game {
        games::built_in().remove(0)
    }

    fn target() -> i64 {
        release_instant(&gta(), "UTC").unwrap()
    }

    #[test]
    fn release_instant_is_midnight_on_19_november_in_the_given_zone() {
        assert_eq!(iso(target()), "2026-11-19T00:00:00.000Z");
        assert_eq!(iso(release_instant(&gta(), "Europe/Zagreb").unwrap()), "2026-11-18T23:00:00.000Z");
    }

    #[test]
    fn release_instant_moves_westwards_with_the_sun() {
        let at = |zone| release_instant(&gta(), zone).unwrap();
        assert!(at("Pacific/Auckland") < at("Asia/Tokyo"));
        assert!(at("Asia/Tokyo") < at("Europe/London"));
        assert!(at("Europe/London") < at("America/Los_Angeles"));
    }

    #[test]
    fn breakdown_splits_the_remaining_time_into_units() {
        let total = 6 * DAY + 3 * HOUR + 12 * MINUTE + 8 * SECOND + 250;
        assert_eq!(
            breakdown(target() - total, target()),
            Breakdown { elapsed: false, days: 6, hours: 3, minutes: 12, seconds: 8, milliseconds: 250, total }
        );
    }

    #[test]
    fn breakdown_shows_one_second_until_the_final_millisecond_has_passed() {
        let t = target();
        assert_eq!(breakdown(t - 1000, t).seconds, 1);
        assert_eq!(breakdown(t - 999, t).seconds, 0);
        assert!(!breakdown(t - 1, t).elapsed);
    }

    #[test]
    fn breakdown_flips_to_elapsed_exactly_at_the_target_and_counts_upwards() {
        let t = target();
        let at_target = breakdown(t, t);
        assert!(at_target.elapsed);
        assert_eq!(at_target.total, 0);
        assert_eq!(at_target.seconds, 0);

        let later = breakdown(t + 2 * DAY + 90 * SECOND, t);
        assert!(later.elapsed);
        assert_eq!((later.days, later.hours, later.minutes, later.seconds), (2, 0, 1, 30));
    }

    #[test]
    fn breakdown_carries_more_than_99_days_without_wrapping() {
        assert_eq!(breakdown(target() - 400 * DAY, target()).days, 400);
    }

    #[test]
    fn ms_until_next_second_returns_the_distance_to_the_next_change_of_the_seconds_digit() {
        let t = target();
        // 5.250 s remain and "5" is showing; "4" appears once 4.999 s remain.
        assert_eq!(ms_until_next_second(t - 5_250, t), 251);
        assert_eq!(ms_until_next_second(t - 5_001, t), 2);
        assert_eq!(ms_until_next_second(t - 5_000, t), 1);
        assert_eq!(ms_until_next_second(t - 4_999, t), 1000);
    }

    #[test]
    fn ms_until_next_second_lands_exactly_on_the_target_for_the_final_change() {
        let t = target();
        assert_eq!(ms_until_next_second(t - 999, t), 999);
        assert_eq!(ms_until_next_second(t - 1, t), 1);
    }

    #[test]
    fn ms_until_next_second_keeps_ticking_in_phase_after_the_target() {
        let t = target();
        assert_eq!(ms_until_next_second(t, t), 1000);
        assert_eq!(ms_until_next_second(t + 250, t), 750);
        assert_eq!(ms_until_next_second(t + 999, t), 1);
    }

    #[test]
    fn ms_until_next_second_always_schedules_a_tick_between_1ms_and_1s_away() {
        let t = target();
        for offset in (-3000..=3000).step_by(37) {
            let wait = ms_until_next_second(t + offset, t);
            assert!((1..=1000).contains(&wait), "{offset}: {wait}");
        }
    }

    #[test]
    fn a_scheduled_tick_lands_on_a_new_seconds_value() {
        let t = target();
        for offset in [-5_250, -5_000, -1_000, -999, -1, 0, 1, 400, 1_999] {
            let now = t + offset;
            let wait = ms_until_next_second(now, t);
            let before = breakdown(now, t);
            let after = breakdown(now + wait, t);
            assert!(after.seconds != before.seconds || after.elapsed != before.elapsed, "{offset}");
            // One millisecond earlier the display must still be unchanged, so
            // the tick is neither late nor wasted.
            let just_before = breakdown(now + wait - 1, t);
            assert_eq!(just_before.seconds, before.seconds, "{offset}");
            assert_eq!(just_before.elapsed, before.elapsed, "{offset}");
        }
    }

    #[test]
    fn totals_express_the_remaining_time_in_single_units() {
        let now = target() - (6 * DAY + 3 * HOUR);
        let t = totals(now, target(), "UTC").unwrap();
        assert_eq!(t.hours, 147);
        assert_eq!(t.minutes, 147 * 60);
        assert_eq!(t.seconds, 147 * 3600);
        assert!((t.weeks - 0.875).abs() < 0.0005);
        assert_eq!(t.sleeps, 7);
        assert_eq!(t.weekends, 1);
    }

    #[test]
    fn totals_are_all_zeros_once_the_target_has_passed() {
        let t = totals(target() + HOUR, target(), "UTC").unwrap();
        assert_eq!(t, Totals { weeks: 0.0, hours: 0, minutes: 0, seconds: 0, sleeps: 0, weekends: 0 });
    }

    #[test]
    fn wait_progress_starts_at_the_first_trailer_and_ends_at_release() {
        let (t, g) = (target(), gta());
        assert_eq!(wait_origin(&g), Date::new(2023, 12, 4).noon_utc());
        assert_eq!(wait_progress(&g, wait_origin(&g), t), 0.0);
        assert_eq!(wait_progress(&g, t, t), 1.0);
        assert!((wait_progress(&g, (wait_origin(&g) + t) / 2, t) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn wait_progress_is_clamped_outside_the_interval() {
        let g = gta();
        assert_eq!(wait_progress(&g, wait_origin(&g) - DAY, target()), 0.0);
        assert_eq!(wait_progress(&g, target() + DAY, target()), 1.0);
    }

    #[test]
    fn timeline_places_milestones_in_chronological_order() {
        let g = gta();
        let positions: Vec<f64> =
            g.milestones.iter().map(|m| timeline_position(&g, m.date.noon_utc(), target())).collect();
        assert_eq!(positions[0], 0.0);
        for pair in positions.windows(2) {
            assert!(pair[1] > pair[0]);
            assert!(pair[1] < 1.0);
        }
    }

    #[test]
    fn every_game_resolves_to_midnight_on_its_date() {
        for game in games::built_in() {
            let t = release_instant(&game, "UTC").unwrap();
            assert_eq!(iso(t), format!("{}T00:00:00.000Z", game.date), "{}", game.id);
            assert!(wait_origin(&game) < t, "{}", game.id);
        }
    }

    #[test]
    fn a_custom_game_counts_from_the_day_it_was_added() {
        let custom = games::CustomGame::new("Mine", Date::new(2027, 1, 1), 18, 30, Date::new(2026, 12, 1)).unwrap();
        let game = custom.to_game();
        assert_eq!(iso(release_instant(&game, "UTC").unwrap()), "2027-01-01T18:30:00.000Z");
        assert_eq!(wait_origin(&game), Date::new(2026, 12, 1).noon_utc());
    }

    #[test]
    fn format_compact_pads_every_unit_except_days() {
        let now = target() - (6 * DAY + 3 * HOUR + 2 * MINUTE + 8 * SECOND);
        assert_eq!(format_compact(&breakdown(now, target())), "6d 03h 02m 08s");
    }

    #[test]
    fn wording_matches_the_web_version() {
        let t = release_instant(&gta(), "Europe/Zagreb").unwrap();
        assert_eq!(wording::long_date(t, "Europe/Zagreb").unwrap(), "Thursday, November 19, 2026");
        assert_eq!(wording::time(t, "Europe/Zagreb").unwrap(), "00:00");
        assert_eq!(wording::short_date_time(t, "America/New_York").unwrap(), "Wed, Nov 18, 18:00");
        assert_eq!(wording::zone_text("Europe/Zagreb", t).unwrap(), "Zagreb time, UTC+01:00");
        assert_eq!(wording::short_date(wait_origin(&gta())), "Dec 4, 2023");
        assert_eq!(wording::integer(12_723_000), "12,723,000");
        assert_eq!(wording::integer(7), "7");
        assert_eq!(wording::integer(-1234), "-1,234");
    }
}
