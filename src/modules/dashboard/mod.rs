mod dash;
mod media;
mod performance;
mod size_motion;
mod visualizer;
mod wallpapers;

use std::sync::atomic::{AtomicBool, Ordering};

use amane::{Center, Color, Column, Parent, Pointer, Rectangle, Row, Service};

use crate::material::{
    self, icon as material_icon, shape as material_shape, space as material_space,
};
use crate::theme::ThemeColors;

pub use size_motion::{sample as dashboard_size_sample, set_target as set_dashboard_size_target};

// Material 3 Navigation Rail's standard indicator width.
const NAVIGATION_WIDTH: f32 = 56.0;
const NAVIGATION_ITEM_HEIGHT: f32 = 42.0;
const NAVIGATION_INDICATOR_HEIGHT: f32 = 32.0;
const NAVIGATION_ICON_SIZE: f32 = 24.0;
const NAVIGATION_HEIGHT: f32 = 195.0;
const NAVIGATION_GAP: f32 = material_space::SM;
const PANEL_PADDING: f32 = material_space::LG;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum DashboardTab {
    #[default]
    Dash,
    Media,
    Performance,
    Wallpapers,
}

#[derive(Default)]
struct DashboardTabState {
    selected: DashboardTab,
}

impl Service for DashboardTabState {
    fn new() -> Self {
        Self::default()
    }

    fn listen() {}
}

static MEDIA_TAB_VISITED: AtomicBool = AtomicBool::new(false);
static PERFORMANCE_TAB_VISITED: AtomicBool = AtomicBool::new(false);

/// The panel and content dimensions for the currently selected Dashboard page.
/// Dimensions are derived from each page's layout, not a single global size.
#[derive(Clone, Copy, Debug)]
pub struct DashboardDimensions {
    pub content_width: f32,
    pub content_height: f32,
    pub panel_width: f32,
    pub panel_height: f32,
}

fn selected_tab() -> DashboardTab {
    DashboardTabState::read().selected
}

pub fn dashboard_dimensions() -> DashboardDimensions {
    let (content_width, content_height) = match selected_tab() {
        DashboardTab::Dash => dash::content_size(),
        DashboardTab::Media => media::content_size(),
        DashboardTab::Performance => performance::content_size(),
        DashboardTab::Wallpapers => wallpapers::content_size(),
    };

    let panel_width = content_width + NAVIGATION_WIDTH + NAVIGATION_GAP + PANEL_PADDING * 2.0;
    let panel_height = content_height.max(NAVIGATION_HEIGHT) + PANEL_PADDING * 2.0;

    DashboardDimensions {
        content_width,
        content_height,
        panel_width,
        panel_height,
    }
}

fn select_tab(tab: DashboardTab) {
    let current = selected_tab();
    if current == tab {
        return;
    }

    DashboardTabState::write().selected = tab;
}

/// Dashboard pages with a compact, vertically centered session selector.
pub fn dashboard_panel(
    theme: ThemeColors,
    visible: bool,
    animated_panel_width: f32,
    animated_panel_height: f32,
) -> Rectangle {
    let selected = selected_tab();
    let dimensions = dashboard_dimensions();
    let media_visible = visible && selected == DashboardTab::Media;

    if media_visible {
        MEDIA_TAB_VISITED.store(true, Ordering::Relaxed);
    } else if MEDIA_TAB_VISITED.load(Ordering::Relaxed) {
        media::disable();
    }

    let performance_visible = visible && selected == DashboardTab::Performance;
    if performance_visible {
        PERFORMANCE_TAB_VISITED.store(true, Ordering::Relaxed);
    } else if PERFORMANCE_TAB_VISITED.load(Ordering::Relaxed) {
        performance::set_enabled(false);
    }

    let content = if !visible {
        Rectangle::new()
            .width(dimensions.content_width)
            .height(dimensions.content_height)
    } else {
        match selected {
            DashboardTab::Dash => dash::view(theme),
            DashboardTab::Media => media::view(theme),
            DashboardTab::Performance => performance::view(theme),
            DashboardTab::Wallpapers => wallpapers::view(theme),
        }
    };

    let navigation = Column::new(vec![
        Box::new(tab_button(
            material::icon::HOME,
            selected == DashboardTab::Dash,
            theme,
            || select_tab(DashboardTab::Dash),
        )),
        Box::new(tab_button(
            material::icon::MUSIC_NOTE,
            selected == DashboardTab::Media,
            theme,
            || select_tab(DashboardTab::Media),
        )),
        Box::new(tab_button(
            material::icon::MONITORING,
            selected == DashboardTab::Performance,
            theme,
            || select_tab(DashboardTab::Performance),
        )),
        Box::new(tab_button(
            material::icon::WALLPAPER,
            selected == DashboardTab::Wallpapers,
            theme,
            || select_tab(DashboardTab::Wallpapers),
        )),
    ])
    .width(NAVIGATION_WIDTH)
    .height(NAVIGATION_HEIGHT)
    .gap(material_space::SM)
    .align(Center)
    .justify(Center);

    let content = Rectangle::new()
        .width(dimensions.content_width)
        .height(dimensions.content_height)
        .align_child(amane::Start, amane::Start)
        .child(content);

    Rectangle::new()
        .width(animated_panel_width)
        .height(animated_panel_height)
        .radius(material_shape::PANEL)
        .clip()
        .padding(PANEL_PADDING)
        .child(
            Row::new(vec![Box::new(content), Box::new(navigation)])
                .width(Parent)
                .height(Parent)
                .gap(NAVIGATION_GAP)
                // Keep the four-button session dock vertically centered beside each page.
                .align(Center)
                .justify(amane::Start),
        )
}

fn tab_button(
    icon: &'static str,
    active: bool,
    theme: ThemeColors,
    on_select: impl Fn() + 'static,
) -> Rectangle {
    let background = if active {
        theme.active_surface
    } else {
        Color::TRANSPARENT
    };
    let foreground = if active {
        theme.active_text
    } else {
        theme.muted
    };

    let indicator = Rectangle::new()
        .width(NAVIGATION_WIDTH)
        .height(NAVIGATION_INDICATOR_HEIGHT)
        .radius(material_shape::FULL)
        .fill(background)
        .align_child(Center, Center)
        .child(material_icon(icon, NAVIGATION_ICON_SIZE, foreground));

    Rectangle::new()
        .width(NAVIGATION_WIDTH)
        .height(NAVIGATION_ITEM_HEIGHT)
        .fill(Color::TRANSPARENT)
        .cursor(Pointer)
        .align_child(Center, Center)
        .child(indicator)
        .on_click(move |_| on_select())
}
