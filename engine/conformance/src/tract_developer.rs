// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops `tdev_…` for `fc4_core::tract_developer`
//! (`docs/specs/tract-developer.md`): a scripted world (growth developers,
//! lots, lot configurations, buildings and families, cell grids, networks,
//! construction resources, cheats) implementing `TractDeveloperHost`, an
//! event log of the host calls that change something, the draw trace of the
//! shared statics, and ops that drive the tract developer. See README.md
//! ("Tract developer ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::{Rc, Weak};

use fc4_core::development::{ConstructionProfile, GrowthProfile, Outcome};
use fc4_core::property::PropertyHolder;
use fc4_core::rng::{CrtRand, FixedSeeds, RzRandom};
use fc4_core::stl::{stl_sort, StlHashSet};
use fc4_core::stream::{ByteReader, ByteWriter};
use fc4_core::tract_developer::{
    answers_iid, candidate_rect, capacity_window, configuration_filter_includes, corner_ok, desired_of, roads_check,
    score_candidate, style_group, style_points, zone_family, BuildingProfile, Candidate, Cap, CellRect, Cheat,
    DrawKind, SharedStatics, TractDeveloper, TractDeveloperHost, TractDeveloperStatics, Tuning, RNG_SEED_XOR,
};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{boolean, field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, f32_of, i64_of, ji, matches, opt, show, str_of, u32_of, EventLog, R,
};

// ---- world -------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Dev {
    target_zone: i32,
    subtype: u32,
    purpose: u32,
    wealth: u32,
    info: ConstructionProfile,
    tract_size: i32,
    shift: u32,
    desirability: BTreeMap<(i32, i32), i16>,
    capacity: BTreeMap<(i32, i32), u32>,
    demand: Option<f32>,
    caps: [u8; 4],
    monopoly_use: i32,
    styles: Vec<(u32, f32)>,
}

#[derive(Clone, Debug)]
struct Lot {
    x: i32,
    z: i32,
    w: u32,
    h: u32,
    facing: i32,
    slope: f32,
    zone: i32,
    state: u32,
    livable: bool,
    has_capacity: Option<bool>,
    days: u32,
    wealth: u32,
    historical: bool,
    building: u32,
    capacity: u16,
    available: Option<u16>,
    road: bool,
    organic: bool,
    cfg_stage: Option<u8>,
    can_create: i32,
    population: BTreeMap<u32, u16>,
}

impl Lot {
    /// The lot's rectangle from its location, size and facing (§4.10.1
    /// table, written out here independently of the core).
    fn rect(&self) -> CellRect {
        let (x, z, w, h) = (self.x, self.z, self.w as i32, self.h as i32);
        match self.facing.rem_euclid(4) {
            0 => CellRect::new(x - w + 1, z, x, z + h - 1),
            1 => CellRect::new(x - h + 1, z - w + 1, x, z),
            2 => CellRect::new(x, z - h + 1, x + w - 1, z),
            _ => CellRect::new(x, z, x + h - 1, z + w - 1),
        }
    }
}

#[derive(Clone, Debug)]
struct Cfg {
    id: u32,
    w: u32,
    h: u32,
    min_slope: f32,
    max_slope: f32,
    stage: u8,
    min_cap: u16,
    max_cap: u16,
    zones: Option<Vec<i32>>,
    wealths: Option<Vec<u32>>,
    purposes: Option<Vec<u32>>,
    styles: Option<Vec<u32>>,
    roads: Vec<bool>,
    primary: Option<Vec<u32>>,
}

#[derive(Clone, Debug)]
struct Bld {
    profile: BuildingProfile,
    groups: Option<Vec<u32>>,
    monopoly: bool,
    fields: Option<Vec<u32>>,
}

#[derive(Default)]
struct Grid {
    default: i64,
    cells: BTreeMap<(i32, i32), i64>,
}

impl Grid {
    fn get(&self, x: i32, z: i32) -> i64 {
        self.cells.get(&(x, z)).copied().unwrap_or(self.default)
    }
}

pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    sim: RefCell<Option<TractDeveloper>>,
    statics: RefCell<Option<SharedStatics>>,
    devs: RefCell<BTreeMap<u64, Dev>>,
    lots: RefCell<BTreeMap<u64, Lot>>,
    cfgs: RefCell<BTreeMap<u64, Cfg>>,
    buildings: RefCell<BTreeMap<u32, Bld>>,
    families: RefCell<BTreeMap<u32, Vec<u32>>>,
    grids: RefCell<BTreeMap<String, Grid>>,
    networks: RefCell<BTreeMap<(i32, i32), Vec<u32>>>,
    surrounding_rect: RefCell<Vec<u64>>,
    surrounding_lot: RefCell<BTreeMap<u64, Vec<u64>>>,
    filters: RefCell<BTreeMap<u64, (u32, u32)>>,
    next_token: Cell<u64>,
    city: Cell<(i32, i32)>,
    supply: Cell<i64>,
    used: Cell<i64>,
    cheats: Cell<[bool; 4]>,
    anchor: Cell<bool>,
    construction_sets_state: Cell<bool>,
    no_services: Cell<bool>,
    no_simulator: Cell<bool>,
    no_message_server: Cell<bool>,
    create_fails: Cell<bool>,
    /// The zone developer's file-local generator (zones.md §9), drawn by
    /// `CreateLotIfPossible` with facing −1 (R9).
    zone_rng: RefCell<RzRandom>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    outcomes: RefCell<Vec<Outcome>>,
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            me: RefCell::new(Weak::new()),
            events: EventLog::default(),
            sim: RefCell::new(None),
            statics: RefCell::new(None),
            devs: RefCell::new(BTreeMap::new()),
            lots: RefCell::new(BTreeMap::new()),
            cfgs: RefCell::new(BTreeMap::new()),
            buildings: RefCell::new(BTreeMap::new()),
            families: RefCell::new(BTreeMap::new()),
            grids: RefCell::new(BTreeMap::new()),
            networks: RefCell::new(BTreeMap::new()),
            surrounding_rect: RefCell::new(Vec::new()),
            surrounding_lot: RefCell::new(BTreeMap::new()),
            filters: RefCell::new(BTreeMap::new()),
            next_token: Cell::new(0x10000),
            city: Cell::new((64, 64)),
            supply: Cell::new(1_000_000_000),
            used: Cell::new(0),
            cheats: Cell::new([false; 4]),
            anchor: Cell::new(false),
            construction_sets_state: Cell::new(true),
            no_services: Cell::new(false),
            no_simulator: Cell::new(false),
            no_message_server: Cell::new(false),
            create_fails: Cell::new(false),
            zone_rng: RefCell::new(zone_generator(ZONE_SEED)),
            tuning: RefCell::new(Some(Rc::new(Tuning::shipped_exemplar()))),
            outcomes: RefCell::new(Vec::new()),
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        let mut g = w.grids.borrow_mut();
        for (name, d) in [
            ("zone", 1),
            ("land_value", 0),
            ("powered", 2),
            ("watered", 1),
            ("fire", 1),
            ("park", 1),
            ("road_access", 1),
        ] {
            g.insert(
                name.to_owned(),
                Grid {
                    default: d,
                    cells: BTreeMap::new(),
                },
            );
        }
        drop(g);
        w
    }

    fn rc(&self) -> R<Rc<World>> {
        self.me.borrow().upgrade().ok_or_else(|| "world gone".to_owned())
    }

    fn grid(&self, name: &str, x: i32, z: i32) -> i64 {
        self.grids.borrow().get(name).map_or(0, |g| g.get(x, z))
    }

    fn token(&self) -> u64 {
        let t = self.next_token.get();
        self.next_token.set(t + 1);
        t
    }

    fn with_sim<T>(&self, f: impl FnOnce(&mut TractDeveloper) -> R<T>) -> R<T> {
        // Take the simulator out so host callbacks can borrow the world.
        let mut s = self
            .sim
            .borrow_mut()
            .take()
            .ok_or("no tract developer (use tdev_create)")?;
        let r = f(&mut s);
        *self.sim.borrow_mut() = Some(s);
        r
    }

    fn dev(&self, d: u64) -> Option<Dev> {
        self.devs.borrow().get(&d).cloned()
    }

    fn lot<T: Default>(&self, l: u64, f: impl FnOnce(&Lot) -> T) -> T {
        self.lots.borrow().get(&l).map(f).unwrap_or_default()
    }

    fn lot_mut(&self, l: u64, f: impl FnOnce(&mut Lot)) {
        if let Some(lot) = self.lots.borrow_mut().get_mut(&l) {
            f(lot);
        }
    }

    fn cfg<T: Default>(&self, c: u64, f: impl FnOnce(&Cfg) -> T) -> T {
        self.cfgs.borrow().get(&c).map(f).unwrap_or_default()
    }

    fn bld<T: Default>(&self, b: u32, f: impl FnOnce(&Bld) -> T) -> T {
        self.buildings.borrow().get(&b).map(f).unwrap_or_default()
    }
}

/// The zone developer generator's seed at clock value 0 (zones.md §9).
const ZONE_SEED: u32 = 0x8C09_C68A;

fn zone_generator(seed: u32) -> RzRandom {
    let mut g = RzRandom::new(
        1,
        &FixedSeeds {
            wall_clock: 1,
            process_clock: 1,
        },
    );
    g.set_state(seed, seed, None);
    g
}

fn rect_json(r: CellRect) -> Json {
    Json::Array(vec![ji(r.x1), ji(r.z1), ji(r.x2), ji(r.z2)])
}

fn tok(t: u64) -> Json {
    ji(t as i64)
}

impl TractDeveloperHost for World {
    fn acquire_services(&self) -> bool {
        self.events.push("acquire_services", vec![]);
        !self.no_services.get()
    }
    fn add_agent(&self, level: u32, message: u32) -> bool {
        if self.no_simulator.get() {
            return false;
        }
        self.events
            .push("add_agent", vec![("level", ji(level)), ("message", ji(message))]);
        true
    }
    fn acquire_message_server(&self) -> bool {
        self.events.push("acquire_message_server", vec![]);
        !self.no_message_server.get()
    }
    fn release_message_server(&self) {
        self.events.push("release_message_server", vec![]);
    }
    fn remove_agent(&self) {
        self.events.push("remove_agent", vec![]);
    }
    fn release_services(&self) {
        self.events.push("release_services", vec![]);
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }

    fn dev_target_zone(&self, dev: u64) -> i32 {
        self.dev(dev).map_or(0, |d| d.target_zone)
    }
    fn dev_subtype(&self, dev: u64) -> u32 {
        self.dev(dev).map_or(0, |d| d.subtype)
    }
    fn dev_purpose(&self, dev: u64) -> u32 {
        self.dev(dev).map_or(0, |d| d.purpose)
    }
    fn dev_wealth(&self, dev: u64) -> u32 {
        self.dev(dev).map_or(0, |d| d.wealth)
    }
    fn dev_construction_info(&self, dev: u64) -> ConstructionProfile {
        self.dev(dev).map(|d| d.info).unwrap_or_default()
    }
    fn dev_desirability_tract_size(&self, dev: u64) -> i32 {
        self.dev(dev).map_or(0, |d| d.tract_size)
    }
    fn dev_desirability_shift(&self, dev: u64) -> u32 {
        self.dev(dev).map_or(0, |d| d.shift)
    }
    fn dev_desirability_value(&self, dev: u64, tx: i32, tz: i32) -> i16 {
        self.dev(dev)
            .and_then(|d| d.desirability.get(&(tx, tz)).copied())
            .unwrap_or(0)
    }
    fn dev_capacity_value(&self, dev: u64, tx: i32, tz: i32) -> u32 {
        self.dev(dev)
            .and_then(|d| d.capacity.get(&(tx, tz)).copied())
            .unwrap_or(0)
    }
    fn dev_source_demand(&self, dev: u64) -> Option<f32> {
        self.dev(dev).and_then(|d| d.demand)
    }
    fn dev_cap(&self, dev: u64, cap: Cap) -> u8 {
        let i = match cap {
            Cap::Power => 0,
            Cap::Water => 1,
            Cap::Fire => 2,
            Cap::Park => 3,
        };
        self.dev(dev).map_or(0, |d| d.caps[i])
    }
    fn dev_monopoly_use(&self, dev: u64) -> i32 {
        self.dev(dev).map_or(-1, |d| d.monopoly_use)
    }
    fn dev_style_percentages(&self, dev: u64) -> Vec<(u32, f32)> {
        self.dev(dev).map(|d| d.styles).unwrap_or_default()
    }

    fn building_profile(&self, building: u32) -> BuildingProfile {
        self.bld(building, |b| b.profile.clone())
    }
    fn building_types_by_family(&self, family: u32) -> Vec<u32> {
        self.families.borrow().get(&family).cloned().unwrap_or_default()
    }
    fn building_in_occupant_group(&self, building: u32, group: u32) -> bool {
        self.bld(building, |b| b.groups.as_ref().map_or(true, |g| g.contains(&group)))
    }
    fn building_is_monopoly(&self, building: u32) -> bool {
        self.bld(building, |b| b.monopoly)
    }
    fn building_field_ids(&self, building: u32) -> Option<Vec<u32>> {
        self.bld(building, |b| b.fields.clone())
    }
    fn developer_for(&self, purpose: u32, wealth: u32) -> Option<u64> {
        self.devs
            .borrow()
            .iter()
            .find(|(_, d)| d.purpose == purpose && d.wealth == wealth)
            .map(|(&k, _)| k)
    }
    fn claim_construction_resources(&self, amount: u16) -> bool {
        let ok = self.used.get() + i64::from(amount) <= self.supply.get();
        if ok {
            self.used.set(self.used.get() + i64::from(amount));
        }
        self.events
            .push("claim", vec![("amount", ji(amount)), ("ok", Json::Bool(ok))]);
        ok
    }
    fn release_construction_resources(&self, amount: u16) {
        self.used.set(self.used.get() - i64::from(amount));
        self.events.push("release", vec![("amount", ji(amount))]);
    }
    fn cheat(&self, cheat: Cheat) -> bool {
        let c = self.cheats.get();
        match cheat {
            Cheat::IgnorePlumbing => c[0],
            Cheat::IgnorePower => c[1],
            Cheat::IgnoreCaps => c[2],
            Cheat::DebugGrowth => c[3],
        }
    }

    fn needs_anchor_building(&self, rect: CellRect, purpose: u32) -> bool {
        self.events.push(
            "needs_anchor",
            vec![("rect", rect_json(rect)), ("purpose", ji(purpose))],
        );
        self.anchor.get()
    }
    fn start_construction(&self, lot: u64, cfg: u64, building: u32, flag: bool) -> bool {
        self.events.push(
            "start_construction",
            vec![
                ("lot", tok(lot)),
                ("cfg", tok(cfg)),
                ("building", ji(building)),
                ("flag", Json::Bool(flag)),
            ],
        );
        if self.construction_sets_state.get() {
            self.lot_mut(lot, |l| {
                l.state = 2;
                l.building = building;
            });
        }
        true
    }
    fn abandon_building(&self, lot: u64, reason: u32) -> bool {
        self.events
            .push("abandon", vec![("lot", tok(lot)), ("reason", ji(reason))]);
        self.lot_mut(lot, |l| l.state = 4);
        true
    }
    fn reoccupy_building(&self, lot: u64, dev_type: u32) -> bool {
        self.events
            .push("reoccupy", vec![("lot", tok(lot)), ("dev_type", ji(dev_type))]);
        self.lot_mut(lot, |l| l.state = 3);
        true
    }
    fn area_slope(&self, _rect: CellRect) -> f32 {
        0.0
    }

    fn get_lot(&self, x: i32, z: i32) -> Option<u64> {
        self.lots.borrow().iter().find_map(|(&k, l)| {
            let r = l.rect();
            (r.x1 <= x && x <= r.x2 && r.z1 <= z && z <= r.z2).then_some(k)
        })
    }
    fn lots_in_cell_rect(&self, rect: CellRect) -> Vec<u64> {
        self.lots
            .borrow()
            .iter()
            .filter(|(_, l)| {
                let r = l.rect();
                r.x1 <= rect.x2 && rect.x1 <= r.x2 && r.z1 <= rect.z2 && rect.z1 <= r.z2
            })
            .map(|(&k, _)| k)
            .collect()
    }
    fn lots_surrounding_lot(&self, lot: u64, flags: u32) -> Vec<u64> {
        if flags != 1 {
            return Vec::new();
        }
        self.surrounding_lot.borrow().get(&lot).cloned().unwrap_or_default()
    }
    fn lots_surrounding_rect(&self, _rect: CellRect, flags: u32) -> Vec<u64> {
        if flags != 1 {
            return Vec::new();
        }
        self.surrounding_rect.borrow().clone()
    }
    fn can_create_lot(&self, x: i32, z: i32, _w: i32, _h: i32) -> i32 {
        self.get_lot(x, z).map_or(7, |l| self.lot(l, |l| l.can_create))
    }
    fn create_lot_if_possible(&self, x: i32, z: i32, w: i32, h: i32, facing: i32) -> Option<u64> {
        // A free rectangle (code 8) with facing −1 goes through
        // CreateLotToFaceRoad with an empty list: one U(4) on the zone
        // developer's generator on entry (zones.md §7.7, §9 Z2c; R9). The
        // harness has no road model, so the lot faces the drawn value (the
        // "no side has a road" case).
        let mut facing_made = facing;
        if !self.create_fails.get() && facing == -1 {
            let r = self.zone_rng.borrow_mut().uniform_u32(4);
            if let Some(st) = self.statics.borrow().as_ref() {
                st.borrow_mut().record(DrawKind::R9, 4, f64::from(r));
            }
            self.events.push("zone_draw", vec![("value", ji(r))]);
            facing_made = r as i32;
        }
        if self.create_fails.get() {
            self.events.push(
                "create_lot",
                vec![
                    ("x", ji(x)),
                    ("z", ji(z)),
                    ("w", ji(w)),
                    ("h", ji(h)),
                    ("facing", ji(facing)),
                    ("lot", Json::Null),
                ],
            );
            return None;
        }
        let t = self.token();
        let f = if facing_made < 0 { 0 } else { facing_made.rem_euclid(4) };
        let (x2, z2) = (x + w - 1, z + h - 1);
        let (lx, lz, lw, lh) = match f {
            0 => (x2, z, w, h),
            1 => (x2, z2, h, w),
            2 => (x, z2, w, h),
            _ => (x, z, h, w),
        };
        let zone = self.grid("zone", x, z) as i32;
        self.lots.borrow_mut().insert(
            t,
            Lot {
                x: lx,
                z: lz,
                w: lw as u32,
                h: lh as u32,
                facing: f,
                zone,
                ..default_lot()
            },
        );
        self.events.push(
            "create_lot",
            vec![
                ("x", ji(x)),
                ("z", ji(z)),
                ("w", ji(w)),
                ("h", ji(h)),
                ("facing", ji(facing)),
                ("lot", tok(t)),
            ],
        );
        Some(t)
    }
    fn delete_lot(&self, lot: u64) -> bool {
        self.events.push("delete_lot", vec![("lot", tok(lot))]);
        self.lots.borrow_mut().remove(&lot).is_some()
    }

    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        self.lots
            .borrow()
            .get(&lot)
            .map_or(CellRect::new(0, 0, -1, -1), Lot::rect)
    }
    fn lot_slope(&self, lot: u64) -> f32 {
        self.lot(lot, |l| l.slope)
    }
    fn lot_location(&self, lot: u64) -> (i32, i32) {
        self.lot(lot, |l| (l.x, l.z))
    }
    fn lot_size(&self, lot: u64) -> (u32, u32) {
        self.lot(lot, |l| (l.w, l.h))
    }
    fn lot_facing(&self, lot: u64) -> i32 {
        self.lot(lot, |l| l.facing)
    }
    fn lot_set_facing(&self, lot: u64, facing: i32) {
        self.events
            .push("set_facing", vec![("lot", tok(lot)), ("facing", ji(facing))]);
        self.lot_mut(lot, |l| l.facing = facing);
    }
    fn lot_has_road_access(&self, lot: u64) -> bool {
        self.lot(lot, |l| l.road)
    }
    fn lot_has_organic_road_access(&self, lot: u64, _purpose: u32) -> bool {
        self.lot(lot, |l| l.organic)
    }
    fn lot_zone_type(&self, lot: u64) -> i32 {
        self.lot(lot, |l| l.zone)
    }
    fn lot_state(&self, lot: u64) -> u32 {
        self.lot(lot, |l| l.state)
    }
    fn lot_set_state(&self, lot: u64, state: u32, wealth: u32, reason: u32) {
        self.events.push(
            "set_state",
            vec![
                ("lot", tok(lot)),
                ("state", ji(state)),
                ("wealth", ji(wealth)),
                ("reason", ji(reason)),
            ],
        );
        self.lot_mut(lot, |l| l.state = state);
    }
    fn lot_is_livable(&self, lot: u64) -> bool {
        self.lot(lot, |l| l.livable)
    }
    fn lot_has_capacity(&self, lot: u64) -> bool {
        self.lot(lot, |l| l.has_capacity.unwrap_or(l.building != 0))
    }
    fn lot_days_since_state_change(&self, lot: u64) -> u32 {
        self.lot(lot, |l| l.days)
    }
    fn lot_occupant_wealth(&self, lot: u64) -> u32 {
        self.lot(lot, |l| l.wealth)
    }
    fn lot_is_historical(&self, lot: u64) -> bool {
        self.lot(lot, |l| l.historical)
    }
    fn lot_building_type(&self, lot: u64) -> u32 {
        self.lot(lot, |l| l.building)
    }
    fn lot_capacity(&self, lot: u64, _dev_type: u32, _flag: bool) -> u16 {
        self.lot(lot, |l| l.capacity)
    }
    fn lot_available_capacity(&self, lot: u64, _dev_type: u32, _flag: bool) -> u16 {
        self.lot(lot, |l| l.available.unwrap_or(l.capacity))
    }
    fn lot_set_population(&self, lot: u64, dev_type: u32, population: u16) {
        self.events.push(
            "set_population",
            vec![
                ("lot", tok(lot)),
                ("dev_type", ji(dev_type)),
                ("population", ji(population)),
            ],
        );
        self.lot_mut(lot, |l| {
            l.population.insert(dev_type, population);
        });
    }
    fn lot_configuration_stage(&self, lot: u64) -> Option<u8> {
        self.lot(lot, |l| l.cfg_stage)
    }

    fn cfg_size(&self, cfg: u64) -> (u32, u32) {
        self.cfg(cfg, |c| (c.w, c.h))
    }
    fn cfg_min_slope(&self, cfg: u64) -> f32 {
        self.cfg(cfg, |c| c.min_slope)
    }
    fn cfg_max_slope(&self, cfg: u64) -> f32 {
        self.cfg(cfg, |c| c.max_slope)
    }
    fn cfg_growth_stage(&self, cfg: u64) -> u8 {
        self.cfg(cfg, |c| c.stage)
    }
    fn cfg_min_capacity(&self, cfg: u64) -> u16 {
        self.cfg(cfg, |c| c.min_cap)
    }
    fn cfg_max_capacity(&self, cfg: u64) -> u16 {
        self.cfg(cfg, |c| c.max_cap)
    }
    fn cfg_compatible_zone(&self, cfg: u64, zone: i32) -> bool {
        self.cfg(cfg, |c| c.zones.as_ref().map_or(true, |v| v.contains(&zone)))
    }
    fn cfg_compatible_wealth(&self, cfg: u64, wealth: u32) -> bool {
        self.cfg(cfg, |c| c.wealths.as_ref().map_or(true, |v| v.contains(&wealth)))
    }
    fn cfg_compatible_purpose(&self, cfg: u64, purpose: u32) -> bool {
        self.cfg(cfg, |c| c.purposes.as_ref().map_or(true, |v| v.contains(&purpose)))
    }
    fn cfg_compatible_style(&self, cfg: u64, style: u32) -> bool {
        self.cfg(cfg, |c| c.styles.as_ref().map_or(true, |v| v.contains(&style)))
    }
    fn cfg_required_roads(&self, cfg: u64) -> Vec<bool> {
        self.cfg(cfg, |c| c.roads.clone())
    }
    fn cfg_primary_building_ids(&self, cfg: u64) -> Option<Vec<u32>> {
        self.cfg(cfg, |c| c.primary.clone())
    }

    fn get_lot_configuration(&self, id: u32) -> Option<u64> {
        self.cfgs.borrow().iter().find(|(_, c)| c.id == id).map(|(&k, _)| k)
    }
    fn register_configuration_filter(&self, wealth: u32, purpose: u32) -> u64 {
        let t = self.token();
        self.filters.borrow_mut().insert(t, (wealth, purpose));
        self.events.push(
            "register_filter",
            vec![("wealth", ji(wealth)), ("purpose", ji(purpose)), ("filter", tok(t))],
        );
        t
    }
    fn unregister_configuration_filter(&self, filter: u64) {
        self.filters.borrow_mut().remove(&filter);
        self.events.push("unregister_filter", vec![("filter", tok(filter))]);
    }
    fn configurations_by_filter(&self, filter: u64) -> Vec<u64> {
        let Some((w, p)) = self.filters.borrow().get(&filter).copied() else {
            return Vec::new();
        };
        let all: Vec<u64> = self.cfgs.borrow().keys().copied().collect();
        all.into_iter()
            .filter(|&c| configuration_filter_includes(self, c, w, p))
            .collect()
    }

    fn zone_at(&self, x: i32, z: i32) -> i32 {
        self.grid("zone", x, z) as i32
    }
    fn land_value(&self, x: i32, z: i32) -> u8 {
        self.grid("land_value", x, z) as u8
    }
    fn powered_value(&self, x: i32, z: i32) -> i32 {
        self.grid("powered", x, z) as i32
    }
    fn is_cell_watered(&self, x: i32, z: i32) -> bool {
        self.grid("watered", x, z) != 0
    }
    fn fire_coverage(&self, x: i32, z: i32) -> u8 {
        self.grid("fire", x, z) as u8
    }
    fn park_value(&self, x: i32, z: i32) -> i16 {
        self.grid("park", x, z) as i16
    }
    fn road_access_value(&self, x: i32, z: i32) -> i32 {
        self.grid("road_access", x, z) as i32
    }
    fn city_cell_counts(&self) -> (i32, i32) {
        self.city.get()
    }
    fn find_network_tile(&self, x: i32, z: i32) -> Option<u64> {
        self.networks
            .borrow()
            .contains_key(&(x, z))
            .then(|| ((x as u32 as u64) << 32) | u64::from(z as u32))
    }
    fn network_is_of_type(&self, tile: u64, network_type: u32) -> bool {
        let (x, z) = ((tile >> 32) as u32 as i32, tile as u32 as i32);
        self.networks
            .borrow()
            .get(&(x, z))
            .is_some_and(|v| v.contains(&network_type))
    }
    fn demolish_lot(&self, lot: u64) -> bool {
        self.events.push("demolish", vec![("lot", tok(lot))]);
        self.lot_mut(lot, |l| {
            l.building = 0;
            l.has_capacity = Some(false);
        });
        true
    }
    fn redisplay_terrain(&self, rect: CellRect) {
        self.events.push("redisplay", vec![("rect", rect_json(rect))]);
    }
}

fn default_lot() -> Lot {
    Lot {
        x: 0,
        z: 0,
        w: 1,
        h: 1,
        facing: 0,
        slope: 0.0,
        zone: 1,
        state: 1,
        livable: false,
        has_capacity: None,
        days: 0,
        wealth: 0,
        historical: false,
        building: 0,
        capacity: 0,
        available: None,
        road: true,
        organic: true,
        cfg_stage: None,
        can_create: 7,
        population: BTreeMap::new(),
    }
}

// ---- dispatch ----------------------------------------------------------------------------------

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "tract_developer", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("tdev_")?;
    let w = world(host);
    Some(match rest {
        "world" => set_world(&w, op),
        "developer" => developer(&w, op),
        "lot" => lot(&w, op),
        "cfg" => cfg(&w, op),
        "building" => building(&w, op),
        "family" => family(&w, op),
        "cells" => cells(&w, op),
        "networks" => networks(&w, op),
        "surrounding" => surrounding(&w, op),
        "tuning" => tuning(&w, op),
        "create" => create(&w, op),
        "rng" => rng(&w, op),
        "init" => w.with_sim(|s| check_expect(op, Json::Bool(s.init()))),
        "shutdown" => w.with_sim(|s| check_expect(op, Json::Bool(s.shutdown()))),
        "message" => w.with_sim(|s| {
            let m = u32_of(field(op, "msg")?, "msg")?;
            check_expect(op, Json::Bool(s.do_message(m)))
        }),
        "new_year" => w.with_sim(|s| {
            let n = opt(op, "count").map_or(Ok(1), |c| i64_of(c, "count"))?;
            for _ in 0..n {
                s.new_year();
            }
            Ok(())
        }),
        "set_active_styles" => w.with_sim(|s| {
            let v = u32_list(field(op, "styles")?, "styles")?;
            s.set_active_styles(&v);
            Ok(())
        }),
        "use_all_styles" => w.with_sim(|s| {
            s.use_all_styles_at_once(bool_of(field(op, "value")?, "value")?);
            Ok(())
        }),
        "set" => set_state(&w, op),
        "grow" => grow(&w, op),
        "decline" => decline(&w, op),
        "update_population" => update_population(&w, op),
        "save" => save(&w, op),
        "load" => load(&w, op),
        "expect" => expect(&w, op),
        "expect_lot" => expect_lot(&w, op),
        "expect_hist" => expect_hist(&w, op),
        "expect_draws" => expect_draws(&w, op),
        "expect_events" => w.events.expect_events(op),
        "expect_event_count" => w.events.expect_count(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }
        "eval" => eval(op),
        _ => Err(format!("unknown op '{name}'")),
    })
}

// ---- helpers -------------------------------------------------------------------------------------

fn i32_of(j: &Json, what: &str) -> R<i32> {
    Ok(i64_of(j, what)? as i32)
}

fn u32_list(j: &Json, what: &str) -> R<Vec<u32>> {
    arr(j, what)?.iter().map(|v| u32_of(v, what)).collect()
}

fn i32_list(j: &Json, what: &str) -> R<Vec<i32>> {
    arr(j, what)?.iter().map(|v| i32_of(v, what)).collect()
}

fn bool_list(j: &Json, what: &str) -> R<Vec<bool>> {
    arr(j, what)?.iter().map(|v| bool_of(v, what)).collect()
}

fn f32_num(j: &Json, what: &str) -> R<f32> {
    match j.as_str() {
        Some("nan") | Some("NaN") => Ok(f32::NAN),
        _ => f32_of(j, what),
    }
}

fn four(j: &Json, what: &str) -> R<[i32; 4]> {
    let v = i32_list(j, what)?;
    let mut out = [0; 4];
    for (o, x) in out.iter_mut().zip(v) {
        *o = x;
    }
    Ok(out)
}

fn pair(j: &Json, what: &str) -> R<(i32, i32)> {
    let a = arr(j, what)?;
    if a.len() != 2 {
        return Err(format!("'{what}' must be [x, z]"));
    }
    Ok((i32_of(&a[0], what)?, i32_of(&a[1], what)?))
}

fn token_of(op: &Json, key: &str) -> R<u64> {
    Ok(i64_of(field(op, key)?, key)? as u64)
}

// ---- world ops ---------------------------------------------------------------------------------

/// `tdev_world`: `city [x, z]`, `supply`, `used`, `cheats {ignore_plumbing,
/// ignore_power, ignore_caps, debug_growth}`, `anchor`,
/// `construction_sets_state`, `no_services`, `no_simulator`,
/// `no_message_server`, `create_fails` (`CreateLotIfPossible` gives no
/// lot; the event is still logged with `lot: null`), `zone_seed` (the
/// zone developer generator's LCG and multiplicative state; default
/// `0x8C09C68A`, its seed at clock value 0, zones.md §9).
fn set_world(w: &World, op: &Json) -> R<()> {
    if let Some(c) = opt(op, "city") {
        w.city.set(pair(c, "city")?);
    }
    if let Some(v) = opt(op, "supply") {
        w.supply.set(i64_of(v, "supply")?);
    }
    if let Some(v) = opt(op, "used") {
        w.used.set(i64_of(v, "used")?);
    }
    if let Some(c) = opt(op, "cheats") {
        let mut ch = w.cheats.get();
        for (k, v) in c.as_object().ok_or("'cheats' must be an object")? {
            let i = match k.as_str() {
                "ignore_plumbing" => 0,
                "ignore_power" => 1,
                "ignore_caps" => 2,
                "debug_growth" => 3,
                other => return Err(format!("unknown cheat '{other}'")),
            };
            ch[i] = bool_of(v, k)?;
        }
        w.cheats.set(ch);
    }
    for (k, cell) in [
        ("anchor", &w.anchor),
        ("construction_sets_state", &w.construction_sets_state),
        ("no_services", &w.no_services),
        ("no_simulator", &w.no_simulator),
        ("no_message_server", &w.no_message_server),
        ("create_fails", &w.create_fails),
    ] {
        if let Some(v) = opt(op, k) {
            cell.set(bool_of(v, k)?);
        }
    }
    if let Some(v) = opt(op, "zone_seed") {
        *w.zone_rng.borrow_mut() = zone_generator(u32_of(v, "zone_seed")?);
    }
    Ok(())
}

/// `tdev_developer`: `id`, `target_zone`, `subtype`, `purpose`, `wealth`,
/// `zone_code`, `land_value [min, max]`, `tract_size` (4), `shift` (2),
/// `desirability [[tx, tz, v]]`, `capacity [[tx, tz, v]]`, `demand`
/// (number, "nan" or null), `caps [power, water, fire, park]` (255 each),
/// `monopoly_use` (−1), `styles [[group, fraction]]`.
fn developer(w: &World, op: &Json) -> R<()> {
    let id = token_of(op, "id")?;
    let mut d = w.dev(id).unwrap_or(Dev {
        target_zone: 1,
        subtype: 0x1010,
        purpose: 1,
        wealth: 1,
        info: ConstructionProfile {
            zone: 1,
            wealth: 1,
            purpose: 1,
            ..Default::default()
        },
        tract_size: 4,
        shift: 2,
        desirability: BTreeMap::new(),
        capacity: BTreeMap::new(),
        demand: Some(1.0),
        caps: [255; 4],
        monopoly_use: -1,
        styles: Vec::new(),
    });
    for (k, v) in op.as_object().ok_or("op must be an object")? {
        match k.as_str() {
            "op" | "id" | "description" => {}
            "target_zone" => d.target_zone = i32_of(v, k)?,
            "subtype" => d.subtype = u32_of(v, k)?,
            "purpose" => {
                d.purpose = u32_of(v, k)?;
                d.info.purpose = d.purpose;
            }
            "wealth" => {
                d.wealth = u32_of(v, k)?;
                d.info.wealth = d.wealth;
            }
            "zone_code" => d.info.zone = i32_of(v, k)?,
            "land_value" => {
                let (a, b) = pair(v, k)?;
                d.info.land_value_min = a as u8;
                d.info.land_value_max = b as u8;
            }
            "tract_size" => d.tract_size = i32_of(v, k)?,
            "shift" => d.shift = u32_of(v, k)?,
            "desirability" | "capacity" => {
                for e in arr(v, k)? {
                    let t = i32_list(e, k)?;
                    if t.len() != 3 {
                        return Err(format!("'{k}' entries are [tx, tz, value]"));
                    }
                    if k == "desirability" {
                        d.desirability.insert((t[0], t[1]), t[2] as i16);
                    } else {
                        d.capacity.insert((t[0], t[1]), t[2] as u32);
                    }
                }
            }
            "demand" => {
                d.demand = match v {
                    Json::Null => None,
                    _ => Some(f32_num(v, k)?),
                }
            }
            "caps" => {
                let c = u32_list(v, k)?;
                for (o, x) in d.caps.iter_mut().zip(c) {
                    *o = x as u8;
                }
            }
            "monopoly_use" => d.monopoly_use = i32_of(v, k)?,
            "styles" => {
                d.styles = arr(v, k)?
                    .iter()
                    .map(|e| {
                        let a = arr(e, k)?;
                        Ok((u32_of(field_at(a, 0)?, k)?, f32_num(field_at(a, 1)?, k)?))
                    })
                    .collect::<R<Vec<_>>>()?
            }
            other => return Err(format!("unknown developer field '{other}'")),
        }
    }
    w.devs.borrow_mut().insert(id, d);
    Ok(())
}

fn field_at(a: &[Json], i: usize) -> R<&Json> {
    a.get(i).ok_or_else(|| format!("missing element {i}"))
}

/// `tdev_lot`: `id`, `at [x, z]` (location), `size [w, h]`, `facing`,
/// `slope`, `zone`, `state`, `livable`, `has_capacity`, `days`, `wealth`,
/// `historical`, `building`, `capacity`, `available`, `road`, `organic`,
/// `cfg_stage`, `can_create`; `remove: true` deletes it. Fields not given
/// keep their value (defaults: 1 × 1 facing 0, zone 1, state 1, roads,
/// can_create 7).
fn lot(w: &World, op: &Json) -> R<()> {
    let id = token_of(op, "id")?;
    if opt(op, "remove").and_then(boolean).unwrap_or(false) {
        w.lots.borrow_mut().remove(&id);
        return Ok(());
    }
    let mut l = w.lots.borrow().get(&id).cloned().unwrap_or_else(default_lot);
    for (k, v) in op.as_object().ok_or("op must be an object")? {
        match k.as_str() {
            "op" | "id" | "description" => {}
            "at" => (l.x, l.z) = pair(v, k)?,
            "size" => {
                let (a, b) = pair(v, k)?;
                (l.w, l.h) = (a as u32, b as u32);
            }
            "facing" => l.facing = i32_of(v, k)?,
            "slope" => l.slope = f32_num(v, k)?,
            "zone" => l.zone = i32_of(v, k)?,
            "state" => l.state = u32_of(v, k)?,
            "livable" => l.livable = bool_of(v, k)?,
            "has_capacity" => l.has_capacity = Some(bool_of(v, k)?),
            "days" => l.days = u32_of(v, k)?,
            "wealth" => l.wealth = u32_of(v, k)?,
            "historical" => l.historical = bool_of(v, k)?,
            "building" => l.building = u32_of(v, k)?,
            "capacity" => l.capacity = u32_of(v, k)? as u16,
            "available" => l.available = Some(u32_of(v, k)? as u16),
            "road" => l.road = bool_of(v, k)?,
            "organic" => l.organic = bool_of(v, k)?,
            "cfg_stage" => {
                l.cfg_stage = match v {
                    Json::Null => None,
                    _ => Some(u32_of(v, k)? as u8),
                }
            }
            "can_create" => l.can_create = i32_of(v, k)?,
            other => return Err(format!("unknown lot field '{other}'")),
        }
    }
    w.lots.borrow_mut().insert(id, l);
    Ok(())
}

/// `tdev_cfg`: `id` (token), `cfg_id` (the configuration ID), `size [w,
/// h]`, `slope [min, max]`, `stage`, `capacity [min, max]`, `zones`,
/// `wealths`, `purposes`, `styles` (absent: compatible with all), `roads`
/// (required-road bits), `primary` (family IDs; null: no primary object).
fn cfg(w: &World, op: &Json) -> R<()> {
    let id = token_of(op, "id")?;
    let mut c = w.cfgs.borrow().get(&id).cloned().unwrap_or(Cfg {
        id: 0,
        w: 1,
        h: 1,
        min_slope: -100.0,
        max_slope: 100.0,
        stage: 1,
        min_cap: 0,
        max_cap: 65535,
        zones: None,
        wealths: None,
        purposes: None,
        styles: None,
        roads: Vec::new(),
        primary: Some(Vec::new()),
    });
    for (k, v) in op.as_object().ok_or("op must be an object")? {
        match k.as_str() {
            "op" | "id" | "description" => {}
            "cfg_id" => c.id = u32_of(v, k)?,
            "size" => {
                let (a, b) = pair(v, k)?;
                (c.w, c.h) = (a as u32, b as u32);
            }
            "slope" => {
                let a = arr(v, k)?;
                c.min_slope = f32_num(field_at(a, 0)?, k)?;
                c.max_slope = f32_num(field_at(a, 1)?, k)?;
            }
            "stage" => c.stage = u32_of(v, k)? as u8,
            "capacity" => {
                let (a, b) = pair(v, k)?;
                (c.min_cap, c.max_cap) = (a as u16, b as u16);
            }
            "zones" => c.zones = Some(i32_list(v, k)?),
            "wealths" => c.wealths = Some(u32_list(v, k)?),
            "purposes" => c.purposes = Some(u32_list(v, k)?),
            "styles" => c.styles = Some(u32_list(v, k)?),
            "roads" => c.roads = bool_list(v, k)?,
            "primary" => {
                c.primary = match v {
                    Json::Null => None,
                    _ => Some(u32_list(v, k)?),
                }
            }
            other => return Err(format!("unknown cfg field '{other}'")),
        }
    }
    w.cfgs.borrow_mut().insert(id, c);
    Ok(())
}

/// A byte-sized profile field (`BuildingProfile.purpose` / `.wealth`).
fn byte_of(v: &Json, k: &str) -> R<u8> {
    u8::try_from(u32_of(v, k)?).map_err(|_| format!("'{k}' must fit a byte"))
}

/// `tdev_building`: `id` (type), `purpose`, `wealth`, `capacity`, `groups`
/// (absent: in every group), `monopoly`, `fields` (field IDs).
fn building(w: &World, op: &Json) -> R<()> {
    let id = u32_of(field(op, "id")?, "id")?;
    let mut b = w.buildings.borrow().get(&id).cloned().unwrap_or(Bld {
        profile: BuildingProfile::basic(1, 1, 1),
        groups: None,
        monopoly: false,
        fields: None,
    });
    for (k, v) in op.as_object().ok_or("op must be an object")? {
        match k.as_str() {
            "op" | "id" | "description" => {}
            "purpose" => b.profile.purpose = byte_of(v, k)?,
            "wealth" => b.profile.wealth = byte_of(v, k)?,
            "capacity" => b.profile.capacity = u32_of(v, k)? as u16,
            "groups" => b.groups = Some(u32_list(v, k)?),
            "monopoly" => b.monopoly = bool_of(v, k)?,
            "fields" => b.fields = Some(u32_list(v, k)?),
            other => return Err(format!("unknown building field '{other}'")),
        }
    }
    w.buildings.borrow_mut().insert(id, b);
    Ok(())
}

/// `tdev_family`: `id`, `members` (in BDS insertion order).
fn family(w: &World, op: &Json) -> R<()> {
    let id = u32_of(field(op, "id")?, "id")?;
    let m = u32_list(field(op, "members")?, "members")?;
    w.families.borrow_mut().insert(id, m);
    Ok(())
}

/// `tdev_cells`: `grid` (zone, land_value, powered, watered, fire, park,
/// road_access), `default`, `cells [[x, z, v]]`, `rects [[x1, z1, x2, z2,
/// v]]`.
fn cells(w: &World, op: &Json) -> R<()> {
    let name = str_of(field(op, "grid")?, "grid")?.to_owned();
    let mut grids = w.grids.borrow_mut();
    let g = grids.get_mut(&name).ok_or_else(|| format!("unknown grid '{name}'"))?;
    if let Some(d) = opt(op, "default") {
        g.default = i64_of(d, "default")?;
    }
    if let Some(c) = opt(op, "cells") {
        for e in arr(c, "cells")? {
            let t = i32_list(e, "cells")?;
            if t.len() != 3 {
                return Err("cells entries are [x, z, v]".into());
            }
            g.cells.insert((t[0], t[1]), i64::from(t[2]));
        }
    }
    if let Some(c) = opt(op, "rects") {
        for e in arr(c, "rects")? {
            let t = i32_list(e, "rects")?;
            if t.len() != 5 {
                return Err("rects entries are [x1, z1, x2, z2, v]".into());
            }
            for z in t[1]..=t[3] {
                for x in t[0]..=t[2] {
                    g.cells.insert((x, z), i64::from(t[4]));
                }
            }
        }
    }
    Ok(())
}

/// `tdev_networks`: `cells [[x, z, [types]]]` (network tiles).
fn networks(w: &World, op: &Json) -> R<()> {
    for e in arr(field(op, "cells")?, "cells")? {
        let a = arr(e, "cells")?;
        let x = i32_of(field_at(a, 0)?, "x")?;
        let z = i32_of(field_at(a, 1)?, "z")?;
        let t = u32_list(field_at(a, 2)?, "types")?;
        w.networks.borrow_mut().insert((x, z), t);
    }
    Ok(())
}

/// `tdev_surrounding`: `rect` (the lots `GetLotsSurroundingRect` appends
/// for flag 1; other flags append nothing), `lots {"lot": [lots]}` (the
/// same for `GetLotsSurroundingLot`).
fn surrounding(w: &World, op: &Json) -> R<()> {
    if let Some(r) = opt(op, "rect") {
        *w.surrounding_rect.borrow_mut() = arr(r, "rect")?
            .iter()
            .map(|v| i64_of(v, "rect").map(|t| t as u64))
            .collect::<R<_>>()?;
    }
    if let Some(m) = opt(op, "lots") {
        for (k, v) in m.as_object().ok_or("'lots' must be an object")? {
            let key = Json::Str(k.clone()).as_i64().ok_or("bad lot key")? as u64;
            let list = arr(v, "lots")?
                .iter()
                .map(|t| i64_of(t, "lots").map(|t| t as u64))
                .collect::<R<Vec<_>>>()?;
            w.surrounding_lot.borrow_mut().insert(key, list);
        }
    }
    Ok(())
}

/// `tdev_tuning`: `shipped` (default true: the §4.21 shipped exemplar),
/// `none` (no exemplar), `props` (merged over it), `remove` (IDs).
fn tuning(w: &World, op: &Json) -> R<()> {
    if opt(op, "none").and_then(boolean).unwrap_or(false) {
        *w.tuning.borrow_mut() = None;
        return Ok(());
    }
    let mut h = if opt(op, "shipped").and_then(boolean).unwrap_or(true) {
        Tuning::shipped_exemplar()
    } else {
        PropertyHolder::new()
    };
    if let Some(p) = opt(op, "props") {
        let extra = scalar::holder_of(p)?;
        for id in extra.ids() {
            if let Some(v) = extra.get(id) {
                h.add(id, v.clone());
            }
        }
    }
    if let Some(r) = opt(op, "remove") {
        for id in arr(r, "remove")? {
            h.remove(u32_of(id, "remove")?);
        }
    }
    *w.tuning.borrow_mut() = Some(Rc::new(h));
    Ok(())
}

/// `tdev_create`: `seed` (the file-local generator's seed exactly; default
/// `0x4C09C113`, the original's seed at clock value 0, i.e. `0 XOR
/// RNG_SEED_XOR`) or `session` (`TractDeveloperStatics::from_session`,
/// foundations §6.3), `crt_seed` (default 1),
/// `trace` (default true), `histogram` (default true), `shared` (reuse the
/// statics of the previous instance).
fn create(w: &World, op: &Json) -> R<()> {
    let seed = opt(op, "seed").map_or(Ok(RNG_SEED_XOR), |v| u32_of(v, "seed"))?;
    let seeds = FixedSeeds {
        wall_clock: 1,
        process_clock: 1,
    };
    let shared = opt(op, "shared").and_then(boolean).unwrap_or(false);
    let statics = match (shared, w.statics.borrow().clone()) {
        (true, Some(s)) => s,
        _ => match opt(op, "session") {
            Some(v) => TractDeveloperStatics::from_session(u32_of(v, "session")?, &seeds),
            None => TractDeveloperStatics::new(seed, &seeds),
        },
    };
    if let Some(c) = opt(op, "crt_seed") {
        statics.borrow_mut().crt = CrtRand::new(u32_of(c, "crt_seed")?);
    }
    if opt(op, "trace").and_then(boolean).unwrap_or(true) {
        statics.borrow_mut().trace = Some(Vec::new());
    }
    let mut s = TractDeveloper::new(w.rc()? as Rc<dyn TractDeveloperHost>, statics.clone());
    s.set_histogram_enabled(opt(op, "histogram").and_then(boolean).unwrap_or(true));
    *w.statics.borrow_mut() = Some(statics);
    *w.sim.borrow_mut() = Some(s);
    Ok(())
}

/// `tdev_rng`: `seed` (`Seed(s)` of the file-local generator) or `lcg` and
/// `mul`; `crt` (the `rand()` state).
fn rng(w: &World, op: &Json) -> R<()> {
    let st = w.statics.borrow().clone().ok_or("no statics (use tdev_create)")?;
    let mut s = st.borrow_mut();
    if let Some(v) = opt(op, "seed") {
        let seed = u32_of(v, "seed")?;
        s.rng = RzRandom::new(
            seed,
            &FixedSeeds {
                wall_clock: 1,
                process_clock: 1,
            },
        );
    }
    if opt(op, "lcg").is_some() || opt(op, "mul").is_some() {
        let lcg = opt(op, "lcg").map_or(Ok(s.rng.lcg_state()), |v| u32_of(v, "lcg"))?;
        let mul = opt(op, "mul").map_or(Ok(s.rng.mul_state()), |v| u32_of(v, "mul"))?;
        s.rng.set_state(lcg, mul, None);
    }
    if let Some(v) = opt(op, "crt") {
        s.crt = CrtRand::new(u32_of(v, "crt")?);
    }
    Ok(())
}

/// `tdev_set`: `style_index`, `years_since`, `years_between`,
/// `single_style`, `styles` (assigned directly, not through
/// `SetActiveStyles`).
fn set_state(w: &World, op: &Json) -> R<()> {
    w.with_sim(|s| {
        if let Some(v) = opt(op, "styles") {
            s.assign_active_styles(u32_list(v, "styles")?);
        }
        if let Some(v) = opt(op, "style_index") {
            s.set_current_style_index(i32_of(v, "style_index")?);
        }
        if let Some(v) = opt(op, "years_since") {
            s.set_years_since_style_change(i32_of(v, "years_since")?);
        }
        if let Some(v) = opt(op, "years_between") {
            s.set_years_between_style_changes(i32_of(v, "years_between")?);
        }
        if let Some(v) = opt(op, "single_style") {
            s.use_all_styles_at_once(!bool_of(v, "single_style")?);
        }
        Ok(())
    })
}

// ---- calls -------------------------------------------------------------------------------------

fn profile_of(op: &Json) -> R<GrowthProfile> {
    let (x, z) = pair(field(op, "tract")?, "tract")?;
    Ok(GrowthProfile {
        developer: token_of(op, "dev")?,
        x,
        z,
        stage: opt(op, "stage").map_or(Ok(0), |v| i32_of(v, "stage"))?,
    })
}

/// `tdev_grow`: `dev`, `tract [x, z]`, `stage`; `append` (keep the
/// outcomes list of the previous call); checks `expect` (built),
/// `outcomes` (the whole list), `out_flag`.
fn grow(w: &World, op: &Json) -> R<()> {
    let p = profile_of(op)?;
    let mut list = if opt(op, "append").and_then(boolean).unwrap_or(false) {
        std::mem::take(&mut *w.outcomes.borrow_mut())
    } else {
        Vec::new()
    };
    let mut flag = opt(op, "out_flag_in").and_then(boolean).unwrap_or(false);
    let built = w.with_sim(|s| Ok(s.grow(&p, &mut list, &mut flag)))?;
    let codes: Vec<Json> = list.iter().map(|o| ji(o.code())).collect();
    *w.outcomes.borrow_mut() = list;
    check_expect(op, ji(built))?;
    if let Some(want) = opt(op, "outcomes") {
        let got = Json::Array(codes);
        if !matches(&got, want, false) {
            return Err(format!("outcomes {}, expected {}", show(&got), show(want)));
        }
    }
    if let Some(want) = opt(op, "out_flag") {
        if bool_of(want, "out_flag")? != flag {
            return Err(format!("out_flag {flag}, expected {}", show(want)));
        }
    }
    Ok(())
}

/// `tdev_decline`: `dev`, `tract`; `expect` (the sum).
fn decline(w: &World, op: &Json) -> R<()> {
    let p = profile_of(op)?;
    let sum = w.with_sim(|s| Ok(s.decline(&p)))?;
    check_expect(op, ji(sum))
}

/// `tdev_update_population`: `dev`, `tract`.
fn update_population(w: &World, op: &Json) -> R<()> {
    let p = profile_of(op)?;
    w.with_sim(|s| {
        s.update_population(&p);
        Ok(())
    })
}

/// `tdev_save`: `expect` (hex bytes of the record), `ok`.
fn save(w: &World, op: &Json) -> R<()> {
    w.with_sim(|s| {
        let mut b = ByteWriter::new();
        let ok = s.write(&mut b);
        if let Some(v) = opt(op, "ok") {
            if bool_of(v, "ok")? != ok {
                return Err(format!("write returned {ok}"));
            }
        }
        check_expect(op, Json::Str(hex_encode(b.bytes())))
    })
}

/// `tdev_load`: `hex`; `expect` (the `Read` result).
fn load(w: &World, op: &Json) -> R<()> {
    let bytes = hex_decode(str_of(field(op, "hex")?, "hex")?)?;
    w.with_sim(|s| {
        let ok = s.read(&mut ByteReader::new(&bytes));
        check_expect(op, Json::Bool(ok))
    })
}

// ---- checks ------------------------------------------------------------------------------------

/// `tdev_expect`: any of `style_index`, `years_since`, `years_between`,
/// `single_style`, `all_styles`, `styles`, `current_style`, `desired`
/// (`[desired, desiredMax]` of the last grow), `chosen_cfg`,
/// `chosen_building`, `lcg`, `mul`, `crt`, `initialised`, `configured`,
/// `filters` (count), `tuning {field: value}`, `used` (resources), `lots`
/// (tokens of the lots that exist).
fn expect(w: &World, op: &Json) -> R<()> {
    let statics = w.statics.borrow().clone();
    w.with_sim(|s| {
        let mut got: Vec<(String, Json)> = Vec::new();
        let mut put = |k: &str, v: Json| got.push((k.to_owned(), v));
        put("style_index", ji(s.current_style_index()));
        put("years_since", ji(s.years_since_style_change()));
        put("years_between", ji(s.years_between_style_changes()));
        put("single_style", Json::Bool(!s.is_using_all_styles_at_once()));
        put("all_styles", Json::Bool(s.is_using_all_styles_at_once()));
        put(
            "styles",
            Json::Array(s.active_styles().iter().map(|&v| ji(v)).collect()),
        );
        put("current_style", ji(s.current_style()));
        let (d, dm) = s.desired();
        put("desired", Json::Array(vec![ji(d), ji(dm)]));
        let (cfg, b) = s.chosen();
        put("chosen_cfg", cfg.map_or(Json::Null, tok));
        put("chosen_building", ji(b));
        if let Some(st) = &statics {
            let st = st.borrow();
            put("lcg", ji(st.rng.lcg_state()));
            put("mul", ji(st.rng.mul_state()));
            put("crt", ji(st.crt.state()));
        }
        put("initialised", Json::Bool(s.is_initialised()));
        put("configured", Json::Bool(s.is_configured()));
        put("filters", ji(s.filters().len() as i64));
        let t = s.tuning();
        put(
            "tuning",
            Json::Object(vec![
                ("overbuild".into(), Json::Float(f64::from(t.overbuild))),
                (
                    "reoccupy".into(),
                    Json::Array(t.reoccupy.iter().map(|&v| Json::Float(f64::from(v))).collect()),
                ),
                (
                    "redevelop_size_threshold".into(),
                    Json::Float(f64::from(t.redevelop_size_threshold)),
                ),
                (
                    "demolition".into(),
                    Json::Array(t.demolition.iter().map(|&v| Json::Float(f64::from(v))).collect()),
                ),
                (
                    "new_lot".into(),
                    Json::Array(t.new_lot.iter().map(|&v| Json::Float(f64::from(v))).collect()),
                ),
                (
                    "occupancy_variation".into(),
                    Json::Float(f64::from(t.occupancy_variation)),
                ),
                (
                    "min_vacancy".into(),
                    Json::Array(t.min_vacancy.iter().map(|&v| Json::Float(f64::from(v))).collect()),
                ),
                ("kick_out".into(), Json::Bool(t.kick_out)),
                (
                    "available_styles".into(),
                    Json::Array(t.available_styles.iter().map(|&v| ji(v)).collect()),
                ),
            ]),
        );
        put("used", ji(w.used.get()));
        put("lots", Json::Array(w.lots.borrow().keys().map(|&k| tok(k)).collect()));
        let want = op.as_object().ok_or("op must be an object")?;
        let mut errors = Vec::new();
        for (k, v) in want {
            if k == "op" || k == "description" {
                continue;
            }
            match got.iter().find(|(g, _)| g == k) {
                None => errors.push(format!("unknown field '{k}'")),
                Some((_, g)) => {
                    let partial = k == "tuning";
                    if !matches(g, v, partial) {
                        errors.push(format!("{k}: got {}, expected {}", show(g), show(v)));
                    }
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    })
}

/// `tdev_expect_lot`: `lot`; any of `exists`, `state`, `facing`,
/// `building`, `population {devType: n}`, `rect`.
fn expect_lot(w: &World, op: &Json) -> R<()> {
    let id = token_of(op, "lot")?;
    let lots = w.lots.borrow();
    let l = lots.get(&id);
    if let Some(e) = opt(op, "exists") {
        if bool_of(e, "exists")? != l.is_some() {
            return Err(format!("lot {id} exists: {}", l.is_some()));
        }
    }
    let Some(l) = l else {
        return Ok(());
    };
    let mut got = vec![
        ("state".to_owned(), ji(l.state)),
        ("facing".to_owned(), ji(l.facing)),
        ("building".to_owned(), ji(l.building)),
        ("rect".to_owned(), rect_json(l.rect())),
        (
            "population".to_owned(),
            Json::Object(l.population.iter().map(|(k, v)| (format!("{k}"), ji(*v))).collect()),
        ),
    ];
    got.push(("exists".to_owned(), Json::Bool(true)));
    let mut want = Vec::new();
    for (k, v) in op.as_object().ok_or("op must be an object")? {
        if !matches!(k.as_str(), "op" | "lot" | "description") {
            want.push((k.clone(), v.clone()));
        }
    }
    scalar::check_state(&got, &Json::Object(want))
}

/// `tdev_expect_hist`: `row`, `counts {code: n}` (every other counter of
/// the row must be 0).
fn expect_hist(w: &World, op: &Json) -> R<()> {
    let row = u32_of(field(op, "row")?, "row")? as usize;
    let mut want = [0u32; 28];
    for (k, v) in field(op, "counts")?.as_object().ok_or("'counts' must be an object")? {
        let c: usize = k.parse().map_err(|_| format!("bad code '{k}'"))?;
        *want.get_mut(c).ok_or("code out of range")? = u32_of(v, "count")?;
    }
    w.with_sim(|s| {
        let got = s.histogram().get(row).copied().ok_or("row out of range")?;
        if got != want {
            let nz = |a: &[u32; 28]| {
                a.iter()
                    .enumerate()
                    .filter(|(_, &n)| n != 0)
                    .map(|(i, n)| format!("{i}: {n}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            return Err(format!("row {row}: got {{{}}}, expected {{{}}}", nz(&got), nz(&want)));
        }
        Ok(())
    })
}

/// `tdev_expect_draws`: the draws traced since the last check, one entry
/// each: `"R1"` (kind only) or `["R1", arg, value]` (value compared
/// exactly); `keep: true` does not clear the trace.
fn expect_draws(w: &World, op: &Json) -> R<()> {
    let st = w.statics.borrow().clone().ok_or("no statics (use tdev_create)")?;
    let mut st = st.borrow_mut();
    let trace = st.trace.get_or_insert_with(Vec::new);
    let got = trace.clone();
    if !opt(op, "keep").and_then(boolean).unwrap_or(false) {
        trace.clear();
    }
    let want = arr(field(op, "draws")?, "draws")?;
    let show_got = || {
        got.iter()
            .map(|d| format!("[{}, {}, {}]", d.kind.name(), d.arg, d.value))
            .collect::<Vec<_>>()
            .join(", ")
    };
    if got.len() != want.len() {
        return Err(format!("{} draws, expected {}: {}", got.len(), want.len(), show_got()));
    }
    for (i, (g, wnt)) in got.iter().zip(want).enumerate() {
        let ok = match wnt {
            Json::Str(k) => g.kind.name() == k,
            Json::Array(a) => {
                let k = str_of(field_at(a, 0)?, "kind")?;
                let n = u32_of(field_at(a, 1)?, "arg")?;
                let v = field_at(a, 2)?.as_f64().ok_or("draw value must be a number")?;
                g.kind.name() == k && g.arg == n && g.value == v
            }
            _ => return Err("draw entries are \"Rn\" or [\"Rn\", arg, value]".into()),
        };
        if !ok {
            return Err(format!("draw {i} differs: got {}", show_got()));
        }
    }
    Ok(())
}

// ---- pure functions ----------------------------------------------------------------------------

fn bits(v: f64) -> Json {
    ji((v as f32).to_bits())
}

fn candidate_of(j: &Json) -> R<Candidate> {
    let mut c = Candidate::default();
    for (k, v) in j.as_object().ok_or("candidate must be an object")? {
        match k.as_str() {
            "kind" => c.kind = u32_of(v, k)? as u8,
            "existing" => c.existing = i32_of(v, k)?,
            "abandoned" => c.abandoned = i32_of(v, k)?,
            "n" => c.n = i32_of(v, k)?,
            "facing" => c.facing = i32_of(v, k)?,
            "lots" => {
                let n = u32_of(v, k)?;
                c.lot_set = (0..u64::from(n)).collect::<BTreeSet<u64>>();
            }
            "street" => c.street = four(v, k)?,
            "road" => c.road = four(v, k)?,
            "avenue" => c.avenue = four(v, k)?,
            "rail" => c.rail = four(v, k)?,
            other => return Err(format!("unknown candidate field '{other}'")),
        }
    }
    Ok(c)
}

/// `tdev_eval`: `fn`, `args`, `expect`. Functions: `outcome_codes` (the
/// 28 codes in order), `best [codes]`, `desired [demand, overbuild]`,
/// `capacity_window [candidate, desiredMax, built]`, `rect [x, z, w, h,
/// facing]`, `corner [bits, candidate]`, `roads_check [bits, candidate,
/// requireRoads]` (21 or `[27, corner]`), `style_points [fraction]`,
/// `style_group [[[key, fraction]], r]`, `score [candidate, zone, tuning
/// {demolition, new_lot}, seed]` (`{score, bits, mul}`), `hash_order
/// [values]`, `stl_sort [scores]` (the sorted indices), `query_interface
/// [iid]`, `zone_family [zone]`.
fn eval(op: &Json) -> R<()> {
    let f = str_of(field(op, "fn")?, "fn")?;
    let got = match f {
        "outcome_codes" => Json::Array(Outcome::ALL.iter().map(|o| ji(o.code())).collect()),
        "best" => {
            let v = u32_list(arg(op, 0)?, "codes")?;
            let os: Vec<Outcome> = v.iter().filter_map(|&c| Outcome::from_code(c)).collect();
            ji(os.into_iter().max().map_or(0, Outcome::code))
        }
        "desired" => {
            let d = match arg(op, 0)? {
                Json::Null => None,
                v => Some(f32_num(v, "demand")?),
            };
            let (a, b) = desired_of(d, f32_num(arg(op, 1)?, "overbuild")?);
            Json::Array(vec![ji(a), ji(b)])
        }
        "capacity_window" => {
            let c = candidate_of(arg(op, 0)?)?;
            let (lo, hi) = capacity_window(&c, i32_of(arg(op, 1)?, "desiredMax")?, i32_of(arg(op, 2)?, "built")?);
            Json::Array(vec![ji(lo), ji(hi)])
        }
        "rect" => {
            let a: Vec<i32> = (0..5).map(|i| i32_of(arg(op, i)?, "rect")).collect::<R<_>>()?;
            rect_json(candidate_rect(a[0], a[1], a[2], a[3], a[4]))
        }
        "corner" => Json::Bool(corner_ok(&bool_list(arg(op, 0)?, "bits")?, &candidate_of(arg(op, 1)?)?)),
        "roads_check" => {
            let r = roads_check(
                &bool_list(arg(op, 0)?, "bits")?,
                &candidate_of(arg(op, 1)?)?,
                bool_of(arg(op, 2)?, "requireRoads")?,
            );
            match r {
                None => ji(21),
                Some(c) => Json::Array(vec![ji(27), Json::Bool(c)]),
            }
        }
        "style_points" => ji(style_points(f32_num(arg(op, 0)?, "fraction")?)),
        "style_group" => {
            let pcts = arr(arg(op, 0)?, "pcts")?
                .iter()
                .map(|e| {
                    let a = arr(e, "pct")?;
                    Ok((u32_of(field_at(a, 0)?, "key")?, f32_num(field_at(a, 1)?, "fraction")?))
                })
                .collect::<R<Vec<_>>>()?;
            ji(style_group(&pcts, u32_of(arg(op, 1)?, "r")?))
        }
        "score" => {
            let c = candidate_of(arg(op, 0)?)?;
            let zone = i32_of(arg(op, 1)?, "zone")?;
            let mut t = Tuning::code_defaults();
            if let Some(tj) = op.get("args").and_then(Json::as_array).and_then(|a| a.get(2)) {
                if let Some(d) = opt(tj, "demolition") {
                    let a = arr(d, "demolition")?;
                    t.demolition = [f32_num(field_at(a, 0)?, "d")?, f32_num(field_at(a, 1)?, "d")?];
                }
                if let Some(d) = opt(tj, "new_lot") {
                    let a = arr(d, "new_lot")?;
                    t.new_lot = [f32_num(field_at(a, 0)?, "n")?, f32_num(field_at(a, 1)?, "n")?];
                }
            }
            let seed = match op.get("args").and_then(Json::as_array).and_then(|a| a.get(3)) {
                Some(s) => u32_of(s, "seed")?,
                None => 1,
            };
            let mut g = RzRandom::new(
                seed,
                &FixedSeeds {
                    wall_clock: 1,
                    process_clock: 1,
                },
            );
            let mut draws = 0;
            let s = score_candidate(&c, zone, &t, &mut || {
                draws += 1;
                g.uniform_f64()
            });
            Json::Object(vec![
                ("score".into(), Json::Float(s)),
                ("bits".into(), bits(s)),
                ("draws".into(), ji(draws)),
                ("mul".into(), ji(g.mul_state())),
            ])
        }
        "hash_order" => {
            let mut h = StlHashSet::with_hint(100);
            for v in u32_list(arg(op, 0)?, "values")? {
                h.insert(v);
            }
            Json::Array(h.iter_order().into_iter().map(ji).collect())
        }
        "stl_sort" => {
            let scores: Vec<f64> = arr(arg(op, 0)?, "scores")?
                .iter()
                .map(|v| f32_num(v, "score").map(f64::from))
                .collect::<R<_>>()?;
            let mut idx: Vec<usize> = (0..scores.len()).collect();
            stl_sort(&mut idx, &mut |&a: &usize, &b: &usize| scores[a] < scores[b]);
            Json::Array(idx.into_iter().map(|i| ji(i as i64)).collect())
        }
        "query_interface" => Json::Bool(answers_iid(u32_of(arg(op, 0)?, "iid")?)),
        "zone_family" => ji(zone_family(i32_of(arg(op, 0)?, "zone")?)),
        other => return Err(format!("unknown tdev_eval fn '{other}'")),
    };
    check_expect(op, got)
}
