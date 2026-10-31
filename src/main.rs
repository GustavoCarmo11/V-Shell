mod theme;
mod wallpaper;

use wallpaper::WallpaperService;

use amane::{
    App, Center, Column, Full, Horizontal, Image, Layer, LayerWindow, Margin, Monitor, Parent,
    Pointer, Rectangle, Service, Stack, Text, Vertical, Widget, Workspace, Workspaces, Zone,
};

use theme::{Theme, ThemeColors, ThemeMode};

// --------------------------------------------------
// MAIN
// --------------------------------------------------

fn main() {
    App::new()
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

    // Imagem anterior: desaparece gradualmente.
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

    // Imagem nova: aparece gradualmente.
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

    // Fallback caso ainda não exista wallpaper.
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

fn bar(monitor: &Monitor) -> LayerWindow {
    // Copiamos a paleta para não manter o lock durante
    // a construção dos componentes.
    let theme = Theme::snapshot();

    let mut buttons: Vec<Box<dyn Widget>> = Vec::new();

    buttons.push(Box::new(divider(theme)));

    // Troca de wallpaper.
    buttons.push(Box::new(wallpaper_button(theme)));

    // Alternância de tema.
    buttons.push(Box::new(theme_button(theme)));

    // Workspaces reais do Hyprland.
    let workspaces = Workspaces::read();

    for workspace in workspaces.list() {
        if workspace.output() == Some(monitor.name.as_str()) {
            buttons.push(Box::new(workspace_button(workspace, theme)));
        }
    }

    buttons.push(Box::new(divider(theme)));

    // Alternância de tema.
    buttons.push(Box::new(theme_button(theme)));

    buttons.push(Box::new(bar_button("A", false, theme)));
    buttons.push(Box::new(bar_button("P", false, theme)));

    LayerWindow::new()
        .width(68.0)
        .height(390.0)
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
                .padding(10.0)
                .align_child(Center, Center)
                .child(
                    Column::new(buttons)
                        .width(Parent)
                        .height(Parent)
                        .gap(8.0)
                        .align(Center)
                        .justify(Center),
                ),
        )
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

    let foreground = if active { theme.accent } else { theme.text };

    let id = workspace.id();

    let label = workspace
        .name()
        .map(str::to_owned)
        .unwrap_or_else(|| workspace.index().to_string());

    Rectangle::new()
        .width(42.0)
        .height(38.0)
        .radius(13.0)
        .fill(background)
        .cursor(Pointer)
        .on_click(move |_| {
            Workspaces::focus(id);
        })
        .align_child(Center, Center)
        .child(Text::new(&label).size(14.0).color(foreground))
}

// --------------------------------------------------
// THEME BUTTON
// --------------------------------------------------

fn theme_button(theme: ThemeColors) -> Rectangle {
    let label = match theme.mode {
        ThemeMode::Dark => "L",
        ThemeMode::Light => "D",
    };

    Rectangle::new()
        .width(42.0)
        .height(38.0)
        .radius(13.0)
        .fill(theme.button_surface)
        .cursor(Pointer)
        .on_click(|_| {
            Theme::toggle();
        })
        .align_child(Center, Center)
        .child(Text::new(label).size(14.0).color(theme.accent))
}

fn wallpaper_button(theme: ThemeColors) -> Rectangle {
    Rectangle::new()
        .width(42.0)
        .height(38.0)
        .radius(13.0)
        .fill(theme.button_surface)
        .cursor(Pointer)
        .on_click(|_| match WallpaperService::next() {
            Some(path) => {
                println!("Wallpaper selecionado: {}", path.display());
            }

            None => {
                eprintln!("Nenhum wallpaper disponível.");
            }
        })
        .align_child(Center, Center)
        .child(Text::new("W").size(14.0).color(theme.accent))
}
// --------------------------------------------------
// COMPONENTS
// --------------------------------------------------

fn bar_button(label: &'static str, active: bool, theme: ThemeColors) -> Rectangle {
    let background = if active {
        theme.active_surface
    } else {
        theme.button_surface
    };

    let foreground = if active { theme.accent } else { theme.text };

    Rectangle::new()
        .width(42.0)
        .height(38.0)
        .radius(13.0)
        .fill(background)
        .cursor(Pointer)
        .on_click(move |_| {
            println!("caelestia-rs: clicked {label}");
        })
        .align_child(Center, Center)
        .child(Text::new(label).size(14.0).color(foreground))
}

fn divider(theme: ThemeColors) -> Rectangle {
    Rectangle::new().width(26.0).height(1.0).fill(theme.divider)
}
