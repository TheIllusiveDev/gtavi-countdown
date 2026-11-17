namespace GtaviCountdown.Core;

/// <summary>
/// Validation for calendar files the desktop app writes on behalf of the UI.
/// </summary>
/// <remarks>
/// The file name and contents come from the web view. They are treated as
/// untrusted input: saving must not be usable to write an arbitrary file to
/// an arbitrary place, whatever script ends up running in the page.
/// </remarks>
public static class CalendarFile
{
    /// <summary>Largest calendar file the app will write. A single event is under 1 KiB.</summary>
    public const int MaxBytes = 64 * 1024;

    /// <summary>
    /// Checks a calendar export and returns the file name to save it under.
    /// The name must be a bare <c>.ics</c> file name with no directory
    /// components, and the contents must be an iCalendar object of a sane size.
    /// </summary>
    /// <exception cref="ArgumentException">The name or contents are not acceptable.</exception>
    public static string Validate(string fileName, string contents)
    {
        var name = fileName.Trim();
        static bool Allowed(char c) => char.IsAsciiLetterOrDigit(c) || c is '-' or '_' or '.';
        if (name.Length == 0
            || name.Length > 64
            || name.StartsWith('.')
            || !name.All(Allowed)
            || !name.EndsWith(".ics", StringComparison.OrdinalIgnoreCase))
        {
            throw new ArgumentException("The calendar file name is not valid.", nameof(fileName));
        }
        if (System.Text.Encoding.UTF8.GetByteCount(contents) > MaxBytes)
        {
            throw new ArgumentException("The calendar file is too large.", nameof(contents));
        }
        var body = contents.TrimEnd();
        if (!body.StartsWith("BEGIN:VCALENDAR", StringComparison.Ordinal) || !body.EndsWith("END:VCALENDAR", StringComparison.Ordinal))
        {
            throw new ArgumentException("The data is not a calendar file.", nameof(contents));
        }
        return name;
    }
}
