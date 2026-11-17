#include "zones.hpp"

#include <algorithm>
#include <array>
#include <chrono>
#include <cmath>
#include <format>
#include <set>
#include <stdexcept>

namespace gtavi::zones {

using namespace std::chrono;

UnknownTimeZone::UnknownTimeZone(std::string_view zone)
    : std::runtime_error(std::format("Unknown time zone \"{}\".", zone)) {}

namespace {

// Continents and oceans that prefix geographic IANA zones. Anything else
// (the inverted "Etc/GMT+N" family, legacy aliases such as "US/Eastern" or
// bare abbreviations such as "EST") would confuse more than it helps in a picker.
constexpr std::array<std::string_view, 10> geographic_areas{
    "Africa/", "America/", "Antarctica/", "Arctic/", "Asia/", "Atlantic/",
    "Australia/", "Europe/", "Indian/", "Pacific/",
};

constexpr std::array<std::string_view, 17> curated_zones{
    "UTC", "Pacific/Auckland", "Australia/Sydney", "Asia/Tokyo", "Asia/Seoul", "Asia/Shanghai",
    "Asia/Kolkata", "Asia/Dubai", "Europe/Moscow", "Europe/Berlin", "Europe/London",
    "America/Sao_Paulo", "America/New_York", "America/Chicago", "America/Denver",
    "America/Los_Angeles", "Pacific/Honolulu",
};

const time_zone* lookup(std::string_view zone) noexcept {
    if (zone.empty()) return nullptr;
    try {
        return locate_zone(zone);
    } catch (const std::exception&) {
        return nullptr;
    }
}

const time_zone& resolve(std::string_view zone) {
    if (auto* tz = lookup(zone)) return *tz;
    throw UnknownTimeZone(zone);
}

sys_time<milliseconds> instant(std::int64_t utc_ms) { return sys_time<milliseconds>{milliseconds{utc_ms}}; }

}  // namespace

WallTime wall_time_at(std::int64_t utc_ms, std::string_view zone) {
    auto local = resolve(zone).to_local(instant(utc_ms));
    auto day_start = floor<days>(local);
    year_month_day ymd{day_start};
    hh_mm_ss hms{floor<seconds>(local - day_start)};
    return {int(ymd.year()), int(unsigned(ymd.month())), int(unsigned(ymd.day())),
            int(hms.hours().count()), int(hms.minutes().count()), int(hms.seconds().count())};
}

std::int64_t offset_at(std::int64_t utc_ms, std::string_view zone) {
    auto info = resolve(zone).get_info(instant(utc_ms));
    return duration_cast<milliseconds>(info.offset).count();
}

std::int64_t zoned_time_to_utc(int year, int month, int day, int hour, int minute, std::string_view zone) {
    auto wall_days = sys_days{std::chrono::year{year} / month / day};
    std::int64_t wall = duration_cast<milliseconds>((wall_days + hours{hour} + minutes{minute}).time_since_epoch()).count();
    auto first_guess = wall - offset_at(wall, zone);
    return wall - offset_at(first_guess, zone);
}

std::string format_offset(std::int64_t offset_ms) {
    if (offset_ms == 0) return "UTC";
    char sign = offset_ms < 0 ? '-' : '+';
    // Round half away from zero, as the other versions do.
    auto total_minutes = static_cast<std::int64_t>(std::llround(std::abs(offset_ms) / 60'000.0));
    return std::format("UTC{}{:02}:{:02}", sign, total_minutes / 60, total_minutes % 60);
}

std::string system_time_zone() {
    try {
        std::string id{current_zone()->name()};
        return is_valid_time_zone(id) ? id : "UTC";
    } catch (const std::exception&) {
        return "UTC";
    }
}

bool is_valid_time_zone(std::string_view zone) { return lookup(zone) != nullptr; }

std::vector<std::string> list_time_zones() {
    auto geographic = [](std::string_view id) {
        return std::ranges::any_of(geographic_areas, [&](std::string_view area) { return id.starts_with(area); });
    };
    try {
        std::set<std::string> found;
        const auto& db = get_tzdb();
        for (const auto& zone : db.zones)
            if (geographic(zone.name())) found.emplace(zone.name());
        // Links cover familiar names such as Europe/Zagreb that tzdata now
        // folds into another zone.
        for (const auto& link : db.links)
            if (geographic(link.name())) found.emplace(link.name());
        if (found.size() >= 10) {
            std::vector<std::string> result{"UTC"};
            result.insert(result.end(), found.begin(), found.end());
            return result;
        }
    } catch (const std::exception&) {
        // Fall through to the curated list.
    }
    std::vector<std::string> result;
    for (auto zone : curated_zones)
        if (is_valid_time_zone(zone)) result.emplace_back(zone);
    return result;
}

std::string zone_city(std::string_view zone) {
    auto cut = zone.rfind('/');
    std::string city{cut == std::string_view::npos ? zone : zone.substr(cut + 1)};
    std::ranges::replace(city, '_', ' ');
    return city;
}

std::string zone_region(std::string_view zone) {
    auto cut = zone.rfind('/');
    if (cut == std::string_view::npos) return "";
    std::string region;
    for (char c : zone.substr(0, cut)) {
        if (c == '/') region += " / ";
        else region += c == '_' ? ' ' : c;
    }
    return region;
}

namespace {
std::int64_t day_number(const WallTime& w) {
    return sys_days{std::chrono::year{w.year} / w.month / w.day}.time_since_epoch().count();
}
}  // namespace

int calendar_days_between(std::int64_t from_ms, std::int64_t to_ms, std::string_view zone) {
    return static_cast<int>(day_number(wall_time_at(to_ms, zone)) - day_number(wall_time_at(from_ms, zone)));
}

int weekends_between(std::int64_t from_ms, std::int64_t to_ms, std::string_view zone) {
    int days_between = calendar_days_between(from_ms, to_ms, zone);
    if (days_between <= 1) return 0;
    auto start = wall_time_at(from_ms, zone);
    // Day of week of the starting date: 0 is Sunday, 6 is Saturday.
    int start_dow = static_cast<int>(weekday{sys_days{std::chrono::year{start.year} / start.month / start.day}}.c_encoding());
    // Days until the first Saturday after the starting date (1 to 7).
    int until_saturday = (6 - start_dow + 7) % 7;
    if (until_saturday == 0) until_saturday = 7;
    if (until_saturday >= days_between) return 0;
    return 1 + (days_between - 1 - until_saturday) / 7;
}

}  // namespace gtavi::zones
