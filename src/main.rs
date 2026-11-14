mod drawer;
mod launcher;
mod motion;
mod special_workspace;
mod theme;
mod wallpaper;

use amane::{
    App, Apps, Canvas, Center, Color, Column, End, Full, Horizontal, Image, InputArea, Keyboard,
    Layer, LayerWindow, Monitor, Parent, Path, Pointer, Rectangle, Service, Shape, SpaceBetween,
    Stack, Start, Text, Vertical, Widget, Workspace, Workspaces, Zone,
};

use drawer::{DASHBOARD_HEIGHT, DASHBOARD_WIDTH, DrawerState};
use launcher::{
    LauncherState, PANEL_BOTTOM_GAP, PANEL_HEIGHT as LAUNCHER_HEIGHT, PANEL_WIDTH as LAUNCHER_WIDTH,
};
use motion::{Motion, PanelId};
use special_workspace::SpecialWorkspace;
use theme::{Theme, ThemeColors, ThemeMode};
use wallpaper::WallpaperService;

const RAIL_WIDTH: f32 = 60.0;
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
    let panel_width = DASHBOARD_WIDTH;
    let panel_height = DASHBOARD_HEIGHT;
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
            dashboard_panel(theme)
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
    let mut workspace_buttons: Vec<Box<dyn Widget>> = Vec::new();

    if special_active {
        // A visible special workspace replaces the regular workspace list.
        workspace_buttons.push(Box::new(special_workspace_button(theme)));
    } else {
        let workspaces = Workspaces::read();

        for workspace in workspaces.list().iter() {
            workspace_buttons.push(Box::new(workspace_button(workspace, theme)));
        }
    }

    let workspace_group = Column::new(workspace_buttons)
        .width(Parent)
        .gap(6.0)
        .align(Center)
        .justify(Start);

    // The central slot is reserved for active-window metadata. Its data source
    // will be connected later; for now it identifies the future information area.
    let workspace_section = Column::new(vec![Box::new(workspace_group), Box::new(divider(theme))])
        .width(Parent)
        .gap(10.0)
        .align(Center)
        .justify(Start);

    let window_section = active_window_area(theme);

    let bottom_section = Column::new(vec![
        Box::new(dashboard_button(theme)),
        Box::new(home_button(theme)),
        Box::new(divider(theme)),
        Box::new(wallpaper_button(theme)),
        Box::new(theme_button(theme)),
        Box::new(divider(theme)),
        Box::new(launcher_button(theme)),
        Box::new(session_button(theme)),
    ])
    .width(Parent)
    .gap(6.0)
    .align(Center)
    .justify(End);

    // Workspaces stay at the top, the active-window slot occupies the middle,
    // and the existing shell actions remain grouped at the bottom.
    Rectangle::new()
        .width(RAIL_WIDTH)
        .height(height)
        .padding(6.0)
        .align_child(Center, Center)
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
}

// --------------------------------------------------
// DASHBOARD PANEL
// --------------------------------------------------

fn dashboard_panel(theme: ThemeColors) -> Rectangle {
    let intro_card = Rectangle::new()
        .width(Parent)
        .height(132.0)
        .radius(20.0)
        .fill(theme.button_surface)
        .padding(18.0)
        .align_child(Start, Start)
        .child(
            Column::new(vec![
                Box::new(Text::new("G-Shell").size(19.0).color(theme.text)),
                Box::new(
                    Text::new("Seu ambiente, em um só lugar.")
                        .size(13.0)
                        .color(theme.muted),
                ),
            ])
            .width(Parent)
            .height(Parent)
            .gap(8.0)
            .align(Start)
            .justify(Start),
        );

    let module_card = Rectangle::new()
        .width(Parent)
        .height(108.0)
        .radius(20.0)
        .fill(theme.button_surface)
        .padding(18.0)
        .align_child(Start, Start)
        .child(
            Column::new(vec![
                Box::new(Text::new("Módulos").size(16.0).color(theme.text)),
                Box::new(
                    Text::new("Media e Desempenho serão integrados aqui.")
                        .size(13.0)
                        .color(theme.muted),
                ),
            ])
            .width(Parent)
            .height(Parent)
            .gap(8.0)
            .align(Start)
            .justify(Start),
        );

    // Transparent content container: the shared SDF paints the panel behind it.
    Rectangle::new()
        .width(DASHBOARD_WIDTH)
        .height(DASHBOARD_HEIGHT)
        .radius(BORDER_RADIUS)
        .clip()
        .padding(24.0)
        .align_child(Start, Start)
        .child(
            Column::new(vec![
                Box::new(Text::new("Dashboard").size(26.0).color(theme.text)),
                Box::new(Text::new("Visão geral").size(13.0).color(theme.muted)),
                Box::new(intro_card),
                Box::new(module_card),
            ])
            .width(Parent)
            .height(Parent)
            .gap(14.0)
            .align(Start)
            .justify(Start),
        )
}

// --------------------------------------------------
// DASHBOARD BUTTON
// --------------------------------------------------

fn dashboard_button(theme: ThemeColors) -> Rectangle {
    let open = DrawerState::is_open();

    let background = if open {
        theme.active_surface
    } else {
        theme.button_surface
    };

    let foreground = if open {
        theme.active_text
    } else {
        theme.accent
    };

    icon_button("\u{e871}", background, foreground).on_click(|_| Motion::open(PanelId::Dashboard))
}

// --------------------------------------------------
// ICON BUTTON
// --------------------------------------------------

fn icon_button(icon: &'static str, background: Color, foreground: Color) -> Rectangle {
    Rectangle::new()
        .width(38.0)
        .height(38.0)
        .radius(13.0)
        .fill(background)
        .cursor(Pointer)
        .align_child(Center, Center)
        .child(
            Text::new(icon)
                .font("Material Symbols Rounded")
                .size(21.0)
                .color(foreground),
        )
}

// --------------------------------------------------
// HOME
// --------------------------------------------------

fn home_button(theme: ThemeColors) -> Rectangle {
    icon_button("\u{e9b2}", theme.button_surface, theme.muted).on_click(|_| {
        println!("caelestia-rs: home");
    })
}

// --------------------------------------------------
// WORKSPACES
// --------------------------------------------------

fn workspace_button(workspace: &Workspace, theme: ThemeColors) -> Rectangle {
    let active = workspace.active();
    let occupied = workspace.windows() > 0;
    let id = workspace.id();

    let foreground = if active {
        theme.active_text
    } else {
        theme.text
    };

    let label = workspace
        .name()
        .map(str::to_owned)
        .unwrap_or_else(|| workspace.index().to_string());

    let background = if active {
        Some((6, theme.active_surface))
    } else if occupied {
        Some((4, theme.button_surface))
    } else {
        None
    };

    workspace_indicator(label, foreground, background, false)
        .cursor(Pointer)
        .on_click(move |_| {
            Workspaces::focus(id);
        })
}

fn special_workspace_button(theme: ThemeColors) -> Rectangle {
    workspace_indicator(
        "\u{e838}".to_owned(),
        theme.active_text,
        Some((6, theme.active_surface)),
        true,
    )
}

fn workspace_indicator(
    label: String,
    foreground: Color,
    background: Option<(usize, Color)>,
    is_icon: bool,
) -> Rectangle {
    let label = if is_icon {
        Text::new(label)
            .font("Material Symbols Rounded")
            .size(21.0)
            .color(foreground)
    } else {
        Text::new(label).size(14.0).color(foreground)
    };

    let label_layer = Rectangle::new()
        .width(Parent)
        .height(Parent)
        .align_child(Center, Center)
        .child(label);

    let mut layers: Vec<Box<dyn Widget>> = Vec::new();

    if let Some((sides, fill)) = background {
        layers.push(Box::new(cookie_canvas(38.0, sides, fill)));
    }

    layers.push(Box::new(label_layer));

    Rectangle::new()
        .width(38.0)
        .height(38.0)
        .fill(Color::TRANSPARENT)
        .child(Stack::new(layers).width(Parent).height(Parent))
}

// A compact placeholder for future active-window metadata.
fn active_window_area(theme: ThemeColors) -> Rectangle {
    Rectangle::new()
        .width(44.0)
        .height(76.0)
        .radius(15.0)
        .fill(theme.button_surface)
        .align_child(Center, Center)
        .child(
            Column::new(vec![
                Box::new(
                    Text::new("\u{e8f4}")
                        .font("Material Symbols Rounded")
                        .size(19.0)
                        .color(theme.muted),
                ),
                Box::new(Text::new("Janela").size(9.0).color(theme.muted)),
                Box::new(Text::new("ativa").size(9.0).color(theme.muted)),
            ])
            .align(Center)
            .justify(Center)
            .gap(3.0),
        )
}

// Material 3's Cookie4Sided and Cookie6Sided are rounded polygons. This helper
// reproduces their repeated vertices and rounded corners as a small Canvas path.
fn cookie_canvas(size: f32, sides: usize, fill: Color) -> Canvas {
    #[derive(Clone, Copy)]
    struct Vertex {
        x: f32,
        y: f32,
        rounding: f32,
    }

    let (seeds, repetitions): ([(f32, f32, f32); 2], usize) = match sides {
        4 => ([(1.237, 1.236, 0.258), (0.500, 0.918, 0.233)], 4),
        _ => ([(0.723, 0.884, 0.394), (0.500, 1.099, 0.398)], 6),
    };

    let mut vertices = Vec::with_capacity(repetitions * seeds.len());

    for repetition in 0..repetitions {
        let angle = repetition as f32 * std::f32::consts::TAU / repetitions as f32;
        let (sin, cos) = angle.sin_cos();

        for (x, y, rounding) in seeds {
            let dx = x - 0.5;
            let dy = y - 0.5;

            vertices.push(Vertex {
                x: 0.5 + dx * cos - dy * sin,
                y: 0.5 + dx * sin + dy * cos,
                rounding,
            });
        }
    }

    let min_x = vertices
        .iter()
        .map(|point| point.x)
        .fold(f32::INFINITY, f32::min);
    let max_x = vertices
        .iter()
        .map(|point| point.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = vertices
        .iter()
        .map(|point| point.y)
        .fold(f32::INFINITY, f32::min);
    let max_y = vertices
        .iter()
        .map(|point| point.y)
        .fold(f32::NEG_INFINITY, f32::max);

    let available = size - 4.0;
    let scale = available / (max_x - min_x).max(max_y - min_y);
    let offset_x = (size - (max_x - min_x) * scale) * 0.5;
    let offset_y = (size - (max_y - min_y) * scale) * 0.5;

    for point in &mut vertices {
        point.x = (point.x - min_x) * scale + offset_x;
        point.y = (point.y - min_y) * scale + offset_y;
    }

    let mut entries = Vec::with_capacity(vertices.len());
    let mut exits = Vec::with_capacity(vertices.len());

    for index in 0..vertices.len() {
        let previous = vertices[(index + vertices.len() - 1) % vertices.len()];
        let current = vertices[index];
        let next = vertices[(index + 1) % vertices.len()];

        let to_previous = (previous.x - current.x, previous.y - current.y);
        let to_next = (next.x - current.x, next.y - current.y);
        let previous_length =
            (to_previous.0 * to_previous.0 + to_previous.1 * to_previous.1).sqrt();
        let next_length = (to_next.0 * to_next.0 + to_next.1 * to_next.1).sqrt();

        let previous_unit = (
            to_previous.0 / previous_length.max(0.001),
            to_previous.1 / previous_length.max(0.001),
        );
        let next_unit = (
            to_next.0 / next_length.max(0.001),
            to_next.1 / next_length.max(0.001),
        );

        let dot = (previous_unit.0 * next_unit.0 + previous_unit.1 * next_unit.1).clamp(-1.0, 1.0);
        let interior_half_angle = dot.acos() * 0.5;
        let tangent = interior_half_angle.tan().max(0.001);
        let corner_radius = current.rounding * available;
        let distance = (corner_radius / tangent)
            .min(previous_length * 0.46)
            .min(next_length * 0.46);

        entries.push((
            current.x + previous_unit.0 * distance,
            current.y + previous_unit.1 * distance,
        ));
        exits.push((
            current.x + next_unit.0 * distance,
            current.y + next_unit.1 * distance,
        ));
    }

    let mut path = Path::new().move_to(entries[0].0, entries[0].1);

    for index in 0..vertices.len() {
        let current = vertices[index];
        let exit = exits[index];
        let next_entry = entries[(index + 1) % vertices.len()];

        path = path
            .quad_to(current.x, current.y, exit.0, exit.1)
            .line_to(next_entry.0, next_entry.1);
    }

    let path = path.close().fill(fill);

    Canvas::new()
        .width(size)
        .height(size)
        .shapes(vec![Box::new(path)])
}

// --------------------------------------------------
// WALLPAPER
// --------------------------------------------------

fn wallpaper_button(theme: ThemeColors) -> Rectangle {
    icon_button("\u{e1bc}", theme.button_surface, theme.accent).on_click(|_| {
        match WallpaperService::next() {
            Some(path) => {
                println!("Wallpaper: {}", path.display());
            }
            None => {
                eprintln!("Nenhum wallpaper disponível.");
            }
        }
    })
}

// --------------------------------------------------
// LIGHT / DARK
// --------------------------------------------------

fn theme_button(theme: ThemeColors) -> Rectangle {
    let icon = match theme.mode {
        ThemeMode::Dark => "\u{e518}",
        ThemeMode::Light => "\u{e51c}",
    };

    icon_button(icon, theme.button_surface, theme.accent).on_click(|_| {
        Theme::toggle();
    })
}

// --------------------------------------------------
// LAUNCHER
// --------------------------------------------------

fn launcher_button(theme: ThemeColors) -> Rectangle {
    let open = LauncherState::is_open();
    let background = if open {
        theme.active_surface
    } else {
        theme.button_surface
    };
    let foreground = if open { theme.active_text } else { theme.text };

    icon_button("\u{e5c3}", background, foreground).on_click(|_| LauncherState::open())
}

// --------------------------------------------------
// SESSION
// --------------------------------------------------

fn session_button(theme: ThemeColors) -> Rectangle {
    icon_button("\u{f8c7}", theme.button_surface, theme.text).on_click(|_| {
        println!("caelestia-rs: session");
    })
}

// --------------------------------------------------
// DIVIDER
// --------------------------------------------------

fn divider(theme: ThemeColors) -> Rectangle {
    Rectangle::new().width(22.0).height(1.0).fill(theme.divider)
}
