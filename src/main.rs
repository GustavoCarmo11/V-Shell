
use amane::{
    App, Center, Color, Column, Full, Horizontal, Layer,
    LayerWindow, Margin, Monitor, Parent, Pointer,
    Rectangle, Service, Text, Vertical, Widget,
    Workspaces, Workspace, Zone,
};

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
    LayerWindow::new()
        .width(Full)
        .height(Full)
        .layer(Layer::Background)
        .space(Zone::Ignore)
        .namespace("caelestia-rs-background")
        .click_through()
        .child(
            Rectangle::new()
                .width(Parent)
                .height(Parent),
        )
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
        .child(
            Rectangle::new()
                .width(Parent)
                .height(Parent),
        )
}

// --------------------------------------------------
// BAR
// --------------------------------------------------


fn bar(monitor: &Monitor) -> LayerWindow {
    let mut buttons: Vec<Box<dyn Widget>> = Vec::new();

    buttons.push(Box::new(bar_button("C", true)));
    buttons.push(Box::new(divider()));

    // Lê os workspaces monitorados pelo Amane.
    let workspaces = Workspaces::read();

    for workspace in workspaces.list() {
        // Cada monitor mostra seus próprios workspaces.
        if workspace.output() == Some(monitor.name.as_str()) {
            buttons.push(Box::new(workspace_button(workspace)));
        }
    }

    buttons.push(Box::new(divider()));
    buttons.push(Box::new(bar_button("A", false)));
    buttons.push(Box::new(bar_button("P", false)));

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
                .fill("#181825ee")
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



fn workspace_button(workspace: &Workspace) -> Rectangle {
    let active = workspace.active();

    let background = if active {
        "#45475a"
    } else {
        "#313244"
    };

    let foreground = if active {
        "#89b4fa"
    } else {
        "#cdd6f4"
    };

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
        .child(
            Text::new(&label)
                .size(14.0)
                .color(Color::from(foreground)),
        )
}
// --------------------------------------------------
// COMPONENTS
// --------------------------------------------------

fn bar_button(label: &'static str, active: bool) -> Rectangle {
    let background = if active {
        "#45475a"
    } else {
        "#313244"
    };

    let foreground = if active {
        "#89b4fa"
    } else {
        "#cdd6f4"
    };

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
        .child(
            Text::new(label)
                .size(14.0)
                .color(Color::from(foreground)),
        )
}

fn divider() -> Rectangle {
    Rectangle::new()
        .width(26.0)
        .height(1.0)
        .fill("#585b70")
}
