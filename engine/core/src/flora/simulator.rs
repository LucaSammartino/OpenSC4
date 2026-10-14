// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cISC4FloraSimulator` (§3–§11).

use std::any::Any;
use std::cell::{Cell, RefCell, RefMut};
use std::collections::HashSet;
use std::rc::Rc;

use crate::grid::{CellIterator, MODE_TABLE};
use crate::property::PropertyHolder;
use crate::props::{self, round_half_even_f32};
use crate::rng::CrtRand;
use crate::stream::{InStream, OutStream, ResKey, SaveObject};

use super::registry::{advance_cluster_type, FloraRecord, SharedRegistry};
use super::{
    blocks_flora, blocks_per_month, boxes_intersect, elevation_index, flag, ftol, msg, obliterated_occupant_key,
    occupant_type, prop, scale_wild_seeds, score_from_table, state, CellRect, FloraHost, FloraOccupant, FloraRandom,
    Message, OccupantFilter, OccupantId, RngState, SegmentId, StreamId, Vec3, AGENT_FLAGS, AGENT_LEVELS, AGENT_NAME,
    BURNT_TREE, CLSID_FLORA_SIMULATOR, FLORA_CHEAT, FLORA_CHEAT_NAME, FLORA_EXEMPLAR_TYPE, IID_OCCUPANT,
    MAX_BLAST_CENTRES, MAX_BLAST_SEEDS, MAX_BLOCKS_PER_MONTH, MAX_BURNT_COUNT, MAX_MERGE_ROUNDS, MAX_OBLITERATED,
    MAX_SEEDS_PER_CYCLE, MAX_WILD_SEEDS, OBLITERATED_COUNT_KEY, PLACE_FLORA_COMMAND, PLACE_FLORA_NAME, TUNING_INSTANCE,
};

/// The record version (§10.1); every other version is rejected.
pub const FLORA_RECORD_VERSION: u16 = 2;

/// [choice] The most type draws wild seeding makes per month (a scripted
/// generator could keep drawing skipped types).
const MAX_WILD_DRAWS: u32 = 1 << 20;

/// The `max.y` of the building and network clearance boxes (§6.10.1):
/// `2 + 1.7e38` (and `0 + 1.7e38`), one rounding.
fn sky(base: f64) -> f32 {
    (base + 1.7e38) as f32
}

/// One 24-bit rounding (foundations §18).
fn r24(x: f64) -> f32 {
    x as f32
}

/// The simulator state (§3.1) apart from the generators.
#[derive(Clone, Debug, PartialEq)]
pub struct FloraData {
    pub initialized: bool,
    pub ref_count: u32,
    /// Set when `Release` destroyed the object (§1).
    pub destroyed: bool,
    pub enabled: bool,
    pub propagation: bool,
    pub propagation_forced: bool,
    pub preserve: bool,
    pub size_x: f32,
    pub size_z: f32,
    pub cells_x: i32,
    pub cells_z: i32,
    pub today: u32,
    /// Months per full pass.
    pub period: u32,
    pub blocks_per_month: u32,
    pub wild_seeds_per_month: u32,
    /// Never set by the original (only read).
    pub keep_iterator_flag: u8,
    /// The 2 × 2-cell block iterator (table mode after `Init`).
    pub iterator: CellIterator,
    pub elevation_scale: f32,
    /// Burnt flora, oldest first; one host reference each.
    pub burnt: Vec<OccupantId>,
    pub burn_accumulator: f32,
    pub burnt_removal_accumulator: f32,
    pub burn_rate: f32,
    pub burnt_removal_step: f32,
    /// Whether the simulator (and the agents) are held (§4.1 step 4).
    pub simulator_held: bool,
}

impl Default for FloraData {
    /// The constructor values (§3.1).
    fn default() -> Self {
        FloraData {
            initialized: false,
            ref_count: 0,
            destroyed: false,
            enabled: true,
            propagation: true,
            propagation_forced: false,
            preserve: false,
            size_x: 1.0,
            size_z: 1.0,
            cells_x: 1,
            cells_z: 1,
            today: 0,
            period: 6,
            blocks_per_month: 1,
            wild_seeds_per_month: 5,
            keep_iterator_flag: 0,
            iterator: CellIterator::constructed(),
            elevation_scale: 0.08,
            burnt: Vec::new(),
            burn_accumulator: 0.0,
            burnt_removal_accumulator: 0.0,
            burn_rate: 1.0,
            burnt_removal_step: 4.0,
            simulator_held: false,
        }
    }
}

/// The number of seeds per cycle (§6.7.2): `roundHalfEven(v)`; [choice] NaN
/// and negative values give 0, large values are capped.
fn seed_count(v: f32) -> i64 {
    let r = round_half_even_f32(v);
    if r >= 1.0 {
        ftol(f64::from(r)).clamp(0, MAX_SEEDS_PER_CYCLE)
    } else {
        0
    }
}

/// The deterministic outcome of one visit of §6.7.2
/// ([`predict_visit`]): what `SimulateExistingFloraOccupant` does to the
/// occupant itself, without the seeds it drops (random positions, §8).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VisitOutcome {
    /// The record was found (no record: nothing happens).
    pub known: bool,
    /// Rebirth: birth date := today, state := 0 (nothing else).
    pub reborn: bool,
    /// The state after the visit.
    pub state: i8,
    /// Removed from the occupant manager (a change to stage 0 or 4).
    pub removed: bool,
    /// The seeding loop ran at least once: last seeding date := now.
    pub seeded: bool,
    /// The visit read the score (no rebirth).
    pub scored: bool,
}

/// One visit of §6.7.2 as a pure function of the occupant's values, its
/// record (`None`: unknown type), `today`, `propagation`, the simulator's
/// day number `now` and its score at the occupant's position (`score` is
/// only called when the original reads it). The same rule as
/// [`FloraSim::simulate_existing`] (tested against it); the flora
/// shadow applies it to the original's census.
pub fn predict_visit(
    rec: Option<&FloraRecord>,
    fo: &FloraOccupant,
    today: u32,
    propagation: bool,
    now: u32,
    score: impl FnOnce() -> f32,
) -> VisitOutcome {
    let Some(rec) = rec else {
        return VisitOutcome {
            state: fo.state,
            ..VisitOutcome::default()
        };
    };
    let age = today.wrapping_sub(fo.birth_date);
    let a = f64::from(age);
    let st = fo.state;
    let mut stage = if a >= f64::from(rec.max_age) {
        state::DYING
    } else if a >= f64::from(rec.lifespan) {
        state::BURNT
    } else if a >= f64::from(rec.maturity_age) {
        state::MATURE
    } else if a >= f64::from(0.5 * rec.maturity_age) {
        state::MATURING
    } else {
        state::YOUNG
    };
    if st >= state::BURNT || stage >= state::BURNT {
        return VisitOutcome {
            known: true,
            reborn: true,
            state: state::YOUNG,
            ..VisitOutcome::default()
        };
    }
    let s = score();
    let mut can_seed = false;
    if propagation || !rec.has_flag(flag::SKIP_SCORE) {
        if s < rec.min_score {
            stage = state::DYING;
        } else if a >= f64::from(rec.seeding_age) {
            can_seed = true;
        }
    }
    let removed = stage != st && (stage == state::YOUNG || stage == state::DYING);
    let seeded = propagation
        && can_seed
        && f64::from(r24(f64::from(fo.last_seeding_date) + f64::from(rec.seeding_interval))) < f64::from(now)
        && seed_count(rec.seeds_per_cycle) > 0;
    VisitOutcome {
        known: true,
        reborn: false,
        state: stage,
        removed,
        seeded,
        scored: true,
    }
}

/// The digits a `blast` word starts with ([choice], saturating).
fn parse_unsigned(w: Option<&&str>) -> u32 {
    let Some(w) = w else { return 0 };
    let mut v: u64 = 0;
    for c in w.bytes() {
        if !c.is_ascii_digit() {
            break;
        }
        v = (v * 10 + u64::from(c - b'0')).min(u64::from(u32::MAX));
    }
    v as u32
}

/// The flora simulator.
pub struct FloraSim {
    host: Rc<dyn FloraHost>,
    registry: SharedRegistry,
    token: Cell<u64>,
    d: RefCell<FloraData>,
    rng: RefCell<Box<dyn FloraRandom>>,
    crt: RefCell<CrtRand>,
}

impl FloraSim {
    /// A new simulator with the constructor values of §3.1, the process's
    /// registry and the given generator (normally a [`super::FloraRng`]).
    pub fn new(
        host: Rc<dyn FloraHost>,
        registry: SharedRegistry,
        token: u64,
        rng: Box<dyn FloraRandom>,
    ) -> Rc<FloraSim> {
        Rc::new(FloraSim {
            host,
            registry,
            token: Cell::new(token),
            d: RefCell::new(FloraData::default()),
            rng: RefCell::new(rng),
            crt: RefCell::new(CrtRand::default()),
        })
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    pub fn registry(&self) -> SharedRegistry {
        self.registry.clone()
    }

    /// Replaces the generator.
    pub fn set_random(&self, rng: Box<dyn FloraRandom>) {
        *self.rng.borrow_mut() = rng;
    }

    /// The generator's saved state.
    pub fn rng_state(&self) -> RngState {
        self.rng.borrow().state()
    }

    /// `RandomDoubleUniform()` on the simulator's generator (tests).
    pub fn draw_uniform(&self) -> f64 {
        self.rng.borrow_mut().uniform_f64()
    }

    /// The `rand()` generator of the block iterator.
    pub fn crt(&self) -> CrtRand {
        *self.crt.borrow()
    }

    pub fn set_crt(&self, crt: CrtRand) {
        *self.crt.borrow_mut() = crt;
    }

    pub fn snapshot(&self) -> FloraData {
        self.d.borrow().clone()
    }

    /// Direct state access, for hosts that restore state and for tests.
    pub fn data_mut(&self) -> RefMut<'_, FloraData> {
        self.d.borrow_mut()
    }

    fn record(&self, t: u32) -> Option<Rc<FloraRecord>> {
        self.registry.borrow().get(t)
    }

    // ---- lifecycle (§4) ----------------------------------------------------------------------------

    /// `Init()` (§4.1). Always true.
    pub fn init(&self) -> bool {
        if self.d.borrow().initialized {
            return true;
        }
        self.d.borrow_mut().initialized = true;
        let h = self.host.clone();
        let first = !self.registry.borrow().is_loaded();
        if first {
            self.registry.borrow_mut().set_loaded(true);
            h.register_app_shutdown_callback();
            self.load_all_types();
        }
        h.subscribe(&msg::SUBSCRIPTIONS);
        h.register_command(PLACE_FLORA_COMMAND, PLACE_FLORA_NAME);
        if !self.d.borrow().simulator_held {
            self.d.borrow_mut().simulator_held = true;
            for level in AGENT_LEVELS {
                h.add_agent(level, AGENT_NAME, AGENT_FLAGS);
            }
        }
        h.acquire_services();
        if h.city_exists() {
            let (sx, sz) = h.city_size();
            let (cx, cz) = h.city_cell_counts();
            let mut d = self.d.borrow_mut();
            d.size_x = sx;
            d.size_z = sz;
            d.cells_x = cx;
            d.cells_z = cz;
            d.blocks_per_month = blocks_per_month(cx, cz, d.period);
        }
        if self.d.borrow().keep_iterator_flag == 0 {
            {
                let mut d = self.d.borrow_mut();
                let x1 = (d.cells_x / 2).wrapping_sub(1).max(0);
                let z1 = (d.cells_z / 2).wrapping_sub(1).max(0);
                d.iterator.set_rect(0, 0, x1, z1);
                d.iterator.dirty = 1;
                d.iterator.mode = MODE_TABLE;
            }
            self.reset_iterator();
        }
        self.registry.borrow_mut().insert_default();
        self.read_tuning();
        let established = h.city_established();
        {
            let mut d = self.d.borrow_mut();
            d.propagation = !established || d.propagation_forced;
        }
        h.register_cheat(FLORA_CHEAT, FLORA_CHEAT_NAME);
        if let Some(seg) = h.obliterated_segment() {
            self.restore_obliterated(seg);
        }
        true
    }

    /// `Shutdown()` (§4.2). Always true.
    pub fn shutdown(&self) -> bool {
        if !self.d.borrow().initialized {
            return true;
        }
        let burnt = {
            let mut d = self.d.borrow_mut();
            d.initialized = false;
            std::mem::take(&mut d.burnt)
        };
        let h = self.host.clone();
        for o in burnt {
            h.release(o);
        }
        h.unregister_cheat(FLORA_CHEAT);
        h.release_services();
        if std::mem::take(&mut self.d.borrow_mut().simulator_held) {
            h.remove_agent();
        }
        h.unsubscribe(&msg::UNSUBSCRIPTIONS);
        h.unregister_command(PLACE_FLORA_COMMAND);
        true
    }

    pub fn is_initialized(&self) -> bool {
        self.d.borrow().initialized
    }

    /// `AddRef()`.
    pub fn add_ref(&self) -> u32 {
        let mut d = self.d.borrow_mut();
        d.ref_count = d.ref_count.wrapping_add(1);
        d.ref_count
    }

    /// `Release()` (§1): above 1, decrement; otherwise `Shutdown()` and
    /// destruction; returns 0.
    pub fn release(&self) -> u32 {
        {
            let mut d = self.d.borrow_mut();
            if d.ref_count > 1 {
                d.ref_count -= 1;
                return d.ref_count;
            }
        }
        self.shutdown();
        self.d.borrow_mut().destroyed = true;
        0
    }

    /// Resets the block iterator with shuffling (the shared iterator
    /// refills the table in natural order first, weather §6.7).
    fn reset_iterator(&self) {
        let mut d = self.d.borrow_mut();
        let mut crt = self.crt.borrow_mut();
        d.iterator.reset(true, &mut crt);
    }

    /// Loads every flora exemplar and posts FloraTypesUpdated (§4.1 step 2).
    fn load_all_types(&self) {
        for (_key, ex) in self.host.flora_exemplars() {
            if props::u32_of(&ex, prop::EXEMPLAR_TYPE) == Some(FLORA_EXEMPLAR_TYPE) {
                self.add_static_flora_data_from(Some(&ex));
            }
        }
        let n = self.registry.borrow().map_len() as u32;
        self.host.post(msg::FLORA_TYPES_UPDATED, n);
    }

    /// The tuning read (§7): nothing changes without the exemplar.
    pub fn read_tuning(&self) {
        let Some(h) = self.host.tuning() else { return };
        let mut d = self.d.borrow_mut();
        if let Some(v) = props::f32_of(&h, prop::BURN_RATE) {
            d.burn_rate = r24(1.0 / f64::from(v));
        }
        if let Some(v) = props::f32_of(&h, prop::BURNT_REMOVAL_STEP) {
            d.burnt_removal_step = v;
        }
        if let Some(v) = props::u32_of(&h, prop::PERIOD) {
            d.period = v;
        }
        if let Some(v) = props::u32_of(&h, prop::WILD_SEEDS) {
            d.wild_seeds_per_month = v;
        }
        if let Some(v) = props::f32_of(&h, prop::ELEVATION_SCALE) {
            d.elevation_scale = r24(16.0 / f64::from(v));
        }
        d.blocks_per_month = blocks_per_month(d.cells_x, d.cells_z, d.period);
        d.wild_seeds_per_month = scale_wild_seeds(d.wild_seeds_per_month, d.size_x, d.size_z);
    }

    // ---- messages (§5) -----------------------------------------------------------------------------

    /// `DoMessage(msg)` (§5). Always true.
    pub fn do_message(&self, m: &Message) -> bool {
        let h = self.host.clone();
        match m.msg_type {
            msg::SIM_NEW_DAY => self.daily(),
            msg::SIM_NEW_MONTH => {
                let enabled = {
                    let mut d = self.d.borrow_mut();
                    d.today = m.data1;
                    d.enabled
                };
                if enabled {
                    self.monthly();
                }
            }
            msg::INSERT_OCCUPANT => {
                if let Some(occ) = m.occupant {
                    if !h.occupant_has_property(occ, prop::PREVIEW_OCCUPANT) {
                        self.occupant_inserted(occ, m.data3);
                    }
                }
            }
            msg::REMOVE_OCCUPANT => {
                if let Some(occ) = m.occupant {
                    if !h.occupant_has_property(occ, prop::PREVIEW_OCCUPANT) && m.data3 == occupant_type::FLORA {
                        let found = {
                            let mut d = self.d.borrow_mut();
                            match d.burnt.iter().position(|&o| o == occ) {
                                Some(i) => {
                                    d.burnt.remove(i);
                                    true
                                }
                                None => false,
                            }
                        };
                        if found {
                            h.release(occ);
                        }
                    }
                }
            }
            msg::TERRAIN_HEIGHTS_CHANGED => {
                if h.terrain_initialized() {
                    let (cx, cz) = {
                        let d = self.d.borrow();
                        (d.cells_x, d.cells_z)
                    };
                    let rect = (
                        ((m.data1 >> 16) as i32 - 1).max(0),
                        ((m.data2 >> 16) as i32 - 1).max(0),
                        ((m.data1 & 0xFFFF) as i32).min(cx.wrapping_sub(1)),
                        ((m.data2 & 0xFFFF) as i32).min(cz.wrapping_sub(1)),
                    );
                    self.terrain_altitude_change(rect);
                }
            }
            msg::CITY_ESTABLISHED => {
                let mut d = self.d.borrow_mut();
                d.propagation = d.propagation_forced;
            }
            msg::CITY_OBLITERATED => self.d.borrow_mut().propagation = true,
            msg::PLACE_FLORA => {
                if let [t, p1, p2, ..] = m.params.as_slice() {
                    // [quirk §14.3] p2 lands in y and z = −1.
                    let mut pos = [p1.as_f32, p2.as_f32, -1.0];
                    self.add_new_flora_occupant(t.as_u32, &mut pos, false);
                }
            }
            msg::CHEAT => {
                let words: Vec<&str> = m.text.as_deref().unwrap_or("").split_ascii_whitespace().collect();
                if m.data1 == FLORA_CHEAT && words.len() >= 2 {
                    self.cheat(&words);
                } else {
                    // [quirk §14.2] handled like the exemplar-changed message.
                    self.exemplar_changed(m.data2);
                }
            }
            msg::EXEMPLAR_CHANGED => self.exemplar_changed(m.data2),
            _ => {}
        }
        true
    }

    /// The exemplar-changed handler (§5): reload every type when `inst` is a
    /// registered type, re-read the tuning for the tuning instance.
    pub fn exemplar_changed(&self, inst: u32) {
        let known = self.registry.borrow().contains(inst);
        if known {
            self.registry.borrow_mut().clear();
            self.load_all_types();
        }
        if inst == TUNING_INSTANCE {
            self.read_tuning();
        }
    }

    /// The `Flora …` cheat (§5.2); `words[0]` is the cheat name.
    pub fn cheat(&self, words: &[&str]) {
        let Some(w1) = words.get(1) else { return };
        let off = words.get(2).is_some_and(|w| w.eq_ignore_ascii_case("off"));
        match w1.to_ascii_lowercase().as_str() {
            "on" => self.d.borrow_mut().enabled = true,
            "off" => self.enable_flora(false),
            "preserve" => self.d.borrow_mut().preserve = !off,
            "propogation" => {
                let mut d = self.d.borrow_mut();
                d.propagation_forced = !off;
                d.propagation = !off;
            }
            "blast" => self.blast(
                parse_unsigned(words.get(2)),
                parse_unsigned(words.get(3)),
                parse_unsigned(words.get(4)),
            ),
            _ => {}
        }
    }

    /// `Flora blast n m s` (§5.2).
    pub fn blast(&self, n: u32, m: u32, s: u32) {
        let (sx, sz) = {
            let d = self.d.borrow();
            (d.size_x, d.size_z)
        };
        let mut total: u64 = 0;
        'centres: for _ in 0..n.min(MAX_BLAST_CENTRES) {
            let cx = r24(self.rng.borrow_mut().uniform_f64() * f64::from(sx));
            let cz = r24(self.rng.borrow_mut().uniform_f64() * f64::from(sz));
            for _ in 0..m {
                if total >= MAX_BLAST_SEEDS {
                    break 'centres;
                }
                total += 1;
                let gx = self.rng.borrow_mut().gaussian();
                let x = r24(f64::from(r24(gx * f64::from(s))) + f64::from(cx));
                let gz = self.rng.borrow_mut().gaussian();
                let z = r24(f64::from(r24(gz * f64::from(s))) + f64::from(cz));
                // [quirk §14.13] inclusive in z.
                if x >= 0.0 && z >= 0.0 && x < sx && z <= sz {
                    let (t, _) = self.best_type(x, z);
                    self.drop_seed(t, x, z);
                }
            }
        }
    }

    /// `EnableFlora(on)` (§5.2, §6.10.4): only a true → false change removes
    /// every flora occupant of the city.
    pub fn enable_flora(&self, on: bool) {
        if on {
            self.d.borrow_mut().enabled = true;
            return;
        }
        let was = std::mem::replace(&mut self.d.borrow_mut().enabled, false);
        if was {
            for o in self.host.occupants_in_cells(None, occupant_type::FLORA) {
                self.demolish_flora_occupant(o, 0);
            }
        }
    }

    // ---- scores and placement (§6.1–§6.6) ----------------------------------------------------------

    /// `ElevationIndex(x, z, water)` (§6.1).
    pub fn elevation_index_at(&self, x: f32, z: f32, water: bool) -> usize {
        let alt = self.host.altitude(x, z);
        let sea = self.host.sea_level();
        elevation_index(alt, sea, water, self.d.borrow().elevation_scale)
    }

    /// `Score(type, x, z)` (§6.2); 0 for an unknown type.
    pub fn score(&self, t: u32, x: f32, z: f32) -> f32 {
        let Some(rec) = self.record(t) else { return 0.0 };
        let m = self.host.moisture_type(x, z);
        let e = self.elevation_index_at(x, z, rec.water);
        score_from_table(&rec.table, e, m)
    }

    /// `GetBestFloraTypeForLocation(x, z, &score)` (§6.3): (type, score).
    pub fn best_type(&self, x: f32, z: f32) -> (u32, f32) {
        let records = self.registry.borrow().records();
        let m = self.host.moisture_type(x, z);
        let mut e_cache: [Option<usize>; 2] = [None, None];
        let mut best = (0u32, 0.0f32);
        for rec in records {
            if rec.has_flag(flag::NEVER_AUTO) {
                continue;
            }
            let slot = usize::from(rec.water);
            let e = match e_cache[slot] {
                Some(e) => e,
                None => {
                    let e = self.elevation_index_at(x, z, rec.water);
                    e_cache[slot] = Some(e);
                    e
                }
            };
            let s = score_from_table(&rec.table, e, m);
            if s > best.1 {
                best = (rec.type_id, s);
            }
        }
        best
    }

    /// `LocationIsOKForNewFlora(type, x, z, exclude)` (§6.4).
    pub fn location_ok(&self, t: u32, x: f32, z: f32, exclude: Option<OccupantId>) -> bool {
        let Some(rec) = self.record(t) else { return false };
        let h = self.host.clone();
        let (alt, on_lot) = h.lot_savvy_altitude(x, z);
        if !on_lot {
            if alt < h.sea_level() && !rec.water {
                return false;
            }
            let n = h.face_normal(x, z);
            if n[1] <= 1.0 - rec.slope_tolerance {
                return false;
            }
        }
        let r = rec.clearance_radius;
        let min = [x - r, alt - 16.0, z - r];
        let max = [x + r, alt + rec.height, z + r];
        let (sx, sz) = h.city_size();
        if min[0] < 0.0 || min[2] < 0.0 || sx < max[0] || sz < max[2] {
            return false;
        }
        for o in h.occupants_in_bbox(min, max, OccupantFilter::Blocking) {
            if Some(o) != exclude && boxes_intersect(h.occupant_bounding_box(o), (min, max)) {
                return false;
            }
        }
        true
    }

    /// `AddNewFloraOccupant(type, pos, force)` (§6.5): the inserted occupant
    /// (borrowed) or `None`. `pos` is rewritten [quirk §14.4].
    pub fn add_new_flora_occupant(&self, t: u32, pos: &mut Vec3, force: bool) -> Option<OccupantId> {
        let h = self.host.clone();
        let mut t = t;
        if t == 0 {
            t = self.best_type(pos[0], pos[2]).0;
            if t == 0 {
                return None;
            }
        } else if !self.registry.borrow().contains(t) {
            return None;
        }
        if !force && self.registry.borrow().contains(t) {
            for _ in 0..MAX_MERGE_ROUNDS {
                let Some(rec) = self.record(t) else { break };
                let mut n: i64 = 1;
                let mut sx = 2.0 * pos[0];
                let mut sz = 2.0 * pos[2];
                let alt = h.altitude(pos[0], pos[2]);
                let (cr, ch) = (rec.cluster_radius, rec.cluster_height);
                let min = [pos[0] - cr, alt - ch, pos[2] - cr];
                let max = [pos[0] + cr, alt + ch, pos[2] + cr];
                for o in h.occupants_in_bbox(min, max, OccupantFilter::Type(occupant_type::FLORA)) {
                    let Some(fo) = h.flora_occupant(o) else { continue };
                    let t2 = advance_cluster_type(&self.registry.borrow(), fo.flora_type, t);
                    if t2 == 0 {
                        continue;
                    }
                    let (bmin, bmax) = h.occupant_bounding_box(o);
                    let cx = bmin[0] + bmax[0];
                    let cz = bmin[2] + bmax[2];
                    let div = (2 * (n + 1)) as f64;
                    let x = r24(f64::from(sx + cx) / div);
                    let z = r24(f64::from(sz + cz) / div);
                    if self.location_ok(t2, x, z, Some(o)) {
                        h.remove_occupant(o);
                        sx += cx;
                        sz += cz;
                        t = t2;
                        n += 1;
                    }
                }
                let div = (2 * n) as f64;
                *pos = [r24(f64::from(sx) / div), -1.0, r24(f64::from(sz) / div)];
                if n <= 1 {
                    break;
                }
            }
            if !self.location_ok(t, pos[0], pos[2], None) {
                return None;
            }
        }
        let occ = h.create_flora_occupant()?;
        h.set_flora_type(occ, t);
        let orientation = self.rng.borrow_mut().range_i32(0, 4);
        h.set_flora_orientation(occ, orientation as u32);
        let rec = self.record(t);
        if pos[1] <= 0.0 {
            let (mut y, _) = h.lot_savvy_altitude(pos[0], pos[2]);
            if !rec.as_ref().is_some_and(|r| r.water) {
                let sea = h.sea_level();
                if y < sea {
                    y = sea;
                }
            }
            pos[1] = y;
            if pos[1] < 0.0 {
                h.release(occ);
                return None;
            }
        }
        if let Some(r) = rec {
            h.set_flora_record(occ, Some(r));
        }
        h.set_occupant_position(occ, *pos);
        let now = h.sim_date_number();
        h.set_birth_date(occ, now);
        h.set_last_seeding_date(occ, now);
        h.init_occupant(occ);
        let ok = self.add_flora_occupant(occ);
        h.release(occ);
        ok.then_some(occ)
    }

    /// `AddNewFloraOccupant(occ)` (slot 18): the occupant manager's insert;
    /// false without one.
    pub fn add_flora_occupant(&self, occ: OccupantId) -> bool {
        self.host.has_occupant_manager() && self.host.insert_occupant(occ)
    }

    /// `DropSeed(type, x, z)` (§6.6).
    pub fn drop_seed(&self, t: u32, x: f32, z: f32) -> bool {
        if !self.location_ok(t, x, z, None) {
            return false;
        }
        let Some(rec) = self.record(t) else { return false };
        let s = self.score(t, x, z);
        if s < rec.min_score {
            return false;
        }
        let u = self.rng.borrow_mut().uniform_f64();
        let lhs = r24(f64::from(s - rec.min_score) * u);
        let rhs = (1.0 - rec.min_score) * rec.seed_probability;
        if lhs < rhs {
            let mut pos = [x, 0.0, z];
            self.add_new_flora_occupant(t, &mut pos, false);
            return true;
        }
        false
    }

    // ---- monthly (§6.7) ----------------------------------------------------------------------------

    /// The monthly simulation (§6.7): block visits, then wild seeds. The
    /// caller checks `enabled`.
    pub fn monthly(&self) {
        let h = self.host.clone();
        let mut k = self.d.borrow().blocks_per_month.min(MAX_BLOCKS_PER_MONTH);
        let mut wrapped = false;
        while k > 0 {
            let next = self.d.borrow_mut().iterator.next_cell();
            let Some((bx, bz)) = next else {
                if wrapped {
                    break; // [choice] an empty iterator
                }
                wrapped = true;
                self.reset_iterator();
                continue;
            };
            wrapped = false;
            let (x0, z0) = (bx.wrapping_mul(2), bz.wrapping_mul(2));
            let rect = (x0, z0, x0.wrapping_add(1), z0.wrapping_add(1));
            for o in h.occupants_in_cells(Some(rect), occupant_type::FLORA) {
                if h.occupant_in_manager(o) {
                    if let Some(fo) = h.flora_occupant(o) {
                        self.simulate_existing(o, fo);
                    }
                }
            }
            k -= 1;
        }
        self.wild_seeds();
    }

    /// `SimulateExistingFloraOccupant` (§6.7.2).
    pub fn simulate_existing(&self, occ: OccupantId, fo: FloraOccupant) {
        let h = self.host.clone();
        let (today, propagation, sx, sz) = {
            let d = self.d.borrow();
            (d.today, d.propagation, d.size_x, d.size_z)
        };
        let age = today.wrapping_sub(fo.birth_date);
        let Some(rec) = self.record(fo.flora_type) else { return };
        let st = fo.state;
        let a = f64::from(age);
        let reset = st >= state::BURNT;
        let mut stage = if a >= f64::from(rec.max_age) {
            state::DYING
        } else if a >= f64::from(rec.lifespan) {
            state::BURNT
        } else if a >= f64::from(rec.maturity_age) {
            state::MATURE
        } else if a >= f64::from(0.5 * rec.maturity_age) {
            state::MATURING
        } else {
            state::YOUNG
        };
        if reset || stage >= state::BURNT {
            // [quirk §14.6] rebirth as a young plant.
            h.set_birth_date(occ, today);
            h.set_flora_state(occ, state::YOUNG);
            h.post_occupant_change(occ);
            return;
        }
        let pos = h.occupant_position(occ);
        let s = self.score(fo.flora_type, pos[0], pos[2]);
        let mut can_seed = false;
        if propagation || !rec.has_flag(flag::SKIP_SCORE) {
            if s < rec.min_score {
                stage = state::DYING;
            } else if a >= f64::from(rec.seeding_age) {
                can_seed = true;
            }
        }
        if stage != st {
            h.set_flora_state(occ, stage);
            match stage {
                state::MATURING | state::MATURE => h.post_occupant_change(occ),
                state::YOUNG | state::DYING => {
                    h.remove_occupant(occ);
                }
                _ => {}
            }
        }
        if propagation && can_seed {
            let now = h.sim_date_number();
            let due = r24(f64::from(fo.last_seeding_date) + f64::from(rec.seeding_interval));
            if f64::from(due) < f64::from(now) {
                let spread = rec.seed_spread;
                let seed_type = rec.seed_type.get();
                for _ in 0..seed_count(rec.seeds_per_cycle) {
                    let x = self
                        .rng
                        .borrow_mut()
                        .range_f64(f64::from(pos[0] - spread), f64::from(pos[0] + spread));
                    let z = self
                        .rng
                        .borrow_mut()
                        .range_f64(f64::from(pos[2] - spread), f64::from(pos[2] + spread));
                    if x >= 0.0 && x < f64::from(sx) && z >= 0.0 && z < f64::from(sz) {
                        self.drop_seed(seed_type, x as f32, z as f32);
                    }
                    h.set_last_seeding_date(occ, now);
                }
            }
        }
    }

    /// Wild seeds (§6.7.3).
    pub fn wild_seeds(&self) {
        let h = self.host.clone();
        let (propagation, wild, sx, sz) = {
            let d = self.d.borrow();
            (d.propagation, d.wild_seeds_per_month, d.size_x, d.size_z)
        };
        if !propagation || !h.terrain_initialized() || wild == 0 {
            return;
        }
        let types = self.registry.borrow().types().to_vec();
        let eligible = |t: u32| self.record(t).is_some_and(|r| !r.has_flag(flag::NEVER_AUTO));
        if !types.iter().any(|&t| eligible(t)) {
            return; // [choice] the original hangs or reads out of bounds
        }
        let (wx, wz) = (ftol(f64::from(sx)) as u32, ftol(f64::from(sz)) as u32);
        let mut k = wild.min(MAX_WILD_SEEDS);
        let mut draws = 0;
        while k > 0 && draws < MAX_WILD_DRAWS {
            draws += 1;
            let i = self.rng.borrow_mut().uniform_u32(types.len() as u32);
            let Some(&t) = types.get(i as usize) else { continue };
            if !eligible(t) {
                continue;
            }
            let x = self.rng.borrow_mut().uniform_u32(wx);
            let z = self.rng.borrow_mut().uniform_u32(wz);
            self.drop_seed(t, x as f32, z as f32);
            k -= 1;
        }
    }

    // ---- burnt flora (§6.8, §6.9) ------------------------------------------------------------------

    /// The daily burnt-flora removal (§6.8).
    pub fn daily(&self) {
        let n = {
            let mut d = self.d.borrow_mut();
            d.burn_accumulator += d.burn_rate;
            if d.burn_accumulator >= 1.0 {
                d.burn_accumulator -= 1.0;
                d.burnt_removal_accumulator += d.burnt_removal_step;
            }
            if d.burnt_removal_accumulator >= 1.0 {
                let n = ftol(f64::from(d.burnt_removal_accumulator.floor()));
                d.burnt_removal_accumulator = r24(f64::from(d.burnt_removal_accumulator) - n as f64);
                n
            } else {
                0
            }
        };
        let mut left = n;
        while left > 0 {
            let o = {
                let mut d = self.d.borrow_mut();
                if d.burnt.is_empty() {
                    break;
                }
                d.burnt.remove(0)
            };
            self.host.remove_occupant(o);
            self.host.release(o);
            left -= 1;
        }
    }

    /// `KillFloraOccupant(occ)` (§6.9).
    pub fn kill_flora_occupant(&self, occ: OccupantId) -> bool {
        let h = self.host.clone();
        let Some(fo) = h.flora_occupant(occ) else { return false };
        if !h.occupant_in_manager(occ) {
            return false;
        }
        if fo.state == state::BURNT {
            return true;
        }
        h.set_flora_state(occ, state::BURNT);
        h.set_flora_type(occ, BURNT_TREE);
        h.set_flora_record(occ, self.record(BURNT_TREE));
        h.post_occupant_change(occ);
        h.add_ref(occ);
        self.d.borrow_mut().burnt.push(occ);
        true
    }

    // ---- removal and relocation (§6.10) ------------------------------------------------------------

    /// The InsertOccupant handling (§6.10.1) for an occupant of type `t`.
    pub fn occupant_inserted(&self, occ: OccupantId, t: u32) {
        let h = self.host.clone();
        if t == occupant_type::BUILDING {
            let (mn, mx) = h.occupant_bounding_box(occ);
            let min = [mn[0] - 2.0, -2.0, mn[2] - 2.0];
            let max = [mx[0] + 2.0, sky(2.0), mx[2] + 2.0];
            self.demolish_in_bounding_box(min, max, 0);
        } else if t == occupant_type::NETWORK && blocks_flora(t, h.network_flags(occ)) {
            let (mn, mx) = h.occupant_bounding_box(occ);
            let min = [mn[0], 0.0, mn[2]];
            let max = [mx[0], sky(0.0), mx[2]];
            self.demolish_in_bounding_box(min, max, 0);
        }
    }

    /// `DoTerrainAltitudeChange(rect)` (§6.10.2).
    pub fn terrain_altitude_change(&self, rect: CellRect) {
        let h = self.host.clone();
        let preserve = self.d.borrow().preserve;
        for o in h.occupants_in_cells(Some(rect), occupant_type::FLORA) {
            let Some(fo) = h.flora_occupant(o) else { continue };
            let mut pos = h.occupant_position(o);
            let (alt, _) = h.lot_savvy_altitude(pos[0], pos[2]);
            if preserve || f64::from((alt - pos[1]).abs()) < 0.001 {
                let (mut y, _) = h.lot_savvy_altitude(pos[0], pos[2]);
                if !self.record(fo.flora_type).is_some_and(|r| r.water) {
                    let sea = h.sea_level();
                    if y < sea {
                        y = sea;
                    }
                }
                if f64::from((y - pos[1]).abs()) >= 0.001 {
                    pos[1] = y;
                    h.set_occupant_position(o, pos);
                    h.move_occupant(o);
                }
            } else {
                h.remove_occupant(o);
            }
        }
    }

    /// `DemolishFloraOccupant(occ, flags)` (slot 19): removes **any**
    /// occupant [quirk §14.7].
    pub fn demolish_flora_occupant(&self, occ: OccupantId, _flags: i32) -> bool {
        self.host.remove_occupant(occ)
    }

    /// `DemolishFloraInCellArea(rect, flags)` (slot 20).
    pub fn demolish_in_cell_area(&self, rect: CellRect, _flags: i32) -> bool {
        let mut all = true;
        for o in self.host.occupants_in_cells(Some(rect), occupant_type::FLORA) {
            let ok = self.demolish_flora_occupant(o, 0);
            all = all && ok;
        }
        all
    }

    /// `DemolishFloraInBoundingBox(min, max, flags)` (slot 21).
    pub fn demolish_in_bounding_box(&self, min: Vec3, max: Vec3, _flags: i32) -> bool {
        let mut all = true;
        for o in self
            .host
            .occupants_in_bbox(min, max, OccupantFilter::Type(occupant_type::FLORA))
        {
            let ok = self.demolish_flora_occupant(o, 0);
            all = all && ok;
        }
        all
    }

    /// `GetCostOfOccupantDemolition` (slot 22): always 0.
    pub fn cost_of_occupant_demolition(&self, _occ: OccupantId, _flags: i32) -> i64 {
        0
    }

    // ---- the type registry (slots 5–14) ------------------------------------------------------------

    /// `AddStaticFloraData(key, exemplar)` (slot 5): false without an
    /// exemplar (the plugin loads it by key when none is given).
    pub fn add_static_flora_data_from(&self, exemplar: Option<&PropertyHolder>) -> bool {
        let Some(ex) = exemplar else { return false };
        self.add_static_flora_data(Rc::new(FloraRecord::from_exemplar(ex)))
    }

    /// `AddStaticFloraData(record)` (slot 6).
    pub fn add_static_flora_data(&self, record: Rc<FloraRecord>) -> bool {
        self.registry.borrow_mut().add(record)
    }

    /// `GetStaticFloraData(type)` (slot 7).
    pub fn static_flora_data(&self, t: u32) -> Option<Rc<FloraRecord>> {
        self.record(t)
    }

    /// `GetAllFloraTypes(vector)` (slot 8): the type array.
    pub fn all_flora_types(&self) -> Vec<u32> {
        self.registry.borrow().types().to_vec()
    }

    /// `GetAllFloraTypes(buffer, count)` (slot 9). [choice] The copy is also
    /// limited to the buffer's length.
    pub fn all_flora_types_into(&self, buffer: Option<&mut [u32]>, count: &mut u32) -> u32 {
        let reg = self.registry.borrow();
        let types = reg.types();
        let n = types.len() as u32;
        match buffer {
            None => {
                *count = n;
                n
            }
            Some(buf) => {
                let c = (*count).min(n).min(buf.len() as u32);
                buf[..c as usize].copy_from_slice(&types[..c as usize]);
                *count = c;
                c
            }
        }
    }

    /// `GetAllFloraFamilyTypes(vector)` (slot 10).
    pub fn all_flora_family_types(&self) -> Vec<u32> {
        self.registry.borrow().family_ids()
    }

    /// `IsFloraTypeValid(type)` (slot 11).
    pub fn is_flora_type_valid(&self, t: u32) -> bool {
        self.registry.borrow().contains(t)
    }

    /// `IsFloraFamilyTypeValid(id)` (slot 12): a family or a registered type.
    pub fn is_flora_family_type_valid(&self, id: u32) -> bool {
        let reg = self.registry.borrow();
        reg.family(id).is_some() || reg.contains(id)
    }

    /// `GetFloraTypesByFilter(set, filter)` (slot 13): every map key the
    /// filter accepts (`None` = all) is inserted; returns the number of
    /// insert attempts.
    pub fn flora_types_by_filter(&self, set: &mut HashSet<u32>, filter: Option<&dyn Fn(u32) -> bool>) -> u32 {
        let keys = self.registry.borrow().keys();
        let mut n = 0u32;
        for t in keys {
            if filter.map_or(true, |f| f(t)) {
                set.insert(t);
                n = n.wrapping_add(1);
            }
        }
        n
    }

    /// `GetFloraTypesByFamily(set, family)` (slot 14).
    pub fn flora_types_by_family(&self, set: &mut HashSet<u32>, family: u32) -> u32 {
        let reg = self.registry.borrow();
        let Some(types) = reg.family(family) else { return 0 };
        set.extend(types.iter().copied());
        types.len() as u32
    }

    // ---- obliteration (§10.3) ----------------------------------------------------------------------

    /// Opens one obliteration record (§10.3, the game's record-stream
    /// helpers): the segment's stream, then the record's `u16` version (0
    /// written; read and never checked). A stream with `GetError()` set is
    /// closed again and the record is "not open".
    fn open_record(&self, seg: SegmentId, key: ResKey, write: bool) -> Option<StreamId> {
        let h = &self.host;
        let s = h.segment_open(seg, key, write)?;
        if write {
            h.stream_put_u16(s, 0);
        } else {
            let _version = h.stream_get_u16(s);
        }
        if h.stream_error(s) {
            h.segment_close(s);
            return None;
        }
        Some(s)
    }

    /// `SaveObliterated(segment)` (slot 24, §10.3): the count record
    /// (`u16 0`, `u32 count`, also for 0), then record `i` for the i-th
    /// flora occupant of the list (`i` advances for every occupant): `u16 0`
    /// and the occupant's own `Write`, with its flag 1 cleared around it.
    /// Always true.
    pub fn save_obliterated(&self, seg: SegmentId) -> bool {
        let h = self.host.clone();
        let list = h.occupants_in_manager_cells(occupant_type::FLORA);
        if let Some(s) = self.open_record(seg, OBLITERATED_COUNT_KEY, true) {
            h.stream_put_u32(s, list.len() as u32);
            h.segment_close(s);
        }
        for (i, &o) in list.iter().enumerate() {
            // [question] The flag is cleared only when the record opens.
            if let Some(s) = self.open_record(seg, obliterated_occupant_key(i as u32), true) {
                h.set_occupant_in_manager(o, false);
                h.write_occupant(o, s);
                h.set_occupant_in_manager(o, true);
                h.segment_close(s);
            }
        }
        true
    }

    /// Restores the obliterated city's flora (§10.3, `Init` step 12). The
    /// count (0 unless its record opens) is read before the occupant-manager
    /// test: false without an occupant manager, else true. For each `i` below
    /// the count whose record opens: a new flora occupant, its `Read` (result
    /// ignored), `Init`, `InsertOccupant`, `Release`.
    pub fn restore_obliterated(&self, seg: SegmentId) -> bool {
        let h = self.host.clone();
        let mut count = 0u32;
        if let Some(s) = self.open_record(seg, OBLITERATED_COUNT_KEY, false) {
            // [question] A failed GetUint32 leaves the count at 0.
            if let Some(v) = h.stream_get_u32(s) {
                count = v;
            }
            h.segment_close(s);
        }
        if !h.has_occupant_manager() {
            return false;
        }
        for i in 0..count.min(MAX_OBLITERATED) {
            let Some(s) = self.open_record(seg, obliterated_occupant_key(i), false) else {
                continue;
            };
            let Some(occ) = h.create_flora_occupant() else {
                h.segment_close(s);
                continue;
            };
            h.read_occupant(occ, s);
            // [question] The record is closed before Init and the insert.
            h.segment_close(s);
            h.init_occupant(occ);
            h.insert_occupant(occ);
            h.release(occ);
        }
        true
    }
}

// ---- persistence (§10.1) -----------------------------------------------------------------------------

impl SaveObject for FloraSim {
    fn class_id(&self) -> u32 {
        CLSID_FLORA_SIMULATOR
    }

    fn write(&self, out: &mut dyn OutStream) -> bool {
        let d = self.d.borrow().clone();
        out.put_u16(FLORA_RECORD_VERSION);
        out.put_bool(d.enabled);
        out.put_bool(d.propagation);
        out.put_bool(d.propagation_forced);
        out.put_bool(d.preserve);
        out.put_f32(d.size_x);
        out.put_f32(d.size_z);
        out.put_i32(d.cells_x);
        out.put_i32(d.cells_z);
        out.put_u32(d.today);
        out.put_u32(d.period);
        out.put_u32(d.blocks_per_month);
        out.put_u32(d.wild_seeds_per_month);
        out.put_u8(d.keep_iterator_flag);
        d.iterator.write(out);
        let s = self.rng.borrow().state();
        out.put_u32(s.lcg);
        out.put_u32(s.mul);
        out.put_i32(s.has_cached_gaussian);
        out.put_f64(s.cached_gaussian);
        out.put_u32(d.burnt.len() as u32);
        for &o in &d.burnt {
            out.put_ref(&self.host.object_ref(o));
        }
        out.put_f32(d.burn_accumulator);
        out.put_f32(d.burnt_removal_accumulator);
        out.error() == 0
    }

    fn read(&self, inp: &mut dyn InStream) -> bool {
        if inp.get_u16() != Some(FLORA_RECORD_VERSION) {
            return false;
        }
        let old_burnt = {
            let mut d = self.d.borrow_mut();
            let d = &mut *d;
            for field in [
                &mut d.enabled,
                &mut d.propagation,
                &mut d.propagation_forced,
                &mut d.preserve,
            ] {
                if let Some(b) = inp.get_u8() {
                    *field = b != 0;
                }
            }
            for field in [&mut d.size_x, &mut d.size_z] {
                if let Some(v) = inp.get_f32() {
                    *field = v;
                }
            }
            for field in [&mut d.cells_x, &mut d.cells_z] {
                if let Some(v) = inp.get_i32() {
                    *field = v;
                }
            }
            for field in [
                &mut d.today,
                &mut d.period,
                &mut d.blocks_per_month,
                &mut d.wild_seeds_per_month,
            ] {
                if let Some(v) = inp.get_u32() {
                    *field = v;
                }
            }
            if let Some(v) = inp.get_u8() {
                d.keep_iterator_flag = v;
            }
            if let Some(it) = CellIterator::read(inp) {
                d.iterator = it;
            }
            std::mem::take(&mut d.burnt)
        };
        let (lcg, mul, has, cached) = (inp.get_u32(), inp.get_u32(), inp.get_i32(), inp.get_f64());
        if let (Some(lcg), Some(mul), Some(has), Some(cached)) = (lcg, mul, has, cached) {
            self.rng.borrow_mut().set_state(RngState {
                lcg,
                mul,
                has_cached_gaussian: has,
                cached_gaussian: cached,
            });
        }
        // The list reader clears the list ([choice]: releasing its references).
        for o in old_burnt {
            self.host.release(o);
        }
        let count = inp.get_u32().unwrap_or(0);
        if count != 0 && count < MAX_BURNT_COUNT {
            let mut list = Vec::new();
            for _ in 0..count {
                let Some(r) = inp.get_ref() else { break };
                if r.is_null() {
                    continue;
                }
                if let Some(o) = self.host.resolve_ref(&r, IID_OCCUPANT) {
                    list.push(o);
                }
            }
            self.d.borrow_mut().burnt = list;
        }
        let mut d = self.d.borrow_mut();
        if let Some(v) = inp.get_f32() {
            d.burn_accumulator = v;
        }
        if let Some(v) = inp.get_f32() {
            d.burnt_removal_accumulator = v;
        }
        inp.error() == 0
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}
