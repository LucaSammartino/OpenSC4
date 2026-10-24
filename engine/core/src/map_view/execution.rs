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
    /// Fresh low word of CRT wall-clock milliseconds, sampled at Update tail.
    fn clock_ms(&mut self) -> u32 {
        0
    }
    fn child_bitmap_present(&mut self) -> Result<bool, OwnerError> {
        Ok(true)
    }
    fn bitmap_size(&mut self) -> Result<[usize; 2], OwnerError> {
        Ok([1, 1])
    }
    fn overlay_register(&mut self) -> Result<(), OwnerError> {
        Ok(())
    }
    fn overlay_replace(&mut self, _cells: &MarkedCells) -> Result<(), OwnerError> {
        Ok(())
    }
    fn orientation(&mut self) -> u32 {
        0
    }
    fn incident_icon(&mut self, _key: [u32; 3], _flag: u32) -> Result<IncidentIcon, OwnerError> {
        Err(OwnerError::Foreign(12))
    }
    fn release_incident_icon(&mut self, _handle: u64) -> Result<(), OwnerError> {
        Ok(())
    }
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
    pub(super) fn raw(&self) -> (ValueType, Values<'_>) {
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
    fn developer_debug_entries(&mut self, _source: u32) -> Result<Option<Vec<[i32; 3]>>, OwnerError> {
        Err(OwnerError::Foreign(11))
    }
    fn developer_present(&mut self, _source: u32) -> Result<bool, OwnerError> {
        Ok(true)
    }
    fn lots(&mut self) -> Result<Vec<MapLot>, OwnerError> {
        Ok(Vec::new())
    }
    fn incidents(&mut self) -> Result<Vec<MapIncident>, OwnerError> {
        Ok(Vec::new())
    }
    fn zone_grid(&mut self) -> Result<RawMapGrid, OwnerError> {
        Ok(RawMapGrid {
            width: 0,
            height: 0,
            cell_size: [16.0; 2],
            values: GridValues::U8(Vec::new()),
        })
    }
    fn zone_occupant(&mut self, _x: usize, _z: usize) -> Result<Option<u64>, OwnerError> {
        Ok(None)
    }
    fn occupant_position(&mut self, _occupant: u64) -> Result<[f32; 3], OwnerError> {
        Ok([0.0; 3])
    }
    fn retain_occupant(&mut self, _occupant: u64) -> Result<(), OwnerError> {
        Ok(())
    }
    fn release_building_list(&mut self) -> Result<(), OwnerError> {
        Ok(())
    }
    fn developer_words(&mut self, _source: u32, _slot: u8) -> Result<WordGrid, OwnerError> {
        Err(OwnerError::Foreign(15))
    }
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
    fn city_world_height(&mut self) -> Result<f32, OwnerError> {
        Err(OwnerError::Foreign(94))
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
    pub(super) fn bool_or(&self, id: u32, absent: bool) -> Result<bool, Unsupported> {
        if self.find(id)?.is_some() {
            self.bool_field(id)
        } else {
            Ok(absent)
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
            hide_empty_bases_and_excluded_decals: self.bool_or(0x4a0b_47e2, false)?,
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
        if raw.find(RAMP)?.is_some() {
            self.palettes = Some(build_palettes(
                &self.config.as_ref().expect("stored selection").ramp_pairs,
            )?);
        }
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
        let mut buildings = if (1..=9).contains(&kind) {
            simulators.buildings(highlight_request(kind)?)?
        } else {
            Vec::new()
        };
        if kind == 6 {
            let zones = simulators.zone_grid()?;
            if let GridValues::U8(values) = zones.values {
                for x in 0..zones.width {
                    for z in 0..zones.height {
                        if values.get(x * zones.height + z) == Some(&0x0e) {
                            if let Some(id) = simulators.zone_occupant(x, z)? {
                                buildings.push(RawBuilding {
                                    id,
                                    kind: BuildingKind::GarbageBuilding,
                                    coverage: Coverage::default(),
                                });
                            }
                        }
                    }
                }
            }
        }
        let collection = collect_highlights(kind, &buildings)?;
        let mut candidates = Vec::new();
        for candidate in collection.candidates {
            if candidate.id == 0 {
                continue;
            }
            simulators.retain_occupant(candidate.id)?;
            candidates.push(HighlightRecord {
                occupant: Some(candidate.id),
                position: simulators.occupant_position(candidate.id)?,
                radius: match candidate.radius {
                    HighlightRadius::Value(v) => v,
                    _ => 0.0,
                },
                effect: None,
            });
        }
        if (1..=9).contains(&kind) {
            simulators.release_building_list()?;
        }
        self.reconcile_buildings(candidates, renderer)?;
        Ok(None)
    }

    /// Signed-pointer ordered multiset merge. Equal non-null keys retain old geometry/effects.
    pub fn reconcile_buildings(
        &mut self,
        mut candidates: Vec<HighlightRecord>,
        renderer: &mut impl Renderer,
    ) -> Result<bool, OwnerError> {
        let key = |record: &HighlightRecord| record.occupant.unwrap_or(0) as u32 as i32;
        candidates.sort_by_key(key);
        self.retained_highlights.sort_by_key(key);
        let mut old = std::mem::take(&mut self.retained_highlights).into_iter().peekable();
        let mut new = candidates.into_iter().peekable();
        let mut changed = false;
        while old.peek().is_some() || new.peek().is_some() {
            match (old.peek(), new.peek()) {
                (Some(a), Some(b)) if a.occupant.is_some() && key(a) == key(b) => {
                    let temporary = new.next().expect("peeked");
                    if let Some(occupant) = temporary.occupant {
                        renderer.release_occupant(occupant)?;
                    }
                    self.retained_highlights.push(old.next().expect("peeked"));
                }
                (Some(a), Some(b)) if key(a) < key(b) => {
                    old.next().expect("peeked").remove(renderer)?;
                    changed = true;
                }
                (Some(_), None) => {
                    old.next().expect("peeked").remove(renderer)?;
                    changed = true;
                }
                _ => {
                    let mut record = new.next().expect("new key");
                    record.add(&self.highlight_style.name, renderer)?;
                    self.retained_highlights.push(record);
                    changed = true;
                }
            }
        }
        self.dirty |= changed;
        Ok(changed)
    }

    pub fn draw_with(
        &mut self,
        renderer: &mut impl Renderer,
        simulators: &mut impl Simulators,
    ) -> Result<OwnerUpdate, OwnerError> {
        if !self.active {
            return Ok(OwnerUpdate::Inactive);
        }
        self.dirty = false;
        if self.overlay_enabled && !self.overlay.registered() {
            renderer.overlay_register()?;
            self.overlay.plan(true, &[])?;
        }
        if !renderer.child_bitmap_present()? {
            self.last_redraw_ms = u64::from(renderer.clock_ms());
            return Ok(OwnerUpdate::DrawNothing);
        }
        if self.source >= 64 && self.source != 76 && !simulators.developer_present(self.source)? {
            return Ok(OwnerUpdate::DrawNothing);
        }
        let result = self.draw_body(renderer, simulators)?;
        if let Some(handle) = self.custom_view {
            renderer.update_custom_view(handle)?;
        }
        self.last_redraw_ms = u64::from(renderer.clock_ms());
        Ok(result)
    }

    fn draw_body(
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
            SourceSelection::DrawNothing => {
                let [width, height] = renderer.bitmap_size()?;
                let mut pixels = vec![0; width * height];
                self.decorate_bitmap(width, height, &mut pixels, renderer, simulators)?;
                renderer.primitive(RenderCommand::Bitmap {
                    class: 0x2073_2180,
                    iid: 0xc470_d325,
                    bits: 32,
                    width,
                    height,
                    cell_size: [16.0; 2],
                    own_cells: false,
                    pixels: &pixels,
                })?;
                return Ok(OwnerUpdate::DrawNothing);
            }
            SourceSelection::TrafficVolumeQV2 => return Err(Unsupported::TrafficQV2.into()),
            SourceSelection::Grid(request) => request.value_type,
            SourceSelection::Developer { grid, .. } => match grid {
                DeveloperGrid::CapacityPercentage => ValueType::I16,
                DeveloperGrid::Unsigned16Slot14 => ValueType::U16,
                DeveloperGrid::Signed16Slot13 => ValueType::I16,
                DeveloperGrid::DebugEntriesTimes12 => ValueType::I16,
            },
        };
        let debug_grid = matches!(
            request,
            SourceSelection::Developer {
                grid: DeveloperGrid::DebugEntriesTimes12,
                ..
            }
        );
        let mut grid = if debug_grid {
            let geometry_request = match request {
                SourceSelection::Developer {
                    developer_type, layer, ..
                } => SourceSelection::Developer {
                    developer_type,
                    layer,
                    grid: DeveloperGrid::Signed16Slot13,
                },
                _ => unreachable!(),
            };
            let geometry = simulators.grid(geometry_request)?;
            let Some(entries) = simulators.developer_debug_entries(self.source)? else {
                return self.draw_empty(renderer, simulators);
            };
            let mut values = vec![
                0i16;
                geometry
                    .width
                    .checked_mul(geometry.height)
                    .ok_or(Unsupported::GridGeometry)?
            ];
            for [x, z, value] in entries {
                if x < 0 || z < 0 || x as usize >= geometry.width || z as usize >= geometry.height {
                    return Err(Unsupported::GridGeometry.into());
                }
                values[x as usize * geometry.height + z as usize] = value.wrapping_mul(12) as i16;
            }
            RawMapGrid {
                values: GridValues::I16(values),
                ..geometry
            }
        } else if matches!(
            request,
            SourceSelection::Developer {
                grid: DeveloperGrid::CapacityPercentage,
                ..
            }
        ) {
            let population = simulators.developer_words(self.source, 15)?;
            let capacity = simulators.developer_words(self.source, 14)?;
            capacity_percentage(&population, &capacity, simulators.city_cells()?)?
        } else {
            simulators.grid(request)?
        };
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
        // Word conversion feeds the ordinary byte drawing/interpolation path.
        if matches!(grid.values, GridValues::U16(_) | GridValues::I16(_)) {
            let indices = (0..values.len())
                .map(|i| values.index(i, self.factors))
                .collect::<Result<Vec<_>, _>>()?;
            grid.values = GridValues::U8(indices);
        }
        if let Some(config) = &self.config {
            if config.interpolate && !debug_grid {
                let cells = simulators.city_cells()?;
                if cells.contains(&0) {
                    return Err(Unsupported::GridGeometry.into());
                }
                if grid.width < cells[0] || grid.height < cells[1] {
                    grid = resample_grid(&grid, cells)?;
                    grid.cell_size = [
                        simulators.city_world_width()? / cells[0] as f32,
                        simulators.city_world_height()? / cells[1] as f32,
                    ];
                }
            }
            if let Some(mask) = config.block_by_lot {
                if let GridValues::I8(values) = &grid.values {
                    grid.values = GridValues::U8(values.iter().copied().map(i8_index).collect());
                }
                block_lots(&mut grid, mask, &simulators.lots()?)?;
            }
        }
        if self.overlay_enabled {
            // Source 9's separate damaged-road rule remains unsettled QV2.
            if self.source == 9 {
                return Err(Unsupported::TrafficMarkersQV4.into());
            }
            let cells = marked_cells_scaled(
                &grid,
                &self.palettes.as_ref().ok_or(Unsupported::RampGeneration)?.straight,
                self.factors,
            )?;
            renderer.overlay_replace(&cells)?;
        }
        let count = grid.width * grid.height;
        let (_, values) = grid.values.raw();
        let palette = &self.palettes.as_ref().ok_or(Unsupported::RampGeneration)?.premultiplied;
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(count)
            .map_err(|_| Unsupported::HelperAllocation)?;
        pixels.resize(count, 0);
        emit_colors_scaled(values, palette, &mut pixels, self.factors)?;
        if self.source == 4 {
            let city = simulators.city_cells()?;
            let turn = renderer.orientation().wrapping_neg() & 3;
            for incident in simulators.incidents()? {
                let icon = renderer.incident_icon(incident.key, 1)?;
                let center = incident_center(incident.cell, grid.width, city[0]);
                // Each false lock skips this incident only; always release its created reference.
                let blend = if renderer.acquire_bitmap(0x8040, grid.width, grid.height)?.is_some() {
                    let source_lock = renderer.acquire_bitmap(0x8040, icon.width, icon.height)?;
                    let result = if source_lock.is_some() {
                        blit_incident(&mut pixels, [grid.width, grid.height], &icon, center, turn)
                    } else {
                        Ok(())
                    };
                    if source_lock.is_some() {
                        renderer.release_bitmap()?;
                    }
                    renderer.release_bitmap()?;
                    result
                } else {
                    Ok(())
                };
                renderer.release_incident_icon(icon.handle)?;
                blend?;
            }
        }
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
        Ok(OwnerUpdate::Drawn)
    }

    fn draw_empty(
        &mut self,
        renderer: &mut impl Renderer,
        simulators: &mut impl Simulators,
    ) -> Result<OwnerUpdate, OwnerError> {
        let [width, height] = renderer.bitmap_size()?;
        let mut pixels = vec![0; width.checked_mul(height).ok_or(Unsupported::GridGeometry)?];
        self.decorate_bitmap(width, height, &mut pixels, renderer, simulators)?;
        renderer.primitive(RenderCommand::Bitmap {
            class: 0x2073_2180,
            iid: 0xc470_d325,
            bits: 32,
            width,
            height,
            cell_size: [16.0; 2],
            own_cells: false,
            pixels: &pixels,
        })?;
        Ok(OwnerUpdate::DrawNothing)
    }

    pub fn tick_with(
        &mut self,
        now_ms: u64,
        renderer: &mut impl Renderer,
        simulators: &mut impl Simulators,
    ) -> Result<OwnerTick, OwnerError> {
        let elapsed = u64::from((now_ms as u32).wrapping_sub(self.last_redraw_ms as u32));
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
