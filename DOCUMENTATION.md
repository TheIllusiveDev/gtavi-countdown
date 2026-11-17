# GTA VI Countdown: Technical Documentation

This document describes how the application is built and why. It is written for developers who want to understand, modify or package the project. For installation and everyday commands, see [README.md](README.md).

The countdown exists in three versions: the original web application with its Tauri desktop shell (repository root), a C# port (`csharp/`) and a C++ port (`cpp/`). Most of this document describes the original, because the ports follow it closely: the same release data, timekeeping model, shader, wording and settings format. Where a port does something differently, the section says so, and sections 8 and 9 describe each port's own structure.

## Contents

1. [Architecture](#1-architecture)
2. [Technology choices](#2-technology-choices)
3. [The release moment](#3-the-release-moment)
4. [Timekeeping](#4-timekeeping)
5. [Rendering](#5-rendering)
6. [Settings](#6-settings)
7. [Desktop shell](#7-desktop-shell)
8. [The C# port](#8-the-c-port)
9. [The C++ port](#9-the-c-port)
10. [Module reference](#10-module-reference)
11. [Building, testing and verifying](#11-building-testing-and-verifying)
12. [Performance](#12-performance)
13. [Accessibility](#13-accessibility)
14. [Customisation](#14-customisation)
15. [Troubleshooting](#15-troubleshooting)
16. [Known limitations](#16-known-limitations)

## 1. Architecture

### Three versions

| | Original | C# port | C++ port |
| --- | --- | --- | --- |
| Shared logic | TypeScript modules in `src/lib/` | `GtaviCountdown.Core` | `cpp/src/core/` (static library) |
| User interface | Svelte components | Razor components in `GtaviCountdown.UI` | GTK 4 widgets in `cpp/src/app/` |
| Per-frame drawing | WebGL and DOM | WebGL and DOM, in JavaScript | OpenGL in a `Gtk::GLArea`, GSK snapshots |
| Desktop host | Tauri (Rust) | Photino.Blazor | the GTK application itself |
| SNTP client | `timesync` crate (Rust) | `TimeSync/Sntp.cs` | `sntp.cpp`, on POSIX sockets |

In every version the logic that can be tested without a user interface (time zones, countdown arithmetic, iCalendar output, settings validation, calendar-file validation and the SNTP client) is kept apart from the interface and has its own unit tests. The tests of the ports are ports of the original's TypeScript and Rust tests, so all three are checked against the same cases.

### The original

The original is a single-page web application with an optional native shell around it.

```text
+--------------------------------------------------------------+
|  Web application (src/)                                      |
|                                                              |
|   App.svelte ---- components/ ---- gl/ (WebGL backdrop)      |
|       |                                                      |
|       +---- time/      release data, zone maths, clock       |
|       +---- settings   validated, persisted preferences      |
|       +---- platform   the only module that knows about      |
|                |       the desktop shell                     |
+----------------|---------------------------------------------+
                 |  Tauri IPC (desktop only)
+----------------v---------------------------------------------+
|  Desktop shell (src-tauri/)                                  |
|                                                              |
|   sync_time ------- crates/timesync  (SNTP over UDP)         |
|   save_calendar --- calendar.rs      (input validation)      |
|   plugins:  notification, window-state                       |
+--------------------------------------------------------------+
```

Three properties shape the design:

- **The web application is complete on its own.** Everything except network time correction and the always-on-top option works in a browser. The desktop shell adds capabilities; it does not own any user interface.
- **One seam between the two halves.** `src/lib/platform.ts` is the only module that imports Tauri APIs. Each function in it has a browser implementation and a desktop implementation behind the same signature, and the Tauri modules are loaded with dynamic imports so that a browser never evaluates them.
- **Pure logic is separated from reactive state.** Time zone arithmetic, countdown arithmetic, calendar generation and settings validation are plain TypeScript modules with unit tests. Reactive state lives in two small `.svelte.ts` modules that wrap them.

## 2. Technology choices

**Bun** is the package manager, script runner and test runner. No Node.js-specific tooling is required.

**Svelte 5** compiles components to direct DOM operations with fine-grained reactivity. For this application that means a once-per-second tick updates exactly the text nodes whose digits changed, with no virtual DOM diff. The runtime that ships is small: the whole application, including the shader source, is about 34 kB of JavaScript after gzip.

**Vite 8** provides the development server and the production build. The build targets ES2022, which every supported browser and embedded webview implements, so no legacy transforms are emitted.

**Tauri 2** was chosen for the desktop shell over the alternatives for three reasons. It uses the operating system's webview instead of bundling a browser engine, which keeps the release binary at about 5 MB. Its backend is Rust, which made it straightforward to add a dependency-free SNTP client. And its capability system denies every native API by default, so the window is granted only the handful of permissions it uses.

**WebGL 1** is used for the backdrop rather than WebGL 2 or WebGPU. Nothing in the scene needs newer features, and WebGL 1 is available in every webview Tauri can run on, including WebKitGTK.

**Blazor** was chosen for the C# port because one set of Razor components can run both in the browser, on WebAssembly, and on the desktop. **Photino.Blazor** hosts those components in the system web view, the same approach as Tauri, instead of bundling a browser engine.

**GTK 4 and gtkmm** were chosen for the C++ port to show the countdown as a native Linux application without a web view. GTK 4 renders through the GPU, supports CSS custom properties for theming and lets widgets draw directly into its scene graph, which is what the custom-drawn figures and timeline need. **Meson** is the build system the GTK ecosystem uses, and it compiles the stylesheet and shaders into the binary as a GResource.

## 3. The release moment

The release is defined as a wall-clock time, not an instant: midnight at the start of November 19, 2026, in the viewer's region. The instant this denotes depends on the time zone. Midnight in Auckland arrives 21 hours before midnight in Los Angeles.

The definition lives in `src/lib/time/release.ts`:

```ts
export const RELEASE = {
  title: "Grand Theft Auto VI",
  year: 2026,
  month: 11,
  day: 19,
  hour: 0,
  minute: 0,
  platforms: ["PlayStation 5", "Xbox Series X|S"],
} as const;
```

`releaseInstant(timeZone)` in `countdown.ts` converts this wall-clock time to a UTC instant for a given IANA zone. The zone is the device's own unless the user selects another in the settings.

The date is the one announced by Rockstar Games. The unlock hour is the timing reported by the press at the time of writing, and the user interface says so. If either changes, `release.ts` is the only file to edit.

### Time zone arithmetic

JavaScript offers no direct way to ask which instant a wall-clock time denotes in an arbitrary zone. `src/lib/time/zones.ts` derives it from `Intl.DateTimeFormat`:

1. `wallTimeAt(instant, zone)` formats an instant in the zone and reads the parts back as numbers.
2. `offsetAt(instant, zone)` subtracts the instant from that wall time, treated as UTC, to obtain the zone's offset at that moment.
3. `zonedTimeToUtc(...)` inverts the mapping. It guesses the instant using the offset at the wall time read as UTC, then recomputes the offset at the guess. The second pass corrects the result when the first guess falls on the other side of a daylight-saving transition.

Formatters are cached per zone because constructing them is the expensive step.

The ports use their platform's time zone database instead: `TimeZoneInfo` in C# and the C++20 `<chrono>` time zone database (the system tzdata) in C++. Both use the same two-pass conversion.

Two calendar helpers build on this. `calendarDaysBetween` counts midnights between two instants in a zone, which is the number of "sleeps". `weekendsBetween` counts the Saturdays strictly between two dates. Both work on calendar dates rather than elapsed hours, so a daylight-saving change in between does not shift the count.

## 4. Timekeeping

### The clock

`src/lib/time/clock.svelte.ts` exports a single `clock` object with one reactive field, `now`. The rest of the user interface derives from it.

- **No accumulated error.** Every tick reads `Date.now()`. A timer that fires late, a throttled background tab or a laptop waking from sleep can delay an update, but the value shown afterwards is always correct.
- **Ticks are in phase with the target.** `msUntilNextSecond(now, target)` returns the time until the displayed seconds value next changes. The clock schedules its next tick for that moment, so the display changes when the remaining time crosses a whole second, not at an arbitrary phase set by when the page loaded. The final tick before release lands on the release instant itself.
- **One update per second.** `now` changes only on those ticks. Everything derived from it, including the totals and the progress bar, re-renders at most once per second.
- **Resynchronisation.** The clock re-reads the time and reschedules on `visibilitychange`, `focus` and `pageshow`, so a tab that was in the background is correct on the frame it becomes visible.

### Display rule

Remaining time is rounded down. With 5.25 seconds left the display shows 5, and it shows 0 during the last second. At the release instant the application switches to the released state and counts upwards from zero. The same rule applies to the milliseconds readout, so the two always agree.

### Milliseconds

The optional milliseconds readout changes on every frame. Routing sixty updates a second through the reactive graph would re-run every derivation that depends on the time. Instead, `Countdown.svelte` runs a `requestAnimationFrame` loop that reads `clock.precise()` and writes one text node directly. Nothing else in the page is touched, and the loop exists only while the readout is enabled.

### Network time correction

A countdown is only as accurate as the clock it reads. On the desktop, the application measures the system clock's error and corrects for it.

`clock.offset` is a number of milliseconds added to `Date.now()`. At start-up, and every six hours afterwards, the application calls the `sync_time` command, which performs an SNTP exchange in Rust and returns the measured offset. The clock applies it and re-phases its tick. If a later measurement fails, the last good correction is kept; if the first one fails, the system clock is used uncorrected and the settings panel reports the reason. The footer shows which source is in use.

The correction can be switched off in the settings, in which case no network request is made. Section 7 describes the protocol implementation and exactly what is sent.

## 5. Rendering

### Digits

Each digit is a fixed-size cell (`Digit.svelte`). When its value changes, the old glyph slides down and fades while the new one slides in from above. The motion is a Svelte transition with a `css` callback, which Svelte turns into a Web Animations API animation of `transform` and `opacity`. Both properties are animated by the compositor, so a roll costs no script time per frame and no layout.

The cells use `contain: size layout style`. Their size is fixed, so a change inside one can never trigger layout outside it. Paint is deliberately not contained, because clipping would cut the glyph's text shadow into a visible rectangle.

Digits are keyed by their position from the right. When the number of digits changes, for example from 100 days to 99, the remaining digits keep their identity and do not animate.

### Backdrop

The scene is a fragment shader (`src/lib/gl/shader.ts`) drawn on one full-screen triangle. It evaluates, per pixel:

- a three-stop sky gradient with hash-based stars that fade towards the horizon,
- a sun with a vertical colour grade and slits that open towards its lower edge,
- cloud streaks from four octaves of horizontally stretched value noise,
- two rows of procedurally generated buildings, the nearer with lit windows,
- water that mirrors the scene about the horizon, displaced by ripples that grow towards the viewer, with glitter in the column of light under the sun,
- a vignette and film grain. The grain also acts as dithering and removes the banding that smooth gradients show on 8-bit displays.

The sky function is evaluated once per pixel. Below the horizon it is called with mirrored coordinates rather than being rendered to a texture and sampled, which avoids a second pass.

The artwork is original and procedural. The project contains no images from the game.

### Composition follows the layout

The horizon and the sun are positioned by the page, not fixed in the shader. `App.svelte` measures the bottom of the timer and the top of the lower panel with a `ResizeObserver` and passes two numbers to the renderer: the height of the horizon and the radius of the sun. The sun therefore always sits in the gap beneath the digits and the skyline always meets the top of the panel, on any window size and in every layout tier. The same horizon value is exposed to CSS as `--horizon` so that the static fallback gradient lines up with it.

### Protecting the frame rate

`src/lib/gl/scene.ts` wraps the shader in a renderer that manages its own cost:

| Mechanism | Behaviour |
| --- | --- |
| Pixel ratio cap | The canvas is rendered at no more than 1.5 device pixels per CSS pixel. The scene is soft and gains nothing beyond that. |
| Adaptive resolution | Frame times are measured. After 24 frames slower than 1/42 s the internal resolution steps down through 100%, 75%, 55%, 40% and 30%. It does not step back up, which avoids oscillation. |
| Visibility | Rendering stops while the document is hidden and resumes when it is shown. |
| Low preset | Renders at 55% resolution and about 30 frames per second. |
| Off | The canvas is removed and its context released; a CSS gradient is shown instead. |
| Reduced motion | When the operating system asks for reduced motion, one still frame is drawn and redrawn only on resize or theme change. |
| Context loss | The renderer rebuilds its program when the context is restored. |
| No WebGL | The CSS gradient stays visible. Nothing else depends on the canvas. |

### Layout tiers

The page is laid out for one screen without scrolling at common desktop sizes, and adapts by height as well as width:

| Condition | Layout |
| --- | --- |
| Width over 640 px, height over 860 px | Full layout. |
| Height up to 860 px | Tighter vertical spacing. |
| Height up to 660 px | Footer hidden. |
| Height up to 560 px | Timeline and totals hidden; the timer is centred. This is the compact, widget-like mode. |
| Height up to 400 px | Title and timer only. |
| Width up to 640 px | Single column; the page may scroll. Intermediate timeline labels are hidden. |

## 6. Settings

The web builds store settings in `localStorage` under the key `gtavi-countdown:settings:v1` as one JSON object. The C# and C++ desktop applications write the same object to `~/.config/gtavi-countdown/settings.json` (`%APPDATA%\gtavi-countdown\settings.json` on Windows), so they share it. Each version validates the object with its own port of the same rules.

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `timeZone` | IANA zone name or `null` | `null` | Zone in which release midnight is evaluated. `null` follows the device. |
| `showMilliseconds` | boolean | `false` | Show the per-frame milliseconds readout. |
| `theme` | `"dusk"`, `"midnight"` or `"dawn"` | `"dusk"` | Colour palette. |
| `effects` | `"auto"`, `"low"` or `"off"` | `"auto"` | Rendering budget for the backdrop. |
| `notify` | boolean | `false` | Send a notification at release. |
| `networkTime` | boolean | `true` | Desktop only. Correct the clock over NTP. |
| `alwaysOnTop` | boolean | `false` | Desktop only. Keep the window above others. |

`sanitizeSettings` in `src/lib/settings.ts` (and `SettingsCodec.Sanitize` and `settings::sanitize` in the ports) treats stored data as untrusted. It starts from the defaults and copies across only fields that have the right type and an allowed value, including a check that a stored zone name is one the runtime recognises. Corrupt or outdated data therefore degrades to defaults field by field instead of breaking the application. Storage failures, such as a private window with storage disabled, are caught and ignored.

## 7. Desktop shell

This section describes the Tauri shell of the original. The C# and C++ desktop applications implement the same capabilities; sections 8 and 9 describe how.

The shell is the Cargo workspace in `src-tauri/`. It contains the application crate and one library crate, `timesync`.

### Commands

| Command | Arguments | Returns | Purpose |
| --- | --- | --- | --- |
| `sync_time` | none | `{ offsetMs, roundTripMs, server, stratum }` | Measures the system clock error. Runs on the blocking thread pool so the window never stalls on the network. |
| `save_calendar` | `fileName`, `contents` | The path written | Writes an `.ics` file to the user's download folder, falling back to the documents folder and then the home folder. |

### The timesync crate

`src-tauri/crates/timesync` is an SNTP client (RFC 4330) that uses only the Rust standard library.

For each measurement it sends a 48-byte client packet carrying the local send time, validates the reply and applies the standard calculation:

```text
offset = ((t2 - t1) + (t3 - t4)) / 2
delay  =  (t4 - t1) - (t3 - t2)
```

where `t1` and `t4` are the local send and receive times and `t2` and `t3` are the server's receive and transmit times.

Details worth knowing:

- The interval between `t1` and `t4` is measured with a monotonic clock, so a clock adjustment during the exchange cannot corrupt the result.
- Replies are rejected unless they are server-mode packets, come from a synchronised server (not stratum 0, not the alarm leap indicator, non-zero transmit time) and echo the exact timestamp that was sent. The socket is connected to the server's address, so datagrams from other sources are discarded by the operating system.
- Three exchanges are made and the one with the shortest round trip is used, because the possible error is bounded by half the round trip.
- Timestamp conversion handles the NTP era rollover in 2036.
- Servers are tried in order: `time.cloudflare.com`, `pool.ntp.org`, `time.google.com`. Each exchange has a 1.5 second timeout.

**What is sent.** The request contains the protocol header and the local time. It carries no identifier, and nothing about the user or the application. This is the same exchange an operating system performs when it sets its clock.

### Calendar validation

`save_calendar` receives its arguments from the webview and treats them as untrusted. `src-tauri/src/calendar.rs` accepts only a bare file name of letters, digits, hyphens, underscores and dots that ends in `.ics` and does not begin with a dot, and only content that is at most 64 KiB and is delimited as an iCalendar object. Path separators, parent-directory segments and other extensions are rejected, so the command cannot be used to write an arbitrary file to an arbitrary location.

### Permissions

`src-tauri/capabilities/default.json` grants the main window:

- `core:default`
- `core:window:allow-set-always-on-top`
- `core:window:allow-set-fullscreen`
- `core:window:allow-is-fullscreen`
- `notification:default`
- `window-state:default`

No file system, shell, HTTP or dialog permissions are granted. The two commands above are the only application-defined entry points.

### Content security policy

The production policy in `tauri.conf.json` restricts every resource type to the application's own bundle:

```text
default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline';
img-src 'self' data:; font-src 'self'; connect-src ipc: http://ipc.localhost
```

The page cannot load remote scripts, styles, fonts or images, and cannot make network requests other than IPC calls to the shell. A separate, slightly wider `devCsp` allows the Vite development server and its hot-reload socket.

### Plugins

- `tauri-plugin-window-state` restores the window's size and position between launches.
- `tauri-plugin-notification` delivers the release notification through the operating system.

### Release profile

The release profile optimises for size: one codegen unit, link-time optimisation, `opt-level = "s"`, `panic = "abort"` and stripped symbols.

## 8. The C# port

The C# port in `csharp/` targets .NET 10. Its user interface is written once, as a Razor class library, and runs in two hosts.

| Project | Contents | Ported from |
| --- | --- | --- |
| `GtaviCountdown.Core` | Release data, time zone arithmetic, countdown maths, iCalendar writer, settings validation, theme palettes, calendar-file validation and the SNTP client. No UI dependencies. | `src/lib/time`, `src/lib/ics.ts`, `src/lib/settings.ts`, `src/lib/themes.ts`, `src-tauri/src/calendar.rs`, `src-tauri/crates/timesync` |
| `GtaviCountdown.UI` | Every component with its scoped CSS, the fonts, and `wwwroot/js`, which holds the WebGL backdrop and thin wrappers over browser APIs. | `src/App.svelte`, `src/lib/components`, `src/lib/gl`, `src/lib/platform.ts` |
| `GtaviCountdown.Web` | WebAssembly host, `index.html`, icons, manifest and Cloudflare Pages headers. | `index.html`, `public/` |
| `GtaviCountdown.Desktop` | Photino host: network time, always on top, fullscreen, notifications, saving the calendar file, and the window size and position. | `src-tauri/src/lib.rs` |
| `GtaviCountdown.Tests` | xUnit ports of every TypeScript and Rust test, including the loopback NTP server tests. | `tests/`, Rust `#[cfg(test)]` modules |

### One seam

As in the original, the user interface reaches the host only through one interface, `IPlatform` (`GtaviCountdown.UI/Platform/IPlatform.cs`). The web host uses `BrowserPlatform`, which wraps browser APIs; the desktop host uses `DesktopPlatform`.

### What stays in JavaScript

.NET re-renders at most once a second, on the tick where the seconds value changes. Work that must happen on every frame stays in JavaScript: the backdrop shader (`wwwroot/js/scene.js`, the same renderer as the original), the milliseconds readout, and measuring the layout the backdrop is composed around. The clock (`Clock.cs`) follows the model in section 4, with a `System.Threading.Timer` whose ticks are marshalled onto the renderer.

### Differences from the original

- **Download size.** The web build downloads the .NET runtime: about 2 MB with Brotli compression, much more than the Svelte build. ICU is left out (`InvariantGlobalization`) because every string is formatted with the invariant culture. Time zone data is still included.
- **Time zones** come from `TimeZoneInfo` instead of `Intl`. The zone picker lists the geographic IANA zones the runtime knows. On Windows that is the smaller set that maps from Windows zone IDs.
- **Content security policy.** The build writes an import map inline into `index.html`. `scripts/publish-web.sh` computes its SHA-256 hash and adds it to `_headers`. Apart from that hash and `'wasm-unsafe-eval'`, which WebAssembly requires, the policy is the same as the original's.
- **Notifications on Linux** go through `notify-send`, which needs no application registration. Windows and macOS use Photino's notification API.
- **Window state** is saved to a small JSON file when the window closes, instead of by Tauri's window-state plugin.

## 9. The C++ port

The C++ port in `cpp/` is a native GTK 4 application for Linux, written in C++23 against gtkmm 4 and built with Meson. It draws everything itself; there is no web view.

### Structure

- **`cpp/src/core/`** is a static library with no GTK dependency: release data, time zones on the C++20 `<chrono>` time zone database, countdown maths, English date wording, the iCalendar writer, settings validation with nlohmann/json, theme palettes, calendar-file validation and an SNTP client on POSIX sockets. The GoogleTest suite in `cpp/tests/` covers it.
- **`cpp/src/app/`** is the application. `MainWindow` plays the part of `App.svelte`; each component of the original has a widget of its own.
- **`cpp/data/`** holds the stylesheet, the two shader files and the icon, compiled into the binary as a GResource, plus the fonts and the desktop entry, which are installed alongside it.

### Styling and layout

`data/style.css` is the design of the web version translated to GTK CSS. Theme colours are CSS custom properties on the window, switched by a `theme-dusk`, `theme-midnight` or `theme-dawn` class. The custom-drawn widgets take the same colours from `ui_colors.hpp`.

GTK has no media queries and no `clamp()` over the viewport. `MainWindow::apply_layout` therefore applies the layout tiers of section 5 itself whenever the window size changes, and rewrites a second, small style provider with the fluid sizes (title, stats figures, spacing). The change is applied from an idle callback, because changing styles during size allocation would re-enter it. Widgets whose size follows the window must not hold the window at its old size, so the countdown measures a small minimum and scales its drawing down for the moment until the next pass resizes it, the details panel sits in a `Clamp` container that gives it the available width up to 980 pixels, and labels that could block shrinking are allowed to ellipsize.

### Drawing

| Widget | How it draws |
| --- | --- |
| `Backdrop` | The fallback gradient and the scrim as GSK gradient nodes, with the shader scene between them in a `Gtk::GLArea`. |
| `CountdownView` | Figures, colons, unit labels and the milliseconds readout as Pango layouts and shapes on a `Gtk::Snapshot`. The roll of a changing figure and the breathing colons are driven by frame-clock tick callbacks, which run only while something moves. |
| `TimelineTrack` | The track, fill, milestone dots and legend, with tooltips from a hit test on the dots. |
| `Icon` | The same SVG path data as the web icons, stroked with a GSK path in the widget's CSS colour. |

Everything else (header, stats, footer, settings sheet, toast) is ordinary GTK widgets styled by the stylesheet.

### The backdrop on OpenGL

`backdrop.cpp` loads `scene.vert` and `scene.frag` from the resource bundle and prepends `#version 150` for an OpenGL 3.2 core context or `#version 300 es` for an OpenGL ES context, whichever GTK provides. The renderer behaves like the WebGL one in section 5:

- the pixel ratio is capped at 1.5 and the same adaptive governor steps the resolution down after a run of slow frames; reduced resolutions are rendered to an offscreen framebuffer and scaled up with `glBlitFramebuffer`,
- the Low preset renders at 55% resolution and about 30 frames per second,
- Off removes the GL area and releases its context,
- when the desktop disables animations (`gtk-enable-animations`), one still frame is drawn and the tick callback is removed,
- if a context cannot be created or the shader fails to compile, the GL area is removed and the gradient stays visible,
- the composition follows the layout: after each allocation the window measures the bottom of the hero and the top of the details panel and passes the horizon and sun radius to the renderer and to the fallback gradient.

### Fonts

The bundled fonts are TrueType files, not the WOFF2 files of the web versions. Pango reads font data through HarfBuzz, which does not decode WOFF2, so a WOFF2 file is accepted by fontconfig and then silently replaced by a fallback font. The TTF files were produced from the WOFF2 files with FreeType and keep their variable `wght` and `wdth` axes. They are registered for the process only with `pango_font_map_add_font_file` and are not installed system-wide. The Archivo subset carries the family name "Archivo SemiBold", which is the name the stylesheet and the drawing code ask for.

### Desktop capabilities

| Capability | Implementation |
| --- | --- |
| Network time | `platform::sync_network_time` calls the core SNTP client with the same servers, timeout and sample count as the Tauri shell. It runs on a background thread (`background.hpp`), and the result is posted back to the GLib main loop and dropped if the window has closed. |
| Clock | `Clock` re-reads the system clock on every tick and schedules the next one with a high-priority GLib timeout on the boundary from `ms_until_next_second`. It also resynchronises when the window becomes active. |
| Notifications | A `GNotification` through GIO, which needs a running notification service. |
| Always on top | GTK 4 has no API for it. On X11 the window manager is asked directly with an EWMH `_NET_WM_STATE_ABOVE` message. Wayland offers no such request, so the option is unavailable there and the pin button is hidden. |
| Calendar | The file is validated with the same rules as `calendar.rs` and written atomically to the download folder. |
| Window state | Size and maximised state are saved to `window-state.json` on close. GTK 4 cannot place a window, so the position is left to the window manager. |
| Window decorations | The header is part of the content so that the backdrop shows behind it. An empty title bar keeps client-side decorations, and `Gtk::WindowControls` draws the window buttons in the header. The whole content is a `Gtk::WindowHandle`, so it can be dragged. |

## 10. Module reference

### Web application

| Path | Responsibility |
| --- | --- |
| `src/main.ts` | Mounts the root component. |
| `src/App.svelte` | Layout, derived wording, composition measurement, keyboard shortcuts, clipboard, calendar, notification and clock-sync actions. |
| `src/app.css` | Font faces, design tokens per theme, resets and shared utility classes. |
| `src/lib/time/release.ts` | The release date and the milestone list. |
| `src/lib/time/zones.ts` | Zone offsets, wall-time conversion, zone listing and calendar-day arithmetic. |
| `src/lib/time/countdown.ts` | Release instant, breakdown into units, tick scheduling, totals and timeline positions. |
| `src/lib/time/clock.svelte.ts` | The reactive clock. |
| `src/lib/settings.ts` | Settings schema, defaults and validation. |
| `src/lib/settings.svelte.ts` | The reactive, persisted settings store. |
| `src/lib/themes.ts` | Shader palettes, fallback gradients and swatches for each theme. |
| `src/lib/ics.ts` | iCalendar generation with escaping and line folding per RFC 5545. |
| `src/lib/platform.ts` | Environment detection and the browser and desktop implementations of each native capability. |
| `src/lib/gl/shader.ts` | GLSL source for the backdrop. |
| `src/lib/gl/scene.ts` | WebGL renderer with adaptive quality. |
| `src/lib/components/Backdrop.svelte` | Hosts the canvas and the fallback gradient. |
| `src/lib/components/Countdown.svelte` | The four units, the separators and the milliseconds readout. |
| `src/lib/components/Digit.svelte` | One rolling digit. |
| `src/lib/components/Timeline.svelte` | Progress since the first trailer, with milestones. |
| `src/lib/components/Stats.svelte` | Sleeps, weekends and totals. |
| `src/lib/components/SettingsPanel.svelte` | The settings drawer, built on the native `dialog` element. |
| `src/lib/components/ZonePicker.svelte` | Filterable list of time zones, ordered by when they reach release. |
| `src/lib/components/Switch.svelte` | Accessible toggle. |
| `src/lib/components/Icon.svelte` | Inline SVG icons. |

### Desktop shell

| Path | Responsibility |
| --- | --- |
| `src-tauri/src/main.rs` | Binary entry point. |
| `src-tauri/src/lib.rs` | Application setup and the two commands. |
| `src-tauri/src/calendar.rs` | Validation of calendar exports. |
| `src-tauri/crates/timesync/src/lib.rs` | SNTP client. |
| `src-tauri/tauri.conf.json` | Window, security policy, build hooks and bundle metadata. |
| `src-tauri/capabilities/default.json` | Permissions for the main window. |
| `src-tauri/app-icon.svg` | Source artwork for the icons in `src-tauri/icons/`. |

### C# port

The Core project has one file per original module: `Release.cs`, `Zones.cs`, `Countdown.cs`, `Wording.cs`, `Ics.cs`, `Settings.cs`, `Themes.cs`, `CalendarFile.cs` and `TimeSync/Sntp.cs`. The UI project has one Razor component per Svelte component under `Components/`, with `CountdownApp.razor` as the root and `Clock.cs` as the clock.

### C++ port

| Path | Responsibility |
| --- | --- |
| `cpp/src/core/release.*` | The release date and the milestone list. |
| `cpp/src/core/zones.*` | Zone offsets, wall-time conversion, zone listing and calendar-day arithmetic. |
| `cpp/src/core/countdown.*` | Release instant, breakdown into units, tick scheduling, totals and timeline positions. |
| `cpp/src/core/wording.*` | Dates, times, zone names and numbers as shown in the interface. |
| `cpp/src/core/ics.*` | iCalendar generation. |
| `cpp/src/core/settings.*` | Settings schema, validation and serialisation. |
| `cpp/src/core/themes.*` | Shader palettes, fallback gradients and swatches. |
| `cpp/src/core/calendar_file.*` | Validation of calendar exports. |
| `cpp/src/core/sntp.*` | SNTP client. |
| `cpp/src/app/main.cpp` | Application start-up: fonts, stylesheet and the single window. |
| `cpp/src/app/main_window.*` | Layout and layout tiers, wording, actions, shortcuts, settings changes and network time. |
| `cpp/src/app/backdrop.*` | The OpenGL scene, fallback gradient and scrim. |
| `cpp/src/app/countdown_view.*` | The four units, the colons and the milliseconds readout. |
| `cpp/src/app/timeline_view.*` | Progress since the first trailer, with milestones. |
| `cpp/src/app/stats_view.*` | Sleeps, weekends and totals, in three arrangements. |
| `cpp/src/app/settings_panel.*` | The settings sheet and the time zone picker. |
| `cpp/src/app/clock.*` | The once-per-second clock. |
| `cpp/src/app/platform.*` | Settings file, network time, notifications, always on top and the calendar file. |
| `cpp/src/app/app_paths.*`, `window_state.*` | File locations and the saved window size. |
| `cpp/src/app/background.hpp` | Blocking work on a thread, with the result handed back to the main loop. |
| `cpp/src/app/icon.*`, `drawing.hpp`, `clamp.hpp`, `ui_colors.hpp` | Icons, drawing helpers, the max-width container and theme colours. |
| `cpp/data/` | Stylesheet, shaders, fonts, icon and desktop entry. |

## 11. Building, testing and verifying

### Commands

The original:

```sh
bun install               # install dependencies
bun run check             # type-check TypeScript and Svelte
bun test                  # TypeScript unit tests
bun run build             # production web build

cd src-tauri
cargo test --workspace    # Rust unit tests
cd ..
bun run desktop:build     # release binary and installers
```

The C# port, from `csharp/`:

```sh
dotnet test                                       # build and run the xUnit tests
dotnet run --project src/GtaviCountdown.Desktop   # desktop app
scripts/publish-web.sh                            # static site in dist/site
```

The C++ port, from `cpp/`:

```sh
meson setup build
ninja -C build
meson test -C build       # GoogleTest suite
```

The C++ build uses `warning_level=3` and compiles without warnings.

### What the tests cover

**TypeScript (`tests/`, 45 tests)**

- `zones.test.ts`: offsets including fractional and daylight-saving cases, release midnight in eight zones, conversions on both sides of a daylight-saving change, round trips, formatting, and the sleeps and weekends counts.
- `countdown.test.ts`: unit breakdown, behaviour at and after the target, tick scheduling (including that each scheduled tick lands exactly on a change of the display and not a millisecond late), totals, progress and milestone ordering.
- `ics.test.ts`: date formatting, escaping, folding at 75 octets without splitting multi-byte characters, and the structure of a generated calendar.
- `settings.test.ts`: validation of stored settings against malformed, partial and hostile input.

**Rust (`src-tauri/`, 15 tests)**

- `timesync`: packet encoding, timestamp conversion and the 2036 era rollover, the offset formula, rejection of each kind of invalid reply, and three tests against a local UDP server: a known clock skew is measured, a dead server is skipped, and total failure is reported.
- Application crate: calendar file name and content validation, including path traversal attempts.

**C# (`csharp/tests/`)** ports every TypeScript and Rust test above to xUnit, including the loopback NTP server tests.

**C++ (`cpp/tests/`, 102 tests)** ports the same cases to GoogleTest: zones, countdown, iCalendar, settings, calendar-file validation and the SNTP client with its loopback server tests. The GTK application has no automated tests.

### Regenerating icons

The icons are generated from the SVG source:

```sh
bun tauri icon src-tauri/app-icon.svg
```

The command also produces mobile and Windows Store variants; only the desktop icons referenced in `tauri.conf.json` are kept in the repository.

### Packaging

`bun run desktop:build` produces the installers appropriate to the host platform: `.deb`, `.rpm` and AppImage on Linux, `.msi` and NSIS on Windows, `.app` and `.dmg` on macOS. Installers are built for the host platform, so build on the platform you are targeting. The bundle identifier, `app.gtavi.countdown`, is set in `tauri.conf.json` and should be changed to a domain you control before distributing builds.

The web build in `dist/` can be deployed to any static host as it is. For Cloudflare Pages, `bun run deploy` builds and uploads it with Wrangler (configured in `wrangler.toml`), and `public/_headers` supplies the response headers; the README has the dashboard settings for Git-based deploys.

The home-screen icons in `public/icons/` are square, full-bleed renders of `src-tauri/app-icon.svg` (iOS and Android apply their own mask).

The C++ application installs with `meson install`: the binary, the fonts in `share/gtavi-countdown/fonts`, the desktop entry and the scalable icon, all under the configured prefix. The desktop entry's application ID, `app.gtavi.countdown`, is also the GTK application ID; change both before distributing builds.

## 12. Performance

Measured figures for the original, from a production build:

| Metric | Value |
| --- | --- |
| JavaScript, gzip | about 34 kB for the application, plus about 5 kB of Tauri API chunks loaded only on the desktop |
| CSS, gzip | about 5 kB |
| Fonts | three WOFF2 files, about 217 kB in total; only the Latin subsets are bundled |
| Idle script time, backdrop off | about 2.4 ms per second (headless Chromium) |
| Layouts while idle | about 2 per second |
| JavaScript heap | under 3 MB |
| Release binary (Linux x86-64) | about 5.2 MB |
| Debian package | about 2.4 MB |

The cost of the backdrop depends on the GPU and is governed by the mechanisms in section 5.

The C++ binary is about 0.9 MB after stripping, built with the default `debugoptimized` build type, and the three TrueType fonts add about 570 kB. Like the original it redraws text at most once a second; only the backdrop, a rolling figure, the breathing colons and the milliseconds readout draw on every frame, each through its own tick callback.

The techniques behind these numbers:

- One reactive update per second, scheduled on the boundary where the display changes.
- Digit animations run on the compositor.
- The milliseconds readout bypasses the reactive graph and writes one text node.
- The progress bar is scaled with a transform instead of being resized.
- The time zone list uses `content-visibility: auto`, so rows outside the scroll viewport are not laid out or painted, and it is built only while the settings panel is open.
- No `backdrop-filter` is used. Panels are translucent solid colours, which avoids re-blurring the animated canvas every frame.
- Fonts are variable: one Archivo file supplies every weight and width in the interface.

## 13. Accessibility

- The timer is exposed with `role="timer"` and a label that reads the days, hours and minutes. The label changes once a minute, not once a second, and the animated digits are hidden from assistive technology.
- Every control is a native `button` or `input` with a text label. Toggles use `role="switch"`, and the theme and backdrop selectors are radio groups.
- The settings drawer is a native modal `dialog`, which provides the focus trap, Escape handling and inert background.
- Focus indicators are always visible for keyboard users.
- With the reduced motion preference set, digits change without animation, the backdrop is a still frame and transitions are disabled.
- Text over the scene is protected by a scrim and text shadows so that contrast holds in every theme.
- In the C++ application the countdown widget has the timer role and the same spoken label, every control has an accessible label, toasts are announced, and the GTK animation setting stands in for the reduced motion preference. GTK has no inert background, so while the settings sheet is open the scrim takes pointer input and the content behind it cannot take keyboard focus.

## 14. Customisation

### Changing the date or hour

Edit `RELEASE` in `src/lib/time/release.ts`, `Release` in `csharp/src/GtaviCountdown.Core/Release.cs` and `release` in `cpp/src/core/release.hpp`. Tests that assert the specific date will need the same change.

### Adding a milestone

Append an entry to the milestone list in the same files. Entries are ordered by date; the first is the origin of the progress bar.

### Adding a theme

1. Add the name to `THEMES` in `src/lib/settings.ts`.
2. Add an entry to `THEME_INFO` in `src/lib/themes.ts` with the shader palette, a fallback gradient and three swatch colours.
3. Add a `:root[data-theme="..."]` block to `src/app.css` with the accent, panel and scrim colours.
4. Make the same changes in the ports: `Settings.cs`, `Themes.cs` and `app.css` in the C# port; `settings.cpp`, `themes.cpp`, `ui_colors.hpp` and a `window.countdown.theme-...` block in `data/style.css` in the C++ port.

### Changing the time servers

Edit `TIME_SERVERS` in `src-tauri/src/lib.rs`, the server list in `csharp/src/GtaviCountdown.Desktop/DesktopPlatform.cs` or `time_servers` in `cpp/src/app/platform.cpp`. Entries are `host:port` strings and are tried in order.

## 15. Troubleshooting

**The desktop build fails with a message about `pkg-config` or a missing `.pc` file.**
The Tauri system dependencies are not installed. Install the packages listed in the README for your distribution.

**The desktop window is blank or shows a connection error.**
A binary produced by a plain `cargo build` loads the development server at `http://localhost:1420`. Use `bun run desktop:dev`, which starts that server, or `bun run desktop:build`, which embeds the built assets.

**The backdrop looks soft or blocky.**
The adaptive governor has lowered the rendering resolution because frames were slow. This is expected with software rendering, which is common under WSL 2 and in virtual machines. Set the backdrop to Off in the settings for the lightest option.

**The backdrop is a plain gradient.**
WebGL is unavailable or disabled in the webview, or the backdrop is set to Off.

**The footer says "System clock" on the desktop.**
Network time is switched off, or no time server could be reached. The settings panel shows the reason. UDP port 123 must be allowed outbound.

**No notification appears at release.**
Notifications require permission from the browser or operating system, and the application must be open at the moment of release. On Linux the desktop application needs a running notification service.

**The C++ build stops at `meson setup` with a dependency that was not found or is too old.**
Install the packages listed in the README for your distribution. gtkmm 4.14 and GTK 4.16 are the minimum; older distributions do not ship them.

**The C++ application shows a generic sans-serif font.**
The bundled fonts were not found. They are read from the installed data directory, or from `cpp/data/fonts` when running from the build directory. `GTAVI_COUNTDOWN_DATADIR` can point at another directory that contains a `fonts` folder.

**The always-on-top option is greyed out in the C++ application.**
The session runs on Wayland, which does not let applications keep a window above others. Under X11 the option works.

**Under WSL 2 the time drifts after the host sleeps.**
This is a known behaviour of the WSL 2 virtual machine clock. Network time correction, which is on by default in the desktop application, compensates for it.

## 16. Known limitations

- The release notification is sent only while the application is running. There is no background service.
- Network time correction is available only in the desktop application. Browsers cannot send UDP packets, so the web version trusts the system clock.
- The unlock hour is a single rule, midnight in the selected zone. If the publisher uses a different rule for a particular region or platform, select the zone whose midnight matches it, or edit `release.ts`.
- The interface is in English.
- The C++ port runs on Linux only. It uses POSIX sockets, XDG folders and, for always on top, X11.
- The C++ window cannot be made narrower than about 500 pixels, because the header must fit the window buttons as well as the tools. The web versions shrink to phone width.
- The C++ application remembers the window size but not its position, because GTK 4 cannot place windows.
