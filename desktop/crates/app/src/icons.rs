//! The interface icons, from the same SVG path data as the web icons, and the
//! application icon, rasterised from the shapes of its SVG source.

use eframe::egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, Vec2, pos2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Sliders,
    Expand,
    Compress,
    Pin,
    Calendar,
    Copy,
    Close,
    Chevron,
    Sync,
}

impl Icon {
    fn path(self) -> &'static str {
        match self {
            Icon::Sliders => "M4 7h9M17 7h3M4 17h3M11 17h9M15 4.5v5M9 14.5v5",
            Icon::Expand => "M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5",
            Icon::Compress => "M9 4v5H4M15 4v5h5M9 20v-5H4M15 20v-5h5",
            Icon::Pin => "M9 4h6l-1 6 3 3v2H7v-2l3-3-1-6ZM12 15v6",
            Icon::Calendar => "M5 6h14v14H5zM5 10h14M9 3.5v4M15 3.5v4",
            Icon::Copy => "M9 9h11v11H9zM5 15H4V4h11v1",
            Icon::Close => "M6 6l12 12M18 6L6 18",
            Icon::Chevron => "M8 10l4 4 4-4",
            Icon::Sync => "M20 11a8 8 0 0 0-14.5-4M4 4v3.5h3.5M4 13a8 8 0 0 0 14.5 4M20 20v-3.5h-3.5",
        }
    }
}

/// Strokes an icon on a 24-unit grid, scaled to `size` points and centred on `center`.
pub fn paint(painter: &Painter, icon: Icon, center: Pos2, size: f32, color: Color32) {
    let scale = size / 24.0;
    let origin = center - Vec2::splat(size / 2.0);
    let width = 1.75 * scale;
    for (points, closed) in parse(icon.path()) {
        let points: Vec<Pos2> = points.iter().map(|p| origin + p.to_vec2() * scale).collect();
        if closed {
            painter.add(Shape::closed_line(points, Stroke::new(width, color)));
        } else {
            // Round caps, as the SVG strokes have.
            if let (Some(first), Some(last)) = (points.first(), points.last()) {
                painter.circle_filled(*first, width / 2.0, color);
                painter.circle_filled(*last, width / 2.0, color);
            }
            painter.add(Shape::line(points, Stroke::new(width, color)));
        }
    }
}

/// Splits SVG path data into polylines. Supports the commands the icons use:
/// M, L, H, V, A and Z, absolute and relative.
fn parse(data: &str) -> Vec<(Vec<Pos2>, bool)> {
    let mut tokens = Tokens { bytes: data.as_bytes(), at: 0 };
    let mut paths: Vec<(Vec<Pos2>, bool)> = Vec::new();
    let mut current: Vec<Pos2> = Vec::new();
    let mut pen = Pos2::ZERO;
    let mut start = Pos2::ZERO;
    let mut command = b'M';
    let flush = |current: &mut Vec<Pos2>, paths: &mut Vec<(Vec<Pos2>, bool)>, closed: bool| {
        if current.len() > 1 {
            paths.push((std::mem::take(current), closed));
        }
        current.clear();
    };
    while let Some(next) = tokens.command_or_number() {
        if let Some(c) = next {
            command = c;
            if c.eq_ignore_ascii_case(&b'z') {
                flush(&mut current, &mut paths, true);
                pen = start;
                continue;
            }
        }
        let relative = command.is_ascii_lowercase();
        let base = if relative { pen.to_vec2() } else { Vec2::ZERO };
        match command.to_ascii_uppercase() {
            b'M' => {
                flush(&mut current, &mut paths, false);
                pen = Pos2::new(tokens.number(), tokens.number()) + base;
                start = pen;
                current.push(pen);
                // Further pairs after a move are line segments.
                command = if relative { b'l' } else { b'L' };
            }
            b'L' => {
                pen = Pos2::new(tokens.number(), tokens.number()) + base;
                current.push(pen);
            }
            b'H' => {
                pen.x = tokens.number() + base.x;
                current.push(pen);
            }
            b'V' => {
                pen.y = tokens.number() + base.y;
                current.push(pen);
            }
            b'A' => {
                let (rx, ry, rotation) = (tokens.number(), tokens.number(), tokens.number());
                let (large, sweep) = (tokens.number() != 0.0, tokens.number() != 0.0);
                let end = Pos2::new(tokens.number(), tokens.number()) + base;
                current.extend(arc(pen, end, rx, ry, rotation, large, sweep));
                pen = end;
            }
            _ => break,
        }
    }
    flush(&mut current, &mut paths, false);
    paths
}

struct Tokens<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Tokens<'_> {
    fn skip(&mut self) {
        while self.at < self.bytes.len() && matches!(self.bytes[self.at], b' ' | b',' | b'\n') {
            self.at += 1;
        }
    }

    /// `Some(Some(c))` for a command letter, `Some(None)` when a number
    /// follows (an implicit repeat of the last command), `None` at the end.
    fn command_or_number(&mut self) -> Option<Option<u8>> {
        self.skip();
        let c = *self.bytes.get(self.at)?;
        if c.is_ascii_alphabetic() {
            self.at += 1;
            Some(Some(c))
        } else {
            Some(None)
        }
    }

    fn number(&mut self) -> f32 {
        self.skip();
        let begin = self.at;
        let mut seen_dot = false;
        if matches!(self.bytes.get(self.at), Some(b'-' | b'+')) {
            self.at += 1;
        }
        while let Some(&c) = self.bytes.get(self.at) {
            if c.is_ascii_digit() || (c == b'.' && !seen_dot) {
                seen_dot |= c == b'.';
                self.at += 1;
            } else {
                break;
            }
        }
        std::str::from_utf8(&self.bytes[begin..self.at]).ok().and_then(|s| s.parse().ok()).unwrap_or(0.0)
    }
}

/// Points along an SVG elliptical arc, by the endpoint-to-centre conversion
/// of the SVG specification (appendix B.2.4).
fn arc(from: Pos2, to: Pos2, rx: f32, ry: f32, rotation: f32, large: bool, sweep: bool) -> Vec<Pos2> {
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx == 0.0 || ry == 0.0 || from == to {
        return vec![to];
    }
    let (sin, cos) = rotation.to_radians().sin_cos();
    let d = (from - to) / 2.0;
    let x1 = cos * d.x + sin * d.y;
    let y1 = -sin * d.x + cos * d.y;
    let lambda = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
    if lambda > 1.0 {
        rx *= lambda.sqrt();
        ry *= lambda.sqrt();
    }
    let numerator = (rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1).max(0.0);
    let mut factor = (numerator / (rx * rx * y1 * y1 + ry * ry * x1 * x1)).sqrt();
    if large == sweep {
        factor = -factor;
    }
    let cx1 = factor * rx * y1 / ry;
    let cy1 = -factor * ry * x1 / rx;
    let mid = (from.to_vec2() + to.to_vec2()) / 2.0;
    let center = Vec2::new(cos * cx1 - sin * cy1, sin * cx1 + cos * cy1) + mid;
    let angle = |ux: f32, uy: f32| uy.atan2(ux);
    let theta1 = angle((x1 - cx1) / rx, (y1 - cy1) / ry);
    let mut delta = angle((-x1 - cx1) / rx, (-y1 - cy1) / ry) - theta1;
    let tau = std::f32::consts::TAU;
    if sweep && delta < 0.0 {
        delta += tau;
    } else if !sweep && delta > 0.0 {
        delta -= tau;
    }
    let steps = ((delta.abs() / tau) * 48.0).ceil().max(2.0) as usize;
    (1..=steps)
        .map(|i| {
            let t = theta1 + delta * i as f32 / steps as f32;
            let (s, c) = t.sin_cos();
            let (ex, ey) = (rx * c, ry * s);
            Pos2::new(cos * ex - sin * ey, sin * ex + cos * ey) + center
        })
        .collect()
}

// ------------------------------------------------------ application icon

fn hex(color: u32) -> [f32; 3] {
    [((color >> 16) & 0xff) as f32, ((color >> 8) & 0xff) as f32, (color & 0xff) as f32]
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn inside_rounded(p: Pos2, rect: Rect, radius: f32) -> bool {
    if !rect.contains(p) {
        return false;
    }
    let cx = p.x.clamp(rect.left() + radius, rect.right() - radius);
    let cy = p.y.clamp(rect.top() + radius, rect.bottom() - radius);
    (p.x - cx).powi(2) + (p.y - cy).powi(2) <= radius * radius
}

/// Colour and coverage of the icon at a point on its 512-unit canvas,
/// following `assets/app.gtavi.countdown.svg` shape by shape.
fn icon_sample(p: Pos2) -> Option<[f32; 3]> {
    if !inside_rounded(p, Rect::from_min_size(Pos2::ZERO, Vec2::splat(512.0)), 112.0) {
        return None;
    }
    let t = p.y / 512.0;
    let mut color = if t < 0.55 {
        mix(hex(0x1b0b3a), hex(0x8a1f6b), t / 0.55)
    } else {
        mix(hex(0x8a1f6b), hex(0xff7a3d), (t - 0.55) / 0.45)
    };
    if (p - pos2(256.0, 330.0)).length() <= 150.0 {
        color = mix(hex(0xffe27a), hex(0xff4f8b), ((p.y - 180.0) / 300.0).clamp(0.0, 1.0));
    }
    let slit = |x: f32, y: f32, w: f32, h: f32| Rect::from_min_size(pos2(x, y), Vec2::new(w, h)).contains(p);
    if slit(96.0, 318.0, 320.0, 10.0) || slit(96.0, 352.0, 320.0, 14.0) {
        color = hex(0x8a1f6b);
    }
    if p.y >= 388.0 {
        color = hex(0x140a2c);
    }
    let bar = |x: f32, y: f32, w: f32| inside_rounded(p, Rect::from_min_size(pos2(x, y), Vec2::new(w, 8.0)), 4.0);
    if bar(196.0, 410.0, 120.0) || bar(216.0, 436.0, 80.0) || bar(236.0, 462.0, 40.0) {
        color = mix(color, hex(0xff9a5c), 0.85);
    }
    Some(color)
}

/// The application icon as straight RGBA pixels, `size` by `size`.
pub fn app_icon_rgba(size: usize) -> Vec<u8> {
    const SAMPLES: usize = 4;
    let mut pixels = Vec::with_capacity(size * size * 4);
    let unit = 512.0 / size as f32;
    for y in 0..size {
        for x in 0..size {
            let mut sum = [0.0f32; 3];
            let mut covered = 0;
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let p = pos2(
                        (x as f32 + (sx as f32 + 0.5) / SAMPLES as f32) * unit,
                        (y as f32 + (sy as f32 + 0.5) / SAMPLES as f32) * unit,
                    );
                    if let Some(c) = icon_sample(p) {
                        sum = [sum[0] + c[0], sum[1] + c[1], sum[2] + c[2]];
                        covered += 1;
                    }
                }
            }
            if covered == 0 {
                pixels.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                let n = covered as f32;
                pixels.extend_from_slice(&[
                    (sum[0] / n).round() as u8,
                    (sum[1] / n).round() as u8,
                    (sum[2] / n).round() as u8,
                    (covered * 255 / (SAMPLES * SAMPLES)) as u8,
                ]);
            }
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_icon_into_visible_strokes() {
        for icon in [
            Icon::Sliders,
            Icon::Expand,
            Icon::Compress,
            Icon::Pin,
            Icon::Calendar,
            Icon::Copy,
            Icon::Close,
            Icon::Chevron,
            Icon::Sync,
        ] {
            let paths = parse(icon.path());
            assert!(!paths.is_empty(), "{icon:?}");
            for (points, _) in paths {
                for p in points {
                    assert!((0.0..=24.0).contains(&p.x) && (0.0..=24.0).contains(&p.y), "{icon:?}: {p:?}");
                }
            }
        }
    }

    #[test]
    fn implicit_repeats_continue_the_last_command() {
        let paths = parse("M8 10l4 4 4-4");
        assert_eq!(paths, vec![(vec![pos2(8.0, 10.0), pos2(12.0, 14.0), pos2(16.0, 10.0)], false)]);
    }

    #[test]
    fn arcs_end_where_the_path_says() {
        let paths = parse("M20 11a8 8 0 0 0-14.5-4");
        let last = *paths[0].0.last().unwrap();
        assert!((last - pos2(5.5, 7.0)).length() < 1e-3);
    }

    #[test]
    fn the_app_icon_has_transparent_rounded_corners() {
        let pixels = app_icon_rgba(64);
        assert_eq!(pixels.len(), 64 * 64 * 4);
        assert_eq!(pixels[3], 0);
        let centre = (32 * 64 + 32) * 4;
        assert_eq!(pixels[centre + 3], 255);
    }
}
