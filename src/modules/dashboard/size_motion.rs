use std::sync::Mutex;
use std::time::{Duration, Instant};

use amane::{Service, request_frame};

// Critically damped response: the panel eases into its new bounds without
// overshooting or changing the Dashboard's open/close spring in motion/presets.rs.
const OMEGA: f32 = 20.0;
const REST_DELTA: f32 = 0.4;
const REST_SPEED: f32 = 2.0;
const MAX_DURATION: Duration = Duration::from_millis(750);

#[derive(Clone, Copy)]
struct SpringSample {
    position: f32,
    velocity: f32,
    settled: bool,
}

struct DimensionSpring {
    start: f32,
    target: f32,
    initial_velocity: f32,
    started: Instant,
}

impl DimensionSpring {
    fn new(value: f32) -> Self {
        Self {
            start: value,
            target: value,
            initial_velocity: 0.0,
            started: Instant::now(),
        }
    }

    fn target(&self) -> f32 {
        self.target
    }

    /// Retarget from the currently sampled position and velocity so rapidly
    /// switching tabs cannot cause a sudden jump or reset the active motion.
    fn set_target(&mut self, target: f32) -> bool {
        if !target.is_finite() || (target - self.target).abs() <= 0.5 {
            return false;
        }

        let current = self.sample();
        self.start = current.position;
        self.initial_velocity = current.velocity;
        self.target = target;
        self.started = Instant::now();
        true
    }

    fn sample(&self) -> SpringSample {
        let elapsed = self.started.elapsed();
        let t = elapsed.as_secs_f32();
        let displacement = self.start - self.target;
        let b = self.initial_velocity + OMEGA * displacement;
        let envelope = (-OMEGA * t).exp();
        let position_term = displacement + b * t;
        let position = self.target + envelope * position_term;
        let velocity = envelope * (b - OMEGA * position_term);

        let reached_rest =
            (position - self.target).abs() <= REST_DELTA && velocity.abs() <= REST_SPEED;
        let timed_out = elapsed >= MAX_DURATION;

        if reached_rest || timed_out {
            SpringSample {
                position: self.target,
                velocity: 0.0,
                settled: true,
            }
        } else {
            SpringSample {
                position,
                velocity,
                settled: false,
            }
        }
    }
}

struct DashboardDimensionsMotion {
    width: DimensionSpring,
    height: DimensionSpring,
}

struct DashboardSizeService {
    // Initialized on the first layout pass, so startup does not animate from
    // an arbitrary placeholder size. Later tab changes are spring-animated.
    inner: Mutex<Option<DashboardDimensionsMotion>>,
}

impl Service for DashboardSizeService {
    fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    fn listen() {}
}

pub fn set_target(width: f32, height: f32) {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return;
    }

    let state = DashboardSizeService::read();
    let mut inner = state
        .inner
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let changed = if let Some(dimensions) = inner.as_mut() {
        let width_changed = (dimensions.width.target() - width).abs() > 0.5;
        let height_changed = (dimensions.height.target() - height).abs() > 0.5;

        let width_retargeted = width_changed && dimensions.width.set_target(width);
        let height_retargeted = height_changed && dimensions.height.set_target(height);
        width_retargeted || height_retargeted
    } else {
        *inner = Some(DashboardDimensionsMotion {
            width: DimensionSpring::new(width),
            height: DimensionSpring::new(height),
        });
        false
    };

    drop(inner);
    drop(state);

    if changed {
        request_frame();
    }
}

pub fn sample() -> (f32, f32) {
    let state = DashboardSizeService::read();
    let inner = state
        .inner
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let Some(dimensions) = inner.as_ref() else {
        return (0.0, 0.0);
    };

    let width = dimensions.width.sample();
    let height = dimensions.height.sample();
    let settled = width.settled && height.settled;
    let result = (width.position, height.position);

    drop(inner);
    drop(state);

    if !settled {
        request_frame();
    }

    result
}
