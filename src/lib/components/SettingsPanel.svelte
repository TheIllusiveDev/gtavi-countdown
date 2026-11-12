<script lang="ts" module>
  import type { TimeSync } from "../platform";

  export type SyncState =
    | { status: "idle" }
    | { status: "syncing" }
    | { status: "ok"; result: TimeSync; at: number }
    | { status: "error"; message: string };
</script>

<script lang="ts">
  import Icon from "./Icon.svelte";
  import Switch from "./Switch.svelte";
  import ZonePicker from "./ZonePicker.svelte";
  import { isDesktop } from "../platform";
  import { EFFECT_LEVELS, THEMES, settings, type EffectLevel } from "../settings.svelte";
  import { THEME_INFO } from "../themes";

  let {
    open = $bindable(),
    sync,
    onsync,
    onnotify,
  }: {
    open: boolean;
    sync: SyncState;
    onsync: () => void;
    /** Called when the user turns notifications on; resolves to whether permission was granted. */
    onnotify: (enabled: boolean) => void;
  } = $props();

  let dialog: HTMLDialogElement | undefined = $state();

  // The native dialog provides the focus trap, Escape handling and inert
  // background; this effect only mirrors the `open` prop onto it.
  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    else if (!open && dialog.open) dialog.close();
  });

  const EFFECT_LABELS: Record<EffectLevel, string> = {
    auto: "Auto",
    low: "Low",
    off: "Off",
  };

  const syncText = $derived.by(() => {
    switch (sync.status) {
      case "syncing":
        return "Measuring the clock offset.";
      case "ok": {
        const offset = sync.result.offsetMs;
        const sign = offset >= 0 ? "+" : "-";
        const size = Math.abs(offset);
        const amount = size >= 1000 ? `${(size / 1000).toFixed(2)} s` : `${Math.round(size)} ms`;
        return `Corrected by ${sign}${amount} using ${sync.result.server} (round trip ${Math.round(sync.result.roundTripMs)} ms).`;
      }
      case "error":
        return `Using the system clock. ${sync.message}`;
      default:
        return "Using the system clock.";
    }
  });

  const SHORTCUTS: ReadonlyArray<readonly [string, string]> = [
    ["S", "Open or close settings"],
    ["F", "Toggle fullscreen"],
    ["M", "Toggle milliseconds"],
    ["T", "Next theme"],
    ["C", "Copy the countdown"],
    ["Esc", "Close settings"],
  ];
</script>

<dialog
  bind:this={dialog}
  aria-labelledby="settings-title"
  onclose={() => (open = false)}
  onclick={(event) => {
    // A click on the backdrop lands on the dialog element itself.
    if (event.target === dialog) open = false;
  }}
>
  <div class="sheet">
    <header>
      <h2 id="settings-title">Settings</h2>
      <button type="button" class="close" aria-label="Close settings" onclick={() => (open = false)}>
        <Icon name="close" />
      </button>
    </header>

    <div class="body">
      <section>
        <h3 class="label">Release time zone</h3>
        <p class="note">
          The game unlocks at midnight local time, so the moment depends on where you are. Zones
          are listed in the order they reach it.
        </p>
        {#if open}
          <ZonePicker
            value={settings.data.timeZone}
            onselect={(zone) => (settings.data.timeZone = zone)}
          />
        {/if}
      </section>

      <section>
        <h3 class="label">Display</h3>
        <div class="field">
          <span class="field-title" id="theme-label">Theme</span>
          <div class="themes" role="radiogroup" aria-labelledby="theme-label">
            {#each THEMES as theme (theme)}
              {@const info = THEME_INFO[theme]}
              <button
                type="button"
                class="theme"
                role="radio"
                aria-checked={settings.data.theme === theme}
                onclick={() => (settings.data.theme = theme)}
              >
                <span
                  class="swatch"
                  style:background="linear-gradient(180deg, {info.swatch[0]}, {info.swatch[1]} 55%, {info
                    .swatch[2]})"
                ></span>
                {info.label}
              </button>
            {/each}
          </div>
        </div>

        <div class="field">
          <span class="field-title" id="effects-label">Animated backdrop</span>
          <div class="segmented" role="radiogroup" aria-labelledby="effects-label">
            {#each EFFECT_LEVELS as level (level)}
              <button
                type="button"
                role="radio"
                aria-checked={settings.data.effects === level}
                onclick={() => (settings.data.effects = level)}
              >
                {EFFECT_LABELS[level]}
              </button>
            {/each}
          </div>
          <p class="note">
            Auto lowers the rendering resolution when the frame rate drops. Off replaces the scene
            with a still gradient.
          </p>
        </div>

        <Switch
          label="Milliseconds"
          hint="Adds a per-frame readout after the seconds."
          checked={settings.data.showMilliseconds}
          onchange={(value) => (settings.data.showMilliseconds = value)}
        />
      </section>

      <section>
        <h3 class="label">Alerts</h3>
        <Switch
          label="Notify me at release"
          hint="Shows a system notification when the countdown reaches zero while the app is open."
          checked={settings.data.notify}
          onchange={onnotify}
        />
      </section>

      {#if isDesktop}
        <section>
          <h3 class="label">Desktop</h3>
          <Switch
            label="Always on top"
            hint="Keeps the window above other windows."
            checked={settings.data.alwaysOnTop}
            onchange={(value) => (settings.data.alwaysOnTop = value)}
          />
          <Switch
            label="Network time"
            hint="Corrects the system clock against an NTP server at start-up."
            checked={settings.data.networkTime}
            onchange={(value) => (settings.data.networkTime = value)}
          />
          <div class="sync">
            <p class="note" aria-live="polite">{syncText}</p>
            <button
              type="button"
              class="button"
              disabled={!settings.data.networkTime || sync.status === "syncing"}
              onclick={onsync}
            >
              <Icon name="sync" size={15} />
              Sync now
            </button>
          </div>
        </section>
      {/if}

      <section>
        <h3 class="label">Keyboard shortcuts</h3>
        <dl class="shortcuts">
          {#each SHORTCUTS as [key, action] (key)}
            <div>
              <dt><kbd>{key}</kbd></dt>
              <dd>{action}</dd>
            </div>
          {/each}
        </dl>
      </section>

      <section class="reset">
        <button type="button" class="button" onclick={() => settings.reset()}>
          Restore defaults
        </button>
      </section>
    </div>
  </div>
</dialog>

<style>
  dialog {
    position: fixed;
    inset: 0 0 0 auto;
    width: min(420px, 100vw);
    height: 100dvh;
    max-width: none;
    max-height: none;
    margin: 0;
    padding: 0;
    border: 0;
    border-left: 1px solid var(--line);
    background: var(--panel-solid);
    color: var(--ink);
    overflow: hidden;
    box-shadow: -24px 0 60px rgb(0 0 0 / 0.35);
  }

  dialog[open] {
    animation: slide-in 320ms var(--ease-out);
  }

  dialog::backdrop {
    background: rgb(var(--scrim) / 0.55);
    animation: fade-in 240ms ease;
  }

  @keyframes slide-in {
    from {
      transform: translateX(32px);
      opacity: 0;
    }
  }

  @keyframes fade-in {
    from {
      opacity: 0;
    }
  }

  .sheet {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 18px 20px 14px 24px;
    border-bottom: 1px solid var(--line);
  }

  h2 {
    font-size: 1.125rem;
    font-weight: 700;
    font-stretch: 112%;
    letter-spacing: 0.01em;
  }

  .close {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 50%;
    color: var(--ink-soft);
  }

  .close:hover {
    background: rgb(255 255 255 / 0.1);
    color: var(--ink);
  }

  .body {
    flex: 1;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: 4px 24px 28px;
  }

  section {
    padding: 20px 0;
    border-bottom: 1px solid var(--line);
  }

  section:last-child {
    border-bottom: 0;
  }

  h3 {
    margin-bottom: 10px;
  }

  .note {
    font-size: 0.8125rem;
    line-height: 1.45;
    color: var(--ink-faint);
    margin-bottom: 12px;
    text-wrap: pretty;
  }

  .field {
    padding: 6px 0 10px;
  }

  .field .note {
    margin: 8px 0 0;
  }

  .field-title {
    display: block;
    font-size: 0.875rem;
    font-weight: 500;
    margin-bottom: 8px;
  }

  .themes {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }

  .theme {
    display: flex;
    flex-direction: column;
    gap: 7px;
    padding: 6px 6px 8px;
    border-radius: var(--radius-small);
    border: 1px solid var(--line);
    font-size: 0.8125rem;
    font-weight: 500;
    text-align: center;
    color: var(--ink-soft);
  }

  .theme:hover {
    border-color: var(--line-strong);
  }

  .theme[aria-checked="true"] {
    border-color: var(--accent);
    color: var(--ink);
    background: rgb(255 255 255 / 0.06);
  }

  .swatch {
    display: block;
    height: 44px;
    border-radius: 5px;
  }

  .segmented {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    padding: 3px;
    border-radius: var(--radius-small);
    background: rgb(255 255 255 / 0.08);
  }

  .segmented button {
    height: 32px;
    border-radius: 6px;
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--ink-soft);
  }

  .segmented button[aria-checked="true"] {
    background: rgb(255 255 255 / 0.18);
    color: var(--ink);
  }

  .sync {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding-top: 6px;
  }

  .button {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 34px;
    padding: 0 14px;
    border-radius: 17px;
    border: 1px solid var(--line-strong);
    font-size: 0.8125rem;
    font-weight: 500;
  }

  .button:hover:not(:disabled) {
    background: rgb(255 255 255 / 0.1);
  }

  .button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .shortcuts {
    margin: 0;
    display: grid;
    gap: 7px;
  }

  .shortcuts div {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  dt {
    flex: none;
    width: 44px;
  }

  dd {
    margin: 0;
    font-size: 0.8125rem;
    color: var(--ink-soft);
  }

  kbd {
    display: inline-block;
    min-width: 26px;
    padding: 2px 6px;
    border-radius: 5px;
    border: 1px solid var(--line-strong);
    background: rgb(255 255 255 / 0.06);
    font-family: var(--font-mono);
    font-size: 0.6875rem;
    text-align: center;
  }

  .reset {
    padding-bottom: 0;
  }
</style>
