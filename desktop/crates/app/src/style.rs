//! Fonts, colours and text helpers shared by every part of the interface.
//! The values mirror the stylesheet of the web version.

use std::sync::Arc;

use eframe::egui::{
    self, Align, Color32, FontData, FontDefinitions, FontFamily, FontId, Galley, Painter, Pos2, Rect, Shape, Stroke,
    epaint::{Shadow, text::LayoutJob, text::TextFormat, text::VariationCoords},
};
use gtavi_core::themes::{Rgb, Theme};

pub const INK: Color32 = Color32::WHITE;

pub fn ink(alpha: f32) -> Color32 {
    Color32::from_white_alpha((alpha * 255.0).round() as u8)
}

pub fn ink_soft() -> Color32 {
    ink(0.76)
}

pub fn ink_faint() -> Color32 {
    ink(0.56)
}

pub fn line() -> Color32 {
    ink(0.14)
}

pub fn line_strong() -> Color32 {
    ink(0.26)
}

pub fn rgb(color: Rgb) -> Color32 {
    Color32::from_rgb(color.0, color.1, color.2)
}

pub fn rgba(color: Rgb, alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(color.0, color.1, color.2, (alpha.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// The colours of the current theme, ready to draw with.
#[derive(Clone, Copy)]
pub struct Colors {
    pub accent: Color32,
    pub accent_2: Color32,
    pub base: Color32,
    pub panel: Color32,
    pub panel_solid: Color32,
    scrim: Rgb,
    accent_rgb: Rgb,
}

impl Colors {
    pub fn of(theme: Theme) -> Self {
        let ui = theme.info().ui;
        Colors {
            accent: rgb(ui.accent),
            accent_2: rgb(ui.accent_2),
            base: rgb(ui.base),
            panel: rgba(ui.panel.0, ui.panel.1),
            panel_solid: rgb(ui.panel_solid),
            scrim: ui.scrim,
            accent_rgb: ui.accent,
        }
    }

    pub fn scrim(&self, alpha: f32) -> Color32 {
        rgba(self.scrim, alpha)
    }

    pub fn accent_alpha(&self, alpha: f32) -> Color32 {
        rgba(self.accent_rgb, alpha)
    }
}

// ------------------------------------------------------------------ fonts

const SANS: &str = "archivo";
const MONO: &str = "jetbrains-mono";

/// Registers the bundled typefaces. Both are variable fonts; every weight and
/// width in the interface comes from these three files.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let files: [(&str, &'static [u8]); 3] = [
        ("archivo-latin", include_bytes!("../../../assets/fonts/archivo-latin-standard-normal.ttf")),
        ("archivo-latin-ext", include_bytes!("../../../assets/fonts/archivo-latin-ext-standard-normal.ttf")),
        ("jetbrains-mono", include_bytes!("../../../assets/fonts/jetbrains-mono-latin-wght-normal.ttf")),
    ];
    for (name, data) in files {
        fonts.font_data.insert(name.to_owned(), Arc::new(FontData::from_static(data)));
    }
    // egui's own fonts stay at the end of each list for anything else.
    let fallback = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let mut sans = vec!["archivo-latin".to_owned(), "archivo-latin-ext".to_owned()];
    sans.extend(fallback.iter().cloned());
    let mut mono = vec!["jetbrains-mono".to_owned()];
    mono.extend(fallback.iter().cloned());
    fonts.families.insert(FontFamily::Name(SANS.into()), sans.clone());
    fonts.families.insert(FontFamily::Name(MONO.into()), mono);
    fonts.families.insert(FontFamily::Proportional, sans);
    ctx.set_fonts(fonts);
}

/// A font at an absolute size in points, with its variable axes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Font {
    pub mono: bool,
    pub size: f32,
    pub weight: f32,
    /// The `wdth` axis in percent; ignored by the monospaced face.
    pub width: f32,
    /// Extra space after each letter, in points.
    pub spacing: f32,
}

impl Font {
    pub const fn sans(size: f32, weight: f32) -> Font {
        Font { mono: false, size, weight, width: 100.0, spacing: 0.0 }
    }

    pub const fn mono(size: f32, weight: f32) -> Font {
        Font { mono: true, size, weight, width: 100.0, spacing: 0.0 }
    }

    pub const fn width(mut self, width: f32) -> Font {
        self.width = width;
        self
    }

    pub const fn spacing(mut self, spacing: f32) -> Font {
        self.spacing = spacing;
        self
    }

    /// The small upper-case label used for eyebrows, units and captions.
    pub const fn caps() -> Font {
        Font::mono(11.0, 500.0).spacing(1.76)
    }

    pub fn id(&self) -> FontId {
        FontId::new(self.size, FontFamily::Name(if self.mono { MONO } else { SANS }.into()))
    }

    pub fn format(&self, color: Color32) -> TextFormat {
        let mut coords = VariationCoords::new([(b"wght", self.weight)]);
        if !self.mono {
            coords.push(b"wdth", self.width);
        }
        TextFormat {
            font_id: self.id(),
            extra_letter_spacing: self.spacing,
            color,
            coords,
            ..Default::default()
        }
    }
}

/// A layout job for one run of text, optionally wrapped and centred.
pub fn job(text: &str, font: Font, color: Color32, wrap: Option<f32>, center: bool) -> LayoutJob {
    let mut job = LayoutJob::single_section(text.to_owned(), font.format(color));
    if let Some(width) = wrap {
        job.wrap.max_width = width.max(1.0);
    }
    if center {
        job.halign = Align::Center;
    }
    job
}

pub fn galley(painter: &Painter, text: &str, font: Font, color: Color32) -> Arc<Galley> {
    painter.layout_job(job(text, font, color, None, false))
}

pub fn wrapped(painter: &Painter, text: &str, font: Font, color: Color32, width: f32, center: bool) -> Arc<Galley> {
    painter.layout_job(job(text, font, color, Some(width), center))
}

/// Size of a galley without the letter spacing that trails its last letter.
pub fn visible_width(galley: &Galley, font: Font) -> f32 {
    (galley.size().x - font.spacing).max(0.0)
}

/// Places a galley so that its bounding box has its top left at `pos`.
pub fn paint_at(painter: &Painter, pos: Pos2, galley: &Arc<Galley>) {
    painter.galley(pos - galley.rect.min.to_vec2(), galley.clone(), INK);
}

/// Places a galley with its horizontal centre at `x` and its top at `top`.
pub fn paint_centered(painter: &Painter, x: f32, top: f32, galley: &Arc<Galley>) {
    let pos = Pos2::new(x - galley.rect.center().x, top - galley.rect.top());
    painter.galley(pos, galley.clone(), INK);
}

/// Text over the scene keeps its contrast with a soft dark halo, standing in
/// for the CSS text shadow of the web version.
pub fn paint_shadowed(painter: &Painter, pos: Pos2, galley: &Arc<Galley>, shadow: Color32, blur: f32) {
    let rect = galley.rect.translate(pos.to_vec2() - galley.rect.min.to_vec2());
    soft_shadow(painter, rect.shrink2(egui::vec2(0.0, rect.height() * 0.2)), shadow, blur);
    painter.galley(pos - galley.rect.min.to_vec2(), galley.clone(), INK);
}

/// A blurred rectangle, used under text and panels.
pub fn soft_shadow(painter: &Painter, rect: Rect, color: Color32, blur: f32) {
    let shadow = Shadow { offset: [0, 0], blur: blur.clamp(0.0, 255.0) as u8, spread: 0, color };
    painter.add(shadow.as_shape(rect, (rect.height() / 2.0).min(255.0) as u8));
}

/// A vertical gradient across `rect` through `(position, colour)` stops,
/// positions from 0 at the top to 1 at the bottom.
pub fn vertical_gradient(rect: Rect, stops: &[(f32, Color32)]) -> Shape {
    let mut mesh = egui::Mesh::default();
    for (i, (position, color)) in stops.iter().enumerate() {
        let y = rect.top() + rect.height() * position;
        mesh.colored_vertex(Pos2::new(rect.left(), y), *color);
        mesh.colored_vertex(Pos2::new(rect.right(), y), *color);
        if i > 0 {
            let base = (i as u32 - 1) * 2;
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base + 1, base + 3, base + 2);
        }
    }
    Shape::mesh(mesh)
}

/// A horizontal gradient from `left` to `right` across `rect`.
pub fn horizontal_gradient(rect: Rect, left: Color32, right: Color32) -> Shape {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), left);
    mesh.colored_vertex(rect.right_top(), right);
    mesh.colored_vertex(rect.left_bottom(), left);
    mesh.colored_vertex(rect.right_bottom(), right);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    Shape::mesh(mesh)
}

/// Focus indicators are always visible for keyboard users.
pub fn focus_ring(painter: &Painter, rect: Rect, radius: f32, accent: Color32) {
    painter.rect_stroke(rect.expand(3.0), radius + 3.0, Stroke::new(2.0, accent), egui::StrokeKind::Outside);
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    a.lerp_to_gamma(b, t.clamp(0.0, 1.0))
}

/// `clamp(low, value, high)` as CSS writes it.
pub fn clamp(low: f32, value: f32, high: f32) -> f32 {
    value.clamp(low, high)
}

pub fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

pub fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
