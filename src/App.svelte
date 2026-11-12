<script lang="ts">
  import { onMount, untrack } from "svelte";
  import Backdrop from "./lib/components/Backdrop.svelte";
  import Countdown from "./lib/components/Countdown.svelte";
  import Icon from "./lib/components/Icon.svelte";
  import SettingsPanel, { type SyncState } from "./lib/components/SettingsPanel.svelte";
  import Stats from "./lib/components/Stats.svelte";
  import Timeline from "./lib/components/Timeline.svelte";
  import type { Composition } from "./lib/gl/scene";
  import { buildCalendar } from "./lib/ics";
  import {
    copyText,
    isDesktop,
    notify,
    requestNotificationPermission,
    saveCalendar,
    setAlwaysOnTop,
    syncNetworkTime,
    toggleFullscreen,
  } from "./lib/platform";
  import { settings } from "./lib/settings.svelte";
  import { clock } from "./lib/time/clock.svelte";
  import { breakdown, formatCompact, releaseInstant } from "./lib/time/countdown";
  import { RELEASE } from "./lib/time/release";
  import { formatOffset, offsetAt, systemTimeZone, zoneCity } from "./lib/time/zones";

  /** How often the desktop app re-measures the clock offset. */
  const RESYNC_INTERVAL_MS = 6 * 60 * 60 * 1000;

  let settingsOpen = $state(false);
  let fullscreen = $state(false);
  let sync = $state<SyncState>({ status: "idle" });
  let toast = $state("");

  let hero: HTMLElement | undefined = $state();
  let details: HTMLElement | undefined = $state();
  let composition = $state<Composition>({ horizon: 0.66, sunRadius: 0.16 });
  let toastTimer: ReturnType<typeof setTimeout> | undefined;

  const zone = $derived(settings.zone);
  const target = $derived(releaseInstant(zone));
  const parts = $derived(breakdown(clock.now, target));

  // ---------------------------------------------------------------- wording

  const targetDate = $derived(
    new Intl.DateTimeFormat("en-US", {
      timeZone: zone,
      weekday: "long",
      month: "long",
      day: "numeric",
      year: "numeric",
    }).format(target),
  );

  const targetTime = $derived(
    new Intl.DateTimeFormat("en-US", {
      timeZone: zone,
      hour: "2-digit",
      minute: "2-digit",
      hourCycle: "h23",
    }).format(target),
  );

  const zoneText = $derived(`${zoneCity(zone)} time, ${formatOffset(offsetAt(target, zone))}`);

  /**
   * When the chosen zone differs from the device's, the same instant is also
   * shown on the device's own clock, which is what the viewer will be reading.
   */
  const localEquivalent = $derived.by(() => {
    const system = systemTimeZone();
    if (offsetAt(target, system) === offsetAt(target, zone)) return "";
    return new Intl.DateTimeFormat("en-US", {
      timeZone: system,
      weekday: "short",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
      hourCycle: "h23",
    }).format(target);
  });

  // --------------------------------------------------------------- effects

  onMount(() => {
    const stop = clock.start(untrack(() => target));
    const onFullscreen = () => (fullscreen = document.fullscreenElement !== null);
    document.addEventListener("fullscreenchange", onFullscreen);
    return () => {
      stop();
      document.removeEventListener("fullscreenchange", onFullscreen);
    };
  });

  // The backdrop is composed around the content: the horizon sits just above
  // the lower panels and the sun fills the gap beneath the digits, whatever
  // the viewport. Positions are measured in document space, so scrolling a
  // tall page does not move the scene.
  $effect(() => {
    if (!hero || !details) return;
    const top = hero;
    const bottom = details;
    const measure = () => {
      const height = window.innerHeight || 1;
      const heroBottom = top.offsetTop + top.offsetHeight;
      // In the compact layout the lower panels are hidden; the horizon then
      // drops to the bottom of the window, clear of the timer.
      const hidden = bottom.offsetParent === null;
      const anchor = hidden ? Math.max(heroBottom + 44, height * 0.82) : bottom.offsetTop - 18;
      const horizon = Math.min(Math.max(anchor, height * 0.42), height * 0.9);
      // The sun is sunk 12% below the horizon, so 1.12 radii show above it.
      const room = (horizon - heroBottom - 6) / 1.12;
      const radius = Math.min(Math.max(room, 30), height * 0.2);
      const next = { horizon: horizon / height, sunRadius: radius / height };
      if (
        Math.abs(next.horizon - composition.horizon) > 0.001 ||
        Math.abs(next.sunRadius - composition.sunRadius) > 0.001
      ) {
        composition = next;
      }
    };
    const observer = new ResizeObserver(measure);
    observer.observe(top);
    observer.observe(bottom);
    window.addEventListener("resize", measure);
    measure();
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", measure);
    };
  });

  // Re-phase the tick whenever the target instant moves.
  $effect(() => clock.retarget(target));

  // Persist on any change. Serialising reads every field, which subscribes
  // this effect to all of them.
  $effect(() => {
    JSON.stringify(settings.data);
    settings.persist();
  });

  $effect(() => {
    const theme = settings.data.theme;
    document.documentElement.dataset.theme = theme;
    const base = getComputedStyle(document.documentElement).getPropertyValue("--base").trim();
    document.querySelector('meta[name="theme-color"]')?.setAttribute("content", base);
  });

  // The tab title follows the countdown at minute resolution; `minuteStamp`
  // changes once a minute, so the title is not rewritten every second.
  const minuteStamp = $derived(
    `${parts.elapsed ? "+" : ""}${parts.days}d ${String(parts.hours).padStart(2, "0")}h ${String(parts.minutes).padStart(2, "0")}m`,
  );
  $effect(() => {
    document.title = parts.elapsed ? "GTA VI is out" : `${minuteStamp} | GTA VI Countdown`;
  });

  $effect(() => {
    const enabled = settings.data.alwaysOnTop;
    setAlwaysOnTop(enabled).catch(() => {
      // The window manager may refuse; the setting stays as the user left it.
    });
  });

  // Measure the clock at start-up and again every few hours: a machine that
  // sleeps, or a virtual machine whose clock drifts, would otherwise wander
  // away from the correction taken at launch.
  $effect(() => {
    if (!isDesktop) return;
    if (!settings.data.networkTime) {
      clock.setOffset(0);
      sync = { status: "idle" };
      return;
    }
    untrack(runSync);
    const timer = setInterval(runSync, RESYNC_INTERVAL_MS);
    return () => clearInterval(timer);
  });

  // Fire the release notification on the tick that crosses zero. Opening the
  // app after release must not notify, hence the comparison with the
  // previous state rather than a check of the current one.
  let wasElapsed: boolean | null = null;
  $effect(() => {
    const elapsed = parts.elapsed;
    if (wasElapsed === false && elapsed) {
      showToast(`${RELEASE.title} is out now.`);
      if (settings.data.notify) {
        void notify(`${RELEASE.title} is out now`, "The wait is over.");
      }
    }
    wasElapsed = elapsed;
  });

  // --------------------------------------------------------------- actions

  function showToast(message: string): void {
    toast = message;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = ""), 3600);
  }

  async function runSync(): Promise<void> {
    if (!isDesktop || sync.status === "syncing") return;
    const previous = sync;
    sync = { status: "syncing" };
    try {
      const result = await syncNetworkTime();
      // The setting may have been switched off while the request was in flight.
      if (!settings.data.networkTime) return;
      clock.setOffset(result.offsetMs);
      sync = { status: "ok", result, at: Date.now() };
    } catch (error) {
      if (!settings.data.networkTime) return;
      // A failed refresh keeps the last good correction rather than
      // discarding it; only a first failure falls back to the system clock.
      sync =
        previous.status === "ok"
          ? previous
          : { status: "error", message: String(error instanceof Error ? error.message : error) };
    }
  }

  async function setNotify(enabled: boolean): Promise<void> {
    if (!enabled) {
      settings.data.notify = false;
      return;
    }
    const granted = await requestNotificationPermission().catch(() => false);
    settings.data.notify = granted;
    showToast(
      granted
        ? "You will be notified at release while the app is open."
        : "Notifications are blocked for this app.",
    );
  }

  async function onFullscreen(): Promise<void> {
    try {
      fullscreen = await toggleFullscreen();
    } catch {
      showToast("Fullscreen is not available here.");
    }
  }

  async function onCopy(): Promise<void> {
    const now = breakdown(clock.precise(), target);
    const text = now.elapsed
      ? `${RELEASE.title} has been out for ${formatCompact(now)}.`
      : `${RELEASE.title} unlocks in ${formatCompact(now)} (${targetDate} at ${targetTime}, ${zoneText}).`;
    showToast((await copyText(text)) ? "Countdown copied to the clipboard." : "Copying failed.");
  }

  async function onCalendar(): Promise<void> {
    const calendar = buildCalendar({
      uid: `release-${target}@gtavi-countdown`,
      start: target,
      durationMinutes: 60,
      summary: `${RELEASE.title} release`,
      description: `${RELEASE.title} unlocks on ${targetDate} at ${targetTime} (${zoneText}). Platforms at launch: ${RELEASE.platforms.join(", ")}.`,
      reminders: [24 * 60, 60],
    });
    try {
      const path = await saveCalendar("gta-vi-release.ics", calendar);
      showToast(path ? `Calendar event saved to ${path}` : "Calendar event downloaded.");
    } catch (error) {
      showToast(`Could not save the calendar event. ${error instanceof Error ? error.message : error}`);
    }
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.ctrlKey || event.metaKey || event.altKey || event.repeat) return;
    const element = event.target as HTMLElement | null;
    if (element?.closest("input, textarea, select, [contenteditable]")) return;

    switch (event.key.toLowerCase()) {
      case "s":
        settingsOpen = !settingsOpen;
        break;
      case "f":
        void onFullscreen();
        break;
      case "m":
        settings.data.showMilliseconds = !settings.data.showMilliseconds;
        break;
      case "t":
        settings.cycleTheme();
        break;
      case "c":
        void onCopy();
        break;
      default:
        return;
    }
    event.preventDefault();
  }
</script>

<svelte:window onkeydown={onKeydown} />

<Backdrop theme={settings.data.theme} effects={settings.data.effects} {composition} />

<div class="shell">
  <header class="bar">
    <div class="brand">
      <span class="brand-name">GTA VI</span>
      <span class="brand-sub">Countdown</span>
    </div>

    <nav class="tools" aria-label="Actions">
      <button type="button" class="tool" onclick={onCalendar} title="Add to calendar">
        <Icon name="calendar" />
        <span class="visually-hidden">Add to calendar</span>
      </button>
      <button type="button" class="tool" onclick={onCopy} title="Copy the countdown (C)">
        <Icon name="copy" />
        <span class="visually-hidden">Copy the countdown</span>
      </button>
      {#if isDesktop}
        <button
          type="button"
          class="tool"
          aria-pressed={settings.data.alwaysOnTop}
          onclick={() => (settings.data.alwaysOnTop = !settings.data.alwaysOnTop)}
          title="Always on top"
        >
          <Icon name="pin" />
          <span class="visually-hidden">Always on top</span>
        </button>
      {/if}
      <button type="button" class="tool" onclick={onFullscreen} title="Toggle fullscreen (F)">
        <Icon name={fullscreen ? "compress" : "expand"} />
        <span class="visually-hidden">Toggle fullscreen</span>
      </button>
      <button
        type="button"
        class="tool"
        aria-haspopup="dialog"
        onclick={() => (settingsOpen = true)}
        title="Settings (S)"
      >
        <Icon name="sliders" />
        <span class="visually-hidden">Settings</span>
      </button>
    </nav>
  </header>

  <main>
    <div class="hero" bind:this={hero}>
      <p class="label eyebrow" class:out={parts.elapsed}>
        {parts.elapsed ? "Out now" : "Countdown to"}
      </p>
      <h1>{RELEASE.title}</h1>

      <p class="when">
        <span>
          {parts.elapsed ? "Released" : "Unlocks"}
          {targetDate} at {targetTime}
        </span>
        <button
          type="button"
          class="zone"
          onclick={() => (settingsOpen = true)}
          title="Change the time zone"
        >
          {zoneText}
        </button>
      </p>
      {#if localEquivalent}
        <p class="local">{localEquivalent} on this device</p>
      {/if}

      <div class="timer">
        <Countdown {parts} {target} showMilliseconds={settings.data.showMilliseconds} />
      </div>
      {#if parts.elapsed}
        <p class="label since">Since release</p>
      {/if}
    </div>

    <div class="details" bind:this={details}>
      <Timeline now={clock.now} {target} />
      {#if !parts.elapsed}
        <Stats now={clock.now} {target} timeZone={zone} />
      {/if}
    </div>
  </main>

  <footer>
    <p>
      Unofficial fan project. Not affiliated with Rockstar Games or Take-Two Interactive. The
      unlock hour is as reported and may change.
    </p>
    <p class="clock-source">
      {#if sync.status === "ok"}
        Network time
      {:else if sync.status === "syncing"}
        Syncing clock
      {:else}
        System clock
      {/if}
    </p>
  </footer>
</div>

<SettingsPanel bind:open={settingsOpen} {sync} onsync={runSync} onnotify={setNotify} />

<div class="toast" class:visible={toast !== ""} role="status" aria-live="polite">
  {toast}
</div>

<style>
  .shell {
    position: relative;
    z-index: 1;
    display: grid;
    grid-template-rows: auto 1fr auto;
    min-height: 100dvh;
    padding: 0 var(--gutter);
    padding-left: max(var(--gutter), env(safe-area-inset-left));
    padding-right: max(var(--gutter), env(safe-area-inset-right));
  }

  /* ------------------------------------------------------------- header */

  .bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    padding: 18px 0 8px;
    padding-top: max(18px, env(safe-area-inset-top));
  }

  .brand {
    display: flex;
    align-items: baseline;
    gap: 9px;
    white-space: nowrap;
  }

  .brand-name {
    font-size: 1.0625rem;
    font-weight: 800;
    font-stretch: 118%;
    letter-spacing: 0.02em;
  }

  .brand-sub {
    font-family: var(--font-mono);
    font-size: 0.6875rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--ink-soft);
  }

  .tools {
    display: flex;
    gap: 6px;
  }

  .tool {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: var(--panel);
    border: 1px solid var(--line);
    color: var(--ink-soft);
    transition:
      color 140ms ease,
      border-color 140ms ease,
      transform 140ms ease;
  }

  .tool:hover {
    color: var(--ink);
    border-color: var(--line-strong);
  }

  .tool:active {
    transform: scale(0.94);
  }

  .tool[aria-pressed="true"] {
    color: var(--base);
    background: var(--accent);
    border-color: transparent;
  }

  /* --------------------------------------------------------------- main */

  main {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: space-between;
    /* The gap is where the sun sits; it must never close completely. */
    gap: clamp(72px, 14vh, 150px);
    padding: clamp(14px, 3.6vh, 40px) 0 clamp(10px, 2vh, 24px);
    min-width: 0;
  }

  .hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    max-width: 100%;
  }

  .eyebrow {
    color: var(--ink-soft);
    text-shadow: 0 1px 10px rgb(var(--scrim) / 0.8);
  }

  .eyebrow.out {
    color: var(--accent);
  }

  h1 {
    margin: 6px 0 0;
    font-size: clamp(1.5rem, 4.4vw, 3.25rem);
    font-weight: 800;
    font-stretch: 122%;
    line-height: 1.05;
    letter-spacing: 0.005em;
    text-transform: uppercase;
    text-wrap: balance;
    text-shadow: 0 2px 24px rgb(var(--scrim) / 0.7);
  }

  .when {
    margin-top: clamp(10px, 1.8vh, 18px);
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    align-items: center;
    gap: 6px 12px;
    font-size: clamp(0.9375rem, 1.5vw, 1.125rem);
    font-weight: 500;
    text-shadow: 0 1px 12px rgb(var(--scrim) / 0.9);
  }

  .zone {
    padding: 4px 12px;
    border-radius: 999px;
    background: var(--panel);
    border: 1px solid var(--line);
    font-family: var(--font-mono);
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--ink-soft);
    text-shadow: none;
    transition:
      color 140ms ease,
      border-color 140ms ease;
  }

  .zone:hover {
    color: var(--ink);
    border-color: var(--line-strong);
  }

  .local {
    margin-top: 8px;
    font-family: var(--font-mono);
    font-size: 0.75rem;
    color: var(--ink-soft);
    text-shadow: 0 1px 10px rgb(var(--scrim) / 0.9);
  }

  .timer {
    margin-top: clamp(20px, 4.4vh, 48px);
    max-width: 100%;
  }

  .since {
    margin-top: 14px;
    color: var(--accent);
  }

  .details {
    width: min(100%, 980px);
    border-radius: var(--radius);
    background: var(--panel);
    border: 1px solid var(--line);
    overflow: hidden;
  }

  /* ------------------------------------------------------------- footer */

  footer {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 8px 24px;
    padding: 10px 0 16px;
    padding-bottom: max(16px, env(safe-area-inset-bottom));
    font-size: 0.75rem;
    color: var(--ink-faint);
  }

  .clock-source {
    flex: none;
    font-family: var(--font-mono);
    font-size: 0.6875rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  @media (max-width: 640px) {
    footer {
      flex-direction: column;
    }

    .brand-sub {
      display: none;
    }
  }

  /* -------------------------------------------------------------- toast */

  .toast {
    position: fixed;
    left: 50%;
    bottom: 56px;
    z-index: 20;
    max-width: min(92vw, 560px);
    padding: 10px 18px;
    border-radius: 999px;
    background: var(--panel-solid);
    border: 1px solid var(--line-strong);
    box-shadow: 0 12px 36px rgb(0 0 0 / 0.4);
    font-size: 0.875rem;
    text-align: center;
    overflow-wrap: anywhere;
    pointer-events: none;
    opacity: 0;
    transform: translate(-50%, 12px);
    transition:
      opacity 200ms ease,
      transform 320ms var(--ease-out);
  }

  .toast.visible {
    opacity: 1;
    transform: translate(-50%, 0);
  }

  /* Short windows: tighten the vertical rhythm so everything stays on one
     screen, then shed secondary content until only the timer is left. That
     last state doubles as a compact always-on-top widget on the desktop. */
  @media (max-height: 860px) and (min-width: 641px) {
    .bar {
      padding: 12px 0 4px;
    }

    main {
      gap: clamp(56px, 11vh, 90px);
      padding: 6px 0 8px;
    }

    .timer {
      margin-top: clamp(12px, 3vh, 24px);
    }

    footer {
      padding: 6px 0 10px;
    }
  }

  @media (max-height: 660px) and (min-width: 641px) {
    main {
      gap: clamp(44px, 9vh, 70px);
      padding-bottom: 14px;
    }

    footer {
      display: none;
    }
  }

  @media (max-height: 560px) {
    main {
      justify-content: center;
      gap: 0;
    }

    .details,
    footer {
      display: none;
    }
  }

  @media (max-height: 400px) {
    .eyebrow,
    .when,
    .local {
      display: none;
    }

    .timer {
      margin-top: 10px;
    }
  }
</style>
