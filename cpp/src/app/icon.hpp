#pragma once
// A stroked line icon drawn from SVG path data on a 24 by 24 grid, in the
// widget's CSS colour, like the inline SVG icons of the web version.

#include <gtkmm/widget.h>

#include <string>

namespace gtavi {

enum class IconName { sliders, expand, compress, pin, calendar, copy, close, chevron, check, sync, search };

class Icon : public Gtk::Widget {
public:
    explicit Icon(IconName name, int size = 18);
    ~Icon() override;

    void set_icon(IconName name);

protected:
    Gtk::SizeRequestMode get_request_mode_vfunc() const override;
    void measure_vfunc(Gtk::Orientation orientation, int for_size, int& minimum, int& natural,
                       int& minimum_baseline, int& natural_baseline) const override;
    void snapshot_vfunc(const Glib::RefPtr<Gtk::Snapshot>& snapshot) override;

private:
    int size_;
    struct _GskPath* path_ = nullptr;
};

}  // namespace gtavi
