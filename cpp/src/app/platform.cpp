#include "platform.hpp"

#include <giomm/notification.h>

#include <chrono>
#include <fstream>
#include <sstream>

#ifdef HAVE_X11
#include <X11/Xatom.h>
#include <X11/Xlib.h>
#include <gdk/x11/gdkx.h>
#endif

#include "app_paths.hpp"
#include "calendar_file.hpp"
#include "sntp.hpp"

namespace gtavi::platform {

namespace fs = std::filesystem;
using namespace std::chrono_literals;

namespace {

// Time servers, tried in order. All are anycast services with global reach.
const std::vector<std::string> time_servers{"time.cloudflare.com:123", "pool.ntp.org:123", "time.google.com:123"};
// How long to wait for each reply before moving on.
constexpr auto time_server_timeout = 1500ms;
// Exchanges per measurement; the one with the shortest round trip wins.
constexpr int time_samples = 3;

// Writes through a temporary file so that a crash never leaves half a file.
void write_atomically(const fs::path& path, const std::string& contents) {
    fs::create_directories(path.parent_path());
    auto temporary = path;
    temporary += ".tmp";
    {
        std::ofstream out(temporary, std::ios::binary | std::ios::trunc);
        if (!out) throw std::runtime_error("cannot open " + temporary.string());
        out << contents;
        out.flush();
        if (!out) throw std::runtime_error("cannot write " + temporary.string());
    }
    fs::rename(temporary, path);
}

}  // namespace

std::optional<std::string> load_settings() {
    std::ifstream in(app_paths::settings_file(), std::ios::binary);
    if (!in) return std::nullopt;
    std::ostringstream buffer;
    buffer << in.rdbuf();
    return buffer.str();
}

void save_settings(const std::string& json) {
    try {
        write_atomically(app_paths::settings_file(), json);
    } catch (const std::exception&) {
        // Persistence is a convenience; the app works without it.
    }
}

TimeSyncResult sync_network_time() {
    sntp::Measurement m;
    try {
        m = sntp::measure(time_servers, time_server_timeout, time_samples);
    } catch (const sntp::Error& error) {
        throw std::runtime_error(capitalise(error.what()) + ".");
    }
    auto server = m.server;
    if (server.ends_with(":123")) server.resize(server.size() - 4);
    return {m.offset_ms, m.round_trip_ms, server, m.stratum};
}

bool can_keep_above([[maybe_unused]] Gtk::Window& window) {
#ifdef HAVE_X11
    return GDK_IS_X11_DISPLAY(window.get_display()->gobj());
#else
    return false;
#endif
}

bool set_keep_above([[maybe_unused]] Gtk::Window& window, [[maybe_unused]] bool enabled) {
#ifdef HAVE_X11
    auto* display = window.get_display()->gobj();
    auto surface = window.get_surface();
    if (!GDK_IS_X11_DISPLAY(display) || !surface) return false;
    // GTK 4 dropped gtk_window_set_keep_above; the EWMH request it sent is
    // made here directly.
    // GTK deprecates its X11 API as a whole, but it is still the only way to
    // reach the window manager.
    G_GNUC_BEGIN_IGNORE_DEPRECATIONS
    Display* xdisplay = gdk_x11_display_get_xdisplay(display);
    Window xwindow = gdk_x11_surface_get_xid(surface->gobj());
    G_GNUC_END_IGNORE_DEPRECATIONS
    XEvent event{};
    event.xclient.type = ClientMessage;
    event.xclient.window = xwindow;
    event.xclient.message_type = XInternAtom(xdisplay, "_NET_WM_STATE", False);
    event.xclient.format = 32;
    event.xclient.data.l[0] = enabled ? 1 : 0;  // _NET_WM_STATE_ADD or _REMOVE
    event.xclient.data.l[1] = static_cast<long>(XInternAtom(xdisplay, "_NET_WM_STATE_ABOVE", False));
    event.xclient.data.l[2] = 0;
    event.xclient.data.l[3] = 1;  // Source: a normal application.
    XSendEvent(xdisplay, DefaultRootWindow(xdisplay), False, SubstructureRedirectMask | SubstructureNotifyMask,
               &event);
    XFlush(xdisplay);
    return true;
#else
    return false;
#endif
}

void notify(Gtk::Application& app, const std::string& title, const std::string& body) {
    auto notification = Gio::Notification::create(title);
    notification->set_body(body);
    notification->set_priority(Gio::Notification::Priority::HIGH);
    app.send_notification("release", notification);
}

fs::path save_calendar(const std::string& file_name, const std::string& contents) {
    auto name = calendar_file::validate(file_name, contents);
    auto path = app_paths::downloads_dir() / name;
    try {
        write_atomically(path, contents);
    } catch (const std::exception& error) {
        throw std::runtime_error("Could not write " + path.string() + ": " + error.what());
    }
    return path;
}

std::string capitalise(std::string text) {
    if (!text.empty() && text[0] >= 'a' && text[0] <= 'z') text[0] = static_cast<char>(text[0] - 'a' + 'A');
    return text;
}

}  // namespace gtavi::platform
