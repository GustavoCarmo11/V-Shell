use std::{
    env,
    io::{BufRead, BufReader, Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
};

use amane::Service;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveWindowInfo {
    pub class: String,
    pub title: String,
}

#[derive(Default)]
pub struct ActiveWindow {
    current: Option<ActiveWindowInfo>,
}

impl ActiveWindow {
    pub fn current() -> Option<ActiveWindowInfo> {
        Self::read().current.clone()
    }

    fn socket_path(name: &str) -> Option<PathBuf> {
        let runtime = env::var_os("XDG_RUNTIME_DIR")?;
        let instance = env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;

        Some(
            PathBuf::from(runtime)
                .join("hypr")
                .join(instance)
                .join(name),
        )
    }

    fn request(command: &str) -> Option<String> {
        let path = Self::socket_path(".socket.sock")?;
        let mut stream = UnixStream::connect(path).ok()?;

        stream.write_all(command.as_bytes()).ok()?;

        let mut response = String::new();
        stream.read_to_string(&mut response).ok()?;

        Some(response)
    }

    fn initial_window() -> Option<ActiveWindowInfo> {
        let response = Self::request("j/activewindow")?;
        let value: Value = serde_json::from_str(&response).ok()?;

        let class = value
            .get("class")
            .or_else(|| value.get("initialClass"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();

        let title = value
            .get("title")
            .or_else(|| value.get("initialTitle"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();

        if class.is_empty() && title.is_empty() {
            None
        } else {
            Some(ActiveWindowInfo { class, title })
        }
    }

    fn set_current(next: Option<ActiveWindowInfo>) {
        let changed = Self::read().current != next;

        if changed {
            Self::write().current = next;
        }
    }

    fn apply_event(event: &str) {
        let Some((kind, payload)) = event.split_once(">>") else {
            return;
        };

        match kind {
            // The activewindow event contains both the app class and title.
            "activewindow" => {
                let (class, title) = payload.split_once(',').unwrap_or((payload, ""));
                let class = class.trim().to_owned();
                let title = title.trim().to_owned();

                if class.is_empty() && title.is_empty() {
                    Self::set_current(None);
                } else {
                    Self::set_current(Some(ActiveWindowInfo { class, title }));
                }
            }
            // Hyprland sends windowtitle with only the address. Query the
            // active window to refresh its title without polling.
            "windowtitle" | "windowtitlev2" | "activewindowv2" => {
                if let Some(window) = Self::initial_window() {
                    Self::set_current(Some(window));
                }
            }
            _ => {}
        }
    }
}

impl Service for ActiveWindow {
    fn new() -> Self {
        Self::default()
    }

    fn listen() {
        let Some(path) = Self::socket_path(".socket2.sock") else {
            return;
        };

        let Ok(stream) = UnixStream::connect(path) else {
            return;
        };

        let events = BufReader::new(stream).lines();

        // Subscribe primeiro e depois consulta o estado inicial.
        {
            let mut service = Self::write();
            service.current = Self::initial_window();
        }

        for event in events {
            let Ok(event) = event else {
                break;
            };

            Self::apply_event(&event);
        }
    }
}
