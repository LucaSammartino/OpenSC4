// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The game effect (effects.md §9.2, §9.3): burns, spills, demolishes,
//! extinguishes and posts at its cell, once per new cell; pauses the
//! simulator while it runs.

use crate::x87::r24;

use super::directory::{game_flag, GameRecord};
use super::{EffectPost, EffectsHost, CELL_SIZE};

/// The fire cause a game effect sets after igniting (§9.3 step 1).
pub const BURN_CAUSE: u32 = 4;

/// An inclusive cell rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EffectRect {
    pub x0: i32,
    pub z0: i32,
    pub x1: i32,
    pub z1: i32,
}

/// The demolition filter of a game record (`0x005BCE80`, §9.3 step 3).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DemolishFilter {
    pub no_networks: bool,
    pub no_flora: bool,
    /// Metres; 0 = no limit.
    pub min_size: f32,
    pub max_size: f32,
}

/// What the filter needs to know about an occupant.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OccupantFacts {
    /// A signpost (`0x4B44FBE2`).
    pub signpost: bool,
    /// A text decal (`0x2B3B7D86`).
    pub text_decal: bool,
    /// A network occupant (`0xA821EF94`).
    pub network: bool,
    /// Flora (`0x74758926`).
    pub flora: bool,
    /// The bounding box's extents (metres), in any order.
    pub extents: [f32; 3],
}

impl DemolishFilter {
    pub fn of(record: &GameRecord) -> DemolishFilter {
        DemolishFilter {
            no_networks: record.flags & game_flag::NO_NETWORKS != 0,
            no_flora: record.flags & game_flag::NO_FLORA != 0,
            min_size: record.min_demolish_size,
            max_size: record.max_demolish_size,
        }
    }

    /// Whether the filter skips (keeps) the occupant: signposts and text
    /// decals always; networks with `noNetworks`; flora with `noFlora`;
    /// with `max > 0`, a largest extent `≥ max`; with `min > 0`, a smallest
    /// extent `≤ min`.
    pub fn skips(&self, o: &OccupantFacts) -> bool {
        if o.signpost || o.text_decal {
            return true;
        }
        if self.no_networks && o.network {
            return true;
        }
        if self.no_flora && o.flora {
            return true;
        }
        let largest = o.extents.iter().copied().fold(f32::MIN, f32::max);
        let smallest = o.extents.iter().copied().fold(f32::MAX, f32::min);
        if self.max_size > 0.0 && largest >= self.max_size {
            return true;
        }
        self.min_size > 0.0 && smallest <= self.min_size
    }
}

/// One game component (vtable `0x00AA0FF8`, §9.2).
#[derive(Clone, Debug, PartialEq)]
pub struct GameEffect {
    pub record: GameRecord,
    pub pending: bool,
    pub started: bool,
    pub timer: f32,
    pub count: u32,
    pub cell: (i32, i32),
    /// `+0x58`: the message's data1, initially the record's message data;
    /// parameter 13 replaces it (a host token, e.g. the disaster instance).
    pub data: u64,
    /// An effect map is applied (render, step 6).
    pub map_applied: bool,
    /// Opaque render-service identity; never interpreted as a native pointer.
    pub map_token: u64,
    /// Diagnostics: `Apply` calls made.
    pub applies: u32,
}

impl GameEffect {
    /// The constructor (`0x005BC310`).
    pub fn new(record: &GameRecord) -> GameEffect {
        GameEffect {
            record: *record,
            pending: true,
            started: false,
            timer: 0.0,
            count: 0,
            cell: (-1, -1),
            data: u64::from(record.message_data),
            map_applied: false,
            map_token: 0,
            applies: 0,
        }
    }

    /// `SetParameter(13, p)` (slot 6): `data := *p`. Other indices are
    /// ignored.
    pub fn set_parameter(&mut self, index: u32, value: u64) {
        if index == 13 {
            self.data = u64::from(value as u32);
        }
    }

    /// `SetTransform` (§9.2) with an identity local placement (EF7): the cell
    /// is `(floor(x / 16), floor(z / 16))`; a new cell sets `pending`.
    pub fn set_position(&mut self, x: f32, z: f32) {
        let cell = (
            (f64::from(x) / f64::from(CELL_SIZE)).floor() as i32,
            (f64::from(z) / f64::from(CELL_SIZE)).floor() as i32,
        );
        if cell != self.cell {
            self.cell = cell;
            self.pending = true;
        }
    }

    /// `SetTransform` at the semantic transform boundary (§9.2).
    pub fn set_transform(&mut self, local_offset: [f32; 3], parent: &super::transform::ParentTransform) {
        let position = parent.world_position(local_offset);
        self.set_position(position[0], position[2]);
    }

    pub fn is_active(&self) -> bool {
        self.started
    }

    /// `Start` (`0x005BD070`): only when not started and the simulator
    /// exists.
    pub fn start(&mut self, host: &mut dyn EffectsHost) {
        if self.started || !host.simulator_exists() {
            return;
        }
        self.started = true;
        self.count = 0;
        let f = self.record.flags;
        if f & game_flag::PAUSE_SIM != 0 {
            host.sim_pause();
        }
        if f & game_flag::PAUSE_SIM_HIDDEN != 0 {
            host.sim_hidden_pause();
        }
        if f & game_flag::PAUSE_CLOCK != 0 {
            host.clock_pause(true);
        }
        self.timer = 0.0;
    }

    /// `Stop` (`0x005BD340`): only when started; undoes the pauses (resume,
    /// hidden resume, clock) and removes the effect map.
    pub fn stop(&mut self, host: &mut dyn EffectsHost) {
        if !self.started {
            return;
        }
        self.started = false;
        let f = self.record.flags;
        if f & game_flag::PAUSE_SIM != 0 {
            host.sim_resume();
        }
        if f & game_flag::PAUSE_SIM_HIDDEN != 0 {
            host.sim_hidden_resume();
        }
        if f & game_flag::PAUSE_CLOCK != 0 {
            host.clock_pause(false);
        }
        if self.map_applied {
            host.remove_effect_map(self.map_token);
            self.map_applied = false;
        }
    }

    /// `Update(dt)` (`0x005BD4D0`).
    pub fn update(&mut self, dt: f32, host: &mut dyn EffectsHost) {
        if self.record.flags & game_flag::SINGLE != 0 {
            // [quirk] The pauses are not undone.
            self.apply(host);
            self.started = false;
            return;
        }
        self.timer = r24(f64::from(self.timer) + f64::from(dt)) as f32;
        if self.record.duration > 0.0 && self.timer > self.record.duration {
            self.stop(host);
        } else if self.pending {
            self.apply(host);
            self.count = self.count.wrapping_add(1);
            self.pending = false;
        }
    }

    /// The demolition rectangle at the cell, clipped to the city (§9.3 step
    /// 3); `None` when nothing of it is in the city [choice].
    pub fn demolish_rect(&self, cells: (i32, i32)) -> Option<EffectRect> {
        let r = self.record.demolish_radius;
        let (cx, cz) = self.cell;
        let rect = EffectRect {
            x0: cx.saturating_sub(r).max(0),
            z0: cz.saturating_sub(r).max(0),
            x1: cx.saturating_add(r).min(cells.0 - 1),
            z1: cz.saturating_add(r).min(cells.1 - 1),
        };
        (rect.x0 <= rect.x1 && rect.z0 <= rect.z1).then_some(rect)
    }

    /// `Apply` (`0x005BD3C0`, §9.3): burn, toxic, demolish, extinguish,
    /// message, map, in that order.
    fn apply(&mut self, host: &mut dyn EffectsHost) {
        let (cx, cz) = self.cell;
        let g = self.record;
        self.applies += 1;
        if g.burn_radius > 0 {
            host.ignite_cell_radius(cx, cz, 0, g.burn_radius);
            host.set_fire_cause(BURN_CAUSE);
        }
        if g.toxic > 0 {
            // [quirk] The amount is ignored (the spill is always 120).
            host.toxic_attempt_start(cx, cz);
        }
        if g.demolish_radius > 0 {
            if let Some(rect) = self.demolish_rect(host.city_cells()) {
                host.demolish_region(
                    rect,
                    g.demolish_flags,
                    g.flags & game_flag::DEZONE != 0,
                    &DemolishFilter::of(&g),
                );
            }
        }
        if g.extinguish > 0 {
            host.extinguish_cell(cx, cz, g.extinguish);
        }
        if g.message_type != 0 {
            host.post(EffectPost {
                msg_type: g.message_type,
                data1: self.data,
                data2: cx,
                data3: cz,
            });
        }
        if g.map_kind != 0 {
            // Render: the previous map goes when count > 0, then this one is
            // applied at the cell.
            if self.count > 0 {
                host.remove_effect_map(self.map_token);
            }
            self.map_token = host.apply_effect_map(&g, self.cell);
            self.map_applied = true;
        }
    }
}
