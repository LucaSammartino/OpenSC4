// SPDX-License-Identifier: GPL-3.0-or-later
//! Private static-link word transport to the production view utility.
//! VU16 uses private static-link adapters; no public C ABI export is added.
use fc4_core::view_utilities::*;

fn vector(words: &[u32], at: usize) -> Vec3 {
    Vec3::new(
        f32::from_bits(words[at]),
        f32::from_bits(words[at + 1]),
        f32::from_bits(words[at + 2]),
    )
}

fn store(words: &mut [u32], at: usize, v: Vec3) {
    words[at..at + 3].copy_from_slice(&[v.x.to_bits(), v.y.to_bits(), v.z.to_bits()]);
}

// §§7.1.1/13.3: independently owned counterpart of the native resource graph.
fn fixture_mesh(input: &[u32]) -> Mesh {
    let group = PrimitiveGroup {
        kind: input[0],
        first: 1,
        count: 3,
    };
    Mesh {
        vertices: vec![
            Vec3::new(99.0, 99.0, 99.0),
            Vec3::ZERO,
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
        ],
        indices: (input[1] != 0).then(|| vec![99, 1, 2, 3]),
        groups: vec![group, group],
    }
}

fn fixture_transform(input: &[u32]) -> Transform {
    Transform {
        kind: input[4] as u8,
        ..Transform::default()
    }
}

struct FixtureCamera(std::cell::Cell<u32>);
impl CameraRayProvider for FixtureCamera {
    fn eye_ray(&self, x: f32, y: f32) -> Option<Ray> {
        self.0.set(self.0.get() + 1);
        // camera.md §1.6 perspective; R maps local y to +z and local z to -y.
        Some(Ray {
            origin: Vec3::new(0.0, 4.0, 0.0),
            direction: Vec3::new(-(2.0 * x / 8.0 - 1.0), -1.0, -(2.0 * (8.0 - y) / 8.0 - 1.0)),
        })
    }
}

fn fixture_pick(op: u32, input: &[u32], out: &mut [u32]) {
    let mesh = fixture_mesh(input);
    let mut pick = Pick {
        ray: Ray {
            origin: Vec3::new(0.5, 0.5, 2.0),
            direction: Vec3::new(0.0, 0.0, -1.0),
        },
        nearest_distance: f32::from_bits(input[5]),
    };
    let hit = if op == 0x007f91c0 {
        mesh_pick(&mut pick, &mesh, &fixture_transform(input))
    } else {
        struct Instance<'a> {
            input: &'a [u32],
            calls: std::cell::Cell<u32>,
        }
        impl ModelInstance for Instance<'_> {
            fn fill_transform(&self, _: usize, transform: &mut Transform) {
                self.calls.set(self.calls.get() + 1);
                *transform = fixture_transform(self.input);
            }
        }
        let instance = Instance {
            input,
            calls: std::cell::Cell::new(0),
        };
        let model = Model {
            meshes: vec![mesh; input[6] as usize],
        };
        let hit = model_pick(&mut pick, &instance, &model);
        out[20] = 1;
        out[21] = input[6];
        out[22] = instance.calls.get();
        let mut trace: u32 = 1;
        for i in 0..input[6] {
            trace = (trace.wrapping_mul(17) + 10 + i).wrapping_mul(17) + 100;
        }
        out[23] = trace;
        out[24] = 0; // all initial transforms have the specified identity fields
        hit
    };
    out[10] = pick.nearest_distance.to_bits();
    out[31] = u32::from(hit);
}

/// R-class cached arithmetic, with an owned copy of the published §1 inputs.
/// # Safety
/// Snapshot references 284 words; input and output reference 32 words each.
/// All three ranges are initialized and disjoint.
#[no_mangle]
pub(crate) unsafe extern "C" fn fc4_viewutil_shadow_cached(
    op: u32,
    snapshot: *const u32,
    input: *const u32,
    output: *mut u32,
) {
    // SAFETY: fixed-size private transport contract above.
    let words = unsafe { std::slice::from_raw_parts(snapshot, 0x470 / 4) };
    // SAFETY: fixed-size private transport contract above.
    let input = unsafe { std::slice::from_raw_parts(input, 32) };
    // SAFETY: fixed-size private transport contract above.
    let out = unsafe { std::slice::from_raw_parts_mut(output, 32) };
    let Some(view) = ViewUtilities::from_native_cached_words(words) else {
        return;
    };
    let p = vector(input, 0);
    let bounds = Aabb {
        min: p,
        max: vector(input, 3),
    };
    match op {
        0x007f6620 => {
            let (mut x, mut z) = (p.x, p.y);
            view.clip_position_to_city_2d(&mut x, &mut z);
            out[..2].copy_from_slice(&[x.to_bits(), z.to_bits()]);
        }
        0x007f6690 => out[0] = view.viewport_units_to_world_units(p.x).to_bits(),
        0x007f66a0 => {
            let (zoom, rotation) = view.get_zoom_and_rotation();
            out[..2].copy_from_slice(&[zoom, rotation as u32]);
        }
        0x007f66c0 => store(
            out,
            0,
            view.project_point_from_view_coordinates_to_viewport_coordinates(p),
        ),
        0x007f6710 => store(out, 0, view.project_point_to_fixed_view_coordinates(p)),
        0x007f6770 => out[0] = u32::from(view.is_bounding_box_in_view_frustum(bounds)),
        0x007f6910 => out[0] = u32::from(view.is_bounding_box_in_draw_frustum(bounds)),
        0x007f7f60 => {
            if let Some(bounds) = view.get_frustum_aabb_city_clipped(input[6] != 0) {
                store(out, 0, bounds.min);
                store(out, 3, bounds.max);
            }
        }
        0x007f6ba0 => {
            let mut v = vector(out, 0);
            view.get_city_corner_position(input[0], &mut v);
            store(out, 0, v);
        }
        0x007f7710 => {
            let mut v = p;
            view.clip_world_position_to_city_position(&mut v);
            store(out, 0, v);
        }
        0x007f7750 => {
            let (a, b) = view.get_visible_edges();
            out[..2].copy_from_slice(&[a as u32, b as u32]);
        }
        0x007f77d0 => store(out, 0, view.project_point_to_view_coordinates(p)),
        0x007f7850 => store(out, 0, view.project_point_to_viewport_coordinates(p)),
        0x007f78a0 => out[0] = view.project_point_to_viewport_z(p).to_bits(),
        0x007f78e0 => store(out, 0, view.project_point_to_viewport_coordinates_2d(p)),
        0x007f7920 => store(
            out,
            0,
            view.unproject_point_from_view_coordinates_to_world_coordinates(p),
        ),
        0x007f79b0 => store(
            out,
            0,
            view.unproject_point_from_viewport_coordinates_to_world_coordinates(p),
        ),
        0x007f7a40 => {
            let mut rect = PixelRect::new(out[0] as i32, out[1] as i32, out[2] as i32, out[3] as i32);
            view.project_bounding_box_to_viewport_rectangle(bounds, &mut rect);
            out[..4].copy_from_slice(&[
                rect.top_left.x as u32,
                rect.top_left.y as u32,
                rect.bottom_right.x as u32,
                rect.bottom_right.y as u32,
            ]);
        }
        0x007f7e50 => out[0] = u32::from(view.is_point_in_view_frustum(p)),
        0x007f8390 => store(
            out,
            0,
            view.project_point_from_viewport_coordinates_to_fixed_view_coordinates(p),
        ),
        _ => {}
    }
}

/// VU16 local call transport. Scalar results are observed after float32 store.
/// # Safety
/// Input and output each reference 32 initialized words, in disjoint storage.
#[no_mangle]
pub(crate) unsafe extern "C" fn fc4_viewutil_shadow_local(op: u32, input: *const u32, output: *mut u32) {
    // SAFETY: fixed-size private plugin transport contract above.
    let input = unsafe { std::slice::from_raw_parts(input, 32) };
    // SAFETY: fixed-size private plugin transport contract above.
    let out = unsafe { std::slice::from_raw_parts_mut(output, 32) };
    let f = |i| f32::from_bits(input[i]);
    let ray = Ray {
        origin: vector(input, 0),
        direction: vector(input, 3),
    };
    match op {
        0x005d6900 => {
            let points = [vector(input, 0), vector(input, 3), vector(input, 6)];
            if let Some(b) = point_array_aabb(&points, input[9] as i32) {
                store(out, 0, b.min);
                store(out, 3, b.max);
            }
        }
        0x005d6f10 => {
            let mut points = [vector(input, 0), vector(input, 3), Vec3::ZERO];
            let destination = input[6] as usize;
            if cross_in_storage(&mut points, 0, 1, destination).is_some() {
                store(out, 0, points[destination]);
            }
        }
        0x007f6550 => out[0] = min_of_four_floats([f(0), f(1), f(2), f(3)]).to_bits(),
        0x007f65b0 => out[0] = max_of_four_floats([f(0), f(1), f(2), f(3)]).to_bits(),
        0x007f6ab0 => store(out, 0, get_ray_intersection_with_ground_plane(ray)),
        0x007f6af0 => {
            let mut pick = Pick {
                ray,
                nearest_distance: f(6),
            };
            out[1] = u32::from(triangle_pick(
                &mut pick,
                vector(input, 7),
                vector(input, 10),
                vector(input, 13),
            ));
            out[0] = pick.nearest_distance.to_bits();
        }
        0x007f6b50 => {
            let mut point = vector(out, 0);
            out[3] = u32::from(ray_intersects_ground_plane(ray, &mut point));
            store(out, 0, point);
        }
        0x007f6cb0 => out[0] = get_object_y_rotation_angle(input[0] as i32).to_bits(),
        0x007f6cc0 => {
            let mut v = vector(input, 0);
            rotate_vector_by_view_rotation(&mut v, input[3] as i32);
            store(out, 0, v);
        }
        0x007f6d10 => {
            let mut v = vector(out, 0);
            get_vector_for_edge_into(input[0] as i32, &mut v);
            store(out, 0, v);
        }
        0x007f6d70 | 0x007f6db0 => {
            let result = if op == 0x007f6d70 {
                round_down_to_nearest_block_boundary(f(0), input[1] as i32)
            } else {
                round_up_to_nearest_block_boundary(f(0), input[1] as i32)
            };
            if let Some(v) = result {
                out[0] = (v as f32).to_bits();
            }
        }
        0x007f7780 => {
            if let Some(a) = get_camera_view_angles_for_rotation(input[0] as i32, input[1]) {
                out[..3].copy_from_slice(&[a.yaw.to_bits(), a.pitch.to_bits(), a.ground_pitch.to_bits()]);
            }
        }
        0x007f8300 => {
            if let Some(v) = get_city_width_from_ortho_view(f(0), input[1]) {
                out[0] = v.to_bits();
            }
        }
        0x0080ae80 => {
            out[0] = PrimitiveGroup {
                kind: input[0],
                first: 0,
                count: input[1],
            }
            .triangle_count() as u32
        }
        0x0080aef0 => {
            let mut triple = [out[0], out[1], out[2]];
            primitive_vertex_indices(input[0], input[1], &mut triple);
            out[..3].copy_from_slice(&triple);
        }
        0x0080b020 => {
            let bytes: Vec<u8> = input[..16].iter().flat_map(|v| v.to_le_bytes()).collect();
            if let Some(mesh) = Mesh::from_vertex_data(&bytes, input[18] as u16, 3, None, vec![]) {
                for i in 0..3 {
                    store(out, i * 3, mesh.vertices[input[19 + i] as usize]);
                }
            }
        }
        0x0080b090 => {
            let bytes: Vec<u8> = input[..16].iter().flat_map(|v| v.to_le_bytes()).collect();
            let mut triple = [out[0], out[1], out[2]];
            if decode_indices(input[18], &bytes, [input[19], input[20], input[21]], &mut triple).is_some() {
                out[..3].copy_from_slice(&triple);
            }
        }
        _ => {}
    }
}

/// # Safety
/// `words` must point to three readable and writable u32 words.
#[no_mangle]
unsafe extern "C" fn fc4_viewutil_shadow_rotate(words: *mut u32, rotation: i32) {
    if words.is_null() {
        return;
    }
    // SAFETY: the caller supplies exactly three initialized words.
    let words = unsafe { std::slice::from_raw_parts_mut(words, 3) };
    let mut v = Vec3::new(
        f32::from_bits(words[0]),
        f32::from_bits(words[1]),
        f32::from_bits(words[2]),
    );
    rotate_vector_by_view_rotation(&mut v, rotation);
    words.copy_from_slice(&[v.x.to_bits(), v.y.to_bits(), v.z.to_bits()]);
}

/// Private word transport for fully specified owner/iterator storage (§13).
/// # Safety
/// `owner` has 284 mutable words, `input` 64 readable words and `out` 32
/// mutable words. All three initialized buffers must be disjoint.
#[no_mangle]
unsafe extern "C" fn fc4_viewutil_shadow_state(op: u32, owner: *mut u32, input: *const u32, out: *mut u32) {
    if owner.is_null() || input.is_null() || out.is_null() {
        return;
    }
    // SAFETY: fixed disjoint initialized buffers supplied by the observer.
    let (owner, input, out) = unsafe {
        (
            std::slice::from_raw_parts_mut(owner, 0x470 / 4),
            std::slice::from_raw_parts(input, 64),
            std::slice::from_raw_parts_mut(out, 32),
        )
    };
    let Some(mut view) = ViewUtilities::from_native_cached_words(owner) else {
        return;
    };
    match op {
        0x007f91c0 | 0x007f9a40 => fixture_pick(op, input, out),
        0x0080b320 => {
            let mesh = fixture_mesh(input);
            let group = mesh.groups[input[7] as usize];
            owner[1] = group.kind;
            owner[2] = group.count;
            owner[3] = 1;
            owner[4] = 12;
            owner[5] = input[8].wrapping_add(if input[1] == 0 { 12 } else { 0 });
            owner[6] = 3;
            owner[7] = if input[1] != 0 { input[9].wrapping_add(2) } else { 0 };
            owner[8] = group.triangle_count() as u32;
            owner[10] = 0;
        }
        0x007f9160 | 0x007f9ea0 | 0x007f9f50 => {
            let camera = FixtureCamera(std::cell::Cell::new(0));
            view.native_camera_presence(true);
            if op == 0x007f9160 {
                let mut p = vector(out, 0);
                out[31] = u32::from(view.get_viewport_point_intersection_with_ground_plane(
                    Some(&camera),
                    input[0] as i32 as f32,
                    input[1] as i32 as f32,
                    &mut p,
                ));
                store(out, 0, p);
            } else if op == 0x007f9ea0 {
                if let Some(points) = view.get_viewport_rect_ground_corners(
                    Some(&camera),
                    PixelRect::new(input[0] as i32, input[1] as i32, input[2] as i32, input[3] as i32),
                ) {
                    for (i, p) in points.into_iter().enumerate() {
                        out[i * 2] = p.x.to_bits();
                        out[i * 2 + 1] = p.z.to_bits();
                    }
                }
            } else {
                view.build_draw_frustum_data_from_viewport_rect(PixelRect::new(0, 0, 8, 8), Some(&camera));
                view.native_whole_draw_words(owner);
            }
            out[16] = camera.0.get();
        }
        0x007f6df0 => {
            if let Some(initialized) = ViewUtilities::native_constructor_words(owner) {
                owner.copy_from_slice(&initialized);
            }
            out[31] = 1; // constructor returns its own receiver
        }
        0x007f83f0 => {
            // Only the missing-renderer path is exercised here. Preserve all
            // stale geometry and invalidate both byte flags, as in §2/§6.
            view.set_renderer_present(false);
            view.update_camera_position(None);
            owner[0] = (owner[0] & !0xff) | u32::from(view.is_valid());
            owner[0x444 / 4] &= !0xffff;
        }
        0x007f9080 => {
            let mut ray = Ray {
                origin: vector(out, 0),
                direction: vector(out, 3),
            };
            out[31] = u32::from(view.viewport_position_to_ray(
                None,
                input[0] as i32 as f32,
                input[1] as i32 as f32,
                &mut ray,
            ));
            store(out, 0, ray.origin);
            store(out, 3, ray.direction);
        }
        0x007f9b40 => {
            if let Some(area) = view.get_ground_plane_visible_area_rectangle(None) {
                for (i, p) in [area.inner.min, area.inner.max, area.outer.min, area.outer.max]
                    .into_iter()
                    .enumerate()
                {
                    if input[0] & (1 << i) != 0 {
                        store(out, i * 3, p);
                    }
                }
            }
        }
        0x0080b000 => {
            if TriangleIterator::native_non_deleting_destructor_words(&mut owner[..11], input[0]).is_some() {
                out[31] = 1; // the native result is the same receiver
            }
        }
        0x0080b3b0 => {
            let mesh = Mesh {
                vertices: (0..16).map(|i| vector(input, i * 3)).collect(),
                indices: (input[59] != 0).then(|| (0..16).map(|i| (15 - i) as u16).collect()),
                groups: vec![PrimitiveGroup {
                    kind: input[56],
                    first: 0,
                    count: input[57],
                }],
            };
            if let Some(mut iterator) = TriangleIterator::select_group(&mesh, 0) {
                let mut discarded = [Vec3::ZERO; 3];
                for _ in 0..input[58] {
                    let _ = iterator.next_triangle(&mut discarded);
                }
                let mut triangle = [vector(out, 0), vector(out, 3), vector(out, 6)];
                if let Ok(found) = iterator.next_triangle(&mut triangle) {
                    out[31] = u32::from(found);
                    if found {
                        owner[10] = owner[10].wrapping_add(1);
                    }
                    for (i, p) in triangle.into_iter().enumerate() {
                        store(out, i * 3, p);
                    }
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_preserves_padding_beside_byte_flags() {
        let mut owner = [0xcccc_cccc; 0x470 / 4];
        owner[0] = 0x1234_5678;
        owner[0x228 / 4] = 0x9abc_def0;
        owner[0x444 / 4] = 0x1357_9bdf;
        let input = [0; 64];
        let mut out = [0; 32];
        // SAFETY: fixed disjoint initialized arrays.
        unsafe { fc4_viewutil_shadow_state(0x007f6df0, owner.as_mut_ptr(), input.as_ptr(), out.as_mut_ptr()) };
        assert_eq!(owner[0], 0x1234_5600);
        assert_eq!(owner[0x228 / 4], 0x9abc_de00);
        assert_eq!(owner[0x444 / 4], 0x1357_0000);
        assert_eq!(owner[1], 0);
        assert_eq!(out[31], 1);
    }

    #[test]
    fn private_mesh_fixture_checks_strict_distance_and_model_provider_calls() {
        for indexed in [0, 1] {
            for (nearest, hit) in [(100.0_f32, true), (2.0, false), (1.0, false)] {
                let mut input = [0; 64];
                input[1] = indexed;
                input[5] = nearest.to_bits();
                input[6] = 2;
                let mut owner = [0; 0x470 / 4];
                let mut out = [0xcccc_cccc; 32];
                // SAFETY: initialized, disjoint buffers with the transport sizes.
                unsafe {
                    fc4_viewutil_shadow_state(0x007f91c0, owner.as_mut_ptr(), input.as_ptr(), out.as_mut_ptr());
                }
                assert_eq!(out[31], u32::from(hit));
                assert_eq!(out[10], nearest.min(2.0).to_bits());
                assert_eq!(out[9], 0xcccc_cccc);
                // SAFETY: same fixed transport buffers.
                unsafe {
                    fc4_viewutil_shadow_state(0x007f9a40, owner.as_mut_ptr(), input.as_ptr(), out.as_mut_ptr());
                }
                assert_eq!(&out[20..23], &[1, 2, 2]);
                assert_eq!(out[24], 0);
                assert_eq!(out[31], u32::from(hit));
            }
        }
    }

    #[test]
    fn private_camera_fixture_checks_successful_ground_wrappers_and_owner_preservation() {
        let mut input = [0; 64];
        input[0] = 4;
        input[1] = 4;
        let mut owner = [0; 0x470 / 4];
        owner[1] = 1;
        let before = owner;
        let mut out = [0xcccc_cccc; 32];
        // SAFETY: initialized, disjoint buffers with the transport sizes.
        unsafe {
            fc4_viewutil_shadow_state(0x007f9160, owner.as_mut_ptr(), input.as_ptr(), out.as_mut_ptr());
        }
        assert_eq!(&out[..3], &[0, 0, 0]);
        assert_eq!(out[16], 1);
        assert_eq!(out[31], 1);
        assert_eq!(owner, before);
        assert_eq!(out[3], 0xcccc_cccc);
    }

    #[test]
    fn private_state_transport_preserves_stale_geometry_and_failure_outputs() {
        let mut owner = [0x3f80_0000; 0x470 / 4];
        owner[1] = 0;
        owner[0] = 0xaabb_cc01;
        owner[0x444 / 4] = 0xaabb_0101;
        let before = owner;
        let input = [0; 64];
        let mut out = [0x4120_0000; 32];
        // SAFETY: disjoint initialized arrays with the documented sizes.
        unsafe { fc4_viewutil_shadow_state(0x007f9080, owner.as_mut_ptr(), input.as_ptr(), out.as_mut_ptr()) };
        assert_eq!(owner, before);
        assert_eq!(&out[..31], &[0x4120_0000; 31]);
        assert_eq!(out[31], 0);
        // SAFETY: same fixed buffers.
        unsafe { fc4_viewutil_shadow_state(0x007f83f0, owner.as_mut_ptr(), input.as_ptr(), out.as_mut_ptr()) };
        let mut expected = before;
        expected[0] = 0xaabb_cc00;
        expected[0x444 / 4] = 0xaabb_0000;
        assert_eq!(owner, expected);
    }

    #[test]
    fn warm_visible_area_copies_each_optional_output_without_touching_owner() {
        let mut owner = [0; 0x470 / 4];
        owner[0x444 / 4] = 1;
        for i in 0..12 {
            owner[0x414 / 4 + i] = (i as f32).to_bits();
        }
        let before = owner;
        for mask in 0..16 {
            let mut input = [0; 64];
            input[0] = mask;
            let mut out = [0x4120_0000; 32];
            // SAFETY: fixed disjoint buffers, all sixteen optional-pointer masks.
            unsafe { fc4_viewutil_shadow_state(0x007f9b40, owner.as_mut_ptr(), input.as_ptr(), out.as_mut_ptr()) };
            assert_eq!(owner, before);
            for (i, value) in out[..12].iter().enumerate() {
                assert_eq!(
                    *value,
                    if mask & (1 << (i / 3)) != 0 {
                        (i as f32).to_bits()
                    } else {
                        0x4120_0000
                    }
                );
            }
            assert_eq!(&out[12..], &[0x4120_0000; 20]);
        }
    }

    #[test]
    fn object_angle_transport_keeps_negative_zero_and_signed_extrema() {
        for (rotation, expected) in [
            (0_i32, 0x8000_0000),
            (1, 0xbfc9_0fdb),
            (-1, 0x3fc9_0fdb),
            (i32::MIN, 0x4f49_0fdb),
            (i32::MAX, 0xcf49_0fdb),
        ] {
            let mut input = [0; 32];
            input[0] = rotation as u32;
            let mut out = [0x4120_0000; 32];
            // SAFETY: fixed initialized arrays are disjoint.
            unsafe { fc4_viewutil_shadow_local(0x007f6cb0, input.as_ptr(), out.as_mut_ptr()) };
            assert_eq!(out[0], expected, "rotation={rotation}");
            assert_eq!(out[1..], [0x4120_0000; 31]);
        }
    }

    #[test]
    fn word_transport_preserves_y_payload_and_signed_zero() {
        let mut words = [0, 0x7fc1_2345, 0x8000_0000];
        // SAFETY: local three-word array.
        unsafe { fc4_viewutil_shadow_rotate(words.as_mut_ptr(), 1) };
        assert_eq!(words, [0, 0x7fc1_2345, 0]);
        words = [0x7fc2_3456, 0x8000_0000, 0x3f80_0000];
        // SAFETY: local three-word array.
        unsafe { fc4_viewutil_shadow_rotate(words.as_mut_ptr(), -1) };
        assert_eq!(words, [0x7fc2_3456, 0x8000_0000, 0x3f80_0000]);
    }

    #[test]
    fn local_transport_preserves_partial_writes_and_rounds_native_f32_input() {
        let mut input = [0; 32];
        let mut out = [0x4120_0000; 32];
        input[0] = (-1_i32) as u32;
        // SAFETY: fixed local arrays are disjoint.
        unsafe { fc4_viewutil_shadow_local(0x007f6d10, input.as_ptr(), out.as_mut_ptr()) };
        assert_eq!(&out[..3], &[0x4120_0000, 0, 0x4120_0000]);
        input[0] = 2_147_483_648.0_f32.to_bits();
        input[1] = 64;
        // SAFETY: fixed local arrays are disjoint.
        unsafe { fc4_viewutil_shadow_local(0x007f6db0, input.as_ptr(), out.as_mut_ptr()) };
        assert_eq!(out[0], (-2_147_483_648.0_f32).to_bits());
        input[0] = 99;
        let before = out;
        // SAFETY: fixed local arrays are disjoint.
        unsafe { fc4_viewutil_shadow_local(0x0080aef0, input.as_ptr(), out.as_mut_ptr()) };
        assert_eq!(out, before);
    }
}
