// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Footprint { pub min: Vec2, pub max: Vec2 }

impl Footprint {
    pub(super) fn valid(self) -> bool {
        self.min.into_iter().chain(self.max).all(f32::is_finite)
            && self.min[0] <= self.max[0] && self.min[1] <= self.max[1]
    }
}

/// A labelled planar face. Bounds and repeat counts deliberately do not imply
/// a native vertex order, UV origin, winding or upload format (LM3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoundationFace {
    pub bounds: Bounds,
    pub repeats: Vec2,
    pub colour: Option<Vec3>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FoundationShape {
    pub sides: [FoundationFace; 4],
    pub top: FoundationFace,
}

impl FoundationShape {
    fn new(footprint: Footprint, base: f32, top: f32, scale: f32) -> Self {
        let [x0, z0] = footprint.min;
        let [x1, z1] = footprint.max;
        let width = x1 - x0;
        let depth = z1 - z0;
        let height = top - base;
        let face = |min, max, repeats| FoundationFace { bounds: Bounds { min, max }, repeats, colour: None };
        Self {
            sides: [
                face([x0, base, z0], [x1, top, z0], [width / CELL_METRES, height / CELL_METRES]),
                face([x1, base, z0], [x1, top, z1], [depth / CELL_METRES, height / CELL_METRES]),
                face([x0, base, z1], [x1, top, z1], [width / CELL_METRES, height / CELL_METRES]),
                face([x0, base, z0], [x0, top, z1], [depth / CELL_METRES, height / CELL_METRES]),
            ],
            top: face([x0, top, z0], [x1, top, z1], [width / scale, depth / scale]),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoundationTextureKeys {
    pub sides: [Option<u32>; 5],
    pub tops: [Option<u32>; 5],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderHeight { pub ground: f32, pub water: Option<f32> }

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeightAdjustment {
    pub base: f32,
    pub base_changed: bool,
    pub rebuilt: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoundationPart { Side(Edge), Top }

pub struct FoundationDraw<'a, T> {
    pub part: FoundationPart,
    pub face: FoundationFace,
    pub texture: &'a T,
    /// §2 and graphics-formats §3.7: four-vertex primitive type 6.
    pub primitive_type: u8,
    pub state: DrawState,
}

pub struct FoundationModel<T> {
    footprint: Footprint,
    base: f32,
    top: f32,
    top_scale: f32,
    shape: FoundationShape,
    keys: FoundationTextureKeys,
    side_texture: Option<T>,
    top_texture: Option<T>,
    zoom: Option<Zoom>,
    rotation: Option<Edge>,
}

impl<T: Texture> FoundationModel<T> {
    /// Explicit geometry/keys, not a reconstruction of native constructor defaults.
    pub fn new(footprint: Footprint, base: f32, top: f32, top_scale: f32, keys: FoundationTextureKeys) -> Result<Self, ContractError> {
        if !footprint.valid() || !base.is_finite() || !top.is_finite() || base > top
            || !top_scale.is_finite() || top_scale <= 0.0
            || !(top - base).is_finite()
        {
            return Err(ContractError::InvalidInput);
        }
        Ok(Self {
            footprint, base, top, top_scale,
            shape: FoundationShape::new(footprint, base, top, top_scale), keys,
            side_texture: None, top_texture: None, zoom: None, rotation: None,
        })
    }

    pub fn shape(&self) -> &FoundationShape { &self.shape }
    pub fn base(&self) -> f32 { self.base }
    pub fn zoom(&self) -> Option<Zoom> { self.zoom }
    pub fn rotation(&self) -> Option<Edge> { self.rotation }
    pub fn side_texture(&self) -> Option<&T> { self.side_texture.as_ref() }
    pub fn top_texture(&self) -> Option<&T> { self.top_texture.as_ref() }

    pub fn sort_value(&self) -> u32 {
        self.side_texture.as_ref().map_or(0xE986_2AB6, |texture| texture.id() | 0x2AB6)
    }

    pub fn setup_vertices<L: Lighting>(&mut self, geometry: bool, colours: bool, lighting: &mut L) {
        if geometry {
            let colours = self.shape.sides.map(|face| face.colour);
            let top_colour = self.shape.top.colour;
            self.shape = FoundationShape::new(self.footprint, self.base, self.top, self.top_scale);
            for (face, colour) in self.shape.sides.iter_mut().zip(colours) { face.colour = colour; }
            self.shape.top.colour = top_colour;
        }
        if colours {
            for edge in Edge::ALL {
                self.shape.sides[edge.index()].colour = Some(lighting.for_normal(edge.normal()));
            }
            self.shape.top.colour = Some(lighting.for_normal([0.0, 1.0, 0.0]));
        }
    }

    /// Samples are primitive border heights; enumeration of the native occupant
    /// border remains LM3. Whole steps use the spec's inferred upward rounding.
    /// Geometry rebuild preserves existing normal colours; relighting is explicit.
    pub fn adjust_height_to_terrain<L: Lighting>(&mut self, border: &[BorderHeight], force_rebuild: bool, lighting: &mut L) -> Result<HeightAdjustment, ContractError> {
        if border.is_empty() || border.iter().any(|v| !v.ground.is_finite() || v.water.is_some_and(|w| !w.is_finite())) {
            return Err(ContractError::InvalidInput);
        }
        let mut drop = 0.0_f32;
        for sample in border {
            let height = sample.water.map_or(sample.ground, |water| sample.ground.max(water));
            drop = drop.max(self.top - height);
        }
        let candidate = self.top - (drop / CELL_METRES).ceil() * CELL_METRES;
        if !candidate.is_finite() { return Err(ContractError::InvalidInput); }
        let base_changed = candidate < self.base;
        if base_changed { self.base = candidate; }
        let rebuilt = base_changed || force_rebuild;
        if rebuilt { self.setup_vertices(true, false, lighting); }
        Ok(HeightAdjustment { base: self.base, base_changed, rebuilt })
    }

    pub fn load_textures<F: TextureFactory<Binding = T>>(&mut self, zoom: Zoom, factory: &mut F) {
        self.side_texture = load_with_fallback(factory,
            self.keys.sides[zoom.index()].map(TextureKey::FoundationSide),
            TextureKey::FoundationSide(0x69F2_0000 + zoom.value()));
        self.top_texture = load_with_fallback(factory,
            self.keys.tops[zoom.index()].map(TextureKey::FoundationTop),
            TextureKey::FoundationTop(0x0912_220F + zoom.value()));
        self.zoom = Some(zoom);
    }

    pub fn draw<F: TextureFactory<Binding = T>, L: Lighting>(&mut self, highlighted: bool, zoom: Zoom, rotation: Edge, factory: &mut F, lighting: &mut L) -> Vec<FoundationDraw<'_, T>> {
        if highlighted { return Vec::new(); }
        if self.rotation != Some(rotation) {
            self.rotation = Some(rotation);
            self.setup_vertices(false, true, lighting);
        }
        if self.zoom != Some(zoom) { self.load_textures(zoom, factory); }
        let mut draws = Vec::with_capacity(3);
        // LM5: literal "with a side texture ... then the top" reading. A top-only
        // draw after a missing side binding is not specified.
        if let Some(texture) = self.side_texture.as_ref() {
            for edge in rotation.visible_foundation_edges() {
                draws.push(FoundationDraw {
                    part: FoundationPart::Side(edge), face: self.shape.sides[edge.index()],
                    texture, primitive_type: 6, state: DrawState::OPAQUE,
                });
            }
            if let Some(texture) = self.top_texture.as_ref() {
                draws.push(FoundationDraw { part: FoundationPart::Top, face: self.shape.top, texture, primitive_type: 6, state: DrawState::OPAQUE });
            }
        }
        draws
    }
}
