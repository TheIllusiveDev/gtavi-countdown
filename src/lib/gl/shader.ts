/**
 * GLSL ES 1.00 sources for the backdrop: a coastal skyline at sunset, drawn
 * entirely in a fragment shader on a single full-screen triangle.
 *
 * WebGL 1 is targeted on purpose. Nothing here needs WebGL 2, and WebGL 1 is
 * available in every embedded webview the desktop shell can run on.
 */

export const VERTEX_SHADER = /* glsl */ `
attribute vec2 aPosition;
void main() {
  gl_Position = vec4(aPosition, 0.0, 1.0);
}
`;

export const FRAGMENT_SHADER = /* glsl */ `
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
