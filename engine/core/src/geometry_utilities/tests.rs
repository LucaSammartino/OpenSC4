// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn published_normal_box_ray_and_behind_triangle_vectors() {
    assert_eq!(
        get_normal_to_triangle([1., 0., 0.], [0.; 3], [0., 0., 1.]),
        [0., 1., 0.]
    );
    let mut hit = [99.; 3];
    assert!(ray_intersects_bounding_box(
        [-5., 0.5, 0.5],
        [1., 0., 0.],
        [0.; 3],
        [1.; 3],
        Some(&mut hit)
    ));
    assert_eq!(hit, [0., 0.5, 0.5]);
    assert!(ray_intersects_triangle(
        [0.2, 5., 0.2],
        [0., 1., 0.],
        [0.; 3],
        [1., 0., 0.],
        [0., 0., 1.],
        Some(&mut hit)
    ));
    assert_eq!(hit, [0.2, 0., 0.2]);
}

#[test]
fn asymmetric_lengths_area_plane_and_unbounded_distance() {
    let a = [2., -3., 7.];
    let b = [-1., 1., -5.];
    assert_eq!(length_of_line_squared(a, b), 169.);
    assert_eq!(length_of_line(a, b), 13.);
    assert_eq!(length_of_line_squared_2d([2., -3.], [-1., 1.]), 25.);
    assert_eq!(length_of_line_2d([2., -3.], [-1., 1.]), 5.);
    assert_eq!(twice_triangle_area([2., 3., 7.], [2., -1., 7.], [5., -1., 7.]), 12.);
    assert_eq!(
        get_normal_to_triangle([2., 3., 7.], [2., -1., 7.], [5., -1., 7.]),
        [0., 0., 12.]
    );
    assert_eq!(
        point_and_vector_to_plane_equation([2., -3., 7.], [3., 2., -1.]),
        [3., 2., -1., 7.]
    );
    assert_eq!(point_distance_to_ray([-7., 5., 4.], [2., 1., 4.], [3., 0., 0.]), 4.);
    assert!(point_distance_to_ray(a, b, [0.; 3]).is_nan());
    assert_eq!(twice_triangle_area(a, a, b), 0.);
}

#[test]
fn separate_binary32_operations_and_first_bound_signed_zero() {
    // GU2: these distinguish the documented provisional association from
    // a fused cross product or a double-precision dot followed by one store.
    let plane = point_and_vector_to_plane_equation([16_777_216., 1., -16_777_216.], [1.; 3]);
    assert_eq!(plane[3].to_bits(), (-0.0f32).to_bits());
    let normal = get_normal_to_triangle([8193., 0., 8192.], [0.; 3], [8192., 0., 8191.]);
    assert_eq!(normal[1], 0.);
    let mut min = [99.; 3];
    assert!(bounding_boxes_intersect(
        [-0.0; 3],
        [1.; 3],
        [0.0; 3],
        [2.; 3],
        Some(&mut min),
        None
    ));
    // GU5: the first operand wins a tie, retaining its signed zero.
    assert!(min.iter().all(|x| x.to_bits() == (-0.0f32).to_bits()));
    // The published strict inequalities do not separately reject empty boxes.
    assert!(bounding_boxes_intersect(
        [0.; 3], [0.; 3], [-1.; 3], [1.; 3], None, None
    ));
}

#[test]
fn rectangle_distance_regions_and_half_open_membership() {
    let min = [-2., 3.];
    let max = [4., 8.];
    for p in [min, max, [-2., 8.], [1., 5.]] {
        assert_eq!(point_distance_to_rectangle(p, min, max), 0.);
    }
    for (p, distance) in [
        ([1., 10.], 2.),
        ([1., 0.], 3.),
        ([-4., 6.], 2.),
        ([7., 6.], 3.),
        ([7., 12.], 5.),
        ([-5., -1.], 5.),
    ] {
        assert_eq!(point_distance_to_rectangle(p, min, max), distance);
    }
    assert!(point_in_rectangle_2d(min, min, max));
    assert!(!point_in_rectangle_2d(max, min, max));
    assert!(!point_in_rectangle_2d([4., 4.], min, max));
    assert!(!point_in_rectangle_2d([0., 8.], min, max));
    assert!(!point_in_rectangle_2d([f32::NAN, 4.], min, max));
}

#[test]
fn box_overlap_strict_nan_and_output_gating() {
    assert!(!bounding_boxes_intersect(
        [0.; 3],
        [1.; 3],
        [1., 0., 0.],
        [2., 1., 1.],
        None,
        None
    ));
    let mut min = [99.; 3];
    let mut max = [99.; 3];
    assert!(bounding_boxes_intersect(
        [-4., 2., -8.],
        [3., 9., 7.],
        [-1., -3., 4.],
        [8., 6., 12.],
        Some(&mut min),
        Some(&mut max)
    ));
    assert_eq!(min, [-1., 2., 4.]);
    assert_eq!(max, [3., 6., 7.]);
    assert!(bounding_boxes_intersect(
        [0.; 3],
        [1.; 3],
        [f32::NAN, 0., 0.],
        [2.; 3],
        None,
        Some(&mut max)
    ));
    assert_eq!(max, [3., 6., 7.]); // outMin gates both outputs, §4.6.
    assert!(!bounding_boxes_intersect(
        [0.; 3],
        [1.; 3],
        [f32::NAN, 1., 0.],
        [2.; 3],
        Some(&mut min),
        Some(&mut max)
    ));
    assert_eq!(min, [-1., 2., 4.]);
}

#[test]
fn bounding_box_ray_inside_stationary_away_miss_and_corner() {
    let min = [-2., 3., 7.];
    let max = [4., 8., 11.];
    let mut hit = [99.; 3];
    assert!(ray_intersects_bounding_box(min, [0.; 3], min, max, Some(&mut hit)));
    assert_eq!(hit, min);
    for d in [[0.; 3], [-1., 0., 0.], [1., 7., 0.]] {
        hit = [99.; 3];
        assert!(!ray_intersects_bounding_box([-5., 4., 8.], d, min, max, Some(&mut hit)));
        assert_eq!(hit, [99.; 3]);
    }
    assert!(ray_intersects_bounding_box(
        [-5., 0., 4.],
        [1.; 3],
        min,
        max,
        Some(&mut hit)
    ));
    assert_eq!(hit, min);
    assert!(ray_intersects_bounding_box([7., 5., 9.], [-1., 0., 0.], min, max, None));
}

#[test]
fn triangle_edges_winding_degenerate_and_absolute_determinant() {
    let a = [2., 3., 7.];
    let b = [5., 3., 7.];
    let c = [2., 3., 9.];
    let mut hit = [99.; 3];
    for (b, c) in [(b, c), (c, b)] {
        for p in [a, b, c, [3.5, 3., 8.]] {
            assert!(ray_intersects_triangle(
                [p[0], 10., p[2]],
                [0., 2., 0.],
                a,
                b,
                c,
                Some(&mut hit)
            ));
            assert_eq!(hit, p);
        }
    }
    assert!(!ray_intersects_triangle(
        [5., 10., 9.],
        [0., 1., 0.],
        a,
        b,
        c,
        Some(&mut hit)
    ));
    assert!(!ray_intersects_triangle(a, [0., 1., 0.], a, a, c, None));
    for (extent, expected) in [(EPSILON, true), (f32::from_bits(EPSILON.to_bits() - 1), false)] {
        assert_eq!(
            ray_intersects_triangle([0.; 3], [0., 1., 0.], [0.; 3], [1., 0., 0.], [0., 0., extent], None),
            expected
        );
    }
}

#[test]
fn plane_parallel_epsilon_on_plane_and_behind() {
    let mut hit = [99.; 3];
    let plane = [0., 1., 0., -3.];
    assert!(ray_intersects_plane([2., 8., -4.], [0., 2., 0.], plane, Some(&mut hit)));
    assert_eq!(hit, [2., 3., -4.]);
    assert!(!ray_intersects_plane(
        [2., 8., -4.],
        [1., 0., 0.],
        plane,
        Some(&mut hit)
    ));
    assert_eq!(hit, [0.; 3]);
    assert!(ray_intersects_plane([2., 3., -4.], [1., 0., 0.], plane, Some(&mut hit)));
    assert_eq!(hit, [2., 3., -4.]);
    assert!(ray_intersects_plane([0.; 3], [EPSILON, 0., 0.], [1., 0., 0., 1.], None));
    assert!(!ray_intersects_plane(
        [0.; 3],
        [f32::from_bits(EPSILON.to_bits() - 1), 0., 0.],
        [1., 0., 0., 1.],
        None
    ));
    assert!(!ray_intersects_plane([0.; 3], [0.; 3], [1., 0., 0., EPSILON], None));
    assert!(ray_intersects_plane(
        [0.; 3],
        [0.; 3],
        [1., 0., 0., f32::from_bits(EPSILON.to_bits() - 1)],
        None
    ));
}

#[test]
fn published_rays_and_segments_and_null_output_quirk() {
    let mut hit = [99.; 2];
    assert!(!rays_intersect_2d([0.; 2], [1., 0.], [2., -1.], [1., 1.], None));
    assert!(rays_intersect_2d(
        [0.; 2],
        [1., 0.],
        [2., -1.],
        [1., 1.],
        Some(&mut hit)
    ));
    assert_eq!(hit, [3., 0.]);
    assert!(lines_intersect_2d([0.; 2], [2.; 2], [0., 2.], [2., 0.], Some(&mut hit)));
    assert_eq!(hit, [1.; 2]);
    // Parallel directions can return true only in the null-output branch.
    assert!(rays_intersect_2d([0.; 2], [1.; 2], [0.; 2], [2.; 2], None));
    assert!(!rays_intersect_2d([0.; 2], [1.; 2], [0.; 2], [2.; 2], Some(&mut hit)));
    assert!(!rays_intersect_2d([0.; 2], [0.; 2], [0.; 2], [EPSILON, 1.], None));
}

#[test]
fn segments_endpoints_parallel_outside_and_epsilon() {
    assert!(lines_intersect_2d([-3., 2.], [4., 2.], [4., 2.], [5., 7.], None));
    assert!(!lines_intersect_2d([-3., 2.], [4., 2.], [5., 0.], [5., 7.], None));
    assert!(!lines_intersect_2d([0.; 2], [2.; 2], [1.; 2], [3.; 2], None));
    assert!(!lines_intersect_2d([1.; 2], [1.; 2], [0.; 2], [2.; 2], None));
    for (extent, expected) in [(EPSILON, true), (f32::from_bits(EPSILON.to_bits() - 1), false)] {
        assert_eq!(
            lines_intersect_2d([0.; 2], [1., 0.], [0.; 2], [0., extent], None),
            expected
        );
    }
}

#[test]
fn vector_scale_boundary_and_noncommuting_rotation_translation() {
    let mut v = [3., 4.];
    scale_vector_2d(&mut v, -10.);
    assert_eq!(v, [-6., -8.]);
    v = [EPSILON, 0.];
    scale_vector_2d(&mut v, 100.);
    assert_eq!(v, [EPSILON, 0.]);
    v = [0.; 2];
    scale_vector_2d(&mut v, 100.);
    assert_eq!(v, [0.; 2]);
    let c = [7., -2.];
    let result = rotate_point_2d(c, [10., 3.], std::f32::consts::FRAC_PI_2);
    assert!((result[0] - 2.).abs() < 0.000001 && (result[1] - 1.).abs() < 0.000001);
    assert_ne!(result, rotate_point_2d([0.; 2], [10., 3.], std::f32::consts::FRAC_PI_2));
}

#[test]
fn published_polygon_clip_and_right_side_winding() {
    let square = [[0., 0.], [0., 4.], [4., 4.], [4., 0.]];
    let clip = [[2., 2.], [2., 6.], [6., 6.], [6., 2.]];
    let mut result = clip_polygon_to_polygon_2d(&square, &clip, 40);
    result.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(result, [[2., 2.], [2., 4.], [4., 2.], [4., 4.]]);
    let reversed: Vec<_> = clip.into_iter().rev().collect();
    assert!(clip_polygon_to_polygon_2d(&square, &reversed, 40).is_empty());
    assert_eq!(clip_polygon_to_polygon_2d(&square, &[], 0), square);
}

#[test]
fn clipping_empty_capacity_boundary_and_unsupported_short_crossing() {
    assert!(clip_polygon_to_edge_2d([0.; 2], [0., 1.], &[], 5).is_empty());
    let poly = [[-1., 2.], [1., 2.], [1., -2.], [-1., -2.]];
    let all = clip_polygon_to_edge_2d([0.; 2], [0., 1.], &poly, 10);
    assert_eq!(all, [[0., 2.], [1., 2.], [1., -2.], [0., -2.]]);
    for cap in 0..4 {
        assert_eq!(clip_polygon_to_edge_2d([0.; 2], [0., 1.], &poly, cap), all[..cap]);
    }
    // GU5 DIFFERS: an uninitialized failed-crossing read has no point oracle.
    let mut backing = [[99.; 2]; 10];
    let mut count = 0;
    assert_eq!(clipping::clip_edge_signed([0.; 2], [0., 0.001], &[[-1., 0.], [1., 0.]], 2,
        &mut backing, 10, &mut count), Err(clipping::ClipUnsupported::UninitializedCrossing));
    assert_eq!(clip_polygon_to_edge_2d([0.; 2], [0., 1.], &[[0., 2.]], 10), [[0., 2.]]);
}

struct Draws {
    values: std::vec::IntoIter<f64>,
    count: usize,
}
impl DirectionRandom for Draws {
    fn range_uniform(&mut self, min: f64, max: f64) -> f64 {
        assert_eq!((min, max), (-1., 1.));
        self.count += 1;
        self.values.next().expect("unexpected random draw")
    }
}
#[test]
fn random_draw_count_rejection_and_zero_degeneracy() {
    let mut random = Draws {
        values: vec![1., 1., 0., 0.].into_iter(),
        count: 0,
    };
    let v = random_direction_2(&mut random);
    assert_eq!(v, [1. / 2f32.sqrt(); 2]);
    assert_eq!(random.count, 2);
    assert!(random_direction_2(&mut random).iter().all(|x| x.is_nan()));
    assert_eq!(random.count, 4);
    let mut random = Draws {
        values: vec![1., 1., 1., 0., 0., 0., 0.005, 0., 0., 0.6, 0.8, 0.].into_iter(),
        count: 0,
    };
    assert_eq!(random_direction_3(&mut random), [0.6, 0.8, 0.]);
    assert_eq!(random.count, 12);
    let mut random = Draws {
        values: vec![0.01, 0., 0., 1., 0., 0.].into_iter(),
        count: 0,
    };
    // Both closed rejection boundaries are accepted, in exactly three draws.
    assert_eq!(random_direction_3(&mut random), [1., 0., 0.]);
    assert_eq!(random.count, 3);
    assert_eq!(random_direction_3(&mut random), [1., 0., 0.]);
    assert_eq!(random.count, 6);
}

#[test]
fn published_builder_colour_layout_ties_and_wrap() {
    assert_eq!(std::mem::size_of::<Vertex>(), 16);
    let mut builder = MeshBuilder::new();
    assert_eq!(builder.arrays_allocated(), (false, false));
    assert!(!builder.changed());
    builder.set_light_direction([1., 0., 0.]);
    builder.set_normal([-1., 0., 0.]);
    builder.set_colour(1., 0., 0.);
    builder.begin_primitive(0);
    builder.add_vertex([2., -7., 4.]);
    assert_eq!(builder.vertices()[0].colour, [0, 0, 155, 255]);
    builder.set_normal([1., 0., 0.]);
    builder.set_colour_alpha(0.5 / 256., 1.5 / 256., 2.5 / 256., 1.5);
    builder.add_vertex([-3., 5., 9.]);
    assert_eq!(builder.vertices()[1].colour, [2, 2, 0, 127]);
    builder.end_primitive();
    assert_eq!(builder.primitives()[0].count, 2);
    assert_eq!(
        builder.mesh().bounds(),
        Some(Bounds {
            min: [-3., -7., 4.],
            max: [2., 5., 9.]
        })
    );
    assert!(!builder.changed());
    let old_bounds = builder.bounds();
    builder.reset();
    assert_eq!(builder.arrays_allocated(), (true, true));
    assert!(builder.mesh().vertices().is_empty());
    assert_eq!(builder.bounds(), old_bounds); // GU3 DIFFERS: Reset does not write bounds.
}

#[test]
fn asymmetric_box_mesh_face_order_offsets_lighting_and_bounds() {
    let min = [-2., 3., -7.];
    let max = [5., 11., 13.];
    let corners = box_corners(min, max);
    assert_eq!(corners[5], [5., 3., 13.]);
    assert_eq!(corners[6], [-2., 11., 13.]);
    let mut builder = MeshBuilder::new();
    builder.set_colour(1., 0.5, 0.25);
    builder.set_light_direction([0.25, 0.5, 1.]);
    builder.begin_primitive(3);
    builder.add_vertex([0., 4., 0.]);
    builder.end_primitive();
    add_box_mesh(&mut builder, &corners);
    assert_eq!(
        builder.primitives()[1],
        Primitive {
            primitive_type: 6,
            first: 1,
            count: 24
        }
    );
    let order = [2, 3, 1, 0, 4, 5, 7, 6, 0, 1, 5, 4, 1, 3, 7, 5, 3, 2, 6, 7, 2, 0, 4, 6];
    for (vertex, index) in builder.vertices()[1..].iter().zip(order) {
        assert_eq!(vertex.position, corners[index]);
    }
    assert_eq!(builder.vertices()[1].colour[2], 155);
    assert_eq!(builder.vertices()[5].colour[2], 255);
    assert_eq!(builder.mesh().bounds(), Some(Bounds { min, max }));
    add_box_mesh(&mut builder, &box_corners([-9.; 3], [20.; 3]));
    assert_eq!(
        builder.mesh().bounds(),
        Some(Bounds {
            min: [-9.; 3],
            max: [20.; 3]
        })
    );
}
