import { DEFAULTS, STORAGE_KEY, THEMES, sanitizeSettings, type SettingsData } from "./settings";
import { systemTimeZone } from "./time/zones";

export { EFFECT_LEVELS, THEMES, sanitizeSettings } from "./settings";
export type { EffectLevel, SettingsData, Theme } from "./settings";

function load(): SettingsData {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return sanitizeSettings(stored ? JSON.parse(stored) : null);
  } catch {
    // Storage can be unavailable (private windows, blocked site data).
    return { ...DEFAULTS };
  }
}

class Settings {
  data = $state<SettingsData>(load());

  /** The zone actually in effect, resolving the "follow system" choice. */
  readonly zone = $derived(this.data.timeZone ?? systemTimeZone());

  /** Writes the current settings to storage. Safe to call on every change. */
  persist(): void {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this.data));
    } catch {
      // Persistence is a convenience; the app works without it.
    }
  }

  cycleTheme(): void {
    const next = (THEMES.indexOf(this.data.theme) + 1) % THEMES.length;
    this.data.theme = THEMES[next]!;
  }

  reset(): void {
    this.data = { ...DEFAULTS };
  }
}

export const settings = new Settings();
