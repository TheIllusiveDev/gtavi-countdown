import { describe, expect, test } from "bun:test";
import { buildCalendar, icsDate, icsEscape, icsFold } from "../src/lib/ics";

describe("icsDate", () => {
  test("formats an instant as a UTC date-time", () => {
    expect(icsDate(Date.UTC(2026, 10, 18, 23, 0, 0))).toBe("20261118T230000Z");
    expect(icsDate(Date.UTC(2026, 0, 2, 3, 4, 5, 678))).toBe("20260102T030405Z");
  });
});

describe("icsEscape", () => {
  test("escapes the characters RFC 5545 reserves in text", () => {
    expect(icsEscape("PS5, Xbox; both")).toBe("PS5\\, Xbox\\; both");
    expect(icsEscape("a\\b")).toBe("a\\\\b");
    expect(icsEscape("line one\nline two\r\nline three")).toBe("line one\\nline two\\nline three");
  });
});

describe("icsFold", () => {
  const bytes = (text: string) => new TextEncoder().encode(text).length;

  test("leaves short lines alone", () => {
    expect(icsFold("SUMMARY:Short")).toBe("SUMMARY:Short");
    expect(icsFold("X".repeat(75))).toBe("X".repeat(75));
  });

  test("folds long lines at 75 octets with a leading space", () => {
    const folded = icsFold("DESCRIPTION:" + "a".repeat(200));
    const lines = folded.split("\r\n");
    expect(lines.length).toBeGreaterThan(1);
    for (const line of lines) expect(bytes(line)).toBeLessThanOrEqual(75);
    for (const line of lines.slice(1)) expect(line.startsWith(" ")).toBe(true);
    // Unfolding restores the original content line.
    expect(lines.map((line, i) => (i === 0 ? line : line.slice(1))).join("")).toBe(
      "DESCRIPTION:" + "a".repeat(200),
    );
  });

  test("never splits a multi-byte character", () => {
    const original = "SUMMARY:" + "é中\u{1F3AE}".repeat(30);
    const lines = icsFold(original).split("\r\n");
    for (const line of lines) expect(bytes(line)).toBeLessThanOrEqual(75);
    expect(lines.map((line, i) => (i === 0 ? line : line.slice(1))).join("")).toBe(original);
  });
});

describe("buildCalendar", () => {
  const calendar = buildCalendar({
    uid: "release-1@gtavi-countdown",
    start: Date.UTC(2026, 10, 18, 23, 0, 0),
    durationMinutes: 60,
    summary: "Grand Theft Auto VI release",
    description: "Platforms: PlayStation 5, Xbox Series X|S.",
    reminders: [1440, 60],
    stamp: Date.UTC(2026, 10, 12, 19, 0, 0),
  });
  const lines = calendar.split("\r\n");

  test("produces a well-formed calendar object", () => {
    expect(lines[0]).toBe("BEGIN:VCALENDAR");
    expect(lines.at(-2)).toBe("END:VCALENDAR");
    expect(lines.at(-1)).toBe("");
    expect(calendar).not.toMatch(/[^\r]\n/);
    expect(lines).toContain("VERSION:2.0");
    expect(lines).toContain("UID:release-1@gtavi-countdown");
    expect(lines).toContain("DTSTAMP:20261112T190000Z");
    expect(lines).toContain("DTSTART:20261118T230000Z");
    expect(lines).toContain("DTEND:20261119T000000Z");
    expect(lines).toContain("SUMMARY:Grand Theft Auto VI release");
    expect(lines).toContain("DESCRIPTION:Platforms: PlayStation 5\\, Xbox Series X|S.");
  });

  test("adds one alarm per reminder", () => {
    expect(lines.filter((line) => line === "BEGIN:VALARM")).toHaveLength(2);
    expect(lines.filter((line) => line === "END:VALARM")).toHaveLength(2);
    expect(lines).toContain("TRIGGER:-PT1440M");
    expect(lines).toContain("TRIGGER:-PT60M");
  });

  test("balances every BEGIN with an END", () => {
    const begins = lines.filter((line) => line.startsWith("BEGIN:")).length;
    const ends = lines.filter((line) => line.startsWith("END:")).length;
    expect(begins).toBe(ends);
  });
});
