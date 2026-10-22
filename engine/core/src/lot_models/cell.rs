// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub const CELL_TEXTURE_TYPE: u32 = 0x7AB5_0E44;
pub const CELL_TEXTURE_GROUP: u32 = 0x0986_135E;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Orientation(u8);
impl Orientation {
    pub fn new(value: u8) -> Result<Self, ContractError> {
        if value < 4 { Ok(Self(value)) } else { Err(ContractError::InvalidInput) }
    }
    pub fn value(self) -> usize { self.0 as usize }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CornerStep { Forward, Backward }

/// Explicit corner data and positive-step direction: the native cycle and
/// strip order are not published (LM6). No layout/default is supplied by core.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellCornerLayout {
    positions: [Vec2; 4],
    texture_corners: [Vec2; 4],
    step: CornerStep,
}

impl CellCornerLayout {
    pub fn new(positions: [Vec2; 4], texture_corners: [Vec2; 4], step: CornerStep) -> Result<Self, ContractError> {
        let is_square = |corners: &[Vec2; 4]| {
            [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]].iter()
                .all(|corner| corners.contains(corner))
        };
        if !is_square(&positions) || !is_square(&texture_corners) { return Err(ContractError::InvalidInput); }
        Ok(Self { positions, texture_corners, step })
    }

    pub fn texture_corners(self, orientation: Orientation, mirror: bool) -> [Vec2; 4] {
        let steps = match self.step { CornerStep::Forward => orientation.value(), CornerStep::Backward => (4 - orientation.value()) % 4 };
        std::array::from_fn(|index| {
            let [u, v] = self.texture_corners[(index + steps) % 4];
            // Literal §4: rotate corners, then mirror u. Native order is LM6.
            [if mirror { 1.0 - u } else { u }, v]
        })
    }

    pub fn positions(&self) -> &[Vec2; 4] { &self.positions }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellRecord {
    pub texture_key: u32,
    pub cell: [i32; 2],
    pub orientation: Orientation,
    pub mirror: bool,
    pub rgb: [u8; 3],
    pub alpha: u8,
    pub mode: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LotSample { pub flags: u32, pub altitude: f32 }

/// Primitive lot/terrain queries; no cell-model output may be supplied here.
pub trait CellPlacementSource {
    fn lot_at(&mut self, cell: [i32; 2]) -> Option<LotSample>;
    /// Heights follow the explicit CellCornerLayout positions.
    fn terrain_corners(&mut self, cell: [i32; 2]) -> [f32; 4];
}

#[derive(Clone, Debug, PartialEq)]
pub struct CellPlacement {
    pub flat: bool,
    pub positions: [Vec3; 4],
    pub colours: Option<[Vec3; 4]>,
    pub bounds: Bounds,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellMode { Opaque, Alpha, Tinted }

pub fn cell_mode(record_mode: u8, record_alpha: u8, texture_has_alpha: bool) -> CellMode {
    if record_mode != 0 { CellMode::Tinted }
    else if texture_has_alpha || record_alpha != 255 { CellMode::Alpha }
    else { CellMode::Opaque }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnvironmentColour { pub rgb: Vec3, pub alpha: f32 }

/// Effective inputs to the combiner, not an invented implementation of its
/// unpublished per-mode instruction record (LM1, LM7).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CombinerRequest { pub mode: u8, pub environment: EnvironmentColour }

pub fn tinted_combiner(record: &CellRecord, global_light: Vec3, no_add_signed: bool) -> CombinerRequest {
    let substitute = record.mode == 3 && no_add_signed;
    let alpha = f32::from(record.alpha) / 255.0;
    CombinerRequest {
        mode: if substitute { 1 } else { record.mode },
        environment: EnvironmentColour {
            rgb: std::array::from_fn(|i| f32::from(record.rgb[i]) * global_light[i] / 255.0),
            alpha: if substitute { alpha * 0.5 } else { alpha },
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureFilter { Linear }

pub enum CellGeometryDraw<'a> {
    /// Four vertices in the caller's explicit layout order. This does not certify
    /// that layout as the native strip cycle (LM6).
    FlatStrip { placement: &'a CellPlacement, corners: [Vec2; 4] },
    /// The terrain view is a foreign drawing primitive, not an original cell
    /// model. The byte alpha is kept separately from the combiner's float alpha.
    TerrainQuad { cell: [i32; 2], corners: [Vec2; 4], alpha: u8, forced: bool },
}

pub struct CellDraw<'a, T> {
    pub texture: &'a T,
    pub mode: CellMode,
    pub state: DrawState,
    pub clamp: bool,
    /// Only tinted cells prescribe linear filtering; other context state is kept.
    pub filter: Option<TextureFilter>,
    pub combiner: Option<CombinerRequest>,
    pub geometry: CellGeometryDraw<'a>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellDrawContext {
    pub highlighted: bool,
    pub zoom: Zoom,
    /// The raw public condition: draw-context +0x04 is nonzero.
    pub translucent_pass: bool,
    /// The raw public value: +0x20 == 8 forces terrain drawing.
    pub terrain_mode: u32,
    pub global_light: Vec3,
    pub no_add_signed: bool,
}

pub struct CellTextureModel<T> {
    record: CellRecord,
    layout: CellCornerLayout,
    placement: Option<CellPlacement>,
    texture: Option<T>,
    zoom: Option<Zoom>,
}

impl<T: Texture> CellTextureModel<T> {
    pub fn new(record: CellRecord, layout: CellCornerLayout) -> Self {
        Self { record, layout, placement: None, texture: None, zoom: None }
    }

    pub fn record(&self) -> &CellRecord { &self.record }
    pub fn placement(&self) -> Option<&CellPlacement> { self.placement.as_ref() }
    pub fn texture(&self) -> Option<&T> { self.texture.as_ref() }
    pub fn zoom(&self) -> Option<Zoom> { self.zoom }
    pub fn corners(&self) -> [Vec2; 4] { self.layout.texture_corners(self.record.orientation, self.record.mirror) }

    pub fn place<S: CellPlacementSource, L: Lighting>(&mut self, source: &mut S, lighting: &mut L) -> Result<(), ContractError> {
        let lot = source.lot_at(self.record.cell);
        let flat = lot.is_some_and(|sample| sample.flags & 1 != 0);
        let heights = if flat {
            [lot.expect("flat implies a lot sample").altitude; 4]
        } else { source.terrain_corners(self.record.cell) };
        if heights.into_iter().any(|v| !v.is_finite()) { return Err(ContractError::InvalidInput); }
        // LM8: multiplication in the existing f32 coordinate domain, without
        // a guessed native i32 multiply/overflow policy.
        let [x, z] = self.record.cell.map(|v| v as f32 * CELL_METRES);
        let positions = std::array::from_fn(|i| [
            x + self.layout.positions[i][0] * CELL_METRES,
            heights[i],
            z + self.layout.positions[i][1] * CELL_METRES,
        ]);
        let colours = if flat { Some(positions.map(|p| lighting.at_corner(p, [0.0, 1.0, 0.0]))) } else { None };
        let low = heights.into_iter().fold(f32::INFINITY, f32::min);
        let high = heights.into_iter().fold(f32::NEG_INFINITY, f32::max);
        self.placement = Some(CellPlacement {
            flat, positions, colours,
            bounds: Bounds { min: [x + 0.1, low - 2.0, z + 0.1], max: [x + CELL_METRES - 0.1, high + 2.0, z + CELL_METRES - 0.1] },
        });
        Ok(())
    }

    pub fn load_texture<F: TextureFactory<Binding = T>>(&mut self, zoom: Zoom, factory: &mut F) -> Result<(), ContractError> {
        // No wrap/saturate guess for an unpublished key overflow case (LM5).
        let instance = self.record.texture_key.checked_add(zoom.value()).ok_or(ContractError::TextureKeyOverflow)?;
        let key = |instance| TextureKey::Cell { type_id: CELL_TEXTURE_TYPE, group: CELL_TEXTURE_GROUP, instance };
        self.texture = load_with_fallback(factory, Some(key(instance)), key(self.record.texture_key));
        self.zoom = Some(zoom);
        Ok(())
    }

    pub fn draw<F: TextureFactory<Binding = T>>(&mut self, context: CellDrawContext, factory: &mut F) -> Result<Option<CellDraw<'_, T>>, ContractError> {
        if context.highlighted { return Ok(None); }
        if self.zoom != Some(context.zoom) { self.load_texture(context.zoom, factory)?; }
        let Some(texture) = self.texture.as_ref() else { return Ok(None); };
        let mode = cell_mode(self.record.mode, self.record.alpha, texture.has_alpha());
        if mode != CellMode::Opaque && !context.translucent_pass { return Ok(None); }
        let placement = self.placement.as_ref().ok_or(ContractError::InvalidInput)?;
        let corners = self.corners();
        let geometry = if placement.flat {
            CellGeometryDraw::FlatStrip { placement, corners }
        } else {
            CellGeometryDraw::TerrainQuad { cell: self.record.cell, corners, alpha: self.record.alpha, forced: context.terrain_mode == 8 }
        };
        Ok(Some(CellDraw {
            texture, mode,
            state: match mode { CellMode::Opaque => DrawState::OPAQUE, CellMode::Alpha => DrawState::ALPHA, CellMode::Tinted => DrawState::TINTED },
            clamp: true,
            filter: if mode == CellMode::Tinted { Some(TextureFilter::Linear) } else { None },
            combiner: if mode == CellMode::Tinted { Some(tinted_combiner(&self.record, context.global_light, context.no_add_signed)) } else { None },
            geometry,
        }))
    }
}
