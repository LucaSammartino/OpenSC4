// SPDX-License-Identifier: GPL-3.0-or-later
//! Pure geometry from geometry-utilities.md §§1–6. No game pointers or globals.
//!
//! `pc24` exposes checked, bounded PC24/nearest/masked numeric operations.
//! Legacy value adapters use separate f32 operations, never FMA, and are not
//! ambient x87/PC64/extended-exponent models. See README.md for exact limits.

pub mod clipping;
mod mesh;
pub mod pc24;
pub mod raw_texture;
mod texture;
pub use mesh::{add_box_mesh, box_corners, Bounds, BuilderUnsupported, MeshBuilder, Primitive, Vertex};
pub use texture::{create_texture_from_text, Surface, TextFont, TextureHost, TextureImage};

pub type Vec2 = [f32; 2];
pub type Vec3 = [f32; 3];
pub type Plane = [f32; 4];
pub const EPSILON: f32 = 0.01;

fn sub<const N: usize>(a: [f32; N], b: [f32; N]) -> [f32; N] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot<const N: usize>(a: [f32; N], b: [f32; N]) -> f32 {
    // Start with the first product, avoiding an extra +0 (signed zero matters).
    let mut result = a[0] * b[0];
    for i in 1..N {
        result += a[i] * b[i];
    }
    result
}
fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn along<const N: usize>(o: [f32; N], d: [f32; N], t: f32) -> [f32; N] {
    std::array::from_fn(|i| o[i] + t * d[i])
}
fn store<T>(out: Option<&mut T>, value: T) {
    if let Some(out) = out {
        *out = value;
    }
}
fn closed(value: f32, lo: f32, hi: f32) -> bool {
    (lo..=hi).contains(&value)
}

pub fn length_of_line_squared(a: Vec3, b: Vec3) -> f32 {
    let d = sub(b, a);
    (d[2] * d[2] + d[1] * d[1]) + d[0] * d[0]
}
pub fn length_of_line_squared_2d(a: Vec2, b: Vec2) -> f32 {
    let d = sub(b, a);
    d[1] * d[1] + d[0] * d[0]
}
pub fn length_of_line(a: Vec3, b: Vec3) -> f32 {
    length_of_line_squared(a, b).sqrt()
}
pub fn length_of_line_2d(a: Vec2, b: Vec2) -> f32 {
    length_of_line_squared_2d(a, b).sqrt()
}
pub fn twice_triangle_area(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let n = cross(sub(p, a), sub(b, a));
    dot(n, n).sqrt()
}
/// Whole-line distance; deliberately no clamp or zero-direction fallback (§2.1).
pub fn point_distance_to_ray(p: Vec3, o: Vec3, d: Vec3) -> f32 {
    let pd = (p[1] * d[1] + p[2] * d[2]) + p[0] * d[0];
    let od = (o[1] * d[1] + o[0] * d[0]) + o[2] * d[2];
    let t = (pd - od) / ((d[0] * d[0] + d[1] * d[1]) + d[2] * d[2]);
    let r = sub(p, along(o, d, t));
    ((r[0] * r[0] + r[1] * r[1]) + r[2] * r[2]).sqrt()
}
pub fn point_distance_to_rectangle(p: Vec2, min: Vec2, max: Vec2) -> f32 {
    // §2.2 selects low before high, with unordered treated as within.
    let side = |i: usize| {
        if p[i] < min[i] {
            Some((min[i], min[i] - p[i]))
        } else if p[i] > max[i] {
            Some((max[i], p[i] - max[i]))
        } else {
            None
        }
    };
    match (side(0), side(1)) {
        (None, None) => 0.0,
        (Some((_, d)), None) | (None, Some((_, d))) => d,
        (Some((x, _)), Some((y, _))) => {
            let dx = p[0] - x;
            let dy = p[1] - y;
            (dy * dy + dx * dx).sqrt()
        }
    }
}
pub fn get_normal_to_triangle(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    cross(sub(c, b), sub(a, b))
}
pub fn point_and_vector_to_plane_equation(p: Vec3, n: Vec3) -> Plane {
    [n[0], n[1], n[2], -((p[1] * n[1] + p[2] * n[2]) + p[0] * n[0])]
}

pub fn ray_intersects_bounding_box(o: Vec3, d: Vec3, min: Vec3, max: Vec3, hit: Option<&mut Vec3>) -> bool {
    let mut face = [0.0; 3];
    let mut candidate = [-1.0; 3];
    let mut outside = false;
    for i in 0..3 {
        let is_outside = if o[i] < min[i] {
            face[i] = min[i];
            true
        } else if o[i] > max[i] {
            face[i] = max[i];
            true
        } else {
            false
        };
        outside |= is_outside;
        if is_outside && d[i] != 0.0 {
            candidate[i] = (face[i] - o[i]) / d[i];
        }
    }
    if !outside {
        store(hit, o);
        return true;
    }
    let mut axis = 0;
    for i in 1..3 {
        if candidate[i] > candidate[axis] {
            axis = i;
        }
    }
    let t = candidate[axis];
    if t < 0.0 {
        return false;
    }
    let mut point = along(o, d, t);
    point[axis] = face[axis];
    for i in 0..3 {
        if i != axis && (point[i] < min[i] || point[i] > max[i]) {
            return false;
        }
    }
    store(hit, point);
    true
}
#[allow(clippy::manual_range_contains)] // Ordered rejections deliberately accept quiet NaN.
pub fn ray_intersects_triangle(o: Vec3, d: Vec3, a: Vec3, b: Vec3, c: Vec3, hit: Option<&mut Vec3>) -> bool {
    let e1 = sub(b, a);
    let e2 = sub(c, a);
    let p = cross(d, e2);
    let dot_zyx = |a: Vec3, b: Vec3| (a[2] * b[2] + a[1] * b[1]) + a[0] * b[0];
    let det = dot_zyx(e1, p);
    if det.abs() < EPSILON {
        return false;
    }
    let s = sub(o, a);
    let reciprocal = 1.0 / det; // Published binary32 reciprocal store.
    let u = dot_zyx(s, p) * reciprocal;
    let q = cross(s, e1);
    let v = dot_zyx(d, q) * reciprocal;
    if u < 0.0 || u > 1.0 || v < 0.0 || u + v > 1.0 {
        return false;
    }
    if let Some(hit) = hit {
        *hit = along(o, d, dot_zyx(e2, q) * reciprocal);
    }
    true
}
pub fn ray_intersects_plane(o: Vec3, d: Vec3, plane: Plane, hit: Option<&mut Vec3>) -> bool {
    let n = [plane[0], plane[1], plane[2]];
    let s = ((o[1] * n[1] + o[2] * n[2]) + o[0] * n[0]) + plane[3];
    let k = (d[0] * n[0] + d[2] * n[2]) + d[1] * n[1];
    if k.abs().partial_cmp(&EPSILON) != Some(std::cmp::Ordering::Less) {
        if let Some(hit) = hit {
            let t = -(s / k);
            *hit = along(o, d, t);
        }
        true
    } else if s.abs().partial_cmp(&EPSILON) != Some(std::cmp::Ordering::Less) {
        store(hit, [0.0; 3]);
        false
    } else {
        store(hit, o);
        true
    }
}
pub fn rays_intersect_2d(p1: Vec2, d1: Vec2, p2: Vec2, d2: Vec2, hit: Option<&mut Vec2>) -> bool {
    let Some(hit) = hit else {
        return (d2[0] - d1[0]).abs() > EPSILON && (d2[1] - d1[1]).abs() > EPSILON;
    };
    let neg_y = -d2[1];
    let k = d2[0] * d1[1] + neg_y * d1[0];
    if k.abs() < EPSILON {
        return false;
    }
    let t = (d2[0] * (p2[1] - p1[1]) + neg_y * (p2[0] - p1[0])) / k;
    *hit = along(p1, d1, t);
    true
}
pub fn lines_intersect_2d(a1: Vec2, a2: Vec2, b1: Vec2, b2: Vec2, hit: Option<&mut Vec2>) -> bool {
    let a = sub(a2, a1);
    let b = sub(b1, b2);
    let delta = sub(b1, a1);
    let k = b[1] * a[0] - a[1] * b[0];
    if k.abs() < EPSILON {
        return false;
    }
    let reciprocal = 1.0 / k;
    let ua = (b[1] * delta[0] - delta[1] * b[0]) * reciprocal;
    let ub = (delta[1] * a[0] - a[1] * delta[0]) * reciprocal;
    if !(closed(ua, 0.0, 1.0) && closed(ub, 0.0, 1.0)) {
        return false;
    }
    store(hit, along(a1, a, ua));
    true
}
pub fn bounding_boxes_intersect(
    a_min: Vec3,
    a_max: Vec3,
    b_min: Vec3,
    b_max: Vec3,
    out_min: Option<&mut Vec3>,
    out_max: Option<&mut Vec3>,
) -> bool {
    try_bounding_boxes_intersect(a_min, a_max, b_min, b_max, out_min, out_max)
        .expect("nonnull maximum required when overlapping minimum output is supplied")
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequiredMaximumOutput;
pub fn try_bounding_boxes_intersect(
    a_min: Vec3,
    a_max: Vec3,
    b_min: Vec3,
    b_max: Vec3,
    out_min: Option<&mut Vec3>,
    out_max: Option<&mut Vec3>,
) -> Result<bool, RequiredMaximumOutput> {
    for i in 0..3 {
        // §4.6 explicitly counts unordered comparisons as overlap.
        if a_min[i] >= b_max[i] || b_min[i] >= a_max[i] {
            return Ok(false);
        }
    }
    if let Some(out_min) = out_min {
        let out_max = out_max.ok_or(RequiredMaximumOutput)?;
        // §4.6 selects the second operand on ties and unordered comparisons.
        *out_min = std::array::from_fn(|i| if a_min[i] > b_min[i] { a_min[i] } else { b_min[i] });
        *out_max = std::array::from_fn(|i| if a_max[i] < b_max[i] { a_max[i] } else { b_max[i] });
    }
    Ok(true)
}
pub fn point_in_rectangle_2d(p: Vec2, min: Vec2, max: Vec2) -> bool {
    p[0] >= min[0] && p[0] < max[0] && p[1] >= min[1] && p[1] < max[1]
}
pub fn scale_vector_2d(v: &mut Vec2, length: f32) {
    let magnitude = dot(*v, *v).sqrt();
    if magnitude > EPSILON {
        let factor = length / magnitude;
        *v = v.map(|x| x * factor);
    }
}
pub fn rotate_point_2d(c: Vec2, p: Vec2, angle: f32) -> Vec2 {
    let [dx, dy] = sub(p, c);
    // GU2: published formula only; trig argument reduction is not specified.
    let (s, co) = angle.sin_cos();
    [c[0] + (co * dx - s * dy), c[1] + (s * dx + co * dy)]
}
pub fn clip_polygon_to_edge_2d(e0: Vec2, e1: Vec2, poly: &[Vec2], max: usize) -> Vec<Vec2> {
    // Qualified adapter: count and max must fit signed words; use the signed
    // API for explicit errors, nonpositive counts and observable backing tails.
    let n = i32::try_from(poly.len()).expect("signed polygon count");
    let max = i32::try_from(max).expect("signed capacity");
    let mut out = vec![[0.; 2]; poly.len().saturating_mul(2).min(max as usize)];
    let mut count = 0;
    clipping::clip_edge_signed(e0, e1, poly, n, &mut out, max, &mut count).expect("defined PC24 clipping domain");
    out.truncate(count as usize);
    out
}
pub fn clip_polygon_to_polygon_2d(poly: &[Vec2], clip: &[Vec2], max: usize) -> Vec<Vec2> {
    let mut out = vec![[0.; 2]; poly.len().max(40)];
    let mut count = 0;
    clipping::clip_polygon_signed(
        poly,
        i32::try_from(poly.len()).expect("signed count"),
        clip,
        i32::try_from(clip.len()).expect("signed count"),
        &mut out,
        i32::try_from(max).expect("signed capacity"),
        &mut count,
    )
    .expect("defined PC24 clipping domain");
    out.truncate(count as usize);
    out
}

/// Injectable range source; real RNGs retain their foundation §6.1 sequences.
pub trait DirectionRandom {
    fn range_uniform(&mut self, min: f64, max: f64) -> f64;
}
impl DirectionRandom for crate::rng::RzRandom {
    fn range_uniform(&mut self, min: f64, max: f64) -> f64 {
        self.range_f64(min, max)
    }
}
impl DirectionRandom for crate::rng::RzRandomSuper {
    fn range_uniform(&mut self, min: f64, max: f64) -> f64 {
        self.range_f64(min, max)
    }
}
pub fn random_direction_2(random: &mut impl DirectionRandom) -> Vec2 {
    let v = [
        random.range_uniform(-1.0, 1.0) as f32,
        random.range_uniform(-1.0, 1.0) as f32,
    ];
    let length = dot(v, v).sqrt();
    v.map(|x| x / length) // Exactly two draws, including the zero vector.
}
pub fn random_direction_3(random: &mut impl DirectionRandom) -> Vec3 {
    loop {
        let v = [
            random.range_uniform(-1.0, 1.0) as f32,
            random.range_uniform(-1.0, 1.0) as f32,
            random.range_uniform(-1.0, 1.0) as f32,
        ];
        let squared = dot(v, v);
        if (0.0001..=1.0).contains(&squared) {
            let length = squared.sqrt();
            return v.map(|x| x / length);
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod contract_tests;
