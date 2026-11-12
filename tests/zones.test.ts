import { describe, expect, test } from "bun:test";
import {
  calendarDaysBetween,
  formatOffset,
  isValidTimeZone,
  listTimeZones,
  offsetAt,
  wallTimeAt,
  weekendsBetween,
  zoneCity,
  zoneRegion,
  zonedTimeToUtc,
} from "../src/lib/time/zones";

const HOUR = 3_600_000;

describe("offsetAt", () => {
  test("returns zero for UTC", () => {
    expect(offsetAt(Date.UTC(2026, 10, 19), "UTC")).toBe(0);
  });

  test("follows daylight saving in Europe", () => {
    expect(offsetAt(Date.UTC(2026, 10, 19), "Europe/Zagreb")).toBe(1 * HOUR);
    expect(offsetAt(Date.UTC(2026, 6, 1), "Europe/Zagreb")).toBe(2 * HOUR);
  });

  test("handles zones west of Greenwich and fractional offsets", () => {
    expect(offsetAt(Date.UTC(2026, 10, 19), "America/New_York")).toBe(-5 * HOUR);
    expect(offsetAt(Date.UTC(2026, 10, 19), "Asia/Kolkata")).toBe(5.5 * HOUR);
    expect(offsetAt(Date.UTC(2026, 10, 19), "Pacific/Chatham")).toBe(13.75 * HOUR);
  });

  test("ignores the sub-second part of the instant", () => {
    expect(offsetAt(Date.UTC(2026, 10, 19, 0, 0, 0, 999), "Asia/Tokyo")).toBe(9 * HOUR);
  });
});

describe("zonedTimeToUtc", () => {
  test("resolves release midnight around the world", () => {
    const midnight = (zone: string) => new Date(zonedTimeToUtc(2026, 11, 19, 0, 0, zone)).toISOString();
    expect(midnight("UTC")).toBe("2026-11-19T00:00:00.000Z");
    expect(midnight("Europe/London")).toBe("2026-11-19T00:00:00.000Z");
    expect(midnight("Europe/Zagreb")).toBe("2026-11-18T23:00:00.000Z");
    expect(midnight("Pacific/Auckland")).toBe("2026-11-18T11:00:00.000Z");
    expect(midnight("Asia/Tokyo")).toBe("2026-11-18T15:00:00.000Z");
    expect(midnight("Asia/Kolkata")).toBe("2026-11-18T18:30:00.000Z");
    expect(midnight("America/New_York")).toBe("2026-11-19T05:00:00.000Z");
    expect(midnight("America/Los_Angeles")).toBe("2026-11-19T08:00:00.000Z");
  });

  test("is correct on both sides of a daylight-saving change", () => {
    // Europe leaves summer time on 25 October 2026 at 01:00 UTC.
    expect(new Date(zonedTimeToUtc(2026, 10, 25, 0, 30, "Europe/Berlin")).toISOString()).toBe(
      "2026-10-24T22:30:00.000Z",
    );
    expect(new Date(zonedTimeToUtc(2026, 10, 25, 12, 0, "Europe/Berlin")).toISOString()).toBe(
      "2026-10-25T11:00:00.000Z",
    );
  });

  test("round-trips through wallTimeAt", () => {
    for (const zone of ["Australia/Sydney", "America/Sao_Paulo", "Asia/Kathmandu", "Pacific/Honolulu"]) {
      const instant = zonedTimeToUtc(2026, 11, 19, 0, 0, zone);
      expect(wallTimeAt(instant, zone)).toEqual({
        year: 2026,
        month: 11,
        day: 19,
        hour: 0,
        minute: 0,
        second: 0,
      });
    }
  });
});

describe("formatOffset", () => {
  test("formats positive, negative, fractional and zero offsets", () => {
    expect(formatOffset(0)).toBe("UTC");
    expect(formatOffset(1 * HOUR)).toBe("UTC+01:00");
    expect(formatOffset(-8 * HOUR)).toBe("UTC-08:00");
    expect(formatOffset(5.5 * HOUR)).toBe("UTC+05:30");
    expect(formatOffset(-3.5 * HOUR)).toBe("UTC-03:30");
    expect(formatOffset(14 * HOUR)).toBe("UTC+14:00");
  });
});

describe("zone names", () => {
  test("validates identifiers", () => {
    expect(isValidTimeZone("Europe/Zagreb")).toBe(true);
    expect(isValidTimeZone("Mars/Olympus_Mons")).toBe(false);
    expect(isValidTimeZone("")).toBe(false);
  });

  test("lists zones including UTC", () => {
    const zones = listTimeZones();
    expect(zones.length).toBeGreaterThan(10);
    expect(zones).toContain("UTC");
    expect(zones.every(isValidTimeZone)).toBe(true);
  });

  test("lists geographic zones only, without duplicates", () => {
    const zones = listTimeZones();
    expect(zones.filter((zone) => zone.startsWith("Etc/"))).toEqual([]);
    expect(zones.filter((zone) => zone !== "UTC" && !zone.includes("/"))).toEqual([]);
    expect(new Set(zones).size).toBe(zones.length);
    expect(zones).toContain("Pacific/Auckland");
    expect(zones).toContain("America/Los_Angeles");
  });

  test("splits identifiers into city and region", () => {
    expect(zoneCity("America/Argentina/Buenos_Aires")).toBe("Buenos Aires");
    expect(zoneRegion("America/Argentina/Buenos_Aires")).toBe("America / Argentina");
    expect(zoneCity("UTC")).toBe("UTC");
    expect(zoneRegion("UTC")).toBe("");
  });
});

describe("calendar arithmetic", () => {
  const zone = "Europe/Zagreb";
  const release = zonedTimeToUtc(2026, 11, 19, 0, 0, zone);

  test("counts sleeps as midnights in the target zone", () => {
    // Thursday evening, a week before release: seven nights to go.
    expect(calendarDaysBetween(zonedTimeToUtc(2026, 11, 12, 20, 30, zone), release, zone)).toBe(7);
    // One minute past midnight on the 18th: still one night.
    expect(calendarDaysBetween(zonedTimeToUtc(2026, 11, 18, 0, 1, zone), release, zone)).toBe(1);
    expect(calendarDaysBetween(zonedTimeToUtc(2026, 11, 18, 23, 59, zone), release, zone)).toBe(1);
  });

  test("counts across a daylight-saving change without drifting", () => {
    const from = zonedTimeToUtc(2026, 10, 20, 23, 0, zone);
    expect(calendarDaysBetween(from, release, zone)).toBe(30);
  });

  test("counts the Saturdays strictly between two dates", () => {
    const at = (day: number, hour = 12) => zonedTimeToUtc(2026, 11, day, hour, 0, zone);
    // 12 November 2026 is a Thursday; the 14th is the only Saturday before the 19th.
    expect(weekendsBetween(at(12), release, zone)).toBe(1);
    expect(weekendsBetween(at(13), release, zone)).toBe(1);
    // On the Saturday itself the weekend has begun and no longer counts.
    expect(weekendsBetween(at(14), release, zone)).toBe(0);
    expect(weekendsBetween(at(18), release, zone)).toBe(0);
    // From Sunday 1 November: the 7th and the 14th.
    expect(weekendsBetween(at(1), release, zone)).toBe(2);
    // From Saturday 31 October: the 7th and the 14th.
    expect(weekendsBetween(zonedTimeToUtc(2026, 10, 31, 12, 0, zone), release, zone)).toBe(2);
  });
});
