using GtaviCountdown.Core;
using GtaviCountdown.UI.Platform;
using Microsoft.AspNetCore.Components;
using Microsoft.JSInterop;

namespace GtaviCountdown.UI;

/// <summary>The whole countdown: the root component both hosts render.</summary>
public partial class CountdownApp
{
    /// <summary>How often the desktop app re-measures the clock offset.</summary>
    private static readonly TimeSpan ResyncInterval = TimeSpan.FromHours(6);

    private SettingsData _settings = SettingsData.Defaults;
    private string _zone = "UTC";
    private long _target;

    private bool _ready;
    private bool _settingsOpen;
    private bool _fullscreen;
    private bool _isTouch;
    private bool _canShare;
    private bool _canFullscreen;
    private SyncState _sync = new SyncState.Idle();
    private string _toast = "";
    private CancellationTokenSource? _toastTimer;
    private Timer? _resyncTimer;
    private bool? _wasElapsed;
    private string _title = "";

    private ElementReference _hero;
    private ElementReference _details;
    private DotNetObjectReference<CountdownApp>? _self;
    private IJSObjectReference? _window;
    private IJSObjectReference? _composition;

    // ------------------------------------------------------------- wording

    private Breakdown Parts => Countdown.Breakdown(Clock.Now, _target);

    private string TargetDate => Wording.LongDate(_target, _zone);

    private string TargetTime => Wording.Time(_target, _zone);

    private string ZoneText => Wording.ZoneText(_zone, _target);

    /// <summary>
    /// When the chosen zone differs from the device's, the same instant is
    /// also shown on the device's own clock, which is what the viewer will be reading.
    /// </summary>
    private string LocalEquivalent
    {
        get
        {
            var system = Zones.SystemTimeZone();
            if (Zones.OffsetAt(_target, system) == Zones.OffsetAt(_target, _zone)) return "";
            return Wording.ShortDateTime(_target, system);
        }
    }

    // ----------------------------------------------------------- lifecycle

    protected override async Task OnAfterRenderAsync(bool firstRender)
    {
        if (firstRender)
        {
            await StartAsync();
            return;
        }
        if (_ready && _composition is null)
        {
            // The backdrop is composed around the content, so it can only be
            // measured once the content exists.
            _composition = await Browser.ObserveCompositionAsync(_hero, _details);
        }
    }

    private async Task StartAsync()
    {
        _settings = SettingsCodec.Sanitize(await Platform.LoadSettingsAsync());
        var capabilities = await Browser.CapabilitiesAsync();
        _isTouch = capabilities.IsTouch;
        _canShare = !Platform.IsDesktop && capabilities.IsTouch && capabilities.Share;
        _canFullscreen = await Platform.CanFullscreenAsync();
        SetZone();

        _self = DotNetObjectReference.Create(this);
        _window = await Browser.AttachWindowAsync(_self);
        await Browser.SetThemeAsync(_settings.Theme);
        if (_settings.AlwaysOnTop) await SetAlwaysOnTopAsync(true);

        _wasElapsed = Parts.Elapsed;
        Clock.Ticked += OnTicked;
        Clock.Start(_target);
        ConfigureNetworkTime();

        _ready = true;
        StateHasChanged();
    }

    private void SetZone()
    {
        _zone = _settings.TimeZone ?? Zones.SystemTimeZone();
        _target = Countdown.ReleaseInstant(_zone);
    }

    // The clock may tick on a timer thread; the work is marshalled onto the
    // renderer before any state is touched.
    private void OnTicked() => _ = InvokeAsync(OnTickAsync);

    private async Task OnTickAsync()
    {
        var parts = Parts;

        // Fire the release notification on the tick that crosses zero.
        // Opening the app after release must not notify, hence the comparison
        // with the previous state rather than a check of the current one.
        if (_wasElapsed == false && parts.Elapsed)
        {
            ShowToast($"{Release.Title} is out now.");
            if (_settings.Notify) await Platform.NotifyAsync($"{Release.Title} is out now", "The wait is over.");
        }
        _wasElapsed = parts.Elapsed;

        // The window title follows the countdown at minute resolution.
        var title = parts.Elapsed
            ? "GTA VI is out"
            : FormattableString.Invariant($"{parts.Days}d {parts.Hours:00}h {parts.Minutes:00}m | GTA VI Countdown");
        if (title != _title)
        {
            _title = title;
            await Browser.SetTitleAsync(title);
        }

        StateHasChanged();
    }

    // ------------------------------------------------------------ settings

    private async Task UpdateSettingsAsync(Func<SettingsData, SettingsData> change)
    {
        var previous = _settings;
        var next = change(previous);
        if (next == previous) return;
        _settings = next;

        await Platform.SaveSettingsAsync(SettingsCodec.Serialize(next));
        if (next.Theme != previous.Theme) await Browser.SetThemeAsync(next.Theme);
        if (next.TimeZone != previous.TimeZone)
        {
            SetZone();
            // Re-phase the tick, since the target instant has moved.
            Clock.Retarget(_target);
        }
        if (next.AlwaysOnTop != previous.AlwaysOnTop) await SetAlwaysOnTopAsync(next.AlwaysOnTop);
        if (next.NetworkTime != previous.NetworkTime) ConfigureNetworkTime();
        StateHasChanged();
    }

    private async Task SetAlwaysOnTopAsync(bool enabled)
    {
        try
        {
            await Platform.SetAlwaysOnTopAsync(enabled);
        }
        catch (Exception)
        {
            // The window manager may refuse; the setting stays as the user left it.
        }
    }

    // Measure the clock at start-up and again every few hours: a machine that
    // sleeps, or a virtual machine whose clock drifts, would otherwise wander
    // away from the correction taken at launch.
    private void ConfigureNetworkTime()
    {
        _resyncTimer?.Dispose();
        _resyncTimer = null;
        if (!Platform.IsDesktop) return;
        if (!_settings.NetworkTime)
        {
            Clock.SetOffset(0);
            _sync = new SyncState.Idle();
            return;
        }
        _ = RunSyncAsync();
        _resyncTimer = new Timer(_ => InvokeAsync(RunSyncAsync), null, ResyncInterval, ResyncInterval);
    }

    private async Task RunSyncAsync()
    {
        if (!Platform.IsDesktop || _sync is SyncState.Syncing) return;
        var previous = _sync;
        _sync = new SyncState.Syncing();
        StateHasChanged();
        try
        {
            var result = await Platform.SyncNetworkTimeAsync();
            // The setting may have been switched off while the request was in flight.
            if (!_settings.NetworkTime) return;
            Clock.SetOffset(result.OffsetMs);
            _sync = new SyncState.Ok(result, Clock.Precise());
        }
        catch (Exception error)
        {
            if (!_settings.NetworkTime) return;
            // A failed refresh keeps the last good correction rather than
            // discarding it; only a first failure falls back to the system clock.
            _sync = previous is SyncState.Ok ? previous : new SyncState.Error(error.Message);
        }
        StateHasChanged();
    }

    private async Task SetNotifyAsync(bool enabled)
    {
        if (!enabled)
        {
            await UpdateSettingsAsync(s => s with { Notify = false });
            return;
        }
        bool granted;
        try
        {
            granted = await Platform.RequestNotificationPermissionAsync();
        }
        catch (Exception)
        {
            granted = false;
        }
        await UpdateSettingsAsync(s => s with { Notify = granted });
        ShowToast(granted
            ? "You will be notified at release while the app is open."
            : "Notifications are blocked for this app.");
    }

    // ------------------------------------------------------------- actions

    private void ShowToast(string message)
    {
        _toast = message;
        _toastTimer?.Cancel();
        _toastTimer = new CancellationTokenSource();
        var token = _toastTimer.Token;
        _ = InvokeAsync(async () =>
        {
            try
            {
                await Task.Delay(3600, token);
            }
            catch (TaskCanceledException)
            {
                return;
            }
            _toast = "";
            StateHasChanged();
        });
        StateHasChanged();
    }

    private async Task OnFullscreenAsync()
    {
        try
        {
            _fullscreen = await Platform.ToggleFullscreenAsync();
        }
        catch (Exception)
        {
            ShowToast("Fullscreen is not available here.");
        }
    }

    private async Task OnCopyAsync()
    {
        var now = Countdown.Breakdown(Clock.Precise(), _target);
        var text = now.Elapsed
            ? $"{Release.Title} has been out for {Countdown.FormatCompact(now)}."
            : $"{Release.Title} unlocks in {Countdown.FormatCompact(now)} ({TargetDate} at {TargetTime}, {ZoneText}).";
        // Phones get the system share sheet; a dismissed sheet needs no message.
        if (_canShare)
        {
            var outcome = await Browser.ShareTextAsync("GTA VI Countdown", text);
            if (outcome != "failed") return;
        }
        ShowToast(await Browser.CopyTextAsync(text) ? "Countdown copied to the clipboard." : "Copying failed.");
    }

    private async Task OnCalendarAsync()
    {
        var calendar = Ics.BuildCalendar(new CalendarEvent(
            Uid: $"release-{_target}@gtavi-countdown",
            Start: _target,
            DurationMinutes: 60,
            Summary: $"{Release.Title} release",
            Description: $"{Release.Title} unlocks on {TargetDate} at {TargetTime} ({ZoneText}). " +
                         $"Platforms at launch: {string.Join(", ", Release.Platforms)}.",
            Reminders: [24 * 60, 60]));
        try
        {
            var path = await Platform.SaveCalendarAsync("gta-vi-release.ics", calendar);
            ShowToast(path is not null ? $"Calendar event saved to {path}" : "Calendar event downloaded.");
        }
        catch (Exception error)
        {
            ShowToast($"Could not save the calendar event. {error.Message}");
        }
    }

    // ------------------------------------------------- calls from the page

    [JSInvokable]
    public async Task OnShortcut(string key)
    {
        switch (key)
        {
            case "s":
                _settingsOpen = !_settingsOpen;
                StateHasChanged();
                break;
            case "f":
                await OnFullscreenAsync();
                StateHasChanged();
                break;
            case "m":
                await UpdateSettingsAsync(s => s with { ShowMilliseconds = !s.ShowMilliseconds });
                break;
            case "t":
                await UpdateSettingsAsync(s => s with { Theme = SettingsCodec.Next(s.Theme) });
                break;
            case "c":
                await OnCopyAsync();
                break;
        }
    }

    /// <summary>The page became visible or focused again; timers may have been throttled meanwhile.</summary>
    [JSInvokable]
    public void OnResync() => Clock.Resync();

    [JSInvokable]
    public void OnFullscreenChanged(bool fullscreen)
    {
        _fullscreen = fullscreen;
        StateHasChanged();
    }

    public async ValueTask DisposeAsync()
    {
        Clock.Ticked -= OnTicked;
        Clock.Stop();
        _resyncTimer?.Dispose();
        _toastTimer?.Cancel();
        try
        {
            if (_window is not null)
            {
                await _window.InvokeVoidAsync("dispose");
                await _window.DisposeAsync();
            }
            if (_composition is not null)
            {
                await _composition.InvokeVoidAsync("dispose");
                await _composition.DisposeAsync();
            }
        }
        catch (JSDisconnectedException)
        {
            // The page is already gone.
        }
        _self?.Dispose();
    }
}
