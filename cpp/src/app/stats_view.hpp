#pragma once
// The remaining time in other units: sleeps, weekends and totals.

#include <gtkmm/box.h>
#include <gtkmm/grid.h>
#include <gtkmm/label.h>

#include <array>
#include <cstdint>
#include <string_view>

namespace gtavi {

class StatsView : public Gtk::Grid {
public:
    // Five across; two then three; or two columns with the last tile on its own row.
    enum class Layout { wide, medium, narrow };

    StatsView();

    void set_data(std::int64_t now, std::int64_t target, std::string_view zone);
    void set_layout(Layout layout);

private:
    // Owned here rather than by the grid, because re-placing a tile removes it from the grid.
    struct Tile {
        Gtk::Box box{Gtk::Orientation::VERTICAL, 2};
        Gtk::Label value;
        Gtk::Label caption;
    };

    void place();

    std::array<Tile, 5> tiles_;
    Layout layout_ = Layout::wide;
    bool placed_ = false;
};

}  // namespace gtavi
