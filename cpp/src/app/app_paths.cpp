#include "app_paths.hpp"

#include <glib.h>

#include <cstdlib>

namespace gtavi::app_paths {

namespace fs = std::filesystem;

fs::path config_dir() { return fs::path{g_get_user_config_dir()} / "gtavi-countdown"; }

fs::path settings_file() { return config_dir() / "settings.json"; }

fs::path window_state_file() { return config_dir() / "window-state.json"; }

fs::path downloads_dir() {
    fs::path home{g_get_home_dir()};
    for (auto kind : {G_USER_DIRECTORY_DOWNLOAD, G_USER_DIRECTORY_DOCUMENTS}) {
        const char* dir = g_get_user_special_dir(kind);
        // A special folder set to the home folder itself means "none".
        if (dir != nullptr && *dir != '\0' && fs::path{dir} != home) return dir;
    }
    std::error_code error;
    if (auto downloads = home / "Downloads"; fs::is_directory(downloads, error)) return downloads;
    return home;
}

std::optional<fs::path> fonts_dir() {
    std::error_code error;
    if (const char* override_dir = std::getenv("GTAVI_COUNTDOWN_DATADIR")) {
        if (auto dir = fs::path{override_dir} / "fonts"; fs::is_directory(dir, error)) return dir;
    }
    for (const char* base : {PKGDATADIR, SOURCE_DATADIR}) {
        if (auto dir = fs::path{base} / "fonts"; fs::is_directory(dir, error)) return dir;
    }
    return std::nullopt;
}

}  // namespace gtavi::app_paths
