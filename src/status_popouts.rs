use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use amane::{
    Audio, Bluetooth, Center, Color, Column, Network, Parent, Rectangle, Service, Stack, Text,
    TextInput, Widget, request_frame,
};

use crate::material::{self, shape as material_shape, space as material_space};
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
static AUDIO_DEVICES: OnceLock<Mutex<AudioDevices>> = OnceLock::new();
static AUDIO_DEVICES_WORKER: AtomicBool = AtomicBool::new(false);
static POWER_PROFILE: OnceLock<Mutex<String>> = OnceLock::new();

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct AudioDevice {
    name: String,
    description: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct AudioDevices {
    outputs: Vec<AudioDevice>,
    inputs: Vec<AudioDevice>,
    default_output: String,
    default_input: String,
}

fn audio_devices() -> AudioDevices {
    if !AUDIO_DEVICES_WORKER.swap(true, Ordering::AcqRel) {
        thread::spawn(|| {
            loop {
                let popup_visible = Motion::progress(PanelId::QuickSettings) > 0.001;
                if active_section() != StatusSection::Audio
                    || (!STATUS_TRIGGER_HOVERED.load(Ordering::Relaxed) && !popup_visible)
                {
                    thread::sleep(Duration::from_millis(500));
                    continue;
                }
                let next = read_audio_devices();
                let devices = AUDIO_DEVICES.get_or_init(|| Mutex::new(AudioDevices::default()));
                let changed = {
                    let mut current = devices.lock().unwrap_or_else(|error| error.into_inner());
                    if *current == next {
                        false
                    } else {
                        *current = next;
                        true
                    }
                };
                if changed {
                    request_frame();
                }
                thread::sleep(Duration::from_secs(2));
            }
        });
    }

    AUDIO_DEVICES
        .get_or_init(|| Mutex::new(AudioDevices::default()))
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

fn read_audio_devices() -> AudioDevices {
    fn list(kind: &str) -> Vec<AudioDevice> {
        let Ok(output) = Command::new("pactl")
            .args(["-f", "json", "list", kind])
            .output()
        else {
            return Vec::new();
        };
        if !output.status.success() {
            return Vec::new();
        }
        serde_json::from_slice::<serde_json::Value>(&output.stdout)
            .ok()
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|device| {
                let name = device.get("name")?.as_str()?.to_owned();
                if kind == "sources" && name.ends_with(".monitor") {
                    return None;
                }
                let description = device
                    .get("description")
                    .and_then(serde_json::Value::as_str)
                    .filter(|value| !value.is_empty())
                    .unwrap_or(&name)
                    .to_owned();
                Some(AudioDevice { name, description })
            })
            .collect()
    }

    fn default(kind: &str) -> String {
        Command::new("pactl")
            .arg(format!("get-default-{kind}"))
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
            .unwrap_or_default()
    }

    AudioDevices {
        outputs: list("sinks"),
        inputs: list("sources"),
        default_output: default("sink"),
        default_input: default("source"),
    }
}

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
    let audio_devices = if active_section() == StatusSection::Audio
        || AUDIO_DEVICES_WORKER.load(Ordering::Acquire)
    {
        audio_devices()
    } else {
        AudioDevices::default()
    };

    let (current, previous, fade, panel_height) = {
        let mut state = selection()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let current = state.current;
        let previous = state.previous;

        let desired_height = section_height(current, &network, &bluetooth, &audio_devices);
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
            previous,
            false,
            theme,
            &network,
            &bluetooth,
            &battery,
            &audio,
            &audio_devices,
        );
        content_layers.push(Box::new(
            Rectangle::new()
                .width(Parent)
                .height(Parent)
                .opacity(1.0 - fade)
                .child(outgoing),
        ));
    }
    let incoming = section_content(
        current,
        true,
        theme,
        &network,
        &bluetooth,
        &battery,
        &audio,
        &audio_devices,
    );
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

fn section_height(
    section: StatusSection,
    network: &Network,
    bluetooth: &Bluetooth,
    audio: &AudioDevices,
) -> f32 {
    match section {
        StatusSection::Battery => 180.0,
        StatusSection::Wifi => {
            let networks = network.access_points().len().min(8) as f32;
            (280.0 + networks * 36.0).clamp(320.0, 600.0)
        }
        StatusSection::Bluetooth => {
            let devices = bluetooth.devices().len().min(7) as f32;
            (280.0 + devices * 36.0).clamp(300.0, 560.0)
        }
        StatusSection::Audio => {
            let devices = audio.outputs.len().min(4) + audio.inputs.len().min(4);
            (270.0 + devices as f32 * 30.0).clamp(300.0, 540.0)
        }
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
    audio_devices: &AudioDevices,
) -> Rectangle {
    match section {
        StatusSection::Battery => battery_content(theme, battery),
        StatusSection::Wifi => wifi_content(theme, network, active),
        StatusSection::Bluetooth => bluetooth_content(theme, bluetooth),
        StatusSection::Audio => audio_content(theme, audio, audio_devices),
    }
}

fn battery_content(theme: ThemeColors, battery: &RailBattery) -> Rectangle {
    if !battery.present {
        return card_content(
            vec![Box::new(label("Bateria indisponível", theme.text, 18.0))],
            theme,
        );
    }
    let battery_state = amane::Battery::read();
    let battery_detail = if battery.charging {
        "Carregando".to_owned()
    } else if battery_state.full() || battery.percent == 100 {
        "Carga completa!".to_owned()
    } else {
        "Em uso".to_owned()
    };
    let mut children: Vec<Box<dyn Widget>> = vec![Box::new(label(
        format!("Restante: {}%", battery.percent),
        theme.text,
        18.0,
    ))];
    children.push(Box::new(label(battery_detail, theme.text, 18.0)));
    children.push(Box::new(profile_picker(theme)));
    card_content(children, theme)
}

fn selected_power_profile() -> String {
    POWER_PROFILE
        .get_or_init(|| {
            let profile = Command::new("powerprofilesctl")
                .arg("get")
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
                .unwrap_or_default();
            Mutex::new(profile)
        })
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

fn profile_picker(theme: ThemeColors) -> Rectangle {
    let selected = selected_power_profile();
    let profiles = [
        ("power-saver", material::icon::ECO, "Economia"),
        ("balanced", material::icon::BALANCE, "Equilibrado"),
        ("performance", material::icon::ROCKET, "Desempenho"),
    ];
    let choices = profiles
        .into_iter()
        .map(|(profile, icon, _label)| {
            let active = selected == profile;
            let profile_id = profile.to_owned();
            let foreground = if active {
                theme.active_text
            } else {
                theme.muted
            };
            Box::new(
                Rectangle::new()
                    .width(Parent)
                    .height(56.0)
                    .radius(material_shape::FULL)
                    .fill(if active {
                        theme.active_surface
                    } else {
                        Color::TRANSPARENT
                    })
                    .cursor(amane::Pointer)
                    .align_child(Center, Center)
                    .child(crate::material::icon(icon, 27.0, foreground))
                    .on_click(move |_| {
                        let profile_id = profile_id.clone();
                        thread::spawn(move || {
                            let succeeded = Command::new("powerprofilesctl")
                                .arg("set")
                                .arg(&profile_id)
                                .status()
                                .is_ok_and(|status| status.success());
                            if succeeded {
                                *POWER_PROFILE
                                    .get_or_init(|| Mutex::new(String::new()))
                                    .lock()
                                    .unwrap_or_else(|error| error.into_inner()) = profile_id;
                                request_frame();
                            }
                        });
                    }),
            ) as Box<dyn Widget>
        })
        .collect();

    Rectangle::new()
        .width(Parent)
        .height(68.0)
        .radius(material_shape::FULL)
        .fill(theme.workspace_group_surface)
        .padding(material_space::XS)
        .child(
            amane::Row::new(choices)
                .width(Parent)
                .height(Parent)
                .align(Center)
                .justify(amane::Start),
        )
}

fn wifi_content(theme: ThemeColors, network: &Network, active: bool) -> Rectangle {
    let mut children: Vec<Box<dyn Widget>> = vec![Box::new(label("Wireless", theme.text, 19.0))];
    let wifi_enabled = network.wifi_enabled();
    children.push(Box::new(toggle_row(
        "Ativado",
        wifi_enabled,
        theme,
        move || Network::set_wifi(!wifi_enabled),
    )));
    children.push(Box::new(label(
        format!("{} rede(s) disponível(is)", network.access_points().len()),
        theme.muted,
        12.0,
    )));
    if network.access_points().is_empty() && wifi_enabled {
        children.push(Box::new(label(
            "Nenhuma rede encontrada. Atualize a busca.",
            theme.muted,
            13.0,
        )));
    }
    for point in network.access_points().iter().take(8) {
        let ssid = point.ssid().to_owned();
        let saved = point.saved();
        let secured = point.secured();
        let connected = point.active();
        let selected_ssid = ssid.clone();
        children.push(Box::new(wifi_network_row(
            &ssid,
            point.strength(),
            secured,
            connected,
            theme,
            move || {
                if saved || !secured {
                    Network::connect(&selected_ssid, None);
                } else {
                    *WIFI_SELECTION
                        .get_or_init(|| Mutex::new(String::new()))
                        .lock()
                        .unwrap_or_else(|error| error.into_inner()) = selected_ssid.clone();
                    TextInput::set_text("rail-wifi-password", "");
                }
            },
        )));
    }
    if active
        && WIFI_SELECTION.get().is_some_and(|selection| {
            !selection
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .is_empty()
        })
    {
        children.push(Box::new(
            Rectangle::new()
                .width(Parent)
                .height(44.0)
                .radius(material_shape::CONTROL)
                .fill(theme.button_surface)
                .padding(material_space::SM)
                .align_child(amane::Start, Center)
                .child(
                    TextInput::new("rail-wifi-password")
                        .width(Parent)
                        .size(14.0)
                        .color(theme.text)
                        .placeholder("Digite a senha e pressione Enter")
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
                ),
        ));
    }
    children.push(Box::new(primary_action(
        "Buscar redes",
        material::icon::SEARCH,
        theme,
        Network::scan,
    )));
    card_content(children, theme)
}

fn bluetooth_content(theme: ThemeColors, bluetooth: &Bluetooth) -> Rectangle {
    let mut children: Vec<Box<dyn Widget>> = vec![Box::new(label("Bluetooth", theme.text, 19.0))];
    let powered = bluetooth.powered();
    children.push(Box::new(toggle_row("Ativado", powered, theme, move || {
        Bluetooth::set_powered(!powered)
    })));
    if powered {
        let scanning = bluetooth.scanning();
        children.push(Box::new(toggle_row(
            "Buscando dispositivos",
            scanning,
            theme,
            move || {
                if scanning {
                    Bluetooth::stop_scan();
                } else {
                    Bluetooth::start_scan();
                }
            },
        )));
    }
    let visible_devices = bluetooth.devices().len();
    children.push(Box::new(label(
        format!("{visible_devices} dispositivos disponíveis"),
        theme.muted,
        12.0,
    )));
    for device in bluetooth.devices().iter().take(7) {
        let path = device.path().to_owned();
        let connected = device.connected();
        let paired = device.paired();
        let name = device.name().to_owned();
        let description = if connected {
            "Conectado"
        } else if paired {
            "Emparelhado"
        } else {
            "Disponível"
        };
        children.push(Box::new(device_row(
            material::icon::BLUETOOTH,
            &name,
            description,
            connected,
            theme,
            move || {
                if connected {
                    Bluetooth::disconnect(&path);
                } else if paired {
                    Bluetooth::connect(&path);
                } else {
                    Bluetooth::pair(&path);
                }
            },
        )));
    }
    if powered && bluetooth.devices().is_empty() {
        children.push(Box::new(label(
            "Nenhum dispositivo disponível",
            theme.muted,
            13.0,
        )));
    }
    children.push(Box::new(primary_action(
        "Abrir configurações",
        material::icon::SETTINGS,
        theme,
        || {
            let _ = Command::new("blueman-manager").spawn();
        },
    )));
    card_content(children, theme)
}

fn audio_content(theme: ThemeColors, audio: &Audio, devices: &AudioDevices) -> Rectangle {
    let mut children: Vec<Box<dyn Widget>> =
        vec![Box::new(label("Dispositivo de saída", theme.text, 17.0))];
    if devices.outputs.is_empty() {
        children.push(Box::new(label(
            "Nenhum dispositivo de saída encontrado",
            theme.muted,
            12.0,
        )));
    }
    for device in devices.outputs.iter().take(4) {
        let selected = device.name == devices.default_output;
        let name = device.name.clone();
        children.push(Box::new(audio_device_row(
            &device.description,
            selected,
            theme,
            move || set_audio_device("sink", &name),
        )));
    }

    children.push(Box::new(label("Dispositivo de entrada", theme.text, 17.0)));
    if devices.inputs.is_empty() {
        children.push(Box::new(label(
            "Nenhum dispositivo de entrada encontrado",
            theme.muted,
            12.0,
        )));
    }
    for device in devices.inputs.iter().take(4) {
        let selected = device.name == devices.default_input;
        let name = device.name.clone();
        children.push(Box::new(audio_device_row(
            &device.description,
            selected,
            theme,
            move || set_audio_device("source", &name),
        )));
    }

    children.push(Box::new(label(
        if audio.muted() {
            "Volume (mudo)".to_owned()
        } else {
            format!("Volume ({}%)", audio.volume())
        },
        theme.text,
        17.0,
    )));
    children.push(Box::new(volume_slider(audio.volume(), false, theme)));
    children.push(Box::new(primary_action(
        "Abrir configurações",
        material::icon::SETTINGS,
        theme,
        || {
            let _ = Command::new("pavucontrol").spawn();
        },
    )));
    card_content(children, theme)
}

fn set_audio_device(kind: &'static str, name: &str) {
    let device = name.to_owned();
    thread::spawn(move || {
        let succeeded = Command::new("pactl")
            .arg(format!("set-default-{kind}"))
            .arg(&device)
            .status()
            .is_ok_and(|status| status.success());
        if succeeded {
            if let Some(devices) = AUDIO_DEVICES.get() {
                let mut devices = devices.lock().unwrap_or_else(|error| error.into_inner());
                if kind == "sink" {
                    devices.default_output = device;
                } else {
                    devices.default_input = device;
                }
            }
            request_frame();
        }
    });
}

fn audio_device_row(
    title: &str,
    selected: bool,
    theme: ThemeColors,
    on_select: impl Fn() + 'static,
) -> Rectangle {
    let radio = if selected {
        material::icon::RADIO_CHECKED
    } else {
        material::icon::RADIO_UNCHECKED
    };
    Rectangle::new()
        .width(Parent)
        .height(34.0)
        .fill(Color::TRANSPARENT)
        .cursor(amane::Pointer)
        .child(
            amane::Row::new(vec![
                Box::new(crate::material::icon(
                    radio,
                    22.0,
                    if selected { theme.accent } else { theme.muted },
                )),
                Box::new(
                    Text::new(title.to_owned())
                        .size(14.0)
                        .color(if selected { theme.accent } else { theme.text })
                        .elide(),
                ),
            ])
            .width(Parent)
            .height(Parent)
            .gap(material_space::SM)
            .align(Center)
            .justify(amane::Start),
        )
        .on_click(move |_| on_select())
}

fn volume_slider(level: u8, input: bool, theme: ThemeColors) -> Rectangle {
    let on_drag_input = input;
    let on_scroll_input = input;
    let fill_width = 268.0 * level as f32 / 100.0;
    Rectangle::new()
        .width(Parent)
        .height(42.0)
        .fill(Color::TRANSPARENT)
        .child(
            Stack::new(vec![
                Box::new(
                    Rectangle::new()
                        .width(Parent)
                        .height(30.0)
                        .radius(material_shape::FULL)
                        .fill(theme.workspace_group_surface)
                        .align_child(amane::Start, Center)
                        .child(
                            Rectangle::new()
                                .width(fill_width)
                                .height(30.0)
                                .radius(material_shape::FULL)
                                .fill(theme.accent),
                        ),
                ),
                Box::new(
                    Rectangle::new()
                        .width(6.0)
                        .height(42.0)
                        .radius(material_shape::FULL)
                        .fill(theme.active_text)
                        .translate(fill_width - 3.0, 0.0),
                ),
            ])
            .width(Parent)
            .height(Parent),
        )
        .cursor(amane::Pointer)
        .on_drag(move |point| {
            let value = ((point.x / 268.0) * 100.0).round().clamp(0.0, 100.0) as u8;
            set_audio_level(on_drag_input, value);
        })
        .on_scroll(move |scroll| {
            let step = if scroll.y > 0.0 { 5 } else { -5 };
            let value = (level as i16 + step).clamp(0, 100) as u8;
            set_audio_level(on_scroll_input, value);
        })
}

fn set_audio_level(input: bool, value: u8) {
    if input {
        Audio::set_microphone_volume(value);
    } else {
        Audio::set_volume(value);
    }
}

fn wifi_network_row(
    ssid: &str,
    strength: u8,
    secured: bool,
    connected: bool,
    theme: ThemeColors,
    on_select: impl Fn() + 'static,
) -> Rectangle {
    let foreground = if connected { theme.accent } else { theme.text };
    let lock: Box<dyn Widget> = if secured {
        Box::new(crate::material::icon(
            material::icon::LOCK,
            17.0,
            theme.muted,
        ))
    } else {
        Box::new(Rectangle::new().width(18.0).height(18.0))
    };
    let connected_icon = if connected {
        material::icon::CHECK
    } else {
        material::icon::LINK
    };
    Rectangle::new()
        .width(Parent)
        .height(36.0)
        .fill(Color::TRANSPARENT)
        .cursor(amane::Pointer)
        .child(
            amane::Row::new(vec![
                Box::new(crate::material::icon(
                    material::icon::SIGNAL_WIFI,
                    20.0,
                    if strength >= 60 {
                        theme.accent
                    } else {
                        theme.muted
                    },
                )),
                lock,
                Box::new(
                    Text::new(ssid.to_owned())
                        .size(14.0)
                        .color(foreground)
                        .elide(),
                ),
                Box::new(crate::material::icon(
                    connected_icon,
                    19.0,
                    if connected { theme.accent } else { theme.muted },
                )),
            ])
            .width(Parent)
            .height(Parent)
            .gap(material_space::SM)
            .align(Center)
            .justify(amane::SpaceBetween),
        )
        .on_click(move |_| on_select())
}

fn device_row(
    icon: &'static str,
    title: &str,
    detail: &str,
    connected: bool,
    theme: ThemeColors,
    on_select: impl Fn() + 'static,
) -> Rectangle {
    Rectangle::new()
        .width(Parent)
        .height(36.0)
        .fill(Color::TRANSPARENT)
        .cursor(amane::Pointer)
        .child(
            amane::Row::new(vec![
                Box::new(crate::material::icon(icon, 18.0, theme.muted)),
                Box::new(
                    Column::new(vec![
                        Box::new(
                            Text::new(title.to_owned())
                                .size(14.0)
                                .color(if connected { theme.accent } else { theme.text })
                                .elide(),
                        ),
                        Box::new(Text::new(detail.to_owned()).size(11.0).color(theme.muted)),
                    ])
                    .width(Parent)
                    .height(Parent)
                    .gap(material_space::XS)
                    .align(amane::Start)
                    .justify(Center),
                ),
                Box::new(crate::material::icon(
                    if connected {
                        material::icon::LINK
                    } else {
                        material::icon::ADD
                    },
                    18.0,
                    if connected { theme.accent } else { theme.muted },
                )),
            ])
            .width(Parent)
            .height(Parent)
            .gap(material_space::SM)
            .align(Center)
            .justify(amane::SpaceBetween),
        )
        .on_click(move |_| on_select())
}

fn toggle_row(
    title: &str,
    enabled: bool,
    theme: ThemeColors,
    on_toggle: impl Fn() + 'static,
) -> Rectangle {
    let track = Rectangle::new()
        .width(50.0)
        .height(30.0)
        .radius(material_shape::FULL)
        .fill(if enabled {
            theme.active_surface
        } else {
            theme.workspace_group_surface
        })
        .align_child(amane::Start, Center)
        .padding(material_space::XS)
        .child(
            Rectangle::new()
                .width(22.0)
                .height(22.0)
                .radius(material_shape::FULL)
                .fill(if enabled { theme.accent } else { theme.muted })
                .align_child(Center, Center)
                .translate(if enabled { 20.0 } else { 0.0 }, 0.0)
                .child(crate::material::icon(
                    if enabled {
                        material::icon::CHECK
                    } else {
                        material::icon::CLOSE
                    },
                    14.0,
                    if enabled {
                        theme.active_text
                    } else {
                        theme.text
                    },
                )),
        );
    Rectangle::new()
        .width(Parent)
        .height(42.0)
        .fill(Color::TRANSPARENT)
        .cursor(amane::Pointer)
        .align_child(amane::Start, Center)
        .child(
            amane::Row::new(vec![
                Box::new(Text::new(title.to_owned()).size(14.0).color(theme.text)),
                Box::new(track),
            ])
            .width(Parent)
            .height(Parent)
            .align(Center)
            .justify(amane::SpaceBetween),
        )
        .on_click(move |_| on_toggle())
}

fn primary_action(
    title: &str,
    icon: &'static str,
    theme: ThemeColors,
    on_click: impl Fn() + 'static,
) -> Rectangle {
    Rectangle::new()
        .width(Parent)
        .height(42.0)
        .radius(material_shape::FULL)
        .fill(theme.active_surface)
        .cursor(amane::Pointer)
        .align_child(Center, Center)
        .child(
            amane::Row::new(vec![
                Box::new(crate::material::icon(icon, 20.0, theme.active_text)),
                Box::new(
                    Text::new(title.to_owned())
                        .size(14.0)
                        .color(theme.active_text),
                ),
            ])
            .height(Parent)
            .gap(material_space::SM)
            .align(Center)
            .justify(Center),
        )
        .on_click(move |_| on_click())
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
                .align(amane::Start)
                .justify(amane::Start),
        )
}

fn label(title: impl Into<String>, color: Color, size: f32) -> Text {
    Text::new(title).size(size).color(color)
}
