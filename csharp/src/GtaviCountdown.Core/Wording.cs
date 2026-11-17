using System.Globalization;

namespace GtaviCountdown.Core;

/// <summary>
/// The human-readable dates and times shown around the countdown. Formatting
/// uses the invariant culture, whose names are English, so the output does
/// not depend on the host's locale or on ICU being present.
/// </summary>
public static class Wording
{
    private static readonly CultureInfo Invariant = CultureInfo.InvariantCulture;

    private static DateTimeOffset In(long utcMs, string timeZone) =>
        TimeZoneInfo.ConvertTime(DateTimeOffset.FromUnixTimeMilliseconds(utcMs),
            TimeZoneInfo.FindSystemTimeZoneById(timeZone));

    /// <summary><c>Thursday, November 19, 2026</c></summary>
    public static string LongDate(long utcMs, string timeZone) =>
        In(utcMs, timeZone).ToString("dddd, MMMM d, yyyy", Invariant);

    /// <summary><c>00:00</c></summary>
    public static string Time(long utcMs, string timeZone) =>
        In(utcMs, timeZone).ToString("HH:mm", Invariant);

    /// <summary><c>Wed, Nov 18, 18:00</c></summary>
    public static string ShortDateTime(long utcMs, string timeZone) =>
        In(utcMs, timeZone).ToString("ddd, MMM d, HH:mm", Invariant);

    /// <summary><c>Dec 4, 2023</c>, read in UTC.</summary>
    public static string ShortDate(long utcMs) =>
        DateTimeOffset.FromUnixTimeMilliseconds(utcMs).ToString("MMM d, yyyy", Invariant);

    /// <summary><c>Zagreb time, UTC+01:00</c></summary>
    public static string ZoneText(string timeZone, long at) =>
        $"{Zones.ZoneCity(timeZone)} time, {Zones.FormatOffset(Zones.OffsetAt(at, timeZone))}";

    /// <summary>Formats a whole number with thousands separators: <c>12,345</c>.</summary>
    public static string Integer(long value) => value.ToString("N0", Invariant);
}
