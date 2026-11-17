#pragma once
// The human-readable dates and times shown around the countdown. Names are
// English and formatting does not depend on the host's locale.

#include <cstdint>
#include <string>
#include <string_view>

namespace gtavi::wording {

std::string long_date(std::int64_t utc_ms, std::string_view zone);       // Thursday, November 19, 2026
std::string time(std::int64_t utc_ms, std::string_view zone);            // 00:00
std::string short_date_time(std::int64_t utc_ms, std::string_view zone); // Wed, Nov 18, 18:00
std::string short_date(std::int64_t utc_ms);                             // Dec 4, 2023 (read in UTC)
std::string zone_text(std::string_view zone, std::int64_t at);           // Zagreb time, UTC+01:00
std::string integer(std::int64_t value);                                 // 12,345

}  // namespace gtavi::wording
