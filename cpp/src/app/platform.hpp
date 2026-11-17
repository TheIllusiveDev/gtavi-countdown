#pragma once
// What the desktop app does beyond drawing: storage, network time,
// notifications, window management and writing the calendar file.

#include <gtkmm/application.h>
#include <gtkmm/window.h>

#include <filesystem>
#include <optional>
#include <string>

namespace gtavi::platform {

// The stored settings JSON, or nothing when there is none or it cannot be read.
std::optional<std::string> load_settings();
// Persistence is a convenience; failures are ignored.
void save_settings(const std::string& json);

struct TimeSyncResult {
    double offset_ms;
    double round_trip_ms;
    std::string server;  // Host only, without the port.
    int stratum;
};

// Measures the system clock against public time servers. Blocks on the
// network, so it must run off the main thread. Throws std::runtime_error with
// a sentence suitable for display.
TimeSyncResult sync_network_time();

// Whether the windowing system lets the app keep a window above others.
bool can_keep_above(Gtk::Window& window);
// Asks the window manager to keep the window above others. Returns false when
// that is not possible (Wayland has no such request).
bool set_keep_above(Gtk::Window& window, bool enabled);

// Shows a desktop notification through GIO.
void notify(Gtk::Application& app, const std::string& title, const std::string& body);

// Validates and writes a calendar file to the download folder. Returns the
// path written; throws std::exception with a message on failure.
std::filesystem::path save_calendar(const std::string& file_name, const std::string& contents);

// "no time server" becomes "No time server".
std::string capitalise(std::string text);

}  // namespace gtavi::platform
