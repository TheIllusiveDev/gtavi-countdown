#include "icon.hpp"

#include <gdkmm/frameclock.h>
#include <gtk/gtk.h>
#include <gtkmm/snapshot.h>

namespace gtavi {

namespace {

const char* path_data(IconName name) {
    switch (name) {
        case IconName::sliders: return "M4 7h9M17 7h3M4 17h3M11 17h9M15 4.5v5M9 14.5v5";
        case IconName::expand: return "M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5";
        case IconName::compress: return "M9 4v5H4M15 4v5h5M9 20v-5H4M15 20v-5h5";
        case IconName::pin: return "M9 4h6l-1 6 3 3v2H7v-2l3-3-1-6ZM12 15v6";
        case IconName::calendar: return "M5 6h14v14H5zM5 10h14M9 3.5v4M15 3.5v4";
        case IconName::copy: return "M9 9h11v11H9zM5 15H4V4h11v1";
        case IconName::close: return "M6 6l12 12M18 6L6 18";
        case IconName::chevron: return "M8 10l4 4 4-4";
        case IconName::check: return "M5 12.5l4.5 4.5L19 7.5";
        case IconName::sync: return "M20 11a8 8 0 0 0-14.5-4M4 4v3.5h3.5M4 13a8 8 0 0 0 14.5 4M20 20v-3.5h-3.5";
        case IconName::search: return "M11 18a7 7 0 1 1 0-14 7 7 0 0 1 0 14ZM20 20l-4-4";
    }
    return "";
}

}  // namespace

Icon::Icon(IconName name, int size) : Glib::ObjectBase("GtaviIcon"), size_(size) {
    set_icon(name);
    set_halign(Gtk::Align::CENTER);
    set_valign(Gtk::Align::CENTER);
    set_can_target(false);
    gtk_accessible_update_state(GTK_ACCESSIBLE(gobj()), GTK_ACCESSIBLE_STATE_HIDDEN, TRUE, -1);
}

Icon::~Icon() {
    if (path_ != nullptr) gsk_path_unref(path_);
}

void Icon::set_icon(IconName name) {
    if (path_ != nullptr) gsk_path_unref(path_);
    path_ = gsk_path_parse(path_data(name));
    queue_draw();
}

Gtk::SizeRequestMode Icon::get_request_mode_vfunc() const { return Gtk::SizeRequestMode::CONSTANT_SIZE; }

void Icon::measure_vfunc(Gtk::Orientation, int, int& minimum, int& natural, int& minimum_baseline,
                         int& natural_baseline) const {
    minimum = natural = size_;
    minimum_baseline = natural_baseline = -1;
}

void Icon::snapshot_vfunc(const Glib::RefPtr<Gtk::Snapshot>& snapshot) {
    if (path_ == nullptr) return;
    auto* s = snapshot->gobj();
    auto color = get_color();
    float scale = static_cast<float>(size_) / 24.0f;
    gtk_snapshot_save(s);
    gtk_snapshot_scale(s, scale, scale);
    auto* stroke = gsk_stroke_new(1.75f);
    gsk_stroke_set_line_cap(stroke, GSK_LINE_CAP_ROUND);
    gsk_stroke_set_line_join(stroke, GSK_LINE_JOIN_ROUND);
    gtk_snapshot_append_stroke(s, path_, stroke, color.gobj());
    gsk_stroke_free(stroke);
    gtk_snapshot_restore(s);
}

}  // namespace gtavi
