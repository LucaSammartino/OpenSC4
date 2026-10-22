// SPDX-License-Identifier: GPL-3.0-or-later
//! Transform, bounds and pick walk: model-instances §§1,5; view-utilities §7.
use super::Mesh;

pub type Vec3 = [f32; 3];
pub type Matrix3 = [Vec3; 3];
pub const IDENTITY: Matrix3 = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];

pub(crate) fn add(a: Vec3, b: Vec3) -> Vec3 {
    std::array::from_fn(|i| a[i] + b[i])
}
pub(crate) fn sub(a: Vec3, b: Vec3) -> Vec3 {
    std::array::from_fn(|i| a[i] - b[i])
}
pub(crate) fn mul(a: Vec3, s: f32) -> Vec3 {
    a.map(|v| v * s)
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
pub fn matrix_vector(m: Matrix3, v: Vec3) -> Vec3 {
    m.map(|row| dot(row, v))
}

/// cS3DTransform; semantic state, deliberately not an ABI struct.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub kind: u8,
    pub rotation: Matrix3,
    pub translation: Vec3,
    pub scale: f32,
    pub change_counter: u8,
}
impl Transform {
    pub fn identity() -> Self {
        Self {
            kind: 0,
            rotation: IDENTITY,
            translation: [0.; 3],
            scale: 1.,
            change_counter: 0,
        }
    }
    pub fn make_identity(&mut self) {
        if self.kind != 0 {
            self.rotation = IDENTITY;
            self.translation = [0.; 3];
            self.scale = 1.;
            self.kind = 0;
            self.change_counter = self.change_counter.wrapping_add(1);
        }
    }
    pub fn point(&self, v: Vec3) -> Vec3 {
        match self.kind {
            0 => v,
            1 => add(v, self.translation),
            2 => mul(v, self.scale),
            3 => add(mul(v, self.scale), self.translation),
            4 => matrix_vector(self.rotation, v),
            5 => add(matrix_vector(self.rotation, v), self.translation),
            _ => add(mul(matrix_vector(self.rotation, v), self.scale), self.translation),
        }
    }
}
impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}
impl BoundingBox {
    pub const EMPTY: Self = Self {
        min: [1.; 3],
        max: [0.; 3],
    };
    pub fn is_empty(self) -> bool {
        (0..3).any(|i| self.min[i] > self.max[i])
    }
    pub fn center(self) -> Vec3 {
        mul(add(self.min, self.max), 0.5)
    }
    pub fn from_points(points: impl IntoIterator<Item = Vec3>) -> Self {
        let mut points = points.into_iter();
        let Some(first) = points.next() else {
            return Self::EMPTY;
        };
        let mut result = Self { min: first, max: first };
        for p in points {
            for (i, value) in p.into_iter().enumerate() {
                if value < result.min[i] {
                    result.min[i] = value;
                }
                if value > result.max[i] {
                    result.max[i] = value;
                }
            }
        }
        result
    }
    pub fn transformed(self, transform: &Transform) -> Self {
        // MI4: full row matrix, independent of kind and endpoint ordering.
        let lo = self.min.map(|v| v * transform.scale);
        let hi = self.max.map(|v| v * transform.scale);
        let mut result = Self {
            min: transform.translation,
            max: transform.translation,
        };
        for r in 0..3 {
            for j in 0..3 {
                let a = transform.rotation[r][j] * lo[j]; // native binary32 store
                                                          // MI11 [compat-limit]: f64 models wider transients, not every
                                                          // x87 precision-control setting. Keep the published stores.
                let b = f64::from(transform.rotation[r][j]) * f64::from(hi[j]);
                let (small, large) = if f64::from(a) < b {
                    (f64::from(a), b)
                } else {
                    (b, f64::from(a))
                };
                result.min[r] = (f64::from(result.min[r]) + small) as f32;
                result.max[r] = (f64::from(result.max[r]) + large) as f32;
            }
        }
        result
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Primitive {
    pub kind: u32,
    pub first: u32,
    pub count: u32,
}
impl Primitive {
    pub fn triangle_count(self) -> i32 {
        match self.kind {
            0 => (self.count as i32) / 3,
            1 | 2 | 7 => (self.count as i32).wrapping_sub(2),
            3 => self.count as i32,
            4 | 6 => (self.count as i32) / 2,
            5 => (self.count as i32).wrapping_sub(1),
            _ => 0,
        }
    }
    pub fn triangle(self, i: u32) -> Option<[usize; 3]> {
        if i64::from(i) >= i64::from(self.triangle_count()) {
            return None;
        }
        let i = u64::from(i);
        let offsets = match self.kind {
            0 => [3 * i, 3 * i + 1, 3 * i + 2],
            1 => [i, i + 1, i + 2],
            2 => [0, i + 1, i + 2],
            3 => [i, i, i],
            4 => [2 * i, 2 * i + 1, 2 * i],
            5 => [i, i + 1, i],
            6 if i % 2 == 0 => [2 * i, 2 * i + 1, 2 * i + 2],
            6 => [2 * i, 2 * i + 2, 2 * i + 3],
            7 => {
                let j = i + i % 2;
                [j, j + 1, j + 2]
            }
            _ => return None,
        };
        let mut result = [0; 3];
        for (to, offset) in result.iter_mut().zip(offsets) {
            *to = usize::try_from(u64::from(self.first) + offset).ok()?;
        }
        Some(result)
    }
}

/// MI9: invalid references in malformed meshes are skipped, never dereferenced;
/// native signed nonpositive primitive counts produce no visits.
pub(crate) fn walk_triangles(mesh: &Mesh, mut visit: impl FnMut([Vec3; 3], [usize; 3]) -> bool) -> bool {
    let Some(block) = &mesh.vertices else {
        return false;
    };
    let vertices = block.read();
    let indices = mesh.indices.as_ref().map(|v| v.read());
    let Some(groups) = &mesh.primitives else {
        return false;
    };
    for group in groups.read().iter() {
        for i in 0..group.triangle_count() {
            let Some(mut ids) = group.triangle(i as u32) else {
                continue;
            };
            let mut valid = true;
            if let Some(indices) = &indices {
                for id in &mut ids {
                    if let Some(value) = indices.get(*id) {
                        *id = usize::from(*value);
                    } else {
                        valid = false;
                        break;
                    }
                }
            }
            if !valid {
                continue;
            }
            let [Some(a), Some(b), Some(c)] = ids.map(|id| vertices.vertices.get(id).map(|v| v.position)) else {
                continue;
            };
            if visit([a, b, c], ids) {
                return true;
            }
        }
    }
    false
}

#[derive(Clone, Copy, Debug)]
pub struct PickContext {
    pub origin: Vec3,
    pub direction: Vec3,
    pub nearest: f32,
}
impl PickContext {
    pub fn triangle_pick(&mut self, a: Vec3, b: Vec3, c: Vec3) -> bool {
        let e1 = sub(b, a);
        let e2 = sub(c, a);
        let p = cross(self.direction, e2);
        let det = dot(e1, p);
        if det.abs() < 0.01 {
            return false;
        }
        let s = sub(self.origin, a);
        let u = dot(s, p) / det;
        let q = cross(s, e1);
        let v = dot(self.direction, q) / det;
        // Native unordered barycentrics pass; the ordered nearest comparison
        // below still rejects a resulting NaN distance (view-utilities §7.2).
        #[allow(clippy::manual_range_contains)]
        if u < 0. || u > 1. || v < 0. || u + v > 1. {
            return false;
        }
        let t = dot(e2, q) / det;
        let hit = add(self.origin, mul(self.direction, t));
        let delta = sub(hit, self.origin);
        let distance = dot(delta, delta).sqrt();
        if distance < self.nearest {
            self.nearest = distance;
            true
        } else {
            false
        }
    }
    pub fn mesh_pick(&mut self, mesh: &Mesh, transform: &Transform) -> bool {
        walk_triangles(mesh, |points, _| {
            let [a, b, c] = points.map(|p| transform.point(p));
            self.triangle_pick(a, b, c)
        })
    }
    pub fn model_pick(&mut self, meshes: &[Mesh], transform: &Transform) -> bool {
        let mut hit = false;
        for mesh in meshes {
            hit = self.mesh_pick(mesh, transform) || hit;
        }
        hit
    }
}
