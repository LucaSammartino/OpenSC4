// SPDX-License-Identifier: GPL-3.0-or-later
use super::control::clamp_polygon;
use super::geometry::snap;
use super::*;
use crate::network_occupants::xform::{Xform, XFORM_MATRIX, XFORM_SCALE, XFORM_TRANSLATION};
use crate::stream::{ByteReader, ByteWriter, OutStream};
use std::sync::{Arc, RwLock};

fn near(a: f32, b: f32, tolerance: f32) {
    assert!((a - b).abs() <= tolerance, "{a:?} != {b:?} within {tolerance}");
}
fn near3(a: Vec3, b: Vec3, tolerance: f32) {
    for i in 0..3 {
        near(a[i], b[i], tolerance);
    }
}
fn handle() -> CameraHandle {
    Arc::new(RwLock::new(S3dCamera::new()))
}
fn control() -> CameraControl {
    let mut c = CameraControl::new();
    c.set_view_utilities(Some(Arc::new(PublishedView)));
    c.init([2048.0, 0.0, 2048.0], handle());
    c.take_messages();
    c
}
#[derive(Debug)]
struct PublishedView;
impl CameraViewUtilities for PublishedView {
    fn city_corners(&self, rotation: u32, [w, d]: Vec2) -> [Vec3; 4] {
        // Foreign utility inputs from its published corner table.
        let points = match rotation {
            0 => [[0.0, d], [0.0, 0.0], [w, 0.0], [w, d]],
            1 => [[0.0, 0.0], [w, 0.0], [w, w], [0.0, d]],
            2 => [[w, 0.0], [w, d], [0.0, d], [0.0, 0.0]],
            3 => [[w, d], [0.0, d], [0.0, 0.0], [w, 0.0]],
            _ => panic!("fixture requires a valid rotation"),
        };
        points.map(|p| [p[0], 0.0, p[1]])
    }
    fn update_camera_position(&self, _: &CameraControl) {}
}
fn kinds(c: &CameraControl) -> Vec<ViewMessageKind> {
    c.messages().iter().map(|m| m.kind).collect()
}

#[test]
fn camera_defaults_and_identity_reference_ledger() {
    let mut c = S3dCamera::new();
    assert_eq!(c.class_id(), 0xE9C6_262A);
    assert_eq!(c.reference_count(), 0);
    assert!(!c.query_interface(0xDEAD));
    assert!(c.query_interface(1));
    assert!(c.query_interface(CAMERA_CLASS_ID));
    assert_eq!(c.reference_count(), 2);
    assert_eq!(c.release(), 1);
    assert_eq!(c.release(), 0);
    assert_eq!(c.release(), 0);
    assert_eq!(c.viewport(), [100.0, 100.0]);
    assert_eq!(c.depth_size(), 65536.0);
    assert_eq!(c.depth_bias(), 0.0);
    assert_eq!(c.projection_type(), 0);
    assert_eq!(c.local_xform(), &Xform::default());
    assert_eq!(c.view_xform(), &Xform::default());
}

#[test]
fn viewport_resets_subview_and_exact_counters() {
    let mut c = S3dCamera::new();
    c.set_subview(1.0, 2.0, 30.0, 40.0);
    assert!(c.subview_in_use());
    assert_eq!(c.projection_state(), 2);
    c.set_viewport(800.0, 600.0);
    assert!(!c.subview_in_use());
    assert_eq!(
        c.subview(),
        Viewport {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0
        }
    );
    assert_eq!(c.projection_state(), 4);
    c.set_depth_bias(1.0);
    c.set_depth_size(128.0);
    c.set_pixel_offset(2.0, 3.0);
    c.set_ortho(-4.0, 4.0, -2.0, 2.0, 2.0, 6.0);
    assert_eq!(c.projection_state(), 8);
}

#[test]
fn specified_ortho_matrix_and_independent_points() {
    let mut c = S3dCamera::new();
    c.set_viewport(800.0, 600.0);
    c.set_ortho(-400.0, 400.0, -300.0, 300.0, 10.0, 20.0);
    assert_eq!(
        *c.projection_matrix(),
        [
            [1.0 / 400.0, 0.0, 0.0, 0.0],
            [0.0, 1.0 / 300.0, 0.0, 0.0],
            [0.0, 0.0, -0.2, -3.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
    );
    assert_eq!(c.project([0.0, 0.0, -15.0]), Some([400.0, 300.0, 0.0]));
    assert_eq!(c.project([400.0, 0.0, -10.0]), Some([800.0, 300.0, -1.0]));
    assert_eq!(c.unproject([400.0, 300.0, 0.0]), Some([0.0, 0.0, -15.0]));
    assert_eq!(c.unproject([800.0, 300.0, -1.0]), Some([400.0, 0.0, -10.0]));
    c.set_pixel_offset(1.0, 0.0);
    assert_eq!(c.projection_matrix()[0][3], -2.0 / 800.0);
    assert_eq!(c.project([0.0, 0.0, -15.0]).unwrap()[0], 399.0);
}

#[test]
fn exact_subview_offset_depth_bias_matrix_and_inverse() {
    let mut c = S3dCamera::new();
    c.set_viewport(8.0, 4.0);
    c.set_ortho(-4.0, 4.0, -2.0, 2.0, 2.0, 6.0);
    c.set_subview(2.0, 1.0, 4.0, 2.0);
    c.set_pixel_offset(1.0, -1.0);
    c.set_depth_size(16.0);
    c.set_depth_bias(4.0);
    assert_eq!(
        *c.projection_matrix(),
        [
            [0.5, 0.0, 0.0, -0.5],
            [0.0, 1.0, 0.0, 1.0],
            [0.0, 0.0, -0.5, -2.25],
            [0.0, 0.0, 0.0, 1.0],
        ]
    );
    assert_eq!(
        *c.inverse_projection_matrix(),
        [
            [2.0, 0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0, -1.0],
            [0.0, 0.0, -2.0, -4.5],
            [0.0, 0.0, 0.0, 1.0],
        ]
    );
    assert_eq!(c.project([0.0, 0.0, -4.0]), Some([3.0, 1.0, -0.25]));
    assert_eq!(c.unproject([3.0, 1.0, -0.25]), Some([0.0, 0.0, -4.0]));
    assert_eq!(
        c.view_volume(),
        ViewVolume {
            left: -3.0,
            right: 5.0,
            bottom: -1.0,
            top: 3.0,
            near: 2.0,
            far: 6.0
        }
    );
}

#[test]
fn exact_perspective_asymmetric_matrix_and_direct_projection() {
    let mut c = S3dCamera::new();
    c.set_viewport(8.0, 4.0);
    c.set_frustum(-1.0, 3.0, -2.0, 2.0, 2.0, 6.0);
    c.set_pixel_offset(1.0, -1.0);
    c.set_depth_size(16.0);
    c.set_depth_bias(4.0);
    assert_eq!(
        *c.projection_matrix(),
        [
            [1.0, 0.0, 0.75, 0.0],
            [0.0, 1.0, -0.5, 0.0],
            [0.0, 0.0, -1.75, -6.0],
            [0.0, 0.0, -1.0, 0.0],
        ]
    );
    assert_eq!(
        *c.inverse_projection_matrix(),
        [
            [1.0, 0.0, 0.0, 0.75],
            [0.0, 1.0, 0.0, -0.5],
            [0.0, 0.0, 0.0, -1.0],
            [0.0, 0.0, -1.0 / 6.0, 1.75 / 6.0],
        ]
    );
    assert_eq!(c.project([0.0, 0.0, -4.0]), Some([1.0, 1.0, 0.25]));
    near3(c.unproject([1.0, 1.0, 0.25]).unwrap(), [0.0, 0.0, -4.0], 0.000001);
    assert_eq!(c.projection_type(), 2);
    assert!(c.project([0.0, 0.0, 0.0]).is_none());
    assert!(c.project([0.0, 0.0, -0.00009999]).is_none());
    assert!(c.project([0.0, 0.0, -0.0001]).is_some());
    assert!(c.unproject([4.0, 2.0, 1.75]).is_none());
}

#[test]
fn perspective_subview_has_independent_offset_signs() {
    let mut c = S3dCamera::new();
    c.set_viewport(8.0, 4.0);
    c.set_frustum(-1.0, 3.0, -2.0, 2.0, 2.0, 6.0);
    c.set_subview(0.0, 0.0, 4.0, 2.0);
    assert_eq!(
        *c.projection_matrix(),
        [
            [2.0, 0.0, 0.0, 0.0],
            [0.0, 2.0, -1.0, 0.0],
            [0.0, 0.0, -2.0, -6.0],
            [0.0, 0.0, -1.0, 0.0],
        ]
    );
    assert_eq!(c.project([0.0, 0.0, -4.0]), Some([2.0, 0.0, 0.5]));
}

#[test]
fn look_at_axes_position_and_counter_are_owned() {
    let mut c = S3dCamera::new();
    c.set_position([0.0, 0.0, 10.0]);
    assert_eq!(c.view_state(), 1);
    c.set_look_at([0.0; 3], [0.0, 1.0, 0.0]);
    assert_eq!(c.view_state(), 3);
    assert_eq!(c.local_xform().matrix, [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
    assert_eq!(c.view_xform().translation, [0.0, 0.0, -10.0]);
    assert_eq!(c.look_at(), [0.0; 3]);
    assert_eq!(c.up(), [0.0, 1.0, 0.0]);
    c.reset_xforms();
    assert_eq!(c.view_state(), 4);
    assert_eq!(c.local_xform().mask, 0);
    assert_eq!(c.view_xform().mask, 0);
    assert_eq!(c.position(), [0.0; 3]);
    c.reset_xforms();
    assert_eq!(c.view_state(), 4);
}

#[test]
fn inverse_view_includes_rotation_translation_and_scale() {
    let mut c = S3dCamera::new();
    c.set_local_xform(Xform {
        counter: 1,
        mask: XFORM_MATRIX | XFORM_TRANSLATION | XFORM_SCALE,
        matrix: [0.0, 0.0, 1.0, 0.0, 1.0, 0.0, -1.0, 0.0, 0.0],
        translation: [10.0, 20.0, 30.0],
        scale: 2.0,
    });
    assert_eq!(c.view_xform().scale, 0.5);
    assert_eq!(c.view_xform().translation, [15.0, -10.0, -5.0]);
    c.set_viewport(8.0, 4.0);
    c.set_ortho(-4.0, 4.0, -2.0, 2.0, -2.0, 2.0);
    assert_eq!(c.project([10.0, 20.0, 28.0]), Some([5.0, 2.0, 0.0]));
    assert_eq!(c.unproject([5.0, 2.0, 0.0]), Some([10.0, 20.0, 28.0]));
}

#[test]
fn transform_counter_wrap_and_identity_without_flags() {
    let mut c = S3dCamera::new();
    for _ in 0..256 {
        c.set_position([1.0, 2.0, 3.0]);
    }
    assert_eq!(c.view_state(), 0);
    assert_eq!(c.view_xform().translation, [-1.0, -2.0, -3.0]);
    c.reset_xforms();
    assert_eq!(c.view_state(), 1);
    assert_eq!(c.view_xform().translation, [0.0; 3]);
}

#[test]
fn degenerate_look_at_and_zero_viewport_retain_ieee_results() {
    let mut c = S3dCamera::new();
    c.set_look_at([0.0; 3], [0.0, 1.0, 0.0]);
    assert!(c.local_xform().matrix.iter().any(|v| v.is_nan()));
    c.reset_xforms();
    c.set_position([0.0, 1.0, 0.0]);
    c.set_look_at([0.0; 3], [0.0, 1.0, 0.0]);
    assert!(c.local_xform().matrix.iter().any(|v| v.is_nan()));
    c.set_viewport(0.0, 0.0);
    assert!(c.projection_matrix().iter().flatten().any(|v| !v.is_finite()));
    c.set_ortho(0.0, 0.0, 1.0, 1.0, 2.0, 2.0);
    assert!(!c.projection_matrix()[0][0].is_finite());
}

#[test]
fn orthographic_eye_ray_is_far_length_with_depth_zero_pick() {
    let mut c = S3dCamera::new();
    c.set_viewport(8.0, 4.0);
    c.set_ortho(-4.0, 4.0, -2.0, 2.0, 2.0, 6.0);
    c.set_position([0.0, 0.0, 10.0]);
    c.set_look_at([0.0; 3], [0.0, 1.0, 0.0]);
    assert_eq!(
        c.eye_ray([4.0, 2.0]),
        Some(Ray {
            origin: [0.0, 0.0, 12.0],
            direction: [0.0, 0.0, -6.0]
        })
    );
    assert_eq!(c.eye_ray([8.0, 0.0]).unwrap().origin, [4.0, 2.0, 12.0]);
    assert_eq!(c.eye_rays(&[[4.0, 2.0], [8.0, 0.0]]).len(), 2);
    let default = S3dCamera::new().eye_ray([50.0, 50.0]).unwrap();
    assert_eq!(default.direction, [0.0; 3]);
    assert_eq!(default.origin, [0.0, 0.0, -1.5]);
}

#[test]
fn perspective_eye_ray_uses_whole_viewport_and_literal_negation() {
    let mut c = S3dCamera::new();
    c.set_viewport(8.0, 4.0);
    c.set_frustum(-10.0, 10.0, -20.0, 20.0, 2.0, 6.0);
    c.set_subview(3.0, 1.0, 2.0, 1.0);
    c.set_pixel_offset(7.0, -5.0);
    assert_eq!(
        c.eye_ray([0.0, 0.0]),
        Some(Ray {
            origin: [0.0; 3],
            direction: [1.0, -1.0, 2.0]
        })
    );
    assert_eq!(c.eye_ray([4.0, 2.0]).unwrap().direction, [0.0, 0.0, 2.0]);
}

#[test]
fn snapping_is_ties_to_even_with_negative_ties() {
    for (input, output) in [
        (0.5, 0.0),
        (1.5, 2.0),
        (2.5, 2.0),
        (-0.5, 0.0),
        (-1.5, -2.0),
        (-2.5, -2.0),
    ] {
        assert_eq!(snap(input), output);
    }
}

#[test]
fn published_zoom_tables_and_yaws_all_combinations() {
    let expected_mpp = [2.613126, 1.306563, 0.653281, 0.286370, 0.143185];
    let pitches = [30.0_f32, 35.0, 40.0, 45.0, 45.0];
    // Public CA4/CA5 correction: 3π/8, 7π/8, 11π/8, 15π/8.
    let yaws = [1.1780972, 2.7488935, 4.31969, 5.8904862];
    let steps = [100.0, 50.0, 25.0, 12.5, 6.25];
    let mut c = control();
    c.set_scroll_bounds_in_use(false);
    for z in 0..5 {
        for r in 0..4 {
            c.set_zoom_and_rotation(z, r);
            near(c.metres_per_pixel(), expected_mpp[z as usize], 0.000001);
            near(c.pitch(), pitches[z as usize].to_radians(), 0.0000001);
            near(c.yaw(), yaws[r as usize], 0.000001);
            near(c.pixels_per_metre() * c.metres_per_pixel(), 1.0, 0.0000001);
            assert_eq!(c.scroll_step(), steps[z as usize]);
        }
    }
}

#[test]
fn eye_offset_and_plane_equation_are_independently_constrained() {
    let c = control();
    let ox = 4096.0 * (2.0 - 2.0_f32.sqrt()).sqrt();
    let oz = 4096.0 * (2.0 + 2.0_f32.sqrt()).sqrt();
    near3(c.eye_offset(), [ox, 8192.0 / 3.0_f32.sqrt(), oz], 0.002);
    let k = 1.0 - 2048.0 * (ox + oz) / (8192.0 * 8192.0 * (4.0 / 3.0));
    near3(
        c.eye_position(),
        [2048.0 + k * ox, k * 8192.0 / 3.0_f32.sqrt(), 2048.0 + k * oz],
        0.004,
    );
    let [a, b, d, e] = c.camera_plane();
    let eye = c.eye_position();
    near(a * eye[0] + b * eye[1] + d * eye[2] + e, 0.0, 0.001);
    let delta = super::geometry::sub(c.eye_position(), c.camera_plane_origin());
    near(super::geometry::dot(delta, c.eye_offset()), 0.0, 16.0);
}

#[test]
fn rotation_corners_and_clip_city_minimum() {
    let mut c = control();
    let clip = c.distances_to_clip_planes();
    c.set_city_size(1024.0, 2048.0);
    assert_eq!(c.distances_to_clip_planes(), clip);
    for (r, corner) in [
        [0.0, 0.0, 0.0],
        [1024.0, 0.0, 0.0],
        [1024.0, 0.0, 2048.0],
        [0.0, 0.0, 2048.0],
    ]
    .into_iter()
    .enumerate()
    {
        c.set_rotation(r as u32);
        assert_eq!(c.rotation_corner(), corner);
    }
    c.set_rotation(0);
    c.set_city_size(8192.0, 4096.0);
    assert!(c.distances_to_clip_planes()[0] < clip[0]);
    assert_eq!(c.distances_to_clip_planes()[1], clip[1]);
}

#[test]
fn pixel_corner_is_snapped_and_tweak_changes_it_directly() {
    let mut c = control();
    for r in 0..4 {
        c.set_rotation(r);
        c.set_view_target([1234.25, 0.0, 987.75]);
        let camera = c.camera().unwrap();
        let p = camera.read().unwrap().project(c.rotation_corner()).unwrap();
        near(p[0], snap(p[0]), 0.00025);
        near(p[1], snap(p[1]), 0.00025);
    }
    c.set_camera_position_tweak_add(2.0, 3.0);
    let camera = c.camera().unwrap();
    let offset = camera.read().unwrap().pixel_offset();
    c.set_camera_position_tweak_add(2.0, 3.0);
    assert_eq!(camera.read().unwrap().pixel_offset(), offset);
    assert_eq!(c.position_tweak(), [2.0, 3.0]);
}

#[test]
fn zoom_fractional_magnification_refusal_and_virtual_threshold() {
    let mut c = control();
    assert!(c.set_zoom(4.5));
    assert_eq!(c.zoom(), 4);
    near(c.custom_magnification(), std::f32::consts::SQRT_2, 0.0000001);
    assert_eq!(c.virtual_zoom(), 4);
    assert!(!c.set_zoom(4.5));
    assert!(c.set_zoom(4.75));
    near(c.custom_magnification(), 1.6817929, 0.0000001);
    assert_eq!(c.virtual_zoom(), 5);
    for z in [-1.0, 5.0, 100.0, f32::NAN, f32::INFINITY] {
        assert!(!c.set_zoom(z));
    }
    assert!(c.set_zoom(4.0));
    assert_eq!(c.custom_magnification(), 1.0);
    assert!(c.set_zoom(2.9));
    assert_eq!(c.zoom(), 2);
    assert!(c.set_zoom(2.1));
    c.set_custom_magnification(1.5);
    assert_eq!(c.virtual_zoom(), 2);
    c.set_custom_magnification(f32::from_bits(1.5_f32.to_bits() + 1));
    assert_eq!(c.virtual_zoom(), 3);
}

#[test]
fn custom_magnification_checks_the_old_value_even_when_new_is_invalid() {
    for invalid in [0.0, -1.0, 1000.0, f32::INFINITY, f32::NAN] {
        let mut c = control();
        c.set_custom_magnification(invalid);
        assert_eq!(c.custom_magnification().to_bits(), invalid.to_bits());
        c.take_messages();
        c.set_custom_magnification(1.0);
        assert_eq!(c.custom_magnification().to_bits(), invalid.to_bits());
        assert!(c.messages().is_empty());
    }
}

#[test]
fn rotation_message_order_and_pairs_are_exact() {
    let mut c = control();
    c.set_scroll_bounds_in_use(false);
    assert!(c.set_rotation(1));
    assert_eq!(
        kinds(&c),
        vec![
            ViewMessageKind::ViewZRChange1,
            ViewMessageKind::CameraUpdated,
            ViewMessageKind::ViewZRChange2,
            ViewMessageKind::ViewZRChange3
        ]
    );
    for m in c.messages() {
        assert_eq!(m.previous, ZoomRotation { zoom: 0, rotation: 0 });
        assert_eq!(m.new, ZoomRotation { zoom: 0, rotation: 1 });
    }
    c.take_messages();
    assert!(c.set_rotation(1));
    assert!(c.set_rotation(4));
    assert!(!c.set_zoom_and_rotation(0, 1));
    assert!(!c.set_zoom_and_rotation(5, 0));
    assert!(!c.set_zoom_and_rotation(0, u32::MAX));
    assert!(c.messages().is_empty());
}

#[test]
fn magnification_refresh_precedes_zoom_change_and_target_message_order() {
    let mut c = control();
    c.set_scroll_bounds_in_use(false);
    c.set_zoom(4.5);
    assert_eq!(
        kinds(&c),
        vec![
            ViewMessageKind::PostViewRefresh,
            ViewMessageKind::ViewZRChange1,
            ViewMessageKind::CameraUpdated,
            ViewMessageKind::ViewZRChange2,
            ViewMessageKind::ViewZRChange3
        ]
    );
    c.take_messages();
    c.set_view_target([500.0, 0.0, 500.0]);
    assert_eq!(
        kinds(&c),
        vec![
            ViewMessageKind::CameraUpdated,
            ViewMessageKind::ViewTChange2,
            ViewMessageKind::ViewTChange3
        ]
    );
    assert_eq!(ViewMessageKind::ViewZRChange1 as u32, 0xA6B79600);
    assert_eq!(ViewMessageKind::ViewTChange3 as u32, 0xA6B79612);
    assert_eq!(ViewMessageKind::PostViewRefresh as u32, 0xA6B79621);
}

#[test]
fn target_validation_and_velocity_nan_only_test() {
    let mut c = control();
    for target in [[f32::NAN, 0.0, 0.0], [0.0, f32::INFINITY, 0.0], [0.0, 0.0, -100001.0]] {
        assert!(!c.set_view_target(target));
    }
    assert!(c.messages().is_empty());
    c.set_view_target_velocity([1.0, 2.0, 3.0]);
    c.set_view_target_velocity([f32::NAN, 0.0, 0.0]);
    assert_eq!(c.view_target_velocity(), [1.0, 2.0, 3.0]);
    c.set_view_target_velocity([f32::INFINITY, 0.0, 0.0]);
    assert_eq!(c.view_target_velocity()[0], f32::INFINITY);
    assert!(c.set_view_target([100000.0, -100000.0, 100000.0]));
    assert_eq!(c.view_target_velocity(), [0.0; 3]);
}

#[test]
fn ground_intersection_has_exact_threshold_and_rejects_behind() {
    assert_eq!(
        intersect_with_ground_plane([1.0, -1.0, 0.0], [0.0, 100.0, 0.0]),
        Some([100.0, 0.0, 0.0])
    );
    assert_eq!(intersect_with_ground_plane([1.0, 0.0, 0.0], [0.0, 100.0, 0.0]), None);
    assert_eq!(intersect_with_ground_plane([0.0, 1.0, 0.0], [0.0, 100.0, 0.0]), None);
    assert!(intersect_with_ground_plane([0.0, -0.00009999, 0.0], [0.0, 1.0, 0.0]).is_none());
    assert!(intersect_with_ground_plane([0.0, -0.0001, 0.0], [0.0, 1.0, 0.0]).is_some());
    assert_eq!(
        intersect_with_ground_plane([0.0, -1.0, 0.0], [1.0, 0.0, 2.0]),
        Some([1.0, 0.0, 2.0])
    );
}

#[test]
fn camera_plane_conversion_has_independent_axis_and_origin_results() {
    let c = control();
    let origin = c.camera_plane_origin();
    let sin_yaw = (2.0 + 2.0_f32.sqrt()).sqrt() * 0.5;
    let cos_yaw = (2.0 - 2.0_f32.sqrt()).sqrt() * 0.5;
    near3(
        origin,
        [8192.0 * cos_yaw, 8192.0 / 3.0_f32.sqrt(), 8192.0 * sin_yaw],
        0.002,
    );
    assert_eq!(c.convert_world_point_to_camera_plane_point(origin), [0.0; 2]);
    assert_eq!(c.convert_camera_plane_point_to_world_point([0.0; 2]), origin);
    let q = c.convert_world_point_to_camera_plane_point([0.0; 3]);
    near(q[0], 0.0, 0.001);
    near(q[1], 0.0, 0.001);
    let p = c.convert_camera_plane_point_to_world_point([100.0, 0.0]);
    near3(
        super::geometry::sub(p, origin),
        [100.0 * sin_yaw, 0.0, -100.0 * cos_yaw],
        0.001,
    );
}

#[test]
fn scroll_polygon_preserves_published_lift_and_six_points() {
    let c = control();
    let bounds = c.scroll_bounds();
    assert_eq!(bounds.len(), 6);
    // Analytic projections at yaw 3π/8, pitch 30°, zoom 0.
    let sin_yaw = (2.0 + 2.0_f32.sqrt()).sqrt() * 0.5;
    let cos_yaw = (2.0 - 2.0_f32.sqrt()).sqrt() * 0.5;
    let lift = (8192.0 / 3.0_f32.sqrt() * 0.48) / (3.0_f32.sqrt() * 0.5) - 2.613126 * 300.0;
    near(bounds[0][0], 0.0, 0.002);
    near(bounds[0][1], 1837.5022, 0.006);
    near(bounds[1][0], -4096.0 * cos_yaw - 500.0, 0.003);
    near(bounds[1][1], lift - 2048.0 * sin_yaw, 0.006);
    near(bounds[2][0], -4096.0 * cos_yaw - 500.0, 0.003);
    near(bounds[2][1], -2048.0 * sin_yaw, 0.003);
    near(bounds[3][0], 4096.0 * (sin_yaw - cos_yaw), 0.002);
    near(bounds[3][1], -2048.0 * (cos_yaw + sin_yaw), 0.003);
    near(bounds[4][0], 4096.0 * sin_yaw + 500.0, 0.003);
    near(bounds[5][1], lift - 2048.0 * cos_yaw, 0.006);
}

const SQUARE: [Vec2; 4] = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
#[test]
fn scroll_clamp_disabled_empty_inside_and_outward_cut_are_exact() {
    assert_eq!(clamp_polygon(&SQUARE, false, [5.0, 5.0], [15.0, 5.0]), [15.0, 5.0]);
    assert_eq!(clamp_polygon(&[], true, [5.0, 5.0], [15.0, 5.0]), [15.0, 5.0]);
    assert_eq!(clamp_polygon(&SQUARE, true, [5.0, 5.0], [6.0, 7.0]), [6.0, 7.0]);
    let p = clamp_polygon(&SQUARE, true, [5.0, 5.0], [15.0, 5.0]);
    near(p[0], 9.999, 0.000001);
    assert_eq!(p[1], 5.0);
    let slide = clamp_polygon(&SQUARE, true, [5.0, 5.0], [15.0, 8.0]);
    near(slide[0], 9.999, 0.000001);
    assert_eq!(slide[1], 8.0);
}

#[test]
fn scroll_clamp_corner_reverse_winding_outside_and_short_edge() {
    let corner = clamp_polygon(&SQUARE, true, [5.0, 5.0], [15.0, 15.0]);
    // CA6: the low second fraction becomes zero; opposite history stops.
    near(corner[0], 9.999, 0.000001);
    near(corner[1], 9.999, 0.000001);
    assert!(corner[0] > 9.99 && corner[1] > 9.99);
    let mut reverse = SQUARE;
    reverse.reverse();
    assert_eq!(clamp_polygon(&reverse, true, [5.0, 5.0], [6.0, 5.0]), [5.0, 5.0]);
    assert_eq!(clamp_polygon(&SQUARE, true, [15.0, 5.0], [15.0, 5.0]), [15.0, 5.0]);
    let short = [[0.0, 0.0], [1e-10, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let p = clamp_polygon(&short, true, [5.0, 5.0], [15.0, 5.0]);
    near(p[0], 9.999, 0.000001);
    assert_eq!(clamp_polygon(&[[0.0, 0.0]], true, [1.0, 1.0], [2.0, 2.0]), [2.0, 2.0]);
}

#[test]
fn clamping_an_outside_target_moves_it_and_zeros_velocity() {
    let mut c = control();
    c.set_view_target([6000.0, 0.0, 6000.0]);
    c.set_view_target_velocity([1.0, 2.0, 3.0]);
    c.take_messages();
    c.clamp_target_to_scroll_bounds();
    assert_ne!(c.view_target(), [6000.0, 0.0, 6000.0]);
    assert_eq!(c.view_target_velocity(), [0.0; 3]);
    assert_eq!(
        kinds(&c),
        vec![
            ViewMessageKind::CameraUpdated,
            ViewMessageKind::ViewTChange2,
            ViewMessageKind::ViewTChange3
        ]
    );
}

#[test]
fn inconsistent_scroll_edges_accept_stationary_outside_start() {
    // Contradictory half-planes do not project a stationary outside point:
    // all start/end distances are equal, so no edge can be selected (§2.5).
    let polygon = [
        [0.0, 0.0],
        [3.0, 0.0],
        [3.0, 3.0],
        [2.0, 3.0],
        [2.0, 1.0],
        [1.0, 1.0],
        [1.0, 3.0],
        [0.0, 3.0],
    ];
    assert_eq!(clamp_polygon(&polygon, true, [1.5, 2.0], [1.5, 2.0]), [1.5, 2.0]);
}

#[test]
fn failed_ground_hit_retains_target_and_velocity_but_sends_updates() {
    let mut c = control();
    // This target is past the fixed eye plane, reversing the look direction.
    c.set_view_target([15000.0, 0.0, 15000.0]);
    c.set_view_target_velocity([1.0, 2.0, 3.0]);
    c.take_messages();
    c.clamp_target_to_scroll_bounds();
    assert_eq!(c.view_target(), [15000.0, 0.0, 15000.0]);
    assert_eq!(c.view_target_velocity(), [1.0, 2.0, 3.0]);
    assert_eq!(
        kinds(&c),
        vec![
            ViewMessageKind::CameraUpdated,
            ViewMessageKind::ViewTChange2,
            ViewMessageKind::ViewTChange3
        ]
    );
}

struct Terrain {
    calls: Vec<(Vec3, Vec3)>,
    hit: Option<Vec3>,
}
impl TerrainView for Terrain {
    fn ray_cast(&mut self, origin: Vec3, direction: Vec3) -> Option<Vec3> {
        self.calls.push((origin, direction));
        self.hit
    }
}
#[test]
fn scrolling_uses_only_foreign_terrain_query_then_ground_fallback() {
    let mut c = CameraControl::new();
    assert!(!c.scroll_by_viewport_angle(0.0, 1.0, None));
    c.init([2048.0, 0.0, 2048.0], handle());
    c.set_scroll_bounds_in_use(false);
    let mut terrain = Terrain {
        calls: Vec::new(),
        hit: Some([20.0, 7.0, 30.0]),
    };
    c.set_view_target_velocity([1.0, 2.0, 3.0]);
    assert!(c.scroll_by_viewport_angle(0.0, 1.0, Some(&mut terrain)));
    assert_eq!(terrain.calls.len(), 1);
    assert_eq!(c.view_target(), [20.0, 7.0, 30.0]);
    assert_eq!(c.view_target_velocity(), [1.0, 2.0, 3.0]);
    c.set_view_target([2048.0, 0.0, 2048.0]);
    terrain.hit = None;
    assert!(c.scroll_by_viewport_angle(0.0, 1.0, Some(&mut terrain)));
    let sin_yaw = (2.0 + 2.0_f32.sqrt()).sqrt() * 0.5;
    let cos_yaw = (2.0 - 2.0_f32.sqrt()).sqrt() * 0.5;
    near3(
        c.view_target(),
        [2048.0 + 100.0 * sin_yaw, 0.0, 2048.0 - 100.0 * cos_yaw],
        0.002,
    );
    assert_eq!(terrain.calls.len(), 2);
    c.set_view_target([2048.0, 0.0, 2048.0]);
    assert!(c.scroll_by_viewport_angle(std::f32::consts::FRAC_PI_2, 1.0, None));
    near3(
        c.view_target(),
        [2048.0 + 200.0 * cos_yaw, 0.0, 2048.0 + 200.0 * sin_yaw],
        0.004,
    );
}

#[test]
fn attach_detach_init_once_and_native_destructor_does_not_detach() {
    let cam = handle();
    {
        let mut c = CameraControl::new();
        assert_eq!(c.viewport_size(), [800, 600]);
        c.set_viewport_size(1024, 768);
        c.attach_to_camera(cam.clone());
        assert_eq!(cam.read().unwrap().reference_count(), 1);
        assert_eq!(cam.read().unwrap().viewport(), [1024.0, 768.0]);
        c.attach_to_camera(cam.clone());
        assert_eq!(cam.read().unwrap().reference_count(), 1);
        assert!(c.init([1.0, 0.0, 2.0], cam.clone()));
        assert_eq!(kinds(&c), vec![ViewMessageKind::PostViewInit]);
        c.take_messages();
        assert!(c.init([3.0, 0.0, 4.0], handle()));
        assert_eq!(c.view_target(), [1.0, 0.0, 2.0]);
        assert!(c.messages().is_empty());
        let second = handle();
        c.attach_to_camera(second.clone());
        assert_eq!(cam.read().unwrap().reference_count(), 0);
        assert_eq!(second.read().unwrap().reference_count(), 1);
        assert_eq!(kinds(&c), vec![ViewMessageKind::PostViewInit]);
        c.detach_from_camera();
        assert_eq!(second.read().unwrap().reference_count(), 0);
        c.attach_to_camera(cam.clone());
    }
    // Native deleting destruction does not detach. External teardown must
    // call Detach; Rust's Arc lifetime alone does not decrement the ledger.
    assert_eq!(cam.read().unwrap().reference_count(), 1);
}

fn record_bytes() -> Vec<u8> {
    // Independent record with deliberately different current/new pairs.
    let mut bytes = vec![1, 0];
    for v in [123.0_f32, 4.0, 567.0] {
        bytes.extend(v.to_le_bytes());
    }
    for v in [1_u32, 2, 3, 0, 4, 1] {
        bytes.extend(v.to_le_bytes());
    }
    bytes
}
#[test]
fn save_bytes_exact_and_init_copies_loaded_current_pair_to_new() {
    let bytes = record_bytes();
    assert_eq!(bytes.len(), 38);
    let mut c = CameraControl::new();
    assert!(c.read(&mut ByteReader::new(&bytes)));
    assert!(c.loaded());
    assert!(!c.initialised());
    assert!(c.camera().is_none());
    assert!(c.messages().is_empty());
    assert_eq!(c.view_target(), [123.0, 4.0, 567.0]);
    assert_eq!(c.zoom(), 3);
    assert_eq!(c.rotation(), 0);
    let mut out = ByteWriter::new();
    assert!(c.write(&mut out));
    assert_eq!(out.bytes(), bytes);
    c.set_city_size(1024.0, 1024.0);
    c.init([999.0; 3], handle());
    assert_eq!(c.zoom(), 3);
    assert_eq!(c.rotation(), 0);
    assert_eq!(c.new_zoom_and_rotation(), ZoomRotation { zoom: 3, rotation: 0 });
    assert_eq!(c.view_target(), [123.0, 4.0, 567.0]);
    assert_eq!(c.custom_magnification(), 1.0);
}

#[test]
fn record_bad_version_truncation_and_sticky_write_error() {
    let bytes = record_bytes();
    for n in 0..bytes.len() {
        let mut c = CameraControl::new();
        assert!(!c.read(&mut ByteReader::new(&bytes[..n])));
        assert!(c.loaded());
        let expected = [123.0, 4.0, 567.0];
        for (i, value) in c.view_target().into_iter().enumerate() {
            assert_eq!(value, if n >= 2 + (i + 1) * 4 { expected[i] } else { 0.0 });
        }
    }
    let mut bad = bytes;
    bad[0] = 2;
    assert!(!CameraControl::new().read(&mut ByteReader::new(&bad)));
    let mut out = ByteWriter::new();
    out.set_error();
    assert!(!CameraControl::new().write(&mut out));
    assert!(out.bytes().is_empty());
}

#[test]
fn owners_can_be_read_on_a_second_thread() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<S3dCamera>();
    assert_send_sync::<CameraControl>();
    let c = Arc::new(control());
    let reader = Arc::clone(&c);
    let pair = std::thread::spawn(move || (reader.zoom(), reader.eye_position()))
        .join()
        .unwrap();
    assert_eq!(pair, (c.zoom(), c.eye_position()));
}

#[test]
fn perspective_ray_applies_local_rotation_without_scale() {
    let mut c = S3dCamera::new();
    c.set_viewport(8.0, 4.0);
    c.set_frustum(-1.0, 1.0, -1.0, 1.0, 2.0, 6.0);
    c.set_local_xform(Xform {
        counter: 1,
        mask: XFORM_MATRIX | XFORM_TRANSLATION | XFORM_SCALE,
        matrix: [0.0, 0.0, 1.0, 0.0, 1.0, 0.0, -1.0, 0.0, 0.0],
        translation: [10.0, 20.0, 30.0],
        scale: 7.0,
    });
    assert_eq!(
        c.eye_ray([0.0, 0.0]),
        Some(Ray {
            origin: [10.0, 20.0, 30.0],
            direction: [2.0, -1.0, -1.0],
        })
    );
}

#[test]
fn rectangular_city_scroll_corner_retains_foreign_view_quirk() {
    let mut c = control();
    c.set_scroll_bounds_in_use(false);
    c.set_city_size(2048.0, 4096.0);
    c.set_view_target([1024.0, 0.0, 2048.0]);
    c.set_rotation(1);
    c.set_scroll_bounds_in_use(true);
    // Foreign utility corner 2 is (W, 0, W), not (W, 0, D).
    near(
        c.scroll_bounds()[4][0],
        1024.0 * (2.0 + 2.0_f32.sqrt()).sqrt() + 500.0,
        0.003,
    );
    near(c.scroll_bounds()[4][1], -512.0 * (2.0 - 2.0_f32.sqrt()).sqrt(), 0.003);
}

#[test]
fn safety_policy_contains_malformed_saved_lookups_without_claiming_native_values() {
    let mut bytes = vec![1, 0];
    for _ in 0..3 {
        bytes.extend(0.0_f32.to_le_bytes());
    }
    for _ in 0..6 {
        bytes.extend(u32::MAX.to_le_bytes());
    }
    let mut c = CameraControl::new();
    assert!(c.read(&mut ByteReader::new(&bytes)));
    c.init([1.0; 3], handle());
    assert_eq!(c.zoom(), u32::MAX);
    assert_eq!(c.rotation(), u32::MAX);
    assert_eq!(c.scroll_step(), 6.25);
    c.set_custom_magnification(2.0);
    assert_eq!(c.virtual_zoom(), 0);
    let mut out = ByteWriter::new();
    assert!(c.write(&mut out));
    assert_eq!(out.bytes(), bytes);
}

#[test]
fn viewport_setter_updates_camera_without_implicit_control_recomputation() {
    let mut c = control();
    let cam = c.camera().unwrap();
    let box_before = cam.read().unwrap().raw_view_volume();
    let bounds_before = c.scroll_bounds().to_vec();
    c.set_viewport_size(1600, 1200);
    assert_eq!(cam.read().unwrap().viewport(), [1600.0, 1200.0]);
    assert_eq!(cam.read().unwrap().raw_view_volume(), box_before);
    assert_eq!(c.scroll_bounds(), bounds_before);
    assert!(c.messages().is_empty());
    c.update_camera_zoom_and_rotation_params();
    assert_eq!(cam.read().unwrap().raw_view_volume().right, box_before.right * 2.0);
}

#[test]
fn independent_ca3_constructor_and_degenerate_look_at() {
    let mut c = S3dCamera::new();
    assert_eq!(c.look_at(), [0.0; 3]);
    assert_eq!(c.up(), [0.0; 3]);
    assert_eq!(c.projection_state(), 1);
    assert_eq!(
        c.raw_view_volume(),
        ViewVolume {
            left: -1.0,
            right: 1.0,
            bottom: -1.0,
            top: 1.0,
            near: 1.0,
            far: 2.0
        }
    );
    assert_eq!(
        *c.projection_matrix(),
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, -2.0, -3.0],
            [0.0, 0.0, 0.0, 1.0]
        ]
    );
    c.set_look_at([0.0; 3], [0.0, 1.0, 0.0]);
    assert!(c.local_xform().matrix.iter().all(|x| x.is_nan()));
    assert_eq!(c.view_state(), 2);
    c.set_position([0.0, 0.0, 1.0]);
    c.set_look_at([0.0; 3], [0.0, 0.0, 1.0]);
    assert_eq!(
        [
            c.local_xform().matrix[2],
            c.local_xform().matrix[5],
            c.local_xform().matrix[8]
        ],
        [0.0, 0.0, 1.0]
    );
    assert!(c.local_xform().matrix[0].is_nan());
    let before = c.projection_state();
    c.set_ortho(-1.0, 1.0, -1.0, 1.0, 1.0, 2.0);
    c.set_ortho(-1.0, 1.0, -1.0, 1.0, 1.0, 2.0);
    assert_eq!(c.projection_state(), before + 2);
    c.set_subview(3.0, 4.0, 20.0, 30.0);
    c.reset_subview();
    assert_eq!(c.projection_state(), before + 4);
    assert!(!c.subview_in_use());
    assert_eq!(
        c.subview(),
        Viewport {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0
        }
    );
}

#[test]
fn independent_ca4_camera_less_init_detach_and_raw_read() {
    let mut c = CameraControl::new();
    assert_eq!(c.view_direction(), [0.0; 3]);
    assert_eq!(c.camera_plane_axes(), [[0.0; 3], [0.0, 1.0, 0.0]]);
    assert_eq!(c.distances_to_clip_planes(), [0.0; 2]);
    assert!(c.init([1.0, 2.0, 3.0], None));
    assert!(c.initialised());
    assert!(c.init([9.0; 3], handle()));
    assert!(c.camera().is_none());
    assert_eq!(c.view_target(), [1.0, 2.0, 3.0]);
    c.attach_to_camera(handle());
    c.detach_from_camera();
    assert!(c.initialised());
    assert!(c.init([9.0; 3], handle()));
    assert!(c.camera().is_none());
    let mut bytes = record_bytes();
    bytes[2..6].copy_from_slice(&f32::NAN.to_le_bytes());
    bytes[22..26].copy_from_slice(&17_u32.to_le_bytes());
    bytes[26..30].copy_from_slice(&9_u32.to_le_bytes());
    assert!(c.read(&mut ByteReader::new(&bytes)));
    assert!(c.view_target()[0].is_nan());
    assert_eq!((c.zoom(), c.rotation()), (17, 9));
    let mut output = [123.0, 456.0, 789.0];
    c.rotation_corner_into(9, &mut output);
    assert_eq!(output, [123.0, 0.0, 789.0]);
    // Do not dereference invalid tables as a native fidelity assertion.
}

#[test]
fn independent_ca4_failed_stream_attempts_all_destinations_without_rollback() {
    use crate::stream::{InStream, RefValue};
    struct PartialStream {
        floats: usize,
        integers: usize,
    }
    impl InStream for PartialStream {
        fn get_u16(&mut self) -> Option<u16> {
            Some(1)
        }
        fn get_f32(&mut self) -> Option<f32> {
            self.floats += 1;
            Some(6.0 + self.floats as f32)
        }
        fn get_u32(&mut self) -> Option<u32> {
            self.integers += 1;
            (self.integers == 1).then_some(17)
        }
        fn error(&self) -> i32 {
            i32::from(self.integers > 1)
        }
        fn get_u8(&mut self) -> Option<u8> {
            unreachable!()
        }
        fn get_u64(&mut self) -> Option<u64> {
            unreachable!()
        }
        fn get_f64(&mut self) -> Option<f64> {
            unreachable!()
        }
        fn get_bytes(&mut self, _: usize) -> Option<Vec<u8>> {
            unreachable!()
        }
        fn get_str(&mut self) -> Option<Vec<u8>> {
            unreachable!()
        }
        fn get_ref(&mut self) -> Option<RefValue> {
            unreachable!()
        }
        fn skip(&mut self, _: usize) -> bool {
            unreachable!()
        }
    }
    let mut c = CameraControl::new();
    c.set_view_target([1.0, 2.0, 3.0]);
    let mut input = PartialStream { floats: 0, integers: 0 };
    assert!(!c.read(&mut input));
    assert!(c.loaded());
    assert_eq!(c.view_target(), [7.0, 8.0, 9.0]);
    assert_eq!(c.previous_zoom_and_rotation(), ZoomRotation { zoom: 17, rotation: 0 });
    assert_eq!(input.integers, 6);
    assert_eq!((c.zoom(), c.rotation()), (0, 0));
}

#[test]
fn independent_ca5_reset_order_rotation_preservation_and_float_equality() {
    let mut c = control();
    c.set_scroll_bounds_in_use(false);
    c.set_zoom_and_rotation(4, 0);
    c.set_custom_magnification(2.0);
    c.take_messages();
    assert!(!c.set_zoom_and_rotation(4, 0));
    assert_eq!(c.custom_magnification(), 1.0);
    assert_eq!(kinds(&c), vec![ViewMessageKind::PostViewRefresh]);
    c.set_custom_magnification(2.0);
    c.take_messages();
    assert!(c.set_rotation(1));
    assert_eq!(c.custom_magnification(), 2.0);
    assert_eq!(
        kinds(&c),
        vec![
            ViewMessageKind::ViewZRChange1,
            ViewMessageKind::CameraUpdated,
            ViewMessageKind::ViewZRChange2,
            ViewMessageKind::ViewZRChange3
        ]
    );
    c.set_zoom_and_rotation(0, 0);
    c.take_messages();
    assert!(c.set_zoom(0.5));
    assert!(c.set_zoom(0.5));
    assert_eq!(c.zoom(), 0);
    assert_eq!(c.messages().len(), 8);
    c.take_messages();
    assert!(c.set_rotation(u32::MAX));
    assert!(!c.set_zoom(f32::NAN));
    assert!(c.messages().is_empty());
    c.set_custom_magnification(f32::NAN);
    c.set_custom_magnification(1.0);
    assert!(c.custom_magnification().is_nan());
    let mut detached = CameraControl::new();
    assert!(!detached.set_zoom(2.0));
    assert_eq!(detached.zoom(), 2);
    assert!(!detached.set_rotation(1));
    assert_eq!(detached.rotation(), 1);
    assert!(detached.messages().is_empty());
}

#[test]
fn independent_ca5_tweak_and_signed_viewport_preserve_existing_bounds() {
    let mut c = control();
    let bounds = c.scroll_bounds().to_vec();
    c.set_camera_position_tweak_add(2.0, 3.0);
    c.set_camera_position_tweak_add(4.0, 5.0);
    assert_eq!(c.position_tweak(), [4.0, 5.0]);
    assert_eq!(c.scroll_bounds(), bounds);
    let cam = c.camera().unwrap();
    let volume = cam.read().unwrap().raw_view_volume();
    let counter = cam.read().unwrap().projection_state();
    let eye = c.eye_position();
    c.set_viewport_size(-1024, 768);
    assert_eq!(c.viewport_size(), [-1024, 768]);
    assert_eq!(cam.read().unwrap().viewport(), [-1024.0, 768.0]);
    assert_eq!(cam.read().unwrap().projection_state(), counter + 2);
    assert_eq!(cam.read().unwrap().raw_view_volume(), volume);
    assert_eq!(c.eye_position(), eye);
    assert_eq!(c.scroll_bounds(), bounds);
    assert!(c.messages().is_empty());
}

#[test]
fn independent_ca6_outside_boundary_fraction_and_length_discriminators() {
    for (from, to, expected) in [
        ([12.0, 5.0], [12.0, 5.0], [12.0, 5.0]),
        ([12.0, 5.0], [13.0, 6.0], [12.0, 6.0]),
        ([12.0, 5.0], [11.0, 6.0], [11.0, 6.0]),
        ([5.0, 5.0], [10.0, 5.0], [10.0, 5.0]),
    ] {
        assert_eq!(clamp_polygon(&SQUARE, true, from, to), expected);
    }
    near(clamp_polygon(&SQUARE, true, [5.0, 5.0], [25.0, 5.0])[0], 9.998, 0.0001);
    let narrow = [[0.0, 0.0], [1.0, 0.0], [1.0, 10.0], [0.0, 10.0]];
    assert_eq!(clamp_polygon(&narrow, true, [0.0, 5.0], [2000.0, 5.0]), [0.0, 5.0]);
    near(clamp_polygon(&narrow, true, [0.0, 5.0], [1000.0, 5.0])[0], 0.9, 0.0001);
    for (bits, expected) in [(0x322B_CC77, 1.0), (0x322B_CC78, -0.0002)] {
        let polygon = [[0.0, 0.0], [0.0, f32::from_bits(bits)]];
        near(
            clamp_polygon(&polygon, true, [-1.0, 0.0], [1.0, 0.0])[0],
            expected,
            0.0001,
        );
    }
}

#[test]
fn synchronous_callbacks_observe_live_fields_reentry_and_reused_message() {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    };
    #[derive(Debug)]
    struct Observation {
        kind: ViewMessageKind,
        rotation: u32,
        projection: u32,
        data1: usize,
        data2: usize,
        message: usize,
    }
    #[derive(Debug, Default)]
    struct Server {
        observations: Mutex<Vec<Observation>>,
        entered: AtomicBool,
    }
    impl ViewMessageServer for Server {
        fn send(&self, control: &mut CameraControl, message: &LiveViewMessage) {
            let data1 = match message.data1(control) {
                ViewMessageData::Control(value) => value as *const CameraControl as usize,
                ViewMessageData::Pair(value) => value as *const ZoomRotation as usize,
            };
            let data2 = message.data2(control).map_or(0, |p| p as *const ZoomRotation as usize);
            let projection = control.camera().unwrap().read().unwrap().projection_state();
            self.observations.lock().unwrap().push(Observation {
                kind: message.kind,
                rotation: control.rotation(),
                projection,
                data1,
                data2,
                message: message as *const LiveViewMessage as usize,
            });
            if message.kind == ViewMessageKind::CameraUpdated && !self.entered.swap(true, Ordering::SeqCst) {
                assert!(control.set_rotation(2));
            }
        }
    }
    let mut c = control();
    c.set_scroll_bounds_in_use(false);
    let old_projection = c.camera().unwrap().read().unwrap().projection_state();
    let server = Arc::new(Server::default());
    c.set_message_server(Some(server.clone()));
    assert!(c.set_rotation(1));
    let observations = server.observations.lock().unwrap();
    assert_eq!(observations.len(), 8);
    assert_eq!(observations[0].kind, ViewMessageKind::ViewZRChange1);
    assert_eq!(observations[0].rotation, 1);
    assert_eq!(observations[0].projection, old_projection);
    assert!(observations[1].projection > old_projection);
    assert_eq!(observations[6].rotation, 2);
    assert_eq!(observations[6].kind, ViewMessageKind::ViewZRChange2);
    assert_eq!(observations[7].kind, ViewMessageKind::ViewZRChange3);
    for i in [6, 7] {
        assert_eq!(observations[i].message, observations[1].message);
        assert_eq!(observations[i].data1, observations[0].data1);
        assert_eq!(observations[i].data2, observations[0].data2);
    }
    assert_eq!(observations[1].data1, &c as *const CameraControl as usize);
    assert_eq!(observations[1].data2, 0);
}
