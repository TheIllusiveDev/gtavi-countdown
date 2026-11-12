import { MILESTONES, RELEASE, milestoneInstant } from "./release";
import { calendarDaysBetween, weekendsBetween, zonedTimeToUtc } from "./zones";

export const SECOND = 1000;
export const MINUTE = 60 * SECOND;
export const HOUR = 60 * MINUTE;
export const DAY = 24 * HOUR;

export interface Breakdown {
  /** True once the target instant has passed; the parts then count upwards. */
  elapsed: boolean;
  days: number;
  hours: number;
  minutes: number;
  seconds: number;
  milliseconds: number;
  /** Absolute distance to the target in milliseconds. */
  total: number;
}

/** The instant the game unlocks for someone whose clock reads `timeZone`. */
export function releaseInstant(timeZone: string): number {
  return zonedTimeToUtc(
    RELEASE.year,
    RELEASE.month,
    RELEASE.day,
    RELEASE.hour,
    RELEASE.minute,
    timeZone,
  );
}

/** Splits the distance between `now` and `target` into display units. */
export function breakdown(now: number, target: number): Breakdown {
  const diff = target - now;
  const elapsed = diff <= 0;
  const total = Math.abs(diff);
  return {
    elapsed,
    total,
    days: Math.floor(total / DAY),
    hours: Math.floor((total % DAY) / HOUR),
    minutes: Math.floor((total % HOUR) / MINUTE),
    seconds: Math.floor((total % MINUTE) / SECOND),
    milliseconds: Math.floor(total % SECOND),
  };
}

/**
 * Milliseconds until the display changes next. Scheduling the tick on this
 * boundary keeps the display in phase with the target instead of with the
 * moment the page happened to load.
 *
 * Before the target the seconds are floored, so the reading drops one
 * millisecond after each whole second of remaining time; the final change is
 * the flip to "elapsed" at the target itself. After the target the reading
 * rises on each whole second of elapsed time.
 */
export function msUntilNextSecond(now: number, target: number): number {
  const diff = target - now;
  if (diff <= 0) return SECOND - (-diff % SECOND);
  if (diff < SECOND) return diff;
  return (diff % SECOND) + 1;
}

export interface Totals {
  weeks: number;
  hours: number;
  minutes: number;
  seconds: number;
  /** Midnights between now and the target in the target zone. */
  sleeps: number;
  /** Full weekends (Saturdays) between now and the target. */
  weekends: number;
}

/** Alternative single-unit readings of the remaining time. */
export function totals(now: number, target: number, timeZone: string): Totals {
  const remaining = Math.max(0, target - now);
  return {
    weeks: remaining / (7 * DAY),
    hours: Math.floor(remaining / HOUR),
    minutes: Math.floor(remaining / MINUTE),
    seconds: Math.floor(remaining / SECOND),
    sleeps: remaining === 0 ? 0 : Math.max(0, calendarDaysBetween(now, target, timeZone)),
    weekends: remaining === 0 ? 0 : weekendsBetween(now, target, timeZone),
  };
}

/** The instant the wait began: the publication of the first trailer. */
export const WAIT_ORIGIN = milestoneInstant(MILESTONES[0]!.date);

/** Fraction of the wait since the first trailer that is over, from 0 to 1. */
export function waitProgress(now: number, target: number): number {
  const span = target - WAIT_ORIGIN;
  if (span <= 0) return 1;
  return Math.min(1, Math.max(0, (now - WAIT_ORIGIN) / span));
}

/** Position of an instant on the wait timeline, from 0 to 1. */
export function timelinePosition(instant: number, target: number): number {
  const span = target - WAIT_ORIGIN;
  if (span <= 0) return 1;
  return Math.min(1, Math.max(0, (instant - WAIT_ORIGIN) / span));
}

/** Compact one-line reading such as `6d 03h 12m 08s`. */
export function formatCompact(b: Breakdown): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${b.days}d ${pad(b.hours)}h ${pad(b.minutes)}m ${pad(b.seconds)}s`;
}
