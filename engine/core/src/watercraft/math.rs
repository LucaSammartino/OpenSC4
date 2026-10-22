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

/// Shortest horizontal rotation, with sign from the y cross product (§5.3).
pub(super) fn turn(a: Vec3, b: Vec3, max_radians: f32) -> Option<Vec3> {
    let a = unit(flat(a))?;
    let b = unit(flat(b))?;
    let radians = angle(a, b)?;
    if radians <= max_radians {
        return Some(b);
    }
    // WC7: antiparallel directions do not specify a sign. Do not invent one.
    let cross_y = a[2] * b[0] - a[0] * b[2];
    if cross_y == 0.0 {
        return None;
    }
    let theta = max_radians * cross_y.signum();
    let (sin, cos) = theta.sin_cos();
    Some([a[0] * cos + a[2] * sin, 0.0, a[2] * cos - a[0] * sin])
}

pub(super) fn approach(a: Vec3, b: Vec3, distance: f32) -> Vec3 {
    let delta = sub(b, a);
    let len = length(delta);
    if len <= distance || len == 0.0 {
        b
    } else {
        add(a, scale(delta, distance / len))
    }
}
