#include "timeline_view.hpp"

#include <gdkmm/frameclock.h>
#include <gtk/gtk.h>
#include <gtkmm/snapshot.h>
#include <gtkmm/tooltip.h>

#include <algorithm>
#include <cmath>
#include <format>

#include "countdown.hpp"
#include "drawing.hpp"
#include "release.hpp"
#include "wording.hpp"

namespace gtavi {

namespace {

constexpr double mark_radius = 5;
constexpr double track_height = 6;
constexpr double legend_gap = 10;
constexpr double legend_height = 38;
constexpr double line_height = 1.3;

void class_init(void* g_class, void*) { gtk_widget_class_set_css_name(GTK_WIDGET_CLASS(g_class), "timeline"); }

std::string release_date_text() {
    auto date = std::format("{:04}-{:02}-{:02}", release::year, release::month, release::day);
    return wording::short_date(release::milestone_instant(date));
}

}  // namespace

// ------------------------------------------------------------------ track

TimelineTrack::TimelineTrack()
    : Glib::ObjectBase("GtaviTimelineTrack"), Glib::ExtraClassInit(class_init), release_date_(release_date_text()) {
    set_hexpand(true);
    set_has_tooltip(true);
    signal_query_tooltip().connect(sigc::mem_fun(*this, &TimelineTrack::on_query_tooltip), false);
    gtk_accessible_update_property(GTK_ACCESSIBLE(gobj()), GTK_ACCESSIBLE_PROPERTY_LABEL,
                                   "Share of the wait that is over", -1);
}

void TimelineTrack::set_data(std::int64_t now, std::int64_t target) {
    progress_ = countdown::wait_progress(now, target);
    marks_.clear();
    for (const auto& milestone : release::milestones) {
        auto instant = release::milestone_instant(milestone.date);
        marks_.push_back({std::string{milestone.title}, std::string{milestone.detail}, wording::short_date(instant),
                          countdown::timeline_position(instant, target)});
    }
    queue_draw();
}

void TimelineTrack::set_colors(const UiColors& colors) {
    colors_ = colors;
    queue_draw();
}

void TimelineTrack::set_compact(bool compact) {
    if (compact == compact_) return;
    compact_ = compact;
    queue_draw();
}

Gtk::SizeRequestMode TimelineTrack::get_request_mode_vfunc() const { return Gtk::SizeRequestMode::CONSTANT_SIZE; }

void TimelineTrack::measure_vfunc(Gtk::Orientation orientation, int, int& minimum, int& natural,
                                  int& minimum_baseline, int& natural_baseline) const {
    if (orientation == Gtk::Orientation::HORIZONTAL) {
        minimum = 120;
        natural = 480;
    } else {
        minimum = natural = static_cast<int>(2 * mark_radius + legend_gap + legend_height);
    }
    minimum_baseline = natural_baseline = -1;
}

// Centre of a milestone dot. The first dot sits inside the start of the track.
double TimelineTrack::mark_x(std::size_t index) const {
    double x = marks_[index].position * get_width();
    return index == 0 ? x + mark_radius : x;
}

bool TimelineTrack::on_query_tooltip(int x, int y, bool, const Glib::RefPtr<Gtk::Tooltip>& tooltip) {
    if (y > 2 * mark_radius + 6) return false;
    for (std::size_t i = 0; i < marks_.size(); ++i) {
        if (std::abs(x - mark_x(i)) <= mark_radius + 3) {
            tooltip->set_text(marks_[i].date_text + ": " + marks_[i].detail);
            return true;
        }
    }
    return false;
}

void TimelineTrack::snapshot_vfunc(const Glib::RefPtr<Gtk::Snapshot>& snapshot) {
    auto* s = snapshot->gobj();
    double width = get_width();
    auto ink = get_color();
    double cy = mark_radius;
    auto scrim_ring = drawing::rgba(colors_.scrim, 0.55);
    Gdk::RGBA scrim{scrim_ring.red, scrim_ring.green, scrim_ring.blue, scrim_ring.alpha};

    // Track and fill. The fill is the full gradient compressed into the share
    // that is over, as the scaled element of the web version is.
    auto track = drawing::rect(0, cy - track_height / 2, width, track_height);
    drawing::rounded_rect(s, track, track_height / 2, Gdk::RGBA{"rgba(255,255,255,0.16)"});
    double fill_width = width * std::clamp(progress_, 0.0, 1.0);
    if (fill_width > 0) {
        auto fill = drawing::rect(0, cy - track_height / 2, fill_width, track_height);
        graphene_point_t start = {0, 0};
        graphene_point_t end = {static_cast<float>(fill_width), 0};
        std::array<GskColorStop, 2> stops{{{0, *colors_.accent_2.gobj()}, {1, *colors_.accent.gobj()}}};
        drawing::push_rounded(s, fill, track_height / 2);
        gtk_snapshot_append_linear_gradient(s, &fill, &start, &end, stops.data(), stops.size());
        gtk_snapshot_pop(s);
    }

    for (std::size_t i = 0; i < marks_.size(); ++i) {
        double cx = mark_x(i);
        drawing::circle(s, cx, cy, mark_radius + 3, scrim);
        drawing::circle(s, cx, cy, mark_radius, ink);
    }
    // The release: a hollow ring that fills once it is reached.
    double end_x = width - mark_radius;
    drawing::circle(s, end_x, cy, mark_radius + 3, scrim);
    drawing::circle(s, end_x, cy, mark_radius, colors_.accent);
    if (progress_ < 1) drawing::circle(s, end_x, cy, mark_radius - 2, colors_.panel_solid);

    // Legend: centred under the dots, the first aligned left and the release right.
    auto title_font = drawing::font(drawing::sans_family, 12, 600);
    auto date_font = drawing::font(drawing::mono_family, 11, 400);
    auto faint = drawing::rgba(ink, 0.56);
    Gdk::RGBA faint_color{faint.red, faint.green, faint.blue, faint.alpha};
    double top = 2 * mark_radius + legend_gap;
    auto entry = [&](const std::string& title, const std::string& date, double x, int align) {
        auto title_layout = drawing::layout(*this, title, title_font);
        auto date_layout = drawing::layout(*this, date, date_font);
        auto title_size = drawing::size_of(title_layout);
        auto date_size = drawing::size_of(date_layout);
        auto place = [&](double w) { return align < 0 ? x : align > 0 ? x - w : x - w / 2; };
        double title_line = 12 * line_height;
        drawing::text(s, title_layout, place(title_size.width), top + (title_line - title_size.height) / 2, ink);
        drawing::text(s, date_layout, place(date_size.width),
                      top + title_line + (11 * line_height - date_size.height) / 2, faint_color);
    };
    for (std::size_t i = 0; i < marks_.size(); ++i) {
        // The labels would collide on narrow windows; the dots keep their
        // tooltips and the first and last labels anchor the scale.
        if (i > 0 && compact_) continue;
        entry(marks_[i].title, marks_[i].date_text, marks_[i].position * width, i == 0 ? -1 : 0);
    }
    entry("Release", release_date_, width, 1);
}

// ------------------------------------------------------------------- view

TimelineView::TimelineView() : Gtk::Box(Gtk::Orientation::VERTICAL) {
    add_css_class("timeline");
    gtk_accessible_update_property(GTK_ACCESSIBLE(gobj()), GTK_ACCESSIBLE_PROPERTY_LABEL,
                                   "Progress of the wait since the first trailer", -1);

    auto* header = Gtk::make_managed<Gtk::Box>(Gtk::Orientation::HORIZONTAL);
    header->set_margin_bottom(10);
    auto* caption = Gtk::make_managed<Gtk::Label>("THE WAIT SINCE TRAILER 1");
    caption->add_css_class("label-caps");
    caption->set_hexpand(true);
    caption->set_halign(Gtk::Align::START);
    caption->set_valign(Gtk::Align::BASELINE_CENTER);
    percent_.add_css_class("percent");
    percent_.add_css_class("mono");
    percent_.set_valign(Gtk::Align::BASELINE_CENTER);
    header->append(*caption);
    header->append(percent_);

    append(*header);
    append(track_);
}

void TimelineView::set_data(std::int64_t now, std::int64_t target) {
    track_.set_data(now, target);
    double progress = countdown::wait_progress(now, target);
    // Two decimals while counting; the bar itself moves about 0.1% per day.
    auto text = progress >= 1 ? std::string{"100"} : std::format("{:.2f}", std::floor(progress * 10000) / 100);
    if (text == percent_text_) return;
    percent_text_ = text;
    percent_.set_markup("<span weight=\"600\" foreground=\"#ffffff\">" + text + "</span>% over");
}

}  // namespace gtavi
