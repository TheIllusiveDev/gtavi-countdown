import { describe, expect, test } from "bun:test";
import {
  DAY,
  HOUR,
  MINUTE,
  SECOND,
  WAIT_ORIGIN,
  breakdown,
  formatCompact,
  msUntilNextSecond,
  releaseInstant,
  timelinePosition,
  totals,
  waitProgress,
} from "../src/lib/time/countdown";
import { MILESTONES, milestoneInstant } from "../src/lib/time/release";

const target = releaseInstant("UTC");

describe("releaseInstant", () => {
  test("is midnight on 19 November 2026 in the given zone", () => {
    expect(new Date(target).toISOString()).toBe("2026-11-19T00:00:00.000Z");
    expect(new Date(releaseInstant("Europe/Zagreb")).toISOString()).toBe("2026-11-18T23:00:00.000Z");
  });

  test("moves westwards with the sun", () => {
    expect(releaseInstant("Pacific/Auckland")).toBeLessThan(releaseInstant("Asia/Tokyo"));
    expect(releaseInstant("Asia/Tokyo")).toBeLessThan(releaseInstant("Europe/London"));
    expect(releaseInstant("Europe/London")).toBeLessThan(releaseInstant("America/Los_Angeles"));
  });
});

describe("breakdown", () => {
  test("splits the remaining time into units", () => {
    const now = target - (6 * DAY + 3 * HOUR + 12 * MINUTE + 8 * SECOND + 250);
    expect(breakdown(now, target)).toEqual({
      elapsed: false,
      total: 6 * DAY + 3 * HOUR + 12 * MINUTE + 8 * SECOND + 250,
      days: 6,
      hours: 3,
      minutes: 12,
      seconds: 8,
      milliseconds: 250,
    });
  });

  test("shows one second until the final millisecond has passed", () => {
    expect(breakdown(target - 1000, target).seconds).toBe(1);
    expect(breakdown(target - 999, target).seconds).toBe(0);
    expect(breakdown(target - 1, target).elapsed).toBe(false);
  });

  test("flips to elapsed exactly at the target and counts upwards", () => {
    expect(breakdown(target, target)).toMatchObject({ elapsed: true, total: 0, seconds: 0 });
    const later = breakdown(target + 2 * DAY + 90 * SECOND, target);
    expect(later).toMatchObject({ elapsed: true, days: 2, hours: 0, minutes: 1, seconds: 30 });
  });

  test("carries more than 99 days without wrapping", () => {
    expect(breakdown(target - 400 * DAY, target).days).toBe(400);
  });
});

describe("msUntilNextSecond", () => {
  test("returns the distance to the next change of the seconds digit", () => {
    // 5.250 s remain and "5" is showing; "4" appears once 4.999 s remain.
    expect(msUntilNextSecond(target - 5_250, target)).toBe(251);
    expect(msUntilNextSecond(target - 5_001, target)).toBe(2);
    expect(msUntilNextSecond(target - 5_000, target)).toBe(1);
    expect(msUntilNextSecond(target - 4_999, target)).toBe(1000);
  });

  test("lands exactly on the target for the final change", () => {
    expect(msUntilNextSecond(target - 999, target)).toBe(999);
    expect(msUntilNextSecond(target - 1, target)).toBe(1);
  });

  test("keeps ticking in phase after the target", () => {
    expect(msUntilNextSecond(target, target)).toBe(1000);
    expect(msUntilNextSecond(target + 250, target)).toBe(750);
    expect(msUntilNextSecond(target + 999, target)).toBe(1);
  });

  test("always schedules a tick between 1 ms and 1 s away", () => {
    for (let offset = -3000; offset <= 3000; offset += 37) {
      const wait = msUntilNextSecond(target + offset, target);
      expect(wait).toBeGreaterThanOrEqual(1);
      expect(wait).toBeLessThanOrEqual(1000);
    }
  });

  test("the scheduled tick lands on a new seconds value", () => {
    for (const offset of [-5_250, -5_000, -1_000, -999, -1, 0, 1, 400, 1_999]) {
      const now = target + offset;
      const wait = msUntilNextSecond(now, target);
      const before = breakdown(now, target);
      const after = breakdown(now + wait, target);
      expect(after.seconds !== before.seconds || after.elapsed !== before.elapsed).toBe(true);
      // One millisecond earlier the display must still be unchanged, so the
      // tick is neither late nor wasted.
      const justBefore = breakdown(now + wait - 1, target);
      expect(justBefore.seconds).toBe(before.seconds);
      expect(justBefore.elapsed).toBe(before.elapsed);
    }
  });
});

describe("totals", () => {
  test("expresses the remaining time in single units", () => {
    const zone = "UTC";
    const now = target - (6 * DAY + 3 * HOUR);
    const t = totals(now, target, zone);
    expect(t.hours).toBe(147);
    expect(t.minutes).toBe(147 * 60);
    expect(t.seconds).toBe(147 * 3600);
    expect(t.weeks).toBeCloseTo(0.875, 3);
    expect(t.sleeps).toBe(7);
    expect(t.weekends).toBe(1);
  });

  test("is all zeros once the target has passed", () => {
    expect(totals(target + HOUR, target, "UTC")).toEqual({
      weeks: 0,
      hours: 0,
      minutes: 0,
      seconds: 0,
      sleeps: 0,
      weekends: 0,
    });
  });
});

describe("wait progress", () => {
  test("starts at the first trailer and ends at release", () => {
    expect(WAIT_ORIGIN).toBe(milestoneInstant(MILESTONES[0]!.date));
    expect(waitProgress(WAIT_ORIGIN, target)).toBe(0);
    expect(waitProgress(target, target)).toBe(1);
    expect(waitProgress((WAIT_ORIGIN + target) / 2, target)).toBeCloseTo(0.5, 9);
  });

  test("is clamped outside the interval", () => {
    expect(waitProgress(WAIT_ORIGIN - DAY, target)).toBe(0);
    expect(waitProgress(target + DAY, target)).toBe(1);
  });

  test("places milestones in chronological order", () => {
    const positions = MILESTONES.map((m) => timelinePosition(milestoneInstant(m.date), target));
    expect(positions[0]).toBe(0);
    for (let i = 1; i < positions.length; i++) {
      expect(positions[i]!).toBeGreaterThan(positions[i - 1]!);
      expect(positions[i]!).toBeLessThan(1);
    }
  });
});

describe("formatCompact", () => {
  test("pads every unit except days", () => {
    const now = target - (6 * DAY + 3 * HOUR + 2 * MINUTE + 8 * SECOND);
    expect(formatCompact(breakdown(now, target))).toBe("6d 03h 02m 08s");
  });
});
