// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub const WALL_TEXTURE_GROUP: u32 = 0x891B_0E1A;
pub const WALL_SEAM_METRES: f32 = 0.2;
pub const WALL_OUTWARD_OFFSET_RANGE: Vec2 = [0.001, 0.002];

/// Published finite arithmetic subset: binary32 operands, PC=53/64,
/// RC=nearest-even. No claim about the ambient native control word (LM8).
fn stored_add(a: f32, b: f32) -> f32 {
    (f64::from(a) + f64::from(b)) as f32
}

pub(super) fn lattice_index(q: f32, upper: bool) -> Result<i32, ContractError> {
    // This bounded domain makes every operation in the bias technique exact
    // before the final binary32 store under both published ledger precisions.
    if !q.is_finite() || !(0.0..=16_777_216.0).contains(&q) {
        return Err(ContractError::InvalidInput);
    }
    let scaled = f64::from(q) * 0.0625;
    let biased = if upper { scaled - f64::from(f32::from_bits(0x3f66_6666)) } else { scaled };
    let word = ((biased + 12_582_912.0) as f32).to_bits();
    Ok(word.wrapping_sub(0x4b40_0000).wrapping_add(u32::from(upper)) as i32)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WallVertex {
    pub position: Vec3,
    /// In-memory BGRA bytes, not a host-packed colour.
    pub colour: [u8; 4],
    pub uv: Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WallRange {
    pub start: [i32; 2],
    pub end: i32,
    pub inner: [i32; 2],
}

impl WallRange {
    pub fn new(side: Edge, bounds: Bounds) -> Result<Self, ContractError> {
        let z1 = lattice_index(bounds.max[2], true)?;
        let x1 = lattice_index(bounds.max[0], true)?;
        let z0 = lattice_index(bounds.min[2], false)?;
        let x0 = lattice_index(bounds.min[0], false)?;
        Ok(match side {
            Edge::North => Self { start: [x0,z0], end: x1, inner: [x0,z0] },
            Edge::East => Self { start: [x1,z0], end: z1, inner: [x1-1,z0] },
            Edge::South => Self { start: [x0,z1], end: x1, inner: [x0,z1-1] },
            Edge::West => Self { start: [x0,z0], end: z1, inner: [x0,z0] },
        })
    }
}

/// Individual returning game-interface primitives only. Ownership/lifetime of
/// any backing game objects belongs to the adapter, outside LM4.
pub trait WallTerrain {
    fn inner_lot_height(&mut self, cell: [i32;2]) -> Option<f32>;
    fn terrain_height(&mut self, cell: [i32;2]) -> f32;
    fn extent_x(&mut self) -> i32;
    fn extent_z(&mut self) -> i32;
    fn lot_flags(&mut self, cell: [i32;2], argument: u32) -> Option<u32>;
}

pub trait WallLighting {
    fn components(&mut self, position: Vec3, normal: Vec3) -> [i32;3];
}

/// Primitive edge coordinates and terrain sample, not owner-generated geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WallSample {
    pub position: Vec2,
    pub terrain_height: f32,
}

/// A semantic band between two 16 m edge samples. No triangle winding, diagonal
/// or 0.001/0.002 offset assignment is asserted (LM4).
#[derive(Clone, Debug, PartialEq)]
pub struct WallBand {
    pub edge_positions: [Vec2; 2],
    pub terrain_heights: Vec2,
    pub lot_height: f32,
    pub expanded_heights: [Vec2; 2],
    pub vertical_repeats: Vec2,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WallShape {
    pub side: Edge,
    pub bands: Vec<WallBand>,
    pub outward_normal: Vec3,
    pub outward_offset_range: Vec2,
    pub records: Vec<WallVertex>,
    pub bounds: Bounds,
    pub local_bounds: Bounds,
    pub emitting_segments: u8,
    pub geometry_present: bool,
    prefix_records: usize,
}

impl WallShape {
    pub fn from_samples(
        side: Edge,
        lot_height: f32,
        samples: &[WallSample],
        texture_scale: f32,
    ) -> Result<Self, ContractError> {
        if !lot_height.is_finite()
            || !texture_scale.is_finite()
            || samples.len() < 2
            || samples
                .iter()
                .any(|s| !s.terrain_height.is_finite() || !s.position.into_iter().all(f32::is_finite))
        {
            return Err(ContractError::InvalidInput);
        }
        let mut bands = Vec::new();
        let mut result = Self::empty(side);
        result.begin(lot_height, samples[0].position);
        for pair in samples.windows(2) {
            let [a, b] = [pair[0], pair[1]];
            let dx = (b.position[0] - a.position[0]).abs();
            let dz = (b.position[1] - a.position[1]).abs();
            let valid_segment = match side {
                Edge::North | Edge::South => dx == CELL_METRES && dz == 0.0,
                Edge::East | Edge::West => dx == 0.0 && dz == CELL_METRES,
            };
            if !valid_segment {
                return Err(ContractError::InvalidInput);
            }
            // §3.2 eligibility precedes the T split; all-below emits nothing.
            if !(a.terrain_height > lot_height || b.terrain_height > lot_height) {
                continue;
            }
            let heights = [a.terrain_height, b.terrain_height].map(|terrain| {
                [
                    terrain.min(lot_height) - WALL_SEAM_METRES,
                    terrain.max(lot_height) + WALL_SEAM_METRES,
                ]
            });
            let vertical_repeats = heights.map(|[bottom, top]| (top - bottom) * texture_scale);
            if heights
                .into_iter()
                .flatten()
                .chain(vertical_repeats)
                .any(|v| !v.is_finite())
            {
                return Err(ContractError::InvalidInput);
            }
            bands.push(WallBand {
                edge_positions: [a.position, b.position],
                terrain_heights: [a.terrain_height, b.terrain_height],
                lot_height,
                expanded_heights: heights,
                vertical_repeats,
            });
            result.emit(lot_height, a, b, texture_scale)?;
        }
        result.bands = bands;
        result.finish(samples[0]);
        Ok(result)
    }

    pub fn triangles(&self) -> Result<Vec<Vec3>, ContractError> {
        Ok(self.records.iter().map(|v| v.position).collect())
    }

    pub fn empty(side: Edge) -> Self {
        let bounds = Bounds { min: [0.;3], max: [0.;3] };
        Self { side, bands: Vec::new(), outward_normal: side.normal(),
            outward_offset_range: WALL_OUTWARD_OFFSET_RANGE, records: Vec::new(),
            bounds, local_bounds: bounds, emitting_segments: 0, geometry_present: false, prefix_records: 0 }
    }

    fn axis(&self) -> usize { if matches!(self.side, Edge::North | Edge::South) { 0 } else { 2 } }

    fn begin(&mut self, lot: f32, start: Vec2) {
        let axis = self.axis();
        let across = 2-axis;
        self.bounds.min = [start[0],lot,start[1]];
        self.bounds.max = self.bounds.min;
        self.bounds.min[axis] = f32::MAX;
        self.bounds.max[axis] = -f32::MAX;
        self.bounds.min[across] = stored_add(self.bounds.min[across],0.001);
        self.bounds.max[across] = stored_add(self.bounds.max[across],0.002);
        self.emitting_segments = 0;
        self.prefix_records = 0;
        self.geometry_present = false;
    }

    fn emit(&mut self, lot: f32, a: WallSample, b: WallSample, scale: f32) -> Result<(),ContractError> {
        let t = stored_add(lot,-WALL_SEAM_METRES);
        let h0 = a.terrain_height;
        let h1 = b.terrain_height;
        let ap = stored_add(h0,WALL_SEAM_METRES);
        let bp = stored_add(h1,WALL_SEAM_METRES);
        let am = stored_add(h0,-WALL_SEAM_METRES);
        let bm = stored_add(h1,-WALL_SEAM_METRES);
        if ![lot,h0,h1,scale,t,ap,bp,am,bm].into_iter().all(f32::is_finite) {
            return Err(ContractError::InvalidInput);
        }
        // Callers own eligibility. No interpolated crossing or side reversal.
        let (points, count) = if h0 > t && h1 > t {
            ([(a,t),(b,t),(a,ap),(a,ap),(b,t),(b,bp)],6)
        } else if h0 > t {
            ([(a,t),(b,bm),(a,ap),(a,t),(a,t),(a,t)],3)
        } else {
            ([(a,am),(b,t),(b,bp),(a,t),(a,t),(a,t)],3)
        };
        if self.emitting_segments == 255 { return Err(ContractError::InvalidInput); }
        let axis = self.axis();
        // Portable allocation policy: Vec capacity is not a native heap claim.
        let offset = self.written_records();
        self.records.try_reserve((offset+count).saturating_sub(self.records.len())).map_err(|_|ContractError::InvalidInput)?;
        for (i,(sample,y)) in points.into_iter().take(count).enumerate() {
            let position = [sample.position[0],y,sample.position[1]];
            let uv = [(f64::from(position[axis])*f64::from(scale)) as f32,(f64::from(y)*f64::from(scale)) as f32];
            if !uv.into_iter().all(f32::is_finite) { return Err(ContractError::InvalidInput); }
            if offset+i < self.records.len() {
                self.records[offset+i].position = position;
                self.records[offset+i].uv = uv;
            } else { self.records.push(WallVertex { position, uv, colour: [0;4] }); }
        }
        self.prefix_records += count;
        self.emitting_segments += 1;
        self.bounds.min[axis] = self.bounds.min[axis].min(a.position[axis/2]);
        self.bounds.max[axis] = self.bounds.max[axis].max(b.position[axis/2]);
        if h0 > t { self.bounds.max[1] = self.bounds.max[1].max(h0); }
        else { self.bounds.min[1] = self.bounds.min[1].min(h0); }
        if h1 > t { self.bounds.max[1] = self.bounds.max[1].max(h1); }
        else { self.bounds.min[1] = self.bounds.min[1].min(h1); }
        Ok(())
    }

    fn written_records(&self) -> usize { self.prefix_records }

    fn finish(&mut self, start: WallSample) {
        self.geometry_present = self.emitting_segments != 0;
        if self.geometry_present {
            let axis = self.axis();
            self.bounds.min[axis] = stored_add(self.bounds.min[axis],0.001);
            self.bounds.max[axis] = stored_add(self.bounds.max[axis],-0.001);
            self.local_bounds = Bounds { min: [0.;3], max: std::array::from_fn(|i|stored_add(self.bounds.max[i],-self.bounds.min[i])) };
        } else {
            self.bounds.min = [start.position[0], start.terrain_height, start.position[1]];
            self.bounds.max = self.bounds.min.map(|v|stored_add(v,1.));
            self.local_bounds = self.bounds;
        }
    }

    pub fn recolour<L: WallLighting>(&mut self, light: &mut L) {
        let normal = self.side.normal().map(|v|-v);
        for v in &mut self.records {
            let c = light.components(v.position,normal);
            v.colour = [c[2] as u8,c[1] as u8,c[0] as u8,255];
        }
    }

    /// Returns the published bounds-change result, distinct from presence.
    pub fn rebuild<H: WallTerrain>(&mut self, box_: Bounds, scale: f32, host: &mut H) -> Result<bool,ContractError> {
        let range = WallRange::new(self.side,box_)?;
        self.emitting_segments = 0;
        self.prefix_records = 0;
        let Some(lot) = host.inner_lot_height(range.inner) else { return Ok(false); };
        if !lot.is_finite() || !scale.is_finite() { return Err(ContractError::InvalidInput); }
        let mut cell = range.start;
        let varying = self.axis()/2;
        let position = |c: [i32;2]| -> Vec2 { c.map(|v|(v*16) as f32) };
        let first = host.terrain_height(cell);
        if !first.is_finite() { return Err(ContractError::InvalidInput); }
        let previous = self.bounds;
        self.begin(lot,position(cell));
        let mut h0 = first;
        while cell[varying] < range.end {
            let mut next = cell;
            next[varying] += 1;
            let h1 = host.terrain_height(next);
            if !h1.is_finite() { return Err(ContractError::InvalidInput); }
            if h0 > lot || h1 > lot {
                let extent_x = host.extent_x();
                let extent_z = host.extent_z();
                let across = match self.side {
                    Edge::North => [cell[0],cell[1]-1],
                    Edge::West => [cell[0]-1,cell[1]],
                    _ => cell,
                };
                let flat = across[0] >= 0 && across[1] >= 0 && across[0] < extent_x && across[1] < extent_z
                    && host.lot_flags(across,0).is_some_and(across_edge_lot_is_flat);
                if !flat {
                    self.emit(lot,WallSample { position:position(cell), terrain_height:h0 },WallSample { position:position(next), terrain_height:h1 },scale)?;
                }
            }
            h0 = h1;
            cell = next;
        }
        let fallback = if self.emitting_segments == 0 { host.terrain_height(range.start) } else { first };
        if !fallback.is_finite() { return Err(ContractError::InvalidInput); }
        self.finish(WallSample { position:position(range.start), terrain_height:fallback });
        Ok(self.geometry_present && [3,4,5,0,1,2].into_iter().any(|i| {
            let (new,old) = if i<3 {(self.bounds.min[i],previous.min[i])} else {(self.bounds.max[i-3],previous.max[i-3])};
            (f64::from(new)-f64::from(old)).abs() > f64::from(f32::from_bits(0x38d1_b717))
        }))
    }
}

pub fn across_edge_lot_is_flat(flags: u32) -> bool {
    flags & 1 != 0
}

pub struct WallDraw<'a, T> {
    pub shape: &'a WallShape,
    pub texture: &'a T,
    pub state: DrawState,
    /// A rotation change requires recolouring, whose formula is not published.
    /// A native adapter must not treat this semantic plan as a ready triangle list.
    pub recolour_required: bool,
}

pub struct RetainingWallModel<T> {
    shape: WallShape,
    keys: [Option<u32>; 5],
    facing_test: bool,
    texture: Option<T>,
    zoom: Option<Zoom>,
    rotation: Option<Edge>,
    recolour_pending: bool,
}

impl<T: Texture> RetainingWallModel<T> {
    /// Convenience sampled-edge entry; `WallShape::rebuild` owns native ranges
    /// and raw terrain/lot query order.
    pub fn init(
        side: Edge,
        lot_height: f32,
        samples: &[WallSample],
        texture_scale: f32,
        keys: [Option<u32>; 5],
        facing_test: bool,
    ) -> Result<Self, ContractError> {
        Ok(Self {
            shape: WallShape::from_samples(side, lot_height, samples, texture_scale)?,
            keys,
            facing_test,
            texture: None,
            zoom: None,
            rotation: None,
            recolour_pending: false,
        })
    }

    pub fn shape(&self) -> &WallShape {
        &self.shape
    }
    pub fn texture(&self) -> Option<&T> {
        self.texture.as_ref()
    }
    pub fn zoom(&self) -> Option<Zoom> {
        self.zoom
    }
    pub fn rotation(&self) -> Option<Edge> {
        self.rotation
    }
    pub fn recolour_pending(&self) -> bool {
        self.recolour_pending
    }
    pub fn facing_test(&self) -> bool {
        self.facing_test
    }
    pub fn set_facing_test(&mut self, enabled: bool) {
        self.facing_test = enabled;
    }

    pub fn setup_vertices(
        &mut self,
        lot_height: f32,
        samples: &[WallSample],
        texture_scale: f32,
    ) -> Result<(), ContractError> {
        let shape = WallShape::from_samples(self.shape.side, lot_height, samples, texture_scale)?;
        self.shape = shape;
        self.recolour_pending = true;
        Ok(())
    }

    pub fn sort_value(&self) -> u32 {
        self.texture
            .as_ref()
            .map_or(0xC986_2A86, |texture| texture.id() | 0x2A86)
    }

    pub fn load_texture<F: TextureFactory<Binding = T>>(&mut self, zoom: Zoom, factory: &mut F) {
        self.texture = load_with_fallback(
            factory,
            self.keys[zoom.index()].map(|instance| TextureKey::Wall {
                group: WALL_TEXTURE_GROUP,
                instance,
            }),
            TextureKey::Wall {
                group: WALL_TEXTURE_GROUP,
                instance: 0x695C_00C6 + zoom.value(),
            },
        );
        self.zoom = Some(zoom);
    }

    pub fn draw<F: TextureFactory<Binding = T>>(
        &mut self,
        highlighted: bool,
        zoom: Zoom,
        rotation: Edge,
        factory: &mut F,
    ) -> Option<WallDraw<'_, T>> {
        if highlighted {
            return None;
        }
        if self.zoom != Some(zoom) {
            self.load_texture(zoom, factory);
        }
        if self.rotation != Some(rotation) {
            self.rotation = Some(rotation);
            self.recolour_pending = true;
        }
        if self.facing_test && !self.shape.side.wall_faces(rotation) {
            return None;
        }
        // LM5: an unresolved texture supplies no drawable band plan.
        self.texture.as_ref().map(|texture| WallDraw {
            shape: &self.shape,
            texture,
            state: DrawState::OPAQUE,
            recolour_required: self.recolour_pending,
        })
    }
}
