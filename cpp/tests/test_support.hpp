#pragma once

#include <chrono>
#include <cstdint>
#include <format>
#include <string>

namespace gtavi::test {

inline std::int64_t utc(int year, int month, int day, int hour = 0, int minute = 0, int second = 0, int ms = 0) {
    using namespace std::chrono;
    auto t = sys_days{std::chrono::year{year} / month / day} + hours{hour} + minutes{minute} + seconds{second} +
             milliseconds{ms};
    return duration_cast<milliseconds>(t.time_since_epoch()).count();
}

inline std::string iso(std::int64_t ms) {
    using namespace std::chrono;
    return std::format("{:%Y-%m-%dT%H:%M:%S}Z", sys_time<milliseconds>{milliseconds{ms}});
}

}  // namespace gtavi::test
