import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import { Backdrop } from "./components/Backdrop";
import { Countdown } from "./components/Countdown";
import { Icon } from "./components/Icon";
import { SettingsPanel } from "./components/SettingsPanel";
import { Stats } from "./components/Stats";
import { Timeline } from "./components/Timeline";
import { cx } from "./lib/cx";
import type { Composition } from "./lib/gl/scene";
import { buildCalendar } from "./lib/ics";
import {
  canFullscreen,
  canShare,
  copyText,
  notify,
  requestNotificationPermission,
  saveCalendar,
  shareText,
  toggleFullscreen,
} from "./lib/platform";
import { clock } from "./lib/time/clock";
import { breakdown, formatCompact, releaseInstant } from "./lib/time/countdown";
import { gameSlug, resolveGame } from "./lib/time/games";
import { formatOffset, offsetAt, systemTimeZone, zoneCity } from "./lib/time/zones";
import { useSettings } from "./lib/useSettings";
import styles from "./App.module.css";

const TOAST_MS = 3600;

export function App() {
  const { settings, update, cycleTheme, reset } = useSettings();
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [fullscreen, setFullscreen] = useState(false);
  const [toast, setToast] = useState("");
  const [composition, setComposition] = useState<Composition>({ horizon: 0.66, sunRadius: 0.16 });
  const toastTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const hero = useRef<HTMLDivElement>(null);
  const details = useRef<HTMLDivElement>(null);

  const game = useMemo(() => resolveGame(settings.game, settings.customGame), [settings.game, settings.customGame]);
  const zone = settings.timeZone ?? systemTimeZone();
  const target = useMemo(() => releaseInstant(game, zone), [game, zone]);
  const now = useSyncExternalStore(clock.subscribe, clock.getSnapshot);
  const parts = breakdown(now, target);

  // ---------------------------------------------------------------- wording

  const { targetDate, targetTime, zoneText, localEquivalent } = useMemo(() => {
    const format = (timeZone: string, options: Intl.DateTimeFormatOptions) =>
      new Intl.DateTimeFormat("en-US", { timeZone, ...options }).format(target);
    const system = systemTimeZone();
    return {
      targetDate: format(zone, { weekday: "long", month: "long", day: "numeric", year: "numeric" }),
      targetTime: format(zone, { hour: "2-digit", minute: "2-digit", hourCycle: "h23" }),
      zoneText: `${zoneCity(zone)} time, ${formatOffset(offsetAt(target, zone))}`,
      // When the chosen zone differs from the device's, the same instant is
      // also shown on the device's own clock, which is what the viewer reads.
      localEquivalent:
        offsetAt(target, system) === offsetAt(target, zone)
          ? ""
          : format(system, {
              weekday: "short",
              month: "short",
              day: "numeric",
              hour: "2-digit",
              minute: "2-digit",
              hourCycle: "h23",
            }),
    };
  }, [target, zone]);

  // --------------------------------------------------------------- effects

  useEffect(() => {
    const stop = clock.start(target);
    const onFullscreen = () => setFullscreen(document.fullscreenElement !== null);
    document.addEventListener("fullscreenchange", onFullscreen);
    return () => {
      stop();
      document.removeEventListener("fullscreenchange", onFullscreen);
    };
    // The clock starts once; target changes re-phase it below.
  }, []);

  // Re-phase the tick whenever the target instant moves.
  useEffect(() => clock.retarget(target), [target]);

  // The backdrop is composed around the content: the horizon sits just above
  // the lower panel and the sun fills the gap beneath the digits, whatever
  // the viewport. Positions are measured in document space, so scrolling a
  // tall page does not move the scene.
  useLayoutEffect(() => {
    const top = hero.current;
    const bottom = details.current;
    if (!top || !bottom) return;
    const measure = () => {
      const height = window.innerHeight || 1;
      const heroBottom = top.offsetTop + top.offsetHeight;
      // In the compact layout the lower panel is hidden; the horizon then
      // drops to the bottom of the window, clear of the timer.
      const hidden = bottom.offsetParent === null;
      const anchor = hidden ? Math.max(heroBottom + 44, height * 0.82) : bottom.offsetTop - 18;
      const horizon = Math.min(Math.max(anchor, height * 0.42), height * 0.9);
      // The sun is sunk 12% below the horizon, so 1.12 radii show above it.
      const room = (horizon - heroBottom - 6) / 1.12;
      const radius = Math.min(Math.max(room, 30), height * 0.2);
      const next = { horizon: horizon / height, sunRadius: radius / height };
      setComposition((current) =>
        Math.abs(next.horizon - current.horizon) > 0.001 || Math.abs(next.sunRadius - current.sunRadius) > 0.001
          ? next
          : current,
      );
    };
    const observer = new ResizeObserver(measure);
    observer.observe(top);
    observer.observe(bottom);
    window.addEventListener("resize", measure);
    measure();
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", measure);
    };
  }, []);

  useEffect(() => {
    document.documentElement.dataset.theme = settings.theme;
    const base = getComputedStyle(document.documentElement).getPropertyValue("--base").trim();
    document.querySelector('meta[name="theme-color"]')?.setAttribute("content", base);
  }, [settings.theme]);

  // The tab title follows the countdown at minute resolution.
  const pad = (n: number) => String(n).padStart(2, "0");
  const title = parts.elapsed
    ? `${game.shortTitle} is out`
    : `${parts.days}d ${pad(parts.hours)}h ${pad(parts.minutes)}m | ${game.shortTitle} Countdown`;
  useEffect(() => {
    document.title = title;
  }, [title]);

  const showToast = useCallback((message: string) => {
    setToast(message);
    clearTimeout(toastTimer.current);
    toastTimer.current = setTimeout(() => setToast(""), TOAST_MS);
  }, []);

  // Fire the release notification on the tick that crosses zero. Opening the
  // page after release, or switching to a game that is already out, must not
  // notify, hence the comparison with the previous state of the same target.
  const previous = useRef<{ target: number; elapsed: boolean } | null>(null);
  useEffect(() => {
    const last = previous.current;
    if (last && last.target === target && !last.elapsed && parts.elapsed) {
      showToast(`${game.title} is out now.`);
      if (settings.notify) void notify(`${game.title} is out now`, "The wait is over.");
    }
    previous.current = { target, elapsed: parts.elapsed };
  }, [target, parts.elapsed, game.title, settings.notify, showToast]);

  // --------------------------------------------------------------- actions

  const setNotify = useCallback(
    async (enabled: boolean) => {
      if (!enabled) return update({ notify: false });
      const granted = await requestNotificationPermission().catch(() => false);
      update({ notify: granted });
      showToast(
        granted ? "You will be notified at release while the page is open." : "Notifications are blocked for this site.",
      );
    },
    [update, showToast],
  );

  const onFullscreen = useCallback(async () => {
    try {
      setFullscreen(await toggleFullscreen());
    } catch {
      showToast("Fullscreen is not available here.");
    }
  }, [showToast]);

  const onCopy = useCallback(async () => {
    const reading = breakdown(clock.precise(), target);
    const text = reading.elapsed
      ? `${game.title} has been out for ${formatCompact(reading)}.`
      : `${game.title} unlocks in ${formatCompact(reading)} (${targetDate} at ${targetTime}, ${zoneText}).`;
    // Phones get the system share sheet; a dismissed sheet needs no message.
    if (canShare) {
      const outcome = await shareText(`${game.shortTitle} Countdown`, text);
      if (outcome !== "failed") return;
    }
    showToast((await copyText(text)) ? "Countdown copied to the clipboard." : "Copying failed.");
  }, [target, game, targetDate, targetTime, zoneText, showToast]);

  const onCalendar = useCallback(() => {
    const platforms = game.platforms.length ? ` Platforms at launch: ${game.platforms.join(", ")}.` : "";
    const calendar = buildCalendar({
      uid: `release-${gameSlug(game)}-${target}@gtavi-countdown`,
      start: target,
      durationMinutes: 60,
      summary: `${game.title} release`,
      description: `${game.title} unlocks on ${targetDate} at ${targetTime} (${zoneText}).${platforms}`,
      reminders: [24 * 60, 60],
    });
    saveCalendar(`${gameSlug(game)}-release.ics`, calendar);
    showToast("Calendar event downloaded.");
  }, [game, target, targetDate, targetTime, zoneText, showToast]);

  // Keyboard shortcuts, read through a ref so the listener is added once.
  const actions = useRef({ onFullscreen, onCopy, cycleTheme, update, showMilliseconds: settings.showMilliseconds });
  actions.current = { onFullscreen, onCopy, cycleTheme, update, showMilliseconds: settings.showMilliseconds };
  useEffect(() => {
    const onKeydown = (event: KeyboardEvent) => {
      if (event.ctrlKey || event.metaKey || event.altKey || event.repeat) return;
      const element = event.target as HTMLElement | null;
      if (element?.closest("input, textarea, select, [contenteditable]")) return;
      const a = actions.current;
      switch (event.key.toLowerCase()) {
        case "s":
          setSettingsOpen((open) => !open);
          break;
        case "f":
          void a.onFullscreen();
          break;
        case "m":
          a.update({ showMilliseconds: !a.showMilliseconds });
          break;
        case "t":
          a.cycleTheme();
          break;
        case "c":
          void a.onCopy();
          break;
        default:
          return;
      }
      event.preventDefault();
    };
    window.addEventListener("keydown", onKeydown);
    return () => window.removeEventListener("keydown", onKeydown);
  }, []);

  const long = [...game.title].length > 24;

  return (
    <>
      <Backdrop theme={settings.theme} effects={settings.effects} composition={composition} />

      <div className={styles.shell}>
        <header className={styles.bar}>
          <div className={styles.brand}>
            <img className={styles.brandMark} src="./favicon.svg" alt="" width="26" height="26" />
            <span className={styles.brandName}>{game.shortTitle}</span>
            <span className={styles.brandSub}>Countdown</span>
          </div>

          <nav className={styles.tools} aria-label="Actions">
            <button type="button" className={styles.tool} onClick={onCalendar} title="Add to calendar">
              <Icon name="calendar" />
              <span className="visually-hidden">Add to calendar</span>
            </button>
            {canShare ? (
              <button type="button" className={styles.tool} onClick={onCopy} title="Share the countdown">
                <Icon name="share" />
                <span className="visually-hidden">Share the countdown</span>
              </button>
            ) : (
              <button type="button" className={styles.tool} onClick={onCopy} title="Copy the countdown (C)">
                <Icon name="copy" />
                <span className="visually-hidden">Copy the countdown</span>
              </button>
            )}
            {canFullscreen && (
              <button type="button" className={styles.tool} onClick={onFullscreen} title="Toggle fullscreen (F)">
                <Icon name={fullscreen ? "compress" : "expand"} />
                <span className="visually-hidden">Toggle fullscreen</span>
              </button>
            )}
            <button
              type="button"
              className={styles.tool}
              aria-haspopup="dialog"
              onClick={() => setSettingsOpen(true)}
              title="Settings (S)"
            >
              <Icon name="sliders" />
              <span className="visually-hidden">Settings</span>
            </button>
          </nav>
        </header>

        <main className={styles.main}>
          <div className={styles.hero} ref={hero}>
            <p className={cx("label", styles.eyebrow, parts.elapsed && styles.out)}>
              {parts.elapsed ? "Out now" : "Countdown to"}
            </p>
            <h1 className={cx(styles.title, long && styles.longTitle)}>{game.title}</h1>

            <p className={styles.when}>
              <span>
                {parts.elapsed ? "Released" : "Unlocks"} {targetDate} at {targetTime}
              </span>
              <button
                type="button"
                className={styles.zone}
                onClick={() => setSettingsOpen(true)}
                title="Change the time zone"
              >
                {zoneText}
                <Icon name="chevron" size={14} />
              </button>
            </p>
            {localEquivalent && <p className={styles.local}>{localEquivalent} on this device</p>}

            <div className={styles.timer}>
              <Countdown parts={parts} target={target} showMilliseconds={settings.showMilliseconds} />
            </div>
            {parts.elapsed && <p className={cx("label", styles.since)}>Since release</p>}
          </div>

          <div className={styles.details} ref={details}>
            <Timeline game={game} now={now} target={target} />
            {!parts.elapsed && <Stats now={now} target={target} timeZone={zone} />}
          </div>
        </main>

        <footer className={styles.footer}>
          <p>
            Unofficial fan project, not affiliated with the developers or publishers of the games shown. Release dates
            and unlock hours are as reported and may change.
          </p>
          <p className={styles.clockSource}>System clock</p>
        </footer>
      </div>

      <SettingsPanel
        open={settingsOpen}
        settings={settings}
        reference={target}
        onClose={() => setSettingsOpen(false)}
        onChange={update}
        onNotify={(enabled) => void setNotify(enabled)}
        onReset={reset}
      />

      <div className={cx(styles.toast, toast !== "" && styles.visible)} role="status" aria-live="polite">
        {toast}
      </div>
    </>
  );
}
