//! Everything that touches the operating system: files, network time,
//! notifications and the desktop's animation preference.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use gtavi_core::calendar_file;

/// Time servers, tried in order. All are anycast services with global reach.
const TIME_SERVERS: &[&str] = &["time.cloudflare.com:123", "pool.ntp.org:123", "time.google.com:123"];
/// How long to wait for each reply before moving on.
const TIME_SERVER_TIMEOUT: Duration = Duration::from_millis(1500);
/// Exchanges per measurement; the one with the shortest round trip wins.
const TIME_SAMPLES: usize = 3;

// ------------------------------------------------------------------ paths

/// `~/.config/gtavi-countdown` on Linux, `%APPDATA%\gtavi-countdown` on
/// Windows, `~/Library/Application Support/gtavi-countdown` on macOS.
pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("gtavi-countdown"))
}

fn settings_file() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("settings.json"))
}

fn window_state_file() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("window-state.json"))
}

/// The download folder, then the documents folder, then the home folder. A
/// special folder set to the home folder itself means "none".
pub fn downloads_dir() -> Option<PathBuf> {
    let home = dirs::home_dir();
    [dirs::download_dir(), dirs::document_dir()]
        .into_iter()
        .flatten()
        .find(|dir| Some(dir) != home.as_ref())
        .or_else(|| home.as_ref().map(|h| h.join("Downloads")).filter(|d| d.is_dir()))
        .or(home)
}

/// Writes through a temporary file so that a crash never leaves half a file.
fn write_atomically(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    let temporary = PathBuf::from(temporary);
    {
        let mut file = fs::File::create(&temporary)?;
        file.write_all(contents)?;
        file.sync_all()?;
    }
    fs::rename(&temporary, path)
}

// --------------------------------------------------------------- settings

pub fn load_settings() -> Option<String> {
    fs::read_to_string(settings_file()?).ok()
}

/// Persistence is a convenience; the app works without it.
pub fn save_settings(json: &str) {
    if let Some(path) = settings_file() {
        let _ = write_atomically(&path, json.as_bytes());
    }
}

// ----------------------------------------------------------- window state

/// The size, position and maximised state of the window, in points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowState {
    pub position: Option<[f32; 2]>,
    pub size: [f32; 2],
    pub maximized: bool,
}

/// Reads `width`, `height`, `left`, `top` and `maximized`, and also the
/// capitalised keys that earlier desktop versions wrote (`Left`, `Top`, `Width`, `Height`,
/// `Maximized`).
pub fn load_window_state() -> Option<WindowState> {
    let text = fs::read_to_string(window_state_file()?).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    let object = json.as_object()?;
    let field = |name: &str| -> Option<&serde_json::Value> {
        object.get(name).or_else(|| {
            let mut capital = name.to_owned();
            capital[..1].make_ascii_uppercase();
            object.get(&capital)
        })
    };
    let number = |name: &str| field(name).and_then(serde_json::Value::as_f64).map(|n| n as f32);
    let (width, height) = (number("width")?, number("height")?);
    // Ignore anything that would leave the window unusable.
    if !(360.0..=16384.0).contains(&width) || !(240.0..=16384.0).contains(&height) {
        return None;
    }
    let position = match (number("left"), number("top")) {
        (Some(x), Some(y)) if x.abs() < 65536.0 && y.abs() < 65536.0 => Some([x, y]),
        _ => None,
    };
    let maximized = field("maximized").and_then(serde_json::Value::as_bool).unwrap_or(false);
    Some(WindowState { position, size: [width, height], maximized })
}

/// Losing the window size is not worth failing over.
pub fn save_window_state(state: &WindowState) {
    let Some(path) = window_state_file() else { return };
    let mut json = serde_json::Map::new();
    if let Some([x, y]) = state.position {
        json.insert("left".into(), (x.round() as i64).into());
        json.insert("top".into(), (y.round() as i64).into());
    }
    json.insert("width".into(), (state.size[0].round() as i64).into());
    json.insert("height".into(), (state.size[1].round() as i64).into());
    json.insert("maximized".into(), state.maximized.into());
    if let Ok(text) = serde_json::to_string_pretty(&serde_json::Value::Object(json)) {
        let _ = write_atomically(&path, (text + "\n").as_bytes());
    }
}

// ----------------------------------------------------------- network time

/// The result of a clock measurement.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeSync {
    pub offset_ms: f64,
    pub round_trip_ms: f64,
    pub server: String,
}

/// Measures the error of the system clock against a network time server.
/// Blocks on the network; call it from a background thread.
pub fn sync_network_time() -> Result<TimeSync, String> {
    let measurement = timesync::measure(TIME_SERVERS, TIME_SERVER_TIMEOUT, TIME_SAMPLES)
        .map_err(|error| capitalise(&error.to_string()) + ".")?;
    Ok(TimeSync {
        offset_ms: measurement.offset_ms,
        round_trip_ms: measurement.round_trip_ms,
        // Report the host without the port.
        server: measurement.server.trim_end_matches(":123").to_owned(),
    })
}

fn capitalise(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

// --------------------------------------------------------- notifications

/// Shows a system notification. Runs on a background thread, because the
/// notification service is reached over D-Bus on Linux and may be slow.
pub fn notify(title: &str, body: &str) {
    let (title, body) = (title.to_owned(), body.to_owned());
    std::thread::spawn(move || {
        let _ = notify_rust::Notification::new().appname("GTA VI Countdown").summary(&title).body(&body).show();
    });
}

// --------------------------------------------------------------- calendar

/// Validates and writes a calendar file to the download folder, returning
/// the path written.
pub fn save_calendar(file_name: &str, contents: &str) -> Result<PathBuf, String> {
    let name = calendar_file::validate(file_name, contents)?;
    let directory = downloads_dir().ok_or("No folder to save into.")?;
    let path = directory.join(name);
    write_atomically(&path, contents.as_bytes())
        .map_err(|error| format!("Could not write {}: {error}", path.display()))?;
    Ok(path)
}

// ------------------------------------------------------- reduced motion

/// Whether the desktop asks applications not to animate. GNOME and the
/// desktops that share its settings expose this as `enable-animations`; other
/// platforms are taken to allow animation.
pub fn reduced_motion() -> bool {
    if std::env::var_os("GTAVI_COUNTDOWN_REDUCED_MOTION").is_some_and(|v| v != "0") {
        return true;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let output = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "enable-animations"])
            .stderr(std::process::Stdio::null())
            .output();
        if let Ok(output) = output
            && output.status.success()
        {
            return String::from_utf8_lossy(&output.stdout).trim() == "false";
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::capitalise;

    #[test]
    fn capitalises_the_first_letter_only() {
        assert_eq!(capitalise("no time server"), "No time server");
        assert_eq!(capitalise(""), "");
        assert_eq!(capitalise("élan vital"), "Élan vital");
    }
}
