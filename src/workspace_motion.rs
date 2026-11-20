use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use amane::{Service, request_frame};

// Previous Waybar CSS:
// transition: all 0.3s cubic-bezier(.55, -.04, .3, 1.44);
// Each workspace button now morphs independently with this same timing curve.
const WORKSPACE_DURATION: Duration = Duration::from_millis(300);
const BEZIER_X1: f32 = 0.55;
const BEZIER_Y1: f32 = -0.04;
const BEZIER_X2: f32 = 0.30;
const BEZIER_Y2: f32 = 1.44;

// The special-workspace transition remains a restrained physical spring.
const SPECIAL_OMEGA: f32 = 17.0;
const SPECIAL_DAMPING: f32 = 1.0;
const SPECIAL_REST_DELTA: f32 = 0.002;
const SPECIAL_REST_SPEED: f32 = 0.02;
const MAX_DURATION: Duration = Duration::from_millis(850);
const SPECIAL_PULSE_DURATION: Duration = Duration::from_millis(520);
const GROUP_HEIGHT_OMEGA: f32 = 13.0;
const GROUP_HEIGHT_REST_DELTA: f32 = 0.12;
const GROUP_HEIGHT_REST_SPEED: f32 = 0.20;

#[derive(Clone, Copy, Debug)]
pub struct WorkspaceMotionSample {
    /// Crossfade between regular workspaces (0) and the special workspace (1).
    pub special_progress: f32,
    /// A short scale pulse used when the special workspace appears.
    pub special_scale: f32,
}

#[derive(Clone, Copy)]
struct ScalarSample {
    position: f32,
    velocity: f32,
    settled: bool,
}

/// A CSS-compatible per-workspace style transition.
///
/// The old active button shrinks to a circle while the new active button grows
/// into a capsule. Re-targeting starts from the currently rendered progress,
/// so rapid workspace changes do not snap back to either endpoint.
struct BezierTrack {
    start: f32,
    target: f32,
    started: Instant,
}

impl BezierTrack {
    fn resting(value: f32) -> Self {
        Self {
            start: value,
            target: value,
            started: Instant::now(),
        }
    }

    fn retarget(&mut self, target: f32) -> bool {
        if !target.is_finite() || (target - self.target).abs() < 0.001 {
            return false;
        }

        self.start = self.sample().position;
        self.target = target;
        self.started = Instant::now();
        true
    }

    fn sample(&self) -> ScalarSample {
        if (self.target - self.start).abs() < 0.001 {
            return ScalarSample {
                position: self.target,
                velocity: 0.0,
                settled: true,
            };
        }

        let elapsed = self.started.elapsed();
        let duration = WORKSPACE_DURATION.as_secs_f32();
        let progress = (elapsed.as_secs_f32() / duration).clamp(0.0, 1.0);

        if progress >= 1.0 {
            return ScalarSample {
                position: self.target,
                velocity: 0.0,
                settled: true,
            };
        }

        let (eased, slope) = css_cubic_bezier(progress);
        let delta = self.target - self.start;

        ScalarSample {
            position: self.start + delta * eased,
            velocity: delta * slope / duration,
            settled: false,
        }
    }
}

fn bezier_component(t: f32, p1: f32, p2: f32) -> f32 {
    let inverse = 1.0 - t;
    3.0 * inverse * inverse * t * p1 + 3.0 * inverse * t * t * p2 + t * t * t
}

fn bezier_derivative(t: f32, p1: f32, p2: f32) -> f32 {
    let inverse = 1.0 - t;
    3.0 * inverse * inverse * p1 + 6.0 * inverse * t * (p2 - p1) + 3.0 * t * t * (1.0 - p2)
}

/// Returns eased progress and dy/dx for cubic-bezier(.55, -.04, .3, 1.44).
/// Bisection solves x(t) because the CSS curve is parameterized by x, not t.
fn css_cubic_bezier(progress: f32) -> (f32, f32) {
    let mut low = 0.0;
    let mut high = 1.0;

    for _ in 0..18 {
        let middle = (low + high) * 0.5;
        let x = bezier_component(middle, BEZIER_X1, BEZIER_X2);
        if x < progress {
            low = middle;
        } else {
            high = middle;
        }
    }

    let t = (low + high) * 0.5;
    let y = bezier_component(t, BEZIER_Y1, BEZIER_Y2);
    let dx = bezier_derivative(t, BEZIER_X1, BEZIER_X2);
    let dy = bezier_derivative(t, BEZIER_Y1, BEZIER_Y2);
    let slope = if dx.abs() > 0.0001 { dy / dx } else { 0.0 };

    (y, slope)
}

struct SpringTrack {
    start: f32,
    target: f32,
    initial_velocity: f32,
    started: Instant,
    omega: f32,
    damping: f32,
    rest_delta: f32,
    rest_speed: f32,
}

impl SpringTrack {
    fn resting(value: f32, omega: f32, damping: f32, rest_delta: f32, rest_speed: f32) -> Self {
        Self {
            start: value,
            target: value,
            initial_velocity: 0.0,
            started: Instant::now(),
            omega,
            damping,
            rest_delta,
            rest_speed,
        }
    }

    fn retarget(&mut self, target: f32) -> bool {
        if !target.is_finite() || (target - self.target).abs() < 0.001 {
            return false;
        }

        let current = self.sample();
        self.start = current.position;
        self.initial_velocity = current.velocity;
        self.target = target;
        self.started = Instant::now();
        true
    }

    fn sample(&self) -> ScalarSample {
        let elapsed = self.started.elapsed();
        let t = elapsed.as_secs_f32();
        let displacement = self.start - self.target;
        let alpha = self.damping * self.omega;

        let (position, velocity) = if self.damping < 0.999 {
            let damped_frequency = self.omega * (1.0 - self.damping * self.damping).sqrt();
            let b = (self.initial_velocity + alpha * displacement) / damped_frequency;
            let envelope = (-alpha * t).exp();
            let cosine = (damped_frequency * t).cos();
            let sine = (damped_frequency * t).sin();
            let wave = displacement * cosine + b * sine;
            let position = self.target + envelope * wave;
            let velocity = envelope
                * (-alpha * wave - displacement * damped_frequency * sine
                    + b * damped_frequency * cosine);
            (position, velocity)
        } else {
            let b = self.initial_velocity + self.omega * displacement;
            let envelope = (-self.omega * t).exp();
            let wave = displacement + b * t;
            let position = self.target + envelope * wave;
            let velocity = envelope * (b - self.omega * wave);
            (position, velocity)
        };

        let settled =
            (position - self.target).abs() <= self.rest_delta && velocity.abs() <= self.rest_speed;

        if settled || elapsed >= MAX_DURATION {
            ScalarSample {
                position: self.target,
                velocity: 0.0,
                settled: true,
            }
        } else {
            ScalarSample {
                position,
                velocity,
                settled: false,
            }
        }
    }
}

struct WorkspaceTransition {
    group_height: SpringTrack,
    workspaces: HashMap<usize, BezierTrack>,
    special: SpringTrack,
    special_active: bool,
    special_started: Option<Instant>,
}

impl WorkspaceTransition {
    fn sample_group_height(&self) -> (f32, bool) {
        let sample = self.group_height.sample();
        (sample.position, !sample.settled)
    }

    fn new(
        active_index: Option<usize>,
        workspace_count: usize,
        special_active: bool,
        group_height: f32,
    ) -> Self {
        let workspaces = (0..workspace_count)
            .map(|index| {
                let progress = if !special_active && active_index == Some(index) {
                    1.0
                } else {
                    0.0
                };

                (index, BezierTrack::resting(progress))
            })
            .collect();

        Self {
            workspaces,
            special: SpringTrack::resting(
                if special_active { 1.0 } else { 0.0 },
                SPECIAL_OMEGA,
                SPECIAL_DAMPING,
                SPECIAL_REST_DELTA,
                SPECIAL_REST_SPEED,
            ),
            group_height: SpringTrack::resting(
                group_height,
                GROUP_HEIGHT_OMEGA,
                1.0,
                GROUP_HEIGHT_REST_DELTA,
                GROUP_HEIGHT_REST_SPEED,
            ),
            special_active,
            special_started: None,
        }
    }

    fn retarget(
        &mut self,
        active_index: Option<usize>,
        workspace_count: usize,
        special_active: bool,
        group_height: f32,
    ) -> bool {
        let mut changed = false;

        self.workspaces.retain(|index, _| *index < workspace_count);

        for index in 0..workspace_count {
            let target = if !special_active && active_index == Some(index) {
                1.0
            } else {
                0.0
            };

            if let Some(track) = self.workspaces.get_mut(&index) {
                changed |= track.retarget(target);
            } else {
                self.workspaces.insert(index, BezierTrack::resting(target));
            }
        }

        if self.special_active != special_active {
            self.special_active = special_active;
            changed |= self
                .special
                .retarget(if special_active { 1.0 } else { 0.0 });
            self.special_started = if special_active {
                Some(Instant::now())
            } else {
                None
            };
        }
        // Não reduz a altura ao entrar no special workspace.
        if !special_active {
            changed |= self.group_height.retarget(group_height);
        }
        changed
    }

    fn workspace_progress(&self, index: usize) -> (f32, bool) {
        let Some(track) = self.workspaces.get(&index) else {
            return (0.0, false);
        };
        let sample = track.sample();
        (sample.position, !sample.settled)
    }

    fn sample(&self) -> (WorkspaceMotionSample, bool) {
        let special = self.special.sample();

        let (special_scale, pulse_active) = if let Some(started) = self.special_started {
            let elapsed = started.elapsed();
            if elapsed < SPECIAL_PULSE_DURATION {
                let t = elapsed.as_secs_f32();
                let pulse = 1.0 + 0.105 * (-13.0 * t).exp() * (18.0 * t).sin();
                (pulse, true)
            } else {
                (1.0, false)
            }
        } else {
            (1.0, false)
        };

        (
            WorkspaceMotionSample {
                special_progress: special.position.clamp(0.0, 1.0),
                special_scale,
            },
            !special.settled || pulse_active,
        )
    }
}

#[derive(Default)]
struct WorkspaceMotionService {
    outputs: Mutex<HashMap<String, WorkspaceTransition>>,
}

impl Service for WorkspaceMotionService {
    fn new() -> Self {
        Self::default()
    }

    fn listen() {}
}

/// Drives independent CSS-style workspace transitions for every monitor.
pub struct WorkspaceMotion;

impl WorkspaceMotion {
    pub fn set_target(
        output: &str,
        active_index: Option<usize>,
        workspace_count: usize,
        special_active: bool,
        group_height: f32,
    ) {
        let service = WorkspaceMotionService::read();
        let mut outputs = service
            .outputs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let changed = if let Some(transition) = outputs.get_mut(output) {
            transition.retarget(active_index, workspace_count, special_active, group_height)
        } else {
            outputs.insert(
                output.to_owned(),
                WorkspaceTransition::new(
                    active_index,
                    workspace_count,
                    special_active,
                    group_height,
                ),
            );
            false
        };

        drop(outputs);
        drop(service);

        if changed {
            request_frame();
        }
    }

    /// Returns the current animated height of the workspace group.
    ///
    /// The fallback is used before this output has received its first target.
    pub fn group_height(output: &str, fallback: f32) -> f32 {
        let service = WorkspaceMotionService::read();
        let outputs = service
            .outputs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let Some(transition) = outputs.get(output) else {
            return fallback;
        };

        let (height, animating) = transition.sample_group_height();

        drop(outputs);
        drop(service);

        if animating {
            request_frame();
        }

        height
    }

    /// Returns the current active-state progress for a workspace (0..1, with
    /// the CSS curve's small overshoot retained for the shape interpolation).
    pub fn workspace_progress(output: &str, index: usize) -> f32 {
        let service = WorkspaceMotionService::read();
        let outputs = service
            .outputs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let Some(transition) = outputs.get(output) else {
            return 0.0;
        };

        let (progress, animating) = transition.workspace_progress(index);
        drop(outputs);
        drop(service);

        if animating {
            request_frame();
        }

        progress
    }

    pub fn sample(output: &str) -> WorkspaceMotionSample {
        let service = WorkspaceMotionService::read();
        let outputs = service
            .outputs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let Some(transition) = outputs.get(output) else {
            return WorkspaceMotionSample {
                special_progress: 0.0,
                special_scale: 1.0,
            };
        };

        let (sample, animating) = transition.sample();
        drop(outputs);
        drop(service);

        if animating {
            request_frame();
        }

        sample
    }
}
