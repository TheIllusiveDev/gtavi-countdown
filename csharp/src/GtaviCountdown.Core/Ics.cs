using System.Globalization;
using System.Text;

namespace GtaviCountdown.Core;

/// <param name="Start">Event start as a UTC instant in Unix milliseconds.</param>
/// <param name="Reminders">Minutes before the start at which to raise a reminder.</param>
/// <param name="Stamp">Creation stamp in Unix milliseconds; defaults to the current time.</param>
public sealed record CalendarEvent(
    string Uid,
    long Start,
    int DurationMinutes,
    string Summary,
    string Description,
    IReadOnlyList<int> Reminders,
    long? Stamp = null);

/// <summary>Minimal iCalendar (RFC 5545) writer for a single event with reminders.</summary>
public static class Ics
{
    /// <summary><c>2026-11-18T23:00:00.000Z</c> becomes <c>20261118T230000Z</c>.</summary>
    public static string Date(long utcMs) =>
        DateTimeOffset.FromUnixTimeMilliseconds(utcMs).UtcDateTime.ToString("yyyyMMdd'T'HHmmss'Z'", CultureInfo.InvariantCulture);

    /// <summary>Escapes TEXT values as required by RFC 5545 section 3.3.11.</summary>
    public static string Escape(string text) =>
        text.Replace("\\", "\\\\")
            .Replace(";", "\\;")
            .Replace(",", "\\,")
            .Replace("\r\n", "\\n")
            .Replace("\n", "\\n");

    /// <summary>Folds a content line so that no physical line exceeds 75 octets.</summary>
    public static string Fold(string line)
    {
        if (Encoding.UTF8.GetByteCount(line) <= 75) return line;

        var parts = new List<string>();
        var current = new StringBuilder();
        var currentBytes = 0;
        // The first line may hold 75 octets; continuation lines start with a
        // space, which leaves 74 octets for content.
        var limit = 75;
        // Iterating runes keeps surrogate pairs (emoji and the like) together.
        foreach (var rune in line.EnumerateRunes())
        {
            var size = rune.Utf8SequenceLength;
            if (currentBytes + size > limit)
            {
                parts.Add(current.ToString());
                current.Clear();
                currentBytes = 0;
                limit = 74;
            }
            current.Append(rune.ToString());
            currentBytes += size;
        }
        parts.Add(current.ToString());
        return string.Join("\r\n ", parts);
    }

    public static string BuildCalendar(CalendarEvent calendarEvent)
    {
        var e = calendarEvent;
        var lines = new List<string>
        {
            "BEGIN:VCALENDAR",
            "VERSION:2.0",
            "PRODID:-//gtavi-countdown//Release Countdown//EN",
            "CALSCALE:GREGORIAN",
            "METHOD:PUBLISH",
            "BEGIN:VEVENT",
            $"UID:{e.Uid}",
            $"DTSTAMP:{Date(e.Stamp ?? DateTimeOffset.UtcNow.ToUnixTimeMilliseconds())}",
            $"DTSTART:{Date(e.Start)}",
            $"DTEND:{Date(e.Start + e.DurationMinutes * 60_000L)}",
            $"SUMMARY:{Escape(e.Summary)}",
            $"DESCRIPTION:{Escape(e.Description)}",
            "TRANSP:TRANSPARENT",
        };
        foreach (var minutes in e.Reminders)
        {
            lines.Add("BEGIN:VALARM");
            lines.Add("ACTION:DISPLAY");
            lines.Add($"DESCRIPTION:{Escape(e.Summary)}");
            lines.Add(string.Create(CultureInfo.InvariantCulture, $"TRIGGER:-PT{minutes}M"));
            lines.Add("END:VALARM");
        }
        lines.Add("END:VEVENT");
        lines.Add("END:VCALENDAR");
        return string.Join("\r\n", lines.Select(Fold)) + "\r\n";
    }
}
