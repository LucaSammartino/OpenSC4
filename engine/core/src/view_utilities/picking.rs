// SPDX-License-Identifier: GPL-3.0-or-later
//! §7 owned triangle traversal, resource transforms and nearest-distance state.
use super::{math::GEOMETRY_TOLERANCE, Ray, Vec3};

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
        self.apply_vertex(p, false)
    }

    fn apply_vertex(&self, p: Vec3, first: bool) -> Vec3 {
        // §7.2 row order z,y,x. f64 policy retains intermediates at the known
        // stores; it is not a claim of x87 extended/trig bit conformance.
        let dot = |row: [f32; 3]| {
            (f64::from(p.z) * f64::from(row[2]) + f64::from(p.y) * f64::from(row[1]))
                + f64::from(p.x) * f64::from(row[0])
        };
        let rows = self.matrix.map(dot);
        let matrix = || Vec3::new(rows[0] as f32, rows[1] as f32, rows[2] as f32);
        match self.kind {
            0 => p,
            1 => p + self.translation,
            2 => p * self.scale,
            3 => p * self.scale + self.translation,
            4 => matrix(),
            5 => matrix() + self.translation,
            _ => {
                let t = self.translation;
                let scale = f64::from(self.scale);
                let component = |row: f64, translation: f32, z: bool| {
                    if first && !z {
                        // x/y store scaled row before translation.
                        (row * scale) as f32 + translation
                    } else if first {
                        // First z instead stores row before scale/translation.
                        (f64::from(row as f32) * scale + f64::from(translation)) as f32
                    } else {
                        (row * scale + f64::from(translation)) as f32
                    }
                };
                Vec3::new(
                    component(rows[0], t.x, false),
                    component(rows[1], t.y, false),
                    component(rows[2], t.z, true),
                )
            }
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
    pub fn triangle_count(self) -> i32 {
        let n = self.count as i32;
        match self.kind {
            0 => n / 3,
            1 | 2 | 7 => n.wrapping_sub(2),
            3 => n,
            4 | 6 => n / 2,
            5 => n.wrapping_sub(1),
            _ => 0,
        }
    }
}

/// Relative indices, exactly as the §7.1 table, including unusual types 6/7.
/// Bounds refer to triangle count, not vertex count; mesh access is checked.
pub fn triangle_indices(group: PrimitiveGroup, i: u32) -> Option<[u32; 3]> {
    if group.triangle_count() <= 0 || i >= group.triangle_count() as u32 {
        return None;
    }
    let mut out = [0; 3];
    primitive_vertex_indices(group.kind, i, &mut out);
    Some(out)
}

/// Raw helper has no count guard; unknown type preserves all output words.
pub fn primitive_vertex_indices(kind: u32, i: u32, out: &mut [u32; 3]) {
    let a = i.wrapping_mul(2);
    let b = i.wrapping_mul(3);
    *out = match kind {
        0 => [b, b.wrapping_add(1), b.wrapping_add(2)],
        1 => [i, i.wrapping_add(1), i.wrapping_add(2)],
        2 => [0, i.wrapping_add(1), i.wrapping_add(2)],
        3 => [i, i, i],
        4 => [a, a.wrapping_add(1), a],
        5 => [i, i.wrapping_add(1), i],
        6 if i % 2 == 0 => [a, a.wrapping_add(1), a.wrapping_add(2)],
        6 => [a, a.wrapping_add(2), a.wrapping_add(3)],
        7 => {
            let j = i.wrapping_add(((i as i32) % 2) as u32);
            [j, j.wrapping_add(1), j.wrapping_add(2)]
        }
        _ => return,
    };
}

/// §12 generic neighbour index decoder. Unknown format preserves output. Byte
/// accesses are checked; native invalid addresses are not reproduced. Numeric
/// formats have the explicit finite bias-range precondition below (VU13).
pub fn decode_indices(format: u32, data: &[u8], indices: [u32; 3], out: &mut [u32; 3]) -> Option<()> {
    let width = match format {
        0 | 1 => 1,
        2 | 3 => 2,
        4..=6 => 4,
        7 => 8,
        _ => return Some(()),
    };
    let mut result = [0; 3];
    for (slot, index) in result.iter_mut().zip(indices) {
        let offset = index.wrapping_mul(width) as usize;
        let bytes = data.get(offset..offset.checked_add(width as usize)?)?;
        *slot = match format {
            0 => i32::from(bytes[0] as i8) as u32,
            1 => u32::from(bytes[0]),
            2 => i32::from(i16::from_le_bytes(bytes.try_into().ok()?)) as u32,
            3 => u32::from(u16::from_le_bytes(bytes.try_into().ok()?)),
            4 | 5 => u32::from_le_bytes(bytes.try_into().ok()?),
            6 | 7 => {
                let value = if format == 6 {
                    f32::from_le_bytes(bytes.try_into().ok()?)
                } else {
                    f64::from_le_bytes(bytes.try_into().ok()?) as f32
                };
                // Within this interval the bias conversion is nearest/even;
                // correcting a rounded-up result gives floor, including exact
                // integers. No successful result invented for unclear extremes.
                if !(-4_194_304.0..4_194_304.0).contains(&value) {
                    return None;
                }
                let nearest = crate::x87::rhe(value);
                if nearest as f32 > value {
                    nearest.wrapping_sub(1) as u32
                } else {
                    nearest as u32
                }
            }
            _ => unreachable!(),
        };
    }
    *out = result;
    Some(())
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
        // Stride below 12 is legal if each overlapping 12-byte read exists.
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
        let index = usize::try_from(first.wrapping_add(relative)).ok()?;
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

/// §7.2 two-sided line/triangle predicate, including unordered comparisons.
/// Rejection preserves the optional output. There is deliberately no t test.
#[allow(clippy::manual_range_contains)] // unordered barycentrics pass native rejection tests
pub fn ray_intersects_triangle(ray: Ray, a: Vec3, b: Vec3, c: Vec3, out: Option<&mut Vec3>) -> bool {
    let e1 = b - a;
    let e2 = c - a;
    let p = ray.direction.cross(e2);
    let determinant = e1.dot(p);
    if determinant.abs() < GEOMETRY_TOLERANCE {
        return false;
    }
    let inverse = 1.0 / determinant;
    let delta = ray.origin - a;
    let u = delta.dot(p) * inverse;
    if u < 0.0 || u > 1.0 {
        return false;
    }
    let q = delta.cross(e1);
    let v = ray.direction.dot(q) * inverse;
    if v < 0.0 || u + v > 1.0 {
        return false;
    }
    let t = e2.dot(q) * inverse;
    if let Some(out) = out {
        *out = ray.origin + ray.direction * t;
    }
    true
}

pub fn triangle_pick(pick: &mut Pick, a: Vec3, b: Vec3, c: Vec3) -> bool {
    let mut point = Vec3::ZERO;
    if !ray_intersects_triangle(pick.ray, a, b, c, Some(&mut point)) {
        return false;
    }
    let delta = point - pick.ray.origin;
    let distance = delta.dot(delta).sqrt();
    if distance < pick.nearest_distance {
        pick.nearest_distance = distance;
        true
    } else {
        false
    }
}

pub fn mesh_pick(pick: &mut Pick, mesh: &Mesh, transform: &Transform) -> bool {
    // Checked adapter: malformed resource aborts, never skips to a later hit.
    mesh_pick_checked(pick, mesh, transform).unwrap_or(false)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidMeshReference;

/// Borrowed owned-resource iterator: dropping it never frees vertex/index data.
/// Rust scope/drop replaces the native iterator vtable/deleting-destructor ABI.
pub struct TriangleIterator<'a> {
    mesh: &'a Mesh,
    group: PrimitiveGroup,
    current: i32,
}

impl<'a> TriangleIterator<'a> {
    pub fn select_group(mesh: &'a Mesh, index: usize) -> Option<Self> {
        Some(Self {
            mesh,
            group: *mesh.groups.get(index)?,
            current: 0,
        })
    }

    pub fn next_triangle(&mut self, out: &mut [Vec3; 3]) -> Result<bool, InvalidMeshReference> {
        if self.current >= self.group.triangle_count() {
            return Ok(false);
        }
        let mut indices = [0; 3];
        primitive_vertex_indices(self.group.kind, self.current as u32, &mut indices);
        let points = indices.map(|i| self.mesh.vertex(self.group.first, i));
        let [Some(a), Some(b), Some(c)] = points else {
            return Err(InvalidMeshReference);
        };
        *out = [a, b, c];
        self.current = self.current.wrapping_add(1);
        Ok(true)
    }
}

pub fn mesh_pick_checked(pick: &mut Pick, mesh: &Mesh, transform: &Transform) -> Result<bool, InvalidMeshReference> {
    if mesh.vertices.is_empty() {
        return Ok(false);
    }
    for group in 0..mesh.groups.len() {
        let mut iterator = TriangleIterator::select_group(mesh, group).ok_or(InvalidMeshReference)?;
        let mut points = [Vec3::ZERO; 3];
        while iterator.next_triangle(&mut points)? {
            let [a, b, c] = points;
            if triangle_pick(
                pick,
                transform.apply_vertex(a, true),
                transform.apply_vertex(b, false),
                transform.apply_vertex(c, false),
            ) {
                return Ok(true); // First improving triangle, not nearest in mesh.
            }
        }
    }
    Ok(false)
}

pub fn model_pick(pick: &mut Pick, instance: &dyn ModelInstance, model: &Model) -> bool {
    model_pick_checked(pick, instance, model).unwrap_or(false)
}

pub fn model_pick_checked(
    pick: &mut Pick,
    instance: &dyn ModelInstance,
    model: &Model,
) -> Result<bool, InvalidMeshReference> {
    let mut hit = false;
    for (index, mesh) in model.meshes.iter().enumerate() {
        let mut transform = Transform::default();
        instance.fill_transform(index, &mut transform);
        // Do not short circuit: subsequent meshes can improve the distance.
        hit |= mesh_pick_checked(pick, mesh, &transform)?;
    }
    Ok(hit)
}
