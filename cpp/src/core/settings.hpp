#pragma once
// The settings schema on disk and its validation. The stored form is the same
// JSON object the web and C# versions write, so all of them read each other's data.

#include <optional>
#include <string>
#include <string_view>

#include "themes.hpp"

namespace gtavi {

// Rendering budget for the animated backdrop.
enum class EffectLevel { automatic, low, off };

struct Settings {
    std::optional<std::string> time_zone;  // IANA zone for the release midnight; empty follows the system.
    bool show_milliseconds = false;
    Theme theme = Theme::dusk;
    EffectLevel effects = EffectLevel::automatic;
    bool notify = false;        // Send a notification at the moment of release.
    bool network_time = true;   // Correct the system clock against a network time server.
    bool always_on_top = false; // Keep the window above other windows.
    bool operator==(const Settings&) const = default;
};

namespace settings {

// The lower-case name used in storage: "dusk", "auto".
std::string_view key(Theme theme);
std::string_view key(EffectLevel level);

// Merges untrusted stored JSON over the defaults, discarding invalid fields.
Settings sanitize(std::string_view json);

std::string serialize(const Settings& data);

// The theme after `theme`, wrapping around.
Theme next(Theme theme);

}  // namespace settings
}  // namespace gtavi
