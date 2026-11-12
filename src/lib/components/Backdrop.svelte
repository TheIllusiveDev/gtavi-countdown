<script lang="ts">
  import { untrack } from "svelte";
  import { Scene, type Composition } from "../gl/scene";
  import { THEME_INFO } from "../themes";
  import type { EffectLevel, Theme } from "../settings.svelte";

  let {
    theme,
    effects,
    composition,
  }: { theme: Theme; effects: EffectLevel; composition: Composition } = $props();

  let canvas: HTMLCanvasElement | undefined = $state();
  let scene: Scene | null = null;
  /** False until a GL context exists; the CSS gradient shows through meanwhile. */
  let live = $state(false);

  const motionQuery =
    typeof matchMedia === "function" ? matchMedia("(prefers-reduced-motion: reduce)") : null;
  let reducedMotion = $state(motionQuery?.matches ?? false);

  $effect(() => {
    if (!motionQuery) return;
    const update = () => (reducedMotion = motionQuery.matches);
    motionQuery.addEventListener("change", update);
    return () => motionQuery.removeEventListener("change", update);
  });

  // Create the renderer when effects are enabled, tear it down when they are
  // switched off so that no GPU memory or context is held needlessly.
  $effect(() => {
    if (effects === "off" || !canvas) return;
    // The palette is read untracked: a theme change must not rebuild the
    // context, it is applied by the effect below.
    const created = new Scene(canvas, untrack(() => THEME_INFO[theme].palette));
    if (!created.init()) return;
    scene = created;
    live = true;
    return () => {
      created.destroy();
      scene = null;
      live = false;
    };
  });

  $effect(() => {
    const palette = THEME_INFO[theme].palette;
    const quality = effects === "low" ? "low" : "auto";
    const animated = !reducedMotion;
    const placement = { ...composition };
    if (!live || !scene) return;
    scene.setComposition(placement);
    scene.setPalette(palette);
    scene.setQuality(quality);
    scene.setAnimated(animated);
  });
</script>

<div
  class="backdrop"
  style:background={THEME_INFO[theme].fallback}
  style:--horizon="{(composition.horizon * 100).toFixed(2)}%"
  aria-hidden="true"
>
  {#if effects !== "off"}
    <canvas bind:this={canvas} class:live></canvas>
  {/if}
  <div class="scrim"></div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 0;
    overflow: hidden;
    pointer-events: none;
  }

  canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
    opacity: 0;
    transition: opacity 600ms ease;
  }

  canvas.live {
    opacity: 1;
  }

  /* Darkens the top and bottom edges so that text over the scene keeps its
     contrast regardless of theme. */
  .scrim {
    position: absolute;
    inset: 0;
    background:
      linear-gradient(180deg, rgb(var(--scrim) / 0.5) 0%, rgb(var(--scrim) / 0) 22%),
      linear-gradient(0deg, rgb(var(--scrim) / 0.72) 0%, rgb(var(--scrim) / 0) 46%);
  }
</style>
