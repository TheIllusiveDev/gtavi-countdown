using System.Globalization;

namespace GtaviCountdown.Core;

/// <summary>Colours handed to the backdrop shader, each as normalised RGB.</summary>
/// <param name="Stars">Star brightness from 0 to 1.</param>
/// <param name="Bands">Strength of the slits across the sun from 0 to 1.</param>
public sealed record Palette(
    float[] SkyTop,
    float[] SkyMid,
    float[] SkyLow,
    float[] SunHot,
    float[] SunCool,
    float[] Glow,
    float[] Water,
    float Stars,
    float Bands);

/// <param name="Fallback">CSS gradient used when the shader is disabled or WebGL is unavailable.</param>
/// <param name="Swatch">Three colours for the swatch in the settings panel.</param>
public sealed record ThemeInfo(string Label, Palette Palette, string Fallback, IReadOnlyList<string> Swatch);

public static class Themes
{
    /// <summary>Parses <c>#rrggbb</c> into normalised components.</summary>
    public static float[] Rgb(string hex)
    {
        var n = int.Parse(hex.AsSpan(1), NumberStyles.HexNumber, CultureInfo.InvariantCulture);
        return [((n >> 16) & 255) / 255f, ((n >> 8) & 255) / 255f, (n & 255) / 255f];
    }

    public static readonly IReadOnlyDictionary<Theme, ThemeInfo> Info = new Dictionary<Theme, ThemeInfo>
    {
        [Theme.Dusk] = new(
            "Dusk",
            new Palette(
                SkyTop: Rgb("#120a30"),
                SkyMid: Rgb("#7b1d72"),
                SkyLow: Rgb("#f2643d"),
                SunHot: Rgb("#ffe27a"),
                SunCool: Rgb("#ff3d81"),
                Glow: Rgb("#5c2410"),
                Water: Rgb("#1b0b3d"),
                Stars: 0.55f,
                Bands: 1f),
            "linear-gradient(180deg, #120a30 0%, #4a1560 34%, #a02a6c 52%, #f2643d var(--horizon, 66%), #2a1050 calc(var(--horizon, 66%) + 0.2%), #140a2c 100%)",
            ["#120a30", "#a02a6c", "#f2643d"]),
        [Theme.Midnight] = new(
            "Midnight",
            new Palette(
                SkyTop: Rgb("#020611"),
                SkyMid: Rgb("#0b2356"),
                SkyLow: Rgb("#157f8f"),
                SunHot: Rgb("#f6f9ff"),
                SunCool: Rgb("#a9c8ff"),
                Glow: Rgb("#0b2c44"),
                Water: Rgb("#030b1e"),
                Stars: 1f,
                Bands: 0f),
            "linear-gradient(180deg, #020611 0%, #081a40 36%, #0e4a70 54%, #157f8f var(--horizon, 66%), #06142e calc(var(--horizon, 66%) + 0.2%), #020611 100%)",
            ["#020611", "#0b2356", "#157f8f"]),
        [Theme.Dawn] = new(
            "Dawn",
            new Palette(
                SkyTop: Rgb("#10224f"),
                SkyMid: Rgb("#a8527f"),
                SkyLow: Rgb("#f7a765"),
                SunHot: Rgb("#fff1c2"),
                SunCool: Rgb("#ff9a5e"),
                Glow: Rgb("#4a2a16"),
                Water: Rgb("#142a55"),
                Stars: 0.2f,
                Bands: 0.6f),
            "linear-gradient(180deg, #10224f 0%, #5a3a70 34%, #b85f7d 52%, #f7a765 var(--horizon, 66%), #1c356a calc(var(--horizon, 66%) + 0.2%), #0f1f45 100%)",
            ["#10224f", "#a8527f", "#f7a765"]),
    };
}
