use amane::{
    Apps, Center, Column, DesktopApp, Image, Parent, Pointer, Rectangle, Row, ScrollArea, Service,
    Stack, Start, Text, TextInput, Widget,
};

use crate::motion::{Motion, PanelId, PanelMotionSample};
use crate::theme::ThemeColors;

pub const PANEL_WIDTH: f32 = 640.0;
pub const PANEL_HEIGHT: f32 = 516.0;
// The Launcher touches the inner edge of the bottom frame so the shared SDF
// joins it to the rail through the continuous bottom perimeter.
pub const PANEL_BOTTOM_GAP: f32 = crate::BORDER_SIZE;

const SEARCH_HEIGHT: f32 = 42.0;
const SEARCH_BACKGROUND_OPACITY: f32 = 0.90;
const PANEL_PADDING: f32 = 24.0;
const CONTENT_GAP: f32 = 10.0;
const LIST_HEIGHT: f32 = PANEL_HEIGHT - PANEL_PADDING * 2.0 - SEARCH_HEIGHT - CONTENT_GAP;
const ROW_HEIGHT: f32 = 56.0;
const ROW_GAP: f32 = 4.0;
const SEARCH_ID: &str = "g-shell-launcher-search";
const SCROLL_ID: &str = "g-shell-launcher-app-list";

#[derive(Default)]
struct LauncherUiState {
    query: String,
}

impl Service for LauncherUiState {
    fn new() -> Self {
        Self::default()
    }

    fn listen() {}
}

/// Launcher state and view backed by Amane's XDG desktop-entry service.
pub struct LauncherState;

impl LauncherState {
    pub fn open() {
        Motion::open(PanelId::Launcher);
    }

    pub fn close() {
        // Do not leave the hidden Launcher search field consuming keyboard input.
        TextInput::clear_focus(SEARCH_ID);
        Motion::close(PanelId::Launcher);
    }

    /// Binds the whole Launcher surface to the shared hover lifecycle and
    /// releases search focus when the pointer leaves the panel.
    pub fn hover_area(widget: Rectangle) -> Rectangle {
        Motion::panel_hover_with_exit(PanelId::Launcher, widget, || {
            TextInput::clear_focus(SEARCH_ID);
        })
    }

    pub fn is_open() -> bool {
        Motion::is_open(PanelId::Launcher)
    }

    /// The launcher travels upward from the bottom-center anchor.
    pub fn sample(travel_y: f32) -> PanelMotionSample {
        Motion::sample(PanelId::Launcher, (0.0, travel_y))
    }

    fn query() -> String {
        LauncherUiState::read().query.clone()
    }

    fn set_query(query: String) {
        let mut state = LauncherUiState::write();
        state.query = query;
    }

    pub fn panel(theme: ThemeColors) -> Rectangle {
        let query = Self::query();
        let needle = query.trim().to_lowercase();

        // Amane scans XDG application directories asynchronously. Games are
        // included whenever they provide normal .desktop entries.
        let app_service = Apps::read();
        let mut rows: Vec<Box<dyn Widget>> = app_service
            .list()
            .iter()
            .filter(|app| {
                needle.is_empty()
                    || app.name().to_lowercase().contains(&needle)
                    || app
                        .description()
                        .is_some_and(|description| description.to_lowercase().contains(&needle))
            })
            .cloned()
            .map(|app| Box::new(app_row(app, theme)) as Box<dyn Widget>)
            .collect();
        drop(app_service);

        if rows.is_empty() {
            let message = if query.trim().is_empty() {
                "Procurando aplicativos instalados..."
            } else {
                "Nenhum aplicativo encontrado para essa busca."
            };

            rows.push(Box::new(
                Rectangle::new()
                    .width(Parent)
                    .height(LIST_HEIGHT)
                    .align_child(Center, Center)
                    .child(Text::new(message).size(14.0).color(theme.muted)),
            ));
        }

        // ScrollArea preserves its scroll offset by ID across immediate-mode
        // rebuilds and handles mouse-wheel/touchpad scrolling.
        let app_list = ScrollArea::new(SCROLL_ID, Column::new(rows).gap(ROW_GAP))
            .width(Parent)
            .height(LIST_HEIGHT);

        // Stack keeps the search background translucent without fading the text.
        let search_background = Rectangle::new()
            .width(Parent)
            .height(Parent)
            .radius(12.0)
            .fill(theme.button_surface)
            .opacity(SEARCH_BACKGROUND_OPACITY);

        // Focus is requested as soon as the Launcher is shown. The text field
        // remains unfocused while the panel is hidden.
        let search = TextInput::new(SEARCH_ID)
            .width(Parent)
            .size(14.0)
            .color(theme.text)
            .placeholder("Buscar aplicativos, jogos e programas...")
            .on_change(Self::set_query);
        let search = if Self::is_open() {
            search.focused()
        } else {
            search
        };

        let search_input = Rectangle::new()
            .width(Parent)
            .height(Parent)
            .padding(12.0)
            .align_child(Start, Center)
            .child(search);

        let search = Rectangle::new()
            .width(Parent)
            .height(SEARCH_HEIGHT)
            .radius(12.0)
            .clip()
            .child(
                Stack::new(vec![Box::new(search_background), Box::new(search_input)])
                    .width(Parent)
                    .height(Parent),
            );

        // The shared fullscreen SDF paints this surface and connects its lower
        // edge to the bottom frame. This Rectangle only clips and arranges UI.
        Rectangle::new()
            .width(PANEL_WIDTH)
            .height(PANEL_HEIGHT)
            .radius(26.0)
            .clip()
            .on_click(|_| {})
            .padding(PANEL_PADDING)
            .align_child(Start, Start)
            .child(
                Column::new(vec![Box::new(search), Box::new(app_list)])
                    .width(Parent)
                    .height(Parent)
                    .gap(CONTENT_GAP)
                    .align(Start)
                    .justify(Start),
            )
    }
}

fn app_row(app: DesktopApp, theme: ThemeColors) -> Rectangle {
    let name = compact_name(app.name(), 56);
    let description = app.description().map(str::to_owned);
    let launch_app = app.clone();

    // Render the desktop-entry icon directly, without a colored tile.
    let icon: Box<dyn Widget> = if let Some(path) = app.icon_path() {
        Box::new(
            Rectangle::new()
                .width(40.0)
                .height(40.0)
                .align_child(Center, Center)
                .child(
                    Rectangle::new()
                        .width(Parent)
                        .height(Parent)
                        .fill(Image::contain(path.to_path_buf())),
                ),
        )
    } else {
        Box::new(
            Rectangle::new()
                .width(40.0)
                .height(40.0)
                .align_child(Center, Center)
                .child(
                    Text::new("\u{e5c3}")
                        .font("Material Symbols Rounded")
                        .size(27.0)
                        .color(theme.muted),
                ),
        )
    };

    let mut text_items: Vec<Box<dyn Widget>> =
        vec![Box::new(Text::new(name).size(15.0).color(theme.text))];

    if let Some(description) = description.filter(|value| !value.trim().is_empty()) {
        text_items.push(Box::new(
            Text::new(compact_name(&description, 84))
                .size(12.0)
                .color(theme.muted),
        ));
    }

    let text = Column::new(text_items)
        .width(Parent)
        .height(Parent)
        .gap(3.0)
        .align(Start)
        .justify(Center);

    Rectangle::new()
        .width(Parent)
        .height(ROW_HEIGHT)
        .radius(12.0)
        .cursor(Pointer)
        .on_click(move |_| {
            launch_app.launch();
            LauncherState::close();
        })
        .padding(8.0)
        .align_child(Start, Center)
        .child(
            Row::new(vec![icon, Box::new(text)])
                .width(Parent)
                .height(Parent)
                .gap(14.0)
                .align(Center)
                .justify(Start),
        )
}

fn compact_name(name: &str, max_chars: usize) -> String {
    let mut characters = name.chars();
    let visible: String = characters.by_ref().take(max_chars).collect();
    if characters.next().is_some() {
        format!("{visible}…")
    } else {
        visible
    }
}
