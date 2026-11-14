use amane::{
    Apps, Center, Column, DesktopApp, Image, Key, Parent, Pointer, Rectangle, Row, ScrollArea,
    Service, Stack, Start, Text, TextInput, Widget,
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
const ROW_PITCH: f32 = ROW_HEIGHT + ROW_GAP;
const SEARCH_ID: &str = "g-shell-launcher-search";
const SCROLL_ID: &str = "g-shell-launcher-app-list";

#[derive(Default)]
struct LauncherUiState {
    query: String,
    selected: usize,
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

    /// The Launcher is hover-latched: leaving its bounds does not dismiss it.
    /// It stays open until an outside click or an explicit close action.
    pub fn hover_area(widget: Rectangle) -> Rectangle {
        widget.on_hover(|inside| {
            if inside {
                Motion::open(PanelId::Launcher);
            }
        })
    }

    pub fn is_open() -> bool {
        Motion::is_open(PanelId::Launcher)
    }

    /// The launcher travels upward from the bottom-center anchor.
    pub fn sample(travel_y: f32) -> PanelMotionSample {
        Motion::sample(PanelId::Launcher, (0.0, travel_y))
    }

    fn query_and_selected() -> (String, usize) {
        let state = LauncherUiState::read();
        (state.query.clone(), state.selected)
    }

    fn set_query(query: String) {
        let mut state = LauncherUiState::write();
        state.query = query;
        state.selected = 0;
        drop(state);

        // A new query starts at the top of its result set.
        ScrollArea::set_offset(SCROLL_ID, 0.0);
    }

    fn matches(app: &DesktopApp, needle: &str) -> bool {
        needle.is_empty()
            || app.name().to_lowercase().contains(needle)
            || app
                .description()
                .is_some_and(|description| description.to_lowercase().contains(needle))
    }

    fn matching_count(query: &str) -> usize {
        let needle = query.trim().to_lowercase();
        Apps::read()
            .list()
            .iter()
            .filter(|app| Self::matches(app, &needle))
            .count()
    }

    fn matching_app(query: &str, index: usize) -> Option<DesktopApp> {
        let needle = query.trim().to_lowercase();
        Apps::read()
            .list()
            .iter()
            .filter(|app| Self::matches(app, &needle))
            .nth(index)
            .cloned()
    }

    fn move_selection(direction: i8) {
        let (query, current) = Self::query_and_selected();
        let count = Self::matching_count(&query);
        if count == 0 {
            return;
        }

        let current = current.min(count - 1);
        let selected = if direction < 0 {
            current.saturating_sub(1)
        } else {
            (current + 1).min(count - 1)
        };
        if selected == current {
            return;
        }

        let mut state = LauncherUiState::write();
        state.selected = selected;
        drop(state);

        // Keep the highlighted result inside the viewport while navigating.
        let current_offset = ScrollArea::offset(SCROLL_ID);
        let item_top = selected as f32 * ROW_PITCH;
        let item_bottom = item_top + ROW_HEIGHT;
        let viewport_bottom = current_offset + LIST_HEIGHT;
        let next_offset = if item_top < current_offset {
            item_top
        } else if item_bottom > viewport_bottom {
            item_bottom - LIST_HEIGHT
        } else {
            current_offset
        };

        if (next_offset - current_offset).abs() > f32::EPSILON {
            ScrollArea::set_offset(SCROLL_ID, next_offset);
        }
    }

    fn launch_selected() {
        let (query, selected) = Self::query_and_selected();
        let Some(app) = Self::matching_app(&query, selected) else {
            return;
        };

        app.launch();
        Self::close();
    }

    /// Called by the focused search field before its normal text-editing logic.
    /// Up/Down move through filtered apps, Enter launches the highlighted app.
    fn handle_search_key(key: Key) -> bool {
        match key {
            Key::Up => {
                Self::move_selection(-1);
                true
            }
            Key::Down => {
                Self::move_selection(1);
                true
            }
            Key::Enter => {
                Self::launch_selected();
                true
            }
            _ => false,
        }
    }

    pub fn panel(theme: ThemeColors, build_app_list: bool) -> Rectangle {
        let (query, selected) = Self::query_and_selected();
        let app_list: Box<dyn Widget> = if build_app_list {
            let needle = query.trim().to_lowercase();
            let app_service = Apps::read();
            let matching_count = app_service
                .list()
                .iter()
                .filter(|app| Self::matches(app, &needle))
                .count();

            if matching_count == 0 {
                let message = if query.trim().is_empty() {
                    "Procurando aplicativos instalados..."
                } else {
                    "Nenhum aplicativo encontrado para essa busca."
                };

                Box::new(
                    Rectangle::new()
                        .width(Parent)
                        .height(LIST_HEIGHT)
                        .align_child(Center, Center)
                        .child(Text::new(message).size(14.0).color(theme.muted)),
                )
            } else {
                // Only build rows inside the current viewport. The previous
                // implementation recreated a widget and icon for every app on
                // each animation frame; virtualization keeps this to ~8 rows.
                let selected = selected.min(matching_count - 1);
                let visible_rows = (LIST_HEIGHT / ROW_PITCH).ceil() as usize + 1;
                let content_height = matching_count as f32 * ROW_PITCH - ROW_GAP;
                let max_offset = (content_height - LIST_HEIGHT).max(0.0);
                let offset = ScrollArea::offset(SCROLL_ID).clamp(0.0, max_offset);
                let first_visible =
                    ((offset / ROW_PITCH).floor() as usize).min(matching_count.saturating_sub(1));

                // A full-height base gives ScrollArea the complete virtual
                // content height; only visible row widgets are placed above it.
                let mut layers: Vec<Box<dyn Widget>> = vec![Box::new(
                    Rectangle::new()
                        .width(Parent)
                        .height(content_height.max(LIST_HEIGHT)),
                )];

                for (index, app) in app_service
                    .list()
                    .iter()
                    .filter(|app| Self::matches(app, &needle))
                    .enumerate()
                    .skip(first_visible)
                    .take(visible_rows)
                {
                    layers.push(Box::new(
                        app_row(app.clone(), theme, index == selected)
                            .translate(0.0, index as f32 * ROW_PITCH),
                    ));
                }

                Box::new(
                    ScrollArea::new(
                        SCROLL_ID,
                        Stack::new(layers)
                            .width(Parent)
                            .height(content_height.max(LIST_HEIGHT)),
                    )
                    .width(Parent)
                    .height(LIST_HEIGHT),
                )
            }
        } else {
            // Keep layout stable while the Launcher is hidden.
            Box::new(Rectangle::new().width(Parent).height(LIST_HEIGHT))
        };

        // Stack keeps the search background translucent without fading the text.
        let search_background = Rectangle::new()
            .width(Parent)
            .height(Parent)
            .radius(12.0)
            .fill(theme.button_surface)
            .opacity(SEARCH_BACKGROUND_OPACITY);

        // Focus is requested as soon as the Launcher is shown. The text field
        // also handles the launcher's navigation keys while preserving normal
        // character entry, Left/Right caret movement and Backspace editing.
        let search = TextInput::new(SEARCH_ID)
            .width(Parent)
            .size(14.0)
            .color(theme.text)
            .placeholder("Buscar aplicativos, jogos e programas...")
            .on_change(Self::set_query)
            .on_key(Self::handle_search_key);
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
                Column::new(vec![Box::new(search) as Box<dyn Widget>, app_list])
                    .width(Parent)
                    .height(Parent)
                    .gap(CONTENT_GAP)
                    .align(Start)
                    .justify(Start),
            )
    }
}

fn app_row(app: DesktopApp, theme: ThemeColors, selected: bool) -> Rectangle {
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

    let background = Rectangle::new()
        .width(Parent)
        .height(Parent)
        .radius(12.0)
        .fill(theme.accent)
        .opacity(if selected { 0.18 } else { 0.0 });

    let content = Rectangle::new()
        .width(Parent)
        .height(Parent)
        .padding(8.0)
        .align_child(Start, Center)
        .child(
            Row::new(vec![icon, Box::new(text)])
                .width(Parent)
                .height(Parent)
                .gap(14.0)
                .align(Center)
                .justify(Start),
        );

    Rectangle::new()
        .width(Parent)
        .height(ROW_HEIGHT)
        .radius(12.0)
        .cursor(Pointer)
        .on_click(move |_| {
            launch_app.launch();
            LauncherState::close();
        })
        .child(
            Stack::new(vec![Box::new(background), Box::new(content)])
                .width(Parent)
                .height(Parent),
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
