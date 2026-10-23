// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

/// PathInfo is an owned record, not an independently exposed COM object (§3.3).
#[derive(Debug)]
pub struct PathInfo {
    pub colour: u32,
    pub priority: f32,
    pub flags: u32,
    pub points: Vec<Vec3>,
    pub texture: Option<Handle>,
    /// NewPath's repeat is unwritten. Only SetPathTexture establishes a value.
    pub repeat: Option<f32>,
    id: u64,
}
impl PathInfo {
    fn cleanup(&mut self, resources: &mut impl Resources) {
        if let Some(texture) = self.texture.take() {
            resources.release(texture);
        }
        // Binding release precedes destruction of the point deque.
        self.points = Vec::new();
    }
}

#[derive(Clone, Copy, Debug)]
enum Selected {
    Sentinel,
    Path(u64),
    Stale,
}
pub struct PathView {
    pub thickness: f32,
    pub initialized: bool,
    pub plain_state: Option<RenderState>,
    pub textured_state: Option<RenderState>,
    paths: Vec<PathInfo>,
    selected: Selected,
    next_id: u64,
}
impl Default for PathView {
    fn default() -> Self {
        Self {
            thickness: 2.0,
            initialized: false,
            plain_state: None,
            textured_state: None,
            paths: Vec::new(),
            selected: Selected::Sentinel,
            next_id: 0,
        }
    }
}
impl PathView {
    pub fn paths(&self) -> &[PathInfo] {
        &self.paths
    }
    pub fn init(&mut self, prefer_opaque: bool, resources: &mut impl Resources) -> bool {
        if !self.initialized {
            self.initialized = true;
            if resources.render_service_available() {
                let blend = if prefer_opaque { Blend::Off } else { Blend::SourceAlpha };
                self.plain_state = Some(overlay_state(DepthTest::Off, 0, blend));
                self.textured_state = Some(overlay_state(DepthTest::Off, 1, blend));
            }
        }
        true
    }
    pub fn shutdown(&mut self) {
        self.initialized = false;
    }
    pub fn set_thickness(&mut self, thickness: f32) {
        self.thickness = thickness;
    }
    pub fn new_path(&mut self, colour: u32, priority: f32, flags: u32) {
        let index = self
            .paths
            .iter()
            .position(|old| old.priority >= priority)
            .unwrap_or(self.paths.len());
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.paths.insert(
            index,
            PathInfo {
                colour,
                priority,
                flags,
                points: Vec::new(),
                texture: None,
                repeat: None,
                id,
            },
        );
        self.selected = Selected::Path(id);
    }
    fn selected_mut(&mut self) -> Result<Option<&mut PathInfo>> {
        match self.selected {
            Selected::Sentinel => Ok(None),
            Selected::Stale => Err(Boundary::Undefined("selected path was cleared")),
            Selected::Path(id) => Ok(self.paths.iter_mut().find(|p| p.id == id)),
        }
    }
    pub fn add_path_vertex(&mut self, point: Vec3) -> Result<()> {
        if let Some(path) = self.selected_mut()? {
            if let Some(previous) = path.points.last() {
                let dx = f64::from(point.x) - f64::from(previous.x);
                let dz = f64::from(point.z) - f64::from(previous.z);
                if dx * dx + dz * dz < 1e-5_f64 {
                    return Ok(());
                }
            }
            path.points.push(point);
        }
        Ok(())
    }
    pub fn set_path_texture(&mut self, key: ResourceKey, repeat: f32, resources: &mut impl Resources) -> Result<()> {
        let path = self
            .selected_mut()?
            .ok_or(Boundary::Undefined("no selected path texture"))?;
        if let Some(old) = path.texture.take() {
            resources.release(old);
        }
        path.texture = resources.binding(key);
        path.repeat = Some(repeat);
        Ok(())
    }
    pub fn clear_paths(&mut self, resources: &mut impl Resources) {
        for path in &mut self.paths {
            path.cleanup(resources);
        }
        if !self.paths.is_empty() {
            self.selected = Selected::Stale;
        }
        self.paths.clear();
    }
    pub fn update(&self, renderer: &mut impl Renderer) {
        if self.paths.is_empty() {
            return;
        }
        let offset = renderer.depth_offset();
        renderer.emit(Command::DepthOffset(15));
        renderer.emit(Command::Identity);
        for path in &self.paths {
            if path.points.len() < 2 {
                continue;
            }
            let state = if path.texture.is_some() {
                self.textured_state
            } else {
                self.plain_state
            };
            if let Some(state) = state {
                renderer.emit(Command::State(state));
            }
            if let Some(texture) = path.texture {
                renderer.emit(Command::Texture(texture));
                renderer.emit(Command::RepeatWrap);
            }
            for draw in draw_path(path, self.thickness) {
                renderer.emit(Command::Draw(draw));
            }
            if path.texture.is_some() {
                if let Some(state) = self.plain_state {
                    renderer.emit(Command::State(state));
                }
            }
        }
        renderer.emit(Command::DepthOffset(offset));
    }
}
impl ViewOwner for PathView {
    fn deleting_cleanup(&mut self, resources: &mut impl Resources) {
        self.clear_paths(resources);
        self.paths = Vec::new();
    }
}

pub fn horizontal_normal(a: Vec3, b: Vec3) -> Vec3 {
    let d = b.minus(a);
    Vec3::new(d.z, 0.0, -d.x).times(1.0 / d.horizontal_length())
}
pub fn end_offsets(point: Vec3, neighbour: Vec3, thickness: f32) -> [Vec3; 2] {
    let offset = horizontal_normal(point, neighbour).times(thickness);
    [point.plus(offset), point.minus(offset)]
}
pub fn joint_offsets(previous: Vec3, point: Vec3, next: Vec3, thickness: f32) -> [Vec3; 2] {
    let outgoing = horizontal_normal(point, next);
    let m = horizontal_normal(previous, point).plus(outgoing).times(0.5);
    let q = f64::from(m.x).powi(2) + f64::from(m.z).powi(2);
    let offset = if q < 0.0001_f64 {
        outgoing
    } else {
        m.times((f64::from(thickness) / q) as f32)
    };
    [point.plus(offset), point.minus(offset)]
}
fn vertex(position: Vec3, colour: u32, uv: Option<[f32; 2]>) -> Vertex {
    Vertex {
        position,
        colour: Some(Colour::packed(colour)),
        uv,
    }
}
pub fn draw_arrowhead(tip: &mut Vec3, neighbour: Vec3, thickness: f32, colour: u32) -> Option<Draw> {
    let delta = neighbour.minus(*tip);
    let length = delta.length();
    let a = (length - thickness).min((4.5_f64 * f64::from(thickness)) as f32);
    if a < thickness {
        return None;
    }
    let base = tip.plus(delta.times(a / length));
    let wing = horizontal_normal(*tip, neighbour).times(2.0 * thickness);
    let vertices = [*tip, base.minus(wing), base.plus(wing)]
        .into_iter()
        .map(|p| vertex(p, colour, Some([0.0, 0.0])))
        .collect();
    *tip = base;
    Some(Draw {
        primitive: 0,
        format: 10,
        vertices,
        indices: Vec::new(),
    })
}
pub fn draw_path(path: &PathInfo, thickness: f32) -> Vec<Draw> {
    let mut draws = Vec::new();
    let mut points = path.points.clone();
    if points.len() < 2 {
        return draws;
    }
    let last = points.len() - 1;
    if path.flags & 1 != 0 {
        let neighbour = points[1];
        if let Some(arrow) = draw_arrowhead(&mut points[0], neighbour, thickness, path.colour) {
            draws.push(arrow);
        }
    }
    let mut strip = Vec::with_capacity(points.len() * 2);
    let mut s = 0.0;
    let mut last_projection = None;
    for i in 0..last {
        let a = points[i];
        let b = points[i + 1];
        let projection = path.repeat.map(|r| {
            let k = r / (2.0 * thickness);
            let segment = b.minus(a);
            let length = segment.length();
            let d = segment.times(k / length);
            let intercept = s - d.dot(a);
            s += length * k;
            (d, intercept)
        });
        let pair = if i == 0 {
            end_offsets(a, b, thickness)
        } else {
            joint_offsets(points[i - 1], a, b, thickness)
        };
        append_pair(&mut strip, pair, projection, path.colour);
        last_projection = projection;
    }
    // Projection above uses the final segment BEFORE shortening the end arrow.
    if path.flags & 2 != 0 {
        let neighbour = points[last - 1];
        if let Some(arrow) = draw_arrowhead(&mut points[last], neighbour, thickness, path.colour) {
            draws.push(arrow);
        }
    }
    // End pair must retain the forward segment normal.
    let offset = horizontal_normal(points[last - 1], points[last]).times(thickness);
    let pair = [points[last].plus(offset), points[last].minus(offset)];
    append_pair(&mut strip, pair, last_projection, path.colour);
    draws.push(Draw {
        // view-utilities §7.1: primitive 1 is a triangle strip (4 is lines).
        primitive: 1,
        format: 10,
        vertices: strip,
        indices: Vec::new(),
    });
    draws
}
fn append_pair(vertices: &mut Vec<Vertex>, pair: [Vec3; 2], projection: Option<(Vec3, f32)>, colour: u32) {
    for (side, position) in pair.into_iter().enumerate() {
        let uv = projection.map(|(d, b)| [d.dot(position) + b, side as f32]);
        vertices.push(vertex(position, colour, uv));
    }
}
