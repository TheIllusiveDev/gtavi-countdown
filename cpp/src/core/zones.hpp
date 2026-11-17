#pragma once
// Time zone arithmetic on IANA identifiers, built on the C++20 <chrono> time
// zone database (the system tzdata). Instants are Unix milliseconds
// throughout, as in the rest of the app.

#include <cstdint>
#include <stdexcept>
#include <string>
#include <string_view>
#include <vector>

namespace gtavi {

// A wall-clock reading in some time zone, to the second. Month is 1-based.
struct WallTime {
    int year, month, day, hour, minute, second;
    bool operator==(const WallTime&) const = default;
};

namespace zones {

// Thrown for identifiers the time zone database does not know.
struct UnknownTimeZone : std::runtime_error {
    explicit UnknownTimeZone(std::string_view zone);
};

// The wall-clock reading in `zone` at the instant `utc_ms`.
WallTime wall_time_at(std::int64_t utc_ms, std::string_view zone);

// Offset of `zone` from UTC at `utc_ms`, in milliseconds. Positive east of Greenwich.
std::int64_t offset_at(std::int64_t utc_ms, std::string_view zone);

// Converts a wall-clock time in `zone` to the UTC instant it denotes. The
// offset is resolved in two passes so the result is correct when the first
// guess lands on the other side of a daylight-saving transition.
std::int64_t zoned_time_to_utc(int year, int month, int day, int hour, int minute, std::string_view zone);

// "UTC+05:30", "UTC-08:00" or "UTC".
std::string format_offset(std::int64_t offset_ms);

// The IANA zone the host system is configured for, or "UTC".
std::string system_time_zone();

bool is_valid_time_zone(std::string_view zone);

// Every geographic IANA zone the database knows, plus UTC first, falling back
// to a short curated list when the database cannot be enumerated.
std::vector<std::string> list_time_zones();

// "America/Argentina/Buenos_Aires" becomes "Buenos Aires".
std::string zone_city(std::string_view zone);

// "America/Argentina/Buenos_Aires" becomes "America / Argentina".
std::string zone_region(std::string_view zone);

// Whole calendar days from the date of `from_ms` to the date of `to_ms`, both
// read in `zone`: the number of midnights in between ("sleeps").
int calendar_days_between(std::int64_t from_ms, std::int64_t to_ms, std::string_view zone);

// Saturdays strictly after the date of `from_ms` and strictly before the date
// of `to_ms` in `zone`: the full weekends still to get through.
int weekends_between(std::int64_t from_ms, std::int64_t to_ms, std::string_view zone);

}  // namespace zones
}  // namespace gtavi
