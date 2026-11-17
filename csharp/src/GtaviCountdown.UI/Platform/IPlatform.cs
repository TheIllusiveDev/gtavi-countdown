namespace GtaviCountdown.UI.Platform;

/// <summary>Result of a clock measurement against a network time server.</summary>
/// <param name="OffsetMs">Milliseconds to add to the system clock to obtain network time.</param>
/// <param name="RoundTripMs">Round-trip delay of the measurement in milliseconds.</param>
/// <param name="Server">Host name of the server that answered.</param>
/// <param name="Stratum">Distance of the server from a reference clock (1 is best).</param>
public sealed record TimeSyncResult(double OffsetMs, double RoundTripMs, string Server, int Stratum);

/// <summary>
/// The seam between the web app and the desktop shell. Each host registers
/// one implementation; the UI never asks which one it has, only what it can do.
/// </summary>
public interface IPlatform
{
    /// <summary>True when running inside the desktop shell.</summary>
    bool IsDesktop { get; }

    /// <summary>The stored settings JSON, or null when there is none.</summary>
    ValueTask<string?> LoadSettingsAsync();

    /// <summary>Stores the settings JSON. Failures are swallowed: persistence is a convenience.</summary>
    ValueTask SaveSettingsAsync(string json);

    /// <summary>Measures the system clock error against a network time server.</summary>
    /// <exception cref="InvalidOperationException">The measurement failed; the message is user-facing.</exception>
    Task<TimeSyncResult> SyncNetworkTimeAsync();

    ValueTask SetAlwaysOnTopAsync(bool enabled);

    /// <summary>False where the window cannot go fullscreen, notably Safari on the iPhone.</summary>
    ValueTask<bool> CanFullscreenAsync();

    /// <summary>Toggles fullscreen and resolves to the new state.</summary>
    ValueTask<bool> ToggleFullscreenAsync();

    /// <summary>Asks for permission to show notifications. Resolves to whether it is granted.</summary>
    ValueTask<bool> RequestNotificationPermissionAsync();

    /// <summary>Shows a system notification. Resolves to false when it could not be shown.</summary>
    ValueTask<bool> NotifyAsync(string title, string body);

    /// <summary>
    /// Saves a calendar file. On the desktop it is written to the user's
    /// download folder and the full path is returned; in a browser it is
    /// offered as a download and null is returned.
    /// </summary>
    ValueTask<string?> SaveCalendarAsync(string fileName, string contents);
}
