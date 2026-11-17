using GtaviCountdown.UI;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;

var builder = WebAssemblyHostBuilder.CreateDefault(args);
builder.RootComponents.Add<CountdownApp>("#app");
builder.Services.AddCountdownUI().AddBrowserPlatform();

await builder.Build().RunAsync();
