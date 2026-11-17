using GtaviCountdown.Core;

namespace GtaviCountdown.UI.Platform;

/// <summary>The platform as seen from a plain browser tab.</summary>
public sealed class BrowserPlatform(BrowserInterop browser) : IPlatform
{
    public bool IsDesktop => false;

    public ValueTask<string?> LoadSettingsAsync() => browser.StorageGetAsync(SettingsCodec.StorageKey);

    public ValueTask SaveSettingsAsync(string json) => browser.StorageSetAsync(SettingsCodec.StorageKey, json);

    public Task<TimeSyncResult> SyncNetworkTimeAsync() =>
        Task.FromException<TimeSyncResult>(new InvalidOperationException("Network time is only available in the desktop app."));

    // Browsers do not let pages float above other windows.
    public ValueTask SetAlwaysOnTopAsync(bool enabled) => ValueTask.CompletedTask;

    public async ValueTask<bool> CanFullscreenAsync() => (await browser.CapabilitiesAsync()).FullscreenEnabled;

    public ValueTask<bool> ToggleFullscreenAsync() => browser.ToggleFullscreenAsync();

    public ValueTask<bool> RequestNotificationPermissionAsync() => browser.RequestNotificationPermissionAsync();

    public ValueTask<bool> NotifyAsync(string title, string body) => browser.NotifyAsync(title, body);

    public async ValueTask<string?> SaveCalendarAsync(string fileName, string contents)
    {
        await browser.DownloadAsync(fileName, contents, "text/calendar;charset=utf-8");
        return null;
    }
}
