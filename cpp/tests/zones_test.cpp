#include <gtest/gtest.h>

#include <algorithm>
#include <set>

#include "test_support.hpp"
#include "zones.hpp"

using namespace gtavi;
using namespace gtavi::zones;
using gtavi::test::iso;
using gtavi::test::utc;

namespace {
constexpr std::int64_t hour_ms = 3'600'000;
}

TEST(Zones, OffsetAtReturnsZeroForUtc) { EXPECT_EQ(offset_at(utc(2026, 11, 19), "UTC"), 0); }

TEST(Zones, OffsetAtFollowsDaylightSavingInEurope) {
    EXPECT_EQ(offset_at(utc(2026, 11, 19), "Europe/Zagreb"), 1 * hour_ms);
    EXPECT_EQ(offset_at(utc(2026, 7, 1), "Europe/Zagreb"), 2 * hour_ms);
}

TEST(Zones, OffsetAtHandlesZonesWestOfGreenwichAndFractionalOffsets) {
    EXPECT_EQ(offset_at(utc(2026, 11, 19), "America/New_York"), -5 * hour_ms);
    EXPECT_EQ(offset_at(utc(2026, 11, 19), "Asia/Kolkata"), hour_ms * 11 / 2);
    EXPECT_EQ(offset_at(utc(2026, 11, 19), "Pacific/Chatham"), hour_ms * 55 / 4);
}

TEST(Zones, OffsetAtIgnoresTheSubSecondPartOfTheInstant) {
    EXPECT_EQ(offset_at(utc(2026, 11, 19, 0, 0, 0, 999), "Asia/Tokyo"), 9 * hour_ms);
}

TEST(Zones, UnknownZonesThrow) { EXPECT_THROW(offset_at(0, "Mars/Olympus_Mons"), UnknownTimeZone); }

struct MidnightCase {
    const char* zone;
    const char* expected;
};

class ReleaseMidnight : public testing::TestWithParam<MidnightCase> {};

TEST_P(ReleaseMidnight, ResolvesAroundTheWorld) {
    EXPECT_EQ(iso(zoned_time_to_utc(2026, 11, 19, 0, 0, GetParam().zone)), GetParam().expected);
}

INSTANTIATE_TEST_SUITE_P(Zones, ReleaseMidnight,
                         testing::Values(MidnightCase{"UTC", "2026-11-19T00:00:00.000Z"},
                                         MidnightCase{"Europe/London", "2026-11-19T00:00:00.000Z"},
                                         MidnightCase{"Europe/Zagreb", "2026-11-18T23:00:00.000Z"},
                                         MidnightCase{"Pacific/Auckland", "2026-11-18T11:00:00.000Z"},
                                         MidnightCase{"Asia/Tokyo", "2026-11-18T15:00:00.000Z"},
                                         MidnightCase{"Asia/Kolkata", "2026-11-18T18:30:00.000Z"},
                                         MidnightCase{"America/New_York", "2026-11-19T05:00:00.000Z"},
                                         MidnightCase{"America/Los_Angeles", "2026-11-19T08:00:00.000Z"}));

TEST(Zones, ZonedTimeToUtcIsCorrectOnBothSidesOfADaylightSavingChange) {
    // Europe leaves summer time on 25 October 2026 at 01:00 UTC.
    EXPECT_EQ(iso(zoned_time_to_utc(2026, 10, 25, 0, 30, "Europe/Berlin")), "2026-10-24T22:30:00.000Z");
    EXPECT_EQ(iso(zoned_time_to_utc(2026, 10, 25, 12, 0, "Europe/Berlin")), "2026-10-25T11:00:00.000Z");
}

class RoundTrip : public testing::TestWithParam<const char*> {};

TEST_P(RoundTrip, ThroughWallTimeAt) {
    auto instant = zoned_time_to_utc(2026, 11, 19, 0, 0, GetParam());
    EXPECT_EQ(wall_time_at(instant, GetParam()), (WallTime{2026, 11, 19, 0, 0, 0}));
}

INSTANTIATE_TEST_SUITE_P(Zones, RoundTrip,
                         testing::Values("Australia/Sydney", "America/Sao_Paulo", "Asia/Kathmandu", "Pacific/Honolulu"));

TEST(Zones, FormatOffsetFormatsPositiveNegativeFractionalAndZeroOffsets) {
    EXPECT_EQ(format_offset(0), "UTC");
    EXPECT_EQ(format_offset(1 * hour_ms), "UTC+01:00");
    EXPECT_EQ(format_offset(-8 * hour_ms), "UTC-08:00");
    EXPECT_EQ(format_offset(hour_ms * 11 / 2), "UTC+05:30");
    EXPECT_EQ(format_offset(-hour_ms * 7 / 2), "UTC-03:30");
    EXPECT_EQ(format_offset(14 * hour_ms), "UTC+14:00");
}

TEST(Zones, ZoneNamesAreValidated) {
    EXPECT_TRUE(is_valid_time_zone("Europe/Zagreb"));
    EXPECT_FALSE(is_valid_time_zone("Mars/Olympus_Mons"));
    EXPECT_FALSE(is_valid_time_zone(""));
}

TEST(Zones, SystemTimeZoneIsValid) { EXPECT_TRUE(is_valid_time_zone(system_time_zone())); }

TEST(Zones, ListTimeZonesIncludesUtc) {
    auto zones = list_time_zones();
    EXPECT_GT(zones.size(), 10u);
    EXPECT_EQ(zones.front(), "UTC");
    for (const auto& zone : zones) EXPECT_TRUE(is_valid_time_zone(zone)) << zone;
}

TEST(Zones, ListTimeZonesListsGeographicZonesOnlyWithoutDuplicates) {
    auto zones = list_time_zones();
    for (const auto& zone : zones) {
        EXPECT_FALSE(zone.starts_with("Etc/")) << zone;
        EXPECT_TRUE(zone == "UTC" || zone.contains('/')) << zone;
    }
    EXPECT_EQ(std::set<std::string>(zones.begin(), zones.end()).size(), zones.size());
    EXPECT_NE(std::ranges::find(zones, "Pacific/Auckland"), zones.end());
    EXPECT_NE(std::ranges::find(zones, "America/Los_Angeles"), zones.end());
    EXPECT_NE(std::ranges::find(zones, "Europe/Zagreb"), zones.end());
}

TEST(Zones, ZoneNamesSplitIntoCityAndRegion) {
    EXPECT_EQ(zone_city("America/Argentina/Buenos_Aires"), "Buenos Aires");
    EXPECT_EQ(zone_region("America/Argentina/Buenos_Aires"), "America / Argentina");
    EXPECT_EQ(zone_city("UTC"), "UTC");
    EXPECT_EQ(zone_region("UTC"), "");
}

namespace {
constexpr const char* zone = "Europe/Zagreb";
const std::int64_t release_at = zoned_time_to_utc(2026, 11, 19, 0, 0, zone);
std::int64_t at(int day, int hour = 12) { return zoned_time_to_utc(2026, 11, day, hour, 0, zone); }
}  // namespace

TEST(Zones, CalendarDaysBetweenCountsSleepsAsMidnightsInTheTargetZone) {
    // Thursday evening, a week before release: seven nights to go.
    EXPECT_EQ(calendar_days_between(zoned_time_to_utc(2026, 11, 12, 20, 30, zone), release_at, zone), 7);
    // One minute past midnight on the 18th: still one night.
    EXPECT_EQ(calendar_days_between(zoned_time_to_utc(2026, 11, 18, 0, 1, zone), release_at, zone), 1);
    EXPECT_EQ(calendar_days_between(zoned_time_to_utc(2026, 11, 18, 23, 59, zone), release_at, zone), 1);
}

TEST(Zones, CalendarDaysBetweenCountsAcrossADaylightSavingChangeWithoutDrifting) {
    EXPECT_EQ(calendar_days_between(zoned_time_to_utc(2026, 10, 20, 23, 0, zone), release_at, zone), 30);
}

TEST(Zones, WeekendsBetweenCountsTheSaturdaysStrictlyBetweenTwoDates) {
    // 12 November 2026 is a Thursday; the 14th is the only Saturday before the 19th.
    EXPECT_EQ(weekends_between(at(12), release_at, zone), 1);
    EXPECT_EQ(weekends_between(at(13), release_at, zone), 1);
    // On the Saturday itself the weekend has begun and no longer counts.
    EXPECT_EQ(weekends_between(at(14), release_at, zone), 0);
    EXPECT_EQ(weekends_between(at(18), release_at, zone), 0);
    // From Sunday 1 November: the 7th and the 14th.
    EXPECT_EQ(weekends_between(at(1), release_at, zone), 2);
    // From Saturday 31 October: the 7th and the 14th.
    EXPECT_EQ(weekends_between(zoned_time_to_utc(2026, 10, 31, 12, 0, zone), release_at, zone), 2);
}
