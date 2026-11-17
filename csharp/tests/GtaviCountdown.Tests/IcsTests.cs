using System.Text;

namespace GtaviCountdown.Tests;

public class IcsTests
{
    private static long Utc(int year, int month, int day, int hour, int minute, int second, int ms = 0) =>
        new DateTimeOffset(year, month, day, hour, minute, second, ms, TimeSpan.Zero).ToUnixTimeMilliseconds();

    private static int Bytes(string text) => Encoding.UTF8.GetByteCount(text);

    private static string Unfold(string[] lines) =>
        string.Concat(lines.Select((line, i) => i == 0 ? line : line[1..]));

    [Fact]
    public void Date_FormatsAnInstantAsAUtcDateTime()
    {
        Assert.Equal("20261118T230000Z", Ics.Date(Utc(2026, 11, 18, 23, 0, 0)));
        Assert.Equal("20260102T030405Z", Ics.Date(Utc(2026, 1, 2, 3, 4, 5, 678)));
    }

    [Fact]
    public void Escape_EscapesTheCharactersRfc5545ReservesInText()
    {
        Assert.Equal("PS5\\, Xbox\\; both", Ics.Escape("PS5, Xbox; both"));
        Assert.Equal("a\\\\b", Ics.Escape("a\\b"));
        Assert.Equal("line one\\nline two\\nline three", Ics.Escape("line one\nline two\r\nline three"));
    }

    [Fact]
    public void Fold_LeavesShortLinesAlone()
    {
        Assert.Equal("SUMMARY:Short", Ics.Fold("SUMMARY:Short"));
        Assert.Equal(new string('X', 75), Ics.Fold(new string('X', 75)));
    }

    [Fact]
    public void Fold_FoldsLongLinesAt75OctetsWithALeadingSpace()
    {
        var original = "DESCRIPTION:" + new string('a', 200);
        var lines = Ics.Fold(original).Split("\r\n");
        Assert.True(lines.Length > 1);
        Assert.All(lines, line => Assert.True(Bytes(line) <= 75));
        Assert.All(lines.Skip(1), line => Assert.StartsWith(" ", line));
        // Unfolding restores the original content line.
        Assert.Equal(original, Unfold(lines));
    }

    [Fact]
    public void Fold_NeverSplitsAMultiByteCharacter()
    {
        var original = "SUMMARY:" + string.Concat(Enumerable.Repeat("é中\U0001F3AE", 30));
        var lines = Ics.Fold(original).Split("\r\n");
        Assert.All(lines, line => Assert.True(Bytes(line) <= 75));
        Assert.Equal(original, Unfold(lines));
    }

    private static readonly string Calendar = Ics.BuildCalendar(new CalendarEvent(
        Uid: "release-1@gtavi-countdown",
        Start: Utc(2026, 11, 18, 23, 0, 0),
        DurationMinutes: 60,
        Summary: "Grand Theft Auto VI release",
        Description: "Platforms: PlayStation 5, Xbox Series X|S.",
        Reminders: [1440, 60],
        Stamp: Utc(2026, 11, 12, 19, 0, 0)));

    private static readonly string[] Lines = Calendar.Split("\r\n");

    [Fact]
    public void BuildCalendar_ProducesAWellFormedCalendarObject()
    {
        Assert.Equal("BEGIN:VCALENDAR", Lines[0]);
        Assert.Equal("END:VCALENDAR", Lines[^2]);
        Assert.Equal("", Lines[^1]);
        Assert.DoesNotMatch("[^\r]\n", Calendar);
        Assert.Contains("VERSION:2.0", Lines);
        Assert.Contains("UID:release-1@gtavi-countdown", Lines);
        Assert.Contains("DTSTAMP:20261112T190000Z", Lines);
        Assert.Contains("DTSTART:20261118T230000Z", Lines);
        Assert.Contains("DTEND:20261119T000000Z", Lines);
        Assert.Contains("SUMMARY:Grand Theft Auto VI release", Lines);
        Assert.Contains("DESCRIPTION:Platforms: PlayStation 5\\, Xbox Series X|S.", Lines);
    }

    [Fact]
    public void BuildCalendar_AddsOneAlarmPerReminder()
    {
        Assert.Equal(2, Lines.Count(line => line == "BEGIN:VALARM"));
        Assert.Equal(2, Lines.Count(line => line == "END:VALARM"));
        Assert.Contains("TRIGGER:-PT1440M", Lines);
        Assert.Contains("TRIGGER:-PT60M", Lines);
    }

    [Fact]
    public void BuildCalendar_BalancesEveryBeginWithAnEnd()
    {
        Assert.Equal(Lines.Count(l => l.StartsWith("BEGIN:")), Lines.Count(l => l.StartsWith("END:")));
    }
}
