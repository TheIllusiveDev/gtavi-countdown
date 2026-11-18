// Scales the scene, rendered at a reduced resolution, up to the window,
// with linear filtering.

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

@group(0) @binding(0) var scene_texture: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;

@fragment
fn fs_blit(in: VertexOutput) -> @location(0) vec4<f32> {
  return textureSample(scene_texture, scene_sampler, in.uv);
}
