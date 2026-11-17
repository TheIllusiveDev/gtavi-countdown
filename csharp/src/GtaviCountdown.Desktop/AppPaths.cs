namespace GtaviCountdown.Desktop;

/// <summary>Where the desktop app keeps its files.</summary>
public static class AppPaths
{
    public static string ConfigDirectory { get; } = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData, Environment.SpecialFolderOption.DoNotVerify),
        "gtavi-countdown");

    public static string Settings => Path.Combine(ConfigDirectory, "settings.json");

    public static string WindowState => Path.Combine(ConfigDirectory, "window-state.json");

    /// <summary>
    /// The user's download folder: the XDG setting on Linux, the conventional
    /// folder elsewhere, falling back to the documents and home folders.
    /// </summary>
    public static string Downloads()
    {
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        if (OperatingSystem.IsLinux() && XdgDownloadDir(home) is { } xdg) return xdg;

        var downloads = Path.Combine(home, "Downloads");
        if (Directory.Exists(downloads)) return downloads;
        var documents = Environment.GetFolderPath(Environment.SpecialFolder.MyDocuments);
        return string.IsNullOrEmpty(documents) ? home : documents;
    }

    /// <summary>Reads <c>XDG_DOWNLOAD_DIR</c> from <c>user-dirs.dirs</c>.</summary>
    private static string? XdgDownloadDir(string home)
    {
        try
        {
            var configHome = Environment.GetEnvironmentVariable("XDG_CONFIG_HOME");
            if (string.IsNullOrEmpty(configHome)) configHome = Path.Combine(home, ".config");
            var file = Path.Combine(configHome, "user-dirs.dirs");
            if (!File.Exists(file)) return null;
            foreach (var line in File.ReadLines(file))
            {
                var trimmed = line.Trim();
                if (!trimmed.StartsWith("XDG_DOWNLOAD_DIR=", StringComparison.Ordinal)) continue;
                var value = trimmed["XDG_DOWNLOAD_DIR=".Length..].Trim('"').Replace("$HOME", home);
                // A download folder set to the home folder itself means "none".
                return Path.IsPathRooted(value) && value.TrimEnd('/') != home.TrimEnd('/') ? value : null;
            }
        }
        catch (IOException)
        {
        }
        return null;
    }
}
