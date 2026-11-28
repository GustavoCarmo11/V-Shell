mod deformation;
mod presets;
mod spring;

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use amane::{Rectangle, Service, Transform, request_frame};

use self::deformation::BlobDeformer;
use self::presets::{LAUNCHER_CLOSE, LAUNCHER_OPEN, PANEL_CLOSE, PANEL_OPEN, STATUS_CLOSE};
use self::spring::Spring;

/// Delay that bridges gaps between the rail trigger and a panel's hit region.
/// The layer-shell surface may emit pointer-leave while the cursor crosses a
/// non-interactive part of the screen. A short grace period avoids closing
/// the panel before the pointer can enter it.
pub const PANEL_HOVER_LEAVE_GRACE: Duration = Duration::from_millis(180);
pub const TRIGGER_HOVER_LEAVE_GRACE: Duration = Duration::from_millis(600);

/// Stable identifiers for independent shell-panel transitions.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PanelId {
    Dashboard,
    Launcher,
    QuickSettings,
    Media,
}

impl PanelId {
    const ALL: [Self; 4] = [
        Self::Dashboard,
        Self::Launcher,
        Self::QuickSettings,
        Self::Media,
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
    close_deadline: Option<Instant>,
}

impl PanelState {
    fn new() -> Self {
        Self {
            motion: Spring::new(0.0, PANEL_OPEN),
            deformer: BlobDeformer::new(),
            close_deadline: None,
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
    /// Entering cancels a pending close; leaving starts a short grace period.
    pub fn panel_hover(panel: PanelId, widget: Rectangle) -> Rectangle {
        Self::panel_hover_with_exit(panel, widget, || {})
    }

    /// Same panel lifecycle, with cleanup when the pointer leaves the panel.
    /// The close is delayed so the cursor can cross a gap into another target.
    pub fn panel_hover_with_exit(
        panel: PanelId,
        widget: Rectangle,
        on_exit: impl Fn() + 'static,
    ) -> Rectangle {
        widget.on_hover(move |inside| {
            if inside {
                Self::open(panel);
            } else {
                Self::schedule_close(panel, PANEL_HOVER_LEAVE_GRACE);
                on_exit();
            }
        })
    }

    /// Binds a hover trigger that opens a panel and leaves dismissal to an
    /// explicit action, such as clicking outside the Launcher.
    pub fn trigger_hover_latched(panel: PanelId, widget: Rectangle) -> Rectangle {
        widget.on_hover(move |inside| {
            if inside {
                Self::open(panel);
            }
        })
    }

    /// Binds a rail trigger to a panel. Leaving the trigger schedules a close,
    /// but does not close immediately; entering the panel cancels that close.
    pub fn trigger_hover(panel: PanelId, widget: Rectangle) -> Rectangle {
        widget.on_hover(move |inside| {
            if inside {
                Self::open(panel);
            } else {
                Self::schedule_close(panel, TRIGGER_HOVER_LEAVE_GRACE);
            }
        })
    }

    pub fn open(panel: PanelId) {
        Self::set_open(panel, true);
    }

    /// Explicit close for clicks/actions. Unlike hover exit, this is immediate.
    pub fn close(panel: PanelId) {
        Self::set_open(panel, false);
    }

    /// Delay closing one panel while the pointer crosses between hit regions.
    pub fn schedule_close(panel: PanelId, grace: Duration) {
        let state = MotionState::write();
        let mut inner = state
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let panel_state = inner
            .get_mut(&panel)
            .expect("PanelId must be registered in MotionState::new");

        let candidate = Instant::now() + grace;
        let should_request = if panel_state.motion.target() >= 0.5 {
            match panel_state.close_deadline {
                Some(existing) if existing <= candidate => false,
                _ => {
                    panel_state.close_deadline = Some(candidate);
                    true
                }
            }
        } else {
            false
        };

        drop(inner);
        drop(state);

        if should_request {
            request_frame();
        }
    }

    /// A Wayland pointer-leave can occur when crossing an input-region gap.
    /// Schedule all active panels, allowing whichever panel is entered next
    /// to cancel its own pending close. A panel's own exit handler can shorten
    /// this deadline when the cursor has actually left that panel.
    pub fn schedule_close_all(grace: Duration) {
        let state = MotionState::write();
        let mut inner = state
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let deadline = Instant::now() + grace;
        let mut scheduled = false;

        for panel_state in inner.values_mut() {
            if panel_state.motion.target() < 0.5 {
                continue;
            }

            match panel_state.close_deadline {
                Some(existing) if existing <= deadline => {}
                _ => {
                    panel_state.close_deadline = Some(deadline);
                    scheduled = true;
                }
            }
        }

        drop(inner);
        drop(state);

        if scheduled {
            request_frame();
        }
    }

    pub fn set_open(panel: PanelId, open: bool) {
        let state = MotionState::write();
        let mut inner = state
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if open {
            for (other, other_state) in inner.iter_mut() {
                if *other == panel || other_state.motion.target() < 0.5 {
                    continue;
                }
                other_state.close_deadline = None;
                let config = if *other == PanelId::Launcher {
                    LAUNCHER_CLOSE
                } else {
                    PANEL_CLOSE
                };
                other_state.motion.set_target_with_config(0.0, config);
            }
        }

        let panel_state = inner
            .get_mut(&panel)
            .expect("PanelId must be registered in MotionState::new");

        // Re-entering the trigger or panel cancels a pending delayed close,
        // even if the spring destination is already open.
        if open {
            panel_state.close_deadline = None;
        } else {
            panel_state.close_deadline = None;
        }

        let (target, config) = match (panel, open) {
            (PanelId::Launcher, true) => (1.0, LAUNCHER_OPEN),
            (PanelId::Launcher, false) => (0.0, LAUNCHER_CLOSE),
            (PanelId::QuickSettings, false) => (0.0, STATUS_CLOSE),
            (_, true) => (1.0, PANEL_OPEN),
            (_, false) => (0.0, PANEL_CLOSE),
        };

        // Ignore duplicate notifications without restarting the spring.
        if (panel_state.motion.target() - target).abs() <= f32::EPSILON {
            return;
        }

        // Retarget from the current position and velocity.
        panel_state.motion.set_target_with_config(target, config);
    }

    /// Samples a panel and updates its deformation from the panel's path.
    /// `travel_per_progress` is the scene-space displacement for progress 0 -> 1.
    pub fn sample(panel: PanelId, travel_per_progress: (f32, f32)) -> PanelMotionSample {
        let state = MotionState::read();
        let mut inner = state
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let panel_state = inner
            .get_mut(&panel)
            .expect("PanelId must be registered in MotionState::new");

        let mut waiting_for_hover_destination = false;
        if let Some(deadline) = panel_state.close_deadline {
            if Instant::now() >= deadline {
                panel_state.close_deadline = None;

                let (target, config) = match panel {
                    PanelId::Launcher => (0.0, LAUNCHER_CLOSE),
                    PanelId::QuickSettings => (0.0, STATUS_CLOSE),
                    _ => (0.0, PANEL_CLOSE),
                };

                if (panel_state.motion.target() - target).abs() > f32::EPSILON {
                    panel_state.motion.set_target_with_config(target, config);
                }
            } else {
                waiting_for_hover_destination = true;
            }
        }

        let movement = panel_state.motion.sample();
        let velocity_x = movement.velocity * finite_or_zero(travel_per_progress.0);
        let velocity_y = movement.velocity * finite_or_zero(travel_per_progress.1);
        let deformation_active = panel_state.deformer.update(velocity_x, velocity_y);
        let deformation = panel_state.deformer.matrix().as_transform();
        let target_is_open = panel_state.motion.target() >= 0.5;
        let settled = movement.settled && !deformation_active;

        if waiting_for_hover_destination || !settled {
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
