#pragma once
// The whole countdown: header, hero, timeline, stats and footer over the
// backdrop, with the settings sheet and toasts layered on top.

#include <gtkmm/applicationwindow.h>
#include <gtkmm/box.h>
#include <gtkmm/button.h>
#include <gtkmm/cssprovider.h>
#include <gtkmm/label.h>
#include <gtkmm/overlay.h>
#include <gtkmm/revealer.h>
#include <gtkmm/scrolledwindow.h>
#include <gtkmm/togglebutton.h>
#include <gtkmm/windowcontrols.h>
#include <gtkmm/windowhandle.h>

#include <cstdint>
#include <memory>
#include <optional>
#include <string>
#include <variant>

#include "backdrop.hpp"
#include "clamp.hpp"
#include "clock.hpp"
#include "countdown_view.hpp"
#include "icon.hpp"
#include "platform.hpp"
#include "settings.hpp"
#include "settings_panel.hpp"
#include "stats_view.hpp"
#include "timeline_view.hpp"
#include "ui_colors.hpp"

namespace gtavi {

namespace sync_state {
struct Idle {};
struct Syncing {};
struct Ok {
    platform::TimeSyncResult result;
};
struct Error {
    std::string message;
};
}  // namespace sync_state

using SyncState = std::variant<sync_state::Idle, sync_state::Syncing, sync_state::Ok, sync_state::Error>;

class MainWindow : public Gtk::ApplicationWindow {
public:
    explicit MainWindow(const Glib::RefPtr<Gtk::Application>& app);
    ~MainWindow() override;

protected:
    void size_allocate_vfunc(int width, int height, int baseline) override;
    bool on_close_request() override;

private:
    void build_header();
    void build_hero();
    void build_footer();

    void set_zone();
    void apply_theme();
    void update_settings(Settings next);
    void on_tick();
    void refresh_zone_text();
    void refresh_sync();
    void apply_layout(int width, int height);
    void compose_backdrop();

    void configure_network_time();
    void run_sync();
    void set_notify(bool enabled);
    void set_always_on_top(bool enabled);

    void show_toast(const std::string& message);
    void toggle_fullscreen();
    void toggle_settings();
    void copy_countdown();
    void save_calendar();
    bool on_key_pressed(guint keyval, guint keycode, Gdk::ModifierType state);

    std::string target_date() const;
    std::string target_time() const;
    std::string zone_text() const;

    Settings settings_;
    std::string zone_ = "UTC";
    std::int64_t target_ = 0;
    Clock clock_;
    SyncState sync_ = sync_state::Idle{};
    UiColors colors_;
    bool can_keep_above_ = false;
    std::optional<bool> was_elapsed_;
    std::string title_text_;
    // Background work holds a weak reference to this; once the window is
    // gone, its results are dropped.
    std::shared_ptr<int> alive_ = std::make_shared<int>(0);

    Glib::RefPtr<Gtk::CssProvider> layout_css_;
    std::string layout_css_text_;
    int layout_width_ = 0;
    int layout_height_ = 0;
    bool tiny_ = false;
    sigc::connection layout_idle_;
    sigc::connection toast_timer_;
    sigc::connection resync_timer_;

    Gtk::Overlay root_;
    Backdrop backdrop_;
    Gtk::WindowHandle handle_;
    Gtk::ScrolledWindow scroller_;
    Gtk::Box shell_{Gtk::Orientation::VERTICAL};

    Gtk::Box bar_{Gtk::Orientation::HORIZONTAL, 16};
    Gtk::Label brand_sub_;
    Gtk::ToggleButton pin_button_;
    Gtk::Button fullscreen_button_;
    Icon fullscreen_icon_{IconName::expand};
    Gtk::WindowControls controls_{Gtk::PackType::END};

    Gtk::Box main_{Gtk::Orientation::VERTICAL};
    Gtk::Box hero_{Gtk::Orientation::VERTICAL};
    Gtk::Label eyebrow_;
    Gtk::Label title_;
    Gtk::Box when_{Gtk::Orientation::HORIZONTAL, 12};
    Gtk::Label when_text_;
    Gtk::Button zone_button_;
    Gtk::Label zone_label_;
    Gtk::Label local_;
    CountdownView countdown_;
    Gtk::Label since_;
    Gtk::Box gap_;
    Gtk::Box details_{Gtk::Orientation::VERTICAL};
    TimelineView timeline_;
    StatsView stats_;
    Clamp details_clamp_{details_, 980};

    Gtk::Box footer_{Gtk::Orientation::HORIZONTAL, 24};
    Gtk::Label disclaimer_;
    Gtk::Label clock_source_;

    Gtk::Revealer toast_revealer_;
    Gtk::Label toast_;

    SettingsPanel settings_panel_;
};

}  // namespace gtavi
