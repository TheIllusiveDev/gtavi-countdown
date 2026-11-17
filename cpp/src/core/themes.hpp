#pragma once
// Theme palettes for the backdrop shader and the static fallback.

#include <array>
#include <string_view>

namespace gtavi {

enum class Theme { dusk, midnight, dawn };

struct Rgb {
    float r, g, b;
    bool operator==(const Rgb&) const = default;
};

// Colours handed to the backdrop shader.
struct Palette {
    Rgb sky_top, sky_mid, sky_low, sun_hot, sun_cool, glow, water;
    float stars;  // Star brightness from 0 to 1.
    float bands;  // Strength of the slits across the sun from 0 to 1.
};

// A gradient stop: position from 0 (top) to 1 (bottom). A position of
// `horizon` is placed on the horizon line, wherever the layout puts it.
struct GradientStop {
    float position;
    Rgb color;
    bool at_horizon = false;  // position is an offset from the horizon instead.
};

struct ThemeInfo {
    std::string_view label;
    Palette palette;
    // Drawn when the shader is off or OpenGL is unavailable. Mirrors the CSS
    // gradient of the web version.
    std::array<GradientStop, 6> fallback;
    std::array<Rgb, 3> swatch;  // Three colours for the swatch in the settings panel.
};

namespace themes {

inline constexpr std::array<Theme, 3> all{Theme::dusk, Theme::midnight, Theme::dawn};

// Parses "#rrggbb" into normalised components.
constexpr Rgb rgb(std::string_view hex) {
    auto nibble = [](char c) -> int {
        if (c >= '0' && c <= '9') return c - '0';
        if (c >= 'a' && c <= 'f') return c - 'a' + 10;
        if (c >= 'A' && c <= 'F') return c - 'A' + 10;
        throw "invalid hex colour";
    };
    auto byte = [&](std::size_t i) { return float(nibble(hex[i]) * 16 + nibble(hex[i + 1])) / 255.0f; };
    if (hex.size() != 7 || hex[0] != '#') throw "invalid hex colour";
    return {byte(1), byte(3), byte(5)};
}

const ThemeInfo& info(Theme theme);

}  // namespace themes
}  // namespace gtavi
