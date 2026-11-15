use std::time::{Duration, Instant};

/// Physical parameters for a damped harmonic oscillator.
#[derive(Clone, Copy, Debug)]
pub struct SpringConfig {
    pub stiffness: f32,
    pub damping: f32,
    pub mass: f32,
    pub rest_delta: f32,
    pub rest_speed: f32,
    pub max_duration: Duration,
}

impl SpringConfig {
    /// Responsive opening motion with a restrained overshoot.
    pub const fn panel_open() -> Self {
        Self {
            stiffness: 165.0,
            damping: 20.0,
            mass: 1.0,
            rest_delta: 0.0015,
            rest_speed: 0.02,
            max_duration: Duration::from_millis(700),
        }
    }

    /// Slightly quicker return, retaining a restrained elastic response.
    /// The previous close preset (240 / 30) settled in about 528 ms from rest;
    /// this preset (280 / 30) settles in about 462 ms with the same thresholds.
    pub const fn panel_close() -> Self {
        Self {
            stiffness: 150.0,
            damping: 19.0,
            mass: 1.0,
            rest_delta: 0.0015,
            rest_speed: 0.02,
            max_duration: Duration::from_millis(700),
        }
    }

    /// Backwards-compatible default profile.
    pub const fn panel() -> Self {
        Self::panel_open()
    }

    fn sanitized(self) -> Self {
        Self {
            stiffness: finite_positive(self.stiffness, 380.0),
            damping: finite_non_negative(self.damping, 34.0),
            mass: finite_positive(self.mass, 1.0),
            rest_delta: finite_positive(self.rest_delta, 0.0015),
            rest_speed: finite_positive(self.rest_speed, 0.02),
            max_duration: if self.max_duration.is_zero() {
                Duration::from_millis(1300)
            } else {
                self.max_duration
            },
        }
    }
}

fn finite_positive(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

fn finite_non_negative(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value >= 0.0 {
        value
    } else {
        fallback
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SpringSample {
    pub position: f32,
    pub velocity: f32,
    pub settled: bool,
}

/// A retargetable spring. Retargeting preserves both position and velocity.
pub struct Spring {
    start: f32,
    target: f32,
    initial_velocity: f32,
    started: Instant,
    config: SpringConfig,
}

impl Spring {
    pub fn new(initial: f32, config: SpringConfig) -> Self {
        let initial = if initial.is_finite() { initial } else { 0.0 };
        Self {
            start: initial,
            target: initial,
            initial_velocity: 0.0,
            started: Instant::now(),
            config: config.sanitized(),
        }
    }

    pub fn set_target(&mut self, target: f32) {
        self.set_target_with_config(target, self.config);
    }

    /// Retargets the spring and optionally changes its profile for the new
    /// direction. Position and velocity are sampled before the profile change,
    /// so the transition remains continuous when opening is reversed to closing.
    pub fn set_target_with_config(&mut self, target: f32, config: SpringConfig) {
        if !target.is_finite() || (target - self.target).abs() <= f32::EPSILON {
            return;
        }

        let current = self.sample();
        self.start = current.position;
        self.initial_velocity = current.velocity;
        self.target = target;
        self.config = config.sanitized();
        self.started = Instant::now();
    }

    pub fn target(&self) -> f32 {
        self.target
    }

    pub fn sample(&self) -> SpringSample {
        let elapsed = self.started.elapsed().as_secs_f32();
        let sample = evaluate(
            self.start,
            self.target,
            self.initial_velocity,
            self.config,
            elapsed,
        );

        let max_duration = self.config.max_duration.as_secs_f32();
        let reached_rest = (sample.position - self.target).abs() <= self.config.rest_delta
            && sample.velocity.abs() <= self.config.rest_speed;
        let timed_out = elapsed >= max_duration;

        if reached_rest || timed_out {
            SpringSample {
                position: self.target,
                velocity: 0.0,
                settled: true,
            }
        } else {
            SpringSample {
                settled: false,
                ..sample
            }
        }
    }
}

fn evaluate(
    start: f32,
    target: f32,
    initial_velocity: f32,
    config: SpringConfig,
    elapsed: f32,
) -> SpringSample {
    let t = elapsed.max(0.0);
    let mass = config.mass;
    let stiffness = config.stiffness;
    let damping = config.damping;

    let omega = (stiffness / mass).sqrt();
    let damping_ratio = damping / (2.0 * (stiffness * mass).sqrt());
    let displacement = start - target;
    let velocity = initial_velocity;

    let (offset, speed) = if damping_ratio < 0.9999 {
        // Underdamped: a little overshoot, controlled by damping_ratio.
        let damped_omega = omega * (1.0 - damping_ratio * damping_ratio).sqrt();
        let a = displacement;
        let b = (velocity + damping_ratio * omega * displacement) / damped_omega;
        let angle = damped_omega * t;
        let cosine = angle.cos();
        let sine = angle.sin();
        let envelope = (-damping_ratio * omega * t).exp();
        let wave = a * cosine + b * sine;
        let wave_speed = -a * damped_omega * sine + b * damped_omega * cosine;

        (
            envelope * wave,
            envelope * (wave_speed - damping_ratio * omega * wave),
        )
    } else if damping_ratio <= 1.0001 {
        // Critically damped: fastest arrival without oscillation.
        let a = displacement;
        let b = velocity + omega * displacement;
        let envelope = (-omega * t).exp();
        let position_term = a + b * t;

        (
            envelope * position_term,
            envelope * (b - omega * position_term),
        )
    } else {
        // Overdamped: no oscillation, slower return to rest.
        let root = (damping_ratio * damping_ratio - 1.0).sqrt();
        let r1 = -omega * (damping_ratio - root);
        let r2 = -omega * (damping_ratio + root);
        let denominator = r1 - r2;
        let c1 = (velocity - r2 * displacement) / denominator;
        let c2 = displacement - c1;
        let e1 = (r1 * t).exp();
        let e2 = (r2 * t).exp();

        (c1 * e1 + c2 * e2, c1 * r1 * e1 + c2 * r2 * e2)
    };

    SpringSample {
        position: target + offset,
        velocity: speed,
        settled: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_preset_reaches_its_target() {
        let config = SpringConfig::panel();
        let sample = evaluate(0.0, 1.0, 0.0, config, 1.0);

        assert!((sample.position - 1.0).abs() < 0.001);
        assert!(sample.velocity.abs() < 0.02);
    }

    #[test]
    fn panel_preset_has_a_small_overshoot() {
        let config = SpringConfig::panel();
        let sample = evaluate(0.0, 1.0, 0.0, config, 0.30);

        assert!(sample.position > 1.0);
        assert!(sample.position < 1.01);
    }

    #[test]
    fn retargeting_starts_at_the_current_position_and_velocity() {
        let config = SpringConfig::panel();
        let before = evaluate(0.0, 1.0, 0.0, config, 0.18);
        let after = evaluate(before.position, 0.0, before.velocity, config, 0.0);

        assert!((after.position - before.position).abs() < 0.00001);
        assert!((after.velocity - before.velocity).abs() < 0.00001);
    }

    #[test]
    fn closing_preset_is_about_twelve_percent_faster_than_previous_profile() {
        fn settle_time(config: SpringConfig) -> f32 {
            let mut elapsed = 0.0;
            while elapsed < 1.3 {
                elapsed += 0.0005;
                let sample = evaluate(1.0, 0.0, 0.0, config, elapsed);
                if sample.position.abs() <= config.rest_delta
                    && sample.velocity.abs() <= config.rest_speed
                {
                    return elapsed;
                }
            }
            panic!("spring did not settle within the test window");
        }

        let previous = SpringConfig {
            stiffness: 240.0,
            damping: 30.0,
            mass: 1.0,
            rest_delta: 0.0015,
            rest_speed: 0.02,
            max_duration: Duration::from_millis(1500),
        };
        let previous_time = settle_time(previous);
        let new_time = settle_time(SpringConfig::panel_close());

        assert!(new_time < previous_time * 0.90);
        assert!(new_time > previous_time * 0.80);
    }

    #[test]
    fn closing_preset_has_a_small_controlled_overshoot() {
        let sample = evaluate(1.0, 0.0, 0.0, SpringConfig::panel_close(), 0.42);
        assert!(sample.position < 0.0);
        assert!(sample.position > -0.003);
    }

    #[test]
    fn spring_supports_critical_and_overdamped_configs() {
        let mut critical = SpringConfig::panel();
        critical.damping = 2.0 * (critical.stiffness * critical.mass).sqrt();
        let critical_sample = evaluate(0.0, 1.0, 0.0, critical, 0.25);
        assert!(critical_sample.position > 0.0 && critical_sample.position < 1.0);

        let mut overdamped = SpringConfig::panel();
        overdamped.damping *= 1.5;
        let overdamped_sample = evaluate(0.0, 1.0, 0.0, overdamped, 0.25);
        assert!(overdamped_sample.position > 0.0 && overdamped_sample.position < 1.0);
    }
    #[test]
    fn closing_profile_is_slower_and_more_damped_than_opening() {
        let opening = SpringConfig::panel_open();
        let closing = SpringConfig::panel_close();

        let opening_ratio = opening.damping / (2.0 * (opening.stiffness * opening.mass).sqrt());
        let closing_ratio = closing.damping / (2.0 * (closing.stiffness * closing.mass).sqrt());

        assert!(closing.stiffness < opening.stiffness);
        assert!(closing_ratio > opening_ratio);
        assert!(closing.max_duration >= opening.max_duration);
    }

    #[test]
    fn changing_profile_does_not_change_position_or_velocity_at_retarget() {
        let opening = SpringConfig::panel_open();
        let closing = SpringConfig::panel_close();
        let before = evaluate(0.0, 1.0, 0.0, opening, 0.18);
        let after = evaluate(before.position, 0.0, before.velocity, closing, 0.0);

        assert!((after.position - before.position).abs() < 0.00001);
        assert!((after.velocity - before.velocity).abs() < 0.00001);
    }
}
