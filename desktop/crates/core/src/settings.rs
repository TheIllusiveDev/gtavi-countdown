//! The settings schema on disk and its validation. The stored form is the
//! same JSON object the web version keeps in local storage.

use serde_json::{Map, Value};

use crate::games::{self, CustomGame, Date};

use crate::themes::Theme;
use crate::zones;

/// Rendering budget for the animated backdrop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EffectLevel {
    /// Lowers the resolution when frames are slow.
    #[default]
    Auto,
    /// Full device resolution, more cloud detail, never lowered.
    High,
    /// Reduced resolution and frame rate.
    Low,
    /// A still gradient.
    Off,
}

impl EffectLevel {
    pub const ALL: [EffectLevel; 4] = [EffectLevel::Auto, EffectLevel::High, EffectLevel::Low, EffectLevel::Off];

    /// The lower-case name used in storage.
    pub fn key(self) -> &'static str {
        match self {
            EffectLevel::Auto => "auto",
            EffectLevel::High => "high",
            EffectLevel::Low => "low",
            EffectLevel::Off => "off",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            EffectLevel::Auto => "Auto",
            EffectLevel::High => "High",
            EffectLevel::Low => "Low",
            EffectLevel::Off => "Off",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// IANA zone for the release midnight; `None` follows the system.
    pub time_zone: Option<String>,
    /// A built-in game's id, or "custom" for `custom_game`.
    pub game: String,
    pub custom_game: Option<CustomGame>,
    pub show_milliseconds: bool,
    pub theme: Theme,
    pub effects: EffectLevel,
    /// Send a notification at the moment of release.
    pub notify: bool,
    /// Correct the system clock against a network time server.
    pub network_time: bool,
    /// Keep the window above other windows.
    pub always_on_top: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            time_zone: None,
            game: games::DEFAULT_GAME.to_owned(),
            custom_game: None,
            show_milliseconds: false,
            theme: Theme::Dusk,
            effects: EffectLevel::Auto,
            notify: false,
            network_time: true,
            always_on_top: false,
        }
    }
}

fn read_custom_game(value: &Value) -> Option<CustomGame> {
    let object = value.as_object()?;
    let text = |name: &str| object.get(name).and_then(Value::as_str);
    let (hour, minute) = games::parse_time(text("time")?)?;
    CustomGame::new(text("title")?, Date::parse(text("date")?)?, hour, minute, Date::parse(text("added")?)?)
}

fn read_bool(raw: &Map<String, Value>, name: &str, field: &mut bool) {
    if let Some(Value::Bool(value)) = raw.get(name) {
        *field = *value;
    }
}

/// Merges untrusted stored JSON over the defaults, discarding invalid fields.
pub fn sanitize(json: &str) -> Settings {
    let mut result = Settings::default();
    let Ok(Value::Object(raw)) = serde_json::from_str::<Value>(json) else {
        return result;
    };
    if let Some(Value::String(zone)) = raw.get("timeZone")
        && zones::is_valid_time_zone(zone)
    {
        result.time_zone = Some(zone.clone());
    }
    result.custom_game = raw.get("customGame").and_then(read_custom_game);
    if let Some(Value::String(id)) = raw.get("game") {
        let known = games::built_in().iter().any(|g| &g.id == id);
        if known || (id == games::CUSTOM && result.custom_game.is_some()) {
            result.game = id.clone();
        }
    }
    read_bool(&raw, "showMilliseconds", &mut result.show_milliseconds);
    if let Some(Value::String(key)) = raw.get("theme")
        && let Some(theme) = Theme::ALL.into_iter().find(|t| t.key() == key)
    {
        result.theme = theme;
    }
    if let Some(Value::String(key)) = raw.get("effects")
        && let Some(level) = EffectLevel::ALL.into_iter().find(|l| l.key() == key)
    {
        result.effects = level;
    }
    read_bool(&raw, "notify", &mut result.notify);
    read_bool(&raw, "networkTime", &mut result.network_time);
    read_bool(&raw, "alwaysOnTop", &mut result.always_on_top);
    result
}

/// The stored form, byte for byte the same as the web version writes.
pub fn serialize(data: &Settings) -> String {
    let string = |text: &str| Value::String(text.to_owned()).to_string();
    let zone = data.time_zone.as_deref().map_or("null".to_owned(), string);
    let custom = data.custom_game.as_ref().map_or("null".to_owned(), |g| {
        format!(
            r#"{{"title":{},"date":"{}","time":"{:02}:{:02}","added":"{}"}}"#,
            string(&g.title),
            g.date,
            g.hour,
            g.minute,
            g.added
        )
    });
    format!(
        r#"{{"timeZone":{zone},"game":{},"customGame":{custom},"showMilliseconds":{},"theme":"{}","effects":"{}","notify":{},"networkTime":{},"alwaysOnTop":{}}}"#,
        string(&data.game),
        data.show_milliseconds,
        data.theme.key(),
        data.effects.key(),
        data.notify,
        data.network_time,
        data.always_on_top
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored() -> Settings {
        Settings {
            time_zone: Some("Pacific/Auckland".into()),
            game: "custom".into(),
            custom_game: CustomGame::new("My \"game\"", Date::new(2027, 5, 1), 9, 5, Date::new(2026, 11, 18)),
            show_milliseconds: true,
            theme: Theme::Midnight,
            effects: EffectLevel::Low,
            notify: true,
            network_time: false,
            always_on_top: true,
        }
    }

    #[test]
    fn malformed_settings_fall_back_to_the_defaults() {
        for json in ["", "   ", "null", "42", "\"text\"", "[]", "true", "{not json"] {
            assert_eq!(sanitize(json), Settings::default(), "{json}");
        }
    }

    #[test]
    fn sanitize_keeps_valid_fields() {
        assert_eq!(sanitize(&serialize(&stored())), stored());
    }

    #[test]
    fn sanitize_reads_the_stored_form_of_the_web_version() {
        let json = r#"{"timeZone":"Pacific/Auckland","game":"custom",
                       "customGame":{"title":"My \"game\"","date":"2027-05-01","time":"09:05","added":"2026-11-18"},
                       "showMilliseconds":true,"theme":"midnight","effects":"low",
                       "notify":true,"networkTime":false,"alwaysOnTop":true}"#;
        assert_eq!(sanitize(json), stored());
    }

    #[test]
    fn sanitize_replaces_invalid_fields_with_their_defaults_one_by_one() {
        let result = sanitize(
            r#"{"timeZone":"Atlantis/Lost_City","showMilliseconds":"yes","theme":"neon","effects":"ultra",
                "notify":1,"networkTime":false,"alwaysOnTop":null}"#,
        );
        assert_eq!(result, Settings { network_time: false, ..Settings::default() });
    }

    #[test]
    fn sanitize_ignores_unknown_keys() {
        let result = sanitize(r#"{"theme":"dawn","injected":"<script>"}"#);
        assert_eq!(result, Settings { theme: Theme::Dawn, ..Settings::default() });
        assert!(!serialize(&result).contains("injected"));
    }

    #[test]
    fn sanitize_treats_a_null_time_zone_as_following_the_device() {
        assert_eq!(sanitize(r#"{"timeZone":null}"#).time_zone, None);
    }

    #[test]
    fn serialize_matches_the_other_versions_byte_for_byte() {
        assert_eq!(
            serialize(&Settings::default()),
            r#"{"timeZone":null,"game":"gta-vi","customGame":null,"showMilliseconds":false,"theme":"dusk","effects":"auto","notify":false,"networkTime":true,"alwaysOnTop":false}"#
        );
    }

    #[test]
    fn the_game_falls_back_when_it_cannot_be_used() {
        assert_eq!(sanitize(r#"{"game":"fable"}"#).game, "fable");
        assert_eq!(sanitize(r#"{"game":"half-life-3"}"#).game, games::DEFAULT_GAME);
        // "custom" needs a valid custom game.
        assert_eq!(sanitize(r#"{"game":"custom"}"#).game, games::DEFAULT_GAME);
        let bad = r#"{"game":"custom","customGame":{"title":"x","date":"2027-02-30","time":"00:00","added":"2026-11-18"}}"#;
        assert_eq!(sanitize(bad), Settings::default());
        // A custom game is kept while another game is selected.
        let kept = r#"{"game":"fable","customGame":{"title":"x","date":"2027-02-01","time":"00:00","added":"2026-11-18"}}"#;
        assert!(sanitize(kept).custom_game.is_some());
    }

    #[test]
    fn effects_accept_the_high_preset() {
        assert_eq!(sanitize(r#"{"effects":"high"}"#).effects, EffectLevel::High);
    }

    #[test]
    fn next_cycles_through_the_themes() {
        assert_eq!(Theme::Dusk.next(), Theme::Midnight);
        assert_eq!(Theme::Midnight.next(), Theme::Dawn);
        assert_eq!(Theme::Dawn.next(), Theme::Dusk);
    }
}
