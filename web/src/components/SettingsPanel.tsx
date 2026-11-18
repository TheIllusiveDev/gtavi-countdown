import { useEffect, useRef } from "react";
import { isTouch } from "../lib/platform";
import { EFFECT_LEVELS, THEMES, type EffectLevel, type SettingsData } from "../lib/settings";
import type { CustomGame } from "../lib/time/games";
import { THEME_INFO } from "../lib/themes";
import { GamePicker } from "./GamePicker";
import { Icon } from "./Icon";
import { Switch } from "./Switch";
import { ZonePicker } from "./ZonePicker";
import styles from "./SettingsPanel.module.css";

const EFFECT_LABELS: Record<EffectLevel, string> = {
  auto: "Auto",
  high: "High",
  low: "Low",
  off: "Off",
};

const SHORTCUTS: ReadonlyArray<readonly [string, string]> = [
  ["S", "Open or close settings"],
  ["F", "Toggle fullscreen"],
  ["M", "Toggle milliseconds"],
  ["T", "Next theme"],
  ["C", "Copy the countdown"],
  ["Esc", "Close settings"],
];

export function SettingsPanel({
  open,
  settings,
  reference,
  onClose,
  onChange,
  onNotify,
  onReset,
}: {
  open: boolean;
  settings: SettingsData;
  /** The selected release, at which zone offsets are listed. */
  reference: number;
  onClose: () => void;
  onChange: (patch: Partial<SettingsData>) => void;
  /** Called when the user turns notifications on or off. */
  onNotify: (enabled: boolean) => void;
  onReset: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);

  // The native dialog provides the focus trap, Escape handling and inert
  // background; this effect only mirrors the `open` prop onto it.
  useEffect(() => {
    const node = dialog.current;
    if (!node) return;
    if (open && !node.open) node.showModal();
    else if (!open && node.open) node.close();
  }, [open]);

  return (
    <dialog
      ref={dialog}
      className={styles.dialog}
      aria-labelledby="settings-title"
      onClose={onClose}
      onClick={(event) => {
        // A click on the backdrop lands on the dialog element itself.
        if (event.target === dialog.current) onClose();
      }}
    >
      <div className={styles.sheet}>
        <header className={styles.header}>
          <h2 id="settings-title" className={styles.heading}>
            Settings
          </h2>
          <button type="button" className={styles.close} aria-label="Close settings" onClick={onClose}>
            <Icon name="close" />
          </button>
        </header>

        {/* The content mounts only while open, so the zone list is built on demand. */}
        {open && (
          <div className={styles.body}>
            <section className={styles.section}>
              <h3 className="label">Game</h3>
              <GamePicker
                game={settings.game}
                customGame={settings.customGame}
                onSelect={(game: string, customGame?: CustomGame) =>
                  onChange(customGame ? { game, customGame } : { game })
                }
              />
            </section>

            <section className={styles.section}>
              <h3 className="label">Release time zone</h3>
              <p className={styles.note}>
                Games unlock at a local time, usually midnight, so the moment depends on where you are. Zones are
                listed in the order they reach it.
              </p>
              <ZonePicker
                value={settings.timeZone}
                reference={reference}
                onSelect={(timeZone) => onChange({ timeZone })}
              />
            </section>

            <section className={styles.section}>
              <h3 className="label">Display</h3>
              <div className={styles.field}>
                <span className={styles.fieldTitle} id="theme-label">
                  Theme
                </span>
                <div className={styles.themes} role="radiogroup" aria-labelledby="theme-label">
                  {THEMES.map((theme) => {
                    const info = THEME_INFO[theme];
                    return (
                      <button
                        key={theme}
                        type="button"
                        className={styles.theme}
                        role="radio"
                        aria-checked={settings.theme === theme}
                        onClick={() => onChange({ theme })}
                      >
                        <span
                          className={styles.swatch}
                          style={{
                            background: `linear-gradient(180deg, ${info.swatch[0]}, ${info.swatch[1]} 55%, ${info.swatch[2]})`,
                          }}
                        />
                        {info.label}
                      </button>
                    );
                  })}
                </div>
              </div>

              <div className={styles.field}>
                <span className={styles.fieldTitle} id="effects-label">
                  Animated backdrop
                </span>
                <div className={styles.segmented} role="radiogroup" aria-labelledby="effects-label">
                  {EFFECT_LEVELS.map((level) => (
                    <button
                      key={level}
                      type="button"
                      role="radio"
                      aria-checked={settings.effects === level}
                      onClick={() => onChange({ effects: level })}
                    >
                      {EFFECT_LABELS[level]}
                    </button>
                  ))}
                </div>
                <p className={styles.note}>
                  Auto lowers the rendering resolution when the frame rate drops. High renders every device pixel with
                  more cloud detail. Off replaces the scene with a still gradient.
                </p>
              </div>

              <Switch
                label="Milliseconds"
                hint="Adds a per-frame readout after the seconds."
                checked={settings.showMilliseconds}
                onChange={(showMilliseconds) => onChange({ showMilliseconds })}
              />
            </section>

            <section className={styles.section}>
              <h3 className="label">Alerts</h3>
              <Switch
                label="Notify me at release"
                hint="Shows a system notification when the countdown reaches zero while the page is open."
                checked={settings.notify}
                onChange={onNotify}
              />
            </section>

            {!isTouch && (
              <section className={styles.section}>
                <h3 className="label">Keyboard shortcuts</h3>
                <dl className={styles.shortcuts}>
                  {SHORTCUTS.map(([key, action]) => (
                    <div key={key} className={styles.shortcut}>
                      <dt className={styles.key}>
                        <kbd className={styles.kbd}>{key}</kbd>
                      </dt>
                      <dd className={styles.action}>{action}</dd>
                    </div>
                  ))}
                </dl>
              </section>
            )}

            <section className={`${styles.section} ${styles.reset}`}>
              <button type="button" className={styles.button} onClick={onReset}>
                Restore defaults
              </button>
            </section>
          </div>
        )}
      </div>
    </dialog>
  );
}
