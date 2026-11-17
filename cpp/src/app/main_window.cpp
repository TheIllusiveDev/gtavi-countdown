#include "main_window.hpp"

#include <gdkmm/clipboard.h>
#include <gtk/gtk.h>
#include <gtkmm/eventcontrollerkey.h>
#include <gtkmm/image.h>

#include <algorithm>
#include <chrono>
#include <cmath>
#include <format>

#include "background.hpp"
#include "countdown.hpp"
#include "ics.hpp"
#include "release.hpp"
#include "window_state.hpp"
#include "wording.hpp"
#include "zones.hpp"

namespace gtavi {

namespace {

// How often the clock is measured again while network time is on.
constexpr unsigned resync_interval_seconds = 6 * 60 * 60;
constexpr unsigned toast_ms = 3600;

std::string upper(std::string text) {
    std::ranges::transform(text, text.begin(), [](unsigned char c) { return static_cast<char>(std::toupper(c)); });
    return text;
}

double clamp(double low, double value, double high) { return std::clamp(value, low, high); }

void accessible_label(Gtk::Widget& widget, const std::string& text) {
    gtk_accessible_update_property(GTK_ACCESSIBLE(widget.gobj()), GTK_ACCESSIBLE_PROPERTY_LABEL, text.c_str(), -1);
}

Gtk::Button& tool_button(IconName icon, const std::string& tooltip, const std::string& name) {
    auto& button = *Gtk::make_managed<Gtk::Button>();
    button.add_css_class("tool");
    button.set_child(*Gtk::make_managed<Icon>(icon));
    button.set_tooltip_text(tooltip);
    button.set_valign(Gtk::Align::CENTER);
    accessible_label(button, name);
    return button;
}

Settings load_settings() {
    auto stored = platform::load_settings();
    return settings::sanitize(stored.value_or(""));
}

std::string sync_ok_text(const platform::TimeSyncResult& result) {
    auto sign = result.offset_ms >= 0 ? "+" : "-";
    auto size = std::abs(result.offset_ms);
    auto amount = size >= 1000 ? std::format("{:.2f} s", size / 1000) : std::format("{} ms", std::llround(size));
    return std::format("Corrected by {}{} using {} (round trip {} ms).", sign, amount, result.server,
                       std::llround(result.round_trip_ms));
}

}  // namespace

MainWindow::MainWindow(const Glib::RefPtr<Gtk::Application>& app)
    : Gtk::ApplicationWindow(app),
      settings_(load_settings()),
      can_keep_above_(platform::can_keep_above(*this)),
      backdrop_(settings_.theme, settings_.effects),
      countdown_([this] { return clock_.precise(); }),
      settings_panel_(
          {
              .change = [this](const Settings& next) { update_settings(next); },
              .sync = [this] { run_sync(); },
              .notify = [this](bool enabled) { set_notify(enabled); },
              .closed = [this] { shell_.set_can_focus(true); },
          },
          can_keep_above_) {
    set_title("GTA VI Countdown");
    set_icon_name(APP_ID);
    add_css_class("countdown");
    set_size_request(360, 240);
    if (auto state = window_state::load()) {
        set_default_size(state->width, state->height);
        if (state->maximized) maximize();
    } else {
        set_default_size(1100, 760);
    }

    // The header is part of the content so that the backdrop shows behind
    // it; an empty title bar keeps client-side decorations (shadows and resize
    // edges) without drawing one.
    auto& titlebar = *Gtk::make_managed<Gtk::Box>();
    titlebar.set_visible(false);
    set_titlebar(titlebar);

    layout_css_ = Gtk::CssProvider::create();
    Gtk::StyleProvider::add_provider_for_display(get_display(), layout_css_,
                                                 GTK_STYLE_PROVIDER_PRIORITY_APPLICATION + 1);

    build_header();
    build_hero();

    // Main: the hero at the top, the details at the bottom, and between them
    // the gap where the sun sits, which never closes completely.
    main_.set_vexpand(true);
    main_.append(hero_);
    gap_.set_vexpand(true);
    main_.append(gap_);
    details_.add_css_class("details");
    details_.set_overflow(Gtk::Overflow::HIDDEN);
    details_.append(timeline_);
    details_.append(stats_);
    main_.append(details_clamp_);

    shell_.append(bar_);
    shell_.append(main_);
    build_footer();
    shell_.append(footer_);

    scroller_.set_policy(Gtk::PolicyType::NEVER, Gtk::PolicyType::AUTOMATIC);
    scroller_.set_child(shell_);
    handle_.set_child(scroller_);

    toast_.add_css_class("toast");
    toast_.set_wrap(true);
    toast_.set_wrap_mode(Pango::WrapMode::WORD_CHAR);
    toast_.set_max_width_chars(60);
    toast_.set_justify(Gtk::Justification::CENTER);
    gtk_accessible_update_property(GTK_ACCESSIBLE(toast_.gobj()), GTK_ACCESSIBLE_PROPERTY_LABEL, "Status", -1);
    toast_revealer_.set_child(toast_);
    toast_revealer_.set_transition_type(Gtk::RevealerTransitionType::CROSSFADE);
    toast_revealer_.set_transition_duration(200);
    toast_revealer_.set_halign(Gtk::Align::CENTER);
    toast_revealer_.set_valign(Gtk::Align::END);
    toast_revealer_.set_can_target(false);

    root_.set_child(backdrop_);
    root_.add_overlay(handle_);
    root_.set_measure_overlay(handle_, true);
    root_.add_overlay(toast_revealer_);
    root_.add_overlay(settings_panel_);
    set_child(root_);

    auto keys = Gtk::EventControllerKey::create();
    keys->signal_key_pressed().connect(sigc::mem_fun(*this, &MainWindow::on_key_pressed), false);
    add_controller(keys);

    // Timers are not throttled on the desktop, but the machine may have slept.
    property_is_active().signal_changed().connect([this] {
        if (is_active()) clock_.resync();
    });
    property_fullscreened().signal_changed().connect([this] {
        bool fullscreen = is_fullscreen();
        fullscreen_icon_.set_icon(fullscreen ? IconName::compress : IconName::expand);
    });
    signal_map().connect([this] {
        if (settings_.always_on_top) set_always_on_top(true);
    });

    set_zone();
    apply_theme();
    refresh_zone_text();
    settings_panel_.set_settings(settings_);
    countdown_.set_show_milliseconds(settings_.show_milliseconds);
    pin_button_.set_active(settings_.always_on_top);
    int width = 0;
    int height = 0;
    get_default_size(width, height);
    apply_layout(width, height);

    was_elapsed_ = countdown::breakdown(clock_.now(), target_).elapsed;
    clock_.signal_ticked().connect(sigc::mem_fun(*this, &MainWindow::on_tick));
    clock_.start(target_);
    configure_network_time();
    refresh_sync();
}

MainWindow::~MainWindow() {
    clock_.stop();
    layout_idle_.disconnect();
    toast_timer_.disconnect();
    resync_timer_.disconnect();
    Gtk::StyleProvider::remove_provider_for_display(get_display(), layout_css_);
}

// ------------------------------------------------------------------ build

void MainWindow::build_header() {
    bar_.add_css_class("bar");

    auto& brand = *Gtk::make_managed<Gtk::Box>(Gtk::Orientation::HORIZONTAL, 9);
    brand.set_valign(Gtk::Align::CENTER);
    auto& mark = *Gtk::make_managed<Gtk::Image>();
    mark.set_from_resource("/app/gtavi/countdown/brand.svg");
    mark.set_pixel_size(26);
    brand.append(mark);
    auto& name = *Gtk::make_managed<Gtk::Label>("GTA VI");
    name.add_css_class("brand-name");
    brand.append(name);
    brand_sub_.set_text("COUNTDOWN");
    brand_sub_.add_css_class("label-caps");
    brand_sub_.add_css_class("brand-sub");
    // It hides on narrow windows; until then it must not hold the window wide.
    brand_sub_.set_ellipsize(Pango::EllipsizeMode::END);
    brand.append(brand_sub_);
    brand.set_hexpand(true);
    bar_.append(brand);

    auto& tools = *Gtk::make_managed<Gtk::Box>(Gtk::Orientation::HORIZONTAL, 6);
    accessible_label(tools, "Actions");

    auto& calendar = tool_button(IconName::calendar, "Add to calendar", "Add to calendar");
    calendar.signal_clicked().connect([this] { save_calendar(); });
    tools.append(calendar);

    auto& copy = tool_button(IconName::copy, "Copy the countdown (C)", "Copy the countdown");
    copy.signal_clicked().connect([this] { copy_countdown(); });
    tools.append(copy);

    // Always on top needs a window manager that honours the request (X11).
    pin_button_.add_css_class("tool");
    pin_button_.set_child(*Gtk::make_managed<Icon>(IconName::pin));
    pin_button_.set_tooltip_text("Always on top");
    pin_button_.set_valign(Gtk::Align::CENTER);
    pin_button_.set_visible(can_keep_above_);
    accessible_label(pin_button_, "Always on top");
    pin_button_.signal_toggled().connect([this] {
        if (pin_button_.get_active() != settings_.always_on_top) {
            auto next = settings_;
            next.always_on_top = pin_button_.get_active();
            update_settings(next);
        }
    });
    tools.append(pin_button_);

    fullscreen_button_.add_css_class("tool");
    fullscreen_button_.set_child(fullscreen_icon_);
    fullscreen_button_.set_tooltip_text("Toggle fullscreen (F)");
    fullscreen_button_.set_valign(Gtk::Align::CENTER);
    accessible_label(fullscreen_button_, "Toggle fullscreen");
    fullscreen_button_.signal_clicked().connect([this] { toggle_fullscreen(); });
    tools.append(fullscreen_button_);

    auto& settings = tool_button(IconName::sliders, "Settings (S)", "Settings");
    settings.signal_clicked().connect([this] { toggle_settings(); });
    tools.append(settings);

    bar_.append(tools);
    controls_.set_valign(Gtk::Align::CENTER);
    bar_.append(controls_);
}

void MainWindow::build_hero() {
    hero_.set_halign(Gtk::Align::CENTER);

    eyebrow_.add_css_class("label-caps");
    eyebrow_.add_css_class("eyebrow");
    hero_.append(eyebrow_);

    title_.set_text(upper(std::string{release::title}));
    title_.add_css_class("title");
    title_.set_wrap(true);
    title_.set_justify(Gtk::Justification::CENTER);
    title_.set_margin_top(6);
    accessible_label(title_, std::string{release::title});
    hero_.append(title_);

    when_.add_css_class("when");
    when_.set_halign(Gtk::Align::CENTER);
    when_text_.set_wrap(true);
    when_text_.set_justify(Gtk::Justification::CENTER);
    when_.append(when_text_);
    auto& zone_content = *Gtk::make_managed<Gtk::Box>(Gtk::Orientation::HORIZONTAL, 4);
    zone_label_.add_css_class("mono");
    zone_content.append(zone_label_);
    auto& chevron = *Gtk::make_managed<Icon>(IconName::chevron, 14);
    chevron.set_opacity(0.7);
    zone_content.append(chevron);
    zone_button_.set_child(zone_content);
    zone_button_.add_css_class("zone");
    zone_button_.set_halign(Gtk::Align::CENTER);
    zone_button_.set_valign(Gtk::Align::CENTER);
    zone_button_.set_tooltip_text("Change the time zone");
    zone_button_.signal_clicked().connect([this] {
        shell_.set_can_focus(false);
        settings_panel_.open();
    });
    when_.append(zone_button_);
    hero_.append(when_);

    local_.add_css_class("local");
    local_.add_css_class("mono");
    local_.set_margin_top(8);
    hero_.append(local_);

    hero_.append(countdown_);

    since_.set_text("SINCE RELEASE");
    since_.add_css_class("label-caps");
    since_.add_css_class("since");
    since_.set_margin_top(14);
    hero_.append(since_);
}

void MainWindow::build_footer() {
    footer_.add_css_class("footer");
    disclaimer_.set_text(
        "Unofficial fan project. Not affiliated with Rockstar Games or Take-Two Interactive. The unlock hour is as "
        "reported and may change.");
    disclaimer_.set_wrap(true);
    disclaimer_.set_xalign(0);
    disclaimer_.set_hexpand(true);
    footer_.append(disclaimer_);
    clock_source_.add_css_class("clock-source");
    clock_source_.add_css_class("mono");
    clock_source_.set_valign(Gtk::Align::END);
    footer_.append(clock_source_);
}

// ---------------------------------------------------------------- wording

std::string MainWindow::target_date() const { return wording::long_date(target_, zone_); }

std::string MainWindow::target_time() const { return wording::time(target_, zone_); }

std::string MainWindow::zone_text() const { return wording::zone_text(zone_, target_); }

void MainWindow::refresh_zone_text() {
    zone_label_.set_text(zone_text());
    // When the chosen zone differs from the device's, the same instant is also
    // shown on the device's own clock, which is what the viewer will be reading.
    auto system = zones::system_time_zone();
    if (zones::offset_at(target_, system) == zones::offset_at(target_, zone_)) {
        local_.set_text("");
    } else {
        local_.set_text(wording::short_date_time(target_, system) + " on this device");
    }
    local_.set_visible(!local_.get_text().empty() && layout_height_ > 400);
}

void MainWindow::refresh_sync() {
    std::string source;
    std::string text;
    bool busy = false;
    if (std::holds_alternative<sync_state::Syncing>(sync_)) {
        source = "SYNCING CLOCK";
        text = "Measuring the clock offset.";
        busy = true;
    } else if (auto* ok = std::get_if<sync_state::Ok>(&sync_)) {
        source = "NETWORK TIME";
        text = sync_ok_text(ok->result);
    } else if (auto* error = std::get_if<sync_state::Error>(&sync_)) {
        source = "SYSTEM CLOCK";
        text = "Using the system clock. " + error->message;
    } else {
        source = "SYSTEM CLOCK";
        text = "Using the system clock.";
    }
    clock_source_.set_text(source);
    settings_panel_.set_sync(text, busy);
}

// ---------------------------------------------------------------- state

void MainWindow::set_zone() {
    zone_ = settings_.time_zone.value_or(zones::system_time_zone());
    target_ = countdown::release_instant(zone_);
}

void MainWindow::apply_theme() {
    for (auto theme : themes::all) remove_css_class(std::format("theme-{}", settings::key(theme)));
    add_css_class(std::format("theme-{}", settings::key(settings_.theme)));
    colors_ = ui_colors(settings_.theme);
    countdown_.set_colors(colors_);
    timeline_.set_colors(colors_);
    backdrop_.set_theme(settings_.theme);
}

void MainWindow::update_settings(Settings next) {
    auto previous = settings_;
    if (next == previous) {
        // A control may have moved on its own (a refused toggle); put it back.
        settings_panel_.set_settings(settings_);
        return;
    }
    settings_ = next;

    platform::save_settings(settings::serialize(next));
    settings_panel_.set_settings(next);
    if (next.theme != previous.theme) apply_theme();
    if (next.effects != previous.effects) backdrop_.set_effects(next.effects);
    if (next.show_milliseconds != previous.show_milliseconds) {
        countdown_.set_show_milliseconds(next.show_milliseconds);
        // The figures are sized differently with the readout.
        apply_layout(layout_width_, layout_height_);
    }
    if (next.time_zone != previous.time_zone) {
        set_zone();
        refresh_zone_text();
        // Re-phase the tick, since the target instant has moved.
        clock_.retarget(target_);
    }
    if (next.always_on_top != previous.always_on_top) {
        pin_button_.set_active(next.always_on_top);
        set_always_on_top(next.always_on_top);
    }
    if (next.network_time != previous.network_time) configure_network_time();
    refresh_sync();
    on_tick();
}

void MainWindow::on_tick() {
    auto parts = countdown::breakdown(clock_.now(), target_);

    // Fire the release notification on the tick that crosses zero. Opening
    // the app after release must not notify, hence the comparison with the
    // previous state rather than a check of the current one.
    if (was_elapsed_ == false && parts.elapsed) {
        show_toast(std::string{release::title} + " is out now.");
        if (settings_.notify) {
            if (auto app = get_application()) {
                platform::notify(*app, std::string{release::title} + " is out now", "The wait is over.");
            }
        }
    }
    was_elapsed_ = parts.elapsed;

    // The window title follows the countdown at minute resolution.
    auto title = parts.elapsed ? std::string{"GTA VI is out"}
                               : std::format("{}d {:02}h {:02}m | GTA VI Countdown", parts.days, parts.hours,
                                             parts.minutes);
    if (title != title_text_) {
        title_text_ = title;
        set_title(title);
    }

    eyebrow_.set_text(parts.elapsed ? "OUT NOW" : "COUNTDOWN TO");
    if (parts.elapsed) eyebrow_.add_css_class("out"); else eyebrow_.remove_css_class("out");
    when_text_.set_text(std::format("{} {} at {}", parts.elapsed ? "Released" : "Unlocks", target_date(),
                                    target_time()));
    countdown_.set_parts(parts, target_);
    since_.set_visible(parts.elapsed);
    timeline_.set_data(clock_.now(), target_);
    stats_.set_visible(!parts.elapsed);
    if (!parts.elapsed) stats_.set_data(clock_.now(), target_, zone_);
}

// ---------------------------------------------------------------- layout

// GTK has no media queries; the window applies the breakpoints and fluid
// sizes of the web version itself whenever its size changes.
void MainWindow::size_allocate_vfunc(int width, int height, int baseline) {
    Gtk::ApplicationWindow::size_allocate_vfunc(width, height, baseline);
    int w = root_.get_width();
    int h = root_.get_height();
    if ((w != layout_width_ || h != layout_height_) && !layout_idle_.connected() && w > 0 && h > 0) {
        // Changing styles during allocation would re-enter it; the next idle is soon enough.
        layout_idle_ = Glib::signal_idle().connect(
            [this] {
                apply_layout(root_.get_width(), root_.get_height());
                return false;
            },
            Glib::PRIORITY_HIGH_IDLE);
    }
    compose_backdrop();
}

void MainWindow::apply_layout(int width, int height) {
    if (width <= 0 || height <= 0) return;
    layout_width_ = width;
    layout_height_ = height;
    double w = width;
    double h = height;

    bool narrow = width <= 640;
    bool phone = width <= 520;
    // Short windows tighten the vertical rhythm, then shed secondary content
    // until only the timer is left. That last state doubles as a compact
    // always-on-top widget.
    bool short1 = height <= 860 && !narrow;
    bool short2 = height <= 660 && !narrow;
    bool tiny = height <= 560;
    bool micro = height <= 400;
    tiny_ = tiny;

    double gutter = clamp(16, w * 0.032, 40);
    shell_.set_margin_start(static_cast<int>(gutter));
    shell_.set_margin_end(static_cast<int>(gutter));

    double main_top = narrow ? clamp(20, h * 0.05, 48) : short1 ? 6 : clamp(14, h * 0.036, 40);
    double main_bottom = short2 ? 14 : short1 ? 8 : clamp(10, h * 0.02, 24);
    double gap = tiny     ? 0
                 : narrow ? clamp(96, h * 0.18, 170)
                 : short2 ? clamp(44, h * 0.09, 70)
                 : short1 ? clamp(56, h * 0.11, 90)
                          : clamp(72, h * 0.14, 150);
    main_.set_margin_top(static_cast<int>(main_top));
    main_.set_margin_bottom(static_cast<int>(main_bottom));
    gap_.set_size_request(-1, static_cast<int>(gap));
    gap_.set_vexpand(!tiny);
    main_.set_valign(tiny ? Gtk::Align::CENTER : Gtk::Align::FILL);

    double timer_top = micro ? 10 : short1 ? clamp(12, h * 0.03, 24) : clamp(20, h * 0.044, 48);
    countdown_.set_margin_top(static_cast<int>(timer_top));
    double size = settings_.show_milliseconds
                      ? clamp(48, std::min(w * (phone ? 0.15 : 0.155), h * 0.23), 204)
                      : clamp(54.4, std::min(w * (phone ? 0.195 : 0.185), h * 0.23), 204);
    countdown_.set_figure_size(size, phone ? 0.04 : 0.07);

    brand_sub_.set_visible(!narrow);
    when_.set_orientation(narrow ? Gtk::Orientation::VERTICAL : Gtk::Orientation::HORIZONTAL);
    when_.set_spacing(narrow ? 8 : 12);
    when_.set_margin_top(static_cast<int>(clamp(10, h * 0.018, 18)));
    eyebrow_.set_visible(!micro);
    when_.set_visible(!micro);
    local_.set_visible(!micro && !local_.get_text().empty());

    details_clamp_.set_visible(!tiny);
    stats_.set_layout(width > 720 ? StatsView::Layout::wide
                      : width > 520 ? StatsView::Layout::medium
                                    : StatsView::Layout::narrow);
    timeline_.set_compact(width <= 560);

    footer_.set_visible(!short2 && !tiny);
    footer_.set_orientation(narrow ? Gtk::Orientation::VERTICAL : Gtk::Orientation::HORIZONTAL);
    footer_.set_spacing(narrow ? 6 : 24);
    disclaimer_.set_xalign(narrow ? 0.5f : 0.0f);
    disclaimer_.set_justify(narrow ? Gtk::Justification::CENTER : Gtk::Justification::LEFT);
    clock_source_.set_halign(narrow ? Gtk::Align::CENTER : Gtk::Align::FILL);

    settings_panel_.set_sheet_width(std::min(420, width));

    // Fluid type sizes, as clamp() expressions over the viewport in the web version.
    auto css = std::format(
        ".bar {{ padding: {}px 0 {}px; }}\n"
        ".title {{ font-size: {:.1f}px; }}\n"
        ".when {{ font-size: {:.1f}px; }}\n"
        ".tile .value {{ font-size: {:.1f}px; }}\n"
        ".footer {{ padding: {}px 0 {}px; }}\n",
        short1 ? 12 : 18, short1 ? 4 : 8, clamp(24, w * 0.044, 52), clamp(15, w * 0.015, 18),
        clamp(18.4, w * 0.021, 25.6), short1 ? 6 : 10, short1 ? 10 : 16);
    if (css != layout_css_text_) {
        layout_css_text_ = css;
        layout_css_->load_from_string(css);
    }
}

// Composes the backdrop around the content: the horizon sits just above the
// lower panels and the sun fills the gap beneath the digits. Positions are
// measured in the scrolled content, so scrolling does not move the scene.
void MainWindow::compose_backdrop() {
    double height = backdrop_.get_height();
    if (height <= 0) return;
    graphene_rect_t hero{};
    if (!gtk_widget_compute_bounds(GTK_WIDGET(hero_.gobj()), GTK_WIDGET(shell_.gobj()), &hero)) return;
    double hero_bottom = hero.origin.y + hero.size.height;

    double anchor = 0;
    graphene_rect_t details{};
    bool shown = details_clamp_.get_visible() &&
                 gtk_widget_compute_bounds(GTK_WIDGET(details_.gobj()), GTK_WIDGET(shell_.gobj()), &details);
    // In the compact layout the lower panels are hidden; the horizon then
    // drops to the bottom of the window, clear of the timer.
    anchor = shown ? details.origin.y - 18 : std::max(hero_bottom + 44, height * 0.82);
    double horizon = std::clamp(anchor, height * 0.42, height * 0.9);
    // The sun is sunk 12% below the horizon, so 1.12 radii show above it.
    double room = (horizon - hero_bottom - 6) / 1.12;
    double radius = std::min(std::max(room, 30.0), height * 0.2);
    Composition next{static_cast<float>(horizon / height), static_cast<float>(radius / height)};
    backdrop_.set_composition(next);
}

// ---------------------------------------------------------------- network time

// Measure the clock at start-up and again every few hours: a machine that
// sleeps, or a virtual machine whose clock drifts, would otherwise wander
// away from the correction taken at launch.
void MainWindow::configure_network_time() {
    resync_timer_.disconnect();
    if (!settings_.network_time) {
        clock_.set_offset(0);
        sync_ = sync_state::Idle{};
        refresh_sync();
        return;
    }
    run_sync();
    resync_timer_ = Glib::signal_timeout().connect_seconds(
        [this] {
            run_sync();
            return true;
        },
        resync_interval_seconds);
}

void MainWindow::run_sync() {
    if (!settings_.network_time || std::holds_alternative<sync_state::Syncing>(sync_)) return;
    auto previous = sync_;
    sync_ = sync_state::Syncing{};
    refresh_sync();
    run_in_background<platform::TimeSyncResult>(
        alive_, [] { return platform::sync_network_time(); },
        [this, previous](std::expected<platform::TimeSyncResult, std::string> result) {
            // The setting may have been switched off while the request was in flight.
            if (!settings_.network_time) return;
            if (result) {
                clock_.set_offset(result->offset_ms);
                sync_ = sync_state::Ok{*result};
            } else if (std::holds_alternative<sync_state::Ok>(previous)) {
                // A failed refresh keeps the last good correction rather than
                // discarding it; only a first failure falls back to the system clock.
                sync_ = previous;
            } else {
                sync_ = sync_state::Error{result.error()};
            }
            refresh_sync();
        });
}

// Desktop notifications need no permission; the setting is simply stored.
void MainWindow::set_notify(bool enabled) {
    auto next = settings_;
    next.notify = enabled;
    update_settings(next);
    if (enabled) show_toast("You will be notified at release while the app is open.");
}

void MainWindow::set_always_on_top(bool enabled) {
    // The window manager may refuse; the setting stays as the user left it.
    platform::set_keep_above(*this, enabled);
}

// ---------------------------------------------------------------- actions

void MainWindow::show_toast(const std::string& message) {
    toast_.set_text(message);
    toast_revealer_.set_reveal_child(true);
    gtk_accessible_announce(GTK_ACCESSIBLE(toast_.gobj()), message.c_str(),
                            GTK_ACCESSIBLE_ANNOUNCEMENT_PRIORITY_MEDIUM);
    toast_timer_.disconnect();
    toast_timer_ = Glib::signal_timeout().connect(
        [this] {
            toast_revealer_.set_reveal_child(false);
            return false;
        },
        toast_ms);
}

void MainWindow::toggle_fullscreen() {
    if (is_fullscreen()) {
        unfullscreen();
    } else {
        fullscreen();
    }
}

void MainWindow::toggle_settings() {
    if (settings_panel_.is_open()) {
        settings_panel_.close();
    } else {
        // Keyboard focus stays inside the sheet while it is open.
        shell_.set_can_focus(false);
        settings_panel_.open();
    }
}

void MainWindow::copy_countdown() {
    auto now = countdown::breakdown(clock_.precise(), target_);
    auto title = std::string{release::title};
    auto text = now.elapsed ? std::format("{} has been out for {}.", title, countdown::format_compact(now))
                            : std::format("{} unlocks in {} ({} at {}, {}).", title, countdown::format_compact(now),
                                          target_date(), target_time(), zone_text());
    get_clipboard()->set_text(text);
    show_toast("Countdown copied to the clipboard.");
}

void MainWindow::save_calendar() {
    auto title = std::string{release::title};
    std::string platforms;
    for (auto platform : release::platforms) {
        if (!platforms.empty()) platforms += ", ";
        platforms += platform;
    }
    CalendarEvent event{
        .uid = std::format("release-{}@gtavi-countdown", target_),
        .start = target_,
        .duration_minutes = 60,
        .summary = title + " release",
        .description = std::format("{} unlocks on {} at {} ({}). Platforms at launch: {}.", title, target_date(),
                                   target_time(), zone_text(), platforms),
        .reminders = {24 * 60, 60},
        .stamp = std::nullopt,
    };
    try {
        auto path = platform::save_calendar("gta-vi-release.ics", ics::build_calendar(event));
        show_toast("Calendar event saved to " + path.string());
    } catch (const std::exception& error) {
        show_toast(std::string{"Could not save the calendar event. "} + error.what());
    }
}

bool MainWindow::on_key_pressed(guint keyval, guint, Gdk::ModifierType state) {
    using Gdk::ModifierType;
    auto modifiers = state & (ModifierType::CONTROL_MASK | ModifierType::ALT_MASK | ModifierType::SUPER_MASK |
                              ModifierType::META_MASK);
    if (modifiers != ModifierType{}) return false;
    switch (gdk_keyval_to_lower(keyval)) {
        case GDK_KEY_Escape:
            if (!settings_panel_.is_open()) return false;
            settings_panel_.close();
            return true;
        case GDK_KEY_s:
            toggle_settings();
            return true;
        case GDK_KEY_f:
            toggle_fullscreen();
            return true;
        case GDK_KEY_m: {
            auto next = settings_;
            next.show_milliseconds = !next.show_milliseconds;
            update_settings(next);
            return true;
        }
        case GDK_KEY_t: {
            auto next = settings_;
            next.theme = settings::next(next.theme);
            update_settings(next);
            return true;
        }
        case GDK_KEY_c:
            copy_countdown();
            return true;
        default:
            return false;
    }
}

bool MainWindow::on_close_request() {
    window_state::save(*this);
    return Gtk::ApplicationWindow::on_close_request();
}

}  // namespace gtavi
