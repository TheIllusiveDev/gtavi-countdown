//! Desktop shell for the countdown.
//!
//! The user interface is the web app in `../src`. This crate hosts it in a
//! native window and adds the two things a web page cannot do on its own:
//! measuring the true time over NTP and writing a file to the download folder.

mod calendar;

use std::time::Duration;

use serde::Serialize;
use tauri::Manager;

/// Time servers, tried in order. All are anycast services with global reach.
const TIME_SERVERS: &[&str] = &["time.cloudflare.com:123", "pool.ntp.org:123", "time.google.com:123"];

/// How long to wait for each reply before moving on.
const TIME_SERVER_TIMEOUT: Duration = Duration::from_millis(1500);

/// Exchanges per measurement; the one with the shortest round trip wins.
const TIME_SAMPLES: usize = 3;

/// Result of a clock measurement, as delivered to the web app.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TimeSync {
    offset_ms: f64,
    round_trip_ms: f64,
    server: String,
    stratum: u8,
}

/// Measures the error of the system clock against a network time server.
///
/// The exchange blocks on the network, so it runs on the blocking thread pool
/// and never stalls the event loop or the window.
#[tauri::command]
async fn sync_time() -> Result<TimeSync, String> {
    let measurement = tauri::async_runtime::spawn_blocking(|| {
        timesync::measure(TIME_SERVERS, TIME_SERVER_TIMEOUT, TIME_SAMPLES)
    })
    .await
    .map_err(|error| format!("The time measurement was interrupted: {error}"))?
    .map_err(|error| capitalise(&error.to_string()) + ".")?;

    Ok(TimeSync {
        offset_ms: measurement.offset_ms,
        round_trip_ms: measurement.round_trip_ms,
        // Report the host without the port.
        server: measurement.server.trim_end_matches(":123").to_owned(),
        stratum: measurement.stratum,
    })
}

/// Writes a calendar file to the user's download folder and returns its path.
#[tauri::command]
fn save_calendar(
    app: tauri::AppHandle,
    file_name: String,
    contents: String,
) -> Result<String, String> {
    let name = calendar::validate(&file_name, &contents)?;
    let directory = app
        .path()
        .download_dir()
        .or_else(|_| app.path().document_dir())
        .or_else(|_| app.path().home_dir())
        .map_err(|error| format!("No folder to save into: {error}"))?;
    let path = directory.join(name);
    std::fs::create_dir_all(&directory)
        .and_then(|()| std::fs::write(&path, contents))
        .map_err(|error| format!("Could not write {}: {error}", path.display()))?;
    Ok(path.display().to_string())
}

fn capitalise(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Builds and runs the application. Called from `main.rs`.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Remembers the size and position of the window between launches.
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![sync_time, save_calendar])
        .run(tauri::generate_context!())
        .expect("failed to start the application");
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
