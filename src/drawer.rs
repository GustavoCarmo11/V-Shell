use crate::motion::{Motion, PanelId, PanelMotionSample};

pub const DASHBOARD_WIDTH: f32 = 420.0;
pub const DASHBOARD_HEIGHT: f32 = 390.0;

/// Dashboard controls backed by the shared motion and deformation engine.
pub struct DrawerState;

impl DrawerState {
    pub fn open() {
        Motion::open(PanelId::Dashboard);
    }

    pub fn close() {
        Motion::close(PanelId::Dashboard);
    }

    /// Samples the panel with its actual vertical travel in logical pixels.
    pub fn sample(travel_y: f32) -> PanelMotionSample {
        Motion::sample(PanelId::Dashboard, (0.0, travel_y))
    }

    pub fn progress() -> f32 {
        Motion::progress(PanelId::Dashboard)
    }

    pub fn is_open() -> bool {
        Motion::is_open(PanelId::Dashboard)
    }
}
