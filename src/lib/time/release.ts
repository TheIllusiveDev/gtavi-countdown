/**
 * Release facts. This is the single place to edit if the date ever moves.
 *
 * Rockstar Games has announced the calendar date. The unlock hour used here,
 * midnight local time in each region, is the timing reported by the press at
 * the time of writing; it is a wall-clock time, so the absolute instant
 * depends on the time zone the countdown is evaluated in.
 */
export const RELEASE = {
  title: "Grand Theft Auto VI",
  year: 2026,
  /** 1-based month. */
  month: 11,
  day: 19,
  hour: 0,
  minute: 0,
  platforms: ["PlayStation 5", "Xbox Series X|S"],
} as const;

export interface Milestone {
  /** Calendar date (UTC) of the event, as `YYYY-MM-DD`. */
  date: string;
  title: string;
  detail: string;
}

/**
 * Public milestones on the road to release, oldest first. The first entry is
 * the origin of the progress bar; the release itself is appended at runtime
 * because its instant depends on the selected time zone.
 */
export const MILESTONES: readonly Milestone[] = [
  {
    date: "2023-12-04",
    title: "Trailer 1",
    detail: "First trailer published, with a 2025 release window.",
  },
  {
    date: "2025-05-02",
    title: "First delay",
    detail: "Release moved to May 26, 2026. Trailer 2 followed on May 6.",
  },
  {
    date: "2025-11-06",
    title: "Second delay",
    detail: "Release moved to November 19, 2026.",
  },
];

/** Converts a `YYYY-MM-DD` milestone date to an instant at noon UTC. */
export function milestoneInstant(date: string): number {
  return Date.parse(`${date}T12:00:00Z`);
}
