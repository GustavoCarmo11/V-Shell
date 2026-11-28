use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use amane::{
    Audio, Bluetooth, Center, Color, Column, Network, Parent, Rectangle, Service, Stack, Text,
    TextInput, Widget, request_frame,
};

use crate::material::{shape as material_shape, space as material_space};
use crate::motion::{Motion, PanelId, TRIGGER_HOVER_LEAVE_GRACE};
use crate::rail_battery::RailBattery;
use crate::theme::ThemeColors;

const POPOUT_WIDTH: f32 = 300.0;
const CONTENT_FADE_DURATION: Duration = Duration::from_millis(250);
const SIZE_TRANSITION_DURATION: Duration = Duration::from_millis(240);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatusSection {
    Battery,
    Wifi,
    Bluetooth,
    Audio,
}

#[derive(Clone, Copy)]
struct SelectionState {
    current: StatusSection,
    previous: StatusSection,
    changed_at: Instant,
    current_height: f32,
    from_height: f32,
    target_height: f32,
    size_changed_at: Instant,
}

impl Default for SelectionState {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            current: StatusSection::Battery,
            previous: StatusSection::Battery,
            changed_at: now,
            current_height: 0.0,
            from_height: 0.0,
            target_height: 0.0,
            size_changed_at: now,
        }
    }
}

static SELECTION: OnceLock<Mutex<SelectionState>> = OnceLock::new();
static WIFI_SELECTION: OnceLock<Mutex<String>> = OnceLock::new();
static STATUS_TRIGGER_HOVERED: AtomicBool = AtomicBool::new(false);

fn selection() -> &'static Mutex<SelectionState> {
    SELECTION.get_or_init(|| Mutex::new(SelectionState::default()))
}

pub fn active_section() -> StatusSection {
    selection()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .current
}

pub fn status_trigger(section: StatusSection, widget: Rectangle) -> Rectangle {
    widget.on_hover(move |inside| {
        if inside {
            STATUS_TRIGGER_HOVERED.store(true, Ordering::Relaxed);
            let mut state = selection()
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if state.current != section {
                state.previous = state.current;
                state.current = section;
                state.changed_at = Instant::now();
                state.from_height = state.current_height;
                state.size_changed_at = state.changed_at;
                request_frame();
            }
            drop(state);
            Motion::open(PanelId::QuickSettings);
        } else {
            STATUS_TRIGGER_HOVERED.store(false, Ordering::Relaxed);
            Motion::schedule_close(PanelId::QuickSettings, TRIGGER_HOVER_LEAVE_GRACE);
        }
    })
}

pub struct StatusPopout {
    pub widget: Rectangle,
    pub progress: f32,
    /// Extra shader uniforms: bounds, shape parameters and deformation matrix.
    pub shader_values: [[f32; 4]; 3],
}

pub fn status_popout(
    theme: ThemeColors,
    width: f32,
    height: f32,
    bottom_frame: f32,
) -> StatusPopout {
    let network = Network::read();
    let bluetooth = Bluetooth::read();
    let battery = RailBattery::snapshot();
    let audio = Audio::read();

    let (current, previous, fade, panel_height) = {
        let mut state = selection()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let current = state.current;
        let previous = state.previous;

        let desired_height = section_height(current, &network, &bluetooth);
        if (desired_height - state.target_height).abs() > 1.0 {
            state.from_height = state.current_height;
            state.target_height = desired_height;
            state.size_changed_at = Instant::now();
        }

        let size_progress = (state.size_changed_at.elapsed().as_secs_f32()
            / SIZE_TRANSITION_DURATION.as_secs_f32())
        .clamp(0.0, 1.0);
        let size_eased = smoothstep(size_progress);
        state.current_height =
            state.from_height + (state.target_height - state.from_height) * size_eased;

        let fade = smoothstep(
            (state.changed_at.elapsed().as_secs_f32() / CONTENT_FADE_DURATION.as_secs_f32())
                .clamp(0.0, 1.0),
        );
        if fade < 1.0 || size_progress < 1.0 {
            request_frame();
        } else {
            state.previous = state.current;
        }

        (current, previous, fade, state.current_height)
    };

    let motion = Motion::sample(PanelId::QuickSettings, (182.0, 0.0));
    let mut content_layers: Vec<Box<dyn Widget>> = Vec::new();
    if previous != current && fade < 1.0 {
        let outgoing = section_content(
            previous, false, theme, &network, &bluetooth, &battery, &audio,
        );
        content_layers.push(Box::new(
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .opacity(1.0 - fade)
                .child(outgoing),
        ));
    }
    let incoming = section_content(current, true, theme, &network, &bluetooth, &battery, &audio);
    content_layers.push(Box::new(
        Rectangle::new()
            .width(Parent)
            .height(Parent)
            .opacity(fade)
            .child(incoming),
    ));

    let content = Stack::new(content_layers).width(Parent).height(Parent);
    let content_opacity = smoothstep((motion.progress - 0.45) / 0.55);
    let panel = Rectangle::new()
        .width(POPOUT_WIDTH)
        .height(panel_height.max(1.0))
        .fill(Color::TRANSPARENT)
        .align_child(Center, Center)
        .child(content)
        .opacity(content_opacity);

    let progress = motion.progress.clamp(0.0, 1.0);
    let visible_width = 40.0 + (POPOUT_WIDTH - 40.0) * progress;
    let visible_height = panel_height.max(1.0);
    let surface_x = 52.0 - 52.0 * (1.0 - progress);
    let surface_y = (height - visible_height - bottom_frame).max(16.0);
    let shader_values = [
        [surface_x, surface_y, visible_width, visible_height],
        [material_shape::PANEL, progress, 24.0, 0.0],
        [
            motion.deformation.sx,
            motion.deformation.kx,
            motion.deformation.ky,
            motion.deformation.sy,
        ],
    ];

    let panel_viewport = Rectangle::new()
        .width(visible_width)
        .height(visible_height)
        .clip()
        .child(panel)
        .transform_matrix(motion.deformation)
        .translate(surface_x, surface_y);
    let panel_viewport = if motion.progress > 0.001 {
        Motion::panel_hover_with_exit(PanelId::QuickSettings, panel_viewport, || {
            // The expanding panel can move its hit bounds away from a pointer
            // that is still on the rail icon. Keep it open until both regions
            // have actually been left.
            if STATUS_TRIGGER_HOVERED.load(Ordering::Relaxed) {
                Motion::open(PanelId::QuickSettings);
            }
        })
    } else {
        panel_viewport
    };
    let viewport = Rectangle::new()
        .width(width)
        .height(height)
        .child(panel_viewport);

    StatusPopout {
        widget: viewport,
        progress: motion.progress,
        shader_values,
    }
}

fn smoothstep(value: f32) -> f32 {
    let t = value.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn section_height(section: StatusSection, network: &Network, bluetooth: &Bluetooth) -> f32 {
    match section {
        StatusSection::Battery => 280.0,
        StatusSection::Wifi => {
            let networks = network.access_points().len().min(6) as f32;
            (244.0 + networks * 40.0).clamp(260.0, 500.0)
        }
        StatusSection::Bluetooth => {
            let devices = bluetooth.devices().len().min(7) as f32;
            let controls = if bluetooth.powered() { 2.0 } else { 1.0 };
            (162.0 + (devices + controls) * 42.0).clamp(230.0, 500.0)
        }
        StatusSection::Audio => 370.0,
    }
}

fn section_content(
    section: StatusSection,
    active: bool,
    theme: ThemeColors,
    network: &Network,
    bluetooth: &Bluetooth,
    battery: &RailBattery,
    audio: &Audio,
) -> Rectangle {
    match section {
        StatusSection::Battery => battery_content(theme, battery),
        StatusSection::Wifi => wifi_content(theme, network, active),
        StatusSection::Bluetooth => bluetooth_content(theme, bluetooth),
        StatusSection::Audio => audio_content(theme, audio),
    }
}

fn battery_content(theme: ThemeColors, battery: &RailBattery) -> Rectangle {
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

fn wifi_content(theme: ThemeColors, network: &Network, active: bool) -> Rectangle {
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
                        .unwrap_or_else(|error| error.into_inner()) = ssid.clone();
                    TextInput::set_text("rail-wifi-password", "");
                }
            },
        )));
    }
    if active {
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
                        .unwrap_or_else(|error| error.into_inner())
                        .clone();
                    if !ssid.is_empty() {
                        Network::connect(&ssid, Some(&password));
                    }
                }),
        ));
    }
    card_content(children, theme)
}

fn bluetooth_content(theme: ThemeColors, bluetooth: &Bluetooth) -> Rectangle {
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
    if powered {
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
            } else if paired {
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

fn audio_content(theme: ThemeColors, audio: &Audio) -> Rectangle {
    let mut children: Vec<Box<dyn Widget>> = vec![
        Box::new(label("ÁUDIO", theme.accent, 12.0)),
        Box::new(label("SAÍDA", theme.accent, 11.0)),
    ];
    children.push(Box::new(label(
        format!(
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
        format!(
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
    amane::Row::new(vec![
        Box::new(action("−", theme, move || set(lower))),
        Box::new(action("+", theme, move || set(higher))),
    ])
    .width(Parent)
    .height(36.0)
    .gap(material_space::SM)
}

fn card_content(children: Vec<Box<dyn Widget>>, _theme: ThemeColors) -> Rectangle {
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
}

fn action(title: &str, theme: ThemeColors, on_click: impl Fn() + 'static) -> Rectangle {
    Rectangle::new()
        .width(Parent)
        .height(36.0)
        .radius(material_shape::CONTROL)
        .fill(theme.workspace_group_surface)
        .cursor(amane::Pointer)
        .padding(material_space::SM)
        .child(label(title.to_owned(), theme.text, 14.0))
        .on_click(move |_| on_click())
}

fn label(title: impl Into<String>, color: Color, size: f32) -> Text {
    Text::new(title).size(size).color(color)
}
