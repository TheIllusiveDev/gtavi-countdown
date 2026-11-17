#include "window_state.hpp"

#include <gtkmm/window.h>
#include <nlohmann/json.hpp>

#include <fstream>

#include "app_paths.hpp"

namespace gtavi::window_state {

std::optional<State> load() {
    try {
        std::ifstream in(app_paths::window_state_file());
        if (!in) return std::nullopt;
        auto json = nlohmann::json::parse(in, nullptr, /*allow_exceptions=*/false);
        if (!json.is_object()) return std::nullopt;
        State state{json.value("width", 0), json.value("height", 0), json.value("maximized", false)};
        // Ignore anything that would leave the window unusable.
        if (state.width < 360 || state.width > 16384 || state.height < 240 || state.height > 16384) return std::nullopt;
        return state;
    } catch (const std::exception&) {
        return std::nullopt;
    }
}

void save(const Gtk::Window& window) {
    try {
        int width = 0;
        int height = 0;
        window.get_default_size(width, height);
        nlohmann::ordered_json json{{"width", width}, {"height", height}, {"maximized", window.is_maximized()}};
        std::filesystem::create_directories(app_paths::config_dir());
        std::ofstream(app_paths::window_state_file()) << json.dump(2) << '\n';
    } catch (const std::exception&) {
        // Losing the window size is not worth failing over.
    }
}

}  // namespace gtavi::window_state
