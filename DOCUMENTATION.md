# GTA VI Countdown: Technical Documentation

This document describes how the application is built and why. It is written for developers who want to understand, modify or package the project. For installation and everyday commands, see [README.md](README.md).

The countdown exists in two versions: a web app in React and TypeScript (`web/`) and a native desktop app in Rust (`desktop/`). They share one design, one list of games, one timekeeping model, one backdrop shader (in GLSL for the web, WGSL for the desktop), the same wording and the same settings format. Where they differ, the section says so.

## Contents

1. [Architecture](#1-architecture)
2. [Technology choices](#2-technology-choices)
3. [Games and the release moment](#3-games-and-the-release-moment)
4. [Timekeeping](#4-timekeeping)
5. [Rendering](#5-rendering)
6. [Settings](#6-settings)
7. [The web app](#7-the-web-app)
8. [The desktop app](#8-the-desktop-app)
9. [Module reference](#9-module-reference)
10. [Building, testing and verifying](#10-building-testing-and-verifying)
11. [Performance](#11-performance)
12. [Accessibility](#12-accessibility)
13. [Customisation](#13-customisation)
14. [Troubleshooting](#14-troubleshooting)
15. [Known limitations](#15-known-limitations)

## 1. Architecture

| | Web app | Desktop app |
| --- | --- | --- |
| Shared logic | TypeScript modules in `web/src/lib/` | the `gtavi-core` crate |
| User interface | React components in `web/src/components/` | egui, drawn immediately each frame, in `crates/app` |
| Per-frame drawing | WebGL and the DOM | wgpu (Vulkan, Metal, DirectX 12 or OpenGL ES) and egui |
| Host | the browser | eframe (winit) |
| SNTP client | none: browsers cannot send UDP | the `timesync` crate |

In both, the logic that can be tested without a user interface (the games, time zones, countdown arithmetic, iCalendar output and settings validation) is kept apart from the interface and has its own unit tests. The Rust tests are ports of the TypeScript tests, so both apps are checked against the same cases.

Three properties shape the design:

- **Pure logic is separated from state and rendering.** Time zone arithmetic, countdown arithmetic, calendar generation and settings validation are plain modules. In the web app, React state lives in `App.tsx` and two small hooks; in the desktop app, in the `App` struct.
- **The interface re-renders at most once a second.** Everything that changes faster (the backdrop, a rolling figure, the milliseconds readout) bypasses the once-per-second update.
- **Each game is data.** Nothing outside the games modules knows which game is selected; the rest of the code reads a `Game` value.

## 2. Technology choices

**React 19 and Vite 8** for the web app. React is the most widely known component model, which lowers the barrier for anyone changing the app, and Vite gives fast development builds and a small static output. Components keep their styles in CSS modules, so class names are scoped as they are in single-file components. The clock is an external store read with `useSyncExternalStore`, so a tick re-renders only the components that read the time.

**Bun** is the package manager, script runner and test runner. No Node.js-specific tooling is required.

**WebGL 1** is used for the web backdrop rather than WebGL 2 or WebGPU. Nothing in the scene needs newer features, and WebGL 1 runs everywhere.

**Rust with egui and eframe** for the desktop app. It shows the countdown as a native application on Linux, Windows and macOS from one code base, with no web view and no system development libraries at build time. egui is an immediate-mode toolkit: the window is described anew on every frame, which suits a layout computed from the window size, and its text renderer supports the variable font axes the design uses.

**wgpu** renders the desktop backdrop. It speaks Vulkan on Linux and Windows, Metal on macOS and DirectX 12 on Windows, and falls back to OpenGL ES 3.0 where none of those is available. The shader is written in WGSL, which wgpu translates for each backend.

**jiff** supplies time zones in Rust. It reads the system tzdata and bundles a copy on Windows, where there is none.

## 3. Games and the release moment

### The list

Each game is a record in `web/src/lib/time/games.ts` and `desktop/crates/core/src/games.rs`:

| Field | Meaning |
| --- | --- |
| `id` | Stable identifier stored in the settings, such as `gta-vi`, or `custom`. |
| `title`, `shortTitle` | "Grand Theft Auto VI" for the hero, "GTA VI" for the header and the window title. |
| `date`, `hour`, `minute` | The unlock time as a wall-clock time. |
| `platforms` | Listed in the settings and in the calendar event. |
| `milestones` | Dated events, oldest first. The first is the origin of the progress bar. |

The built-in games are Grand Theft Auto VI (November 19, 2026), Metro 2039 (February 4, 2027), God of War Laufey (February 16, 2027), Fable (February 23, 2027) and Final Fantasy VII Revelation (April 8, 2027). Their dates are the publishers' announcements; the unlock hour of each is assumed to be midnight local time, and the interface says it may change.

A **custom game** has a title of up to 60 characters, a date, an unlock time and the date it was added. It becomes a `Game` with one milestone, "Added", so its progress bar measures the wait since you started counting. Editing a custom game keeps its original added date.

### From a wall-clock time to an instant

A release is defined as a wall-clock time, not an instant: midnight at the start of the release date in the viewer's region. The instant this denotes depends on the time zone. Midnight in Auckland arrives 21 hours before midnight in Los Angeles. `releaseInstant(game, zone)` converts the game's wall-clock time to a UTC instant for an IANA zone, which is the device's own unless the user selects another.

JavaScript offers no direct way to ask which instant a wall-clock time denotes in an arbitrary zone. `web/src/lib/time/zones.ts` derives it from `Intl.DateTimeFormat`:

1. `wallTimeAt(instant, zone)` formats an instant in the zone and reads the parts back as numbers.
2. `offsetAt(instant, zone)` subtracts the instant from that wall time, treated as UTC, to obtain the zone's offset at that moment.
3. `zonedTimeToUtc(...)` inverts the mapping. It guesses the instant using the offset at the wall time read as UTC, then recomputes the offset at the guess. The second pass corrects the result when the first guess falls on the other side of a daylight-saving transition.

The Rust app asks `jiff` for offsets and uses the same two-pass conversion, with its own civil-date arithmetic for the rest.

Two calendar helpers build on this. `calendarDaysBetween` counts midnights between two instants in a zone, which is the number of "sleeps". `weekendsBetween` counts the Saturdays strictly between two dates. Both work on calendar dates rather than elapsed hours, so a daylight-saving change in between does not shift the count.

Milestones are calendar dates and are placed at noon UTC, so that they fall on the same day in every zone.

## 4. Timekeeping

### The clock

The web app's `web/src/lib/time/clock.ts` exports a `clock` object with one value, `now`, which React reads through `useSyncExternalStore`. The rest of the interface derives from it.

- **No accumulated error.** Every tick reads `Date.now()`. A timer that fires late, a throttled background tab or a laptop waking from sleep can delay an update, but the value shown afterwards is always correct.
- **Ticks are in phase with the target.** `msUntilNextSecond(now, target)` returns the time until the displayed seconds value next changes. The clock schedules its next tick for that moment, so the display changes when the remaining time crosses a whole second, not at an arbitrary phase set by when the page loaded. The final tick before release lands on the release instant itself.
- **One update per second.** `now` changes only on those ticks. Everything derived from it, including the totals and the progress bar, re-renders at most once per second.
- **Resynchronisation.** The clock re-reads the time and reschedules on `visibilitychange`, `focus` and `pageshow`, and whenever the target moves (a different game or zone).

The desktop app's `Clock` follows the same model. egui has no timers, so on each frame it checks whether the next boundary has passed, and asks eframe for the next frame at that boundary when nothing else is moving.

### Display rule

Remaining time is rounded down. With 5.25 seconds left the display shows 5, and it shows 0 during the last second. At the release instant the app switches to the released state and counts upwards from zero. The same rule applies to the milliseconds readout, so the two always agree.

The release notification fires on the tick that crosses zero. It compares with the previous tick of the same target, so opening the app after release, or switching to a game that is already out, does not notify.

### Milliseconds

The optional milliseconds readout changes on every frame. In the web app, `Countdown.tsx` runs a `requestAnimationFrame` loop that reads `clock.precise()` and writes one text node directly; React is not involved. In the desktop app, the readout is drawn on each frame from the precise clock while it is shown.

### Network time correction (desktop)

A countdown is only as accurate as the clock it reads. The desktop app measures the system clock's error at start-up, and every six hours afterwards, on a background thread, and adds the measured offset to the clock. If a later measurement fails, the last good correction is kept; if the first one fails, the system clock is used uncorrected and the settings panel reports the reason. The footer shows which source is in use. Browsers cannot send UDP packets, so the web app trusts the system clock.

The `timesync` crate (`desktop/crates/timesync`) is an SNTP client (RFC 4330) that uses only the Rust standard library. For each measurement it sends a 48-byte client packet carrying the local send time, validates the reply and applies the standard calculation:

```text
offset = ((t2 - t1) + (t3 - t4)) / 2
delay  =  (t4 - t1) - (t3 - t2)
```

where `t1` and `t4` are the local send and receive times and `t2` and `t3` are the server's receive and transmit times.

- The interval between `t1` and `t4` is measured with a monotonic clock, so a clock adjustment during the exchange cannot corrupt the result.
- Replies are rejected unless they are server-mode packets, come from a synchronised server (not stratum 0, not the alarm leap indicator, non-zero transmit time) and echo the exact timestamp that was sent. The socket is connected to the server's address, so datagrams from other sources are discarded by the operating system.
- Three exchanges are made and the one with the shortest round trip is used, because the possible error is bounded by half the round trip.
- Timestamp conversion handles the NTP era rollover in 2036.
- Servers are tried in order: `time.cloudflare.com`, `pool.ntp.org`, `time.google.com`. Each exchange has a 1.5 second timeout.

**What is sent.** The request contains the protocol header and the local time. It carries no identifier, and nothing about the user or the app. This is the same exchange an operating system performs when it sets its clock. The correction can be switched off in the settings, in which case no network request is made.

## 5. Rendering

### Digits

Each figure is a fixed-size cell. When its value changes, the old glyph slides down and fades while the new one slides in from above, over 460 ms with a cubic ease-out. In the web app (`Digit.tsx`) both are CSS animations of `transform` and `opacity`, so the roll runs on the compositor and costs no script time per frame; the outgoing glyph is removed when its animation ends. The cells use `contain: size layout style`, so a change inside one can never trigger layout outside it. Digits are keyed by their position from the right: when the number of digits changes, from 100 days to 99, the remaining digits keep their identity and do not animate. The desktop app draws the same roll in `countdown_view.rs`.

### Backdrop

The scene is a fragment shader drawn on one full-screen triangle (`web/src/lib/gl/shader.ts` for WebGL, `desktop/assets/scene.wgsl` for wgpu). It evaluates, per pixel:

- a three-stop sky gradient with hash-based stars that fade towards the horizon,
- a sun with a vertical colour grade and slits that open towards its lower edge,
- cloud streaks from four octaves of horizontally stretched value noise, six on the High preset,
- two rows of procedurally generated buildings, the nearer with lit windows,
- water that mirrors the scene about the horizon, displaced by ripples that grow towards the viewer, with glitter in the column of light under the sun,
- a vignette and film grain. The grain also acts as dithering and removes the banding that smooth gradients show on 8-bit displays.

The sky function is evaluated once per pixel. Below the horizon it is called with mirrored coordinates rather than being rendered to a texture and sampled, which avoids a second pass. The artwork is original and procedural; the project contains no images from any game.

### Composition follows the layout

The horizon and the sun are positioned by the layout, not fixed in the shader. The web app measures the bottom of the hero and the top of the details panel with a `ResizeObserver`; the desktop app computes both while laying out each frame. Two numbers go to the renderer: the height of the horizon and the radius of the sun. The sun therefore always sits in the gap beneath the digits and the skyline always meets the top of the panel, on any window size and in every layout tier. The still gradient uses the same horizon.

### Quality presets

| Preset | Resolution | Clouds | Frame rate | Governor |
| --- | --- | --- | --- | --- |
| Auto | up to 1.5 device pixels per CSS pixel (or point) | 4 octaves | display rate | steps down through 100%, 75%, 55%, 40% and 30% after 24 frames slower than 1/42 s, and never back up |
| High | every device pixel, up to a ratio of 3 | 6 octaves | display rate (the desktop app paces up to 120 frames per second) | none |
| Low | 55% of Auto | 4 octaves | about 30 frames per second | none |
| Off | the shader is not run; a still gradient is drawn | | | |

Other protections, in both apps:

| Mechanism | Behaviour |
| --- | --- |
| Visibility | Rendering stops while the page or window is hidden. |
| Reduced motion | When the system asks for reduced motion, one still frame is drawn and redrawn only on resize or theme change. |
| Off | The web canvas is removed and its context released; the desktop app releases its offscreen texture. |
| Failure | If WebGL or a GPU adapter is unavailable, the gradient stays visible and nothing else depends on the scene. |

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

The web app expresses these as media queries. The desktop app applies the same breakpoints and `clamp()` sizes in `App::measure`. Long titles step down a size, and a long custom title is cut short in the header rather than widening the page.

## 6. Settings

The web app stores settings in `localStorage` under the key `gtavi-countdown:settings:v1`. The desktop app writes the same JSON object to `settings.json` in its configuration folder (README, "Settings and files"). Both write the keys in the same order.

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `timeZone` | IANA zone name or `null` | `null` | Zone in which the release is evaluated. `null` follows the device. |
| `game` | game id | `"gta-vi"` | The selected game, or `"custom"`. |
| `customGame` | object or `null` | `null` | `{ "title", "date": "YYYY-MM-DD", "time": "HH:MM", "added": "YYYY-MM-DD" }`. Kept while another game is selected. |
| `showMilliseconds` | boolean | `false` | Show the per-frame milliseconds readout. |
| `theme` | `"dusk"`, `"midnight"` or `"dawn"` | `"dusk"` | Colour palette. |
| `effects` | `"auto"`, `"high"`, `"low"` or `"off"` | `"auto"` | Backdrop quality preset. |
| `notify` | boolean | `false` | Send a notification at release. |
| `networkTime` | boolean | `true` | Desktop only. Correct the clock over NTP. |
| `alwaysOnTop` | boolean | `false` | Desktop only. Keep the window above others. |

`sanitizeSettings` in `web/src/lib/settings.ts` (and `settings::sanitize` in Rust) treats stored data as untrusted. It starts from the defaults and copies across only fields that have the right type and an allowed value, including a check that a stored zone name is one the runtime recognises and that a custom game's title, dates and time are valid. `"custom"` is accepted only together with a valid custom game, and an unknown game falls back to GTA VI. Corrupt or outdated data therefore degrades to defaults field by field instead of breaking the app. Storage failures, such as a private window with storage disabled, are caught and ignored.

## 7. The web app

### Structure

- `web/src/main.tsx` mounts `App` in strict mode.
- `web/src/App.tsx` owns the settings (through `useSettings`), the selected game and target instant, the composition of the backdrop, the toast, the actions and the keyboard shortcuts.
- `web/src/components/` holds one component per part of the page, each with a CSS module: `Backdrop`, `Countdown`, `Digit`, `Timeline`, `Stats`, `SettingsPanel`, `GamePicker`, `ZonePicker`, `Switch` and `Icon`.
- `web/src/lib/` holds the logic and the browser seams: `time/` (games, zones, countdown, clock), `gl/` (renderer and shader), `ics.ts`, `settings.ts`, `useSettings.ts`, `themes.ts`, `platform.ts`, `useReducedMotion.ts` and `cx.ts`.

The settings sheet is a native modal `dialog`, which provides the focus trap, Escape handling and inert background. Its content mounts only while it is open, so the zone list is built on demand and listed with offsets at the selected release.

### Browser capabilities

`web/src/lib/platform.ts` wraps each browser capability so that the app degrades where one is missing: fullscreen (not on the iPhone), the share sheet (preferred over copying on touch devices), notifications (asked for when the user turns the alert on), the clipboard (with a fallback for older contexts) and the calendar download.

### Content security policy

`web/public/_headers` restricts every resource type to the site's own origin:

```text
default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:;
font-src 'self'; connect-src 'self'; manifest-src 'self'; object-src 'none'; base-uri 'self';
form-action 'none'; frame-ancestors 'none'
```

The page cannot load remote scripts, styles, fonts or images. `'unsafe-inline'` for styles covers the inline styles React writes for positions and gradients.

## 8. The desktop app

### Structure

The Cargo workspace in `desktop/` has three crates:

- **`crates/core`** (`gtavi-core`) has no user interface dependency: the games, time zones on `jiff`, countdown maths, English date wording, the iCalendar writer, settings validation with `serde_json`, theme palettes and interface colours, and calendar-file validation.
- **`crates/timesync`** is the SNTP client described in section 4.
- **`crates/app`** (`gtavi-countdown`) is the application. `App` in `app.rs` plays the part of `App.tsx`.

`desktop/assets/` holds the WGSL shaders, the fonts, the icon and a desktop entry. The shaders and fonts are compiled into the binary with `include_str!` and `include_bytes!`, so the executable is self-contained.

### Immediate-mode layout

egui has no retained widget tree and no stylesheet. On every frame `App::measure` applies the layout tiers and fluid sizes to the window size, lays out each piece of text and adds up the heights of the header, hero, panel and footer. The gap that holds the sun takes the height left over, and content taller than the window scrolls. Because measuring comes before drawing, the composition of the backdrop is known in the same frame.

Controls are drawn by hand to match the design: round tool buttons, the zone pill, the game cards, switches, segmented control, theme swatches and the virtualised zone list. Text fields are egui's `TextEdit` without its frame. Text uses the bundled variable fonts with their `wght` and `wdth` axes set per run. egui has no blurred text shadow; a soft rectangle shadow behind each line and a tight offset copy of the figures stand in for the CSS text shadows.

### The backdrop on wgpu

`backdrop.rs` creates two render pipelines once, when the app starts, and stores them in egui's renderer:

1. **Scene.** In the paint callback's `prepare` step, the shader renders into an offscreen `Rgba8Unorm` texture at the resolution the preset allows, in its own render pass.
2. **Blit.** In the `paint` step, a second small shader samples that texture with linear filtering into egui's own render pass, scaling it to the window.

The scene is always drawn offscreen, so changing resolution never touches egui's surface. The texture is plain 8-bit and blitted unchanged, as the WebGL version draws into its canvas, because egui composites in gamma space. Uniforms are one 128-byte block laid out as WGSL lays out its struct. The governor, presets and still frame behave as in section 5; Off releases the offscreen texture.

wgpu chooses the backend: Vulkan on Linux and Windows, Metal on macOS, otherwise DirectX 12 or OpenGL ES. `WGPU_BACKEND` overrides the choice, and the settings sheet shows the backend and adapter in use, for example "Rendering with Vulkan on …".

### Pacing frames

egui redraws the whole window on every frame, so the app decides how often frames are drawn instead of relying on vsync, which some compositors do not apply. When nothing moves, the next frame is requested for the next tick of the clock, so the window draws once a second. The animated scene, a rolling figure, the milliseconds readout, the settings sheet and the toast run at 60 frames per second (High up to 120), the Low scene at about 30, and the breathing colons alone at about 15. Frames are paced from the start of the previous frame, so drawing time is not added on top.

### Desktop capabilities

| Capability | Implementation |
| --- | --- |
| Network time | `platform::sync_network_time` calls `timesync` on a background thread; the result comes back over a channel and wakes the window. |
| Clock | `Clock` re-reads the system clock each frame and advances the display at the boundary from `ms_until_next_second`. It resynchronises when the window regains focus. |
| Notifications | `notify-rust`: D-Bus on Linux, the notification centre on macOS and toast notifications on Windows. |
| Always on top | `ViewportCommand::WindowLevel`. Wayland offers no such request, so the pin button is hidden there and the setting is shown as unavailable. |
| Calendar | The file name and contents are validated (a bare `.ics` name of letters, digits, hyphens, underscores and dots, not starting with a dot, and at most 64 KiB of iCalendar data) and written atomically to the download folder found by `dirs`. |
| Window state | Size, position and maximised state are saved to `window-state.json` on close. Wayland does not report or accept window positions. |
| Reduced motion | On Linux the GNOME `enable-animations` setting is read once at start-up with `gsettings`. `GTAVI_COUNTDOWN_REDUCED_MOTION=1` forces it on any platform. |
| Window decorations | The system's: the native title bar shows the countdown at minute resolution. |

### Developer aids

`GTAVI_COUNTDOWN_SCREENSHOT=out.png` saves a PNG of the window once the scene has faded in, then quits, and prints the renderer in use; add `GTAVI_COUNTDOWN_SCREENSHOT_SETTINGS=1` to capture the settings sheet open. In debug builds, `GTAVI_COUNTDOWN_DEBUG_NOW` starts the clock at a given Unix time in milliseconds, which is how the switch to the released state can be tried out before a release.

## 9. Module reference

### Web app

| Path | Responsibility |
| --- | --- |
| `web/src/main.tsx` | Mounts the root component. |
| `web/src/App.tsx` | Layout, wording, composition measurement, keyboard shortcuts, clipboard, calendar and notification actions. |
| `web/src/app.css` | Font faces, design tokens per theme, resets and shared utility classes. |
| `web/src/lib/time/games.ts` | The built-in games, custom games, date and time parsing. |
| `web/src/lib/time/zones.ts` | Zone offsets, wall-time conversion, zone listing and calendar-day arithmetic. |
| `web/src/lib/time/countdown.ts` | Release instant, breakdown into units, tick scheduling, totals and timeline positions. |
| `web/src/lib/time/clock.ts` | The once-per-second clock, as an external store. |
| `web/src/lib/settings.ts` | Settings schema, defaults, validation and serialisation. |
| `web/src/lib/useSettings.ts` | The persisted settings, as a hook. |
| `web/src/lib/themes.ts` | Shader palettes, fallback gradients and swatches for each theme. |
| `web/src/lib/ics.ts` | iCalendar generation with escaping and line folding per RFC 5545. |
| `web/src/lib/platform.ts` | Fullscreen, sharing, notifications, clipboard and the calendar download. |
| `web/src/lib/gl/shader.ts` | GLSL source for the backdrop. |
| `web/src/lib/gl/scene.ts` | WebGL renderer with the quality presets. |
| `web/src/components/Backdrop.tsx` | Hosts the canvas and the fallback gradient. |
| `web/src/components/Countdown.tsx` | The four units, the separators and the milliseconds readout. |
| `web/src/components/Digit.tsx` | One rolling digit. |
| `web/src/components/Timeline.tsx` | Progress since the game's first milestone. |
| `web/src/components/Stats.tsx` | Sleeps, weekends and totals. |
| `web/src/components/SettingsPanel.tsx` | The settings sheet, on the native `dialog` element. |
| `web/src/components/GamePicker.tsx` | The game list and the custom game form. |
| `web/src/components/ZonePicker.tsx` | Filterable list of time zones, ordered by when they reach the release. |
| `web/src/components/Switch.tsx`, `Icon.tsx` | Accessible toggle; inline SVG icons. |

### Desktop app

| Path | Responsibility |
| --- | --- |
| `crates/core/src/games.rs` | The built-in games, custom games, date and time parsing. |
| `crates/core/src/zones.rs` | Zone offsets, wall-time conversion, zone listing, civil-date arithmetic and the sleeps and weekends counts. |
| `crates/core/src/countdown.rs` | Release instant, breakdown into units, tick scheduling, totals and timeline positions. |
| `crates/core/src/wording.rs` | Dates, times, zone names and numbers as shown in the interface. |
| `crates/core/src/ics.rs` | iCalendar generation. |
| `crates/core/src/settings.rs` | Settings schema, validation and serialisation. |
| `crates/core/src/themes.rs` | Shader palettes, fallback gradients, swatches and interface colours. |
| `crates/core/src/calendar_file.rs` | Validation of calendar exports. |
| `crates/timesync/src/lib.rs` | SNTP client. |
| `crates/app/src/main.rs` | Window options, the restored window state, the application icon and the wgpu renderer. |
| `crates/app/src/app.rs` | Measurement and layout tiers, wording, actions, shortcuts, settings changes, network time and frame pacing. |
| `crates/app/src/backdrop.rs` | The wgpu scene and blit pipelines, the governor, the fallback gradient and the scrim. |
| `crates/app/src/countdown_view.rs` | The four units, the colons and the milliseconds readout. |
| `crates/app/src/details.rs` | The panel, the timeline and the totals. |
| `crates/app/src/settings_panel.rs` | The settings sheet, the game picker and custom game form, and the time zone picker. |
| `crates/app/src/clock.rs` | The once-per-second clock. |
| `crates/app/src/platform.rs` | Settings and window-state files, network time, notifications, the calendar file and the reduced motion preference. |
| `crates/app/src/style.rs`, `icons.rs` | Fonts, colours, text and gradient helpers; icons, including the window icon rasterised from its SVG shapes. |
| `crates/app/src/screenshot.rs` | The screenshot developer aid. |
| `assets/scene.wgsl`, `assets/blit.wgsl` | The backdrop shader and the scaling pass. |

## 10. Building, testing and verifying

### Commands

The web app, from `web/`:

```sh
bun install               # install dependencies
bun run check             # type-check TypeScript and TSX
bun test                  # unit tests
bun run build             # type-check and build the site into dist/
```

The desktop app, from `desktop/`:

```sh
cargo test --workspace    # core, SNTP and app tests
cargo run --release       # run the app
cargo build --release     # binary in target/release/gtavi-countdown
```

Both type-check and compile without warnings.

### What the tests cover

**TypeScript (`web/tests/`, 51 tests)**

- `zones.test.ts`: offsets including fractional and daylight-saving cases, release midnight in eight zones, conversions on both sides of a daylight-saving change, round trips, formatting, and the sleeps and weekends counts.
- `countdown.test.ts`: unit breakdown, behaviour at and after the target, tick scheduling (including that each scheduled tick lands exactly on a change of the display and not a millisecond late), totals, progress, milestone ordering, every built-in game's release instant and custom games.
- `ics.test.ts`: date formatting, escaping, folding at 75 octets without splitting multi-byte characters, and the structure of a generated calendar.
- `settings.test.ts`: validation of stored settings against malformed, partial and hostile input, game and custom game validation, the High preset, and the serialised form shared with the Rust app.

**Rust (`desktop/`, 83 tests)**

- `gtavi-core` (67): the same cases as the TypeScript suite, plus civil-date arithmetic, strict date and time parsing, the built-in game list and calendar-file validation, including path traversal attempts.
- `timesync` (11): packet encoding, timestamp conversion and the 2036 era rollover, the offset formula, rejection of each kind of invalid reply, and three tests against a local UDP server: a known clock skew is measured, a dead server is skipped, and total failure is reported.
- The app (5): the icon path parser, the application icon and error wording.

Neither interface has automated tests. The web app was checked in headless Chromium, the desktop app with `GTAVI_COUNTDOWN_SCREENSHOT` on both the Vulkan and OpenGL backends.

### Packaging

The web build in `web/dist/` can be deployed to any static host as it is. For Cloudflare Pages, `bun run deploy` builds and uploads it with Wrangler (configured in `web/wrangler.toml`), and `web/public/_headers` supplies the response headers; the README has the dashboard settings for Git-based deploys. The home-screen icons in `web/public/icons/` are square, full-bleed renders of the app icon.

The desktop app is one self-contained executable. `cargo install --path desktop/crates/app` installs it to `~/.cargo/bin`; the desktop entry and the scalable icon are in `desktop/assets/` for packagers. Its application ID, `app.gtavi.countdown`, is set in `main.rs` and the desktop entry; change both before distributing builds. The release profile uses one codegen unit, link-time optimisation, `panic = "abort"` and stripped symbols.

## 11. Performance

| Metric | Web app | Desktop app |
| --- | --- | --- |
| Code | about 86 kB of JavaScript after gzip, React included | about 15 MB binary, fonts and shaders included |
| CSS | about 6 kB after gzip | |
| Fonts | three WOFF2 files, about 217 kB; only the Latin subsets are bundled | three TrueType files, about 570 kB, compiled in |

The cost of the backdrop depends on the GPU and is governed by the presets in section 5. Techniques behind the numbers:

- One update of the interface per second, scheduled on the boundary where the display changes.
- Digit animations run on the compositor in the browser.
- The milliseconds readout bypasses React and writes one text node.
- The progress bar is scaled with a transform instead of being resized.
- The time zone list uses `content-visibility: auto`, so rows outside the scroll viewport are not laid out or painted, and it is built only while the settings sheet is open. The desktop list draws only the visible rows.
- No `backdrop-filter` over the animated canvas. Panels are translucent solid colours, which avoids re-blurring the scene every frame.
- Fonts are variable: one Archivo file supplies every weight and width.
- The desktop app paces its own frames (section 8), because egui redraws the whole window on each frame.

## 12. Accessibility

- The timer has the timer role and a label that reads the days, hours and minutes. The label changes once a minute, not once a second, and the animated digits are hidden from assistive technology.
- In the web app, every control is a native `button` or `input` with a text label. Toggles use `role="switch"`; the game, theme and backdrop selectors are radio groups. The settings sheet is a native modal `dialog`, which provides the focus trap, Escape handling and inert background. Focus indicators are always visible for keyboard users.
- The desktop app exposes its interface through AccessKit: the countdown has the timer role, switches, radio choices and buttons carry their roles, labels and states, and the toast is a labelled element. The focused control always shows a focus ring. The settings scrim takes pointer input, but egui has no inert background, so the content behind it can still take keyboard focus.
- With the reduced motion preference set, digits change without animation, the backdrop is a still frame and transitions are disabled.
- Text over the scene is protected by a scrim and text shadows so that contrast holds in every theme.

## 13. Customisation

### Changing a date or adding a game

Edit `GAMES` in `web/src/lib/time/games.ts` and `built_in()` in `desktop/crates/core/src/games.rs`. Keep games in release order and milestones oldest first; the first milestone is the origin of the progress bar. Tests that assert specific dates will need the same change.

### Adding a theme

1. Add the name to `THEMES` in `web/src/lib/settings.ts`.
2. Add an entry to `THEME_INFO` in `web/src/lib/themes.ts` with the shader palette, a fallback gradient and three swatch colours.
3. Add a `:root[data-theme="..."]` block to `web/src/app.css` with the accent, panel and scrim colours.
4. In Rust, add a variant to `Theme` and a `ThemeInfo` in `desktop/crates/core/src/themes.rs`.

### Changing the time servers

Edit `TIME_SERVERS` in `desktop/crates/app/src/platform.rs`. Entries are `host:port` strings and are tried in order.

## 14. Troubleshooting

**The web backdrop is a plain gradient.**
WebGL is unavailable or disabled in the browser, or the backdrop is set to Off.

**The desktop backdrop looks soft or blocky.**
The Auto governor has lowered the rendering resolution because frames were slow. This is expected with software rendering, which is common under WSL 2 and in virtual machines. Choose Off for the lightest option, or High to keep full resolution regardless.

**The desktop backdrop is a plain gradient.**
No GPU adapter was found, or the backdrop is set to Off. The settings sheet says which renderer is in use. Install the Vulkan driver for your GPU (README, "Requirements"), or try `WGPU_BACKEND=gl`.

**The desktop app exits at start-up with "Library libxkbcommon-x11.so could not be loaded".**
The session is X11 and the X11 keyboard library is missing. Install `libxkbcommon-x11-0` (Debian and Ubuntu) or `libxkbcommon-x11` (Fedora, Arch).

**The footer says "System clock" on the desktop.**
Network time is switched off, or no time server could be reached. The settings sheet shows the reason. UDP port 123 must be allowed outbound.

**No notification appears at release.**
Notifications need permission from the browser, and the page or app must be open at the moment of release. On Linux the desktop app needs a running notification service.

**Under WSL 2 the time drifts after the host sleeps.**
This is a known behaviour of the WSL 2 virtual machine clock. Network time correction, which is on by default in the desktop app, compensates for it.

## 15. Known limitations

- The release notification is sent only while the app is open. There is no background service.
- Network time correction and always on top are available only in the desktop app.
- Each unlock hour is a single rule, midnight in the selected zone unless a custom game says otherwise. If a publisher uses a different rule for a region or platform, select the zone whose midnight matches it, or enter the game as a custom game.
- One custom game can be kept at a time.
- The interface is in English.
- The desktop app uses the system title bar rather than drawing the header into it, so the backdrop starts below the title bar.
- The desktop app reads the reduced motion preference only from GNOME's settings on Linux, once at start-up. On Windows and macOS it animates unless `GTAVI_COUNTDOWN_REDUCED_MOTION` is set.
- Under WSLg, the WSLg compositor (Weston) crashes intermittently when the desktop app opens a tall, narrow window (around 420 by 820 points) with the scene enabled, on both the Vulkan and OpenGL backends. The crash is a segmentation fault in the compositor, not in the app, and setting the backdrop to Off avoided it in testing.
