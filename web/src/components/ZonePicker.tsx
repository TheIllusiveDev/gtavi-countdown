import { useMemo, useState } from "react";
import { formatOffset, listTimeZones, offsetAt, systemTimeZone, zoneCity, zoneRegion } from "../lib/time/zones";
import { Icon } from "./Icon";
import styles from "./ZonePicker.module.css";

interface ZoneEntry {
  id: string;
  city: string;
  region: string;
  offset: number;
  offsetText: string;
  /** Lower-cased text the filter matches against. */
  haystack: string;
}

function entry(id: string, reference: number): ZoneEntry {
  const offset = offsetAt(reference, id);
  const offsetText = formatOffset(offset);
  const city = zoneCity(id);
  const region = zoneRegion(id);
  return { id, city, region, offset, offsetText, haystack: `${id} ${city} ${region} ${offsetText}`.toLowerCase() };
}

/**
 * The filterable list of zones. Offsets are evaluated at `reference`, the
 * selected release, not today, so that zones which change their clocks
 * before then are listed with the right value.
 */
export function ZonePicker({
  value,
  reference,
  onSelect,
}: {
  value: string | null;
  reference: number;
  onSelect: (timeZone: string | null) => void;
}) {
  // The zone furthest east unlocks first, so the list doubles as the order in
  // which the release moves around the planet.
  const zones = useMemo(
    () =>
      listTimeZones()
        .map((id) => entry(id, reference))
        .sort((a, b) => b.offset - a.offset || a.city.localeCompare(b.city)),
    [reference],
  );
  const system = useMemo(() => entry(systemTimeZone(), reference), [reference]);
  const [query, setQuery] = useState("");

  const needle = query.trim().toLowerCase().replaceAll("_", " ");
  const words = needle ? needle.split(/\s+/) : [];
  const matches = words.length ? zones.filter((zone) => words.every((word) => zone.haystack.includes(word))) : zones;

  return (
    <div className={styles.picker}>
      <label className={styles.search}>
        <Icon name="search" size={16} />
        <span className="visually-hidden">Filter time zones</span>
        <input
          className={styles.input}
          type="search"
          placeholder="Search city, region or offset"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          autoComplete="off"
          autoCapitalize="off"
          spellCheck={false}
        />
      </label>

      <ul className={styles.list} aria-label="Time zones">
        {!needle && (
          <li className={styles.item}>
            <button type="button" className={styles.option} aria-pressed={value === null} onClick={() => onSelect(null)}>
              <span className={styles.name}>
                <span className={styles.city}>Follow this device</span>
                <span className={styles.region}>{system.city}</span>
              </span>
              <span className={styles.offset}>{system.offsetText}</span>
            </button>
          </li>
        )}
        {matches.map((zone) => (
          <li key={zone.id} className={styles.item}>
            <button
              type="button"
              className={styles.option}
              aria-pressed={value === zone.id}
              onClick={() => onSelect(zone.id)}
            >
              <span className={styles.name}>
                <span className={styles.city}>{zone.city}</span>
                {zone.region && <span className={styles.region}>{zone.region}</span>}
              </span>
              <span className={styles.offset}>{zone.offsetText}</span>
            </button>
          </li>
        ))}
        {matches.length === 0 && <li className={styles.empty}>No time zone matches "{query.trim()}".</li>}
      </ul>
    </div>
  );
}
