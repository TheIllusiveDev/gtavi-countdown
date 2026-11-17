using System.Text.Json;
using Photino.NET;

namespace GtaviCountdown.Desktop;

/// <summary>Remembers the size and position of the window between launches.</summary>
public sealed record WindowState(int Left, int Top, int Width, int Height, bool Maximized)
{
    private static readonly JsonSerializerOptions Options = new() { WriteIndented = true };

    public static WindowState? Load()
    {
        try
        {
            if (!File.Exists(AppPaths.WindowState)) return null;
            var state = JsonSerializer.Deserialize<WindowState>(File.ReadAllText(AppPaths.WindowState));
            // Ignore anything that would leave the window unusable.
            return state is { Width: >= 360 and <= 16384, Height: >= 240 and <= 16384 } ? state : null;
        }
        catch (Exception)
        {
            return null;
        }
    }

    public static void Save(PhotinoWindow window)
    {
        try
        {
            var state = new WindowState(window.Left, window.Top, window.Width, window.Height, window.Maximized);
            Directory.CreateDirectory(AppPaths.ConfigDirectory);
            File.WriteAllText(AppPaths.WindowState, JsonSerializer.Serialize(state, Options));
        }
        catch (Exception)
        {
            // Losing the window position is not worth failing over.
        }
    }
}
