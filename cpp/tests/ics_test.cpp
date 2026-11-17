#include <gtest/gtest.h>

#include <algorithm>
#include <regex>

#include "ics.hpp"
#include "test_support.hpp"

using namespace gtavi;
using gtavi::test::utc;

namespace {

std::vector<std::string> split_crlf(const std::string& text) {
    std::vector<std::string> out;
    std::size_t start = 0;
    while (true) {
        auto cut = text.find("\r\n", start);
        if (cut == std::string::npos) {
            out.push_back(text.substr(start));
            return out;
        }
        out.push_back(text.substr(start, cut - start));
        start = cut + 2;
    }
}

std::string unfold(const std::vector<std::string>& lines) {
    std::string out;
    for (std::size_t i = 0; i < lines.size(); ++i) out += i == 0 ? lines[i] : lines[i].substr(1);
    return out;
}

bool contains(const std::vector<std::string>& lines, std::string_view line) {
    return std::ranges::find(lines, line) != lines.end();
}

long count(const std::vector<std::string>& lines, auto predicate) { return std::ranges::count_if(lines, predicate); }

}  // namespace

TEST(Ics, DateFormatsAnInstantAsAUtcDateTime) {
    EXPECT_EQ(ics::date(utc(2026, 11, 18, 23, 0, 0)), "20261118T230000Z");
    EXPECT_EQ(ics::date(utc(2026, 1, 2, 3, 4, 5, 678)), "20260102T030405Z");
}

TEST(Ics, EscapeEscapesTheCharactersRfc5545ReservesInText) {
    EXPECT_EQ(ics::escape("PS5, Xbox; both"), "PS5\\, Xbox\\; both");
    EXPECT_EQ(ics::escape("a\\b"), "a\\\\b");
    EXPECT_EQ(ics::escape("line one\nline two\r\nline three"), "line one\\nline two\\nline three");
}

TEST(Ics, FoldLeavesShortLinesAlone) {
    EXPECT_EQ(ics::fold("SUMMARY:Short"), "SUMMARY:Short");
    EXPECT_EQ(ics::fold(std::string(75, 'X')), std::string(75, 'X'));
}

TEST(Ics, FoldFoldsLongLinesAt75OctetsWithALeadingSpace) {
    auto original = "DESCRIPTION:" + std::string(200, 'a');
    auto lines = split_crlf(ics::fold(original));
    EXPECT_GT(lines.size(), 1u);
    for (const auto& line : lines) EXPECT_LE(line.size(), 75u);
    for (std::size_t i = 1; i < lines.size(); ++i) EXPECT_TRUE(lines[i].starts_with(' '));
    // Unfolding restores the original content line.
    EXPECT_EQ(unfold(lines), original);
}

TEST(Ics, FoldNeverSplitsAMultiByteCharacter) {
    std::string original = "SUMMARY:";
    for (int i = 0; i < 30; ++i) original += "é中\U0001F3AE";
    auto lines = split_crlf(ics::fold(original));
    for (const auto& line : lines) {
        EXPECT_LE(line.size(), 75u);
        // No line may end on a lead byte or start with a continuation byte.
        auto body = std::string_view{line}.substr(line.starts_with(' ') ? 1 : 0);
        ASSERT_FALSE(body.empty());
        EXPECT_NE(static_cast<unsigned char>(body.front()) & 0xC0, 0x80);
    }
    EXPECT_EQ(unfold(lines), original);
}

namespace {
const std::string calendar = ics::build_calendar({
    .uid = "release-1@gtavi-countdown",
    .start = utc(2026, 11, 18, 23, 0, 0),
    .duration_minutes = 60,
    .summary = "Grand Theft Auto VI release",
    .description = "Platforms: PlayStation 5, Xbox Series X|S.",
    .reminders = {1440, 60},
    .stamp = utc(2026, 11, 12, 19, 0, 0),
});
const std::vector<std::string> lines = split_crlf(calendar);
}  // namespace

TEST(Ics, BuildCalendarProducesAWellFormedCalendarObject) {
    EXPECT_EQ(lines.front(), "BEGIN:VCALENDAR");
    EXPECT_EQ(lines[lines.size() - 2], "END:VCALENDAR");
    EXPECT_EQ(lines.back(), "");
    EXPECT_FALSE(std::regex_search(calendar, std::regex("[^\r]\n")));
    EXPECT_TRUE(contains(lines, "VERSION:2.0"));
    EXPECT_TRUE(contains(lines, "UID:release-1@gtavi-countdown"));
    EXPECT_TRUE(contains(lines, "DTSTAMP:20261112T190000Z"));
    EXPECT_TRUE(contains(lines, "DTSTART:20261118T230000Z"));
    EXPECT_TRUE(contains(lines, "DTEND:20261119T000000Z"));
    EXPECT_TRUE(contains(lines, "SUMMARY:Grand Theft Auto VI release"));
    EXPECT_TRUE(contains(lines, "DESCRIPTION:Platforms: PlayStation 5\\, Xbox Series X|S."));
}

TEST(Ics, BuildCalendarAddsOneAlarmPerReminder) {
    EXPECT_EQ(count(lines, [](const std::string& l) { return l == "BEGIN:VALARM"; }), 2);
    EXPECT_EQ(count(lines, [](const std::string& l) { return l == "END:VALARM"; }), 2);
    EXPECT_TRUE(contains(lines, "TRIGGER:-PT1440M"));
    EXPECT_TRUE(contains(lines, "TRIGGER:-PT60M"));
}

TEST(Ics, BuildCalendarBalancesEveryBeginWithAnEnd) {
    EXPECT_EQ(count(lines, [](const std::string& l) { return l.starts_with("BEGIN:"); }),
              count(lines, [](const std::string& l) { return l.starts_with("END:"); }));
}
