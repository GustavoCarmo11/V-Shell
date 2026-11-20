use std::{env, fs, path::PathBuf, time::Duration};

use amane::{Animation, Color, Easing, Service};

// --------------------------------------------------
// THEME MODE
// --------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

// --------------------------------------------------
// THEME COLORS
// --------------------------------------------------

#[derive(Clone, Copy)]
pub struct ThemeColors {
    pub mode: ThemeMode,

    // Surfaces
    pub bar_surface: Color,
    pub button_surface: Color,
    pub workspace_group_surface: Color,
    pub workspace_occupied_surface: Color,
    pub active_surface: Color,

    // Active workspace content
    pub active_text: Color,

    // General content
    pub accent: Color,
    pub text: Color,
    pub muted: Color,
    pub divider: Color,
}

// --------------------------------------------------
// THEME SERVICE
// --------------------------------------------------

pub struct Theme {
    mode: ThemeMode,

    // Surfaces
    bar_surface: Animation<Color>,
    button_surface: Animation<Color>,
    workspace_group_surface: Animation<Color>,
    workspace_occupied_surface: Animation<Color>,
    active_surface: Animation<Color>,

    // Active workspace
    active_text: Animation<Color>,

    // General content
    accent: Animation<Color>,
    text: Animation<Color>,
    muted: Animation<Color>,
    divider: Animation<Color>,
}

// --------------------------------------------------
// ANIMATION CONFIGURATION
// --------------------------------------------------

const MODE_TRANSITION_DURATION: Duration = Duration::from_millis(300);

const WALLPAPER_TRANSITION_DURATION: Duration = Duration::from_millis(700);

fn animated_color(color: Color) -> Animation<Color> {
    Animation::new(color)
        .duration(MODE_TRANSITION_DURATION)
        .easing(Easing::InOut)
}

fn retarget_color(animation: &mut Animation<Color>, target: Color, duration: Duration) {
    let current = animation.value();

    *animation = Animation::new(current)
        .duration(duration)
        .easing(Easing::InOut);

    animation.to(target);
}

// --------------------------------------------------
// THEME COLORS IMPLEMENTATION
// --------------------------------------------------

impl ThemeColors {
    pub fn load(mode: ThemeMode) -> Self {
        let fallback = Self::fallback(mode);

        let contents = fs::read_to_string(palette_path()).unwrap_or_default();

        let prefix = match mode {
            ThemeMode::Dark => "dark",
            ThemeMode::Light => "light",
        };

        let key = |role: &str| format!("{prefix}.{role}");

        Self {
            mode,

            // Use the lowest neutral container as the rail base. This anchors
            // the bar to a darker/lighter endpoint and leaves more separation
            // for the workspace surfaces above it.
            bar_surface: palette_color(
                &contents,
                &key("surface_container_lowest"),
                fallback.bar_surface,
                248,
            ),

            button_surface: palette_color(
                &contents,
                &key("surface_container"),
                fallback.button_surface,
                255,
            ),

            // A dedicated intermediate surface for the workspace capsule.
            workspace_group_surface: palette_color(
                &contents,
                &key("surface_container_high"),
                fallback.workspace_group_surface,
                255,
            ),

            // Use Material 3's strongest surface container role for a clearer
            // distinction between occupied inactive workspaces and the rail.
            workspace_occupied_surface: palette_color(
                &contents,
                &key("surface_container_highest"),
                fallback.workspace_occupied_surface,
                255,
            ),

            active_surface: palette_color(
                &contents,
                &key("primary_container"),
                fallback.active_surface,
                255,
            ),

            active_text: palette_color(
                &contents,
                &key("on_primary_container"),
                fallback.active_text,
                255,
            ),

            accent: palette_color(&contents, &key("primary"), fallback.accent, 255),

            text: palette_color(&contents, &key("on_surface"), fallback.text, 255),

            muted: palette_color(&contents, &key("on_surface_variant"), fallback.muted, 255),

            divider: palette_color(&contents, &key("outline_variant"), fallback.divider, 255),
        }
    }

    // --------------------------------------------------
    // FALLBACK PALETTES
    // --------------------------------------------------

    fn fallback(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self {
                mode,

                bar_surface: Color::rgba(13, 14, 23, 248),
                button_surface: Color::rgb(49, 50, 68),
                workspace_group_surface: Color::rgb(61, 64, 84),
                workspace_occupied_surface: Color::rgb(85, 88, 106),
                active_surface: Color::rgb(69, 71, 90),

                active_text: Color::rgb(255, 219, 213),

                accent: Color::rgb(137, 180, 250),
                text: Color::rgb(205, 214, 244),
                muted: Color::rgb(166, 173, 200),
                divider: Color::rgb(88, 91, 112),
            },

            ThemeMode::Light => Self {
                mode,

                bar_surface: Color::rgba(250, 250, 253, 248),
                button_surface: Color::rgb(220, 224, 232),
                workspace_group_surface: Color::rgb(207, 211, 224),
                workspace_occupied_surface: Color::rgb(192, 196, 210),
                active_surface: Color::rgb(204, 208, 218),

                active_text: Color::rgb(96, 43, 34),

                accent: Color::rgb(30, 102, 245),
                text: Color::rgb(76, 79, 105),
                muted: Color::rgb(108, 111, 133),
                divider: Color::rgb(172, 176, 190),
            },
        }
    }
}

// --------------------------------------------------
// THEME SERVICE METHODS
// --------------------------------------------------

impl Theme {
    // Retorna a paleta atual com valores interpolados.
    pub fn snapshot() -> ThemeColors {
        let theme = Self::read();

        ThemeColors {
            mode: theme.mode,

            bar_surface: theme.bar_surface.value(),
            button_surface: theme.button_surface.value(),
            workspace_group_surface: theme.workspace_group_surface.value(),
            workspace_occupied_surface: theme.workspace_occupied_surface.value(),
            active_surface: theme.active_surface.value(),

            active_text: theme.active_text.value(),

            accent: theme.accent.value(),
            text: theme.text.value(),
            muted: theme.muted.value(),
            divider: theme.divider.value(),
        }
    }

    // Atualiza os destinos de todas as animações.
    fn animate_to(&mut self, colors: ThemeColors, duration: Duration) {
        retarget_color(&mut self.bar_surface, colors.bar_surface, duration);

        retarget_color(&mut self.button_surface, colors.button_surface, duration);

        retarget_color(
            &mut self.workspace_group_surface,
            colors.workspace_group_surface,
            duration,
        );

        retarget_color(
            &mut self.workspace_occupied_surface,
            colors.workspace_occupied_surface,
            duration,
        );

        retarget_color(&mut self.active_surface, colors.active_surface, duration);

        retarget_color(&mut self.active_text, colors.active_text, duration);

        retarget_color(&mut self.accent, colors.accent, duration);

        retarget_color(&mut self.text, colors.text, duration);

        retarget_color(&mut self.muted, colors.muted, duration);

        retarget_color(&mut self.divider, colors.divider, duration);
    }

    // --------------------------------------------------
    // LIGHT / DARK TOGGLE
    // --------------------------------------------------

    pub fn toggle() {
        let next_mode = match Self::read().mode {
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
        };

        let colors = ThemeColors::load(next_mode);

        let mut theme = Self::write();

        theme.mode = next_mode;

        theme.animate_to(colors, MODE_TRANSITION_DURATION);

        println!("Theme changed to {:?}", next_mode);
    }

    // --------------------------------------------------
    // WALLPAPER THEME RELOAD
    // --------------------------------------------------

    pub fn reload_for_wallpaper() {
        let next_mode = detected_wallpaper_mode().unwrap_or_else(|| Self::read().mode);

        let colors = ThemeColors::load(next_mode);

        let mut theme = Self::write();

        theme.mode = next_mode;

        theme.animate_to(colors, WALLPAPER_TRANSITION_DURATION);

        println!("Wallpaper theme detected: {:?}", next_mode);
    }
}

// --------------------------------------------------
// SERVICE INITIALIZATION
// --------------------------------------------------

impl Service for Theme {
    fn new() -> Self {
        let mode = detected_wallpaper_mode().unwrap_or(ThemeMode::Dark);

        let colors = ThemeColors::load(mode);

        Self {
            mode,

            bar_surface: animated_color(colors.bar_surface),
            button_surface: animated_color(colors.button_surface),
            workspace_group_surface: animated_color(colors.workspace_group_surface),
            workspace_occupied_surface: animated_color(colors.workspace_occupied_surface),
            active_surface: animated_color(colors.active_surface),

            active_text: animated_color(colors.active_text),

            accent: animated_color(colors.accent),
            text: animated_color(colors.text),
            muted: animated_color(colors.muted),
            divider: animated_color(colors.divider),
        }
    }

    fn listen() {}
}

// --------------------------------------------------
// PALETTE FILE
// --------------------------------------------------

fn palette_path() -> PathBuf {
    let base = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .unwrap_or_else(|| PathBuf::from("."));

    base.join("caelestia-rs").join("palette.txt")
}

// --------------------------------------------------
// WALLPAPER MODE DETECTION
// --------------------------------------------------

fn detected_wallpaper_mode() -> Option<ThemeMode> {
    let contents = fs::read_to_string(palette_path()).ok()?;

    contents.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;

        if key.trim() != "suggested_mode" {
            return None;
        }

        match value.trim() {
            "light" => Some(ThemeMode::Light),
            "dark" => Some(ThemeMode::Dark),
            _ => None,
        }
    })
}

// --------------------------------------------------
// COLOR PARSING
// --------------------------------------------------

fn palette_color(contents: &str, key: &str, fallback: Color, alpha: u8) -> Color {
    let value = contents.lines().find_map(|line| {
        let (name, value) = line.split_once('=')?;

        if name.trim() == key {
            Some(value.trim())
        } else {
            None
        }
    });

    value
        .and_then(|value| parse_hex(value, alpha))
        .unwrap_or(fallback)
}

fn parse_hex(value: &str, alpha: u8) -> Option<Color> {
    let hex = value.strip_prefix('#').unwrap_or(value);

    if hex.len() != 6 {
        return None;
    }

    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;

    Some(Color::rgba(red, green, blue, alpha))
}
