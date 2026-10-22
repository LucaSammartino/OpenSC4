// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub const WALL_TEXTURE_GROUP: u32 = 0x891B_0E1A;
pub const WALL_SEAM_METRES: f32 = 0.2;
pub const WALL_OUTWARD_OFFSET_RANGE: Vec2 = [0.001, 0.002];

/// Primitive edge coordinates and terrain sample, not owner-generated geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WallSample { pub position: Vec2, pub terrain_height: f32 }

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
}

impl WallShape {
    pub fn from_samples(side: Edge, lot_height: f32, samples: &[WallSample], texture_scale: f32) -> Result<Self, ContractError> {
        if !lot_height.is_finite() || !texture_scale.is_finite() || samples.len() < 2
            || samples.iter().any(|s| !s.terrain_height.is_finite() || !s.position.into_iter().all(f32::is_finite))
        { return Err(ContractError::InvalidInput); }
        let mut bands = Vec::new();
        for pair in samples.windows(2) {
            let [a, b] = [pair[0], pair[1]];
            let dx = (b.position[0] - a.position[0]).abs();
            let dz = (b.position[1] - a.position[1]).abs();
            let valid_segment = match side {
                Edge::North | Edge::South => dx == CELL_METRES && dz == 0.0,
                Edge::East | Edge::West => dx == 0.0 && dz == CELL_METRES,
            };
            if !valid_segment { return Err(ContractError::InvalidInput); }
            // Literal §3: an entirely level segment gets nothing. A segment with
            // only one equal endpoint is retained as a band, not triangulated.
            if a.terrain_height == lot_height && b.terrain_height == lot_height { continue; }
            let heights = [a.terrain_height, b.terrain_height].map(|terrain| [
                terrain.min(lot_height) - WALL_SEAM_METRES,
                terrain.max(lot_height) + WALL_SEAM_METRES,
            ]);
            let vertical_repeats = heights.map(|[bottom, top]| (top - bottom) * texture_scale);
            if heights.into_iter().flatten().chain(vertical_repeats).any(|v| !v.is_finite()) {
                return Err(ContractError::InvalidInput);
            }
            bands.push(WallBand {
                edge_positions: [a.position, b.position], terrain_heights: [a.terrain_height, b.terrain_height],
                lot_height, expanded_heights: heights, vertical_repeats,
            });
        }
        Ok(Self { side, bands, outward_normal: side.normal(), outward_offset_range: WALL_OUTWARD_OFFSET_RANGE })
    }

    /// Exact triangles cannot be recovered from the published offset range and
    /// seam description. This is intentionally not an original-wall callback.
    pub fn triangles(&self) -> Result<Vec<Vec3>, ContractError> {
        Err(ContractError::WallTriangleLayout)
    }
}

pub fn across_edge_lot_is_flat(flags: u32) -> bool { flags & 1 != 0 }

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
    /// The native box-to-cell enumeration is not specified (LM4). This input is
    /// an explicitly sampled edge, never a delegated original-owner shape.
    pub fn init(side: Edge, lot_height: f32, samples: &[WallSample], texture_scale: f32, keys: [Option<u32>; 5], facing_test: bool) -> Result<Self, ContractError> {
        Ok(Self {
            shape: WallShape::from_samples(side, lot_height, samples, texture_scale)?, keys, facing_test,
            texture: None, zoom: None, rotation: None, recolour_pending: false,
        })
    }

    pub fn shape(&self) -> &WallShape { &self.shape }
    pub fn texture(&self) -> Option<&T> { self.texture.as_ref() }
    pub fn zoom(&self) -> Option<Zoom> { self.zoom }
    pub fn rotation(&self) -> Option<Edge> { self.rotation }
    pub fn recolour_pending(&self) -> bool { self.recolour_pending }
    pub fn facing_test(&self) -> bool { self.facing_test }
    pub fn set_facing_test(&mut self, enabled: bool) { self.facing_test = enabled; }

    pub fn setup_vertices(&mut self, lot_height: f32, samples: &[WallSample], texture_scale: f32) -> Result<(), ContractError> {
        let shape = WallShape::from_samples(self.shape.side, lot_height, samples, texture_scale)?;
        self.shape = shape;
        self.recolour_pending = true;
        Ok(())
    }

    pub fn sort_value(&self) -> u32 {
        self.texture.as_ref().map_or(0xC986_2A86, |texture| texture.id() | 0x2A86)
    }

    pub fn load_texture<F: TextureFactory<Binding = T>>(&mut self, zoom: Zoom, factory: &mut F) {
        self.texture = load_with_fallback(factory,
            self.keys[zoom.index()].map(|instance| TextureKey::Wall { group: WALL_TEXTURE_GROUP, instance }),
            TextureKey::Wall { group: WALL_TEXTURE_GROUP, instance: 0x695C_00C6 + zoom.value() });
        self.zoom = Some(zoom);
    }

    pub fn draw<F: TextureFactory<Binding = T>>(&mut self, highlighted: bool, zoom: Zoom, rotation: Edge, factory: &mut F) -> Option<WallDraw<'_, T>> {
        if highlighted { return None; }
        if self.zoom != Some(zoom) { self.load_texture(zoom, factory); }
        if self.rotation != Some(rotation) {
            self.rotation = Some(rotation);
            self.recolour_pending = true;
        }
        if self.facing_test && !self.shape.side.wall_faces(rotation) { return None; }
        // LM5: an unresolved texture supplies no drawable band plan.
        self.texture.as_ref().map(|texture| WallDraw {
            shape: &self.shape, texture, state: DrawState::OPAQUE, recolour_required: self.recolour_pending,
        })
    }
}
