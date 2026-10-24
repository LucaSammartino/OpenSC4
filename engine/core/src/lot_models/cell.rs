// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub const CELL_TEXTURE_TYPE: u32 = 0x7AB5_0E44;
pub const CELL_TEXTURE_GROUP: u32 = 0x0986_135E;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Orientation(u8);
impl Orientation {
    /// Raw record bytes >3 retain UVs rather than selecting a table (§4.1).
    pub fn from_byte(value: u8) -> Self {
        Self(value)
    }
    pub fn new(value: u8) -> Result<Self, ContractError> {
        if value < 4 {
            Ok(Self(value))
        } else {
            Err(ContractError::InvalidInput)
        }
    }
    pub fn value(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CornerStep {
    Forward,
    Backward,
}

/// Legacy explicit corner helper. The cell owner always uses the settled LM6
/// native strip and orientation table, independent of this compatibility helper.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellCornerLayout {
    positions: [Vec2; 4],
    texture_corners: [Vec2; 4],
    step: CornerStep,
}

impl CellCornerLayout {
    pub fn native() -> Self {
        Self {
            positions: [[0., 0.], [1., 0.], [0., 1.], [1., 1.]],
            texture_corners: [[0., 0.], [1., 0.], [0., 1.], [1., 1.]],
            step: CornerStep::Forward,
        }
    }

    pub fn native_texture_corners(orientation: Orientation, mirror: bool) -> [Vec2; 4] {
        // Unknown initial UV bytes remain LM2. This standalone helper starts
        // with explicit zero storage; the owner retains its actual prior UVs.
        let mut corners = [[0.; 2]; 4];
        Self::update_native_corners(&mut corners, orientation, mirror);
        corners
    }

    pub fn update_native_corners(corners: &mut [Vec2; 4], orientation: Orientation, mirror: bool) {
        let tables = [
            [[0., 0.], [1., 0.], [0., 1.], [1., 1.]],
            [[0., 1.], [0., 0.], [1., 1.], [1., 0.]],
            [[1., 1.], [0., 1.], [1., 0.], [0., 0.]],
            [[1., 0.], [1., 1.], [0., 0.], [0., 1.]],
        ];
        if let Some(table) = tables.get(orientation.value()) {
            *corners = *table;
        }
        if mirror {
            for [u, _] in corners {
                *u = 1. - *u;
            }
        }
    }
    pub fn new(positions: [Vec2; 4], texture_corners: [Vec2; 4], step: CornerStep) -> Result<Self, ContractError> {
        let is_square = |corners: &[Vec2; 4]| {
            [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]
                .iter()
                .all(|corner| corners.contains(corner))
        };
        if !is_square(&positions) || !is_square(&texture_corners) {
            return Err(ContractError::InvalidInput);
        }
        Ok(Self {
            positions,
            texture_corners,
            step,
        })
    }

    pub fn texture_corners(self, orientation: Orientation, mirror: bool) -> [Vec2; 4] {
        let steps = match self.step {
            CornerStep::Forward => orientation.value(),
            CornerStep::Backward => (4 - orientation.value()) % 4,
        };
        std::array::from_fn(|index| {
            let [u, v] = self.texture_corners[(index + steps) % 4];
            // Literal §4: rotate corners, then mirror u. Native order is LM6.
            [if mirror { 1.0 - u } else { u }, v]
        })
    }

    pub fn positions(&self) -> &[Vec2; 4] {
        &self.positions
    }
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
pub struct LotSample {
    pub flags: u32,
    pub altitude: f32,
}

/// Primitive lot/terrain queries; no cell-model output may be supplied here.
pub trait CellPlacementSource {
    fn lot_at(&mut self, cell: [i32; 2]) -> Option<LotSample>;
    /// Individual terrain slot17 outputs in LM6 order: (0,0),(1,0),(0,1),(1,1).
    fn terrain_corners(&mut self, cell: [i32; 2]) -> [f32; 4];
    fn bypass_flat(&mut self) -> bool {
        false
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CellPlacement {
    pub flat: bool,
    pub positions: [Vec3; 4],
    pub colours: Option<[Vec3; 4]>,
    pub bounds: Bounds,
    pub vertices: [WallVertex; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellMode {
    Opaque,
    Alpha,
    Tinted,
}

pub fn cell_mode(record_mode: u8, record_alpha: u8, texture_has_alpha: bool) -> CellMode {
    if record_mode != 255 {
        CellMode::Tinted
    } else if texture_has_alpha || record_alpha != 255 {
        CellMode::Alpha
    } else {
        CellMode::Opaque
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnvironmentColour {
    pub rgb: Vec3,
    pub alpha: f32,
}

/// Owned LM7 bytes and environment colour. Modes 0–3 have published driver
/// meanings; upstream authors and other raw mode meanings remain LM1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CombinerRequest {
    pub mode: u8,
    pub environment: EnvironmentColour,
    pub bytes: [u8; 12],
}

pub fn tinted_combiner(record: &CellRecord, global_light: Vec3, no_add_signed: bool) -> CombinerRequest {
    let substitute = record.mode == 3 && no_add_signed;
    let k = f64::from(f32::from_bits(0x3b80_8081));
    let alpha = if substitute { record.alpha >> 1 } else { record.alpha };
    let mode = if substitute { 1 } else { record.mode };
    CombinerRequest {
        mode,
        bytes: [mode, 0, 2, 0x21, 1, 0, 3, 1, 0, 0, 0, 0],
        environment: EnvironmentColour {
            rgb: record
                .rgb
                .map(|v| (f64::from(v) * f64::from(global_light[0]) * k) as f32),
            alpha: (f64::from(alpha) * k) as f32,
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureFilter {
    Linear,
}

pub enum CellGeometryDraw<'a> {
    /// Four records in the settled LM6 native strip order.
    FlatStrip {
        placement: &'a CellPlacement,
        corners: [Vec2; 4],
    },
    /// The terrain view is a foreign drawing primitive, not an original cell
    /// model. The byte alpha is kept separately from the combiner's float alpha.
    TerrainQuad {
        cell: [i32; 2],
        corners: [Vec2; 4],
        alpha: u8,
        forced: bool,
    },
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellPackedState {
    pub value: u32,
    pub mask: u32,
}
impl CellPackedState {
    pub const OPAQUE: Self = Self {
        value: 0x0200_01a7,
        mask: 0x1e00_03ff,
    };
    pub const ALPHA: Self = Self {
        value: 0x020a_91bf,
        mask: 0x1fff_ffff,
    };
    pub const TINTED: Self = Self {
        value: 0x000a_80b5,
        mask: 0x101f_e3ff,
    };
    pub fn apply(self, old: u32) -> u32 {
        (old & !self.mask) | self.value
    }
}

/// Shared Init outputs; absence of the render service leaves them untouched.
/// Pre-Init native image state is unpublished (LM2), represented by None.
#[derive(Default)]
pub struct CellRenderInitialization {
    pub states: Option<[CellPackedState; 3]>,
    pub combiner: Option<[u8; 12]>,
}

impl CellRenderInitialization {
    pub fn init(&mut self, render_service_present: bool) -> bool {
        if !render_service_present {
            return false;
        }
        self.states = Some([CellPackedState::OPAQUE, CellPackedState::ALPHA, CellPackedState::TINTED]);
        self.combiner = Some([0, 0, 2, 0x21, 1, 0, 3, 1, 0, 0, 0, 0]);
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellRgbOperation {
    SelectArg1,
    Modulate,
    Add,
    AddSigned,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellDecodedCombiner {
    pub rgb_operation: CellRgbOperation,
    pub alpha_operation: u8,
    pub rgb_sources: [u8; 3],
    pub rgb_operands: [u8; 3],
    pub alpha_sources: [u8; 3],
    pub alpha_operands: [u8; 3],
    pub scales: [u8; 2],
}

impl CombinerRequest {
    pub fn decode(&self) -> Result<CellDecodedCombiner, ContractError> {
        if self.bytes[1..] != [0, 2, 0x21, 1, 0, 3, 1, 0, 0, 0, 0] {
            return Err(ContractError::InvalidInput);
        }
        let rgb_operation = match self.bytes[0] {
            0 => CellRgbOperation::SelectArg1,
            1 => CellRgbOperation::Modulate,
            2 => CellRgbOperation::Add,
            3 => CellRgbOperation::AddSigned,
            _ => return Err(ContractError::InvalidInput),
        };
        Ok(CellDecodedCombiner {
            rgb_operation,
            alpha_operation: 1,
            rgb_sources: [0, 2, 1],
            rgb_operands: [2, 2, 4],
            alpha_sources: [0, 3, 1],
            alpha_operands: [4, 4, 4],
            scales: [0, 0],
        })
    }
}

/// Typed requests consumed by a foreign context. Applying state includes that
/// context's unconditional wrap/filter/transform-reset requests, as documented
/// in §4.3; no duplicated context/device algorithm lives in this owner.
#[derive(Clone, Debug, PartialEq)]
pub enum CellCommand {
    ModelTransform,
    ResetShade,
    Texture {
        word: u32,
        stage: u8,
    },
    State(CellPackedState),
    Wrap(u8, u8, u8),
    Filter(u8, u8, u8),
    TexColour(EnvironmentColour),
    Combiner {
        bytes: [u8; 12],
        decoded: CellDecodedCombiner,
        stage: u8,
    },
    Flat {
        primitive: u8,
        format: u8,
        vertices: [WallVertex; 4],
    },
    Terrain {
        cell: [u8; 2],
        corners: [Vec2; 4],
        alpha: f32,
        forced: bool,
    },
}

pub trait CellCommands {
    /// Error denotes an unsupported foreign boundary, not native false/rollback.
    fn request(&mut self, command: CellCommand) -> Result<(), ContractError>;
    fn global_light(&mut self) -> Vec3;
    fn no_add_signed(&mut self) -> bool;
}

pub struct CellTextureModel<T> {
    record: CellRecord,
    layout: CellCornerLayout,
    placement: Option<CellPlacement>,
    texture: Option<T>,
    texture_alpha: bool,
    zoom: Option<Zoom>,
    rotation: Option<Edge>,
    corners: [Vec2; 4],
    basic: BasicModel,
}

impl<T> Drop for CellTextureModel<T> {
    fn drop(&mut self) {
        drop(self.texture.take());
    }
}

impl<T: Texture> LotModelLifetime for CellTextureModel<T> {
    fn basic(&self) -> &BasicModel {
        &self.basic
    }
    fn basic_mut(&mut self) -> &mut BasicModel {
        &mut self.basic
    }
    fn owner_interface(&self) -> ModelInterface {
        ModelInterface::Cell
    }
    fn destroy_resources(&mut self) {
        drop(self.texture.take());
    }
}

impl<T: Texture> CellTextureModel<T> {
    /// Fully initialized owner-request domain only; missing render-service Init
    /// and pre-init image state remain unsupported (LM2/LM7).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_owned<F: TextureFactory<Binding = T>, S: CellPlacementSource, L: WallLighting, C: CellCommands>(
        &mut self,
        context: CellDrawContext,
        rotation: Edge,
        source: &mut S,
        light: &mut L,
        factory: &mut F,
        commands: &mut C,
    ) -> Result<u8, ContractError> {
        if context.highlighted {
            return Ok(1);
        }
        if self.zoom != Some(context.zoom) {
            self.update_vertices(self.zoom.is_none(), true, source, light)?;
            self.zoom = Some(context.zoom);
            self.load_texture(context.zoom, factory)?;
        }
        if self.rotation != Some(rotation) {
            self.update_vertices(false, true, source, light)?;
            self.rotation = Some(rotation);
        }
        let Some(texture) = self.texture.as_ref() else {
            return Ok(1);
        };
        let mode = cell_mode(self.record.mode, self.record.alpha, self.texture_alpha);
        if mode != CellMode::Opaque && !context.translucent_pass {
            return Ok(2);
        }
        if self.record.mode != 255 && self.record.mode > 3 {
            return Err(ContractError::InvalidInput);
        }
        let count = texture.element_count();
        if count == 0 {
            return Err(ContractError::InvalidInput);
        }
        let index = if context.zoom.index() < count {
            context.zoom.index()
        } else {
            0
        };
        let word = texture.element_word(index).ok_or(ContractError::InvalidInput)?;
        commands.request(CellCommand::ModelTransform)?;
        commands.request(CellCommand::ResetShade)?;
        commands.request(CellCommand::Texture { word, stage: 0 })?;
        commands.request(CellCommand::State(match mode {
            CellMode::Opaque => CellPackedState::OPAQUE,
            CellMode::Alpha => CellPackedState::ALPHA,
            CellMode::Tinted => CellPackedState::TINTED,
        }))?;
        commands.request(CellCommand::Wrap(2, 2, 0))?;
        if mode == CellMode::Tinted {
            commands.request(CellCommand::Filter(1, 1, 0))?;
            let global = commands.global_light();
            if !global.into_iter().all(f32::is_finite) {
                return Err(ContractError::InvalidInput);
            }
            let no_add = self.record.mode == 3 && commands.no_add_signed();
            let combiner = tinted_combiner(&self.record, global, no_add);
            if !combiner
                .environment
                .rgb
                .into_iter()
                .chain([combiner.environment.alpha])
                .all(f32::is_finite)
            {
                return Err(ContractError::InvalidInput);
            }
            commands.request(CellCommand::TexColour(combiner.environment))?;
            commands.request(CellCommand::Combiner {
                bytes: combiner.bytes,
                decoded: combiner.decode()?,
                stage: 0,
            })?;
        }
        let placement = self.placement.as_ref().ok_or(ContractError::InvalidInput)?;
        if placement.flat {
            commands.request(CellCommand::Flat {
                primitive: 1,
                format: 10,
                vertices: placement.vertices,
            })?;
        } else {
            commands.request(CellCommand::Terrain {
                cell: self.record.cell.map(|v| v as u8),
                corners: self.corners(),
                alpha: (f64::from(self.record.alpha) * f64::from(f32::from_bits(0x3b80_8081))) as f32,
                forced: context.terrain_mode == 8,
            })?;
        }
        Ok(1)
    }
    /// Compatibility signature. The owner now determines the native strip;
    /// caller-supplied historical layouts no longer determine placement or UVs.
    pub fn new(record: CellRecord, _layout: CellCornerLayout) -> Self {
        Self::from_record(record)
    }

    pub fn from_record(record: CellRecord) -> Self {
        Self {
            record,
            layout: CellCornerLayout::native(),
            placement: None,
            texture: None,
            texture_alpha: false,
            zoom: None,
            rotation: None,
            corners: CellCornerLayout::native_texture_corners(record.orientation, record.mirror),
            basic: BasicModel::default(),
        }
    }

    pub fn record(&self) -> &CellRecord {
        &self.record
    }
    pub fn placement(&self) -> Option<&CellPlacement> {
        self.placement.as_ref()
    }
    pub fn texture(&self) -> Option<&T> {
        self.texture.as_ref()
    }
    pub fn texture_has_alpha(&self) -> bool {
        self.texture_alpha
    }
    pub fn zoom(&self) -> Option<Zoom> {
        self.zoom
    }
    pub fn corners(&self) -> [Vec2; 4] {
        self.corners
    }

    pub fn place<S: CellPlacementSource, L: Lighting>(
        &mut self,
        source: &mut S,
        lighting: &mut L,
    ) -> Result<(), ContractError> {
        self.place_geometry(source)?;
        if let Some(p) = self.placement.as_mut().filter(|p| p.flat) {
            p.colours = Some(p.positions.map(|v| lighting.at_corner(v, [0., 1., 0.])));
        }
        Ok(())
    }

    fn place_geometry<S: CellPlacementSource>(&mut self, source: &mut S) -> Result<(), ContractError> {
        if self.record.cell.into_iter().any(|v| !(0..=255).contains(&v)) {
            return Err(ContractError::InvalidInput);
        }
        let lot = source.lot_at(self.record.cell);
        let flat = lot.is_some_and(|sample| sample.flags & 1 != 0) && !source.bypass_flat();
        let heights = if flat {
            [lot.expect("flat implies a lot sample").altitude; 4]
        } else {
            source.terrain_corners(self.record.cell)
        };
        if heights.into_iter().any(|v| !v.is_finite()) {
            return Err(ContractError::InvalidInput);
        }
        // LM8: multiplication in the existing f32 coordinate domain, without
        // a guessed native i32 multiply/overflow policy.
        let [x, z] = self.record.cell.map(|v| v as f32 * CELL_METRES);
        let positions = std::array::from_fn(|i| {
            [
                x + self.layout.positions[i][0] * CELL_METRES,
                heights[i],
                z + self.layout.positions[i][1] * CELL_METRES,
            ]
        });
        let colours = self.placement.as_ref().and_then(|p| p.colours);
        let bytes = self
            .placement
            .as_ref()
            .map(|p| p.vertices.map(|v| v.colour))
            .unwrap_or([[0; 4]; 4]);
        let low = heights.into_iter().fold(f32::INFINITY, f32::min);
        let high = heights.into_iter().fold(f32::NEG_INFINITY, f32::max);
        let min = [x + 0.1, low - 2.0, z + 0.1];
        let mut max = [x + CELL_METRES - 0.1, high + 2.0, z + CELL_METRES - 0.1];
        for i in 0..3 {
            if max[i] <= min[i] {
                max[i] = (f64::from(min[i]) + f64::from(f32::from_bits(0x3dcc_cccd))) as f32;
            }
        }
        if !min.into_iter().chain(max).all(f32::is_finite) {
            return Err(ContractError::InvalidInput);
        }
        self.placement = Some(CellPlacement {
            flat,
            positions,
            colours,
            vertices: std::array::from_fn(|i| WallVertex {
                position: positions[i],
                colour: bytes[i],
                uv: self.corners()[i],
            }),
            bounds: Bounds { min, max },
        });
        Ok(())
    }

    pub fn update_vertices<S: CellPlacementSource, L: WallLighting>(
        &mut self,
        geometry: bool,
        colours: bool,
        source: &mut S,
        light: &mut L,
    ) -> Result<(), ContractError> {
        if geometry {
            self.place_geometry(source)?;
        }
        if colours {
            if let Some(p) = self.placement.as_mut().filter(|p| p.flat) {
                for v in &mut p.vertices {
                    let c = light.components(v.position, [0., 1., 0.]);
                    v.colour = [c[2] as u8, c[1] as u8, c[0] as u8, self.record.alpha];
                }
            }
        }
        Ok(())
    }

    pub fn load_texture<F: TextureFactory<Binding = T>>(
        &mut self,
        zoom: Zoom,
        factory: &mut F,
    ) -> Result<bool, ContractError> {
        self.zoom = Some(zoom);
        if self.record.texture_key == 0 {
            self.texture = None;
            self.texture_alpha = false;
            return Ok(true); // A disabled key is a successful null binding.
        }
        let instance = self.record.texture_key.wrapping_add(zoom.value());
        let key = |instance| TextureKey::Cell {
            type_id: CELL_TEXTURE_TYPE,
            group: CELL_TEXTURE_GROUP,
            instance,
        };
        let replacement = load_with_fallback(factory, Some(key(instance)), key(self.record.texture_key));
        self.texture_alpha = replacement.as_ref().is_some_and(Texture::has_alpha);
        if self
            .texture
            .as_ref()
            .zip(replacement.as_ref())
            .is_some_and(|(old, new)| old.same_binding(new))
        {
            drop(replacement); // Same pointer: release the extra acquisition.
        } else {
            self.texture = replacement;
        }
        self.zoom = Some(zoom);
        Ok(self.texture.is_some())
    }

    pub fn sort_value(&self) -> u32 {
        self.texture.as_ref().map_or(self.record.texture_key, Texture::id)
    }

    pub fn set_texture<F: TextureFactory<Binding = T>>(
        &mut self,
        record: CellRecord,
        zoom: Zoom,
        factory: &mut F,
    ) -> Result<(), ContractError> {
        self.record = record;
        self.load_texture(zoom, factory)?;
        CellCornerLayout::update_native_corners(&mut self.corners, record.orientation, record.mirror);
        let corners = self.corners();
        if let Some(p) = self.placement.as_mut() {
            for (v, uv) in p.vertices.iter_mut().zip(corners) {
                v.uv = uv;
            }
        }
        Ok(())
    }

    pub fn draw<F: TextureFactory<Binding = T>>(
        &mut self,
        context: CellDrawContext,
        factory: &mut F,
    ) -> Result<Option<CellDraw<'_, T>>, ContractError> {
        if context.highlighted {
            return Ok(None);
        }
        if self.zoom != Some(context.zoom) {
            self.load_texture(context.zoom, factory)?;
        }
        let Some(texture) = self.texture.as_ref() else {
            return Ok(None);
        };
        let mode = cell_mode(self.record.mode, self.record.alpha, self.texture_alpha);
        if self.record.mode != 255 && self.record.mode > 3 {
            return Err(ContractError::InvalidInput);
        }
        if mode != CellMode::Opaque && !context.translucent_pass {
            return Ok(None);
        }
        let placement = self.placement.as_ref().ok_or(ContractError::InvalidInput)?;
        let corners = self.corners();
        let geometry = if placement.flat {
            CellGeometryDraw::FlatStrip { placement, corners }
        } else {
            CellGeometryDraw::TerrainQuad {
                cell: self.record.cell,
                corners,
                alpha: self.record.alpha,
                forced: context.terrain_mode == 8,
            }
        };
        Ok(Some(CellDraw {
            texture,
            mode,
            state: match mode {
                CellMode::Opaque => DrawState::OPAQUE,
                CellMode::Alpha => DrawState::ALPHA,
                CellMode::Tinted => DrawState::TINTED,
            },
            clamp: true,
            filter: if mode == CellMode::Tinted {
                Some(TextureFilter::Linear)
            } else {
                None
            },
            combiner: if mode == CellMode::Tinted {
                Some(tinted_combiner(
                    &self.record,
                    context.global_light,
                    context.no_add_signed,
                ))
            } else {
                None
            },
            geometry,
        }))
    }
}
