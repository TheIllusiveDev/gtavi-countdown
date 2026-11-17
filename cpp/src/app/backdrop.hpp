#pragma once
// The backdrop behind the countdown: the animated shader scene when effects
// are on and OpenGL works, otherwise a still gradient, with a scrim on top
// that keeps text readable at the top and bottom edges.

#include <gtkmm/widget.h>

#include <memory>

#include "settings.hpp"
#include "themes.hpp"

namespace gtavi {

// Where the scene is composed around the content, as fractions of the height
// measured from the top: the horizon, and the radius of the sun.
struct Composition {
    float horizon = 0.66f;
    float sun_radius = 0.16f;
    bool operator==(const Composition&) const = default;
};

class Backdrop : public Gtk::Widget {
public:
    Backdrop(Theme theme, EffectLevel effects);
    ~Backdrop() override;

    void set_theme(Theme theme);
    void set_effects(EffectLevel effects);
    void set_composition(Composition composition);

protected:
    void snapshot_vfunc(const Glib::RefPtr<Gtk::Snapshot>& snapshot) override;

private:
    class Scene;

    void mount();
    void unmount();

    Theme theme_;
    EffectLevel effects_;
    Composition composition_;
    std::unique_ptr<Scene> scene_;
};

}  // namespace gtavi
