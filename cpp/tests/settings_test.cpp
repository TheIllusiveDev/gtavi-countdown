#include <gtest/gtest.h>

#include "settings.hpp"

using namespace gtavi;

class MalformedSettings : public testing::TestWithParam<const char*> {};

TEST_P(MalformedSettings, FallBackToTheDefaults) { EXPECT_EQ(settings::sanitize(GetParam()), Settings{}); }

INSTANTIATE_TEST_SUITE_P(Settings, MalformedSettings,
                         testing::Values("", "   ", "null", "42", "\"text\"", "[]", "true", "{not json"));

namespace {
const Settings stored{"Pacific/Auckland", true, Theme::midnight, EffectLevel::low, true, false, true};
}

TEST(Settings, SanitizeKeepsValidFields) { EXPECT_EQ(settings::sanitize(settings::serialize(stored)), stored); }

TEST(Settings, SanitizeReadsTheStoredFormOfTheWebVersion) {
    auto json = R"({"timeZone":"Pacific/Auckland","showMilliseconds":true,"theme":"midnight","effects":"low",
                   "notify":true,"networkTime":false,"alwaysOnTop":true})";
    EXPECT_EQ(settings::sanitize(json), stored);
}

TEST(Settings, SanitizeReplacesInvalidFieldsWithTheirDefaultsOneByOne) {
    auto result = settings::sanitize(
        R"({"timeZone":"Atlantis/Lost_City","showMilliseconds":"yes","theme":"neon","effects":"ultra",
            "notify":1,"networkTime":false,"alwaysOnTop":null})");
    Settings expected;
    expected.network_time = false;
    EXPECT_EQ(result, expected);
}

TEST(Settings, SanitizeIgnoresUnknownKeys) {
    auto result = settings::sanitize(R"({"theme":"dawn","injected":"<script>"})");
    Settings expected;
    expected.theme = Theme::dawn;
    EXPECT_EQ(result, expected);
    EXPECT_EQ(settings::serialize(result).find("injected"), std::string::npos);
}

TEST(Settings, SanitizeTreatsANullTimeZoneAsFollowingTheDevice) {
    EXPECT_FALSE(settings::sanitize(R"({"timeZone":null})").time_zone.has_value());
}

TEST(Settings, SerializeMatchesTheOtherVersionsByteForByte) {
    EXPECT_EQ(settings::serialize(Settings{}),
              R"({"timeZone":null,"showMilliseconds":false,"theme":"dusk","effects":"auto",)"
              R"("notify":false,"networkTime":true,"alwaysOnTop":false})");
}

TEST(Settings, NextCyclesThroughTheThemes) {
    EXPECT_EQ(settings::next(Theme::dusk), Theme::midnight);
    EXPECT_EQ(settings::next(Theme::midnight), Theme::dawn);
    EXPECT_EQ(settings::next(Theme::dawn), Theme::dusk);
}
