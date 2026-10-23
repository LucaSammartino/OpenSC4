// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

use super::lifecycle::Event;
use super::retained::{RawTuningSource, ReadArithmetic, RetainedTuning, RippleProcess};
use super::{dimensions, Cell, CellRect, ContractGap, Error, RippleTuning, TerrainProvider, TextureFactory, Zoom};
use std::sync::Arc;
use std::sync::Mutex;

pub const ANIMATION_TICK: u32 = 0x86AD_10EE;
pub const TERRAIN_CHANGED: u32 = 0x278E_6003;
pub const REBUILD_NOW: u32 = 0x8917_1496;
pub const REBUILD_DELAY: u64 = 2_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RippleMessage {
    AnimationTick(u64),
    TerrainChanged(CellRect),
    RebuildNow,
}

impl RippleMessage {
    pub fn id(self) -> u32 {
        match self {
            Self::AnimationTick(_) => ANIMATION_TICK,
            Self::TerrainChanged(_) => TERRAIN_CHANGED,
            Self::RebuildNow => REBUILD_NOW,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RebuildState {
    pending: bool,
    elapsed: u64,
    accumulated_area: u64,
    dirty: Option<CellRect>,
}

impl RebuildState {
    pub fn pending(&self) -> bool {
        self.pending
    }
    pub fn elapsed(&self) -> u64 {
        self.elapsed
    }
    pub fn accumulated_area(&self) -> u64 {
        self.accumulated_area
    }
    pub fn dirty_rectangle(&self) -> Option<CellRect> {
        self.dirty
    }

    pub fn arm(&mut self) {
        // Literal "an armed rebuild happens after"; re-arming doesn't restart
        // its clock. Native repeated-edit/reset behavior remains WR10.
        if !self.pending {
            self.pending = true;
            self.elapsed = 0;
        }
    }

    pub fn tick(&mut self, elapsed: u64, counter: u64, remembered: u64, tuning: &RippleTuning) -> bool {
        self.tick_threshold(elapsed, counter, remembered, tuning.edit_count_threshold())
    }

    fn tick_threshold(&mut self, elapsed: u64, counter: u64, remembered: u64, threshold: u64) -> bool {
        if self.pending {
            self.elapsed = self.elapsed.saturating_add(elapsed);
        } else if counter.saturating_sub(remembered) > threshold {
            // Literal arm at this tick, start counting with the next one; WR10.
            self.arm();
        }
        self.pending && self.elapsed >= REBUILD_DELAY
    }

    /// Returns true when §1.5 requests intersection checking/removal of strips.
    /// Rectangle area means cells (width * height), not inclusive vertices; WR10.
    pub fn terrain_edit(&mut self, rect: CellRect, tuning: &RippleTuning) -> bool {
        self.terrain_edit_threshold(rect, tuning.edit_area_threshold)
    }

    fn terrain_edit_threshold(&mut self, rect: CellRect, threshold: u32) -> bool {
        self.accumulated_area = self.accumulated_area.saturating_add(rect.cell_area());
        if self.accumulated_area > u64::from(threshold) {
            self.accumulated_area = 0;
            self.arm();
            true
        } else {
            self.dirty = Some(self.dirty.map_or(rect, |old| old.union(rect)));
            if self.dirty.is_some_and(|dirty| dirty.cell_area() > 400) {
                self.arm();
            }
            false
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RippleUpdate {
    Disabled,
    Empty,
}

/// Published state and event portion of cSTEWaterRipple. Full trace/render
/// ownership is blocked by WR4–WR8; no installation of host-computed strips.
#[derive(Default)]
pub struct WaterRipple {
    process: Option<Arc<Mutex<RippleProcess>>>,
    initialized: bool,
    pub lifecycle_events: Vec<Event>,
    terrain: Option<Arc<dyn TerrainProvider>>,
    factory: Option<Arc<dyn TextureFactory>>,
    counts: [u32; 2],
    zoom: Option<Zoom>,
    detected: bool,
    remembered_counter: u64,
    rebuild: RebuildState,
}

impl WaterRipple {
    /// §2.4 normal required-service domain. Shared process storage is supplied
    /// explicitly, so reset and another instance operate on the same globals.
    pub fn init(
        &mut self,
        terrain: Arc<dyn TerrainProvider>,
        factory: Arc<dyn TextureFactory>,
        source: &dyn RawTuningSource,
        process: Arc<Mutex<RippleProcess>>,
        arithmetic: ReadArithmetic,
    ) -> Result<bool, Error> {
        if self.initialized {
            return Ok(false);
        }
        self.process = Some(process.clone());
        process
            .lock()
            .expect("non-reentrant tuning domain")
            .eligible_init(source, arithmetic)
            .map_err(Error::UnsupportedTuning)?;
        self.initialized = true;
        self.lifecycle_events.push(Event::Initialized);
        let counts = dimensions(terrain.as_ref())?;
        self.terrain = Some(terrain);
        self.lifecycle_events.push(Event::AcquireTerrain);
        self.factory = Some(factory);
        self.counts = counts;
        self.lifecycle_events.push(Event::Subscribe);
        Ok(true)
    }

    pub fn tuning(&self) -> Option<RetainedTuning> {
        self.process
            .as_ref()
            .map(|p| p.lock().expect("non-reentrant tuning domain").tuning.clone())
    }
    pub fn rebuild_state(&self) -> &RebuildState {
        &self.rebuild
    }
    pub fn remembered_counter(&self) -> u64 {
        self.remembered_counter
    }
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn eligible_flags(flags: u32) -> bool {
        flags & 0x10 != 0 && flags & 0x01 == 0
    }

    /// §1.2, the exact seed predicate. This is not a shore trace.
    pub fn shore_seeds(&self) -> Result<Vec<Cell>, Error> {
        let terrain = self.terrain.as_ref().ok_or(Error::NotInitialized)?;
        let mut seeds = Vec::new();
        // Enumeration order is a Rust diagnostic choice, not trace order (WR4).
        for z in 0..=self.counts[1] {
            for x in 0..=self.counts[0] {
                let cell = Cell { x, z };
                if Self::eligible_flags(terrain.vertex_flags(cell)) {
                    seeds.push(cell);
                }
            }
        }
        Ok(seeds)
    }

    pub fn detect_shores(&mut self) -> Result<(), Error> {
        if !self.shore_seeds()?.is_empty() {
            // Connectivity, trace direction, marking, termination and duplicate
            // suppression are absent, not permission to call original output.
            return Err(Error::Gap(ContractGap::ShoreTraceWr4));
        }
        self.remembered_counter = self.terrain.as_ref().ok_or(Error::NotInitialized)?.edit_counter();
        self.detected = true;
        Ok(())
    }

    pub fn update(&mut self, zoom: Zoom, water_effects: bool) -> Result<RippleUpdate, Error> {
        if self.terrain.is_none() {
            return Err(Error::NotInitialized);
        }
        if !self.tuning().ok_or(Error::NotInitialized)?.enabled {
            return Ok(RippleUpdate::Disabled);
        }
        // §1.4 first Update detects even if drawing is disabled. The property
        // gates drawing, not detection. There are no generated strips until WR4.
        if !self.detected {
            self.detect_shores()?;
        }
        self.zoom = Some(zoom);
        let _ = water_effects;
        Ok(RippleUpdate::Empty)
    }

    pub fn do_message(&mut self, message: RippleMessage) -> Result<(), Error> {
        let terrain = self.terrain.as_ref().ok_or(Error::NotInitialized)?;
        let tuning = self.tuning().ok_or(Error::NotInitialized)?;
        match message {
            RippleMessage::AnimationTick(elapsed) => {
                if self.rebuild.tick_threshold(
                    elapsed,
                    terrain.edit_counter(),
                    self.remembered_counter,
                    u64::from(tuning.minimum).pow(2) / 10,
                ) {
                    self.rebuild_now()?;
                }
            }
            RippleMessage::TerrainChanged(rect) => {
                super::vertex_index(rect.min, self.counts)?;
                super::vertex_index(rect.max, self.counts)?;
                CellRect::new(rect.min, rect.max)?;
                self.rebuild.terrain_edit_threshold(rect, tuning.edit_area);
                // No strips exist in this bounded owner, so intersection removal
                // is vacuous. A nonempty shore cannot pass detect_shores.
            }
            RippleMessage::RebuildNow => self.rebuild_now()?,
        }
        Ok(())
    }

    pub fn rebuild_now(&mut self) -> Result<(), Error> {
        self.detect_shores()?;
        self.rebuild = RebuildState::default();
        Ok(())
    }

    pub fn shutdown(&mut self) {
        if !self.initialized {
            return;
        }
        self.initialized = false;
        self.lifecycle_events.push(Event::Unsubscribe);
        if self.terrain.is_some() {
            self.lifecycle_events.push(Event::ReleaseTerrain);
        }
        self.terrain = None;
        self.factory = None;
        // Shores/first Update/zoom/dirty state and process globals survive.
        // No binding is held by this incomplete strip owner. Rust Arc Drop is
        // memory safety, never evidence of an additional native Release event.
    }
}
