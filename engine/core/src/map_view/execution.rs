// SPDX-License-Identifier: GPL-3.0-or-later
//! Owner orchestration; foreign interfaces expose primitives and raw data only.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnerError {
    Unsupported(Unsupported),
    /// Invalid adapter/service input, distinct from normally returning false.
    Foreign(u32),
    MissingExemplar,
    MissingMessageServer,
}

impl From<Unsupported> for OwnerError {
    fn from(value: Unsupported) -> Self {
        Self::Unsupported(value)
    }
}

pub trait Resources {
    fn exemplar(&mut self, key: ExemplarKey) -> Result<Option<RawExemplar>, OwnerError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OccupantFilter {
    pub types: Vec<u32>,
    pub list_present: bool,
    pub exclude_listed: bool,
    pub hide_empty_bases_and_excluded_decals: bool,
}

impl OccupantFilter {
    pub fn includes_type(&self, occupant_type: u32) -> bool {
        if self.exclude_listed {
            !self.types.contains(&occupant_type)
        } else {
            !self.list_present || self.types.contains(&occupant_type)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderSetting {
    OccupantFilter(OccupantFilter),
    TerrainMode { property: u32, mode: u32 },
    ForceEmptyLotBaseTextures(bool),
    ZoneOption(u8),
}

pub enum RenderCommand<'a> {
    Setting(RenderSetting),
    ClearMap,
    /// The adapter creates/presents the native bitmap; all pixels are core-made.
    Bitmap {
        class: u32,
        iid: u32,
        bits: u8,
        width: usize,
        height: usize,
        cell_size: [f32; 2],
        own_cells: bool,
        pixels: &'a [u32],
    },
}

pub trait Renderer {
    fn render(&mut self, command: RenderCommand<'_>) -> Result<(), OwnerError>;
    fn primitive(&mut self, command: RenderCommand<'_>) -> Result<bool, OwnerError> {
        self.render(command)?;
        Ok(true)
    }
    fn update_custom_view(&mut self, _handle: u64) -> Result<bool, OwnerError> {
        Ok(true)
    }
    fn create_visual_effect(&mut self, _name: &str) -> Result<Option<u64>, OwnerError> {
        Err(OwnerError::Foreign(7))
    }
    fn visual_effect(&mut self, _effect: VisualEffect) -> Result<bool, OwnerError> {
        Err(OwnerError::Foreign(8))
    }
    fn highlight_occupant(&mut self, _occupant: u64, _highlight: bool, _argument: u32) -> Result<bool, OwnerError> {
        Err(OwnerError::Foreign(17))
    }
    fn release_occupant(&mut self, _occupant: u64) -> Result<(), OwnerError> {
        Err(OwnerError::Foreign(0))
    }
    /// Buffer acquisition flags are always 0x8040. None is an ordinary failure.
    fn acquire_bitmap(&mut self, _flags: u32, width: usize, height: usize) -> Result<Option<ClipRect>, OwnerError> {
        Ok(Some(ClipRect {
            left: 0,
            top: 0,
            right: width as i32,
            bottom: height as i32,
        }))
    }
    fn release_bitmap(&mut self) -> Result<(), OwnerError> {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum GridValues {
    U8(Vec<u8>),
    I8(Vec<i8>),
    U16(Vec<u16>),
    I16(Vec<i16>),
    F32(Vec<f32>),
}

impl GridValues {
    fn raw(&self) -> (ValueType, Values<'_>) {
        match self {
            Self::U8(v) => (ValueType::U8, Values::U8(v)),
            Self::I8(v) => (ValueType::I8, Values::I8(v)),
            Self::U16(v) => (ValueType::U16, Values::U16(v)),
            Self::I16(v) => (ValueType::I16, Values::I16(v)),
            Self::F32(v) => (ValueType::F32, Values::F32(v)),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RawMapGrid {
    pub width: usize,
    pub height: usize,
    pub cell_size: [f32; 2],
    pub values: GridValues,
}

pub trait Simulators {
    fn grid(&mut self, request: SourceSelection) -> Result<RawMapGrid, OwnerError>;
    /// City slots 93/94, used only to decide whether resampling is needed.
    fn city_cells(&mut self) -> Result<[usize; 2], OwnerError>;
    fn buildings(&mut self, request: HighlightRequest) -> Result<Vec<RawBuilding>, OwnerError>;
    /// Crime slot 8 outputs; None means false with no output writes.
    fn crime_hotspot(&mut self) -> Result<Option<CrimeHotspot>, OwnerError> {
        Err(OwnerError::Foreign(8))
    }
    fn terrain_altitude(&mut self, _x: f32, _z: f32) -> Result<f32, OwnerError> {
        Err(OwnerError::Foreign(13))
    }
    fn occupant_cells(&mut self, _occupant: u64) -> Result<[i32; 4], OwnerError> {
        Err(OwnerError::Foreign(11))
    }
    fn city_world_width(&mut self) -> Result<f32, OwnerError> {
        Err(OwnerError::Foreign(93))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnerUpdate {
    Inactive,
    Cleared,
    DrawNothing,
    Drawn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnerSelection {
    Inactive,
    CurrentView,
    Selected {
        /// QV7 does not prevent the unconditional selection redraw or hooking.
        highlight_boundary: Option<Unsupported>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnerTick {
    pub highlight_boundary: Option<Unsupported>,
    pub due: bool,
    pub update: Option<Result<OwnerUpdate, OwnerError>>,
}

impl RawExemplar {
    fn bool_or(&self, id: u32, absent: bool) -> Result<bool, Unsupported> {
        match self.find(id)? {
            Some(RawValue::Bool(value)) => Ok(*value),
            Some(_) => Err(Unsupported::MalformedProperty(id)),
            None => Ok(absent),
        }
    }

    pub fn render_settings(&self) -> Result<Vec<RenderSetting>, Unsupported> {
        let types = match self.find(OCCUPANT_TYPES)? {
            None => Vec::new(),
            Some(RawValue::U32List(types)) => types.iter().copied().filter(|v| *v != 0).collect(),
            Some(_) => return Err(Unsupported::MalformedProperty(OCCUPANT_TYPES)),
        };
        let mut settings = vec![RenderSetting::OccupantFilter(OccupantFilter {
            types,
            list_present: self.find(OCCUPANT_TYPES)?.is_some(),
            exclude_listed: self.bool_or(0x4a0b_47e1, false)?,
            // QV6: unlike E1 and F0, E2 has no documented absence default.
            hide_empty_bases_and_excluded_decals: self.bool_field(0x4a0b_47e2)?,
        })];
        for property in [TERRAIN_MODE, TERRAIN_MODE_SECOND] {
            settings.push(RenderSetting::TerrainMode {
                property,
                mode: self.terrain_mode(property)?,
            });
        }
        settings.push(RenderSetting::ForceEmptyLotBaseTextures(
            self.bool_or(0x4a0b_47f0, false)?,
        ));
        if let Some(value) = self.find(0x4a0b_47e6)? {
            let RawValue::U8(value) = value else {
                return Err(Unsupported::MalformedProperty(0x4a0b_47e6));
            };
            settings.push(RenderSetting::ZoneOption(*value));
        }
        Ok(settings)
    }
}

impl MapView {
    /// §1: latch survives missing server; ordinary false primitives are ignored.
    pub fn init_with(&mut self, windows: &mut impl WindowSystem) -> Result<(), OwnerError> {
        for effect in self.init()?.effects {
            if matches!(effect, Effect::Subscribe(_)) && !windows.message_server_present() {
                return Err(OwnerError::MissingMessageServer);
            }
            windows.primitive(WindowEffect::Lifecycle(effect))?;
        }
        Ok(())
    }

    pub fn owned_palettes(&self) -> Option<&OwnedPalettes> {
        self.palettes.as_ref()
    }

    pub fn set_overlay_enabled(&mut self, enabled: bool) {
        self.overlay_enabled = enabled;
    }

    pub fn select_with(
        &mut self,
        view: u32,
        resources: &mut impl Resources,
        windows: &mut impl WindowSystem,
        renderer: &mut impl Renderer,
        simulators: &mut impl Simulators,
    ) -> Result<OwnerSelection, OwnerError> {
        let start = self.begin_selection(view);
        for effect in start.effects {
            windows.primitive(WindowEffect::Lifecycle(effect))?;
        }
        let key = match start.outcome {
            SelectionOutcome::Inactive => return Ok(OwnerSelection::Inactive),
            SelectionOutcome::CurrentView => return Ok(OwnerSelection::CurrentView),
            SelectionOutcome::NeedsExemplar(key) => key,
            SelectionOutcome::Unsupported(error) => return Err(error.into()),
            SelectionOutcome::MissingExemplar => return Err(OwnerError::MissingExemplar),
        };
        let raw = resources.exemplar(key)?.ok_or(OwnerError::MissingExemplar)?;
        self.store_selection(&raw)?;
        self.highlight_style = HighlightStyle::from_exemplar(&raw)?;
        self.palettes = Some(build_palettes(
            &self.config.as_ref().expect("stored selection").ramp_pairs,
        )?);
        for setting in raw.render_settings()? {
            renderer.primitive(RenderCommand::Setting(setting))?;
        }
        // Literal property presence creates a foreign class. No pointer is
        // released unless this owner received a successfully created handle.
        if let Some(class) = raw.optional_u32(0x4a0b_47eb)? {
            self.custom_view = windows.create_custom_view(class, 0x8a17_5d6b)?;
            if let Some(handle) = self.custom_view {
                windows.primitive(WindowEffect::InitCustomView(handle))?;
            }
        }
        let highlight_boundary = self.poll_highlights(renderer, simulators)?;
        self.draw_with(renderer, simulators)?;
        if windows.query_control(0x4203, 0xca31_8385)? {
            windows.primitive(WindowEffect::ChildFlag {
                child: 0x4203,
                iid: 0xca31_8385,
                value: raw.bool_field(CHILD_FLAG)?,
            })?;
        }
        self.current_view = Some(view);
        for effect in self.hook_update_messages()? {
            windows.primitive(WindowEffect::Lifecycle(effect))?;
        }
        Ok(OwnerSelection::Selected { highlight_boundary })
    }

    pub fn poll_highlights(
        &mut self,
        renderer: &mut impl Renderer,
        simulators: &mut impl Simulators,
    ) -> Result<Option<Unsupported>, OwnerError> {
        let kind = self.config.as_ref().map_or(0, |config| config.highlight_kind);
        if kind == 5 {
            let hotspot = simulators.crime_hotspot()?;
            let mut candidate = if let Some(hotspot) = hotspot {
                let x = ((f64::from(hotspot.x) + 0.5) * 16.0) as f32;
                let z = ((f64::from(hotspot.z) + 0.5) * 16.0) as f32;
                let y = simulators.terrain_altitude(x, z)?;
                let mut record = HighlightRecord {
                    occupant: None,
                    position: [x, y, z],
                    radius: 100.0,
                    effect: None,
                };
                record.effect = record.create_effect(&self.highlight_style.name, 100.0, renderer)?;
                Some(record)
            } else {
                None
            };
            let changed = candidate.is_some() || !self.retained_highlights.is_empty();
            let previous = std::mem::take(&mut self.retained_highlights);
            for record in &previous {
                record.stop(renderer)?;
            }
            if let Some(mut record) = candidate.take() {
                record.add(&self.highlight_style.name, renderer)?;
                self.retained_highlights.push(record);
            }
            for record in previous {
                record.release(renderer)?;
            }
            self.dirty |= changed;
            return Ok(None);
        }
        self.dirty |= !self.retained_highlights.is_empty();
        for record in self.retained_highlights.drain(..) {
            record.remove(renderer)?;
        }
        if kind == 0 {
            // Only clearing retained records is settled; other kind-0 state is QV7.
            return Ok(Some(Unsupported::HighlightKind(0)));
        }
        let request = match highlight_request(kind) {
            Ok(request) => request,
            Err(error) => return Ok(Some(error)),
        };
        let buildings = simulators.buildings(request)?;
        Ok(self.update_highlights(&buildings).err())
    }

    pub fn draw_with(
        &mut self,
        renderer: &mut impl Renderer,
        simulators: &mut impl Simulators,
    ) -> Result<OwnerUpdate, OwnerError> {
        if !self.active {
            return Ok(OwnerUpdate::Inactive);
        }
        let layer = self.config.as_ref().map_or(1, |config| config.layer);
        let request = select_source(self.source, layer, self.developer_debug)?;
        let expected = match request {
            SourceSelection::Clear => {
                renderer.render(RenderCommand::ClearMap)?;
                return Ok(OwnerUpdate::Cleared);
            }
            SourceSelection::DrawNothing => return Ok(OwnerUpdate::DrawNothing),
            SourceSelection::TrafficVolumeQV2 => return Err(Unsupported::TrafficQV2.into()),
            SourceSelection::Grid(request) => request.value_type,
            SourceSelection::Developer { grid, .. } => match grid {
                DeveloperGrid::Unsigned16Slot14 => ValueType::U16,
                DeveloperGrid::Signed16Slot13 => ValueType::I16,
                DeveloperGrid::DebugEntriesTimes12 => return Err(Unsupported::DeveloperDebugLayout.into()),
            },
        };
        let grid = simulators.grid(request)?;
        let count = grid
            .width
            .checked_mul(grid.height)
            .filter(|n| *n > 0)
            .ok_or(Unsupported::GridGeometry)?;
        let (actual, values) = grid.values.raw();
        if actual != expected {
            return Err(Unsupported::GridType.into());
        }
        if values.len() != count {
            return Err(Unsupported::BufferLength.into());
        }
        if grid.cell_size.iter().any(|v| !v.is_finite() || *v <= 0.0) {
            return Err(Unsupported::GridGeometry.into());
        }
        if let Some(config) = &self.config {
            if config.interpolate {
                let cells = simulators.city_cells()?;
                if cells.contains(&0) {
                    return Err(Unsupported::GridGeometry.into());
                }
                if grid.width < cells[0] || grid.height < cells[1] {
                    return Err(Unsupported::GridInterpolation.into());
                }
            }
            if config.block_by_lot.is_some() {
                return Err(Unsupported::LotBlockingQV4.into());
            }
        }
        if self.source == 4 {
            return Err(Unsupported::CrimeMarkersQV4.into());
        }
        if self.overlay_enabled {
            return Err(if self.source == 9 {
                Unsupported::TrafficMarkersQV4
            } else {
                Unsupported::OverlayGeometryQV10
            }
            .into());
        }
        let palette = &self.palettes.as_ref().ok_or(Unsupported::RampGeneration)?.premultiplied;
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(count)
            .map_err(|_| Unsupported::HelperAllocation)?;
        pixels.resize(count, 0);
        emit_colors(values, palette, &mut pixels)?;
        self.decorate_bitmap(grid.width, grid.height, &mut pixels, renderer, simulators)?;
        renderer.primitive(RenderCommand::Bitmap {
            class: 0x2073_2180,
            iid: 0xc470_d325,
            bits: 32,
            width: grid.width,
            height: grid.height,
            cell_size: grid.cell_size,
            own_cells: matches!(request, SourceSelection::Grid(GridRequest { own_cells: true, .. })),
            pixels: &pixels,
        })?;
        if let Some(handle) = self.custom_view {
            renderer.update_custom_view(handle)?;
        }
        Ok(OwnerUpdate::Drawn)
    }

    pub fn tick_with(
        &mut self,
        now_ms: u64,
        renderer: &mut impl Renderer,
        simulators: &mut impl Simulators,
    ) -> Result<OwnerTick, OwnerError> {
        let elapsed = now_ms
            .checked_sub(self.last_redraw_ms)
            .ok_or(Unsupported::ClockRegression)?;
        self.notify(ANIMATION_TICK, None)?;
        let highlight_boundary = self.poll_highlights(renderer, simulators)?;
        let interval = if self.source == 76 {
            self.dirty = true;
            250
        } else {
            1000
        };
        let due = self.dirty && elapsed >= interval;
        let update = if due {
            let result = self.draw_with(renderer, simulators);
            // QV9: only a successful bitmap presentation acknowledges a redraw.
            // Empty/failed/unsupported paths retain both dirty and the clock.
            if result == Ok(OwnerUpdate::Drawn) {
                self.dirty = false;
                self.last_redraw_ms = now_ms;
            }
            Some(result)
        } else {
            None
        };
        Ok(OwnerTick {
            highlight_boundary,
            due,
            update,
        })
    }
}
