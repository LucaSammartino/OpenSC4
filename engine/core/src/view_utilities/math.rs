// SPDX-License-Identifier: GPL-3.0-or-later
//! Small value types for view-utilities.md. Arithmetic follows the written order,
//! with separate f32 operations (no fused multiply-add).

use std::ops::{Add, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
    pub fn dot(self, other: Self) -> f32 {
        (self.x * other.x + self.y * other.y) + self.z * other.z
    }
    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }
    pub fn normalized(self) -> Self {
        // VU5: zero-length/nonfinite camera basis behavior is not specified.
        // Literal division preserves IEEE NaNs rather than inventing a basis.
        self * (1.0 / self.dot(self).sqrt())
    }
    pub fn min(self, other: Self) -> Self {
        Self::new(self.x.min(other.x), self.y.min(other.y), self.z.min(other.z))
    }
    pub fn max(self, other: Self) -> Self {
        Self::new(self.x.max(other.x), self.y.max(other.y), self.z.max(other.z))
    }
}

impl Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}
impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}
impl Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}
impl Neg for Vec3 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pixel {
    pub x: i32,
    pub y: i32,
}

impl Pixel {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PixelRect {
    pub top_left: Pixel,
    pub bottom_right: Pixel,
}

impl PixelRect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            top_left: Pixel::new(left, top),
            bottom_right: Pixel::new(right, bottom),
        }
    }
    pub fn center(self) -> Pixel {
        // VU8: integer halves are specified, overflow is not. Widen first.
        Pixel::new(
            ((i64::from(self.top_left.x) + i64::from(self.bottom_right.x)) / 2) as i32,
            ((i64::from(self.top_left.y) + i64::from(self.bottom_right.y)) / 2) as i32,
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn from_points(points: &[Vec3]) -> Self {
        let Some(&first) = points.first() else {
            return Self::default();
        };
        points
            .iter()
            .skip(1)
            .fold(Self { min: first, max: first }, |b, &p| Self {
                min: b.min.min(p),
                max: b.max.max(p),
            })
    }
    pub(crate) fn support(self, normal: Vec3) -> Vec3 {
        Vec3::new(
            if normal.x >= 0.0 { self.max.x } else { self.min.x },
            if normal.y >= 0.0 { self.max.y } else { self.min.y },
            if normal.z >= 0.0 { self.max.z } else { self.min.z },
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

/// §4.1 unguarded ground intersection. Parallel rays deliberately produce
/// infinities/NaNs; this is different from the checked ray-plane test.
pub fn get_ray_intersection_with_ground_plane(ray: Ray) -> Vec3 {
    ray.origin + ray.direction * (-ray.origin.y / ray.direction.y)
}

/// §4.1 geometry ray-plane test. VU6: literal forward half-ray, including t=0;
/// exactly parallel rays miss and leave the output alone. No epsilon is invented.
pub fn ray_intersects_ground_plane(ray: Ray, out: &mut Vec3) -> bool {
    if ray.direction.y == 0.0 {
        return false;
    }
    let t = -ray.origin.y / ray.direction.y;
    if t >= 0.0 {
        *out = ray.origin + ray.direction * t;
        true
    } else {
        false
    }
}
