mod active_window;
mod drawer;
mod launcher;
mod modules;
mod motion;
mod special_workspace;
mod theme;
mod wallpaper;
mod workspace_motion;

use amane::{
    App, Apps, Center, Color, Column, End, Full, Horizontal, Image, InputArea, Keyboard, Layer,
    LayerWindow, Monitor, Parent, Pointer, Rectangle, Service, SpaceBetween, Stack, Start, Text,
    Vertical, Widget, Workspace, Workspaces, Zone,
};

use active_window::ActiveWindow;
use drawer::DrawerState;
use launcher::{
    LauncherState, PANEL_BOTTOM_GAP, PANEL_HEIGHT as LAUNCHER_HEIGHT, PANEL_WIDTH as LAUNCHER_WIDTH,
};
use modules::dashboard::{
    dashboard_dimensions, dashboard_panel, dashboard_size_sample, set_dashboard_size_target,
};
use motion::{Motion, PanelId};
use special_workspace::SpecialWorkspace;
use theme::{Theme, ThemeColors, ThemeMode};
use wallpaper::WallpaperService;
use workspace_motion::WorkspaceMotion;

const RAIL_WIDTH: f32 = 60.0;
const WORKSPACE_INDICATOR_SIZE: f32 = 44.0;
const WORKSPACE_GAP: f32 = 3.0;
const BORDER_SIZE: f32 = 10.0;
const BORDER_RADIUS: f32 = 24.0;
const INNER_LEFT_RADIUS: f32 = 30.0;
const INNER_RIGHT_RADIUS: f32 = 24.0;
const DASHBOARD_TOP: f32 = BORDER_SIZE;
const DASHBOARD_MERGE_RADIUS: f32 = 19.0;

// Hover targets are placed on the monitor edges where each panel emerges.
const ORIGIN_HOVER_DEPTH: f32 = BORDER_SIZE + 28.0;
// --------------------------------------------------
// MAIN
// --------------------------------------------------

fn main() {
    // Subscribe to Hyprland's active-window events before the first frame.
    let _ = ActiveWindow::current();

    // Start Amane's desktop-entry scan before the Launcher is first opened.
    // Apps::listen scans XDG application and icon directories asynchronously.
    let _ = Apps::read().list().len();

    App::new()
        .font("Rubik")
        .window_per_monitor(background)
        // Quatro exclusões invisíveis: esquerda (rail), topo, direita e base.
        .window_per_monitor(exclusion_left)
        .window_per_monitor(exclusion_top)
        .window_per_monitor(exclusion_right)
        .window_per_monitor(exclusion_bottom)
        // Uma única janela reúne moldura, rail, Dashboard e Launcher.
        .window_per_monitor(content_window)
        .run();
}

// --------------------------------------------------
// BACKGROUND
// --------------------------------------------------

fn background(_monitor: &Monitor) -> LayerWindow {
    let (previous, current, progress) = WallpaperService::transition_snapshot();

    let mut layers: Vec<Box<dyn Widget>> = Vec::new();

    if let Some(path) = previous {
        let path = path.to_string_lossy().into_owned();

        layers.push(Box::new(
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .fill(Image::cover(path.as_str()))
                .opacity(1.0 - progress),
        ));
    }

    if let Some(path) = current {
        let path = path.to_string_lossy().into_owned();

        layers.push(Box::new(
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .fill(Image::cover(path.as_str()))
                .opacity(progress),
        ));
    }

    if layers.is_empty() {
        layers.push(Box::new(Rectangle::new().width(Parent).height(Parent)));
    }

    LayerWindow::new()
        .width(Full)
        .height(Full)
        .layer(Layer::Background)
        .space(Zone::Ignore)
        .namespace("caelestia-rs-background")
        .click_through()
        .child(Stack::new(layers))
}

// --------------------------------------------------
// EXCLUSIONS
// --------------------------------------------------

fn exclusion_left(_monitor: &Monitor) -> LayerWindow {
    LayerWindow::new()
        .width(RAIL_WIDTH)
        .height(Full)
        .anchor_horizontal(Horizontal::Left)
        .layer(Layer::Top)
        .space(Zone::Reserve)
        .namespace("caelestia-rs-exclusion-left")
        .click_through()
        .child(Rectangle::new().width(Parent).height(Parent))
}

fn exclusion_top(_monitor: &Monitor) -> LayerWindow {
    LayerWindow::new()
        .width(Full)
        .height(BORDER_SIZE)
        .anchor_vertical(Vertical::Top)
        .layer(Layer::Top)
        .space(Zone::Reserve)
        .namespace("caelestia-rs-exclusion-top")
        .click_through()
        .child(Rectangle::new().width(Parent).height(Parent))
}

fn exclusion_right(_monitor: &Monitor) -> LayerWindow {
    LayerWindow::new()
        .width(BORDER_SIZE)
        .height(Full)
        .anchor_horizontal(Horizontal::Right)
        .layer(Layer::Top)
        .space(Zone::Reserve)
        .namespace("caelestia-rs-exclusion-right")
        .click_through()
        .child(Rectangle::new().width(Parent).height(Parent))
}

fn exclusion_bottom(_monitor: &Monitor) -> LayerWindow {
    LayerWindow::new()
        .width(Full)
        .height(BORDER_SIZE)
        .anchor_vertical(Vertical::Bottom)
        .layer(Layer::Top)
        .space(Zone::Reserve)
        .namespace("caelestia-rs-exclusion-bottom")
        .click_through()
        .child(Rectangle::new().width(Parent).height(Parent))
}

// --------------------------------------------------
// UNIFIED CONTENT WINDOW
// --------------------------------------------------

fn content_window(monitor: &Monitor) -> LayerWindow {
    let theme = Theme::snapshot();

    // BlobRect follows the Dashboard's actual scene-space velocity.
    // The same affine deformation is applied to its widget and to its SDF.
    let dashboard_target = dashboard_dimensions();
    set_dashboard_size_target(dashboard_target.panel_width, dashboard_target.panel_height);
    let (panel_width, panel_height) = dashboard_size_sample();
    let travel_y = panel_height + DASHBOARD_TOP;
    let motion = DrawerState::sample(travel_y);

    let motion_progress = motion.progress;
    let shape_progress = motion_progress.clamp(0.0, 1.0);
    let deformation = motion.deformation;

    let panel_left = ((monitor.width as f32 - panel_width) / 2.0).max(0.0);
    let panel_top = -panel_height + travel_y * motion_progress;

    // The Launcher uses the shared spring and BlobRect deformation engine,
    // but travels upward from the bottom-center origin.
    let launcher_travel = LAUNCHER_HEIGHT + PANEL_BOTTOM_GAP;
    let launcher_motion = LauncherState::sample(-launcher_travel);
    let launcher_progress = launcher_motion.progress.clamp(0.0, 1.0);
    let launcher_deformation = launcher_motion.deformation;
    // Building every desktop-entry row on every animation frame is expensive.
    // App scanning is prewarmed during startup and the list is virtualized, so
    // its first visible rows can be built during the opening transition.
    let render_launcher_apps = launcher_progress > 0.001;
    // The panel is bottom-aligned. When open, its lower edge meets the inner
    // edge of the bottom frame; when closed, it is fully below the monitor.
    let launcher_translate_y = LAUNCHER_HEIGHT - launcher_travel * launcher_progress;
    let launcher_panel_left = ((monitor.width as f32 - LAUNCHER_WIDTH) / 2.0).max(0.0);
    let launcher_panel_top = monitor.height as f32 - launcher_travel * launcher_progress;

    // Keep each invisible origin target only on the part of the edge that is
    // not already covered by its moving panel. This lets the panel itself own
    // hover events as soon as it reaches the pointer.
    let dashboard_origin_y = (panel_top + panel_height).clamp(0.0, ORIGIN_HOVER_DEPTH);
    let dashboard_origin_height = (ORIGIN_HOVER_DEPTH - dashboard_origin_y).max(0.0);
    let launcher_origin_y = monitor.height as f32 - ORIGIN_HOVER_DEPTH;
    let launcher_origin_bottom = launcher_panel_top.clamp(launcher_origin_y, monitor.height as f32);
    let launcher_origin_height = (launcher_origin_bottom - launcher_origin_y).max(0.0);

    // Pointer input covers the rail and the two panel-origin areas.
    // Dashboard emerges from the top-center edge; Launcher emerges from bottom-center.
    // These source regions remain interactive even when the panels are hidden.
    let mut input_areas = vec![
        InputArea {
            x: 0,
            y: 0,
            width: RAIL_WIDTH as i32,
            height: monitor.height as i32,
        },
        InputArea {
            x: panel_left.floor() as i32,
            y: 0,
            width: panel_width.ceil() as i32,
            height: ORIGIN_HOVER_DEPTH.ceil() as i32,
        },
        InputArea {
            x: launcher_panel_left.floor() as i32,
            y: (monitor.height as f32 - ORIGIN_HOVER_DEPTH).floor() as i32,
            width: LAUNCHER_WIDTH.ceil() as i32,
            height: ORIGIN_HOVER_DEPTH.ceil() as i32,
        },
    ];

    if motion_progress > 0.001 {
        // The transformed panel can extend beyond its layout rectangle. Expand
        // the layer-shell input area to the exact axis-aligned bounds of its
        // centered affine transform so pointer interaction follows the shape.
        let half_width = panel_width * 0.5;
        let half_height = panel_height * 0.5;
        let extent_x = deformation.sx.abs() * half_width + deformation.kx.abs() * half_height;
        let extent_y = deformation.ky.abs() * half_width + deformation.sy.abs() * half_height;
        let center_x = panel_left + half_width;
        let center_y = panel_top + half_height;

        let input_left = (center_x - extent_x).max(0.0);
        let input_right = (center_x + extent_x).min(monitor.width as f32);
        let input_top = (center_y - extent_y).max(0.0);
        let input_bottom = (center_y + extent_y).min(monitor.height as f32);

        if input_right > input_left && input_bottom > input_top {
            input_areas.push(InputArea {
                x: input_left.floor() as i32,
                y: input_top.floor() as i32,
                width: (input_right - input_left).ceil() as i32,
                height: (input_bottom - input_top).ceil() as i32,
            });
        }
    }

    if launcher_motion.is_open {
        // While the Launcher is active, keep the whole monitor in the input
        // region so clicks outside the panel can dismiss it.
        input_areas.clear();
        input_areas.push(InputArea {
            x: 0,
            y: 0,
            width: monitor.width as i32,
            height: monitor.height as i32,
        });
    }

    if launcher_progress > 0.001 && !launcher_motion.is_open {
        // During the closing animation, keep only the panel bounds interactive.
        // While open, the full-monitor area above already covers this rectangle.
        let half_width = LAUNCHER_WIDTH * 0.5;
        let half_height = LAUNCHER_HEIGHT * 0.5;
        let extent_x = launcher_deformation.sx.abs() * half_width
            + launcher_deformation.kx.abs() * half_height;
        let extent_y = launcher_deformation.ky.abs() * half_width
            + launcher_deformation.sy.abs() * half_height;
        let center_x = monitor.width as f32 * 0.5;
        let center_y = launcher_panel_top + half_height;

        let input_left = (center_x - extent_x).max(0.0);
        let input_right = (center_x + extent_x).min(monitor.width as f32);
        let input_top = (center_y - extent_y).max(0.0);
        let input_bottom = (center_y + extent_y).min(monitor.height as f32);

        if input_right > input_left && input_bottom > input_top {
            input_areas.push(InputArea {
                x: input_left.floor() as i32,
                y: input_top.floor() as i32,
                width: (input_right - input_left).ceil() as i32,
                height: (input_bottom - input_top).ceil() as i32,
            });
        }
    }

    let surface = theme.bar_surface;

    let shader_values = vec![
        // Rail width, frame thickness, inner-left radius, inner-right radius.
        [
            RAIL_WIDTH,
            BORDER_SIZE,
            INNER_LEFT_RADIUS,
            INNER_RIGHT_RADIUS,
        ],
        // Shared surface RGBA.
        [
            surface.red() as f32 / 255.0,
            surface.green() as f32 / 255.0,
            surface.blue() as f32 / 255.0,
            surface.alpha() as f32 / 255.0,
        ],
        // Dashboard geometry in full-window coordinates.
        [panel_left, panel_top, panel_width, panel_height],
        // Dashboard radius, progress and merge radius.
        [BORDER_RADIUS, shape_progress, DASHBOARD_MERGE_RADIUS, 0.0],
        // Same centered 2D deformation matrix used by the Dashboard widget.
        [
            deformation.sx,
            deformation.kx,
            deformation.ky,
            deformation.sy,
        ],
        // Launcher geometry in full-window coordinates.
        [
            launcher_panel_left,
            launcher_panel_top,
            LAUNCHER_WIDTH,
            LAUNCHER_HEIGHT,
        ],
        // Launcher corner radius, visibility progress and merge radius.
        [26.0, launcher_progress, 19.0, 0.0],
        // Same centered 2D deformation matrix used by the Launcher widget.
        [
            launcher_deformation.sx,
            launcher_deformation.kx,
            launcher_deformation.ky,
            launcher_deformation.sy,
        ],
    ];

    let surface_layer = Rectangle::new()
        .width(Parent)
        .height(Parent)
        .fill(Color::TRANSPARENT)
        .shader("shaders/shell-frame.frag")
        .shader_values(shader_values);

    // The rail contains controls only. Its background is supplied by the SDF.
    let rail_layer = Rectangle::new()
        .width(Parent)
        .height(Parent)
        .align_child(Start, Start)
        .child(rail(theme, monitor.height as f32, &monitor.name));

    // The Dashboard shares the same SDF surface and retains its centered top anchor.
    let dashboard_layer = Rectangle::new()
        .width(Parent)
        .height(Parent)
        .align_child(Center, Start)
        .child(Motion::panel_hover(
            PanelId::Dashboard,
            dashboard_panel(theme, DrawerState::is_open(), panel_width, panel_height)
                .transform_matrix(deformation)
                .translate(0.0, panel_top),
        ));

    // The Launcher shares the fullscreen SDF with the rail and bottom frame.
    // Its bottom edge meets the inner edge of the bottom frame, so the bottom
    // perimeter visually connects the panel back to the left rail.
    let launcher_layer = Rectangle::new()
        .width(Parent)
        .height(Parent)
        .align_child(Center, End)
        .child(LauncherState::hover_area(
            LauncherState::panel(theme, render_launcher_apps)
                .transform_matrix(launcher_motion.deformation)
                .translate(0.0, launcher_translate_y),
        ));

    // Hover is detected at each module's origin in the perimeter, not on rail icons.
    // Dashboard starts at the top-center edge.
    let dashboard_origin_hover = Motion::trigger_hover(
        PanelId::Dashboard,
        Rectangle::new()
            .width(panel_width)
            .height(dashboard_origin_height)
            .fill(Color::TRANSPARENT)
            .cursor(Pointer)
            .translate(panel_left, dashboard_origin_y),
    );

    // Launcher starts at the bottom-center edge.
    let launcher_origin_hover = Motion::trigger_hover_latched(
        PanelId::Launcher,
        Rectangle::new()
            .width(LAUNCHER_WIDTH)
            .height(launcher_origin_height)
            .fill(Color::TRANSPARENT)
            .cursor(Pointer)
            .translate(launcher_panel_left, launcher_origin_y),
    );

    LayerWindow::new()
        .width(Full)
        .height(Full)
        .layer(Layer::Top)
        .space(Zone::Ignore)
        .namespace("caelestia-rs-bar")
        // The layer surface only takes keyboard focus while the Launcher is open.
        // Exclusive is necessary because OnDemand would require a click first.
        .keyboard(if LauncherState::is_open() {
            Keyboard::Exclusive
        } else {
            Keyboard::None
        })
        .input_region(input_areas)
        .child(
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                // The full-screen input region is enabled only while the
                // Launcher is open. A click not handled by a panel/control
                // therefore dismisses it, even after the pointer left the panel.
                .on_click(|_| {
                    if LauncherState::is_open() {
                        LauncherState::close();
                    }
                })
                .child(
                    Stack::new(vec![
                        Box::new(surface_layer),
                        Box::new(rail_layer),
                        Box::new(dashboard_layer),
                        Box::new(launcher_layer),
                        Box::new(dashboard_origin_hover),
                        Box::new(launcher_origin_hover),
                    ])
                    .width(Parent)
                    .height(Parent),
                ),
        )
}

// --------------------------------------------------
// RAIL CONTENT
// --------------------------------------------------

fn rail(theme: ThemeColors, height: f32, output: &str) -> Rectangle {
    let special_active = SpecialWorkspace::active_on(output);

    let workspaces = Workspaces::read();
    let workspace_list = workspaces.list();
    let workspace_count = workspace_list.len();

    let active_index = workspace_list
        .iter()
        .position(|workspace| workspace.active());

    // The group target follows the number of regular workspaces. It is kept
    // unchanged while the special workspace is visible.
    let regular_height = if workspace_count == 0 {
        WORKSPACE_INDICATOR_SIZE
    } else {
        workspace_count as f32 * WORKSPACE_INDICATOR_SIZE
            + workspace_count.saturating_sub(1) as f32 * WORKSPACE_GAP
    };

    WorkspaceMotion::set_target(
        output,
        active_index,
        workspace_count,
        special_active,
        regular_height,
    );

    let workspace_motion = WorkspaceMotion::sample(output);
    let workspace_group_height = WorkspaceMotion::group_height(output, regular_height);
    let special_progress = workspace_motion.special_progress;
    let regular_opacity = 1.0 - special_progress;

    let mut workspace_buttons: Vec<Box<dyn Widget>> = Vec::new();
    if special_progress < 0.999 {
        for (index, workspace) in workspace_list.iter().enumerate() {
            let active_progress = WorkspaceMotion::workspace_progress(output, index);
            workspace_buttons.push(Box::new(workspace_button(
                workspace,
                theme,
                active_progress,
            )));
        }
    }

    let workspace_column = Column::new(workspace_buttons)
        .width(Parent)
        .height(regular_height)
        .gap(WORKSPACE_GAP)
        .align(Center)
        .justify(Start);

    let mut workspace_layers: Vec<Box<dyn Widget>> = Vec::new();

    // A subtly translucent surface expands and contracts with the workspace
    // count. Its height is independent of the special-workspace crossfade.
    workspace_layers.push(Box::new(
        Rectangle::new()
            .width(Parent)
            .height(workspace_group_height)
            .align_child(Center, Center)
            .child(
                Rectangle::new()
                    .width(36.0)
                    .height(Parent)
                    .radius(19.0)
                    .fill(theme.workspace_group_surface)
                    .opacity(0.99),
            ),
    ));

    workspace_layers.push(Box::new(
        Rectangle::new()
            .width(Parent)
            .height(regular_height)
            .opacity(regular_opacity)
            .child(workspace_column),
    ));

    // Keep the special-workspace star centered in the full group, without
    // collapsing the group's height.
    if special_progress > 0.001 {
        workspace_layers.push(Box::new(
            Rectangle::new()
                .width(Parent)
                .height(workspace_group_height)
                .opacity(special_progress)
                .align_child(Center, Center)
                .child(
                    Text::new("\u{f526}")
                        .font("Material Symbols Rounded")
                        .size(22.0 * workspace_motion.special_scale)
                        .color(theme.active_text),
                ),
        ));
    }

    let workspace_group = Rectangle::new()
        .width(Parent)
        .height(workspace_group_height)
        .clip()
        .child(
            Stack::new(workspace_layers)
                .width(Parent)
                .height(workspace_group_height),
        );

    let workspace_section = Column::new(vec![
        Box::new(Rectangle::new().width(1.0).height(10.0)),
        Box::new(workspace_group),
    ])
    .width(Parent)
    .gap(10.0)
    .align(Center)
    .justify(Start);

    // The active-window title uses the free vertical space between the
    // workspaces and the bottom action plate, like Caelestia's bar component.
    let window_area_height =
        (height - 12.0 - (20.0 + workspace_group_height) - 96.0).max(80.0);
    let window_section = active_window_area(theme, window_area_height);

    // One shared background behind both bottom actions, using the exact
    // surface colour of the active workspace indicator. Individual buttons
    // remain transparent.
    let bottom_actions = Column::new(vec![
        Box::new(theme_button(theme)),
        Box::new(session_button(theme)),
    ])
    .width(Parent)
    .height(Parent)
    .gap(7.0)
    .align(Center)
    .justify(Center);

    let bottom_section = Rectangle::new()
        .width(36.0)
        .height(96.0)
        .radius(16.0)
        .fill(theme.active_surface)
        .opacity(0.97)
        .align_child(Center, Center)
        .child(bottom_actions);

    Rectangle::new()
        .width(RAIL_WIDTH)
        .height(height)
        .padding(6.0)
        .align_child(Center, Center)
        .child(
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .child(
                    Column::new(vec![
                        Box::new(workspace_section),
                        Box::new(window_section),
                        Box::new(bottom_section),
                    ])
                    .width(Parent)
                    .height(Parent)
                    .align(Center)
                    .justify(SpaceBetween),
                )
                // The content was a couple of pixels right of the visual center.
                .translate(-2.0, 0.0),
        )
}

// --------------------------------------------------
// WORKSPACES
// --------------------------------------------------

fn workspace_button(workspace: &Workspace, theme: ThemeColors, active_progress: f32) -> Rectangle {
    let id = workspace.id();

    // Keep the existing Waybar-inspired morph, but make the indicators
    // narrower and remove their numeric labels entirely.
    let morph = active_progress.clamp(-0.10, 1.10);
    let active_opacity = active_progress.clamp(0.0, 1.0);
    let width = 14.0 + 3.0 * morph;
    let indicator_height = 30.0 + 10.0 * morph;
    let radius = indicator_height * 0.5;
    let x = (WORKSPACE_INDICATOR_SIZE - width) * 0.5;
    let y = (WORKSPACE_INDICATOR_SIZE - indicator_height) * 0.5;

    let mut layers: Vec<Box<dyn Widget>> = Vec::new();

    layers.push(Box::new(
        Rectangle::new()
            .width(width)
            .height(indicator_height)
            .radius(radius)
            .fill(theme.workspace_occupied_surface)
            .opacity(0.97)
            .translate(x, y),
    ));

    if active_opacity > 0.001 {
        layers.push(Box::new(
            Rectangle::new()
                .width(width)
                .height(indicator_height)
                .radius(radius)
                .fill(theme.active_surface)
                .opacity(active_opacity * 0.97)
                .translate(x, y),
        ));
    }

    Rectangle::new()
        .width(WORKSPACE_INDICATOR_SIZE)
        .height(WORKSPACE_INDICATOR_SIZE)
        .fill(Color::TRANSPARENT)
        .cursor(Pointer)
        .child(Stack::new(layers).width(Parent).height(Parent))
        .on_click(move |_| {
            Workspaces::focus(id);
        })
}

fn active_window_area(theme: ThemeColors, available_height: f32) -> Rectangle {
    let window = ActiveWindow::current();
    let title = window
        .as_ref()
        .map(|window| window.title.as_str())
        .filter(|title| !title.trim().is_empty())
        .unwrap_or("Desktop");

    // Caelestia rotates a single title label by 90 degrees and elides it to
    // the available vertical space. This shell keeps only the text (no icon)
    // and uses a character cap as a predictable approximation of text metrics.
    let font_size = 12.0;
    let max_chars = (((available_height - 16.0) / (font_size * 0.62)).floor() as usize)
        .clamp(8, 64);
    let title_label = compact_window_label(title, max_chars);

    // Lay out the text horizontally first, then rotate the entire line around
    // its center so the visible text runs vertically along the rail.
    let rotated_line = Rectangle::new()
        .width(available_height)
        .height(48.0)
        .fill(Color::TRANSPARENT)
        .align_child(Center, Center)
        .child(Text::new(title_label).size(font_size).color(theme.text))
        .rotate(90.0);

    Rectangle::new()
        .width(Parent)
        .height(available_height)
        .fill(Color::TRANSPARENT)
        .clip()
        .align_child(Center, Center)
        .child(rotated_line)
}

fn compact_window_label(value: &str, max_chars: usize) -> String {
    let value = value.trim();
    let chars: Vec<char> = value.chars().collect();

    if chars.len() <= max_chars {
        return value.to_owned();
    }

    let mut result: String = chars
        .iter()
        .take(max_chars.saturating_sub(3))
        .copied()
        .collect();
    result.push_str("...");
    result
}

// --------------------------------------------------
// ICON BUTTON
// --------------------------------------------------

fn icon_button(icon: &'static str, foreground: Color) -> Rectangle {
    Rectangle::new()
        .width(32.0)
        .height(38.0)
        .fill(Color::TRANSPARENT)
        .cursor(Pointer)
        .align_child(Center, Center)
        .child(
            Text::new(icon)
                .font("Material Symbols Rounded")
                .size(19.0)
                .color(foreground),
        )
}

// --------------------------------------------------
// LIGHT / DARK
// --------------------------------------------------

fn theme_button(theme: ThemeColors) -> Rectangle {
    let icon = match theme.mode {
        ThemeMode::Dark => "\u{e518}",
        ThemeMode::Light => "\u{e51c}",
    };

    icon_button(icon, theme.active_text).on_click(|_| {
        Theme::toggle();
    })
}

// --------------------------------------------------
// SESSION
// --------------------------------------------------

fn session_button(theme: ThemeColors) -> Rectangle {
    icon_button("\u{f8c7}", theme.active_text).on_click(|_| {
        println!("caelestia-rs: session");
    })
}
