import { FRAGMENT_SHADER, VERTEX_SHADER } from "./shader";

type Rgb = readonly [number, number, number];

export interface Palette {
  skyTop: Rgb;
  skyMid: Rgb;
  skyLow: Rgb;
  sunHot: Rgb;
  sunCool: Rgb;
  glow: Rgb;
  water: Rgb;
  /** Star brightness from 0 to 1. */
  stars: number;
  /** Strength of the slits across the sun from 0 to 1. */
  bands: number;
}

/** Parses `#rrggbb` into normalised components. */
export function rgb(hex: string): Rgb {
  const n = Number.parseInt(hex.slice(1), 16);
  return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
}

export type Quality = "auto" | "low";

/** Where the scene sits in the viewport, as fractions of its height. */
export interface Composition {
  /** Distance of the horizon from the top edge, from 0 to 1. */
  horizon: number;
  /** Radius of the sun. */
  sunRadius: number;
}

const UNIFORMS = [
  "uResolution",
  "uTime",
  "uSkyTop",
  "uSkyMid",
  "uSkyLow",
  "uSunHot",
  "uSunCool",
  "uGlow",
  "uWater",
  "uStars",
  "uBands",
  "uHorizon",
  "uSunRadius",
] as const;
type UniformName = (typeof UNIFORMS)[number];

/** Device pixels are capped: the scene is soft and gains nothing beyond this. */
const MAX_PIXEL_RATIO = 1.5;
/** Resolution scales the adaptive governor steps through, best first. */
const SCALE_STEPS = [1, 0.75, 0.55, 0.4, 0.3] as const;
/** A frame slower than this counts against the current resolution. */
const SLOW_FRAME_MS = 1000 / 42;
/** Consecutive slow frames tolerated before stepping the resolution down. */
const SLOW_FRAME_LIMIT = 24;

/**
 * Renders the backdrop shader into a canvas.
 *
 * The renderer protects the frame rate of the rest of the page: it measures
 * frame times and lowers its internal resolution when the GPU cannot keep up
 * (software rasterisers, integrated graphics driving large displays), stops
 * drawing while the page is hidden, and can render a single still frame.
 */
export class Scene {
  readonly canvas: HTMLCanvasElement;
  #gl: WebGLRenderingContext | null = null;
  #program: WebGLProgram | null = null;
  #locations = {} as Record<UniformName, WebGLUniformLocation | null>;
  #palette: Palette;
  #composition: Composition = { horizon: 0.66, sunRadius: 0.16 };
  #quality: Quality = "auto";
  #animated = true;
  #scaleIndex = 0;
  #slowFrames = 0;
  #frame = 0;
  #lastFrame = 0;
  #lastDraw = 0;
  #epoch = performance.now();
  #observer: ResizeObserver | null = null;
  #destroyed = false;

  constructor(canvas: HTMLCanvasElement, palette: Palette) {
    this.canvas = canvas;
    this.#palette = palette;
  }

  /** Creates the GL context. Returns false when WebGL is unavailable. */
  init(): boolean {
    const gl = this.canvas.getContext("webgl", {
      alpha: false,
      antialias: false,
      depth: false,
      stencil: false,
      powerPreference: "high-performance",
      preserveDrawingBuffer: false,
    });
    if (!gl) return false;
    this.#gl = gl;
    if (!this.#build()) return false;

    this.canvas.addEventListener("webglcontextlost", this.#onContextLost);
    this.canvas.addEventListener("webglcontextrestored", this.#onContextRestored);
    document.addEventListener("visibilitychange", this.#onVisibility);

    this.#observer = new ResizeObserver(() => this.#resize());
    this.#observer.observe(this.canvas);
    this.#resize();
    this.#schedule();
    return true;
  }

  setPalette(palette: Palette): void {
    this.#palette = palette;
    this.#drawStill();
  }

  setComposition(composition: Composition): void {
    this.#composition = composition;
    this.#drawStill();
  }

  setQuality(quality: Quality): void {
    if (quality === this.#quality) return;
    this.#quality = quality;
    this.#scaleIndex = quality === "low" ? 2 : 0;
    this.#slowFrames = 0;
    this.#resize();
  }

  /** Animated scenes run continuously; still scenes redraw only on demand. */
  setAnimated(animated: boolean): void {
    if (animated === this.#animated) return;
    this.#animated = animated;
    if (animated) this.#schedule();
    else this.#drawStill();
  }

  destroy(): void {
    this.#destroyed = true;
    cancelAnimationFrame(this.#frame);
    this.#observer?.disconnect();
    this.canvas.removeEventListener("webglcontextlost", this.#onContextLost);
    this.canvas.removeEventListener("webglcontextrestored", this.#onContextRestored);
    document.removeEventListener("visibilitychange", this.#onVisibility);
    const gl = this.#gl;
    if (gl && this.#program) gl.deleteProgram(this.#program);
    this.#gl = null;
    this.#program = null;
  }

  #build(): boolean {
    const gl = this.#gl;
    if (!gl) return false;

    const compile = (type: number, source: string): WebGLShader | null => {
      const shader = gl.createShader(type);
      if (!shader) return null;
      gl.shaderSource(shader, source);
      gl.compileShader(shader);
      if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
        console.error("Backdrop shader failed to compile:", gl.getShaderInfoLog(shader));
        gl.deleteShader(shader);
        return null;
      }
      return shader;
    };

    const vertex = compile(gl.VERTEX_SHADER, VERTEX_SHADER);
    const fragment = compile(gl.FRAGMENT_SHADER, FRAGMENT_SHADER);
    const program = gl.createProgram();
    if (!vertex || !fragment || !program) return false;

    gl.attachShader(program, vertex);
    gl.attachShader(program, fragment);
    gl.bindAttribLocation(program, 0, "aPosition");
    gl.linkProgram(program);
    // Shaders are owned by the program from here on.
    gl.deleteShader(vertex);
    gl.deleteShader(fragment);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      console.error("Backdrop program failed to link:", gl.getProgramInfoLog(program));
      return false;
    }

    // One oversized triangle covers the viewport with no diagonal seam.
    const buffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);

    gl.useProgram(program);
    gl.disable(gl.DEPTH_TEST);
    gl.disable(gl.BLEND);
    for (const name of UNIFORMS) {
      this.#locations[name] = gl.getUniformLocation(program, name);
    }
    this.#program = program;
    return true;
  }

  #resize(): void {
    const gl = this.#gl;
    if (!gl) return;
    const ratio = Math.min(window.devicePixelRatio || 1, MAX_PIXEL_RATIO);
    const scale = ratio * SCALE_STEPS[this.#scaleIndex]!;
    const width = Math.max(1, Math.round(this.canvas.clientWidth * scale));
    const height = Math.max(1, Math.round(this.canvas.clientHeight * scale));
    if (this.canvas.width !== width || this.canvas.height !== height) {
      this.canvas.width = width;
      this.canvas.height = height;
      gl.viewport(0, 0, width, height);
    }
    this.#drawStill();
  }

  #draw(now: number): void {
    const gl = this.#gl;
    if (!gl || !this.#program || gl.isContextLost()) return;
    const u = this.#locations;
    const p = this.#palette;
    // Still frames use a fixed time so that they are reproducible.
    const seconds = this.#animated ? (now - this.#epoch) / 1000 : 12;
    gl.uniform2f(u.uResolution, this.canvas.width, this.canvas.height);
    gl.uniform1f(u.uTime, seconds);
    gl.uniform3f(u.uSkyTop, ...p.skyTop);
    gl.uniform3f(u.uSkyMid, ...p.skyMid);
    gl.uniform3f(u.uSkyLow, ...p.skyLow);
    gl.uniform3f(u.uSunHot, ...p.sunHot);
    gl.uniform3f(u.uSunCool, ...p.sunCool);
    gl.uniform3f(u.uGlow, ...p.glow);
    gl.uniform3f(u.uWater, ...p.water);
    gl.uniform1f(u.uStars, p.stars);
    gl.uniform1f(u.uBands, p.bands);
    gl.uniform1f(u.uHorizon, 0.5 - this.#composition.horizon);
    gl.uniform1f(u.uSunRadius, this.#composition.sunRadius);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }

  #drawStill(): void {
    if (!this.#animated) this.#draw(performance.now());
  }

  #schedule(): void {
    cancelAnimationFrame(this.#frame);
    if (this.#destroyed || !this.#animated || document.hidden) return;
    this.#lastFrame = 0;
    this.#frame = requestAnimationFrame(this.#tick);
  }

  #tick = (now: number): void => {
    if (this.#destroyed || !this.#animated || document.hidden) return;
    this.#frame = requestAnimationFrame(this.#tick);

    // The low preset halves the frame rate as well as the resolution.
    if (this.#quality === "low" && now - this.#lastDraw < 1000 / 31) return;

    if (this.#quality === "auto" && this.#lastFrame !== 0) {
      this.#govern(now - this.#lastFrame);
    }
    this.#lastFrame = now;
    this.#lastDraw = now;
    this.#draw(now);
  };

  /** Steps the resolution down after a sustained run of slow frames. */
  #govern(frameMs: number): void {
    // Long gaps are tab switches or debugger pauses, not rendering cost.
    if (frameMs > 250) return;
    if (frameMs > SLOW_FRAME_MS) {
      this.#slowFrames += 1;
    } else {
      this.#slowFrames = Math.max(0, this.#slowFrames - 2);
    }
    if (this.#slowFrames >= SLOW_FRAME_LIMIT && this.#scaleIndex < SCALE_STEPS.length - 1) {
      this.#scaleIndex += 1;
      this.#slowFrames = 0;
      this.#resize();
    }
  }

  #onVisibility = (): void => {
    if (document.hidden) cancelAnimationFrame(this.#frame);
    else this.#schedule();
  };

  #onContextLost = (event: Event): void => {
    event.preventDefault();
    cancelAnimationFrame(this.#frame);
    this.#program = null;
  };

  #onContextRestored = (): void => {
    if (this.#build()) {
      this.#resize();
      this.#schedule();
    }
  };
}
