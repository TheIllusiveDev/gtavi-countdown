#pragma once
// A container that gives its child the available width up to a maximum and
// centres it, like `width: min(100%, max)` in CSS. A size request would do
// the same at one window size, but it would also stop the window shrinking.

#include <gtkmm/widget.h>

#include <algorithm>

namespace gtavi {

class Clamp : public Gtk::Widget {
public:
    Clamp(Gtk::Widget& child, int maximum) : Glib::ObjectBase("GtaviClamp"), child_(child), maximum_(maximum) {
        child_.set_parent(*this);
    }

    ~Clamp() override { child_.unparent(); }

protected:
    Gtk::SizeRequestMode get_request_mode_vfunc() const override { return child_.get_request_mode(); }

    void measure_vfunc(Gtk::Orientation orientation, int for_size, int& minimum, int& natural,
                       int& minimum_baseline, int& natural_baseline) const override {
        // The child is never given more than the maximum, so its height is
        // measured for at most that width.
        if (orientation == Gtk::Orientation::VERTICAL && for_size > maximum_) for_size = maximum_;
        child_.measure(orientation, for_size, minimum, natural, minimum_baseline, natural_baseline);
        if (orientation == Gtk::Orientation::HORIZONTAL) {
            minimum = std::min(minimum, maximum_);
            natural = std::max(minimum, maximum_);
        }
    }

    void size_allocate_vfunc(int width, int height, int baseline) override {
        int child_width = std::min(width, maximum_);
        Gtk::Allocation allocation{(width - child_width) / 2, 0, child_width, height};
        child_.size_allocate(allocation, baseline);
    }

private:
    Gtk::Widget& child_;
    int maximum_;
};

}  // namespace gtavi
