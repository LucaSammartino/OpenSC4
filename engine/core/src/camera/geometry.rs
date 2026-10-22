// SPDX-License-Identifier: GPL-3.0-or-later
use crate::network_occupants::xform::Xform;

pub type Vec2 = [f32; 2];
pub type Vec3 = [f32; 3];
/// Row major, applied to column vectors.
pub type Mat4 = [[f32; 4]; 4];

pub(crate) fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub(crate) fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub(crate) fn mul(a: Vec3, s: f32) -> Vec3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
pub(crate) fn dot(a: Vec3, b: Vec3) -> f32 {
    (a[0] * b[0] + a[1] * b[1]) + a[2] * b[2]
}
pub(crate) fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub(super) fn normalize(a: Vec3) -> Vec3 {
    // CA3: no fallback axes are specified for a zero/parallel look-at.
    // Literal normalization retains IEEE division-by-zero/NaN behaviour.
    mul(a, 1.0 / dot(a, a).sqrt())
}
pub(super) fn row(m: &[f32; 9], i: usize) -> Vec3 {
    [m[i * 3], m[i * 3 + 1], m[i * 3 + 2]]
}
pub(super) fn rotate(m: &[f32; 9], v: Vec3) -> Vec3 {
    [dot(row(m, 0), v), dot(row(m, 1), v), dot(row(m, 2), v)]
}
pub(super) fn transpose(m: &[f32; 9]) -> [f32; 9] {
    [m[0], m[3], m[6], m[1], m[4], m[7], m[2], m[5], m[8]]
}
pub(crate) fn transform(x: &Xform, p: Vec3) -> Vec3 {
    add(mul(rotate(&x.matrix, p), x.scale), x.translation)
}
pub(super) fn apply(m: &Mat4, p: [f32; 4]) -> [f32; 4] {
    m.map(|r| ((r[0] * p[0] + r[1] * p[1]) + r[2] * p[2]) + r[3] * p[3])
}
pub(super) fn homogeneous(p: Vec3) -> [f32; 4] {
    [p[0], p[1], p[2], 1.0]
}
pub(super) fn divide(p: [f32; 4]) -> Option<Vec3> {
    if p[3].abs() < 0.0001 {
        None
    } else {
        Some([p[0] / p[3], p[1] / p[3], p[2] / p[3]])
    }
}

/// Camera-control §2.4: rejects a parallel ray and an intersection behind it.
pub fn intersect_with_ground_plane(direction: Vec3, origin: Vec3) -> Option<Vec3> {
    if direction[1].abs() < 0.0001 {
        return None;
    }
    let t = -origin[1] / direction[1];
    if t < 0.0 {
        None
    } else {
        Some(add(origin, mul(direction, t)))
    }
}

pub(super) fn snap(x: f32) -> f32 {
    // §2.3: explicitly round at the f32 store between addition/subtraction.
    let rounded = x + 12_582_912.0_f32;
    rounded - 12_582_912.0_f32
}
