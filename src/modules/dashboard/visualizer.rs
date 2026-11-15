use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use amane::Service;

#[derive(Default)]
struct SpectrumState {
    enabled: bool,
    available: bool,
    bars: Vec<u8>,
}

impl Service for SpectrumState {
    fn new() -> Self {
        Self::default()
    }

    fn listen() {
        loop {
            if !Self::read().enabled {
                thread::sleep(Duration::from_millis(400));
                continue;
            }

            let child_result = Command::new("cava")
                .arg("-p")
                .arg("/dev/stdin")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn();

            let Ok(mut child) = child_result else {
                set_available(false);
                thread::sleep(Duration::from_secs(3));
                continue;
            };

            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(CAVA_CONFIG.as_bytes());
            }

            set_available(true);
            let Some(stdout) = child.stdout.take() else {
                let _ = child.kill();
                let _ = child.wait();
                set_available(false);
                continue;
            };

            let mut reader = BufReader::new(stdout);
            loop {
                if !Self::read().enabled {
                    break;
                }

                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        let bars = parse_frame(&line);
                        if !bars.is_empty() {
                            let mut state = Self::write();
                            state.bars = bars;
                        }
                    }
                }
            }

            let _ = child.kill();
            let _ = child.wait();
            set_available(false);
            if Self::read().enabled {
                // Avoid a tight restart loop if CAVA exists but cannot open an input.
                thread::sleep(Duration::from_secs(2));
            }
        }
    }
}

const CAVA_CONFIG: &str = r#"[general]
framerate = 30
bars = 32
sleep_timer = 3

[output]
method = raw
raw_target = /dev/stdout
channels = mono
data_format = ascii
ascii_max_range = 100
bar_delimiter = 59
frame_delimiter = 10
"#;

pub(super) fn set_enabled(enabled: bool) {
    let current = SpectrumState::read().enabled;
    if current != enabled {
        let mut state = SpectrumState::write();
        state.enabled = enabled;
        if !enabled {
            state.available = false;
            state.bars.clear();
        }
    }
}

pub(super) fn snapshot() -> (Vec<u8>, bool) {
    let state = SpectrumState::read();
    (state.bars.clone(), state.available)
}

fn set_available(available: bool) {
    let current = SpectrumState::read().available;
    if current != available {
        SpectrumState::write().available = available;
    }
}

fn parse_frame(line: &str) -> Vec<u8> {
    line.trim()
        .split(';')
        .filter_map(|value| value.trim().parse::<u8>().ok())
        .map(|value| value.min(100))
        .collect()
}
