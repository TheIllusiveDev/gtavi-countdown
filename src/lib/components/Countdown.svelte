<script lang="ts">
  import Digit from "./Digit.svelte";
  import { clock } from "../time/clock.svelte";
  import type { Breakdown } from "../time/countdown";

  let {
    parts,
    target,
    showMilliseconds,
  }: { parts: Breakdown; target: number; showMilliseconds: boolean } = $props();

  const pad = (n: number, width = 2) => String(n).padStart(width, "0");

  const units = $derived([
    { key: "days", label: parts.days === 1 ? "Day" : "Days", text: pad(parts.days) },
    { key: "hours", label: parts.hours === 1 ? "Hour" : "Hours", text: pad(parts.hours) },
    { key: "minutes", label: parts.minutes === 1 ? "Minute" : "Minutes", text: pad(parts.minutes) },
    { key: "seconds", label: parts.seconds === 1 ? "Second" : "Seconds", text: pad(parts.seconds) },
  ]);

  /** Spoken form for assistive technology; the animated digits are hidden. */
  const spoken = $derived(
    `${parts.days} days, ${parts.hours} hours, ${parts.minutes} minutes` +
      (parts.elapsed ? " since release" : " remaining"),
  );

  let millis: HTMLSpanElement | undefined = $state();

  // The milliseconds readout changes every frame. Routing that through the
  // reactive graph would re-run derivations sixty times a second, so the
  // frame loop writes the text node directly and nothing else is touched.
  $effect(() => {
    const node = millis;
    if (!showMilliseconds || !node) return;
    const goal = target;
    let frame = 0;
    let last = "";
    const paint = () => {
      frame = requestAnimationFrame(paint);
      const text = pad(Math.floor(Math.abs(goal - clock.precise()) % 1000), 3);
      if (text !== last) {
        node.textContent = text;
        last = text;
      }
    };
    paint();
    return () => cancelAnimationFrame(frame);
  });
</script>

<div class="countdown" class:with-millis={showMilliseconds} role="timer" aria-label={spoken}>
  {#each units as unit, index (unit.key)}
    {#if index > 0}
      <span class="separator" aria-hidden="true"></span>
    {/if}
    <div class="unit">
      <div class="digits">
        {#each unit.text as char, position (unit.text.length - position)}
          <Digit value={char} />
        {/each}
        {#if unit.key === "seconds" && showMilliseconds}
          <span class="millis" aria-hidden="true">.<span bind:this={millis}>000</span></span>
        {/if}
      </div>
      <div class="label unit-label" aria-hidden="true">{unit.label}</div>
    </div>
  {/each}
</div>

<style>
  .countdown {
    --size: clamp(3.4rem, min(18.5vw, 23vh), 12.75rem);
    display: flex;
    justify-content: center;
    align-items: flex-start;
    gap: calc(var(--size) * 0.07);
    font-size: var(--size);
    font-weight: 800;
    font-stretch: 62%;
    line-height: 1;
    letter-spacing: -0.01em;
    font-variant-numeric: tabular-nums lining-nums;
    color: var(--ink);
    /* Two shadows: a tight one for edge contrast, a wide one for depth. */
    text-shadow:
      0 0.02em 0.03em rgb(var(--scrim) / 0.55),
      0 0.1em 0.5em rgb(var(--scrim) / 0.5);
    user-select: none;
  }

  /* The readout overhangs the row on the right; the figures shrink a little
     on narrow screens so that it stays inside the viewport. */
  .countdown.with-millis {
    --size: clamp(3rem, min(15.5vw, 23vh), 12.75rem);
  }

  .unit {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: calc(var(--size) * 0.06);
  }

  .digits {
    position: relative;
    display: flex;
  }

  /* The colon is drawn rather than typeset: two round dots, centred on the
     height of the figures, that breathe once per two seconds. */
  .separator {
    flex: none;
    width: 0.09em;
    height: 1em;
    background:
      radial-gradient(circle closest-side, currentColor 92%, transparent) center 34% / 100% 0.09em
        no-repeat,
      radial-gradient(circle closest-side, currentColor 92%, transparent) center 68% / 100% 0.09em
        no-repeat;
    opacity: 0.5;
    animation: pulse 2s ease-in-out infinite;
  }

  .unit-label {
    font-size: clamp(0.625rem, calc(var(--size) * 0.085), 0.9rem);
    color: var(--ink-soft);
    text-shadow: 0 1px 8px rgb(var(--scrim) / 0.8);
  }

  .millis {
    font-family: var(--font-mono);
    font-size: 0.26em;
    font-weight: 600;
    font-stretch: 100%;
    letter-spacing: 0;
    color: var(--accent);
    /* Hung to the right of the seconds, on the baseline of the figures and
       out of the flow, so that toggling it never moves the main digits. */
    position: absolute;
    left: 100%;
    bottom: 0.52em;
    margin-left: 0.12em;
    line-height: 1;
    white-space: nowrap;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 0.5;
    }
    50% {
      opacity: 0.22;
    }
  }
</style>
