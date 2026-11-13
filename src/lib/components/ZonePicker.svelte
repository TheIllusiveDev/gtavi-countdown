<script lang="ts">
  import Icon from "./Icon.svelte";
  import { releaseInstant } from "../time/countdown";
  import {
    formatOffset,
    listTimeZones,
    offsetAt,
    systemTimeZone,
    zoneCity,
    zoneRegion,
  } from "../time/zones";

  let {
    value,
    onselect,
  }: { value: string | null; onselect: (timeZone: string | null) => void } = $props();

  interface ZoneEntry {
    id: string;
    city: string;
    region: string;
    offset: number;
    offsetText: string;
    /** Lower-cased text the filter matches against. */
    haystack: string;
  }

  const system = systemTimeZone();

  // Offsets are evaluated at the release itself, not today, so that zones
  // which change their clocks before then are listed with the right value.
  const reference = releaseInstant("UTC");

  function entry(id: string): ZoneEntry {
    const offset = offsetAt(reference, id);
    const offsetText = formatOffset(offset);
    const city = zoneCity(id);
    const region = zoneRegion(id);
    return {
      id,
      city,
      region,
      offset,
      offsetText,
      haystack: `${id} ${city} ${region} ${offsetText}`.toLowerCase(),
    };
  }

  // Built once per mount. The zone furthest east unlocks first, so the list
  // doubles as the order in which the release moves around the planet.
  const zones: ZoneEntry[] = listTimeZones()
    .map(entry)
    .sort((a, b) => b.offset - a.offset || a.city.localeCompare(b.city));

  const systemEntry = entry(system);

  let query = $state("");

  const matches = $derived.by(() => {
    const needle = query.trim().toLowerCase().replaceAll("_", " ");
    if (!needle) return zones;
    const words = needle.split(/\s+/);
    return zones.filter((zone) => words.every((word) => zone.haystack.includes(word)));
  });
</script>

<div class="picker">
  <label class="search">
    <Icon name="search" size={16} />
    <span class="visually-hidden">Filter time zones</span>
    <input
      type="search"
      placeholder="Search city, region or offset"
      bind:value={query}
      autocomplete="off"
      autocapitalize="off"
      spellcheck="false"
    />
  </label>

  <ul class="list" aria-label="Time zones">
    {#if !query.trim()}
      <li>
        <button
          type="button"
          class="option"
          aria-pressed={value === null}
          onclick={() => onselect(null)}
        >
          <span class="name">
            <span class="city">Follow this device</span>
            <span class="region">{systemEntry.city}</span>
          </span>
          <span class="offset">{systemEntry.offsetText}</span>
        </button>
      </li>
    {/if}
    {#each matches as zone (zone.id)}
      <li>
        <button
          type="button"
          class="option"
          aria-pressed={value === zone.id}
          onclick={() => onselect(zone.id)}
        >
          <span class="name">
            <span class="city">{zone.city}</span>
            {#if zone.region}<span class="region">{zone.region}</span>{/if}
          </span>
          <span class="offset">{zone.offsetText}</span>
        </button>
      </li>
    {:else}
      <li class="empty">No time zone matches "{query.trim()}".</li>
    {/each}
  </ul>
</div>

<style>
  .picker {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 11px;
    height: 38px;
    border-radius: var(--radius-small);
    background: rgb(255 255 255 / 0.07);
    border: 1px solid var(--line);
    color: var(--ink-faint);
  }

  .search:focus-within {
    border-color: var(--accent);
  }

  input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: 0;
    outline: 0;
    background: none;
    color: var(--ink);
    font-size: 0.875rem;
  }

  /* iOS zooms the page into any focused field under 16px. */
  @media (pointer: coarse) {
    input {
      font-size: 16px;
    }

    .search {
      height: 44px;
    }

    .option {
      min-height: 44px;
    }
  }

  input::placeholder {
    color: var(--ink-faint);
  }

  .list {
    margin: 0;
    padding: 4px;
    list-style: none;
    height: 216px;
    overflow-y: auto;
    overscroll-behavior: contain;
    border-radius: var(--radius-small);
    border: 1px solid var(--line);
    background: rgb(0 0 0 / 0.18);
  }

  li {
    /* Rows far outside the scroll viewport skip layout and paint. */
    content-visibility: auto;
    contain-intrinsic-size: auto 36px;
  }

  .option {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-height: 36px;
    padding: 5px 9px;
    border-radius: 6px;
    text-align: left;
  }

  @media (hover: hover) {
    .option:hover {
      background: rgb(255 255 255 / 0.08);
    }
  }

  .option[aria-pressed="true"] {
    background: rgb(255 255 255 / 0.14);
    box-shadow: inset 2px 0 0 var(--accent);
  }

  .name {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
  }

  .city {
    font-size: 0.875rem;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .region {
    font-size: 0.75rem;
    color: var(--ink-faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .offset {
    flex: none;
    font-family: var(--font-mono);
    font-size: 0.6875rem;
    color: var(--ink-soft);
  }

  .empty {
    padding: 14px 9px;
    font-size: 0.8125rem;
    color: var(--ink-faint);
  }
</style>
