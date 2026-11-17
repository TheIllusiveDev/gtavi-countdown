#pragma once
// Minimal iCalendar (RFC 5545) writer for a single event with reminders.

#include <cstdint>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

namespace gtavi {

struct CalendarEvent {
    std::string uid;
    std::int64_t start;  // UTC instant in Unix milliseconds.
    int duration_minutes;
    std::string summary;
    std::string description;
    std::vector<int> reminders;          // Minutes before the start.
    std::optional<std::int64_t> stamp;  // Creation stamp; defaults to now.
};

namespace ics {

// 2026-11-18T23:00:00.000Z becomes 20261118T230000Z.
std::string date(std::int64_t utc_ms);

// Escapes TEXT values as required by RFC 5545 section 3.3.11.
std::string escape(std::string_view text);

// Folds a content line so that no physical line exceeds 75 octets, never
// splitting a UTF-8 sequence.
std::string fold(std::string_view line);

std::string build_calendar(const CalendarEvent& event);

}  // namespace ics
}  // namespace gtavi
