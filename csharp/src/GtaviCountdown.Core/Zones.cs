using System.Collections.Concurrent;
using System.Globalization;

namespace GtaviCountdown.Core;

/// <summary>A wall-clock reading in some time zone, to the second.</summary>
/// <param name="Month">1-based month.</param>
public readonly record struct WallTime(int Year, int Month, int Day, int Hour, int Minute, int Second);

/// <summary>
/// Time zone arithmetic on IANA identifiers, built on <see cref="TimeZoneInfo"/>.
/// Instants are Unix milliseconds throughout, as in the rest of the app.
/// </summary>
public static class Zones
{
    private static readonly ConcurrentDictionary<string, TimeZoneInfo?> Cache = new(StringComparer.Ordinal);

    /// <summary>
    /// Continents and oceans that prefix geographic IANA zones. Anything else
    /// (the inverted "Etc/GMT+N" family, legacy aliases such as "US/Eastern"
    /// or bare abbreviations such as "EST") would confuse more than it helps
    /// in a picker.
    /// </summary>
    private static readonly string[] GeographicAreas =
    [
        "Africa/", "America/", "Antarctica/", "Arctic/", "Asia/", "Atlantic/",
        "Australia/", "Europe/", "Indian/", "Pacific/",
    ];

    private static readonly string[] CuratedZones =
    [
        "UTC",
        "Pacific/Auckland",
        "Australia/Sydney",
        "Asia/Tokyo",
        "Asia/Seoul",
        "Asia/Shanghai",
        "Asia/Kolkata",
        "Asia/Dubai",
        "Europe/Moscow",
        "Europe/Berlin",
        "Europe/London",
        "America/Sao_Paulo",
        "America/New_York",
        "America/Chicago",
        "America/Denver",
        "America/Los_Angeles",
        "Pacific/Honolulu",
    ];

    private static TimeZoneInfo? Lookup(string timeZone) =>
        Cache.GetOrAdd(timeZone, static id =>
            TimeZoneInfo.TryFindSystemTimeZoneById(id, out var zone) ? zone : null);

    private static TimeZoneInfo Resolve(string timeZone) =>
        Lookup(timeZone) ?? throw new TimeZoneNotFoundException($"Unknown time zone \"{timeZone}\".");

    /// <summary>Returns the wall-clock reading in <paramref name="timeZone"/> at the instant <paramref name="utcMs"/>.</summary>
    public static WallTime WallTimeAt(long utcMs, string timeZone)
    {
        var local = TimeZoneInfo.ConvertTime(DateTimeOffset.FromUnixTimeMilliseconds(utcMs), Resolve(timeZone));
        return new WallTime(local.Year, local.Month, local.Day, local.Hour, local.Minute, local.Second);
    }

    /// <summary>
    /// Offset of <paramref name="timeZone"/> from UTC at the instant <paramref name="utcMs"/>, in milliseconds.
    /// Positive values are east of Greenwich (for example +3 600 000 for CET).
    /// </summary>
    public static long OffsetAt(long utcMs, string timeZone) =>
        (long)Resolve(timeZone).GetUtcOffset(DateTimeOffset.FromUnixTimeMilliseconds(utcMs)).TotalMilliseconds;

    /// <summary>
    /// Converts a wall-clock time in <paramref name="timeZone"/> to the UTC instant it denotes.
    /// </summary>
    /// <remarks>
    /// The offset is resolved in two passes so that the result is correct when
    /// the first guess lands on the other side of a daylight-saving transition.
    /// </remarks>
    public static long ZonedTimeToUtc(int year, int month, int day, int hour, int minute, string timeZone)
    {
        var wall = new DateTimeOffset(year, month, day, hour, minute, 0, TimeSpan.Zero).ToUnixTimeMilliseconds();
        var firstGuess = wall - OffsetAt(wall, timeZone);
        return wall - OffsetAt(firstGuess, timeZone);
    }

    /// <summary>Formats an offset in milliseconds as <c>UTC+05:30</c>, <c>UTC-08:00</c> or <c>UTC</c>.</summary>
    public static string FormatOffset(long offsetMs)
    {
        if (offsetMs == 0) return "UTC";
        var sign = offsetMs < 0 ? '-' : '+';
        var totalMinutes = (long)Math.Round(Math.Abs(offsetMs) / 60_000.0, MidpointRounding.AwayFromZero);
        return string.Create(CultureInfo.InvariantCulture, $"UTC{sign}{totalMinutes / 60:00}:{totalMinutes % 60:00}");
    }

    /// <summary>The IANA time zone the host system is configured for, or UTC.</summary>
    public static string SystemTimeZone()
    {
        try
        {
            var id = TimeZoneInfo.Local.Id;
            // Windows reports its own identifiers; the rest of the app speaks IANA.
            if (TimeZoneInfo.TryConvertWindowsIdToIanaId(id, out var iana)) id = iana;
            return IsValidTimeZone(id) ? id : "UTC";
        }
        catch (Exception)
        {
            return "UTC";
        }
    }

    /// <summary>True when the runtime recognises <paramref name="timeZone"/> as a time zone identifier.</summary>
    public static bool IsValidTimeZone(string? timeZone) =>
        !string.IsNullOrEmpty(timeZone) && Lookup(timeZone) is not null;

    /// <summary>
    /// Every geographic IANA zone the runtime knows, plus UTC, falling back to
    /// a short curated list on runtimes that cannot enumerate their zones.
    /// </summary>
    public static IReadOnlyList<string> ListTimeZones()
    {
        try
        {
            var zones = new SortedSet<string>(StringComparer.Ordinal);
            foreach (var zone in TimeZoneInfo.GetSystemTimeZones())
            {
                var id = zone.Id;
                if (TimeZoneInfo.TryConvertWindowsIdToIanaId(id, out var iana)) id = iana;
                if (GeographicAreas.Any(area => id.StartsWith(area, StringComparison.Ordinal)) && IsValidTimeZone(id))
                {
                    zones.Add(id);
                }
            }
            if (zones.Count >= 10) return ["UTC", .. zones];
        }
        catch (Exception)
        {
            // Fall through to the curated list.
        }
        return CuratedZones.Where(IsValidTimeZone).ToArray();
    }

    /// <summary><c>America/Argentina/Buenos_Aires</c> becomes <c>Buenos Aires</c>.</summary>
    public static string ZoneCity(string timeZone) =>
        timeZone[(timeZone.LastIndexOf('/') + 1)..].Replace('_', ' ');

    /// <summary><c>America/Argentina/Buenos_Aires</c> becomes <c>America / Argentina</c>.</summary>
    public static string ZoneRegion(string timeZone)
    {
        var cut = timeZone.LastIndexOf('/');
        return cut == -1 ? "" : timeZone[..cut].Replace('_', ' ').Replace("/", " / ");
    }

    /// <summary>
    /// Whole calendar days from the date of <paramref name="fromMs"/> to the date
    /// of <paramref name="toMs"/>, both read in <paramref name="timeZone"/>. This
    /// is the number of midnights in between: "sleeps".
    /// </summary>
    public static int CalendarDaysBetween(long fromMs, long toMs, string timeZone)
    {
        var a = WallTimeAt(fromMs, timeZone);
        var b = WallTimeAt(toMs, timeZone);
        return new DateOnly(b.Year, b.Month, b.Day).DayNumber - new DateOnly(a.Year, a.Month, a.Day).DayNumber;
    }

    /// <summary>
    /// Number of Saturdays strictly after the date of <paramref name="fromMs"/> and
    /// strictly before the date of <paramref name="toMs"/> in <paramref name="timeZone"/>:
    /// the full weekends still to get through.
    /// </summary>
    public static int WeekendsBetween(long fromMs, long toMs, string timeZone)
    {
        var days = CalendarDaysBetween(fromMs, toMs, timeZone);
        if (days <= 1) return 0;
        var a = WallTimeAt(fromMs, timeZone);
        // Day of week of the starting date: 0 is Sunday, 6 is Saturday.
        var startDow = (int)new DateOnly(a.Year, a.Month, a.Day).DayOfWeek;
        // Days until the first Saturday after the starting date (1 to 7).
        var untilSaturday = (6 - startDow + 7) % 7;
        if (untilSaturday == 0) untilSaturday = 7;
        if (untilSaturday >= days) return 0;
        return 1 + (days - 1 - untilSaturday) / 7;
    }
}
