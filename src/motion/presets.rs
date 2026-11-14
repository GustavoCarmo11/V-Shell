use std::time::Duration;

use super::spring::SpringConfig;

/// Shared opening profile: responsive, with a restrained overshoot.
pub const PANEL_OPEN: SpringConfig = SpringConfig::panel_open();

/// Quicker closing profile with controlled damping.
pub const PANEL_CLOSE: SpringConfig = SpringConfig::panel_close();

/// Launcher opens a little more deliberately than the Dashboard.
pub const LAUNCHER_OPEN: SpringConfig = SpringConfig {
    stiffness: 260.0,
    damping: 31.0,
    mass: 1.0,
    rest_delta: 0.0015,
    rest_speed: 0.02,
    max_duration: Duration::from_millis(1500),
};

/// Launcher also closes more gently, without changing other panels.
pub const LAUNCHER_CLOSE: SpringConfig = SpringConfig {
    stiffness: 240.0,
    damping: 30.0,
    mass: 1.0,
    rest_delta: 0.0015,
    rest_speed: 0.02,
    max_duration: Duration::from_millis(1500),
};
