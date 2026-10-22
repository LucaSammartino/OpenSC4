// SPDX-License-Identifier: GPL-3.0-or-later
use super::{
    geometry::{add, dot, walk_triangles},
    *,
};

/// Foreign lighting service (lighting §4.4). The processor supplies the normals
/// and rotation it computed; the provider only supplies the lighting colours.
pub trait VertexLighting {
    fn get_vertex_colors(&mut self, normals: &[Vec3], rotation: Matrix3, colours: &mut [u32]);
}

/// cS3DModelProcessor. The instance and mesh-container references are separately
/// owned, even though both refer to the same most-derived allocation.
#[derive(Default)]
pub struct ModelProcessor {
    instance: Option<InstanceReference>,
    container: Option<InstanceReference>,
    borrowed_container: Option<BorrowedInstanceReference>,
    already_released: bool,
    originals: Vec<Option<Reference<VertexBlock>>>,
    normals: Option<Vec<Vec<Vec3>>>,
    mesh_order: Vec<usize>,
}
impl ModelProcessor {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn target_format(&self) -> u32 {
        10
    }
    pub fn has_instance(&self) -> bool {
        self.instance.is_some()
    }
    pub fn has_container(&self) -> bool {
        self.container.is_some() || self.borrowed_container.is_some()
    }
    pub fn container_is_retained(&self) -> bool {
        self.container.is_some()
    }
    fn access_container(&self) -> Option<InstanceReference> {
        self.container
            .as_ref()
            .map(InstanceReference::borrowed_access)
            .or_else(|| {
                self.borrowed_container
                    .as_ref()
                    .and_then(BorrowedInstanceReference::access)
            })
    }
    pub fn normals(&self) -> Option<&[Vec<Vec3>]> {
        self.normals.as_deref()
    }
    pub fn mesh_order(&self) -> &[usize] {
        &self.mesh_order
    }
    fn restore_originals(&mut self) {
        if let Some(container) = self.access_container() {
            container.with_instance_mut(|instance| {
                for (i, original) in self.originals.iter().enumerate() {
                    if let Some(mesh) = instance.mesh_mut(i) {
                        // Retain the restored slot before releasing the replaced
                        // block, then discard the saved reference (MI6).
                        mesh.vertices = original.clone();
                    }
                }
            });
        }
    }
    pub fn set_model(&mut self, instance: Option<&InstanceReference>) -> bool {
        self.restore_originals();
        self.originals.clear();
        self.normals = None;
        self.mesh_order.clear();
        self.instance = None;
        self.container = None;
        self.borrowed_container = None;
        self.already_released = false;
        let Some(instance) = instance else {
            return false;
        };
        if !instance.has_mesh_container() || instance.animated_frame_count().is_some_and(|n| n > 1) {
            return false;
        }
        let valid = instance.with_instance(|model| {
            model.mesh_count() != 0
                && model
                    .meshes()
                    .iter()
                    .all(|mesh| mesh.vertices.as_ref().is_some_and(|v| v.read().format == 2))
        });
        if !valid {
            return false;
        }
        self.instance = Some(instance.clone());
        self.container = Some(instance.clone());
        instance.with_instance_mut(|instance| {
            for i in 0..instance.mesh_count() {
                let Some(mesh) = instance.mesh_mut(i) else {
                    continue;
                };
                let original = mesh.vertices.take();
                if let Some(block) = &original {
                    let block = block.read();
                    let vertices = block
                        .vertices
                        .iter()
                        .map(|v| Vertex {
                            colour: 0xFFFF_FFFF,
                            ..*v
                        })
                        .collect();
                    mesh.vertices = Some(Reference::new(VertexBlock {
                        format: 10,
                        vertices,
                        bounds: block.bounds,
                    }));
                }
                self.originals.push(original);
                self.mesh_order.push(i);
            }
        });
        true
    }
    pub fn create_vertex_normals(&mut self) {
        let Some(container) = self.access_container() else {
            return;
        };
        self.normals = Some(container.with_instance(|instance| {
            instance
                .meshes()
                .iter()
                .map(|mesh| {
                    // MI11: retain wider sums until the published score store;
                    // exact x87 control-word-dependent last bits are not claimed.
                    let count = mesh.vertices.as_ref().map_or(0, |v| v.read().vertices.len());
                    let mut sums = vec![[0.; 3]; count];
                    let mut weights = vec![0_f32; count];
                    walk_triangles(mesh, |[a, b, c], ids| {
                        // Newell's formula, unnormalized: length = twice triangle area.
                        let mut normal = [0.; 3];
                        for (p, q) in [(a, b), (b, c), (c, a)] {
                            normal[0] += (p[1] - q[1]) * (p[2] + q[2]);
                            normal[1] += (p[2] - q[2]) * (p[0] + q[0]);
                            normal[2] += (p[0] - q[0]) * (p[1] + q[1]);
                        }
                        let weight = dot(normal, normal).sqrt();
                        for id in ids {
                            sums[id] = add(sums[id], normal);
                            weights[id] += weight;
                        }
                        false
                    });
                    for (normal, weight) in sums.iter_mut().zip(weights) {
                        if weight > 0. {
                            *normal = normal.map(|component| component / weight);
                        }
                    }
                    sums
                })
                .collect()
        }));
    }
    pub fn light_model(&mut self, lighting: &mut impl VertexLighting, flip: bool) {
        if self.normals.is_none() {
            self.create_vertex_normals();
        }
        let (Some(container), Some(normals)) = (self.access_container(), &self.normals) else {
            return;
        };
        container.with_instance_mut(|instance| {
            if let Some(material) = instance.material_mut(0) {
                material.flags |= 8;
            }
            let mut rotation = instance.transform().rotation;
            if flip {
                rotation = rotation.map(|row| row.map(|v| -v));
            }
            for (i, normals) in normals.iter().enumerate() {
                if !self
                    .originals
                    .get(i)
                    .and_then(|b| b.as_ref())
                    .is_some_and(|b| b.read().format == 2)
                {
                    continue;
                }
                let Some(block) = instance.mesh(i).and_then(|m| m.vertices.as_ref()) else {
                    continue;
                };
                let mut block = block.write();
                let mut colours = block.vertices.iter().map(|v| v.colour).collect::<Vec<_>>();
                lighting.get_vertex_colors(normals, rotation, &mut colours);
                for (v, colour) in block.vertices.iter_mut().zip(colours) {
                    v.colour = colour;
                }
            }
        });
    }
    pub fn sort_meshes(&mut self, direction: Vec3) {
        let Some(container) = self.access_container() else {
            return;
        };
        let depths = container.with_instance(|model| {
            model
                .meshes()
                .iter()
                .map(|mesh| {
                    let b = mesh.bounds();
                    let center: [f64; 3] = std::array::from_fn(|i| (f64::from(b.min[i]) + f64::from(b.max[i])) * 0.5);
                    ((center[0] * f64::from(direction[0]) + center[1] * f64::from(direction[1]))
                        + center[2] * f64::from(direction[2])) as f32
                })
                .collect::<Vec<_>>()
        });
        // MI8: every invocation starts with the container's original mesh order.
        self.mesh_order.clear();
        // Scan from the front: a NaN can make backward insertion differ.
        for value in 0..depths.len() {
            let position = self
                .mesh_order
                .iter()
                .position(|&old| depths[value].partial_cmp(&depths[old]) != Some(std::cmp::Ordering::Less))
                .unwrap_or(self.mesh_order.len());
            self.mesh_order.insert(position, value);
        }
    }
    pub fn release_model(&mut self) -> Result<(), ReleaseModelError> {
        // MI6 [choice]: reject undefined native no-container/repeated releases.
        // Keep a weak borrowed pointer and arrays for later destruction/rebind;
        // the caller must independently keep the container alive.
        if self.already_released || self.container.is_none() {
            return Err(ReleaseModelError);
        }
        self.borrowed_container = self.container.as_ref().map(InstanceReference::downgrade);
        self.instance = None;
        self.container = None;
        self.already_released = true;
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReleaseModelError;
impl Drop for ModelProcessor {
    fn drop(&mut self) {
        self.instance = None;
        self.restore_originals();
        self.container = None;
        self.originals.clear();
        self.normals = None;
        self.mesh_order.clear();
    }
}
