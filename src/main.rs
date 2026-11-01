mod theme;
mod wallpaper;

use amane::{
    App, Center, Color, Column, Full, Horizontal, Image, Layer, LayerWindow, Margin, Monitor,
    Parent, Pointer, Rectangle, Service, Stack, Text, Vertical, Widget, Workspace, Workspaces,
    Zone,
};

use theme::{Theme, ThemeColors, ThemeMode};
use wallpaper::WallpaperService;

// --------------------------------------------------
// MAIN
// --------------------------------------------------

fn main() {
    App::new()
        .font("Rubik")
        .window_per_monitor(background)
        .window_per_monitor(drawers)
        .window_per_monitor(bar)
        .run();
}

// --------------------------------------------------
// BACKGROUND
// --------------------------------------------------

fn background(_monitor: &Monitor) -> LayerWindow {
    let (previous, current, progress) = WallpaperService::transition_snapshot();

    let mut layers: Vec<Box<dyn Widget>> = Vec::new();

    // Wallpaper anterior.
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

    // Wallpaper atual.
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

    // Superfície vazia caso não exista wallpaper.
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
// DRAWERS
// --------------------------------------------------

fn drawers(_monitor: &Monitor) -> LayerWindow {
    LayerWindow::new()
        .width(Full)
        .height(Full)
        .layer(Layer::Top)
        .space(Zone::Ignore)
        .namespace("caelestia-rs-drawers")
        .click_through()
        .child(Rectangle::new().width(Parent).height(Parent))
}

// --------------------------------------------------
// BAR
// --------------------------------------------------

fn bar(_monitor: &Monitor) -> LayerWindow {
    // Snapshot das cores animadas.
    let theme = Theme::snapshot();

    let mut buttons: Vec<Box<dyn Widget>> = Vec::new();

    // Home.
    buttons.push(Box::new(home_button(theme)));

    buttons.push(Box::new(divider(theme)));

    // Workspaces do monitor atual.

    // Workspaces do Hyprland.
    let workspaces = Workspaces::read();
    let workspace_list = workspaces.list();

    println!(
        "[caelestia-rs] Workspaces recebidos: {}",
        workspace_list.len()
    );

    let mut workspace_count = 0usize;

    // Temporariamente exibimos todos os workspaces,
    // sem filtrar pelo monitor.
    for workspace in workspace_list.iter() {
        println!(
            "[caelestia-rs] Workspace {} | output={:?} | active={}",
            workspace.index(),
            workspace.output(),
            workspace.active()
        );

        workspace_count += 1;

        buttons.push(Box::new(workspace_button(workspace, theme)));
    }

    println!(
        "[caelestia-rs] Workspaces adicionados à barra: {}",
        workspace_count
    );

    buttons.push(Box::new(divider(theme)));

    // Wallpaper.
    buttons.push(Box::new(wallpaper_button(theme)));

    // Light / Dark.
    buttons.push(Box::new(theme_button(theme)));

    buttons.push(Box::new(divider(theme)));

    // Launcher.
    buttons.push(Box::new(launcher_button(theme)));

    // Sessão.
    buttons.push(Box::new(session_button(theme)));

    // A altura acompanha a quantidade de workspaces.
    let bar_height = (278.0 + workspace_count as f32 * 49.0).clamp(360.0, 760.0);

    LayerWindow::new()
        .width(68.0)
        .height(bar_height)
        .anchor_horizontal(Horizontal::Left)
        .anchor_vertical(Vertical::Middle)
        .margin(Margin {
            top: 0,
            right: 0,
            bottom: 0,
            left: 12,
        })
        .layer(Layer::Top)
        .space(Zone::Ignore)
        .namespace("caelestia-rs-bar")
        .child(
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .radius(28.0)
                .fill(theme.bar_surface)
                .padding(8.0)
                .align_child(Center, Center)
                .child(
                    Column::new(buttons)
                        .width(Parent)
                        .height(Parent)
                        .gap(7.0)
                        .align(Center)
                        .justify(Center),
                ),
        )
}

// --------------------------------------------------
// ICON BUTTON
// --------------------------------------------------

fn icon_button(icon: &'static str, background: Color, foreground: Color) -> Rectangle {
    Rectangle::new()
        .width(42.0)
        .height(42.0)
        .radius(14.0)
        .fill(background)
        .cursor(Pointer)
        .align_child(Center, Center)
        .child(
            Text::new(icon)
                .font("Material Symbols Rounded")
                .size(24.0)
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
        .width(42.0)
        .height(42.0)
        .radius(14.0)
        .fill(background)
        .cursor(Pointer)
        .on_click(move |_| {
            Workspaces::focus(id);
        })
        .align_child(Center, Center)
        .child(Text::new(label).size(15.0).color(foreground))
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
    icon_button("\u{e5c3}", theme.button_surface, theme.text).on_click(|_| {
        println!("caelestia-rs: launcher");
    })
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
    Rectangle::new().width(26.0).height(1.0).fill(theme.divider)
}
