// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The lot developer's state and behaviour (§1–§11, §13).

use std::cell::RefCell;
use std::rc::Rc;

use super::boundary::BoundaryRule;
use super::geometry::{development_cost, rubble_count};
use super::terrain::TerrainView;
use super::tuning::Tuning;
use super::{
    lot_prop, lot_state, msg, object_type, reason, zot, BuildingProfile, BurntBuilding, CellRect, CfgObject, Cheat,
    LotDevMessage, LotDeveloperHost, TextureRecord, Vec3, AGENT_LEVELS, ANCHOR_GROUP, BASE_TEXTURE_EXTRA_DEFAULT,
    COFFEE_BAR_BUILDING, COFFEE_BAR_CFG, OCCUPANT_TYPE_BUILDING, OCCUPANT_TYPE_NETWORK, OCCUPANT_TYPE_PROP,
    PROP_NIGHT_LIGHT, PROP_ZOT_NUMBER, SUBSCRIBED, TEXTURE_RESOURCE,
};
use crate::rng::{CrtRand, FixedSeeds, RzRandom, SeedSource};
use crate::stl::StlHashSet;
use crate::stream::{InStream, OutStream, RefValue};
use crate::x87::{ftol32, r24};

/// Save record version (§13).
pub const RECORD_VERSION: u16 = 1;
/// The hard-coded night accumulator threshold in ms (§8.3, quirk 10).
pub const NIGHT_THRESHOLD_MS: u32 = 200;
/// [choice] The default daily work budget that replaces the 5 ms CPU-cycle
/// slicer (§4.1): effectively unbounded, so one delivery finishes the day.
pub const DEFAULT_DAILY_WORK_BUDGET: u32 = u32::MAX;
/// [choice] The most buildings `ClearLotBuildings` removes (§7.1).
pub const MAX_CLEAR_BUILDINGS: u32 = 1 << 16;
/// [choice] The most night-deque slots a save read allocates (§13).
pub const MAX_SAVED_BUILDINGS: usize = 1 << 20;
/// The terrain-change kind that skips the foundation re-check (§6.4, slot
/// 37): compared as a **whole 32-bit value**, never a byte.
pub const KIND_NO_FOUNDATION_RECHECK: u32 = 5;
/// The lot's cell size in metres, hard-coded by `ClearLotBlockingObjects`
/// (§6.2).
pub const CELL_METRES: f64 = 16.0;
/// The height a zot prop is raised above its anchor (§5.6).
pub const ZOT_HEIGHT: f64 = 3.0;
/// The altitude margin of the blocking-object box (§6.2).
pub const BLOCKING_TOP_MARGIN: f64 = 32.0;
/// The slope above which `UpdateLotObjectAlitudes` runs (§6.4).
pub const OBJECT_ALTITUDE_SLOPE: f32 = 0.25;
/// The empty-lot commute check's age threshold in days (§4.2 step 4).
pub const EMPTY_LOT_COMMUTE_DAYS: u32 = 30;
/// Days without zots after the city is established (§3.2).
pub const NO_ZOT_DAYS: u32 = 31;
/// The rubbles' non-burnt type table (§7.3).
pub const RUBBLE_TYPES: [u32; 6] = [
    0x1DCF_0000,
    0x1DD0_0000,
    0x1DD1_0000,
    0x1DD2_0000,
    0x1DD3_0000,
    0x1DD3_0000,
];
/// The rubbles' burnt corner types (§7.3).
pub const RUBBLE_BURNT_CORNER: [u32; 2] = [0x1C48_0000, 0x1C46_0000];
/// The rubbles' burnt interior types (§7.3).
pub const RUBBLE_BURNT_INTERIOR: [u32; 2] = [0x1C47_0000, 0x1C49_0000];
/// The animation-tick priority the lot developer registers with (§1.3 step
/// 10, §6.7.8): every frame.
pub const TICK_PRIORITY: u32 = 0xC0;

/// Which random draw of §10.2 a [`Draw`] record is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawKind {
    /// `TestDesirability`: `RandomSint32RangeUniform(0, 7)` on the temporary
    /// generator seeded `(x << 16) + z`.
    R1,
    /// `PickUnitToJoin`: `Uniform(candidates)`.
    R2,
    /// `TestCoffeeBar`: `Uniform(100)`.
    R3,
    /// `PickLotBaseTextureForLot`: `Uniform(n)` over the object's IDs.
    R4,
    /// `CreateRandomLotProp`: `Uniform(n)` over the expanded IDs.
    R5,
    /// `PickFloraTypeForLot`: `Uniform(n)` over the object's IDs.
    R6,
    /// `PickFloraTypeForLot`: `Uniform(size)` over the family set.
    R7,
    /// `AddLotFoundation` (slot 54): `Uniform(n)` over the foundation IDs.
    R8,
    /// Rubble corner: `Uniform(2)`.
    R9,
    /// Rubble corner: `Uniform(burnt ? 2 : 6)`.
    R10,
    /// Rubble interior orientation: `Uniform(4)`.
    R11,
    /// Rubble interior type: `Uniform(burnt ? 2 : 6)`.
    R12,
    /// Rubble interior x: `RandomDoubleRangeGaussian`.
    R13,
    /// Rubble interior z: `RandomDoubleRangeGaussian`.
    R14,
    /// Night shuffle: one `rand()` of the `random_shuffle` (§8.2).
    R15,
}

impl DrawKind {
    pub fn name(self) -> &'static str {
        match self {
            DrawKind::R1 => "R1",
            DrawKind::R2 => "R2",
            DrawKind::R3 => "R3",
            DrawKind::R4 => "R4",
            DrawKind::R5 => "R5",
            DrawKind::R6 => "R6",
            DrawKind::R7 => "R7",
            DrawKind::R8 => "R8",
            DrawKind::R9 => "R9",
            DrawKind::R10 => "R10",
            DrawKind::R11 => "R11",
            DrawKind::R12 => "R12",
            DrawKind::R13 => "R13",
            DrawKind::R14 => "R14",
            DrawKind::R15 => "R15",
        }
    }
}

/// One random draw (the debug trace of [`LotDeveloperStatics`]): `arg` is the
/// `n` of `RandomUint32Uniform(n)` (for R15 the deque size), 0 for the
/// Gaussian draws; `value` is the drawn value (for R15 the raw `rand()`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Draw {
    pub kind: DrawKind,
    pub arg: u32,
    pub value: f64,
}

/// The class-wide statics (§10.1): the file-local `cRZRandom` `0x00B49AF0`,
/// the process-wide C `rand()` state (R15) and the draw trace.
#[derive(Clone, Debug)]
pub struct LotDeveloperStatics {
    /// The file-local `cRZRandom` (§10.1), drawn by R2–R14.
    pub rng: RzRandom,
    /// The C runtime `rand()` the night shuffle draws from (foundations
    /// §12.6: state 1, never seeded by the game).
    pub crt: CrtRand,
    /// The clock values the generators fall back to (R1's temporary
    /// generator reads one only when its seed is `0xFFFFFFFF`).
    pub seeds: FixedSeeds,
    /// Every draw, when tracing is on (debug and conformance).
    pub trace: Option<Vec<Draw>>,
}

/// The statics as one object handed to the instance (one per process, like
/// the original's static).
pub type SharedStatics = Rc<RefCell<LotDeveloperStatics>>;

impl LotDeveloperStatics {
    /// The statics with the file-local generator seeded with `seed` and
    /// `rand()` at its process-start state 1. Tests use a fixed seed; the
    /// original's seed is the local wall clock XOR [`crate::rng::file_seed::LOT_DEVELOPER`].
    pub fn new(seed: u32, seeds: FixedSeeds) -> SharedStatics {
        Rc::new(RefCell::new(LotDeveloperStatics {
            rng: RzRandom::new(seed, &seeds),
            crt: CrtRand::default(),
            seeds,
            trace: None,
        }))
    }

    /// The statics seeded from the session seed `session` (foundations §6.3
    /// *[choice]*): `derive_seed(session, 0x6C09C0F4)`.
    pub fn from_session(session: u32, seeds: FixedSeeds) -> SharedStatics {
        LotDeveloperStatics::new(
            crate::seeding::seed(session, crate::seeding::keys::LOT_DEVELOPER),
            seeds,
        )
    }

    /// Appends a draw to the trace (when tracing is on).
    pub fn record(&mut self, kind: DrawKind, arg: u32, value: f64) {
        if let Some(t) = self.trace.as_mut() {
            t.push(Draw { kind, arg, value });
        }
    }
}

/// One lot developer (§1).
pub struct LotDeveloper {
    host: Rc<dyn LotDeveloperHost>,
    statics: SharedStatics,
    initialised: bool,
    // ---- persistent (§2.1) ----
    /// `vertsX`, the extents (saved), the cell counts `Init` stores, the
    /// tuning (§3.5) and the boundary-map switch: everything the terrain and
    /// geometry slots read ([`TerrainView`]).
    view: TerrainView,
    /// The night deque: one entry per building, `None` for a slot a failed
    /// save read left null.
    night: Vec<Option<u64>>,
    night_cursor: usize,
    night_mode: u32,
    lit_flag: bool,
    // ---- not saved (§2.2) ----
    daily_state: u32,
    iterator: Option<u64>,
    visited: i32,
    net_changed: bool,
    unit_changed: bool,
    net_snap: bool,
    unit_snap: bool,
    zots_allowed: bool,
    no_zot_until_day: u32,
    can_check_commute: bool,
    force_empty: bool,
    night_acc: u32,
    night_batch: u32,
    daily_work_budget: u32,
}

impl LotDeveloper {
    /// The constructor's state (§1.2, §2): daily state 3, both latches set,
    /// tuning defaults, `nightAcc` 200, `nightBatch` 8.
    pub fn new(host: Rc<dyn LotDeveloperHost>, statics: SharedStatics) -> LotDeveloper {
        LotDeveloper {
            host,
            statics,
            initialised: false,
            view: TerrainView::default(),
            night: Vec::new(),
            night_cursor: 0,
            night_mode: 0,
            lit_flag: false,
            daily_state: 3,
            iterator: None,
            visited: 0,
            net_changed: true,
            unit_changed: true,
            net_snap: false,
            unit_snap: false,
            zots_allowed: false,
            no_zot_until_day: 0,
            can_check_commute: false,
            force_empty: false,
            night_acc: NIGHT_THRESHOLD_MS,
            night_batch: 8,
            daily_work_budget: DEFAULT_DAILY_WORK_BUDGET,
        }
    }

    pub fn statics(&self) -> SharedStatics {
        self.statics.clone()
    }

    pub fn is_initialised(&self) -> bool {
        self.initialised
    }

    pub fn tuning(&self) -> &Tuning {
        &self.view.tuning
    }

    /// Replaces the tuning (tests and hosts that read it themselves).
    pub fn set_tuning(&mut self, t: Tuning) {
        self.view.tuning = t;
    }

    // ---- saved state accessors (§2.1) ----

    pub fn verts_x(&self) -> i32 {
        self.view.verts_x
    }
    pub fn extent_x(&self) -> f32 {
        self.view.extent_x
    }
    pub fn extent_z(&self) -> f32 {
        self.view.extent_z
    }
    pub fn night_len(&self) -> usize {
        self.night.len()
    }
    /// The night deque in order (`None` for a null slot).
    pub fn night(&self) -> &[Option<u64>] {
        &self.night
    }
    pub fn night_cursor(&self) -> usize {
        self.night_cursor
    }
    pub fn night_mode(&self) -> u32 {
        self.night_mode
    }
    pub fn lit_flag(&self) -> bool {
        self.lit_flag
    }

    pub fn set_night_mode(&mut self, m: u32) {
        self.night_mode = m;
    }
    pub fn set_lit_flag(&mut self, f: bool) {
        self.lit_flag = f;
    }
    pub fn set_night_cursor(&mut self, i: usize) {
        self.night_cursor = i;
    }

    // ---- unsaved state accessors (§2.2) ----

    pub fn daily_state(&self) -> u32 {
        self.daily_state
    }
    pub fn visited(&self) -> i32 {
        self.visited
    }
    pub fn net_changed(&self) -> bool {
        self.net_changed
    }
    pub fn unit_changed(&self) -> bool {
        self.unit_changed
    }
    pub fn net_snap(&self) -> bool {
        self.net_snap
    }
    pub fn unit_snap(&self) -> bool {
        self.unit_snap
    }
    pub fn zots_allowed(&self) -> bool {
        self.zots_allowed
    }
    pub fn no_zot_until_day(&self) -> u32 {
        self.no_zot_until_day
    }
    pub fn can_check_commute(&self) -> bool {
        self.can_check_commute
    }
    /// `GetForceEmptyLotBaseTextures` (slot 46).
    pub fn force_empty(&self) -> bool {
        self.force_empty
    }
    pub fn night_acc(&self) -> u32 {
        self.night_acc
    }
    pub fn night_batch(&self) -> u32 {
        self.night_batch
    }
    pub fn cell_counts(&self) -> (i32, i32) {
        (self.view.cell_count_x, self.view.cell_count_z)
    }

    pub fn set_daily_state(&mut self, s: u32) {
        self.daily_state = s;
    }
    pub fn set_visited(&mut self, v: i32) {
        self.visited = v;
    }
    pub fn set_net_changed(&mut self, v: bool) {
        self.net_changed = v;
    }
    pub fn set_unit_changed(&mut self, v: bool) {
        self.unit_changed = v;
    }
    pub fn set_no_zot_until_day(&mut self, d: u32) {
        self.no_zot_until_day = d;
    }
    pub fn set_can_check_commute(&mut self, v: bool) {
        self.can_check_commute = v;
    }
    pub fn set_night_acc(&mut self, v: u32) {
        self.night_acc = v;
    }
    pub fn set_night_batch(&mut self, v: u32) {
        self.night_batch = v;
    }

    /// [choice] The work budget that replaces the 5 ms CPU-cycle slicer
    /// (§4.1): at most this many lots are updated per delivery of the day
    /// message. It never changes which lots are visited or in what order.
    pub fn daily_work_budget(&self) -> u32 {
        self.daily_work_budget
    }

    pub fn set_daily_work_budget(&mut self, n: u32) {
        self.daily_work_budget = n;
    }

    /// Sets the force-empty flag without the every-lot walk of slot 47
    /// (hosts and tests that place the flag directly).
    pub fn set_force_empty_direct(&mut self, f: bool) {
        self.force_empty = f;
    }

    // ---- lifecycle (§1.3, §1.4) ----

    /// `Init` (slot 3, §1.3). A second call returns true at once.
    pub fn init(&mut self) -> bool {
        if self.initialised {
            return true;
        }
        // 2. the tuning.
        self.read_tuning();
        // 3. the services; nothing is subscribed when one is missing.
        if !self.host.acquire_services() {
            return false;
        }
        // 4. the cell counts (stored on every Init).
        let (cx, cz) = self.host.city_cell_counts();
        self.view.cell_count_x = cx;
        self.view.cell_count_z = cz;
        // 5. the terrain extents.
        let (tx, tz) = self.host.terrain_cell_counts();
        let cw = f64::from(self.host.terrain_cell_width());
        self.view.verts_x = tx.wrapping_add(1);
        self.view.extent_x = (r24(f64::from(tx) * cw)) as f32;
        self.view.extent_z = (r24(f64::from(tz) * cw)) as f32;
        // 7. daily state.
        self.daily_state = 0;
        // 8. the simulator and the agent.
        if !self.host.has_simulator() {
            return false;
        }
        for (level, message) in AGENT_LEVELS {
            self.host.add_agent(level, message);
        }
        // 9. the message server and the nine subscriptions.
        if !self.host.acquire_message_server() {
            return false;
        }
        for &m in SUBSCRIBED.iter() {
            self.host.subscribe(m);
        }
        // 10. the animation-tick manager.
        self.host.register_animation_tick();
        // 11. the boundary rules.
        self.view.boundary_ready = true;
        // 12. force a base-texture refresh: flip the flag, then restore it,
        // so slot 47 sees a change and walks every lot.
        let original = self.force_empty;
        self.force_empty = !original;
        self.set_force_empty_lot_base_texture(original);
        // 13.
        self.can_check_commute = false;
        self.no_zot_until_day = 0;
        self.initialised = true;
        true
    }

    /// `ReadSimulatorTunableValues` (§3.5). The night interval goes into the
    /// accumulator and the night count into the batch size (quirk 10).
    pub fn read_tuning(&mut self) {
        let h = self.host.tuning();
        self.view
            .tuning
            .read(h.as_deref(), &mut self.night_acc, &mut self.night_batch);
    }

    /// `Shutdown` (slot 4, §1.4): always true.
    pub fn shutdown(&mut self) -> bool {
        if self.initialised {
            self.initialised = false;
            for b in std::mem::take(&mut self.night).into_iter().flatten() {
                self.host.building_release(b);
            }
            self.night_cursor = 0;
            self.host.remove_agent();
            self.host.unregister_animation_tick();
            for &m in SUBSCRIBED.iter() {
                self.host.unsubscribe(m);
            }
            self.host.release_message_server();
            self.host.release_services();
            self.view.boundary_ready = false;
            if let Some(it) = self.iterator.take() {
                self.host.safe_lot_iterator_free(it);
            }
        }
        true
    }

    /// `DoMessage` (sub-object +4 slot 3, §3.2).
    pub fn do_message(&mut self, m: &LotDevMessage) -> bool {
        match m.msg_type {
            msg::DAY_NIGHT_CHANGED => {
                self.on_day_night_changed(m);
                true
            }
            msg::OCCUPANT_INSERTED => {
                if m.data3 == OCCUPANT_TYPE_BUILDING {
                    if let Some(occ) = m.void1 {
                        self.on_building_inserted(occ);
                    }
                } else if m.data3 == OCCUPANT_TYPE_NETWORK {
                    self.net_changed = true;
                    self.unit_changed = true;
                }
                true
            }
            msg::OCCUPANT_REMOVED => {
                if m.data3 == OCCUPANT_TYPE_BUILDING {
                    if let Some(occ) = m.void1 {
                        self.on_building_removed(occ);
                    }
                } else if m.data3 == OCCUPANT_TYPE_NETWORK {
                    self.net_changed = true;
                    self.unit_changed = true;
                }
                true
            }
            msg::LOT_STATE_CHANGED => {
                self.on_lot_state_changed(m);
                true
            }
            msg::LOT_CONDITION_CHANGED => {
                self.on_lot_condition_changed(m);
                true
            }
            // Subscribed but ignored (§12.1 quirk 15).
            msg::TERRAIN_CHANGED => true,
            msg::POWER_CHANGED => {
                let rect = CellRect::new(m.data1 as i32, m.data2 as i32, m.data3 as i32, m.data4 as i32);
                self.do_power_changed(rect);
                true
            }
            msg::CITY_ESTABLISHED => {
                self.no_zot_until_day = self.host.sim_date().wrapping_add(NO_ZOT_DAYS);
                true
            }
            msg::TRAFFIC_MAP_CHANGED => {
                // NaN > 0.0 is false, as in the original.
                self.can_check_commute = self.host.jobs_demand().is_some_and(|v| v > 0.0);
                true
            }
            msg::ANIMATION_TICK => {
                if self.night_mode == 0 {
                    return false;
                }
                self.on_animation_tick(m.data2 / 1000);
                true
            }
            msg::SIM_NEW_DAY => self.do_daily_simulation(),
            _ => true,
        }
    }

    // ---- the daily pass (§4) ----

    /// `DoDailySimulation` (level-4 agent, §4.1). Returns the "finished for
    /// today" flag: false makes the simulator re-deliver the day message.
    pub fn do_daily_simulation(&mut self) -> bool {
        if self.daily_state == 0 || self.daily_state == 3 {
            match self.iterator {
                None => self.iterator = Some(self.host.safe_lot_iterator()),
                Some(it) => self.host.safe_lot_iterator_rewind(it),
            }
            if self.host.lot_count() <= 0 {
                return true;
            }
            self.visited = 0;
            self.net_snap = self.net_changed;
            self.unit_snap = self.unit_changed;
            self.net_changed = false;
            self.unit_changed = false;
            self.zots_allowed = self.host.sim_date() > self.no_zot_until_day;
            self.daily_state = 1;
        }
        if self.daily_state != 1 {
            return true;
        }
        let n = self.host.lot_count();
        if n <= 0 {
            self.daily_state = 3;
            return true;
        }
        let per_day = n / 28 + 1;
        let mut budget = self.daily_work_budget;
        while self.visited < per_day {
            if budget == 0 {
                break;
            }
            budget -= 1;
            let Some(it) = self.iterator else {
                self.daily_state = 3;
                break;
            };
            let Some(lot) = self.host.safe_lot_iterator_next(it) else {
                self.daily_state = 3;
                break;
            };
            self.update_lot(lot);
            self.visited += 1;
        }
        if self.visited >= per_day {
            self.visited = 0;
            return true;
        }
        false
    }

    /// The lots one day processes, `n / 28 + 1` (§4.1).
    pub fn per_day(&self) -> i32 {
        let n = self.host.lot_count();
        if n <= 0 {
            0
        } else {
            n / 28 + 1
        }
    }

    /// `UpdateLot(lot)` (§4.2). Returns "state changed".
    pub fn update_lot(&mut self, lot: u64) -> bool {
        let (x, z) = self.host.lot_location(lot);
        // 1. network refresh (`RotateLotToFaceRoad` is dead, quirk 2).
        if self.net_snap {
            self.host.lot_update_road_access(lot);
        }
        // 2–4. the building tests.
        let changed = self.update_lot_building(lot, x, z);
        // 5. the business unit.
        self.update_lot_unit(lot);
        changed
    }

    /// `UpdateLot` steps 2–4 (§4.2).
    fn update_lot_building(&mut self, lot: u64, x: i32, z: i32) -> bool {
        let btype = self.host.lot_building_type(lot, false);
        if btype == 0 {
            // 4. an empty lot, only when zots are allowed.
            if self.zots_allowed {
                if self.host.lot_days_since_state_change(lot) > EMPTY_LOT_COMMUTE_DAYS && self.can_check_commute {
                    // A temporary profile of purpose 1 for zones 1–3 and
                    // purpose 3 for zones 4–6; other zones get no call.
                    let purpose = match self.host.lot_zone_type(lot) {
                        1..=3 => Some(1u32),
                        4..=6 => Some(3),
                        _ => None,
                    };
                    if let Some(purpose) = purpose {
                        let tmp = BuildingProfile {
                            purpose: purpose as u8,
                            ..BuildingProfile::default()
                        };
                        self.test_travel(lot, &tmp, x, z);
                    }
                }
                if is_empty_lot_suitable_for_zot(self.host.lot_zone_type(lot), x, z) {
                    self.test_power(lot, x, z);
                }
            }
            return false;
        }
        let p = self.host.building_profile(btype);
        if p.purpose == 0 {
            // 3. a civic / plopped building.
            let powered = self.test_power(lot, x, z) == 2;
            match self.host.lot_condition(lot) {
                0 => {
                    if !powered {
                        self.host.lot_set_condition(lot, 1);
                    }
                    // "stop (no travel test)" ends the purpose-0 block; step
                    // 5 still runs *[inferred]*.
                    return false;
                }
                1 => {
                    if powered {
                        self.host.lot_set_condition(lot, 0);
                    }
                }
                5 | 6 => self.host.lot_set_condition(lot, 0),
                _ => {}
            }
            if powered {
                self.test_travel(lot, &p, x, z);
            }
            return false;
        }
        // 3. a grown building: all four tests always run, in this order; the
        // last failing one gives the reason.
        let s = self.host.lot_state(lot);
        let mut new = s;
        let mut why = 0u32;
        if !self.test_desirability(lot, x, z) {
            new = lot_state::ABANDONED;
            why = reason::LOW_DESIRABILITY;
        }
        if self.test_power(lot, x, z) == 0 {
            new = lot_state::ABANDONED;
            why = reason::NO_POWER;
        }
        if !self.test_water(lot, x, z) {
            new = lot_state::ABANDONED;
            why = reason::NO_WATER;
        }
        if !self.test_travel(lot, &p, x, z) {
            new = lot_state::ABANDONED;
            why = reason::COMMUTE;
        }
        let mut changed = false;
        if new != s {
            self.host.lot_set_state(lot, new, 0, why);
            changed = true;
        }
        let c = self.test_condition(lot, x, z);
        self.host.lot_set_condition(lot, c);
        if self.host.cheat(Cheat::CoffeeBar) && p.purpose == 2 {
            self.test_coffee_bar(lot);
        }
        changed
    }

    /// `UpdateLot` step 5 (§4.2).
    fn update_lot_unit(&mut self, lot: u64) {
        match self.host.lot_unit(lot) {
            Some(unit) => {
                if !is_first_member(&self.host, unit, lot) {
                    return;
                }
                if self.unit_snap {
                    let mut u = Some(unit);
                    self.update_travel_designates(&mut u);
                }
                if let Some(u2) = self.host.lot_unit(lot) {
                    if is_first_member(&self.host, u2, lot) {
                        self.update_travel_populations(u2);
                    }
                }
            }
            None => {
                self.host.lot_reset_travel_population(lot);
                self.host.lot_add_travel_population(lot, lot);
                if self.host.lot_is_livable(lot) && !self.host.lot_has_road_access(lot) {
                    self.pick_unit_to_join(lot);
                }
            }
        }
    }

    // ---- the tests (§5) ----

    /// `TestDesirability` (§5.1). Draws R1 on a temporary generator seeded
    /// `(x << 16) + z`, so each location has a fixed tolerance of `r + 2`.
    pub fn test_desirability(&mut self, lot: u64, x: i32, z: i32) -> bool {
        let n = self.read_counter(lot, lot_prop::LOW_DESIRABILITY);
        if !self.host.lot_is_livable(lot) {
            self.write_counter(lot, lot_prop::LOW_DESIRABILITY, 0);
            return true;
        }
        let p = self.host.building_profile(self.host.lot_building_type(lot, false));
        let dev_type = self
            .host
            .developer_type(u32::from(p.purpose), self.host.lot_occupant_wealth(lot));
        let Some(dev) = self.host.developer_of_type(dev_type) else {
            self.write_counter(lot, lot_prop::LOW_DESIRABILITY, 0);
            return true;
        };
        if self.host.dev_purpose(dev) == 5 {
            // The counter is untouched.
            return true;
        }
        let v = self.host.dev_desirability(dev, x, z);
        let d = (f32::from(v)) * 0.01f32;
        // Spec §5.1 `not (d < abandonThreshold)`: a NaN threshold counts as
        // "not low", so keep the strict `<` rather than rewriting it as `>=`.
        let low = d < self.view.tuning.thresholds[0];
        if !low {
            self.write_counter(lot, lot_prop::LOW_DESIRABILITY, 0);
            return true;
        }
        let seed = ((x as u32) << 16).wrapping_add(z as u32);
        let r = {
            let s = self.statics.borrow();
            let mut g = RzRandom::new(seed, &s.seeds);
            g.range_i32(0, 7)
        };
        self.record(DrawKind::R1, 7, f64::from(r));
        let np = n.wrapping_add(1);
        self.write_counter(lot, lot_prop::LOW_DESIRABILITY, np);
        np <= (r as u32).wrapping_add(2)
    }

    /// The tolerance `r + 2` of the location `(x, z)` (§5.1, §16 test 1).
    pub fn desirability_tolerance(x: i32, z: i32, seeds: &dyn SeedSource) -> u32 {
        let seed = ((x as u32) << 16).wrapping_add(z as u32);
        let mut g = RzRandom::new(seed, seeds);
        (g.range_i32(0, 7) as u32).wrapping_add(2)
    }

    /// `TestPower` (§5.2): 2 powered, 1 unpowered within the grace of two
    /// visits, 0 unpowered for the third time or more.
    pub fn test_power(&mut self, lot: u64, x: i32, z: i32) -> i32 {
        let n = self.read_counter(lot, lot_prop::NO_POWER);
        let b = self.host.lot_building(lot);
        let needs = b.is_none() || self.host.lot_needs_power(lot);
        let (cx, cz) = match b {
            Some(b) => self.building_centre_cell(b),
            None => (x, z),
        };
        let powered = self.host.cheat(Cheat::IgnorePower) || !needs || self.host.is_powered(cx, cz);
        if powered != self.host.lot_has_power(lot) {
            self.host.lot_set_has_power(lot, powered);
            let c = self.host.lot_condition(lot);
            if !powered && c == 0 {
                self.host.lot_set_condition(lot, 1);
            }
            if powered && c == 1 {
                self.host.lot_set_condition(lot, 0);
            }
        }
        if !powered {
            let np = n.wrapping_add(1);
            self.write_counter(lot, lot_prop::NO_POWER, np);
            self.add_building_zot_prop(lot, zot::POWER_PROP, zot::POWER);
            return if np <= 2 { 1 } else { 0 };
        }
        self.write_counter(lot, lot_prop::NO_POWER, 0);
        // An empty lot's power zot is not removed here (quirk 5).
        if b.is_some() && self.current_zot(lot) == zot::POWER {
            self.remove_lot_prop(lot, zot::POWER);
            self.write_zot(lot, zot::NONE);
        }
        2
    }

    /// `TestWater` (§5.3). Uses the lot's location cell, not the building
    /// centre.
    pub fn test_water(&mut self, lot: u64, x: i32, z: i32) -> bool {
        let n = self.read_counter(lot, lot_prop::NO_WATER);
        let mut needs = false;
        let s = self.host.lot_state(lot);
        if (2..=4).contains(&s) {
            let p = self.host.building_profile(self.host.lot_building_type(lot, false));
            if p.purpose == 0 {
                needs = true;
            } else if let (Some(dev), Some(cfg)) = (
                self.host.developer_for(u32::from(p.purpose), u32::from(p.wealth)),
                self.host.lot_configuration(lot),
            ) {
                needs = self.host.cfg_growth_stage(cfg) > self.host.dev_water_supply_cap(dev);
            }
        }
        let watered = self.host.cheat(Cheat::IgnorePlumbing) || self.host.is_cell_watered(x, z);
        if !watered && needs {
            let np = n.wrapping_add(1);
            self.write_counter(lot, lot_prop::NO_WATER, np);
            self.add_building_zot_prop(lot, zot::WATER_PROP, zot::WATER);
            return np <= 2;
        }
        self.write_counter(lot, lot_prop::NO_WATER, 0);
        if self.current_zot(lot) == zot::WATER {
            self.remove_lot_prop(lot, zot::WATER);
            self.write_zot(lot, zot::NONE);
        }
        true
    }

    /// `TestTravel` (§5.4). Road access and the commute share one counter.
    pub fn test_travel(&mut self, lot: u64, p: &BuildingProfile, x: i32, z: i32) -> bool {
        let s = self.host.lot_state(lot);
        if !matches!(s, 1 | 3 | 4) {
            return self.travel_pass(lot, false);
        }
        let mut trip_ok = true;
        let purpose = u32::from(p.purpose);
        let road = if purpose == 1 {
            let r = self.host.lot_has_road_access(lot) && self.host.lot_has_job_access(lot);
            trip_ok = self.host.trip_length(x, z) != 0;
            r
        } else if (5..=8).contains(&purpose) {
            self.host.lot_has_organic_road_access(lot, purpose)
        } else {
            let jobs = self.host.lot_travel_jobs(lot, 1) > 0.0
                || self.host.lot_travel_jobs(lot, 2) > 0.0
                || self.host.lot_travel_jobs(lot, 3) > 0.0;
            if !jobs {
                return self.travel_pass(lot, false);
            }
            self.host.lot_has_road_access(lot)
        };
        if road && trip_ok {
            return self.travel_pass(lot, false);
        }
        let n = self.read_counter(lot, lot_prop::TRAVEL);
        let np = n.wrapping_add(1);
        self.write_counter(lot, lot_prop::TRAVEL, np);
        if !road {
            self.add_building_zot_prop(lot, zot::ROAD_PROP, zot::ROAD);
            return np <= 2;
        }
        if s == lot_state::OCCUPIED && np > 6 {
            self.add_building_zot_prop(lot, zot::COMMUTE_PROP, zot::COMMUTE);
            return false;
        }
        if s == lot_state::OCCUPIED && np > 2 {
            self.add_building_zot_prop(lot, zot::COMMUTE_PROP, zot::COMMUTE);
            return true;
        }
        self.travel_pass(lot, true)
    }

    /// `PASS` (`keep_counter` false) and `PASS_KEEP_COUNTER` of §5.4.
    fn travel_pass(&mut self, lot: u64, keep_counter: bool) -> bool {
        if !keep_counter && self.host.lot_property(lot, lot_prop::TRAVEL).is_some() {
            self.write_counter(lot, lot_prop::TRAVEL, 0);
        }
        self.remove_building_zot_prop(lot, zot::ROAD);
        if self.current_zot(lot) == zot::COMMUTE {
            self.remove_lot_prop(lot, zot::COMMUTE);
            self.write_zot(lot, zot::NONE);
        }
        true
    }

    /// `TestCondition` (§5.5): the condition `UpdateLot` sets.
    pub fn test_condition(&mut self, lot: u64, _x: i32, _z: i32) -> u32 {
        let s = self.host.lot_state(lot);
        let c = self.host.lot_condition(lot);
        let stay_new = u32::from(self.view.tuning.stay_new_time);
        if s == lot_state::OCCUPIED {
            let p = self.host.building_profile(self.host.lot_building_type(lot, false));
            let mut d = i32::from(p.wealth).wrapping_sub(self.host.lot_occupant_wealth(lot) as i32);
            let b = (c as i32).wrapping_sub(1) > d;
            if b || self.host.lot_days_since_state_change(lot) > stay_new {
                let f = self.host.lot_occupancy(lot);
                let distress = self.view.tuning.thresholds[1];
                let renew = self.view.tuning.thresholds[2];
                // Spec §5.5 lists the distress and the mismatched-wealth renew
                // cases as separate branches; they happen to share a body.
                #[allow(clippy::if_same_then_else)]
                if f < distress {
                    d += 1;
                } else if b && f < renew {
                    d += 1;
                }
            }
            if d > 2 {
                return 3;
            }
            if d == 0 {
                return if self.host.lot_property(lot, lot_prop::NO_POWER).is_some_and(|v| v != 0) {
                    1
                } else {
                    0
                };
            }
            if d < 0 {
                return c;
            }
            return (d + 1) as u32;
        }
        if s == lot_state::ABANDONED {
            let mut c = c;
            if c == 0 {
                c = 1;
            }
            if self.host.lot_days_since_state_change(lot) > stay_new && c <= 3 {
                return c + 1;
            }
            return c;
        }
        c
    }

    /// `TestCoffeeBar` (§5.8, cheat only). R3 is always drawn first.
    pub fn test_coffee_bar(&mut self, lot: u64) {
        let r = self.draw_u32(DrawKind::R3, 100);
        if self.host.lot_is_historical(lot) {
            return;
        }
        let s = self.host.lot_state(lot);
        if s != lot_state::OCCUPIED && s != lot_state::ABANDONED {
            return;
        }
        if r >= 9 {
            return;
        }
        let Some(cfg) = self.host.lot_configuration(lot) else {
            return;
        };
        if self.host.cfg_id(cfg) == COFFEE_BAR_CFG {
            return;
        }
        if self.host.cfg_growth_stage(cfg) > 3 {
            return;
        }
        let Some(target) = self.host.get_lot_configuration(COFFEE_BAR_CFG) else {
            return;
        };
        if self.host.lot_size(lot) != self.host.cfg_size(target) {
            return;
        }
        self.host.demolish_lot(lot);
        let cap = self.host.building_profile(COFFEE_BAR_BUILDING).capacity;
        if self.host.claim_construction_resources(cap) {
            self.start_construction(lot, Some(target), COFFEE_BAR_BUILDING, false);
            self.host.lot_set_historical(lot, true);
        }
    }

    // ---- zots (§5.6) ----

    /// `AddBuildingZotProp(lot, propType, zot)` (§5.6). Lower numbers win.
    pub fn add_building_zot_prop(&mut self, lot: u64, prop_type: u32, which: u32) {
        let b = self.host.lot_building(lot);
        let cfg = self.host.lot_configuration(lot);
        // The road notification is posted before the priority check
        // (quirk 4).
        if which == zot::ROAD {
            self.host.post_message(msg::LOT_DEVELOPER_NOTIFICATION, 1, Some(lot));
        }
        let cur = self.current_zot(lot);
        if cur != 0 && which >= cur {
            return;
        }
        self.remove_lot_prop(lot, cur);
        self.write_zot(lot, which);
        let has_object = cfg.is_some_and(|c| self.host.cfg_primary_building_object(c).is_some());
        let mut pos = match (b, has_object) {
            (Some(b), true) => {
                let (p, _orient) = self.building_position(lot);
                let (_min, max) = self.host.occupant_bounding_box(b);
                [p[0], max[1], p[2]]
            }
            _ => self.host.lot_center(lot),
        };
        pos[1] = (r24(f64::from(pos[1]) + ZOT_HEIGHT)) as f32;
        let orient = self.host.view_rotation();
        let Some(occ) = self.host.prop_add(prop_type, pos, orient) else {
            return;
        };
        self.host.occupant_set_removal_flags(occ, 3);
        self.host.occupant_set_property(occ, PROP_ZOT_NUMBER, which);
    }

    /// `RemoveBuildingZotProp(lot, zot)` (§5.6).
    pub fn remove_building_zot_prop(&mut self, lot: u64, which: u32) {
        if which == zot::ROAD {
            self.host.post_message(msg::LOT_DEVELOPER_NOTIFICATION, 2, Some(lot));
        }
        let cur = self.current_zot(lot);
        if which == zot::NONE || cur == which {
            if cur != 0 {
                self.remove_lot_prop(lot, cur);
            }
            self.write_zot(lot, zot::NONE);
        }
    }

    /// Slot 11 `RemoveBuildingZotProp(lot)`: removes whatever zot is shown.
    /// Returns 0 when there is no zot property.
    pub fn remove_zot(&mut self, lot: u64) -> bool {
        if self.host.lot_property(lot, lot_prop::ZOT).is_none() {
            return false;
        }
        self.remove_building_zot_prop(lot, zot::NONE);
        true
    }

    /// `RemoveLotProp(lot, zot)` (§5.6): removes every prop whose
    /// `0x48E95539` equals `zot`.
    pub fn remove_lot_prop(&mut self, lot: u64, which: u32) -> bool {
        let mut any = false;
        for p in self.host.lot_props(lot) {
            if self.host.occupant_property(p, PROP_ZOT_NUMBER) == Some(which) && self.host.prop_remove(p) {
                any = true;
            }
        }
        any
    }

    /// The zot shown on `lot`, 0 when the property is absent (reading never
    /// creates it).
    pub fn current_zot(&self, lot: u64) -> u32 {
        self.host.lot_property(lot, lot_prop::ZOT).unwrap_or(0)
    }

    fn write_zot(&self, lot: u64, which: u32) {
        self.host.lot_set_property(lot, lot_prop::ZOT, which);
    }

    // ---- business units (§5.7) ----

    /// `PickUnitToJoin(lot)` (§5.7). Draws R2.
    pub fn pick_unit_to_join(&mut self, lot: u64) {
        if self.host.lot_unit(lot).is_some() {
            return;
        }
        let btype = self.host.lot_building_type(lot, true);
        if btype == 0 {
            return;
        }
        let purpose = u32::from(self.host.building_profile(btype).purpose);
        if !(5..=8).contains(&purpose) {
            return;
        }
        let mut neighbours: Vec<u64> = Vec::new();
        for f in [1u32, 2, 4, 8] {
            self.host.lots_surrounding_lot(lot, &mut neighbours, f);
        }
        let mut candidates: Vec<u64> = Vec::new();
        for &nb in &neighbours {
            let nt = self.host.lot_building_type(nb, false);
            if nt == 0 {
                continue;
            }
            if !(self.host.lot_state(nb) > 1 || self.host.lot_unit(nb).is_some()) {
                continue;
            }
            if u32::from(self.host.building_profile(nt).purpose) != purpose {
                continue;
            }
            let ok = if purpose == 5 {
                self.host.lot_building(nb).is_some()
            } else {
                self.host.lot_has_road_access(nb)
                    || self
                        .host
                        .lot_travel_designate(nb)
                        .is_some_and(|d| self.host.lot_has_road_access(d))
            };
            if ok {
                candidates.push(nb);
            }
        }
        if candidates.is_empty() {
            return;
        }
        let i = self.draw_u32(DrawKind::R2, candidates.len() as u32) as usize;
        let Some(&c) = candidates.get(i.min(candidates.len() - 1)) else {
            return;
        };
        let unit = match self.host.lot_unit(c) {
            Some(u) => u,
            None => {
                let u = self.host.unit_create();
                self.host.unit_add(u, c);
                u
            }
        };
        self.host.unit_add(unit, lot);
        if purpose == 5 {
            let members = self.host.unit_members(unit);
            self.host.lot_set_travel_designate(lot, members.first().copied());
        } else {
            self.unit_changed = true;
        }
    }

    /// Slot 34 `UpdateTravelDesignates(u&)` (§5.7). The reference is
    /// replaced by the unit a dissolved member joins, or by null.
    pub fn update_travel_designates(&mut self, unit: &mut Option<u64>) {
        let Some(u) = *unit else {
            return;
        };
        let members = self.host.unit_members(u);
        let anchor = members
            .iter()
            .copied()
            .find(|&m| self.host.lot_has_road_access(m) && self.host.lot_building(m).is_some());
        if let Some(anchor) = anchor {
            // §5.7 table (answers §12.2 Q9): a designate that is a member
            // with road access is kept (no clear, no anchor step); a
            // non-member, or a member without road access, is cleared; a
            // null designate is not cleared. Then the anchor step
            // `if L ≠ anchor: L.SetTravelDesignate(anchor)`.
            for &m in &members {
                match self.host.lot_travel_designate(m) {
                    None => {}
                    Some(d) => {
                        if members.contains(&d) && self.host.lot_has_road_access(d) {
                            continue;
                        }
                        self.host.lot_set_travel_designate(m, None);
                    }
                }
                if m != anchor {
                    self.host.lot_set_travel_designate(m, Some(anchor));
                }
            }
            return;
        }
        // Without an anchor the unit is dissolved: `members` is the snapshot
        // copy `M` (§5.7 step 1).
        for &m in &members {
            self.host.unit_remove(u, m);
            self.pick_unit_to_join(m);
            if let Some(u2) = self.host.lot_unit(m) {
                // Step 3 (answers §12.2 Q10): every lot of `M`, in copy order
                // and including `m` itself, is removed from the unit it is in
                // (if any) and added to `u'`.
                for &x in &members {
                    if let Some(cur) = self.host.lot_unit(x) {
                        self.host.unit_remove(cur, x);
                    }
                    self.host.unit_add(u2, x);
                }
                *unit = Some(u2);
                return;
            }
        }
        *unit = None;
    }

    /// Slot 35 `UpdateTravelPopulations(u)` (§5.7).
    pub fn update_travel_populations(&mut self, unit: u64) {
        let members = self.host.unit_members(unit);
        for &m in &members {
            self.host.lot_reset_travel_population(m);
        }
        for &m in &members {
            let to = self.host.lot_travel_designate(m).unwrap_or(m);
            self.host.lot_add_travel_population(to, m);
        }
    }

    /// Slot 33 `RemoveLotFromItsBusinessUnit(lot)` (§5.7).
    pub fn remove_lot_from_its_business_unit(&mut self, lot: u64) -> bool {
        let Some(unit) = self.host.lot_unit(lot) else {
            return false;
        };
        let ok = self.host.unit_remove(unit, lot);
        self.unit_changed = true;
        ok
    }

    /// Slot 36 `NeedsAnchorBuilding(rect, purpose)` (§5.7).
    pub fn needs_anchor_building(&mut self, rect: CellRect, purpose: i32) -> bool {
        if !(6..=8).contains(&purpose) {
            return false;
        }
        for lot in self.host.lots_in_cell_rect(rect, false) {
            if let Some(b) = self.host.lot_building(lot) {
                if self.host.occupant_in_group(b, ANCHOR_GROUP) {
                    return true;
                }
            }
        }
        let mut neighbours: Vec<u64> = Vec::new();
        for f in [1u32, 2, 4, 8] {
            self.host.lots_surrounding_rect(rect, &mut neighbours, f);
        }
        for nb in neighbours {
            let nt = self.host.lot_building_type(nb, false);
            if nt == 0 {
                continue;
            }
            if !(self.host.lot_state(nb) > 1 || self.host.lot_unit(nb).is_some()) {
                continue;
            }
            if u32::from(self.host.building_profile(nt).purpose) == purpose as u32 {
                return false;
            }
        }
        true
    }

    // ---- construction (§6) ----

    /// Slot 6 `StartConstruction(lot, cfg, type, animate)` (§6.1).
    pub fn start_construction(&mut self, lot: u64, cfg: Option<u64>, btype: u32, animate: bool) -> bool {
        self.host.lot_set_configuration(lot, cfg);
        self.host.lot_set_building_type(lot, btype);
        self.host.lot_set_state(lot, lot_state::CONSTRUCTING, 0, 0);
        self.host.lot_set_condition(lot, 1);
        self.pick_unit_to_join(lot);
        let flags = self.host.lot_flags(lot);
        self.host.lot_set_flags(lot, flags & !7);
        self.host.lot_set_vertices_locked(lot, true);
        if self.host.lot_property(lot, lot_prop::ZOT).is_some() {
            self.host.post_message(msg::LOT_DEVELOPER_NOTIFICATION, 2, Some(lot));
            // Only a road zot is removed; other zots stay (quirk 6).
            if self.current_zot(lot) == zot::ROAD {
                self.remove_lot_prop(lot, zot::ROAD);
                self.write_zot(lot, zot::NONE);
            }
        }
        let ok = if btype == 0 {
            true
        } else {
            self.place_building(lot, cfg, btype, animate)
        };
        // END: state 3 even when no building was created (quirk 7).
        if !animate {
            self.host.lot_set_state(lot, lot_state::OCCUPIED, 0, 0);
        }
        ok
    }

    /// `StartConstruction` from the foundation step to the hand-off (§6.1).
    fn place_building(&mut self, lot: u64, cfg: Option<u64>, btype: u32, animate: bool) -> bool {
        let water = cfg.is_some_and(|c| self.host.cfg_is_compatible_with_water(c));
        let max_slope = cfg.map_or(0.0, |c| self.host.cfg_max_slope_before_foundation(c));
        let not_farm = u32::from(self.host.building_profile(btype).purpose) != 5;
        let rect = self.host.lot_bounding_rect(lot);
        let mut found = false;
        if water && max_slope == 0.0 {
            found = self.add_lot_foundation(lot, cfg, false, true, true);
        }
        if !found {
            let wide = rect.x2.wrapping_sub(rect.x1) > 1 && rect.z2.wrapping_sub(rect.z1) > 1;
            if not_farm && self.view.tuning.attempt_internal_lot_levelling && wide {
                self.view
                    .level_area_internal_vertices(&*self.host, rect, -1.0, water, false, Some(lot));
            }
            let mut s = self.get_lot_slope(lot, water);
            if s > self.view.tuning.max_slope_for_flatness && s > max_slope {
                if not_farm && self.view.tuning.attempt_lot_levelling && self.levelled(rect, water, lot) {
                    s = self.get_lot_slope(lot, water);
                }
                if s > max_slope {
                    found = self.add_lot_foundation(lot, cfg, false, true, true);
                }
            }
        }
        self.clear_lot_blocking_objects(lot, rect);
        let (pos, orient) = self.building_position(lot);
        let Some(b) = self.host.add_new_building_occupant(btype, pos, orient) else {
            return false;
        };
        if !found {
            if let Some(c) = cfg {
                self.level_or_found_building(b, c, water, lot);
            }
        }
        if animate {
            self.host.start_construction_occupant(lot);
        }
        true
    }

    /// Slot 7 `ConstructDefaultLot(lot, cfg, type, animate)` (§6.4). No
    /// random draw; the type is the family-expanded set's first element in
    /// STLport iteration order.
    pub fn construct_default_lot(&mut self, lot: u64, cfg: Option<u64>, btype: u32, animate: bool) -> bool {
        let mut t = btype;
        if t == 0 {
            if let Some(c) = cfg {
                if let Some(obj) = self.host.cfg_primary_building_object(c) {
                    let mut set = StlHashSet::with_hint(100);
                    for id in obj.ids {
                        if self.host.is_building_type(id) {
                            set.insert(id);
                        } else {
                            for m in self.host.building_types_by_family(id) {
                                set.insert(m);
                            }
                        }
                    }
                    if let Some(&first) = set.iter_order().first() {
                        t = first;
                    }
                }
            }
        }
        self.start_construction(lot, cfg, t, animate)
    }

    /// Slot 8 `AbandonBuilding(lot, reason)` (§6.4): exactly
    /// `SetState(4, 0, reason)`, so the state changes synchronously.
    pub fn abandon_building(&mut self, lot: u64, why: u32) -> bool {
        self.host.lot_set_state(lot, lot_state::ABANDONED, 0, why)
    }

    /// Slot 9 `ReoccupyBuilding(lot, devType)` (§6.4).
    pub fn reoccupy_building(&mut self, lot: u64, dev_type: u32) -> bool {
        let w = self.host.wealth_of_type(dev_type);
        let pw = u32::from(
            self.host
                .building_profile(self.host.lot_building_type(lot, false))
                .wealth,
        );
        let d = pw as i32 - w as i32;
        let condition = if d == 1 {
            2
        } else if d == 2 {
            3
        } else if self.host.lot_property(lot, lot_prop::NO_POWER).is_some_and(|v| v != 0) {
            1
        } else {
            0
        };
        self.host.lot_set_state(lot, lot_state::OCCUPIED, w, 0);
        self.host.lot_set_condition(lot, condition);
        let cap = self.host.lot_available_capacity(lot, dev_type, false);
        self.host.lot_set_population(lot, dev_type, cap)
    }

    /// Slot 10 `ChangeBuilding(lot, type)` (§6.4). A no-op returning 0 when
    /// the lot has no configuration; the lot's building type is not changed.
    pub fn change_building(&mut self, lot: u64, btype: u32) -> bool {
        let Some(cfg) = self.host.lot_configuration(lot) else {
            return false;
        };
        self.clear_lot_buildings(lot, lot_state::READY, true, 0);
        let (pos, orient) = self.building_position(lot);
        if let Some(b) = self.host.add_new_building_occupant(btype, pos, orient) {
            if self.host.lot_flags(lot) & 3 == 0 {
                let water = self.host.cfg_is_compatible_with_water(cfg);
                self.level_or_found_building(b, cfg, water, lot);
            }
        }
        // Also when no building was created (quirk 7).
        self.host.lot_set_state(lot, lot_state::OCCUPIED, 0, 0)
    }

    /// `GetPositionForBuilding` (slot 40) as `StartConstruction` and
    /// `ChangeBuilding` use it: `(0, 0, 0)` and orientation 0 when the slot
    /// writes nothing *[choice]*.
    fn building_position(&self, lot: u64) -> (Vec3, i32) {
        self.view
            .position_for_building(&*self.host, lot)
            .unwrap_or(([0.0; 3], 0))
    }

    /// `AttemptAreaLevelling(rect, …, true, perVertex,
    /// maxAreaSlopeForLevelling, lot)` (slot 21) tested as a bool: the low
    /// byte of the count *[inferred: callers test AL, §17.4]*.
    fn levelled(&self, rect: CellRect, per_vertex: bool, lot: u64) -> bool {
        let area = self.view.tuning.max_area_slope_for_levelling;
        let n = self
            .view
            .attempt_area_levelling(&*self.host, rect, true, per_vertex, area, Some(lot));
        n & 0xFF != 0
    }

    /// The building-slope step of `StartConstruction` / `ChangeBuilding`
    /// (§6.1, §6.4): a building steeper than `kMaxSlopeForFlatness` gets its
    /// cells levelled (slot 21 on slot 22's rectangle) and, if still too
    /// steep, a building foundation (slot 58).
    fn level_or_found_building(&mut self, b: u64, cfg: u64, water: bool, lot: u64) {
        let flat = self.view.tuning.max_slope_for_flatness;
        let mut so = self.get_occupant_slope(b, water);
        if so > flat {
            let coverage = self.occupant_cell_coverage(b);
            if self.levelled(coverage, water, lot) {
                so = self.get_occupant_slope(b, water);
            }
            if so > flat {
                self.view.add_building_foundation(&*self.host, b, cfg);
            }
        }
    }

    /// Slot 37 `UpdateLotElements(lot, kind)` (§6.4). `kind` is the BDS's
    /// **whole 32-bit** terrain-change kind (`building-development.md`
    /// §4.11); its only use is the full 32-bit compare `kind != 5`. Always
    /// returns true.
    pub fn update_lot_elements(&mut self, lot: u64, kind: u32) -> bool {
        if self.host.lot_state(lot) < lot_state::CONSTRUCTING {
            return true;
        }
        let mut added = false;
        if self.host.lot_flags(lot) & 3 != 0 {
            self.update_lot_foundation(lot);
            self.host.update_lot_retaining_walls(lot);
        } else if kind != KIND_NO_FOUNDATION_RECHECK {
            if let Some(cfg) = self.host.lot_configuration(lot) {
                let water = self.host.cfg_is_compatible_with_water(cfg);
                let s = self.get_lot_slope(lot, water);
                if self.host.cfg_max_slope_before_foundation(cfg) < s {
                    let has_water = self.host.cfg_has_objects_of_type(cfg, object_type::WATER);
                    self.add_lot_foundation(lot, Some(cfg), has_water, false, true);
                    added = true;
                }
                if s.abs() > OBJECT_ALTITUDE_SLOPE {
                    self.host.update_lot_object_altitudes(lot);
                }
            }
        }
        self.host.update_lot_base_texture_occupant(lot, added);
        self.host.lot_update_slope(lot);
        true
    }

    /// Slot 54 `AddLotFoundation(lot, cfg, useLotAltitude, level, walls)`
    /// (§17.5.4; R8).
    pub fn add_lot_foundation(
        &mut self,
        lot: u64,
        cfg: Option<u64>,
        use_lot_altitude: bool,
        level: bool,
        walls: bool,
    ) -> bool {
        self.view
            .add_lot_foundation(&*self.host, &self.statics, lot, cfg, use_lot_altitude, level, walls)
    }

    /// Slot 55 `AddLotFoundation(lot, type, useLotAltitude, level, walls)`
    /// (§17.5.4).
    pub fn add_lot_foundation_type(
        &mut self,
        lot: u64,
        ftype: u32,
        use_lot_altitude: bool,
        level: bool,
        walls: bool,
    ) -> bool {
        self.view
            .add_lot_foundation_type(&*self.host, &self.statics, lot, ftype, use_lot_altitude, level, walls)
    }

    /// Slot 61 `UpdateLotFoundation(lot)` (§17.5.4; R8 possible).
    pub fn update_lot_foundation(&mut self, lot: u64) -> bool {
        self.view.update_lot_foundation(&*self.host, &self.statics, lot)
    }

    /// `UpdateSurroundingLotFoundations(lot)` (§17.5.4).
    pub fn update_surrounding_lot_foundations(&mut self, lot: u64) {
        self.view
            .update_surrounding_lot_foundations(&*self.host, &self.statics, lot);
    }

    /// `ClearLotBlockingObjects` (§6.2): bulldozes the flora on the lot.
    pub fn clear_lot_blocking_objects(&mut self, lot: u64, rect: CellRect) {
        let _ = lot;
        let top = self.lowest_or_highest_area_altitude(rect, false, true);
        let top = (r24(f64::from(top) + BLOCKING_TOP_MARGIN)) as f32;
        let shrink_in = |v: i32| (r24(r24(f64::from(v) * CELL_METRES) + 0.5)) as f32;
        let shrink_out = |v: i32| (r24(r24(f64::from(v) * CELL_METRES) - 0.5)) as f32;
        let min = [shrink_in(rect.x1), 0.0, shrink_in(rect.z1)];
        let max = [
            shrink_out(rect.x2.wrapping_add(1)),
            top,
            shrink_out(rect.z2.wrapping_add(1)),
        ];
        for occ in self.host.occupants_in_bbox(min, max) {
            self.host.demolish_occupant(occ);
        }
    }

    // ---- lot-state and condition messages (§6.5) ----

    fn on_lot_state_changed(&mut self, m: &LotDevMessage) {
        let (old, new) = (m.data1, m.data2);
        let a = new == lot_state::OCCUPIED || new == lot_state::ABANDONED;
        let b = old == lot_state::CONSTRUCTING && new == lot_state::OCCUPIED;
        if !a && !b {
            return;
        }
        let Some(l) = m.lot else {
            return;
        };
        let (x, z) = self.host.lot_location(l);
        let Some(l2) = self.host.get_lot(x, z, true) else {
            return;
        };
        let Some(cfg) = self.host.lot_configuration(l2) else {
            return;
        };
        // The work is applied to the lot found at that location, and only
        // when the sizes match (quirk 8).
        if self.host.lot_size(l2) != self.host.lot_size(l) {
            return;
        }
        if a {
            self.set_normal_lot_base_texture(l2, Some(cfg), true);
        }
        if b {
            self.add_lot_props(l2, cfg);
            self.add_lot_flora(l2, cfg);
            self.host.add_network_connection_nodes(l2, cfg);
        }
    }

    fn on_lot_condition_changed(&mut self, m: &LotDevMessage) {
        let Some(l) = m.lot else {
            return;
        };
        let (x, z) = self.host.lot_location(l);
        let Some(l2) = self.host.get_lot(x, z, true) else {
            return;
        };
        let s = self.host.lot_state(l2);
        if s == lot_state::OCCUPIED || s == lot_state::ABANDONED {
            let cfg = self.host.lot_configuration(l2);
            self.set_normal_lot_base_texture(l2, cfg, true);
        }
    }

    /// `AddLotProps(lot, cfg, 1)` (§6.5). Draws R5 per prop object.
    pub fn add_lot_props(&mut self, lot: u64, cfg: u64) {
        let objects = self.host.cfg_objects(cfg);
        for (index, o) in objects.iter().enumerate() {
            if o.object_type == object_type::PROP && o.flags & 1 == 0 {
                self.create_random_lot_prop(lot, cfg, index, o);
            }
        }
    }

    /// `CreateRandomLotProp(lot, index)` (§6.5).
    fn create_random_lot_prop(&mut self, lot: u64, cfg: u64, index: usize, o: &CfgObject) {
        let (pos, orient) = self.view.object_position(&*self.host, lot, Some(cfg), o);
        let ids = self.host.object_id_array(cfg, index);
        if ids.is_empty() {
            return;
        }
        let i = self.draw_u32(DrawKind::R5, ids.len() as u32) as usize;
        let id = ids[i.min(ids.len() - 1)];
        if id == 0 {
            return;
        }
        if let Some(occ) = self.host.prop_add(id, pos, orient) {
            self.host.occupant_set_lot_tag(occ, o.tag);
        }
    }

    /// `AddLotFlora(lot, cfg, 1)` (§6.5). Draws R6 / R7 per flora object.
    pub fn add_lot_flora(&mut self, lot: u64, cfg: u64) {
        let objects = self.host.cfg_objects(cfg);
        for o in objects.iter() {
            if o.object_type != object_type::FLORA {
                continue;
            }
            let (pos, _orient) = self.view.object_position(&*self.host, lot, Some(cfg), o);
            let t = self.pick_flora_type_for_lot(&o.ids);
            if t == 0 {
                continue;
            }
            if let Some(occ) = self.host.flora_add(t, pos) {
                self.host.occupant_set_lot_tag(occ, o.tag);
            }
        }
    }

    /// `PickFloraTypeForLot(ids, pos)` (§6.5).
    pub fn pick_flora_type_for_lot(&mut self, ids: &[u32]) -> u32 {
        if ids.is_empty() {
            return 0;
        }
        let i = self.draw_u32(DrawKind::R6, ids.len() as u32) as usize;
        let id = ids[i.min(ids.len() - 1)];
        if self.host.flora_type_valid(id) {
            return id;
        }
        let Some(members) = self.host.flora_types_by_family(id) else {
            return 0;
        };
        let mut set = StlHashSet::with_hint(100);
        for m in members {
            set.insert(m);
        }
        if set.is_empty() {
            return 0;
        }
        let order = set.iter_order();
        let k = self.draw_u32(DrawKind::R7, order.len() as u32) as usize;
        order[k.min(order.len() - 1)]
    }

    // ---- base textures (§6.6) ----

    /// Slot 44 `SetNormalLotBaseTexture(lot, cfg, flag)` (§6.6). Draws R4
    /// per texture object. The third argument is unused by the behaviour the
    /// spec states; the plugin returns the lot where the original's return
    /// is undefined.
    pub fn set_normal_lot_base_texture(&mut self, lot: u64, cfg: Option<u64>, _flag: bool) {
        let Some(cfg) = cfg else {
            return;
        };
        let zone = self.host.lot_zone_type(lot);
        if self.force_empty && !matches!(zone, 0 | 11 | 12 | 15) {
            self.view.set_empty_lot_base_texture(&*self.host, lot);
            return;
        }
        let objects = self.host.cfg_objects(cfg);
        let mut records: Vec<TextureRecord> = Vec::new();
        for o in objects.iter() {
            if o.object_type != object_type::TEXTURE || o.ids.is_empty() {
                continue;
            }
            let (pos, orient) = self.view.object_position(&*self.host, lot, Some(cfg), o);
            let tex = if o.ids[0] != 0 {
                self.pick_lot_base_texture_for_lot(&o.ids, lot)
            } else {
                0
            };
            if tex == 0 {
                continue;
            }
            let x = ftol32(r24(f64::from(pos[0]) * 0.0625));
            let z = ftol32(r24(f64::from(pos[2]) * 0.0625));
            // [choice] The city size does not change at run time, so the
            // "inside the city" test reads the live counts rather than the
            // ones `Init` stored (they are the same values).
            let (nx, nz) = self.host.city_cell_counts();
            if x < 0 || z < 0 || x >= nx || z >= nz {
                continue;
            }
            records.push(TextureRecord {
                texture: tex,
                x,
                z,
                orientation: orient as u8,
                byte3: o.byte3,
                extra: BASE_TEXTURE_EXTRA_DEFAULT,
                frame: 0xFF,
            });
        }
        if records.is_empty() {
            return;
        }
        let Some(occ) = self.host.base_texture_occupant(lot, true) else {
            return;
        };
        self.host.base_texture_set(occ, &records);
        if self.host.occupant_is_flag_set(occ, 1) {
            self.host.occupant_changed(occ);
            self.host.occupant_post_message(occ, msg::OCCUPANT_CHANGED);
        } else {
            self.host.occupant_insert(occ);
        }
    }

    /// `PickLotBaseTextureForLot(ids, lot)` (§6.6). Draws R4.
    pub fn pick_lot_base_texture_for_lot(&mut self, ids: &[u32], lot: u64) -> u32 {
        if ids.is_empty() {
            return 0;
        }
        let i = self.draw_u32(DrawKind::R4, ids.len() as u32) as usize;
        let base = ids[i.min(ids.len() - 1)];
        let mut wealth = self.host.lot_occupant_wealth(lot);
        if wealth == 0 {
            let btype = self.host.lot_building_type(lot, false);
            let pw = if btype != 0 {
                u32::from(self.host.building_profile(btype).wealth)
            } else {
                0
            };
            wealth = if pw != 0 { pw } else { 1 };
        }
        let decay = match self.host.lot_condition(lot) {
            0..=1 => 0u32,
            2..=3 => 1,
            _ => 2,
        };
        let hi = base & 0xFFFF_0000;
        for cand in [
            hi | (((wealth << 4) | decay) << 8),
            hi | (wealth << 12),
            hi | (decay << 8),
        ] {
            if self.host.resource_exists(TEXTURE_RESOURCE.0, TEXTURE_RESOURCE.1, cand) {
                return cand;
            }
        }
        base
    }

    /// Slot 47 `SetForceEmptyLotBaseTextures(f)` (§6.6). Walks every lot
    /// only when the value differs.
    pub fn set_force_empty_lot_base_texture(&mut self, f: bool) {
        if f == self.force_empty {
            return;
        }
        self.force_empty = f;
        let it = self.host.safe_lot_iterator();
        while let Some(lot) = self.host.safe_lot_iterator_next(it) {
            if f {
                self.view.set_empty_lot_base_texture(&*self.host, lot);
            } else {
                let cfg = self.host.lot_configuration(lot);
                self.set_normal_lot_base_texture(lot, cfg, true);
            }
        }
        self.host.safe_lot_iterator_free(it);
    }

    // ---- demolition and explosion (§7) ----

    /// `ClearLotBuildings(lot, state, removeFoundation, flags)` (§7.1).
    /// Returns slot 11's AL. [choice] It stops when the host hands back the
    /// same building twice, or after [`MAX_CLEAR_BUILDINGS`].
    pub fn clear_lot_buildings(&mut self, lot: u64, state: u32, remove_foundation: bool, flags: u32) -> bool {
        self.host.lot_set_state(lot, state, 0, 0);
        let mut previous: Option<u64> = None;
        let mut n = 0u32;
        while let Some(b) = self.host.first_building(lot) {
            if previous == Some(b) || n >= MAX_CLEAR_BUILDINGS {
                break;
            }
            previous = Some(b);
            n += 1;
            self.host.remove_building_props(b);
            if remove_foundation {
                self.view.remove_building_foundation(&*self.host, b);
            }
            self.host.occupant_remove(b, 1, flags);
        }
        self.remove_zot(lot)
    }

    /// Slot 12 `StartDemolishLot(lot)` (§7.2).
    pub fn start_demolish_lot(&mut self, lot: u64) -> bool {
        if self.host.lot_flags(lot) & 1 != 0 {
            let rect = self.host.lot_bounding_rect(lot);
            self.host.flora_demolish_in_cell_area(rect);
        }
        self.host.remove_lot_props(lot);
        self.write_zot(lot, zot::NONE);
        self.host.remove_network_connection_nodes(lot);
        self.clear_lot_buildings(lot, lot_state::DESTRUCTING, true, 0)
    }

    /// Slot 13 `EndDemolishLot(lot)` (§7.2). A kept lot ends in state 1
    /// (quirk 13); returns true where the original is undefined.
    pub fn end_demolish_lot(&mut self, lot: u64) -> bool {
        if self.host.lot_is_initialized(lot) {
            self.host.lot_set_state(lot, lot_state::DESTROYED, 0, 0);
            self.clear_lot(lot);
            if matches!(self.host.lot_zone_type(lot), 0 | 11 | 12 | 15) {
                self.host.delete_lot(lot);
            }
        }
        true
    }

    /// `ClearLot(lot)` (§7.2).
    pub fn clear_lot(&mut self, lot: u64) {
        self.host.add_retaining_wall_occupant(lot, 0);
        let (x, z) = self.host.lot_location(lot);
        self.view.remove_lot_foundation_at(&*self.host, x, z);
        self.update_surrounding_lot_foundations(lot);
        self.host.update_surrounding_retaining_walls(lot);
        self.host.lot_set_vertices_locked(lot, false);
        self.host.lot_set_state(lot, lot_state::READY, 0, 0);
        self.host.lot_set_building_type(lot, 0);
        self.host.lot_set_configuration(lot, None);
        self.view.set_empty_lot_base_texture(&*self.host, lot);
    }

    /// Slot 14 `ExplodeLot(lot, burnt)` (§7.2). Returns true where the
    /// original is undefined.
    pub fn explode_lot(&mut self, lot: u64, burnt: bool) -> bool {
        if matches!(self.host.lot_state(lot), 5 | 6) {
            return true;
        }
        let burnt_building: Option<BurntBuilding> = self
            .host
            .first_building(lot)
            .and_then(|b| self.host.make_burnt_building(b));
        self.clear_lot_buildings(lot, lot_state::DESTROYED, false, 4);
        self.host.remove_lot_props(lot);
        self.host.remove_network_connection_nodes(lot);
        if let Some(bb) = &burnt_building {
            let ex = (r24(f64::from(bb.max[0]) - f64::from(bb.min[0]))) as f32;
            let ez = (r24(f64::from(bb.max[2]) - f64::from(bb.min[2]))) as f32;
            let s = if ex > ez { ex } else { ez };
            let instance = if burnt {
                super::RUBBLE_DECAL.2
            } else {
                super::RUBBLE_DECAL.3
            };
            self.host
                .add_rubble_decal(lot, bb, instance, (r24(f64::from(s) * 2.0)) as f32);
            self.add_lot_rubble_props(lot, bb.min, bb.max, burnt);
            self.host.prop_add_to_lot(lot, bb.occupant);
        }
        if burnt {
            self.host.lot_set_condition(lot, 6);
        }
        self.host.lot_set_building_type(lot, 0);
        true
    }

    /// `AddLotRubbleProps(lot, min, max, burnt)` (§7.3). Draws R9–R14.
    pub fn add_lot_rubble_props(&mut self, lot: u64, min: Vec3, max: Vec3, burnt: bool) {
        let shape = rubble_count(min, max);
        // Corners: x from max for indices 0 and 3, z from max for 0 and 1.
        let xf = [true, false, false, true];
        let zf = [true, true, false, false];
        for i in 0..shape.count {
            let orient;
            let ptype;
            let mut pos;
            if i < 4 {
                if shape.small {
                    continue;
                }
                if self.draw_u32(DrawKind::R9, 2) == 0 {
                    continue;
                }
                let d2 = if shape.large {
                    self.draw_u32(DrawKind::R10, if burnt { 2 } else { 6 }) as usize
                } else {
                    0
                };
                orient = if d2 == 0 { ((i + 2) % 4) as u32 } else { i as u32 };
                ptype = if burnt {
                    RUBBLE_BURNT_CORNER[d2.min(1)]
                } else {
                    RUBBLE_TYPES[d2.min(5)]
                };
                let idx = i as usize;
                pos = [
                    if xf[idx] { max[0] } else { min[0] },
                    min[1],
                    if zf[idx] { max[2] } else { min[2] },
                ];
            } else {
                orient = self.draw_u32(DrawKind::R11, 4);
                let d2 = self.draw_u32(DrawKind::R12, if burnt { 2 } else { 6 }) as usize;
                ptype = if burnt {
                    RUBBLE_BURNT_INTERIOR[d2.min(1)]
                } else {
                    RUBBLE_TYPES[d2.min(5)]
                };
                let gx = self.draw_gaussian(DrawKind::R13, f64::from(min[0]), f64::from(max[0]));
                let gz = self.draw_gaussian(DrawKind::R14, f64::from(min[2]), f64::from(max[2]));
                pos = [gx as f32, min[1], gz as f32];
            }
            if ptype == 0 {
                continue;
            }
            let Some(occ) = self.host.prop_create(ptype) else {
                continue;
            };
            self.host.occupant_init(occ);
            self.host.occupant_set_flag(occ, 0x40);
            self.host.occupant_set_orientation(occ, orient);
            self.host.occupant_set_position(occ, pos);
            let (bmin, bmax) = self.host.occupant_bounding_box(occ);
            pos[0] = clamp_rubble(pos[0], bmin[0], bmax[0], min[0], max[0]);
            pos[2] = clamp_rubble(pos[2], bmin[2], bmax[2], min[2], max[2]);
            self.host.occupant_set_position(occ, pos);
            self.host.prop_add_to_lot(lot, occ);
        }
    }

    // ---- night lights (§8) ----

    fn on_building_inserted(&mut self, occ: u64) {
        let Some(b) = self.host.occupant_as_building(occ) else {
            return;
        };
        self.host.building_add_ref(b);
        self.night.push(Some(b));
        // New buildings join unlit: `litFlag` is never set at run time
        // (quirk 9).
        self.host.building_set_lit(b, self.lit_flag);
        if self.night_cursor > self.night.len() {
            self.night_cursor = self.night.len();
        }
    }

    fn on_building_removed(&mut self, occ: u64) {
        let Some(b) = self.host.occupant_as_building(occ) else {
            return;
        };
        let Some(i) = self.night.iter().position(|e| *e == Some(b)) else {
            return;
        };
        self.night.remove(i);
        if self.night_cursor > i {
            self.night_cursor -= 1;
        }
        if self.night_cursor > self.night.len() {
            self.night_cursor = self.night.len();
        }
        self.host.building_release(b);
    }

    fn on_day_night_changed(&mut self, m: &LotDevMessage) {
        let night = m.data2 == 0;
        let mode = self.host.visual_time_mode();
        if (night && mode == 2) || (!night && mode == 1) {
            self.update_all(mode);
            self.host.post_message(msg::NIGHT_LIGHTS_UPDATED, 0, None);
            return;
        }
        self.night_mode = if night { 2 } else { 1 };
        self.shuffle_night();
        self.night_cursor = 0;
    }

    /// `random_shuffle` of the whole deque (§8.2, R15): `n − 1` `rand()`
    /// calls, `j := rand() mod (i + 1)`, swap elements i and j.
    fn shuffle_night(&mut self) {
        let n = self.night.len();
        for i in 1..n {
            let r = {
                let mut s = self.statics.borrow_mut();
                let r = s.crt.rand();
                s.record(DrawKind::R15, n as u32, f64::from(r));
                r
            };
            let j = (r as usize) % (i + 1);
            self.night.swap(i, j);
        }
    }

    /// `UpdateAll(mode)` (§8.2): the immediate whole-city update.
    pub fn update_all(&mut self, mode: u32) {
        let is_night = mode == 2;
        let n = self.night.len();
        for i in 0..n {
            if let Some(b) = self.night[i] {
                self.light(b, is_night);
            }
        }
        self.night_cursor = 0;
        self.night_mode = 0;
    }

    /// `DoAnimationTick(ms)` (§8.3).
    pub fn on_animation_tick(&mut self, ms: u32) {
        self.night_acc = self.night_acc.wrapping_add(ms);
        if self.night_acc <= NIGHT_THRESHOLD_MS {
            return;
        }
        let is_night = self.night_mode == 2;
        self.night_acc = 0;
        for _ in 0..self.night_batch {
            if self.night_cursor >= self.night.len() {
                self.night_mode = 0;
                break;
            }
            let b = self.night[self.night_cursor];
            self.night_cursor += 1;
            if let Some(b) = b {
                self.light(b, is_night);
            }
        }
    }

    /// `Light(b, isNight)` (§8.4).
    pub fn light(&mut self, b: u64, is_night: bool) {
        self.host.building_set_lit(b, is_night);
        let r = self.host.occupant_city_cells(b);
        let cx = (r.x1.wrapping_add(r.x2)) >> 1;
        let cz = (r.z1.wrapping_add(r.z2)) >> 1;
        let Some(lot) = self.host.get_lot(cx, cz, false) else {
            return;
        };
        // §8.4 step 4 (answers §12.2 Q8): the building's cells only find the
        // lot; the prop query takes the lot's own rectangle (lot slot 14).
        let r = self.host.lot_bounding_rect(lot);
        for p in self.host.occupants_in_cells(r, OCCUPANT_TYPE_PROP) {
            if self.host.occupant_property(p, PROP_NIGHT_LIGHT).is_some_and(|v| v != 0) {
                self.host.occupant_post_message(p, msg::OCCUPANT_LIGHT);
            }
        }
    }

    // ---- power changed (§9) ----

    /// `DoPowerChanged` (§9). No counters change and no state changes here.
    pub fn do_power_changed(&mut self, rect: CellRect) {
        for lot in self.host.lots_in_cell_rect(rect, true) {
            let b = self.host.lot_building(lot);
            let (x, z) = self.host.lot_location(lot);
            let (cx, cz) = match b {
                Some(b) => self.building_centre_cell(b),
                None => (x, z),
            };
            // No "needs power" test in the powered computation here.
            let powered = self.host.cheat(Cheat::IgnorePower) || self.host.is_powered(cx, cz);
            if powered != self.host.lot_has_power(lot) {
                self.host.lot_set_has_power(lot, powered);
                let c = self.host.lot_condition(lot);
                if !powered && c == 0 {
                    self.host.lot_set_condition(lot, 1);
                }
                if powered && c == 1 {
                    self.host.lot_set_condition(lot, 0);
                }
            }
            let needs = b.is_none() || self.host.lot_needs_power(lot);
            if !powered && needs {
                let suitable = b.is_some() || is_empty_lot_suitable_for_zot(self.host.lot_zone_type(lot), x, z);
                if suitable {
                    self.add_building_zot_prop(lot, zot::POWER_PROP, zot::POWER);
                }
            } else if self.current_zot(lot) == zot::POWER {
                // Also for empty lots (quirk 5).
                self.remove_lot_prop(lot, zot::POWER);
                self.write_zot(lot, zot::NONE);
            }
        }
    }

    // ---- terrain and cost slots (§11.1, §11.4) ----

    /// The values the terrain and geometry slots read, for hosts that
    /// answer them while the lot developer is busy ([`TerrainView`]).
    pub fn terrain_view(&self) -> TerrainView {
        self.view.clone()
    }

    /// Slot 15 `GetTerrainAltitudeByVertex(x, z, withWater)`.
    pub fn get_terrain_altitude_by_vertex(&self, x: i32, z: i32, with_water: bool) -> f32 {
        self.view.altitude_by_vertex(&*self.host, x, z, with_water)
    }

    /// Slots 16 / 17 `GetLowestOrHighestAreaAltitude(rect, lowest,
    /// withWater)` over the vertices `x1 ..= x2 + 1`, `z1 ..= z2 + 1`.
    pub fn lowest_or_highest_area_altitude(&self, rect: CellRect, lowest: bool, with_water: bool) -> f32 {
        self.view
            .lowest_or_highest_area_altitude(&*self.host, rect, lowest, with_water)
    }

    /// Slot 18 `GetAverageAreaAltitude(rect, withWater)`.
    pub fn average_area_altitude(&self, rect: CellRect, with_water: bool) -> f32 {
        self.view.average_area_altitude(&*self.host, rect, with_water)
    }

    /// Slot 27 `GetAreaAltitudeRange(rect, withWater)`: max − min over the
    /// vertices.
    pub fn area_altitude_range(&self, rect: CellRect, with_water: bool) -> f32 {
        self.view.area_altitude_range(&*self.host, rect, with_water)
    }

    /// Slot 25 `IsAreaFlat(rect, withWater)`: the altitude range within
    /// `0xE900EA24` *[inferred: `<=`]*.
    pub fn is_area_flat(&self, rect: CellRect, with_water: bool) -> bool {
        self.view.is_area_flat(&*self.host, rect, with_water)
    }

    /// Slot 26 `IsLotFlat(lot, withWater)`.
    pub fn is_lot_flat(&self, lot: u64, with_water: bool) -> bool {
        self.view.is_lot_flat(&*self.host, lot, with_water)
    }

    /// Slot 29 `GetAreaSlope(rect, withWater)` (§11.4).
    pub fn get_area_slope(&self, rect: CellRect, with_water: bool) -> f32 {
        self.view.area_slope(&*self.host, rect, with_water)
    }

    /// Slot 30 `GetLotSlope(lot, withWater)`.
    pub fn get_lot_slope(&self, lot: u64, with_water: bool) -> f32 {
        self.view.lot_slope(&*self.host, lot, with_water)
    }

    /// Slot 31 `GetOccupantSlope(occupant, withWater)`: slot 29 on slot 22's
    /// rectangle.
    pub fn get_occupant_slope(&self, occ: u64, with_water: bool) -> f32 {
        self.view.occupant_slope(&*self.host, occ, with_water)
    }

    /// Slot 22 `GetOccupantCellCoverage(occupant, rect&)`: the bounding box
    /// shrunk by 0.05 on each side, converted with `PositionToCell`.
    pub fn occupant_cell_coverage(&self, occ: u64) -> CellRect {
        self.view.occupant_cell_coverage(&*self.host, occ)
    }

    /// Slot 32 `GetLotDevelopmentCost(rect)` (§11.4).
    pub fn get_lot_development_cost(&self, rect: CellRect) -> i64 {
        let slope = self.get_area_slope(rect, true);
        development_cost(rect, slope, &self.view.tuning.cost_curve)
    }

    /// Slot 23 `IsAreaDeveloped(rect)` → always false.
    pub fn is_area_developed(&self, _rect: CellRect) -> bool {
        false
    }

    /// Slot 24 `IsLotDeveloped(lot)` → false.
    pub fn is_lot_developed(&self, _lot: u64) -> bool {
        false
    }

    // ---- geometry (§17) ----

    /// Slot 16 `GetLowestOrHighestAreaAltitude(SC4Rect<float>, lowest,
    /// withWater)` (§17.2).
    pub fn lowest_or_highest_area_altitude_f(&self, rect: [f32; 4], lowest: bool, with_water: bool) -> f32 {
        self.view
            .lowest_or_highest_area_altitude_f(&*self.host, rect, lowest, with_water)
    }

    /// Slot 19 `LevelAreaInternalVertices` (§17.4): the call count.
    pub fn level_area_internal_vertices(
        &self,
        rect: CellRect,
        target: f32,
        skip_water: bool,
        force: bool,
        lot: Option<u64>,
    ) -> i32 {
        self.view
            .level_area_internal_vertices(&*self.host, rect, target, skip_water, force, lot)
    }

    /// Slot 20 `LevelBorderingVertices` (§17.4).
    pub fn level_bordering_vertices(&self, rect: CellRect, h: f32, lot: Option<u64>) -> i32 {
        self.view.level_bordering_vertices(&*self.host, rect, h, lot)
    }

    /// Slot 21 `AttemptAreaLevelling` (§17.4).
    pub fn attempt_area_levelling(
        &self,
        rect: CellRect,
        with_water: bool,
        per_vertex: bool,
        max_area_slope: f32,
        lot: Option<u64>,
    ) -> i32 {
        self.view
            .attempt_area_levelling(&*self.host, rect, with_water, per_vertex, max_area_slope, lot)
    }

    /// Slot 39 `GetPositionForLotConfigurationObject` (§17.6).
    pub fn object_position(&self, lot: u64, cfg: Option<u64>, obj: &CfgObject) -> (Vec3, i32) {
        self.view.object_position(&*self.host, lot, cfg, obj)
    }

    /// Slot 40 `GetPositionForBuilding` (§17.6).
    pub fn position_for_building(&self, lot: u64) -> Option<(Vec3, i32)> {
        self.view.position_for_building(&*self.host, lot)
    }

    /// Slot 42 `GetBoundaryLotTexture` (§17.7).
    pub fn boundary_lot_texture(&self, sides: [i32; 4]) -> Option<BoundaryRule> {
        self.view.boundary_lot_texture(sides)
    }

    /// Slot 43 `SetEmptyLotBaseTexture` (§17.7).
    pub fn set_empty_lot_base_texture(&self, lot: u64) {
        self.view.set_empty_lot_base_texture(&*self.host, lot);
    }

    /// Slot 48 `GetFoundationByOccupant` (§17.5.1).
    pub fn foundation_by_occupant(&self, occ: u64) -> Option<u64> {
        self.view.foundation_by_occupant(&*self.host, occ)
    }

    /// Slot 49 `GetFoundationByStandardCityCell` (§17.5.1).
    pub fn foundation_by_cell(&self, kind: i32, x: i32, z: i32) -> Option<u64> {
        self.view.foundation_by_cell(&*self.host, kind, x, z)
    }

    /// Slot 50 `GetFoundationByPosition` (§17.5.1).
    pub fn foundation_by_position(&self, kind: i32, x: f32, z: f32) -> Option<u64> {
        self.view.foundation_by_position(&*self.host, kind, x, z)
    }

    /// Slot 51 `GetLotFoundation` (§17.5.1).
    pub fn lot_foundation(&self, lot: u64) -> Option<u64> {
        self.view.lot_foundation(&*self.host, lot)
    }

    /// Slot 52 `RemoveFoundation` (§17.5.2).
    pub fn remove_foundation(&self, kind: i32, x: f32, z: f32) -> bool {
        self.view.remove_foundation(&*self.host, kind, x, z)
    }

    /// Slot 53 `RemoveLotFoundation(x, z)` (§17.5.2).
    pub fn remove_lot_foundation_at(&self, x: i32, z: i32) -> bool {
        self.view.remove_lot_foundation_at(&*self.host, x, z)
    }

    /// Slot 56 `IsLotObjectFoundationRequired(occ)` (§17.5.5).
    pub fn is_occupant_foundation_required(&self, occ: u64) -> bool {
        self.view.is_occupant_foundation_required(&*self.host, occ)
    }

    /// Slot 57 `IsLotObjectFoundationRequired(min, max)` (§17.5.5).
    pub fn is_box_foundation_required(&self, min: Vec3, max: Vec3) -> bool {
        self.view.is_box_foundation_required(&*self.host, min, max)
    }

    /// Slot 58 `AddBuildingFoundation(building, cfg)` (§17.5.5).
    pub fn add_building_foundation(&self, building: u64, cfg: u64) -> bool {
        self.view.add_building_foundation(&*self.host, building, cfg)
    }

    /// Slot 59 `AddBuildingFoundation(building, type)` (§17.5.5).
    pub fn add_building_foundation_type(&self, building: u64, ftype: u32) -> bool {
        self.view.add_building_foundation_type(&*self.host, building, ftype)
    }

    /// Slot 60 `RemoveBuildingFoundation` (§17.5.2).
    pub fn remove_building_foundation(&self, occ: u64) -> bool {
        self.view.remove_building_foundation(&*self.host, occ)
    }

    // ---- persistence (§13) ----

    /// `Write` (§13). Returns whether the stream has no error.
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        let mut ok = out.put_u16(RECORD_VERSION);
        ok &= out.put_i32(self.view.verts_x);
        ok &= out.put_f32(self.view.extent_x);
        ok &= out.put_f32(self.view.extent_z);
        ok &= out.put_u32(self.night.len() as u32);
        for e in &self.night {
            let r = match e {
                Some(b) => self.host.building_ref(*b),
                None => RefValue::Null,
            };
            ok &= out.put_ref(&r);
        }
        ok &= out.put_u32(self.night_cursor as u32);
        ok &= out.put_u8(self.night_mode as u8);
        ok &= out.put_bool(self.lit_flag);
        ok &= out.error() == 0;
        ok
    }

    /// `Read` (§13). Any version other than 1 → false with nothing more
    /// read. `nightAcc` becomes 0.
    pub fn read(&mut self, inp: &mut dyn InStream) -> bool {
        let Some(version) = inp.get_u16() else {
            return false;
        };
        if version != RECORD_VERSION {
            return false;
        }
        let mut ok = true;
        match inp.get_i32() {
            Some(v) => self.view.verts_x = v,
            None => ok = false,
        }
        match inp.get_f32() {
            Some(v) => self.view.extent_x = v,
            None => ok = false,
        }
        match inp.get_f32() {
            Some(v) => self.view.extent_z = v,
            None => ok = false,
        }
        let count = inp.get_u32().unwrap_or_else(|| {
            ok = false;
            0
        }) as usize;
        let count = count.min(MAX_SAVED_BUILDINGS);
        let mut night = vec![None; count];
        for slot in night.iter_mut() {
            let Some(r) = inp.get_ref_as(super::IID_BUILDING_OCCUPANT) else {
                ok = false;
                break;
            };
            if r.is_null() {
                continue;
            }
            match self.host.resolve_building(&r) {
                Some(b) => *slot = Some(b),
                None => {
                    ok = false;
                    break;
                }
            }
        }
        self.night = night;
        match inp.get_u32() {
            Some(i) => self.night_cursor = (i as usize).min(self.night.len()),
            None => ok = false,
        }
        match inp.get_u8() {
            Some(m) => self.night_mode = u32::from(m),
            None => ok = false,
        }
        match inp.get_u8() {
            Some(b) => self.lit_flag = b != 0,
            None => ok = false,
        }
        self.night_acc = 0;
        ok &= inp.error() == 0;
        ok
    }

    // ---- helpers ----

    /// The building's centre cell (§5.2): the bounding box's centre divided
    /// by twice the cell width, truncated.
    fn building_centre_cell(&self, b: u64) -> (i32, i32) {
        let (min, max) = self.host.occupant_bounding_box(b);
        let (cw_x, cw_z) = self.host.city_cell_widths();
        let cx = ftol32(r24(
            r24(f64::from(min[0]) + f64::from(max[0])) / r24(f64::from(cw_x) + f64::from(cw_x))
        ));
        let cz = ftol32(r24(
            r24(f64::from(min[2]) + f64::from(max[2])) / r24(f64::from(cw_z) + f64::from(cw_z))
        ));
        (cx, cz)
    }

    /// Reads a lot counter, creating it as 0 when absent (§0 table).
    fn read_counter(&self, lot: u64, id: u32) -> u32 {
        match self.host.lot_property(lot, id) {
            Some(v) => v,
            None => {
                self.host.lot_set_property(lot, id, 0);
                0
            }
        }
    }

    fn write_counter(&self, lot: u64, id: u32, v: u32) {
        self.host.lot_set_property(lot, id, v);
    }

    fn draw_u32(&self, kind: DrawKind, n: u32) -> u32 {
        let mut s = self.statics.borrow_mut();
        let v = s.rng.uniform_u32(n);
        s.record(kind, n, f64::from(v));
        v
    }

    fn draw_gaussian(&self, kind: DrawKind, a: f64, b: f64) -> f64 {
        let mut s = self.statics.borrow_mut();
        let v = s.rng.range_gaussian(a, b);
        s.record(kind, 0, v);
        v
    }

    fn record(&self, kind: DrawKind, arg: u32, value: f64) {
        self.statics.borrow_mut().record(kind, arg, value);
    }
}

/// `IsEmptyLotSuitableForZot(lot, x, z)` (§5.6): zones 1–6 →
/// `(x + z) mod 4 = 0`; zones 7–9 → `x mod 4 = 0 and z mod 4 = 0`; other
/// zones → false. C remainder (truncating), as Rust's `%`.
pub fn is_empty_lot_suitable_for_zot(zone: i32, x: i32, z: i32) -> bool {
    match zone {
        1..=6 => x.wrapping_add(z) % 4 == 0,
        7..=9 => x % 4 == 0 && z % 4 == 0,
        _ => false,
    }
}

/// One rubble axis clamp (§7.3): shift the position back inside the box when
/// the prop's bounding box sticks out.
fn clamp_rubble(pos: f32, bmin: f32, bmax: f32, min: f32, max: f32) -> f32 {
    if bmin < min {
        (r24(f64::from(pos) + r24(f64::from(min) - f64::from(bmin)))) as f32
    } else if bmax > max {
        (r24(f64::from(pos) + r24(f64::from(max) - f64::from(bmax)))) as f32
    } else {
        pos
    }
}

/// Whether `lot` is `unit`'s first member (§4.2 step 5).
fn is_first_member(host: &Rc<dyn LotDeveloperHost>, unit: u64, lot: u64) -> bool {
    host.unit_members(unit).first().copied() == Some(lot)
}
