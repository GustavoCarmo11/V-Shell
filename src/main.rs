mod drawer;
mod launcher;
mod motion;
mod theme;
mod wallpaper;

use amane::{
    App, Center, Color, Column, End, Full, Horizontal, Image, InputArea, Keyboard, Layer,
    LayerWindow, Monitor, Parent, Pointer, Rectangle, Service, Stack, Start, Text, Vertical,
    Widget, Workspace, Workspaces, Zone,
};

use drawer::{DASHBOARD_HEIGHT, DASHBOARD_WIDTH, DrawerState};
use launcher::{
    LauncherState, PANEL_BOTTOM_GAP, PANEL_HEIGHT as LAUNCHER_HEIGHT, PANEL_WIDTH as LAUNCHER_WIDTH,
};
use motion::{Motion, PanelId};
use theme::{Theme, ThemeColors, ThemeMode};
use wallpaper::WallpaperService;

const RAIL_WIDTH: f32 = 60.0;
const BORDER_SIZE: f32 = 10.0;
const BORDER_RADIUS: f32 = 24.0;
const INNER_LEFT_RADIUS: f32 = 30.0;
const INNER_RIGHT_RADIUS: f32 = 24.0;
const DASHBOARD_TOP: f32 = BORDER_SIZE;
const DASHBOARD_MERGE_RADIUS: f32 = 19.0;
// --------------------------------------------------
// MAIN
// --------------------------------------------------

fn main() {
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
    // The panel is bottom-aligned. When open, its lower edge meets the inner
    // edge of the bottom frame; when closed, it is fully below the monitor.
    let launcher_translate_y = LAUNCHER_HEIGHT - launcher_travel * launcher_progress;
    let launcher_panel_left = ((monitor.width as f32 - LAUNCHER_WIDTH) / 2.0).max(0.0);
    let launcher_panel_top = monitor.height as f32 - launcher_travel * launcher_progress;

    // Pointer input covers the rail, visible panels and the bottom corridor
    // connecting the rail trigger to the Launcher.
    let mut input_areas = vec![InputArea {
        x: 0,
        y: 0,
        width: RAIL_WIDTH as i32,
        height: monitor.height as i32,
    }];

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

    if LauncherState::is_open() || launcher_progress > 0.001 {
        // A bottom corridor connects the rail trigger to the bottom-center panel.
        // It keeps hover continuous while the pointer travels to the Launcher.
        let corridor_top =
            (monitor.height as f32 - LAUNCHER_HEIGHT - PANEL_BOTTOM_GAP - 180.0).max(0.0);
        input_areas.push(InputArea {
            x: 0,
            y: corridor_top.floor() as i32,
            width: monitor.width as i32,
            height: (monitor.height as f32 - corridor_top).ceil() as i32,
        });

        // The transformed panel can extend beyond its layout rectangle.
        if launcher_progress > 0.001 {
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
        .child(rail(theme, monitor.height as f32));

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
            LauncherState::panel(theme)
                .transform_matrix(launcher_motion.deformation)
                .translate(0.0, launcher_translate_y),
        ));

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
                .on_hover(|inside| {
                    if !inside {
                        LauncherState::close();
                    }
                })
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
                    ])
                    .width(Parent)
                    .height(Parent),
                ),
        )
}

// --------------------------------------------------
// RAIL CONTENT
// --------------------------------------------------

fn rail(theme: ThemeColors, height: f32) -> Rectangle {
    let mut buttons: Vec<Box<dyn Widget>> = Vec::new();

    buttons.push(Box::new(dashboard_button(theme)));
    buttons.push(Box::new(divider(theme)));

    buttons.push(Box::new(home_button(theme)));
    buttons.push(Box::new(divider(theme)));

    let workspaces = Workspaces::read();
    let workspace_list = workspaces.list();

    for workspace in workspace_list.iter() {
        buttons.push(Box::new(workspace_button(workspace, theme)));
    }

    buttons.push(Box::new(divider(theme)));

    buttons.push(Box::new(wallpaper_button(theme)));
    buttons.push(Box::new(theme_button(theme)));

    buttons.push(Box::new(divider(theme)));

    buttons.push(Box::new(launcher_button(theme)));
    buttons.push(Box::new(session_button(theme)));

    // No background or border here: the fullscreen SDF draws the shared surface.
    Rectangle::new()
        .width(RAIL_WIDTH)
        .height(height)
        .padding(6.0)
        .align_child(Center, Center)
        .child(
            Column::new(buttons)
                .width(Parent)
                .height(Parent)
                .gap(6.0)
                .align(Center)
                .justify(Center),
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

    Motion::trigger_hover(
        PanelId::Dashboard,
        icon_button("\u{e871}", background, foreground),
    )
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

    let background = if active {
        theme.active_surface
    } else {
        theme.button_surface
    };

    let foreground = if active {
        theme.active_text
    } else {
        theme.text
    };

    let id = workspace.id();

    let label = workspace
        .name()
        .map(str::to_owned)
        .unwrap_or_else(|| workspace.index().to_string());

    Rectangle::new()
        .width(38.0)
        .height(38.0)
        .radius(13.0)
        .fill(background)
        .cursor(Pointer)
        .on_click(move |_| {
            Workspaces::focus(id);
        })
        .align_child(Center, Center)
        .child(Text::new(label).size(14.0).color(foreground))
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

    Motion::trigger_hover(
        PanelId::Launcher,
        icon_button("\u{e5c3}", background, foreground),
    )
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
