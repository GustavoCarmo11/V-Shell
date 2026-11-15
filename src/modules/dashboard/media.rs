use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use amane::{
    Argument, Bus, Canvas, Cap, Center, Color, Column, Image, Media, Parent, Path, Rectangle, Row,
    ScrollArea, Service, Shape, Stack, Start, Text, Widget,
};

use crate::theme::ThemeColors;

use super::visualizer;

const COVER_SIZE: f32 = 250.0;
const DETAILS_WIDTH: f32 = 252.0;
const LYRICS_WIDTH: f32 = 270.0;
const CONTENT_WIDTH: f32 = 804.0;
const CONTENT_HEIGHT: f32 = 410.0;
const COLUMN_GAP: f32 = 16.0;

#[derive(Default)]
struct TrackExtras {
    enabled: bool,
    track_key: String,
    artwork: Option<PathBuf>,
    album: String,
    lyrics: String,
}

impl Service for TrackExtras {
    fn new() -> Self {
        Self::default()
    }

    fn listen() {
        loop {
            if !Self::read().enabled {
                thread::sleep(Duration::from_millis(800));
                continue;
            }

            let active = {
                let media = Media::read();
                media.active().map(|player| {
                    (
                        player.name().to_owned(),
                        player.title().to_owned(),
                        player.artist().to_owned(),
                        player.art_url().to_owned(),
                    )
                })
            };

            let Some((player, title, artist, art_url)) = active else {
                let previous_key = Self::read().track_key.clone();
                if !previous_key.is_empty() {
                    let mut state = Self::write();
                    state.track_key.clear();
                    state.artwork = None;
                    state.album.clear();
                    state.lyrics.clear();
                }
                thread::sleep(Duration::from_secs(2));
                continue;
            };

            let key = format!("{player}\n{title}\n{artist}\n{art_url}");
            let current_key = Self::read().track_key.clone();
            if key != current_key {
                // Resolve artwork and metadata before taking the service write lock.
                let artwork = resolve_artwork(&art_url);
                let (album, lyrics) = fetch_track_extras(&player);

                let mut state = Self::write();
                state.track_key = key;
                state.artwork = artwork;
                state.album = album;
                state.lyrics = lyrics;
            }

            thread::sleep(Duration::from_secs(2));
        }
    }
}

pub(super) fn set_enabled(enabled: bool) {
    let current = TrackExtras::read().enabled;
    if current != enabled {
        TrackExtras::write().enabled = enabled;
    }
}

pub(super) fn disable() {
    set_enabled(false);
    visualizer::set_enabled(false);
}

struct PlayerSnapshot {
    title: String,
    artist: String,
    identity: String,
    playing: bool,
    position: Duration,
    length: Duration,
}

pub(super) fn content_size() -> (f32, f32) {
    if Media::read().active().is_some() {
        (CONTENT_WIDTH, CONTENT_HEIGHT)
    } else {
        // Keep a spacious empty state without reserving the full three-column player layout.
        (600.0, 276.0)
    }
}

pub(super) fn view(theme: ThemeColors) -> Rectangle {
    let snapshot = {
        let media = Media::read();
        media.active().map(|player| PlayerSnapshot {
            title: player.title().to_owned(),
            artist: player.artist().to_owned(),
            identity: player.identity().to_owned(),
            playing: player.playing(),
            position: player.position(),
            length: player.length(),
        })
    };

    let Some(player) = snapshot else {
        set_enabled(true);
        visualizer::set_enabled(false);
        return no_player_view(theme);
    };

    set_enabled(true);
    visualizer::set_enabled(player.playing);

    let extras = TrackExtras::read();
    let lyrics = if extras.lyrics.trim().is_empty() {
        "Este player não disponibiliza letras pela interface MPRIS (xesam:asText).".to_owned()
    } else {
        extras.lyrics.clone()
    };
    let album = if extras.album.trim().is_empty() {
        "Álbum não informado".to_owned()
    } else {
        extras.album.clone()
    };
    let artwork_path = extras.artwork.clone();
    drop(extras);

    let (bars, cava_available) = visualizer::snapshot();
    let art = artwork_view(theme, artwork_path, bars);
    let cover_column = Column::new(vec![
        Box::new(art),
        Box::new(
            Text::new(if !player.playing {
                "VISUALIZADOR PAUSADO"
            } else if cava_available {
                "VISUALIZADOR DE ÁUDIO"
            } else {
                "INSTALE CAVA PARA O ESPECTRO"
            })
            .size(9.0)
            .color(theme.muted),
        ),
    ])
    .width(COVER_SIZE)
    .height(CONTENT_HEIGHT)
    .gap(8.0)
    .align(Center)
    .justify(Center);

    let inner_details_width = DETAILS_WIDTH - 36.0;
    let progress = if player.length.as_secs() > 0 {
        (player.position.as_secs_f32() / player.length.as_secs_f32()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let progress_bar = Stack::new(vec![
        Box::new(
            Rectangle::new()
                .width(Parent)
                .height(5.0)
                .radius(3.0)
                .fill(theme.divider),
        ),
        Box::new(
            Rectangle::new()
                .width(inner_details_width * progress)
                .height(5.0)
                .radius(3.0)
                .fill(theme.accent),
        ),
    ])
    .width(inner_details_width)
    .height(5.0);

    let time_row = Row::new(vec![
        Box::new(
            Text::new(format_duration(player.position))
                .size(11.0)
                .color(theme.muted),
        ),
        Box::new(Text::new("/").size(11.0).color(theme.muted)),
        Box::new(
            Text::new(format_duration(player.length))
                .size(11.0)
                .color(theme.muted),
        ),
    ])
    .width(inner_details_width)
    .height(16.0)
    .gap(6.0)
    .align(Center)
    .justify(Start);

    let play_pause_icon = if player.playing {
        "\u{e034}"
    } else {
        "\u{e037}"
    };
    let controls = Row::new(vec![
        Box::new(control_button("\u{e045}", theme, Media::previous, 42.0)),
        Box::new(control_button(
            play_pause_icon,
            theme,
            Media::play_pause,
            48.0,
        )),
        Box::new(control_button("\u{e044}", theme, Media::next, 42.0)),
    ])
    .width(inner_details_width)
    .height(48.0)
    .gap(14.0)
    .align(Center)
    .justify(Center);

    let details_panel = Rectangle::new()
        .width(DETAILS_WIDTH)
        .height(CONTENT_HEIGHT)
        .radius(24.0)
        .fill(theme.button_surface)
        .padding(18.0)
        .child(
            Column::new(vec![
                Box::new(Text::new("TOCANDO AGORA").size(10.0).color(theme.accent)),
                Box::new(
                    Text::new(player.title.clone())
                        .size(23.0)
                        .color(theme.text)
                        .elide(),
                ),
                Box::new(
                    Text::new(player.artist.clone())
                        .size(16.0)
                        .color(theme.muted)
                        .elide(),
                ),
                Box::new(Text::new(album).size(13.0).color(theme.accent).elide()),
                Box::new(progress_bar),
                Box::new(time_row),
                Box::new(controls),
                Box::new(
                    Text::new(player.identity)
                        .size(10.0)
                        .color(theme.muted)
                        .elide(),
                ),
            ])
            .width(Parent)
            .height(Parent)
            .gap(12.0)
            .align(Start)
            .justify(Center),
        );

    let lyrics_panel = Rectangle::new()
        .width(LYRICS_WIDTH)
        .height(CONTENT_HEIGHT)
        .radius(24.0)
        .fill(theme.button_surface)
        .padding(17.0)
        .child(
            Column::new(vec![
                Box::new(
                    Row::new(vec![
                        Box::new(
                            Text::new("\u{e05f}")
                                .font("Material Symbols Rounded")
                                .size(17.0)
                                .color(theme.accent),
                        ),
                        Box::new(Text::new("LETRAS").size(12.0).color(theme.text)),
                    ])
                    .width(Parent)
                    .height(22.0)
                    .gap(8.0)
                    .align(Center)
                    .justify(Start),
                ),
                Box::new(
                    ScrollArea::new(
                        "gshell-dashboard-media-lyrics",
                        Text::new(lyrics).size(13.0).color(theme.text).wrap(),
                    )
                    .width(Parent)
                    .height(330.0),
                ),
            ])
            .width(Parent)
            .height(Parent)
            .gap(12.0)
            .align(Start)
            .justify(Start),
        );

    Rectangle::new()
        .width(CONTENT_WIDTH)
        .height(CONTENT_HEIGHT)
        .fill(Color::TRANSPARENT)
        .child(
            Row::new(vec![
                Box::new(cover_column),
                Box::new(details_panel),
                Box::new(lyrics_panel),
            ])
            .width(CONTENT_WIDTH)
            .height(CONTENT_HEIGHT)
            .gap(COLUMN_GAP)
            .align(Start)
            .justify(Start),
        )
}

fn no_player_view(theme: ThemeColors) -> Rectangle {
    Rectangle::new()
        .width(600.0)
        .height(276.0)
        .fill(Color::TRANSPARENT)
        .align_child(Center, Center)
        .child(
            Rectangle::new()
                .width(420.0)
                .height(210.0)
                .radius(28.0)
                .fill(theme.button_surface)
                .padding(24.0)
                .align_child(Center, Center)
                .child(
                    Column::new(vec![
                        Box::new(
                            Text::new("\u{e04f}")
                                .font("Material Symbols Rounded")
                                .size(58.0)
                                .color(theme.accent),
                        ),
                        Box::new(Text::new("Nenhum player ativo").size(21.0).color(theme.text)),
                        Box::new(
                            Text::new("Abra um player compatível com MPRIS para ver capa, controles e letras.")
                                .size(13.0)
                                .color(theme.muted)
                                .wrap(),
                        ),
                    ])
                    .width(Parent)
                    .gap(12.0)
                    .align(Center)
                    .justify(Center),
                ),
        )
}

fn artwork_view(theme: ThemeColors, artwork: Option<PathBuf>, bars: Vec<u8>) -> Rectangle {
    let center = COVER_SIZE / 2.0;
    let radius = COVER_SIZE * 0.455;
    let inner = COVER_SIZE * 0.395;
    let mut shapes: Vec<Box<dyn Shape>> = Vec::new();
    shapes.push(Box::new(
        Path::new()
            .arc(center, center, radius, 0.0, 360.0)
            .stroke(1.4, theme.divider),
    ));

    let count = bars.len().max(48);
    for index in 0..count {
        let value = bars.get(index).copied().unwrap_or(0) as f32 / 100.0;
        let angle = index as f32 / count as f32 * std::f32::consts::TAU;
        let outer = inner + 5.0 + value * 19.0;
        let x1 = center + angle.sin() * inner;
        let y1 = center - angle.cos() * inner;
        let x2 = center + angle.sin() * outer;
        let y2 = center - angle.cos() * outer;
        if value > 0.01 {
            shapes.push(Box::new(
                Path::new()
                    .move_to(x1, y1)
                    .line_to(x2, y2)
                    .stroke(3.0, theme.accent)
                    .cap(Cap::Round),
            ));
        }
    }

    let visualizer_canvas = Canvas::new()
        .width(COVER_SIZE)
        .height(COVER_SIZE)
        .shapes(shapes);
    let cover_size = COVER_SIZE * 0.72;
    let cover = if let Some(path) = artwork.filter(|path| path.is_file()) {
        Rectangle::new()
            .width(cover_size)
            .height(cover_size)
            .radius(25.0)
            .clip()
            .fill(Image::cover(path))
    } else {
        Rectangle::new()
            .width(cover_size)
            .height(cover_size)
            .radius(25.0)
            .fill(theme.button_surface)
            .align_child(Center, Center)
            .child(
                Text::new("\u{e04a}")
                    .font("Material Symbols Rounded")
                    .size(54.0)
                    .color(theme.muted),
            )
    };

    let cover_center = Rectangle::new()
        .width(COVER_SIZE)
        .height(COVER_SIZE)
        .align_child(Center, Center)
        .child(cover);

    let layers: Vec<Box<dyn Widget>> = vec![Box::new(visualizer_canvas), Box::new(cover_center)];

    Rectangle::new()
        .width(COVER_SIZE)
        .height(COVER_SIZE)
        .child(Stack::new(layers).width(Parent).height(Parent))
}

fn control_button(icon: &'static str, theme: ThemeColors, action: fn(), size: f32) -> Rectangle {
    Rectangle::new()
        .width(size)
        .height(size)
        .radius(size / 2.0)
        .fill(theme.button_surface)
        .cursor(amane::Pointer)
        .align_child(Center, Center)
        .child(
            Text::new(icon)
                .font("Material Symbols Rounded")
                .size(if size > 44.0 { 24.0 } else { 20.0 })
                .color(theme.text),
        )
        .on_click(move |_| action())
}

fn format_duration(duration: Duration) -> String {
    let total = duration.as_secs();
    format!("{}:{:02}", total / 60, total % 60)
}

fn fetch_track_extras(player: &str) -> (String, String) {
    let arguments = [Argument::from("org.mpris.MediaPlayer2.Player")];
    let properties = Bus::session().call(
        player,
        "/org/mpris/MediaPlayer2",
        "org.freedesktop.DBus.Properties",
        "GetAll",
        &arguments,
    );
    let metadata = properties.get("Metadata");
    (
        metadata.get("xesam:album").text().to_owned(),
        metadata.get("xesam:asText").text().to_owned(),
    )
}

fn resolve_artwork(source: &str) -> Option<PathBuf> {
    if source.is_empty() {
        return None;
    }

    if let Some(path) = source.strip_prefix("file://") {
        let path = path.strip_prefix("localhost").unwrap_or(path);
        let decoded = percent_decode(path);
        let path = PathBuf::from(decoded);
        return path.is_file().then_some(path);
    }

    if source.starts_with('/') {
        let path = PathBuf::from(source);
        return path.is_file().then_some(path);
    }

    if !(source.starts_with("https://") || source.starts_with("http://")) {
        let path = PathBuf::from(source);
        return path.is_file().then_some(path);
    }

    let cache_dir = cache_dir()?.join("gshell").join("media");
    fs::create_dir_all(&cache_dir).ok()?;

    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    let hash = hasher.finish();
    let extension = source
        .split('?')
        .next()
        .and_then(|url| url.rsplit('.').next())
        .filter(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "jpg" | "jpeg" | "png" | "webp"
            )
        })
        .unwrap_or("jpg");
    let destination = cache_dir.join(format!("{hash:016x}.{extension}"));

    if destination.is_file() {
        return Some(destination);
    }

    let status = Command::new("curl")
        .args([
            "--fail",
            "--location",
            "--silent",
            "--show-error",
            "--max-time",
            "8",
            "-o",
        ])
        .arg(&destination)
        .arg(source)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;

    if status.success() && destination.is_file() {
        Some(destination)
    } else {
        let _ = fs::remove_file(destination);
        None
    }
}

fn cache_dir() -> Option<PathBuf> {
    env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or_default();
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                decoded.push(byte);
                index += 3;
                continue;
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}
