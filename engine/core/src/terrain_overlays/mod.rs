// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The publicly specified subset of `cSTEOverlayManager` (overlay-manager.md).
//!
//! This is a core implementation, not a replacement registration. In particular,
//! Settled owner calculations use raw resource dimensions/settings and retain references.
//! The selected numeric domain is explicit; incomplete rendering remains a named gap.
//! Slot 20 follows the corrected public contract: odd input only ORs self-lighting;
//! even input never clears it and this call never invalidates terrain.
//!
//! Rust `Arc`s represent owned references; there is no interior mutability in
//! getters. The eventual ABI adapter must provide synchronization around mutations.

mod model;
mod placement;
mod records;
mod render;
mod round;

pub use records::{CellRect, Handle, OverlaySnapshot, Placement, TextureScale};
pub use render::{
    BlendFactor, DepthFunction, DrawState, FrameOptions, RenderSelection, DRAW_STATES,
};

use records::{Arena, HandleRecord, Overlay, Target};
use std::collections::BTreeMap;
use std::sync::Arc;

pub const IID: u32 = 0x895D_6544;
pub const CHANGE_MESSAGE: u32 = 0xC9F7_75BB;
pub const SHADOW_TEXTURE_KEY: u32 = 0xAA59_2B77;

pub mod flag {
    pub const SHOWN: u32 = 1;
    pub const REPEAT: u32 = 1 << 1;
    pub const PART: u32 = 1 << 4;
    pub const TEXTURE_MATRIX: u32 = 1 << 5;
    pub const SELF_LIT: u32 = 1 << 6;
    pub const SNAPSHOT: u32 = 1 << 7;
}

/// Named gaps, not guessed compatibility behavior. See the corresponding question.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractGap {
    PlacementAndAddArgumentsOv1,
    ShadowAndModelConstructionOv2,
    ChangeMessageOv3,
    GetterAndDefaultsOv4,
    HandlePlacementOv5,
    StaticAlphaStepOv6,
    WorldTextureTransformOv7,
    DrawingDetailsOv8,
    RoundPartitionOv9,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Unsupported(Unsupported),
    NotInitialized,
    Gap(ContractGap),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    PlacementNumericDomain,
    InvalidResourceDimensions,
    GroupTextureScale,
    ModelNumericDomain,
    MissingLighting,
    VisibleRows,
    /// Portable unsafe-service boundaries preserve Init's published prefix.
    MissingFactory,
    MissingTerrain,
    /// Portable validation is not asserted as native behavior.
    InvalidTerrain,
    InvalidHandle,
}

/// External resources only; no overlay placement or draw results are provided.
pub trait TextureBinding: Send + Sync {
    fn size(&self) -> [u32; 2];
}

pub trait TextureSource: Send + Sync {
    fn size(&self) -> [u32; 2];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingLookup {
    Slot5,
    Slot6,
    ShadowSlot11,
}

pub trait TextureFactory: Send + Sync {
    fn binding(&self, key: u32, lookup: BindingLookup) -> Option<Arc<dyn TextureBinding>>;
}

pub trait TerrainProvider: Send + Sync {
    fn cells(&self) -> [u32; 2];
    fn cell_width(&self) -> f32;
}

pub trait ViewProvider: Send + Sync {
    fn terrain(&self) -> Option<Arc<dyn TerrainProvider>>;
    fn texture_factory(&self) -> Option<Arc<dyn TextureFactory>>;
    fn shadow_quality(&self) -> i32;
    fn shadow_debug(&self) -> i32;
    fn model_lighting_slot9(&self) -> Option<Arc<dyn ModelLighting>> {
        None
    }
    fn selection_byte(&self) -> u8 {
        1
    }
    fn subscribe(&self, message: u32);
    fn unsubscribe(&self, message: u32);
}

/// Borrowed opaque backing only. No source-geometry or descriptor layout is asserted.
#[derive(Clone, Copy)]
pub struct GeometryToken<'a>(pub &'a [u8]);
#[derive(Clone, Copy)]
pub struct ProjectionDescriptor<'a>(pub &'a [u8]);

/// Explicit foreign lighting results from §3.7, not a native binding claim.
pub trait ModelLighting: Send + Sync {
    fn projection_slot17(
        &self,
        n: i32,
        source: GeometryToken<'_>,
        pairs: &mut [[f32; 2]],
        descriptor: ProjectionDescriptor<'_>,
        primary: &mut [f32; 16],
    ) -> bool;
    fn terrain_rect_slot18(
        &self,
        n: i32,
        source: GeometryToken<'_>,
        world_box: [f32; 6],
    ) -> Option<CellRect>;
    fn direction_slot13(&self) -> [f32; 3];
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geometry {
    pub cells: [u32; 2],
    pub cell_scale: f32,
    pub metres: [f32; 2],
}

#[derive(Default)]
struct Resources {
    view: Option<Arc<dyn ViewProvider>>,
    factory: Option<Arc<dyn TextureFactory>>,
    terrain: Option<Arc<dyn TerrainProvider>>,
}

/// `None` is a dynamic manager; static managers index internal half-open cells.
type CellIndex = BTreeMap<[u32; 2], Vec<u32>>;

pub struct OverlayManager {
    collection_stamp: u16,
    resources: Resources,
    shadow_binding: Option<Arc<dyn TextureBinding>>,
    initialized: bool,
    geometry: Option<Geometry>,
    dynamic: bool,
    water: bool,
    shadow_quality: i32,
    shadow_debug: i32,
    selection_byte: u8,
    overlays: Arena<Overlay>,
    handles: Arena<HandleRecord>,
    groups: Arena<[Option<u32>; 8]>,
    cell_index: Option<CellIndex>,
    changed: Vec<CellRect>,
    decals_drawn: u32,
    shadows_drawn: u32,
}

impl OverlayManager {
    pub fn new() -> Self {
        Self {
            collection_stamp: 0,
            resources: Resources::default(),
            shadow_binding: None,
            initialized: false,
            geometry: None,
            dynamic: false,
            water: false,
            shadow_quality: 2,
            shadow_debug: 0,
            selection_byte: 1,
            overlays: Arena::default(),
            handles: Arena::default(),
            groups: Arena::default(),
            cell_index: None,
            changed: Vec::new(),
            decals_drawn: 0,
            shadows_drawn: 0,
        }
    }

    /// §1.1 mutating protocol; unsupported services preserve the documented prefix.
    pub fn init(
        &mut self,
        view: Arc<dyn ViewProvider>,
        dynamic: bool,
        water: bool,
    ) -> Result<(), Error> {
        replace_reference(&mut self.resources.view, view.clone());
        self.dynamic = dynamic;
        self.water = water;
        self.initialized = true;
        let factory = view
            .texture_factory()
            .ok_or(Error::Unsupported(Unsupported::MissingFactory))?;
        replace_reference(&mut self.resources.factory, factory.clone());
        let terrain = view
            .terrain()
            .ok_or(Error::Unsupported(Unsupported::MissingTerrain))?;
        replace_reference(&mut self.resources.terrain, terrain.clone());
        let cells = terrain.cells();
        let width = terrain.cell_width();
        let metres = [cells[0] as f32 * width, cells[1] as f32 * width];
        if cells.contains(&0)
            || !width.is_finite()
            || width <= 0.0
            || metres.iter().any(|v| !v.is_finite())
            || !(1.0 / width).is_finite()
        {
            return Err(Error::Unsupported(Unsupported::InvalidTerrain));
        }
        self.geometry = Some(Geometry {
            cells,
            cell_scale: 1.0 / width,
            metres,
        });
        self.decals_drawn = 0;
        self.shadows_drawn = 0;
        drop(self.shadow_binding.take());
        self.shadow_binding = factory.binding(SHADOW_TEXTURE_KEY, BindingLookup::Slot5);
        if !dynamic {
            self.cell_index = Some(BTreeMap::new());
            self.collection_stamp = 0;
        }
        view.subscribe(CHANGE_MESSAGE);
        self.shadow_quality = view.shadow_quality();
        self.shadow_debug = view.shadow_debug();
        Ok(())
    }

    /// Â§1. Unsubscribe while the view is alive, then drop owned references.
    pub fn shutdown(&mut self) {
        if let Some(view) = &self.resources.view {
            view.unsubscribe(CHANGE_MESSAGE);
        }
        drop(self.resources.view.take());
        drop(self.resources.terrain.take());
        drop(self.resources.factory.take());
        self.overlays.clear();
        self.handles.clear();
        self.initialized = false;
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn geometry(&self) -> Option<Geometry> {
        self.geometry
    }

    pub fn is_dynamic(&self) -> bool {
        self.dynamic
    }

    pub fn is_water(&self) -> bool {
        self.water
    }

    pub fn shadow_settings(&self) -> [i32; 2] {
        [self.shadow_quality, self.shadow_debug]
    }

    /// Slots 3/4. No external provider supplies this owner's missing geometry.
    pub fn add_decal_source(
        &mut self,
        source: Arc<dyn TextureSource>,
        position: [f32; 2],
        width: f32,
        turns: f32,
    ) -> Result<Handle, Error> {
        self.require_init()?;
        let dimensions = source.size();
        self.add_ordinary(
            None,
            Some(source),
            Placement([
                position[0],
                position[1],
                width,
                turns,
                1.0,
                1.0,
                1.0,
                0.0,
                0.0,
            ]),
            dimensions,
        )
    }

    pub fn add_decal_binding(
        &mut self,
        binding: Option<Arc<dyn TextureBinding>>,
        position: [f32; 2],
        width: f32,
        turns: f32,
    ) -> Result<Handle, Error> {
        self.require_init()?;
        let Some(binding) = binding else {
            return Ok(Handle::NONE);
        };
        let dimensions = binding.size();
        self.add_ordinary(
            Some(binding),
            None,
            Placement([
                position[0],
                position[1],
                width,
                turns,
                1.0,
                1.0,
                1.0,
                0.0,
                0.0,
            ]),
            dimensions,
        )
    }

    fn add_ordinary(
        &mut self,
        binding: Option<Arc<dyn TextureBinding>>,
        source: Option<Arc<dyn TextureSource>>,
        placement: Placement,
        dimensions: [u32; 2],
    ) -> Result<Handle, Error> {
        if dimensions.contains(&0) {
            return Err(Error::Unsupported(Unsupported::InvalidResourceDimensions));
        }
        let aspect = (f64::from(dimensions[1]) / f64::from(dimensions[0])) as f32;
        // Ordinary adds consume centre/width/turns and construct the other five defaults.
        let [cx, cz, w, turns, ..] = placement.0;
        let placement = Placement([cx, cz, w, turns, 1.0, 1.0, 1.0, 0.0, 0.0]);
        let (primary, cells, repeat, _) = placement::calculate(
            placement,
            aspect,
            self.geometry.ok_or(Error::NotInitialized)?,
        )?;
        let index = self.overlays.insert(Overlay {
            stamp: 0,
            secondary: None,
            primary,
            cells,
            flags: 0x11 | if repeat { 2 } else { 0 },
            alpha: 1.0,
            color: [1.0; 3],
            draw_state: 0,
            texture_scale: None,
            binding,
            source,
        });
        self.index_and_mark(index, cells);
        let h = self.handles.insert(HandleRecord {
            placement,
            aspect,
            target: Target::Overlay(index),
        });
        Ok(Handle((h | 0x8000_0000) as i32))
    }

    fn index_and_mark(&mut self, overlay: u32, cells: CellRect) {
        if self.dynamic {
            return;
        }
        if let Some(index) = &mut self.cell_index {
            // Native insertion traverses X then Z, prepending each individual entry.
            for x in cells.min[0]..cells.max[0] {
                for z in cells.min[1]..cells.max[1] {
                    index.entry([x, z]).or_default().insert(0, overlay);
                }
            }
        }
        self.mark(cells);
    }

    fn mark(&mut self, cells: CellRect) {
        if !self.dynamic && !cells.is_empty() {
            self.changed.push(cells);
        }
    }

    /// Slots 5 and 6 resolve a raw key, then invoke the ordinary binding add.
    pub fn add_decal_key(
        &mut self,
        key: u32,
        position: [f32; 2],
        width: f32,
        turns: f32,
        lookup: BindingLookup,
    ) -> Result<Handle, Error> {
        self.require_init()?;
        let resources = &self.resources;
        if lookup == BindingLookup::ShadowSlot11 {
            return Err(Error::Gap(ContractGap::ShadowAndModelConstructionOv2));
        }
        match resources
            .factory
            .as_ref()
            .ok_or(Error::Unsupported(Unsupported::MissingFactory))?
            .binding(key, lookup)
        {
            None => Ok(Handle::NONE),
            Some(binding) => self.add_decal_binding(Some(binding), position, width, turns),
        }
    }

    pub fn add_round_decal(
        &mut self,
        key: u32,
        position: [f32; 2],
        width: f32,
        turns: f32,
    ) -> Result<Handle, Error> {
        self.require_init()?;
        let Some(binding) = self
            .resources
            .factory
            .as_ref()
            .ok_or(Error::Unsupported(Unsupported::MissingFactory))?
            .binding(key, BindingLookup::Slot5)
        else {
            return Ok(Handle::NONE);
        };
        let dimensions = binding.size();
        if dimensions.contains(&0) {
            return Err(Error::Unsupported(Unsupported::InvalidResourceDimensions));
        }
        let aspect = (f64::from(dimensions[1]) / f64::from(dimensions[0])) as f32;
        let p = Placement([
            position[0],
            position[1],
            width,
            turns,
            1.0,
            1.0,
            1.0,
            0.0,
            0.0,
        ]);
        let (primary, _, repeat, bounds) = placement::calculate(p, aspect, self.geometry.unwrap())?;
        let rectangles = round::partition(bounds, self.geometry.unwrap().cells)?;
        let mut parts = [None; 8];
        for (part, cells) in parts.iter_mut().zip(rectangles) {
            let index = self.overlays.insert(Overlay {
                stamp: 0,
                secondary: None,
                primary,
                cells,
                flags: 0x11 | if repeat { 2 } else { 0 },
                alpha: 1.0,
                color: [1.0; 3],
                draw_state: 0,
                texture_scale: None,
                binding: Some(binding.clone()),
                source: None,
            });
            *part = Some(index);
        }
        let group = self.groups.insert(parts);
        let handle = self.handles.insert(HandleRecord {
            placement: p,
            aspect,
            target: Target::Group(group),
        });
        for (part, cells) in parts.into_iter().zip(rectangles) {
            self.index_and_mark(part.unwrap(), cells);
        }
        Ok(Handle((handle | 0x8000_0000) as i32))
    }

    /// OV2 raw rectangle helper. This is an owner operation, not a host result.
    pub fn add_overlay_for_rect(
        &mut self,
        binding: Option<Arc<dyn TextureBinding>>,
        cells: CellRect,
        primary: [f32; 16],
        extra_flags: u32,
    ) -> Result<Handle, Error> {
        self.require_init()?;
        let Some(binding) = binding else {
            return Ok(Handle::NONE);
        };
        let index = self.overlays.insert(Overlay {
            stamp: 0,
            secondary: None,
            primary,
            cells,
            flags: flag::SHOWN | extra_flags,
            alpha: 1.0,
            color: [1.0; 3],
            draw_state: 0,
            texture_scale: None,
            binding: Some(binding),
            source: None,
        });
        self.index_and_mark(index, cells);
        Ok(Handle(index as i32))
    }

    pub fn add_city_decal(&mut self, key: u32) -> Result<Handle, Error> {
        self.require_init()?;
        let cells = self.geometry.ok_or(Error::NotInitialized)?.cells;
        // Selected positive signed-dimension domain; no claim about native zero division.
        if cells[0] == 0 || cells[0] > i32::MAX as u32 {
            return Err(Error::Unsupported(Unsupported::InvalidTerrain));
        }
        let binding = self
            .resources
            .factory
            .as_ref()
            .ok_or(Error::Unsupported(Unsupported::MissingFactory))?
            .binding(key, BindingLookup::Slot5);
        let mut matrix = [0.0; 16];
        matrix[5] = (2.0_f64 / f64::from(cells[0] as i32)) as f32;
        matrix[10] = 1.0;
        matrix[15] = 1.0;
        self.add_overlay_for_rect(
            binding,
            CellRect {
                min: [0, 0],
                max: cells,
            },
            matrix,
            0x12,
        )
    }

    /// Slots 9â€“11 have no published complete argument contract. Only slot 10's
    /// quality-zero early return can be implemented without inventing inputs.
    pub fn check_shadow_add(&self, factory_form: bool) -> Result<Handle, Error> {
        self.require_init()?;
        if factory_form && self.shadow_quality == 0 {
            Ok(Handle::NONE)
        } else {
            Err(Error::Gap(ContractGap::ShadowAndModelConstructionOv2))
        }
    }

    /// Six public model inputs. Lighting tokens remain borrowed and opaque;
    /// ordinary fallback bounds and every secondary coefficient are owned here.
    pub fn add_model_shadow(
        &mut self,
        binding: Option<Arc<dyn TextureBinding>>,
        n: i32,
        source: GeometryToken<'_>,
        pairs: &mut [[f32; 2]],
        world_box: [f32; 6],
        descriptor: ProjectionDescriptor<'_>,
    ) -> Result<Handle, Error> {
        self.require_init()?;
        if self.shadow_quality == 0 {
            return Ok(Handle::NONE);
        }
        if n <= 0 || n as usize > pairs.len() {
            return Err(Error::Unsupported(Unsupported::ModelNumericDomain));
        }
        let view = self.resources.view.as_ref().ok_or(Error::NotInitialized)?;
        let lighting = view
            .model_lighting_slot9()
            .ok_or(Error::Unsupported(Unsupported::MissingLighting))?;
        let mut primary = [0.0; 16];
        if !lighting.projection_slot17(
            n,
            source,
            &mut pairs[..n as usize],
            descriptor,
            &mut primary,
        ) {
            return Ok(Handle::NONE);
        }
        let cells = if view.shadow_quality() >= 5 {
            let Some(cells) = lighting.terrain_rect_slot18(n, source, world_box) else {
                return Ok(Handle::NONE);
            };
            cells
        } else {
            model::fallback(
                lighting.direction_slot13(),
                world_box,
                self.geometry.ok_or(Error::NotInitialized)?,
            )?
        };
        let handle = self.add_overlay_for_rect(binding, cells, primary, 8)?;
        if handle == Handle::NONE {
            return Ok(handle);
        }
        // Insertion/MarkChanged has already occurred. Unsupported arithmetic
        // preserves that allocated prefix; no native atomic failure is inferred.
        let secondary = model::secondary(primary, &pairs[..n as usize])?;
        self.overlays.get_mut(handle.index()).unwrap().secondary = Some(secondary);
        Ok(handle)
    }

    pub fn on_message(&mut self, message: u32) -> Result<(), Error> {
        if message == CHANGE_MESSAGE {
            self.require_init()?;
            let view = self.resources.view.as_ref().ok_or(Error::NotInitialized)?;
            let settings = [view.shadow_quality(), view.shadow_debug()];
            let selection = view.selection_byte();
            self.handle_settings_message(message, settings, selection);
        }
        Ok(())
    }

    /// Raw individual settings values supplied by the render-properties boundary (§5).
    pub fn handle_settings_message(
        &mut self,
        message: u32,
        integers: [i32; 2],
        selection: u8,
    ) -> bool {
        if message == CHANGE_MESSAGE {
            self.shadow_quality = integers[0];
            self.shadow_debug = integers[1];
            self.selection_byte = selection;
        }
        true
    }

    pub fn set_position(&mut self, handle: Handle, xz: [f32; 2]) -> Result<(), Error> {
        if handle == Handle::NONE || !handle.is_indirect() {
            return Ok(());
        }
        let mut p = self
            .handles
            .get(handle.index())
            .ok_or(Error::Unsupported(Unsupported::InvalidHandle))?
            .placement;
        let tolerance = f64::from(f32::from_bits(0x38d1_b717));
        if (f64::from(xz[0]) - f64::from(p.0[0])).abs() < tolerance
            && (f64::from(xz[1]) - f64::from(p.0[1])).abs() < tolerance
        {
            return Ok(());
        }
        p.0[0] = xz[0];
        p.0[1] = xz[1];
        self.set_placement(handle, p)
    }

    pub fn set_placement(&mut self, handle: Handle, placement: Placement) -> Result<(), Error> {
        if handle == Handle::NONE || !handle.is_indirect() {
            return Ok(());
        }
        let record = self
            .handles
            .get(handle.index())
            .ok_or(Error::Unsupported(Unsupported::InvalidHandle))?;
        let target = record.target;
        let (matrix, cells, repeat, bounds) = placement::calculate(
            placement,
            record.aspect,
            self.geometry.ok_or(Error::NotInitialized)?,
        )?;
        if let Target::Group(group) = target {
            let parts = self
                .groups
                .get(group)
                .copied()
                .ok_or(Error::Unsupported(Unsupported::InvalidHandle))?;
            let rectangles = round::partition(bounds, self.geometry.unwrap().cells)?;
            self.handles.get_mut(handle.index()).unwrap().placement = placement;
            for (part, cells) in parts.into_iter().zip(rectangles) {
                self.update_placed_part(
                    part.ok_or(Error::Unsupported(Unsupported::InvalidHandle))?,
                    matrix,
                    cells,
                    repeat,
                )?;
            }
            return Ok(());
        }
        let Target::Overlay(index) = target else {
            return Err(Error::Gap(ContractGap::RoundPartitionOv9));
        };
        self.handles.get_mut(handle.index()).unwrap().placement = placement;
        self.update_placed_part(index, matrix, cells, repeat)
    }

    fn update_placed_part(
        &mut self,
        index: u32,
        matrix: [f32; 16],
        cells: CellRect,
        repeat: bool,
    ) -> Result<(), Error> {
        let old = self
            .overlays
            .get(index)
            .ok_or(Error::Unsupported(Unsupported::InvalidHandle))?
            .cells;
        if old != cells {
            self.unindex(index, old);
            self.mark(old);
        }
        let overlay = self.overlays.get_mut(index).unwrap();
        overlay.primary = matrix;
        overlay.cells = cells;
        overlay.flags = (overlay.flags & !flag::REPEAT) | if repeat { flag::REPEAT } else { 0 };
        if old != cells {
            self.index_and_mark(index, cells);
        } else {
            self.mark(cells);
        }
        Ok(())
    }

    pub fn get_placement(&self, handle: Handle, out: &mut Placement) -> Result<(), Error> {
        if handle == Handle::NONE {
            return Ok(());
        }
        if !handle.is_indirect() {
            return Ok(());
        }
        *out = self
            .handles
            .get(handle.index())
            .ok_or(Error::Unsupported(Unsupported::InvalidHandle))?
            .placement;
        Ok(())
    }

    pub fn remove(&mut self, handle: Handle) -> Result<(), Error> {
        if handle == Handle::NONE {
            return Ok(());
        }
        let target = self.target(handle)?;
        let indices = self.indices(target)?;
        if handle.is_indirect() {
            self.handles.remove(handle.index());
        }
        for index in indices {
            let rectangle = self
                .overlays
                .get(index)
                .ok_or(Error::Unsupported(Unsupported::InvalidHandle))?
                .cells;
            self.unindex(index, rectangle);
            self.mark(rectangle);
            self.overlays.remove(index);
        }
        if let Target::Group(index) = target {
            self.groups.remove(index);
        }
        if handle.is_indirect() {
            self.handles.remove(handle.index());
        }
        Ok(())
    }

    pub fn set_shown(&mut self, handle: Handle, shown: bool) -> Result<(), Error> {
        self.edit(handle, true, |overlay| {
            let before = overlay.flags;
            overlay.flags = (before & !flag::SHOWN) | u32::from(shown);
            true
        })
    }

    pub fn set_alpha(&mut self, handle: Handle, alpha: f32) -> Result<(), Error> {
        let dynamic = self.dynamic;
        self.edit(handle, true, |overlay| {
            let accepted = dynamic
                || (overlay.alpha != alpha
                    && (alpha == 0.0
                        || alpha == 1.0
                        || (f64::from(alpha) - f64::from(overlay.alpha)).abs() > 0.05_f64));
            if accepted {
                overlay.alpha = alpha;
            }
            accepted
        })
    }

    pub fn set_color(&mut self, handle: Handle, rgb: [f32; 3]) -> Result<(), Error> {
        self.edit(handle, true, |overlay| {
            overlay.color = rgb;
            true
        })
    }

    pub fn set_draw_state(&mut self, handle: Handle, index: u8) -> Result<(), Error> {
        self.edit(handle, true, |overlay| {
            overlay.draw_state = index;
            true
        })
    }

    /// Slot 20: test the low bit, OR only, and never mark changed (Â§9 correction).
    pub fn enable_self_lighting(&mut self, handle: Handle, on: u32) -> Result<(), Error> {
        self.edit(handle, false, |overlay| {
            if on & 1 != 0 {
                overlay.flags |= flag::SELF_LIT;
            }
            false
        })
    }

    pub fn set_texture_scale(
        &mut self,
        handle: Handle,
        size: f32,
        offset: f32,
    ) -> Result<(), Error> {
        if handle == Handle::NONE {
            return Ok(());
        }
        if let Target::Group(_) = self.target(handle)? {
            return Err(Error::Unsupported(Unsupported::GroupTextureScale));
        }
        // Selected finite nonzero domain; native exceptional FP behavior is unqualified.
        if !size.is_finite() || size == 0.0 || !offset.is_finite() {
            return Err(Error::Unsupported(Unsupported::PlacementNumericDomain));
        }
        let r = 1.0 / f64::from(size);
        let t = (r * f64::from(offset) + 1.0) as f32;
        self.edit(handle, true, |overlay| {
            overlay.primary = [
                r as f32,
                0.0,
                0.0,
                0.0,
                0.0,
                (-r) as f32,
                0.0,
                0.0,
                r as f32,
                0.0,
                1.0,
                0.0,
                0.0,
                t,
                0.0,
                1.0,
            ];
            overlay.texture_scale = Some(TextureScale::new(size, offset));
            overlay.flags |= flag::TEXTURE_MATRIX;
            true
        })
    }

    /// Portable optional wrapper of slot 22's shown bit.
    pub fn get_field(&self, handle: Handle) -> Result<Option<u32>, Error> {
        if handle == Handle::NONE {
            return Ok(None);
        }
        Ok(Some(self.single(handle)?.flags & 1))
    }

    pub fn alpha(&self, handle: Handle) -> Result<f32, Error> {
        if handle == Handle::NONE {
            return Ok(0.0);
        }
        Ok(self.single(handle)?.alpha)
    }

    /// Portable optional diagnostic; the native output form writes zeros for -1.
    pub fn color(&self, handle: Handle) -> Result<Option<[f32; 3]>, Error> {
        self.optional_single(handle).map(|o| o.map(|o| o.color))
    }

    pub fn draw_state(&self, handle: Handle) -> Result<Option<u8>, Error> {
        self.optional_single(handle)
            .map(|o| o.map(|o| o.draw_state))
    }

    pub fn shown(&self, handle: Handle) -> Result<Option<bool>, Error> {
        self.optional_single(handle)
            .map(|o| o.map(|o| o.flags & flag::SHOWN != 0))
    }

    pub fn stats_string(&self) -> String {
        format!(
            "{} decals, {} shadows",
            self.decals_drawn, self.shadows_drawn
        )
    }

    pub fn self_lit(&self, handle: Handle) -> Result<bool, Error> {
        Ok(self
            .optional_single(handle)?
            .is_some_and(|o| o.flags & flag::SELF_LIT != 0))
    }

    pub fn color_into(&self, handle: Handle, out: &mut [f32; 3]) -> Result<(), Error> {
        *out = self.color(handle)?.unwrap_or([0.0; 3]);
        Ok(())
    }

    /// Core diagnostics, not slot 22 or a construction entry point.
    pub fn snapshot(&self, handle: Handle) -> Result<Option<OverlaySnapshot>, Error> {
        self.optional_single(handle)
            .map(|o| o.map(Overlay::snapshot))
    }

    pub fn take_changed_rectangles(&mut self) -> Vec<CellRect> {
        std::mem::take(&mut self.changed)
    }

    fn require_init(&self) -> Result<(), Error> {
        if self.initialized {
            Ok(())
        } else {
            Err(Error::NotInitialized)
        }
    }

    fn target(&self, handle: Handle) -> Result<Target, Error> {
        if handle.is_indirect() {
            self.handles
                .get(handle.index())
                .map(|r| r.target)
                .ok_or(Error::Unsupported(Unsupported::InvalidHandle))
        } else if handle != Handle::NONE && self.overlays.get(handle.index()).is_some() {
            Ok(Target::Overlay(handle.index()))
        } else {
            Err(Error::Unsupported(Unsupported::InvalidHandle))
        }
    }

    fn indices(&self, target: Target) -> Result<Vec<u32>, Error> {
        let indices = match target {
            Target::Overlay(index) => vec![index],
            Target::Group(index) => self
                .groups
                .get(index)
                .ok_or(Error::Unsupported(Unsupported::InvalidHandle))?
                .iter()
                .flatten()
                .copied()
                .collect(),
        };
        // [choice, OV5] Malformed/stale records fail before any mutation.
        if indices.iter().any(|&i| self.overlays.get(i).is_none()) {
            return Err(Error::Unsupported(Unsupported::InvalidHandle));
        }
        Ok(indices)
    }

    fn handle_indices(&self, handle: Handle) -> Result<Vec<u32>, Error> {
        if handle == Handle::NONE {
            Ok(Vec::new())
        } else {
            self.indices(self.target(handle)?)
        }
    }

    fn single(&self, handle: Handle) -> Result<&Overlay, Error> {
        match self.target(handle)? {
            Target::Overlay(i) => self
                .overlays
                .get(i)
                .ok_or(Error::Unsupported(Unsupported::InvalidHandle)),
            Target::Group(i) => self
                .groups
                .get(i)
                .and_then(|g| g[0])
                .and_then(|i| self.overlays.get(i))
                .ok_or(Error::Unsupported(Unsupported::InvalidHandle)),
        }
    }

    fn optional_single(&self, handle: Handle) -> Result<Option<&Overlay>, Error> {
        if handle == Handle::NONE {
            Ok(None)
        } else {
            self.single(handle).map(Some)
        }
    }

    fn edit(
        &mut self,
        handle: Handle,
        invalidate: bool,
        mut apply: impl FnMut(&mut Overlay) -> bool,
    ) -> Result<(), Error> {
        for index in self.handle_indices(handle)? {
            if let Some(overlay) = self.overlays.get_mut(index) {
                if apply(overlay) && invalidate && !self.dynamic && !overlay.cells.is_empty() {
                    self.changed.push(overlay.cells);
                }
            }
        }
        Ok(())
    }

    fn unindex(&mut self, overlay: u32, rectangle: CellRect) {
        if self.dynamic {
            return;
        }
        if let Some(index) = &mut self.cell_index {
            for cell in rectangle.cells() {
                if let Some(entries) = index.get_mut(&cell) {
                    entries.retain(|&i| i != overlay);
                    if entries.is_empty() {
                        index.remove(&cell);
                    }
                }
            }
        }
    }
}

impl Drop for OverlayManager {
    fn drop(&mut self) {
        // Distinct destructor protocol (§1.2), deliberately no unsubscribe.
        self.cell_index = None;
        drop(self.shadow_binding.take());
        self.groups.clear();
        self.handles.clear();
        self.overlays.clear();
        drop(self.resources.factory.take());
        drop(self.resources.view.take());
        drop(self.resources.terrain.take());
    }
}

impl Default for OverlayManager {
    fn default() -> Self {
        Self::new()
    }
}

fn replace_reference<T: ?Sized>(slot: &mut Option<Arc<T>>, value: Arc<T>) {
    if slot.as_ref().is_some_and(|old| Arc::ptr_eq(old, &value)) {
        return;
    }
    // Incoming Arc was acquired first; publish it before releasing the old reference.
    drop(slot.replace(value));
}

#[cfg(test)]
mod tests;
