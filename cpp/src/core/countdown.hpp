#pragma once
// Countdown maths. Instants and durations are milliseconds.

#include <cstdint>
#include <string>
#include <string_view>

namespace gtavi {

// The distance to the target split into display units.
struct Breakdown {
    bool elapsed;  // True once the target has passed; the parts then count upwards.
    std::int64_t days;
    int hours, minutes, seconds, milliseconds;
    std::int64_t total;  // Absolute distance to the target.
    bool operator==(const Breakdown&) const = default;
};

// Alternative single-unit readings of the remaining time.
struct Totals {
    double weeks;
    std::int64_t hours, minutes, seconds;
    int sleeps;    // Midnights between now and the target in the target zone.
    int weekends;  // Full weekends (Saturdays) between now and the target.
};

namespace countdown {

inline constexpr std::int64_t second = 1000;
inline constexpr std::int64_t minute = 60 * second;
inline constexpr std::int64_t hour = 60 * minute;
inline constexpr std::int64_t day = 24 * hour;

// The instant the game unlocks for someone whose clock reads `zone`.
std::int64_t release_instant(std::string_view zone);

Breakdown breakdown(std::int64_t now, std::int64_t target);

// Milliseconds until the display changes next. Scheduling the tick on this
// boundary keeps the display in phase with the target instead of with the
// moment the app happened to start. Before the target the seconds are floored,
// so the reading drops one millisecond after each whole second of remaining
// time; after it, the reading rises on each whole second of elapsed time.
std::int64_t ms_until_next_second(std::int64_t now, std::int64_t target);

Totals totals(std::int64_t now, std::int64_t target, std::string_view zone);

// The instant the wait began: the publication of the first trailer.
std::int64_t wait_origin();

// Fraction of the wait since the first trailer that is over, from 0 to 1.
double wait_progress(std::int64_t now, std::int64_t target);

// Position of an instant on the wait timeline, from 0 to 1.
double timeline_position(std::int64_t instant, std::int64_t target);

// Compact one-line reading such as "6d 03h 12m 08s".
std::string format_compact(const Breakdown& b);

}  // namespace countdown
}  // namespace gtavi
