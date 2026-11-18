//! The main window: layout and layout tiers, wording, actions, keyboard
//! shortcuts, settings changes and network time. It plays the part of
//! `App.tsx` in the web version.

use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align2, Color32, CursorIcon, Galley, Id, Key, Order, Rect, Response, ScrollArea, Sense, Stroke, StrokeKind,
    TextureHandle, Ui, Vec2, ViewportCommand, WidgetInfo, Role, pos2, vec2,
};
use eframe::epaint::text::LayoutJob;
use gtavi_core::countdown::{self, Breakdown, Totals};
use gtavi_core::ics::{self, CalendarEvent};
use gtavi_core::settings::{self, EffectLevel, Settings};
use gtavi_core::games::{self, Game};
use gtavi_core::{wording, zones};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::backdrop::{self, Backdrop, Composition};
use crate::clock::Clock;
use crate::countdown_view::CountdownView;
use crate::details::{self, StatsLayout, Timeline};
use crate::icons::{self, Icon};
use crate::platform::{self, TimeSync, WindowState};
use crate::settings_panel::{self, SettingsPanel};
use crate::style::{self, Colors, Font};

/// How often the clock is measured again while network time is on.
const RESYNC_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const TOAST: Duration = Duration::from_millis(3600);
const TOAST_FADE: f32 = 0.2;
/// egui redraws the whole window on every frame, so the breathing colons,
/// a two-second fade, are drawn at about 15 frames per second when nothing
/// else moves.
const COLON_FRAME: Duration = Duration::from_millis(66);

enum Sync {
    Idle,
    Syncing { previous: Box<Sync> },
    Ok(TimeSync),
    Error(String),
}

/// Things the interface asked for during a frame, applied after drawing.
enum Request {
    Settings(Settings),
    ToggleSettings,
    CloseSettings,
    OpenSettings,
    Notify(bool),
    Sync,
    Copy,
    Calendar,
    Fullscreen,
}

pub struct App {
    settings: Settings,
    game: Game,
    zone: String,
    target: i64,
    clock: Clock,
    parts: Breakdown,
    was_elapsed: bool,
    totals: Totals,
    countdown: CountdownView,
    timeline: Timeline,
    backdrop: Backdrop,
    release_backdrop: bool,
    panel: SettingsPanel,
    /// False when the desktop asks for reduced motion.
    animated: bool,
    can_keep_above: Option<bool>,
    sync: Sync,
    sync_result: Option<Receiver<Result<TimeSync, String>>>,
    last_sync: Option<Instant>,
    toast: Option<(String, Instant)>,
    title: String,
    local_text: String,
    brand: TextureHandle,
    focused: bool,
    /// The window's size and position while neither maximised nor fullscreen.
    window: Option<WindowState>,
    started: Instant,
    screenshot: Option<crate::screenshot::Screenshot>,
}

fn upper(text: &str) -> String {
    text.to_uppercase()
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        style::install_fonts(&cc.egui_ctx);
        let settings = settings::sanitize(&platform::load_settings().unwrap_or_default());
        let animated = !platform::reduced_motion();
        let icon = icons::app_icon_rgba(64);
        let brand = cc.egui_ctx.load_texture(
            "brand",
            egui::ColorImage::from_rgba_unmultiplied([64, 64], &icon),
            egui::TextureOptions::LINEAR,
        );
        let (game, zone, target) = game_zone_and_target(&settings);
        let now = Clock::new(target).now();
        let mut app = App {
            backdrop: Backdrop::new(cc.wgpu_render_state.as_ref(), animated, settings.effects),
            release_backdrop: false,
            game,
            zone,
            target,
            clock: Clock::new(target),
            parts: countdown::breakdown(now, target),
            was_elapsed: countdown::breakdown(now, target).elapsed,
            totals: Totals { weeks: 0.0, hours: 0, minutes: 0, seconds: 0, sleeps: 0, weekends: 0 },
            countdown: CountdownView::new(),
            timeline: Timeline::new(),
            panel: SettingsPanel::new(),
            animated,
            can_keep_above: None,
            sync: Sync::Idle,
            sync_result: None,
            last_sync: None,
            toast: None,
            title: String::new(),
            local_text: String::new(),
            brand,
            focused: true,
            window: platform::load_window_state(),
            started: Instant::now(),
            screenshot: crate::screenshot::Screenshot::from_env(),
            settings,
        };
        if let Some(state) = &cc.wgpu_render_state {
            let info = state.adapter.get_info();
            app.panel.renderer_text = format!("Rendering with {:?} on {}.", info.backend, info.name);
            if app.screenshot.is_some() {
                eprintln!("{}", app.panel.renderer_text);
            }
        } else {
            app.panel.renderer_text = "No GPU renderer is available; the still gradient is shown.".to_owned();
        }
        app.apply_theme(&cc.egui_ctx);
        app.refresh_zone_text();
        app.refresh_sync();
        if app.settings.network_time {
            app.run_sync(&cc.egui_ctx);
        }
        app.on_tick(&cc.egui_ctx);
        app
    }

    fn colors(&self) -> Colors {
        Colors::of(self.settings.theme)
    }

    // ------------------------------------------------------------ wording

    fn target_date(&self) -> String {
        wording::long_date(self.target, &self.zone).unwrap_or_default()
    }

    fn target_time(&self) -> String {
        wording::time(self.target, &self.zone).unwrap_or_default()
    }

    fn zone_text(&self) -> String {
        wording::zone_text(&self.zone, self.target).unwrap_or_default()
    }

    /// When the chosen zone differs from the device's, the same instant is
    /// also shown on the device's own clock, which is what the viewer will be reading.
    fn refresh_zone_text(&mut self) {
        let system = zones::system_time_zone();
        let same = zones::offset_at(self.target, &system).ok() == zones::offset_at(self.target, &self.zone).ok();
        self.local_text = if same {
            String::new()
        } else {
            format!("{} on this device", wording::short_date_time(self.target, &system).unwrap_or_default())
        };
    }

    fn refresh_sync(&mut self) {
        let (text, busy) = match &self.sync {
            Sync::Syncing { .. } => ("Measuring the clock offset.".to_owned(), true),
            Sync::Ok(result) => (sync_ok_text(result), false),
            Sync::Error(message) => (format!("Using the system clock. {message}"), false),
            Sync::Idle => ("Using the system clock.".to_owned(), false),
        };
        self.panel.sync_text = text;
        self.panel.sync_busy = busy;
    }

    fn clock_source(&self) -> &'static str {
        match self.sync {
            Sync::Syncing { .. } => "SYNCING CLOCK",
            Sync::Ok(_) => "NETWORK TIME",
            Sync::Error(_) | Sync::Idle => "SYSTEM CLOCK",
        }
    }

    // -------------------------------------------------------------- state

    fn apply_theme(&self, ctx: &egui::Context) {
        let colors = self.colors();
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = colors.base;
        visuals.window_fill = colors.panel_solid;
        visuals.window_stroke = Stroke::new(1.0, style::line_strong());
        visuals.window_corner_radius = 8.into();
        visuals.window_shadow = egui::epaint::Shadow::NONE;
        visuals.popup_shadow = egui::epaint::Shadow::NONE;
        visuals.override_text_color = Some(style::INK);
        visuals.selection.bg_fill = colors.accent_2;
        visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
        visuals.text_cursor.stroke = Stroke::new(2.0, colors.accent);
        visuals.extreme_bg_color = Color32::TRANSPARENT;
        ctx.set_visuals(visuals);
        ctx.global_style_mut(|style| {
            style.spacing.item_spacing = Vec2::ZERO;
            style.spacing.scroll.bar_width = 4.0;
            style.spacing.scroll.floating = true;
        });
    }

    fn update_settings(&mut self, ctx: &egui::Context, next: Settings) {
        if next == self.settings {
            return;
        }
        let previous = std::mem::replace(&mut self.settings, next.clone());
        platform::save_settings(&settings::serialize(&next));
        if next.theme != previous.theme {
            self.apply_theme(ctx);
        }
        if next.effects != previous.effects {
            self.backdrop.set_effects(next.effects);
            self.release_backdrop = next.effects == EffectLevel::Off;
        }
        if next.time_zone != previous.time_zone || next.game != previous.game || next.custom_game != previous.custom_game {
            (self.game, self.zone, self.target) = game_zone_and_target(&next);
            self.refresh_zone_text();
            // Re-phase the tick, since the target instant has moved. Switching
            // to a game that is already out must not announce its release.
            self.clock.retarget(self.target);
            self.was_elapsed = countdown::breakdown(self.clock.now(), self.target).elapsed;
        }
        if next.always_on_top != previous.always_on_top {
            set_always_on_top(ctx, next.always_on_top);
        }
        if next.network_time != previous.network_time {
            if next.network_time {
                self.run_sync(ctx);
            } else {
                self.clock.set_offset(0.0);
                self.sync = Sync::Idle;
                self.sync_result = None;
            }
        }
        self.refresh_sync();
        self.on_tick(ctx);
    }

    fn on_tick(&mut self, ctx: &egui::Context) {
        let now = self.clock.now();
        let parts = countdown::breakdown(now, self.target);
        // Fire the release notification on the tick that crosses zero.
        // Opening the app after release must not notify, hence the comparison
        // with the previous state rather than a check of the current one.
        if !self.was_elapsed && parts.elapsed {
            self.show_toast(format!("{} is out now.", self.game.title));
            if self.settings.notify {
                platform::notify(&format!("{} is out now", self.game.title), "The wait is over.");
            }
        }
        self.was_elapsed = parts.elapsed;

        // The window title follows the countdown at minute resolution.
        let title = if parts.elapsed {
            format!("{} is out", self.game.short_title)
        } else {
            format!("{}d {:02}h {:02}m | {} Countdown", parts.days, parts.hours, parts.minutes, self.game.short_title)
        };
        if title != self.title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        self.parts = parts;
        self.countdown.set_parts(&parts, self.animated);
        self.timeline.set_data(&self.game, now, self.target);
        if !parts.elapsed {
            self.totals = countdown::totals(now, self.target, &self.zone).unwrap_or(self.totals);
        }
    }

    // ------------------------------------------------------- network time

    /// Measures the clock on a background thread. A failed refresh keeps the
    /// last good correction rather than discarding it; only a first failure
    /// falls back to the system clock.
    fn run_sync(&mut self, ctx: &egui::Context) {
        if !self.settings.network_time || matches!(self.sync, Sync::Syncing { .. }) {
            return;
        }
        let previous = std::mem::replace(&mut self.sync, Sync::Idle);
        self.sync = Sync::Syncing { previous: Box::new(previous) };
        self.last_sync = Some(Instant::now());
        let (sender, receiver) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = sender.send(platform::sync_network_time());
            ctx.request_repaint();
        });
        self.sync_result = Some(receiver);
        self.refresh_sync();
    }

    fn poll_sync(&mut self, ctx: &egui::Context) {
        let Some(receiver) = &self.sync_result else {
            // Measure again every few hours: a machine that sleeps, or a
            // virtual machine whose clock drifts, would otherwise wander away
            // from the correction taken at launch.
            if self.settings.network_time && self.last_sync.is_none_or(|t| t.elapsed() > RESYNC_INTERVAL) {
                self.run_sync(ctx);
            }
            return;
        };
        let Ok(result) = receiver.try_recv() else { return };
        self.sync_result = None;
        let previous = match std::mem::replace(&mut self.sync, Sync::Idle) {
            Sync::Syncing { previous } => *previous,
            other => other,
        };
        self.sync = match result {
            Ok(result) => {
                self.clock.set_offset(result.offset_ms);
                Sync::Ok(result)
            }
            Err(_) if matches!(previous, Sync::Ok(_)) => previous,
            Err(message) => Sync::Error(message),
        };
        self.refresh_sync();
        self.on_tick(ctx);
    }

    // ------------------------------------------------------------ actions

    fn show_toast(&mut self, message: String) {
        self.toast = Some((message, Instant::now()));
    }

    fn copy_countdown(&mut self, ctx: &egui::Context) {
        let now = countdown::breakdown(self.clock.precise(), self.target);
        let compact = countdown::format_compact(&now);
        let text = if now.elapsed {
            format!("{} has been out for {compact}.", self.game.title)
        } else {
            format!(
                "{} unlocks in {compact} ({} at {}, {}).",
                self.game.title,
                self.target_date(),
                self.target_time(),
                self.zone_text()
            )
        };
        ctx.copy_text(text);
        self.show_toast("Countdown copied to the clipboard.".to_owned());
    }

    fn save_calendar(&mut self) {
        let event = CalendarEvent {
            uid: format!("release-{}-{}@gtavi-countdown", self.game.slug(), self.target),
            start: self.target,
            duration_minutes: 60,
            summary: format!("{} release", self.game.title),
            description: {
                let mut text = format!(
                    "{} unlocks on {} at {} ({}).",
                    self.game.title,
                    self.target_date(),
                    self.target_time(),
                    self.zone_text()
                );
                if !self.game.platforms.is_empty() {
                    text += &format!(" Platforms at launch: {}.", self.game.platforms.join(", "));
                }
                text
            },
            reminders: vec![24 * 60, 60],
            stamp: None,
        };
        let message = match platform::save_calendar(&format!("{}-release.ics", self.game.slug()), &ics::build_calendar(&event)) {
            Ok(path) => format!("Calendar event saved to {}", path.display()),
            Err(error) => format!("Could not save the calendar event. {error}"),
        };
        self.show_toast(message);
    }

    fn apply(&mut self, ctx: &egui::Context, request: Request) {
        match request {
            Request::Settings(next) => self.update_settings(ctx, next),
            Request::ToggleSettings => self.panel.set_open(!self.panel.open, self.target, &self.settings),
            Request::OpenSettings => self.panel.set_open(true, self.target, &self.settings),
            Request::CloseSettings => self.panel.set_open(false, self.target, &self.settings),
            Request::Notify(enabled) => {
                // Desktop notifications need no permission; the setting is simply stored.
                let next = Settings { notify: enabled, ..self.settings.clone() };
                self.update_settings(ctx, next);
                if enabled {
                    self.show_toast("You will be notified at release while the app is open.".to_owned());
                }
            }
            Request::Sync => self.run_sync(ctx),
            Request::Copy => self.copy_countdown(ctx),
            Request::Calendar => self.save_calendar(),
            Request::Fullscreen => {
                let fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                ctx.send_viewport_cmd(ViewportCommand::Fullscreen(!fullscreen));
            }
        }
    }

    fn shortcuts(&self, ctx: &egui::Context, requests: &mut Vec<Request>) {
        let typing = ctx.text_edit_focused();
        ctx.input(|input| {
            let plain = !input.modifiers.ctrl && !input.modifiers.alt && !input.modifiers.command && !input.modifiers.mac_cmd;
            if !plain {
                return;
            }
            if input.key_pressed(Key::Escape) && self.panel.open {
                requests.push(Request::CloseSettings);
            }
            if typing {
                return;
            }
            if input.key_pressed(Key::S) {
                requests.push(Request::ToggleSettings);
            }
            if input.key_pressed(Key::F) {
                requests.push(Request::Fullscreen);
            }
            if input.key_pressed(Key::M) {
                let next = Settings { show_milliseconds: !self.settings.show_milliseconds, ..self.settings.clone() };
                requests.push(Request::Settings(next));
            }
            if input.key_pressed(Key::T) {
                let next = Settings { theme: self.settings.theme.next(), ..self.settings.clone() };
                requests.push(Request::Settings(next));
            }
            if input.key_pressed(Key::C) {
                requests.push(Request::Copy);
            }
        });
    }

    /// Remembers the window's normal size and position, and saves them when
    /// the window closes.
    fn track_window(&mut self, ctx: &egui::Context) {
        let (inner, outer, maximized, fullscreen, closing) = ctx.input(|i| {
            let v = i.viewport();
            (v.inner_rect, v.outer_rect, v.maximized.unwrap_or(false), v.fullscreen.unwrap_or(false), v.close_requested())
        });
        if !maximized && !fullscreen && let Some(inner) = inner {
            self.window = Some(WindowState {
                position: outer.map(|r| [r.left(), r.top()]),
                size: [inner.width(), inner.height()],
                maximized: false,
            });
        }
        if closing && let Some(mut state) = self.window {
            state.maximized = maximized;
            platform::save_window_state(&state);
        }
    }
}

fn game_zone_and_target(settings: &Settings) -> (Game, String, i64) {
    let game = games::resolve(&settings.game, settings.custom_game.as_ref());
    let zone = settings.time_zone.clone().unwrap_or_else(zones::system_time_zone);
    match countdown::release_instant(&game, &zone) {
        Ok(target) => (game, zone, target),
        Err(_) => {
            let target = countdown::release_instant(&game, "UTC").unwrap_or(0);
            (game, "UTC".to_owned(), target)
        }
    }
}

fn sync_ok_text(result: &TimeSync) -> String {
    let sign = if result.offset_ms >= 0.0 { "+" } else { "-" };
    let size = result.offset_ms.abs();
    let amount = if size >= 1000.0 { format!("{:.2} s", size / 1000.0) } else { format!("{} ms", size.round()) };
    format!(
        "Corrected by {sign}{amount} using {} (round trip {} ms).",
        result.server,
        result.round_trip_ms.round()
    )
}

/// The window manager may refuse; the setting stays as the user left it.
fn set_always_on_top(ctx: &egui::Context, enabled: bool) {
    let level = if enabled { egui::WindowLevel::AlwaysOnTop } else { egui::WindowLevel::Normal };
    ctx.send_viewport_cmd(ViewportCommand::WindowLevel(level));
}

// ------------------------------------------------------------------ layout

/// The layout tiers of the web version, applied to the window size.
struct Tiers {
    narrow: bool,
    phone: bool,
    short1: bool,
    short2: bool,
    tiny: bool,
    micro: bool,
}

impl Tiers {
    fn of(w: f32, h: f32) -> Self {
        let narrow = w <= 640.0;
        // Short windows tighten the vertical rhythm, then shed secondary
        // content until only the timer is left. That last state doubles as a
        // compact always-on-top widget.
        Tiers {
            narrow,
            phone: w <= 520.0,
            short1: h <= 860.0 && !narrow,
            short2: h <= 660.0 && !narrow,
            tiny: h <= 560.0,
            micro: h <= 400.0,
        }
    }
}

/// Everything measured for one frame, in content coordinates.
struct Measured {
    tiers: Tiers,
    gutter: f32,
    content_width: f32,
    bar_top: f32,
    bar_bottom: f32,
    main_top: f32,
    main_bottom: f32,
    gap: f32,
    eyebrow: Option<Arc<Galley>>,
    title: Arc<Galley>,
    when: Option<(Arc<Galley>, Arc<Galley>, bool)>,
    when_top: f32,
    local: Option<Arc<Galley>>,
    timer_top: f32,
    geometry: crate::countdown_view::Geometry,
    since: Option<Arc<Galley>>,
    hero_height: f32,
    details: Option<(f32, f32)>,
    stats_layout: StatsLayout,
    value_size: f32,
    footer: Option<(Arc<Galley>, Arc<Galley>, f32, f32)>,
    footer_height: f32,
}

const BAR_HEIGHT: f32 = 40.0;

impl App {
    fn measure(&self, painter: &egui::Painter, w: f32, h: f32) -> Measured {
        let tiers = Tiers::of(w, h);
        let colors = self.colors();
        let gutter = style::clamp(16.0, w * 0.032, 40.0);
        let cw = (w - 2.0 * gutter).max(1.0);
        let main_top = if tiers.narrow {
            style::clamp(20.0, h * 0.05, 48.0)
        } else if tiers.short1 {
            6.0
        } else {
            style::clamp(14.0, h * 0.036, 40.0)
        };
        let main_bottom = if tiers.short2 {
            14.0
        } else if tiers.short1 {
            8.0
        } else {
            style::clamp(10.0, h * 0.02, 24.0)
        };
        let gap = if tiers.tiny {
            0.0
        } else if tiers.narrow {
            style::clamp(96.0, h * 0.18, 170.0)
        } else if tiers.short2 {
            style::clamp(44.0, h * 0.09, 70.0)
        } else if tiers.short1 {
            style::clamp(56.0, h * 0.11, 90.0)
        } else {
            style::clamp(72.0, h * 0.14, 150.0)
        };
        let (bar_top, bar_bottom) = if tiers.short1 { (12.0, 4.0) } else { (18.0, 8.0) };
        let elapsed = self.parts.elapsed;

        // Hero
        let eyebrow = (!tiers.micro).then(|| {
            let (text, color) = if elapsed { ("OUT NOW", colors.accent) } else { ("COUNTDOWN TO", style::ink_soft()) };
            style::galley(painter, text, Font::caps(), color)
        });
        // Long titles step down a size so that they fit on fewer lines.
        let long = self.game.title.chars().count() > 24;
        let title_size = style::clamp(24.0, w * 0.044, 52.0) * if long { 0.72 } else { 1.0 };
        let title_font = Font::sans(title_size.max(20.0), 800.0).width(122.0);
        let title = style::wrapped(painter, &upper(&self.game.title), title_font, style::INK, cw, true);
        let when_top = style::clamp(10.0, h * 0.018, 18.0);
        let when = (!tiers.micro).then(|| {
            let text = format!(
                "{} {} at {}",
                if elapsed { "Released" } else { "Unlocks" },
                self.target_date(),
                self.target_time()
            );
            let font = Font::sans(style::clamp(15.0, w * 0.015, 18.0), 500.0);
            let text = style::wrapped(painter, &text, font, style::INK, cw, true);
            let zone = style::galley(painter, &self.zone_text(), Font::mono(12.0, 500.0), style::ink_soft());
            let pill = 12.0 + zone.size().x + 4.0 + 14.0 + 8.0;
            let horizontal = !tiers.narrow && text.size().x + 12.0 + pill <= cw;
            (text, zone, horizontal)
        });
        let local = (!tiers.micro && !self.local_text.is_empty())
            .then(|| style::galley(painter, &self.local_text, Font::mono(12.0, 400.0), style::ink_soft()));
        let timer_top = if tiers.micro {
            10.0
        } else if tiers.short1 {
            style::clamp(12.0, h * 0.03, 24.0)
        } else {
            style::clamp(20.0, h * 0.044, 48.0)
        };
        let figure = if self.settings.show_milliseconds {
            style::clamp(48.0, (w * if tiers.phone { 0.15 } else { 0.155 }).min(h * 0.23), 204.0)
        } else {
            style::clamp(54.4, (w * if tiers.phone { 0.195 } else { 0.185 }).min(h * 0.23), 204.0)
        };
        let ratio = if tiers.phone { 0.04 } else { 0.07 };
        let mut geometry = self.countdown.geometry(painter, figure, ratio);
        // The milliseconds hang out to the right of the seconds.
        let reach = if self.settings.show_milliseconds { geometry.size * 0.26 * 2.6 } else { 0.0 };
        if geometry.width + 2.0 * reach > cw {
            let fit = cw / (geometry.width + 2.0 * reach);
            geometry = self.countdown.geometry(painter, figure * fit, ratio);
        }
        let since = elapsed.then(|| style::galley(painter, "SINCE RELEASE", Font::caps(), colors.accent));

        let mut hero_height = 0.0;
        if let Some(eyebrow) = &eyebrow {
            hero_height += eyebrow.size().y + 6.0;
        }
        hero_height += title.size().y;
        if let Some((text, zone, horizontal)) = &when {
            let pill = zone.size().y.max(14.0) + 8.0;
            hero_height += when_top + if *horizontal { text.size().y.max(pill) } else { text.size().y + 8.0 + pill };
        }
        if let Some(local) = &local {
            hero_height += 8.0 + local.size().y;
        }
        hero_height += timer_top + geometry.height;
        if let Some(since) = &since {
            hero_height += 14.0 + since.size().y;
        }

        // Details
        let stats_layout = if w > 720.0 {
            StatsLayout::Wide
        } else if w > 520.0 {
            StatsLayout::Medium
        } else {
            StatsLayout::Narrow
        };
        let value_size = style::clamp(18.4, w * 0.021, 25.6);
        let details = (!tiers.tiny).then(|| {
            let timeline = self.timeline.height(painter);
            let stats = if elapsed { 0.0 } else { details::stats_height(painter, stats_layout, value_size) };
            (timeline, stats)
        });

        // Footer
        let footer_padding = if tiers.short1 { (6.0, 10.0) } else { (10.0, 16.0) };
        let footer = (!tiers.short2 && !tiers.tiny).then(|| {
            let source = style::galley(painter, self.clock_source(), Font::mono(11.0, 400.0).spacing(0.88), style::ink_faint());
            let width = if tiers.narrow { cw } else { cw - source.size().x - 24.0 };
            let disclaimer = style::wrapped(
                painter,
                "Unofficial fan project, not affiliated with the developers or publishers of the games shown. Release dates and unlock hours are as reported and may change.",
                Font::sans(12.0, 400.0),
                style::ink_faint(),
                width,
                tiers.narrow,
            );
            (disclaimer, source, footer_padding.0, footer_padding.1)
        });
        let footer_height = footer.as_ref().map_or(0.0, |(disclaimer, source, top, bottom)| {
            let body = if tiers.narrow {
                disclaimer.size().y + 6.0 + source.size().y
            } else {
                disclaimer.size().y.max(source.size().y)
            };
            top + body + bottom
        });

        Measured {
            tiers,
            gutter,
            content_width: cw,
            bar_top,
            bar_bottom,
            main_top,
            main_bottom,
            gap,
            eyebrow,
            title,
            when,
            when_top,
            local,
            timer_top,
            geometry,
            since,
            hero_height,
            details,
            stats_layout,
            value_size,
            footer,
            footer_height,
        }
    }
}

// --------------------------------------------------------------- drawing

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        let frame_start = Instant::now();
        let ctx = ui.ctx().clone();
        if self.can_keep_above.is_none() {
            // Wayland offers no request to keep a window above others.
            let wayland = matches!(frame.window_handle().map(|h| h.as_raw()), Ok(RawWindowHandle::Wayland(_)));
            self.can_keep_above = Some(!wayland);
            if self.settings.always_on_top && !wayland {
                set_always_on_top(&ctx, true);
            }
        }
        if self.release_backdrop {
            self.backdrop.release(frame.wgpu_render_state());
            self.release_backdrop = false;
        }

        // The machine may have slept; read the clock again when the window
        // comes back into focus.
        let focused = ctx.input(|i| i.focused);
        if focused && !self.focused {
            self.clock.resync();
            self.on_tick(&ctx);
        }
        self.focused = focused;
        self.poll_sync(&ctx);
        if self.clock.poll() {
            self.on_tick(&ctx);
        }
        self.track_window(&ctx);

        let mut requests = Vec::new();
        self.shortcuts(&ctx, &mut requests);
        if let Some(screenshot) = &mut self.screenshot {
            screenshot.update(&ctx);
            // Lets a screenshot show the settings sheet.
            if std::env::var_os("GTAVI_COUNTDOWN_SCREENSHOT_SETTINGS").is_some() && !self.panel.open {
                requests.push(Request::OpenSettings);
            }
        }

        let screen = ui.max_rect();
        let (w, h) = (screen.width(), screen.height());
        let painter = ui.painter().clone();
        let m = self.measure(&painter, w, h);

        // Vertical positions in content coordinates.
        let bar_height = m.bar_top + BAR_HEIGHT + m.bar_bottom;
        let details_height = m.details.map_or(0.0, |(a, b)| a + b);
        let fixed = bar_height + m.main_top + m.hero_height + m.main_bottom + details_height + m.footer_height;
        let (hero_top, gap) = if m.tiers.tiny {
            // The compact layout centres the timer in the space below the header.
            let room = h - bar_height - m.footer_height - m.main_top - m.main_bottom;
            (bar_height + m.main_top + ((room - m.hero_height) / 2.0).max(0.0), 0.0)
        } else {
            (bar_height + m.main_top, m.gap + (h - fixed - m.gap).max(0.0))
        };
        let hero_bottom = hero_top + m.hero_height;
        let details_top = hero_bottom + gap;
        let content_height = if m.tiers.tiny {
            (hero_bottom + m.main_bottom + m.footer_height).max(h)
        } else {
            (fixed + gap).max(h)
        };

        // Compose the backdrop around the content: the horizon sits just above
        // the lower panel and the sun fills the gap beneath the digits.
        // Positions are in the scrolled content, so scrolling does not move the scene.
        let anchor = if m.details.is_some() { details_top - 18.0 } else { (hero_bottom + 44.0).max(h * 0.82) };
        let horizon = anchor.clamp(h * 0.42, h * 0.9);
        let room = (horizon - hero_bottom - 6.0) / 1.12;
        let radius = room.max(30.0).min(h * 0.2);
        let composition = Composition { horizon: horizon / h, sun_radius: radius / h };
        let backdrop_frame =
            self.backdrop.paint(&painter, screen, self.settings.theme, self.settings.effects, composition);

        let colors = self.colors();
        let seconds = self.started.elapsed().as_secs_f32();
        let millis = self
            .settings
            .show_milliseconds
            .then(|| countdown::breakdown(self.clock.precise(), self.target).milliseconds);

        ScrollArea::vertical().id_salt("content").auto_shrink([false, false]).show(ui, |ui| {
            let (content, _) = ui.allocate_exact_size(vec2(w, content_height), Sense::hover());
            let painter = ui.painter().clone();
            let left = content.left() + m.gutter;
            let right = content.right() - m.gutter;
            let cx = (left + right) / 2.0;
            let y0 = content.top();

            // Header
            let bar = Rect::from_min_max(pos2(left, y0 + m.bar_top), pos2(right, y0 + m.bar_top + BAR_HEIGHT));
            self.paint_header(ui, bar, &m, &colors, &mut requests);

            // Hero
            let mut y = y0 + hero_top;
            if let Some(eyebrow) = &m.eyebrow {
                let pos = pos2(cx - style::visible_width(eyebrow, Font::caps()) / 2.0, y);
                style::paint_shadowed(&painter, pos, eyebrow, colors.scrim(0.6), 10.0);
                y += eyebrow.size().y + 6.0;
            }
            let title_rect = Rect::from_center_size(pos2(cx, y + m.title.size().y / 2.0), m.title.size());
            style::soft_shadow(&painter, title_rect.shrink2(vec2(0.0, title_rect.height() * 0.15)), colors.scrim(0.45), 24.0);
            style::paint_centered(&painter, cx, y, &m.title);
            let title_response = ui.interact(title_rect, Id::unique("title"), Sense::hover());
            title_response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &self.game.title));
            y += m.title.size().y;
            if let Some((text, zone, horizontal)) = &m.when {
                y += m.when_top;
                let pill_width = 12.0 + zone.size().x + 4.0 + 14.0 + 8.0;
                let pill_height = zone.size().y.max(14.0) + 8.0;
                let (text_pos, pill_rect, height) = if *horizontal {
                    let total = text.size().x + 12.0 + pill_width;
                    let row = text.size().y.max(pill_height);
                    let x = cx - total / 2.0;
                    (
                        pos2(x + text.size().x / 2.0, y + (row - text.size().y) / 2.0),
                        Rect::from_min_size(pos2(x + text.size().x + 12.0, y + (row - pill_height) / 2.0), vec2(pill_width, pill_height)),
                        row,
                    )
                } else {
                    (
                        pos2(cx, y),
                        Rect::from_min_size(pos2(cx - pill_width / 2.0, y + text.size().y + 8.0), vec2(pill_width, pill_height)),
                        text.size().y + 8.0 + pill_height,
                    )
                };
                let text_rect = Rect::from_center_size(pos2(text_pos.x, text_pos.y + text.size().y / 2.0), text.size());
                style::soft_shadow(&painter, text_rect.shrink2(vec2(0.0, text_rect.height() * 0.2)), colors.scrim(0.5), 14.0);
                style::paint_centered(&painter, text_pos.x, text_pos.y, text);
                if zone_pill(ui, pill_rect, zone, &colors).clicked() {
                    requests.push(Request::OpenSettings);
                }
                y += height;
            }
            if let Some(local) = &m.local {
                y += 8.0;
                style::paint_shadowed(&painter, pos2(cx - local.size().x / 2.0, y), local, colors.scrim(0.6), 10.0);
                y += local.size().y;
            }
            y += m.timer_top;
            self.countdown.paint(&painter, &m.geometry, pos2(cx, y), &colors, self.animated, seconds, millis);
            let timer = Rect::from_center_size(pos2(cx, y + m.geometry.height / 2.0), vec2(m.geometry.width, m.geometry.height));
            let timer_response = ui.interact(timer, Id::unique("timer"), Sense::hover());
            timer_response.widget_info(|| WidgetInfo::labeled(Role::Timer, true, &self.countdown.spoken));
            y += m.geometry.height;
            if let Some(since) = &m.since {
                y += 14.0;
                style::paint_shadowed(&painter, pos2(cx - style::visible_width(since, Font::caps()) / 2.0, y), since, colors.scrim(0.6), 10.0);
            }

            // Details
            if let Some((timeline_height, stats_height)) = m.details {
                let width = m.content_width.min(980.0);
                let panel = Rect::from_min_size(
                    pos2(cx - width / 2.0, y0 + details_top),
                    vec2(width, timeline_height + stats_height),
                );
                details::paint_panel(&painter, panel, &colors);
                let timeline = Rect::from_min_size(panel.min, vec2(width, timeline_height));
                self.timeline.paint(ui, timeline, &colors, w <= 560.0);
                if stats_height > 0.0 {
                    let stats = Rect::from_min_size(pos2(panel.left(), timeline.bottom()), vec2(width, stats_height));
                    details::paint_stats(&painter, stats, &self.totals, m.stats_layout, m.value_size);
                }
            }

            // Footer
            if let Some((disclaimer, source, top, _)) = &m.footer {
                let y = y0 + content_height - m.footer_height + top;
                if m.tiers.narrow {
                    style::paint_centered(&painter, cx, y, disclaimer);
                    let x = cx - source.size().x / 2.0;
                    style::paint_at(&painter, pos2(x, y + disclaimer.size().y + 6.0), source);
                } else {
                    style::paint_at(&painter, pos2(left, y), disclaimer);
                    let body = disclaimer.size().y.max(source.size().y);
                    style::paint_at(&painter, pos2(right - source.size().x, y + body - source.size().y), source);
                }
            }
        });

        // Toast
        let mut toast_visible = false;
        if let Some((message, shown)) = &self.toast {
            let elapsed = shown.elapsed();
            if elapsed >= TOAST {
                self.toast = None;
            } else {
                toast_visible = true;
                let t = elapsed.as_secs_f32();
                let alpha = if !self.animated {
                    1.0
                } else {
                    (t / TOAST_FADE).min((TOAST.as_secs_f32() - t) / TOAST_FADE).clamp(0.0, 1.0)
                };
                paint_toast(&ctx, screen, message, alpha, &colors);
            }
        }

        // Settings
        let can_keep_above = self.can_keep_above.unwrap_or(false);
        for action in self.panel.show(&ctx, screen, &self.settings, &colors, can_keep_above, self.animated) {
            requests.push(match action {
                settings_panel::Action::Change(next) => Request::Settings(next),
                settings_panel::Action::Notify(enabled) => Request::Notify(enabled),
                settings_panel::Action::Sync => Request::Sync,
                settings_panel::Action::Close => Request::CloseSettings,
            });
        }

        for request in requests {
            self.apply(&ctx, request);
        }

        // Frames are requested only while something moves; otherwise the
        // next frame is the next tick of the clock.
        let fast = self.settings.show_milliseconds || self.countdown.rolling() || toast_visible || self.panel.animating(&ctx);
        let mut next = self.clock.until_next_tick();
        if let Some(frame) = backdrop_frame {
            next = next.min(frame);
        }
        if fast {
            next = next.min(backdrop::FRAME);
        } else if self.animated {
            next = next.min(COLON_FRAME);
        }
        // Paced from the start of this frame, so drawing time is not added on top.
        ctx.request_repaint_after(next.saturating_sub(frame_start.elapsed()));
    }

    fn on_exit(&mut self) {
        if let Some(state) = self.window {
            platform::save_window_state(&state);
        }
    }
}

impl App {
    fn paint_header(&self, ui: &mut Ui, bar: Rect, m: &Measured, colors: &Colors, requests: &mut Vec<Request>) {
        let painter = ui.painter().clone();
        // Brand
        let icon = Rect::from_min_size(pos2(bar.left(), bar.center().y - 13.0), Vec2::splat(26.0));
        painter.image(self.brand.id(), icon, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        let name_font = Font::sans(17.0, 800.0).width(118.0).spacing(0.34);
        // A long custom title is cut short rather than running under the tools.
        let room = (bar.width() - 6.0 * (BAR_HEIGHT + 6.0) - 60.0).max(60.0);
        let name = painter.layout_job({
            let mut job = style::job(&upper(&self.game.short_title), name_font, style::INK, Some(room), false);
            job.wrap.max_rows = 1;
            job.wrap.break_anywhere = true;
            job
        });
        style::paint_at(&painter, pos2(icon.right() + 9.0, bar.center().y - name.size().y / 2.0), &name);
        if !m.tiers.narrow && name.size().x < room - 120.0 {
            let sub = style::galley(&painter, "COUNTDOWN", Font::caps(), style::ink_soft());
            let x = icon.right() + 9.0 + name.size().x + 9.0;
            style::paint_at(&painter, pos2(x, bar.center().y - sub.size().y / 2.0 + 1.0), &sub);
        }

        // Tools, right to left.
        let fullscreen = ui.ctx().input(|i| i.viewport().fullscreen.unwrap_or(false));
        let mut tools: Vec<(Icon, &str, &str, bool, Request)> = vec![
            (Icon::Calendar, "Add to calendar", "Add to calendar", false, Request::Calendar),
            (Icon::Copy, "Copy the countdown (C)", "Copy the countdown", false, Request::Copy),
        ];
        if self.can_keep_above == Some(true) {
            let next = Settings { always_on_top: !self.settings.always_on_top, ..self.settings.clone() };
            tools.push((Icon::Pin, "Always on top", "Always on top", self.settings.always_on_top, Request::Settings(next)));
        }
        tools.push((
            if fullscreen { Icon::Compress } else { Icon::Expand },
            "Toggle fullscreen (F)",
            "Toggle fullscreen",
            false,
            Request::Fullscreen,
        ));
        tools.push((Icon::Sliders, "Settings (S)", "Settings", false, Request::ToggleSettings));
        let count = tools.len() as f32;
        let mut x = bar.right() - count * BAR_HEIGHT - (count - 1.0) * 6.0;
        for (icon, tooltip, name, checked, request) in tools {
            let rect = Rect::from_min_size(pos2(x, bar.top()), Vec2::splat(BAR_HEIGHT));
            if tool_button(ui, rect, icon, tooltip, name, checked, colors).clicked() {
                requests.push(request);
            }
            x += BAR_HEIGHT + 6.0;
        }
    }
}

fn tool_button(ui: &mut Ui, rect: Rect, icon: Icon, tooltip: &str, name: &str, checked: bool, colors: &Colors) -> Response {
    if !checked {
        return settings_panel::round_button(ui, rect, icon, tooltip, name, colors, true);
    }
    let response = ui
        .interact(rect, Id::unique(("round-button", name)), Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tooltip);
    response.widget_info(|| WidgetInfo::selected(Role::CheckBox, true, true, name));
    let painter = ui.painter();
    painter.circle_filled(rect.center(), rect.width() / 2.0, colors.accent);
    icons::paint(painter, icon, rect.center(), 20.0, colors.base);
    if response.has_focus() {
        painter.circle_stroke(rect.center(), rect.width() / 2.0 + 3.0, Stroke::new(2.0, colors.accent));
    }
    response
}

/// The time zone under the date, which opens the settings at the zone picker.
fn zone_pill(ui: &mut Ui, rect: Rect, zone: &Arc<Galley>, colors: &Colors) -> Response {
    let response = ui
        .interact(rect, Id::unique("zone-pill"), Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text("Change the time zone");
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, zone.text()));
    let painter = ui.painter();
    let hovered = response.hovered();
    painter.rect_filled(rect, rect.height() / 2.0, colors.panel);
    let border = if hovered { style::line_strong() } else { style::line() };
    painter.rect_stroke(rect, rect.height() / 2.0, Stroke::new(1.0, border), StrokeKind::Inside);
    let color = if hovered { style::INK } else { style::ink_soft() };
    let pos = pos2(rect.left() + 12.0, rect.center().y - zone.size().y / 2.0);
    painter.galley_with_override_text_color(pos - zone.rect.min.to_vec2(), zone.clone(), color);
    icons::paint(painter, Icon::Chevron, pos2(rect.right() - 8.0 - 7.0, rect.center().y), 14.0, color.gamma_multiply(0.7));
    if response.has_focus() {
        style::focus_ring(painter, rect, rect.height() / 2.0, colors.accent);
    }
    response
}

fn paint_toast(ctx: &egui::Context, screen: Rect, message: &str, alpha: f32, colors: &Colors) {
    egui::Area::new(Id::unique("toast"))
        .order(Order::Tooltip)
        .interactable(false)
        .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -56.0))
        .show(ctx, |ui| {
            let painter = ui.painter().clone();
            let max_width = (screen.width() - 64.0).min(480.0);
            let mut job = LayoutJob::single_section(message.to_owned(), Font::sans(14.0, 400.0).format(style::ink(alpha)));
            job.wrap.max_width = max_width;
            job.halign = egui::Align::Center;
            let galley = painter.layout_job(job);
            let size = galley.size() + vec2(36.0, 20.0);
            let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
            response.widget_info(|| WidgetInfo::labeled(Role::Label, true, message));
            let shadow = egui::epaint::Shadow {
                offset: [0, 12],
                blur: 36,
                spread: 0,
                color: Color32::from_black_alpha((102.0 * alpha) as u8),
            };
            painter.add(shadow.as_shape(rect, 255));
            let radius = rect.height() / 2.0;
            painter.rect_filled(rect, radius, colors.panel_solid.gamma_multiply(alpha));
            painter.rect_stroke(rect, radius, Stroke::new(1.0, style::line_strong().gamma_multiply(alpha)), StrokeKind::Inside);
            style::paint_centered(&painter, rect.center().x, rect.top() + 10.0, &galley);
        });
}
