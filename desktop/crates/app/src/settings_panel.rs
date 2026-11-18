//! The settings sheet, which slides in from the right over a scrim, and the
//! time zone picker inside it.

use std::sync::Arc;

use eframe::egui::{
    self, Color32, CursorIcon, Galley, Id, Order, Rect, Response, ScrollArea, Sense, Stroke, StrokeKind,
    Ui, Vec2, WidgetInfo, Role, pos2, vec2,
};
use gtavi_core::games::{self, CustomGame, Date};
use gtavi_core::settings::{EffectLevel, Settings};
use gtavi_core::themes::Theme;
use gtavi_core::{wording, zones};

use crate::icons::{self, Icon};
use crate::style::{self, Colors, Font};

/// What the user asked for in the sheet.
pub enum Action {
    Change(Settings),
    Notify(bool),
    Sync,
    Close,
}

struct ZoneEntry {
    id: String,
    city: String,
    region: String,
    offset_text: String,
    /// Lower-cased text the filter matches against.
    haystack: String,
}

/// Offsets are evaluated at the release itself, not today, so that zones
/// which change their clocks before then are listed with the right value.
fn entry(id: &str, reference: i64) -> (ZoneEntry, i64) {
    let offset = zones::offset_at(reference, id).unwrap_or(0);
    let offset_text = zones::format_offset(offset);
    let city = zones::zone_city(id);
    let region = zones::zone_region(id);
    let haystack = format!("{id} {city} {region} {offset_text}").to_lowercase().replace('_', " ");
    (ZoneEntry { id: id.to_owned(), city, region, offset_text, haystack }, offset)
}

/// The zone furthest east unlocks first, so the list doubles as the order in
/// which the release moves around the planet.
fn all_zones(reference: i64) -> Vec<ZoneEntry> {
    let mut zones: Vec<(ZoneEntry, i64)> = zones::list_time_zones().iter().map(|id| entry(id, reference)).collect();
    zones.sort_by(|(a, ao), (b, bo)| bo.cmp(ao).then_with(|| a.city.to_lowercase().cmp(&b.city.to_lowercase())));
    zones.into_iter().map(|(zone, _)| zone).collect()
}

pub struct SettingsPanel {
    pub open: bool,
    query: String,
    /// The zones with their offsets at `reference`, the selected release.
    zones: Option<(i64, Vec<ZoneEntry>)>,
    system: Option<ZoneEntry>,
    pub sync_text: String,
    pub sync_busy: bool,
    /// "Vulkan on llvmpipe", shown under the backdrop setting.
    pub renderer_text: String,
    custom: CustomForm,
}

/// The fields of the custom game form, as typed.
#[derive(Default)]
struct CustomForm {
    title: String,
    date: String,
    time: String,
    error: Option<&'static str>,
}

const SHEET_PAD: f32 = 24.0;
const ROW_HEIGHT: f32 = 46.0;
const LIST_HEIGHT: f32 = 260.0;

impl SettingsPanel {
    pub fn new() -> Self {
        SettingsPanel {
            open: false,
            query: String::new(),
            zones: None,
            system: None,
            sync_text: String::new(),
            sync_busy: false,
            renderer_text: String::new(),
            custom: CustomForm::default(),
        }
    }

    /// Opens or closes the sheet. `reference` is the selected release, at
    /// which zone offsets are listed.
    pub fn set_open(&mut self, open: bool, reference: i64, settings: &Settings) {
        if open == self.open {
            return;
        }
        self.open = open;
        if open {
            // Built when the sheet opens, not at start-up, and again only if
            // the release it describes has moved.
            if self.zones.as_ref().is_none_or(|(at, _)| *at != reference) {
                self.zones = Some((reference, all_zones(reference)));
            }
            self.system = Some(entry(&zones::system_time_zone(), reference).0);
            self.custom = match &settings.custom_game {
                Some(game) => CustomForm {
                    title: game.title.clone(),
                    date: game.date.to_string(),
                    time: format!("{:02}:{:02}", game.hour, game.minute),
                    error: None,
                },
                None => CustomForm { time: "00:00".into(), ..CustomForm::default() },
            };
        } else {
            self.query.clear();
        }
    }

    /// Whether the sheet is visible, including while it slides out.
    pub fn animating(&self, ctx: &egui::Context) -> bool {
        let t = ctx.animate_bool_with_time(Id::unique("settings-open"), self.open, 0.26);
        t > 0.0 && t < 1.0
    }

    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        screen: Rect,
        settings: &Settings,
        colors: &Colors,
        can_keep_above: bool,
        animated: bool,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        let duration = if animated { 0.26 } else { 0.0 };
        let t = ctx.animate_bool_with_time(Id::unique("settings-open"), self.open, duration);
        if t <= 0.0 {
            return actions;
        }
        let shown = style::ease_out(t);

        // The scrim covers the window; a click on it closes the sheet.
        egui::Area::new(Id::unique("settings-scrim"))
            .order(Order::Middle)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let response = ui.allocate_rect(screen, Sense::click());
                ui.painter().rect_filled(screen, 0.0, colors.scrim(0.55 * shown));
                if response.clicked() {
                    actions.push(Action::Close);
                }
            });

        let width = screen.width().clamp(280.0f32.min(screen.width()), 420.0);
        let left = screen.right() - width * shown;
        let sheet = Rect::from_min_size(pos2(left, screen.top()), vec2(width, screen.height()));
        egui::Area::new(Id::unique("settings-sheet"))
            .order(Order::Foreground)
            .fixed_pos(sheet.min)
            .constrain(false)
            .show(ctx, |ui| {
                ui.set_min_size(sheet.size());
                ui.set_max_size(sheet.size());
                let painter = ui.painter().clone();
                let shadow = egui::epaint::Shadow {
                    offset: [-24, 0],
                    blur: 60,
                    spread: 0,
                    color: Color32::from_black_alpha(89),
                };
                painter.add(shadow.as_shape(sheet, 0));
                painter.rect_filled(sheet, 0.0, colors.panel_solid);
                painter.vline(sheet.left(), sheet.y_range(), Stroke::new(1.0, style::line()));
                ui.spacing_mut().item_spacing = Vec2::ZERO;

                // Header
                let header = Rect::from_min_size(sheet.min, vec2(width, 66.0));
                let title = style::galley(&painter, "Settings", Font::sans(18.0, 700.0).width(112.0), style::INK);
                style::paint_at(&painter, pos2(header.left() + SHEET_PAD, header.center().y - title.size().y / 2.0), &title);
                painter.hline(header.x_range(), header.bottom(), Stroke::new(1.0, style::line()));
                let close = Rect::from_center_size(pos2(header.right() - 20.0 - 17.0, header.center().y), Vec2::splat(34.0));
                let close = round_button(ui, close, Icon::Close, "Close settings (Esc)", "Close settings", colors, false);
                if close.clicked() {
                    actions.push(Action::Close);
                }

                let body = Rect::from_min_max(pos2(sheet.left(), header.bottom() + 1.0), sheet.max);
                let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body));
                ScrollArea::vertical().id_salt("settings-body").auto_shrink([false, false]).show(
                    &mut body_ui,
                    |ui| {
                        ui.spacing_mut().item_spacing = Vec2::ZERO;
                        let inner = width - 2.0 * SHEET_PAD;
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.add_space(SHEET_PAD);
                            ui.vertical(|ui| {
                                ui.set_width(inner);
                                self.sections(ui, inner, settings, colors, can_keep_above, &mut actions);
                            });
                        });
                        ui.add_space(28.0);
                    },
                );
            });
        actions
    }

    fn sections(
        &mut self,
        ui: &mut Ui,
        width: f32,
        settings: &Settings,
        colors: &Colors,
        can_keep_above: bool,
        actions: &mut Vec<Action>,
    ) {
        let change = |edit: &dyn Fn(&mut Settings)| {
            let mut next = settings.clone();
            edit(&mut next);
            Action::Change(next)
        };

        section(ui, width, "GAME", false, |ui| {
            if let Some(next) = self.game_picker(ui, width, settings, colors) {
                actions.push(Action::Change(next));
            }
        });

        section(ui, width, "RELEASE TIME ZONE", false, |ui| {
            note(
                ui,
                width,
                "Games unlock at a local time, usually midnight, so the moment depends on where you are. Zones are listed in the order they reach it.",
            );
            ui.add_space(12.0);
            if let Some(zone) = self.zone_picker(ui, width, settings.time_zone.as_deref(), colors, actions) {
                actions.push(change(&|s| s.time_zone = zone.clone()));
            }
        });

        section(ui, width, "DISPLAY", false, |ui| {
            field_title(ui, "Theme");
            ui.add_space(8.0);
            let (row, _) = ui.allocate_exact_size(vec2(width, 72.0), Sense::hover());
            let w = (width - 16.0) / 3.0;
            for (i, theme) in Theme::ALL.into_iter().enumerate() {
                let rect = Rect::from_min_size(pos2(row.left() + i as f32 * (w + 8.0), row.top()), vec2(w, 72.0));
                if theme_button(ui, rect, theme, settings.theme == theme, colors).clicked() {
                    actions.push(change(&|s| s.theme = theme));
                }
            }
            ui.add_space(12.0);
            field_title(ui, "Animated backdrop");
            ui.add_space(8.0);
            let (row, _) = ui.allocate_exact_size(vec2(width, 36.0), Sense::hover());
            let count = EffectLevel::ALL.len() as f32;
            let w = (width - 6.0 * (count - 1.0)) / count;
            for (i, level) in EffectLevel::ALL.into_iter().enumerate() {
                let rect = Rect::from_min_size(pos2(row.left() + i as f32 * (w + 6.0), row.top()), vec2(w, 36.0));
                if choice_button(ui, rect, level.label(), settings.effects == level, colors).clicked() {
                    actions.push(change(&|s| s.effects = level));
                }
            }
            ui.add_space(8.0);
            note(
                ui,
                width,
                "Auto lowers the rendering resolution when the frame rate drops. High renders every pixel with more cloud detail. Off replaces the scene with a still gradient.",
            );
            if !self.renderer_text.is_empty() {
                ui.add_space(6.0);
                let text = self.renderer_text.clone();
                note(ui, width, &text);
            }
            ui.add_space(12.0);
            if let Some(value) = switch_row(
                ui,
                width,
                "Milliseconds",
                "Adds a per-frame readout after the seconds.",
                settings.show_milliseconds,
                true,
                colors,
            ) {
                actions.push(change(&|s| s.show_milliseconds = value));
            }
        });

        section(ui, width, "ALERTS", false, |ui| {
            if let Some(value) = switch_row(
                ui,
                width,
                "Notify me at release",
                "Shows a system notification when the countdown reaches zero while the app is open.",
                settings.notify,
                true,
                colors,
            ) {
                actions.push(Action::Notify(value));
            }
        });

        section(ui, width, "DESKTOP", false, |ui| {
            let hint = if can_keep_above {
                "Keeps the window above other windows."
            } else {
                "Not available here: Wayland does not let apps keep a window above others."
            };
            if let Some(value) =
                switch_row(ui, width, "Always on top", hint, settings.always_on_top, can_keep_above, colors)
            {
                actions.push(change(&|s| s.always_on_top = value));
            }
            ui.add_space(12.0);
            if let Some(value) = switch_row(
                ui,
                width,
                "Network time",
                "Corrects the system clock against an NTP server at start-up.",
                settings.network_time,
                true,
                colors,
            ) {
                actions.push(change(&|s| s.network_time = value));
            }
            ui.add_space(12.0);
            // The status text and the button share a row.
            let button_width = 112.0;
            let status = style::wrapped(
                ui.painter(),
                &self.sync_text,
                Font::sans(13.0, 400.0),
                style::ink_faint(),
                width - button_width - 12.0,
                false,
            );
            let height = status.size().y.max(36.0);
            let (row, response) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
            response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &self.sync_text));
            style::paint_at(ui.painter(), pos2(row.left(), row.center().y - status.size().y / 2.0), &status);
            let button = Rect::from_min_size(pos2(row.right() - button_width, row.center().y - 18.0), vec2(button_width, 36.0));
            let enabled = settings.network_time && !self.sync_busy;
            if flat_button(ui, button, "Sync now", Some(Icon::Sync), enabled, colors).clicked() {
                actions.push(Action::Sync);
            }
        });

        section(ui, width, "KEYBOARD SHORTCUTS", false, |ui| {
            for (i, (key, action)) in [
                ("S", "Open or close settings"),
                ("F", "Toggle fullscreen"),
                ("M", "Toggle milliseconds"),
                ("T", "Next theme"),
                ("C", "Copy the countdown"),
                ("Esc", "Close settings"),
            ]
            .into_iter()
            .enumerate()
            {
                if i > 0 {
                    ui.add_space(8.0);
                }
                let (row, _) = ui.allocate_exact_size(vec2(width, 24.0), Sense::hover());
                let painter = ui.painter();
                let key_galley = style::galley(painter, key, Font::mono(12.0, 500.0), style::INK);
                let kbd = Rect::from_min_size(row.min, vec2((key_galley.size().x + 14.0).max(26.0), 24.0));
                painter.rect_filled(kbd, 6.0, style::ink(0.06));
                painter.rect_stroke(kbd, 6.0, Stroke::new(1.0, style::line_strong()), StrokeKind::Inside);
                style::paint_centered(painter, kbd.center().x, kbd.center().y - key_galley.size().y / 2.0, &key_galley);
                let text = style::galley(painter, action, Font::sans(13.0, 400.0), style::ink_soft());
                style::paint_at(painter, pos2(row.left() + 46.0, row.center().y - text.size().y / 2.0), &text);
            }
        });

        section(ui, width, "", true, |ui| {
            let (row, _) = ui.allocate_exact_size(vec2(width, 36.0), Sense::hover());
            let button = Rect::from_min_size(row.min, vec2(150.0, 36.0));
            if flat_button(ui, button, "Restore defaults", None, true, colors).clicked() {
                actions.push(Action::Change(Settings::default()));
            }
        });
    }

    /// The filterable list of zones. Returns the chosen zone, with `None`
    /// meaning "follow this device".
    fn zone_picker(
        &mut self,
        ui: &mut Ui,
        width: f32,
        current: Option<&str>,
        colors: &Colors,
        actions: &mut Vec<Action>,
    ) -> Option<Option<String>> {
        let mut chosen = None;
        let response = text_field(ui, width, &mut self.query, "Search city, region or offset", "zone-search", colors);
        if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            actions.push(Action::Close);
        }
        ui.add_space(10.0);

        let query = self.query.to_lowercase().replace('_', " ");
        let needles: Vec<&str> = query.split_whitespace().collect();
        let zones = self.zones.as_ref().map_or(&[][..], |(_, zones)| zones.as_slice());
        // Row 0 follows the device; it is listed only while not filtering.
        let rows: Vec<Option<&ZoneEntry>> = if needles.is_empty() {
            std::iter::once(None).chain(zones.iter().map(Some)).collect()
        } else {
            zones.iter().filter(|z| needles.iter().all(|n| z.haystack.contains(n))).map(Some).collect()
        };

        let (frame, _) = ui.allocate_exact_size(vec2(width, LIST_HEIGHT), Sense::hover());
        let mut list_ui = ui.new_child(egui::UiBuilder::new().max_rect(frame.shrink(1.0)));
        list_ui.set_clip_rect(frame.shrink(1.0));
        if rows.is_empty() {
            let text = format!("No time zone matches \"{}\".", self.query.trim());
            let galley = style::wrapped(list_ui.painter(), &text, Font::sans(13.0, 400.0), style::ink_faint(), width - 24.0, false);
            style::paint_at(list_ui.painter(), frame.min + vec2(12.0, 14.0), &galley);
        } else {
            ScrollArea::vertical().id_salt("zones").auto_shrink([false, false]).show_rows(
                &mut list_ui,
                ROW_HEIGHT,
                rows.len(),
                |ui, range| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    for row in &rows[range] {
                        let (city, region, offset) = match row {
                            Some(zone) => (zone.city.as_str(), zone.region.as_str(), zone.offset_text.as_str()),
                            None => {
                                let system = self.system.as_ref();
                                ("Follow this device", system.map_or("", |s| s.city.as_str()), system.map_or("", |s| s.offset_text.as_str()))
                            }
                        };
                        let selected = match row {
                            Some(zone) => current == Some(zone.id.as_str()),
                            None => current.is_none(),
                        };
                        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
                        let response = response.on_hover_cursor(CursorIcon::PointingHand);
                        response.widget_info(|| WidgetInfo::selected(Role::ListBoxOption, true, selected, city));
                        let painter = ui.painter();
                        if selected {
                            painter.rect_filled(rect, 0.0, colors.accent_alpha(0.16));
                        } else if response.hovered() {
                            painter.rect_filled(rect, 0.0, style::ink(0.06));
                        }
                        if response.has_focus() {
                            painter.rect_stroke(rect.shrink(1.0), 0.0, Stroke::new(2.0, colors.accent), StrokeKind::Inside);
                        }
                        let offset_galley = style::galley(painter, offset, Font::mono(11.0, 400.0), style::ink_soft());
                        let text_width = rect.width() - 36.0 - offset_galley.size().x;
                        let city_galley = ellipsized(painter, city, Font::sans(14.0, 600.0), style::INK, text_width);
                        let mut y = rect.top() + 8.0;
                        if region.is_empty() {
                            y = rect.center().y - city_galley.size().y / 2.0;
                        }
                        style::paint_at(painter, pos2(rect.left() + 12.0, y), &city_galley);
                        if !region.is_empty() {
                            let region_galley = ellipsized(painter, region, Font::sans(11.0, 400.0), style::ink_faint(), text_width);
                            style::paint_at(painter, pos2(rect.left() + 12.0, y + city_galley.size().y + 1.0), &region_galley);
                        }
                        style::paint_at(
                            painter,
                            pos2(rect.right() - 12.0 - offset_galley.size().x, rect.center().y - offset_galley.size().y / 2.0),
                            &offset_galley,
                        );
                        if response.clicked() {
                            chosen = Some(row.map(|zone| zone.id.clone()));
                        }
                    }
                },
            );
        }
        ui.painter().rect_stroke(frame, 9.0, Stroke::new(1.0, style::line()), StrokeKind::Inside);
        chosen
    }
}

fn ellipsized(painter: &egui::Painter, text: &str, font: Font, color: Color32, width: f32) -> Arc<Galley> {
    let mut job = style::job(text, font, color, Some(width), false);
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    painter.layout_job(job)
}

/// A section of the sheet: a caption, its content and a divider below.
fn section(ui: &mut Ui, width: f32, caption: &str, last: bool, content: impl FnOnce(&mut Ui)) {
    ui.add_space(20.0);
    if !caption.is_empty() {
        let galley = style::galley(ui.painter(), caption, Font::caps(), style::ink_faint());
        let (rect, _) = ui.allocate_exact_size(vec2(width, galley.size().y), Sense::hover());
        style::paint_at(ui.painter(), rect.min, &galley);
        ui.add_space(12.0);
    }
    content(ui);
    ui.add_space(20.0);
    if !last {
        let (rect, _) = ui.allocate_exact_size(vec2(width, 1.0), Sense::hover());
        ui.painter().hline(rect.x_range(), rect.top(), Stroke::new(1.0, style::line()));
    }
}

fn note(ui: &mut Ui, width: f32, text: &str) {
    let galley = style::wrapped(ui.painter(), text, Font::sans(13.0, 400.0), style::ink_faint(), width, false);
    let (rect, _) = ui.allocate_exact_size(vec2(width, galley.size().y), Sense::hover());
    style::paint_at(ui.painter(), rect.min, &galley);
}

fn field_title(ui: &mut Ui, text: &str) {
    let galley = style::galley(ui.painter(), text, Font::sans(15.0, 600.0), style::INK);
    let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
    style::paint_at(ui.painter(), rect.min, &galley);
}

/// A round icon button: the close button of the sheet and the header tools.
pub fn round_button(
    ui: &mut Ui,
    rect: Rect,
    icon: Icon,
    tooltip: &str,
    name: &str,
    colors: &Colors,
    framed: bool,
) -> Response {
    let response = ui
        .interact(rect, Id::unique(("round-button", name)), Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tooltip);
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, name));
    let painter = ui.painter();
    let hovered = response.hovered();
    if framed {
        let fill = if response.is_pointer_button_down_on() {
            style::ink(0.16)
        } else if hovered {
            style::ink(0.1)
        } else {
            colors.panel
        };
        painter.circle_filled(rect.center(), rect.width() / 2.0, fill);
        let border = if hovered { style::line_strong() } else { style::line() };
        painter.circle_stroke(rect.center(), rect.width() / 2.0 - 0.5, Stroke::new(1.0, border));
    } else if hovered {
        painter.circle_filled(rect.center(), rect.width() / 2.0, style::ink(0.1));
    }
    let color = if hovered { style::INK } else { style::ink_soft() };
    icons::paint(painter, icon, rect.center(), 20.0, color);
    if response.has_focus() {
        painter.circle_stroke(rect.center(), rect.width() / 2.0 + 3.0, Stroke::new(2.0, colors.accent));
    }
    response
}

fn theme_button(ui: &mut Ui, rect: Rect, theme: Theme, selected: bool, colors: &Colors) -> Response {
    let info = theme.info();
    let response =
        ui.interact(rect, Id::unique(("theme", theme.key())), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
    response.widget_info(|| WidgetInfo::selected(Role::RadioButton, true, selected, info.label));
    let painter = ui.painter();
    chip(painter, rect, selected, response.hovered(), colors);
    let swatch = Rect::from_min_size(rect.min + vec2(10.0, 8.0), vec2(rect.width() - 20.0, 34.0));
    let [a, b, c] = info.swatch.map(style::rgb);
    // A gradient with rounded corners: flat rounded caps above and below a
    // gradient body.
    let inner = swatch.shrink2(vec2(0.0, 6.0));
    let top_cap = Rect::from_min_max(swatch.min, pos2(swatch.right(), inner.top() + 1.0));
    let bottom_cap = Rect::from_min_max(pos2(swatch.left(), inner.bottom() - 1.0), swatch.max);
    painter.rect_filled(top_cap, egui::CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 }, a);
    painter.rect_filled(bottom_cap, egui::CornerRadius { nw: 0, ne: 0, sw: 6, se: 6 }, c);
    painter.add(style::vertical_gradient(inner, &[(0.0, a), (0.58, b), (1.0, c)]));
    let label = style::galley(painter, info.label, Font::sans(14.0, 600.0), if selected { style::INK } else { style::ink_soft() });
    style::paint_centered(painter, rect.center().x, swatch.bottom() + 7.0, &label);
    if response.has_focus() {
        style::focus_ring(painter, rect, 9.0, colors.accent);
    }
    response
}

fn chip(painter: &egui::Painter, rect: Rect, selected: bool, hovered: bool, colors: &Colors) {
    let (fill, border) = if selected {
        (colors.accent_alpha(0.14), colors.accent)
    } else if hovered {
        (style::ink(0.04), style::line_strong())
    } else {
        (style::ink(0.04), style::line())
    };
    painter.rect_filled(rect, 9.0, fill);
    painter.rect_stroke(rect, 9.0, Stroke::new(1.0, border), StrokeKind::Inside);
}

fn choice_button(ui: &mut Ui, rect: Rect, label: &str, selected: bool, colors: &Colors) -> Response {
    let response =
        ui.interact(rect, Id::unique(("choice", label)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
    response.widget_info(|| WidgetInfo::selected(Role::RadioButton, true, selected, label));
    let painter = ui.painter();
    chip(painter, rect, selected, response.hovered(), colors);
    let color = if selected || response.hovered() { style::INK } else { style::ink_soft() };
    let galley = style::galley(painter, label, Font::sans(14.0, 600.0), color);
    style::paint_centered(painter, rect.center().x, rect.center().y - galley.size().y / 2.0, &galley);
    if response.has_focus() {
        style::focus_ring(painter, rect, 9.0, colors.accent);
    }
    response
}

pub fn flat_button(ui: &mut Ui, rect: Rect, label: &str, icon: Option<Icon>, enabled: bool, colors: &Colors) -> Response {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let mut response = ui.interact(rect, Id::unique(("flat", label)), sense);
    if enabled {
        response = response.on_hover_cursor(CursorIcon::PointingHand);
    }
    response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, label));
    let painter = ui.painter();
    let opacity = if enabled { 1.0 } else { 0.45 };
    let fill = if enabled && response.hovered() { style::ink(0.1) } else { style::ink(0.05) };
    painter.rect_filled(rect, 9.0, fill.gamma_multiply(opacity));
    painter.rect_stroke(rect, 9.0, Stroke::new(1.0, style::line_strong().gamma_multiply(opacity)), StrokeKind::Inside);
    let galley = style::galley(painter, label, Font::sans(14.0, 500.0), style::ink(opacity));
    let icon_width = if icon.is_some() { 21.0 } else { 0.0 };
    let left = rect.center().x - (galley.size().x + icon_width) / 2.0;
    if let Some(icon) = icon {
        icons::paint(painter, icon, pos2(left + 7.5, rect.center().y), 15.0, style::ink(opacity));
    }
    style::paint_at(painter, pos2(left + icon_width, rect.center().y - galley.size().y / 2.0), &galley);
    if response.has_focus() {
        style::focus_ring(painter, rect, 9.0, colors.accent);
    }
    response
}

/// A title and hint on the left, a switch on the right. Returns the new
/// value when the switch is flipped.
fn switch_row(
    ui: &mut Ui,
    width: f32,
    title: &str,
    hint: &str,
    value: bool,
    enabled: bool,
    colors: &Colors,
) -> Option<bool> {
    let text_width = width - 60.0;
    let painter = ui.painter().clone();
    let title_galley = style::galley(&painter, title, Font::sans(15.0, 600.0), style::INK);
    let hint_galley = style::wrapped(&painter, hint, Font::sans(13.0, 400.0), style::ink_faint(), text_width, false);
    let height = title_galley.size().y + 2.0 + hint_galley.size().y;
    let (row, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    style::paint_at(&painter, row.min, &title_galley);
    style::paint_at(&painter, row.min + vec2(0.0, title_galley.size().y + 2.0), &hint_galley);

    let track = Rect::from_min_size(pos2(row.right() - 44.0, row.center().y - 12.0), vec2(44.0, 24.0));
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let mut response = ui.interact(track.expand(4.0), Id::unique(("switch", title)), sense);
    if enabled {
        response = response.on_hover_cursor(CursorIcon::PointingHand);
    }
    response.widget_info(|| WidgetInfo::selected(Role::Switch, enabled, value, title));
    let t = ui.ctx().animate_bool_with_time(response.id, value, 0.14);
    let opacity = if enabled { 1.0 } else { 0.45 };
    let fill = style::lerp_color(style::ink(0.18), colors.accent, t).gamma_multiply(opacity);
    painter.rect_filled(track, 12.0, fill);
    let knob = pos2(track.left() + 12.0 + t * 20.0, track.center().y);
    painter.circle_filled(knob + vec2(0.0, 1.0), 9.5, Color32::from_black_alpha(60));
    painter.circle_filled(knob, 9.0, Color32::WHITE.gamma_multiply(opacity));
    if response.has_focus() {
        style::focus_ring(&painter, track, 12.0, colors.accent);
    }
    (enabled && response.clicked()).then_some(!value)
}

/// A one-line text field in the style of the sheet.
fn text_field(ui: &mut Ui, width: f32, text: &mut String, hint: &str, id: &str, colors: &Colors) -> Response {
    let (field, _) = ui.allocate_exact_size(vec2(width, 38.0), Sense::hover());
    let id = Id::unique(("text-field", id));
    let focused = ui.memory(|m| m.has_focus(id));
    // The background goes down first so that the text sits on it.
    ui.painter().rect_filled(field, 9.0, style::ink(0.06));
    let border = if focused { colors.accent } else { style::line() };
    ui.painter().rect_stroke(field, 9.0, Stroke::new(1.0, border), StrokeKind::Inside);
    let edit = egui::TextEdit::singleline(text)
        .id(id)
        .hint_text(hint)
        .frame(egui::Frame::NONE)
        .font(Font::sans(14.0, 500.0).id())
        .text_color(style::INK)
        .desired_width(width - 24.0);
    ui.put(field.shrink2(vec2(12.0, 9.0)), edit)
}

/// A selectable card with a title and a line of detail.
fn choice_card(ui: &mut Ui, width: f32, id: &str, title: &str, detail: &str, selected: bool, colors: &Colors) -> Response {
    let painter = ui.painter().clone();
    let title_galley = ellipsized(&painter, title, Font::sans(14.0, 600.0), style::INK, width - 24.0);
    let detail_galley = ellipsized(&painter, detail, Font::mono(11.0, 400.0), style::ink_faint(), width - 24.0);
    let height = 18.0 + title_galley.size().y + 2.0 + detail_galley.size().y;
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let response = ui.interact(rect, Id::unique(("game", id)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
    response.widget_info(|| WidgetInfo::selected(Role::RadioButton, true, selected, title));
    chip(&painter, rect, selected, response.hovered(), colors);
    style::paint_at(&painter, rect.min + vec2(12.0, 9.0), &title_galley);
    style::paint_at(&painter, rect.min + vec2(12.0, 11.0 + title_galley.size().y), &detail_galley);
    if response.has_focus() {
        style::focus_ring(&painter, rect, 9.0, colors.accent);
    }
    response
}

fn today() -> Date {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64);
    Date::of_instant(now)
}

impl SettingsPanel {
    /// The built-in games and the custom game form. Returns the settings to
    /// apply when the selection changes.
    fn game_picker(&mut self, ui: &mut Ui, width: f32, settings: &Settings, colors: &Colors) -> Option<Settings> {
        let mut next = None;
        for (i, game) in games::built_in().into_iter().enumerate() {
            if i > 0 {
                ui.add_space(6.0);
            }
            let mut detail = wording::short_date(game.date.noon_utc());
            if !game.platforms.is_empty() {
                detail += &format!(" · {}", game.platforms.join(", "));
            }
            let selected = settings.game == game.id;
            if choice_card(ui, width, &game.id, &game.title, &detail, selected, colors).clicked() && !selected {
                next = Some(Settings { game: game.id.clone(), ..settings.clone() });
            }
        }
        ui.add_space(6.0);
        let custom_selected = settings.game == games::CUSTOM;
        let detail = match &settings.custom_game {
            Some(game) => format!("{} · {}", game.title, wording::short_date(game.date.noon_utc())),
            None => "Any game, with your own date and hour".to_owned(),
        };
        let card = choice_card(ui, width, games::CUSTOM, "Custom game", &detail, custom_selected, colors);
        if card.clicked() && !custom_selected && settings.custom_game.is_some() {
            next = Some(Settings { game: games::CUSTOM.into(), ..settings.clone() });
        }

        // The form: title, then date and time side by side.
        ui.add_space(12.0);
        field_title(ui, "Custom game");
        ui.add_space(8.0);
        text_field(ui, width, &mut self.custom.title, "Title", "custom-title", colors);
        ui.add_space(8.0);
        let half = (width - 8.0) / 2.0;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            ui.vertical(|ui| text_field(ui, half, &mut self.custom.date, "Date, YYYY-MM-DD", "custom-date", colors));
            ui.add_space(8.0);
            ui.vertical(|ui| text_field(ui, half, &mut self.custom.time, "Unlock time, HH:MM", "custom-time", colors));
        });
        ui.add_space(8.0);
        if let Some(error) = self.custom.error {
            let galley = style::wrapped(ui.painter(), error, Font::sans(13.0, 400.0), colors.accent, width, false);
            let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
            response.widget_info(|| WidgetInfo::labeled(Role::Label, true, error));
            style::paint_at(ui.painter(), rect.min, &galley);
            ui.add_space(8.0);
        }
        note(ui, width, "The countdown runs to this date and time in the selected time zone.");
        ui.add_space(10.0);
        let (row, _) = ui.allocate_exact_size(vec2(width, 36.0), Sense::hover());
        let button = Rect::from_min_size(row.min, vec2(170.0, 36.0));
        if flat_button(ui, button, "Count down to it", None, true, colors).clicked() {
            match self.parse_custom(settings) {
                Ok(game) => {
                    self.custom.error = None;
                    next = Some(Settings { game: games::CUSTOM.into(), custom_game: Some(game), ..settings.clone() });
                }
                Err(error) => self.custom.error = Some(error),
            }
        }
        next
    }

    fn parse_custom(&self, settings: &Settings) -> Result<CustomGame, &'static str> {
        let title = self.custom.title.trim();
        if title.is_empty() {
            return Err("Enter the game's title.");
        }
        if title.chars().count() > games::MAX_TITLE_CHARS {
            return Err("The title is longer than 60 characters.");
        }
        let date = Date::parse(self.custom.date.trim()).ok_or("Enter the date as YYYY-MM-DD, for example 2027-03-01.")?;
        let (hour, minute) = games::parse_time(self.custom.time.trim()).ok_or("Enter the time as HH:MM, for example 00:00.")?;
        // An edited game keeps the day it was first added.
        let added = settings.custom_game.as_ref().map_or_else(today, |g| g.added);
        CustomGame::new(title, date, hour, minute, added.min(date)).ok_or("The title contains characters that cannot be used.")
    }
}
