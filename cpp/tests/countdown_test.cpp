#include <gtest/gtest.h>

#include "countdown.hpp"
#include "release.hpp"
#include "test_support.hpp"
#include "wording.hpp"

using namespace gtavi;
using namespace gtavi::countdown;
using gtavi::test::iso;

namespace {
const std::int64_t target = release_instant("UTC");
}

TEST(Countdown, ReleaseInstantIsMidnightOn19NovemberInTheGivenZone) {
    EXPECT_EQ(iso(target), "2026-11-19T00:00:00.000Z");
    EXPECT_EQ(iso(release_instant("Europe/Zagreb")), "2026-11-18T23:00:00.000Z");
}

TEST(Countdown, ReleaseInstantMovesWestwardsWithTheSun) {
    EXPECT_LT(release_instant("Pacific/Auckland"), release_instant("Asia/Tokyo"));
    EXPECT_LT(release_instant("Asia/Tokyo"), release_instant("Europe/London"));
    EXPECT_LT(release_instant("Europe/London"), release_instant("America/Los_Angeles"));
}

TEST(Countdown, BreakdownSplitsTheRemainingTimeIntoUnits) {
    auto total = 6 * day + 3 * hour + 12 * minute + 8 * second + 250;
    EXPECT_EQ(breakdown(target - total, target), (Breakdown{false, 6, 3, 12, 8, 250, total}));
}

TEST(Countdown, BreakdownShowsOneSecondUntilTheFinalMillisecondHasPassed) {
    EXPECT_EQ(breakdown(target - 1000, target).seconds, 1);
    EXPECT_EQ(breakdown(target - 999, target).seconds, 0);
    EXPECT_FALSE(breakdown(target - 1, target).elapsed);
}

TEST(Countdown, BreakdownFlipsToElapsedExactlyAtTheTargetAndCountsUpwards) {
    auto at_target = breakdown(target, target);
    EXPECT_TRUE(at_target.elapsed);
    EXPECT_EQ(at_target.total, 0);
    EXPECT_EQ(at_target.seconds, 0);

    auto later = breakdown(target + 2 * day + 90 * second, target);
    EXPECT_TRUE(later.elapsed);
    EXPECT_EQ(later.days, 2);
    EXPECT_EQ(later.hours, 0);
    EXPECT_EQ(later.minutes, 1);
    EXPECT_EQ(later.seconds, 30);
}

TEST(Countdown, BreakdownCarriesMoreThan99DaysWithoutWrapping) {
    EXPECT_EQ(breakdown(target - 400 * day, target).days, 400);
}

TEST(Countdown, MsUntilNextSecondReturnsTheDistanceToTheNextChangeOfTheSecondsDigit) {
    // 5.250 s remain and "5" is showing; "4" appears once 4.999 s remain.
    EXPECT_EQ(ms_until_next_second(target - 5'250, target), 251);
    EXPECT_EQ(ms_until_next_second(target - 5'001, target), 2);
    EXPECT_EQ(ms_until_next_second(target - 5'000, target), 1);
    EXPECT_EQ(ms_until_next_second(target - 4'999, target), 1000);
}

TEST(Countdown, MsUntilNextSecondLandsExactlyOnTheTargetForTheFinalChange) {
    EXPECT_EQ(ms_until_next_second(target - 999, target), 999);
    EXPECT_EQ(ms_until_next_second(target - 1, target), 1);
}

TEST(Countdown, MsUntilNextSecondKeepsTickingInPhaseAfterTheTarget) {
    EXPECT_EQ(ms_until_next_second(target, target), 1000);
    EXPECT_EQ(ms_until_next_second(target + 250, target), 750);
    EXPECT_EQ(ms_until_next_second(target + 999, target), 1);
}

TEST(Countdown, MsUntilNextSecondAlwaysSchedulesATickBetween1msAnd1sAway) {
    for (int offset = -3000; offset <= 3000; offset += 37) {
        auto wait = ms_until_next_second(target + offset, target);
        EXPECT_GE(wait, 1) << offset;
        EXPECT_LE(wait, 1000) << offset;
    }
}

class ScheduledTick : public testing::TestWithParam<std::int64_t> {};

TEST_P(ScheduledTick, LandsOnANewSecondsValue) {
    auto now = target + GetParam();
    auto wait = ms_until_next_second(now, target);
    auto before = breakdown(now, target);
    auto after = breakdown(now + wait, target);
    EXPECT_TRUE(after.seconds != before.seconds || after.elapsed != before.elapsed);
    // One millisecond earlier the display must still be unchanged, so the tick
    // is neither late nor wasted.
    auto just_before = breakdown(now + wait - 1, target);
    EXPECT_EQ(just_before.seconds, before.seconds);
    EXPECT_EQ(just_before.elapsed, before.elapsed);
}

INSTANTIATE_TEST_SUITE_P(Countdown, ScheduledTick,
                         testing::Values(-5'250, -5'000, -1'000, -999, -1, 0, 1, 400, 1'999));

TEST(Countdown, TotalsExpressTheRemainingTimeInSingleUnits) {
    auto now = target - (6 * day + 3 * hour);
    auto t = totals(now, target, "UTC");
    EXPECT_EQ(t.hours, 147);
    EXPECT_EQ(t.minutes, 147 * 60);
    EXPECT_EQ(t.seconds, 147 * 3600);
    EXPECT_NEAR(t.weeks, 0.875, 0.0005);
    EXPECT_EQ(t.sleeps, 7);
    EXPECT_EQ(t.weekends, 1);
}

TEST(Countdown, TotalsAreAllZerosOnceTheTargetHasPassed) {
    auto t = totals(target + hour, target, "UTC");
    EXPECT_EQ(t.weeks, 0);
    EXPECT_EQ(t.hours, 0);
    EXPECT_EQ(t.minutes, 0);
    EXPECT_EQ(t.seconds, 0);
    EXPECT_EQ(t.sleeps, 0);
    EXPECT_EQ(t.weekends, 0);
}

TEST(Countdown, WaitProgressStartsAtTheFirstTrailerAndEndsAtRelease) {
    EXPECT_EQ(wait_origin(), release::milestone_instant(release::milestones[0].date));
    EXPECT_EQ(wait_progress(wait_origin(), target), 0);
    EXPECT_EQ(wait_progress(target, target), 1);
    EXPECT_NEAR(wait_progress((wait_origin() + target) / 2, target), 0.5, 1e-9);
}

TEST(Countdown, WaitProgressIsClampedOutsideTheInterval) {
    EXPECT_EQ(wait_progress(wait_origin() - day, target), 0);
    EXPECT_EQ(wait_progress(target + day, target), 1);
}

TEST(Countdown, TimelinePlacesMilestonesInChronologicalOrder) {
    std::vector<double> positions;
    for (const auto& m : release::milestones) positions.push_back(timeline_position(release::milestone_instant(m.date), target));
    EXPECT_EQ(positions[0], 0);
    for (std::size_t i = 1; i < positions.size(); ++i) {
        EXPECT_GT(positions[i], positions[i - 1]);
        EXPECT_LT(positions[i], 1);
    }
}

TEST(Countdown, FormatCompactPadsEveryUnitExceptDays) {
    auto now = target - (6 * day + 3 * hour + 2 * minute + 8 * second);
    EXPECT_EQ(format_compact(breakdown(now, target)), "6d 03h 02m 08s");
}

TEST(Countdown, WordingMatchesTheWebVersion) {
    auto t = release_instant("Europe/Zagreb");
    EXPECT_EQ(wording::long_date(t, "Europe/Zagreb"), "Thursday, November 19, 2026");
    EXPECT_EQ(wording::time(t, "Europe/Zagreb"), "00:00");
    EXPECT_EQ(wording::short_date_time(t, "America/New_York"), "Wed, Nov 18, 18:00");
    EXPECT_EQ(wording::zone_text("Europe/Zagreb", t), "Zagreb time, UTC+01:00");
    EXPECT_EQ(wording::short_date(wait_origin()), "Dec 4, 2023");
    EXPECT_EQ(wording::integer(12'723'000), "12,723,000");
    EXPECT_EQ(wording::integer(7), "7");
    EXPECT_EQ(wording::integer(-1234), "-1,234");
}
