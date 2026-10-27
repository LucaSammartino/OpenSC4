// SPDX-License-Identifier: GPL-3.0-or-later
//! cSC4DispatchVehicleView, from dispatch-vehicle-view.md and dispatch.md §4.1.3.
pub use crate::dispatch_vehicle_track_info::{flags, Icon, Quad, Rect, Token, TrackInfo, Vec3, Vertex};
use std::collections::BTreeSet;
mod display;
pub use display::{build_display, Camera, DisplayServices, Image, Reference, ResourceOp, Unsupported};
mod services;
pub use services::*;
mod tracking;
mod update;
pub use update::{
    clamp_rectangle, conflict, relocate, screen_quads, DrawOp, FloatRect, FrameServices, Progress, ProgressNode,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Obstacle {
    pub rectangle: Rect,
    pub category: u8,
}

#[derive(Default)]
pub struct DispatchVehicleView {
    pub initialized: bool,
    pub states: Option<[Token; 3]>,
    pub icons: [Option<Icon>; 7],
    pub limits: [u32; 7],
    pub counts: [u32; 7],
    pub active: u32,
    pub windows: Vec<Obstacle>,
    pub(crate) tracks: Vec<TrackInfo>,
    pub(crate) hidden: BTreeSet<u32>,
}
impl DispatchVehicleView {
    pub fn tracks(&self) -> &[TrackInfo] {
        &self.tracks
    }
    pub(crate) fn insert(&mut self, info: TrackInfo) {
        let index = self.tracks.partition_point(|t| t.key() <= info.key());
        self.tracks.insert(index, info);
    }
}

#[cfg(test)]
mod data_tests {
    use super::*;
    #[test]
    fn low_byte_map_is_stable_for_equal_keys() {
        let mut v = DispatchVehicleView::default();
        for (occupant, kind) in [(10, 4), (11, 1), (12, 4), (13, 257)] {
            v.insert(TrackInfo::for_tracking(occupant, kind, 0, None, None));
        }
        assert_eq!(
            v.tracks.iter().map(|t| t.occupant).collect::<Vec<_>>(),
            [11, 13, 10, 12]
        );
    }
}
#[cfg(test)]
mod tests;
