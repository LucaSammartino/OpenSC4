// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::sync::Mutex;

#[derive(Debug, Default)]
struct View {
    observations: Mutex<Vec<(Vec2, usize)>>,
    corners_requested: Mutex<Vec<(u32, Vec2)>>,
}

impl CameraViewUtilities for View {
    fn city_corners(&self, rotation: u32, city_size: Vec2) -> [Vec3; 4] {
        self.corners_requested.lock().unwrap().push((rotation, city_size));
        // Explicit foreign input, deliberately unlike the built-in city table.
        [[100.0, 0.0, 200.0]; 4]
    }
    fn update_camera_position(&self, control: &CameraControl) {
        let camera = control.camera().unwrap();
        assert_eq!(camera.read().unwrap().position(), control.eye_position());
        self.observations
            .lock()
            .unwrap()
            .push((control.position_tweak(), control.bounds.len()));
    }
}

#[test]
fn tweak_optional_view_observes_computed_camera_before_empty_bounds_build() {
    let mut c = CameraControl::new();
    assert_eq!(c.reference_count(), 0);
    assert_eq!(c.add_ref(), 1);
    assert_eq!(c.add_ref(), 2);
    c.init([2048.0, 0.0, 2048.0], Arc::new(RwLock::new(S3dCamera::new())));
    c.bounds.clear();
    c.take_messages();
    c.set_camera_position_tweak_add(2.0, 3.0);
    assert!(c.bounds.is_empty(), "no view utilities: no bounds work in tweak");
    let view = Arc::new(View::default());
    c.set_view_utilities(Some(view.clone()));
    c.set_camera_position_tweak_add(4.0, 5.0);
    assert_eq!(*view.observations.lock().unwrap(), [([4.0, 5.0], 0)]);
    assert_eq!(*view.corners_requested.lock().unwrap(), [(0, [4096.0, 4096.0])]);
    let q = c.convert_world_point_to_camera_plane_point([100.0, 0.0, 200.0]);
    assert_eq!(c.bounds[2], [q[0] - 500.0, q[1]]);
    assert_eq!(c.bounds[3], q);
    let bounds = c.bounds.clone();
    c.set_camera_position_tweak_add(6.0, 7.0);
    c.set_camera_position_tweak_add(6.0, 7.0);
    assert_eq!(*view.observations.lock().unwrap(), [([4.0, 5.0], 0), ([6.0, 7.0], 6)]);
    assert_eq!(view.corners_requested.lock().unwrap().len(), 1);
    assert_eq!(c.bounds, bounds);
    c.set_scroll_bounds_in_use(false);
    c.update_scroll_bounds();
    assert_eq!(view.corners_requested.lock().unwrap().len(), 1);
    c.detach_from_camera();
    c.set_camera_position_tweak_add(8.0, 9.0);
    assert_eq!(view.observations.lock().unwrap().len(), 2);
    assert!(c.messages.is_empty());
}

#[test]
fn scroll_exhaustion_reaches_all_n_projections_before_returning_original() {
    // §6.1 requires a path discriminator: output equality alone cannot prove
    // exhaustion. This triangle consumes all three projections without
    // a no-edge or direction-history stop.
    let polygon = [[-9.0, 10.0], [-7.0, 0.0], [6.0, 6.0]];
    let from = [-2.0, 0.0];
    let mut projections = Vec::new();
    let result = clamp_polygon_observed(&polygon, true, from, [9.0, -5.0], |edge| projections.push(edge));
    assert_eq!(projections.len(), polygon.len());
    assert_eq!(result, from);
}

#[test]
fn tied_scroll_crossing_projects_right_edge_before_history_stop() {
    let polygon = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let mut projections = Vec::new();
    let result = clamp_polygon_observed(&polygon, true, [5.0, 5.0], [15.0, 15.0], |edge| projections.push(edge));
    // Edge 2 is (10,0)->(10,10); the tied top edge must not win. The next
    // intersection hits the opposite direction category before projection.
    assert_eq!(projections, [2]);
    for coordinate in result {
        assert!((coordinate - 9.999).abs() < 0.0001);
    }
}

#[test]
fn below_threshold_target_clamp_keeps_target_velocity_and_sends_nothing() {
    let mut c = CameraControl {
        camera: Some(Arc::new(RwLock::new(S3dCamera::new()))),
        axes: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        corner: [10.0, 5.0, 0.0],
        target: [10.00005, 5.0, 9.0],
        velocity: [1.0, 2.0, 3.0],
        bounds: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
        ..CameraControl::new()
    };
    let state = c.camera().unwrap().read().unwrap().projection_state();
    c.clamp_target_to_scroll_bounds();
    assert_eq!(c.target, [10.00005, 5.0, 9.0]);
    assert_eq!(c.velocity, [1.0, 2.0, 3.0]);
    assert_eq!(c.camera().unwrap().read().unwrap().projection_state(), state);
    assert!(c.messages.is_empty());
}
