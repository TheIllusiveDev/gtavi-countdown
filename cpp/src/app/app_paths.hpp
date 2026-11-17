#pragma once
// Where the desktop app keeps its files.

#include <filesystem>
#include <optional>

namespace gtavi::app_paths {

// ~/.config/gtavi-countdown, shared with the C# desktop app.
std::filesystem::path config_dir();
std::filesystem::path settings_file();
std::filesystem::path window_state_file();

// The user's download folder (XDG_DOWNLOAD_DIR), falling back to the
// documents folder and then the home folder.
std::filesystem::path downloads_dir();

// The folder holding the bundled fonts: the installed data folder, or the
// source tree when running from a build directory.
std::optional<std::filesystem::path> fonts_dir();

}  // namespace gtavi::app_paths
