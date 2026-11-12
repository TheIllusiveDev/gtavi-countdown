/**
 * Minimal iCalendar (RFC 5545) writer for a single event with reminders.
 */

export interface CalendarEvent {
  uid: string;
  /** Event start as a UTC instant in Unix milliseconds. */
  start: number;
  durationMinutes: number;
  summary: string;
  description: string;
  /** Minutes before the start at which to raise a reminder. */
  reminders: number[];
  /** Creation stamp; defaults to the current time. */
  stamp?: number;
}

/** `2026-11-18T23:00:00.000Z` becomes `20261118T230000Z`. */
export function icsDate(utcMs: number): string {
  return new Date(utcMs).toISOString().replace(/[-:]/g, "").replace(/\.\d{3}Z$/, "Z");
}

/** Escapes TEXT values as required by RFC 5545 section 3.3.11. */
export function icsEscape(text: string): string {
  return text
    .replace(/\\/g, "\\\\")
    .replace(/;/g, "\\;")
    .replace(/,/g, "\\,")
    .replace(/\r?\n/g, "\\n");
}

/** Folds a content line so that no physical line exceeds 75 octets. */
export function icsFold(line: string): string {
  const encoder = new TextEncoder();
  if (encoder.encode(line).length <= 75) return line;

  const parts: string[] = [];
  let current = "";
  let currentBytes = 0;
  // The first line may hold 75 octets; continuation lines start with a space,
  // which leaves 74 octets for content.
  let limit = 75;
  for (const char of line) {
    const size = encoder.encode(char).length;
    if (currentBytes + size > limit) {
      parts.push(current);
      current = "";
      currentBytes = 0;
      limit = 74;
    }
    current += char;
    currentBytes += size;
  }
  parts.push(current);
  return parts.join("\r\n ");
}

export function buildCalendar(event: CalendarEvent): string {
  const lines = [
    "BEGIN:VCALENDAR",
    "VERSION:2.0",
    "PRODID:-//gtavi-countdown//Release Countdown//EN",
    "CALSCALE:GREGORIAN",
    "METHOD:PUBLISH",
    "BEGIN:VEVENT",
    `UID:${event.uid}`,
    `DTSTAMP:${icsDate(event.stamp ?? Date.now())}`,
    `DTSTART:${icsDate(event.start)}`,
    `DTEND:${icsDate(event.start + event.durationMinutes * 60_000)}`,
    `SUMMARY:${icsEscape(event.summary)}`,
    `DESCRIPTION:${icsEscape(event.description)}`,
    "TRANSP:TRANSPARENT",
  ];
  for (const minutes of event.reminders) {
    lines.push(
      "BEGIN:VALARM",
      "ACTION:DISPLAY",
      `DESCRIPTION:${icsEscape(event.summary)}`,
      `TRIGGER:-PT${minutes}M`,
      "END:VALARM",
    );
  }
  lines.push("END:VEVENT", "END:VCALENDAR");
  return lines.map(icsFold).join("\r\n") + "\r\n";
}
