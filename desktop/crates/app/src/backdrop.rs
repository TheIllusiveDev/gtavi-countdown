//! The animated backdrop: the sunset scene of the web version, drawn by a
//! WGSL port of its fragment shader through an egui paint callback, with the
//! still gradient underneath and the scrim on top.
//!
//! wgpu runs the shader on Vulkan (Linux, Windows), Metal (macOS) or
//! DirectX 12, falling back to OpenGL ES. The scene is drawn into an
//! offscreen texture at the resolution the quality preset allows, then
//! scaled up into egui's render pass.
//!
//! Like the WebGL renderer it is ported from, it caps the pixel ratio, lowers
//! its internal resolution when frames are slow, and renders one still frame
//! when the desktop disables animations. The High preset renders at the full
//! device resolution with more cloud detail and is never lowered.

use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Painter, Rect};
use eframe::egui_wgpu::{self, CallbackResources, CallbackTrait, ScreenDescriptor};
use eframe::wgpu;
use gtavi_core::settings::EffectLevel;
use gtavi_core::themes::{Palette, Theme};

use crate::style::{self, Colors};

/// Device pixels are capped: the scene is soft and gains little beyond this.
const MAX_PIXEL_RATIO: f32 = 1.5;
/// The High preset renders every device pixel, up to this ratio.
const HIGH_PIXEL_RATIO: f32 = 3.0;
/// Resolution scales the adaptive governor steps through, best first.
const SCALE_STEPS: [f32; 5] = [1.0, 0.75, 0.55, 0.4, 0.3];
/// A frame slower than this counts against the current resolution.
const SLOW_FRAME: Duration = Duration::from_micros(1_000_000 / 42);
/// Consecutive slow frames tolerated before stepping the resolution down.
const SLOW_FRAME_LIMIT: i32 = 24;
/// The scene fades in over this long once it has drawn its first frame.
const FADE_IN: Duration = Duration::from_millis(600);
/// Frames are paced by the app rather than left to vsync, which some
/// compositors (WSLg among them) do not apply.
pub const FRAME: Duration = Duration::from_micros(16_667);
/// The High preset follows displays up to 120 Hz.
pub const HIGH_FRAME: Duration = Duration::from_micros(8_333);
/// The Low preset draws about 30 frames per second.
pub const LOW_FRAME: Duration = Duration::from_micros(33_333);

const SCENE_SHADER: &str = include_str!("../../../assets/scene.wgsl");
const BLIT_SHADER: &str = include_str!("../../../assets/blit.wgsl");

/// Where the horizon and the sun sit, as fractions of the window height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Composition {
    /// Distance of the horizon from the top.
    pub horizon: f32,
    pub sun_radius: f32,
}

impl Default for Composition {
    fn default() -> Self {
        Composition { horizon: 0.68, sun_radius: 0.16 }
    }
}

pub struct Backdrop {
    /// False when there is no wgpu renderer; the gradient shows instead.
    available: bool,
    animated: bool,
    scale_index: usize,
    slow_frames: i32,
    epoch: Instant,
    last_frame: Option<Instant>,
    fade_start: Option<Instant>,
}

impl Backdrop {
    /// Creates the pipelines in egui's renderer, where the paint callback
    /// finds them.
    pub fn new(render_state: Option<&egui_wgpu::RenderState>, animated: bool, effects: EffectLevel) -> Self {
        let available = match render_state {
            Some(state) => {
                let resources = SceneResources::new(&state.device, state.target_format);
                state.renderer.write().callback_resources.insert(resources);
                true
            }
            None => false,
        };
        let mut backdrop = Backdrop {
            available,
            animated,
            scale_index: 0,
            slow_frames: 0,
            epoch: Instant::now(),
            last_frame: None,
            fade_start: None,
        };
        backdrop.set_effects(effects);
        backdrop
    }

    pub fn set_effects(&mut self, effects: EffectLevel) {
        self.scale_index = if effects == EffectLevel::Low { 2 } else { 0 };
        self.slow_frames = 0;
        self.last_frame = None;
    }

    /// Releases the offscreen texture when the scene is switched off, so that
    /// no GPU memory is held needlessly. A later switch back on creates it again.
    pub fn release(&mut self, render_state: Option<&egui_wgpu::RenderState>) {
        if let Some(state) = render_state
            && let Some(resources) = state.renderer.write().callback_resources.get_mut::<SceneResources>()
        {
            resources.target = None;
        }
        self.fade_start = None;
    }

    /// Paints the whole backdrop into `rect` and returns when it next needs a
    /// frame, if it is animating.
    pub fn paint(
        &mut self,
        painter: &Painter,
        rect: Rect,
        theme: Theme,
        effects: EffectLevel,
        composition: Composition,
    ) -> Option<Duration> {
        let info = theme.info();
        let colors = Colors::of(theme);
        let now = Instant::now();
        let scene = effects != EffectLevel::Off && self.available;
        let mut next_frame = None;

        // The scene first, then the still gradient over it while it fades in;
        // the gradient alone when the scene is off or unavailable.
        let mut gradient_alpha = 1.0;
        if scene {
            if effects == EffectLevel::Auto && self.animated {
                self.govern(now);
            }
            let high = effects == EffectLevel::High;
            let callback = SceneCallback {
                palette: info.palette,
                composition,
                // Still frames use a fixed time so that they are reproducible.
                seconds: if self.animated { (now - self.epoch).as_secs_f32() } else { 12.0 },
                scale: if high { 1.0 } else { SCALE_STEPS[self.scale_index] },
                max_ratio: if high { HIGH_PIXEL_RATIO } else { MAX_PIXEL_RATIO },
                octaves: if high { 6.0 } else { 4.0 },
            };
            painter.add(egui_wgpu::Callback::new_paint_callback(rect, callback));

            let start = *self.fade_start.get_or_insert(now);
            gradient_alpha = if self.animated {
                1.0 - ((now - start).as_secs_f32() / FADE_IN.as_secs_f32()).min(1.0)
            } else {
                0.0
            };
            if self.animated {
                next_frame = Some(match effects {
                    EffectLevel::Low => LOW_FRAME,
                    EffectLevel::High => HIGH_FRAME,
                    _ => FRAME,
                });
            }
        }

        if gradient_alpha > 0.0 {
            // Stops placed relative to the horizon follow the composition;
            // out-of-order stops are clamped to the previous one, as in CSS.
            let mut previous = 0.0f32;
            let stops: Vec<(f32, Color32)> = info
                .fallback
                .iter()
                .map(|stop| {
                    let at = if stop.at_horizon { composition.horizon + stop.position } else { stop.position };
                    previous = at.clamp(previous, 1.0);
                    (previous, style::rgba(stop.color, gradient_alpha))
                })
                .collect();
            painter.add(style::vertical_gradient(rect, &stops));
            if scene && gradient_alpha < 1.0 {
                next_frame = Some(FRAME);
            }
        }

        // The scrim darkens the top and bottom edges so that text over the
        // scene keeps its contrast regardless of theme. Each covers only the
        // band where it is visible.
        let band = |from: f32, to: f32| {
            Rect::from_x_y_ranges(rect.x_range(), (rect.top() + rect.height() * from)..=(rect.top() + rect.height() * to))
        };
        painter.add(style::vertical_gradient(band(0.0, 0.22), &[(0.0, colors.scrim(0.5)), (1.0, colors.scrim(0.0))]));
        painter.add(style::vertical_gradient(band(0.54, 1.0), &[(0.0, colors.scrim(0.0)), (1.0, colors.scrim(0.72))]));
        next_frame
    }

    /// Steps the resolution down after a sustained run of slow frames. It
    /// does not step back up, which avoids oscillation.
    fn govern(&mut self, now: Instant) {
        let Some(last) = self.last_frame.replace(now) else { return };
        let frame = now - last;
        // Long gaps are a minimised window or a stalled compositor, not rendering cost.
        if frame > Duration::from_millis(250) {
            return;
        }
        if frame > SLOW_FRAME {
            self.slow_frames += 1;
        } else {
            self.slow_frames = (self.slow_frames - 2).max(0);
        }
        if self.slow_frames >= SLOW_FRAME_LIMIT && self.scale_index < SCALE_STEPS.len() - 1 {
            self.scale_index += 1;
            self.slow_frames = 0;
        }
    }
}

// ------------------------------------------------------------------- GPU

/// The uniform block of `scene.wgsl`, laid out as WGSL lays out its struct:
/// each `vec3` takes 16 bytes, with the following `f32` in its last four.
fn uniforms(frame: &SceneCallback, width: u32, height: u32) -> [f32; 32] {
    let p = &frame.palette;
    let mut u = [0.0f32; 32];
    u[0] = width as f32;
    u[1] = height as f32;
    u[2] = frame.seconds;
    u[3] = p.stars;
    let mut vec3_then = |at: usize, color: gtavi_core::themes::Rgb, scalar: f32| {
        u[at..at + 3].copy_from_slice(&color.to_f32());
        u[at + 3] = scalar;
    };
    vec3_then(4, p.sky_top, p.bands);
    // The shader works in a space where y runs from -0.5 at the bottom to 0.5 at the top.
    vec3_then(8, p.sky_mid, 0.5 - frame.composition.horizon);
    vec3_then(12, p.sky_low, frame.composition.sun_radius);
    vec3_then(16, p.sun_hot, frame.octaves);
    vec3_then(20, p.sun_cool, 0.0);
    vec3_then(24, p.glow, 0.0);
    vec3_then(28, p.water, 0.0);
    u
}

/// Pipelines and buffers, created once and kept in egui's renderer.
struct SceneResources {
    scene_pipeline: wgpu::RenderPipeline,
    blit_pipeline: wgpu::RenderPipeline,
    blit_layout: wgpu::BindGroupLayout,
    uniform_buffer: wgpu::Buffer,
    uniform_group: wgpu::BindGroup,
    sampler: wgpu::Sampler,
    /// The offscreen scene, its blit bind group and its size.
    target: Option<(wgpu::Texture, wgpu::BindGroup, u32, u32)>,
}

/// The scene is drawn into a plain 8-bit texture and blitted unchanged, as
/// the WebGL version draws into its canvas: egui composites in gamma space.
const SCENE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

impl SceneResources {
    fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let scene_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("backdrop scene"),
            source: wgpu::ShaderSource::Wgsl(SCENE_SHADER.into()),
        });
        let blit_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("backdrop blit"),
            source: wgpu::ShaderSource::Wgsl(BLIT_SHADER.into()),
        });

        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("backdrop uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let blit_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("backdrop blit"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline = |label: &str,
                        module: &wgpu::ShaderModule,
                        fragment: &str,
                        layout: &wgpu::BindGroupLayout,
                        format: wgpu::TextureFormat| {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[Some(layout)],
                immediate_size: 0,
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module,
                    entry_point: Some(fragment),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let scene_pipeline = pipeline("backdrop scene", &scene_module, "fs_main", &uniform_layout, SCENE_FORMAT);
        let blit_pipeline = pipeline("backdrop blit", &blit_module, "fs_blit", &blit_layout, target_format);

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("backdrop uniforms"),
            size: 32 * 4,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("backdrop uniforms"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() }],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("backdrop"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        SceneResources { scene_pipeline, blit_pipeline, blit_layout, uniform_buffer, uniform_group, sampler, target: None }
    }

    /// Makes sure the offscreen target exists at the given size.
    fn ensure_target(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if matches!(self.target, Some((_, _, w, h)) if w == width && h == height) {
            return;
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("backdrop scene"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: SCENE_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("backdrop blit"),
            layout: &self.blit_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        });
        self.target = Some((texture, group, width, height));
    }
}

/// One frame of the scene, as recorded by the paint callback.
struct SceneCallback {
    palette: Palette,
    composition: Composition,
    seconds: f32,
    scale: f32,
    max_ratio: f32,
    octaves: f32,
}

impl CallbackTrait for SceneCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen: &ScreenDescriptor,
        encoder: &mut wgpu::CommandEncoder,
        resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let Some(scene) = resources.get_mut::<SceneResources>() else { return Vec::new() };
        let [output_width, output_height] = screen.size_in_pixels;
        if output_width == 0 || output_height == 0 {
            return Vec::new();
        }
        let ppp = screen.pixels_per_point;
        let ratio = ppp.min(self.max_ratio) / ppp * self.scale;
        let width = ((output_width as f32 * ratio).round() as u32).max(1);
        let height = ((output_height as f32 * ratio).round() as u32).max(1);
        scene.ensure_target(device, width, height);
        let data = uniforms(self, width, height);
        let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_ne_bytes()).collect();
        queue.write_buffer(&scene.uniform_buffer, 0, &bytes);

        let Some((texture, _, _, _)) = &scene.target else { return Vec::new() };
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("backdrop scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&scene.scene_pipeline);
        pass.set_bind_group(0, &scene.uniform_group, &[]);
        pass.draw(0..3, 0..1);
        Vec::new()
    }

    fn paint(&self, _info: egui::PaintCallbackInfo, pass: &mut wgpu::RenderPass<'static>, resources: &CallbackResources) {
        let Some(scene) = resources.get::<SceneResources>() else { return };
        let Some((_, group, _, _)) = &scene.target else { return };
        pass.set_pipeline(&scene.blit_pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.draw(0..3, 0..1);
    }
}
