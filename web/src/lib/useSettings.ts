import { useCallback, useEffect, useState } from "react";
import { DEFAULTS, STORAGE_KEY, nextTheme, sanitizeSettings, serializeSettings, type SettingsData } from "./settings";

function load(): SettingsData {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return sanitizeSettings(stored ? JSON.parse(stored) : null);
  } catch {
    // Storage can be unavailable (private windows, blocked site data).
    return { ...DEFAULTS };
  }
}

/** The settings, persisted to local storage on every change. */
export function useSettings() {
  const [settings, setSettings] = useState(load);

  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, serializeSettings(settings));
    } catch {
      // Persistence is a convenience; the app works without it.
    }
  }, [settings]);

  const update = useCallback((patch: Partial<SettingsData>) => setSettings((s) => ({ ...s, ...patch })), []);
  const cycleTheme = useCallback(() => setSettings((s) => ({ ...s, theme: nextTheme(s.theme) })), []);
  const reset = useCallback(() => setSettings({ ...DEFAULTS }), []);
  return { settings, update, cycleTheme, reset };
}
