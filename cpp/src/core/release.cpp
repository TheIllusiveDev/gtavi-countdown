#include "release.hpp"

#include <charconv>
#include <chrono>
#include <stdexcept>

namespace gtavi::release {

namespace {
int field(std::string_view text) {
    int value = 0;
    auto [end, ec] = std::from_chars(text.data(), text.data() + text.size(), value);
    if (ec != std::errc{} || end != text.data() + text.size()) throw std::invalid_argument("bad milestone date");
    return value;
}
}  // namespace

std::int64_t milestone_instant(std::string_view date) {
    using namespace std::chrono;
    if (date.size() != 10 || date[4] != '-' || date[7] != '-') throw std::invalid_argument("bad milestone date");
    year_month_day ymd{std::chrono::year{field(date.substr(0, 4))}, std::chrono::month{unsigned(field(date.substr(5, 2)))},
                       std::chrono::day{unsigned(field(date.substr(8, 2)))}};
    if (!ymd.ok()) throw std::invalid_argument("bad milestone date");
    auto noon = sys_days{ymd} + hours{12};
    return duration_cast<milliseconds>(noon.time_since_epoch()).count();
}

}  // namespace gtavi::release
