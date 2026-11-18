import { useEffect, useRef } from "react";
import { cx } from "../lib/cx";
import { clock } from "../lib/time/clock";
import type { Breakdown } from "../lib/time/countdown";
import { Digit } from "./Digit";
import styles from "./Countdown.module.css";

const pad = (n: number, width = 2) => String(n).padStart(width, "0");

export function Countdown({
  parts,
  target,
  showMilliseconds,
}: {
  parts: Breakdown;
  target: number;
  showMilliseconds: boolean;
}) {
  const units = [
    { key: "days", label: parts.days === 1 ? "Day" : "Days", text: pad(parts.days) },
    { key: "hours", label: parts.hours === 1 ? "Hour" : "Hours", text: pad(parts.hours) },
    { key: "minutes", label: parts.minutes === 1 ? "Minute" : "Minutes", text: pad(parts.minutes) },
    { key: "seconds", label: parts.seconds === 1 ? "Second" : "Seconds", text: pad(parts.seconds) },
  ];

  /** Spoken form for assistive technology; the animated digits are hidden. */
  const spoken =
    `${parts.days} days, ${parts.hours} hours, ${parts.minutes} minutes` +
    (parts.elapsed ? " since release" : " remaining");

  const millis = useRef<HTMLSpanElement>(null);

  // The milliseconds readout changes every frame. Routing that through React
  // would re-render sixty times a second, so the frame loop writes the text
  // node directly and nothing else is touched.
  useEffect(() => {
    const node = millis.current;
    if (!showMilliseconds || !node) return;
    let frame = 0;
    let last = "";
    const paint = () => {
      frame = requestAnimationFrame(paint);
      const text = pad(Math.floor(Math.abs(target - clock.precise()) % 1000), 3);
      if (text !== last) {
        node.textContent = text;
        last = text;
      }
    };
    paint();
    return () => cancelAnimationFrame(frame);
  }, [showMilliseconds, target]);

  return (
    <div className={cx(styles.countdown, showMilliseconds && styles.withMillis)} role="timer" aria-label={spoken}>
      {units.map((unit, index) => (
        <div key={unit.key} className={styles.group}>
          {index > 0 && <span className={styles.separator} aria-hidden="true" />}
          <div className={styles.unit}>
            <div className={styles.digits}>
              {/* Digits are keyed by their position from the right, so a
                  change in their number (100 days to 99) animates none of
                  the others. */}
              {[...unit.text].map((char, position) => (
                <Digit key={unit.text.length - position} value={char} />
              ))}
              {unit.key === "seconds" && showMilliseconds && (
                <span className={styles.millis} aria-hidden="true">
                  .<span ref={millis}>000</span>
                </span>
              )}
            </div>
            <div className={cx("label", styles.unitLabel)} aria-hidden="true">
              {unit.label}
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}
