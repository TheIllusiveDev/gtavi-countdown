#pragma once
// Progress of the wait since the first trailer: a header with the share that
// is over, and a drawn track with the milestones and their legend.

#include <glibmm/extraclassinit.h>
#include <gtkmm/box.h>
#include <gtkmm/label.h>
#include <gtkmm/widget.h>

#include <cstdint>
#include <string>
#include <vector>

#include "ui_colors.hpp"

namespace gtavi {

class TimelineTrack : public Glib::ExtraClassInit, public Gtk::Widget {
public:
    TimelineTrack();

    void set_data(std::int64_t now, std::int64_t target);
    void set_colors(const UiColors& colors);
    // Narrow windows show only the first and last legend entries.
    void set_compact(bool compact);

protected:
    Gtk::SizeRequestMode get_request_mode_vfunc() const override;
    void measure_vfunc(Gtk::Orientation orientation, int for_size, int& minimum, int& natural,
                       int& minimum_baseline, int& natural_baseline) const override;
    void snapshot_vfunc(const Glib::RefPtr<Gtk::Snapshot>& snapshot) override;

private:
    struct Mark {
        std::string title;
        std::string detail;
        std::string date_text;
        double position;
    };

    double mark_x(std::size_t index) const;
    bool on_query_tooltip(int x, int y, bool keyboard, const Glib::RefPtr<Gtk::Tooltip>& tooltip);

    std::vector<Mark> marks_;
    std::string release_date_;
    double progress_ = 0;
    bool compact_ = false;
    UiColors colors_;
};

class TimelineView : public Gtk::Box {
public:
    TimelineView();

    void set_data(std::int64_t now, std::int64_t target);
    void set_colors(const UiColors& colors) { track_.set_colors(colors); }
    void set_compact(bool compact) { track_.set_compact(compact); }

private:
    Gtk::Label percent_;
    TimelineTrack track_;
    std::string percent_text_;
};

}  // namespace gtavi
