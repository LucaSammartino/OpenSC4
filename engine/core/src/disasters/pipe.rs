// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The pipe burst (§8, short form).

use super::system::{Core, InstanceState};
use super::{
    instance_class, kind, msg, zoom_of, DerivedSeeds, DisasterMessage, DisasterPost, DisasterRandom,
    PIPE_BURST_FLAGS, TICK_PRIORITY,
};
use crate::property::PropertyHolder;
use crate::rng::RzRandom;

/// Tuning properties of the "PipeBurst" exemplar (§8).
pub mod pipe_prop {
    pub const BURST_MIN_S: u32 = 0x4A08_C600;
    pub const BURST_MAX_S: u32 = 0x4A08_C601;
}

/// The values of §8, also the defaults [choice, Q20].
pub const SHIPPED_BURST_MIN_S: f32 = 30.0;
pub const SHIPPED_BURST_MAX_S: f32 = 60.0;
pub const SHIPPED_ZOOM: u32 = 3;
/// Wait entries start their leaks after this many ticks (§8).
pub const WAIT_TICKS: u32 = 4;
/// The sinkhole brush (§8). Its strength and width are not given: 1 and the
/// cell width [choice, Q23].
pub const SINKHOLE_BRUSH: u32 = 0x14;
pub const SINKHOLE_STRENGTH: f32 = 1.0;

#[derive(Clone, Debug, PartialEq)]
pub struct PipeTuning {
    pub burst_min_s: f32,
    pub burst_max_s: f32,
    pub zoom: u32,
}

impl Default for PipeTuning {
    fn default() -> PipeTuning {
        PipeTuning::from_holder(None)
    }
}

impl PipeTuning {
    pub fn from_holder(h: Option<&PropertyHolder>) -> PipeTuning {
        let f = |id, d: f32| h.and_then(|h| crate::props::f32_of(h, id)).unwrap_or(d);
        PipeTuning {
            burst_min_s: f(pipe_prop::BURST_MIN_S, SHIPPED_BURST_MIN_S),
            burst_max_s: f(pipe_prop::BURST_MAX_S, SHIPPED_BURST_MAX_S),
            zoom: zoom_of(h, SHIPPED_ZOOM),
        }
    }
}

/// A leak (§8).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Leak {
    pub pipe: u64,
    pub cell: (i32, i32),
    pub elapsed_us: u64,
    pub burst_us: u64,
    /// The leak shows an effect (it can burst).
    pub effect: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Wait {
    pub cells: Vec<(i32, i32)>,
    pub age: u32,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PipeState {
    pub pipe: u64,
    pub leaks: Vec<Leak>,
    pub waits: Vec<Wait>,
}

/// A pipe burst's observable state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PipeSnapshot {
    pub id: u32,
    pub phase: u32,
    pub pipe: u64,
    pub leaks: Vec<Leak>,
    /// (cells, age) of each wait entry.
    pub waits: Vec<(Vec<(i32, i32)>, u32)>,
}

impl Core {
    fn ps(&mut self, id: u32) -> Option<&mut PipeState> {
        match &mut self.instances.get_mut(&id)?.state {
            InstanceState::Pipe(p) => Some(p),
            _ => None,
        }
    }

    fn ps_ref(&self, id: u32) -> Option<&PipeState> {
        match &self.instances.get(&id)?.state {
            InstanceState::Pipe(p) => Some(p),
            _ => None,
        }
    }

    fn pt(&mut self) -> PipeTuning {
        if self.pipe_tuning.is_none() {
            let h = self.host.tuning(super::tuning_instance::PIPE_BURST);
            self.pipe_tuning = Some(PipeTuning::from_holder(h.as_deref()));
        }
        self.pipe_tuning.clone().unwrap_or_default()
    }

    /// `AttemptDisasterStart(x, z)` (§8), cells.
    pub(crate) fn pipe_attempt_start(&mut self, x: i32, z: i32) -> Option<u32> {
        let pipe = self.host.find_watered_pipe(x, z)?;
        self.pipe_create(pipe)
    }

    /// The argument-less start: a throwaway generator (`derive(0x4A132FDF
    /// XOR n)`, foundations §6.3), a random cell (x then z,
    /// `RandomUint32Uniform(count)`), then rings outward over the whole
    /// city in row order until a watered pipe [choice, Q18].
    pub(crate) fn pipe_attempt_start_random(&mut self) -> Option<u32> {
        let (cx, cz) = self.host.city_cells();
        let n = self.pipe_start_count;
        self.pipe_start_count = n.wrapping_add(1);
        let (sx, sz) = match self.rng_pipe_start.as_mut() {
            Some(r) => (r.uniform_u32(cx), r.uniform_u32(cz)),
            None => {
                let s = crate::seeding::seed(self.session_seed, crate::seeding::keys::pipe_burst_start(n));
                let mut r = RzRandom::new(s, &DerivedSeeds(s));
                (
                    DisasterRandom::uniform_u32(&mut r, cx),
                    DisasterRandom::uniform_u32(&mut r, cz),
                )
            }
        };
        let (sx, sz) = (sx as i32, sz as i32);
        let max_r = cx.max(cz) as i32;
        for r in 0..=max_r {
            for dz in -r..=r {
                for dx in -r..=r {
                    if dx.abs().max(dz.abs()) != r {
                        continue;
                    }
                    let (px, pz) = (sx + dx, sz + dz);
                    if !self.inside_city(px, pz) {
                        continue;
                    }
                    if let Some(p) = self.host.find_watered_pipe(px, pz) {
                        return self.pipe_create(p);
                    }
                }
            }
        }
        None
    }

    /// `CreateDisasterInstance`: null when `Init` fails.
    fn pipe_create(&mut self, pipe: u64) -> Option<u32> {
        let id = self.new_instance(
            kind::PIPE_BURST,
            instance_class::PIPE_BURST,
            InstanceState::Pipe(Box::new(PipeState {
                pipe,
                ..PipeState::default()
            })),
        );
        if self.pipe_init(id, pipe) {
            Some(id)
        } else {
            self.instance_finish(id);
            None
        }
    }

    fn pipe_init(&mut self, id: u32, pipe: u64) -> bool {
        let tuning = self.pt();
        self.add_instance_to_list(kind::PIPE_BURST, id);
        self.subscribe(id, msg::PRE_VIEW_LEVEL_CHANGE);
        self.subscribe(id, msg::OCCUPANT_INSERTED);
        self.subscribe(id, msg::OCCUPANT_REMOVED);
        self.register_tick(id, TICK_PRIORITY);
        if !self.start_leak(id, pipe) {
            return false;
        }
        self.register_location(id, 0, 0, pipe, tuning.zoom, true);
        if let Some(i) = self.instances.get_mut(&id) {
            i.phase = 1;
        }
        self.disaster_has_begun(id);
        true
    }

    fn pipe_leaks(&self, pipe: u64) -> bool {
        self.instances.values().any(|i| match &i.state {
            InstanceState::Pipe(p) => p.leaks.iter().any(|l| l.pipe == pipe),
            _ => false,
        })
    }

    /// A leak at a pipe (§8 "Leak"): false when the pipe already leaks (in
    /// any pipe burst [choice]). The burst time is
    /// `RandomDoubleRangeUniform(min, max)` seconds, kept in µs [choice, Q7].
    fn start_leak(&mut self, id: u32, pipe: u64) -> bool {
        if self.pipe_leaks(pipe) {
            return false;
        }
        let tuning = self.pt();
        let host = self.host.clone();
        let s = self
            .rng_pipe
            .range_f64(f64::from(tuning.burst_min_s), f64::from(tuning.burst_max_s));
        let burst_us = (s * 1e6).max(0.0) as u64;
        let effect = host.create_leak_effect(pipe, host.view_level());
        let cell = host.pipe_cell(pipe);
        if let Some(p) = self.ps(id) {
            p.leaks.push(Leak {
                pipe,
                cell,
                elapsed_us: 0,
                burst_us,
                effect,
            });
        }
        host.post(&DisasterPost {
            msg_type: msg::PIPE_BURST_AT_CELL,
            data1: cell.0 as u32 as u64,
            data2: cell.1 as u32,
            ..DisasterPost::default()
        });
        true
    }

    pub(crate) fn pipe_tick(&mut self, id: u32, us: u32) {
        let Some(p) = self.ps(id) else { return };
        let mut bursts = Vec::new();
        for l in p.leaks.iter_mut() {
            l.elapsed_us += u64::from(us);
            if l.effect && l.elapsed_us >= l.burst_us {
                bursts.push(*l);
            }
        }
        // Wait entries made earlier age now; those made by this tick's
        // bursts start ageing next tick [choice].
        let mut ready = Vec::new();
        for w in p.waits.iter_mut() {
            w.age += 1;
        }
        p.waits.retain(|w| {
            if w.age >= WAIT_TICKS {
                ready.push(w.cells.clone());
                false
            } else {
                true
            }
        });
        for l in bursts {
            self.burst_pipe(id, l);
        }
        for cells in ready {
            for (x, z) in cells {
                if let Some(pipe) = self.host.find_watered_pipe(x, z) {
                    self.start_leak(id, pipe);
                }
            }
        }
        if self
            .ps_ref(id)
            .is_some_and(|p| p.leaks.is_empty() && p.waits.is_empty())
        {
            self.unregister_locations(id);
            if let Some(i) = self.instances.get_mut(&id) {
                i.phase = 3;
            }
            self.disaster_has_ended();
            self.instance_finish(id);
        }
    }

    /// `BurstPipe` (§8 tick step 1).
    fn burst_pipe(&mut self, id: u32, l: Leak) {
        let host = self.host.clone();
        host.play_effect("pipe_explodes", l.pipe);
        let cells: Vec<(i32, i32)> = host
            .connected_pipe_cells(l.pipe)
            .into_iter()
            .filter(|&(x, z)| self.inside_city(x, z))
            .collect();
        if let Some(p) = self.ps(id) {
            p.leaks.retain(|x| x.pipe != l.pipe);
            p.waits.push(Wait { cells, age: 0 });
        }
        host.demolish_pipe(l.pipe, PIPE_BURST_FLAGS);
        host.post(&DisasterPost {
            msg_type: msg::PIPE_BURST_AT_CELL,
            data1: l.cell.0 as u32 as u64,
            data2: l.cell.1 as u32,
            ..DisasterPost::default()
        });
        let w = host.cell_width();
        host.apply_terrain_brush(
            SINKHOLE_BRUSH,
            (l.cell.0 as f32 + 0.5) * w,
            (l.cell.1 as f32 + 0.5) * w,
            SINKHOLE_STRENGTH,
            w,
        );
    }

    /// The pipe burst's messages: a view-level change (data1 = the new
    /// level) recreates the effect of every leak without one [inferred];
    /// the occupant messages are not specified and do nothing [question,
    /// Q24].
    pub(crate) fn pipe_message(&mut self, id: u32, m: &DisasterMessage) {
        if m.msg_type != msg::PRE_VIEW_LEVEL_CHANGE {
            return;
        }
        let host = self.host.clone();
        let level = m.data1 as u32;
        let list: Vec<u64> = self.ps_ref(id).map_or(Vec::new(), |p| {
            p.leaks.iter().filter(|l| !l.effect).map(|l| l.pipe).collect()
        });
        for pipe in list {
            if host.create_leak_effect(pipe, level) {
                if let Some(l) = self.ps(id).and_then(|p| p.leaks.iter_mut().find(|l| l.pipe == pipe)) {
                    l.effect = true;
                }
            }
        }
    }

    /// Instance `Shutdown`: nothing beyond the generic cleanup.
    pub(crate) fn pipe_shutdown(&mut self, _id: u32) {}

    pub(crate) fn pipe_snapshot(&self, id: u32) -> Option<PipeSnapshot> {
        let i = self.instances.get(&id)?;
        let p = self.ps_ref(id)?;
        Some(PipeSnapshot {
            id,
            phase: i.phase,
            pipe: p.pipe,
            leaks: p.leaks.clone(),
            waits: p.waits.iter().map(|w| (w.cells.clone(), w.age)).collect(),
        })
    }
}
