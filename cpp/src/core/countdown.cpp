#include "countdown.hpp"

#include <algorithm>
#include <format>

#include "release.hpp"
#include "zones.hpp"

namespace gtavi::countdown {

std::int64_t release_instant(std::string_view zone) {
    return zones::zoned_time_to_utc(release::year, release::month, release::day, release::hour, release::minute, zone);
}

Breakdown breakdown(std::int64_t now, std::int64_t target) {
    auto diff = target - now;
    auto total = diff < 0 ? -diff : diff;
    return {
        .elapsed = diff <= 0,
        .days = total / day,
        .hours = int(total % day / hour),
        .minutes = int(total % hour / minute),
        .seconds = int(total % minute / second),
        .milliseconds = int(total % second),
        .total = total,
    };
}

std::int64_t ms_until_next_second(std::int64_t now, std::int64_t target) {
    auto diff = target - now;
    if (diff <= 0) return second - (-diff % second);
    if (diff < second) return diff;
    return diff % second + 1;
}

Totals totals(std::int64_t now, std::int64_t target, std::string_view zone) {
    auto remaining = std::max<std::int64_t>(0, target - now);
    return {
        .weeks = double(remaining) / (7.0 * day),
        .hours = remaining / hour,
        .minutes = remaining / minute,
        .seconds = remaining / second,
        .sleeps = remaining == 0 ? 0 : std::max(0, zones::calendar_days_between(now, target, zone)),
        .weekends = remaining == 0 ? 0 : zones::weekends_between(now, target, zone),
    };
}

std::int64_t wait_origin() {
    static const std::int64_t origin = release::milestone_instant(release::milestones[0].date);
    return origin;
}

double wait_progress(std::int64_t now, std::int64_t target) { return timeline_position(now, target); }

double timeline_position(std::int64_t instant, std::int64_t target) {
    auto span = target - wait_origin();
    if (span <= 0) return 1;
    return std::clamp(double(instant - wait_origin()) / double(span), 0.0, 1.0);
}

std::string format_compact(const Breakdown& b) {
    return std::format("{}d {:02}h {:02}m {:02}s", b.days, b.hours, b.minutes, b.seconds);
}

}  // namespace gtavi::countdown
