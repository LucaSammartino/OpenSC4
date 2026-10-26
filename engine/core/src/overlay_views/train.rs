// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

/// Explicit process scope: share one instance between all four kinds of views.
#[derive(Default)]
pub struct OverlayProcess {
    pub pulse: Pulse,
    pub vertex_buffer: VertexBufferExtension,
    pub directions: Option<[[Vec3; 4]; 4]>,
}
pub fn edge_directions() -> [[Vec3; 4]; 4] {
    let edges = [
        Vec3::new(0.0, 0.0, 8.0),
        Vec3::new(8.0, 0.0, 0.0),
        Vec3::new(16.0, 0.0, 8.0),
        Vec3::new(8.0, 0.0, 16.0),
    ];
    std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            if i == j {
                Vec3::default()
            } else {
                let d = edges[j].minus(edges[i]);
                d.times(1.0 / d.length())
            }
        })
    })
}
pub fn arrow_key(instance: u32) -> ResourceKey {
    ResourceKey {
        kind: 0x7AB5_0E44,
        group: 0x1ABE_787D,
        instance,
    }
}
pub fn marker_key(instance: u32) -> ResourceKey {
    ResourceKey {
        kind: 0x5AD0_E817,
        group: 0xBADB_57F1,
        instance,
    }
}
pub fn route_transparency(near: bool, selected: bool) -> f32 {
    match (near, selected) {
        (true, true) => 0.8,
        (true, false) => 0.7,
        (false, true) => 0.5,
        (false, false) => 0.4,
    }
}
/// Initialized fields of NO 2.2's 0x38-byte model record; padding +2/+3 is unwritten.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarkerState {
    pub kind: [u8; 2],
    pub rotation: [[f32; 3]; 3],
    pub translation: Vec3,
    pub scale: f32,
}
pub fn marker_state(direction: Vec3, translation: Vec3) -> MarkerState {
    let h = (direction.x * direction.x + direction.z * direction.z).sqrt();
    let (x, y) = if h > 0.0 {
        let x = Vec3::new(direction.z / h, 0.0, -direction.x / h);
        // Stored binary32 X feeds Z cross X, rather than a repaired orthonormal basis.
        let y = Vec3::new(
            direction.y * x.z,
            direction.z * x.x - direction.x * x.z,
            -direction.y * x.x,
        );
        (x, y)
    } else {
        (Vec3::new(0.0, 0.0, 1.0), Vec3::new(1.0, 0.0, 0.0))
    };
    MarkerState {
        kind: [2, 5],
        rotation: [
            [x.x, y.x, direction.x],
            [x.y, y.y, direction.y],
            [x.z, y.z, direction.z],
        ],
        translation,
        scale: 1.0,
    }
}
pub struct TrainJunctionView {
    pub initialized: bool,
    pub state: Option<RenderState>,
    /// Hover cell, then pressed/accepted junction (NO 2.2).
    pub cell_pairs: [Cell; 2],
    /// Outer None = constructor's unwritten binding slot; inner None = provider null.
    pub bindings: [Option<Option<Handle>>; 6],
    pub models: [Option<Handle>; 2],
    pub stacking_offset: Option<f32>,
}
impl Default for TrainJunctionView {
    fn default() -> Self {
        Self {
            initialized: false,
            state: None,
            cell_pairs: [Cell(-1, -1); 2],
            bindings: [None; 6],
            models: [None; 2],
            stacking_offset: None,
        }
    }
}
impl TrainJunctionView {
    /// Inline mouse-move result; failed picking clears only the hover pair.
    pub fn set_hover_cell(&mut self, picked: Option<Cell>) {
        let cell = picked.unwrap_or(Cell(-1, -1));
        if self.cell_pairs[0] != cell {
            self.cell_pairs[0] = cell;
        }
    }
    /// Inline store after network picking and acceptance; not a native setter slot.
    pub fn set_pressed_cell(&mut self, cell: Cell) {
        if self.cell_pairs[1] != cell {
            self.cell_pairs[1] = cell;
        }
    }
    pub fn clear_pressed_cell(&mut self) {
        self.cell_pairs[1] = Cell(-1, -1);
    }
    pub fn init(&mut self, process: &mut OverlayProcess, resources: &mut impl Resources) -> Result<bool> {
        if self.initialized {
            return Ok(true);
        }
        self.initialized = true;
        process.directions = Some(edge_directions());
        if resources.render_service_available() {
            self.state = Some(RenderState {
                depth_write: true,
                alpha_test: true,
                tuple: Some([1, 1, 1, 1, 1, 1, 1, 3, 4, 5, 4, 0, 1, -1]),
                ..overlay_state(DepthTest::LessEqual, 1, Blend::SourceAlpha)
            });
        }
        if !resources.resource_service_available() {
            return Err(Boundary::Undefined("unguarded train resource service"));
        }
        for (i, binding) in self.bindings.iter_mut().enumerate() {
            *binding = Some(resources.binding(arrow_key(0x6BE0_9921 + i as u32)));
        }
        for (i, instance) in [0x8BB7_0000, 0x8BB8_0000].into_iter().enumerate() {
            let temporary = resources.model(marker_key(instance));
            if let Some(model) = temporary {
                if self.models[i] != Some(model) {
                    resources.retain(model);
                    if let Some(old) = self.models[i].replace(model) {
                        resources.release(old);
                    }
                }
                resources.release(model);
            }
        }
        Ok(true)
    }
    pub fn shutdown(&mut self, resources: &mut impl Resources) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }
        self.initialized = false;
        for i in [2, 3, 0, 1, 4, 5] {
            let slot = self.bindings[i]
                .take()
                .ok_or(Boundary::Undefined("unwritten train resource output"))?;
            if let Some(binding) = slot {
                resources.release(binding);
            }
            self.bindings[i] = Some(None);
        }
        for model in &mut self.models {
            if let Some(model) = model.take() {
                resources.release(model);
            }
        }
        Ok(())
    }
    pub fn get_path_bitmap(&self, points: &[Vec3], entry_direction: Vec3, selected: bool) -> Result<Option<Handle>> {
        let instance = classify_path_bitmap(points, entry_direction, selected)?;
        self.bindings[(instance - 0x6BE0_9921) as usize].ok_or(Boundary::Undefined("unwritten train bitmap slot"))
    }
    fn direction(process: &OverlayProcess, key: u32) -> Result<Vec3> {
        let entry = ((key >> 8) & 255) as usize;
        let exit = (key & 255) as usize;
        let directions = process
            .directions
            .as_ref()
            .ok_or(Boundary::Undefined("uninitialized direction table"))?;
        directions
            .get(entry)
            .and_then(|row| row.get(exit))
            .copied()
            .ok_or(Boundary::Undefined("edge byte outside direction table"))
    }
    #[allow(clippy::too_many_arguments)]
    pub fn draw_train_junction(
        &mut self,
        cell: Cell,
        occupant: Handle,
        routes: &RouteSet,
        process: &OverlayProcess,
        network: &mut impl NetworkHost,
        resources: &mut impl Resources,
        renderer: &mut impl Renderer,
    ) -> Result<()> {
        let first = *routes
            .keys
            .first()
            .ok_or(Boundary::Undefined("empty junction routes"))?;
        let chosen = (routes.chosen & 0xFFFF) as usize;
        let selected_key = *routes
            .keys
            .get(chosen)
            .ok_or(Boundary::Undefined("chosen route index outside vector"))?;
        let origin = network
            .route_origin(occupant, first)
            .ok_or(Boundary::Undefined("unguarded first route origin"))?;
        let first_direction = Self::direction(process, first)?;
        let matched = self.cell_pairs.contains(&cell);
        renderer.emit(Command::ResetTransparency);
        renderer.emit(Command::Identity);
        if let Some(state) = self.state {
            renderer.emit(Command::State(state));
        }
        renderer.emit(Command::Transparency(route_transparency(matched, false)));
        for (i, &key) in routes.keys.iter().enumerate() {
            if i != chosen {
                self.draw_route(key, false, origin, occupant, process, network, renderer)?;
            }
        }
        renderer.emit(Command::Transparency(route_transparency(matched, true)));
        self.draw_route(selected_key, true, origin, occupant, process, network, renderer)?;
        let model_index = usize::from(self.cell_pairs[1] == cell);
        if let Some(model) = self.models[model_index] {
            let offset = self
                .stacking_offset
                .ok_or(Boundary::Undefined("unwritten stacking offset"))?;
            let translation = Vec3::new(
                origin.x,
                origin.y + offset + 0.2 + if (selected_key >> 24) & 15 == 7 { 3.0 } else { 0.0 },
                origin.z,
            );
            resources.retain(model);
            renderer.emit(Command::ModelState {
                model,
                state: marker_state(first_direction, translation),
            });
            renderer.emit(Command::ModelTransparency {
                model,
                mode: 1,
                value: route_transparency(matched, true),
            });
            renderer.emit(Command::DrawModel { model, flags: 0 });
            resources.release(model);
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    fn draw_route(
        &mut self,
        key: u32,
        selected: bool,
        origin: Vec3,
        occupant: Handle,
        process: &OverlayProcess,
        network: &mut impl NetworkHost,
        renderer: &mut impl Renderer,
    ) -> Result<()> {
        let Some(path) = network.path(occupant, key) else {
            return Ok(());
        };
        let direction = Self::direction(process, key)?;
        let Some(bitmap) = self.get_path_bitmap(&path.points, direction, selected)? else {
            return Ok(());
        };
        let offset = self
            .stacking_offset
            .ok_or(Boundary::Undefined("unwritten stacking offset"))?;
        let quad = arrow_bitmap_quad(origin, direction, key, offset, |x, z| network.terrain_altitude(x, z));
        self.stacking_offset = Some(offset + 0.01_f32);
        renderer.emit(Command::Texture(bitmap));
        renderer.emit(Command::Draw(quad));
        Ok(())
    }
    pub fn update(
        &mut self,
        process: &OverlayProcess,
        network: &mut impl NetworkHost,
        resources: &mut impl Resources,
        renderer: &mut impl Renderer,
    ) -> Result<()> {
        if !network.manager_available() {
            return Ok(());
        }
        self.stacking_offset = Some(0.1);
        let (min, max) = network.visible_cells();
        for x in min.0..=max.0 {
            for z in min.1..=max.1 {
                let cell = Cell(x, z);
                for occupant in network.occupants(cell) {
                    for entry in 0..4 {
                        let routes = network.routes(occupant, entry);
                        if !routes.keys.is_empty() {
                            self.draw_train_junction(cell, occupant, &routes, process, network, resources, renderer)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
impl ViewOwner for TrainJunctionView {
    fn deleting_cleanup(&mut self, resources: &mut impl Resources) {
        // Deletion is NOT Shutdown: bitmap bindings deliberately remain unreleased.
        for i in [1, 0] {
            if let Some(model) = self.models[i].take() {
                resources.release(model);
            }
        }
    }
}
pub fn classify_path_bitmap(points: &[Vec3], entry: Vec3, selected: bool) -> Result<u32> {
    let end = *points.last().ok_or(Boundary::Undefined("empty train path"))?;
    let mut delta = end.minus(points[points.len().saturating_sub(3)]);
    if delta.horizontal_length() < 0.25 {
        delta = end.minus(points[0]);
    }
    let d = delta.times(1.0 / delta.horizontal_length());
    let c = d.x * entry.z - d.z * entry.x;
    let plain = if c < -0.25 {
        0x6BE0_9923
    } else if c <= 0.25 {
        0x6BE0_9925
    } else {
        0x6BE0_9921
    };
    Ok(plain + u32::from(selected))
}
pub fn arrow_bitmap_quad(
    origin: Vec3,
    direction: Vec3,
    key: u32,
    offset: f32,
    mut terrain_height: impl FnMut(f32, f32) -> f32,
) -> Draw {
    let forward = direction.times(12.0);
    let n = Vec3::new(-direction.z, 0.0, direction.x).times(6.0);
    let query = origin.plus(forward);
    let y = terrain_height(query.x, query.z).max(origin.y) + offset + if (key >> 24) & 15 == 7 { 3.0 } else { 0.0 };
    let positions = [
        origin.plus(n),
        origin.plus(n).plus(forward),
        origin.minus(n).plus(forward),
        origin.minus(n),
    ];
    let uv = [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]];
    Draw {
        primitive: 6,
        format: 2,
        indices: Vec::new(),
        vertices: positions
            .into_iter()
            .zip(uv)
            .map(|(mut position, uv)| {
                position.y = y;
                Vertex {
                    position,
                    colour: None,
                    uv: Some(uv),
                }
            })
            .collect(),
    }
}
