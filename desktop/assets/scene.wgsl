// The animated backdrop: a coastal skyline at sunset, drawn entirely in a
// fragment shader on a single full-screen triangle. A WGSL port of the GLSL
// shader of the web version; wgpu runs it on Vulkan, Metal, DirectX 12 or
// OpenGL ES.

struct Scene {
  resolution: vec2<f32>,
  time: f32,
  stars: f32,
  sky_top: vec3<f32>,
  bands: f32,
  sky_mid: vec3<f32>,
  // Height of the horizon in a space where y runs from -0.5 at the bottom
  // to 0.5 at the top, and the radius of the sun in the same units. Both
  // follow the layout.
  horizon: f32,
  sky_low: vec3<f32>,
  sun_radius: f32,
  sun_hot: vec3<f32>,
  // Octaves of cloud noise: 4 normally, 6 on the High preset.
  octaves: f32,
  sun_cool: vec3<f32>,
  _pad0: f32,
  glow: vec3<f32>,
  _pad1: f32,
  water: vec3<f32>,
  _pad2: f32,
}

@group(0) @binding(0) var<uniform> u: Scene;

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) uv: vec2<f32>,
}

// One oversized triangle covers the viewport with no diagonal seam.
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
  let x = f32(i32(index & 1u) * 4 - 1);
  let y = f32(i32(index >> 1u) * 4 - 1);
  var out: VertexOutput;
  out.position = vec4<f32>(x, y, 0.0, 1.0);
  out.uv = vec2<f32>((x + 1.0) * 0.5, 1.0 - (y + 1.0) * 0.5);
  return out;
}

// GLSL's smoothstep, defined for reversed edges as GLSL does.
fn ss(a: f32, b: f32, x: f32) -> f32 {
  let t = clamp((x - a) / (b - a), 0.0, 1.0);
  return t * t * (3.0 - 2.0 * t);
}

fn hash(q: vec2<f32>) -> f32 {
  var p = fract(q * vec2<f32>(123.34, 456.21));
  p = p + dot(p, p + 45.32);
  return fract(p.x * p.y);
}

fn noise(p: vec2<f32>) -> f32 {
  let i = floor(p);
  let f = fract(p);
  let w = f * f * (3.0 - 2.0 * f);
  return mix(
    mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), w.x),
    mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), w.x),
    w.y
  );
}

fn fbm(q: vec2<f32>) -> f32 {
  var p = q;
  var amplitude = 0.5;
  var sum = 0.0;
  for (var i = 0; i < 6; i = i + 1) {
    if (f32(i) >= u.octaves) {
      break;
    }
    sum = sum + amplitude * noise(p);
    p = p * 2.03 + vec2<f32>(17.1, 9.2);
    amplitude = amplitude * 0.5;
  }
  return sum;
}

// Height profile of one row of buildings. The density is the number of
// buildings per unit of width; the seed decorrelates the rows.
fn towers(x: f32, density: f32, seed: f32) -> f32 {
  let cell = floor(x * density);
  // Districts: broad swells of taller construction separated by low rise.
  let district = ss(0.2, 0.9, noise(vec2<f32>(x * 2.3 + seed * 5.1, seed)));
  let h = hash(vec2<f32>(cell, seed));
  var height = 0.08 + district * (0.2 + 0.8 * h * h);
  // Occasional landmark towers, and narrow gaps between neighbours.
  height = height + step(0.94, hash(vec2<f32>(cell, seed + 13.0))) * district * 0.55;
  height = height * step(0.07, fract(x * density));
  return height;
}

// Colour of the sky, sun, clouds and skyline at p. Also used, mirrored, for
// the reflection in the water, so the scene is evaluated once per pixel.
fn scene(p: vec2<f32>, t: f32, detail: f32) -> vec3<f32> {
  let altitude = clamp((p.y - u.horizon) / (0.5 - u.horizon), 0.0, 1.0);

  var col = mix(u.sky_low, u.sky_mid, ss(0.0, 0.38, altitude));
  col = mix(col, u.sky_top, ss(0.22, 1.0, altitude));

  // Stars, fading out towards the bright horizon.
  let grid = p * 150.0;
  let id = floor(grid);
  let seed = hash(id);
  let offset = vec2<f32>(hash(id + 3.1), hash(id + 7.7)) - 0.5;
  let star = ss(0.16, 0.0, length(fract(grid) - 0.5 - offset * 0.6));
  let twinkle = 0.6 + 0.4 * sin(t * (0.8 + seed * 2.5) + seed * 40.0);
  let stars = step(0.955, seed) * star * twinkle * ss(0.3, 0.95, altitude) * u.stars;

  // Sun: a graded disc with slits opening towards its lower edge.
  let sun_pos = vec2<f32>(0.0, u.horizon + u.sun_radius * 0.12);
  let to_sun = p - sun_pos;
  let dist = length(to_sun);
  var disc = ss(u.sun_radius, u.sun_radius - 0.003, dist);
  let sun_grade = clamp((to_sun.y + u.sun_radius) / (2.0 * u.sun_radius), 0.0, 1.0);
  let sun_col = mix(u.sun_cool, u.sun_hot, ss(0.1, 0.9, sun_grade));
  let below = clamp((u.sun_radius * 0.58 - to_sun.y) / u.sun_radius, 0.0, 1.0);
  let slit = step(below * 0.66, fract(below * 6.5 - t * 0.035));
  disc = disc * mix(1.0, slit, step(0.001, below) * u.bands);

  // Atmospheric glow around the sun and along the horizon.
  let halo = exp(-dist * 4.2) * 0.75 + exp(-dist * 14.0) * 0.35;
  let haze = exp(-altitude * 7.0) * 0.4;
  col = col + u.glow * (halo + haze);

  // Clouds: horizontally stretched noise, lit from below by the horizon.
  let drift = t * 0.011;
  let cloud_noise = fbm(vec2<f32>(p.x * 1.7 + drift, p.y * 8.5 - drift * 0.3));
  let cloud_mask = ss(0.03, 0.2, altitude) * (1.0 - ss(0.55, 1.0, altitude));
  let cloud = ss(0.5, 0.82, cloud_noise) * cloud_mask * detail;
  let cloud_col = mix(u.sky_low * 1.15 + u.glow * 0.35, u.sky_mid * 0.55, ss(0.0, 0.7, altitude));

  col = col + vec3<f32>(stars) * (1.0 - cloud);
  col = mix(col, sun_col, disc);
  col = mix(col, cloud_col, cloud * 0.62);

  // Skyline: a hazy far row behind a dark near row with lit windows.
  let rise = p.y - u.horizon;
  let above = step(0.0, rise);

  let far_top = 0.004 + towers(p.x + 7.3, 64.0, 2.0) * 0.05;
  let far_col = mix(u.sky_low, u.sky_top, 0.72);
  col = mix(col, far_col, above * step(rise, far_top) * 0.85);

  let density = 34.0;
  let near_top = 0.005 + towers(p.x, density, 5.0) * 0.068;
  let building = above * step(rise, near_top);
  // Windows sit on a regular grid inside each building, with dark mullions
  // between them so that they read as separate points of light.
  let pane = vec2<f32>(p.x * density * 5.0, rise * 300.0);
  let pane_id = floor(pane);
  let in_pane = fract(pane);
  let glass = step(0.22, in_pane.x) * step(in_pane.x, 0.78) * step(0.2, in_pane.y) * step(in_pane.y, 0.72);
  let lit = step(0.8, hash(pane_id)) * glass * step(0.006, near_top - rise) * detail;
  let flicker = 0.8 + 0.2 * sin(t * 0.6 + hash(pane_id + 1.3) * 30.0);
  var building_col = u.sky_top * 0.2 + u.sky_mid * 0.04;
  building_col = building_col + mix(u.sun_hot, u.sky_low, hash(pane_id + 9.1) * 0.7) * lit * flicker;
  col = mix(col, building_col, building);

  return col;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  // Framebuffer y runs downwards here; the scene is built with y upwards.
  let frag = vec2<f32>(in.position.x, u.resolution.y - in.position.y);
  let uv = frag / u.resolution;
  let aspect = u.resolution.x / u.resolution.y;
  let p = (uv - 0.5) * vec2<f32>(aspect, 1.0);
  let t = u.time;

  var col: vec3<f32>;
  if (p.y >= u.horizon) {
    col = scene(p, t, 1.0);
  } else {
    // Water: mirror the scene about the horizon and disturb it with ripples
    // that grow towards the viewer.
    let depth = (u.horizon - p.y) / (u.horizon + 0.5);
    let perspective = 1.0 / (depth + 0.06);
    let ripple = noise(vec2<f32>(p.x * 5.0 * perspective * 0.35, perspective * 3.2 + t * 0.55)) - 0.5;
    let swell = noise(vec2<f32>(p.x * 2.2 + t * 0.07, depth * 26.0 - t * 0.32)) - 0.5;
    let q = vec2<f32>(
      p.x + (ripple * 0.05 + swell * 0.03) * (0.25 + depth * 1.6),
      u.horizon + (u.horizon - p.y) * 1.25 + ripple * 0.012 * depth
    );
    let reflection = scene(q, t, 0.0);
    let fresnel = mix(0.82, 0.3, ss(0.0, 0.9, depth));
    col = mix(u.water, reflection, fresnel);

    // Glitter in the column of light under the sun.
    let column = exp(-abs(p.x) * (5.0 - depth * 2.5));
    let sparkle = pow(noise(vec2<f32>(p.x * 60.0 * perspective * 0.2, perspective * 9.0 + t * 1.4)), 5.0);
    col = col + u.sun_hot * sparkle * column * 1.6 * (0.3 + depth);

    // A thin bright line where water meets sky.
    col = col + u.glow * exp(-depth * 90.0) * 0.35;
  }

  // Vignette, then grain. The grain doubles as dithering and removes the
  // banding that smooth gradients show on 8-bit displays.
  let v = uv - 0.5;
  col = col * (1.0 - dot(v, v) * 0.75);
  col = col + (hash(frag + fract(t) * 91.7) - 0.5) * 0.028;

  return vec4<f32>(clamp(col, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
