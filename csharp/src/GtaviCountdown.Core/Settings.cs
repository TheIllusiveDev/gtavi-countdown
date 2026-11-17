using System.Text.Json;
using System.Text.Json.Nodes;

namespace GtaviCountdown.Core;

public enum Theme { Dusk, Midnight, Dawn }

/// <summary>Rendering budget for the animated backdrop.</summary>
public enum EffectLevel { Auto, Low, Off }

/// <param name="TimeZone">IANA zone the release midnight is evaluated in; <c>null</c> follows the system.</param>
/// <param name="Effects">Rendering budget for the animated backdrop.</param>
/// <param name="Notify">Send a notification at the moment of release.</param>
/// <param name="NetworkTime">Desktop only: correct the system clock against a network time server.</param>
/// <param name="AlwaysOnTop">Desktop only: keep the window above other windows.</param>
public sealed record SettingsData(
    string? TimeZone,
    bool ShowMilliseconds,
    Theme Theme,
    EffectLevel Effects,
    bool Notify,
    bool NetworkTime,
    bool AlwaysOnTop)
{
    public static readonly SettingsData Defaults = new(
        TimeZone: null,
        ShowMilliseconds: false,
        Theme: Theme.Dusk,
        Effects: EffectLevel.Auto,
        Notify: false,
        NetworkTime: true,
        AlwaysOnTop: false);
}

/// <summary>
/// The settings schema on disk and its validation. The stored form is the
/// same JSON object the web version writes, so both read each other's data.
/// </summary>
public static class SettingsCodec
{
    public const string StorageKey = "gtavi-countdown:settings:v1";

    public static readonly IReadOnlyList<Theme> Themes = Enum.GetValues<Theme>();
    public static readonly IReadOnlyList<EffectLevel> EffectLevels = Enum.GetValues<EffectLevel>();

    /// <summary>The lower-case name used in storage and in CSS: <c>dusk</c>, <c>auto</c>.</summary>
    public static string Key<T>(T value) where T : struct, Enum => value.ToString().ToLowerInvariant();

    private static bool TryParse<T>(JsonNode? node, out T value) where T : struct, Enum
    {
        value = default;
        if (node is not JsonValue v || !v.TryGetValue(out string? text)) return false;
        foreach (var candidate in Enum.GetValues<T>())
        {
            if (Key(candidate) == text)
            {
                value = candidate;
                return true;
            }
        }
        return false;
    }

    private static bool TryBool(JsonNode? node, out bool value)
    {
        value = false;
        return node is JsonValue v && v.GetValueKind() is JsonValueKind.True or JsonValueKind.False && v.TryGetValue(out value);
    }

    /// <summary>Merges untrusted stored JSON over the defaults, discarding invalid fields.</summary>
    public static SettingsData Sanitize(string? json)
    {
        var result = SettingsData.Defaults;
        if (string.IsNullOrWhiteSpace(json)) return result;

        JsonObject? raw;
        try
        {
            raw = JsonNode.Parse(json) as JsonObject;
        }
        catch (JsonException)
        {
            return result;
        }
        if (raw is null) return result;

        if (raw["timeZone"] is JsonValue zoneValue && zoneValue.TryGetValue(out string? zone) && Zones.IsValidTimeZone(zone))
            result = result with { TimeZone = zone };
        if (TryBool(raw["showMilliseconds"], out var millis)) result = result with { ShowMilliseconds = millis };
        if (TryParse(raw["theme"], out Theme theme)) result = result with { Theme = theme };
        if (TryParse(raw["effects"], out EffectLevel effects)) result = result with { Effects = effects };
        if (TryBool(raw["notify"], out var notify)) result = result with { Notify = notify };
        if (TryBool(raw["networkTime"], out var networkTime)) result = result with { NetworkTime = networkTime };
        if (TryBool(raw["alwaysOnTop"], out var onTop)) result = result with { AlwaysOnTop = onTop };
        return result;
    }

    public static string Serialize(SettingsData data) =>
        new JsonObject
        {
            ["timeZone"] = data.TimeZone,
            ["showMilliseconds"] = data.ShowMilliseconds,
            ["theme"] = Key(data.Theme),
            ["effects"] = Key(data.Effects),
            ["notify"] = data.Notify,
            ["networkTime"] = data.NetworkTime,
            ["alwaysOnTop"] = data.AlwaysOnTop,
        }.ToJsonString();

    /// <summary>The theme after <paramref name="theme"/>, wrapping around.</summary>
    public static Theme Next(Theme theme) => Themes[(Themes.ToList().IndexOf(theme) + 1) % Themes.Count];
}
