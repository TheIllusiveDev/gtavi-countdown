#pragma once
// The settings sheet: a panel that slides in from the right over a scrim,
// with the time zone picker, display options, alerts and desktop options.

#include <gtkmm/box.h>
#include <gtkmm/button.h>
#include <gtkmm/label.h>
#include <gtkmm/listbox.h>
#include <gtkmm/overlay.h>
#include <gtkmm/revealer.h>
#include <gtkmm/scrolledwindow.h>
#include <gtkmm/searchentry.h>
#include <gtkmm/switch.h>
#include <gtkmm/togglebutton.h>

#include <array>
#include <functional>
#include <optional>
#include <string>
#include <vector>

#include "settings.hpp"

namespace gtavi {

// A filterable list of every time zone, in the order the release reaches them.
class ZonePicker : public Gtk::Box {
public:
    explicit ZonePicker(std::function<void(std::optional<std::string>)> on_select);

    void set_value(const std::optional<std::string>& zone);
    Gtk::SearchEntry& search() { return search_; }

private:
    bool filter(Gtk::ListBoxRow* row);
    void on_query_changed();

    std::function<void(std::optional<std::string>)> on_select_;
    Gtk::SearchEntry search_;
    Gtk::ScrolledWindow scroller_;
    Gtk::ListBox list_;
    Gtk::Label empty_;
    std::vector<std::string> needles_;
    std::optional<std::string> value_;
};

class SettingsPanel : public Gtk::Overlay {
public:
    struct Callbacks {
        std::function<void(const Settings&)> change;  // Apply these settings.
        std::function<void()> sync;                   // Measure the clock now.
        std::function<void(bool)> notify;             // The user flipped notifications.
        std::function<void()> closed;                 // The panel was closed.
    };

    SettingsPanel(Callbacks callbacks, bool can_keep_above);

    void set_settings(const Settings& settings);
    void set_sync(const std::string& text, bool busy);
    void set_sheet_width(int width);

    void open();
    void close();
    bool is_open() const { return open_; }

private:
    Gtk::Box& section(const char* title);
    Gtk::Box& switch_row(const char* title, const char* hint, Gtk::Switch& control);
    void change(const std::function<void(Settings&)>& edit);

    Callbacks callbacks_;
    Settings settings_;
    bool open_ = false;
    bool updating_ = false;

    Gtk::Revealer scrim_revealer_;
    Gtk::Box scrim_;
    Gtk::Revealer sheet_revealer_;
    Gtk::Box sheet_{Gtk::Orientation::VERTICAL};
    Gtk::Button close_button_;
    Gtk::ScrolledWindow body_scroller_;
    Gtk::Box body_{Gtk::Orientation::VERTICAL};

    ZonePicker zones_;
    std::array<Gtk::ToggleButton, 3> theme_buttons_;
    std::array<Gtk::ToggleButton, 3> effect_buttons_;
    Gtk::Switch millis_;
    Gtk::Switch notify_;
    Gtk::Switch on_top_;
    Gtk::Switch network_time_;
    Gtk::Label sync_text_;
    Gtk::Button sync_button_;
};

}  // namespace gtavi
