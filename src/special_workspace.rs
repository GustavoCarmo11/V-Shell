use std::{
    collections::HashSet,
    env,
    io::{BufRead, BufReader, Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
};

use amane::Service;
use serde_json::Value;

/// Tracks which Hyprland outputs currently show a special workspace.
///
/// Amane intentionally excludes special workspaces from `Workspaces::list()`,
/// so this small event-driven service listens to Hyprland's active-special IPC
/// event without polling or running `hyprctl` during drawing.
pub struct SpecialWorkspace {
    active_outputs: HashSet<String>,
}

impl SpecialWorkspace {
    pub fn active_on(output: &str) -> bool {
        Self::read().active_outputs.contains(output)
    }

    fn apply_event(event: &str) {
        let Some((kind, payload)) = event.split_once(">>") else {
            return;
        };

        let (workspace_name, output) = match kind {
            "activespecialv2" => {
                let mut fields = payload.splitn(3, ',');
                let _workspace_id = fields.next();
                let name = fields.next().unwrap_or_default();
                let output = fields.next().unwrap_or_default();
                (name, output)
            }
            "activespecial" => {
                let mut fields = payload.splitn(2, ',');
                let name = fields.next().unwrap_or_default();
                let output = fields.next().unwrap_or_default();
                (name, output)
            }
            _ => return,
        };

        let output = output.trim();
        if output.is_empty() {
            return;
        }

        let active = !workspace_name.trim().is_empty();

        // Avoid acquiring a write guard (and notifying subscribers) when the
        // event repeats the state we already have. `quiet()` is intentionally
        // crate-private in Amane, so the no-op check belongs before `write()`.
        let was_active = Self::read().active_outputs.contains(output);
        if was_active == active {
            return;
        }

        let mut service = Self::write();
        if active {
            service.active_outputs.insert(output.to_owned());
        } else {
            service.active_outputs.remove(output);
        }
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

    fn initial_outputs() -> HashSet<String> {
        let Some(response) = Self::request("j/monitors") else {
            return HashSet::new();
        };
        let Ok(monitors) = serde_json::from_str::<Value>(&response) else {
            return HashSet::new();
        };

        monitors
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|monitor| {
                let active_id = monitor
                    .get("specialWorkspace")?
                    .get("id")?
                    .as_i64()
                    .unwrap_or(0);

                if active_id == 0 {
                    return None;
                }

                monitor.get("name")?.as_str().map(str::to_owned)
            })
            .collect()
    }
}

impl Service for SpecialWorkspace {
    fn new() -> Self {
        Self {
            active_outputs: HashSet::new(),
        }
    }

    fn listen() {
        // This feature is specific to Hyprland; other compositors leave it off.
        let Some(path) = Self::socket_path(".socket2.sock") else {
            return;
        };
        let Ok(stream) = UnixStream::connect(path) else {
            return;
        };

        // Subscribe first, then snapshot. Any event occurring during the
        // snapshot remains queued and is applied immediately afterwards.
        let events = BufReader::new(stream).lines();

        {
            let mut service = Self::write();
            service.active_outputs = Self::initial_outputs();
        }

        for event in events {
            let Ok(event) = event else {
                break;
            };
            Self::apply_event(&event);
        }
    }
}
