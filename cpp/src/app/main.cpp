// GTA VI Countdown: the GTK 4 desktop app.

#include <gtkmm/application.h>
#include <gtkmm/cssprovider.h>
#include <gtkmm/settings.h>
#include <pango/pangocairo.h>
#include <pango/pangofc-fontmap.h>

#include <filesystem>
#include <iostream>

#include "app_paths.hpp"
#include "main_window.hpp"

namespace {

// Makes the bundled fonts available to this process only. They are TrueType:
// Pango reads font data through HarfBuzz, which does not decode WOFF2.
void load_fonts() {
    auto dir = gtavi::app_paths::fonts_dir();
    if (!dir) return;
    auto* map = pango_cairo_font_map_get_default();
    std::error_code error;
    for (const auto& file : std::filesystem::directory_iterator(*dir, error)) {
        if (file.path().extension() != ".ttf") continue;
        GError* failure = nullptr;
        if (!pango_font_map_add_font_file(map, file.path().c_str(), &failure)) {
            std::cerr << "Could not load " << file.path() << ": " << (failure ? failure->message : "unknown error")
                      << '\n';
            g_clear_error(&failure);
        }
    }
}

void load_styles() {
    auto provider = Gtk::CssProvider::create();
    provider->load_from_resource("/app/gtavi/countdown/style.css");
    Gtk::StyleProvider::add_provider_for_display(Gdk::Display::get_default(), provider,
                                                 GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);
    // The app is dark whatever the desktop theme; built-in widgets follow.
    if (auto settings = Gtk::Settings::get_default()) {
#if GTK_CHECK_VERSION(4, 20, 0)
        g_object_set(settings->gobj(), "gtk-interface-color-scheme", GTK_INTERFACE_COLOR_SCHEME_DARK, nullptr);
#else
        settings->property_gtk_application_prefer_dark_theme() = true;
#endif
    }
}

}  // namespace

int main(int argc, char* argv[]) {
    auto app = Gtk::Application::create(APP_ID);
    gtavi::MainWindow* window = nullptr;

    app->signal_startup().connect([] {
        load_fonts();
        load_styles();
    });
    app->signal_activate().connect([&] {
        // A second launch raises the window that is already open.
        if (window == nullptr) {
            window = new gtavi::MainWindow(app);
            app->add_window(*window);
            window->signal_hide().connect([&] {
                delete window;
                window = nullptr;
            });
        }
        window->present();
    });
    return app->run(argc, argv);
}
