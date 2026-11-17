#pragma once
// Remembers the size of the window between launches. GTK 4 cannot place a
// window (Wayland does not allow it), so the position is left to the
// window manager.

#include <optional>

namespace Gtk {
class Window;
}

namespace gtavi::window_state {

struct State {
    int width;
    int height;
    bool maximized;
};

std::optional<State> load();
void save(const Gtk::Window& window);

}  // namespace gtavi::window_state
