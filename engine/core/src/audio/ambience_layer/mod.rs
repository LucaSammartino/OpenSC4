// SPDX-License-Identifier: GPL-3.0-or-later
//! Ambience-layer execution, audio-ambience §8. Constructor/extra slots: AA14.
use std::collections::BTreeMap;

use crate::curve::ResponseCurve;

mod runtime;
mod spatial;
pub mod owner;
pub use runtime::{AmbienceLayer, Ease, LayerEventHost, LayerUpdateError, LayerUpdateInput, LoopEntry, ScoreEntry};
pub use spatial::{distributed_positions, Group, GroupLayer};

pub type Position = [f32; 3];
pub type Rectangle = [f32; 4];

/// Foreign exemplar reads. Each call performs one property lookup, not an
/// opaque tuning/filter/update algorithm.
pub trait LayerTuningSource {
    fn text(&mut self, key: u32) -> Option<String>;
    fn words(&mut self, key: u32) -> Option<Vec<u32>>;
    fn word(&mut self, key: u32) -> Option<u32>;
    fn float(&mut self, key: u32) -> Option<f32>;
    fn curve(&mut self, key: u32) -> Option<ResponseCurve>;
}

#[derive(Clone, Debug)]
pub struct LayerTuning {
    pub name: String,
    pub score_ids: Vec<u32>,
    pub filter_ids: [Vec<u32>; 3],
    pub loop_id: u32,
    pub bytes: [u8; 2],
    pub word_1ec: u32,
    pub master_volume: u32,
    pub distance_percent: u32,
    pub group_mode: u32,
    pub score_mode: u32,
    pub sharpen: f32,
    pub curves: [ResponseCurve; 3],
    pub occupant_radius: f32,
    pub cell_radius: f32,
    pub vector_rate: f32,
    pub scalar_rate: f32,
    pub preference_mode: u32,
}

impl LayerTuning {
    /// `constructed_loop_id` is retained on missing resource, rather than
    /// guessing the unknown constructor's loop ID (AA14).
    pub fn defaults(layer_id: u32, constructed_loop_id: u32, preference_mode: u32) -> Self {
        Self {
            name: format!("Noname_0x{layer_id:08x}"),
            score_ids: Vec::new(),
            filter_ids: std::array::from_fn(|_| Vec::new()),
            loop_id: constructed_loop_id,
            bytes: [0x1F; 2],
            word_1ec: 1024,
            master_volume: 1024,
            distance_percent: 100,
            group_mode: 1,
            score_mode: 5,
            sharpen: 1.0,
            curves: [
                ResponseCurve::from_points(&[(0.0, 100.0), (20.0, 100.0)]),
                ResponseCurve::from_points(&[(0.0, 0.0), (20.0, 1024.0)]),
                ResponseCurve::from_points(&[(0.0, 1024.0), (20.0, 1024.0)]),
            ],
            occupant_radius: 50.0,
            cell_radius: 50.0,
            vector_rate: (100.0 * f64::from(0.001f32)) as f32,
            scalar_rate: (4000.0 * f64::from(0.001f32)) as f32,
            preference_mode,
        }
    }

    /// §8.2 reads in the specified order. Missing resource preserves defaults;
    /// present resource clears loop/byte words before their individual reads.
    pub fn read(
        layer_id: u32,
        constructed_loop_id: u32,
        preference_mode: u32,
        source: Option<&mut dyn LayerTuningSource>,
    ) -> (Self, bool) {
        let mut tuning = Self::defaults(layer_id, constructed_loop_id, preference_mode);
        let Some(source) = source else {
            return (tuning, false);
        };
        if let Some(name) = source.text(0x20) {
            tuning.name = name;
        }
        tuning.score_ids = source.words(0xEA3F_BDD7).unwrap_or_default();
        for (list, key) in tuning
            .filter_ids
            .iter_mut()
            .zip([0x2A3F_BDE1, 0x2A3F_BDE5, 0x0A41_0BA0])
        {
            *list = source.words(key).unwrap_or_default();
        }
        tuning.loop_id = source.word(0x6A3F_BDDA).unwrap_or(0);
        tuning.bytes = [
            source.word(0x0A25_896B).unwrap_or(0) as u8,
            source.word(0xAA3F_BDDE).unwrap_or(0) as u8,
        ];
        for (destination, key) in [
            (&mut tuning.word_1ec, 0xEA3F_C42A),
            (&mut tuning.master_volume, 0xAA3F_C48E),
            (&mut tuning.distance_percent, 0x6A3F_C4C7),
            (&mut tuning.group_mode, 0x8A55_2758),
            (&mut tuning.score_mode, 0x8A70_B1A2),
        ] {
            if let Some(value) = source.word(key) {
                *destination = value;
            }
        }
        if let Some(value) = source.float(0x8A3F_C5DD) {
            tuning.sharpen = value;
        }
        for (curve, key) in tuning.curves.iter_mut().zip([0xCA41_32E9, 0x6A41_32F3, 0x6A41_32F6]) {
            if let Some(value) = source.curve(key) {
                *curve = value;
            }
        }
        if let Some(value) = source.float(0x0A41_68E1) {
            tuning.occupant_radius = value;
        }
        tuning.cell_radius = source.float(0x0A56_212A).unwrap_or(tuning.occupant_radius);
        tuning.vector_rate = (f64::from(source.word(0x8AA0_39B7).unwrap_or(100)) * f64::from(0.001f32)) as f32;
        tuning.scalar_rate = (f64::from(source.word(0x4AA0_3DB1).unwrap_or(4000)) * f64::from(0.001f32)) as f32;
        (tuning, true)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum FilterStage {
    Global,
    Cell(i32, i32),
    Occupant(u64),
}
impl FilterStage {
    fn iid(self) -> u32 {
        match self {
            Self::Global => 0xEA40_074F,
            Self::Cell(..) => 0xCA40_0757,
            Self::Occupant(_) => 0xCA40_0753,
        }
    }
}

pub trait FilterHost {
    /// Created reference, acquired by cache only after successful Init.
    fn create_filter(&mut self, clsid: u32, iid: u32) -> Option<u64>;
    fn init_filter(&mut self, filter: u64) -> bool;
    fn query(&mut self, filter: u64, iid: u32) -> Option<u64>;
    fn test(&mut self, interface: u64, stage: FilterStage) -> bool;
    fn release(&mut self, reference: u64);
    fn log(&mut self, message: &'static str);
    fn cell_center(&mut self, x: i32, z: i32) -> (f32, f32);
    fn altitude(&mut self, x: f32, z: f32) -> f32;
    fn occupant_position(&mut self, occupant: u64) -> Position;
}

/// Must be shared across layers; neither cache is reset at Begin/EndUpdate.
#[derive(Debug, Default)]
pub struct LayerGlobals {
    pub filters: BTreeMap<u32, u64>,
    pub cell_cache: Option<((i32, i32), Position)>,
    pub layer_count: u32,
}

#[derive(Debug, Default)]
pub struct LayerFilters {
    pub lists: [Vec<u64>; 3],
    pub bits: [Vec<bool>; 3],
    /// cell/occupant/global counts, accumulated once per listing.
    pub interface_counts: [u32; 3],
    pub cached_global: Option<bool>,
}

impl LayerFilters {
    pub fn create(&mut self, ids: &[Vec<u32>; 3], globals: &mut LayerGlobals, host: &mut impl FilterHost) {
        self.bits = std::array::from_fn(|kind| vec![false; ids[kind].len()]);
        for (kind, list) in ids.iter().enumerate() {
            for id in list {
                let filter = if let Some(filter) = globals.filters.get(id) {
                    *filter
                } else {
                    let Some(filter) = host.create_filter(*id, 0x4A40_074C) else {
                        host.log("Failed to create class object filter");
                        continue;
                    };
                    if !host.init_filter(filter) {
                        host.log("Failed to init filter");
                        host.release(filter);
                        continue;
                    }
                    globals.filters.insert(*id, filter);
                    filter
                };
                let mut queries = Vec::new();
                for (count, iid) in self
                    .interface_counts
                    .iter_mut()
                    .zip([0xCA40_0757, 0xCA40_0753, 0xEA40_074F])
                {
                    if let Some(reference) = host.query(filter, iid) {
                        *count = count.wrapping_add(1);
                        queries.push(reference);
                    }
                }
                self.lists[kind].push(filter);
                for reference in queries {
                    host.release(reference);
                }
            }
        }
    }

    pub fn begin(&mut self) {
        self.cached_global = None;
        for bits in &mut self.bits {
            bits.fill(false);
        }
    }

    fn stage(&mut self, stage: FilterStage, host: &mut impl FilterHost) -> bool {
        let mut answered = 0;
        let mut any_passed = false;
        for kind in 0..3 {
            for (index, filter) in self.lists[kind].iter().enumerate() {
                let Some(interface) = host.query(*filter, stage.iid()) else {
                    continue;
                };
                answered += 1;
                let passed = host.test(interface, stage);
                host.release(interface);
                match kind {
                    0 if !passed => return false,
                    1 if passed => return false,
                    2 if !passed => continue,
                    _ => self.bits[kind][index] = true,
                }
                if kind == 2 {
                    any_passed = true;
                    break;
                }
            }
        }
        answered != 0 && (self.lists[2].is_empty() || any_passed)
    }

    pub fn can_handle(&mut self, sound_enabled: bool, host: &mut impl FilterHost) -> bool {
        if !sound_enabled {
            return false;
        }
        if let Some(cached) = self.cached_global {
            return cached;
        }
        let result = self.stage(FilterStage::Global, host);
        self.cached_global = Some(result);
        result
    }

    #[allow(clippy::too_many_arguments)] // Explicit stage/cache/counter inputs from §8.4.
    pub fn handle_cell(
        &mut self,
        x: i32,
        z: i32,
        sound_enabled: bool,
        groups: &mut GroupLayer,
        counters: &mut [u32; 4],
        globals: &mut LayerGlobals,
        host: &mut impl FilterHost,
    ) {
        if !self.can_handle(sound_enabled, host) {
            return;
        }
        if !self.stage(FilterStage::Cell(x, z), host) {
            counters[3] = counters[3].wrapping_add(1);
            return;
        }
        counters[1] = counters[1].wrapping_add(1);
        let position = if let Some((_, point)) = globals.cell_cache.filter(|(key, _)| *key == (x, z)) {
            point
        } else {
            let (px, pz) = host.cell_center(x, z);
            let point = [px, host.altitude(px, pz), pz];
            globals.cell_cache = Some(((x, z), point));
            point
        };
        groups.insert(position, 0.0);
    }

    pub fn handle_occupant(
        &mut self,
        occupant: Option<u64>,
        sound_enabled: bool,
        groups: &mut GroupLayer,
        counters: &mut [u32; 4],
        host: &mut impl FilterHost,
    ) {
        if !self.can_handle(sound_enabled, host) {
            return;
        }
        let Some(occupant) = occupant else {
            return;
        };
        if !self.stage(FilterStage::Occupant(occupant), host) {
            counters[2] = counters[2].wrapping_add(1);
            return;
        }
        counters[0] = counters[0].wrapping_add(1);
        groups.insert(host.occupant_position(occupant), 0.0);
    }
}

#[cfg(test)]
mod tests;
