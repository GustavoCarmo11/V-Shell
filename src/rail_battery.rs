use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use amane::Service;

const POWER_SUPPLIES: &str = "/sys/class/power_supply";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RailBattery {
    pub present: bool,
    pub percent: u8,
    pub charging: bool,
}

#[derive(Default)]
struct BatteryMonitor {
    path: Option<PathBuf>,
    battery: RailBattery,
}

impl BatteryMonitor {
    pub fn snapshot() -> RailBattery {
        Self::read().battery
    }
}

impl Service for BatteryMonitor {
    fn new() -> Self {
        let path = find_battery();
        let battery = read_battery(path.as_deref());
        Self { path, battery }
    }

    fn interval() -> Duration {
        Duration::from_secs(1)
    }

    fn update(&mut self) -> bool {
        if self.path.as_ref().is_none_or(|path| !path.exists()) {
            self.path = find_battery();
        }

        let next = read_battery(self.path.as_deref());
        if self.battery == next {
            return false;
        }

        self.battery = next;
        true
    }
}

fn find_battery() -> Option<PathBuf> {
    let entries = fs::read_dir(POWER_SUPPLIES).ok()?;

    entries
        .flatten()
        .map(|entry| entry.path())
        .find(|path| read(path, "type") == "Battery" && read(path, "scope") != "Device")
}

fn read_battery(path: Option<&Path>) -> RailBattery {
    let Some(path) = path else {
        return RailBattery::default();
    };

    RailBattery {
        present: true,
        percent: read(path, "capacity").parse::<u8>().unwrap_or(0).min(100),
        charging: read(path, "status") == "Charging",
    }
}

fn read(path: &Path, file: &str) -> String {
    fs::read_to_string(path.join(file))
        .unwrap_or_default()
        .trim()
        .to_owned()
}
