import { useId, useState, type FormEvent } from "react";
import {
  CUSTOM,
  GAMES,
  MAX_TITLE_LENGTH,
  noonUtc,
  parseDate,
  parseTime,
  today,
  validateCustomGame,
  type CustomGame,
} from "../lib/time/games";
import styles from "./GamePicker.module.css";

const dateFormat = new Intl.DateTimeFormat("en-US", {
  timeZone: "UTC",
  year: "numeric",
  month: "short",
  day: "numeric",
});

/**
 * The built-in games as a radio group, and a form for a game of one's own.
 * Selecting the custom entry before one has been entered moves focus to the
 * form instead.
 */
export function GamePicker({
  game,
  customGame,
  onSelect,
}: {
  game: string;
  customGame: CustomGame | null;
  onSelect: (game: string, customGame?: CustomGame) => void;
}) {
  const id = useId();
  const [title, setTitle] = useState(customGame?.title ?? "");
  const [date, setDate] = useState(customGame?.date ?? "");
  const [time, setTime] = useState(customGame?.time ?? "00:00");
  const [error, setError] = useState("");

  function submit(event: FormEvent): void {
    event.preventDefault();
    const trimmed = title.trim();
    if (!trimmed) return setError("Enter the game's title.");
    if ([...trimmed].length > MAX_TITLE_LENGTH) return setError("The title is longer than 60 characters.");
    if (!parseDate(date)) return setError("Enter a valid date.");
    if (!parseTime(time)) return setError("Enter the time as HH:MM, for example 00:00.");
    // An edited game keeps the day it was first added.
    const added = customGame?.added ?? today();
    const next = validateCustomGame({ title: trimmed, date, time, added: added < date ? added : date });
    if (!next) return setError("The title contains characters that cannot be used.");
    setError("");
    onSelect(CUSTOM, next);
  }

  return (
    <div className={styles.picker}>
      <div className={styles.games} role="radiogroup" aria-label="Game">
        {GAMES.map((entry) => (
          <button
            key={entry.id}
            type="button"
            role="radio"
            className={styles.game}
            aria-checked={game === entry.id}
            onClick={() => onSelect(entry.id)}
          >
            <span className={styles.title}>{entry.title}</span>
            <span className={styles.detail}>
              {dateFormat.format(noonUtc(entry.date))} · {entry.platforms.join(", ")}
            </span>
          </button>
        ))}
        <button
          type="button"
          role="radio"
          className={styles.game}
          aria-checked={game === CUSTOM}
          onClick={() => (customGame ? onSelect(CUSTOM) : document.getElementById(`${id}-title`)?.focus())}
        >
          <span className={styles.title}>Custom game</span>
          <span className={styles.detail}>
            {customGame
              ? `${customGame.title} · ${dateFormat.format(noonUtc(customGame.date))}`
              : "Any game, with your own date and hour"}
          </span>
        </button>
      </div>

      <form className={styles.form} onSubmit={submit} noValidate>
        <label className={styles.field}>
          <span className={styles.fieldLabel}>Title</span>
          <input
            id={`${id}-title`}
            className={styles.input}
            value={title}
            maxLength={MAX_TITLE_LENGTH * 2}
            onChange={(event) => setTitle(event.target.value)}
            autoComplete="off"
          />
        </label>
        <div className={styles.row}>
          <label className={styles.field}>
            <span className={styles.fieldLabel}>Date</span>
            <input
              className={styles.input}
              type="date"
              min="1970-01-01"
              max="9999-12-31"
              value={date}
              onChange={(event) => setDate(event.target.value)}
            />
          </label>
          <label className={styles.field}>
            <span className={styles.fieldLabel}>Unlock time</span>
            <input className={styles.input} type="time" value={time} onChange={(event) => setTime(event.target.value)} />
          </label>
        </div>
        {error && (
          <p className={styles.error} role="alert">
            {error}
          </p>
        )}
        <p className={styles.note}>The countdown runs to this date and time in the selected time zone.</p>
        <button type="submit" className={styles.submit}>
          Count down to it
        </button>
      </form>
    </div>
  );
}
