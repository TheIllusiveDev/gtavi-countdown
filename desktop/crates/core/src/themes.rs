//! Theme palettes: the colours handed to the backdrop shader, the still
//! gradient drawn when the shader is off, the settings swatch and the
//! interface colours.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Theme {
    #[default]
    Dusk,
    Midnight,
    Dawn,
}

/// An sRGB colour with 8-bit channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// Normalised components, as the shader takes them.
    pub fn to_f32(self) -> [f32; 3] {
        [f32::from(self.0) / 255.0, f32::from(self.1) / 255.0, f32::from(self.2) / 255.0]
    }
}

/// Parses "#rrggbb" at compile time.
pub const fn rgb(hex: &str) -> Rgb {
    const fn nibble(c: u8) -> u8 {
        match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            b'A'..=b'F' => c - b'A' + 10,
            _ => panic!("invalid hex colour"),
        }
    }
    let b = hex.as_bytes();
    assert!(b.len() == 7 && b[0] == b'#', "invalid hex colour");
    Rgb(
        nibble(b[1]) * 16 + nibble(b[2]),
        nibble(b[3]) * 16 + nibble(b[4]),
        nibble(b[5]) * 16 + nibble(b[6]),
    )
}

/// Colours handed to the backdrop shader.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub sky_top: Rgb,
    pub sky_mid: Rgb,
    pub sky_low: Rgb,
    pub sun_hot: Rgb,
    pub sun_cool: Rgb,
    pub glow: Rgb,
    pub water: Rgb,
    /// Star brightness from 0 to 1.
    pub stars: f32,
    /// Strength of the slits across the sun from 0 to 1.
    pub bands: f32,
}

/// A stop of the still gradient: position from 0 (top) to 1 (bottom), or an
/// offset from the horizon wherever the layout puts it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradientStop {
    pub position: f32,
    pub color: Rgb,
    pub at_horizon: bool,
}

const fn stop(position: f32, color: &str) -> GradientStop {
    GradientStop { position, color: rgb(color), at_horizon: false }
}

const fn horizon_stop(offset: f32, color: &str) -> GradientStop {
    GradientStop { position: offset, color: rgb(color), at_horizon: true }
}

/// Colours of the interface itself, mirroring the custom properties of the
/// web version's stylesheet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiColors {
    pub accent: Rgb,
    pub accent_2: Rgb,
    pub base: Rgb,
    /// Translucent panel background: colour and alpha.
    pub panel: (Rgb, f32),
    pub panel_solid: Rgb,
    /// Opaque; callers apply their own alpha.
    pub scrim: Rgb,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeInfo {
    pub label: &'static str,
    pub palette: Palette,
    /// Drawn when the shader is off or OpenGL is unavailable. Mirrors the CSS
    /// gradient of the web version.
    pub fallback: [GradientStop; 6],
    /// Three colours for the swatch in the settings panel.
    pub swatch: [Rgb; 3],
    pub ui: UiColors,
}

const DUSK: ThemeInfo = ThemeInfo {
    label: "Dusk",
    palette: Palette {
        sky_top: rgb("#120a30"),
        sky_mid: rgb("#7b1d72"),
        sky_low: rgb("#f2643d"),
        sun_hot: rgb("#ffe27a"),
        sun_cool: rgb("#ff3d81"),
        glow: rgb("#5c2410"),
        water: rgb("#1b0b3d"),
        stars: 0.55,
        bands: 1.0,
    },
    fallback: [
        stop(0.0, "#120a30"),
        stop(0.34, "#4a1560"),
        stop(0.52, "#a02a6c"),
        horizon_stop(0.0, "#f2643d"),
        horizon_stop(0.002, "#2a1050"),
        stop(1.0, "#140a2c"),
    ],
    swatch: [rgb("#120a30"), rgb("#a02a6c"), rgb("#f2643d")],
    ui: UiColors {
        accent: rgb("#ff8a5c"),
        accent_2: rgb("#ff4f9a"),
        base: rgb("#120a30"),
        panel: (Rgb(18, 8, 44), 0.62),
        panel_solid: rgb("#170b33"),
        scrim: Rgb(18, 8, 44),
    },
};

const MIDNIGHT: ThemeInfo = ThemeInfo {
    label: "Midnight",
    palette: Palette {
        sky_top: rgb("#020611"),
        sky_mid: rgb("#0b2356"),
        sky_low: rgb("#157f8f"),
        sun_hot: rgb("#f6f9ff"),
        sun_cool: rgb("#a9c8ff"),
        glow: rgb("#0b2c44"),
        water: rgb("#030b1e"),
        stars: 1.0,
        bands: 0.0,
    },
    fallback: [
        stop(0.0, "#020611"),
        stop(0.36, "#081a40"),
        stop(0.54, "#0e4a70"),
        horizon_stop(0.0, "#157f8f"),
        horizon_stop(0.002, "#06142e"),
        stop(1.0, "#020611"),
    ],
    swatch: [rgb("#020611"), rgb("#0b2356"), rgb("#157f8f")],
    ui: UiColors {
        accent: rgb("#6fd3e8"),
        accent_2: rgb("#7a9dff"),
        base: rgb("#020611"),
        panel: (Rgb(4, 12, 30), 0.66),
        panel_solid: rgb("#06122b"),
        scrim: Rgb(2, 8, 22),
    },
};

const DAWN: ThemeInfo = ThemeInfo {
    label: "Dawn",
    palette: Palette {
        sky_top: rgb("#10224f"),
        sky_mid: rgb("#a8527f"),
        sky_low: rgb("#f7a765"),
        sun_hot: rgb("#fff1c2"),
        sun_cool: rgb("#ff9a5e"),
        glow: rgb("#4a2a16"),
        water: rgb("#142a55"),
        stars: 0.2,
        bands: 0.6,
    },
    fallback: [
        stop(0.0, "#10224f"),
        stop(0.34, "#5a3a70"),
        stop(0.52, "#b85f7d"),
        horizon_stop(0.0, "#f7a765"),
        horizon_stop(0.002, "#1c356a"),
        stop(1.0, "#0f1f45"),
    ],
    swatch: [rgb("#10224f"), rgb("#a8527f"), rgb("#f7a765")],
    ui: UiColors {
        accent: rgb("#ffc27a"),
        accent_2: rgb("#ff8fa3"),
        base: rgb("#10224f"),
        panel: (Rgb(12, 24, 60), 0.64),
        panel_solid: rgb("#102250"),
        scrim: Rgb(10, 20, 52),
    },
};

impl Theme {
    pub const ALL: [Theme; 3] = [Theme::Dusk, Theme::Midnight, Theme::Dawn];

    /// The lower-case name used in storage.
    pub fn key(self) -> &'static str {
        match self {
            Theme::Dusk => "dusk",
            Theme::Midnight => "midnight",
            Theme::Dawn => "dawn",
        }
    }

    pub fn info(self) -> &'static ThemeInfo {
        match self {
            Theme::Dusk => &DUSK,
            Theme::Midnight => &MIDNIGHT,
            Theme::Dawn => &DAWN,
        }
    }

    /// The theme after this one, wrapping around.
    pub fn next(self) -> Theme {
        let index = Theme::ALL.iter().position(|t| *t == self).unwrap_or(0);
        Theme::ALL[(index + 1) % Theme::ALL.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_colours_parse() {
        assert_eq!(rgb("#ff8a5c"), Rgb(255, 138, 92));
        assert_eq!(rgb("#000000").to_f32(), [0.0, 0.0, 0.0]);
    }

    #[test]
    fn every_fallback_gradient_spans_the_window() {
        for theme in Theme::ALL {
            let stops = theme.info().fallback;
            assert_eq!(stops[0].position, 0.0);
            assert_eq!(stops[5].position, 1.0);
            assert_eq!(stops.iter().filter(|s| s.at_horizon).count(), 2);
        }
    }
}
