#include "wording.hpp"

#include <array>
#include <chrono>
#include <format>

#include "zones.hpp"

namespace gtavi::wording {

namespace {

constexpr std::array<std::string_view, 7> weekdays{"Sunday", "Monday", "Tuesday", "Wednesday",
                                                   "Thursday", "Friday", "Saturday"};
constexpr std::array<std::string_view, 12> months{"January", "February", "March", "April", "May", "June", "July",
                                                  "August", "September", "October", "November", "December"};

std::string_view weekday_name(const WallTime& w) {
    using namespace std::chrono;
    return weekdays[weekday{sys_days{year{w.year} / w.month / w.day}}.c_encoding()];
}

std::string_view month_name(const WallTime& w) { return months[w.month - 1]; }

}  // namespace

std::string long_date(std::int64_t utc_ms, std::string_view zone) {
    auto w = zones::wall_time_at(utc_ms, zone);
    return std::format("{}, {} {}, {}", weekday_name(w), month_name(w), w.day, w.year);
}

std::string time(std::int64_t utc_ms, std::string_view zone) {
    auto w = zones::wall_time_at(utc_ms, zone);
    return std::format("{:02}:{:02}", w.hour, w.minute);
}

std::string short_date_time(std::int64_t utc_ms, std::string_view zone) {
    auto w = zones::wall_time_at(utc_ms, zone);
    return std::format("{}, {} {}, {:02}:{:02}", weekday_name(w).substr(0, 3), month_name(w).substr(0, 3), w.day,
                       w.hour, w.minute);
}

std::string short_date(std::int64_t utc_ms) {
    auto w = zones::wall_time_at(utc_ms, "UTC");
    return std::format("{} {}, {}", month_name(w).substr(0, 3), w.day, w.year);
}

std::string zone_text(std::string_view zone, std::int64_t at) {
    return std::format("{} time, {}", zones::zone_city(zone), zones::format_offset(zones::offset_at(at, zone)));
}

std::string integer(std::int64_t value) {
    auto digits = std::to_string(value < 0 ? -static_cast<std::uint64_t>(value) : static_cast<std::uint64_t>(value));
    std::string out;
    for (std::size_t i = 0; i < digits.size(); ++i) {
        if (i > 0 && (digits.size() - i) % 3 == 0) out += ',';
        out += digits[i];
    }
    return value < 0 ? "-" + out : out;
}

}  // namespace gtavi::wording
