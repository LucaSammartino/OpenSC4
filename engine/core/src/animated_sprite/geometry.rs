// SPDX-License-Identifier: GPL-3.0-or-later
//! Published transform algebra (view-utilities §7) and billboard primitives.
//! This file retains portable transform/billboard choices; the separately
//! qualified cached path is in cached.rs, without sprite-owner callbacks.

pub type Vec3 = [f32; 3];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub kind: u8,
    pub rotation: [f32; 9],
    pub translation: Vec3,
    pub scale: f32,
    pub change: u8,
}

fn add(a: Vec3, b: Vec3) -> Vec3 {
    std::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: Vec3, b: Vec3) -> Vec3 {
    std::array::from_fn(|i| a[i] - b[i])
}
fn mul(a: Vec3, s: f32) -> Vec3 {
    a.map(|v| v * s)
}
fn rotate(m: [f32; 9], v: Vec3) -> Vec3 {
    std::array::from_fn(|i| (m[i * 3] * v[0] + m[i * 3 + 1] * v[1]) + m[i * 3 + 2] * v[2])
}

impl Transform {
    fn has_translation(self) -> bool {
        !matches!(self.kind, 0 | 2 | 4)
    }
    fn has_rotation(self) -> bool {
        self.kind >= 4
    }
    fn has_scale(self) -> bool {
        self.kind == 2 || self.kind == 3 || self.kind >= 6
    }

    pub fn vector(self, vector: Vec3) -> Vec3 {
        let value = if self.has_rotation() {
            rotate(self.rotation, vector)
        } else {
            vector
        };
        if self.has_scale() {
            mul(value, self.scale)
        } else {
            value
        }
    }

    pub fn point(self, point: Vec3) -> Vec3 {
        let value = self.vector(point);
        if self.has_translation() {
            add(value, self.translation)
        } else {
            value
        }
    }

    /// Rotation inverse is its transpose (camera §1.3). Input is a rotation,
    /// not a general shear matrix. Zero scale has no inverse.
    pub fn inverse_vector(self, vector: Vec3) -> Option<Vec3> {
        let value = if self.has_scale() {
            if self.scale == 0.0 {
                return None;
            }
            mul(vector, 1.0 / self.scale)
        } else {
            vector
        };
        let m = self.rotation;
        Some(if self.has_rotation() {
            rotate([m[0], m[3], m[6], m[1], m[4], m[7], m[2], m[5], m[8]], value)
        } else {
            value
        })
    }

    pub fn inverse_point(self, point: Vec3) -> Option<Vec3> {
        self.inverse_vector(if self.has_translation() {
            sub(point, self.translation)
        } else {
            point
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: Vec3,
    pub max: Vec3,
}
impl Bounds {
    pub fn center(self) -> Vec3 {
        mul(add(self.min, self.max), 0.5)
    }
    pub fn widen(&mut self, corners: &[Vec3]) {
        for corner in corners {
            for (axis, &value) in corner.iter().enumerate() {
                self.min[axis] = self.min[axis].min(value);
                self.max[axis] = self.max[axis].max(value);
            }
        }
    }
}

/// §5's metric quad. This helper's explicit anchor is the lower-left corner;
/// AS7 blocks assigning that convention to native frame offsets or Draw.
/// Axes are camera primitives; the provider cannot return computed vertices.
pub fn billboard_corners(
    anchor: Vec3,
    right: Vec3,
    up: Vec3,
    pixels: [f32; 2],
    offset: [f32; 2],
    metres_per_pixel: f32,
) -> [Vec3; 4] {
    let base = add(
        add(anchor, mul(right, offset[0] * metres_per_pixel)),
        mul(up, offset[1] * metres_per_pixel),
    );
    let width = mul(right, pixels[0] * metres_per_pixel);
    let height = mul(up, pixels[1] * metres_per_pixel);
    [base, add(base, width), add(add(base, width), height), add(base, height)]
}

/// The external TrianglePick primitive (view-utilities §7), not an original
/// sprite Pick method. It owns the ray record and nearest-distance update.
pub trait TrianglePicker {
    fn triangle_pick(&mut self, a: Vec3, b: Vec3, c: Vec3) -> bool;
}

/// §6: raw triangle order 012 then 230, stopping at the first hit.
pub fn pick_quad(corners: &[Vec3; 4], picker: &mut impl TrianglePicker) -> bool {
    picker.triangle_pick(corners[0], corners[1], corners[2]) || picker.triangle_pick(corners[2], corners[3], corners[0])
}
