import { isValidTimeZone } from "./time/zones";

/*
 * The settings schema and its validation. This module is free of reactivity
 * so that it can be unit-tested outside the Svelte compiler; the reactive
 * store that persists these values lives in settings.svelte.ts.
 */

export const THEMES = ["dusk", "midnight", "dawn"] as const;
export type Theme = (typeof THEMES)[number];

export const EFFECT_LEVELS = ["auto", "low", "off"] as const;
export type EffectLevel = (typeof EFFECT_LEVELS)[number];

export interface SettingsData {
  /** IANA zone the release midnight is evaluated in; `null` follows the system. */
  timeZone: string | null;
  showMilliseconds: boolean;
  theme: Theme;
  /** Rendering budget for the animated backdrop. */
  effects: EffectLevel;
  /** Send a notification at the moment of release. */
  notify: boolean;
  /** Desktop only: correct the system clock against a network time server. */
  networkTime: boolean;
  /** Desktop only: keep the window above other windows. */
  alwaysOnTop: boolean;
}

export const STORAGE_KEY = "gtavi-countdown:settings:v1";

export const DEFAULTS: SettingsData = {
  timeZone: null,
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
  if (typeof raw !== "object" || raw === null) return out;
  const r = raw as Record<string, unknown>;

  if (typeof r.timeZone === "string" && isValidTimeZone(r.timeZone)) out.timeZone = r.timeZone;
  if (typeof r.showMilliseconds === "boolean") out.showMilliseconds = r.showMilliseconds;
  if (THEMES.includes(r.theme as Theme)) out.theme = r.theme as Theme;
  if (EFFECT_LEVELS.includes(r.effects as EffectLevel)) out.effects = r.effects as EffectLevel;
  if (typeof r.notify === "boolean") out.notify = r.notify;
  if (typeof r.networkTime === "boolean") out.networkTime = r.networkTime;
  if (typeof r.alwaysOnTop === "boolean") out.alwaysOnTop = r.alwaysOnTop;
  return out;
}
