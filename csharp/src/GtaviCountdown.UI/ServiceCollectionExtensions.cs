using GtaviCountdown.UI.Platform;
using Microsoft.Extensions.DependencyInjection;

namespace GtaviCountdown.UI;

public static class ServiceCollectionExtensions
{
    /// <summary>
    /// Registers the services the countdown UI needs. Hosts add their own
    /// <see cref="IPlatform"/> afterwards; the browser one is the default.
    /// </summary>
    public static IServiceCollection AddCountdownUI(this IServiceCollection services)
    {
        services.AddScoped<BrowserInterop>();
        services.AddScoped<Clock>();
        return services;
    }

    /// <summary>Uses the browser for storage, notifications and downloads.</summary>
    public static IServiceCollection AddBrowserPlatform(this IServiceCollection services) =>
        services.AddScoped<IPlatform, BrowserPlatform>();
}
