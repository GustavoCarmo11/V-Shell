use std::{env, fs, path::PathBuf, time::Duration};

use amane::{Animation, Color, Easing, Service};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

// Snapshot das cores utilizado pelos componentes.
#[derive(Clone, Copy)]
pub struct ThemeColors {
    pub mode: ThemeMode,

    pub bar_surface: Color,
    pub button_surface: Color,
    pub active_surface: Color,

    pub accent: Color,
    pub text: Color,
    pub muted: Color,
    pub divider: Color,
}

// Serviço que mantém e anima a paleta.
pub struct Theme {
    mode: ThemeMode,

    bar_surface: Animation<Color>,
    button_surface: Animation<Color>,
    active_surface: Animation<Color>,

    accent: Animation<Color>,
    text: Animation<Color>,
    muted: Animation<Color>,
    divider: Animation<Color>,
}

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

            bar_surface: palette_color(&contents, &key("surface"), fallback.bar_surface, 190),

            button_surface: palette_color(
                &contents,
                &key("surface_container"),
                fallback.button_surface,
                255,
            ),

            active_surface: palette_color(
                &contents,
                &key("primary_container"),
                fallback.active_surface,
                255,
            ),

            accent: palette_color(&contents, &key("primary"), fallback.accent, 255),

            text: palette_color(&contents, &key("on_surface"), fallback.text, 255),

            muted: palette_color(&contents, &key("on_surface_variant"), fallback.muted, 255),

            divider: palette_color(&contents, &key("outline_variant"), fallback.divider, 255),
        }
    }

    fn fallback(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self {
                mode,
                bar_surface: Color::rgba(24, 24, 37, 190),
                button_surface: Color::rgb(49, 50, 68),
                active_surface: Color::rgb(69, 71, 90),
                accent: Color::rgb(137, 180, 250),
                text: Color::rgb(205, 214, 244),
                muted: Color::rgb(166, 173, 200),
                divider: Color::rgb(88, 91, 112),
            },

            ThemeMode::Light => Self {
                mode,
                bar_surface: Color::rgba(239, 241, 245, 190),
                button_surface: Color::rgb(220, 224, 232),
                active_surface: Color::rgb(204, 208, 218),
                accent: Color::rgb(30, 102, 245),
                text: Color::rgb(76, 79, 105),
                muted: Color::rgb(108, 111, 133),
                divider: Color::rgb(172, 176, 190),
            },
        }
    }
}

impl Theme {
    pub fn reload_for_wallpaper() {
        // Utiliza o modo detectado pelo worker.
        // Se não houver informação, preserva o modo atual.
        let next_mode = detected_wallpaper_mode().unwrap_or_else(|| Self::read().mode);

        let colors = ThemeColors::load(next_mode);

        let mut theme = Self::write();

        theme.mode = next_mode;

        theme.animate_to(colors, WALLPAPER_TRANSITION_DURATION);

        println!("Wallpaper theme detected: {:?}", next_mode);
    }
    // Obtém os valores interpolados para desenhar a interface.
    pub fn snapshot() -> ThemeColors {
        let theme = Self::read();

        ThemeColors {
            mode: theme.mode,

            bar_surface: theme.bar_surface.value(),
            button_surface: theme.button_surface.value(),
            active_surface: theme.active_surface.value(),

            accent: theme.accent.value(),
            text: theme.text.value(),
            muted: theme.muted.value(),
            divider: theme.divider.value(),
        }
    }

    // Atualiza os destinos das animações.
    fn animate_to(&mut self, colors: ThemeColors, duration: Duration) {
        retarget_color(&mut self.bar_surface, colors.bar_surface, duration);
        retarget_color(&mut self.button_surface, colors.button_surface, duration);
        retarget_color(&mut self.active_surface, colors.active_surface, duration);

        retarget_color(&mut self.accent, colors.accent, duration);
        retarget_color(&mut self.text, colors.text, duration);
        retarget_color(&mut self.muted, colors.muted, duration);
        retarget_color(&mut self.divider, colors.divider, duration);
    }

    // Alterna entre Dark e Light.
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

    // Recarrega a paleta após trocar o wallpaper.
    pub fn reload() {
        let mode = Self::read().mode;
        let colors = ThemeColors::load(mode);

        let mut theme = Self::write();

        theme.animate_to(colors, WALLPAPER_TRANSITION_DURATION);

        println!("Theme palette reloaded");
    }
}

impl Service for Theme {
    fn new() -> Self {
        let mode = detected_wallpaper_mode().unwrap_or(ThemeMode::Dark);

        let colors = ThemeColors::load(mode);

        Self {
            mode,

            bar_surface: animated_color(colors.bar_surface),
            button_surface: animated_color(colors.button_surface),
            active_surface: animated_color(colors.active_surface),

            accent: animated_color(colors.accent),
            text: animated_color(colors.text),
            muted: animated_color(colors.muted),
            divider: animated_color(colors.divider),
        }
    }

    fn listen() {}
}

// --------------------------------------------------
// PALETTE HELPERS
// --------------------------------------------------

fn palette_path() -> PathBuf {
    let base = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .unwrap_or_else(|| PathBuf::from("."));

    base.join("caelestia-rs").join("palette.txt")
}

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
