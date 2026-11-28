use std::path::PathBuf;

use amane::{
    Center, Color, Column, Image, Parent, Pointer, Rectangle, Row, Service, SpaceBetween, Stack,
    Text,
};

use crate::material::{
    self, icon as material_icon, shape as material_shape, space as material_space,
    type_size as material_type,
};
use crate::theme::ThemeColors;
use crate::wallpaper::WallpaperService;

const CONTENT_WIDTH: f32 = 820.0;
const CONTENT_HEIGHT: f32 = 410.0;
const SIDE_CARD_WIDTH: f32 = 205.0;
const SIDE_CARD_HEIGHT: f32 = 132.0;
const SELECTED_CARD_WIDTH: f32 = 338.0;
const SELECTED_CARD_HEIGHT: f32 = 216.0;
const CARD_GAP: f32 = material_space::MD;

#[derive(Default)]
struct WallpaperPickerState {
    selected: Option<PathBuf>,
}

impl Service for WallpaperPickerState {
    fn new() -> Self {
        let files = WallpaperService::wallpapers();
        let current = WallpaperService::current();
        let selected = current
            .filter(|path| files.contains(path))
            .or_else(|| files.first().cloned());

        Self { selected }
    }

    fn listen() {}
}

pub(super) fn content_size() -> (f32, f32) {
    (CONTENT_WIDTH, CONTENT_HEIGHT)
}

pub(super) fn view(theme: ThemeColors) -> Rectangle {
    let files = WallpaperService::wallpapers();
    let current = WallpaperService::current();

    if files.is_empty() {
        return empty_view(theme);
    }

    let selected = WallpaperPickerState::read()
        .selected
        .clone()
        .filter(|path| files.contains(path))
        .or_else(|| current.clone().filter(|path| files.contains(path)))
        .unwrap_or_else(|| files[0].clone());

    let selected_index = files.iter().position(|path| path == &selected).unwrap_or(0);

    let previous = if files.len() >= 3 {
        Some(files[(selected_index + files.len() - 1) % files.len()].clone())
    } else {
        None
    };
    let next = if files.len() >= 3 {
        Some(files[(selected_index + 1) % files.len()].clone())
    } else {
        None
    };

    let header = header(&files, selected_index, theme);
    let carousel = carousel(previous, selected.clone(), next, current.as_ref(), theme);

    Rectangle::new()
        .width(CONTENT_WIDTH)
        .height(CONTENT_HEIGHT)
        .fill(Color::TRANSPARENT)
        .child(
            Column::new(vec![Box::new(header), Box::new(carousel)])
                .width(Parent)
                .height(Parent)
                .gap(material_space::MD)
                .justify(SpaceBetween),
        )
}

fn header(files: &[PathBuf], selected_index: usize, theme: ThemeColors) -> Rectangle {
    let directory = WallpaperService::directory();
    let directory_label = if let Ok(home) = std::env::var("HOME") {
        directory
            .strip_prefix(&home)
            .map(|relative| format!("~/{}", relative.display()))
            .unwrap_or_else(|_| directory.display().to_string())
    } else {
        directory.display().to_string()
    };

    let title = Column::new(vec![
        Box::new(
            Text::new("Wallpapers")
                .size(material_type::TITLE_MEDIUM)
                .color(theme.text),
        ),
        Box::new(
            Text::new(directory_label)
                .size(material_type::LABEL_SMALL)
                .color(theme.muted)
                .elide(),
        ),
    ])
    .width(CONTENT_WIDTH - 120.0)
    .gap(material_space::XS)
    .justify(Center);

    let counter = Text::new(format!("{:02} / {:02}", selected_index + 1, files.len()))
        .size(12.0)
        .color(theme.muted);

    let refresh = Rectangle::new()
        .width(34.0)
        .height(34.0)
        .radius(material_shape::CONTROL)
        .fill(theme.button_surface)
        .cursor(Pointer)
        .align_child(Center, Center)
        .child(material_icon(material::icon::REFRESH, 18.0, theme.muted))
        .on_click(|_| {
            let count = WallpaperService::refresh();
            let current = WallpaperService::current();
            let files = WallpaperService::wallpapers();
            let mut state = WallpaperPickerState::write();
            state.selected = current
                .filter(|path| files.contains(path))
                .or_else(|| files.first().cloned());
            println!("Wallpaper: biblioteca atualizada ({count} imagens)");
        });

    Rectangle::new()
        .width(CONTENT_WIDTH)
        .height(38.0)
        .fill(Color::TRANSPARENT)
        .child(
            Row::new(vec![Box::new(title), Box::new(counter), Box::new(refresh)])
                .width(Parent)
                .height(Parent)
                .gap(material_space::MD)
                .align(Center)
                .justify(SpaceBetween),
        )
}

fn carousel(
    previous: Option<PathBuf>,
    selected: PathBuf,
    next: Option<PathBuf>,
    current: Option<&PathBuf>,
    theme: ThemeColors,
) -> Rectangle {
    let center_is_current = current == Some(&selected);
    let center_card = wallpaper_card(
        selected,
        SELECTED_CARD_WIDTH,
        SELECTED_CARD_HEIGHT,
        true,
        center_is_current,
        theme,
        || {},
    );

    let mut cards: Vec<Box<dyn amane::Widget>> = Vec::new();

    if let Some(path) = previous {
        let candidate = path.clone();
        let card = wallpaper_card(
            path,
            SIDE_CARD_WIDTH,
            SIDE_CARD_HEIGHT,
            false,
            current == Some(&candidate),
            theme,
            move || set_preview(candidate.clone()),
        );
        cards.push(Box::new(
            Column::new(vec![Box::new(card)])
                .width(SIDE_CARD_WIDTH + 8.0)
                .height(SELECTED_CARD_HEIGHT + 10.0)
                .align(Center)
                .justify(Center),
        ));
    }

    cards.push(Box::new(
        Column::new(vec![Box::new(center_card)])
            .width(SELECTED_CARD_WIDTH + 8.0)
            .height(SELECTED_CARD_HEIGHT + 10.0)
            .align(Center)
            .justify(Center),
    ));

    if let Some(path) = next {
        let candidate = path.clone();
        let card = wallpaper_card(
            path,
            SIDE_CARD_WIDTH,
            SIDE_CARD_HEIGHT,
            false,
            current == Some(&candidate),
            theme,
            move || set_preview(candidate.clone()),
        );
        cards.push(Box::new(
            Column::new(vec![Box::new(card)])
                .width(SIDE_CARD_WIDTH + 8.0)
                .height(SELECTED_CARD_HEIGHT + 10.0)
                .align(Center)
                .justify(Center),
        ));
    }

    // Three odd-numbered carousel slots mirror Caelestia's centered picker:
    // neighbors remain visible on either side of the current candidate.

    Rectangle::new()
        .width(CONTENT_WIDTH)
        .height(SELECTED_CARD_HEIGHT + 20.0)
        .fill(Color::TRANSPARENT)
        .child(
            Row::new(cards)
                .width(Parent)
                .height(Parent)
                .gap(CARD_GAP)
                .align(Center)
                .justify(Center),
        )
}

fn wallpaper_card(
    path: PathBuf,
    width: f32,
    height: f32,
    selected: bool,
    current: bool,
    theme: ThemeColors,
    on_select: impl Fn() + 'static,
) -> Rectangle {
    let source = path.to_string_lossy().into_owned();
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Wallpaper")
        .to_owned();

    let status = if current {
        " · Atual"
    } else if selected {
        " · Selecionado"
    } else {
        ""
    };
    let caption = format!("{filename}{status}");
    let overlay = Rectangle::new()
        .width(Parent)
        .height(34.0)
        .radius(0.0)
        .fill(theme.bar_surface)
        .opacity(0.88)
        .translate(0.0, height - 34.0)
        .padding(8.0)
        .align_child(amane::Start, Center)
        .child(Text::new(caption).size(11.0).color(theme.text).elide());

    let image = Rectangle::new()
        .width(width)
        .height(height)
        .radius(material_shape::CARD)
        .clip()
        .fill(Image::cover(source.as_str()))
        .child(Stack::new(vec![Box::new(overlay)]))
        .cursor(Pointer)
        .on_click(move |_| on_select());

    if selected {
        Rectangle::new()
            .width(width + 6.0)
            .height(height + 6.0)
            .radius(material_shape::CARD)
            .fill(theme.accent)
            .padding(3.0)
            .child(image)
    } else {
        image.opacity(0.76)
    }
}

fn navigation_button(
    icon: &'static str,
    theme: ThemeColors,
    action: impl Fn() + 'static,
) -> Rectangle {
    Rectangle::new()
        .width(40.0)
        .height(40.0)
        .radius(material_shape::CONTROL)
        .fill(theme.button_surface)
        .cursor(Pointer)
        .align_child(Center, Center)
        .child(
            Text::new(icon)
                .font("Material Symbols Rounded")
                .size(19.0)
                .color(theme.text),
        )
        .on_click(move |_| action())
}

fn move_selection(direction: isize) {
    let files = WallpaperService::wallpapers();
    if files.is_empty() {
        return;
    }

    let selected = WallpaperPickerState::read().selected.clone();
    let index = selected
        .as_ref()
        .and_then(|path| files.iter().position(|candidate| candidate == path))
        .unwrap_or(0);
    let next = (index as isize + direction).rem_euclid(files.len() as isize) as usize;

    WallpaperPickerState::write().selected = Some(files[next].clone());
}

fn empty_view(theme: ThemeColors) -> Rectangle {
    let directory = WallpaperService::directory();
    let message = format!("Nenhum wallpaper encontrado em {}", directory.display());
    let refresh = Rectangle::new()
        .width(130.0)
        .height(40.0)
        .radius(material_shape::CONTROL)
        .fill(theme.active_surface)
        .cursor(Pointer)
        .align_child(Center, Center)
        .child(Text::new("ATUALIZAR").size(11.0).color(theme.active_text))
        .on_click(|_| {
            WallpaperService::refresh();
        });

    Rectangle::new()
        .width(CONTENT_WIDTH)
        .height(CONTENT_HEIGHT)
        .fill(Color::TRANSPARENT)
        .align_child(Center, Center)
        .child(
            Column::new(vec![
                Box::new(material_icon(material::icon::WALLPAPER, 44.0, theme.muted)),
                Box::new(
                    Text::new("Nenhum wallpaper disponível")
                        .size(18.0)
                        .color(theme.text),
                ),
                Box::new(Text::new(message).size(11.0).color(theme.muted).elide()),
                Box::new(refresh),
            ])
            .width(Parent)
            .align(Center)
            .justify(Center)
            .gap(material_space::MD),
        )
}

fn set_preview(path: PathBuf) {
    WallpaperPickerState::write().selected = Some(path);
}
