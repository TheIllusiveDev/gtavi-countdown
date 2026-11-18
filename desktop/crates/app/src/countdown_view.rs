//! The four units of the countdown, the colons between them and the optional
//! milliseconds readout. Figures roll when they change: the old glyph slides
//! down and fades while the new one slides in from above.

use std::time::{Duration, Instant};

use eframe::egui::{self, Painter, Pos2, Rect, pos2, vec2};
use gtavi_core::countdown::Breakdown;

use crate::style::{self, Colors, Font};

const ROLL: Duration = Duration::from_millis(460);
/// How far a glyph travels while rolling, as a fraction of the figure height.
const ROLL_DISTANCE: f32 = 0.58;
/// Colons breathe once per two seconds.
const PULSE_SECONDS: f32 = 2.0;

#[derive(Clone, Copy)]
struct Digit {
    value: char,
    previous: Option<char>,
    changed: Instant,
}

#[derive(Default)]
struct Unit {
    /// `digits[0]` is the rightmost figure.
    digits: Vec<Digit>,
    label: &'static str,
}

/// Sizes of the drawing for a figure height.
pub struct Geometry {
    pub size: f32,
    gap: f32,
    label_size: f32,
    label_gap: f32,
    separator: f32,
    label_height: f32,
    unit_widths: [f32; 4],
    pub width: f32,
    pub height: f32,
}

pub struct CountdownView {
    units: [Unit; 4],
    have_parts: bool,
    pub spoken: String,
}

fn figure_font(size: f32) -> Font {
    Font::sans(size, 800.0).width(62.0)
}

fn label_font(size: f32) -> Font {
    Font::mono(size, 500.0).spacing(size * 0.16)
}

impl CountdownView {
    pub fn new() -> Self {
        let mut view = CountdownView { units: Default::default(), have_parts: false, spoken: String::new() };
        for (unit, label) in view.units.iter_mut().zip(["Days", "Hours", "Minutes", "Seconds"]) {
            unit.label = label;
        }
        view
    }

    /// Takes the reading of a new tick. Digits are keyed by their position
    /// from the right, so a change in the number of digits, from 100 days to
    /// 99, does not animate the others.
    pub fn set_parts(&mut self, parts: &Breakdown, animate: bool) {
        let values = [parts.days, parts.hours, parts.minutes, parts.seconds];
        let singular = ["Day", "Hour", "Minute", "Second"];
        let plural = ["Days", "Hours", "Minutes", "Seconds"];
        let now = Instant::now();
        let animate = animate && self.have_parts;
        for (i, unit) in self.units.iter_mut().enumerate() {
            let text = format!("{:02}", values[i]);
            unit.label = if values[i] == 1 { singular[i] } else { plural[i] };
            let figures: Vec<char> = text.chars().rev().collect();
            unit.digits.truncate(figures.len());
            for (position, value) in figures.into_iter().enumerate() {
                match unit.digits.get_mut(position) {
                    Some(digit) if digit.value != value => {
                        digit.previous = animate.then_some(digit.value);
                        digit.value = value;
                        digit.changed = now;
                    }
                    Some(_) => {}
                    None => unit.digits.push(Digit { value, previous: None, changed: now }),
                }
            }
        }
        self.have_parts = true;
        // The animated figures are hidden from assistive technology; this is what is read out.
        self.spoken = format!(
            "{} days, {} hours, {} minutes{}",
            parts.days,
            parts.hours,
            parts.minutes,
            if parts.elapsed { " since release" } else { " remaining" }
        );
    }

    /// Whether a figure is still rolling.
    pub fn rolling(&self) -> bool {
        self.units
            .iter()
            .flat_map(|u| &u.digits)
            .any(|d| d.previous.is_some() && d.changed.elapsed() < ROLL)
    }

    pub fn geometry(&self, painter: &Painter, size: f32, gap: f32) -> Geometry {
        let label_size = (size * 0.085).clamp(10.0, 14.4);
        let mut g = Geometry {
            size,
            gap: size * gap,
            label_size,
            label_gap: size * 0.06,
            separator: size * 0.09,
            label_height: 0.0,
            unit_widths: [0.0; 4],
            width: 0.0,
            height: 0.0,
        };
        let font = label_font(label_size);
        for (i, unit) in self.units.iter().enumerate() {
            let label = style::galley(painter, &unit.label.to_uppercase(), font, style::INK);
            g.label_height = g.label_height.max(label.size().y);
            let digits = unit.digits.len().max(2) as f32 * size * 0.5;
            g.unit_widths[i] = digits.max(style::visible_width(&label, font));
            g.width += g.unit_widths[i];
        }
        g.width += 3.0 * g.separator + 6.0 * g.gap;
        g.height = size + g.label_gap + g.label_height;
        g
    }

    /// Draws the countdown with its top centre at `top_center`.
    #[allow(clippy::too_many_arguments)]
    pub fn paint(
        &self,
        painter: &Painter,
        g: &Geometry,
        top_center: Pos2,
        colors: &Colors,
        animated: bool,
        clock_seconds: f32,
        millis: Option<i64>,
    ) {
        let size = g.size;
        let mut x = top_center.x - g.width / 2.0;
        let top = top_center.y;
        let font = figure_font(size);

        // The wide shadow of the web version: a soft dark cloud behind the figures.
        let figures = Rect::from_min_size(pos2(x, top + size * 0.12), vec2(g.width, size * 0.76));
        style::soft_shadow(painter, figures.shrink2(vec2(size * 0.2, 0.0)), colors.scrim(0.38), size * 0.9);

        let pulse = if animated {
            let phase = (clock_seconds % PULSE_SECONDS) / (PULSE_SECONDS / 2.0);
            let t = if phase < 1.0 { phase } else { 2.0 - phase };
            0.5 - 0.28 * style::ease_in_out(t)
        } else {
            0.5
        };
        let tight = colors.scrim(0.55);
        let glyph = |value: char, cell_left: f32, offset: f32, opacity: f32| {
            if opacity <= 0.0 {
                return;
            }
            let galley = style::galley(painter, &value.to_string(), font, style::INK);
            let pos = pos2(
                cell_left + (size * 0.5 - galley.size().x) / 2.0,
                top + (size - galley.size().y) / 2.0 + offset,
            );
            // A tight shadow for edge contrast.
            let shadow = egui::Shape::galley_with_override_text_color(
                pos + vec2(0.0, size * 0.02),
                galley.clone(),
                tight.gamma_multiply(opacity),
            );
            painter.add(shadow);
            painter.add(egui::Shape::galley_with_override_text_color(pos, galley, style::ink(opacity)));
        };

        let mut seconds_right = 0.0;
        for (i, unit) in self.units.iter().enumerate() {
            if i > 0 {
                // The colon is drawn rather than typeset: two round dots,
                // centred on the height of the figures.
                x += g.gap;
                let r = g.separator / 2.0;
                let dot = style::ink(pulse);
                painter.circle_filled(pos2(x + r, top + size * 0.34), r, dot);
                painter.circle_filled(pos2(x + r, top + size * 0.68), r, dot);
                x += g.separator + g.gap;
            }
            let cell = size * 0.5;
            let digits_width = unit.digits.len() as f32 * cell;
            let left = x + (g.unit_widths[i] - digits_width) / 2.0;
            for (position, digit) in unit.digits.iter().rev().enumerate() {
                let cx = left + position as f32 * cell;
                let progress = match digit.previous {
                    Some(_) => digit.changed.elapsed().as_secs_f32() / ROLL.as_secs_f32(),
                    None => 1.0,
                };
                if progress >= 1.0 {
                    glyph(digit.value, cx, 0.0, 1.0);
                } else {
                    let e = style::ease_out(progress);
                    if let Some(previous) = digit.previous {
                        glyph(previous, cx, size * ROLL_DISTANCE * e, 1.0 - e);
                    }
                    glyph(digit.value, cx, -size * ROLL_DISTANCE * (1.0 - e), e);
                }
            }
            if i == 3 {
                seconds_right = left + digits_width;
            }
            x += g.unit_widths[i];
        }

        // Unit labels, centred under their figures.
        let mut lx = top_center.x - g.width / 2.0;
        let lfont = label_font(g.label_size);
        for (i, unit) in self.units.iter().enumerate() {
            if i > 0 {
                lx += 2.0 * g.gap + g.separator;
            }
            let galley = style::galley(painter, &unit.label.to_uppercase(), lfont, style::ink_soft());
            let width = style::visible_width(&galley, lfont);
            let pos = pos2(lx + (g.unit_widths[i] - width) / 2.0, top + size + g.label_gap);
            style::paint_shadowed(painter, pos, &galley, colors.scrim(0.5), 8.0);
            lx += g.unit_widths[i];
        }

        // Milliseconds: hung to the right of the seconds, on the baseline of
        // the figures and out of the flow, so that toggling it never moves the digits.
        if let Some(ms) = millis {
            let m = size * 0.26;
            let galley = style::galley(painter, &format!(".{ms:03}"), Font::mono(m, 600.0), colors.accent);
            let box_top = top + size - 0.52 * m - m;
            let pos = pos2(seconds_right + 0.12 * m, box_top + (m - galley.size().y) / 2.0);
            painter.add(egui::Shape::galley_with_override_text_color(
                pos + vec2(0.0, size * 0.02),
                galley.clone(),
                tight,
            ));
            painter.galley(pos, galley, colors.accent);
        }
    }
}
