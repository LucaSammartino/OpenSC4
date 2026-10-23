// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The publicly specified subset of `cSTEOverlayManager` (overlay-manager.md).
//!
//! This is a core implementation, not a replacement registration. In particular,
//! OV1/OV2 leave placement and model-footprint construction unspecified. Add and
//! re-placement operations report those gaps instead of taking computed rectangles,
//! matrices, disc parts or model shadows from a host. No same-owner logic is delegated.
//! Slot 20 follows the corrected public contract: odd input only ORs self-lighting;
//! even input never clears it and this call never invalidates terrain.
//!
//! Rust `Arc`s represent owned references; there is no interior mutability in
//! getters. The eventual ABI adapter must provide synchronization around mutations.

mod records;
mod render;

pub use records::{CellRect, Handle, OverlaySnapshot, Placement, TextureScale};
pub use render::{BlendFactor, DepthFunction, DrawState, FrameOptions, RenderSelection, DRAW_STATES};

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
    NotInitialized,
    AlreadyInitialized,
    MissingTerrain,
    MissingFactory,
    InvalidTerrain,
    InvalidHandle,
    Gap(ContractGap),
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
    fn selection_byte(&self) -> u8 { 1 }
    fn subscribe(&self, message: u32);
    fn unsubscribe(&self, message: u32);
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geometry {
    pub cells: [u32; 2],
    pub cell_scale: f32,
    pub metres: [f32; 2],
}

struct Resources {
    view: Arc<dyn ViewProvider>,
    factory: Arc<dyn TextureFactory>,
    terrain: Arc<dyn TerrainProvider>,
    shadow_binding: Option<Arc<dyn TextureBinding>>,
}

/// `None` is a dynamic manager; static managers index internal half-open cells.
type CellIndex = BTreeMap<[u32; 2], Vec<u32>>;

#[derive(Default)]
pub struct OverlayManager {
    resources: Option<Resources>,
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
        Self { shadow_quality: 2, selection_byte: 1, ..Self::default() }
    }

    /// Â§1. Acquire into temporaries first: failure owns no lasting references.
    /// [choice, OV4] Reject double Init and invalid dimensions atomically. The
    /// spec does not give failure returns, repeat-Init or missing-resource policy.
    pub fn init(&mut self, view: Arc<dyn ViewProvider>, dynamic: bool, water: bool) -> Result<(), Error> {
        if self.is_initialized() {
            return Err(Error::AlreadyInitialized);
        }
        let terrain = view.terrain().ok_or(Error::MissingTerrain)?;
        let factory = view.texture_factory().ok_or(Error::MissingFactory)?;
        let cells = terrain.cells();
        let width = terrain.cell_width();
        let metres = [cells[0] as f32 * width, cells[1] as f32 * width];
        if cells.contains(&0)
            || !width.is_finite()
            || width <= 0.0
            || metres.iter().any(|v| !v.is_finite())
            || !(1.0 / width).is_finite()
        {
            return Err(Error::InvalidTerrain);
        }
        // The binding's key is public; its factory slot is not (OV4). Do not
        // silently select slot 5 or 6. Full shadow drawing remains gated on OV8.
        let shadow_binding = None;
        self.shadow_quality = view.shadow_quality();
        self.shadow_debug = view.shadow_debug();
        self.geometry = Some(Geometry {
            cells,
            cell_scale: 1.0 / width,
            metres,
        });
        self.dynamic = dynamic;
        self.water = water;
        self.cell_index = (!dynamic).then(BTreeMap::new);
        view.subscribe(CHANGE_MESSAGE);
        self.resources = Some(Resources {
            view,
            factory,
            terrain,
            shadow_binding,
        });
        Ok(())
    }

    /// Â§1. Unsubscribe while the view is alive, then drop owned references.
    pub fn shutdown(&mut self) {
        if let Some(resources) = self.resources.take() {
            resources.view.unsubscribe(CHANGE_MESSAGE);
            drop(resources.view);
            drop(resources.factory);
            drop(resources.terrain);
            drop(resources.shadow_binding);
        }
        self.overlays.clear();
        self.handles.clear();
        self.groups.clear();
        self.cell_index = None;
        self.changed.clear();
        self.geometry = None;
        // [choice, OV4] These core diagnostics reset on shutdown; no original
        // constructor defaults or post-shutdown getters are specified.
        self.decals_drawn = 0;
        self.shadows_drawn = 0;
    }

    pub fn is_initialized(&self) -> bool {
        self.resources.is_some()
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
        _source: Arc<dyn TextureSource>,
        _placement: Placement,
        _a: u32,
        _b: u32,
    ) -> Result<Handle, Error> {
        self.require_init()?;
        Err(Error::Gap(ContractGap::PlacementAndAddArgumentsOv1))
    }

    pub fn add_decal_binding(
        &mut self,
        binding: Option<Arc<dyn TextureBinding>>,
        _placement: Placement,
        _a: u32,
        _b: u32,
    ) -> Result<Handle, Error> {
        self.require_init()?;
        if binding.is_none() {
            return Ok(Handle::NONE);
        }
        Err(Error::Gap(ContractGap::PlacementAndAddArgumentsOv1))
    }

    /// Slots 5 and 6. Factory selection is specified, final argument forwarding
    /// is not. Do not invent the fourth argument to slot 4 (OV1).
    pub fn add_decal_key(
        &mut self,
        key: u32,
        _placement: Placement,
        _a: u32,
        lookup: BindingLookup,
    ) -> Result<Handle, Error> {
        let resources = self.resources.as_ref().ok_or(Error::NotInitialized)?;
        if lookup == BindingLookup::ShadowSlot11 {
            return Err(Error::Gap(ContractGap::ShadowAndModelConstructionOv2));
        }
        match resources.factory.binding(key, lookup) {
            None => Ok(Handle::NONE),
            Some(_) => Err(Error::Gap(ContractGap::PlacementAndAddArgumentsOv1)),
        }
    }

    pub fn add_round_decal(&mut self, _key: u32, _placement: Placement) -> Result<Handle, Error> {
        self.require_init()?;
        Err(Error::Gap(ContractGap::RoundPartitionOv9))
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
        let Some(binding) = binding else { return Ok(Handle::NONE) };
        let index = self.overlays.insert(Overlay {
            primary, cells, flags: flag::SHOWN | extra_flags,
            alpha: 1.0, color: [1.0; 3], draw_state: 0,
            texture_scale: None, binding: Some(binding), source: None,
        });
        if let Some(cell_index) = &mut self.cell_index {
            for cell in cells.cells() {
                cell_index.entry(cell).or_default().insert(0, index);
            }
            self.changed.push(cells);
        }
        Ok(Handle(index as i32))
    }

    pub fn add_city_decal(&mut self, key: u32) -> Result<Handle, Error> {
        self.require_init()?;
        let cells = self.geometry.ok_or(Error::NotInitialized)?.cells;
        // Selected positive signed-dimension domain; no claim about native zero division.
        if cells[0] == 0 || cells[0] > i32::MAX as u32 {
            return Err(Error::InvalidTerrain);
        }
        let binding = self.resources.as_ref().unwrap().factory.binding(key, BindingLookup::Slot5);
        let mut matrix = [0.0; 16];
        matrix[5] = (2.0_f64 / f64::from(cells[0] as i32)) as f32;
        matrix[10] = 1.0;
        matrix[15] = 1.0;
        self.add_overlay_for_rect(binding, CellRect { min: [0, 0], max: cells }, matrix, 0x12)
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

    pub fn on_message(&mut self, message: u32) -> Result<(), Error> {
        if message == CHANGE_MESSAGE {
            self.require_init()?;
            let view = &self.resources.as_ref().unwrap().view;
            let settings = [view.shadow_quality(), view.shadow_debug()];
            let selection = view.selection_byte();
            self.handle_settings_message(message, settings, selection);
        }
        Ok(())
    }

    /// Raw individual settings values supplied by the render-properties boundary (§5).
    pub fn handle_settings_message(&mut self, message: u32, integers: [i32; 2], selection: u8) -> bool {
        if message == CHANGE_MESSAGE {
            self.shadow_quality = integers[0];
            self.shadow_debug = integers[1];
            self.selection_byte = selection;
        }
        true
    }

    pub fn set_position(&mut self, handle: Handle, _xz: [f32; 2]) -> Result<(), Error> {
        if handle == Handle::NONE || !handle.is_indirect() {
            return Ok(());
        }
        self.handles.get(handle.index()).ok_or(Error::InvalidHandle)?;
        // OV1: the positions of x/z within the nine floats are not public.
        Err(Error::Gap(ContractGap::PlacementAndAddArgumentsOv1))
    }

    pub fn set_placement(&mut self, handle: Handle, _placement: Placement) -> Result<(), Error> {
        if handle == Handle::NONE || !handle.is_indirect() {
            return Ok(());
        }
        self.target(handle)?;
        // Do not partially update the saved placement when re-placement cannot
        // be computed. Direct-handle behavior is also open (OV5).
        Err(Error::Gap(ContractGap::HandlePlacementOv5))
    }

    pub fn get_placement(&self, handle: Handle, out: &mut Placement) -> Result<(), Error> {
        if handle == Handle::NONE {
            return Ok(());
        }
        if !handle.is_indirect() {
            return Ok(());
        }
        *out = self.handles.get(handle.index()).ok_or(Error::InvalidHandle)?.placement;
        Ok(())
    }

    pub fn remove(&mut self, handle: Handle) -> Result<(), Error> {
        if handle == Handle::NONE {
            return Ok(());
        }
        let target = self.target(handle)?;
        let indices = self.indices(target)?;
        for index in indices {
            let rectangle = self.overlays.get(index).ok_or(Error::InvalidHandle)?.cells;
            self.unindex(index, rectangle);
            self.changed.push(rectangle);
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
            before != overlay.flags
        })
    }

    pub fn set_alpha(&mut self, handle: Handle, alpha: f32) -> Result<(), Error> {
        let indices = self.handle_indices(handle)?;
        // OV6: threshold, rounding and last-redrawn versus last-stored comparison
        // are unknown. Endpoint writes and dynamic writes do not need the step.
        if !self.dynamic
            && alpha != 0.0
            && alpha != 1.0
            && indices
                .iter()
                .any(|&i| self.overlays.get(i).is_some_and(|o| o.alpha != alpha))
        {
            return Err(Error::Gap(ContractGap::StaticAlphaStepOv6));
        }
        self.edit(handle, true, |overlay| {
            let changed = overlay.alpha != alpha;
            overlay.alpha = alpha;
            changed
        })
    }

    pub fn set_color(&mut self, handle: Handle, rgb: [f32; 3]) -> Result<(), Error> {
        self.edit(handle, true, |overlay| {
            let changed = overlay.color != rgb;
            overlay.color = rgb;
            changed
        })
    }

    pub fn set_draw_state(&mut self, handle: Handle, index: u8) -> Result<(), Error> {
        self.edit(handle, true, |overlay| {
            let changed = overlay.draw_state != index;
            overlay.draw_state = index;
            changed
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

    pub fn set_texture_scale(&mut self, handle: Handle, size: f32, offset: f32) -> Result<(), Error> {
        self.edit(handle, true, |overlay| {
            overlay.texture_scale = Some(TextureScale::new(size, offset));
            overlay.flags |= flag::TEXTURE_MATRIX;
            true
        })
    }

    /// Slot 22's field identity is not specified (OV4).
    pub fn get_field(&self, handle: Handle) -> Result<Option<u32>, Error> {
        if handle == Handle::NONE {
            return Ok(None);
        }
        self.target(handle)?;
        Err(Error::Gap(ContractGap::GetterAndDefaultsOv4))
    }

    pub fn alpha(&self, handle: Handle) -> Result<f32, Error> {
        if handle == Handle::NONE {
            return Ok(0.0);
        }
        Ok(self.single(handle)?.alpha)
    }

    /// None leaves an eventual output parameter unchanged; only alpha's null
    /// return is specified. Group scalar getters are unresolved (OV4).
    pub fn color(&self, handle: Handle) -> Result<Option<[f32; 3]>, Error> {
        self.optional_single(handle).map(|o| o.map(|o| o.color))
    }

    pub fn draw_state(&self, handle: Handle) -> Result<Option<u8>, Error> {
        self.optional_single(handle).map(|o| o.map(|o| o.draw_state))
    }

    pub fn shown(&self, handle: Handle) -> Result<Option<bool>, Error> {
        self.optional_single(handle)
            .map(|o| o.map(|o| o.flags & flag::SHOWN != 0))
    }

    pub fn stats_string(&self) -> String {
        format!("{} decals, {} shadows", self.decals_drawn, self.shadows_drawn)
    }

    /// Core diagnostics, not slot 22 or a construction entry point.
    pub fn snapshot(&self, handle: Handle) -> Result<Option<OverlaySnapshot>, Error> {
        self.optional_single(handle).map(|o| o.map(Overlay::snapshot))
    }

    pub fn take_changed_rectangles(&mut self) -> Vec<CellRect> {
        std::mem::take(&mut self.changed)
    }

    fn require_init(&self) -> Result<(), Error> {
        self.resources.as_ref().map(|_| ()).ok_or(Error::NotInitialized)
    }

    fn target(&self, handle: Handle) -> Result<Target, Error> {
        if handle.is_indirect() {
            self.handles
                .get(handle.index())
                .map(|r| r.target)
                .ok_or(Error::InvalidHandle)
        } else if handle != Handle::NONE && self.overlays.get(handle.index()).is_some() {
            Ok(Target::Overlay(handle.index()))
        } else {
            Err(Error::InvalidHandle)
        }
    }

    fn indices(&self, target: Target) -> Result<Vec<u32>, Error> {
        let indices = match target {
            Target::Overlay(index) => vec![index],
            Target::Group(index) => self
                .groups
                .get(index)
                .ok_or(Error::InvalidHandle)?
                .iter()
                .flatten()
                .copied()
                .collect(),
        };
        // [choice, OV5] Malformed/stale records fail before any mutation.
        if indices.iter().any(|&i| self.overlays.get(i).is_none()) {
            return Err(Error::InvalidHandle);
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
            Target::Overlay(i) => self.overlays.get(i).ok_or(Error::InvalidHandle),
            Target::Group(_) => Err(Error::Gap(ContractGap::GetterAndDefaultsOv4)),
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
                if apply(overlay) && invalidate {
                    self.changed.push(overlay.cells);
                }
            }
        }
        Ok(())
    }

    fn unindex(&mut self, overlay: u32, rectangle: CellRect) {
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
        self.shutdown();
    }
}

#[cfg(test)]
mod tests;
