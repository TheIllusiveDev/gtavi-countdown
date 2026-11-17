namespace GtaviCountdown.Core;

/// <summary>
/// Release facts. This is the single place to edit if the date ever moves.
/// </summary>
/// <remarks>
/// Rockstar Games has announced the calendar date. The unlock hour used here,
/// midnight local time in each region, is the timing reported by the press at
/// the time of writing; it is a wall-clock time, so the absolute instant
/// depends on the time zone the countdown is evaluated in.
/// </remarks>
public static class Release
{
    public const string Title = "Grand Theft Auto VI";
    public const int Year = 2026;
    /// <summary>1-based month.</summary>
    public const int Month = 11;
    public const int Day = 19;
    public const int Hour = 0;
    public const int Minute = 0;

    public static readonly IReadOnlyList<string> Platforms = ["PlayStation 5", "Xbox Series X|S"];

    /// <summary>
    /// Public milestones on the road to release, oldest first. The first entry
    /// is the origin of the progress bar; the release itself is added by the
    /// timeline because its instant depends on the selected time zone.
    /// </summary>
    public static readonly IReadOnlyList<Milestone> Milestones =
    [
        new("2023-12-04", "Trailer 1", "First trailer published, with a 2025 release window."),
        new("2025-05-02", "First delay", "Release moved to May 26, 2026. Trailer 2 followed on May 6."),
        new("2025-11-06", "Second delay", "Release moved to November 19, 2026."),
    ];

    /// <summary>Converts a <c>YYYY-MM-DD</c> milestone date to an instant at noon UTC, in Unix milliseconds.</summary>
    public static long MilestoneInstant(string date)
    {
        var day = DateOnly.ParseExact(date, "yyyy-MM-dd", System.Globalization.CultureInfo.InvariantCulture);
        var noon = new DateTimeOffset(day.ToDateTime(new TimeOnly(12, 0)), TimeSpan.Zero);
        return noon.ToUnixTimeMilliseconds();
    }
}

/// <param name="Date">Calendar date (UTC) of the event, as <c>YYYY-MM-DD</c>.</param>
public sealed record Milestone(string Date, string Title, string Detail);
