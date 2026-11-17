# GTA VI Countdown: C# port

A port of the countdown to .NET 10. The user interface is written once in Blazor and runs in two hosts:

- **Web**: Blazor WebAssembly, published as a static site.
- **Desktop**: [Photino.Blazor](https://github.com/tryphotino/photino.Blazor), a native window that uses the system web view (WebKitGTK on Linux, WebView2 on Windows, WKWebView on macOS).

Features, wording, layout and stored settings match the Svelte and Tauri version in the repository root. Both versions store settings in the same JSON format, so the two web builds read each other's settings when served from the same origin.

## Layout

| Project | Contents | Ported from |
| --- | --- | --- |
| `src/GtaviCountdown.Core` | Release data, time zone arithmetic, countdown maths, iCalendar writer, settings validation, theme palettes, calendar-file validation and the SNTP client. No UI dependencies. | `src/lib/time`, `src/lib/ics.ts`, `src/lib/settings.ts`, `src/lib/themes.ts`, `src-tauri/src/calendar.rs`, `src-tauri/crates/timesync` |
| `src/GtaviCountdown.UI` | Razor class library: every component with its scoped CSS, the fonts, and `wwwroot/js`, which holds the WebGL backdrop and thin wrappers over browser APIs. | `src/App.svelte`, `src/lib/components`, `src/lib/gl`, `src/lib/platform.ts` |
| `src/GtaviCountdown.Web` | WebAssembly host, `index.html`, icons, manifest and Cloudflare Pages headers. | `index.html`, `public/` |
| `src/GtaviCountdown.Desktop` | Photino host: network time, always on top, fullscreen, notifications, saving the calendar file, and window size and position kept between launches. | `src-tauri/src/lib.rs` |
| `tests/GtaviCountdown.Tests` | xUnit ports of every TypeScript and Rust test, including the loopback NTP server tests. | `tests/`, Rust `#[cfg(test)]` modules |

The UI reaches the host only through `IPlatform` (`src/GtaviCountdown.UI/Platform/IPlatform.cs`). The web host uses `BrowserPlatform`, which wraps browser APIs. The desktop host uses `DesktopPlatform`.

Work that must happen on every frame (the backdrop shader, the milliseconds readout, and measuring the layout the backdrop is composed around) stays in JavaScript. .NET re-renders at most once a second, on the tick where the seconds value changes.

## Requirements

- [.NET SDK 10](https://dotnet.microsoft.com/download).
- For the desktop app on Linux: WebKitGTK 4.1 and libnotify's `notify-send`. These are the same libraries the Tauri build uses:
  - Fedora: `sudo dnf install webkit2gtk4.1 libnotify`
  - Debian and Ubuntu: `sudo apt install libwebkit2gtk-4.1-0 libnotify-bin`
  - Arch: `sudo pacman -S webkit2gtk-4.1 libnotify`

## Commands

Run all commands from this directory (`csharp/`).

```sh
dotnet test                                       # build everything and run the tests
dotnet run --project src/GtaviCountdown.Web       # web app on http://localhost:1420
dotnet run --project src/GtaviCountdown.Desktop   # desktop app
```

### Publishing the web app

```sh
scripts/publish-web.sh            # writes the static site to dist/site
```

The output is a static site with relative URLs, so it can be served from any path. To deploy it to Cloudflare Pages:

```sh
bunx wrangler pages deploy dist/site --project-name gtavi-countdown
```

The script is needed because the build writes an import map inline into `index.html`. The Content Security Policy in `_headers` allows that script by its SHA-256 hash, which only exists after the build, so the script fills it in. Apart from that hash and `'wasm-unsafe-eval'`, which WebAssembly requires, the policy is the same as the original's.

### Publishing the desktop app

```sh
dotnet publish src/GtaviCountdown.Desktop -c Release -r linux-x64 --self-contained
```

Use `win-x64` or `osx-arm64` for other platforms. The `gtavi-countdown` executable is in `src/GtaviCountdown.Desktop/bin/Release/net10.0/<rid>/publish/`.

Settings and the window position are stored in `~/.config/gtavi-countdown/` on Linux and in `%APPDATA%\gtavi-countdown\` on Windows. Calendar files go to the download folder. On Linux that is the one set in `user-dirs.dirs`.

## Differences from the original

- **Download size.** The web build downloads the .NET runtime: about 2 MB with Brotli compression, much more than the Svelte build. ICU is left out (`InvariantGlobalization`) because every string is formatted with the invariant culture. Time zone data is still included.
- **Time zones** come from `TimeZoneInfo` instead of `Intl`. The zone picker lists the geographic IANA zones the runtime knows. On Windows that is the smaller set that maps from Windows zone IDs.
- **Notifications on Linux** go through `notify-send`, which needs no application registration. Windows and macOS use Photino's notification API.
- **Window state** is saved to a small JSON file when the window closes. The original used Tauri's window-state plugin.
