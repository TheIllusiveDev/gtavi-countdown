/**
 * Time zone arithmetic built on `Intl.DateTimeFormat`.
 *
 * JavaScript has no built-in way to ask "which instant is midnight in
 * Pacific/Auckland on a given date", so these helpers derive UTC offsets from
 * the formatter and invert them. Formatters are expensive to construct and are
 * cached per zone.
 */

const formatters = new Map<string, Intl.DateTimeFormat>();

function formatterFor(timeZone: string): Intl.DateTimeFormat {
  let formatter = formatters.get(timeZone);
  if (!formatter) {
    formatter = new Intl.DateTimeFormat("en-US", {
      timeZone,
      hourCycle: "h23",
      year: "numeric",
      month: "numeric",
      day: "numeric",
      hour: "numeric",
      minute: "numeric",
      second: "numeric",
    });
    formatters.set(timeZone, formatter);
  }
  return formatter;
}

export interface WallTime {
  year: number;
  /** 1-based month. */
  month: number;
  day: number;
  hour: number;
  minute: number;
  second: number;
}

/** Returns the wall-clock reading in `timeZone` at the instant `utcMs`. */
export function wallTimeAt(utcMs: number, timeZone: string): WallTime {
  const wall: WallTime = { year: 0, month: 0, day: 0, hour: 0, minute: 0, second: 0 };
  for (const part of formatterFor(timeZone).formatToParts(utcMs)) {
    if (part.type in wall) {
      wall[part.type as keyof WallTime] = Number(part.value);
    }
  }
  return wall;
}

/**
 * Offset of `timeZone` from UTC at the instant `utcMs`, in milliseconds.
 * Positive values are east of Greenwich (for example +3 600 000 for CET).
 */
export function offsetAt(utcMs: number, timeZone: string): number {
  const w = wallTimeAt(utcMs, timeZone);
  const asUtc = Date.UTC(w.year, w.month - 1, w.day, w.hour, w.minute, w.second);
  // Sub-second precision is dropped by the formatter; compare whole seconds.
  return asUtc - Math.floor(utcMs / 1000) * 1000;
}

/**
 * Converts a wall-clock time in `timeZone` to the UTC instant it denotes.
 *
 * The offset is resolved in two passes so that the result is correct when the
 * first guess lands on the other side of a daylight-saving transition.
 */
export function zonedTimeToUtc(
  year: number,
  month: number,
  day: number,
  hour: number,
  minute: number,
  timeZone: string,
): number {
  const wall = Date.UTC(year, month - 1, day, hour, minute);
  const firstGuess = wall - offsetAt(wall, timeZone);
  return wall - offsetAt(firstGuess, timeZone);
}

/** Formats an offset in milliseconds as `UTC+05:30`, `UTC-08:00` or `UTC`. */
export function formatOffset(offsetMs: number): string {
  if (offsetMs === 0) return "UTC";
  const sign = offsetMs < 0 ? "-" : "+";
  const totalMinutes = Math.round(Math.abs(offsetMs) / 60_000);
  const hours = String(Math.floor(totalMinutes / 60)).padStart(2, "0");
  const minutes = String(totalMinutes % 60).padStart(2, "0");
  return `UTC${sign}${hours}:${minutes}`;
}

/** The time zone the host system is configured for. */
export function systemTimeZone(): string {
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";
  } catch {
    return "UTC";
  }
}

/** True when the runtime recognises `timeZone` as an IANA zone identifier. */
export function isValidTimeZone(timeZone: string): boolean {
  if (!timeZone) return false;
  try {
    new Intl.DateTimeFormat("en-US", { timeZone });
    return true;
  } catch {
    return false;
  }
}

/**
 * Every geographic IANA zone the runtime knows, plus UTC, falling back to a
 * short curated list on runtimes without `Intl.supportedValuesOf`.
 */
export function listTimeZones(): string[] {
  const supported = (
    Intl as typeof Intl & { supportedValuesOf?: (key: string) => string[] }
  ).supportedValuesOf;
  if (typeof supported === "function") {
    try {
      // Keep geographic zones only. Some runtimes also list the "Etc/GMT+N"
      // family, whose signs are inverted by POSIX convention ("Etc/GMT-14" is
      // UTC+14), and bare legacy abbreviations such as "EST"; both would
      // confuse more than they help in a picker.
      const zones = supported("timeZone").filter(
        (zone) => zone.includes("/") && !zone.startsWith("Etc/"),
      );
      return ["UTC", ...zones];
    } catch {
      // Fall through to the curated list.
    }
  }
  return [
    "UTC",
    "Pacific/Auckland",
    "Australia/Sydney",
    "Asia/Tokyo",
    "Asia/Seoul",
    "Asia/Shanghai",
    "Asia/Kolkata",
    "Asia/Dubai",
    "Europe/Moscow",
    "Europe/Berlin",
    "Europe/London",
    "America/Sao_Paulo",
    "America/New_York",
    "America/Chicago",
    "America/Denver",
    "America/Los_Angeles",
    "Pacific/Honolulu",
  ];
}

/** `America/Argentina/Buenos_Aires` becomes `Buenos Aires`. */
export function zoneCity(timeZone: string): string {
  const leaf = timeZone.slice(timeZone.lastIndexOf("/") + 1);
  return leaf.replaceAll("_", " ");
}

/** `America/Argentina/Buenos_Aires` becomes `America / Argentina`. */
export function zoneRegion(timeZone: string): string {
  const cut = timeZone.lastIndexOf("/");
  return cut === -1 ? "" : timeZone.slice(0, cut).replaceAll("_", " ").replaceAll("/", " / ");
}

/**
 * Whole calendar days from the date of `fromMs` to the date of `toMs`, both
 * read in `timeZone`. This is the number of midnights in between: "sleeps".
 */
export function calendarDaysBetween(fromMs: number, toMs: number, timeZone: string): number {
  const a = wallTimeAt(fromMs, timeZone);
  const b = wallTimeAt(toMs, timeZone);
  const dayA = Date.UTC(a.year, a.month - 1, a.day);
  const dayB = Date.UTC(b.year, b.month - 1, b.day);
  return Math.round((dayB - dayA) / 86_400_000);
}

/**
 * Number of Saturdays strictly after the date of `fromMs` and strictly before
 * the date of `toMs` in `timeZone`: the full weekends still to get through.
 */
export function weekendsBetween(fromMs: number, toMs: number, timeZone: string): number {
  const days = calendarDaysBetween(fromMs, toMs, timeZone);
  if (days <= 1) return 0;
  const a = wallTimeAt(fromMs, timeZone);
  // Day of week of the starting date: 0 is Sunday, 6 is Saturday.
  const startDow = new Date(Date.UTC(a.year, a.month - 1, a.day)).getUTCDay();
  // Days until the first Saturday after the starting date (1 to 7).
  const untilSaturday = ((6 - startDow + 7) % 7) || 7;
  if (untilSaturday >= days) return 0;
  return 1 + Math.floor((days - 1 - untilSaturday) / 7);
}
