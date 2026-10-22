// SPDX-License-Identifier: GPL-3.0-or-later
//! §7 owned triangle traversal, resource transforms and nearest-distance state.
use super::{Ray, Vec3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pick {
    pub ray: Ray,
    pub nearest_distance: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub kind: u8,
    pub matrix: [[f32; 3]; 3],
    pub translation: Vec3,
    pub scale: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            kind: 0,
            matrix: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            translation: Vec3::ZERO,
            scale: 1.0,
        }
    }
}

impl Transform {
    pub fn apply(&self, p: Vec3) -> Vec3 {
        let matrix = || {
            Vec3::new(
                Vec3::new(self.matrix[0][0], self.matrix[0][1], self.matrix[0][2]).dot(p),
                Vec3::new(self.matrix[1][0], self.matrix[1][1], self.matrix[1][2]).dot(p),
                Vec3::new(self.matrix[2][0], self.matrix[2][1], self.matrix[2][2]).dot(p),
            )
        };
        match self.kind {
            0 => p,
            1 => p + self.translation,
            2 => p * self.scale,
            3 => p * self.scale + self.translation,
            4 => matrix(),
            5 => matrix() + self.translation,
            _ => matrix() * self.scale + self.translation,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrimitiveGroup {
    pub kind: u32,
    pub first: u32,
    pub count: u32,
}

impl PrimitiveGroup {
    pub fn triangle_count(self) -> u32 {
        // VU10: undersized groups are empty, avoiding native unsigned underflow.
        match self.kind {
            0 => self.count / 3,
            1 | 2 | 7 => self.count.saturating_sub(2),
            3 => self.count,
            4 | 6 => self.count / 2,
            5 => self.count.saturating_sub(1),
            _ => 0,
        }
    }
}

/// Relative indices, exactly as the §7.1 table, including unusual types 6/7.
/// Bounds refer to triangle count, not vertex count; mesh access is checked.
pub fn triangle_indices(group: PrimitiveGroup, i: u32) -> Option<[u32; 3]> {
    if i >= group.triangle_count() {
        return None;
    }
    Some(match group.kind {
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
    })
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub indices: Option<Vec<u16>>,
    pub groups: Vec<PrimitiveGroup>,
}

impl Mesh {
    /// Typed resource input from the §7.1 position/stride/count layout. Padding
    /// is ignored; all positions are copied, never borrowed from a game pointer.
    /// VU10: malformed buffers/strides fail explicitly in safe Rust.
    pub fn from_vertex_data(
        data: &[u8],
        stride: u16,
        count: u16,
        indices: Option<Vec<u16>>,
        groups: Vec<PrimitiveGroup>,
    ) -> Option<Self> {
        if count != 0 && stride < 12 {
            return None;
        }
        let mut vertices = Vec::with_capacity(usize::from(count));
        for i in 0..usize::from(count) {
            let base = i * usize::from(stride);
            let bytes = data.get(base..base + 12)?;
            let component = |offset| Some(f32::from_le_bytes(bytes.get(offset..offset + 4)?.try_into().ok()?));
            vertices.push(Vec3::new(component(0)?, component(4)?, component(8)?));
        }
        Some(Self {
            vertices,
            indices,
            groups,
        })
    }

    fn vertex(&self, first: u32, relative: u32) -> Option<Vec3> {
        let index = usize::try_from(first.checked_add(relative)?).ok()?;
        let index = if let Some(indices) = &self.indices {
            usize::from(*indices.get(index)?)
        } else {
            index
        };
        self.vertices.get(index).copied()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Model {
    pub meshes: Vec<Mesh>,
}

/// The external instance supplies only its resource transform, not picking.
pub trait ModelInstance {
    fn fill_transform(&self, mesh_index: usize, transform: &mut Transform);
}

/// VU6: geometry utility details are missing. Literal two-sided half-ray /
/// triangle intersection, inclusive barycentric edges and t=0, exact zero
/// determinant rejection, no invented epsilon. Direction is not normalized here.
pub fn triangle_pick(pick: &mut Pick, a: Vec3, b: Vec3, c: Vec3) -> bool {
    let e1 = b - a;
    let e2 = c - a;
    let p = pick.ray.direction.cross(e2);
    let determinant = e1.dot(p);
    if determinant == 0.0 {
        return false;
    }
    let inverse = 1.0 / determinant;
    let delta = pick.ray.origin - a;
    let u = delta.dot(p) * inverse;
    if !(0.0..=1.0).contains(&u) {
        return false;
    }
    let q = delta.cross(e1);
    let v = pick.ray.direction.dot(q) * inverse;
    if v < 0.0 || u + v > 1.0 {
        return false;
    }
    let distance = e2.dot(q) * inverse;
    if distance >= 0.0 && distance < pick.nearest_distance {
        pick.nearest_distance = distance;
        true
    } else {
        false
    }
}

pub fn mesh_pick(pick: &mut Pick, mesh: &Mesh, transform: &Transform) -> bool {
    if mesh.vertices.is_empty() {
        return false;
    }
    for &group in &mesh.groups {
        // Every table row references at least one element >= i. Past the
        // available input length all subsequent triangles must be invalid;
        // avoid walking billions of known misses from malformed resource counts.
        let source_len = mesh.indices.as_ref().map_or(mesh.vertices.len(), Vec::len);
        let available = source_len.saturating_sub(group.first as usize);
        let limit = group.triangle_count().min(u32::try_from(available).unwrap_or(u32::MAX));
        for i in 0..limit {
            let Some(indices) = triangle_indices(group, i) else {
                continue;
            };
            let [a, b, c] = indices.map(|index| mesh.vertex(group.first, index));
            // VU10: an invalid resource reference skips that triangle.
            if let (Some(a), Some(b), Some(c)) = (a, b, c) {
                if triangle_pick(pick, transform.apply(a), transform.apply(b), transform.apply(c)) {
                    return true; // §7 first closer triangle, NOT nearest in mesh.
                }
            }
        }
    }
    false
}

pub fn model_pick(pick: &mut Pick, instance: &dyn ModelInstance, model: &Model) -> bool {
    let mut hit = false;
    for (index, mesh) in model.meshes.iter().enumerate() {
        let mut transform = Transform::default();
        instance.fill_transform(index, &mut transform);
        // Do not short circuit: subsequent meshes can improve the distance.
        hit |= mesh_pick(pick, mesh, &transform);
    }
    hit
}
