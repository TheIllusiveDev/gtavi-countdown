using static GtaviCountdown.Core.Countdown;

namespace GtaviCountdown.Tests;

public class CountdownTests
{
    private static readonly long Target = ReleaseInstant("UTC");

    private static string Iso(long ms) =>
        DateTimeOffset.FromUnixTimeMilliseconds(ms).ToString("yyyy-MM-dd'T'HH:mm:ss.fff'Z'");

    [Fact]
    public void ReleaseInstant_IsMidnightOn19NovemberInTheGivenZone()
    {
        Assert.Equal("2026-11-19T00:00:00.000Z", Iso(Target));
        Assert.Equal("2026-11-18T23:00:00.000Z", Iso(ReleaseInstant("Europe/Zagreb")));
    }

    [Fact]
    public void ReleaseInstant_MovesWestwardsWithTheSun()
    {
        Assert.True(ReleaseInstant("Pacific/Auckland") < ReleaseInstant("Asia/Tokyo"));
        Assert.True(ReleaseInstant("Asia/Tokyo") < ReleaseInstant("Europe/London"));
        Assert.True(ReleaseInstant("Europe/London") < ReleaseInstant("America/Los_Angeles"));
    }

    [Fact]
    public void Breakdown_SplitsTheRemainingTimeIntoUnits()
    {
        var total = 6 * Day + 3 * Hour + 12 * Minute + 8 * Second + 250;
        Assert.Equal(new Breakdown(false, 6, 3, 12, 8, 250, total), Breakdown(Target - total, Target));
    }

    [Fact]
    public void Breakdown_ShowsOneSecondUntilTheFinalMillisecondHasPassed()
    {
        Assert.Equal(1, Breakdown(Target - 1000, Target).Seconds);
        Assert.Equal(0, Breakdown(Target - 999, Target).Seconds);
        Assert.False(Breakdown(Target - 1, Target).Elapsed);
    }

    [Fact]
    public void Breakdown_FlipsToElapsedExactlyAtTheTargetAndCountsUpwards()
    {
        var atTarget = Breakdown(Target, Target);
        Assert.True(atTarget.Elapsed);
        Assert.Equal(0, atTarget.Total);
        Assert.Equal(0, atTarget.Seconds);

        var later = Breakdown(Target + 2 * Day + 90 * Second, Target);
        Assert.True(later.Elapsed);
        Assert.Equal(2, later.Days);
        Assert.Equal(0, later.Hours);
        Assert.Equal(1, later.Minutes);
        Assert.Equal(30, later.Seconds);
    }

    [Fact]
    public void Breakdown_CarriesMoreThan99DaysWithoutWrapping()
    {
        Assert.Equal(400, Breakdown(Target - 400 * Day, Target).Days);
    }

    [Fact]
    public void MsUntilNextSecond_ReturnsTheDistanceToTheNextChangeOfTheSecondsDigit()
    {
        // 5.250 s remain and "5" is showing; "4" appears once 4.999 s remain.
        Assert.Equal(251, MsUntilNextSecond(Target - 5_250, Target));
        Assert.Equal(2, MsUntilNextSecond(Target - 5_001, Target));
        Assert.Equal(1, MsUntilNextSecond(Target - 5_000, Target));
        Assert.Equal(1000, MsUntilNextSecond(Target - 4_999, Target));
    }

    [Fact]
    public void MsUntilNextSecond_LandsExactlyOnTheTargetForTheFinalChange()
    {
        Assert.Equal(999, MsUntilNextSecond(Target - 999, Target));
        Assert.Equal(1, MsUntilNextSecond(Target - 1, Target));
    }

    [Fact]
    public void MsUntilNextSecond_KeepsTickingInPhaseAfterTheTarget()
    {
        Assert.Equal(1000, MsUntilNextSecond(Target, Target));
        Assert.Equal(750, MsUntilNextSecond(Target + 250, Target));
        Assert.Equal(1, MsUntilNextSecond(Target + 999, Target));
    }

    [Fact]
    public void MsUntilNextSecond_AlwaysSchedulesATickBetween1msAnd1sAway()
    {
        for (var offset = -3000; offset <= 3000; offset += 37)
        {
            var wait = MsUntilNextSecond(Target + offset, Target);
            Assert.InRange(wait, 1, 1000);
        }
    }

    [Theory]
    [InlineData(-5_250)]
    [InlineData(-5_000)]
    [InlineData(-1_000)]
    [InlineData(-999)]
    [InlineData(-1)]
    [InlineData(0)]
    [InlineData(1)]
    [InlineData(400)]
    [InlineData(1_999)]
    public void MsUntilNextSecond_TheScheduledTickLandsOnANewSecondsValue(long offset)
    {
        var now = Target + offset;
        var wait = MsUntilNextSecond(now, Target);
        var before = Breakdown(now, Target);
        var after = Breakdown(now + wait, Target);
        Assert.True(after.Seconds != before.Seconds || after.Elapsed != before.Elapsed);
        // One millisecond earlier the display must still be unchanged, so the
        // tick is neither late nor wasted.
        var justBefore = Breakdown(now + wait - 1, Target);
        Assert.Equal(before.Seconds, justBefore.Seconds);
        Assert.Equal(before.Elapsed, justBefore.Elapsed);
    }

    [Fact]
    public void Totals_ExpressTheRemainingTimeInSingleUnits()
    {
        var now = Target - (6 * Day + 3 * Hour);
        var t = Totals(now, Target, "UTC");
        Assert.Equal(147, t.Hours);
        Assert.Equal(147 * 60, t.Minutes);
        Assert.Equal(147 * 3600, t.Seconds);
        Assert.Equal(0.875, t.Weeks, 3);
        Assert.Equal(7, t.Sleeps);
        Assert.Equal(1, t.Weekends);
    }

    [Fact]
    public void Totals_AreAllZerosOnceTheTargetHasPassed()
    {
        Assert.Equal(new Totals(0, 0, 0, 0, 0, 0), Totals(Target + Hour, Target, "UTC"));
    }

    [Fact]
    public void WaitProgress_StartsAtTheFirstTrailerAndEndsAtRelease()
    {
        Assert.Equal(Release.MilestoneInstant(Release.Milestones[0].Date), WaitOrigin);
        Assert.Equal(0, WaitProgress(WaitOrigin, Target));
        Assert.Equal(1, WaitProgress(Target, Target));
        Assert.Equal(0.5, WaitProgress((WaitOrigin + Target) / 2, Target), 9);
    }

    [Fact]
    public void WaitProgress_IsClampedOutsideTheInterval()
    {
        Assert.Equal(0, WaitProgress(WaitOrigin - Day, Target));
        Assert.Equal(1, WaitProgress(Target + Day, Target));
    }

    [Fact]
    public void Timeline_PlacesMilestonesInChronologicalOrder()
    {
        var positions = Release.Milestones.Select(m => TimelinePosition(Release.MilestoneInstant(m.Date), Target)).ToList();
        Assert.Equal(0, positions[0]);
        for (var i = 1; i < positions.Count; i++)
        {
            Assert.True(positions[i] > positions[i - 1]);
            Assert.True(positions[i] < 1);
        }
    }

    [Fact]
    public void FormatCompact_PadsEveryUnitExceptDays()
    {
        var now = Target - (6 * Day + 3 * Hour + 2 * Minute + 8 * Second);
        Assert.Equal("6d 03h 02m 08s", FormatCompact(Breakdown(now, Target)));
    }

    [Fact]
    public void Wording_MatchesTheWebVersion()
    {
        var target = ReleaseInstant("Europe/Zagreb");
        Assert.Equal("Thursday, November 19, 2026", Wording.LongDate(target, "Europe/Zagreb"));
        Assert.Equal("00:00", Wording.Time(target, "Europe/Zagreb"));
        Assert.Equal("Wed, Nov 18, 18:00", Wording.ShortDateTime(target, "America/New_York"));
        Assert.Equal("Zagreb time, UTC+01:00", Wording.ZoneText("Europe/Zagreb", target));
        Assert.Equal("Dec 4, 2023", Wording.ShortDate(WaitOrigin));
        Assert.Equal("12,723,000", Wording.Integer(12_723_000));
    }
}
