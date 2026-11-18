import { describe, expect, test } from "bun:test";
import { DEFAULTS, sanitizeSettings, serializeSettings, type SettingsData } from "../src/lib/settings";

describe("sanitizeSettings", () => {
  test("returns the defaults for missing or malformed input", () => {
    for (const input of [null, undefined, 42, "text", [], true]) {
      expect(sanitizeSettings(input)).toEqual(DEFAULTS);
    }
  });

  test("keeps valid fields", () => {
    const stored: SettingsData = {
      timeZone: "Pacific/Auckland",
      game: "custom",
      customGame: { title: 'My "game"', date: "2027-05-01", time: "09:05", added: "2026-11-18" },
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

describe("games in settings", () => {
  test("an unknown or unusable game falls back to the default", () => {
    expect(sanitizeSettings({ game: "fable" }).game).toBe("fable");
    expect(sanitizeSettings({ game: "half-life-3" }).game).toBe(DEFAULTS.game);
    expect(sanitizeSettings({ game: "custom" }).game).toBe(DEFAULTS.game);
    const bad = { game: "custom", customGame: { title: "x", date: "2027-02-30", time: "00:00", added: "2026-11-18" } };
    expect(sanitizeSettings(bad)).toEqual(DEFAULTS);
  });

  test("custom titles are trimmed and checked", () => {
    const base = { date: "2027-02-01", time: "00:00", added: "2026-11-18" };
    expect(sanitizeSettings({ customGame: { ...base, title: "  Mine  " } }).customGame?.title).toBe("Mine");
    expect(sanitizeSettings({ customGame: { ...base, title: "   " } }).customGame).toBeNull();
    expect(sanitizeSettings({ customGame: { ...base, title: "x".repeat(61) } }).customGame).toBeNull();
    expect(sanitizeSettings({ customGame: { ...base, title: "Tab\there" } }).customGame).toBeNull();
    expect(sanitizeSettings({ customGame: { ...base, title: "x", time: "24:00" } }).customGame).toBeNull();
  });

  test("effects accept the high preset", () => {
    expect(sanitizeSettings({ effects: "high" }).effects).toBe("high");
  });

  test("serialises in the same form as the Rust app", () => {
    expect(serializeSettings(DEFAULTS)).toBe(
      '{"timeZone":null,"game":"gta-vi","customGame":null,"showMilliseconds":false,"theme":"dusk","effects":"auto","notify":false,"networkTime":true,"alwaysOnTop":false}',
    );
  });
});
