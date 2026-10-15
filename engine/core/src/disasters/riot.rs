// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The riot (§6).

use super::layer::Timer;
use super::system::{Core, InstanceState};
use super::{history, instance_class, kind, prop, trunc_i32, zoom_of, CellRect, TICK_PRIORITY};
use crate::curve::ResponseCurve;
use crate::property::PropertyHolder;

/// Tuning properties of the "Riot" exemplar (§6.1).
pub mod riot_prop {
    pub const MAX_RIOTS: u32 = 0xCADB_6B3E;
    pub const MAX_LEADERS: u32 = 0x2ADB_6B6D;
    pub const STAGES: u32 = 0xEA40_0620;
    pub const SECONDS_PER_STAGE: u32 = 0xEA40_0621;
    pub const UNKNOWN_622: u32 = 0xEA40_0622;
    pub const LEADERS_SPAWNED: u32 = 0xEA40_0623;
    pub const POLICE_CURVE: u32 = 0xEA40_0624;
}

/// Defaults (§6.1). The maximum riots and leaders have no code default in
/// the spec; we use the literal limits of §6.2 and §6.4 (3 and 10)
/// [choice, Q16]. The zoom level falls back to the fire's default 4
/// [choice, Q16].
pub const DEFAULT_MAX_RIOTS: u32 = 3;
pub const DEFAULT_MAX_LEADERS: u32 = 10;
pub const DEFAULT_STAGES: i32 = 6;
pub const DEFAULT_SECONDS_PER_STAGE: i32 = 30;
pub const DEFAULT_UNKNOWN_622: u32 = 3;
pub const DEFAULT_ZOOM: u32 = 4;
/// The police limit of §6.4 step 2: `trunc(curve(coverage)) < 2`.
pub const POLICE_LIMIT: i32 = 2;
/// The road search radius (§6.4 step 1) and the riot-blocked radius (§6.3).
pub const ROAD_SEARCH_RADIUS: i32 = 7;
pub const BLOCK_RADIUS: i32 = 3;
/// The leader's offset inside its cell: `16 × RandomDoubleUniform()` per
/// axis (aura.md §6.16).
pub const LEADER_OFFSET: f64 = 16.0;

/// Shipped "Riot" values (§6.1).
pub const SHIPPED_LEADERS_SPAWNED: [u32; 6] = [0, 1, 0, 1, 0, 0];
pub const SHIPPED_POLICE_CURVE: [(f32, f32); 4] = [(0.0, 0.0), (64.0, 0.0), (65.0, 1.0), (255.0, 5.0)];

/// The riot tuning as read (§6.1).
#[derive(Clone, Debug, PartialEq)]
pub struct RiotTuning {
    pub max_riots: u32,
    pub max_leaders: u32,
    pub stages: i32,
    pub seconds_per_stage: i32,
    /// `0xEA400622`: read, meaning unknown, not used [question, Q17].
    pub unknown_622: u32,
    pub leaders_spawned: Vec<u32>,
    pub police: ResponseCurve,
    pub zoom: u32,
}

impl Default for RiotTuning {
    fn default() -> RiotTuning {
        RiotTuning::from_holder(None)
    }
}

impl RiotTuning {
    pub fn from_holder(h: Option<&PropertyHolder>) -> RiotTuning {
        let u = |id, d: u32| h.and_then(|h| crate::props::u32_of(h, id)).unwrap_or(d);
        RiotTuning {
            max_riots: u(riot_prop::MAX_RIOTS, DEFAULT_MAX_RIOTS),
            max_leaders: u(riot_prop::MAX_LEADERS, DEFAULT_MAX_LEADERS),
            stages: u(riot_prop::STAGES, DEFAULT_STAGES as u32) as i32,
            seconds_per_stage: u(riot_prop::SECONDS_PER_STAGE, DEFAULT_SECONDS_PER_STAGE as u32) as i32,
            unknown_622: u(riot_prop::UNKNOWN_622, DEFAULT_UNKNOWN_622),
            leaders_spawned: h
                .and_then(|h| crate::props::u32s(h, riot_prop::LEADERS_SPAWNED))
                .unwrap_or_default(),
            police: h
                .and_then(|h| crate::props::curve(h, riot_prop::POLICE_CURVE))
                .unwrap_or_default(),
            zoom: zoom_of(h, DEFAULT_ZOOM),
        }
    }

    /// The stage of a counter (§6.5 step 2): `counter / secondsPerStage`, or
    /// −1 when negative (a zero divisor gives stage 0 [choice]).
    pub fn stage_of(&self, counter: i32) -> i32 {
        if counter < 0 {
            -1
        } else if self.seconds_per_stage <= 0 {
            0
        } else {
            counter / self.seconds_per_stage
        }
    }
}

/// The shipped "Riot" exemplar (§6.1), for tests and tools.
pub fn shipped_riot() -> PropertyHolder {
    use crate::variant::{mem, Elem, Variant};
    let mut h = PropertyHolder::new();
    let one = |v: u32| Variant::Scalar(Elem::U32(v));
    h.add(riot_prop::MAX_RIOTS, one(3));
    h.add(riot_prop::MAX_LEADERS, one(10));
    h.add(riot_prop::STAGES, one(6));
    h.add(riot_prop::SECONDS_PER_STAGE, one(30));
    h.add(
        riot_prop::LEADERS_SPAWNED,
        Variant::Array {
            elem: mem::UINT32,
            items: SHIPPED_LEADERS_SPAWNED.iter().map(|&x| Elem::U32(x)).collect(),
        },
    );
    h.add(
        riot_prop::POLICE_CURVE,
        Variant::Array {
            elem: mem::FLOAT32,
            items: SHIPPED_POLICE_CURVE
                .iter()
                .flat_map(|&(x, y)| [Elem::F32(x), Elem::F32(y)])
                .collect(),
        },
    );
    h.add(prop::ZOOM_LEVEL, Variant::Scalar(Elem::U8(3)));
    h
}

/// A riot leader (§6.4, §6.5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Leader {
    pub automaton: u64,
    pub counter: i32,
    pub stage: i32,
    pub max_stage: i32,
    pub cell: (i32, i32),
}

#[derive(Clone, Debug, Default)]
pub(crate) struct RiotState {
    pub timer: Timer,
    pub next_s: u64,
    pub leaders: Vec<Leader>,
    pub start: (i32, i32),
}

/// A riot's observable state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RiotSnapshot {
    pub id: u32,
    pub phase: u32,
    pub leaders: Vec<Leader>,
    pub elapsed_s: u64,
    pub next_s: u64,
    pub start: (i32, i32),
}

impl Core {
    fn rs(&mut self, id: u32) -> Option<&mut RiotState> {
        match &mut self.instances.get_mut(&id)?.state {
            InstanceState::Riot(r) => Some(r),
            _ => None,
        }
    }

    fn rs_ref(&self, id: u32) -> Option<&RiotState> {
        match &self.instances.get(&id)?.state {
            InstanceState::Riot(r) => Some(r),
            _ => None,
        }
    }

    fn rt(&mut self) -> RiotTuning {
        if self.riot_tuning.is_none() {
            let h = self.host.tuning(super::tuning_instance::RIOT);
            self.riot_tuning = Some(RiotTuning::from_holder(h.as_deref()));
        }
        self.riot_tuning.clone().unwrap_or_default()
    }

    /// `AttemptDisasterStart(x, z)` (§6.2), x and z in cells.
    pub(crate) fn riot_attempt_start(&mut self, x: i32, z: i32) -> Option<u32> {
        let tuning = self.rt();
        if self.active_count(kind::RIOT) >= tuning.max_riots {
            return None;
        }
        let id = self.new_instance(
            kind::RIOT,
            instance_class::RIOT,
            InstanceState::Riot(Box::new(RiotState {
                start: (x, z),
                ..RiotState::default()
            })),
        );
        self.add_instance_to_list(kind::RIOT, id);
        self.riot_init(id, x, z);
        self.add_riot_leader(id, x, z, false);
        Some(id)
    }

    /// Instance `Init` (§6.3). The tick priority is the fire's `0xC0`
    /// [choice]; the first update runs one second after `Init` [choice].
    fn riot_init(&mut self, id: u32, x: i32, z: i32) {
        self.register_tick(id, TICK_PRIORITY);
        let now = self.host.now_ms();
        if let Some(r) = self.rs(id) {
            r.timer.restart(now);
            r.next_s = 1;
        }
        self.mark_riot_network(x, z);
    }

    fn mark_riot_network(&mut self, x: i32, z: i32) {
        let rect = self.clamp_rect(CellRect {
            x0: x.saturating_sub(BLOCK_RADIUS),
            z0: z.saturating_sub(BLOCK_RADIUS),
            x1: x.saturating_add(BLOCK_RADIUS),
            z1: z.saturating_add(BLOCK_RADIUS),
        });
        self.host.mark_riot_network(rect);
    }

    /// The nearest road cell (§6.4 step 1): rings of radius 0..7 around the
    /// start, each ring in row order (z, then x) [choice, Q18], coordinates
    /// clamped to the city (aura.md §6.16).
    pub(crate) fn find_road_cell(&self, x: i32, z: i32) -> Option<(i32, i32)> {
        let (cx, cz) = self.host.city_cells();
        let mx = (cx as i32 - 1).max(0);
        let mz = (cz as i32 - 1).max(0);
        for r in 0..=ROAD_SEARCH_RADIUS {
            for dz in -r..=r {
                for dx in -r..=r {
                    if dx.abs().max(dz.abs()) != r {
                        continue;
                    }
                    let px = x.saturating_add(dx).clamp(0, mx);
                    let pz = z.saturating_add(dz).clamp(0, mz);
                    if self.host.is_road_cell(px, pz) {
                        return Some((px, pz));
                    }
                }
            }
        }
        None
    }

    /// `AddRiotLeader(x, z, spawned)` (§6.4).
    fn add_riot_leader(&mut self, id: u32, x: i32, z: i32, spawned: bool) -> bool {
        let tuning = self.rt();
        let host = self.host.clone();
        let Some((rx, rz)) = self.find_road_cell(x, z) else {
            return false;
        };
        let c = tuning.police.eval(f32::from(host.police_coverage(rx, rz)));
        let leaders = self.rs_ref(id).map_or(0, |r| r.leaders.len());
        if !(trunc_i32(c) < POLICE_LIMIT && (leaders as u64) < u64::from(tuning.max_leaders)) {
            return false;
        }
        let (px, pz) = host.cell_corner_position(rx, rz);
        let ox = LEADER_OFFSET * self.rng_riot.uniform_f64();
        let oz = LEADER_OFFSET * self.rng_riot.uniform_f64();
        let Some(a) = host.create_riot_leader(px + ox as f32, pz + oz as f32) else {
            return false;
        };
        if let Some(r) = self.rs(id) {
            r.leaders.push(Leader {
                automaton: a,
                cell: (rx, rz),
                ..Leader::default()
            });
        }
        self.register_location(id, 0, 0, a, tuning.zoom, !spawned);
        self.mark_riot_network(rx, rz);
        self.riot_update_phase(id);
        true
    }

    /// The animation tick: one update per second of the instance's timer
    /// (one at most per tick, as the fire) [choice].
    pub(crate) fn riot_tick(&mut self, id: u32) {
        let now = self.host.now_ms();
        let due = self.rs_ref(id).is_some_and(|r| r.timer.elapsed_s(now) >= r.next_s);
        if due {
            if let Some(r) = self.rs(id) {
                r.next_s += 1;
            }
            self.riot_update(id);
        }
    }

    /// One riot update (§6.5). Leaders spawned during the pass first move
    /// in the next one [choice].
    pub(crate) fn riot_update(&mut self, id: u32) {
        let tuning = self.rt();
        let host = self.host.clone();
        let list: Vec<u64> = self
            .rs_ref(id)
            .map_or(Vec::new(), |r| r.leaders.iter().map(|l| l.automaton).collect());
        for a in list {
            let Some(l) = self
                .rs_ref(id)
                .and_then(|r| r.leaders.iter().find(|l| l.automaton == a))
                .copied()
            else {
                continue;
            };
            let Some(cell) = host.automaton_cell(a) else {
                self.end_riot_leader(id, a);
                continue;
            };
            let c = tuning.police.eval(f32::from(host.police_coverage(cell.0, cell.1)));
            let counter = l.counter.saturating_add(trunc_i32(1.0f32 - c));
            let stage = tuning.stage_of(counter);
            let mut max_stage = l.max_stage;
            let mut spawn = 0;
            let new_max = stage > l.max_stage && stage < tuning.stages;
            if new_max {
                max_stage = stage;
                spawn = tuning.leaders_spawned.get(stage as usize).copied().unwrap_or(0);
            }
            if let Some(x) = self
                .rs(id)
                .and_then(|r| r.leaders.iter_mut().find(|x| x.automaton == a))
            {
                x.counter = counter;
                x.cell = cell;
                x.stage = stage;
                x.max_stage = max_stage;
            }
            if stage != l.stage {
                host.set_riot_effect(a, stage);
            }
            if new_max {
                for _ in 0..spawn {
                    self.add_riot_leader(id, cell.0, cell.1, true);
                }
                host.send_riot_message(a, stage);
            }
            if stage >= tuning.stages || stage < 0 {
                self.end_riot_leader(id, a);
            } else {
                host.riot_leader_tick(a);
            }
        }
        self.riot_update_phase(id);
    }

    fn end_riot_leader(&mut self, id: u32, a: u64) {
        self.host.end_riot_leader(a);
        self.unregister_location_by_occupant(id, a);
        if let Some(r) = self.rs(id) {
            r.leaders.retain(|l| l.automaton != a);
        }
    }

    /// The phase (§6.7). After 1 → 3 the instance leaves the riot manager
    /// [choice, Q19].
    fn riot_update_phase(&mut self, id: u32) {
        let Some(phase) = self.instances.get(&id).map(|i| i.phase) else {
            return;
        };
        let has = self.rs_ref(id).is_some_and(|r| !r.leaders.is_empty());
        if has && phase == 0 {
            if let Some(i) = self.instances.get_mut(&id) {
                i.phase = 1;
            }
            self.disaster_has_begun(id);
            self.host.record_history(history::RIOTS, 1);
        } else if !has && phase == 1 {
            if let Some(i) = self.instances.get_mut(&id) {
                i.phase = 3;
            }
            self.unregister_locations(id);
            self.disaster_has_ended();
            self.instance_finish(id);
        }
    }

    /// Instance `Shutdown`: every leader ended, the locations dropped
    /// [choice, Q13].
    pub(crate) fn riot_shutdown(&mut self, id: u32) {
        let list: Vec<u64> = self
            .rs_ref(id)
            .map_or(Vec::new(), |r| r.leaders.iter().map(|l| l.automaton).collect());
        for a in list {
            self.end_riot_leader(id, a);
        }
        self.unregister_locations(id);
    }

    pub(crate) fn riot_snapshot(&self, id: u32) -> Option<RiotSnapshot> {
        let i = self.instances.get(&id)?;
        let r = self.rs_ref(id)?;
        let now = self.host.now_ms();
        Some(RiotSnapshot {
            id,
            phase: i.phase,
            leaders: r.leaders.clone(),
            elapsed_s: r.timer.elapsed_s(now),
            next_s: r.next_s,
            start: r.start,
        })
    }
}
