#include <gtest/gtest.h>

#include "calendar_file.hpp"

using namespace gtavi;
using namespace std::string_view_literals;

namespace {
constexpr auto calendar = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nEND:VCALENDAR\r\n"sv;
}

TEST(CalendarFile, AcceptsAPlainCalendarFile) {
    EXPECT_EQ(calendar_file::validate("gta-vi-release.ics", calendar), "gta-vi-release.ics");
    EXPECT_EQ(calendar_file::validate("  Event_1.ICS ", calendar), "Event_1.ICS");
}

class BadName : public testing::TestWithParam<std::string_view> {};

TEST_P(BadName, IsRejected) { EXPECT_THROW(calendar_file::validate(GetParam(), calendar), std::invalid_argument); }

INSTANTIATE_TEST_SUITE_P(CalendarFile, BadName,
                         testing::Values(""sv, "../evil.ics"sv, "/etc/cron.d/evil.ics"sv, "..\\evil.ics"sv,
                                         "sub/dir.ics"sv, ".hidden.ics"sv, "notes.txt"sv, "run.ics.sh"sv,
                                         "nul\0.ics"sv, "café.ics"sv));

TEST(CalendarFile, RejectsOverlongNames) {
    EXPECT_THROW(calendar_file::validate(std::string(80, 'a') + ".ics", calendar), std::invalid_argument);
}

TEST(CalendarFile, RejectsContentThatIsNotACalendar) {
    EXPECT_THROW(calendar_file::validate("a.ics", "#!/bin/sh\nrm -rf ~\n"), std::invalid_argument);
    EXPECT_THROW(calendar_file::validate("a.ics", "BEGIN:VCALENDAR\r\n"), std::invalid_argument);
    auto huge = "BEGIN:VCALENDAR\r\n" + std::string(calendar_file::max_bytes, 'X') + "END:VCALENDAR";
    EXPECT_THROW(calendar_file::validate("a.ics", huge), std::invalid_argument);
}
