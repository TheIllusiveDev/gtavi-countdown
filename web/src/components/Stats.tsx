import { totals } from "../lib/time/countdown";
import styles from "./Stats.module.css";

const integer = new Intl.NumberFormat("en-US", { maximumFractionDigits: 0 });

export function Stats({ now, target, timeZone }: { now: number; target: number; timeZone: string }) {
  const t = totals(now, target, timeZone);
  const tiles = [
    { key: "sleeps", label: t.sleeps === 1 ? "Sleep" : "Sleeps", value: t.sleeps },
    { key: "weekends", label: t.weekends === 1 ? "Weekend" : "Weekends", value: t.weekends },
    { key: "hours", label: "Total hours", value: t.hours },
    { key: "minutes", label: "Total minutes", value: t.minutes },
    { key: "seconds", label: "Total seconds", value: t.seconds },
  ];
  return (
    <section className={styles.stats} aria-label="The remaining time in other units">
      {tiles.map((tile) => (
        <div key={tile.key} className={styles.tile}>
          <span className={styles.value}>{integer.format(tile.value)}</span>
          <span className="label">{tile.label}</span>
        </div>
      ))}
    </section>
  );
}
