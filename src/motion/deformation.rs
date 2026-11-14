use std::time::Instant;

use amane::Transform;

// Reduced visual profile based on Caelestia.Blobs' BlobRect physics.
const STIFFNESS: f32 = 200.0;
const DAMPING: f32 = 16.0;
const DEFORM_SCALE: f32 = 0.00006;
const MAX_STRETCH: f32 = 0.05;
const MIN_SPEED: f32 = 10.0;
const REST_EPSILON: f32 = 0.002;
const MIN_FRAME_DELTA: f32 = 0.001;
const MAX_FRAME_DELTA: f32 = 0.1;

/// Symmetric 2D deformation used by the widget transform and the SDF.
/// The matrix is applied around the widget's center.
#[derive(Clone, Copy, Debug)]
pub struct DeformationMatrix {
    pub m00: f32,
    pub m01: f32,
    pub m11: f32,
}

impl DeformationMatrix {
    pub const IDENTITY: Self = Self {
        m00: 1.0,
        m01: 0.0,
        m11: 1.0,
    };

    /// Builds the linear part of the centered affine transform.
    pub fn as_transform(self) -> Transform {
        Transform::from_row(self.m00, self.m01, self.m01, self.m11, 0.0, 0.0)
    }

    pub fn inverse_point(self, x: f32, y: f32) -> (f32, f32) {
        let determinant = self.m00 * self.m11 - self.m01 * self.m01;
        if !determinant.is_finite() || determinant.abs() < 0.0001 {
            return (x, y);
        }

        let inverse_x = (self.m11 * x - self.m01 * y) / determinant;
        let inverse_y = (-self.m01 * x + self.m00 * y) / determinant;
        (inverse_x, inverse_y)
    }
}

/// BlobRect-compatible velocity-driven deformation.
///
/// Uses the same defaults, stretch target and implicit damping integration as
/// Caelestia.Blobs::BlobRect::updatePhysics().
pub struct BlobDeformer {
    matrix: DeformationMatrix,
    velocity: DeformationMatrix,
    last_update: Instant,
    active: bool,
}

impl BlobDeformer {
    pub fn new() -> Self {
        Self {
            matrix: DeformationMatrix::IDENTITY,
            velocity: DeformationMatrix {
                m00: 0.0,
                m01: 0.0,
                m11: 0.0,
            },
            last_update: Instant::now(),
            active: false,
        }
    }

    /// Updates the deformation target from scene-space panel velocity in px/s.
    pub fn update(&mut self, velocity_x: f32, velocity_y: f32) -> bool {
        let now = Instant::now();
        let dt = now.duration_since(self.last_update).as_secs_f32();
        self.last_update = now;

        let velocity_x = finite_or_zero(velocity_x);
        let velocity_y = finite_or_zero(velocity_y);
        let speed = (velocity_x * velocity_x + velocity_y * velocity_y).sqrt();

        // Match BlobRect: skip invalid frame intervals instead of clamping or
        // subdividing them. A delayed frame must not inject extra spring energy.
        if !(MIN_FRAME_DELTA..=MAX_FRAME_DELTA).contains(&dt) {
            if self.active {
                self.snap_if_resting(0.0);
            }
            return self.active;
        }

        if !self.active {
            if speed < MIN_SPEED {
                return false;
            }
            self.active = true;
        }

        let target = target_from_velocity(velocity_x, velocity_y);

        integrate_component(&mut self.matrix.m00, &mut self.velocity.m00, target.m00, dt);
        integrate_component(&mut self.matrix.m01, &mut self.velocity.m01, target.m01, dt);
        integrate_component(&mut self.matrix.m11, &mut self.velocity.m11, target.m11, dt);

        self.snap_if_resting(speed);
        self.active
    }

    pub fn matrix(&self) -> DeformationMatrix {
        self.matrix
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    fn snap_if_resting(&mut self, speed: f32) {
        let at_rest = (self.matrix.m00 - 1.0).abs() < REST_EPSILON
            && self.matrix.m01.abs() < REST_EPSILON
            && (self.matrix.m11 - 1.0).abs() < REST_EPSILON
            && self.velocity.m00.abs() < REST_EPSILON
            && self.velocity.m01.abs() < REST_EPSILON
            && self.velocity.m11.abs() < REST_EPSILON
            && speed < MIN_SPEED;

        if at_rest {
            self.matrix = DeformationMatrix::IDENTITY;
            self.velocity = DeformationMatrix {
                m00: 0.0,
                m01: 0.0,
                m11: 0.0,
            };
            self.active = false;
        }
    }
}

impl Default for BlobDeformer {
    fn default() -> Self {
        Self::new()
    }
}

fn target_from_velocity(velocity_x: f32, velocity_y: f32) -> DeformationMatrix {
    let speed = (velocity_x * velocity_x + velocity_y * velocity_y).sqrt();
    if !speed.is_finite() || speed <= MIN_SPEED {
        return DeformationMatrix::IDENTITY;
    }

    // Exact BlobRect target: R(theta) * diag(stretch, compress) * R(theta)^T.
    let target_stretch = 1.0 + (speed * DEFORM_SCALE).min(MAX_STRETCH);
    let target_compress = 1.0 / target_stretch;
    let cos_a = velocity_x / speed;
    let sin_a = velocity_y / speed;
    let cos2 = cos_a * cos_a;
    let sin2 = sin_a * sin_a;
    let cos_sin = cos_a * sin_a;

    DeformationMatrix {
        m00: target_stretch * cos2 + target_compress * sin2,
        m01: (target_stretch - target_compress) * cos_sin,
        m11: target_stretch * sin2 + target_compress * cos2,
    }
}

fn integrate_component(value: &mut f32, velocity: &mut f32, target: f32, dt: f32) {
    // Caelestia's implicit damping step. Unlike explicit Euler, this keeps the
    // friction multiplier positive even when a frame delta is relatively long.
    let inverse_damping = 1.0 / (1.0 + DAMPING * dt);
    *velocity = (*velocity - STIFFNESS * (*value - target) * dt) * inverse_damping;
    *value += *velocity * dt;
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_motion_stretches_along_y_and_compresses_x() {
        let target = target_from_velocity(0.0, -1200.0);
        assert!(target.m00 < 1.0);
        assert!(target.m11 > 1.0);
        assert!(target.m01.abs() < 0.000001);
    }

    #[test]
    fn diagonal_motion_produces_symmetric_shear() {
        let target = target_from_velocity(900.0, 900.0);
        assert!(target.m01 > 0.0);
        assert!((target.m00 - target.m11).abs() < 0.000001);
    }

    #[test]
    fn deformation_preserves_area() {
        for (vx, vy) in [(0.0, 900.0), (900.0, 0.0), (900.0, -500.0)] {
            let target = target_from_velocity(vx, vy);
            let determinant = target.m00 * target.m11 - target.m01 * target.m01;
            assert!((determinant - 1.0).abs() < 0.0001);
        }
    }

    #[test]
    fn inverse_point_round_trips_a_deformed_point() {
        let matrix = target_from_velocity(0.0, 1100.0);
        let original = (23.0, -41.0);
        let deformed = (
            matrix.m00 * original.0 + matrix.m01 * original.1,
            matrix.m01 * original.0 + matrix.m11 * original.1,
        );
        let restored = matrix.inverse_point(deformed.0, deformed.1);
        assert!((restored.0 - original.0).abs() < 0.0001);
        assert!((restored.1 - original.1).abs() < 0.0001);
    }

    #[test]
    fn implicit_damping_does_not_flip_velocity_on_a_long_frame() {
        let mut value = 1.0;
        let mut velocity = 0.0;

        integrate_component(&mut value, &mut velocity, 0.0, 0.08);

        assert!(velocity < 0.0);
        assert!(value > 0.0);
        assert!(value < 1.0);
        assert!(value.is_finite() && velocity.is_finite());
    }

    #[test]
    fn restrained_deformation_profile_is_used() {
        assert_eq!(STIFFNESS, 200.0);
        assert_eq!(DAMPING, 16.0);
        assert_eq!(DEFORM_SCALE, 0.00006);
        assert_eq!(MAX_STRETCH, 0.05);
        assert_eq!(MIN_SPEED, 10.0);
    }
    #[test]
    fn reduced_profile_caps_vertical_stretch_at_five_percent() {
        let target = target_from_velocity(0.0, -2000.0);
        assert!(target.m11 <= 1.05 + f32::EPSILON);
        assert!(target.m11 > 1.0);
        assert!(target.m00 < 1.0);
    }
}
