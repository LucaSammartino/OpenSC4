// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

use super::lifecycle::{Event, Interface, References, Release};
use super::mesh::{self, Animation, Parameters, Vertex};
use super::render::{self, Rect, Renderer};
use super::retained::{RawTuningSource, ReadArithmetic, RetainedTuning, RippleProcess};
use super::{dimensions, Cell, CellRect, Error, Point, RippleTuning, TerrainProvider, TextureBinding, TextureFactory, Zoom, RIPPLE_TEXTURE};
use std::sync::{Arc, Mutex};

pub const ANIMATION_TICK: u32 = 0x86AD_10EE;
pub const TERRAIN_CHANGED: u32 = 0x278E_6003;
pub const REBUILD_NOW: u32 = 0x8917_1496;
pub const REBUILD_DELAY: u64 = 2_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RippleMessage { AnimationTick(u64), TerrainChanged(CellRect), RebuildNow }
impl RippleMessage {
    pub fn id(self) -> u32 {
        match self { Self::AnimationTick(_) => ANIMATION_TICK, Self::TerrainChanged(_) => TERRAIN_CHANGED, Self::RebuildNow => REBUILD_NOW }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RebuildState {
    pending: bool,
    elapsed: u32,
    accumulated_area: u32,
    dirty: Rect,
}
impl Default for RebuildState {
    fn default() -> Self { Self { pending: false, elapsed: 0, accumulated_area: 0, dirty: Rect::DIRTY_EMPTY } }
}
impl RebuildState {
    pub fn pending(&self) -> bool { self.pending }
    pub fn elapsed(&self) -> u64 { u64::from(self.elapsed) }
    pub fn accumulated_area(&self) -> u64 { u64::from(self.accumulated_area) }
    pub fn signed_dirty_rectangle(&self) -> Rect { self.dirty }
    pub fn dirty_rectangle(&self) -> Option<CellRect> {
        (self.dirty != Rect::DIRTY_EMPTY).then_some(CellRect {
            min: Cell { x: self.dirty.min_x as u32, z: self.dirty.min_z as u32 },
            max: Cell { x: self.dirty.max_x as u32, z: self.dirty.max_z as u32 },
        })
    }
    pub fn arm(&mut self) { self.pending = true; self.elapsed = 0; }
    pub fn tick(&mut self, elapsed: u64, counter: u64, remembered: u64, tuning: &RippleTuning) -> bool {
        let minimum = tuning.minimum_shore_points;
        self.tick_threshold(elapsed as u32, counter as u32, remembered as u32, minimum.wrapping_mul(minimum) / 10)
    }
    fn tick_threshold(&mut self, elapsed: u32, counter: u32, remembered: u32, threshold: u32) -> bool {
        if self.pending { self.elapsed = self.elapsed.wrapping_add(elapsed); }
        else if counter > remembered.wrapping_add(threshold) { self.arm(); }
        self.pending && self.elapsed > REBUILD_DELAY as u32
    }
    fn begin_edit(&mut self, rect: Rect, threshold: u32) -> bool {
        let area = rect.area();
        if area <= 0 { return false; }
        self.accumulated_area = self.accumulated_area.wrapping_add(area as u32);
        if self.accumulated_area <= threshold { return false; }
        self.accumulated_area = 0;
        true
    }
    fn finish_edit(&mut self, rect: Rect, removed: bool) {
        if removed { self.arm(); }
        else if !self.pending {
            self.dirty = self.dirty.union(rect);
            if self.dirty.area() > 400 { self.arm(); }
        }
    }
    /// Standalone no-strip contact ledger. The owner supplies actual removal.
    pub fn terrain_edit(&mut self, rect: CellRect, tuning: &RippleTuning) -> bool {
        let rect = signed(rect);
        if !self.begin_edit(rect, tuning.edit_area_threshold) { return false; }
        self.finish_edit(rect, false);
        true
    }
}
fn signed(r: CellRect) -> Rect {
    Rect { min_x: r.min.x as i32, min_z: r.min.z as i32, max_x: r.max.x as i32, max_z: r.max.z as i32 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RippleUpdate { Disabled, Empty, Prepared { shores: usize, strips: usize }, Drawn { strips: usize } }

#[derive(Clone, Debug)]
pub struct Strip {
    pub vertices: Vec<Vertex>,
    pub animation: Animation,
    pub rectangles: Vec<Rect>,
    pub bounds: Rect,
}
#[derive(Clone, Debug)]
pub struct Shore { pub points: Vec<Point>, pub strips: Vec<Strip>, pub bounds: Rect }

/// Portable owner. Terrain/renderer primitives remain foreign. Native invalid
/// service/backing/exception domains remain typed errors, never safe success.
#[derive(Default)]
pub struct WaterRipple {
    process: Option<Arc<Mutex<RippleProcess>>>,
    initialized: bool,
    pub lifecycle_events: Vec<Event>,
    references: References,
    terrain: Option<Arc<dyn TerrainProvider>>,
    factory: Option<Arc<dyn TextureFactory>>,
    binding: Option<Arc<dyn TextureBinding>>,
    counts: [u32; 2],
    extent: Point,
    water: f32,
    zoom: Option<Zoom>,
    detected: bool,
    remembered_counter: u32,
    rebuild: RebuildState,
    shores: Vec<Shore>,
    phase: f32,
}
impl WaterRipple {
    pub fn init(&mut self, terrain: Arc<dyn TerrainProvider>, factory: Arc<dyn TextureFactory>, source: &dyn RawTuningSource, process: Arc<Mutex<RippleProcess>>, arithmetic: ReadArithmetic) -> Result<bool, Error> {
        if self.initialized { return Ok(false); }
        process.lock().expect("non-reentrant tuning domain").eligible_init(source, arithmetic).map_err(Error::UnsupportedTuning)?;
        self.process = Some(process);
        self.initialized = true;
        self.lifecycle_events.push(Event::Initialized);
        self.extent = terrain.city_extent()?;
        self.counts = dimensions(terrain.as_ref())?;
        self.water = terrain.water_level();
        self.zoom = Some(terrain.current_zoom()?);
        self.terrain = Some(terrain);
        self.lifecycle_events.push(Event::AcquireTerrain);
        self.factory = Some(factory);
        self.lifecycle_events.push(Event::Subscribe);
        Ok(true)
    }
    pub fn tuning(&self) -> Option<RetainedTuning> {
        self.process.as_ref().map(|p| p.lock().expect("non-reentrant tuning domain").tuning.clone())
    }
    pub fn rebuild_state(&self) -> &RebuildState { &self.rebuild }
    pub fn remembered_counter(&self) -> u64 { u64::from(self.remembered_counter) }
    pub fn is_initialized(&self) -> bool { self.initialized }
    pub fn shores(&self) -> &[Shore] { &self.shores }
    pub fn phase(&self) -> f32 { self.phase }
    pub fn references(&self) -> &References { &self.references }
    pub fn add_ref(&mut self) -> u32 { self.references.add_ref() }
    pub fn remove_ref(&mut self) -> u32 { self.references.remove_ref() }
    pub fn release(&mut self) -> Release { self.references.release() }
    pub fn query_interface(&mut self, id: u32, output: &mut Option<Interface>) -> bool { self.references.query(true, id, output) }
    /// Opaque four-word Pick boundary (§2.5); no semantic argument types invented.
    pub fn pick(&self, _words: &mut [u32; 4]) -> bool { false }
    pub fn eligible_flags(flags: u32) -> bool { flags & 0x10 != 0 && flags & 1 == 0 }
    pub fn shore_seeds(&self) -> Result<Vec<Cell>, Error> {
        let terrain = self.terrain.as_ref().ok_or(Error::NotInitialized)?;
        let mut seeds = Vec::new();
        for z in 0..=self.counts[1] { for x in 0..=self.counts[0] {
            let c = Cell { x, z };
            if Self::eligible_flags(terrain.vertex_flags(c)) { seeds.push(c); }
        }}
        Ok(seeds)
    }
    pub fn detect_shores(&mut self) -> Result<(), Error> {
        let t = self.tuning().ok_or(Error::NotInitialized)?;
        if !t.enabled { return Ok(()); }
        let terrain = self.terrain.as_ref().ok_or(Error::NotInitialized)?;
        self.rebuild.accumulated_area = 0;
        self.remembered_counter = terrain.edit_counter() as u32;
        for points in super::shore::detect(terrain.as_ref(), t.minimum)? {
            self.shores.push(Shore { points, strips: Vec::new(), bounds: Rect::SHORE_EMPTY });
        }
        Ok(())
    }
    pub fn create_ripples(&mut self, zoom: Zoom) -> Result<(), Error> {
        if self.rebuild.pending {
            self.shores.clear();
            self.detect_shores()?;
            self.rebuild.pending = false;
        }
        let t = self.tuning().ok_or(Error::NotInitialized)?;
        self.phase = 0.0;
        self.zoom = Some(zoom);
        if self.shores.is_empty() { return Ok(()); }
        let p = Parameters::from_retained(&t, zoom)?;
        let factory = self.factory.as_ref().ok_or(Error::NotInitialized)?;
        for shore in &mut self.shores {
            shore.strips.clear();
            let binding = &mut self.binding;
            let mut acquire = || {
                if binding.is_none() { *binding = factory.bind_slot6(RIPPLE_TEXTURE); }
                binding.is_some()
            };
            let raw = mesh::traverse(&shore.points, &mut self.phase, self.water, self.extent, &p, &mut acquire)?;
            shore.strips = raw.into_iter().map(|vertices| Strip { vertices, animation: Animation { frame: 0, clock: 0, period: 0 }, rectangles: Vec::new(), bounds: Rect::SHORE_EMPTY }).collect();
        }
        let has_strips = self.shores.iter().any(|s| !s.strips.is_empty());
        if has_strips {
            let binding = self.binding.as_ref().ok_or(Error::UndefinedTexturePreparation)?;
            let mut process = self.process.as_ref().unwrap().lock().expect("non-reentrant process domain");
            let random = process.random.as_mut().ok_or(Error::UnsetRandomState)?;
            for shore in &mut self.shores {
                for strip in &mut shore.strips {
                    strip.animation = Animation::prepare(binding.frame_count(), t.period_bounds, random);
                    mesh::fade(&mut strip.vertices, p.alpha, p.fade)?;
                }
            }
        }
        for shore in &mut self.shores {
            let mut bounds = Rect::DIRTY_EMPTY;
            for strip in &mut shore.strips {
                strip.rectangles = render::contact_rectangles(&strip.vertices);
                strip.bounds = strip.rectangles.iter().copied().reduce(Rect::union).unwrap_or(Rect::SHORE_EMPTY);
                bounds = bounds.union(strip.bounds);
            }
            shore.bounds = if shore.strips.is_empty() { Rect::SHORE_EMPTY } else { bounds };
        }
        Ok(())
    }
    fn prepare_update(&mut self, zoom: Zoom, water_effects: bool) -> Result<RippleUpdate, Error> {
        if self.terrain.is_none() { return Err(Error::NotInitialized); }
        if !self.detected {
            self.detect_shores()?;
            self.create_ripples(self.zoom.ok_or(Error::NotInitialized)?)?;
            self.detected = true;
        }
        if self.shores.is_empty() { return Ok(RippleUpdate::Empty); }
        if !water_effects { return Ok(RippleUpdate::Disabled); }
        if self.zoom != Some(zoom) { self.create_ripples(zoom)?; }
        Ok(RippleUpdate::Prepared { shores: self.shores.len(), strips: self.shores.iter().map(|s| s.strips.len()).sum() })
    }
    pub fn update(&mut self, zoom: Zoom, water_effects: bool) -> Result<RippleUpdate, Error> { self.prepare_update(zoom, water_effects) }
    pub fn update_with_renderer(&mut self, zoom: Zoom, water_effects: bool, renderer: &mut dyn Renderer) -> Result<RippleUpdate, Error> {
        let result = self.prepare_update(zoom, water_effects)?;
        if !matches!(result, RippleUpdate::Prepared { .. }) { return Ok(result); }
        let state = {
            let mut process = self.process.as_ref().unwrap().lock().expect("non-reentrant process domain");
            *process.render_state.get_or_insert_with(|| renderer.pack_state(render::RIPPLE_STATE))
        };
        renderer.set_state(state);
        renderer.default_model_transform();
        let mut drawn = 0;
        for shore in &self.shores {
            if shore.strips.is_empty() || !renderer.visible(shore.bounds.render_box(self.water)) { continue; }
            for strip in &shore.strips {
                if !renderer.visible(strip.bounds.render_box(self.water)) { continue; }
                if let Some(binding) = &self.binding {
                    renderer.bind_frame(binding.as_ref(), strip.animation.frame, 0);
                    renderer.submit(7, 10, &strip.vertices);
                    drawn += 1;
                }
            }
        }
        Ok(RippleUpdate::Drawn { strips: drawn })
    }
    pub fn terrain_edit_packed(&mut self, slot9: u32, slot14: u32) -> Result<(), Error> { self.terrain_edit(Rect::from_packed(slot9, slot14)) }
    fn terrain_edit(&mut self, rect: Rect) -> Result<(), Error> {
        let t = self.tuning().ok_or(Error::NotInitialized)?;
        if !self.rebuild.begin_edit(rect, t.edit_area) { return Ok(()); }
        let mut removed = false;
        for shore in &mut self.shores {
            if !shore.bounds.overlaps(rect) { continue; }
            shore.strips.retain(|strip| {
                let contact = strip.bounds.overlaps(rect) && strip.rectangles.iter().any(|r| r.overlaps(rect));
                removed |= contact;
                !contact
            });
        }
        self.rebuild.finish_edit(rect, removed);
        Ok(())
    }
    pub fn do_message(&mut self, message: RippleMessage) -> Result<(), Error> {
        let terrain = self.terrain.as_ref().ok_or(Error::NotInitialized)?;
        let t = self.tuning().ok_or(Error::NotInitialized)?;
        match message {
            RippleMessage::AnimationTick(elapsed) => {
                if !self.detected { return Ok(()); }
                if self.rebuild.tick_threshold(elapsed as u32, terrain.edit_counter() as u32, self.remembered_counter, t.minimum.wrapping_mul(t.minimum) / 10) {
                    self.shores.clear();
                    self.detect_shores()?;
                    self.create_ripples(self.zoom.ok_or(Error::NotInitialized)?)?;
                }
                if let Some(binding) = &self.binding {
                    let mut process = self.process.as_ref().unwrap().lock().expect("non-reentrant process domain");
                    let random = process.random.as_mut().ok_or(Error::UnsetRandomState)?;
                    for shore in &mut self.shores { for strip in &mut shore.strips {
                        strip.animation.tick(elapsed as u32, binding.frame_count(), t.period_bounds, random);
                    }}
                }
            }
            RippleMessage::TerrainChanged(rect) => self.terrain_edit(signed(rect))?,
            RippleMessage::RebuildNow => self.rebuild_now()?,
        }
        Ok(())
    }
    pub fn rebuild_now(&mut self) -> Result<(), Error> {
        self.shores.clear();
        self.detect_shores()?;
        self.create_ripples(self.zoom.ok_or(Error::NotInitialized)?)
    }
    pub fn shutdown(&mut self) -> bool {
        if !self.initialized { return true; }
        self.initialized = false;
        self.lifecycle_events.push(Event::Unsubscribe);
        if self.terrain.take().is_some() { self.lifecycle_events.push(Event::ReleaseTerrain); }
        if self.binding.take().is_some() { self.lifecycle_events.push(Event::ReleaseBinding); }
        self.factory = None;
        // Collection, first-Update, dirty and process state deliberately survive.
        // Rust Drop is memory safety, not an invented native Shutdown event.
        true
    }
}
