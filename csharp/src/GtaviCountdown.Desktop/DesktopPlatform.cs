using System.Diagnostics;
using GtaviCountdown.Core;
using GtaviCountdown.Core.TimeSync;
using GtaviCountdown.UI.Platform;
using Photino.NET;

namespace GtaviCountdown.Desktop;

/// <summary>
/// The platform inside the native window: the two things a web page cannot do
/// on its own (measuring the true time over NTP and writing a file to the
/// download folder) plus control of the window itself.
/// </summary>
public sealed class DesktopPlatform(WindowHolder holder) : IPlatform
{
    /// <summary>Time servers, tried in order. All are anycast services with global reach.</summary>
    private static readonly string[] TimeServers = ["time.cloudflare.com:123", "pool.ntp.org:123", "time.google.com:123"];

    /// <summary>How long to wait for each reply before moving on.</summary>
    private static readonly TimeSpan TimeServerTimeout = TimeSpan.FromMilliseconds(1500);

    /// <summary>Exchanges per measurement; the one with the shortest round trip wins.</summary>
    private const int TimeSamples = 3;

    private PhotinoWindow Window => holder.Window ?? throw new InvalidOperationException("The window is not open.");

    public bool IsDesktop => true;

    public ValueTask<string?> LoadSettingsAsync()
    {
        try
        {
            return ValueTask.FromResult<string?>(File.Exists(AppPaths.Settings) ? File.ReadAllText(AppPaths.Settings) : null);
        }
        catch (Exception)
        {
            return ValueTask.FromResult<string?>(null);
        }
    }

    public ValueTask SaveSettingsAsync(string json)
    {
        try
        {
            Directory.CreateDirectory(AppPaths.ConfigDirectory);
            File.WriteAllText(AppPaths.Settings, json);
        }
        catch (Exception)
        {
            // Persistence is a convenience; the app works without it.
        }
        return ValueTask.CompletedTask;
    }

    /// <summary>The exchange blocks on the network, so it runs on the thread pool.</summary>
    public async Task<TimeSyncResult> SyncNetworkTimeAsync()
    {
        SntpMeasurement measurement;
        try
        {
            measurement = await Task.Run(() => Sntp.Measure(TimeServers, TimeServerTimeout, TimeSamples));
        }
        catch (SntpException error)
        {
            throw new InvalidOperationException(Capitalise(error.Message) + ".", error);
        }
        return new TimeSyncResult(
            measurement.OffsetMs,
            measurement.RoundTripMs,
            // Report the host without the port.
            measurement.Server.EndsWith(":123", StringComparison.Ordinal) ? measurement.Server[..^4] : measurement.Server,
            measurement.Stratum);
    }

    public ValueTask SetAlwaysOnTopAsync(bool enabled)
    {
        Window.SetTopMost(enabled);
        return ValueTask.CompletedTask;
    }

    public ValueTask<bool> CanFullscreenAsync() => ValueTask.FromResult(true);

    public ValueTask<bool> ToggleFullscreenAsync()
    {
        var next = !Window.FullScreen;
        Window.SetFullScreen(next);
        return ValueTask.FromResult(next);
    }

    // Desktop notifications need no permission.
    public ValueTask<bool> RequestNotificationPermissionAsync() => ValueTask.FromResult(true);

    public ValueTask<bool> NotifyAsync(string title, string body)
    {
        try
        {
            if (OperatingSystem.IsLinux())
            {
                // The freedesktop notification tool is present on every
                // mainstream desktop and needs no application registration.
                var start = new ProcessStartInfo("notify-send") { UseShellExecute = false };
                start.ArgumentList.Add("--app-name=GTA VI Countdown");
                start.ArgumentList.Add(title);
                start.ArgumentList.Add(body);
                using var process = Process.Start(start);
                return ValueTask.FromResult(process is not null);
            }
            Window.SendNotification(title, body);
            return ValueTask.FromResult(true);
        }
        catch (Exception)
        {
            return ValueTask.FromResult(false);
        }
    }

    /// <summary>Writes a calendar file to the user's download folder and returns its path.</summary>
    public ValueTask<string?> SaveCalendarAsync(string fileName, string contents)
    {
        var name = CalendarFile.Validate(fileName, contents);
        var directory = AppPaths.Downloads();
        var path = Path.Combine(directory, name);
        try
        {
            Directory.CreateDirectory(directory);
            File.WriteAllText(path, contents);
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException)
        {
            throw new IOException($"Could not write {path}: {error.Message}", error);
        }
        return ValueTask.FromResult<string?>(path);
    }

    /// <summary>Upper-cases the first letter only: "no time server" becomes "No time server".</summary>
    public static string Capitalise(string text) =>
        text.Length == 0 ? text : char.ToUpperInvariant(text[0]) + text[1..];
}

/// <summary>Gives services access to the window, which exists only once the app is built.</summary>
public sealed class WindowHolder
{
    public PhotinoWindow? Window { get; set; }
}
