import { msUntilNextSecond } from "./countdown";

/**
 * A reactive wall clock that ticks once per second, in phase with a target.
 *
 * `now` only changes when the displayed seconds value changes, so everything
 * derived from it re-renders at most once per second. Sub-second output (the
 * optional milliseconds readout) reads `precise()` from an animation frame and
 * writes straight to the DOM, bypassing reactivity entirely.
 *
 * Every tick re-reads the system clock, so a throttled background tab, a
 * suspended laptop or a late timer can delay an update but never skew it.
 */
class Clock {
  /** Corrected current time in Unix milliseconds, updated once per second. */
  now = $state(Date.now());

  /**
   * Correction added to the system clock, in milliseconds. Set from a network
   * time measurement on the desktop; zero when the system clock is trusted.
   */
  offset = $state(0);

  #target = 0;
  #timer: ReturnType<typeof setTimeout> | undefined;
  #running = false;

  /** Corrected current time, read directly from the system clock. */
  precise(): number {
    return Date.now() + this.offset;
  }

  /** Starts ticking in phase with `target`. Returns a stop function. */
  start(target: number): () => void {
    this.#target = target;
    if (!this.#running) {
      this.#running = true;
      document.addEventListener("visibilitychange", this.#resync);
      window.addEventListener("focus", this.#resync);
      window.addEventListener("pageshow", this.#resync);
    }
    this.#resync();
    return () => this.stop();
  }

  /** Re-aligns the tick with a new target, for example after a zone change. */
  retarget(target: number): void {
    this.#target = target;
    if (this.#running) this.#resync();
  }

  /** Applies a new clock correction and re-aligns the tick immediately. */
  setOffset(offset: number): void {
    this.offset = offset;
    if (this.#running) this.#resync();
  }

  stop(): void {
    this.#running = false;
    clearTimeout(this.#timer);
    document.removeEventListener("visibilitychange", this.#resync);
    window.removeEventListener("focus", this.#resync);
    window.removeEventListener("pageshow", this.#resync);
  }

  #resync = (): void => {
    clearTimeout(this.#timer);
    if (!this.#running) return;
    const now = this.precise();
    this.now = now;
    // Timers may fire a fraction of a millisecond early. Should that happen,
    // this function simply runs again with a wait of a millisecond or two.
    this.#timer = setTimeout(this.#resync, msUntilNextSecond(now, this.#target));
  };
}

export const clock = new Clock();
