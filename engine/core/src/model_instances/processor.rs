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
        self.container.is_some()
    }
    pub fn normals(&self) -> Option<&[Vec<Vec3>]> {
        self.normals.as_deref()
    }
    pub fn mesh_order(&self) -> &[usize] {
        &self.mesh_order
    }
    fn restore_originals(&mut self) {
        if let Some(container) = &self.container {
            container.with_instance_mut(|instance| {
                for (i, original) in self.originals.drain(..).enumerate() {
                    if let Some(mesh) = instance.mesh_mut(i) {
                        mesh.vertices = original;
                    }
                }
            });
        }
        self.originals.clear();
    }
    pub fn set_model(&mut self, instance: Option<&InstanceReference>) -> bool {
        self.restore_originals();
        self.release_model();
        let Some(instance) = instance else {
            return true;
        };
        if instance.animated_frame_count().is_some_and(|n| n > 1) {
            return false;
        }
        self.instance = Some(instance.clone());
        let valid = instance.with_instance(|model| {
            model
                .meshes()
                .iter()
                .all(|mesh| mesh.vertices.as_ref().is_some_and(|v| v.read().format == 2))
        });
        // MI6: only multi-frame animation is explicitly refused. Other formats
        // keep the instance but no container; true means the instance was kept.
        if !valid {
            return true;
        }
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
        let Some(container) = &self.container else {
            return;
        };
        self.normals = Some(container.with_instance(|instance| {
            instance
                .meshes()
                .iter()
                .map(|mesh| {
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
        let (Some(container), Some(normals)) = (&self.container, &self.normals) else {
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
        let Some(container) = &self.container else {
            return;
        };
        let depths = container.with_instance(|model| {
            model
                .meshes()
                .iter()
                .map(|mesh| dot(mesh.bounds().center(), direction))
                .collect::<Vec<_>>()
        });
        // MI8: every invocation starts with the container's original mesh order.
        self.mesh_order = (0..depths.len()).collect();
        // Literal stable insertion sort, including unordered (NaN) comparisons.
        for i in 1..self.mesh_order.len() {
            let value = self.mesh_order[i];
            let mut j = i;
            while j > 0 && depths[value] > depths[self.mesh_order[j - 1]] {
                self.mesh_order[j] = self.mesh_order[j - 1];
                j -= 1;
            }
            self.mesh_order[j] = value;
        }
    }
    pub fn release_model(&mut self) {
        // The published ReleaseModel releases references, without promising to
        // restore vertices. SetModel does that explicitly before calling here.
        self.container = None;
        self.instance = None;
        self.originals.clear();
        self.normals = None;
        self.mesh_order.clear();
    }
}
impl Drop for ModelProcessor {
    fn drop(&mut self) {
        self.release_model();
    }
}
