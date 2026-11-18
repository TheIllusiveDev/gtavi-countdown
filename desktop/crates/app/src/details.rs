//! The lower panel: the timeline of the wait since the first trailer and the
//! remaining time in other units.

use eframe::egui::{self, Align2, Color32, Id, Painter, Rect, Sense, Stroke, Ui, pos2, vec2};
use eframe::epaint::text::LayoutJob;
use gtavi_core::countdown::{self, Totals};
use gtavi_core::games::Game;
use gtavi_core::wording;

use crate::style::{self, Colors, Font};

const MARK_RADIUS: f32 = 5.0;
const TRACK_HEIGHT: f32 = 6.0;
const LEGEND_GAP: f32 = 10.0;
const LEGEND_HEIGHT: f32 = 38.0;
const LINE_HEIGHT: f32 = 1.3;
const PAD: [f32; 4] = [15.0, 18.0, 12.0, 18.0];

// --------------------------------------------------------------- timeline

pub struct Timeline {
    pub progress: f64,
    marks: Vec<Mark>,
    release_date: String,
    caption: String,
}

struct Mark {
    title: String,
    detail: String,
    date: String,
    position: f32,
}

impl Timeline {
    pub fn new() -> Self {
        Timeline { progress: 0.0, marks: Vec::new(), release_date: String::new(), caption: String::new() }
    }

    pub fn set_data(&mut self, game: &Game, now: i64, target: i64) {
        self.progress = countdown::wait_progress(game, now, target);
        self.release_date = wording::short_date(game.date.noon_utc());
        self.caption = game.wait_caption();
        self.marks = game
            .milestones
            .iter()
            .map(|m| {
                let instant = m.date.noon_utc();
                Mark {
                    title: m.title.clone(),
                    detail: m.detail.clone(),
                    date: wording::short_date(instant),
                    position: countdown::timeline_position(game, instant, target) as f32,
                }
            })
            .collect();
    }

    fn header(&self, painter: &Painter) -> (std::sync::Arc<egui::Galley>, std::sync::Arc<egui::Galley>) {
        let caption = style::galley(painter, &self.caption, Font::caps(), style::ink_faint());
        // Two decimals while counting; the bar itself moves about 0.1% per day.
        let value = if self.progress >= 1.0 {
            "100".to_owned()
        } else {
            format!("{:.2}", (self.progress * 10_000.0).floor() / 100.0)
        };
        let mut job = LayoutJob::default();
        job.append(&value, 0.0, Font::mono(12.0, 600.0).format(style::INK));
        job.append("% over", 0.0, Font::mono(12.0, 400.0).format(style::ink_soft()));
        (caption, painter.layout_job(job))
    }

    pub fn height(&self, painter: &Painter) -> f32 {
        let (caption, percent) = self.header(painter);
        PAD[0] + caption.size().y.max(percent.size().y) + 10.0 + 2.0 * MARK_RADIUS + LEGEND_GAP + LEGEND_HEIGHT + PAD[2]
    }

    pub fn paint(&self, ui: &Ui, rect: Rect, colors: &Colors, compact: bool) {
        let painter = ui.painter();
        let inner = Rect::from_min_max(rect.min + vec2(PAD[3], PAD[0]), rect.max - vec2(PAD[1], PAD[2]));
        let (caption, percent) = self.header(painter);
        let header_height = caption.size().y.max(percent.size().y);
        style::paint_at(painter, pos2(inner.left(), inner.top() + (header_height - caption.size().y) / 2.0), &caption);
        style::paint_at(
            painter,
            pos2(inner.right() - percent.size().x, inner.top() + (header_height - percent.size().y) / 2.0),
            &percent,
        );

        let width = inner.width();
        let cy = inner.top() + header_height + 10.0 + MARK_RADIUS;
        let left = inner.left();
        let scrim = colors.scrim(0.55);

        // Track and fill. The fill is the full gradient compressed into the
        // share that is over, as the scaled element of the web version is.
        let track = Rect::from_min_size(pos2(left, cy - TRACK_HEIGHT / 2.0), vec2(width, TRACK_HEIGHT));
        painter.rect_filled(track, TRACK_HEIGHT / 2.0, style::ink(0.16));
        let fill_width = width * self.progress.clamp(0.0, 1.0) as f32;
        if fill_width > 0.0 {
            let r = TRACK_HEIGHT / 2.0;
            if fill_width > 2.0 * r {
                let body = Rect::from_min_max(pos2(left + r, track.top()), pos2(left + fill_width - r, track.bottom()));
                painter.add(style::horizontal_gradient(body, colors.accent_2, colors.accent));
            }
            painter.circle_filled(pos2(left + r, cy), r, colors.accent_2);
            painter.circle_filled(pos2(left + (fill_width - r).max(r), cy), r, colors.accent);
        }

        let mark_x = |i: usize| {
            let x = left + self.marks[i].position * width;
            if i == 0 { x + MARK_RADIUS } else { x }
        };
        for i in 0..self.marks.len() {
            let center = pos2(mark_x(i), cy);
            painter.circle_filled(center, MARK_RADIUS + 3.0, scrim);
            painter.circle_filled(center, MARK_RADIUS, style::INK);
            // Each dot explains itself on hover.
            let hit = Rect::from_center_size(center, vec2(2.0 * MARK_RADIUS + 6.0, 2.0 * MARK_RADIUS + 6.0));
            let mark = &self.marks[i];
            ui.interact(hit, Id::unique(("milestone", i)), Sense::hover())
                .on_hover_text(format!("{}: {}", mark.date, mark.detail));
        }
        // The release: a hollow ring that fills once it is reached.
        let end = pos2(left + width - MARK_RADIUS, cy);
        painter.circle_filled(end, MARK_RADIUS + 3.0, scrim);
        painter.circle_filled(end, MARK_RADIUS, colors.accent);
        if self.progress < 1.0 {
            painter.circle_filled(end, MARK_RADIUS - 2.0, colors.panel_solid);
        }

        // Legend: centred under the dots, the first aligned left and the release right.
        let top = cy + MARK_RADIUS + LEGEND_GAP;
        // Each label is placed only where it does not run into one already
        // placed: the release first, then the origin, then the others. Hidden
        // labels keep their dots and tooltips.
        let mut placed: Vec<(f32, f32)> = Vec::new();
        let mut entry = |title: &str, date: &str, x: f32, align: Align2| {
            let title = style::galley(painter, title, Font::sans(12.0, 600.0), style::INK);
            let date = style::galley(painter, date, Font::mono(11.0, 400.0), style::ink_faint());
            let w = title.size().x.max(date.size().x);
            let place = |w: f32| match align {
                Align2::LEFT_TOP => x,
                Align2::RIGHT_TOP => x - w,
                _ => x - w / 2.0,
            };
            let span = (place(w), place(w) + w);
            if placed.iter().any(|(a, b)| span.0 < b + 12.0 && span.1 + 12.0 > *a) {
                return;
            }
            placed.push(span);
            let title_line = 12.0 * LINE_HEIGHT;
            style::paint_at(painter, pos2(place(title.size().x), top + (title_line - title.size().y) / 2.0), &title);
            style::paint_at(
                painter,
                pos2(place(date.size().x), top + title_line + (11.0 * LINE_HEIGHT - date.size().y) / 2.0),
                &date,
            );
        };
        entry("Release", &self.release_date, left + width, Align2::RIGHT_TOP);
        for (i, mark) in self.marks.iter().enumerate() {
            // The labels would collide on narrow windows; the dots keep their
            // tooltips and the first and last labels anchor the scale.
            if i > 0 && compact {
                continue;
            }
            let align = if i == 0 { Align2::LEFT_TOP } else { Align2::CENTER_TOP };
            entry(&mark.title, &mark.date, left + mark.position * width, align);
        }
    }
}

// ------------------------------------------------------------------ stats

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StatsLayout {
    /// Five tiles in a row.
    Wide,
    /// Two tiles over three.
    Medium,
    /// Two by two, with total seconds across the last row.
    Narrow,
}

impl StatsLayout {
    /// Column, row and span of each tile, on a grid of `columns()` columns.
    fn cells(self) -> [(usize, usize, usize); 5] {
        match self {
            StatsLayout::Wide => [(0, 0, 1), (1, 0, 1), (2, 0, 1), (3, 0, 1), (4, 0, 1)],
            StatsLayout::Medium => [(0, 0, 3), (3, 0, 3), (0, 1, 2), (2, 1, 2), (4, 1, 2)],
            StatsLayout::Narrow => [(0, 0, 1), (1, 0, 1), (0, 1, 1), (1, 1, 1), (0, 2, 2)],
        }
    }

    fn columns(self) -> usize {
        match self {
            StatsLayout::Wide => 5,
            StatsLayout::Medium => 6,
            StatsLayout::Narrow => 2,
        }
    }

    fn rows(self) -> usize {
        match self {
            StatsLayout::Wide => 1,
            StatsLayout::Medium => 2,
            StatsLayout::Narrow => 3,
        }
    }
}

fn tiles(totals: &Totals) -> [(&'static str, i64); 5] {
    [
        (if totals.sleeps == 1 { "SLEEP" } else { "SLEEPS" }, totals.sleeps),
        (if totals.weekends == 1 { "WEEKEND" } else { "WEEKENDS" }, totals.weekends),
        ("TOTAL HOURS", totals.hours),
        ("TOTAL MINUTES", totals.minutes),
        ("TOTAL SECONDS", totals.seconds),
    ]
}

fn value_font(size: f32) -> Font {
    Font::sans(size, 700.0).width(88.0)
}

fn tile_height(painter: &Painter, value_size: f32) -> f32 {
    let value = style::galley(painter, "0", value_font(value_size), style::INK);
    let caption = style::galley(painter, "S", Font::caps(), style::INK);
    13.0 + value.size().y + caption.size().y + 13.0
}

pub fn stats_height(painter: &Painter, layout: StatsLayout, value_size: f32) -> f32 {
    tile_height(painter, value_size) * layout.rows() as f32
}

pub fn paint_stats(painter: &Painter, rect: Rect, totals: &Totals, layout: StatsLayout, value_size: f32) {
    painter.hline(rect.x_range(), rect.top(), Stroke::new(1.0, style::line()));
    let row_height = tile_height(painter, value_size);
    let column = rect.width() / layout.columns() as f32;
    for ((caption, value), (col, row, span)) in tiles(totals).into_iter().zip(layout.cells()) {
        let tile = Rect::from_min_size(
            pos2(rect.left() + col as f32 * column, rect.top() + row as f32 * row_height),
            vec2(column * span as f32, row_height),
        );
        // Hairline dividers on the inner edges only.
        if col > 0 {
            painter.vline(tile.left(), tile.y_range(), Stroke::new(1.0, style::line()));
        }
        if row > 0 {
            painter.hline(tile.x_range(), tile.top(), Stroke::new(1.0, style::line()));
        }
        let value = style::galley(painter, &wording::integer(value), value_font(value_size), style::INK);
        let caption_galley = painter.layout_job({
            let mut job = style::job(caption, Font::caps(), style::ink_faint(), Some(tile.width() - 36.0), false);
            job.wrap.max_rows = 1;
            job.wrap.break_anywhere = true;
            job
        });
        let x = tile.left() + 18.0;
        style::paint_at(painter, pos2(x, tile.top() + 13.0), &value);
        style::paint_at(painter, pos2(x, tile.top() + 13.0 + value.size().y), &caption_galley);
    }
}

/// The translucent panel the timeline and the totals sit on.
pub fn paint_panel(painter: &Painter, rect: Rect, colors: &Colors) {
    let shadow = egui::epaint::Shadow { offset: [0, 24], blur: 60, spread: 0, color: Color32::from_black_alpha(71) };
    painter.add(shadow.as_shape(rect, 14));
    painter.rect_filled(rect, 14.0, colors.panel);
    // A faint sheen along the top edge.
    let sheen = Rect::from_min_size(rect.min + vec2(14.0, 1.0), vec2(rect.width() - 28.0, 120.0f32.min(rect.height() - 2.0)));
    painter.add(style::vertical_gradient(sheen, &[(0.0, style::ink(0.05)), (1.0, style::ink(0.0))]));
    painter.hline(
        (rect.left() + 14.0)..=(rect.right() - 14.0),
        rect.top() + 1.0,
        Stroke::new(1.0, style::ink(0.06)),
    );
    painter.rect_stroke(rect, 14.0, Stroke::new(1.0, style::line()), egui::StrokeKind::Inside);
}
