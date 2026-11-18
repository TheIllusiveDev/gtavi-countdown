import type { CSSProperties } from "react";
import { cx } from "../lib/cx";
import { timelinePosition, waitProgress } from "../lib/time/countdown";
import { noonUtc, waitCaption, type Game } from "../lib/time/games";
import styles from "./Timeline.module.css";

const dateFormat = new Intl.DateTimeFormat("en-US", {
  timeZone: "UTC",
  year: "numeric",
  month: "short",
  day: "numeric",
});

/** Labels closer than this to a shown neighbour or to the release are left
 * out; their dots keep their tooltips. */
const MIN_LABEL_GAP = 0.16;

export function Timeline({ game, now, target }: { game: Game; now: number; target: number }) {
  const progress = waitProgress(game, now, target);
  /** Two decimals while counting; the bar itself moves about 0.1% per day. */
  const percent = progress >= 1 ? "100" : (Math.floor(progress * 10000) / 100).toFixed(2);

  let lastShown = -1;
  const marks = game.milestones.map((milestone, index) => {
    const instant = noonUtc(milestone.date);
    const position = timelinePosition(game, instant, target);
    const first = index === 0;
    const labelled = first || (position - lastShown >= MIN_LABEL_GAP && 1 - position >= MIN_LABEL_GAP);
    if (labelled) lastShown = position;
    return { ...milestone, position, dateText: dateFormat.format(instant), first, labelled };
  });
  const at = (position: number) => ({ left: `${position * 100}%` }) as CSSProperties;

  return (
    <section className={styles.timeline} aria-label={`Progress of the wait since ${game.milestones[0]?.title}`}>
      <header className={styles.header}>
        <span className="label">{waitCaption(game)}</span>
        <span className={styles.percent}>
          <strong>{percent}</strong>% over
        </span>
      </header>

      <div
        className={styles.track}
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.floor(progress * 100)}
        aria-label="Share of the wait that is over"
      >
        <div className={styles.fill} style={{ transform: `scaleX(${progress})` }} />
        {marks.map((mark) => (
          <span
            key={mark.date + mark.title}
            className={cx(styles.mark, mark.first && styles.first)}
            style={at(mark.position)}
            title={`${mark.dateText}: ${mark.detail}`}
          />
        ))}
        <span className={cx(styles.mark, styles.end, progress >= 1 && styles.reached)} style={at(1)} />
      </div>

      <ol className={styles.legend}>
        {marks
          .filter((mark) => mark.labelled)
          .map((mark) => (
            <li key={mark.date + mark.title} style={at(mark.position)} className={cx(mark.first && styles.first)}>
              <span className={styles.legendTitle}>{mark.title}</span>
              <span className={styles.legendDate}>{mark.dateText}</span>
            </li>
          ))}
        <li className={styles.last} style={at(1)}>
          <span className={styles.legendTitle}>Release</span>
          <span className={styles.legendDate}>{dateFormat.format(noonUtc(game.date))}</span>
        </li>
      </ol>
    </section>
  );
}
