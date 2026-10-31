
use std::{
    env,
    fs,
    path::PathBuf,
};

use amane::{Color, Service};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

#[derive(Clone, Copy)]
pub struct Theme {
    pub mode: ThemeMode,

    pub bar_surface: Color,
    pub button_surface: Color,
    pub active_surface: Color,

    pub accent: Color,
    pub text: Color,
    pub muted: Color,
    pub divider: Color,
}

impl Theme {
    // Paletas provisórias caso ainda não exista
    // um arquivo de cores gerado pelo worker.
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self {
                mode,
                bar_surface: Color::rgba(24, 24, 37, 232),
                button_surface: Color::rgb(49, 50, 68),
                active_surface: Color::rgb(69, 71, 90),
                accent: Color::rgb(137, 180, 250),
                text: Color::rgb(205, 214, 244),
                muted: Color::rgb(166, 173, 200),
                divider: Color::rgb(88, 91, 112),
            },

            ThemeMode::Light => Self {
                mode,
                bar_surface: Color::rgba(239, 241, 245, 238),
                button_surface: Color::rgb(220, 224, 232),
                active_surface: Color::rgb(204, 208, 218),
                accent: Color::rgb(30, 102, 245),
                text: Color::rgb(76, 79, 105),
                muted: Color::rgb(108, 111, 133),
                divider: Color::rgb(172, 176, 190),
            },
        }
    }

    // Carrega a paleta do wallpaper para o modo solicitado.
    pub fn load(mode: ThemeMode) -> Self {
        let fallback = Self::for_mode(mode);

        let contents = match fs::read_to_string(palette_path()) {
            Ok(contents) => contents,
            Err(_) => return fallback,
        };

        let prefix = match mode {
            ThemeMode::Dark => "dark",
            ThemeMode::Light => "light",
        };

        let key = |role: &str| format!("{prefix}.{role}");

        Self {
            mode,

            // Superfície principal com alpha.
            bar_surface: palette_color(
                &contents,
                &key("surface"),
                fallback.bar_surface,
                232,
            ),

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

            accent: palette_color(
                &contents,
                &key("primary"),
                fallback.accent,
                255,
            ),

            text: palette_color(
                &contents,
                &key("on_surface"),
                fallback.text,
                255,
            ),

            muted: palette_color(
                &contents,
                &key("on_surface_variant"),
                fallback.muted,
                255,
            ),

            divider: palette_color(
                &contents,
                &key("outline_variant"),
                fallback.divider,
                255,
            ),
        }
    }

    // Alterna entre as duas paletas já calculadas.
    pub fn toggle() {
        let mut theme = Self::write();

        let next_mode = match theme.mode {
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
        };

        *theme = Self::load(next_mode);

        println!("Theme changed to {:?}", next_mode);
    }

    // Recarrega a paleta mantendo o modo atual.
    // Utilizado quando um novo wallpaper é selecionado.
    pub fn reload() {
        let mut theme = Self::write();
        let mode = theme.mode;

        *theme = Self::load(mode);

        println!("Theme palette reloaded");
    }
}

impl Service for Theme {
    fn new() -> Self {
        Self::load(ThemeMode::Dark)
    }

    fn listen() {}
}

// --------------------------------------------------
// PALETTE HELPERS
// --------------------------------------------------

fn palette_path() -> PathBuf {
    let base = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".cache"))
        })
        .unwrap_or_else(|| PathBuf::from("."));

    base.join("caelestia-rs")
        .join("palette.txt")
}

fn palette_color(
    contents: &str,
    key: &str,
    fallback: Color,
    alpha: u8,
) -> Color {
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
