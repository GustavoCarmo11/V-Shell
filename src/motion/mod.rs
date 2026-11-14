mod deformation;
mod presets;
mod spring;

use std::collections::HashMap;
use std::sync::Mutex;

use amane::{Rectangle, Service, Transform, request_frame};

use self::deformation::BlobDeformer;
use self::presets::{LAUNCHER_CLOSE, LAUNCHER_OPEN, PANEL_CLOSE, PANEL_OPEN};
use self::spring::Spring;

/// Stable identifiers for independent shell-panel transitions.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PanelId {
    Dashboard,
    Launcher,
    QuickSettings,
    Media,
    Wifi,
    Bluetooth,
    Audio,
}

impl PanelId {
    const ALL: [Self; 7] = [
        Self::Dashboard,
        Self::Launcher,
        Self::QuickSettings,
        Self::Media,
        Self::Wifi,
        Self::Bluetooth,
        Self::Audio,
    ];
}

/// Coherent sample shared by panel layout, hit testing and the surface shader.
#[derive(Clone, Copy, Debug)]
pub struct PanelMotionSample {
    /// Animated position from closed (0) to open (1).
    pub progress: f32,
    /// Derivative of progress, in normalized units per second.
    pub velocity: f32,
    /// Center-relative affine matrix used by the panel widget and its SDF.
    pub deformation: Transform,
    /// Destination state; it does not imply that the spring has settled.
    pub is_open: bool,
    pub settled: bool,
}

struct PanelState {
    motion: Spring,
    deformer: BlobDeformer,
}

impl PanelState {
    fn new() -> Self {
        Self {
            motion: Spring::new(0.0, PANEL_OPEN),
            deformer: BlobDeformer::new(),
        }
    }
}

struct MotionState {
    // Reading the service registers the view dependency. The mutex allows a
    // frame sample to advance deformation physics without writing the Service
    // and recursively invalidating the window on every frame.
    inner: Mutex<HashMap<PanelId, PanelState>>,
}

impl Service for MotionState {
    fn new() -> Self {
        let panels = PanelId::ALL
            .into_iter()
            .map(|panel| (panel, PanelState::new()))
            .collect();

        Self {
            inner: Mutex::new(panels),
        }
    }

    // The Amane frame clock drives active panel motions; no polling thread.
    fn listen() {}
}

/// Shared panel motion and BlobRect-style deformation engine.
pub struct Motion;

impl Motion {
    /// Binds a complete module surface to the shared hover lifecycle.
    /// Entering opens the panel; leaving closes it. Use this on the outermost
    /// Rectangle of every module so future panels behave consistently.
    pub fn panel_hover(panel: PanelId, widget: Rectangle) -> Rectangle {
        Self::panel_hover_with_exit(panel, widget, || {})
    }

    /// Same panel lifecycle, with a cleanup callback when the pointer leaves.
    /// Use this for modules that own a focused text field or other transient input.
    pub fn panel_hover_with_exit(
        panel: PanelId,
        widget: Rectangle,
        on_exit: impl Fn() + 'static,
    ) -> Rectangle {
        widget.on_hover(move |inside| {
            Self::set_open(panel, inside);
            if !inside {
                on_exit();
            }
        })
    }

    /// Binds a rail trigger to a panel without closing it when the pointer
    /// leaves the icon on its way into the panel.
    pub fn trigger_hover(panel: PanelId, widget: Rectangle) -> Rectangle {
        widget.on_hover(move |inside| {
            if inside {
                Self::open(panel);
            }
        })
    }

    pub fn open(panel: PanelId) {
        Self::set_open(panel, true);
    }

    pub fn close(panel: PanelId) {
        Self::set_open(panel, false);
    }

    pub fn set_open(panel: PanelId, open: bool) {
        let state = MotionState::write();
        let mut inner = state
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let panel_state = inner
            .get_mut(&panel)
            .expect("PanelId must be registered in MotionState::new");

        let (target, config) = match (panel, open) {
            (PanelId::Launcher, true) => (1.0, LAUNCHER_OPEN),
            (PanelId::Launcher, false) => (0.0, LAUNCHER_CLOSE),
            (_, true) => (1.0, PANEL_OPEN),
            (_, false) => (0.0, PANEL_CLOSE),
        };

        // Ignore duplicate hover notifications. A real reversal still retargets
        // the spring and preserves its current position and velocity.
        if (panel_state.motion.target() - target).abs() <= f32::EPSILON {
            return;
        }

        // Spring::set_target_with_config samples the current position and
        // velocity, preserving both if the pointer reverses the transition.
        panel_state.motion.set_target_with_config(target, config);
    }

    /// Samples a panel and updates its deformation from the panel's path.
    /// `travel_per_progress` is the panel's scene-space displacement in pixels
    /// for progress 0 -> 1. For a vertically opening panel, use (0, travel_y).
    pub fn sample(panel: PanelId, travel_per_progress: (f32, f32)) -> PanelMotionSample {
        let state = MotionState::read();
        let mut inner = state
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let panel_state = inner
            .get_mut(&panel)
            .expect("PanelId must be registered in MotionState::new");

        let movement = panel_state.motion.sample();
        let velocity_x = movement.velocity * finite_or_zero(travel_per_progress.0);
        let velocity_y = movement.velocity * finite_or_zero(travel_per_progress.1);
        let deformation_active = panel_state.deformer.update(velocity_x, velocity_y);
        let deformation = panel_state.deformer.matrix().as_transform();
        let target_is_open = panel_state.motion.target() >= 0.5;
        let settled = movement.settled && !deformation_active;

        if !settled {
            request_frame();
        }

        PanelMotionSample {
            progress: movement.position,
            velocity: movement.velocity,
            deformation,
            is_open: target_is_open,
            settled,
        }
    }

    pub fn progress(panel: PanelId) -> f32 {
        Self::sample(panel, (0.0, 0.0)).progress
    }

    pub fn is_open(panel: PanelId) -> bool {
        let state = MotionState::read();
        let inner = state
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        inner
            .get(&panel)
            .expect("PanelId must be registered in MotionState::new")
            .motion
            .target()
            >= 0.5
    }
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}
