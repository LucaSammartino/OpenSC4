// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops `zone_…` for `fc4_core::zones` (`docs/specs/zones.md`): a
//! scripted world (a zone grid and a maximum-capacity grid the two classes
//! create through it, network runs with their edge connectivity, the stub
//! street tool of tests 19–29, lots, occupants, growth developers, the BDS,
//! terrain, budget, demolition and the traffic-simulator insertion point)
//! implementing [`ZonesHost`], an event log of lot creations and `Place`
//! calls, the draw trace of the shared statics, and ops that drive the zone
//! manager and the zone developer. See README.md ("Zones ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::{Rc, Weak};

use fc4_core::development::BuildingProfile;
use fc4_core::grid::{CityDims, SimGrid, TractGrid};
use fc4_core::property::PropertyHolder;
use fc4_core::rng::{derive_seed, FixedSeeds, RzRandom};
use fc4_core::stream::RefValue;
use fc4_core::zones::{
    block_depth, depth_rule, middle_row, shipped_tuning, zone_density, CellRegion, CellRect, Component, Draw,
    GridProvider, GrowthItem, InsertionPoint, LotSize, PlaceZoneResult, Position, Service, SharedZoneStatics,
    ZoneDeveloper, ZoneDeveloperStatics, ZoneManager, ZonesHost, ZonesMessage, IID_LOT, LANDFILL_TYPE,
    NETWORK_OCCUPANT_TYPE, RNG_SEED_XOR,
};

use crate::json::Json;
use crate::runner::{boolean, field, HostRef};
use crate::scalar::{self, arg, arr, bool_of, check_expect, f32_of, i64_of, ji, matches, opt, show, str_of, u32_of, EventLog, R};

// ---- world -------------------------------------------------------------------------------------

/// One network cell: its kind (network type) and the four network edges it
/// connects (0 = −x, 1 = −z, 2 = +x, 3 = +z).
#[derive(Clone, Copy, Debug)]
struct NetCell {
    kind: u32,
    edges: [bool; 4],
}

/// A lot the core creates through `new_lot`. The core owns the cell → lot map;
/// the world keeps only the fields the lot interface exposes.
#[derive(Clone, Copy, Debug)]
struct Lot {
    x: i32,
    z: i32,
    w: i32,
    h: i32,
    facing: i32,
    zone: i32,
    state: u32,
    flags: u32,
    altitude: f32,
    locked: bool,
    cfg: Option<u64>,
    building: Option<u64>,
    business_unit: bool,
    refs: u32,
}

impl Lot {
    fn rect(&self) -> CellRect {
        CellRect::new(self.x, self.z, self.x + self.w - 1, self.z + self.h - 1)
    }
}

/// A lot configuration (§7.2.1).
#[derive(Clone, Copy, Debug)]
struct Cfg {
    w: u32,
    h: u32,
    water: bool,
    zone_ok: bool,
    primary_families: Option<[u32; 4]>,
    n_families: usize,
}

/// A growth developer (§7.2.1).
#[derive(Clone, Debug)]
struct Dev {
    purpose: u32,
    wealth: u32,
    desirability: BTreeMap<(i32, i32), i16>,
    demand: Option<f32>,
    shift: u32,
    growth: Option<Vec<GrowthItem>>,
    desir_value: BTreeMap<(i32, i32), i16>,
}

/// An occupant (buildings, obstacles, landfill).
#[derive(Clone, Copy, Debug)]
struct Occ {
    kind: u32,
    pos: Position,
    can_remove: bool,
}

pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    manager: RefCell<Option<Rc<ZoneManager>>>,
    developer: RefCell<Option<Rc<ZoneDeveloper>>>,
    statics: RefCell<Option<SharedZoneStatics>>,
    // Grids the two classes create through the host.
    zone_grid: RefCell<Option<Rc<SimGrid<i8>>>>,
    maxcap_grid: RefCell<Option<Rc<SimGrid<u16>>>>,
    city: Cell<(i32, i32)>,
    // World model.
    nets: RefCell<BTreeMap<(i32, i32), NetCell>>,
    lots: RefCell<BTreeMap<u64, Lot>>,
    cfgs: RefCell<BTreeMap<u64, Cfg>>,
    devs: RefCell<BTreeMap<u64, Dev>>,
    occupants: RefCell<BTreeMap<u64, Occ>>,
    cell_occupants: RefCell<BTreeMap<(i32, i32), Vec<u64>>>,
    families: RefCell<BTreeMap<u32, Vec<u32>>>,
    buildings: RefCell<BTreeMap<u32, BuildingProfile>>,
    water: RefCell<BTreeSet<(i32, i32)>>,
    terrain: RefCell<BTreeMap<(i32, i32), f32>>,
    next_token: Cell<u64>,
    // Services and flags.
    funds: Cell<i64>,
    services_ok: Cell<bool>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    insertion: RefCell<Option<InsertionPoint>>,
    // Posted messages and host calls.
    deferred: RefCell<Vec<bool>>,
    zoning_changes: RefCell<Vec<(CellRegion, i32, bool)>>,
    demolished: RefCell<Vec<(bool, Vec<u64>, u32)>>,
    // Street stub control: whether `street_place` fails.
    street_fail_cells: RefCell<BTreeSet<(i32, i32)>>,
    street_place_count: Cell<u32>,
    /// The interleaved log of lot creations and Place calls (§15 tests
    /// 19–29: "the ordered log of lot creations and Place calls").
    parcel_log: RefCell<Vec<Json>>,
}

const SEEDS: FixedSeeds = FixedSeeds {
    wall_clock: 0,
    process_clock: 0,
};

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            me: RefCell::new(Weak::new()),
            events: EventLog::default(),
            manager: RefCell::new(None),
            developer: RefCell::new(None),
            statics: RefCell::new(None),
            zone_grid: RefCell::new(None),
            maxcap_grid: RefCell::new(None),
            city: Cell::new((64, 64)),
            nets: RefCell::new(BTreeMap::new()),
            lots: RefCell::new(BTreeMap::new()),
            cfgs: RefCell::new(BTreeMap::new()),
            devs: RefCell::new(BTreeMap::new()),
            occupants: RefCell::new(BTreeMap::new()),
            cell_occupants: RefCell::new(BTreeMap::new()),
            families: RefCell::new(BTreeMap::new()),
            buildings: RefCell::new(BTreeMap::new()),
            water: RefCell::new(BTreeSet::new()),
            terrain: RefCell::new(BTreeMap::new()),
            next_token: Cell::new(0x1000),
            funds: Cell::new(1_000_000_000),
            services_ok: Cell::new(true),
            tuning: RefCell::new(Some(Rc::new(shipped_tuning()))),
            insertion: RefCell::new(None),
            deferred: RefCell::new(Vec::new()),
            zoning_changes: RefCell::new(Vec::new()),
            demolished: RefCell::new(Vec::new()),
            street_fail_cells: RefCell::new(BTreeSet::new()),
            street_place_count: Cell::new(0),
            parcel_log: RefCell::new(Vec::new()),
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn rc(&self) -> R<Rc<World>> {
        self.me.borrow().upgrade().ok_or_else(|| "world gone".to_owned())
    }

    fn token(&self) -> u64 {
        let t = self.next_token.get();
        self.next_token.set(t + 1);
        t
    }

    fn mgr(&self) -> R<Rc<ZoneManager>> {
        self.manager.borrow().clone().ok_or_else(|| "no zone manager (use zone_create)".to_owned())
    }

    fn dev(&self) -> R<Rc<ZoneDeveloper>> {
        self.developer.borrow().clone().ok_or_else(|| "no zone developer (use zone_create)".to_owned())
    }

    fn lot<T: Default>(&self, l: u64, f: impl FnOnce(&Lot) -> T) -> T {
        self.lots.borrow().get(&l).map(f).unwrap_or_default()
    }

    /// Appends one lot creation to the interleaved parcel log, as `(x1, z1)–
    /// (x2, z2) fN` (§15 tests 19–29).
    fn log_lot(&self, x: i32, z: i32, w: i32, h: i32, facing: i32) {
        self.parcel_log
            .borrow_mut()
            .push(Json::Str(format!("({},{})-({},{}) f{}", x, z, x + w - 1, z + h - 1, facing)));
    }

    /// Appends one `Place` call to the interleaved parcel log, as
    /// `P(x1, z1, x2, z2)`.
    fn log_place(&self, a: (i32, i32), b: (i32, i32)) {
        self.parcel_log
            .borrow_mut()
            .push(Json::Str(format!("P({},{},{},{})", a.0, a.1, b.0, b.1)));
    }
}

fn rect_json(r: CellRect) -> Json {
    Json::Array(vec![ji(r.x1), ji(r.z1), ji(r.x2), ji(r.z2)])
}

// ---- ZonesHost ---------------------------------------------------------------------------------

impl ZonesHost for World {
    // ---- lifecycle, services and messages ----
    fn manager_acquire_services(&self) {
        self.events.push("manager_acquire_services", vec![]);
    }
    fn manager_release_services(&self) {
        self.events.push("manager_release_services", vec![]);
    }
    fn developer_acquire_services(&self) -> bool {
        self.events.push("developer_acquire_services", vec![]);
        self.services_ok.get()
    }
    fn developer_release_services(&self) {
        self.events.push("developer_release_services", vec![]);
    }
    fn subscribe(&self, who: Component, messages: &[u32]) {
        self.events.push(
            "subscribe",
            vec![
                ("who", ji(match who { Component::ZoneManager => 6, Component::ZoneDeveloper => 7 })),
                ("messages", Json::Array(messages.iter().map(|&m| ji(m)).collect())),
            ],
        );
    }
    fn unsubscribe(&self, who: Component, messages: &[u32]) {
        self.events.push(
            "unsubscribe",
            vec![
                ("who", ji(match who { Component::ZoneManager => 6, Component::ZoneDeveloper => 7 })),
                ("messages", Json::Array(messages.iter().map(|&m| ji(m)).collect())),
            ],
        );
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn city_cell_counts(&self) -> (i32, i32) {
        self.city.get()
    }
    fn service_ref(&self, _service: Service) -> RefValue {
        RefValue::Null
    }
    fn shutdown_grid(&self, class: u32) {
        self.events.push("shutdown_grid", vec![("class", ji(class))]);
    }
    fn acquire_network_tools(&self) {
        self.events.push("acquire_network_tools", vec![]);
    }
    fn release_network_tools(&self) {
        self.events.push("release_network_tools", vec![]);
    }
    fn post_zoning_change(&self, region: CellRegion, zone: i32, check_slope: bool) {
        self.zoning_changes.borrow_mut().push((region, zone, check_slope));
        self.events.push("zoning_change", vec![("zone", ji(zone)), ("check_slope", Json::Bool(check_slope))]);
    }
    fn post_defer_occupant_operations(&self, begin: bool) {
        self.deferred.borrow_mut().push(begin);
    }

    // ---- the other class ----
    fn zone_type(&self, x: i32, z: i32) -> i32 {
        match self.mgr() {
            Ok(m) => m.zone_type(x, z),
            Err(_) => 0,
        }
    }
    fn lot_manager_get_lot(&self, x: i32, z: i32) -> Option<u64> {
        self.dev().ok()?.get_lot(x, z, false)
    }
    fn lot_manager_lots_in_cell_rect(&self, rect: CellRect, origin_only: bool) -> Vec<u64> {
        match self.dev() {
            Ok(d) => d.get_lots_in_cell_rect(rect, origin_only),
            Err(_) => Vec::new(),
        }
    }
    fn zone_developer_do_parcellization(&self, region: &CellRegion, zone: i32, flag: bool) {
        if let Ok(d) = self.dev() {
            d.do_parcellization(region, zone, flag);
        }
    }

    // ---- budget, demolition, landfill ----
    fn total_funds(&self) -> i64 {
        self.funds.get()
    }
    fn withdraw_funds(&self, amount: i64) -> bool {
        let f = self.funds.get();
        if amount <= f {
            self.funds.set(f - amount);
            true
        } else {
            false
        }
    }
    fn demolish_lots(&self, commit: bool, lots: &[u64], _check_slope: bool, flags: u32) -> i64 {
        self.demolished.borrow_mut().push((commit, lots.to_vec(), flags));
        self.events.push(
            "demolish_lots",
            vec![("commit", Json::Bool(commit)), ("count", ji(lots.len() as i64)), ("flags", ji(flags))],
        );
        0
    }
    fn demolish_region(&self, region: &CellRegion) {
        self.events.push("demolish_region", vec![("count", ji(region.count() as i64))]);
    }
    fn landfill_zoning_cost(&self, _region: &CellRegion) -> i64 {
        0
    }

    // ---- networks ----
    fn find_network_tile(&self, x: i32, z: i32) -> Option<u64> {
        // The tile token is the cell key; its type/flag come from the net map.
        if self.nets.borrow().contains_key(&(x, z)) {
            Some((x as u64) << 32 | (z as u64))
        } else {
            None
        }
    }
    fn tile_occupant_flag_set(&self, _tile: u64, _flag: u32) -> bool {
        false
    }
    fn tile_is_of_type(&self, tile: u64, t: u32) -> bool {
        let x = (tile >> 32) as i32;
        let z = (tile & 0xFFFF_FFFF) as i32;
        self.nets.borrow().get(&(x, z)).is_some_and(|n| n.kind == t)
    }
    fn tile_demolition_cost(&self, _tile: u64) -> i64 {
        0
    }
    fn network_tool_draft_at(&self, _tool: u32, _x: i32, _z: i32) -> bool {
        false
    }
    fn street_place(&self, kind: u32, a: (i32, i32), b: (i32, i32)) -> bool {
        self.street_place_count.set(self.street_place_count.get() + 1);
        self.log_place(a, b);
        // The stub: a segment fails when it contains a fail cell.
        let fail_cells = self.street_fail_cells.borrow();
        let cells = line_cells(a, b);
        if cells.iter().any(|c| fail_cells.contains(c)) {
            return false;
        }
        // Put a street (kind 3) on every cell of a..b with no network.
        let mut nets = self.nets.borrow_mut();
        for &(x, z) in &cells {
            if !nets.contains_key(&(x, z)) {
                nets.insert((x, z), NetCell { kind, edges: [false; 4] });
            }
        }
        // Connect the run along its direction.
        for i in 0..cells.len() {
            let (x, z) = cells[i];
            if i + 1 < cells.len() {
                let (nx, nz) = cells[i + 1];
                let e = edge_to(x, z, nx, nz);
                let e2 = edge_to(nx, nz, x, z);
                if let Some(n) = nets.get_mut(&(x, z)) {
                    if e >= 0 {
                        n.edges[e as usize] = true;
                    }
                }
                if let Some(n) = nets.get_mut(&(nx, nz)) {
                    if e2 >= 0 {
                        n.edges[e2 as usize] = true;
                    }
                }
            }
        }
        true
    }
    fn connects_to_neighbor(&self, occupant: u64, edge: i32, net_type: u32) -> bool {
        if edge < 0 {
            return false;
        }
        let (x, z) = ((occupant >> 32) as i32, (occupant & 0xFFFF_FFFF) as i32);
        self.nets
            .borrow()
            .get(&(x, z))
            .is_some_and(|n| n.kind == net_type && n.edges[edge as usize])
    }
    fn network_tool_occupant_exists(&self, _tool_kind: u32, _x: i32, _z: i32, _flag: u32) -> Option<[u8; 4]> {
        None
    }
    fn street_joining_helper(&self, occupant: u64, data4: u32) {
        self.events.push("street_joining_helper", vec![("data4", ji(data4))]);
        let _ = occupant;
    }

    // ---- terrain ----
    fn cell_has_water(&self, x: i32, z: i32) -> bool {
        self.water.borrow().contains(&(x, z))
    }
    fn terrain_altitude(&self, x: f32, z: f32) -> f32 {
        self.terrain.borrow().get(&(x as i32, z as i32)).copied().unwrap_or(0.0)
    }
    fn terrain_vertex_altitude(&self, vx: i32, vz: i32) -> f32 {
        self.terrain.borrow().get(&(vx, vz)).copied().unwrap_or(0.0)
    }

    // ---- city and occupants ----
    fn location_in_bounds(&self, pos: Position) -> bool {
        let (cx, cz) = self.city.get();
        let cellx = (pos[0] / 16.0) as i32;
        let cellz = (pos[2] / 16.0) as i32;
        (0..cx).contains(&cellx) && (0..cz).contains(&cellz)
    }
    fn position_to_cell(&self, pos: Position) -> Option<(i32, i32)> {
        Some(((pos[0] / 16.0) as i32, (pos[2] / 16.0) as i32))
    }
    fn cell_occupants(&self, x: i32, z: i32, type_filter: Option<u32>) -> Vec<u64> {
        let cell = self.cell_occupants.borrow();
        let occs = self.occupants.borrow();
        let mut out = Vec::new();
        // Network occupants are modelled in the net map, keyed by cell.
        if type_filter == Some(NETWORK_OCCUPANT_TYPE) {
            if let Some(n) = self.nets.borrow().get(&(x, z)) {
                let token = (x as u64) << 32 | (z as u64);
                let _ = n;
                out.push(token);
            }
            return out;
        }
        if let Some(list) = cell.get(&(x, z)) {
            for &o in list {
                if let Some(oc) = occs.get(&o) {
                    if type_filter.is_none() || type_filter == Some(oc.kind) {
                        out.push(o);
                    }
                }
            }
        }
        out
    }
    fn landfill_at(&self, x: i32, z: i32) -> Option<u64> {
        let cell = self.cell_occupants.borrow();
        let occs = self.occupants.borrow();
        cell.get(&(x, z))?.iter().copied().find(|o| {
            occs.get(o).is_some_and(|oc| oc.kind == LANDFILL_TYPE)
        })
    }
    fn occupant_type(&self, occupant: u64) -> u32 {
        self.occupants.borrow().get(&occupant).map_or(0, |o| o.kind)
    }
    fn occupant_position(&self, occupant: u64) -> Position {
        self.occupants.borrow().get(&occupant).map_or([0.0; 3], |o| o.pos)
    }
    fn occupant_bounding_cells(&self, occupant: u64) -> CellRect {
        let p = self.occupant_position(occupant);
        let (x, z) = ((p[0] / 16.0) as i32, (p[2] / 16.0) as i32);
        CellRect::new(x, z, x, z)
    }
    fn occupant_can_remove(&self, occupant: u64, _flags: u32) -> bool {
        self.occupants.borrow().get(&occupant).map_or(true, |o| o.can_remove)
    }
    fn network_has_any_flag(&self, occupant: u64, flags: u32) -> Option<bool> {
        let (x, z) = ((occupant >> 32) as i32, (occupant & 0xFFFF_FFFF) as i32);
        self.nets.borrow().get(&(x, z)).map(|n| (1u32 << n.kind) & flags != 0)
    }
    fn remove_occupant(&self, occupant: u64) {
        self.events.push("remove_occupant", vec![("occ", ji(occupant as i64))]);
    }

    // ---- reference counts ----
    fn add_ref(&self, object: u64) {
        if let Some(l) = self.lots.borrow_mut().get_mut(&object) {
            l.refs += 1;
        }
    }
    fn release(&self, object: u64) {
        if let Some(l) = self.lots.borrow_mut().get_mut(&object) {
            l.refs = l.refs.saturating_sub(1);
        }
    }

    // ---- lots ----
    fn new_lot(&self, x: i32, z: i32, w: i32, h: i32, facing: i32) -> u64 {
        let t = self.token();
        let zone = self.zone_type(x, z);
        self.log_lot(x, z, w, h, facing);
        self.lots.borrow_mut().insert(
            t,
            Lot {
                x,
                z,
                w,
                h,
                facing,
                zone,
                state: 0,
                flags: 0,
                altitude: 0.0,
                locked: false,
                cfg: None,
                building: None,
                business_unit: false,
                refs: 0,
            },
        );
        t
    }
    fn lot_init(&self, _lot: u64) {}
    fn lot_shutdown(&self, _lot: u64) {}
    fn lot_post_city_init(&self, _lot: u64) {}
    fn lot_configuration(&self, lot: u64) -> Option<u64> {
        self.lot(lot, |l| l.cfg)
    }
    fn lot_flags(&self, lot: u64) -> u32 {
        self.lot(lot, |l| l.flags)
    }
    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        self.lot(lot, |l| l.rect())
    }
    fn lot_altitude(&self, lot: u64) -> f32 {
        self.lot(lot, |l| l.altitude)
    }
    fn lot_location(&self, lot: u64) -> (i32, i32) {
        self.lot(lot, |l| (l.x, l.z))
    }
    fn lot_facing(&self, lot: u64) -> i32 {
        self.lot(lot, |l| l.facing)
    }
    fn lot_has_business_unit(&self, lot: u64) -> bool {
        self.lot(lot, |l| l.business_unit)
    }
    fn lot_zone_type(&self, lot: u64) -> i32 {
        self.lot(lot, |l| l.zone)
    }
    fn lot_state(&self, lot: u64) -> u32 {
        self.lot(lot, |l| l.state)
    }
    fn lot_set_state(&self, lot: u64, state: u32, _a: u32, _b: u32) {
        if let Some(l) = self.lots.borrow_mut().get_mut(&lot) {
            l.state = state;
        }
    }
    fn lot_building(&self, lot: u64) -> Option<u64> {
        self.lot(lot, |l| l.building)
    }
    fn lot_set_vertices_locked(&self, lot: u64, locked: bool) {
        if let Some(l) = self.lots.borrow_mut().get_mut(&lot) {
            l.locked = locked;
        }
    }
    fn lot_ref(&self, lot: u64) -> RefValue {
        RefValue::Host {
            token: lot,
            class_id: IID_LOT,
        }
    }
    fn lot_from_ref(&self, r: &RefValue) -> Option<u64> {
        match r {
            RefValue::Host { token, .. } => Some(*token),
            _ => None,
        }
    }

    // ---- lot configurations ----
    fn cfg_compatible_with_water(&self, cfg: u64) -> bool {
        self.cfgs.borrow().get(&cfg).map_or(false, |c| c.water)
    }
    fn cfg_compatible_with_zone(&self, cfg: u64, _zone: i32) -> bool {
        self.cfgs.borrow().get(&cfg).map_or(false, |c| c.zone_ok)
    }
    fn cfg_id(&self, cfg: u64) -> u32 {
        cfg as u32
    }
    fn cfg_size(&self, cfg: u64) -> (u32, u32) {
        self.cfgs.borrow().get(&cfg).map_or((0, 0), |c| (c.w, c.h))
    }
    fn cfg_primary_building_families(&self, cfg: u64) -> Option<Vec<u32>> {
        self.cfgs.borrow().get(&cfg).and_then(|c| {
            c.primary_families.map(|f| f[..c.n_families].to_vec())
        })
    }
    fn configurations_by_filter(&self, _purpose: u32, _wealth: u32, _stage: i32) -> Vec<u64> {
        self.cfgs.borrow().keys().copied().collect()
    }

    // ---- lot developer ----
    fn area_slope(&self, _rect: CellRect, _with_water: bool) -> f32 {
        0.0
    }
    fn attempt_area_levelling(&self, _rect: CellRect) {}
    fn remove_building_zot_prop(&self, _lot: u64) {}
    fn lot_base_texture_occupant(&self, _lot: u64) -> Option<u64> {
        None
    }
    fn remove_lot_from_business_unit(&self, _lot: u64) {}
    fn set_empty_lot_base_texture(&self, _lot: u64) {}

    // ---- traffic simulator ----
    fn desired_lot_insertion_point(&self) -> Option<InsertionPoint> {
        *self.insertion.borrow()
    }
    fn set_lot_insertion_point(&self, _node: Option<u64>) {}

    // ---- BDS and growth developers ----
    fn bds_developer(&self, dev_type: u32) -> Option<u64> {
        self.devs.borrow().iter().find(|(_, d)| d.purpose == dev_type).map(|(&k, _)| k)
    }
    fn building_types_by_family(&self, family: u32) -> Vec<u32> {
        self.families.borrow().get(&family).cloned().unwrap_or_default()
    }
    fn building_profile(&self, building: u32) -> BuildingProfile {
        self.buildings.borrow().get(&building).cloned().unwrap_or_default()
    }
    fn dev_estimate_desirability(&self, dev: u64, x: i32, z: i32) -> i16 {
        self.devs.borrow().get(&dev).and_then(|d| d.desirability.get(&(x, z)).copied()).unwrap_or(0)
    }
    fn dev_source_demand(&self, dev: u64) -> Option<f32> {
        self.devs.borrow().get(&dev).and_then(|d| d.demand)
    }
    fn dev_growth_list(&self, dev: u64) -> Option<Vec<GrowthItem>> {
        self.devs.borrow().get(&dev).and_then(|d| d.growth.clone())
    }
    fn dev_desirability_value(&self, dev: u64, tx: i32, tz: i32) -> i16 {
        self.devs.borrow().get(&dev).and_then(|d| d.desir_value.get(&(tx, tz)).copied()).unwrap_or(0)
    }
    fn dev_desirability_shift(&self, dev: u64) -> u32 {
        self.devs.borrow().get(&dev).map_or(0, |d| d.shift)
    }
    fn dev_purpose(&self, dev: u64) -> u32 {
        self.devs.borrow().get(&dev).map_or(0, |d| d.purpose)
    }
    fn dev_wealth(&self, dev: u64) -> u32 {
        self.devs.borrow().get(&dev).map_or(0, |d| d.wealth)
    }
}

impl GridProvider<i8> for World {
    fn create_grid(&self, _instance: u32, _tract_size: i32) -> Option<Rc<dyn TractGrid<i8>>> {
        if let Some(g) = self.zone_grid.borrow().clone() {
            return Some(g as Rc<dyn TractGrid<i8>>);
        }
        let (cx, cz) = self.city.get();
        let g = SimGrid::<i8>::create(&CityDims { cells_x: cx, cells_z: cz, cell_width_x: 16.0, cell_width_z: 16.0 }, 0, 1);
        *self.zone_grid.borrow_mut() = Some(g.clone());
        Some(g as Rc<dyn TractGrid<i8>>)
    }
}

impl GridProvider<u16> for World {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u16>>> {
        if let Some(g) = self.maxcap_grid.borrow().clone() {
            return Some(g as Rc<dyn TractGrid<u16>>);
        }
        let (cx, cz) = self.city.get();
        let g = SimGrid::<u16>::create(&CityDims { cells_x: cx, cells_z: cz, cell_width_x: 16.0, cell_width_z: 16.0 }, instance, tract_size);
        *self.maxcap_grid.borrow_mut() = Some(g.clone());
        Some(g as Rc<dyn TractGrid<u16>>)
    }
}

/// The cells of a straight line from `a` to `b` (inclusive), in order.
fn line_cells(a: (i32, i32), b: (i32, i32)) -> Vec<(i32, i32)> {
    let ux = (b.0 - a.0).signum();
    let uz = (b.1 - a.1).signum();
    let n = (b.0 - a.0).abs().max((b.1 - a.1).abs());
    (0..=n).map(|i| (a.0 + ux * i, a.1 + uz * i)).collect()
}

/// The network edge from `(x, z)` towards an adjacent `(nx, nz)`: 0 = −x,
/// 1 = −z, 2 = +x, 3 = +z; −1 when not adjacent.
fn edge_to(x: i32, z: i32, nx: i32, nz: i32) -> i32 {
    if nx == x - 1 && nz == z {
        0
    } else if nx == x && nz == z - 1 {
        1
    } else if nx == x + 1 && nz == z {
        2
    } else if nx == x && nz == z + 1 {
        3
    } else {
        -1
    }
}

// ---- ops ---------------------------------------------------------------------------------------

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "zones", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("zone_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "create" => create(&w, op),
        "rng" => rng(&w, op),
        "network" => network(&w, op),
        "zones" => zones(&w, op),
        "water" => water(&w, op),
        "lot" => lot(&w, op),
        "cfg" => cfg(&w, op),
        "dev" => dev(&w, op),
        "family" => family(&w, op),
        "building" => building(&w, op),
        "occupant" => occupant(&w, op),
        "options" => options(&w, op),
        "lot_size" => lot_size(&w, op),
        "street_fail" => street_fail(&w, op),
        "init" => init(&w, op),
        "shutdown" => shutdown(&w, op),
        "message" => message(&w, op),
        "place_zone" => place_zone(&w, op),
        "do_parcellization" => do_parcellization(&w, op),
        "draw_street" => draw_street(&w, op),
        "call" => call(&w, op),
        "eval" => eval(op),
        "expect" => expect(&w, op),
        "expect_parcel_log" => expect_parcel_log(&w, op),
        "expect_log" => w.events.expect_events(op),
        "expect_draws" => expect_draws(&w, op),
        "clear_events" => {
            w.events.clear();
            w.parcel_log.borrow_mut().clear();
            Ok(())
        }
        _ => Err(format!("unknown op '{name}'")),
    })
}

fn rect_of(j: &Json) -> R<CellRect> {
    let a = arr(j, "rect")?;
    Ok(CellRect::new(
        i64_of(field_at(a, 0)?, "x1")? as i32,
        i64_of(field_at(a, 1)?, "z1")? as i32,
        i64_of(field_at(a, 2)?, "x2")? as i32,
        i64_of(field_at(a, 3)?, "z2")? as i32,
    ))
}

fn field_at<'a>(a: &'a [Json], i: usize) -> R<&'a Json> {
    a.get(i).ok_or_else(|| format!("missing element {i}"))
}

/// `zone_env`: `cells` (`[x, z]`), `funds`, `services_ok`, `tuning`
/// (`"shipped"` or null).
fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "cells") {
        let a = arr(v, "cells")?;
        w.city.set((i64_of(field_at(a, 0)?, "x")? as i32, i64_of(field_at(a, 1)?, "z")? as i32));
    }
    if let Some(v) = opt(op, "funds") {
        w.funds.set(i64_of(v, "funds")?);
    }
    if let Some(v) = opt(op, "services_ok") {
        w.services_ok.set(bool_of(v, "services_ok")?);
    }
    if let Some(v) = op.get("tuning") {
        match v {
            Json::Null => *w.tuning.borrow_mut() = None,
            Json::Str(s) if s == "shipped" => *w.tuning.borrow_mut() = Some(Rc::new(shipped_tuning())),
            other => *w.tuning.borrow_mut() = Some(Rc::new(scalar::holder_of(other)?)),
        }
    }
    Ok(())
}

/// `zone_create`: `session` (the session seed) or `seed` (an exact seed) for
/// the shared statics; `manager` / `developer` (default both true).
fn create(w: &World, op: &Json) -> R<()> {
    let statics = match opt(op, "session") {
        Some(v) => ZoneDeveloperStatics::from_session(u32_of(v, "session")?, &SEEDS),
        None => match opt(op, "seed") {
            Some(v) => ZoneDeveloperStatics::new(u32_of(v, "seed")?, &SEEDS),
            None => ZoneDeveloperStatics::from_session(0, &SEEDS),
        },
    };
    if opt(op, "trace").and_then(boolean).unwrap_or(true) {
        statics.borrow_mut().trace = Some(Vec::new());
    }
    *w.statics.borrow_mut() = Some(statics.clone());
    let host = w.rc()? as Rc<dyn ZonesHost>;
    if opt(op, "manager").and_then(boolean).unwrap_or(true) {
        *w.manager.borrow_mut() = Some(Rc::new(ZoneManager::new(host.clone())));
    }
    if opt(op, "developer").and_then(boolean).unwrap_or(true) {
        *w.developer.borrow_mut() = Some(Rc::new(ZoneDeveloper::new(host, statics)));
    }
    Ok(())
}

/// `zone_rng`: `lcg` / `mul` (set the generator state directly) or `seed`.
fn rng(w: &World, op: &Json) -> R<()> {
    let st = w.statics.borrow().clone().ok_or("no statics (use zone_create)")?;
    let mut s = st.borrow_mut();
    if let Some(v) = opt(op, "seed") {
        s.rng = RzRandom::new(u32_of(v, "seed")?, &SEEDS);
    }
    if opt(op, "lcg").is_some() || opt(op, "mul").is_some() {
        let lcg = opt(op, "lcg").map_or(Ok(s.rng.lcg_state()), |v| u32_of(v, "lcg"))?;
        let mul = opt(op, "mul").map_or(Ok(s.rng.mul_state()), |v| u32_of(v, "mul"))?;
        s.rng.set_state(lcg, mul, None);
    }
    Ok(())
}

/// `zone_network`: `runs` (`[{kind, cells or [x1,z1,x2,z2]}]`) laid as
/// straight runs; each cell gets the kind and edges toward its run
/// neighbours. Replaces all networks unless `add` is true.
fn network(w: &World, op: &Json) -> R<()> {
    if !opt(op, "add").and_then(boolean).unwrap_or(false) {
        w.nets.borrow_mut().clear();
    }
    let runs = arr(field(op, "runs")?, "runs")?;
    let mut nets = w.nets.borrow_mut();
    for run in runs {
        let kind = u32_of(field(run, "kind")?, "kind")?;
        let cells: Vec<(i32, i32)> = if let Some(r) = opt(run, "rect") {
            let rc = rect_of(r)?;
            line_cells((rc.x1, rc.z1), (rc.x2, rc.z2))
        } else if let Some(c) = opt(run, "cells") {
            let a = arr(c, "cells")?;
            a.iter()
                .map(|e| {
                    let p = arr(e, "cell")?;
                    Ok((i64_of(field_at(p, 0)?, "x")? as i32, i64_of(field_at(p, 1)?, "z")? as i32))
                })
                .collect::<R<Vec<_>>>()?
        } else {
            return Err("a run needs 'rect' or 'cells'".into());
        };
        for &(x, z) in &cells {
            nets.insert((x, z), NetCell { kind, edges: [false; 4] });
        }
        for i in 0..cells.len() {
            let (x, z) = cells[i];
            if i + 1 < cells.len() {
                let (nx, nz) = cells[i + 1];
                let e = edge_to(x, z, nx, nz);
                let e2 = edge_to(nx, nz, x, z);
                if e >= 0 {
                    nets.get_mut(&(x, z)).unwrap().edges[e as usize] = true;
                }
                if e2 >= 0 {
                    nets.get_mut(&(nx, nz)).unwrap().edges[e2 as usize] = true;
                }
            }
        }
    }
    Ok(())
}

/// `zone_zones`: writes zone types directly into the manager's zone grid
/// (test setup that does not go through `PlaceZone`). `cells` is
/// `[[x, z, v], …]`, or `rect` (`[x1, z1, x2, z2]`) with `v`. The grid must
/// exist (after `zone_init`).
fn zones(w: &World, op: &Json) -> R<()> {
    let m = w.mgr()?;
    let g = m.zone_grid().ok_or("no zone grid (use zone_init)")?;
    if let Some(v) = opt(op, "rect") {
        let r = rect_of(v)?;
        let z = i64_of(field(op, "v")?, "v")? as i8;
        for x in r.x1..=r.x2 {
            for zz in r.z1..=r.z2 {
                g.set(x, zz, z);
            }
        }
    }
    if let Some(v) = opt(op, "cells") {
        for e in arr(v, "cells")? {
            let a = arr(e, "cell")?;
            let x = i64_of(field_at(a, 0)?, "x")? as i32;
            let zz = i64_of(field_at(a, 1)?, "z")? as i32;
            let val = i64_of(field_at(a, 2)?, "v")? as i8;
            g.set(x, zz, val);
        }
    }
    Ok(())
}

/// `zone_water`: `cells` (`[[x, z], …]`), `clear`.
fn water(w: &World, op: &Json) -> R<()> {
    let mut s = w.water.borrow_mut();
    if opt(op, "clear").and_then(boolean).unwrap_or(false) {
        s.clear();
    }
    if let Some(v) = opt(op, "cells") {
        for e in arr(v, "cells")? {
            let p = arr(e, "cell")?;
            s.insert((i64_of(field_at(p, 0)?, "x")? as i32, i64_of(field_at(p, 1)?, "z")? as i32));
        }
    }
    Ok(())
}

/// `zone_lot`: a pre-existing lot `{rect, facing, zone?, cfg?, state?, flags?,
/// building?, business_unit?}`. Written into the developer's cell map through
/// `create_lot` is not possible (it creates via the core), so this inserts a
/// lot the manager sees: it calls the developer's CreateLot and then patches
/// the fields.
fn lot(w: &World, op: &Json) -> R<()> {
    let d = w.dev()?;
    let r = rect_of(field(op, "rect")?)?;
    let facing = opt(op, "facing").map_or(Ok(0), |v| i64_of(v, "facing"))? as i32;
    let lot = d
        .create_lot(r.x1, r.z1, r.x2 - r.x1 + 1, r.z2 - r.z1 + 1, facing)
        .ok_or("create_lot failed")?;
    {
        let mut lots = w.lots.borrow_mut();
        if let Some(l) = lots.get_mut(&lot) {
            if let Some(v) = opt(op, "zone") {
                l.zone = i64_of(v, "zone")? as i32;
            }
            if let Some(v) = opt(op, "state") {
                l.state = u32_of(v, "state")?;
            }
            if let Some(v) = opt(op, "flags") {
                l.flags = u32_of(v, "flags")?;
            }
            if let Some(v) = opt(op, "cfg") {
                l.cfg = Some(u32_of(v, "cfg")? as u64);
            }
            if let Some(v) = opt(op, "building") {
                l.building = Some(u32_of(v, "building")? as u64);
            }
            if let Some(v) = opt(op, "business_unit") {
                l.business_unit = bool_of(v, "business_unit")?;
            }
            if let Some(v) = opt(op, "altitude") {
                l.altitude = f32_of(v, "altitude")?;
            }
        }
    }
    w.events.push("zone_lot", vec![("lot", ji(lot as i64))]);
    Ok(())
}

/// `zone_cfg`: a lot configuration `{id, w, h, water?, zone_ok?, families?}`.
fn cfg(w: &World, op: &Json) -> R<()> {
    let id = u32_of(field(op, "id")?, "id")? as u64;
    let fams: Option<Vec<u32>> = opt(op, "families").map(|v| {
        arr(v, "families")?.iter().map(|e| u32_of(e, "family")).collect::<R<Vec<_>>>()
    }).transpose()?;
    let mut f = [0u32; 4];
    let n = fams.as_ref().map_or(0, |v| v.len().min(4));
    if let Some(v) = &fams {
        for (i, &x) in v.iter().take(4).enumerate() {
            f[i] = x;
        }
    }
    w.cfgs.borrow_mut().insert(
        id,
        Cfg {
            w: u32_of(field(op, "w")?, "w")?,
            h: u32_of(field(op, "h")?, "h")?,
            water: opt(op, "water").and_then(boolean).unwrap_or(false),
            zone_ok: opt(op, "zone_ok").and_then(boolean).unwrap_or(true),
            primary_families: fams.as_ref().map(|_| f),
            n_families: n,
        },
    );
    Ok(())
}

/// `zone_dev`: a growth developer `{token, purpose, wealth, shift?, demand?,
/// desirability? ([[x, z, v]]), growth? ([[x, z, maxStage]]), desir_value?}`.
fn dev(w: &World, op: &Json) -> R<()> {
    let token = u32_of(field(op, "token")?, "token")? as u64;
    let mut d = Dev {
        purpose: u32_of(field(op, "purpose")?, "purpose")?,
        wealth: u32_of(field(op, "wealth")?, "wealth")?,
        desirability: BTreeMap::new(),
        demand: opt(op, "demand").map(|v| f32_of(v, "demand")).transpose()?,
        shift: opt(op, "shift").map_or(Ok(0), |v| u32_of(v, "shift"))?,
        growth: None,
        desir_value: BTreeMap::new(),
    };
    // `desirability` here is EstimateDesirability(x, z); `dev_purpose` is
    // matched against the developer-type key, so `purpose` doubles as the
    // developer type the BDS returns it for.
    if let Some(v) = opt(op, "estimate") {
        for e in arr(v, "estimate")? {
            let a = arr(e, "e")?;
            d.desirability.insert(
                (i64_of(field_at(a, 0)?, "x")? as i32, i64_of(field_at(a, 1)?, "z")? as i32),
                i64_of(field_at(a, 2)?, "v")? as i16,
            );
        }
    }
    if let Some(v) = opt(op, "growth") {
        let mut g = Vec::new();
        for e in arr(v, "growth")? {
            let a = arr(e, "g")?;
            g.push(GrowthItem {
                x: i64_of(field_at(a, 0)?, "x")? as i32,
                z: i64_of(field_at(a, 1)?, "z")? as i32,
                max_stage: i64_of(field_at(a, 2)?, "s")? as i32,
            });
        }
        d.growth = Some(g);
    }
    if let Some(v) = opt(op, "desir_value") {
        for e in arr(v, "desir_value")? {
            let a = arr(e, "d")?;
            d.desir_value.insert(
                (i64_of(field_at(a, 0)?, "x")? as i32, i64_of(field_at(a, 1)?, "z")? as i32),
                i64_of(field_at(a, 2)?, "v")? as i16,
            );
        }
    }
    w.devs.borrow_mut().insert(token, d);
    Ok(())
}

fn family(w: &World, op: &Json) -> R<()> {
    let id = u32_of(field(op, "id")?, "id")?;
    let types = arr(field(op, "types")?, "types")?
        .iter()
        .map(|e| u32_of(e, "type"))
        .collect::<R<Vec<_>>>()?;
    w.families.borrow_mut().insert(id, types);
    Ok(())
}

fn building(w: &World, op: &Json) -> R<()> {
    let id = u32_of(field(op, "id")?, "id")?;
    let mut p = BuildingProfile::default();
    p.purpose = opt(op, "purpose").map_or(Ok(0), |v| u32_of(v, "purpose"))? as u8;
    p.wealth = opt(op, "wealth").map_or(Ok(0), |v| u32_of(v, "wealth"))? as u8;
    p.capacity = opt(op, "capacity").map_or(Ok(0), |v| u32_of(v, "capacity"))? as u16;
    w.buildings.borrow_mut().insert(id, p);
    Ok(())
}

fn occupant(w: &World, op: &Json) -> R<()> {
    let token = u32_of(field(op, "token")?, "token")? as u64;
    let pos = arr(field(op, "pos")?, "pos")?;
    let p = [
        f32_of(field_at(pos, 0)?, "x")?,
        f32_of(field_at(pos, 1)?, "y")?,
        f32_of(field_at(pos, 2)?, "z")?,
    ];
    let kind = u32_of(field(op, "type")?, "type")?;
    w.occupants.borrow_mut().insert(
        token,
        Occ {
            kind,
            pos: p,
            can_remove: opt(op, "can_remove").and_then(boolean).unwrap_or(true),
        },
    );
    let cell = (p[0] as i32 / 16, p[2] as i32 / 16);
    w.cell_occupants.borrow_mut().entry(cell).or_default().push(token);
    Ok(())
}

fn options(w: &World, op: &Json) -> R<()> {
    let d = w.dev()?;
    let a = opt(op, "alternate").and_then(boolean).unwrap_or(false);
    let p = opt(op, "streets").and_then(boolean).unwrap_or(true);
    let c = opt(op, "custom").and_then(boolean).unwrap_or(false);
    d.set_options(a, p, c);
    Ok(())
}

/// `zone_lot_size`: force the lot-size parameters (bypassing
/// `DetermineLotSize`, §7.2.1), as the tests 19–29 common setup does.
fn lot_size(w: &World, op: &Json) -> R<()> {
    let d = w.dev()?;
    let ls = LotSize {
        width: opt(op, "width").map_or(Ok(2), |v| i64_of(v, "width"))? as i32,
        depth: opt(op, "depth").map_or(Ok(2), |v| i64_of(v, "depth"))? as i32,
        min_depth: opt(op, "min_depth").map_or(Ok(1), |v| i64_of(v, "min_depth"))? as i32,
        max_depth: opt(op, "max_depth").map_or(Ok(3), |v| i64_of(v, "max_depth"))? as i32,
        block: opt(op, "block").map_or(Ok(6), |v| i64_of(v, "block"))? as i32,
    };
    d.set_lot_size(ls);
    Ok(())
}

fn street_fail(w: &World, op: &Json) -> R<()> {
    let mut s = w.street_fail_cells.borrow_mut();
    s.clear();
    if let Some(v) = opt(op, "cells") {
        for e in arr(v, "cells")? {
            let p = arr(e, "cell")?;
            s.insert((i64_of(field_at(p, 0)?, "x")? as i32, i64_of(field_at(p, 1)?, "z")? as i32));
        }
    }
    Ok(())
}

fn init(w: &World, op: &Json) -> R<()> {
    let mut ok = true;
    if w.manager.borrow().is_some() {
        ok &= w.mgr()?.init();
    }
    if w.developer.borrow().is_some() {
        ok &= w.dev()?.init();
    }
    check_expect(op, Json::Bool(ok))
}

fn shutdown(w: &World, op: &Json) -> R<()> {
    let mut ok = true;
    if w.manager.borrow().is_some() {
        ok &= w.mgr()?.shutdown();
    }
    if w.developer.borrow().is_some() {
        ok &= w.dev()?.shutdown();
    }
    check_expect(op, Json::Bool(ok))
}

fn message(w: &World, op: &Json) -> R<()> {
    let id = u32_of(field(op, "id")?, "id")?;
    let m = ZonesMessage {
        id,
        occupant: opt(op, "occupant").map(|v| u32_of(v, "occupant")).transpose()?.map(u64::from),
        data1: opt(op, "data1").map_or(Ok(0), |v| u32_of(v, "data1"))?,
        data3: opt(op, "data3").map_or(Ok(0), |v| u32_of(v, "data3"))?,
        data4: opt(op, "data4").map_or(Ok(0), |v| u32_of(v, "data4"))?,
        value: opt(op, "value").map_or(Ok(0), |v| u32_of(v, "value"))?,
    };
    let target = opt(op, "target").and_then(|v| v.as_str()).unwrap_or("manager");
    let ok = match target {
        "developer" => w.dev()?.do_message(&m),
        _ => w.mgr()?.do_message(&m),
    };
    check_expect(op, Json::Bool(ok))
}

/// `zone_place_zone`: `region` (`{rect, cells?}`), `type`, `commit`,
/// `skip_funds`, `check_slope`, `skip_size`, `check_lot_inside`.
fn place_zone(w: &World, op: &Json) -> R<()> {
    let m = w.mgr()?;
    let rj = field(op, "region")?;
    let rect = rect_of(field(rj, "rect")?)?;
    let mut region = match opt(rj, "cells") {
        Some(c) => {
            let cells = arr(c, "cells")?
                .iter()
                .map(|e| {
                    let p = arr(e, "cell")?;
                    Ok((i64_of(field_at(p, 0)?, "x")? as i32, i64_of(field_at(p, 1)?, "z")? as i32))
                })
                .collect::<R<Vec<_>>>()?;
            CellRegion::from_cells(rect, &cells)
        }
        None => CellRegion::new(rect, true),
    };
    let zone_type = i64_of(field(op, "type")?, "type")? as i32;
    let commit = opt(op, "commit").and_then(boolean).unwrap_or(true);
    let skip_funds = opt(op, "skip_funds").and_then(boolean).unwrap_or(false);
    let check_slope = opt(op, "check_slope").and_then(boolean).unwrap_or(false);
    let skip_size = opt(op, "skip_size").and_then(boolean).unwrap_or(false);
    let check_lot_inside = opt(op, "check_lot_inside").and_then(boolean).unwrap_or(true);
    let res: PlaceZoneResult = m.place_zone(&mut region, zone_type, commit, skip_funds, check_slope, skip_size, check_lot_inside);
    w.events.push(
        "place_zone_result",
        vec![("ok", Json::Bool(res.ok)), ("cost", ji(res.cost)), ("error", ji(res.error))],
    );
    if let Some(e) = op.get("expect") {
        let got = Json::Object(vec![
            ("ok".into(), Json::Bool(res.ok)),
            ("cost".into(), ji(res.cost)),
            ("error".into(), ji(res.error)),
        ]);
        if !matches(&got, e, true) {
            return Err(format!("place_zone: got {}, expected {}", show(&got), show(e)));
        }
    }
    Ok(())
}

/// `zone_do_parcellization`: `region` (`{rect, cells?}`), `type`, `flag`.
fn do_parcellization(w: &World, op: &Json) -> R<()> {
    let d = w.dev()?;
    if !opt(op, "keep_log").and_then(boolean).unwrap_or(false) {
        w.parcel_log.borrow_mut().clear();
    }
    let rj = field(op, "region")?;
    let rect = rect_of(field(rj, "rect")?)?;
    let region = match opt(rj, "cells") {
        Some(c) => {
            let cells = arr(c, "cells")?
                .iter()
                .map(|e| {
                    let p = arr(e, "cell")?;
                    Ok((i64_of(field_at(p, 0)?, "x")? as i32, i64_of(field_at(p, 1)?, "z")? as i32))
                })
                .collect::<R<Vec<_>>>()?;
            CellRegion::from_cells(rect, &cells)
        }
        None => CellRegion::new(rect, true),
    };
    let zone_type = i64_of(field(op, "type")?, "type")? as i32;
    let flag = opt(op, "flag").and_then(boolean).unwrap_or(false);
    d.do_parcellization(&region, zone_type, flag);
    Ok(())
}

/// `zone_draw_street`: drive `DrawStreet` directly with a restriction region
/// (test 28). `region` (`{rect, cells?}`), `x1, z1, x2, z2, per_cell`.
fn draw_street(w: &World, op: &Json) -> R<()> {
    let d = w.dev()?;
    if !opt(op, "keep_log").and_then(boolean).unwrap_or(false) {
        w.parcel_log.borrow_mut().clear();
    }
    let rj = field(op, "region")?;
    let rect = rect_of(field(rj, "rect")?)?;
    let region = match opt(rj, "cells") {
        Some(c) => {
            let cells = arr(c, "cells")?
                .iter()
                .map(|e| {
                    let p = arr(e, "cell")?;
                    Ok((i64_of(field_at(p, 0)?, "x")? as i32, i64_of(field_at(p, 1)?, "z")? as i32))
                })
                .collect::<R<Vec<_>>>()?;
            CellRegion::from_cells(rect, &cells)
        }
        None => CellRegion::new(rect, true),
    };
    let x1 = i64_of(field(op, "x1")?, "x1")? as i32;
    let z1 = i64_of(field(op, "z1")?, "z1")? as i32;
    let x2 = i64_of(field(op, "x2")?, "x2")? as i32;
    let z2 = i64_of(field(op, "z2")?, "z2")? as i32;
    let per_cell = opt(op, "per_cell").and_then(boolean).unwrap_or(false);
    d.draw_street_for_test(&region, x1, z1, x2, z2, per_cell);
    Ok(())
}

/// `zone_call`: one method of either class by name.
fn call(w: &World, op: &Json) -> R<()> {
    let method = str_of(field(op, "method")?, "method")?;
    let target = opt(op, "target").and_then(|v| v.as_str()).unwrap_or("developer");
    let a = |i: usize| arg(op, i);
    let got: Json = match (target, method) {
        // ---- zone manager queries (§11.1) ----
        ("manager", "zone_count") => ji(w.mgr()?.zone_count(i64_of(a(0)?, "a")? as i32)),
        ("manager", "zone_type") => {
            let x = i64_of(a(0)?, "x")? as i32;
            let z = i64_of(a(1)?, "z")? as i32;
            ji(w.mgr()?.zone_type(x, z))
        }
        ("manager", "is_rci_zone") => Json::Bool(w.mgr()?.is_rci_zone(i64_of(a(0)?, "a")? as i32)),
        ("manager", "development_failure_count") => {
            let zone = i64_of(a(0)?, "zone")? as i32;
            let result = i64_of(a(1)?, "result")? as i32;
            ji(w.mgr()?.development_failure_count(zone, result))
        }
        ("manager", "undeveloped_total") => ji(w.mgr()?.undeveloped_total()),
        ("manager", "abandoned_tile_count") => ji(w.mgr()?.abandoned_tile_count()),
        ("manager", "min_zone_size") => ji(w.mgr()?.min_zone_size(i64_of(a(0)?, "a")? as i32)),
        ("manager", "max_zone_size") => ji(w.mgr()?.max_zone_size(i64_of(a(0)?, "a")? as i32)),
        ("manager", "zone_display_alpha") => ji(w.mgr()?.zone_display_alpha()),
        ("manager", "default_zone_display_alpha") => ji(w.mgr()?.default_zone_display_alpha()),
        ("manager", "zone_drag_color") => ji(w.mgr()?.zone_drag_color(i64_of(a(0)?, "a")? as i32)),
        ("manager", "texture_for_zone") => match w.mgr()?.texture_for_zone(i64_of(a(0)?, "a")? as i32) {
            Some(k) => Json::Array(vec![ji(k.type_id), ji(k.group), ji(k.instance)]),
            None => Json::Null,
        },
        // ---- lot manager (§8) ----
        ("developer", "can_create_lot") => {
            let x = i64_of(a(0)?, "x")? as i32;
            let z = i64_of(a(1)?, "z")? as i32;
            let ww = i64_of(a(2)?, "w")? as i32;
            let h = i64_of(a(3)?, "h")? as i32;
            let znn = a(4).ok().and_then(boolean).unwrap_or(false);
            let wa = a(5).ok().and_then(boolean).unwrap_or(false);
            ji(w.dev()?.can_create_lot(x, z, ww, h, znn, wa))
        }
        ("developer", "create_lot_if_possible") => {
            let x = i64_of(a(0)?, "x")? as i32;
            let z = i64_of(a(1)?, "z")? as i32;
            let ww = i64_of(a(2)?, "w")? as i32;
            let h = i64_of(a(3)?, "h")? as i32;
            let facing = i64_of(a(4)?, "facing")? as i32;
            let res = w.dev()?.create_lot_if_possible_full(x, z, ww, h, facing, false, false);
            Json::Object(vec![("ok".into(), Json::Bool(res.ok)), ("lot".into(), res.lot.map_or(Json::Null, |l| ji(l as i64)))])
        }
        ("developer", "get_lot_count") => ji(w.dev()?.lot_count() as i64),
        ("developer", "get_random_lot") => match w.dev()?.get_random_lot() {
            Some(l) => ji(l as i64),
            None => Json::Null,
        },
        ("developer", "get_lots_in_cell_rect") => {
            let r = rect_of(a(0)?)?;
            let oo = a(1).ok().and_then(boolean).unwrap_or(false);
            let v: Vec<u64> = w.dev()?.get_lots_in_cell_rect(r, oo);
            Json::Array(v.iter().map(|&l| ji(l as i64)).collect())
        }
        ("developer", "exists_network_of_type") => {
            let x = i64_of(a(0)?, "x")? as i32;
            let z = i64_of(a(1)?, "z")? as i32;
            let f = u32_of(a(2)?, "flags")?;
            Json::Bool(w.dev()?.exists_network_of_type(x, z, f))
        }
        ("developer", "is_cell_water") => {
            let x = i64_of(a(0)?, "x")? as i32;
            let z = i64_of(a(1)?, "z")? as i32;
            Json::Bool(w.dev()?.is_cell_water(x, z))
        }
        ("developer", "get_occupant_lot") => {
            let o = u32_of(a(0)?, "occ")? as u64;
            match w.dev()?.get_occupant_lot(o) {
                Some(l) => ji(l as i64),
                None => Json::Null,
            }
        }
        ("developer", "lot_savvy_terrain_altitude") => {
            let x = f32_of(a(0)?, "x")?;
            let z = f32_of(a(1)?, "z")?;
            let (alt, ok) = w.dev()?.lot_savvy_terrain_altitude(x, z);
            Json::Object(vec![("alt".into(), scalar::jf(alt)), ("ok".into(), Json::Bool(ok))])
        }
        ("developer", "neighbor_cells_at_edge_of_lot") => {
            let r = rect_of(a(0)?)?;
            let f = u32_of(a(1)?, "flags")?;
            let (out, al) = w.dev()?.neighbor_cells_at_edge_of_lot(r, f);
            Json::Object(vec![("out".into(), rect_json(out)), ("al".into(), Json::Bool(al))])
        }
        ("developer", "delete_lots") => {
            let r = rect_of(a(0)?)?;
            w.dev()?.delete_lots(r);
            Json::Bool(true)
        }
        _ => return Err(format!("unknown call '{target}.{method}'")),
    };
    check_expect(op, got)
}

/// `zone_eval`: the pure helpers (`depth_rule`, `middle_row`, `zone_density`,
/// `block_depth`, `derive_seed`).
fn eval(op: &Json) -> R<()> {
    let f = str_of(field(op, "fn")?, "fn")?;
    let args = arr(field(op, "args")?, "args")?;
    let got: Json = match f {
        "depth_rule" => {
            // [rem, front, far, min, max, ld, ps]
            let rem = i64_of(field_at(args, 0)?, "rem")? as i32;
            let front = boolean(field_at(args, 1)?).unwrap_or(false);
            let far = boolean(field_at(args, 2)?).unwrap_or(false);
            let min = i64_of(field_at(args, 3)?, "min")? as i32;
            let max = i64_of(field_at(args, 4)?, "max")? as i32;
            let ld = i64_of(field_at(args, 5)?, "ld")? as i32;
            let ps = i64_of(field_at(args, 6)?, "ps")? as i32;
            ji(depth_rule(rem, front, far, min, max, ld, ps))
        }
        "depth_rule_row" => {
            // [front, far, min, max, ld, ps] → results for rem 1..12.
            let front = boolean(field_at(args, 0)?).unwrap_or(false);
            let far = boolean(field_at(args, 1)?).unwrap_or(false);
            let min = i64_of(field_at(args, 2)?, "min")? as i32;
            let max = i64_of(field_at(args, 3)?, "max")? as i32;
            let ld = i64_of(field_at(args, 4)?, "ld")? as i32;
            let ps = i64_of(field_at(args, 5)?, "ps")? as i32;
            Json::Array((1..=12).map(|rem| ji(depth_rule(rem, front, far, min, max, ld, ps))).collect())
        }
        "middle_row" => {
            let o0 = i64_of(field_at(args, 0)?, "o0")? as i32;
            let so = i64_of(field_at(args, 1)?, "so")? as i32;
            let b1 = i64_of(field_at(args, 2)?, "b1")? as i32;
            let b2 = i64_of(field_at(args, 3)?, "b2")? as i32;
            let lw = i64_of(field_at(args, 4)?, "lw")? as i32;
            ji(middle_row(o0, so, b1, b2, lw))
        }
        "zone_density" => ji(zone_density(i64_of(field_at(args, 0)?, "z")? as i32)),
        "block_depth" => {
            let d = i64_of(field_at(args, 0)?, "d")? as i32;
            let depth = i64_of(field_at(args, 1)?, "depth")? as i32;
            let res = boolean(field_at(args, 2)?).unwrap_or(false);
            ji(block_depth(d, depth, res))
        }
        "derive_seed" => {
            let session = u32_of(field_at(args, 0)?, "session")?;
            ji(derive_seed(session, RNG_SEED_XOR))
        }
        _ => return Err(format!("unknown fn '{f}'")),
    };
    check_expect(op, got)
}

fn expect(w: &World, op: &Json) -> R<()> {
    let state = field(op, "state")?;
    let mut got: Vec<(String, Json)> = Vec::new();
    if let Some(m) = w.manager.borrow().as_ref() {
        got.push(("manager_initialized".to_owned(), Json::Bool(m.is_initialised())));
        got.push(("manager_loaded".to_owned(), Json::Bool(m.is_loaded())));
    }
    if let Some(d) = w.developer.borrow().as_ref() {
        got.push(("developer_initialized".to_owned(), Json::Bool(d.is_initialised())));
        got.push(("developer_loaded".to_owned(), Json::Bool(d.is_loaded())));
        got.push(("lot_count".to_owned(), ji(d.lot_count() as i64)));
        got.push(("real_mode".to_owned(), Json::Bool(d.is_real_mode())));
        let o = d.orientation();
        got.push(("orientation_flag".to_owned(), Json::Bool(o.flag)));
        got.push(("orientation_d".to_owned(), ji(o.d)));
        got.push(("orientation_q".to_owned(), ji(o.q)));
        got.push(("scores".to_owned(), Json::Array(o.scores.iter().map(|&s| ji(s)).collect())));
        got.push(("conditions".to_owned(), Json::Array(o.conditions.iter().map(|&c| ji(c)).collect())));
        let ls = d.lot_size();
        got.push((
            "lot_size".to_owned(),
            Json::Object(vec![
                ("width".into(), ji(ls.width)),
                ("depth".into(), ji(ls.depth)),
                ("min_depth".into(), ji(ls.min_depth)),
                ("max_depth".into(), ji(ls.max_depth)),
                ("block".into(), ji(ls.block)),
            ]),
        ));
        got.push(("deferred".to_owned(), Json::Array(w.deferred.borrow().iter().map(|&b| Json::Bool(b)).collect())));
    }
    got.push(("funds".to_owned(), ji(w.funds.get())));
    got.push(("place_count".to_owned(), ji(w.street_place_count.get())));
    // The shared generator's state (§9): `lcg` and `next_raw` (a peek at the
    // next `NextRaw`, computed on a clone so it is not consumed).
    if let Some(st) = w.statics.borrow().as_ref() {
        let s = st.borrow();
        got.push(("lcg".to_owned(), ji(s.rng.lcg_state())));
        let mut probe = s.rng.clone();
        got.push(("next_raw".to_owned(), ji(probe.next_raw())));
        got.push((
            "draw_count".to_owned(),
            ji(s.trace.as_ref().map_or(0, |t| t.len()) as i64),
        ));
    }
    // Zone grid cells (from the manager's grid).
    if let Some(m) = w.manager.borrow().as_ref() {
        if let Some(g) = m.zone_grid() {
            if let Some(v) = op.get("zones") {
                for e in arr(v, "zones")? {
                    let a = arr(e, "z")?;
                    let x = i64_of(field_at(a, 0)?, "x")? as i32;
                    let z = i64_of(field_at(a, 1)?, "z")? as i32;
                    let want = i64_of(field_at(a, 2)?, "v")? as i32;
                    let gv = i32::from(g.get_cell_value(x, z));
                    if gv != want {
                        return Err(format!("zone ({x},{z}) = {gv}, expected {want}"));
                    }
                }
            }
        }
    }
    scalar::check_state(&got, state)
}

/// `zone_expect_parcel_log`: the interleaved log of lot creations
/// (`"(x1,z1)-(x2,z2) fN"`) and Place calls (`"P(x1,z1,x2,z2)"`), one to one
/// and in order (§15 tests 19–29). Clears the log unless `keep`.
fn expect_parcel_log(w: &World, op: &Json) -> R<()> {
    let got: Vec<Json> = w.parcel_log.borrow().clone();
    if !opt(op, "keep").and_then(boolean).unwrap_or(false) {
        w.parcel_log.borrow_mut().clear();
    }
    let want = arr(field(op, "log")?, "log")?;
    let show_got = || got.iter().map(show).collect::<Vec<_>>().join(", ");
    if got.len() != want.len() {
        return Err(format!("{} log entries, expected {}:\n  got      {}\n  expected {}",
            got.len(), want.len(), show_got(), want.iter().map(show).collect::<Vec<_>>().join(", ")));
    }
    for (i, (g, wnt)) in got.iter().zip(want).enumerate() {
        if !matches(g, wnt, false) {
            return Err(format!("log entry {i}: got {}, expected {}\n  full log: {}", show(g), show(wnt), show_got()));
        }
    }
    Ok(())
}

fn expect_draws(w: &World, op: &Json) -> R<()> {
    let st = w.statics.borrow().clone().ok_or("no statics (use zone_create)")?;
    let mut st = st.borrow_mut();
    let trace = st.trace.get_or_insert_with(Vec::new);
    let got: Vec<Draw> = trace.clone();
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
                let v = u32_of(field_at(a, 2)?, "value")?;
                g.kind.name() == k && g.arg == n && g.value == v
            }
            _ => return Err("draw entries are \"Zn\" or [\"Zn\", arg, value]".into()),
        };
        if !ok {
            return Err(format!("draw {i} differs: got {}", show_got()));
        }
    }
    Ok(())
}
