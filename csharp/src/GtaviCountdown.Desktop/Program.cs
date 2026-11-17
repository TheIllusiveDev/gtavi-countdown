using GtaviCountdown.Desktop;
using GtaviCountdown.UI;
using GtaviCountdown.UI.Platform;
using Microsoft.Extensions.DependencyInjection;
using Photino.Blazor;

/// <summary>
/// Desktop shell for the countdown: the same Blazor UI as the web app, hosted
/// in a native window with the system web view.
/// </summary>
internal static class Program
{
    [STAThread]
    private static void Main(string[] args)
    {
        var builder = PhotinoBlazorAppBuilder.CreateDefault(args);
        var holder = new WindowHolder();
        builder.Services.AddSingleton(holder);
        builder.Services.AddCountdownUI();
        builder.Services.AddScoped<IPlatform, DesktopPlatform>();
        builder.RootComponents.Add<CountdownApp>("#app");

        var app = builder.Build();
        var window = app.MainWindow;
        holder.Window = window;

        window
            .SetLogVerbosity(0)
            .SetTitle("GTA VI Countdown")
            .SetIconFile(Path.Combine(AppContext.BaseDirectory, "wwwroot", "app-icon.png"))
            .SetMinSize(360, 240)
            .SetUseOsDefaultSize(false)
            .SetSize(1280, 800)
            .Center()
            .SetDevToolsEnabled(false)
            .SetContextMenuEnabled(false);

        if (WindowState.Load() is { } state)
        {
            window.SetSize(state.Width, state.Height).SetLeft(state.Left).SetTop(state.Top);
            if (state.Maximized) window.SetMaximized(true);
        }

        window.WindowClosing += (_, _) =>
        {
            WindowState.Save(window);
            return false;
        };

        AppDomain.CurrentDomain.UnhandledException += (_, error) =>
            window.ShowMessage("GTA VI Countdown", error.ExceptionObject.ToString());

        app.Run();
    }
}
