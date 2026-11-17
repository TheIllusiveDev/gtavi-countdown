using System.Globalization;

namespace GtaviCountdown.Core;

/// <summary>The distance to the target split into display units.</summary>
/// <param name="Elapsed">True once the target instant has passed; the parts then count upwards.</param>
/// <param name="Total">Absolute distance to the target in milliseconds.</param>
public readonly record struct Breakdown(
    bool Elapsed,
    long Days,
    int Hours,
    int Minutes,
    int Seconds,
    int Milliseconds,
    long Total);

/// <summary>Alternative single-unit readings of the remaining time.</summary>
/// <param name="Sleeps">Midnights between now and the target in the target zone.</param>
/// <param name="Weekends">Full weekends (Saturdays) between now and the target.</param>
public readonly record struct Totals(double Weeks, long Hours, long Minutes, long Seconds, int Sleeps, int Weekends);

public static class Countdown
{
    public const long Second = 1000;
    public const long Minute = 60 * Second;
    public const long Hour = 60 * Minute;
    public const long Day = 24 * Hour;

    /// <summary>The instant the game unlocks for someone whose clock reads <paramref name="timeZone"/>.</summary>
    public static long ReleaseInstant(string timeZone) =>
        Zones.ZonedTimeToUtc(Release.Year, Release.Month, Release.Day, Release.Hour, Release.Minute, timeZone);

    /// <summary>Splits the distance between <paramref name="now"/> and <paramref name="target"/> into display units.</summary>
    public static Breakdown Breakdown(long now, long target)
    {
        var diff = target - now;
        var total = Math.Abs(diff);
        return new Breakdown(
            Elapsed: diff <= 0,
            Days: total / Day,
            Hours: (int)(total % Day / Hour),
            Minutes: (int)(total % Hour / Minute),
            Seconds: (int)(total % Minute / Second),
            Milliseconds: (int)(total % Second),
            Total: total);
    }

    /// <summary>
    /// Milliseconds until the display changes next. Scheduling the tick on this
    /// boundary keeps the display in phase with the target instead of with the
    /// moment the app happened to start.
    /// </summary>
    /// <remarks>
    /// Before the target the seconds are floored, so the reading drops one
    /// millisecond after each whole second of remaining time; the final change
    /// is the flip to "elapsed" at the target itself. After the target the
    /// reading rises on each whole second of elapsed time.
    /// </remarks>
    public static long MsUntilNextSecond(long now, long target)
    {
        var diff = target - now;
        if (diff <= 0) return Second - (-diff % Second);
        if (diff < Second) return diff;
        return diff % Second + 1;
    }

    /// <summary>Alternative single-unit readings of the remaining time.</summary>
    public static Totals Totals(long now, long target, string timeZone)
    {
        var remaining = Math.Max(0, target - now);
        return new Totals(
            Weeks: remaining / (7.0 * Day),
            Hours: remaining / Hour,
            Minutes: remaining / Minute,
            Seconds: remaining / Second,
            Sleeps: remaining == 0 ? 0 : Math.Max(0, Zones.CalendarDaysBetween(now, target, timeZone)),
            Weekends: remaining == 0 ? 0 : Zones.WeekendsBetween(now, target, timeZone));
    }

    /// <summary>The instant the wait began: the publication of the first trailer.</summary>
    public static readonly long WaitOrigin = Release.MilestoneInstant(Release.Milestones[0].Date);

    /// <summary>Fraction of the wait since the first trailer that is over, from 0 to 1.</summary>
    public static double WaitProgress(long now, long target) => TimelinePosition(now, target);

    /// <summary>Position of an instant on the wait timeline, from 0 to 1.</summary>
    public static double TimelinePosition(long instant, long target)
    {
        var span = target - WaitOrigin;
        if (span <= 0) return 1;
        return Math.Clamp((instant - WaitOrigin) / (double)span, 0, 1);
    }

    /// <summary>Compact one-line reading such as <c>6d 03h 12m 08s</c>.</summary>
    public static string FormatCompact(Breakdown b) =>
        string.Create(CultureInfo.InvariantCulture, $"{b.Days}d {b.Hours:00}h {b.Minutes:00}m {b.Seconds:00}s");
}
