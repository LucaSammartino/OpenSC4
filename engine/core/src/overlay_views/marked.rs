// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cell(pub i32, pub i32);
#[derive(Clone, Copy, Debug)]
pub struct Lot {
    pub occupant_reference: Handle,
    pub lot_reference: Handle,
    pub flags: u32,
    pub show_marks: bool,
}
pub trait CellTerrain {
    fn dimensions(&self) -> (u32, u32);
    fn terrain_height(&mut self, x: i32, z: i32) -> Option<f32>;
    fn water_height(&mut self, x: i32, z: i32) -> f32;
    fn split(&mut self, cell: Cell) -> bool;
    fn in_frustum(&mut self, point: Vec3) -> bool;
    /// City slot 32 + QI 0xA821EF94. Both returned references are temporary/owned.
    fn lookup_lot(&mut self, cell: Cell, filter: Handle) -> Option<Lot>;
    fn lot_altitude(&mut self, lot: Handle, x: f32, z: f32) -> f32;
}

pub struct MarkedCellView {
    pub initialized: bool,
    pub ignore_depth: bool,
    pub use_water: bool,
    pub use_lots: bool,
    pub size: [f32; 2],
    pub inverse: [f32; 2],
    pub colour: u32,
    pub outline: Option<u32>,
    pub colours: Vec<u32>,
    pub cells: Vec<Cell>,
    pub state: Option<RenderState>,
    pub filter: Option<Handle>,
}
impl Default for MarkedCellView {
    fn default() -> Self {
        Self {
            initialized: false,
            ignore_depth: true,
            use_water: false,
            use_lots: false,
            size: [16.0; 2],
            inverse: [1.0 / 16.0; 2],
            colour: 0x80C0_00C0,
            outline: None,
            colours: Vec::new(),
            cells: Vec::new(),
            state: None,
            filter: None,
        }
    }
}
/// §2.2 magic-add conversion, round-to-nearest-even binary32 store, no clamp.
pub fn pack_cell_colour(rgba: [f32; 4]) -> u32 {
    let bytes = rgba.map(|c| {
        let sum = (f64::from(c) + 49152.0) as f32;
        let u = sum.to_bits().wrapping_sub(0x4740_0000);
        (u as u8).wrapping_sub((u >> 8) as u8)
    });
    u32::from_le_bytes([bytes[2], bytes[1], bytes[0], bytes[3]])
}
pub fn lower_terrain_vertex(coordinate: f32) -> i32 {
    let rounded = (f64::from(coordinate) - 0.5 + 12582912.0) as f32;
    let candidate = rounded.to_bits().wrapping_sub(0x4B40_0000) as i32;
    if f64::from(coordinate) == f64::from(candidate) + 1.0 {
        candidate.wrapping_add(1)
    } else {
        candidate
    }
}
impl MarkedCellView {
    pub fn set_should_ignore_depth_buffer(&mut self, ignore: bool) {
        self.ignore_depth = ignore;
    }
    pub fn set_should_use_water_altitude(&mut self, water: bool) {
        self.use_water = water;
    }
    pub fn set_use_lots(&mut self, lots: bool) {
        self.use_lots = lots;
    }
    pub fn set_cell_size(&mut self, width: f32, height: f32) {
        self.size = [width, height];
        self.inverse = [1.0 / width, 1.0 / height];
    }
    pub fn set_cell_colour(&mut self, rgba: [f32; 4]) {
        self.colour = pack_cell_colour(rgba);
        self.colours.clear();
    }
    pub fn set_outline_colour(&mut self, rgba: [f32; 4]) {
        self.outline = Some(pack_cell_colour(rgba));
    }
    /// Deep assignment returns the destination vector; source storage stays owned by its caller.
    pub fn copy_colours(&mut self, colours: &[u32]) -> &mut Vec<u32> {
        self.colours.clear();
        self.colours.extend_from_slice(colours);
        &mut self.colours
    }
    pub fn append_cell(&mut self, cell: Cell) {
        self.cells.push(cell);
    }
    pub fn clear_cells(&mut self) {
        self.cells.clear();
    }
    pub fn add_region_cells(&mut self, min: Cell, max: Cell, mut mask: impl FnMut(Cell) -> bool) {
        for x in min.0..=max.0 {
            for z in min.1..=max.1 {
                let cell = Cell(x, z);
                if mask(cell) {
                    self.cells.push(cell);
                }
            }
        }
    }
    pub fn init(&mut self, resources: &mut impl Resources) -> bool {
        if !self.initialized {
            self.initialized = true;
            if resources.render_service_available() {
                let depth = if self.ignore_depth {
                    DepthTest::Off
                } else {
                    DepthTest::LessEqual
                };
                self.state = Some(overlay_state(depth, 0, Blend::SourceAlpha));
            }
            self.filter = resources.lot_filter(0x1F4F);
        }
        true
    }
    pub fn shutdown(&mut self, resources: &mut impl Resources) {
        if let Some(filter) = self.filter.take() {
            resources.release(filter);
        }
        self.initialized = false;
    }
    pub fn draw_cells(
        &self,
        override_colour: Option<u32>,
        terrain: &mut impl CellTerrain,
        resources: &mut impl Resources,
        renderer: &mut impl Renderer,
    ) -> Result<()> {
        let (width, height) = terrain.dimensions();
        let exact = self.size == [16.0; 2];
        let mut batch = Draw {
            primitive: 0,
            format: 1,
            vertices: Vec::new(),
            indices: Vec::new(),
        };
        for (cell_index, &cell) in self.cells.iter().enumerate() {
            let colour = match override_colour {
                Some(c) => c,
                None if self.colours.is_empty() => self.colour,
                None => *self
                    .colours
                    .get(cell_index)
                    .ok_or(Boundary::Undefined("short per-cell colour vector"))?,
            };
            if exact && (cell.0 as u32 >= width || cell.1 as u32 >= height) {
                continue;
            }
            let mut corners = [Vec3::default(); 4];
            for (i, (dx, dz)) in [(0, 0), (0, 1), (1, 1), (1, 0)].into_iter().enumerate() {
                let gx = i64::from(cell.0) + dx;
                let gz = i64::from(cell.1) + dz;
                let (x, z) = if exact {
                    (gx as f32 * 16.0, gz as f32 * 16.0)
                } else {
                    (
                        gx.clamp(0, i64::from(width)) as f32 * self.size[0],
                        gz.clamp(0, i64::from(height)) as f32 * self.size[1],
                    )
                };
                let (ix, iz) = if exact {
                    (gx as i32, gz as i32)
                } else {
                    (lower_terrain_vertex(x / 16.0), lower_terrain_vertex(z / 16.0))
                };
                let mut y = terrain
                    .terrain_height(ix, iz)
                    .ok_or(Boundary::Undefined("unsafe terrain index"))?;
                if self.use_water {
                    let water = terrain.water_height(ix, iz);
                    if water > y {
                        y = water;
                    }
                }
                corners[i] = Vec3::new(x, y, z);
            }
            if !terrain.in_frustum(corners[0]) && !terrain.in_frustum(corners[2]) {
                continue;
            }
            if exact && self.use_lots {
                let filter = self.filter.ok_or(Boundary::Undefined("missing lot filter"))?;
                if let Some(lot) = terrain.lookup_lot(cell, filter) {
                    let hidden = lot.flags & 0x10_0000 != 0 && !lot.show_marks;
                    if !hidden {
                        for corner in &mut corners {
                            corner.y = terrain.lot_altitude(lot.lot_reference, corner.x, corner.z)
                                + if lot.flags & 0x304 != 0 { 15.5 } else { 0.0 };
                        }
                    }
                    resources.release(lot.lot_reference);
                    resources.release(lot.occupant_reference);
                    if hidden {
                        continue;
                    }
                }
            }
            let base = batch.vertices.len() as u16;
            let indices = if exact && terrain.split(cell) {
                [0, 1, 3, 3, 1, 2]
            } else {
                [0, 1, 2, 0, 2, 3]
            };
            batch.indices.extend(indices.map(|i| base + i));
            batch.vertices.extend(corners.map(|position| Vertex {
                position,
                colour: Some(Colour::packed(colour)),
                uv: None,
            }));
            if batch.vertices.len() == 1024 {
                renderer.emit(Command::Draw(batch));
                batch = Draw {
                    primitive: 0,
                    format: 1,
                    vertices: Vec::new(),
                    indices: Vec::new(),
                };
            }
        }
        if !batch.vertices.is_empty() {
            renderer.emit(Command::Draw(batch));
        }
        Ok(())
    }
    pub fn update(
        &self,
        prefer_opaque: bool,
        terrain: &mut impl CellTerrain,
        resources: &mut impl Resources,
        renderer: &mut impl Renderer,
    ) -> Result<()> {
        if self.cells.is_empty() {
            return Ok(());
        }
        let offset = renderer.depth_offset();
        renderer.emit(Command::DepthOffset(4));
        renderer.emit(Command::Identity);
        if let Some(state) = self.state {
            renderer.emit(Command::State(state));
        }
        let result = self.draw_cells(None, terrain, resources, renderer).and_then(|()| {
            if let Some(colour) = self.outline.filter(|_| !prefer_opaque) {
                renderer.emit(Command::State(overlay_state(DepthTest::Greater, 0, Blend::Additive)));
                self.draw_cells(Some(colour), terrain, resources, renderer)?;
            }
            Ok(())
        });
        renderer.emit(Command::DepthOffset(offset));
        result
    }
}
impl ViewOwner for MarkedCellView {
    fn deleting_cleanup(&mut self, resources: &mut impl Resources) {
        if let Some(filter) = self.filter.take() {
            resources.release(filter);
        }
        self.colours = Vec::new();
        self.cells = Vec::new();
    }
}
