use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use amane::{
    Battery, Center, Color, Column, Cpu, Memory, Network, Parent, Rectangle, Row, Service, Stack,
    Start, Text,
};

use crate::theme::ThemeColors;

const MAIN_WIDTH: f32 = 532.0;
const BATTERY_WIDTH: f32 = 146.0;
const CONTENT_WIDTH: f32 = 694.0;
const CONTENT_HEIGHT: f32 = 326.0;
const ROW_GAP: f32 = 12.0;

#[derive(Default, Clone, PartialEq, Eq)]
struct HardwareStats {
    enabled: bool,
    download_bps: u64,
    upload_bps: u64,
    temperature: Option<i32>,
    temperature_source: String,
    disk_model: String,
    disk_capacity: String,
    root_usage: String,
}

impl Service for HardwareStats {
    fn new() -> Self {
        Self::default()
    }

    fn listen() {
        let (mut previous_rx, mut previous_tx) = network_counters();
        let mut previous_at = Instant::now();

        loop {
            if !Self::read().enabled {
                // Keep the network baseline fresh while this page is hidden.
                (previous_rx, previous_tx) = network_counters();
                previous_at = Instant::now();
                thread::sleep(Duration::from_millis(800));
                continue;
            }

            let (current_rx, current_tx) = network_counters();
            let elapsed = previous_at.elapsed().as_secs_f64().max(0.1);
            let mut fresh = collect_hardware();
            fresh.download_bps = ((current_rx.saturating_sub(previous_rx)) as f64 / elapsed) as u64;
            fresh.upload_bps = ((current_tx.saturating_sub(previous_tx)) as f64 / elapsed) as u64;
            previous_rx = current_rx;
            previous_tx = current_tx;
            previous_at = Instant::now();

            // The tab may have been closed while sysfs/df were being read.
            if !Self::read().enabled {
                thread::sleep(Duration::from_millis(800));
                continue;
            }

            fresh.enabled = true;
            let changed = {
                let current = Self::read();
                current.temperature != fresh.temperature
                    || current.temperature_source != fresh.temperature_source
                    || current.disk_model != fresh.disk_model
                    || current.disk_capacity != fresh.disk_capacity
                    || current.root_usage != fresh.root_usage
                    || current.download_bps != fresh.download_bps
                    || current.upload_bps != fresh.upload_bps
            };
            if changed {
                let mut current = Self::write();
                if current.enabled {
                    *current = fresh;
                }
            }

            thread::sleep(Duration::from_secs(3));
        }
    }
}

pub(super) fn set_enabled(enabled: bool) {
    let current = HardwareStats::read().enabled;
    if current != enabled {
        HardwareStats::write().enabled = enabled;
    }
}

pub(super) fn content_size() -> (f32, f32) {
    (CONTENT_WIDTH, CONTENT_HEIGHT)
}

pub(super) fn view(theme: ThemeColors) -> Rectangle {
    set_enabled(true);

    let cpu = Cpu::read().percent();
    let memory = {
        let memory = Memory::read();
        (memory.percent(), memory.used_kib(), memory.total_kib())
    };
    let network = {
        let network = Network::read();
        let name = if !network.connected() {
            "Desconectado".to_owned()
        } else if network.ssid().is_empty() {
            "Conectado".to_owned()
        } else {
            network.ssid().to_owned()
        };
        (network.connected(), name)
    };
    let battery = {
        let battery = Battery::read();
        (
            battery.present(),
            battery.percent(),
            battery.charging(),
            battery.full(),
        )
    };
    let hardware = HardwareStats::read().clone();

    let cpu_value = format!("{}%", cpu);
    let cpu_detail = "Utilização total do processador".to_owned();
    let temp_value = hardware
        .temperature
        .map(|temperature| format!("{temperature} °C"))
        .unwrap_or_else(|| "N/D".to_owned());
    let temp_detail = if hardware.temperature_source.is_empty() {
        "Sensor indisponível".to_owned()
    } else {
        hardware.temperature_source.clone()
    };
    let ram_value = format!("{}%", memory.0);
    let ram_detail = format!(
        "{} / {} GiB",
        format_gib(memory.1.saturating_mul(1024)),
        format_gib(memory.2.saturating_mul(1024))
    );
    let network_value = if network.0 {
        format!("↓ {}", format_rate(hardware.download_bps))
    } else {
        "Offline".to_owned()
    };
    let network_detail = if network.0 {
        format!("↑ {} · {}", format_rate(hardware.upload_bps), network.1)
    } else {
        network.1
    };
    let disk_value = if hardware.disk_capacity.is_empty() {
        "Não detectado".to_owned()
    } else {
        hardware.disk_capacity.clone()
    };
    let disk_detail = if hardware.disk_model.is_empty() {
        hardware.root_usage.clone()
    } else if hardware.root_usage.is_empty() {
        hardware.disk_model.clone()
    } else {
        format!("{} · {}", hardware.disk_model, hardware.root_usage)
    };

    let cpu_card = hero_card(
        "\u{e30d}",
        "CPU",
        &cpu_value,
        &cpu_detail,
        (cpu as f32 / 100.0).clamp(0.0, 1.0),
        theme.accent,
        theme,
    );
    let temperature_card = hero_card(
        "\u{f076}",
        "TEMPERATURA",
        &temp_value,
        &temp_detail,
        (hardware.temperature.unwrap_or(0) as f32 / 100.0).clamp(0.0, 1.0),
        theme.accent,
        theme,
    );

    let main_top = Row::new(vec![Box::new(cpu_card), Box::new(temperature_card)])
        .width(MAIN_WIDTH)
        .height(164.0)
        .gap(12.0)
        .align(Start)
        .justify(Start);

    let main_bottom = Row::new(vec![
        Box::new(small_metric_card(
            "\u{e322}",
            "RAM",
            &ram_value,
            &ram_detail,
            theme,
        )),
        Box::new(small_metric_card(
            "\u{e640}",
            "REDE",
            &network_value,
            &network_detail,
            theme,
        )),
        Box::new(small_metric_card(
            "\u{e1db}",
            "SSD",
            &disk_value,
            &disk_detail,
            theme,
        )),
    ])
    .width(MAIN_WIDTH)
    .height(150.0)
    .gap(12.0)
    .align(Start)
    .justify(Start);

    let main_column = Column::new(vec![Box::new(main_top), Box::new(main_bottom)])
        .width(MAIN_WIDTH)
        .height(CONTENT_HEIGHT)
        .gap(ROW_GAP)
        .align(Start)
        .justify(Start);

    let battery_card = battery_tank_card(battery.0, battery.1 as f32, battery.2, battery.3, theme);

    Rectangle::new()
        .width(CONTENT_WIDTH)
        .height(CONTENT_HEIGHT)
        .fill(Color::TRANSPARENT)
        .child(
            Row::new(vec![Box::new(main_column), Box::new(battery_card)])
                .width(CONTENT_WIDTH)
                .height(CONTENT_HEIGHT)
                .gap(16.0)
                .align(Start)
                .justify(Start),
        )
}

fn hero_card(
    icon: &'static str,
    label: &str,
    value: &str,
    detail: &str,
    progress: f32,
    accent: Color,
    theme: ThemeColors,
) -> Rectangle {
    const CARD_WIDTH: f32 = 260.0;
    const CARD_HEIGHT: f32 = 164.0;
    const INNER_WIDTH: f32 = CARD_WIDTH - 32.0;

    let progress_bar = progress_bar(progress, INNER_WIDTH, accent, theme.divider);

    Rectangle::new()
        .width(CARD_WIDTH)
        .height(CARD_HEIGHT)
        .radius(22.0)
        .fill(theme.button_surface)
        .padding(16.0)
        .child(
            Column::new(vec![
                Box::new(
                    Row::new(vec![
                        Box::new(
                            Text::new(icon)
                                .font("Material Symbols Rounded")
                                .size(19.0)
                                .color(accent),
                        ),
                        Box::new(Text::new(label.to_owned()).size(12.0).color(accent)),
                    ])
                    .width(Parent)
                    .height(22.0)
                    .gap(8.0)
                    .align(Center)
                    .justify(Start),
                ),
                Box::new(
                    Text::new(value.to_owned())
                        .size(37.0)
                        .color(theme.text)
                        .elide(),
                ),
                Box::new(progress_bar),
                Box::new(
                    Text::new(detail.to_owned())
                        .size(11.0)
                        .color(theme.muted)
                        .elide(),
                ),
            ])
            .width(Parent)
            .height(Parent)
            .gap(8.0)
            .align(Start)
            .justify(Center),
        )
}

fn small_metric_card(
    icon: &'static str,
    label: &str,
    value: &str,
    detail: &str,
    theme: ThemeColors,
) -> Rectangle {
    const CARD_WIDTH: f32 = 169.33;
    const CARD_HEIGHT: f32 = 150.0;

    Rectangle::new()
        .width(CARD_WIDTH)
        .height(CARD_HEIGHT)
        .radius(20.0)
        .fill(theme.button_surface)
        .padding(14.0)
        .child(
            Column::new(vec![
                Box::new(
                    Row::new(vec![
                        Box::new(
                            Text::new(icon)
                                .font("Material Symbols Rounded")
                                .size(18.0)
                                .color(theme.accent),
                        ),
                        Box::new(Text::new(label.to_owned()).size(11.0).color(theme.muted)),
                    ])
                    .width(Parent)
                    .height(22.0)
                    .gap(7.0)
                    .align(Center)
                    .justify(Start),
                ),
                Box::new(
                    Text::new(value.to_owned())
                        .size(23.0)
                        .color(theme.text)
                        .elide(),
                ),
                Box::new(
                    Text::new(detail.to_owned())
                        .size(10.0)
                        .color(theme.muted)
                        .elide(),
                ),
            ])
            .width(Parent)
            .height(Parent)
            .gap(9.0)
            .align(Start)
            .justify(Center),
        )
}

fn progress_bar(progress: f32, width: f32, foreground: Color, background: Color) -> Stack {
    Stack::new(vec![
        Box::new(
            Rectangle::new()
                .width(Parent)
                .height(7.0)
                .radius(4.0)
                .fill(background),
        ),
        Box::new(
            Rectangle::new()
                .width(width * progress.clamp(0.0, 1.0))
                .height(7.0)
                .radius(4.0)
                .fill(foreground),
        ),
    ])
    .width(width)
    .height(7.0)
}

fn battery_tank_card(
    present: bool,
    percentage: f32,
    charging: bool,
    full: bool,
    theme: ThemeColors,
) -> Rectangle {
    const GAUGE_WIDTH: f32 = 88.0;
    const GAUGE_HEIGHT: f32 = 154.0;

    let percent = if present {
        percentage.clamp(0.0, 100.0) / 100.0
    } else {
        0.0
    };
    let filled_height = GAUGE_HEIGHT * percent;
    let empty_height = GAUGE_HEIGHT - filled_height;
    let value = if present {
        format!("{percentage:.0}%")
    } else {
        "N/D".to_owned()
    };
    let status = if !present {
        "Sem bateria"
    } else if charging {
        "Carregando"
    } else if full {
        "Carga completa"
    } else {
        "Na bateria"
    };

    let gauge = Rectangle::new()
        .width(GAUGE_WIDTH)
        .height(GAUGE_HEIGHT)
        .radius(17.0)
        .clip()
        .fill(theme.divider)
        .child(
            Column::new(vec![
                Box::new(
                    Rectangle::new()
                        .width(GAUGE_WIDTH)
                        .height(empty_height.max(0.0))
                        .fill(theme.divider),
                ),
                Box::new(
                    Rectangle::new()
                        .width(GAUGE_WIDTH)
                        .height(filled_height.max(0.1))
                        .fill(theme.accent),
                ),
            ])
            .width(GAUGE_WIDTH)
            .height(GAUGE_HEIGHT)
            .gap(0.0)
            .align(Center)
            .justify(Start),
        );

    Rectangle::new()
        .width(BATTERY_WIDTH)
        .height(CONTENT_HEIGHT)
        .radius(22.0)
        .fill(theme.button_surface)
        .padding(14.0)
        .child(
            Column::new(vec![
                Box::new(
                    Row::new(vec![
                        Box::new(
                            Text::new("\u{e1a4}")
                                .font("Material Symbols Rounded")
                                .size(18.0)
                                .color(theme.accent),
                        ),
                        Box::new(Text::new("BATERIA").size(11.0).color(theme.muted)),
                    ])
                    .width(Parent)
                    .height(22.0)
                    .gap(6.0)
                    .align(Center)
                    .justify(Start),
                ),
                Box::new(Text::new(value).size(28.0).color(theme.text)),
                Box::new(Text::new(status).size(10.0).color(theme.muted).elide()),
                Box::new(gauge),
            ])
            .width(Parent)
            .height(Parent)
            .gap(9.0)
            .align(Center)
            .justify(Center),
        )
}

fn collect_hardware() -> HardwareStats {
    let (temperature, temperature_source) = read_temperature();
    let (disk_model, disk_capacity) = read_disk();
    let root_usage = read_root_usage();

    HardwareStats {
        temperature,
        temperature_source,
        disk_model,
        disk_capacity,
        root_usage,
        ..HardwareStats::default()
    }
}

fn read_temperature() -> (Option<i32>, String) {
    let mut candidates: Vec<(i32, String, bool)> = Vec::new();

    if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
        for entry in entries.flatten() {
            let directory = entry.path();
            let sensor_name = read_trimmed(directory.join("name"));
            let normalized = sensor_name.to_ascii_lowercase();
            let cpu_sensor = ["coretemp", "k10temp", "zenpower", "cpu_thermal", "acpitz"]
                .iter()
                .any(|name| normalized.contains(name));

            let Ok(files) = fs::read_dir(&directory) else {
                continue;
            };
            for file in files.flatten() {
                let name = file.file_name().to_string_lossy().into_owned();
                if !name.starts_with("temp") || !name.ends_with("_input") {
                    continue;
                }
                if let Ok(value) = fs::read_to_string(file.path())
                    && let Ok(milli_celsius) = value.trim().parse::<i32>()
                {
                    candidates.push((milli_celsius / 1000, sensor_name.clone(), cpu_sensor));
                }
            }
        }
    }

    if let Some(candidate) = candidates
        .iter()
        .filter(|candidate| candidate.2)
        .max_by_key(|candidate| candidate.0)
    {
        return (Some(candidate.0), candidate.1.clone());
    }

    if let Some(candidate) = candidates.iter().max_by_key(|candidate| candidate.0) {
        return (Some(candidate.0), candidate.1.clone());
    }

    if let Ok(entries) = fs::read_dir("/sys/class/thermal") {
        for entry in entries.flatten() {
            let path = entry.path().join("temp");
            if let Ok(value) = fs::read_to_string(path)
                && let Ok(milli_celsius) = value.trim().parse::<i32>()
            {
                return (Some(milli_celsius / 1000), "Thermal zone".to_owned());
            }
        }
    }

    (None, String::new())
}

fn read_disk() -> (String, String) {
    let Ok(entries) = fs::read_dir("/sys/block") else {
        return (String::new(), String::new());
    };

    let mut disks: Vec<(bool, String, String)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !(name.starts_with("nvme")
            || name.starts_with("sd")
            || name.starts_with("vd")
            || name.starts_with("mmcblk"))
        {
            continue;
        }

        let rotational = read_trimmed(path.join("queue/rotational"));
        let is_solid_state =
            rotational == "0" || name.starts_with("nvme") || name.starts_with("mmcblk");
        if !is_solid_state {
            continue;
        }

        let sectors = read_trimmed(path.join("size")).parse::<u64>().unwrap_or(0);
        let capacity = format_bytes(sectors.saturating_mul(512));
        let model = [
            path.join("device/model"),
            path.join("device/name"),
            path.join("model"),
        ]
        .iter()
        .map(read_trimmed)
        .find(|value| !value.is_empty())
        .unwrap_or_else(|| name.to_owned());
        disks.push((name.starts_with("nvme"), model, capacity));
    }

    disks.sort_by_key(|(nvme, _, _)| !*nvme);
    disks
        .into_iter()
        .next()
        .map(|(_, model, capacity)| (model.trim().to_owned(), capacity))
        .unwrap_or_default()
}

fn read_root_usage() -> String {
    let output = Command::new("df").args(["-h", "-P", "/"]).output();
    let Ok(output) = output else {
        return String::new();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let Some(line) = text.lines().nth(1) else {
        return String::new();
    };
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 5 {
        return String::new();
    }
    format!("{} / {} · {} em /", fields[2], fields[1], fields[4])
}

fn network_counters() -> (u64, u64) {
    let text = fs::read_to_string("/proc/net/dev").unwrap_or_default();
    let mut received = 0_u64;
    let mut transmitted = 0_u64;

    for line in text.lines().skip(2) {
        let Some((interface, counters)) = line.split_once(':') else {
            continue;
        };
        if interface.trim() == "lo" {
            continue;
        }
        let fields: Vec<u64> = counters
            .split_whitespace()
            .filter_map(|value| value.parse().ok())
            .collect();
        if fields.len() >= 9 {
            received = received.saturating_add(fields[0]);
            transmitted = transmitted.saturating_add(fields[8]);
        }
    }

    (received, transmitted)
}

fn read_trimmed(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path)
        .map(|value| value.trim().to_owned())
        .unwrap_or_default()
}

fn format_bytes(bytes: u64) -> String {
    const GB: f64 = 1_000_000_000.0;
    const TB: f64 = 1_000_000_000_000.0;
    if bytes >= TB as u64 {
        format!("{:.1} TB", bytes as f64 / TB)
    } else {
        format!("{:.0} GB", bytes as f64 / GB)
    }
}

fn format_gib(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / 1_073_741_824.0)
}

fn format_rate(bytes_per_second: u64) -> String {
    if bytes_per_second >= 1_000_000 {
        format!("{:.1} MB/s", bytes_per_second as f64 / 1_000_000.0)
    } else if bytes_per_second >= 1_000 {
        format!("{:.0} KB/s", bytes_per_second as f64 / 1_000.0)
    } else {
        format!("{bytes_per_second} B/s")
    }
}
