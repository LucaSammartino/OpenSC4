// SPDX-License-Identifier: GPL-3.0-or-later
use super::{dot, Vec3};

/// Standard vertex format 1 (graphics-formats §3.5), position then BGRA.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub position: Vec3,
    pub colour: [u8; 4],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Primitive {
    pub primitive_type: u32,
    pub first: usize,
    pub count: usize,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: Vec3,
    pub max: Vec3,
}

/// cS3DMeshBuilder, geometry-utilities §6.1. Rust owns both optional arrays.
/// GU3 tracks the unprovided native lifecycle details; no GZCOM is modelled.
#[derive(Default, Debug)]
pub struct MeshBuilder {
    vertices: Option<Vec<Vertex>>,
    primitives: Option<Vec<Primitive>>,
    light: Vec3,
    normal: Vec3,
    colour: Vec3,
    alpha: f32,
    changed: bool,
    bounds: Option<Bounds>,
}
impl MeshBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn reset(&mut self) {
        if let Some(vertices) = &mut self.vertices {
            vertices.clear();
        }
        if let Some(primitives) = &mut self.primitives {
            primitives.clear();
        }
        // GU3: retain arrays, invalidate the empty bounds and mark changed.
        self.bounds = None;
        self.changed = true;
    }
    pub fn set_light_direction(&mut self, light: Vec3) {
        self.light = light;
    }
    pub fn set_normal(&mut self, normal: Vec3) {
        self.normal = normal;
    }
    pub fn set_colour(&mut self, r: f32, g: f32, b: f32) {
        self.set_colour_alpha(r, g, b, 1.0);
    }
    pub fn set_colour_alpha(&mut self, r: f32, g: f32, b: f32, alpha: f32) {
        self.colour = [r, g, b];
        self.alpha = alpha;
    }
    pub fn begin_primitive(&mut self, primitive_type: u32) {
        let first = self.vertices.get_or_insert_with(Vec::new).len();
        self.primitives.get_or_insert_with(Vec::new).push(Primitive {
            primitive_type,
            first,
            count: 0,
        });
        self.changed = true;
    }
    /// §6.2 closes the last primitive. GU3: no-op when none exists.
    pub fn end_primitive(&mut self) {
        let count = self.vertices().len();
        if let Some(primitive) = self.primitives.as_mut().and_then(|p| p.last_mut()) {
            primitive.count = count - primitive.first;
            self.changed = true;
        }
    }
    pub fn add_vertex(&mut self, position: Vec3) {
        let k = 0.3 + 0.7 * dot(self.light, self.normal);
        let [r, g, b] = self.colour.map(|c| colour_byte(k * c));
        let vertex = Vertex {
            position,
            colour: [b, g, r, colour_byte(self.alpha)],
        };
        // GU3: a standalone AddVertex creates no implicit primitive.
        self.vertices.get_or_insert_with(Vec::new).push(vertex);
        self.changed = true;
    }
    pub fn mesh(&mut self) -> &Self {
        if self.changed {
            self.recompute_bounds();
            self.changed = false;
        }
        self
    }
    pub fn vertices(&self) -> &[Vertex] {
        self.vertices.as_deref().unwrap_or(&[])
    }
    pub fn primitives(&self) -> &[Primitive] {
        self.primitives.as_deref().unwrap_or(&[])
    }
    pub fn bounds(&self) -> Option<Bounds> {
        self.bounds
    }
    pub fn changed(&self) -> bool {
        self.changed
    }
    pub fn arrays_allocated(&self) -> (bool, bool) {
        (self.vertices.is_some(), self.primitives.is_some())
    }
    fn recompute_bounds(&mut self) {
        let mut bounds = None::<Bounds>;
        for vertex in self.vertices() {
            if let Some(bounds) = &mut bounds {
                for i in 0..3 {
                    // GU5: retain earlier extrema on ties and NaNs.
                    if vertex.position[i] < bounds.min[i] {
                        bounds.min[i] = vertex.position[i];
                    }
                    if vertex.position[i] > bounds.max[i] {
                        bounds.max[i] = vertex.position[i];
                    }
                }
            } else {
                bounds = Some(Bounds {
                    min: vertex.position,
                    max: vertex.position,
                });
            }
        }
        self.bounds = bounds;
    }
}

fn colour_byte(value: f32) -> u8 {
    let scaled = f64::from(value * 256.0);
    let floor = scaled.floor();
    let fraction = scaled - floor;
    let rounded = if fraction > 0.5 || (fraction == 0.5 && floor % 2.0 != 0.0) {
        floor + 1.0
    } else {
        floor
    };
    // GU2: integer width/nonfinite behaviour unpublished; x87 i32 indefinite.
    let c = if rounded.is_finite() && (f64::from(i32::MIN)..=f64::from(i32::MAX)).contains(&rounded) {
        rounded as i32
    } else {
        i32::MIN
    };
    c.wrapping_sub(c >> 8) as u8
}

pub fn box_corners(min: Vec3, max: Vec3) -> [Vec3; 8] {
    std::array::from_fn(|corner| {
        std::array::from_fn(|axis| {
            if corner & (1 << axis) != 0 {
                max[axis]
            } else {
                min[axis]
            }
        })
    })
}
pub fn add_box_mesh(builder: &mut MeshBuilder, corners: &[Vec3; 8]) {
    const FACES: [(Vec3, [usize; 4]); 6] = [
        ([0.0, 0.0, -1.0], [2, 3, 1, 0]),
        ([0.0, 0.0, 1.0], [4, 5, 7, 6]),
        ([0.0, -1.0, 0.0], [0, 1, 5, 4]),
        ([1.0, 0.0, 0.0], [1, 3, 7, 5]),
        ([0.0, 1.0, 0.0], [3, 2, 6, 7]),
        ([-1.0, 0.0, 0.0], [2, 0, 4, 6]),
    ];
    builder.begin_primitive(6);
    for (normal, indices) in FACES {
        builder.set_normal(normal);
        for i in indices {
            builder.add_vertex(corners[i]);
        }
    }
    builder.end_primitive();
}
