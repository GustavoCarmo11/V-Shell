//! Shared Material Design 3 tokens and Material Symbols glyphs.
//!
//! Keep visual choices here so components use named roles instead of
//! unrelated numeric values and private-use Unicode characters.

use amane::{Color, Text};

pub const ICON_FONT: &str = "Material Symbols Rounded";

pub mod icon {
    pub const HOME: &str = "\u{e871}";
    pub const MUSIC_NOTE: &str = "\u{e405}";
    pub const MONITORING: &str = "\u{e9e4}";
    pub const WALLPAPER: &str = "\u{e1bc}";
    pub const PERSON: &str = "\u{e7fd}";
    pub const REFRESH: &str = "\u{e5d5}";
    pub const EXPAND_MORE: &str = "\u{e5c3}";
    pub const PLAY: &str = "\u{e034}";
    pub const PAUSE: &str = "\u{e037}";
    pub const SKIP_PREVIOUS: &str = "\u{e045}";
    pub const SKIP_NEXT: &str = "\u{e044}";
    pub const LYRICS: &str = "\u{e05f}";
    pub const MUSIC_OFF: &str = "\u{e04f}";
    pub const ALBUM: &str = "\u{e04a}";
    pub const CPU: &str = "\u{e30d}";
    pub const DEVICE_THERMOSTAT: &str = "\u{f076}";
    pub const MEMORY: &str = "\u{e322}";
    pub const NETWORK: &str = "\u{e640}";
    pub const STORAGE: &str = "\u{e1db}";
    pub const BATTERY_CHARGING: &str = "\u{e1a3}";
    pub const BATTERY_LOW: &str = "\u{e19c}";
    pub const BATTERY_FULL: &str = "\u{e1a4}";
    pub const DARK_MODE: &str = "\u{e518}";
    pub const LIGHT_MODE: &str = "\u{e51c}";
    pub const POWER_SETTINGS: &str = "\u{f8c7}";
    pub const STAR: &str = "\u{e838}";
}

pub mod shape {
    /// Material 3 shape scale, in logical pixels.
    pub const EXTRA_SMALL: f32 = 4.0;
    pub const SMALL: f32 = 8.0;
    pub const MEDIUM: f32 = 12.0;
    pub const LARGE: f32 = 16.0;
    pub const EXTRA_LARGE: f32 = 28.0;
    /// Uses Amane's bounds-aware full radius, producing a pill or circle.
    pub const FULL: amane::Full = amane::Full;

    pub const PANEL: f32 = EXTRA_LARGE;
    pub const CARD: f32 = LARGE;
    pub const CONTROL: f32 = MEDIUM;
}

pub mod space {
    /// Material 3's 4 dp spacing scale, in logical pixels.
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 24.0;
}

pub mod type_size {
    /// Material 3 type scale sizes, in logical pixels.
    pub const LABEL_SMALL: f32 = 11.0;
    pub const LABEL_MEDIUM: f32 = 12.0;
    pub const BODY_MEDIUM: f32 = 14.0;
    pub const BODY_LARGE: f32 = 16.0;
    pub const TITLE_MEDIUM: f32 = 16.0;
    pub const TITLE_LARGE: f32 = 22.0;
    pub const HEADLINE_MEDIUM: f32 = 28.0;
}

/// Builds a consistently styled Material Symbols glyph.
pub fn icon(glyph: &'static str, size: f32, color: Color) -> Text {
    Text::new(glyph).font(ICON_FONT).size(size).color(color)
}
