import { rgb, type Palette } from "./gl/scene";
import type { Theme } from "./settings.svelte";

export interface ThemeInfo {
  label: string;
  /** Colours handed to the backdrop shader. */
  palette: Palette;
  /** CSS gradient used when the shader is disabled or WebGL is unavailable. */
  fallback: string;
  /** Three colours for the swatch in the settings panel. */
  swatch: readonly [string, string, string];
}

export const THEME_INFO: Record<Theme, ThemeInfo> = {
  dusk: {
    label: "Dusk",
    palette: {
      skyTop: rgb("#120a30"),
      skyMid: rgb("#7b1d72"),
      skyLow: rgb("#f2643d"),
      sunHot: rgb("#ffe27a"),
      sunCool: rgb("#ff3d81"),
      glow: rgb("#5c2410"),
      water: rgb("#1b0b3d"),
      stars: 0.55,
      bands: 1,
    },
    fallback:
      "linear-gradient(180deg, #120a30 0%, #4a1560 34%, #a02a6c 52%, #f2643d var(--horizon, 66%), #2a1050 calc(var(--horizon, 66%) + 0.2%), #140a2c 100%)",
    swatch: ["#120a30", "#a02a6c", "#f2643d"],
  },
  midnight: {
    label: "Midnight",
    palette: {
      skyTop: rgb("#020611"),
      skyMid: rgb("#0b2356"),
      skyLow: rgb("#157f8f"),
      sunHot: rgb("#f6f9ff"),
      sunCool: rgb("#a9c8ff"),
      glow: rgb("#0b2c44"),
      water: rgb("#030b1e"),
      stars: 1,
      bands: 0,
    },
    fallback:
      "linear-gradient(180deg, #020611 0%, #081a40 36%, #0e4a70 54%, #157f8f var(--horizon, 66%), #06142e calc(var(--horizon, 66%) + 0.2%), #020611 100%)",
    swatch: ["#020611", "#0b2356", "#157f8f"],
  },
  dawn: {
    label: "Dawn",
    palette: {
      skyTop: rgb("#10224f"),
      skyMid: rgb("#a8527f"),
      skyLow: rgb("#f7a765"),
      sunHot: rgb("#fff1c2"),
      sunCool: rgb("#ff9a5e"),
      glow: rgb("#4a2a16"),
      water: rgb("#142a55"),
      stars: 0.2,
      bands: 0.6,
    },
    fallback:
      "linear-gradient(180deg, #10224f 0%, #5a3a70 34%, #b85f7d 52%, #f7a765 var(--horizon, 66%), #1c356a calc(var(--horizon, 66%) + 0.2%), #0f1f45 100%)",
    swatch: ["#10224f", "#a8527f", "#f7a765"],
  },
};
