namespace GtaviCountdown.Tests;

public class CalendarFileTests
{
    private const string Calendar = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nEND:VCALENDAR\r\n";

    [Fact]
    public void AcceptsAPlainCalendarFile()
    {
        Assert.Equal("gta-vi-release.ics", CalendarFile.Validate("gta-vi-release.ics", Calendar));
        Assert.Equal("Event_1.ICS", CalendarFile.Validate("  Event_1.ICS ", Calendar));
    }

    [Theory]
    [InlineData("")]
    [InlineData("../evil.ics")]
    [InlineData("/etc/cron.d/evil.ics")]
    [InlineData("..\\evil.ics")]
    [InlineData("sub/dir.ics")]
    [InlineData(".hidden.ics")]
    [InlineData("notes.txt")]
    [InlineData("run.ics.sh")]
    [InlineData("nul\0.ics")]
    [InlineData("café.ics")]
    public void RejectsPathsAndOtherExtensions(string name) =>
        Assert.Throws<ArgumentException>(() => CalendarFile.Validate(name, Calendar));

    [Fact]
    public void RejectsOverlongNames() =>
        Assert.Throws<ArgumentException>(() => CalendarFile.Validate(new string('a', 80) + ".ics", Calendar));

    [Fact]
    public void RejectsContentThatIsNotACalendar()
    {
        Assert.Throws<ArgumentException>(() => CalendarFile.Validate("a.ics", "#!/bin/sh\nrm -rf ~\n"));
        Assert.Throws<ArgumentException>(() => CalendarFile.Validate("a.ics", "BEGIN:VCALENDAR\r\n"));
        var huge = $"BEGIN:VCALENDAR\r\n{new string('X', CalendarFile.MaxBytes)}END:VCALENDAR";
        Assert.Throws<ArgumentException>(() => CalendarFile.Validate("a.ics", huge));
    }
}
