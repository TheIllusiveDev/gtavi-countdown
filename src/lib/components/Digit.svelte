<script lang="ts">
  import { cubicOut } from "svelte/easing";
  import type { TransitionConfig } from "svelte/transition";

  let { value }: { value: string } = $props();

  const reducedMotion =
    typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;

  /**
   * Slides a glyph vertically while fading it. Svelte compiles the `css`
   * callback into a keyframe animation, so the roll runs on the compositor
   * and costs no script time per frame.
   */
  function roll(_node: Element, { from }: { from: number }): TransitionConfig {
    if (reducedMotion) return { duration: 0 };
    return {
      duration: 460,
      easing: cubicOut,
      css: (t, u) => `transform: translate3d(0, ${from * u * 58}%, 0); opacity: ${t};`,
    };
  }
</script>

<span class="cell" aria-hidden="true">
  {#key value}
    <span class="glyph" in:roll={{ from: -1 }} out:roll={{ from: 1 }}>{value}</span>
  {/key}
</span>

<style>
  .cell {
    display: inline-grid;
    width: 0.5em;
    height: 1em;
    line-height: 1;
    /* The cell has a fixed size, so its content can never affect the layout
       around it. Paint is deliberately not contained: the glyph fades as it
       travels, and clipping it would cut the text shadow into a visible box. */
    contain: size layout style;
  }

  .glyph {
    grid-area: 1 / 1;
    display: block;
    text-align: center;
    will-change: transform, opacity;
    backface-visibility: hidden;
  }
</style>
