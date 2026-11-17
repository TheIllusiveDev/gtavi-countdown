#include "stats_view.hpp"

#include <gtk/gtk.h>
#include <gtkmm/box.h>

#include "countdown.hpp"
#include "wording.hpp"

namespace gtavi {

StatsView::StatsView() {
    add_css_class("stats");
    set_column_homogeneous(true);
    gtk_accessible_update_property(GTK_ACCESSIBLE(gobj()), GTK_ACCESSIBLE_PROPERTY_LABEL,
                                   "The remaining time in other units", -1);
    for (auto& tile : tiles_) {
        tile.box.add_css_class("tile");
        tile.value.add_css_class("value");
        tile.value.set_halign(Gtk::Align::START);
        tile.caption.add_css_class("label-caps");
        // Ellipsizing keeps the captions from setting a minimum width that
        // would stop the window shrinking to the layouts that give them more
        // room. The caption fills its tile rather than taking its natural
        // width, which GTK measures without the trailing letter spacing and
        // would otherwise cut short.
        tile.caption.set_ellipsize(Pango::EllipsizeMode::END);
        tile.caption.set_xalign(0);
        tile.box.append(tile.value);
        tile.box.append(tile.caption);
    }
    place();
}

void StatsView::set_data(std::int64_t now, std::int64_t target, std::string_view zone) {
    auto t = countdown::totals(now, target, zone);
    auto set = [](Tile& tile, const char* caption, std::int64_t value) {
        tile.caption.set_text(caption);
        tile.value.set_text(wording::integer(value));
    };
    set(tiles_[0], t.sleeps == 1 ? "SLEEP" : "SLEEPS", t.sleeps);
    set(tiles_[1], t.weekends == 1 ? "WEEKEND" : "WEEKENDS", t.weekends);
    set(tiles_[2], "TOTAL HOURS", t.hours);
    set(tiles_[3], "TOTAL MINUTES", t.minutes);
    set(tiles_[4], "TOTAL SECONDS", t.seconds);
}

void StatsView::set_layout(Layout layout) {
    if (layout == layout_ && placed_) return;
    layout_ = layout;
    place();
}

void StatsView::place() {
    for (auto& tile : tiles_) {
        if (tile.box.get_parent() != nullptr) remove(tile.box);
    }
    struct Cell {
        int column, row, span;
    };
    std::array<Cell, 5> cells{};
    switch (layout_) {
        case Layout::wide:
            cells = {{{0, 0, 1}, {1, 0, 1}, {2, 0, 1}, {3, 0, 1}, {4, 0, 1}}};
            break;
        case Layout::medium:
            // Six columns: the first two tiles take three each, the others two.
            cells = {{{0, 0, 3}, {3, 0, 3}, {0, 1, 2}, {2, 1, 2}, {4, 1, 2}}};
            break;
        case Layout::narrow:
            // The largest figure, total seconds, takes the full last row.
            cells = {{{0, 0, 1}, {1, 0, 1}, {0, 1, 1}, {1, 1, 1}, {0, 2, 2}}};
            break;
    }
    for (std::size_t i = 0; i < tiles_.size(); ++i) {
        auto& box = tiles_[i].box;
        const auto& cell = cells[i];
        // Hairline dividers on the inner edges only.
        if (cell.column > 0) box.add_css_class("left-edge"); else box.remove_css_class("left-edge");
        if (cell.row > 0) box.add_css_class("top-edge"); else box.remove_css_class("top-edge");
        attach(box, cell.column, cell.row, cell.span, 1);
    }
    placed_ = true;
}

}  // namespace gtavi
