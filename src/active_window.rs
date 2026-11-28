use std::{
    env,
    io::{BufRead, BufReader, Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    time::Duration,
};

use amane::{Animation, Easing, Service};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveWindowInfo {
    pub class: String,
    pub title: String,
}

#[derive(Default)]
pub struct ActiveWindow {
    current: Option<ActiveWindowInfo>,
    previous_title: Option<String>,
    title_transition: Option<Animation<f32>>,
}

impl ActiveWindow {
    pub fn current() -> Option<ActiveWindowInfo> {
        Self::read().current.clone()
    }

    /// Returns the outgoing title and the incoming title's crossfade progress.
    pub fn title_transition() -> (Option<String>, f32) {
        let active = Self::read();
        let Some(animation) = active.title_transition.as_ref() else {
            return (None, 1.0);
        };

        let progress = animation.value().clamp(0.0, 1.0);
        let previous_title = if progress < 1.0 {
            active.previous_title.clone()
        } else {
            None
        };

        (previous_title, progress)
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
        if Self::read().current == next {
            return;
        }

        let mut active = Self::write();

        if active.current == next {
            return;
        }

        let previous_title = Self::display_title(active.current.as_ref());
        let next_title = Self::display_title(next.as_ref());

        if previous_title != next_title {
            active.previous_title = Some(previous_title.to_owned());

            let mut transition = Animation::new(0.0)
                .duration(Duration::from_millis(230))
                .easing(Easing::InOut);
            transition.to(1.0);
            active.title_transition = Some(transition);
        }

        active.current = next;
    }

    fn display_title(window: Option<&ActiveWindowInfo>) -> &str {
        window
            .map(|window| window.title.as_str())
            .filter(|title| !title.trim().is_empty())
            .unwrap_or("Desktop")
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
