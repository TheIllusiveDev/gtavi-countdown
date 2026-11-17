using GtaviCountdown.Core;
using System.Text.Json.Nodes;
using Microsoft.AspNetCore.Components;
using Microsoft.JSInterop;

namespace GtaviCountdown.UI.Platform;

/// <summary>What the browser engine hosting the UI can do.</summary>
/// <remarks>
/// A class with setters rather than a positional record: the WebAssembly build
/// is trimmed, and trimming drops the constructor parameter names that JSON
/// deserialisation into a record depends on.
/// </remarks>
public sealed class BrowserCapabilities
{
    /// <summary>Phones and tablets: no hover and a coarse pointer.</summary>
    public bool IsTouch { get; set; }

    public bool FullscreenEnabled { get; set; }

    public bool Share { get; set; }

    public bool Notifications { get; set; }
}

/// <summary>
/// Typed access to <c>js/interop.js</c>, the browser half of the UI. The
/// module is imported on first use and shared by every component.
/// </summary>
public sealed class BrowserInterop(IJSRuntime js) : IAsyncDisposable
{
    private const string ModulePath = "./_content/GtaviCountdown.UI/js/interop.js";

    private Task<IJSObjectReference>? _module;

    private Task<IJSObjectReference> Module => _module ??= js.InvokeAsync<IJSObjectReference>("import", ModulePath).AsTask();

    private async ValueTask Void(string identifier, params object?[] args) =>
        await (await Module).InvokeVoidAsync(identifier, args);

    private async ValueTask<T> Call<T>(string identifier, params object?[] args) =>
        await (await Module).InvokeAsync<T>(identifier, args);

    public ValueTask<BrowserCapabilities> CapabilitiesAsync() => Call<BrowserCapabilities>("capabilities");

    /// <summary>
    /// Routes keyboard shortcuts, visibility and fullscreen changes to
    /// <paramref name="receiver"/>, which must expose <c>OnShortcut</c>,
    /// <c>OnResync</c> and <c>OnFullscreenChanged</c>.
    /// </summary>
    public ValueTask<IJSObjectReference> AttachWindowAsync<T>(DotNetObjectReference<T> receiver) where T : class =>
        Call<IJSObjectReference>("attachWindow", receiver);

    public ValueTask SetThemeAsync(Theme theme) => Void("setTheme", SettingsCodec.Key(theme));

    public ValueTask SetTitleAsync(string title) => Void("setTitle", title);

    public ValueTask<string?> StorageGetAsync(string key) => Call<string?>("storageGet", key);

    public ValueTask StorageSetAsync(string key, string value) => Void("storageSet", key, value);

    public ValueTask<bool> CopyTextAsync(string text) => Call<bool>("copyText", text);

    /// <summary>Resolves to "shared", "cancelled" when the user dismissed the sheet, or "failed".</summary>
    public ValueTask<string> ShareTextAsync(string title, string text) => Call<string>("shareText", title, text);

    public ValueTask<bool> ToggleFullscreenAsync() => Call<bool>("toggleFullscreen");

    public ValueTask<bool> RequestNotificationPermissionAsync() => Call<bool>("requestNotificationPermission");

    public ValueTask<bool> NotifyAsync(string title, string body) => Call<bool>("notify", title, body);

    public ValueTask DownloadAsync(string fileName, string contents, string type) =>
        Void("download", fileName, contents, type);

    public ValueTask InitDialogAsync<T>(ElementReference dialog, DotNetObjectReference<T> receiver) where T : class =>
        Void("initDialog", dialog, receiver);

    public ValueTask SetDialogOpenAsync(ElementReference dialog, bool open) => Void("setDialogOpen", dialog, open);

    /// <summary>Starts the per-frame milliseconds readout; the handle has <c>update</c> and <c>stop</c>.</summary>
    public ValueTask<IJSObjectReference> StartMillisAsync(ElementReference node, long target, double offset) =>
        Call<IJSObjectReference>("startMillis", node, target, offset);

    public ValueTask<bool> MountBackdropAsync(ElementReference canvas, Palette palette, EffectLevel effects) =>
        Call<bool>("mountBackdrop", canvas, ToJson(palette), Quality(effects));

    public ValueTask UpdateBackdropAsync(Palette palette, EffectLevel effects) =>
        Void("updateBackdrop", ToJson(palette), Quality(effects));

    public ValueTask UnmountBackdropAsync() => Void("unmountBackdrop");

    public ValueTask<IJSObjectReference> ObserveCompositionAsync(ElementReference hero, ElementReference details) =>
        Call<IJSObjectReference>("observeComposition", hero, details);

    /// <summary>
    /// The palette in the shape the renderer expects, built by hand so that
    /// no reflection is involved (see <see cref="BrowserCapabilities"/>).
    /// </summary>
    private static JsonObject ToJson(Palette p)
    {
        static JsonArray Rgb(float[] c) => new(c.Select(v => (JsonNode?)JsonValue.Create(v)).ToArray());
        return new JsonObject
        {
            ["skyTop"] = Rgb(p.SkyTop),
            ["skyMid"] = Rgb(p.SkyMid),
            ["skyLow"] = Rgb(p.SkyLow),
            ["sunHot"] = Rgb(p.SunHot),
            ["sunCool"] = Rgb(p.SunCool),
            ["glow"] = Rgb(p.Glow),
            ["water"] = Rgb(p.Water),
            ["stars"] = p.Stars,
            ["bands"] = p.Bands,
        };
    }

    private static string Quality(EffectLevel effects) => effects == EffectLevel.Low ? "low" : "auto";

    public async ValueTask DisposeAsync()
    {
        if (_module is null) return;
        try
        {
            await (await _module).DisposeAsync();
        }
        catch (JSDisconnectedException)
        {
            // The page is already gone.
        }
    }
}
