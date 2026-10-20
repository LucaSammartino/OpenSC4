// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The riot (§6).

use super::layer::Timer;
use super::system::{Core, InstanceState};
use super::{history, instance_class, kind, prop, trunc_i32, zoom_of, TICK_PRIORITY};
use crate::curve::ResponseCurve;
use crate::network_manager::RiotHost;
use crate::property::PropertyHolder;

/// [`RiotHost`] over the disasters host: the one lock of
/// `network_manager::riot` reads the game through this (glue24).
struct HostRiot<'a>(&'a dyn super::DisasterHost);

impl RiotHost for HostRiot<'_> {
    fn tile_occupant(&self, x: i32, z: i32) -> Option<u64> {
        self.0.network_occupant_at(x, z)
    }
    fn add_ref(&self, token: u64) {
        self.0.add_ref(token);
    }
    fn release(&self, token: u64) {
        self.0.release(token);
    }
    fn unset_removal_flags(&self, occupant: u64, m: u32) {
        self.0.unset_removal_flags(occupant, m);
    }
    fn set_removal_flags(&self, occupant: u64, m: u32) {
        self.0.set_removal_flags(occupant, m);
    }
}

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

/// Defaults (§6.1, Q16): the two statics (3 riots, 5 leaders), then the
/// instance's own (6 stages of 30 s, leaders spawned 0, 1, 0, 1, 0, 0). The
/// zoom byte has no default: 0 [choice, §6.1].
pub const DEFAULT_MAX_RIOTS: u32 = 3;
pub const DEFAULT_MAX_LEADERS: u32 = 5;
pub const DEFAULT_STAGES: i32 = 6;
pub const DEFAULT_SECONDS_PER_STAGE: i32 = 30;
pub const DEFAULT_UNKNOWN_622: u32 = 3;
pub const DEFAULT_ZOOM: u32 = 0;
pub const DEFAULT_LEADERS_SPAWNED: [u32; 6] = [0, 1, 0, 1, 0, 0];
/// The police curve of an exemplar whose curve has no points (§6.1).
pub const DEFAULT_POLICE_CURVE: [(f32, f32); 3] = [(64.0, 0.0), (65.0, 1.0), (255.0, 3.0)];
/// The police limit of §6.4 step 2: `trunc(curve(coverage)) ≤ 1`.
pub const POLICE_LIMIT: i32 = 2;
/// The road search radius (§6.4 step 1).
pub const ROAD_SEARCH_RADIUS: i32 = 7;
/// The removal lock (§10) is `network_manager::RiotLock`, one implementation
/// for the whole engine (glue24); the privilege it clears is its
/// `PLAYER_REMOVAL`. The ±3 marking radius is that type's `mark`.
pub use crate::network_manager::PLAYER_REMOVAL;
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
    /// `0xEA400622`: stored and never read (§6.1, Q17).
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
                .unwrap_or_else(|| DEFAULT_LEADERS_SPAWNED.to_vec()),
            // An exemplar without points takes the fallback curve; no
            // exemplar leaves it empty (§6.1).
            police: match h {
                None => ResponseCurve::default(),
                Some(h) => crate::props::curve(h, riot_prop::POLICE_CURVE)
                    .filter(|c| !c.is_empty())
                    .unwrap_or_else(|| ResponseCurve::from_points(&DEFAULT_POLICE_CURVE)),
            },
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

/// A riot leader (§6.4, §6.5). A new one starts at stage −1, maximum
/// stage −1, counter 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Leader {
    pub automaton: u64,
    pub counter: i32,
    pub stage: i32,
    pub max_stage: i32,
    pub cell: (i32, i32),
    /// The network occupant it last stood on (AddRef'd; 0 = none).
    pub last_network: u64,
}

impl Default for Leader {
    fn default() -> Leader {
        Leader {
            automaton: 0,
            counter: 0,
            stage: -1,
            max_stage: -1,
            cell: (0, 0),
            last_network: 0,
        }
    }
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

    /// `AttemptDisasterStart(x, z)` (§6.2), x and z in cells: below the
    /// maximum active riots, a new instance in the manager with its first
    /// leader. Always null; the instance is found in the manager's list.
    pub(crate) fn riot_attempt_start(&mut self, x: i32, z: i32) -> Option<u32> {
        self.riot_start(x, z);
        None
    }

    /// The start itself, returning the new instance (for tests).
    pub(crate) fn riot_start(&mut self, x: i32, z: i32) -> Option<u32> {
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
    /// [choice]; `next` starts at 0, so the first step runs at the first
    /// tick, at elapsed 0 (§6.5).
    fn riot_init(&mut self, id: u32, x: i32, z: i32) {
        self.register_tick(id, TICK_PRIORITY);
        let now = self.host.now_ms();
        if let Some(r) = self.rs(id) {
            r.timer.restart(now);
            r.next_s = 0;
        }
        self.mark_riot_network(x, z);
    }

    /// `MarkRiotNetworkOccupants(x, z)` (network-manager.md §10): the
    /// network occupants of the cells within ±3 (x outer) are locked against
    /// the player's bulldozer. The bounds compare unsigned: within 3 cells of
    /// the west or north edge nothing is locked [quirk]. The lock itself is
    /// `network_manager::RiotLock` (glue24).
    fn mark_riot_network(&mut self, x: i32, z: i32) {
        let host = self.host.clone();
        self.riot_lock.mark(&HostRiot(host.as_ref()), x, z);
    }

    /// `ClearRiotNetworkOccupants`: every locked occupant gets its counter
    /// back and its reference released, in locking order [choice]. Run when
    /// the last riot instance leaves the manager and at the manager's
    /// `Shutdown`.
    pub(crate) fn clear_riot_network(&mut self) {
        let host = self.host.clone();
        self.riot_lock.clear(&HostRiot(host.as_ref()));
    }

    pub(crate) fn riot_locked(&self) -> Vec<u64> {
        self.riot_lock.locked()
    }

    /// `FindNearestRoadTile` (§6.4 step 1, Q18): the city's clockwise
    /// spiral (plumbing.md §7.2) from (x, z) over rings 0 to 7; cells
    /// outside the city are skipped. The first road cell wins.
    pub(crate) fn find_road_cell(&self, x: i32, z: i32) -> Option<(i32, i32)> {
        riot_spiral(x, z, ROAD_SEARCH_RADIUS)
            .into_iter()
            .filter(|&(px, pz)| self.inside_city(px, pz))
            .find(|&(px, pz)| self.host.is_road_cell(px, pz))
    }

    /// `AddRiotLeader(x, z, spawned)` (§6.4).
    fn add_riot_leader(&mut self, id: u32, x: i32, z: i32, spawned: bool) -> bool {
        let tuning = self.rt();
        let host = self.host.clone();
        let Some((rx, rz)) = self.find_road_cell(x, z) else {
            return false;
        };
        let c = tuning.police.eval(f32::from(host.police_coverage(rx, rz)));
        if !(trunc_i32(c) < POLICE_LIMIT && self.riot_below_max_leaders(id, &tuning)) {
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

    fn riot_below_max_leaders(&self, id: u32, tuning: &RiotTuning) -> bool {
        let leaders = self.rs_ref(id).map_or(0, |r| r.leaders.len());
        (leaders as u64) < u64::from(tuning.max_leaders)
    }

    /// The animation tick (§6.5): when the whole seconds reach `next`,
    /// `next := elapsed + 1` (no backlog) and one step runs.
    pub(crate) fn riot_tick(&mut self, id: u32) {
        let now = self.host.now_ms();
        let Some((elapsed, next)) = self.rs_ref(id).map(|r| (r.timer.elapsed_s(now), r.next_s)) else {
            return;
        };
        if elapsed >= next {
            if let Some(r) = self.rs(id) {
                r.next_s = elapsed + 1;
            }
            self.riot_update(id);
        }
    }

    /// One riot step (§6.5). The leader list is walked from the front, the
    /// **next** node remembered before each leader is processed (`0x0057E026`):
    /// a leader spawned in step 4 is appended at the end, so it is processed
    /// in the same pass when the spawning leader was not the last node, and
    /// not when it was *[compat]* (the §5.8 rule of the fire list, DS40).
    /// The coverage is read at the automaton's cell (its last known cell once
    /// it is gone).
    pub(crate) fn riot_update(&mut self, id: u32) {
        let tuning = self.rt();
        let host = self.host.clone();
        let mut at = 0usize;
        while let Some(l) = self.rs_ref(id).and_then(|r| r.leaders.get(at).copied()) {
            let a = l.automaton;
            // 0 = the end of the list, taken before this leader is processed.
            let next = self
                .rs_ref(id)
                .and_then(|r| r.leaders.get(at + 1).map(|x| x.automaton))
                .unwrap_or(0);
            let here = host.automaton_cell(a);
            let cell = here.unwrap_or(l.cell);
            let c = tuning.police.eval(f32::from(host.police_coverage(cell.0, cell.1)));
            let counter = l.counter.saturating_add(trunc_i32(1.0f32 - c));
            let stage = if here.is_none() { -1 } else { tuning.stage_of(counter) };
            let new_max = stage > l.max_stage && stage < tuning.stages;
            if let Some(x) = self
                .rs(id)
                .and_then(|r| r.leaders.iter_mut().find(|x| x.automaton == a))
            {
                x.counter = counter;
                x.cell = cell;
                x.stage = stage;
                if new_max {
                    x.max_stage = stage;
                }
            }
            if stage != l.stage {
                host.set_riot_effect(a, stage);
            }
            if new_max {
                let spawn = tuning.leaders_spawned.get(stage as usize).copied().unwrap_or(0);
                if self.riot_below_max_leaders(id, &tuning) && spawn > 0 {
                    for _ in 0..spawn {
                        self.add_riot_leader(id, cell.0, cell.1, true);
                    }
                }
                host.send_riot_message(a, stage);
            }
            if stage >= tuning.stages || stage < 0 {
                self.end_riot_leader(id, a);
            } else {
                // A walking leader keeps locking road (§6.5 step 6).
                let n = host.network_occupant_at(cell.0, cell.1).unwrap_or(0);
                if n != l.last_network {
                    self.mark_riot_network(cell.0, cell.1);
                    if n != 0 {
                        host.add_ref(n);
                    }
                    if l.last_network != 0 {
                        host.release(l.last_network);
                    }
                    if let Some(x) = self
                        .rs(id)
                        .and_then(|r| r.leaders.iter_mut().find(|x| x.automaton == a))
                    {
                        x.last_network = n;
                    }
                }
                host.riot_leader_tick(a);
            }
            if next == 0 {
                break;
            }
            // A step erases the leader it ended, which shifts the entries
            // after it; the remembered node is found by its automaton.
            match self
                .rs_ref(id)
                .and_then(|r| r.leaders.iter().position(|x| x.automaton == next))
            {
                Some(j) => at = j,
                None => break,
            }
        }
        self.riot_update_phase(id);
    }

    /// A leader ends (§6.5 step 5): the automaton set back and detached,
    /// its location dropped, the leader erased and its last network
    /// occupant released.
    fn end_riot_leader(&mut self, id: u32, a: u64) {
        self.host.end_riot_leader(a);
        self.unregister_location_by_occupant(id, a);
        let mut last = 0;
        if let Some(r) = self.rs(id) {
            if let Some(l) = r.leaders.iter().find(|l| l.automaton == a) {
                last = l.last_network;
            }
            r.leaders.retain(|l| l.automaton != a);
        }
        if last != 0 {
            self.host.release(last);
        }
    }

    /// The phase (§6.7). After 1 → 3 the instance stays in its manager and
    /// on the tick (Q19).
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
        }
    }

    /// Instance `Shutdown` (§3.1): every leader ended [choice], the
    /// locations dropped; the caller removes the instance from its manager
    /// (Q13).
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

/// The clockwise spiral of plumbing.md §7.2 around (x, z), rings 0 to
/// `rings`: the centre, then ring r from (x + r, z − r + 1) up +z to
/// (x + r, z + r), −x to (x − r, z + r), −z to (x − r, z − r) and +x to
/// (x + r, z − r).
pub(crate) fn riot_spiral(x: i32, z: i32, rings: i32) -> Vec<(i32, i32)> {
    let mut out = vec![(x, z)];
    for r in 1..=rings {
        let (x0, z0, x1, z1) = (x - r, z - r, x + r, z + r);
        out.extend((z0 + 1..=z1).map(|zz| (x1, zz)));
        out.extend((x0..x1).rev().map(|xx| (xx, z1)));
        out.extend((z0..z1).rev().map(|zz| (x0, zz)));
        out.extend((x0 + 1..=x1).map(|xx| (xx, z0)));
    }
    out
}
