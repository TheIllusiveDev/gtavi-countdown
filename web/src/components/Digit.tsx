import { useState } from "react";
import { cx } from "../lib/cx";
import styles from "./Digit.module.css";

/**
 * One figure that rolls when it changes: the old glyph slides down and fades
 * while the new one slides in from above. Both are CSS animations of
 * `transform` and `opacity`, so the roll runs on the compositor and costs no
 * script time per frame. Reduced motion is handled by the global stylesheet.
 */
export function Digit({ value }: { value: string }) {
  const [state, setState] = useState({ current: value, previous: null as string | null, key: 0 });
  // Derived state: a new value starts a roll on this very render.
  if (state.current !== value) {
    setState({ current: value, previous: state.current, key: state.key + 1 });
  }
  const key = state.key;
  return (
    <span className={styles.cell} aria-hidden="true">
      {state.previous !== null && (
        <span
          key={`out-${key}`}
          className={cx(styles.glyph, styles.out)}
          onAnimationEnd={() => setState((s) => (s.key === key ? { ...s, previous: null } : s))}
        >
          {state.previous}
        </span>
      )}
      {/* The first value appears without a roll. */}
      <span key={`in-${key}`} className={cx(styles.glyph, key > 0 && styles.in)}>
        {state.current}
      </span>
    </span>
  );
}
