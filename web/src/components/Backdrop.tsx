import { useEffect, useRef, useState, type CSSProperties } from "react";
import { cx } from "../lib/cx";
import { Scene, type Composition } from "../lib/gl/scene";
import type { EffectLevel, Theme } from "../lib/settings";
import { THEME_INFO } from "../lib/themes";
import { useReducedMotion } from "../lib/useReducedMotion";
import styles from "./Backdrop.module.css";

export function Backdrop({
  theme,
  effects,
  composition,
}: {
  theme: Theme;
  effects: EffectLevel;
  composition: Composition;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const scene = useRef<Scene | null>(null);
  /** False until a GL context exists; the CSS gradient shows through meanwhile. */
  const [live, setLive] = useState(false);
  const reducedMotion = useReducedMotion();
  const enabled = effects !== "off";

  // The latest values, read when a renderer is created without making the
  // palette a reason to rebuild the context.
  const latest = useRef({ theme, effects, composition, reducedMotion });
  latest.current = { theme, effects, composition, reducedMotion };

  // Create the renderer when effects are enabled, tear it down when they are
  // switched off so that no GPU memory or context is held needlessly.
  useEffect(() => {
    if (!enabled || !canvas.current) return;
    const created = new Scene(canvas.current, THEME_INFO[latest.current.theme].palette);
    if (!created.init()) return;
    scene.current = created;
    setLive(true);
    return () => {
      created.destroy();
      scene.current = null;
      setLive(false);
    };
  }, [enabled]);

  useEffect(() => {
    const current = scene.current;
    if (!live || !current) return;
    current.setComposition({ ...composition });
    current.setPalette(THEME_INFO[theme].palette);
    current.setQuality(effects === "low" ? "low" : effects === "high" ? "high" : "auto");
    current.setAnimated(!reducedMotion);
  }, [live, theme, effects, composition, reducedMotion]);

  const style = {
    background: THEME_INFO[theme].fallback,
    "--horizon": `${(composition.horizon * 100).toFixed(2)}%`,
  } as CSSProperties;

  return (
    <div className={styles.backdrop} style={style} aria-hidden="true">
      {enabled && <canvas ref={canvas} className={cx(styles.canvas, live && styles.live)} />}
      <div className={styles.scrim} />
    </div>
  );
}
