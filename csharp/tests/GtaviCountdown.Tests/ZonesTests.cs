using static GtaviCountdown.Core.Zones;

namespace GtaviCountdown.Tests;

public class ZonesTests
{
    private const long HourMs = 3_600_000;

    private static long Utc(int year, int month, int day, int hour = 0, int minute = 0, int second = 0, int ms = 0) =>
        new DateTimeOffset(year, month, day, hour, minute, second, ms, TimeSpan.Zero).ToUnixTimeMilliseconds();

    private static string Iso(long ms) =>
        DateTimeOffset.FromUnixTimeMilliseconds(ms).ToString("yyyy-MM-dd'T'HH:mm:ss.fff'Z'");

    [Fact]
    public void OffsetAt_ReturnsZeroForUtc() => Assert.Equal(0, OffsetAt(Utc(2026, 11, 19), "UTC"));

    [Fact]
    public void OffsetAt_FollowsDaylightSavingInEurope()
    {
        Assert.Equal(1 * HourMs, OffsetAt(Utc(2026, 11, 19), "Europe/Zagreb"));
        Assert.Equal(2 * HourMs, OffsetAt(Utc(2026, 7, 1), "Europe/Zagreb"));
    }

    [Fact]
    public void OffsetAt_HandlesZonesWestOfGreenwichAndFractionalOffsets()
    {
        Assert.Equal(-5 * HourMs, OffsetAt(Utc(2026, 11, 19), "America/New_York"));
        Assert.Equal((long)(5.5 * HourMs), OffsetAt(Utc(2026, 11, 19), "Asia/Kolkata"));
        Assert.Equal((long)(13.75 * HourMs), OffsetAt(Utc(2026, 11, 19), "Pacific/Chatham"));
    }

    [Fact]
    public void OffsetAt_IgnoresTheSubSecondPartOfTheInstant() =>
        Assert.Equal(9 * HourMs, OffsetAt(Utc(2026, 11, 19, 0, 0, 0, 999), "Asia/Tokyo"));

    [Theory]
    [InlineData("UTC", "2026-11-19T00:00:00.000Z")]
    [InlineData("Europe/London", "2026-11-19T00:00:00.000Z")]
    [InlineData("Europe/Zagreb", "2026-11-18T23:00:00.000Z")]
    [InlineData("Pacific/Auckland", "2026-11-18T11:00:00.000Z")]
    [InlineData("Asia/Tokyo", "2026-11-18T15:00:00.000Z")]
    [InlineData("Asia/Kolkata", "2026-11-18T18:30:00.000Z")]
    [InlineData("America/New_York", "2026-11-19T05:00:00.000Z")]
    [InlineData("America/Los_Angeles", "2026-11-19T08:00:00.000Z")]
    public void ZonedTimeToUtc_ResolvesReleaseMidnightAroundTheWorld(string zone, string expected) =>
        Assert.Equal(expected, Iso(ZonedTimeToUtc(2026, 11, 19, 0, 0, zone)));

    [Fact]
    public void ZonedTimeToUtc_IsCorrectOnBothSidesOfADaylightSavingChange()
    {
        // Europe leaves summer time on 25 October 2026 at 01:00 UTC.
        Assert.Equal("2026-10-24T22:30:00.000Z", Iso(ZonedTimeToUtc(2026, 10, 25, 0, 30, "Europe/Berlin")));
        Assert.Equal("2026-10-25T11:00:00.000Z", Iso(ZonedTimeToUtc(2026, 10, 25, 12, 0, "Europe/Berlin")));
    }

    [Theory]
    [InlineData("Australia/Sydney")]
    [InlineData("America/Sao_Paulo")]
    [InlineData("Asia/Kathmandu")]
    [InlineData("Pacific/Honolulu")]
    public void ZonedTimeToUtc_RoundTripsThroughWallTimeAt(string zone)
    {
        var instant = ZonedTimeToUtc(2026, 11, 19, 0, 0, zone);
        Assert.Equal(new WallTime(2026, 11, 19, 0, 0, 0), WallTimeAt(instant, zone));
    }

    [Fact]
    public void FormatOffset_FormatsPositiveNegativeFractionalAndZeroOffsets()
    {
        Assert.Equal("UTC", FormatOffset(0));
        Assert.Equal("UTC+01:00", FormatOffset(1 * HourMs));
        Assert.Equal("UTC-08:00", FormatOffset(-8 * HourMs));
        Assert.Equal("UTC+05:30", FormatOffset((long)(5.5 * HourMs)));
        Assert.Equal("UTC-03:30", FormatOffset((long)(-3.5 * HourMs)));
        Assert.Equal("UTC+14:00", FormatOffset(14 * HourMs));
    }

    [Fact]
    public void ZoneNames_AreValidated()
    {
        Assert.True(IsValidTimeZone("Europe/Zagreb"));
        Assert.False(IsValidTimeZone("Mars/Olympus_Mons"));
        Assert.False(IsValidTimeZone(""));
        Assert.False(IsValidTimeZone(null));
    }

    [Fact]
    public void ListTimeZones_IncludesUtc()
    {
        var zones = ListTimeZones();
        Assert.True(zones.Count > 10);
        Assert.Contains("UTC", zones);
        Assert.All(zones, zone => Assert.True(IsValidTimeZone(zone), zone));
    }

    [Fact]
    public void ListTimeZones_ListsGeographicZonesOnlyWithoutDuplicates()
    {
        var zones = ListTimeZones();
        Assert.DoesNotContain(zones, zone => zone.StartsWith("Etc/"));
        Assert.DoesNotContain(zones, zone => zone != "UTC" && !zone.Contains('/'));
        Assert.Equal(zones.Count, zones.Distinct().Count());
        Assert.Contains("Pacific/Auckland", zones);
        Assert.Contains("America/Los_Angeles", zones);
    }

    [Fact]
    public void ZoneNames_SplitIntoCityAndRegion()
    {
        Assert.Equal("Buenos Aires", ZoneCity("America/Argentina/Buenos_Aires"));
        Assert.Equal("America / Argentina", ZoneRegion("America/Argentina/Buenos_Aires"));
        Assert.Equal("UTC", ZoneCity("UTC"));
        Assert.Equal("", ZoneRegion("UTC"));
    }

    private const string Zone = "Europe/Zagreb";
    private static readonly long ReleaseAt = ZonedTimeToUtc(2026, 11, 19, 0, 0, Zone);

    [Fact]
    public void CalendarDaysBetween_CountsSleepsAsMidnightsInTheTargetZone()
    {
        // Thursday evening, a week before release: seven nights to go.
        Assert.Equal(7, CalendarDaysBetween(ZonedTimeToUtc(2026, 11, 12, 20, 30, Zone), ReleaseAt, Zone));
        // One minute past midnight on the 18th: still one night.
        Assert.Equal(1, CalendarDaysBetween(ZonedTimeToUtc(2026, 11, 18, 0, 1, Zone), ReleaseAt, Zone));
        Assert.Equal(1, CalendarDaysBetween(ZonedTimeToUtc(2026, 11, 18, 23, 59, Zone), ReleaseAt, Zone));
    }

    [Fact]
    public void CalendarDaysBetween_CountsAcrossADaylightSavingChangeWithoutDrifting()
    {
        var from = ZonedTimeToUtc(2026, 10, 20, 23, 0, Zone);
        Assert.Equal(30, CalendarDaysBetween(from, ReleaseAt, Zone));
    }

    [Fact]
    public void WeekendsBetween_CountsTheSaturdaysStrictlyBetweenTwoDates()
    {
        long At(int day, int hour = 12) => ZonedTimeToUtc(2026, 11, day, hour, 0, Zone);
        // 12 November 2026 is a Thursday; the 14th is the only Saturday before the 19th.
        Assert.Equal(1, WeekendsBetween(At(12), ReleaseAt, Zone));
        Assert.Equal(1, WeekendsBetween(At(13), ReleaseAt, Zone));
        // On the Saturday itself the weekend has begun and no longer counts.
        Assert.Equal(0, WeekendsBetween(At(14), ReleaseAt, Zone));
        Assert.Equal(0, WeekendsBetween(At(18), ReleaseAt, Zone));
        // From Sunday 1 November: the 7th and the 14th.
        Assert.Equal(2, WeekendsBetween(At(1), ReleaseAt, Zone));
        // From Saturday 31 October: the 7th and the 14th.
        Assert.Equal(2, WeekendsBetween(ZonedTimeToUtc(2026, 10, 31, 12, 0, Zone), ReleaseAt, Zone));
    }
}
