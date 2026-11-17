#pragma once
// Theme colours the custom-drawn widgets need. They mirror the custom
// properties in style.css, which drive everything drawn by GTK itself.

#include <gdkmm/rgba.h>

#include "themes.hpp"

namespace gtavi {

struct UiColors {
    Gdk::RGBA ink{"#ffffff"};
    Gdk::RGBA accent;
    Gdk::RGBA accent_2;
    Gdk::RGBA panel_solid;
    Gdk::RGBA scrim;  // Opaque; callers apply their own alpha.
};

inline UiColors ui_colors(Theme theme) {
    switch (theme) {
        case Theme::midnight:
            return {.accent = Gdk::RGBA{"#6fd3e8"}, .accent_2 = Gdk::RGBA{"#7a9dff"},
                    .panel_solid = Gdk::RGBA{"#06122b"}, .scrim = Gdk::RGBA{"rgb(2,8,22)"}};
        case Theme::dawn:
            return {.accent = Gdk::RGBA{"#ffc27a"}, .accent_2 = Gdk::RGBA{"#ff8fa3"},
                    .panel_solid = Gdk::RGBA{"#102250"}, .scrim = Gdk::RGBA{"rgb(10,20,52)"}};
        case Theme::dusk:
            break;
    }
    return {.accent = Gdk::RGBA{"#ff8a5c"}, .accent_2 = Gdk::RGBA{"#ff4f9a"}, .panel_solid = Gdk::RGBA{"#170b33"},
            .scrim = Gdk::RGBA{"rgb(18,8,44)"}};
}

inline Gdk::RGBA with_alpha(Gdk::RGBA color, double alpha) {
    color.set_alpha(alpha);
    return color;
}

}  // namespace gtavi
