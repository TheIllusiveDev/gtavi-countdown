<script lang="ts">
  import { MILESTONES, RELEASE, milestoneInstant } from "../time/release";
  import { timelinePosition, waitProgress } from "../time/countdown";

  let { now, target }: { now: number; target: number } = $props();

  const dateFormat = new Intl.DateTimeFormat("en-US", {
    timeZone: "UTC",
    year: "numeric",
    month: "short",
    day: "numeric",
  });

  const releaseDateText = dateFormat.format(
    Date.UTC(RELEASE.year, RELEASE.month - 1, RELEASE.day, 12),
  );

  const progress = $derived(waitProgress(now, target));

  /** Two decimals while counting; the bar itself moves about 0.1% per day. */
  const percent = $derived(
    progress >= 1 ? "100" : (Math.floor(progress * 10000) / 100).toFixed(2),
  );

  const marks = $derived(
    MILESTONES.map((milestone, index) => {
      const instant = milestoneInstant(milestone.date);
      return {
        ...milestone,
        position: timelinePosition(instant, target),
        dateText: dateFormat.format(instant),
        first: index === 0,
      };
    }),
  );
</script>

<section class="timeline" aria-label="Progress of the wait since the first trailer">
  <header>
    <span class="label">The wait since Trailer 1</span>
    <span class="percent"><strong>{percent}</strong>% over</span>
  </header>

  <div
    class="track"
    role="progressbar"
    aria-valuemin="0"
    aria-valuemax="100"
    aria-valuenow={Math.floor(progress * 100)}
    aria-label="Share of the wait that is over"
  >
    <div class="fill" style:transform="scaleX({progress})"></div>
    {#each marks as mark (mark.date)}
      <span
        class="mark"
        class:first={mark.first}
        style:left="{mark.position * 100}%"
        title="{mark.dateText}: {mark.detail}"
      ></span>
    {/each}
    <span class="mark end" class:reached={progress >= 1} style:left="100%"></span>
  </div>

  <ol class="legend">
    {#each marks as mark (mark.date)}
      <li style:left="{mark.position * 100}%" class:first={mark.first}>
        <span class="legend-title">{mark.title}</span>
        <span class="legend-date">{mark.dateText}</span>
      </li>
    {/each}
    <li class="last" style:left="100%">
      <span class="legend-title">Release</span>
      <span class="legend-date">{releaseDateText}</span>
    </li>
  </ol>
</section>

<style>
  .timeline {
    padding: 15px 18px 12px;
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 10px;
  }

  .percent {
    font-family: var(--font-mono);
    font-size: 0.75rem;
    color: var(--ink-soft);
    font-variant-numeric: tabular-nums;
  }

  .percent strong {
    color: var(--ink);
    font-weight: 600;
  }

  .track {
    position: relative;
    height: 6px;
    border-radius: 3px;
    background: rgb(255 255 255 / 0.16);
  }

  .fill {
    position: absolute;
    inset: 0;
    border-radius: 3px;
    background: linear-gradient(90deg, var(--accent-2), var(--accent));
    transform-origin: left center;
    /* Scaling keeps the update on the compositor; no layout is involved. */
    will-change: transform;
  }

  .mark {
    position: absolute;
    top: 50%;
    width: 10px;
    height: 10px;
    margin: -5px 0 0 -5px;
    border-radius: 50%;
    background: var(--ink);
    box-shadow: 0 0 0 3px rgb(var(--scrim) / 0.55);
  }

  .mark.first {
    margin-left: 0;
  }

  .mark.end {
    margin-left: -10px;
    background: var(--panel-solid);
    box-shadow:
      inset 0 0 0 2px var(--accent),
      0 0 0 3px rgb(var(--scrim) / 0.55);
  }

  .mark.end.reached {
    background: var(--accent);
  }

  .legend {
    position: relative;
    height: 2.4rem;
    margin: 10px 0 0;
    padding: 0;
    list-style: none;
  }

  .legend li {
    position: absolute;
    top: 0;
    display: flex;
    flex-direction: column;
    transform: translateX(-50%);
    text-align: center;
    white-space: nowrap;
    line-height: 1.3;
  }

  .legend li.first {
    transform: none;
    text-align: left;
  }

  .legend li.last {
    transform: translateX(-100%);
    text-align: right;
  }

  .legend-title {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--ink);
  }

  .legend-date {
    font-family: var(--font-mono);
    font-size: 0.6875rem;
    color: var(--ink-faint);
  }

  @media (max-width: 560px) {
    /* The labels would collide on narrow screens; the dots keep their
       tooltips and the first and last labels anchor the scale. */
    .legend li:not(.first):not(.last) {
      display: none;
    }
  }
</style>
