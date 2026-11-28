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
    pub const BATTERY_FULL: &str = "\u{e1a4}";
    pub const DARK_MODE: &str = "\u{e518}";
    pub const LIGHT_MODE: &str = "\u{e51c}";
    pub const WIFI: &str = "\u{e63e}";
    pub const WIFI_OFF: &str = "\u{e648}";
    pub const BLUETOOTH: &str = "\u{e1a7}";
    pub const BLUETOOTH_CONNECTED: &str = "\u{e1a8}";
    pub const BLUETOOTH_DISABLED: &str = "\u{e1a9}";
    pub const VOLUME_UP: &str = "\u{e050}";
    pub const VOLUME_DOWN: &str = "\u{e04d}";
    pub const VOLUME_OFF: &str = "\u{e04f}";
    pub const MIC: &str = "\u{e029}";
    pub const MIC_OFF: &str = "\u{e02b}";
    pub const BATTERY_VERTICAL_FULL: &str = "\u{f24f}";
    pub const BATTERY_FRAME_1: &str = "\u{f257}";
    pub const BATTERY_FRAME_2: &str = "\u{f256}";
    pub const BATTERY_FRAME_3: &str = "\u{f255}";
    pub const BATTERY_FRAME_4: &str = "\u{f254}";
    pub const BATTERY_FRAME_5: &str = "\u{f253}";
    pub const BATTERY_FRAME_6: &str = "\u{f252}";
    pub const BATTERY_FRAME_ALERT: &str = "\u{f251}";
    pub const BATTERY_FRAME_BOLT: &str = "\u{f250}";
    pub const BOLT: &str = "\u{e0e0}";
    pub const STAR: &str = "\u{e838}";
    pub const LOCK: &str = "\u{e897}";
    pub const LINK: &str = "\u{e157}";
    pub const SEARCH: &str = "\u{e8b6}";
    pub const CHECK: &str = "\u{e5ca}";
    pub const CLOSE: &str = "\u{e5cd}";
    pub const RADIO_CHECKED: &str = "\u{e837}";
    pub const RADIO_UNCHECKED: &str = "\u{e836}";
    pub const SIGNAL_WIFI: &str = "\u{e1e1}";
    pub const ADD: &str = "\u{e145}";
    pub const SETTINGS: &str = "\u{e8b8}";
    pub const ECO: &str = "\u{e8cd}";
    pub const BALANCE: &str = "\u{eaf6}";
    pub const ROCKET: &str = "\u{eb9b}";
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
