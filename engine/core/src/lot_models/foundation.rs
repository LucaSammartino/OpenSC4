// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Footprint {
    pub min: Vec2,
    pub max: Vec2,
}

impl Footprint {
    pub(super) fn valid(self) -> bool {
        self.min.into_iter().chain(self.max).all(f32::is_finite)
            && self.min[0] <= self.max[0]
            && self.min[1] <= self.max[1]
            && (self.max[0] - self.min[0]).is_finite()
            && (self.max[1] - self.min[1]).is_finite()
    }
}

/// One ordered LM3 quad. `vertices` owns position/BGRA/UV records; the semantic
/// bounds/repeats/colour fields are retained for existing portable consumers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoundationFace {
    pub bounds: Bounds,
    pub repeats: Vec2,
    pub colour: Option<Vec3>,
    pub vertices: [WallVertex; 4],
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
        let face = |min, max, repeats: Vec2, positions: [Vec3; 4]| FoundationFace {
            bounds: Bounds { min, max },
            repeats,
            colour: None,
            vertices: std::array::from_fn(|i| WallVertex {
                position: positions[i],
                colour: [0; 4],
                uv: [[0., 0.], [0., repeats[1]], [repeats[0], repeats[1]], [repeats[0], 0.]][i],
            }),
        };
        let x = (f64::from(x0) + f64::from(width)) as f32;
        let z = (f64::from(z0) + f64::from(depth)) as f32;
        let b = (f64::from(top) - f64::from(height)) as f32;
        let reciprocal = (1.0 / f64::from(scale)) as f32;
        let uv = |d: f32| ((f64::from(d) * 0.0625) * f64::from(reciprocal) * 16.0) as f32;
        Self {
            sides: [
                face(
                    [x0, base, z0],
                    [x1, top, z0],
                    [width / CELL_METRES, height / CELL_METRES],
                    [[x, top, z0], [x, b, z0], [x0, b, z0], [x0, top, z0]],
                ),
                face(
                    [x1, base, z0],
                    [x1, top, z1],
                    [width / CELL_METRES, height / CELL_METRES],
                    [[x, top, z], [x, b, z], [x, b, z0], [x, top, z0]],
                ),
                face(
                    [x0, base, z1],
                    [x1, top, z1],
                    [width / CELL_METRES, height / CELL_METRES],
                    [[x0, top, z], [x0, b, z], [x, b, z], [x, top, z]],
                ),
                face(
                    [x0, base, z0],
                    [x0, top, z1],
                    [width / CELL_METRES, height / CELL_METRES],
                    [[x0, top, z0], [x0, b, z0], [x0, b, z], [x0, top, z]],
                ),
            ],
            top: face(
                [x0, top, z0],
                [x1, top, z1],
                [uv(width), uv(depth)],
                [[x0, top, z0], [x0, top, z], [x, top, z], [x, top, z0]],
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoundationTextureKeys {
    pub sides: [Option<u32>; 5],
    pub tops: [Option<u32>; 5],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderHeight {
    pub ground: f32,
    pub water: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeightAdjustment {
    pub base: f32,
    pub base_changed: bool,
    pub rebuilt: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoundationPart {
    Side(Edge),
    Top,
}

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
    pub fn new(
        footprint: Footprint,
        base: f32,
        top: f32,
        top_scale: f32,
        keys: FoundationTextureKeys,
    ) -> Result<Self, ContractError> {
        if !footprint.valid()
            || !base.is_finite()
            || !top.is_finite()
            || base > top
            || !top_scale.is_finite()
            // Unsupported numeric domain, not a native zero-scale guard:
            // native unchecked division can yield nonfinite UVs (LM8).
            || top_scale <= 0.0
            || !(top - base).is_finite()
        {
            return Err(ContractError::InvalidInput);
        }
        let shape = FoundationShape::new(footprint, base, top, top_scale);
        if shape
            .sides
            .iter()
            .chain(std::iter::once(&shape.top))
            .any(|face| face.repeats.into_iter().any(|v| !v.is_finite()))
        {
            return Err(ContractError::InvalidInput);
        }
        Ok(Self {
            footprint,
            base,
            top,
            top_scale,
            shape,
            keys,
            side_texture: None,
            top_texture: None,
            zoom: None,
            rotation: None,
        })
    }

    pub fn shape(&self) -> &FoundationShape {
        &self.shape
    }
    pub fn base(&self) -> f32 {
        self.base
    }
    pub fn zoom(&self) -> Option<Zoom> {
        self.zoom
    }
    pub fn rotation(&self) -> Option<Edge> {
        self.rotation
    }
    pub fn side_texture(&self) -> Option<&T> {
        self.side_texture.as_ref()
    }
    pub fn top_texture(&self) -> Option<&T> {
        self.top_texture.as_ref()
    }

    pub fn keys(&self) -> &FoundationTextureKeys {
        &self.keys
    }

    pub fn recolour<L: FoundationLighting>(&mut self, lighting: &mut L) {
        for (i, face) in self
            .shape
            .sides
            .iter_mut()
            .chain(std::iter::once(&mut self.shape.top))
            .enumerate()
        {
            let normal = if i == 4 { [0., 1., 0.] } else { Edge::ALL[i].normal() };
            let c = lighting.components_for_normal(normal);
            for v in &mut face.vertices {
                v.colour = [c[2] as u8, c[1] as u8, c[0] as u8, 255];
            }
        }
    }

    pub fn adjust_from_terrain<H: FoundationTerrain, L: Lighting>(
        &mut self,
        force: bool,
        host: &mut H,
        lighting: &mut L,
    ) -> Result<HeightAdjustment, ContractError> {
        let x0 = super::wall::lattice_index(self.footprint.min[0], false)?;
        let x1 = super::wall::lattice_index(self.footprint.max[0], false)?;
        let z0 = super::wall::lattice_index(self.footprint.min[1], false)?;
        let z1 = super::wall::lattice_index(self.footprint.max[1], false)?;
        let stride = host.width().checked_add(1).ok_or(ContractError::InvalidInput)?;
        let mut samples = Vec::new();
        for x in x0..=x1 {
            for z in z0..=z1 {
                if x != x0 && x != x1 && z != z0 && z != z1 {
                    continue;
                }
                let index = z
                    .checked_mul(stride)
                    .and_then(|v| v.checked_add(x))
                    .ok_or(ContractError::InvalidInput)?;
                let ground = host.ground(index);
                let water = host.water(index);
                samples.try_reserve(1).map_err(|_| ContractError::InvalidInput)?;
                samples.push(BorderHeight {
                    ground,
                    water: Some(water),
                });
            }
        }
        // Reversed ranges have maximumDrop=0, rather than inventing a sample.
        if samples.is_empty() {
            samples.push(BorderHeight {
                ground: self.top,
                water: None,
            });
        }
        self.adjust_height_to_terrain(&samples, force, lighting)
    }

    pub fn sort_value(&self) -> u32 {
        self.side_texture
            .as_ref()
            .or(self.top_texture.as_ref())
            .map_or(0xE986_2AB6, |texture| texture.id() | 0x2AB6)
    }

    pub fn setup_vertices<L: Lighting>(&mut self, geometry: bool, colours: bool, lighting: &mut L) {
        if geometry {
            let colours = self.shape.sides.map(|face| face.colour);
            let bytes = self.shape.sides.map(|face| face.vertices.map(|v| v.colour));
            let top_colour = self.shape.top.colour;
            let top_bytes = self.shape.top.vertices.map(|v| v.colour);
            self.shape = FoundationShape::new(self.footprint, self.base, self.top, self.top_scale);
            for ((face, colour), bytes) in self.shape.sides.iter_mut().zip(colours).zip(bytes) {
                face.colour = colour;
                for (v, c) in face.vertices.iter_mut().zip(bytes) {
                    v.colour = c;
                }
            }
            self.shape.top.colour = top_colour;
            for (v, c) in self.shape.top.vertices.iter_mut().zip(top_bytes) {
                v.colour = c;
            }
        }
        if colours {
            for edge in Edge::ALL {
                self.shape.sides[edge.index()].colour = Some(lighting.for_normal(edge.normal()));
            }
            self.shape.top.colour = Some(lighting.for_normal([0.0, 1.0, 0.0]));
        }
    }

    /// Explicit sample convenience entry; `adjust_from_terrain` owns enumeration.
    /// Geometry rebuild preserves existing normal colours; relighting is explicit.
    pub fn adjust_height_to_terrain<L: Lighting>(
        &mut self,
        border: &[BorderHeight],
        force_rebuild: bool,
        lighting: &mut L,
    ) -> Result<HeightAdjustment, ContractError> {
        if border.is_empty()
            || border
                .iter()
                .any(|v| !v.ground.is_finite() || v.water.is_some_and(|w| !w.is_finite()))
        {
            return Err(ContractError::InvalidInput);
        }
        let mut drop = 0.0_f32;
        for sample in border {
            let height = sample.water.map_or(sample.ground, |water| sample.ground.max(water));
            let difference = f64::from(self.top) - f64::from(height);
            if difference > f64::from(drop) {
                drop = difference as f32;
            }
        }
        // LM3: both counts include +16, held in registers through truncation.
        // Finite, nearest-even PC53 subset; no universal x87 control claim.
        let existing = ((f64::from(self.top) - f64::from(self.base) + 16.0) * 0.0625).trunc();
        let required = ((f64::from(drop) + 16.0) * 0.0625).trunc();
        if !(0.0..=134_217_727.0).contains(&required) || !(0.0..=2_147_483_647.0).contains(&existing) {
            return Err(ContractError::InvalidInput);
        }
        let candidate = (f64::from(self.top) - required * 16.0) as f32;
        if !candidate.is_finite() || !(self.top - candidate).is_finite() {
            return Err(ContractError::InvalidInput);
        }
        let base_changed = required > existing;
        if base_changed {
            self.base = candidate;
        }
        let rebuilt = base_changed || force_rebuild;
        if rebuilt {
            self.setup_vertices(true, false, lighting);
        }
        Ok(HeightAdjustment {
            base: self.base,
            base_changed,
            rebuilt,
        })
    }

    pub fn load_textures<F: TextureFactory<Binding = T>>(&mut self, zoom: Zoom, factory: &mut F) {
        if self.zoom == Some(zoom) {
            return;
        }
        self.zoom = Some(zoom);
        self.side_texture = None;
        self.side_texture = load_owner_key(
            factory,
            &mut self.keys.sides[zoom.index()],
            0x69F2_0000 + zoom.value(),
            TextureKey::FoundationSide,
        );
        self.top_texture = None;
        self.top_texture = load_owner_key(
            factory,
            &mut self.keys.tops[zoom.index()],
            0x0912_220F + zoom.value(),
            TextureKey::FoundationTop,
        );
        self.zoom = Some(zoom);
    }

    pub fn draw<F: TextureFactory<Binding = T>, L: Lighting>(
        &mut self,
        highlighted: bool,
        zoom: Zoom,
        rotation: Edge,
        factory: &mut F,
        lighting: &mut L,
    ) -> Vec<FoundationDraw<'_, T>> {
        if highlighted {
            return Vec::new();
        }
        if self.rotation != Some(rotation) {
            self.rotation = Some(rotation);
            self.setup_vertices(false, true, lighting);
        }
        if self.zoom != Some(zoom) {
            self.load_textures(zoom, factory);
        }
        let mut draws = Vec::with_capacity(3);
        if let Some(texture) = self.side_texture.as_ref().filter(|t| t.id() != 0) {
            for edge in rotation.visible_foundation_edges() {
                draws.push(FoundationDraw {
                    part: FoundationPart::Side(edge),
                    face: self.shape.sides[edge.index()],
                    texture,
                    primitive_type: 6,
                    state: DrawState::OPAQUE,
                });
            }
        }
        if let Some(texture) = self.top_texture.as_ref().filter(|t| t.id() != 0) {
            draws.push(FoundationDraw {
                part: FoundationPart::Top,
                face: self.shape.top,
                texture,
                primitive_type: 6,
                state: DrawState::OPAQUE,
            });
        }
        draws
    }
}
