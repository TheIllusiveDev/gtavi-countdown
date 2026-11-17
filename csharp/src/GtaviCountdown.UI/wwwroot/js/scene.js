/**
 * The animated backdrop: a coastal skyline at sunset, drawn entirely in a
 * fragment shader on a single full-screen triangle. A direct port of the
 * renderer of the web version; WebGL 1 is targeted on purpose, as it is
 * available in every embedded web view the desktop shell can run on.
 */

const VERTEX_SHADER = `
attribute vec2 aPosition;
void main() {
  gl_Position = vec4(aPosition, 0.0, 1.0);
}
`;

const FRAGMENT_SHADER = `
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif

uniform vec2 uResolution;
uniform float uTime;
uniform vec3 uSkyTop;
uniform vec3 uSkyMid;
uniform vec3 uSkyLow;
uniform vec3 uSunHot;
uniform vec3 uSunCool;
uniform vec3 uGlow;
uniform vec3 uWater;
uniform float uStars;
uniform float uBands;
// Height of the horizon in screen space, where y runs from -0.5 to 0.5, and
// the radius of the sun in the same units. Both follow the page layout.
uniform float uHorizon;
uniform float uSunRadius;

float hash(vec2 p) {
  p = fract(p * vec2(123.34, 456.21));
  p += dot(p, p + 45.32);
  return fract(p.x * p.y);
}

float noise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(
    mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x),
    mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x),
    u.y
  );
}

float fbm(vec2 p) {
  float amplitude = 0.5;
  float sum = 0.0;
  for (int i = 0; i < 4; i++) {
    sum += amplitude * noise(p);
    p = p * 2.03 + vec2(17.1, 9.2);
    amplitude *= 0.5;
  }
  return sum;
}

// Height profile of one row of buildings. The density is the number of
// buildings per unit of width; the seed decorrelates the rows.
float towers(float x, float density, float seed) {
  float cell = floor(x * density);
  // Districts: broad swells of taller construction separated by low rise.
  float district = smoothstep(0.2, 0.9, noise(vec2(x * 2.3 + seed * 5.1, seed)));
  float h = hash(vec2(cell, seed));
  float height = 0.08 + district * (0.2 + 0.8 * h * h);
  // Occasional landmark towers, and narrow gaps between neighbours.
  height += step(0.94, hash(vec2(cell, seed + 13.0))) * district * 0.55;
  height *= step(0.07, fract(x * density));
  return height;
}

// Colour of the sky, sun, clouds and skyline at p. Also used, mirrored, for
// the reflection in the water, so the scene is evaluated once per pixel.
vec3 scene(vec2 p, float t, float detail) {
  float altitude = clamp((p.y - uHorizon) / (0.5 - uHorizon), 0.0, 1.0);

  vec3 col = mix(uSkyLow, uSkyMid, smoothstep(0.0, 0.38, altitude));
  col = mix(col, uSkyTop, smoothstep(0.22, 1.0, altitude));

  // Stars, fading out towards the bright horizon.
  vec2 grid = p * 150.0;
  vec2 id = floor(grid);
  float seed = hash(id);
  vec2 offset = vec2(hash(id + 3.1), hash(id + 7.7)) - 0.5;
  float star = smoothstep(0.16, 0.0, length(fract(grid) - 0.5 - offset * 0.6));
  float twinkle = 0.6 + 0.4 * sin(t * (0.8 + seed * 2.5) + seed * 40.0);
  float stars = step(0.955, seed) * star * twinkle * smoothstep(0.3, 0.95, altitude) * uStars;

  // Sun: a graded disc with slits opening towards its lower edge.
  vec2 sunPos = vec2(0.0, uHorizon + uSunRadius * 0.12);
  vec2 toSun = p - sunPos;
  float dist = length(toSun);
  float disc = smoothstep(uSunRadius, uSunRadius - 0.003, dist);
  float sunGrade = clamp((toSun.y + uSunRadius) / (2.0 * uSunRadius), 0.0, 1.0);
  vec3 sunCol = mix(uSunCool, uSunHot, smoothstep(0.1, 0.9, sunGrade));
  float below = clamp((uSunRadius * 0.58 - toSun.y) / uSunRadius, 0.0, 1.0);
  float slit = step(below * 0.66, fract(below * 6.5 - t * 0.035));
  disc *= mix(1.0, slit, step(0.001, below) * uBands);

  // Atmospheric glow around the sun and along the horizon.
  float halo = exp(-dist * 4.2) * 0.75 + exp(-dist * 14.0) * 0.35;
  float haze = exp(-altitude * 7.0) * 0.4;
  col += uGlow * (halo + haze);

  // Clouds: horizontally stretched noise, lit from below by the horizon.
  float drift = t * 0.011;
  float cloudNoise = fbm(vec2(p.x * 1.7 + drift, p.y * 8.5 - drift * 0.3));
  float cloudMask = smoothstep(0.03, 0.2, altitude) * (1.0 - smoothstep(0.55, 1.0, altitude));
  float cloud = smoothstep(0.5, 0.82, cloudNoise) * cloudMask * detail;
  vec3 cloudCol = mix(uSkyLow * 1.15 + uGlow * 0.35, uSkyMid * 0.55, smoothstep(0.0, 0.7, altitude));

  col += vec3(stars) * (1.0 - cloud);
  col = mix(col, sunCol, disc);
  col = mix(col, cloudCol, cloud * 0.62);

  // Skyline: a hazy far row behind a dark near row with lit windows.
  float rise = p.y - uHorizon;
  float above = step(0.0, rise);

  float farTop = 0.004 + towers(p.x + 7.3, 64.0, 2.0) * 0.05;
  vec3 farCol = mix(uSkyLow, uSkyTop, 0.72);
  col = mix(col, farCol, above * step(rise, farTop) * 0.85);

  const float DENSITY = 34.0;
  float nearTop = 0.005 + towers(p.x, DENSITY, 5.0) * 0.068;
  float building = above * step(rise, nearTop);
  // Windows sit on a regular grid inside each building, with dark mullions
  // between them so that they read as separate points of light.
  vec2 pane = vec2(p.x * DENSITY * 5.0, rise * 300.0);
  vec2 paneId = floor(pane);
  vec2 inPane = fract(pane);
  float glass = step(0.22, inPane.x) * step(inPane.x, 0.78) * step(0.2, inPane.y) * step(inPane.y, 0.72);
  float lit = step(0.8, hash(paneId)) * glass * step(0.006, nearTop - rise) * detail;
  float flicker = 0.8 + 0.2 * sin(t * 0.6 + hash(paneId + 1.3) * 30.0);
  vec3 buildingCol = uSkyTop * 0.2 + uSkyMid * 0.04;
  buildingCol += mix(uSunHot, uSkyLow, hash(paneId + 9.1) * 0.7) * lit * flicker;
  col = mix(col, buildingCol, building);

  return col;
}

void main() {
  vec2 uv = gl_FragCoord.xy / uResolution;
  float aspect = uResolution.x / uResolution.y;
  vec2 p = (uv - 0.5) * vec2(aspect, 1.0);
  float t = uTime;

  vec3 col;
  if (p.y >= uHorizon) {
    col = scene(p, t, 1.0);
  } else {
    // Water: mirror the scene about the horizon and disturb it with ripples
    // that grow towards the viewer.
    float depth = (uHorizon - p.y) / (uHorizon + 0.5);
    float perspective = 1.0 / (depth + 0.06);
    float ripple = noise(vec2(p.x * 5.0 * perspective * 0.35, perspective * 3.2 + t * 0.55)) - 0.5;
    float swell = noise(vec2(p.x * 2.2 + t * 0.07, depth * 26.0 - t * 0.32)) - 0.5;
    vec2 q = vec2(
      p.x + (ripple * 0.05 + swell * 0.03) * (0.25 + depth * 1.6),
      uHorizon + (uHorizon - p.y) * 1.25 + ripple * 0.012 * depth
    );
    vec3 reflection = scene(q, t, 0.0);
    float fresnel = mix(0.82, 0.3, smoothstep(0.0, 0.9, depth));
    col = mix(uWater, reflection, fresnel);

    // Glitter in the column of light under the sun.
    float column = exp(-abs(p.x) * (5.0 - depth * 2.5));
    float sparkle = pow(noise(vec2(p.x * 60.0 * perspective * 0.2, perspective * 9.0 + t * 1.4)), 5.0);
    col += uSunHot * sparkle * column * 1.6 * (0.3 + depth);

    // A thin bright line where water meets sky.
    col += uGlow * exp(-depth * 90.0) * 0.35;
  }

  // Vignette, then grain. The grain doubles as dithering and removes the
  // banding that smooth gradients show on 8-bit displays.
  vec2 v = uv - 0.5;
  col *= 1.0 - dot(v, v) * 0.75;
  col += (hash(gl_FragCoord.xy + fract(t) * 91.7) - 0.5) * 0.028;

  gl_FragColor = vec4(clamp(col, 0.0, 1.0), 1.0);
}
`;


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
];

/** Device pixels are capped: the scene is soft and gains nothing beyond this. */
const MAX_PIXEL_RATIO = 1.5;
/** Resolution scales the adaptive governor steps through, best first. */
const SCALE_STEPS = [1, 0.75, 0.55, 0.4, 0.3];
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
 *
 * A palette has the shape { skyTop, skyMid, skyLow, sunHot, sunCool, glow,
 * water: [r, g, b], stars, bands }; a composition is { horizon, sunRadius }
 * as fractions of the viewport height.
 */
export class Scene {
  #gl = null;
  #program = null;
  #locations = {};
  #palette;
  #composition = { horizon: 0.66, sunRadius: 0.16 };
  #quality = "auto";
  #animated = true;
  #scaleIndex = 0;
  #slowFrames = 0;
  #frame = 0;
  #lastFrame = 0;
  #lastDraw = 0;
  #epoch = performance.now();
  #observer = null;
  #destroyed = false;

  constructor(canvas, palette) {
    this.canvas = canvas;
    this.#palette = palette;
  }

  /** Creates the GL context. Returns false when WebGL is unavailable. */
  init() {
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

  setPalette(palette) {
    this.#palette = palette;
    this.#drawStill();
  }

  setComposition(composition) {
    this.#composition = composition;
    this.#drawStill();
  }

  setQuality(quality) {
    if (quality === this.#quality) return;
    this.#quality = quality;
    this.#scaleIndex = quality === "low" ? 2 : 0;
    this.#slowFrames = 0;
    this.#resize();
  }

  /** Animated scenes run continuously; still scenes redraw only on demand. */
  setAnimated(animated) {
    if (animated === this.#animated) return;
    this.#animated = animated;
    if (animated) this.#schedule();
    else this.#drawStill();
  }

  destroy() {
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

  #build() {
    const gl = this.#gl;
    if (!gl) return false;

    const compile = (type, source) => {
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

  #resize() {
    const gl = this.#gl;
    if (!gl) return;
    const ratio = Math.min(window.devicePixelRatio || 1, MAX_PIXEL_RATIO);
    const scale = ratio * SCALE_STEPS[this.#scaleIndex];
    const width = Math.max(1, Math.round(this.canvas.clientWidth * scale));
    const height = Math.max(1, Math.round(this.canvas.clientHeight * scale));
    if (this.canvas.width !== width || this.canvas.height !== height) {
      this.canvas.width = width;
      this.canvas.height = height;
      gl.viewport(0, 0, width, height);
    }
    this.#drawStill();
  }

  #draw(now) {
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

  #drawStill() {
    if (!this.#animated) this.#draw(performance.now());
  }

  #schedule() {
    cancelAnimationFrame(this.#frame);
    if (this.#destroyed || !this.#animated || document.hidden) return;
    this.#lastFrame = 0;
    this.#frame = requestAnimationFrame(this.#tick);
  }

  #tick = (now) => {
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
  #govern(frameMs) {
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

  #onVisibility = () => {
    if (document.hidden) cancelAnimationFrame(this.#frame);
    else this.#schedule();
  };

  #onContextLost = (event) => {
    event.preventDefault();
    cancelAnimationFrame(this.#frame);
    this.#program = null;
  };

  #onContextRestored = () => {
    if (this.#build()) {
      this.#resize();
      this.#schedule();
    }
  };
}
