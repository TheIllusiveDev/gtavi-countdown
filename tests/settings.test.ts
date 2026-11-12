import { describe, expect, test } from "bun:test";
import { DEFAULTS, sanitizeSettings, type SettingsData } from "../src/lib/settings";

describe("sanitizeSettings", () => {
  test("returns the defaults for missing or malformed input", () => {
    for (const input of [null, undefined, 42, "text", [], true]) {
      expect(sanitizeSettings(input)).toEqual(DEFAULTS);
    }
  });

  test("keeps valid fields", () => {
    const stored: SettingsData = {
      timeZone: "Pacific/Auckland",
      showMilliseconds: true,
      theme: "midnight",
      effects: "low",
      notify: true,
      networkTime: false,
      alwaysOnTop: true,
    };
    expect(sanitizeSettings(stored)).toEqual(stored);
  });

  test("replaces invalid fields with their defaults, one by one", () => {
    const result = sanitizeSettings({
      timeZone: "Atlantis/Lost_City",
      showMilliseconds: "yes",
      theme: "neon",
      effects: "ultra",
      notify: 1,
      networkTime: false,
      alwaysOnTop: null,
    });
    expect(result).toEqual({ ...DEFAULTS, networkTime: false });
  });

  test("ignores unknown keys and never returns the defaults object itself", () => {
    const result = sanitizeSettings({ theme: "dawn", injected: "<script>" });
    expect(result).toEqual({ ...DEFAULTS, theme: "dawn" });
    expect(result).not.toBe(DEFAULTS);
    expect("injected" in result).toBe(false);
  });

  test("treats a null time zone as following the device", () => {
    expect(sanitizeSettings({ timeZone: null }).timeZone).toBeNull();
  });
});
