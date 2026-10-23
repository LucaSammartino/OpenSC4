// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::network_occupants::xform::{Xform, XFORM_MATRIX, XFORM_SCALE};
use std::sync::Mutex;

fn camera() -> CameraHandle {
    Arc::new(RwLock::new(S3dCamera::new()))
}
fn fixture() -> CameraControl {
    let mut c = CameraControl::new();
    c.init([2048.0, 0.0, 2048.0], camera());
    c.take_messages();
    c
}
fn kinds(c: &CameraControl) -> Vec<ViewMessageKind> {
    c.messages.iter().map(|m| m.kind).collect()
}
fn post(kind: u32) -> Vec<ViewMessageKind> {
    use ViewMessageKind::*;
    match kind {
        0 => vec![PostViewInit],
        2 => vec![CameraUpdated, ViewZRChange2, ViewZRChange3],
        3 => vec![CameraUpdated, ViewTChange2, ViewTChange3],
        4 => vec![PostViewRefresh],
        _ => vec![],
    }
}
fn sequence(kind: u32) -> Vec<ViewMessageKind> {
    let mut result = vec![];
    if kind == 2 {
        result.push(ViewMessageKind::ViewZRChange1);
    }
    result.extend(post(kind));
    result
}

#[test]
fn ca7_constructor_positive_zeros_and_live_preinit_conversions() {
    let mut c = CameraControl::new();
    for value in c
        .plane
        .into_iter()
        .chain(c.eye_plane)
        .chain(c.eye_pixels)
        .chain(c.previous_eye_points)
    {
        assert_eq!(value.to_bits(), 0);
    }
    assert_eq!((c.metres_per_pixel, c.pixels_per_metre), (1.0, 1.0));
    let cam = camera();
    c.attach_to_camera(cam.clone());
    assert_eq!(c.convert_world_point_to_camera_plane_point([2.0, 3.0, 7.0]), [2.0, 3.0]);
    assert_eq!(c.convert_camera_plane_point_to_world_point([2.0, 3.0]), [2.0, 3.0, 0.0]);
    assert_eq!(c.axes, [[0.0; 3], [0.0, 1.0, 0.0]]);
    assert_eq!(c.plane, [0.0; 4]);
    let mut local = Xform {
        mask: XFORM_SCALE,
        scale: 0.5,
        ..Xform::default()
    };
    local.counter = 1;
    cam.write().unwrap().set_local_xform(local.clone());
    assert_eq!(c.convert_world_point_to_camera_plane_point([2.0, 3.0, 7.0]), [4.0, 6.0]);
    assert_eq!(c.convert_camera_plane_point_to_world_point([4.0, 6.0]), [2.0, 3.0, 0.0]);
    assert_eq!((c.metres_per_pixel, c.pixels_per_metre), (1.0, 1.0));
    // A second external change must be read without a control position update.
    local.counter = 2;
    local.scale = 0.25;
    local.translation = [100.0; 3];
    cam.write().unwrap().set_local_xform(local);
    assert_eq!(
        c.convert_world_point_to_camera_plane_point([2.0, 3.0, 7.0]),
        [8.0, 12.0]
    );
    assert_eq!(
        c.convert_camera_plane_point_to_world_point([8.0, 12.0]),
        [2.0, 3.0, 0.0]
    );
}

#[test]
fn ca7_eye_and_pixel_history_survives_detach_without_conversion_fallback() {
    let mut c = fixture();
    assert_ne!(c.eye, c.target);
    for target in [[1000.0, 0.0, 2000.0], [1200.0, 10.0, 1800.0]] {
        let previous = [c.eye_plane[0], c.eye_plane[1], c.eye_pixels[0], c.eye_pixels[1]];
        c.set_view_target(target);
        let cam = c.camera().unwrap();
        let cam = cam.read().unwrap();
        let view = cam.view_xform();
        let delta = sub(c.eye, c.plane_origin);
        let expected = [
            dot(delta, row(&view.matrix, 0)) * view.scale,
            dot(delta, row(&view.matrix, 1)) * view.scale,
        ];
        assert_eq!(c.eye_camera_plane_point(), expected);
        assert_eq!(c.previous_eye_camera_plane_points(), previous);
        let offset = cam.pixel_offset();
        assert_eq!(
            c.eye_camera_plane_pixels(),
            [
                c.pixels_per_metre * expected[0] + offset[0],
                c.pixels_per_metre * expected[1] + offset[1]
            ]
        );
    }
    let retained = (c.plane, c.eye_plane, c.eye_pixels, c.previous_eye_points);
    c.detach_from_camera();
    assert_eq!((c.plane, c.eye_plane, c.eye_pixels, c.previous_eye_points), retained);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        || c.convert_world_point_to_camera_plane_point([0.0; 3])
    ))
    .is_err());
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        || c.convert_camera_plane_point_to_world_point([0.0; 2])
    ))
    .is_err());
}

#[test]
fn ca7_inverse_keeps_zero_depth_terms_and_float_reciprocal() {
    let mut c = CameraControl::new();
    let cam = camera();
    c.attach_to_camera(cam.clone());
    let mut xform = Xform {
        counter: 1,
        mask: XFORM_MATRIX | XFORM_SCALE,
        scale: 3.0,
        ..Xform::default()
    };
    cam.write().unwrap().set_local_xform(xform.clone());
    let view_scale = cam.read().unwrap().view_xform().scale;
    let reciprocal = 1.0_f32 / view_scale;
    assert_eq!(
        c.convert_camera_plane_point_to_world_point([7.0, 11.0]),
        [7.0 * reciprocal, 11.0 * reciprocal, 0.0]
    );
    xform.counter = 2;
    // Transposition places this nonfinite value in the view back row.
    xform.matrix[2] = f32::INFINITY;
    cam.write().unwrap().set_local_xform(xform);
    assert!(c.convert_camera_plane_point_to_world_point([0.0, 0.0])[0].is_nan());
}

#[test]
fn ca8_rotation_commands_wrap_preserve_magnification_and_use_signed_remainders() {
    let mut c = fixture();
    c.current = ZoomRotation { zoom: 2, rotation: 3 };
    c.magnification = 1.25;
    assert!(c.rotate_left());
    assert_eq!(c.previous, ZoomRotation { zoom: 2, rotation: 3 });
    assert_eq!(c.new, ZoomRotation { zoom: 2, rotation: 0 });
    assert_eq!(c.magnification, 1.25);
    assert_eq!(kinds(&c), sequence(2));
    c.take_messages();
    assert!(c.rotate_right());
    assert_eq!(c.rotation(), 3);
    assert_eq!(kinds(&c), sequence(2));
    for left in [true, false] {
        let original = c.rotation();
        for _ in 0..4 {
            assert!(if left { c.rotate_left() } else { c.rotate_right() });
        }
        assert_eq!(c.rotation(), original);
    }
    let mut c = CameraControl::new();
    assert!(!c.rotate_left());
    assert_eq!(c.current, ZoomRotation { zoom: 0, rotation: 1 });
    assert_eq!(c.new, c.current);
    assert!(c.messages.is_empty());
    c.current.rotation = 0x8000_0000;
    let pairs = (c.previous, c.current, c.new);
    assert!(c.rotate_left());
    assert_eq!((c.previous, c.current, c.new), pairs);
    assert!(c.rotate_right());
    assert_eq!((c.previous, c.current, c.new), pairs);
    c.current.rotation = u32::MAX;
    assert!(!c.rotate_left());
    assert_eq!(c.rotation(), 0);
}

#[test]
fn ca8_zoom_commands_threshold_boundaries_and_false_after_mutation() {
    for (m, zin, zout) in [(1.0, 3, 1), (1.5, 3, 1), (f32::from_bits(1.5_f32.to_bits() + 1), 4, 2)] {
        for (inward, expected) in [(true, zin), (false, zout)] {
            let mut c = fixture();
            c.current.zoom = 2;
            c.magnification = m;
            assert!(if inward { c.zoom_in() } else { c.zoom_out() });
            assert_eq!(c.zoom(), expected);
            assert_eq!(c.magnification, 1.0);
            let mut messages = vec![];
            if m != 1.0 {
                messages.push(ViewMessageKind::PostViewRefresh);
            }
            messages.extend(sequence(2));
            assert_eq!(kinds(&c), messages);
        }
    }
    for m in [1.5, 2.0] {
        let mut c = fixture();
        c.current.zoom = 4;
        c.magnification = m;
        assert!(!c.zoom_in());
        assert_eq!((c.zoom(), c.magnification), (4, m));
        assert!(c.messages.is_empty());
    }
    let mut c = fixture();
    c.current.zoom = 4;
    c.magnification = 2.0;
    assert!(c.zoom_out());
    assert_eq!((c.zoom(), c.magnification), (4, 1.0));
    assert_eq!(
        kinds(&c),
        [vec![ViewMessageKind::PostViewRefresh], sequence(2)].concat()
    );
    let mut c = CameraControl::new();
    assert!(!c.zoom_out());
    assert_eq!(c.current.zoom, 0);
    c.current.zoom = 2;
    assert!(!c.zoom_in());
    assert_eq!(c.previous.zoom, 2);
    assert_eq!(c.current.zoom, 3);
    assert_eq!(c.new.zoom, 3);
    assert!(c.messages.is_empty());
    // Signed wrapped words, not saturating float casts or repaired save data.
    c.current.zoom = u32::MAX;
    assert!(!c.zoom_out());
    assert!(!c.zoom_in()); // v=-1, delegated request 0 mutates camera-less state.
    assert_eq!(c.current.zoom, 0);
    c.current.zoom = 0x8000_0000;
    assert!(!c.zoom_in()); // delegated negative float refused.
    assert_eq!(c.current.zoom, 0x8000_0000);
}

#[derive(Debug, PartialEq)]
enum Event {
    View {
        bounds: usize,
        eye: Vec3,
    },
    Bounds,
    Message {
        kind: ViewMessageKind,
        address: usize,
        target: Vec3,
    },
}
#[derive(Debug, Default)]
struct Host(Mutex<Vec<Event>>);
impl CameraViewUtilities for Host {
    fn city_corners(&self, _: u32, _: Vec2) -> [Vec3; 4] {
        self.0.lock().unwrap().push(Event::Bounds);
        [[0.0; 3]; 4]
    }
    fn update_camera_position(&self, c: &CameraControl) {
        assert_eq!(c.camera().unwrap().read().unwrap().position(), c.eye);
        self.0.lock().unwrap().push(Event::View {
            bounds: c.bounds.len(),
            eye: c.eye,
        });
    }
}
impl ViewMessageServer for Host {
    fn send(&self, c: &mut CameraControl, message: &LiveViewMessage) {
        self.0.lock().unwrap().push(Event::Message {
            kind: message.kind,
            address: message as *const _ as usize,
            target: c.target,
        });
    }
}

#[test]
fn ca9_all_kinds_guard_bounds_and_preserve_pairs_with_synchronous_order() {
    for kind in [0, 1, 2, 3, 4, 5, u32::MAX] {
        for empty in [false, true] {
            for view_present in [false, true] {
                let mut c = fixture();
                // Keep clamp from moving the target in this dispatch matrix.
                c.bounds_in_use = false;
                c.bounds = if empty { vec![] } else { vec![[123.0; 2]; 4] };
                // Enable generation, but avoid clamp with a containing polygon.
                if view_present {
                    c.bounds_in_use = true;
                    if !empty {
                        c.bounds = vec![[-1e6, -1e6], [1e6, -1e6], [1e6, 1e6], [-1e6, 1e6]];
                    }
                }
                // The provided corners yield narrow bounds; disable clamp only
                // by placing the target at the rotation corner for kind 2.
                c.target = [0.0; 3];
                let old_bounds = c.bounds.clone();
                let pairs = (c.previous, c.current, c.new);
                c.metres_per_pixel = 123.0;
                let host = Arc::new(Host::default());
                c.set_message_server(Some(host.clone()));
                if view_present {
                    c.set_view_utilities(Some(host.clone()));
                }
                assert!(c.update(kind));
                assert_eq!((c.previous, c.current, c.new), pairs);
                assert_eq!(c.metres_per_pixel == 123.0, kind == 3);
                assert_eq!(kinds(&c), sequence(kind));
                let events = host.0.lock().unwrap();
                let view_index = events.iter().position(|e| matches!(e, Event::View { .. }));
                let bounds_index = events.iter().position(|e| matches!(e, Event::Bounds));
                assert_eq!(view_index.is_some(), view_present);
                let rebuild = view_present && (empty || matches!(kind, 1 | 2));
                assert_eq!(bounds_index.is_some(), rebuild);
                if let Some(index) = view_index {
                    assert!(matches!(events[index], Event::View { bounds, .. } if bounds == old_bounds.len()));
                    assert_eq!(index, usize::from(kind == 2));
                    if let Some(bounds) = bounds_index {
                        assert_eq!(bounds, index + 1);
                    }
                }
                if !rebuild {
                    assert_eq!(c.bounds, old_bounds);
                }
                let addresses: Vec<_> = events
                    .iter()
                    .filter_map(|e| match e {
                        Event::Message {
                            kind: ViewMessageKind::ViewZRChange1,
                            ..
                        } => None,
                        Event::Message { address, .. } => Some(*address),
                        _ => None,
                    })
                    .collect();
                assert!(addresses.windows(2).all(|p| p[0] == p[1]));
                if let Some(index) = view_index {
                    for event in &events[index + 1..] {
                        if let Event::Message { kind, .. } = event {
                            assert_ne!(*kind, ViewMessageKind::ViewZRChange1);
                        }
                    }
                }
            }
        }
        let mut c = CameraControl::new();
        let host = Arc::new(Host::default());
        c.set_view_utilities(Some(host.clone()));
        c.set_message_server(Some(host.clone()));
        assert!(!c.update(kind));
        assert!(host.0.lock().unwrap().is_empty());
        assert!(c.messages.is_empty());
        assert_eq!(c.eye, [0.0; 3]);
    }
}

#[test]
fn ca9_helpers_without_camera_and_kind_two_clamps_after_zr_messages() {
    let mut c = CameraControl::new();
    for kind in [0, 1, 2, 3, 4, 5, u32::MAX] {
        c.send_pre_view_change(kind);
        c.send_post_view_change(kind);
        assert_eq!(kinds(&c), sequence(kind));
        c.take_messages();
    }
    for kind in [1, 2, 3, 4, 5] {
        for rotation_command in [false, true] {
            if rotation_command && kind != 2 {
                continue;
            }
            let mut c = fixture();
            if rotation_command {
                c.current.rotation = 3;
            }
            let corner = c.convert_world_point_to_camera_plane_point([0.0; 3]);
            c.bounds = vec![
                [corner[0] - 10.0, corner[1] - 10.0],
                [corner[0] + 10.0, corner[1] - 10.0],
                [corner[0] + 10.0, corner[1] + 10.0],
                [corner[0] - 10.0, corner[1] + 10.0],
            ];
            let bounds = c.bounds.clone();
            let target = c.target;
            c.velocity = [1.0; 3];
            let host = Arc::new(Host::default());
            c.set_message_server(Some(host.clone()));
            assert!(if rotation_command {
                c.set_rotation(0)
            } else {
                c.update(kind)
            });
            assert_eq!(c.bounds, bounds);
            let mut expected = sequence(kind);
            if kind == 2 {
                expected.extend(post(3));
                assert_ne!(c.target, target);
                assert_eq!(c.velocity, [0.0; 3]);
            } else {
                assert_eq!(c.target, target);
            }
            assert_eq!(kinds(&c), expected);
            let events = host.0.lock().unwrap();
            for (index, event) in events.iter().enumerate() {
                if let Event::Message { target: observed, .. } = event {
                    assert_eq!(*observed, if kind == 2 && index >= 4 { c.target } else { target });
                }
            }
        }
    }
}

#[test]
fn ca10_query_interface_same_address_output_before_addref_and_rejected_untouched() {
    for iid in [1, CAMERA_CONTROL_CLASS_ID, 0xE4FD_A3D4] {
        let mut owner = CameraControlAllocation::try_create().unwrap();
        let address = owner.control() as *const CameraControl;
        let mut output = std::ptr::null();
        assert!(owner
            .control_mut()
            .query_interface_with(iid, Some(&mut output), |c, out| {
                assert_eq!(*out, c as *const CameraControl);
                assert_eq!(c.references, 0);
                assert_eq!(c.add_ref(), 1);
            }));
        assert_eq!(output, address);
        assert_eq!(owner.control().references, 1);
        assert!(owner.control_mut().query_interface(iid, Some(&mut output)));
        assert_eq!(output, address);
        assert_eq!(owner.control().references, 2);
    }
    let mut c = CameraControl::new();
    let sentinel = &c as *const CameraControl;
    let mut output = sentinel;
    assert!(!c.query_interface(super::super::CAMERA_CLASS_ID, Some(&mut output)));
    assert!(!c.query_interface(super::super::CAMERA_CLASS_ID, None));
    assert_eq!(output, sentinel);
    assert_eq!(c.references, 0);
}

#[test]
fn ca10_unsigned_release_wrapping_addref_and_factory_failure() {
    let mut owner = CameraControlAllocation::try_create().unwrap();
    assert_eq!(owner.control().references, 0);
    assert_eq!(owner.control_mut().add_ref(), 1);
    assert_eq!(owner.control_mut().add_ref(), 2);
    assert_eq!(owner.release(), 1);
    assert_eq!(owner.control().references, 1);
    owner.control_mut().references = u32::MAX;
    assert_eq!(owner.control_mut().add_ref(), 0);
    assert_eq!(owner.control().references, 0);
    owner.control_mut().references = 0x8000_0000;
    assert_eq!(owner.release(), 0x7FFF_FFFF);
    assert_eq!(owner.control().references, 0x7FFF_FFFF);
    for count in [0, 1] {
        let mut owner = CameraControlAllocation::try_create().unwrap();
        owner.control_mut().references = count;
        owner.control_mut().bounds = vec![[0.0; 2]; 6];
        let mut calls = 0;
        assert_eq!(
            owner.release_with(|c, flags| {
                calls += 1;
                assert_eq!((c.references, flags, c.bounds.len()), (count, 1, 6));
            }),
            0
        );
        assert_eq!(calls, 1);
        assert!(owner.destroyed);
        assert_eq!(owner.storage.capacity(), 0);
    }
    assert!(CameraControlAllocation::try_create_with(|storage| {
        assert!(storage.is_empty());
        storage.try_reserve(usize::MAX)
    })
    .is_err());
}

#[test]
fn ca10_deleting_flags_free_bounds_always_object_only_bit_zero_never_camera() {
    for flags in [0, 1, 2, 3, 0x8000_0002, 0x8000_0003] {
        let mut owner = CameraControlAllocation::try_create().unwrap();
        let cam = camera();
        owner.control_mut().attach_to_camera(cam.clone());
        assert_eq!(cam.read().unwrap().reference_count(), 1);
        owner.control_mut().bounds = vec![[1.0; 2]; 6];
        let address = owner.control() as *const CameraControl;
        let returned = owner.deleting_destructor(flags);
        assert_eq!(returned, address);
        assert!(owner.destroyed);
        assert_eq!(owner.storage.is_empty(), flags & 1 != 0);
        if flags & 1 == 0 {
            assert_eq!(owner.storage[0].bounds.capacity(), 0);
        } else {
            assert_eq!(owner.storage.capacity(), 0);
        }
        assert_eq!(cam.read().unwrap().reference_count(), 1);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| owner.deleting_destructor(flags))).is_err());
        drop(owner);
        assert_eq!(cam.read().unwrap().reference_count(), 1);
    }
}
