import { CUSTOM, DEFAULT_GAME, GAMES, validateCustomGame, type CustomGame } from "./time/games";
import { isValidTimeZone } from "./time/zones";

/*
 * The settings schema and its validation. This module is free of React so
 * that it can be unit-tested on its own; the hook that persists these values
 * lives in useSettings.ts. The Rust app stores the same JSON object in its
 * settings file.
 */

export const THEMES = ["dusk", "midnight", "dawn"] as const;
export type Theme = (typeof THEMES)[number];

export const EFFECT_LEVELS = ["auto", "high", "low", "off"] as const;
export type EffectLevel = (typeof EFFECT_LEVELS)[number];

export interface SettingsData {
  /** IANA zone the release is evaluated in; `null` follows the system. */
  timeZone: string | null;
  /** A built-in game's id, or "custom" for `customGame`. */
  game: string;
  customGame: CustomGame | null;
  showMilliseconds: boolean;
  theme: Theme;
  /** Rendering budget for the animated backdrop. */
  effects: EffectLevel;
  /** Send a notification at the moment of release. */
  notify: boolean;
  /** Used by the Rust desktop app only: correct the clock over NTP. */
  networkTime: boolean;
  /** Used by the Rust desktop app only: keep the window above others. */
  alwaysOnTop: boolean;
}

export const STORAGE_KEY = "gtavi-countdown:settings:v1";

export const DEFAULTS: SettingsData = {
  timeZone: null,
  game: DEFAULT_GAME,
  customGame: null,
  showMilliseconds: false,
  theme: "dusk",
  effects: "auto",
  notify: false,
  networkTime: true,
  alwaysOnTop: false,
};

/** Merges untrusted stored JSON over the defaults, discarding invalid fields. */
export function sanitizeSettings(raw: unknown): SettingsData {
  const out: SettingsData = { ...DEFAULTS };
  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) return out;
  const r = raw as Record<string, unknown>;

  if (typeof r.timeZone === "string" && isValidTimeZone(r.timeZone)) out.timeZone = r.timeZone;
  out.customGame = validateCustomGame(r.customGame);
  if (typeof r.game === "string") {
    const known = GAMES.some((game) => game.id === r.game);
    if (known || (r.game === CUSTOM && out.customGame)) out.game = r.game;
  }
  if (typeof r.showMilliseconds === "boolean") out.showMilliseconds = r.showMilliseconds;
  if (THEMES.includes(r.theme as Theme)) out.theme = r.theme as Theme;
  if (EFFECT_LEVELS.includes(r.effects as EffectLevel)) out.effects = r.effects as EffectLevel;
  if (typeof r.notify === "boolean") out.notify = r.notify;
  if (typeof r.networkTime === "boolean") out.networkTime = r.networkTime;
  if (typeof r.alwaysOnTop === "boolean") out.alwaysOnTop = r.alwaysOnTop;
  return out;
}

/** The stored form, with keys in a fixed order that the Rust app also writes. */
export function serializeSettings(data: SettingsData): string {
  const custom = data.customGame;
  return JSON.stringify({
    timeZone: data.timeZone,
    game: data.game,
    customGame: custom ? { title: custom.title, date: custom.date, time: custom.time, added: custom.added } : null,
    showMilliseconds: data.showMilliseconds,
    theme: data.theme,
    effects: data.effects,
    notify: data.notify,
    networkTime: data.networkTime,
    alwaysOnTop: data.alwaysOnTop,
  });
}

export function nextTheme(theme: Theme): Theme {
  return THEMES[(THEMES.indexOf(theme) + 1) % THEMES.length]!;
}
