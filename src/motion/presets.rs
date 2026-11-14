use std::time::Duration;

use super::spring::SpringConfig;

/// Shared opening profile: responsive, with a restrained overshoot.
pub const PANEL_OPEN: SpringConfig = SpringConfig::panel_open();

/// Quicker closing profile with controlled damping.
pub const PANEL_CLOSE: SpringConfig = SpringConfig::panel_close();

/// Launcher opening is about 25% slower than the previous Launcher profile.
pub const LAUNCHER_OPEN: SpringConfig = SpringConfig {
    stiffness: 165.0,
    damping: 25.0,
    mass: 1.0,
    rest_delta: 0.0015,
    rest_speed: 0.02,
    max_duration: Duration::from_millis(500),
};

/// Launcher closing is about 25% slower than the previous Launcher profile.
pub const LAUNCHER_CLOSE: SpringConfig = SpringConfig {
    stiffness: 150.0,
    damping: 24.0,
    mass: 1.0,
    rest_delta: 0.0015,
    rest_speed: 0.02,
    max_duration: Duration::from_millis(400),
};
