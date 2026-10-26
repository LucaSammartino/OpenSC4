// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

// Independent native prose vectors, view-utilities §11.10–21 (published before
// this reconciliation). Expectations below are not derived from implementation.
#[test]
fn native_signed_rotation_partial_writes_and_wrapping_centres() {
    for (r, expected) in [
        (1, Vec3::new(-11.0, 7.0, 2.0)),
        (2, Vec3::new(-2.0, 7.0, -11.0)),
        (3, Vec3::new(11.0, 7.0, -2.0)),
        (-1, Vec3::new(2.0, 7.0, 11.0)),
        (-3, Vec3::new(2.0, 7.0, 11.0)),
        (5, Vec3::new(-11.0, 7.0, 2.0)),
    ] {
        let mut v = Vec3::new(2.0, 7.0, 11.0);
        rotate_vector_by_view_rotation(&mut v, r);
        assert_eq!(v, expected);
    }
    let mut v = Vec3::new(9.0, 8.0, 7.0);
    get_vector_for_edge_into(-1, &mut v);
    assert_eq!(v, Vec3::new(9.0, 0.0, 7.0));
    assert_eq!(PixelRect::new(i32::MAX, 0, i32::MAX, 0).center().x, -1);
    assert_eq!(round_down_to_nearest_block_boundary(-65.0, -64), Some(0));
    assert_eq!(round_up_to_nearest_block_boundary(65.0, -64), Some(0));
}

#[test]
fn native_triangle_line_metric_tolerance_barycentrics_and_nan() {
    let a = Vec3::ZERO;
    let b = Vec3::new(1.0, 0.0, 0.0);
    let c = Vec3::new(0.0, 1.0, 0.0);
    for (b, c) in [(b, c), (c, b)] {
        let mut p = Pick {
            ray: Ray {
                origin: Vec3::new(0.25, 0.25, 2.0),
                direction: Vec3::new(0.0, 0.0, 2.0),
            },
            nearest_distance: 3.0,
        };
        assert!(triangle_pick(&mut p, a, b, c));
        assert_eq!(p.nearest_distance, 2.0);
        assert!(!triangle_pick(&mut p, a, b, c));
        let sentinel = Vec3::new(9.0, 8.0, 7.0);
        let mut hit = sentinel;
        p.ray.direction.z = 1.0 / 128.0;
        assert!(!ray_intersects_triangle(p.ray, a, b, c, Some(&mut hit)));
        assert_eq!(hit, sentinel);
        p.ray.direction.z = 1.0 / 64.0;
        assert!(ray_intersects_triangle(p.ray, a, b, c, Some(&mut hit)));
        assert_eq!(hit, Vec3::new(0.25, 0.25, 0.0));
        p.ray.direction.z = f32::NAN;
        assert!(ray_intersects_triangle(p.ray, a, b, c, Some(&mut hit)));
        assert!(hit.x.is_nan());
        assert!(!triangle_pick(&mut p, a, b, c));
    }
}

#[test]
fn native_plane_tolerance_writes_and_negative_t() {
    let mut out = Vec3::new(9.0, 8.0, 7.0);
    let mut ray = Ray {
        origin: Vec3::new(3.0, 1.0, 4.0),
        direction: Vec3::new(1.0, 1.0 / 128.0, 0.0),
    };
    assert!(!ray_intersects_ground_plane(ray, &mut out));
    assert_eq!(out, Vec3::ZERO);
    ray.origin.y = 1.0 / 256.0;
    assert!(ray_intersects_ground_plane(ray, &mut out));
    assert_eq!(out, ray.origin);
    ray.origin.y = 2.0;
    ray.direction = Vec3::new(0.0, 1.0, 0.0);
    assert!(ray_intersects_ground_plane(ray, &mut out));
    assert_eq!(out, Vec3::new(3.0, 0.0, 4.0));
    assert!(ray_intersects_plane(ray, Vec3::new(0.0, 1.0, 0.0), 0.0, None));
}

#[test]
fn native_minmax_angles_and_rotation_word_rules() {
    for (zoom, bits) in [0x3f06_0a92, 0x3f1c_61aa, 0x3f32_b8c2, 0x3f49_0fdb, 0x3f49_0fdb]
        .into_iter()
        .enumerate()
    {
        assert_eq!(
            get_camera_view_angles_for_rotation(0, zoom as u32)
                .unwrap()
                .pitch
                .to_bits(),
            bits
        );
    }
    assert_eq!(min_of_four_floats([f32::NAN; 4]), f32::MAX);
    assert_eq!(max_of_four_floats([f32::NAN; 4]), -f32::MAX);
    assert_eq!(min_of_four_floats([f32::INFINITY; 4]), f32::MAX);
    assert_eq!(max_of_four_floats([f32::NEG_INFINITY; 4]), -f32::MAX);
    assert_eq!(
        min_of_four_floats([-0.0, 0.0, 1.0, f32::NAN]).to_bits(),
        (-0.0_f32).to_bits()
    );
    let y = f32::from_bits(0x7fa0_0001); // y is a raw untouched word
    let mut v = Vec3::new(0.0, y, -0.0);
    rotate_vector_by_view_rotation(&mut v, 1);
    assert_eq!(v.x.to_bits(), 0);
    assert_eq!(v.z.to_bits(), 0);
    assert_eq!(v.y.to_bits(), y.to_bits());
    let mut v = owner(camera());
    v.camera.rotation = -3;
    assert_eq!(v.get_visible_edges(), (-2, -1));
    v.camera.rotation = i32::MAX;
    assert_eq!(v.get_visible_edges(), (0, -3));
}

#[test]
fn native_reachable_aabb_and_aliasing_cross_helpers() {
    let nan = Vec3::new(f32::NAN, 2.0, 3.0);
    let points = [nan, Vec3::new(-10.0, -20.0, 30.0)];
    let bounds = point_array_aabb(&points, 2).unwrap();
    assert!(bounds.min.x.is_nan() && bounds.max.x.is_nan());
    assert_eq!((bounds.min.y, bounds.max.z), (-20.0, 30.0));
    assert_eq!(
        point_array_aabb(&points[1..], -1),
        Some(Aabb {
            min: points[1],
            max: points[1]
        })
    );
    assert_eq!(point_array_aabb(&[], 0), None);
    for (output, expected) in [
        (2, Vec3::new(-3.0, 6.0, -3.0)),
        (0, Vec3::new(-3.0, 30.0, -135.0)),
        (1, Vec3::new(-3.0, -15.0, -9.0)),
    ] {
        let mut storage = [Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0), Vec3::ZERO];
        assert_eq!(cross_in_storage(&mut storage, 0, 1, output), Some(()));
        assert_eq!(storage[output], expected);
    }
}

#[test]
fn native_matrix_rows_and_checked_model_abort() {
    let mut transform = Transform {
        kind: 4,
        matrix: [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]],
        translation: Vec3::new(10.0, 20.0, 30.0),
        scale: 2.0,
    };
    let p = Vec3::new(1.0, 2.0, 4.0);
    assert_eq!(transform.apply(p), Vec3::new(17.0, 38.0, 59.0));
    transform.kind = 5;
    assert_eq!(transform.apply(p), Vec3::new(27.0, 58.0, 89.0));
    transform.kind = 255;
    assert_eq!(transform.apply(p), Vec3::new(44.0, 96.0, 148.0));
    struct Instance(AtomicUsize);
    impl ModelInstance for Instance {
        fn fill_transform(&self, _: usize, _: &mut Transform) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let instance = Instance(AtomicUsize::new(0));
    let mut invalid = triangle_mesh(&[8.0]);
    invalid.indices = Some(vec![99, 1, 2]);
    let model = Model {
        meshes: vec![invalid, triangle_mesh(&[2.0])],
    };
    let mut p = pick();
    assert_eq!(model_pick_checked(&mut p, &instance, &model), Err(InvalidMeshReference));
    assert_eq!(instance.0.load(Ordering::SeqCst), 1);
    assert_eq!(p.nearest_distance, 100.0);
}

#[test]
fn native_integer_conversion_low_words_and_checked_faults() {
    // §11.18 gives INT_MAX, not representable by the current f32 entry adapter.
    // Exercise the specified post-truncation integer operation pending VU15.
    assert_eq!(round_block_integer(i32::MAX, 64, true), Some(-2_147_483_584));
    assert_eq!(round_down_to_nearest_block_boundary(4_294_967_808.0, 64), Some(512));
    assert_eq!(round_up_to_nearest_block_boundary(f32::INFINITY, 64), Some(0));
    assert_eq!(round_down_to_nearest_block_boundary(-2_147_483_648.0, -1), None);
    assert_eq!(round_up_to_nearest_block_boundary(2.0, 0), None);
    let mut out = [9, 8, 7];
    let bytes: Vec<_> = [f32::NAN, 0.0, 0.0].into_iter().flat_map(f32::to_le_bytes).collect();
    assert_eq!(decode_indices(6, &bytes, [0, 1, 2], &mut out), None);
    assert_eq!(out, [9, 8, 7]);
}

#[test]
fn native_clipping_order_winding_duplicates_empty_and_undefined_crossing() {
    let p = |x, z| Vec3::new(x, 0.0, z);
    let clip = [p(0.0, 0.0), p(0.0, 4.0), p(4.0, 4.0), p(4.0, 0.0)];
    let subject = [p(1.0, 1.0), p(1.0, 3.0), p(3.0, 3.0), p(3.0, 1.0)];
    assert_eq!(clip_polygon_to_polygon_2d(&subject, &clip, 10).unwrap(), subject);
    let boundary = [p(0.0, 0.0), subject[1], subject[2], subject[3]];
    assert_eq!(clip_polygon_to_polygon_2d(&boundary, &clip, 10).unwrap(), boundary);
    let duplicate = [p(-1.0, 0.0), p(0.0, 0.0), p(0.0, 1.0), p(-1.0, 1.0)];
    assert_eq!(
        clip_polygon_to_polygon_2d(&duplicate, &clip, 10).unwrap(),
        [p(0.0, 0.0), p(0.0, 0.0), p(0.0, 1.0), p(0.0, 1.0)]
    );
    let mut reverse = clip;
    reverse.reverse();
    assert!(clip_polygon_to_polygon_2d(&subject, &reverse, 10).unwrap().is_empty());
    let disjoint = [p(8.0, 8.0), p(8.0, 9.0), p(9.0, 9.0), p(9.0, 8.0)];
    assert!(clip_polygon_to_polygon_2d(&disjoint, &clip, 10).unwrap().is_empty());
    let tiny = [p(-0.001, 1.0), p(0.001, 1.0), p(0.001, 2.0), p(-0.001, 2.0)];
    assert_eq!(
        clip_polygon_to_polygon_2d(&tiny, &clip, 10),
        Err(IndeterminateClipIntersection)
    );
}

#[test]
fn native_raw_iterator_and_index_formats_preserve_outputs_on_stop() {
    let mut out = [9, 8, 7];
    primitive_vertex_indices(99, 0, &mut out);
    assert_eq!(out, [9, 8, 7]);
    primitive_vertex_indices(0, u32::MAX, &mut out);
    assert_eq!(out, [u32::MAX - 2, u32::MAX - 1, u32::MAX]);
    primitive_vertex_indices(7, u32::MAX, &mut out);
    assert_eq!(out, [u32::MAX - 1, u32::MAX, 0]);
    for (format, data, expected) in [
        (0, vec![255, 0, 127], [u32::MAX, 0, 127]),
        (1, vec![255, 0, 127], [255, 0, 127]),
        (2, vec![255, 255, 0, 0, 0, 128], [u32::MAX, 0, 0xffff_8000]),
        (3, vec![255, 255, 0, 0, 0, 128], [65535, 0, 32768]),
    ] {
        assert_eq!(decode_indices(format, &data, [0, 1, 2], &mut out), Some(()));
        assert_eq!(out, expected);
    }
    let raw = [u32::MAX, 0, 0x8000_0000];
    let bytes: Vec<_> = raw.into_iter().flat_map(u32::to_le_bytes).collect();
    for format in [4, 5] {
        assert_eq!(decode_indices(format, &bytes, [0, 1, 2], &mut out), Some(()));
        assert_eq!(out, raw);
    }
    for format in [6, 7] {
        let bytes: Vec<_> = if format == 6 {
            [-1.25_f32, 2.0, 3.75].into_iter().flat_map(f32::to_le_bytes).collect()
        } else {
            [-1.25_f64, 2.0, 3.75].into_iter().flat_map(f64::to_le_bytes).collect()
        };
        assert_eq!(decode_indices(format, &bytes, [0, 1, 2], &mut out), Some(()));
        assert_eq!(out, [(-2_i32) as u32, 2, 3]);
    }
    let sentinel = out;
    assert_eq!(decode_indices(99, &[], [0, 1, 2], &mut out), Some(()));
    assert_eq!(out, sentinel);
    assert_eq!(decode_indices(3, &[], [0, 1, 2], &mut out), None);
    assert_eq!(out, sentinel);
    let mesh = Mesh {
        groups: vec![PrimitiveGroup {
            kind: 1,
            first: 0,
            count: 1,
        }],
        ..Mesh::default()
    };
    assert_eq!(mesh.groups[0].triangle_count(), -1);
    let mut iterator = TriangleIterator::select_group(&mesh, 0).unwrap();
    let mut triangle = [Vec3::new(9.0, 8.0, 7.0); 3];
    assert_eq!(iterator.next_triangle(&mut triangle), Ok(false));
    assert_eq!(triangle, [Vec3::new(9.0, 8.0, 7.0); 3]);
    let mut mesh = triangle_mesh(&[8.0, 2.0]);
    mesh.indices = Some(vec![99, 1, 2, 3, 4, 5]);
    assert_eq!(
        mesh_pick_checked(&mut pick(), &mesh, &Transform::default()),
        Err(InvalidMeshReference)
    );
}

#[test]
fn native_offcentre_and_missing_camera_draw_state() {
    let c = camera();
    let mut shifted = c;
    shifted.volume.left += 3.0;
    shifted.volume.right += 3.0;
    let a = owner(c);
    let b = owner(shifted);
    assert_eq!(a.view_frustum().corners, b.view_frustum().corners);
    close(
        b.project_point_to_viewport_coordinates(Vec3::ZERO).x - a.project_point_to_viewport_coordinates(Vec3::ZERO).x,
        -3.0 * a.scales().0.x,
    );
    let rays = Rays::new(0.0);
    let mut v = owner(c);
    assert!(v.build_draw_frustum_data_from_viewport_rect(PixelRect::new(1, 2, 99, 98), Some(&rays)));
    let previous = *v.draw_frustum();
    assert!(!v.update_camera_position(None));
    let rect = PixelRect::new(3, 4, 89, 88);
    assert!(!v.build_draw_frustum_data_from_viewport_rect(rect, None));
    assert_eq!(v.draw_rect(), rect);
    assert!(v.draw_is_narrowed());
    assert_eq!(*v.draw_frustum(), previous);
    assert!(v.viewport_ground.get().is_none());
    assert!(v.visible_area.get().is_none());
}

#[test]
fn native_independent_cache_observations() {
    use std::cell::Cell;
    struct Changing(Cell<usize>);
    impl CameraRayProvider for Changing {
        fn eye_ray(&self, x: f32, y: f32) -> Option<Ray> {
            let count = self.0.get();
            self.0.set(count + 1);
            Some(Ray {
                origin: Vec3::new(x + (count / 4) as f32 * 1000.0, 10.0, y),
                direction: Vec3::new(0.0, -1.0, 0.0),
            })
        }
    }
    for rect in [PixelRect::new(0, 0, 200, 100), PixelRect::new(20, 10, 180, 90)] {
        let mut v = owner(camera());
        let rays = Changing(Cell::new(0));
        v.build_draw_frustum_data_from_viewport_rect(rect, Some(&rays));
        assert_eq!(rays.0.get(), 4);
        let area = v.get_ground_plane_visible_area_rectangle(Some(&rays)).unwrap();
        assert_eq!(rays.0.get(), 8);
        assert_eq!(area.inner.min.x, 1100.0);
        assert_eq!(v.get_ground_plane_visible_area_rectangle(Some(&rays)), Some(area));
        assert_eq!(rays.0.get(), 8);
    }
}
use std::sync::atomic::{AtomicUsize, Ordering};

fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 0.0001, "{a} != {b}");
}
fn vector(a: Vec3, b: Vec3) {
    close(a.x, b.x);
    close(a.y, b.y);
    close(a.z, b.z);
}
fn camera() -> CameraState {
    CameraState {
        position: Vec3::new(100.0, 100.0, 100.0),
        look_at: Vec3::new(100.0, 99.0, 99.0),
        zoom: 4,
        rotation: 0,
        viewport_width: 200,
        viewport_height: 100,
        near_clip_distance: 9.0,
        far_clip_distance: 999.0,
        metres_per_pixel: 0.1,
        pixels_per_metre: 10.0,
        volume: ViewVolume {
            left: -10.0,
            right: 10.0,
            bottom: -5.0,
            top: 5.0,
            near: 2.0,
            far: 102.0,
        },
        projection: Projection::Orthographic,
    }
}
fn owner(c: CameraState) -> ViewUtilities {
    let mut v = ViewUtilities::new();
    v.set_renderer_present(true);
    v.set_city_size(1024.0, 512.0);
    assert!(v.update_camera_position(Some(&c)));
    v
}
struct Rays {
    calls: AtomicUsize,
    offset: f32,
}
impl Rays {
    fn new(offset: f32) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            offset,
        }
    }
}
impl CameraRayProvider for Rays {
    fn eye_ray(&self, x: f32, y: f32) -> Option<Ray> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        // Independent camera input: downwards rays, ground x=x+offset,z=y.
        Some(Ray {
            origin: Vec3::new(x + self.offset, 10.0, y),
            direction: Vec3::new(0.0, -2.0, 0.0),
        })
    }
}

#[test]
fn construction_clears_owned_fields_and_keeps_constructor_constants() {
    let v = ViewUtilities::new();
    assert!(!v.is_valid());
    assert_eq!(v.camera(), &CameraState::default());
    assert_eq!(v.view_frustum(), Frustum::default());
    assert_eq!(*v.draw_frustum(), Frustum::default());
    assert_eq!(v.get_zoom_and_rotation(), (0, 0));
    assert_eq!(v.unused_constructor_fields, [1.0, 1.0, 0.3]);
    assert_eq!(v.city, (0.0, 0.0));
    assert_eq!(v.basis(), (Vec3::ZERO, Vec3::ZERO, Vec3::ZERO));
    assert_eq!(v.scales(), (Vec3::ZERO, Vec3::ZERO));
}

#[test]
fn cached_getters_before_update_and_after_camera_failure() {
    let mut v = ViewUtilities::new();
    assert_eq!(v.get_frustum_aabb(), Aabb::default());
    assert_eq!(v.get_draw_ground_corners(), [(0.0, 0.0); 4]);
    // The native cached-angle getter evaluates the table using zero cached
    // zoom/rotation; it does not copy the constructor-zero angle fields.
    assert_eq!(v.angles(), ViewAngles::default());
    let angles = v.get_camera_view_angles().unwrap();
    assert_eq!(angles.yaw.to_bits(), 0xbec9_0fdb);
    assert_eq!(angles.pitch.to_bits(), 0x3f06_0a92);

    v = owner(camera());
    let rays = Rays::new(0.0);
    assert!(v.build_draw_frustum_data_from_viewport_rect(PixelRect::new(20, 10, 180, 90), Some(&rays)));
    let bounds = v.get_frustum_aabb();
    let corners = v.get_draw_ground_corners();
    let angles = v.get_camera_view_angles();
    assert_eq!(bounds, v.view_frustum().bounds);
    assert_eq!(corners, v.draw_frustum().ground_points.map(|p| (p.x, p.z)));
    let calls = rays.calls.load(Ordering::SeqCst);
    assert!(!v.update_camera_position(None));
    assert!(!v.is_valid());
    assert_eq!(v.get_frustum_aabb(), bounds);
    assert_eq!(v.get_draw_ground_corners(), corners);
    assert_eq!(v.get_camera_view_angles(), angles);
    assert_eq!(rays.calls.load(Ordering::SeqCst), calls);
}

#[test]
fn update_copies_inputs_scales_basis_and_does_not_depend_on_borrowed_storage() {
    let mut c = camera();
    let v = owner(c);
    c.position.x = -1000.0;
    assert_eq!(c.position.x, -1000.0);
    assert_eq!(v.camera().position.x, 100.0);
    assert_eq!(v.get_zoom_and_rotation(), (4, 0));
    assert_eq!(v.camera().near_clip_distance, 9.0);
    assert_eq!(v.camera().far_clip_distance, 999.0);
    assert_eq!(v.scales().0, Vec3::new(10.0, 10.0, 0.01));
    vector(v.scales().1, Vec3::new(0.1, 0.1, 100.0));
    let (f, r, u) = v.basis();
    vector(f, Vec3::new(0.0, -0.70710677, -0.70710677));
    vector(r, Vec3::new(1.0, 0.0, 0.0));
    vector(u, Vec3::new(0.0, 0.70710677, -0.70710677));
    close(v.viewport_units_to_world_units(27.0), 2.7);
}

#[test]
fn failed_update_invalidates_both_caches_without_wiping_old_frustum() {
    let mut v = owner(camera());
    let rays = Rays::new(0.0);
    assert!(v.get_ground_plane_visible_area_rectangle(Some(&rays)).is_some());
    let old = v.view_frustum();
    assert!(!v.update_camera_position(None));
    assert!(!v.is_valid());
    assert!(v.visible_area.get().is_none());
    assert!(v.viewport_ground.get().is_none());
    assert_eq!(v.view_frustum().corners, old.corners);
    v.set_renderer_present(false);
    assert!(!v.update_camera_position(Some(&camera())));
    assert!(!v.is_valid());
}

#[test]
fn angle_contract_vectors_and_all_zoom_table_entries() {
    let a = get_camera_view_angles_for_rotation(1, 4).unwrap();
    close(a.yaw.to_degrees(), 67.5);
    close(a.pitch.to_degrees(), 45.0);
    close(
        get_camera_view_angles_for_rotation(0, 0)
            .unwrap()
            .ground_pitch
            .to_degrees(),
        32.002064,
    );
    for (zoom, expected) in [30.0, 35.0, 40.0, 45.0, 45.0].into_iter().enumerate() {
        close(
            get_camera_view_angles_for_rotation(3, zoom as u32)
                .unwrap()
                .pitch
                .to_degrees(),
            expected,
        );
    }
    close(get_object_y_rotation_angle(3), -4.712389);
    close(get_city_width_from_ortho_view(100.0, 0).unwrap(), 130.6563);
    assert!(get_camera_view_angles_for_rotation(0, 5).is_none());
    assert!(get_city_width_from_ortho_view(100.0, 5).is_none());
}

#[test]
fn coordinate_axes_and_translation_have_independent_expected_values() {
    let v = owner(camera());
    vector(
        v.project_point_to_fixed_view_coordinates(Vec3::new(1.0, 0.0, 0.0)),
        Vec3::new(0.9238795, -0.27059805, 0.27059805),
    );
    vector(
        v.project_point_to_fixed_view_coordinates(Vec3::new(0.0, 1.0, 0.0)),
        Vec3::new(0.0, 0.70710677, 0.70710677),
    );
    vector(
        v.project_point_to_fixed_view_coordinates(Vec3::new(0.0, 0.0, 1.0)),
        Vec3::new(-0.38268343, -0.65328145, 0.65328145),
    );
    vector(
        v.project_point_to_view_coordinates(Vec3::new(100.0, 101.0, 100.0)),
        Vec3::new(0.0, 0.70710677, 0.70710677),
    );
    vector(
        v.unproject_point_from_view_coordinates_to_world_coordinates(Vec3::new(0.0, 1.0, 0.0)),
        Vec3::new(99.7294, 100.70711, 99.34672),
    );
}

#[test]
fn viewport_contract_bounds_depth_and_2d_depth_preservation() {
    let v = owner(camera());
    assert_eq!(
        v.project_point_from_view_coordinates_to_viewport_coordinates(Vec3::new(-10.0, 5.0, -2.0)),
        Vec3::ZERO
    );
    assert_eq!(
        v.project_point_from_view_coordinates_to_viewport_coordinates(Vec3::new(10.0, -5.0, -102.0)),
        Vec3::new(200.0, 100.0, -1.0)
    );
    let p = v.camera().position;
    vector(v.project_point_to_viewport_coordinates(p), Vec3::new(100.0, 50.0, 0.02));
    assert_eq!(v.project_point_to_viewport_coordinates_2d(p).z, 0.0);
    close(v.project_point_to_viewport_z(p), 0.02);
    vector(
        v.project_point_from_viewport_coordinates_to_fixed_view_coordinates(Vec3::new(100.0, 50.0, 0.02)),
        v.project_point_to_fixed_view_coordinates(p),
    );
}

#[test]
fn supplemental_transform_roundtrips_cover_rotations_zooms_and_noncentred_volume() {
    for rotation in 0..4 {
        for zoom in 0..5 {
            let mut c = camera();
            c.rotation = rotation;
            c.zoom = zoom;
            c.volume.left = -7.0;
            let v = owner(c);
            let p = Vec3::new(102.0, 98.0, 105.0);
            vector(
                v.unproject_point_from_view_coordinates_to_world_coordinates(v.project_point_to_view_coordinates(p)),
                p,
            );
            vector(
                v.unproject_point_from_viewport_coordinates_to_world_coordinates(
                    v.project_point_to_viewport_coordinates(p),
                ),
                p,
            );
        }
    }
}

#[test]
fn frustum_corner_order_and_plane_signs_follow_basis_not_euler_projection() {
    let v = owner(camera());
    let f = v.view_frustum();
    let expected = [
        Vec3::new(110.0, 102.12132, 95.050255),
        Vec3::new(110.0, 95.050255, 102.12132),
        Vec3::new(90.0, 95.050255, 102.12132),
        Vec3::new(90.0, 102.12132, 95.050255),
        Vec3::new(110.0, 31.410645, 24.339573),
        Vec3::new(110.0, 24.339573, 31.410645),
        Vec3::new(90.0, 24.339573, 31.410645),
        Vec3::new(90.0, 31.410645, 24.339573),
    ];
    for (a, b) in f.corners.into_iter().zip(expected) {
        vector(a, b);
    }
    vector(f.bounds.min, Vec3::new(90.0, 24.339573, 24.339573));
    vector(f.bounds.max, Vec3::new(110.0, 102.12132, 102.12132));
    let inside = Vec3::new(100.0, 64.64466, 64.64466);
    for plane in f.planes {
        assert!(plane.evaluate(inside) > 0.0);
    }
    close(f.planes[0].evaluate(Vec3::new(90.0, 0.0, 0.0)), 0.0);
    close(f.planes[1].evaluate(Vec3::new(110.0, 0.0, 0.0)), 0.0);
}

#[test]
fn native_perspective_reuses_previous_far_corners_and_compounds() {
    let mut c = camera();
    c.projection = Projection::Perspective;
    c.position = Vec3::new(1.0, 2.0, 3.0);
    c.look_at = Vec3::new(1.0, 2.0, 2.0);
    c.volume.near = 1.0;
    c.volume.far = 2.0;
    let mut v = owner(c);
    let first = v.view_frustum();
    assert_eq!(&first.corners[4..], &[Vec3::new(-1.0, -2.0, -3.0); 4]);
    assert!(v.update_camera_position(Some(&c)));
    assert_eq!(&v.view_frustum().corners[4..], &[Vec3::new(-3.0, -6.0, -9.0); 4]);
}

#[test]
fn clipped_city_polygon_is_clipped_as_a_polygon_not_as_its_aabb() {
    let mut c = camera();
    c.position = Vec3::new(0.0, 100.0, 100.0);
    c.look_at = Vec3::new(0.0, 99.0, 99.0);
    let mut v = owner(c);
    v.set_city_size(5.0, 95.0);
    v.update_camera_position(Some(&c));
    let b = v.get_frustum_aabb_city_clipped(true).unwrap();
    vector(b.min, Vec3::new(0.0, 0.0, 0.0));
    vector(b.max, Vec3::new(5.0, 102.12132, 95.0));
    c.position.x = -100.0;
    c.look_at.x = -100.0;
    v.update_camera_position(Some(&c));
    let b = v.get_frustum_aabb_city_clipped(true).unwrap();
    assert_eq!(b.min, Vec3::ZERO);
    vector(b.max, Vec3::new(0.0, 102.12132, 0.0));
}

#[test]
fn box_visibility_strict_side_boundaries_but_no_near_far_test() {
    let v = owner(camera());
    // On the axis 100 units behind the eye, and 400 beyond the far plane.
    for d in [-100.0, 500.0] {
        let p = Vec3::new(100.0, 100.0 - d, 100.0 - d);
        assert!(v.is_bounding_box_in_view_frustum(Aabb { min: p, max: p }));
    }
    let p = Vec3::new(90.0, 80.0, 80.0);
    assert!(!v.is_bounding_box_in_view_frustum(Aabb { min: p, max: p }));
    assert!(v.is_bounding_box_in_view_frustum(Aabb {
        min: p,
        max: Vec3::new(91.0, 81.0, 81.0)
    }));
    let p = Vec3::new(111.0, 80.0, 80.0);
    assert!(!v.is_bounding_box_in_view_frustum(Aabb { min: p, max: p }));
}

#[test]
fn rotated_city_clip_rejects_aabb_overlap_when_ground_polygon_misses_city() {
    let mut c = camera();
    c.look_at = Vec3::new(101.0, 99.0, 99.0);
    let mut v = owner(c);
    v.set_city_size(100.0, 50.0);
    v.update_camera_position(Some(&c));
    // This diagonal ground band has x+z >= 200-10*sqrt(2) > 185.
    // The city has x+z <= 150. Their AABBs overlap but the polygons do not.
    let b = v.get_frustum_aabb_city_clipped(true).unwrap();
    assert_eq!((b.min.x, b.min.z, b.max.x, b.max.z), (0.0, 0.0, 0.0, 0.0));
    assert!(v.view_frustum().corners[3].x < 100.0);
    assert!(b.max.y > 0.0);
}

#[test]
fn point_visibility_uses_inclusive_xy_and_strict_depth() {
    let mut v = ViewUtilities::new();
    // Isolate exact boundary comparisons with identity cached rotation.
    v.camera.volume = camera().volume;
    v.cos_negative_yaw = 1.0;
    v.cos_pitch = 1.0;
    assert!(v.is_point_in_view_frustum(Vec3::new(-10.0, 5.0, -3.0)));
    assert!(!v.is_point_in_view_frustum(Vec3::new(-10.001, 5.0, -3.0)));
    assert!(!v.is_point_in_view_frustum(Vec3::new(0.0, 0.0, -2.0)));
    assert!(!v.is_point_in_view_frustum(Vec3::new(0.0, 0.0, -102.0)));
    assert!(v.is_point_in_view_frustum(Vec3::new(10.0, -5.0, -101.0)));
}

#[test]
fn ground_ray_contract_and_checked_parallel_behind_surface_cases() {
    let r = Ray {
        origin: Vec3::new(0.0, 100.0, 0.0),
        direction: Vec3::new(0.0, -0.70710677, 0.70710677),
    };
    vector(get_ray_intersection_with_ground_plane(r), Vec3::new(0.0, 0.0, 100.0));
    let mut out = Vec3::new(7.0, 8.0, 9.0);
    assert!(ray_intersects_ground_plane(r, &mut out));
    vector(out, Vec3::new(0.0, 0.0, 100.0));
    assert!(ray_intersects_ground_plane(
        Ray {
            direction: Vec3::new(0.0, 1.0, 0.0),
            ..r
        },
        &mut out
    ));
    assert_eq!(out, Vec3::ZERO);
    let parallel = Ray {
        direction: Vec3::new(1.0, 0.0, 0.0),
        ..r
    };
    assert!(!ray_intersects_ground_plane(parallel, &mut out));
    assert_eq!(out, Vec3::ZERO);
    assert!(!get_ray_intersection_with_ground_plane(parallel).x.is_finite());
    assert!(ray_intersects_ground_plane(
        Ray {
            origin: Vec3::ZERO,
            direction: r.direction
        },
        &mut out
    ));
}

#[test]
fn camera_eye_ray_is_normalized_and_failures_preserve_direct_output() {
    let mut v = owner(camera());
    let rays = Rays::new(3.0);
    let mut out = Ray::default();
    assert!(v.viewport_position_to_ray(Some(&rays), 11.0, 12.0, &mut out));
    assert_eq!(
        out,
        Ray {
            origin: Vec3::new(14.0, 10.0, 12.0),
            direction: Vec3::new(0.0, -1.0, 0.0)
        }
    );
    let previous = out;
    assert!(!v.viewport_position_to_ray(None, 0.0, 0.0, &mut out));
    assert_eq!(out, previous);
    v.set_renderer_present(false);
    assert!(!v.viewport_position_to_ray(Some(&rays), 0.0, 0.0, &mut out));
    assert_eq!(out, previous);
    assert_eq!(rays.calls.load(Ordering::SeqCst), 1);
    let mut ground = Vec3::ZERO;
    assert!(!v.get_viewport_point_intersection_with_ground_plane(None, 0.0, 0.0, &mut ground));
    assert_eq!(ground, Vec3::ZERO); // checked adapter; no native failure coordinate
}

#[test]
fn ground_corner_order_visible_area_truncation_and_cache_invalidation() {
    let mut v = owner(camera());
    let rays = Rays::new(-0.5);
    assert_eq!(
        v.get_viewport_rect_ground_corners(Some(&rays), PixelRect::new(0, 0, 200, 100))
            .unwrap(),
        [
            Vec3::new(-0.5, 0.0, 0.0),
            Vec3::new(199.5, 0.0, 0.0),
            Vec3::new(-0.5, 0.0, 100.0),
            Vec3::new(199.5, 0.0, 100.0)
        ]
    );
    let a = v.get_ground_plane_visible_area_rectangle(Some(&rays)).unwrap();
    assert_eq!(
        a.inner,
        Aabb {
            min: Vec3::new(99.5, 0.0, 0.0),
            max: Vec3::new(99.5, 0.0, 100.0)
        }
    );
    assert_eq!(
        a.outer,
        Aabb {
            min: Vec3::ZERO,
            max: Vec3::new(256.0, 0.0, 128.0)
        }
    );
    let calls = rays.calls.load(Ordering::SeqCst);
    assert_eq!(v.get_ground_plane_visible_area_rectangle(None), Some(a));
    assert_eq!(rays.calls.load(Ordering::SeqCst), calls);
    v.build_draw_frustum_data_from_viewport_rect(PixelRect::new(10, 10, 20, 20), Some(&rays));
    assert!(v.visible_area.get().is_none());
    assert_eq!(v.get_ground_plane_visible_area_rectangle(Some(&rays)), Some(a));
    assert!(v.update_camera_position(Some(&camera())));
    let shifted = Rays::new(1000.0);
    assert_eq!(
        v.get_ground_plane_visible_area_rectangle(Some(&shifted))
            .unwrap()
            .inner
            .min
            .x,
        1100.0
    );
}

#[test]
fn cached_getters_can_be_shared_across_threads() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<ViewUtilities>();
    let v = owner(camera());
    let rays = Rays::new(0.0);
    std::thread::scope(|scope| {
        let a = scope.spawn(|| v.get_ground_plane_visible_area_rectangle(Some(&rays)));
        let b = scope.spawn(|| v.get_ground_plane_visible_area_rectangle(Some(&rays)));
        assert_eq!(a.join().unwrap(), b.join().unwrap());
    });
    assert_eq!(rays.calls.load(Ordering::SeqCst), 4);
}

#[test]
fn every_draw_rectangle_invalidates_both_caches_and_keeps_full_view_visible_area() {
    let mut v = owner(camera());
    let original = Rays::new(0.0);
    assert_eq!(
        v.get_ground_plane_visible_area_rectangle(Some(&original))
            .unwrap()
            .inner
            .min
            .x,
        100.0
    );
    let changed = Rays::new(1000.0);
    v.build_draw_frustum_data_from_viewport_rect(PixelRect::new(50, 25, 150, 75), Some(&changed));
    assert_eq!(
        v.get_ground_plane_visible_area_rectangle(Some(&changed))
            .unwrap()
            .inner
            .min
            .x,
        1100.0
    );
    assert_eq!(changed.calls.load(Ordering::SeqCst), 8); // four dirty + four fresh viewport corners
    let outside_draw = Aabb {
        min: Vec3::new(108.0, 80.0, 80.0),
        max: Vec3::new(108.0, 80.0, 80.0),
    };
    assert!(v.is_bounding_box_in_view_frustum(outside_draw));
    assert!(!v.is_bounding_box_in_draw_frustum(outside_draw));
    let inside_draw = Aabb {
        min: Vec3::new(100.0, 80.0, 80.0),
        max: Vec3::new(100.0, 80.0, 80.0),
    };
    assert!(v.is_bounding_box_in_draw_frustum(inside_draw));
    v.build_draw_frustum_data_from_viewport_rect(PixelRect::new(0, 0, 200, 100), Some(&original));
    assert_eq!(
        v.get_ground_plane_visible_area_rectangle(Some(&original))
            .unwrap()
            .inner
            .min
            .x,
        100.0
    );
}

#[test]
fn draw_full_copy_narrowed_extents_and_integer_centres() {
    let mut v = owner(camera());
    let rays = Rays::new(0.0);
    v.build_draw_frustum_data_from_viewport_rect(PixelRect::new(0, 0, 200, 100), Some(&rays));
    assert!(!v.draw_is_narrowed());
    assert_eq!(*v.draw_frustum(), v.view_frustum());
    v.build_draw_frustum_data_from_viewport_rect(PixelRect::new(11, 21, 110, 80), Some(&rays));
    assert!(v.draw_is_narrowed());
    assert_eq!(v.draw_rect(), PixelRect::new(11, 21, 110, 80));
    let volume = v.draw_frustum().volume;
    close(volume.left, -4.9);
    close(volume.right, 5.0);
    close(volume.top, 2.9);
    close(volume.bottom, -3.0);
    assert_eq!(volume.near, 2.0);
    assert_eq!(volume.far, 102.0);
    assert_eq!(
        v.draw_frustum().ground_points,
        [
            Vec3::new(11.0, 0.0, 21.0),
            Vec3::new(110.0, 0.0, 21.0),
            Vec3::new(11.0, 0.0, 80.0),
            Vec3::new(110.0, 0.0, 80.0)
        ]
    );
    assert_eq!(
        v.get_frustum_aabb_city_clipped(false),
        v.draw_frustum().city_clipped_bounds
    );
    v.build_draw_frustum_data_from_viewport_rect(PixelRect::new(0, 10, 200, 90), Some(&rays));
    assert_eq!(v.draw_frustum().volume.left, -10.0);
    assert_eq!(v.draw_frustum().volume.right, 10.0);
    v.build_draw_frustum_data_from_viewport_rect(PixelRect::new(10, 0, 190, 100), Some(&rays));
    assert_eq!(v.draw_frustum().volume.bottom, -5.0);
    assert_eq!(v.draw_frustum().volume.top, 5.0);
    assert_eq!(PixelRect::new(-4, -6, 1, 1).center(), Pixel::new(-1, -2));
}

#[test]
fn city_helpers_rotation_table_non_square_quirk_and_preserved_invalid_outputs() {
    let mut v = owner(camera());
    v.set_city_size(1024.0, 1024.0);
    let mut corner = Vec3::ZERO;
    assert!(v.get_city_corner_position(0, &mut corner));
    assert_eq!(corner, Vec3::new(0.0, 0.0, 1024.0)); // §11.5
    v.set_city_size(80.0, 48.0);
    let mut c = camera();
    c.rotation = 1;
    assert!(v.update_camera_position(Some(&c)));
    assert!(v.get_city_corner_position(2, &mut corner));
    assert_eq!(corner, Vec3::new(80.0, 0.0, 80.0)); // §11.16
    corner = Vec3::new(9.0, 8.0, 7.0);
    assert!(!v.get_city_corner_position(99, &mut corner));
    assert_eq!(corner, Vec3::new(9.0, 0.0, 7.0));

    let expected = [
        [(0.0, 512.0), (0.0, 0.0), (1024.0, 0.0), (1024.0, 512.0)],
        [(0.0, 0.0), (1024.0, 0.0), (1024.0, 512.0), (0.0, 512.0)],
        [(1024.0, 0.0), (1024.0, 1024.0), (0.0, 512.0), (0.0, 0.0)],
        [(1024.0, 512.0), (0.0, 512.0), (0.0, 0.0), (1024.0, 0.0)],
    ];
    for rotation in 0..4 {
        let mut c = camera();
        c.rotation = rotation;
        let v = owner(c);
        for corner in 0..4 {
            let mut out = Vec3::ZERO;
            assert!(v.get_city_corner_position(corner, &mut out));
            let (x, z) = expected[corner as usize][rotation as usize];
            assert_eq!(out, Vec3::new(x, 0.0, z));
        }
        assert_eq!(v.get_visible_edges(), ((rotation + 1) % 4, (rotation + 2) % 4));
    }
    let mut c = camera();
    c.rotation = 4;
    let v = owner(c);
    let mut p = Vec3::new(7.0, 8.0, 9.0);
    assert!(!v.get_city_corner_position(0, &mut p));
    assert_eq!(p, Vec3::new(7.0, 0.0, 9.0));
    assert!(!v.get_city_corner_position(99, &mut p));
    let mut rect = PixelRect::new(1, 2, 3, 4);
    assert!(!v.project_bounding_box_to_viewport_rectangle(Aabb::default(), &mut rect));
    assert_eq!(rect, PixelRect::new(1, 2, 3, 4));
    assert_eq!(get_vector_for_edge(1), Vec3::new(1.0, 0.0, 0.0));
    assert_eq!(get_vector_for_edge(4), Vec3::new(0.0, 0.0, -1.0));
}

#[test]
fn clipping_and_block_rounding_contract_vectors_and_invalid_inputs() {
    let mut v = owner(camera());
    v.set_city_size(1024.0, 1024.0);
    let mut p = Vec3::new(-3.0, 7.0, 2000.0);
    v.clip_world_position_to_city_position(&mut p);
    assert_eq!(p, Vec3::new(0.0, 7.0, 1023.0));
    let mut p = Vec3::new(1023.75, 9.0, 1024.0);
    v.clip_world_position_to_city_position(&mut p);
    assert_eq!(p, Vec3::new(1023.75, 9.0, 1023.0));
    for (value, down, up) in [
        (-65.0, -128, -64),
        (65.0, 64, 128),
        (0.5, 0, 0),
        (-0.5, 0, 0),
        (-64.0, -64, -64),
        (64.9, 64, 64),
    ] {
        assert_eq!(round_down_to_nearest_block_boundary(value, 64), Some(down));
        assert_eq!(round_up_to_nearest_block_boundary(value, 64), Some(up));
    }
    assert_eq!(round_down_to_nearest_block_boundary(1.0, 0), None);
    assert_eq!(round_up_to_nearest_block_boundary(f32::NAN, 64), Some(0));
    assert_eq!(
        round_up_to_nearest_block_boundary(2147483520.0, 1000),
        Some(-2147482000)
    );
}

#[test]
fn bounding_rectangle_uses_selected_corners_and_ties_even() {
    // Literal §4.4 / §11.18 ties: left subtracts one half before biasing;
    // right does not. Neither operation is a cast or ordinary floor.
    assert_eq!(crate::x87::rhe(2.0 - 0.5), 2);
    assert_eq!(crate::x87::rhe(3.0 - 0.5), 2);
    assert_eq!(crate::x87::rhe(2.5), 2);
    assert_eq!(crate::x87::rhe(3.5), 4);
    let mut v = ViewUtilities::new();
    v.cos_negative_yaw = 1.0;
    v.cos_pitch = 1.0;
    v.scales = Vec3::new(1.0, 1.0, 1.0);
    let b = Aabb {
        min: Vec3::new(2.5, 3.5, 4.5),
        max: Vec3::new(6.5, 7.5, 8.5),
    };
    let expected = [
        PixelRect::new(2, -8, 6, -4),
        PixelRect::new(2, -8, 6, -4),
        PixelRect::new(6, -8, 2, -4),
        PixelRect::new(6, -8, 2, -4),
    ];
    for (rotation, rect) in expected.into_iter().enumerate() {
        v.camera.rotation = rotation as i32;
        let mut out = PixelRect::default();
        assert!(v.project_bounding_box_to_viewport_rectangle(b, &mut out));
        assert_eq!(out, rect);
    }
}

#[test]
fn selected_rectangle_corners_match_exhaustive_box_extrema_for_all_rotations() {
    let bounds = Aabb {
        min: Vec3::new(2.5, 3.5, 4.5),
        max: Vec3::new(6.5, 7.5, 8.5),
    };
    for rotation in 0..4 {
        let mut c = camera();
        c.rotation = rotation;
        let v = owner(c);
        let mut all = Vec::new();
        for x in [bounds.min.x, bounds.max.x] {
            for y in [bounds.min.y, bounds.max.y] {
                for z in [bounds.min.z, bounds.max.z] {
                    all.push(v.project_point_to_viewport_coordinates_2d(Vec3::new(x, y, z)));
                }
            }
        }
        let all = Aabb::from_points(&all);
        let expected = PixelRect::new(
            crate::x87::rhe(all.min.x - 0.5),
            crate::x87::rhe(all.min.y - 0.5),
            crate::x87::rhe(all.max.x),
            crate::x87::rhe(all.max.y),
        );
        let mut out = PixelRect::default();
        assert!(v.project_bounding_box_to_viewport_rectangle(bounds, &mut out));
        assert_eq!(out, expected);
    }
}

fn pick() -> Pick {
    Pick {
        ray: Ray {
            origin: Vec3::ZERO,
            direction: Vec3::new(0.0, 0.0, 1.0),
        },
        nearest_distance: 100.0,
    }
}
fn triangle(z: f32) -> [Vec3; 3] {
    [
        Vec3::new(-1.0, -1.0, z),
        Vec3::new(1.0, -1.0, z),
        Vec3::new(0.0, 1.0, z),
    ]
}
fn triangle_mesh(zs: &[f32]) -> Mesh {
    Mesh {
        vertices: zs.iter().flat_map(|&z| triangle(z)).collect(),
        indices: None,
        groups: vec![PrimitiveGroup {
            kind: 0,
            first: 0,
            count: zs.len() as u32 * 3,
        }],
    }
}

#[test]
fn triangle_pick_distance_strict_improvement_winding_and_failures() {
    let mut p = pick();
    let [a, b, c] = triangle(10.0);
    assert!(triangle_pick(&mut p, a, b, c));
    assert_eq!(p.nearest_distance, 10.0);
    assert!(!triangle_pick(&mut p, a, b, c));
    assert_eq!(p.nearest_distance, 10.0);
    let [a, b, c] = triangle(5.0);
    assert!(triangle_pick(&mut p, a, c, b));
    assert_eq!(p.nearest_distance, 5.0);
    assert!(!triangle_pick(&mut p, a, a, a));
    let [a, b, c] = triangle(-1.0);
    assert!(triangle_pick(&mut p, a, b, c));
    p.ray.direction = Vec3::new(1.0, 0.0, 0.0);
    let [a, b, c] = triangle(1.0);
    assert!(!triangle_pick(&mut p, a, b, c));
    assert_eq!(p.nearest_distance, 1.0);
}

#[test]
fn triangle_boundaries_and_nonunit_euclidean_distance() {
    let mut p = pick();
    p.ray.direction.z = 2.0;
    let [a, b, c] = triangle(10.0);
    assert!(triangle_pick(&mut p, a, b, c));
    assert_eq!(p.nearest_distance, 10.0);
    p = pick();
    p.ray.origin = Vec3::new(0.0, -1.0, 0.0);
    assert!(triangle_pick(&mut p, a, b, c));
    p = pick();
    let [a, b, c] = triangle(0.0);
    assert!(triangle_pick(&mut p, a, b, c));
    assert_eq!(p.nearest_distance, 0.0);
}

#[test]
fn all_primitive_tables_include_unusual_quad_walk_and_degenerate_groups() {
    let expected = [
        (0, 6, vec![[0, 1, 2], [3, 4, 5]]),
        (1, 4, vec![[0, 1, 2], [1, 2, 3]]),
        (2, 4, vec![[0, 1, 2], [0, 2, 3]]),
        (3, 2, vec![[0, 0, 0], [1, 1, 1]]),
        (4, 4, vec![[0, 1, 0], [2, 3, 2]]),
        (5, 3, vec![[0, 1, 0], [1, 2, 1]]),
        (6, 4, vec![[0, 1, 2], [2, 4, 5]]),
        (7, 4, vec![[0, 1, 2], [2, 3, 4]]),
    ];
    for (kind, count, triangles) in expected {
        let g = PrimitiveGroup { kind, first: 99, count };
        assert_eq!(g.triangle_count() as usize, triangles.len());
        for (i, indices) in triangles.into_iter().enumerate() {
            assert_eq!(triangle_indices(g, i as u32), Some(indices));
        }
        assert_eq!(triangle_indices(g, g.triangle_count() as u32), None);
    }
    for kind in [1, 2, 5, 7, 99] {
        assert_eq!(
            (PrimitiveGroup {
                kind,
                first: 0,
                count: 0
            })
            .triangle_count(),
            match kind {
                1 | 2 | 7 => -2,
                5 => -1,
                _ => 0,
            }
        );
    }
    for kind in 3..=5 {
        let mut mesh = triangle_mesh(&[1.0]);
        mesh.groups[0].kind = kind;
        assert!(!mesh_pick(&mut pick(), &mesh, &Transform::default()));
    }
}

#[test]
fn every_transform_kind_uses_specified_order_and_ignores_inactive_fields() {
    let mut t = Transform {
        kind: 0,
        matrix: [[0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]],
        translation: Vec3::new(10.0, 20.0, 30.0),
        scale: 2.0,
    };
    let expected = [
        Vec3::new(1.0, 2.0, 3.0),
        Vec3::new(11.0, 22.0, 33.0),
        Vec3::new(2.0, 4.0, 6.0),
        Vec3::new(12.0, 24.0, 36.0),
        Vec3::new(2.0, 3.0, 1.0),
        Vec3::new(12.0, 23.0, 31.0),
        Vec3::new(14.0, 26.0, 32.0),
    ];
    for (kind, out) in expected.into_iter().enumerate() {
        t.kind = kind as u8;
        assert_eq!(t.apply(Vec3::new(1.0, 2.0, 3.0)), out);
    }
    t.kind = 255;
    assert_eq!(t.apply(Vec3::new(1.0, 2.0, 3.0)), expected[6]);
    t.kind = 0;
    t.matrix = [[f32::NAN; 3]; 3];
    t.scale = f32::NAN;
    t.translation = Vec3::new(f32::NAN, 0.0, 0.0);
    assert_eq!(t.apply(Vec3::new(1.0, 2.0, 3.0)), Vec3::new(1.0, 2.0, 3.0));
}

#[test]
fn mesh_pick_stops_at_first_closer_triangle_then_can_find_later_improvement() {
    let mesh = triangle_mesh(&[10.0, 5.0]);
    let mut p = pick();
    assert!(mesh_pick(&mut p, &mesh, &Transform::default()));
    assert_eq!(p.nearest_distance, 10.0);
    assert!(mesh_pick(&mut p, &mesh, &Transform::default()));
    assert_eq!(p.nearest_distance, 5.0);
    assert!(!mesh_pick(&mut p, &mesh, &Transform::default()));
}

/// The literal §11.20 vector distinguishes stopping within a mesh from walking
/// every mesh, in both resource orders, with the prescribed initial bound.
#[test]
fn conformance_20_mesh_and_model_order_eight_then_two() {
    struct Identity;
    impl ModelInstance for Identity {
        fn fill_transform(&self, _: usize, t: &mut Transform) {
            assert_eq!(*t, Transform::default());
        }
    }
    for (depths, expected) in [([8.0, 2.0], 8.0), ([2.0, 8.0], 2.0)] {
        let mut p = pick();
        p.nearest_distance = 10.0;
        assert!(mesh_pick(&mut p, &triangle_mesh(&depths), &Transform::default()));
        assert_eq!(p.nearest_distance, expected);

        let model = Model {
            meshes: depths.iter().map(|&z| triangle_mesh(&[z])).collect(),
        };
        p.nearest_distance = 10.0;
        assert!(model_pick(&mut p, &Identity, &model));
        assert_eq!(p.nearest_distance, 2.0);
    }
    let mut mesh = triangle_mesh(&[8.0]);
    mesh.groups[0].kind = 1;
    mesh.groups[0].count = 1;
    assert_eq!(mesh.groups[0].triangle_count(), -1);
    let mut p = pick();
    p.nearest_distance = 10.0;
    assert!(!mesh_pick(&mut p, &mesh, &Transform::default()));
    assert_eq!(p.nearest_distance, 10.0);
}

#[test]
fn indexed_mesh_group_first_and_transforms_are_applied_before_pick() {
    let mut mesh = triangle_mesh(&[1.0]);
    mesh.indices = Some(vec![99, 2, 1, 0]);
    mesh.groups[0].first = 1;
    let t = Transform {
        kind: 3,
        scale: 2.0,
        translation: Vec3::new(0.0, 0.0, 7.0),
        ..Transform::default()
    };
    let mut p = pick();
    assert!(mesh_pick(&mut p, &mesh, &t));
    assert_eq!(p.nearest_distance, 9.0);
    mesh.indices = Some(vec![99]);
    assert!(!mesh_pick(&mut pick(), &mesh, &t));
    mesh.groups[0].first = u32::MAX;
    assert!(!mesh_pick(&mut pick(), &mesh, &t));
    assert!(!mesh_pick(&mut pick(), &Mesh::default(), &t));
}

#[test]
fn vertex_stride_count_padding_and_owned_resource_lifetime() {
    let mut bytes = Vec::new();
    for p in triangle(4.0) {
        for f in [p.x, p.y, p.z] {
            bytes.extend_from_slice(&f.to_le_bytes());
        }
        bytes.extend_from_slice(&[0xAA; 4]);
    }
    let groups = vec![PrimitiveGroup {
        kind: 0,
        first: 0,
        count: 3,
    }];
    let mesh = Mesh::from_vertex_data(&bytes, 16, 3, None, groups.clone()).unwrap();
    bytes.fill(0);
    assert_eq!(mesh.vertices, triangle(4.0));
    let mut p = pick();
    assert!(mesh_pick(&mut p, &mesh, &Transform::default()));
    assert_eq!(p.nearest_distance, 4.0);
    assert!(Mesh::from_vertex_data(&bytes, 8, 3, None, groups.clone()).is_some());
    assert!(Mesh::from_vertex_data(&bytes[..10], 16, 3, None, groups).is_none());
    assert!(Mesh::from_vertex_data(&[], 0, 0, None, vec![]).is_some());
}

#[test]
fn hostile_group_counts_are_bounded_by_available_resource_elements() {
    for kind in 0..=8 {
        let mut mesh = triangle_mesh(&[1.0]);
        mesh.groups[0].kind = kind;
        mesh.groups[0].count = u32::MAX;
        let mut p = pick();
        p.ray.direction = Vec3::new(1.0, 0.0, 0.0);
        assert!(!mesh_pick(&mut p, &mesh, &Transform::default()));
        assert_eq!(p.nearest_distance, 100.0);
    }
}

#[test]
fn model_walk_visits_all_meshes_and_resets_transform_for_each_one() {
    struct Instance(AtomicUsize);
    impl ModelInstance for Instance {
        fn fill_transform(&self, index: usize, t: &mut Transform) {
            self.0.fetch_add(1, Ordering::SeqCst);
            assert_eq!(*t, Transform::default());
            if index == 0 {
                t.kind = 1;
                t.translation.z = 10.0;
            }
        }
    }
    let instance = Instance(AtomicUsize::new(0));
    let model = Model {
        meshes: vec![triangle_mesh(&[10.0, 1.0]), triangle_mesh(&[5.0])],
    };
    let mut p = pick();
    assert!(model_pick(&mut p, &instance, &model));
    assert_eq!(p.nearest_distance, 5.0);
    assert_eq!(instance.0.load(Ordering::SeqCst), 2);
    assert!(!model_pick(&mut p, &instance, &Model::default()));
}

#[test]
fn invalid_zoom_fails_and_degenerate_basis_does_not_invent_valid_vectors() {
    let mut v = owner(camera());
    let mut c = camera();
    c.zoom = 5;
    assert!(!v.update_camera_position(Some(&c)));
    assert!(!v.is_valid());
    c = camera();
    c.look_at = c.position;
    assert!(v.update_camera_position(Some(&c)));
    assert!(v.basis().0.x.is_nan());
    c = camera();
    c.volume.right = c.volume.left;
    assert!(v.update_camera_position(Some(&c)));
    assert!(v.scales().0.x.is_infinite());
}
#[test]
fn yaw_rounds_the_product_before_adding_the_offset_under_pc24() {
    // §2.2 + foundations §18: a single final store changes this by one ULP.
    assert_eq!(get_camera_view_angles_for_rotation(3, 3).unwrap().yaw.to_bits(), 0x408a_3ae6);
}
