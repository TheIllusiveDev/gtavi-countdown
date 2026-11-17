#pragma once
// The large countdown: days, hours, minutes and seconds, with drawn colons
// that breathe, figures that roll when they change, and an optional
// milliseconds readout hung to the right of the seconds.

#include <glibmm/extraclassinit.h>
#include <gtkmm/widget.h>

#include <array>
#include <cstdint>
#include <functional>
#include <string>
#include <vector>

#include "countdown.hpp"
#include "ui_colors.hpp"

namespace gtavi {

class CountdownView : public Glib::ExtraClassInit, public Gtk::Widget {
public:
    // `precise_now` reads the corrected clock; the milliseconds readout uses it
    // on every frame so that it agrees with the seconds.
    explicit CountdownView(std::function<std::int64_t()> precise_now);
    ~CountdownView() override;

    void set_parts(const Breakdown& parts, std::int64_t target);
    void set_show_milliseconds(bool show);
    // Height of the figures in pixels, and the gap between units as a fraction of it.
    void set_figure_size(double size, double gap);
    void set_colors(const UiColors& colors);

protected:
    Gtk::SizeRequestMode get_request_mode_vfunc() const override;
    void measure_vfunc(Gtk::Orientation orientation, int for_size, int& minimum, int& natural,
                       int& minimum_baseline, int& natural_baseline) const override;
    void snapshot_vfunc(const Glib::RefPtr<Gtk::Snapshot>& snapshot) override;
    void on_map() override;
    void on_unmap() override;

private:
    // One figure. A change rolls the old glyph out downwards while the new
    // one rolls in from above.
    struct Digit {
        char value = '0';
        char previous = 0;  // 0 when not rolling.
        gint64 changed = 0;  // Monotonic microseconds.
    };

    struct Unit {
        std::string label;
        std::vector<Digit> digits;  // Keyed from the right, so 100 days to 99 drops a figure.
        std::string text;
    };

    struct Geometry {
        double size;
        double gap;
        double label_size;
        double label_gap;
        double label_height;
        double separator;
        std::array<double, 4> unit_widths;
        double width;
        double height;
    };

    Geometry geometry() const;
    void update_tick();
    void remove_tick();

    std::function<std::int64_t()> precise_now_;
    std::array<Unit, 4> units_;
    bool have_parts_ = false;
    bool show_millis_ = false;
    std::int64_t target_ = 0;
    double size_ = 96;
    double gap_ = 0.07;
    UiColors colors_;
    guint tick_ = 0;
};

}  // namespace gtavi
