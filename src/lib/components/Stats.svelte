<script lang="ts">
  import { totals } from "../time/countdown";

  let { now, target, timeZone }: { now: number; target: number; timeZone: string } = $props();

  const integer = new Intl.NumberFormat("en-US", { maximumFractionDigits: 0 });

  const t = $derived(totals(now, target, timeZone));

  const tiles = $derived([
    { key: "sleeps", label: t.sleeps === 1 ? "Sleep" : "Sleeps", value: integer.format(t.sleeps) },
    {
      key: "weekends",
      label: t.weekends === 1 ? "Weekend" : "Weekends",
      value: integer.format(t.weekends),
    },
    { key: "hours", label: "Total hours", value: integer.format(t.hours) },
    { key: "minutes", label: "Total minutes", value: integer.format(t.minutes) },
    { key: "seconds", label: "Total seconds", value: integer.format(t.seconds) },
  ]);
</script>

<section class="stats" aria-label="The remaining time in other units">
  {#each tiles as tile (tile.key)}
    <div class="tile">
      <span class="value">{tile.value}</span>
      <span class="label">{tile.label}</span>
    </div>
  {/each}
</section>

<style>
  .stats {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    border-top: 1px solid var(--line);
    /* Clips the dividers that fall on the outer edge. */
    overflow: hidden;
  }

  .tile {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 13px 18px 13px;
    min-width: 0;
    /* Hairline dividers: every tile draws its top and left edge, and the
       container clips the ones that fall on its own border. */
    box-shadow: -1px -1px 0 var(--line);
    /* A change in one tile never invalidates the layout of another. */
    contain: layout;
  }

  .value {
    font-size: clamp(1.15rem, 2.1vw, 1.6rem);
    font-weight: 700;
    font-stretch: 88%;
    line-height: 1.15;
    font-variant-numeric: tabular-nums lining-nums;
    white-space: nowrap;
  }

  @media (max-width: 720px) {
    .stats {
      grid-template-columns: repeat(6, minmax(0, 1fr));
    }

    .tile {
      grid-column: span 2;
      padding: 12px 13px 11px;
    }

    .tile:nth-child(-n + 2) {
      grid-column: span 3;
    }
  }

  /* Phones: two columns, so the "Total ..." labels stay on one line. The
     largest figure, total seconds, takes the full last row. */
  @media (max-width: 520px) {
    .stats {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .tile,
    .tile:nth-child(-n + 2) {
      grid-column: span 1;
      padding: 12px 16px 11px;
    }

    .tile:last-child {
      grid-column: 1 / -1;
    }
  }
</style>
