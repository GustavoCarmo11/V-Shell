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

use crate::material::{
    self, icon as material_icon, shape as material_shape, space as material_space,
    type_size as material_type,
};
use crate::theme::ThemeColors;

use super::visualizer;

const COVER_SIZE: f32 = 250.0;
const DETAILS_WIDTH: f32 = 252.0;
const LYRICS_WIDTH: f32 = 270.0;
const CONTENT_WIDTH: f32 = 804.0;
const CONTENT_HEIGHT: f32 = 410.0;
const COLUMN_GAP: f32 = material_space::LG;

#[derive(Default)]
struct TrackExtras {
    enabled: bool,
    track_key: String,
    lyrics_key: String,
    artwork: Option<PathBuf>,
    album: String,
    lyrics: String,
    lyrics_source: String,
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
                        player.length(),
                    )
                })
            };

            let Some((player, title, artist, art_url, duration)) = active else {
                let previous_key = Self::read().track_key.clone();
                if !previous_key.is_empty() {
                    let mut state = Self::write();
                    state.track_key.clear();
                    state.lyrics_key.clear();
                    state.artwork = None;
                    state.album.clear();
                    state.lyrics.clear();
                    state.lyrics_source.clear();
                }
                thread::sleep(Duration::from_secs(2));
                continue;
            };

            let key = format!("{player}\n{title}\n{artist}\n{art_url}");
            let current_key = Self::read().track_key.clone();
            if key != current_key {
                // Resolve artwork and metadata before taking the service write lock.
                let artwork = resolve_artwork(&art_url);
                let (album, lyrics, lyrics_source) =
                    fetch_track_extras(&player, &title, &artist, duration);

                let mut state = Self::write();
                state.track_key = key;
                state.lyrics_key = lyrics_key(&title, &artist);
                state.artwork = artwork;
                state.album = album;
                state.lyrics = lyrics;
                state.lyrics_source = lyrics_source;
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
    let extras_match = extras.lyrics_key == lyrics_key(&player.title, &player.artist);
    let lyrics = extras_match
        .then(|| extras.lyrics.clone())
        .unwrap_or_default();
    let lyrics_source = extras_match
        .then(|| extras.lyrics_source.clone())
        .unwrap_or_default();
    let loading_lyrics = !extras_match;
    let album = if !extras_match || extras.album.trim().is_empty() {
        "Álbum não informado".to_owned()
    } else {
        extras.album.clone()
    };
    let artwork_path = extras_match.then(|| extras.artwork.clone()).flatten();
    drop(extras);

    let (bars, cava_available) = visualizer::snapshot();
    let art = artwork_view(theme, artwork_path, bars);
    let cover_column = Rectangle::new()
        .width(COVER_SIZE)
        .height(CONTENT_HEIGHT)
        .radius(material_shape::CARD)
        .fill(theme.button_surface)
        .clip()
        .align_child(Center, Center)
        .child(
            Column::new(vec![
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
            .width(Parent)
            .height(Parent)
            .gap(material_space::SM)
            .align(Center)
            .justify(Center),
        );

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
                .radius(material_shape::EXTRA_SMALL)
                .fill(theme.divider),
        ),
        Box::new(
            Rectangle::new()
                .width(inner_details_width * progress)
                .height(5.0)
                .radius(material_shape::EXTRA_SMALL)
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
    .gap(material_space::SM)
    .align(Center)
    .justify(Start);

    let play_pause_icon = if player.playing {
        material::icon::PAUSE
    } else {
        material::icon::PLAY
    };
    let controls = Row::new(vec![
        Box::new(control_button(
            material::icon::SKIP_PREVIOUS,
            theme,
            Media::previous,
            42.0,
        )),
        Box::new(control_button(
            play_pause_icon,
            theme,
            Media::play_pause,
            48.0,
        )),
        Box::new(control_button(
            material::icon::SKIP_NEXT,
            theme,
            Media::next,
            42.0,
        )),
    ])
    .width(inner_details_width)
    .height(48.0)
    .gap(material_space::LG)
    .align(Center)
    .justify(Center);

    let details_panel = Rectangle::new()
        .width(DETAILS_WIDTH)
        .height(CONTENT_HEIGHT)
        .radius(material_shape::CARD)
        .fill(theme.button_surface)
        .padding(material_space::LG)
        .child(
            Column::new(vec![
                Box::new(Text::new("TOCANDO AGORA").size(10.0).color(theme.accent)),
                Box::new(
                    Text::new(player.title.clone())
                        .size(material_type::TITLE_LARGE)
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
            .gap(material_space::MD)
            .align(Start)
            .justify(Center),
        );

    let lyrics_panel = lyrics_panel(
        &lyrics,
        &lyrics_source,
        loading_lyrics,
        player.position,
        theme,
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
                .radius(material_shape::PANEL)
                .fill(theme.button_surface)
                .padding(material_space::XL)
                .align_child(Center, Center)
                .child(
                    Column::new(vec![
                        Box::new(
                            material_icon(material::icon::MUSIC_OFF, 58.0, theme.accent),
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
                    .gap(material_space::MD)
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
            .radius(material_shape::CARD)
            .clip()
            .fill(Image::cover(path))
    } else {
        Rectangle::new()
            .width(cover_size)
            .height(cover_size)
            .radius(material_shape::CARD)
            .fill(theme.button_surface)
            .align_child(Center, Center)
            .child(material_icon(material::icon::ALBUM, 54.0, theme.muted))
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
        .child(material_icon(
            icon,
            if size > 44.0 { 24.0 } else { 20.0 },
            theme.text,
        ))
        .on_click(move |_| action())
}

fn format_duration(duration: Duration) -> String {
    let total = duration.as_secs();
    format!("{}:{:02}", total / 60, total % 60)
}

struct LyricLine {
    at: Option<Duration>,
    text: String,
}

fn parse_lyrics(raw: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();
    for raw_line in raw.lines() {
        let mut rest = raw_line.trim();
        let mut timestamps = Vec::new();
        while let Some(tag) = rest.strip_prefix('[') {
            let Some((value, tail)) = tag.split_once(']') else {
                break;
            };
            if let Some(timestamp) = parse_lyric_timestamp(value) {
                timestamps.push(timestamp);
                rest = tail.trim_start();
            } else if value.contains(':') {
                // Ignore LRC metadata tags such as [ar:], [ti:] and [offset:].
                rest = tail.trim_start();
            } else {
                break;
            }
        }

        let text = rest.trim();
        if text.is_empty() {
            continue;
        }
        if timestamps.is_empty() {
            lines.push(LyricLine {
                at: None,
                text: text.to_owned(),
            });
        } else {
            lines.extend(timestamps.into_iter().map(|at| LyricLine {
                at: Some(at),
                text: text.to_owned(),
            }));
        }
    }
    lines.sort_by_key(|line| line.at);
    lines
}

fn parse_lyric_timestamp(value: &str) -> Option<Duration> {
    let (minutes, seconds) = value.split_once(':')?;
    let minutes = minutes.parse::<u64>().ok()?;
    let seconds = seconds.parse::<f64>().ok()?;
    if !seconds.is_finite() || !(0.0..60.0).contains(&seconds) {
        return None;
    }
    let millis = minutes
        .saturating_mul(60_000)
        .saturating_add((seconds * 1_000.0).round() as u64);
    Some(Duration::from_millis(millis))
}

fn lyrics_panel(
    raw: &str,
    source: &str,
    loading: bool,
    position: Duration,
    theme: ThemeColors,
) -> Rectangle {
    let lines = parse_lyrics(raw);
    let content: Box<dyn Widget> = if raw.trim().is_empty() {
        let message = if loading {
            "Procurando letras da faixa…"
        } else {
            "Nenhuma letra encontrada para esta faixa."
        };
        Box::new(
            Column::new(vec![
                Box::new(material_icon(
                    if loading {
                        material::icon::LYRICS
                    } else {
                        material::icon::MUSIC_OFF
                    },
                    30.0,
                    theme.muted,
                )),
                Box::new(Text::new(message).size(13.0).color(theme.muted).wrap()),
            ])
            .width(Parent)
            .height(300.0)
            .gap(material_space::MD)
            .align(Center)
            .justify(Center),
        )
    } else if lines.iter().any(|line| line.at.is_some()) {
        let active_index = lines
            .iter()
            .rposition(|line| line.at.is_some_and(|at| at <= position))
            .unwrap_or(0);
        let start = active_index.saturating_sub(2);
        let end = (active_index + 4).min(lines.len());
        let visible = lines[start..end]
            .iter()
            .enumerate()
            .map(|(offset, line)| {
                let active = start + offset == active_index;
                Box::new(
                    Text::new(line.text.clone())
                        .size(if active { 16.0 } else { 13.0 })
                        .color(if active { theme.accent } else { theme.muted })
                        .wrap(),
                ) as Box<dyn Widget>
            })
            .collect();
        Box::new(
            Column::new(visible)
                .width(Parent)
                .height(300.0)
                .gap(material_space::MD)
                .align(Start)
                .justify(Center),
        )
    } else {
        let text = lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        Box::new(
            ScrollArea::new(
                "vshell-dashboard-media-lyrics",
                Text::new(text).size(13.0).color(theme.text).wrap(),
            )
            .width(Parent)
            .height(300.0),
        )
    };

    Rectangle::new()
        .width(LYRICS_WIDTH)
        .height(CONTENT_HEIGHT)
        .radius(material_shape::CARD)
        .fill(theme.button_surface)
        .padding(material_space::LG)
        .child(
            Column::new(vec![
                Box::new(
                    Row::new(vec![
                        Box::new(material_icon(material::icon::LYRICS, 17.0, theme.accent)),
                        Box::new(Text::new("LETRAS").size(12.0).color(theme.text)),
                        Box::new(
                            Text::new(source.to_owned())
                                .size(9.0)
                                .color(theme.muted)
                                .elide(),
                        ),
                    ])
                    .width(Parent)
                    .height(22.0)
                    .gap(material_space::SM)
                    .align(Center)
                    .justify(amane::SpaceBetween),
                ),
                content,
            ])
            .width(Parent)
            .height(Parent)
            .gap(material_space::MD)
            .align(Start)
            .justify(Start),
        )
}

fn fetch_track_extras(
    player: &str,
    title: &str,
    artist: &str,
    duration: Duration,
) -> (String, String, String) {
    let arguments = [Argument::from("org.mpris.MediaPlayer2.Player")];
    let properties = Bus::session().call(
        player,
        "/org/mpris/MediaPlayer2",
        "org.freedesktop.DBus.Properties",
        "GetAll",
        &arguments,
    );
    let metadata = properties.get("Metadata");
    let album = metadata.get("xesam:album").text().to_owned();
    let mpris_lyrics = metadata.get("xesam:asText").text().trim().to_owned();
    if !mpris_lyrics.is_empty() {
        return (album, mpris_lyrics, "MPRIS".to_owned());
    }

    let track_url = metadata.get("xesam:url").text().to_owned();
    if let Some(lyrics) = local_lyrics(&track_url, artist, title) {
        return (album, lyrics, "ARQUIVO LOCAL".to_owned());
    }
    if let Some(lyrics) = fetch_lrclib_lyrics(artist, title, &album, duration) {
        return (album, lyrics, "LRCLIB".to_owned());
    }
    if let Some(lyrics) = fetch_netease_lyrics(artist, title) {
        return (album, lyrics, "NETEASE".to_owned());
    }

    (album, String::new(), String::new())
}

fn lyrics_key(title: &str, artist: &str) -> String {
    format!("{}\n{}", artist.trim(), title.trim())
}

fn curl_json(
    url: &str,
    query: &[(&str, String)],
    headers: &[(&str, &str)],
) -> Option<serde_json::Value> {
    let mut command = Command::new("curl");
    command.args([
        "--fail",
        "--location",
        "--silent",
        "--show-error",
        "--max-time",
        "6",
        "--max-filesize",
        "1048576",
        "--get",
        url,
    ]);
    for (name, value) in headers {
        command.arg("--header").arg(format!("{name}: {value}"));
    }
    for (name, value) in query {
        command
            .arg("--data-urlencode")
            .arg(format!("{name}={value}"));
    }

    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}

fn fetch_lrclib_lyrics(
    artist: &str,
    title: &str,
    album: &str,
    duration: Duration,
) -> Option<String> {
    if artist.trim().is_empty() || title.trim().is_empty() {
        return None;
    }
    let mut query = vec![
        ("track_name", title.to_owned()),
        ("artist_name", artist.to_owned()),
    ];
    if !album.trim().is_empty() {
        query.push(("album_name", album.to_owned()));
    }
    if !duration.is_zero() {
        query.push(("duration", duration.as_secs().to_string()));
    }
    let response = curl_json(
        "https://lrclib.net/api/get",
        &query,
        &[(
            "User-Agent",
            "V-Shell (https://github.com/GustavoCarmo11/V-Shell)",
        )],
    )?;
    response
        .get("syncedLyrics")
        .and_then(serde_json::Value::as_str)
        .filter(|lyrics| !lyrics.trim().is_empty())
        .or_else(|| {
            response
                .get("plainLyrics")
                .and_then(serde_json::Value::as_str)
                .filter(|lyrics| !lyrics.trim().is_empty())
        })
        .map(str::to_owned)
}

fn fetch_netease_lyrics(artist: &str, title: &str) -> Option<String> {
    if artist.trim().is_empty() || title.trim().is_empty() {
        return None;
    }
    let headers = [
        (
            "User-Agent",
            "Mozilla/5.0 (X11; Linux x86_64; rv:120.0) Gecko/20100101 Firefox/120.0",
        ),
        ("Referer", "https://music.163.com/"),
    ];
    let search = curl_json(
        "https://music.163.com/api/search/get",
        &[
            ("s", format!("{title} {artist}")),
            ("type", "1".to_owned()),
            ("limit", "5".to_owned()),
        ],
        &headers,
    )?;
    let songs = search.get("result")?.get("songs")?.as_array()?;
    let normalized_title = normalize_track_text(title);
    let normalized_artist = normalize_track_text(artist);
    let song = songs.iter().find(|song| {
        let candidate_title = song
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(normalize_track_text)
            .unwrap_or_default();
        if candidate_title != normalized_title {
            return false;
        }
        song.get("artists")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|artists| {
                artists.iter().any(|candidate| {
                    candidate
                        .get("name")
                        .and_then(serde_json::Value::as_str)
                        .map(normalize_track_text)
                        .is_some_and(|candidate_artist| {
                            candidate_artist == normalized_artist
                                || candidate_artist.contains(&normalized_artist)
                                || normalized_artist.contains(&candidate_artist)
                        })
                })
            })
    })?;
    let id = song.get("id")?.as_i64()?.to_string();
    let response = curl_json(
        "https://music.163.com/api/song/lyric",
        &[
            ("id", id),
            ("lv", "1".to_owned()),
            ("kv", "1".to_owned()),
            ("tv", "-1".to_owned()),
        ],
        &headers,
    )?;
    response
        .get("lrc")?
        .get("lyric")?
        .as_str()
        .filter(|lyrics| !lyrics.trim().is_empty())
        .map(str::to_owned)
}

fn normalize_track_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn local_lyrics(track_url: &str, artist: &str, title: &str) -> Option<String> {
    let mut candidates = Vec::new();
    if let Some(path) = track_url.strip_prefix("file://") {
        let path = path.strip_prefix("localhost").unwrap_or(path);
        let path = PathBuf::from(percent_decode(path));
        if path.is_file() {
            candidates.push(path.with_extension("lrc"));
        }
    }

    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        let directory = home.join("Music").join("lyrics");
        let artist = safe_filename_part(artist);
        let title = safe_filename_part(title);
        candidates.push(directory.join(format!("{artist} - {title}.lrc")));
        candidates.push(directory.join(artist).join(format!("{title}.lrc")));
    }

    candidates.into_iter().find_map(|path| {
        let metadata = fs::metadata(&path).ok()?;
        if metadata.len() > 512 * 1024 {
            return None;
        }
        let lyrics = fs::read_to_string(path).ok()?;
        (!lyrics.trim().is_empty()).then_some(lyrics)
    })
}

fn safe_filename_part(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if matches!(character, '/' | '\\' | '\0') {
                '_'
            } else {
                character
            }
        })
        .collect()
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

    let cache_dir = cache_dir()?.join("vshell").join("media");
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
