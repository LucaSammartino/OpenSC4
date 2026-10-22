// SPDX-License-Identifier: GPL-3.0-or-later
//! Small value types for view-utilities.md. Arithmetic follows the written order,
//! without fused multiply-add. See README for the portable numeric boundary.

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
        // §2.2: no zero guard. Widen products before sqrt and store components
        // once. f64 is a portable evaluation policy, not x87 bit emulation.
        let x = f64::from(self.x);
        let y = f64::from(self.y);
        let z = f64::from(self.z);
        let inverse = 1.0 / ((x * x + y * y) + z * z).sqrt();
        Self::new((x * inverse) as f32, (y * inverse) as f32, (z * inverse) as f32)
    }
    pub fn min(self, other: Self) -> Self {
        Self::new(
            if other.x < self.x { other.x } else { self.x },
            if other.y < self.y { other.y } else { self.y },
            if other.z < self.z { other.z } else { self.z },
        )
    }
    pub fn max(self, other: Self) -> Self {
        Self::new(
            if other.x > self.x { other.x } else { self.x },
            if other.y > self.y { other.y } else { self.y },
            if other.z > self.z { other.z } else { self.z },
        )
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
        Pixel::new(
            self.top_left.x.wrapping_add(self.bottom_right.x) / 2,
            self.top_left.y.wrapping_add(self.bottom_right.y) / 2,
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

/// §12 005D6900: first point initializes even for a nonpositive count. Empty
/// or insufficient storage has no native defined result; checked policy is None.
pub fn point_array_aabb(points: &[Vec3], count: i32) -> Option<Aabb> {
    let count = usize::try_from(count.max(1)).ok()?;
    Some(Aabb::from_points(points.get(..count)?))
}

/// §12 005D6F10: writes x, then y, then z, rereading aliased operands. Indices
/// make the native aliasing contract expressible without aliased Rust borrows.
pub fn cross_in_storage(storage: &mut [Vec3], a: usize, b: usize, out: usize) -> Option<()> {
    storage.get(a)?;
    storage.get(b)?;
    storage.get(out)?;
    storage[out].x = storage[a].y * storage[b].z - storage[a].z * storage[b].y;
    storage[out].y = storage[a].z * storage[b].x - storage[a].x * storage[b].z;
    storage[out].z = storage[a].x * storage[b].y - storage[a].y * storage[b].x;
    Some(())
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

/// §7.2 geometry plane predicate, including coplanarity and negative t.
pub fn ray_intersects_ground_plane(ray: Ray, out: &mut Vec3) -> bool {
    ray_intersects_plane(ray, Vec3::new(0.0, 1.0, 0.0), 0.0, Some(out))
}

// Exact binary32 word 0x3C23D70A; literal keeps the project's Rust 1.75 MSRV.
pub const GEOMETRY_TOLERANCE: f32 = 0.01;

pub fn ray_intersects_plane(ray: Ray, normal: Vec3, offset: f32, out: Option<&mut Vec3>) -> bool {
    let a = normal.dot(ray.origin) + offset;
    let b = normal.dot(ray.direction);
    let (hit, point) = if b.abs() < GEOMETRY_TOLERANCE {
        if a.abs() < GEOMETRY_TOLERANCE {
            (true, ray.origin)
        } else {
            (false, Vec3::ZERO)
        }
    } else {
        // Unordered b deliberately takes this branch.
        (true, ray.origin - ray.direction * (a / b))
    };
    if let Some(out) = out {
        *out = point;
    }
    hit
}

/// Separate 87-byte helpers (§12); ordered comparisons ignore NaNs and retain
/// the first signed zero. Infinity does not replace the finite starting bound.
pub fn min_of_four_floats(values: [f32; 4]) -> f32 {
    values.into_iter().fold(f32::MAX, |a, b| if b < a { b } else { a })
}
pub fn max_of_four_floats(values: [f32; 4]) -> f32 {
    values.into_iter().fold(-f32::MAX, |a, b| if b > a { b } else { a })
}
