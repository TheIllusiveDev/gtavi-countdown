/**
 * The games the countdown can count down to. This is the single place to
 * edit if a date moves; the Rust app keeps the same list in
 * `rust/crates/core/src/games.rs`.
 *
 * Each game unlocks at a wall-clock time (midnight unless the publisher says
 * otherwise), so the absolute instant depends on the time zone the countdown
 * is evaluated in. The dates are the ones the publishers announced; the
 * unlock hours are as reported and may change.
 */

export interface Milestone {
  /** Calendar date (UTC) of the event, as `YYYY-MM-DD`. */
  date: string;
  title: string;
  detail: string;
}

export interface Game {
  /** Stable identifier used in the settings: "gta-vi", "fable" or "custom". */
  id: string;
  title: string;
  /** Used in the header and the tab title: "GTA VI". */
  shortTitle: string;
  /** Release date as `YYYY-MM-DD`. */
  date: string;
  hour: number;
  minute: number;
  platforms: readonly string[];
  /**
   * Oldest first. The first is the origin of the progress bar; the release
   * itself is added by the timeline because its instant depends on the zone.
   */
  milestones: readonly Milestone[];
}

/** A game the user entered in the settings. */
export interface CustomGame {
  title: string;
  /** `YYYY-MM-DD`. */
  date: string;
  /** `HH:MM`, 24-hour. */
  time: string;
  /** `YYYY-MM-DD`: the day it was added, the origin of its progress bar. */
  added: string;
}

export const DEFAULT_GAME = "gta-vi";
export const CUSTOM = "custom";
export const MAX_TITLE_LENGTH = 60;

/** The built-in games, in release order. */
export const GAMES: readonly Game[] = [
  {
    id: "gta-vi",
    title: "Grand Theft Auto VI",
    shortTitle: "GTA VI",
    date: "2026-11-19",
    hour: 0,
    minute: 0,
    platforms: ["PlayStation 5", "Xbox Series X|S"],
    milestones: [
      { date: "2023-12-04", title: "Trailer 1", detail: "First trailer published, with a 2025 release window." },
      {
        date: "2025-05-02",
        title: "First delay",
        detail: "Release moved to May 26, 2026. Trailer 2 followed on May 6.",
      },
      { date: "2025-11-06", title: "Second delay", detail: "Release moved to November 19, 2026." },
    ],
  },
  {
    id: "metro-2039",
    title: "Metro 2039",
    shortTitle: "Metro 2039",
    date: "2027-02-04",
    hour: 0,
    minute: 0,
    platforms: ["PlayStation 5", "Xbox Series X|S", "PC"],
    milestones: [
      {
        date: "2026-06-07",
        title: "Release window",
        detail: "February 2027 window announced at the Xbox Games Showcase.",
      },
      { date: "2026-09-03", title: "Date announced", detail: "February 4, 2027 confirmed in a State of Play." },
    ],
  },
  {
    id: "god-of-war-laufey",
    title: "God of War Laufey",
    shortTitle: "God of War",
    date: "2027-02-16",
    hour: 0,
    minute: 0,
    platforms: ["PlayStation 5"],
    milestones: [
      {
        date: "2026-07-24",
        title: "Date announced",
        detail: "Release date set for February 16, 2027 at San Diego Comic-Con.",
      },
    ],
  },
  {
    id: "fable",
    title: "Fable",
    shortTitle: "Fable",
    date: "2027-02-23",
    hour: 0,
    minute: 0,
    platforms: ["Xbox Series X|S", "PlayStation 5", "PC"],
    milestones: [
      { date: "2020-07-23", title: "Reveal", detail: "Announced at the Xbox Games Showcase." },
      {
        date: "2026-06-07",
        title: "Date announced",
        detail: "Release set for February 23, 2027 at the Xbox Games Showcase.",
      },
    ],
  },
  {
    id: "final-fantasy-vii-revelation",
    title: "Final Fantasy VII Revelation",
    shortTitle: "FFVII Revelation",
    date: "2027-04-08",
    hour: 0,
    minute: 0,
    platforms: ["PlayStation 5", "Xbox Series X|S", "Nintendo Switch 2", "PC"],
    milestones: [
      { date: "2026-06-06", title: "Reveal", detail: "Announced with a spring 2027 release window." },
      { date: "2026-09-03", title: "Date announced", detail: "April 8, 2027 confirmed in a State of Play." },
    ],
  },
];

/** Parses `YYYY-MM-DD`, accepting only real dates from 1970 to 9999. */
export function parseDate(text: string): { year: number; month: number; day: number } | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(text);
  if (!match) return null;
  const [year, month, day] = [Number(match[1]), Number(match[2]), Number(match[3])];
  if (year < 1970 || month < 1 || month > 12 || day < 1) return null;
  const daysInMonth = new Date(Date.UTC(year, month, 0)).getUTCDate();
  return day <= daysInMonth ? { year, month, day } : null;
}

/** Parses `HH:MM` in 24-hour form. */
export function parseTime(text: string): { hour: number; minute: number } | null {
  const match = /^(\d{2}):(\d{2})$/.exec(text);
  if (!match) return null;
  const [hour, minute] = [Number(match[1]), Number(match[2])];
  return hour < 24 && minute < 60 ? { hour, minute } : null;
}

/** Noon UTC on a `YYYY-MM-DD` date, where milestones are placed so that they
 * fall on the same calendar day in every zone. */
export function noonUtc(date: string): number {
  return Date.parse(`${date}T12:00:00Z`);
}

/** Today's date in UTC as `YYYY-MM-DD`. */
export function today(now = Date.now()): string {
  return new Date(now).toISOString().slice(0, 10);
}

/** Checks a custom game and returns it with its title trimmed, or `null`. */
export function validateCustomGame(raw: unknown): CustomGame | null {
  if (typeof raw !== "object" || raw === null) return null;
  const r = raw as Record<string, unknown>;
  if (typeof r.title !== "string" || typeof r.date !== "string") return null;
  if (typeof r.time !== "string" || typeof r.added !== "string") return null;
  const title = r.title.trim();
  // Counted in code points, as the Rust app counts characters.
  const length = [...title].length;
  if (length < 1 || length > MAX_TITLE_LENGTH || /[\u0000-\u001f\u007f-\u009f]/.test(title)) return null;
  if (!parseDate(r.date) || !parseTime(r.time) || !parseDate(r.added)) return null;
  return { title, date: r.date, time: r.time, added: r.added };
}

export function customToGame(custom: CustomGame): Game {
  const time = parseTime(custom.time) ?? { hour: 0, minute: 0 };
  return {
    id: CUSTOM,
    title: custom.title,
    shortTitle: custom.title,
    date: custom.date,
    hour: time.hour,
    minute: time.minute,
    platforms: [],
    milestones: [{ date: custom.added, title: "Added", detail: "Added to the countdown." }],
  };
}

/** The game the settings select, falling back to the default. */
export function resolveGame(id: string, custom: CustomGame | null): Game {
  if (id === CUSTOM && custom) return customToGame(custom);
  return GAMES.find((game) => game.id === id) ?? GAMES[0]!;
}

/** File-name friendly form of the game, for the calendar export. */
export function gameSlug(game: Game): string {
  if (game.id !== CUSTOM) return game.id;
  const slug = game.title
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 40);
  return slug || "game";
}

/** The caption over the timeline: "The wait since Trailer 1". */
export function waitCaption(game: Game): string {
  return `The wait since ${game.milestones[0]?.title ?? "announcement"}`;
}
