#include "countdown_view.hpp"

#include <gdkmm/frameclock.h>
#include <gtk/gtk.h>
#include <gtkmm/snapshot.h>
#include <gtkmm/settings.h>

#include <algorithm>
#include <cmath>
#include <format>
#include <numbers>

#include "drawing.hpp"

namespace gtavi {

namespace {

constexpr double roll_us = 460'000;
// The smallest figure height the layout asks for.
constexpr double min_size = 48;
// How far a glyph travels while rolling, as a fraction of the figure height.
constexpr double roll_distance = 0.58;
// Colons breathe once per two seconds.
constexpr double pulse_us = 2'000'000;

bool animations_enabled() {
    auto settings = Gtk::Settings::get_default();
    return !settings || settings->property_gtk_enable_animations().get_value();
}

// cubic-bezier(0.33, 1, 0.68, 1) is the cubic ease-out.
double ease_out(double t) { return 1 - std::pow(1 - t, 3); }

double ease_in_out(double t) { return t * t * (3 - 2 * t); }

std::string pad(std::int64_t value) { return std::format("{:02}", value); }

std::string upper(std::string text) {
    for (auto& c : text) {
        if (c >= 'a' && c <= 'z') c = static_cast<char>(c - 'a' + 'A');
    }
    return text;
}

Pango::FontDescription figure_font(double size) { return drawing::font(drawing::sans_family, size, 800, 62); }

Pango::FontDescription label_font(double size) { return drawing::font(drawing::mono_family, size, 500); }

Pango::FontDescription millis_font(double size) { return drawing::font(drawing::mono_family, size, 600); }

void class_init(void* g_class, void*) {
    gtk_widget_class_set_css_name(GTK_WIDGET_CLASS(g_class), "countdown");
    gtk_widget_class_set_accessible_role(GTK_WIDGET_CLASS(g_class), GTK_ACCESSIBLE_ROLE_TIMER);
}

}  // namespace

CountdownView::CountdownView(std::function<std::int64_t()> precise_now)
    : Glib::ObjectBase("GtaviCountdownView"),
      Glib::ExtraClassInit(class_init),
      precise_now_(std::move(precise_now)) {
    set_halign(Gtk::Align::CENTER);
    units_[0].label = "Days";
    units_[1].label = "Hours";
    units_[2].label = "Minutes";
    units_[3].label = "Seconds";
}

CountdownView::~CountdownView() { remove_tick(); }

void CountdownView::set_parts(const Breakdown& parts, std::int64_t target) {
    target_ = target;
    std::array<std::int64_t, 4> values{parts.days, parts.hours, parts.minutes, parts.seconds};
    std::array<const char*, 4> singular{"Day", "Hour", "Minute", "Second"};
    std::array<const char*, 4> plural{"Days", "Hours", "Minutes", "Seconds"};
    auto now = g_get_monotonic_time();
    bool animate = have_parts_ && animations_enabled();
    bool resized = false;

    for (std::size_t i = 0; i < units_.size(); ++i) {
        auto& unit = units_[i];
        auto text = pad(values[i]);
        unit.label = values[i] == 1 ? singular[i] : plural[i];
        if (text.size() != unit.digits.size()) resized = true;
        unit.digits.resize(text.size());
        for (std::size_t position = 0; position < text.size(); ++position) {
            // digits[0] is the rightmost figure.
            auto& digit = unit.digits[position];
            char value = text[text.size() - 1 - position];
            if (!have_parts_ || digit.value == 0) {
                digit = {value, 0, 0};
            } else if (value != digit.value) {
                digit.previous = animate ? digit.value : 0;
                digit.value = value;
                digit.changed = now;
            }
        }
        unit.text = text;
    }
    have_parts_ = true;

    // The animated figures are hidden from assistive technology; this is what is read out.
    auto spoken = std::format("{} days, {} hours, {} minutes{}", parts.days, parts.hours, parts.minutes,
                              parts.elapsed ? " since release" : " remaining");
    gtk_accessible_update_property(GTK_ACCESSIBLE(gobj()), GTK_ACCESSIBLE_PROPERTY_LABEL, spoken.c_str(), -1);

    if (resized) queue_resize();
    update_tick();
    queue_draw();
}

void CountdownView::set_show_milliseconds(bool show) {
    if (show == show_millis_) return;
    show_millis_ = show;
    update_tick();
    queue_draw();
}

void CountdownView::set_figure_size(double size, double gap) {
    if (std::abs(size - size_) < 0.01 && std::abs(gap - gap_) < 0.0001) return;
    size_ = size;
    gap_ = gap;
    queue_resize();
}

void CountdownView::set_colors(const UiColors& colors) {
    colors_ = colors;
    queue_draw();
}

CountdownView::Geometry CountdownView::geometry() const {
    Geometry g{};
    g.size = size_;
    g.gap = size_ * gap_;
    g.label_size = std::clamp(size_ * 0.085, 10.0, 14.4);
    g.label_gap = size_ * 0.06;
    g.separator = size_ * 0.09;
    auto self = const_cast<CountdownView*>(this);
    auto font = label_font(g.label_size);
    g.width = 0;
    for (std::size_t i = 0; i < units_.size(); ++i) {
        const auto& unit = units_[i];
        auto layout = drawing::layout(*self, upper(unit.label), font, {.letter_spacing_px = g.label_size * 0.16});
        auto size = drawing::size_of(layout);
        g.label_height = std::max(g.label_height, size.height);
        double digits = static_cast<double>(std::max<std::size_t>(unit.digits.size(), 2)) * size_ * 0.5;
        g.unit_widths[i] = std::max(digits, size.width);
        g.width += g.unit_widths[i];
    }
    g.width += 3 * g.separator + 6 * g.gap;
    g.height = size_ + g.label_gap + g.label_height;
    return g;
}

Gtk::SizeRequestMode CountdownView::get_request_mode_vfunc() const { return Gtk::SizeRequestMode::CONSTANT_SIZE; }

// The figures are sized from the window, so their natural width must not
// become a minimum that stops the window shrinking: the minimum is the
// smallest size the layout ever asks for, and a narrower allocation scales
// the drawing down.
void CountdownView::measure_vfunc(Gtk::Orientation orientation, int, int& minimum, int& natural,
                                  int& minimum_baseline, int& natural_baseline) const {
    auto g = geometry();
    if (orientation == Gtk::Orientation::HORIZONTAL) {
        natural = static_cast<int>(std::ceil(g.width));
        minimum = std::min(natural, static_cast<int>(std::ceil(g.width * min_size / g.size)));
    } else {
        minimum = natural = static_cast<int>(std::ceil(g.height));
    }
    minimum_baseline = natural_baseline = -1;
}

void CountdownView::on_map() {
    Gtk::Widget::on_map();
    update_tick();
}

void CountdownView::on_unmap() {
    remove_tick();
    Gtk::Widget::on_unmap();
}

// Frames are only requested while something moves: the colons (when
// animations are on), a rolling figure or the milliseconds readout.
void CountdownView::update_tick() {
    bool rolling = false;
    auto now = g_get_monotonic_time();
    for (const auto& unit : units_) {
        for (const auto& digit : unit.digits) {
            if (digit.previous != 0 && now - digit.changed < roll_us) rolling = true;
        }
    }
    bool needed = get_mapped() && (show_millis_ || rolling || animations_enabled());
    if (needed && tick_ == 0) {
        tick_ = add_tick_callback([this](const Glib::RefPtr<Gdk::FrameClock>&) {
            queue_draw();
            return true;
        });
    } else if (!needed) {
        remove_tick();
    }
}

void CountdownView::remove_tick() {
    if (tick_ != 0) remove_tick_callback(tick_);
    tick_ = 0;
}

void CountdownView::snapshot_vfunc(const Glib::RefPtr<Gtk::Snapshot>& snapshot) {
    auto* s = snapshot->gobj();
    auto g = geometry();
    auto ink = get_color();
    auto clock = get_frame_clock();
    gint64 now = clock ? clock->get_frame_time() : g_get_monotonic_time();
    bool animated = animations_enabled();

    // Narrower than natural only while the window is shrinking past a
    // breakpoint; the next layout pass resizes the figures.
    double fit = std::min(1.0, get_width() / g.width);
    if (fit < 1) {
        graphene_point_t origin{static_cast<float>((get_width() - g.width * fit) / 2), 0};
        gtk_snapshot_translate(s, &origin);
        gtk_snapshot_scale(s, static_cast<float>(fit), static_cast<float>(fit));
    }
    double x = fit < 1 ? 0 : (get_width() - g.width) / 2;
    auto font = figure_font(g.size);
    const char* features = "tnum 1, lnum 1";

    // Two shadows: a tight one for edge contrast, a wide one for depth.
    std::array<GskShadow, 2> shadows{{
        {drawing::rgba(colors_.scrim, 0.55), 0, static_cast<float>(g.size * 0.02), static_cast<float>(g.size * 0.03)},
        {drawing::rgba(colors_.scrim, 0.5), 0, static_cast<float>(g.size * 0.1), static_cast<float>(g.size * 0.5)},
    }};

    double pulse = 0.5;
    if (animated) {
        double phase = std::fmod(static_cast<double>(now), pulse_us) / (pulse_us / 2);
        double t = phase < 1 ? phase : 2 - phase;
        pulse = 0.5 - 0.28 * ease_in_out(t);
    }

    double seconds_right = 0;
    gtk_snapshot_push_shadow(s, shadows.data(), shadows.size());
    for (std::size_t i = 0; i < units_.size(); ++i) {
        const auto& unit = units_[i];
        if (i > 0) {
            // The colon is drawn rather than typeset: two round dots, centred on
            // the height of the figures.
            x += g.gap;
            double r = g.separator / 2;
            auto dot = drawing::rgba(ink, pulse);
            Gdk::RGBA color{dot.red, dot.green, dot.blue, dot.alpha};
            drawing::circle(s, x + r, g.size * 0.34, r, color);
            drawing::circle(s, x + r, g.size * 0.68, r, color);
            x += g.separator + g.gap;
        }

        double cell = g.size * 0.5;
        double digits_width = static_cast<double>(unit.digits.size()) * cell;
        double left = x + (g.unit_widths[i] - digits_width) / 2;
        for (std::size_t position = 0; position < unit.digits.size(); ++position) {
            const auto& digit = unit.digits[unit.digits.size() - 1 - position];
            double cx = left + static_cast<double>(position) * cell;
            auto draw_glyph = [&](char value, double offset, double opacity) {
                auto layout = drawing::layout(*this, Glib::ustring(1, value), font, {.features = features});
                auto size = drawing::size_of(layout);
                if (opacity < 1) gtk_snapshot_push_opacity(s, opacity);
                drawing::text(s, layout, cx + (cell - size.width) / 2, (g.size - size.height) / 2 + offset, ink);
                if (opacity < 1) gtk_snapshot_pop(s);
            };
            double progress = digit.previous != 0 ? static_cast<double>(now - digit.changed) / roll_us : 1;
            if (progress >= 1) {
                draw_glyph(digit.value, 0, 1);
            } else {
                double e = ease_out(std::clamp(progress, 0.0, 1.0));
                draw_glyph(digit.previous, g.size * roll_distance * e, 1 - e);
                draw_glyph(digit.value, -g.size * roll_distance * (1 - e), e);
            }
        }
        if (i == 3) seconds_right = left + digits_width;
        x += g.unit_widths[i];
    }
    gtk_snapshot_pop(s);

    // Unit labels, centred under their figures.
    std::array<GskShadow, 1> label_shadow{{{drawing::rgba(colors_.scrim, 0.8), 0, 1, 8}}};
    gtk_snapshot_push_shadow(s, label_shadow.data(), label_shadow.size());
    auto lfont = label_font(g.label_size);
    double lx = fit < 1 ? 0 : (get_width() - g.width) / 2;
    auto soft = drawing::rgba(ink, 0.76);
    Gdk::RGBA soft_color{soft.red, soft.green, soft.blue, soft.alpha};
    for (std::size_t i = 0; i < units_.size(); ++i) {
        if (i > 0) lx += 2 * g.gap + g.separator;
        auto layout = drawing::layout(*this, upper(units_[i].label), lfont, {.letter_spacing_px = g.label_size * 0.16});
        auto size = drawing::size_of(layout);
        // Letter spacing trails the last letter; it is left out of the centring.
        double width = size.width - g.label_size * 0.16;
        drawing::text(s, layout, lx + (g.unit_widths[i] - width) / 2, g.size + g.label_gap, soft_color);
        lx += g.unit_widths[i];
    }
    gtk_snapshot_pop(s);

    // Milliseconds: hung to the right of the seconds, on the baseline of the
    // figures and out of the flow, so that toggling it never moves the digits.
    if (show_millis_ && precise_now_) {
        auto parts = countdown::breakdown(precise_now_(), target_);
        double m = g.size * 0.26;
        auto layout = drawing::layout(*this, std::format(".{:03}", parts.milliseconds), millis_font(m));
        auto size = drawing::size_of(layout);
        double top = g.size - 0.52 * m - m;
        gtk_snapshot_push_shadow(s, shadows.data(), shadows.size());
        drawing::text(s, layout, seconds_right + 0.12 * m, top + (m - size.height) / 2, colors_.accent);
        gtk_snapshot_pop(s);
    }

    update_tick();
}

}  // namespace gtavi
