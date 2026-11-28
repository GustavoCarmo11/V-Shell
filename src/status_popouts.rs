use std::process::Command;
use std::sync::{Mutex, OnceLock};

use amane::{
    Audio, Bluetooth, Center, Color, Column, Network, Parent, Rectangle, Service, Stack, Text,
    TextInput, Widget,
};

use crate::material::{shape as material_shape, space as material_space};
use crate::motion::{Motion, PanelId};
use crate::rail_battery::RailBattery;
use crate::theme::ThemeColors;

static WIFI_SELECTION: OnceLock<Mutex<String>> = OnceLock::new();

pub fn status_popouts(theme: ThemeColors, width: f32, height: f32) -> Rectangle {
    let mut layers: Vec<Box<dyn Widget>> = Vec::new();
    for panel in [
        PanelId::QuickSettings,
        PanelId::Wifi,
        PanelId::Bluetooth,
        PanelId::Audio,
    ] {
        let content = match panel {
            PanelId::QuickSettings => battery_content(theme),
            PanelId::Wifi => wifi_content(theme),
            PanelId::Bluetooth => bluetooth_content(theme),
            PanelId::Audio => audio_content(theme),
            _ => continue,
        };
        let panel_height = match panel {
            PanelId::QuickSettings => 300.0,
            PanelId::Wifi | PanelId::Bluetooth => 480.0,
            PanelId::Audio => 350.0,
            _ => 300.0,
        };
        let motion = Motion::sample(panel, (270.0, 0.0));
        if motion.progress <= 0.001 {
            continue;
        }
        let y = (height - panel_height - 32.0).max(16.0);
        let card = Rectangle::new()
            .width(300.0)
            .height(panel_height)
            .radius(material_shape::PANEL)
            .fill(theme.bar_surface)
            .align_child(Center, Center)
            .child(content)
            .transform_matrix(motion.deformation)
            .opacity(motion.progress.clamp(0.0, 1.0))
            .translate(-270.0 * (1.0 - motion.progress), 0.0);
        let viewport = Rectangle::new()
            .width(300.0)
            .height(panel_height)
            .clip()
            .child(card)
            .translate(52.0, y);
        layers.push(Box::new(Motion::panel_hover(panel, viewport)));
    }
    Rectangle::new()
        .width(width)
        .height(height)
        .child(Stack::new(layers).width(Parent).height(Parent))
}

fn battery_content(theme: ThemeColors) -> Rectangle {
    let battery = RailBattery::snapshot();
    let mut children: Vec<Box<dyn Widget>> = vec![Box::new(label("BATERIA", theme.accent, 12.0))];
    children.push(Box::new(label(
        if battery.present {
            format!(
                "{}%{}",
                battery.percent,
                if battery.charging {
                    " · carregando"
                } else {
                    ""
                }
            )
        } else {
            "Bateria indisponível".to_owned()
        },
        theme.text,
        20.0,
    )));
    children.push(Box::new(label("PERFIL DE ENERGIA", theme.accent, 12.0)));
    for (profile, title) in [
        ("power-saver", "Economia"),
        ("balanced", "Balanceado"),
        ("performance", "Desempenho"),
    ] {
        children.push(Box::new(action(title, theme, move || {
            let profile = profile.to_owned();
            std::thread::spawn(move || {
                let _ = Command::new("powerprofilesctl")
                    .arg("set")
                    .arg(profile)
                    .spawn();
            });
        })));
    }
    card_content(children, theme)
}

fn wifi_content(theme: ThemeColors) -> Rectangle {
    let network = Network::read();
    let mut children: Vec<Box<dyn Widget>> = vec![Box::new(label("WI-FI", theme.accent, 12.0))];
    let wifi_enabled = network.wifi_enabled();
    children.push(Box::new(action(
        if wifi_enabled {
            "Desligar Wi-Fi"
        } else {
            "Ligar Wi-Fi"
        },
        theme,
        move || Network::set_wifi(!wifi_enabled),
    )));
    children.push(Box::new(label(
        if network.connected() {
            format!("Conectado a {}", network.ssid())
        } else if network.wifi_enabled() {
            "Desconectado".into()
        } else {
            "Wi-Fi desligado".into()
        },
        theme.text,
        16.0,
    )));
    children.push(Box::new(action("Buscar redes", theme, Network::scan)));
    for point in network.access_points().iter().take(6) {
        let ssid = point.ssid().to_owned();
        let saved = point.saved();
        let secured = point.secured();
        children.push(Box::new(action(
            &format!("{} · {}%", ssid, point.strength()),
            theme,
            move || {
                if saved || !secured {
                    Network::connect(&ssid, None);
                } else {
                    *WIFI_SELECTION
                        .get_or_init(|| Mutex::new(String::new()))
                        .lock()
                        .unwrap_or_else(|e| e.into_inner()) = ssid.clone();
                    TextInput::set_text("rail-wifi-password", "");
                }
            },
        )));
    }
    children.push(Box::new(
        TextInput::new("rail-wifi-password")
            .width(Parent)
            .size(14.0)
            .color(theme.text)
            .placeholder("Selecione rede protegida e digite a senha")
            .password()
            .focused()
            .on_submit(|password| {
                let ssid = WIFI_SELECTION
                    .get_or_init(|| Mutex::new(String::new()))
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                if !ssid.is_empty() {
                    Network::connect(&ssid, Some(&password));
                }
            }),
    ));
    card_content(children, theme)
}

fn bluetooth_content(theme: ThemeColors) -> Rectangle {
    let bluetooth = Bluetooth::read();
    let mut children: Vec<Box<dyn Widget>> = vec![Box::new(label("BLUETOOTH", theme.accent, 12.0))];
    let powered = bluetooth.powered();
    children.push(Box::new(action(
        if powered {
            "Desligar Bluetooth"
        } else {
            "Ligar Bluetooth"
        },
        theme,
        move || Bluetooth::set_powered(!powered),
    )));
    if bluetooth.powered() {
        children.push(Box::new(action(
            if bluetooth.scanning() {
                "Procurando dispositivos…"
            } else {
                "Procurar dispositivos"
            },
            theme,
            Bluetooth::start_scan,
        )));
    }
    for device in bluetooth.devices().iter().take(7) {
        let path = device.path().to_owned();
        let connected = device.connected();
        let paired = device.paired();
        let title = format!(
            "{} · {}",
            device.name(),
            if connected {
                "Desconectar"
            } else if device.paired() {
                "Conectar"
            } else {
                "Emparelhar"
            }
        );
        children.push(Box::new(action(&title, theme, move || {
            if connected {
                Bluetooth::disconnect(&path);
            } else if paired {
                Bluetooth::connect(&path);
            } else {
                Bluetooth::pair(&path);
            }
        })));
    }
    card_content(children, theme)
}

fn audio_content(theme: ThemeColors) -> Rectangle {
    let audio = Audio::read();
    let mut children: Vec<Box<dyn Widget>> = vec![
        Box::new(label("ÁUDIO", theme.accent, 12.0)),
        Box::new(label("SAÍDA", theme.accent, 11.0)),
    ];
    children.push(Box::new(label(
        &format!(
            "Volume de saída · {}%{}",
            audio.volume(),
            if audio.muted() { " · mudo" } else { "" }
        ),
        theme.text,
        15.0,
    )));
    children.push(Box::new(adjust_row(audio.volume(), false, theme)));
    children.push(Box::new(action(
        if audio.muted() {
            "Ativar saída"
        } else {
            "Silenciar saída"
        },
        theme,
        Audio::toggle_mute,
    )));
    children.push(Box::new(label("ENTRADA", theme.accent, 11.0)));
    children.push(Box::new(label(
        &format!(
            "Microfone · {}%{}",
            audio.microphone_volume(),
            if audio.microphone_muted() {
                " · mudo"
            } else {
                ""
            }
        ),
        theme.text,
        15.0,
    )));
    children.push(Box::new(adjust_row(audio.microphone_volume(), true, theme)));
    children.push(Box::new(action(
        if audio.microphone_muted() {
            "Ativar microfone"
        } else {
            "Silenciar microfone"
        },
        theme,
        Audio::toggle_microphone_mute,
    )));
    card_content(children, theme)
}

fn adjust_row(level: u8, input: bool, theme: ThemeColors) -> amane::Row {
    let set = move |value: u8| {
        if input {
            Audio::set_microphone_volume(value)
        } else {
            Audio::set_volume(value)
        }
    };
    let lower = level.saturating_sub(5);
    let higher = level.saturating_add(5).min(100);
    row(
        vec![
            Box::new(action("−", theme, move || set(lower))),
            Box::new(action("+", theme, move || set(higher))),
        ],
        theme,
    )
}

fn card_content(children: Vec<Box<dyn Widget>>, theme: ThemeColors) -> Rectangle {
    Rectangle::new()
        .width(Parent)
        .height(Parent)
        .padding(material_space::LG)
        .child(
            Column::new(children)
                .width(Parent)
                .height(Parent)
                .gap(material_space::SM)
                .align(Center)
                .justify(amane::Start),
        )
        .fill(theme.bar_surface)
}

fn action(title: &str, theme: ThemeColors, on_click: impl Fn() + 'static) -> Rectangle {
    Rectangle::new()
        .width(Parent)
        .height(36.0)
        .radius(material_shape::CONTROL)
        .fill(theme.workspace_group_surface)
        .cursor(amane::Pointer)
        .padding(material_space::SM)
        .child(label(title, theme.text, 14.0))
        .on_click(move |_| on_click())
}

fn label(title: impl Into<String>, color: Color, size: f32) -> Text {
    Text::new(title).size(size).color(color)
}

fn row(children: Vec<Box<dyn Widget>>, _theme: ThemeColors) -> amane::Row {
    amane::Row::new(children)
        .width(Parent)
        .height(36.0)
        .gap(material_space::SM)
}
