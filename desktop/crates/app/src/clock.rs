//! The once-per-second clock. It holds no running count: every tick reads the
//! system clock again, so a late frame or a machine waking from sleep can
//! delay an update but never make the value shown wrong.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gtavi_core::countdown;

pub struct Clock {
    /// Milliseconds added to the system clock, from the network time measurement.
    offset_ms: f64,
    target: i64,
    /// The instant the display shows; it changes only on ticks.
    now: i64,
    /// When the display next changes, on the corrected clock.
    next_tick: i64,
    /// Debug builds can start the clock at another time, to try the release
    /// moment out: `GTAVI_COUNTDOWN_DEBUG_NOW` takes Unix milliseconds.
    shift_ms: i64,
}

fn system_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

#[cfg(debug_assertions)]
fn debug_shift() -> i64 {
    std::env::var("GTAVI_COUNTDOWN_DEBUG_NOW")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .map_or(0, |now| now - system_ms())
}

#[cfg(not(debug_assertions))]
fn debug_shift() -> i64 {
    0
}

impl Clock {
    pub fn new(target: i64) -> Self {
        let mut clock = Clock { offset_ms: 0.0, target, now: 0, next_tick: 0, shift_ms: debug_shift() };
        clock.resync();
        clock
    }

    /// The corrected time to the millisecond, for the per-frame readout.
    pub fn precise(&self) -> i64 {
        system_ms() + self.offset_ms.round() as i64 + self.shift_ms
    }

    /// The time the display shows.
    pub fn now(&self) -> i64 {
        self.now
    }

    pub fn retarget(&mut self, target: i64) {
        self.target = target;
        self.resync();
    }

    pub fn set_offset(&mut self, offset_ms: f64) {
        self.offset_ms = offset_ms;
        self.resync();
    }

    /// Reads the clock again and schedules the next tick on the boundary where
    /// the seconds value changes, so the display stays in phase with the target.
    pub fn resync(&mut self) {
        self.now = self.precise();
        self.next_tick = self.now + countdown::ms_until_next_second(self.now, self.target);
    }

    /// Advances the display when its next change is due. Returns whether it
    /// did. A clock that jumped backwards also counts as a change.
    pub fn poll(&mut self) -> bool {
        let precise = self.precise();
        if precise >= self.next_tick || precise < self.now {
            self.resync();
            true
        } else {
            false
        }
    }

    /// How long until the display changes next.
    pub fn until_next_tick(&self) -> Duration {
        Duration::from_millis((self.next_tick - self.precise()).clamp(0, 1000) as u64)
    }
}
