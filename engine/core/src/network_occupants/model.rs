//! Network model member bodies, network-occupants.md §§10.1–10.3 (NO21–26).
//! These are ordinary members, not virtual overrides. Resources are external;
//! Bind, Deform, winding, Clip, draw ordering and cache ownership live here.
//! Processor lighting is a foreign primitive (NO28); the owner dispatches it
//! after winding changes and before releasing the processor's source arrays.

use super::record::Vertex;
use std::sync::Arc;

mod prebuilt;
pub use prebuilt::*;

pub const GET_MODEL_INSTANCES_SLOT: u32 = 33;
pub const CELL_SIZE: f32 = 16.0;
pub type CornerHeights = [f32; 4];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelPass {
    Bind,
    Deform,
    FlipAndLight,
    Clip,
}

pub fn flip_bit(orientation_byte: u8) -> bool {
    orientation_byte & 0x80 != 0
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Bounds {
    pub const EMPTY: Self = Self {
        min: [1.0; 3],
        max: [0.0; 3],
    };

    fn include(&mut self, p: [f32; 3]) {
        if self.max[0] < self.min[0] {
            self.min = p;
            self.max = p;
        } else {
            for (axis, value) in p.into_iter().enumerate() {
                if value < self.min[axis] {
                    self.min[axis] = value;
                }
                if value > self.max[axis] {
                    self.max[axis] = value;
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrimitiveRange {
    pub kind: u32,
    pub start: usize,
    pub count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    pub format: u32,
    /// Shared before Bind; Arc::make_mut gives the processor its private copy.
    pub vertices: Arc<Vec<Vertex>>,
    pub indices: Option<Vec<u16>>,
    pub ranges: Vec<PrimitiveRange>,
}

/// NO27 requests the numeric value at 0x00B0E37C. Require it explicitly;
/// never silently assume a colour or accept an original Bind result.
#[derive(Clone, Copy, Debug)]
pub struct ProcessorConstants {
    pub bind_colour: u32,
    /// PackStandardVertexFormat(2), whose numeric encoding is unpublished.
    pub standard_source_format: u32,
    pub processor_format: u32,
}

/// Two planes in the centered ±8 frame. Heights here are the quad's actual
/// heights; from_terrain applies the lane's required height + 0.2 first.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Deformation {
    pub planes: [[f32; 3]; 2],
    pub origin: [f32; 2],
    pub rotation: u8,
    pub flip: bool,
    pub diagonal: bool,
}

impl Deformation {
    pub fn from_corners(c: CornerHeights, piece_y: f32, diagonal: bool) -> Self {
        let [y0, y1, y2, y3] = c.map(f64::from);
        let centre = if diagonal { y1 + y3 } else { y0 + y2 };
        let k = (centre * 0.5 - f64::from(piece_y)) as f32;
        let slope = |a: f64, b: f64| ((a - b) * 0.0625) as f32;
        let planes = if diagonal {
            [[slope(y3, y0), slope(y1, y0), k], [slope(y2, y1), slope(y2, y3), k]]
        } else {
            [[slope(y2, y1), slope(y1, y0), k], [slope(y3, y0), slope(y2, y3), k]]
        };
        Self {
            planes,
            origin: [0.0; 2],
            rotation: 0,
            flip: false,
            diagonal,
        }
    }

    pub fn from_terrain(c: CornerHeights, piece_y: f32, diagonal: bool) -> Self {
        Self::from_corners(c.map(|h| h + 0.2), piece_y, diagonal)
    }

    fn height(&self, p: [f32; 3]) -> f32 {
        let (x, z) = (f64::from(p[0]), f64::from(p[2]));
        let (mut x, mut z) = match self.rotation {
            1 => (-z, x),
            2 => (-x, -z),
            3 => (z, -x),
            _ => (x, z),
        };
        if self.flip {
            x = -x;
        }
        x -= f64::from(self.origin[0]);
        z -= f64::from(self.origin[1]);
        let side = if self.diagonal { x + z } else { x - z };
        // Unordered takes P1, as does the diagonal itself.
        let plane = self.planes[usize::from(side.partial_cmp(&0.0) != Some(std::cmp::Ordering::Less))];
        (((z * f64::from(plane[1]) + x * f64::from(plane[0])) + f64::from(plane[2])) + f64::from(p[1])) as f32
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelTransform {
    pub rotation: [f32; 9],
    pub translation: [f32; 3],
    pub mask: u16,
}

impl ModelTransform {
    pub fn new(yaw: f32, translation: [f32; 3], flip: bool) -> Self {
        let (s, c) = yaw.sin_cos();
        let mut rotation = [c, 0.0, -s, 0.0, 1.0, 0.0, s, 0.0, c];
        if flip {
            for value in &mut rotation[..3] {
                *value = -*value;
            }
        }
        Self {
            rotation,
            translation,
            mask: 0x500,
        }
    }
}

/// Arguments to the external processor, not a guessed lighting result (NO28).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightingRequest {
    pub transform: ModelTransform,
    pub flip: bool,
}

/// `cS3DModelProcessor::LightModel`, a different owner from the network
/// instance. The processor generates normals when absent and lights the
/// private mesh through this transform. There is deliberately no no-op
/// default: a caller must supply this foreign service.
pub trait ProcessorLighting {
    fn light_model(&self, meshes: &mut [Mesh], request: LightingRequest);
}

pub struct NetworkModelInstance {
    pub key: ResourceKey,
    /// Retains the SetModel model reference until instance destruction.
    pub resource: Arc<dyn ModelResource>,
    pub meshes: Vec<Mesh>,
    pub bounds: Bounds,
    pub lod: u16,
    pub transform: ModelTransform,
    pub alpha: Option<f32>,
    pub draw_flags: u32,
    pub passes: Vec<ModelPass>,
    /// Bind retains the source arrays until FlipAndLight's ReleaseModel.
    source_arrays: Vec<Arc<Vec<Vertex>>>,
    pub lighting: Option<LightingRequest>,
}

impl NetworkModelInstance {
    /// SetModel: model resource fills the LOD-0 mesh vector and bounds.
    pub fn create(key: ResourceKey, resource: Arc<dyn ModelResource>) -> Option<Self> {
        let meshes = resource.meshes(0)?;
        let bounds = resource.bounds();
        Some(Self {
            key,
            resource,
            meshes,
            bounds,
            lod: 0,
            transform: ModelTransform::new(0.0, [0.0; 3], false),
            alpha: None,
            draw_flags: 0,
            passes: Vec::new(),
            source_arrays: Vec::new(),
            lighting: None,
        })
    }

    /// Bind copies x/y/z/u/v, fills colour, and keeps the original arrays.
    /// Incompatible source format drops the processor binding, not the model.
    pub fn bind(&mut self, constants: ProcessorConstants) -> bool {
        self.passes.push(ModelPass::Bind);
        self.source_arrays.clear();
        if self.meshes.iter().any(|m| m.format != constants.standard_source_format) {
            return false;
        }
        for mesh in &mut self.meshes {
            self.source_arrays.push(Arc::clone(&mesh.vertices));
            for vertex in Arc::make_mut(&mut mesh.vertices) {
                vertex.color = constants.bind_colour;
            }
            mesh.format = constants.processor_format;
        }
        true
    }

    /// Edits the current private mesh. Freshness belongs to GetModelInstances,
    /// which constructs a separate instance on every query.
    pub fn deform(&mut self, deformation: Deformation) {
        self.passes.push(ModelPass::Deform);
        self.bounds = Bounds::EMPTY;
        for mesh in &mut self.meshes {
            for vertex in Arc::make_mut(&mut mesh.vertices) {
                vertex.pos[1] = deformation.height(vertex.pos);
                self.bounds.include(vertex.pos);
            }
        }
    }

    pub fn flip_and_light<P: ProcessorLighting + ?Sized>(&mut self, flip: bool, processor: &P) {
        self.passes.push(ModelPass::FlipAndLight);
        if flip {
            for mesh in &mut self.meshes {
                if let Some(indices) = &mut mesh.indices {
                    let n = indices.len();
                    indices.reverse();
                    for range in &mut mesh.ranges {
                        range.start = n - range.start - range.count;
                    }
                } else if let Some(min) = mesh.ranges.iter().map(|r| r.start).min() {
                    let max = mesh.ranges.iter().map(|r| r.start + r.count).max().unwrap_or(min);
                    mesh.indices = Some((min..max).rev().map(|i| i as u16).collect());
                    for range in &mut mesh.ranges {
                        range.start = max - range.start - range.count;
                    }
                } else {
                    mesh.indices = Some(Vec::new());
                }
            }
        }
        let mut transform = self.transform;
        if flip {
            for value in &mut transform.rotation {
                *value = -*value;
            }
        }
        let request = LightingRequest { transform, flip };
        self.lighting = Some(request);
        processor.light_model(&mut self.meshes, request);
        self.source_arrays.clear(); // ReleaseModel, even without flip.
    }

    pub fn retained_source_arrays(&self) -> usize {
        self.source_arrays.len()
    }

    /// Network mode 1 is specified. Mode 0 is an explicit remainder (NO29).
    pub fn clip(&mut self, clip_box: Option<Bounds>, mode: u8) -> Result<bool, UnspecifiedClipMode> {
        self.passes.push(ModelPass::Clip);
        let Some(clip_box) = clip_box else {
            return Ok(false);
        };
        let axes: &[usize] = if mode == 0 { &[0, 2] } else { &[1] };
        let needs_cut = axes.iter().any(|&axis| {
            let below = self.bounds.min[axis].partial_cmp(&clip_box.min[axis]);
            let above = self.bounds.max[axis].partial_cmp(&clip_box.max[axis]);
            !matches!(below, Some(std::cmp::Ordering::Equal | std::cmp::Ordering::Greater))
                || !matches!(above, Some(std::cmp::Ordering::Equal | std::cmp::Ordering::Less))
        });
        if !needs_cut {
            return Ok(false);
        }
        if mode == 0 {
            return Err(UnspecifiedClipMode);
        }
        for mesh in &mut self.meshes {
            clip_mesh(mesh, clip_box);
        }
        for axis in 0..3 {
            if self.bounds.min[axis] < clip_box.min[axis] {
                self.bounds.min[axis] = clip_box.min[axis];
            }
            if clip_box.max[axis] < self.bounds.max[axis] {
                self.bounds.max[axis] = clip_box.max[axis];
            }
        }
        Ok(true)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnspecifiedClipMode;

fn outcode(p: [f32; 3], b: Bounds) -> u8 {
    let mut code = 0;
    for (axis, low, high) in [(0, 1, 4), (1, 0x10, 0x20), (2, 2, 8)] {
        if p[axis] < b.min[axis] {
            code |= low;
        }
        if p[axis] > b.max[axis] {
            code |= high;
        }
    }
    code
}

fn lerp_vertex(out: Vertex, inside: Vertex, t: f32, colour: bool) -> Vertex {
    // f32 operands loaded into extended arithmetic; round at each store.
    let lerp = |a: f32, b: f32| ((f64::from(b) - f64::from(a)) * f64::from(t) + f64::from(a)) as f32;
    let color = if colour {
        let mut packed = 0;
        for shift in [24, 16, 8, 0] {
            let a = ((out.color >> shift) & 255) as i32;
            let b = ((inside.color >> shift) & 255) as i32;
            let c = (f64::from(b - a) * f64::from(t) + f64::from(a)).trunc() as u32;
            packed |= (c & 255) << shift;
        }
        packed
    } else {
        0
    };
    Vertex {
        pos: std::array::from_fn(|i| lerp(out.pos[i], inside.pos[i])),
        color,
        u: lerp(out.u, inside.u),
        v: lerp(out.v, inside.v),
    }
}

#[derive(Clone, Copy)]
enum PolygonVertex {
    Source(usize),
    Cut(Vertex),
}

fn cut(out: Vertex, inside: Vertex, code: u8, b: Bounds, colour: bool) -> Vertex {
    let mut t = 0.0_f32;
    for (bit, plane) in [(0x10, b.min[1]), (0x20, b.max[1])] {
        if code & bit != 0 {
            let candidate =
                (f64::from(plane) - f64::from(out.pos[1])) / (f64::from(inside.pos[1]) - f64::from(out.pos[1]));
            if candidate >= f64::from(t) {
                t = candidate as f32;
            }
        }
    }
    lerp_vertex(out, inside, t, colour)
}

fn clip_mesh(mesh: &mut Mesh, b: Bounds) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut ranges = Vec::new();
    let mut source_map = vec![None; mesh.vertices.len()];
    let supported = matches!(mesh.format, 2 | 0x8000_4001 | 10);
    for range in &mesh.ranges {
        let start = indices.len();
        if supported {
            for k in (0..range.count.saturating_sub(2)).step_by(3) {
                let ids: [usize; 3] = std::array::from_fn(|i| {
                    let index = range.start + k + i;
                    mesh.indices.as_ref().map_or(index, |v| usize::from(v[index]))
                });
                let v = ids.map(|i| mesh.vertices[i]);
                let codes = v.map(|v| outcode(v.pos, b));
                if codes[0] & codes[1] & codes[2] != 0 {
                    continue;
                }
                let mut polygon = Vec::new();
                if codes.iter().all(|&c| c == 0) {
                    polygon.extend(ids.map(PolygonVertex::Source));
                } else {
                    for i in 0..3 {
                        let j = (i + 1) % 3;
                        let (a, c) = (codes[i] & 0x30, codes[j] & 0x30);
                        let colour = mesh.format == 10;
                        if a == 0 {
                            polygon.push(PolygonVertex::Source(ids[i]));
                            if c != 0 {
                                polygon.push(PolygonVertex::Cut(cut(v[j], v[i], c, b, colour)));
                            }
                        } else if c == 0 {
                            polygon.push(PolygonVertex::Cut(cut(v[i], v[j], a, b, colour)));
                        } else if a & c == 0 {
                            let denom = f64::from(v[j].pos[1]) - f64::from(v[i].pos[1]);
                            let mut t0 = ((f64::from(b.min[1]) - f64::from(v[i].pos[1])) / denom) as f32;
                            let mut t1 = ((f64::from(b.max[1]) - f64::from(v[i].pos[1])) / denom) as f32;
                            if t0 > t1 {
                                std::mem::swap(&mut t0, &mut t1);
                            }
                            polygon.push(PolygonVertex::Cut(lerp_vertex(v[i], v[j], t0, colour)));
                            polygon.push(PolygonVertex::Cut(lerp_vertex(v[i], v[j], t1, colour)));
                        }
                    }
                }
                // Worked vector A shares fan vertices within this polygon;
                // cuts are never shared across different input triangles.
                let mut polygon_map = vec![None; polygon.len()];
                for k in 1..polygon.len().saturating_sub(1) {
                    for p in [0, k, k + 1] {
                        let index = *polygon_map[p].get_or_insert_with(|| match polygon[p] {
                            PolygonVertex::Source(i) => *source_map[i].get_or_insert_with(|| {
                                let index = vertices.len() as u16;
                                vertices.push(mesh.vertices[i]);
                                index
                            }),
                            PolygonVertex::Cut(v) => {
                                let index = vertices.len() as u16;
                                vertices.push(v);
                                index
                            }
                        });
                        indices.push(index);
                    }
                }
            }
        }
        if indices.len() != start {
            ranges.push(PrimitiveRange {
                kind: range.kind,
                start,
                count: indices.len() - start,
            });
        }
    }
    mesh.vertices = Arc::new(vertices);
    mesh.indices = Some(indices);
    mesh.ranges = ranges;
}
