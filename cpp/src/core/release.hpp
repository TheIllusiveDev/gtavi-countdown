#pragma once
// Release facts. This is the single place to edit if the date ever moves.
//
// Rockstar Games has announced the calendar date. The unlock hour used here,
// midnight local time in each region, is the timing reported by the press at
// the time of writing; it is a wall-clock time, so the absolute instant
// depends on the time zone the countdown is evaluated in.

#include <array>
#include <cstdint>
#include <string_view>

namespace gtavi {

struct Milestone {
    std::string_view date;   // Calendar date (UTC) of the event, as YYYY-MM-DD.
    std::string_view title;
    std::string_view detail;
};

namespace release {

inline constexpr std::string_view title = "Grand Theft Auto VI";
inline constexpr int year = 2026;
inline constexpr int month = 11;  // 1-based
inline constexpr int day = 19;
inline constexpr int hour = 0;
inline constexpr int minute = 0;

inline constexpr std::array<std::string_view, 2> platforms{"PlayStation 5", "Xbox Series X|S"};

// Public milestones on the road to release, oldest first. The first entry is
// the origin of the progress bar; the release itself is added by the timeline
// because its instant depends on the selected time zone.
inline constexpr std::array<Milestone, 3> milestones{{
    {"2023-12-04", "Trailer 1", "First trailer published, with a 2025 release window."},
    {"2025-05-02", "First delay", "Release moved to May 26, 2026. Trailer 2 followed on May 6."},
    {"2025-11-06", "Second delay", "Release moved to November 19, 2026."},
}};

// Converts a YYYY-MM-DD milestone date to an instant at noon UTC, in Unix milliseconds.
std::int64_t milestone_instant(std::string_view date);

}  // namespace release
}  // namespace gtavi
