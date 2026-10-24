// SPDX-License-Identifier: GPL-3.0-or-later
use crate::automata::Vec3;

pub(super) fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub(super) fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub(super) fn scale(a: Vec3, s: f32) -> Vec3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

pub(super) fn dot(a: Vec3, b: Vec3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub(super) fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Deliberately unguarded normalization where the spec has no finite fallback.
pub(super) fn normalize(a: Vec3) -> Vec3 {
    scale(a, 1.0 / length(a))
}

pub(super) fn yaw(a: Vec3, theta: f32) -> Vec3 {
    let (s, c) = theta.sin_cos();
    [c * a[0] + s * a[2], a[1], -s * a[0] + c * a[2]]
}

pub(super) fn length(a: Vec3) -> f32 {
    dot(a, a).sqrt()
}

pub(super) fn unit(a: Vec3) -> Option<Vec3> {
    let len = length(a);
    (len > 0.0 && len.is_finite()).then(|| scale(a, 1.0 / len))
}

pub(super) fn flat(a: Vec3) -> Vec3 {
    [a[0], 0.0, a[2]]
}

pub(super) fn angle(a: Vec3, b: Vec3) -> Option<f32> {
    Some(dot(unit(a)?, unit(b)?).clamp(-1.0, 1.0).acos())
}
