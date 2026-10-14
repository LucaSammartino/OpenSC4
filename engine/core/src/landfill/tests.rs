// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Unit tests for the landfill developer: every numbered conformance test of
//! `docs/specs/landfill.md` §11 (`test_NN_…`), plus the lifecycle and message
//! gating of §1 and §3.
//!
//! Unless a test says otherwise the world is the one §11 states: shipped
//! tuning (§4.1), `tileCap` = 30000, sea level 250, flat terrain at 270 m,
//! sanitation funding 100 %, every zone cell of zone type 14, no roads.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use super::*;
use crate::property::PropertyHolder;
use crate::stream::{ByteReader, ByteWriter, OutStream};

/// A recorded slot-21 call: (rect, f, withWater, waterFlag, maxAreaSlope, result).
type LevellingCall = (CellRect, f32, bool, bool, f32, bool);

/// The sanitation department's token.
const SANITATION: u64 = 0x5A;
/// Cells are 16 m.
const CELL: f32 = 16.0;

#[derive(Clone)]
struct Occ {
    kind: u32,
    x: i32,
    z: i32,
    tile: Option<Rc<LandfillOccupant>>,
}

/// A scripted city implementing [`LandfillHost`] and the lot developer's
/// levelling slots ([`LevellingSlots`]).
struct World {
    cells: Cell<(i32, i32)>,
    base_alt: Cell<f32>,
    alt: RefCell<BTreeMap<(i32, i32), f32>>,
    sea: Cell<f32>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    // Pollution.
    supply: Cell<u32>,
    tile_cap: Cell<u32>,
    uncollected: Cell<i32>,
    garbage_capacity: RefCell<Vec<i32>>,
    supply_deltas: RefCell<Vec<i32>>,
    put_in: RefCell<Vec<i32>>,
    active_landfill: RefCell<BTreeSet<(i32, i32)>>,
    // Budget.
    department: Cell<Option<u64>>,
    funding: Cell<f32>,
    current: Cell<i64>,
    full_expenses: RefCell<Vec<i64>>,
    // Zone manager.
    zone_types: RefCell<BTreeMap<(i32, i32), i32>>,
    default_zone: Cell<i32>,
    placed: RefCell<Vec<(CellRect, i32)>>,
    // Occupants and props.
    occupants: RefCell<BTreeMap<u64, Occ>>,
    next: Cell<u64>,
    reindexed: RefCell<Vec<u64>>,
    occupant_messages: RefCell<Vec<(u64, u32)>>,
    zots: RefCell<BTreeMap<u64, [f32; 3]>>,
    removed_props: RefCell<Vec<u64>>,
    released: RefCell<Vec<u64>>,
    // Traffic.
    connected: Cell<bool>,
    networks: RefCell<BTreeSet<(i32, i32)>>,
    count_calls: Cell<u32>,
    // Lot developer levelling (test 22).
    avg: Cell<f32>,
    slope: Cell<f32>,
    tuning_slope: Cell<f32>,
    levelling: RefCell<Vec<LevellingCall>>,
    modify: RefCell<Vec<ModifyTerrainHeight>>,
    // Misc.
    history: RefCell<Vec<(u32, u32)>>,
    redisplayed: RefCell<Vec<CellRect>>,
    view_level: Cell<u32>,
    seagull: RefCell<Vec<(u64, bool)>>,
    trucks: RefCell<Vec<(u64, bool)>>,
    agents: RefCell<Vec<String>>,
    subscriptions: RefCell<BTreeSet<u32>>,
}

impl World {
    fn new() -> Rc<World> {
        Rc::new(World {
            cells: Cell::new((64, 64)),
            base_alt: Cell::new(270.0),
            alt: RefCell::new(BTreeMap::new()),
            sea: Cell::new(250.0),
            tuning: RefCell::new(Some(Rc::new(shipped_tuning()))),
            supply: Cell::new(0),
            tile_cap: Cell::new(30000),
            uncollected: Cell::new(0),
            garbage_capacity: RefCell::new(Vec::new()),
            supply_deltas: RefCell::new(Vec::new()),
            put_in: RefCell::new(Vec::new()),
            active_landfill: RefCell::new(BTreeSet::new()),
            department: Cell::new(Some(SANITATION)),
            funding: Cell::new(1.0),
            current: Cell::new(0),
            full_expenses: RefCell::new(Vec::new()),
            zone_types: RefCell::new(BTreeMap::new()),
            default_zone: Cell::new(14),
            placed: RefCell::new(Vec::new()),
            occupants: RefCell::new(BTreeMap::new()),
            next: Cell::new(0x1000),
            reindexed: RefCell::new(Vec::new()),
            occupant_messages: RefCell::new(Vec::new()),
            zots: RefCell::new(BTreeMap::new()),
            removed_props: RefCell::new(Vec::new()),
            released: RefCell::new(Vec::new()),
            connected: Cell::new(false),
            networks: RefCell::new(BTreeSet::new()),
            count_calls: Cell::new(0),
            avg: Cell::new(272.0),
            slope: Cell::new(3.0),
            tuning_slope: Cell::new(SHIPPED_MAX_AREA_SLOPE),
            levelling: RefCell::new(Vec::new()),
            modify: RefCell::new(Vec::new()),
            history: RefCell::new(Vec::new()),
            redisplayed: RefCell::new(Vec::new()),
            view_level: Cell::new(1),
            seagull: RefCell::new(Vec::new()),
            trucks: RefCell::new(Vec::new()),
            agents: RefCell::new(Vec::new()),
            subscriptions: RefCell::new(BTreeSet::new()),
        })
    }

    fn token(&self) -> u64 {
        let t = self.next.get();
        self.next.set(t + 1);
        t
    }

    fn set_alt(&self, x: i32, z: i32, a: f32) {
        self.alt.borrow_mut().insert((x, z), a);
    }

    fn altitude(&self, x: i32, z: i32) -> f32 {
        self.alt.borrow().get(&(x, z)).copied().unwrap_or(self.base_alt.get())
    }

    fn occupant_of(&self, x: i32, z: i32, kind: u32) -> Option<(u64, Occ)> {
        self.occupants
            .borrow()
            .iter()
            .find(|(_, o)| o.kind == kind && o.x == x && o.z == z)
            .map(|(t, o)| (*t, o.clone()))
    }

    /// The landfill tile at a cell: (tile, orientation, y).
    fn tile_at(&self, x: i32, z: i32) -> Option<(u32, i32, f32)> {
        let (_, o) = self.occupant_of(x, z, OCCUPANT_TYPE_LANDFILL)?;
        let t = o.tile?;
        Some((t.tile_id(), t.orientation(), t.position()[1]))
    }

    fn add_base_texture(&self, x: i32, z: i32) {
        let t = self.token();
        self.occupants.borrow_mut().insert(
            t,
            Occ {
                kind: OCCUPANT_TYPE_BASE_TEXTURE,
                x,
                z,
                tile: None,
            },
        );
    }
}

impl LevellingSlots for World {
    fn average_area_altitude(&self, _rect: CellRect, _with_water: bool) -> f32 {
        self.avg.get()
    }
    fn area_slope(&self, _rect: CellRect, _with_water: bool) -> f32 {
        self.slope.get()
    }
    fn max_area_slope_allowed_for_levelling(&self) -> f32 {
        self.tuning_slope.get()
    }
    fn modify_terrain_height(&self, call: ModifyTerrainHeight) -> bool {
        self.modify.borrow_mut().push(call);
        true
    }
}

impl LandfillHost for World {
    fn acquire_services(&self) {}
    fn release_services(&self) {}
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        self.agents.borrow_mut().push(format!("{name}/{level}/{flags}"));
    }
    fn remove_agent(&self) {
        self.agents.borrow_mut().clear();
    }
    fn subscribe(&self, msg_type: u32) {
        self.subscriptions.borrow_mut().insert(msg_type);
    }
    fn unsubscribe(&self, msg_type: u32) {
        self.subscriptions.borrow_mut().remove(&msg_type);
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn exemplar(&self, type_id: u32, group: u32, instance: u32) -> Option<Rc<PropertyHolder>> {
        if (type_id, group) != (tile::TYPE, tile::GROUP) {
            return None;
        }
        shipped_tile_exemplar(instance).map(Rc::new)
    }
    fn cell_counts(&self) -> (i32, i32) {
        self.cells.get()
    }
    fn cell_center_to_position(&self, x: i32, z: i32) -> (f32, f32) {
        (x as f32 * CELL + CELL / 2.0, z as f32 * CELL + CELL / 2.0)
    }
    fn cell_altitude(&self, x: i32, z: i32) -> f32 {
        self.altitude(x, z)
    }
    fn altitude_at_nearest_grid(&self, px: f32, pz: f32) -> f32 {
        self.altitude((px / CELL).floor() as i32, (pz / CELL).floor() as i32)
    }
    fn cell_altitude_at_nearest_grid(&self, x: i32, z: i32) -> f32 {
        self.altitude(x, z)
    }
    fn sea_level(&self) -> f32 {
        self.sea.get()
    }
    fn set_altitude(&self, _rect: CellRect, _altitude: f32) {}
    fn redisplay_terrain(&self, rect: CellRect) {
        self.redisplayed.borrow_mut().push(rect);
    }
    fn garbage_supply(&self) -> u32 {
        self.supply.get()
    }
    fn landfill_tile_capacity(&self) -> u32 {
        self.tile_cap.get()
    }
    fn total_garbage_uncollected(&self) -> i32 {
        self.uncollected.get()
    }
    fn add_to_garbage_supply(&self, delta: i32) {
        self.supply_deltas.borrow_mut().push(delta);
        self.supply.set(self.supply.get().wrapping_add(delta as u32));
    }
    fn set_garbage_capacity(&self, available: i32) {
        self.garbage_capacity.borrow_mut().push(available);
    }
    fn set_is_active_landfill(&self, x: i32, z: i32, on: bool) {
        let mut s = self.active_landfill.borrow_mut();
        if on {
            s.insert((x, z));
        } else {
            s.remove(&(x, z));
        }
    }
    fn garbage_put_in_landfill(&self, n: i32) {
        self.put_in.borrow_mut().push(n);
    }
    fn get_department_budget(&self, department: u32) -> Option<u64> {
        (department == budget_ids::SANITATION_DEPARTMENT)
            .then_some(())
            .and(self.department.get())
    }
    fn add_ref(&self, _token: u64) {}
    fn release(&self, token: u64) {
        self.released.borrow_mut().push(token);
    }
    fn department_funding_percentage(&self, _department: u64) -> f32 {
        self.funding.get()
    }
    fn line_item_current_expenses(&self, _department: u64, line: u32) -> Option<i64> {
        (line == budget_ids::LANDFILL_LINE).then_some(self.current.get())
    }
    fn set_line_item_full_expenses(&self, _department: u64, _line: u32, value: i64) {
        self.full_expenses.borrow_mut().push(value);
    }
    fn occupant_item_cost(&self, occupant: u64, property: u32) -> Option<i64> {
        assert_eq!(property, PROP_ITEM_COST);
        let o = self.occupants.borrow().get(&occupant).cloned()?;
        let id = o.tile?.tile_id();
        shipped_tile_costs().into_iter().find(|(t, _)| *t == id).map(|(_, c)| c)
    }
    fn zone_count(&self, zone_type: i32) -> i32 {
        self.zone_types.borrow().values().filter(|&&v| v == zone_type).count() as i32
    }
    fn zone_type(&self, x: i32, z: i32) -> i32 {
        self.zone_types
            .borrow()
            .get(&(x, z))
            .copied()
            .unwrap_or(self.default_zone.get())
    }
    fn place_zone(&self, region: &CellRegion, zone_type: i32) -> bool {
        self.placed.borrow_mut().push((region.rect(), zone_type));
        for (x, z) in region.mask_cells() {
            self.zone_types.borrow_mut().insert((x, z), zone_type);
        }
        true
    }
    fn occupant_at(&self, x: i32, z: i32, occupant_type: u32) -> Option<u64> {
        self.occupant_of(x, z, occupant_type).map(|(t, _)| t)
    }
    fn insert_landfill_occupant(&self, occupant: &Rc<LandfillOccupant>, x: i32, z: i32) -> bool {
        let t = self.token();
        occupant.set_token(t);
        self.occupants.borrow_mut().insert(
            t,
            Occ {
                kind: OCCUPANT_TYPE_LANDFILL,
                x,
                z,
                tile: Some(occupant.clone()),
            },
        );
        true
    }
    fn occupant_remove(&self, occupant: u64) -> bool {
        self.occupants.borrow_mut().remove(&occupant).is_some()
    }
    fn occupant_changed(&self, occupant: u64) {
        self.reindexed.borrow_mut().push(occupant);
    }
    fn occupant_position(&self, occupant: u64) -> [f32; 3] {
        let o = self.occupants.borrow().get(&occupant).cloned().unwrap();
        o.tile.unwrap().position()
    }
    fn occupant_set_position(&self, occupant: u64, position: [f32; 3]) {
        let o = self.occupants.borrow().get(&occupant).cloned().unwrap();
        o.tile.unwrap().set_position(position);
    }
    fn occupant_set_removal_flags(&self, _occupant: u64, _flags: u32) {}
    fn occupant_post_message(&self, occupant: u64, message: u32) {
        self.occupant_messages.borrow_mut().push((occupant, message));
    }
    fn is_landfill_occupant(&self, occupant: u64) -> bool {
        self.occupants.borrow().get(&occupant).is_some_and(|o| o.tile.is_some())
    }
    fn is_empty_landfill_tile(&self, occupant: u64) -> bool {
        let o = self.occupants.borrow().get(&occupant).cloned().unwrap();
        o.tile.unwrap().is_empty_landfill_tile()
    }
    fn remove_incompatible_occupants(&self, x: i32, z: i32) {
        self.occupants
            .borrow_mut()
            .retain(|_, o| !(o.kind == OCCUPANT_TYPE_FLORA && o.x == x && o.z == z));
    }
    fn set_empty_cell_texture(&self, x: i32, z: i32) {
        self.add_base_texture(x, z);
    }
    fn remove_base_texture(&self, x: i32, z: i32) {
        self.occupants
            .borrow_mut()
            .retain(|_, o| !(o.kind == OCCUPANT_TYPE_BASE_TEXTURE && o.x == x && o.z == z));
    }
    fn add_lot_prop(&self, prop_type: u32, position: [f32; 3], _orientation: i32) -> Option<u64> {
        assert_eq!(prop_type, ZOT_PROP);
        let t = self.token();
        self.zots.borrow_mut().insert(t, position);
        Some(t)
    }
    fn remove_prop(&self, occupant: u64) -> bool {
        self.removed_props.borrow_mut().push(occupant);
        self.zots.borrow_mut().remove(&occupant).is_some()
    }
    fn zot_orientation(&self) -> i32 {
        0
    }
    fn find_network_tile(&self, x: i32, z: i32, flag_mask: u32, flag: bool) -> Option<u64> {
        assert_eq!((flag_mask, flag), (NETWORK_TILE_MASK, true));
        self.networks.borrow().contains(&(x, z)).then_some(1)
    }
    fn subnetworks_in_region(&self, _region: &CellRegion) -> Vec<u32> {
        if self.connected.get() {
            vec![1]
        } else {
            Vec::new()
        }
    }
    fn occupant_count_for_all_subnetworks(&self, developer_type: u32) -> Vec<u32> {
        self.count_calls.set(self.count_calls.get() + 1);
        if developer_type == 0x1010 {
            vec![0, 5]
        } else {
            Vec::new()
        }
    }
    fn attempt_area_levelling(
        &self,
        rect: CellRect,
        f: f32,
        with_water: bool,
        water_flag: bool,
        max_area_slope: f32,
    ) -> bool {
        let r = attempt_area_levelling(self, rect, f, with_water, water_flag, max_area_slope, 0);
        self.levelling
            .borrow_mut()
            .push((rect, f, with_water, water_flag, max_area_slope, r));
        r
    }
    fn current_view_level(&self) -> u32 {
        self.view_level.get()
    }
    fn init_zone_effects(&self, _centre: (i32, i32), _rect: CellRect) -> ZoneEffects {
        ZoneEffects {
            truck: Some(self.token()),
            seagull: Some(self.token()),
            seagull_started: false,
        }
    }
    fn shutdown_zone_effects(&self, _effects: ZoneEffects) {}
    fn set_truck_enabled(&self, truck: u64, enabled: bool) {
        self.trucks.borrow_mut().push((truck, enabled));
    }
    fn start_seagull(&self, seagull: u64, flag: u32) {
        assert_eq!(flag, 0);
        self.seagull.borrow_mut().push((seagull, true));
    }
    fn stop_seagull(&self, seagull: u64, flag: u32) {
        assert_eq!(flag, 1);
        self.seagull.borrow_mut().push((seagull, false));
    }
    fn send_history(&self, series: u32, value: u32) {
        self.history.borrow_mut().push((series, value));
    }
    fn occupant_ref(&self, occupant: u64) -> RefValue {
        RefValue::Host {
            token: occupant,
            class_id: IID_OCCUPANT,
        }
    }
    fn resolve_occupant(&self, r: &RefValue) -> Option<u64> {
        match r {
            RefValue::Host { token, .. } => Some(*token),
            _ => None,
        }
    }
}

// ---- helpers ------------------------------------------------------------------------------------

/// A developer after `Init` and PostCityInit (the department is held).
fn developer(w: &Rc<World>) -> Rc<LandfillDeveloper> {
    let d = LandfillDeveloper::new(w.clone(), 1);
    assert!(d.init());
    d.do_message(&LandfillMessage::of_type(msg::POST_CITY_INIT));
    d
}

fn rect(x1: i32, z1: i32, x2: i32, z2: i32) -> CellRect {
    CellRect::new(x1, z1, x2, z2)
}

/// Zones `r` as type `zone_type` (the `0x07699455` message, §3.2).
fn zone(d: &LandfillDeveloper, r: CellRect, zone_type: i32) {
    let mut m = LandfillMessage::of_type(msg::ZONING_CHANGE);
    m.data3 = zone_type as u32;
    m.region = Some(CellRegion::all_set(r));
    assert!(d.do_message(&m));
}

fn month(d: &LandfillDeveloper) {
    assert!(d.do_message(&LandfillMessage::of_type(msg::SIM_NEW_MONTH)));
}

/// `0x278E6003` over one cell rect with `data4 = reason` (§3.2): `data1 =
/// ((x0 + 1) << 16) | x1`, `data2 = ((z0 + 1) << 16) | z1`.
fn terrain(d: &LandfillDeveloper, r: CellRect, reason: u32) {
    let mut m = LandfillMessage::of_type(msg::TERRAIN_MODIFIED);
    m.data1 = (((r.x1 + 1) as u32) << 16) | r.x2 as u32;
    m.data2 = (((r.z1 + 1) as u32) << 16) | r.z2 as u32;
    m.data4 = reason;
    assert!(d.do_message(&m));
}

fn only_zone(d: &LandfillDeveloper) -> Rc<LandfillZone> {
    let zones = d.zone_list();
    assert_eq!(zones.len(), 1, "expected exactly one zone");
    zones[0].clone()
}

fn heap(hx: u32, hz: u32, valid: bool, active: bool, capacity: u32, contained: u32) -> Heap {
    Heap {
        hx,
        hz,
        valid,
        active,
        capacity,
        contained,
    }
}

// ---- §11 conformance tests ------------------------------------------------------------------------

/// Test 1: heap capacity. 4×4 → 16 cells, `W = H = 3`, 9 valid heaps,
/// `c = round(480000 / 9) = 53333`, `totalCapacity = 479997`; 2×2 → one heap
/// of 120000; 1×5 → `W = 0`, no heaps, capacity 0, and `GetLandfillUsage`
/// gets nothing from it.
#[test]
fn test_01_heap_capacity() {
    let w = World::new();
    let d = developer(&w);
    zone(&d, rect(10, 10, 13, 13), 14);
    let z = only_zone(&d);
    assert_eq!((z.w(), z.h(), z.cell_count(), z.num_valid_heaps()), (3, 3, 16, 9));
    assert!(z.heaps().iter().all(|h| h.capacity == 53333));
    assert_eq!(z.recalculate_capacity(30000), 479_997);

    let w = World::new();
    let d = developer(&w);
    zone(&d, rect(10, 10, 11, 11), 14);
    let z = only_zone(&d);
    assert_eq!(z.heaps().len(), 1);
    assert_eq!(z.heaps()[0].capacity, 120_000);

    let w = World::new();
    w.connected.set(true);
    let d = developer(&w);
    zone(&d, rect(10, 10, 10, 14), 14);
    let z = only_zone(&d);
    assert_eq!((z.w(), z.h(), z.heaps().len()), (0, 4, 0));
    assert_eq!(z.get_capacity(), (0, 0, 0));
    w.supply.set(10_000);
    month(&d);
    assert!(z.connected());
    assert_eq!(d.get_landfill_usage(), (0, 0));
    assert_eq!(w.garbage_capacity.borrow().last(), Some(&0));
}

/// Test 2: zoning cost on flat ground: variance 0 ≤ 5 → 0, high dword 0.
#[test]
fn test_02_zoning_cost_flat() {
    let w = World::new();
    let d = developer(&w);
    let cost = d.get_zoning_cost(&CellRegion::all_set(rect(10, 10, 13, 13)));
    assert_eq!(cost, 0);
    assert_eq!(cost >> 32, 0);
}

/// Test 3: 2×2 region with cell-centre altitudes 270, 272, 280, 290 → min
/// 270, max 290, variance 20 > 5, mean 278, `Σ|278 − a| = 28`, cost 112.
#[test]
fn test_03_zoning_cost_slope() {
    let w = World::new();
    w.set_alt(20, 20, 270.0);
    w.set_alt(20, 21, 272.0);
    w.set_alt(21, 20, 280.0);
    w.set_alt(21, 21, 290.0);
    let d = developer(&w);
    let v = altitude_variance(rect(20, 20, 21, 21), 250.0, |x, z| w.altitude(x, z));
    assert_eq!((v.min, v.max, v.variance, v.mean), (270.0, 290.0, 20.0, 278.0));
    let cost = d.get_zoning_cost(&CellRegion::all_set(rect(20, 20, 21, 21)));
    assert_eq!(cost, 112);
    assert_eq!(cost >> 32, 0);
}

/// Test 4: a region intersecting a zone whose `levelAltitude` is 275, over
/// cells at 270 and 280 → no tolerance test, cost `round(4 × 10) = 40`.
#[test]
fn test_04_zoning_cost_extension() {
    let w = World::new();
    let d = developer(&w);
    zone(&d, rect(10, 10, 11, 11), 14);
    only_zone(&d).set_level_altitude(275.0);
    w.set_alt(12, 10, 280.0);
    // Cells (11, 10) at 270 and (12, 10) at 280; the region touches the zone.
    let cost = d.get_zoning_cost(&CellRegion::all_set(rect(11, 10, 12, 10)));
    assert_eq!(cost, 40);
    // Without the zone the variance (10 > 5) and the mean (275) give the same
    // price, so a flat-ish region proves the tolerance test is skipped: a
    // variance of 2 would cost 0 on its own but costs 8 against 275.
    w.set_alt(12, 10, 272.0);
    let cost = d.get_zoning_cost(&CellRegion::all_set(rect(11, 10, 12, 10)));
    assert_eq!(cost, i64::from(magic_round(4.0 * 8.0)));
}

/// Test 5: the sea-level clamp. Centre `(x0, z0)` 251, `(x0+1, z0)` 240
/// (below sea 250, counts as 251), `(x0, z0+1)` 260, `(x0+1, z0+1)` 262 → min
/// 251, max 262, variance 11, mean 256; the cost uses raw altitudes:
/// `5 + 16 + 4 + 6 = 31`, cost 124.
#[test]
fn test_05_sea_level_clamp() {
    let w = World::new();
    w.set_alt(30, 30, 251.0);
    w.set_alt(31, 30, 240.0);
    w.set_alt(30, 31, 260.0);
    w.set_alt(31, 31, 262.0);
    let d = developer(&w);
    let v = altitude_variance(rect(30, 30, 31, 31), 250.0, |x, z| w.altitude(x, z));
    assert_eq!((v.min, v.max, v.variance, v.mean), (251.0, 262.0, 11.0, 256.0));
    assert_eq!(d.get_zoning_cost(&CellRegion::all_set(rect(30, 30, 31, 31))), 124);
}

/// Test 6: first intake of a connected 2×2 zone created this month.
#[test]
fn test_06_first_intake() {
    let w = World::new();
    w.connected.set(true);
    w.uncollected.set(7);
    let d = developer(&w);
    zone(&d, rect(10, 20, 11, 21), 14);
    // §4.4.1 step 3: a new zone does not book its maintenance.
    assert_eq!(d.get_maintenance_costs(), 0);
    assert!(w.full_expenses.borrow().is_empty());
    w.supply.set(10_000);
    month(&d);
    let z = only_zone(&d);
    assert_eq!(z.heaps()[0].contained, 10_000);
    assert!(z.heaps()[0].active);
    assert_eq!(*w.supply_deltas.borrow(), [-10_000]);
    assert_eq!(*w.put_in.borrow(), [10_000]);
    // Cells (0,0) mask 4 → corner or. 1; (1,0) mask 2 → or. 2; (0,1) mask 8
    // → or. 0; (1,1) mask 1 → or. 3.
    assert_eq!(w.tile_at(10, 20), Some((tile::CORNER, 1, 270.0)));
    assert_eq!(w.tile_at(11, 20), Some((tile::CORNER, 2, 270.0)));
    assert_eq!(w.tile_at(10, 21), Some((tile::CORNER, 0, 270.0)));
    assert_eq!(w.tile_at(11, 21), Some((tile::CORNER, 3, 270.0)));
    // No rand() draw.
    assert_eq!(d.crt(), CrtRand::default());
    assert_eq!(*w.garbage_capacity.borrow(), [110_000]);
    assert_eq!(d.get_maintenance_costs(), 40);
    assert_eq!(*w.full_expenses.borrow(), [40]);
    assert_eq!(
        *w.history.borrow(),
        [(history::CAPACITY, 120_000), (history::TOTAL_GARBAGE, 10_007)]
    );
    // The landfill flag is set under the four tiles (§4.8.2).
    assert_eq!(w.active_landfill.borrow().len(), 4);
}

/// Test 7: decay. 10000 → `round(700)` → 9300; 5 → `round(0.35) = 0` → 1 →
/// 4; 40000 → 2800 → capped to 1500 → 38500.
#[test]
fn test_07_decay() {
    let w = World::new();
    let d = developer(&w);
    zone(&d, rect(10, 10, 12, 12), 14);
    let z = only_zone(&d);
    let t = d.tuning();
    for (i, c) in [10_000u32, 5, 40_000].into_iter().enumerate() {
        z.set_heap(i, heap(i as u32 % 2, i as u32 / 2, true, true, 67_500, c));
        z.decay_garbage(i, t.decay_rate, t.max_decay_per_zone);
    }
    let got: Vec<u32> = z.heaps().iter().take(3).map(|h| h.contained).collect();
    assert_eq!(got, [9300, 4, 38_500]);
}

/// Test 8: underfunding. Maintenance 40, current 20, supply 10000 →
/// `t = trunc(0.5 × 10000) = 5000`.
#[test]
fn test_08_underfunding() {
    let w = World::new();
    w.connected.set(true);
    let d = developer(&w);
    zone(&d, rect(10, 20, 11, 21), 14);
    w.supply.set(10_000);
    month(&d);
    assert_eq!(d.get_maintenance_costs(), 40);
    w.current.set(20);
    w.supply.set(10_000);
    w.put_in.borrow_mut().clear();
    month(&d);
    assert_eq!(*w.put_in.borrow(), [5000]);
    // 10000 decayed to 9300, plus 5000.
    assert_eq!(only_zone(&d).heaps()[0].contained, 14_300);
}

/// Test 9: the per-tile cap. Supply 50000 → zone budget `4 × 5000 = 20000` →
/// contained 20000, supply 30000.
#[test]
fn test_09_per_tile_cap() {
    let w = World::new();
    w.connected.set(true);
    let d = developer(&w);
    zone(&d, rect(10, 20, 11, 21), 14);
    w.supply.set(50_000);
    month(&d);
    assert_eq!(only_zone(&d).heaps()[0].contained, 20_000);
    assert_eq!(w.supply.get(), 30_000);
}

/// Test 10: the global cap. Code-default `kMaxGarbageToTakePerMonth` 40000
/// and supply 100000 → `t = 40000`.
#[test]
fn test_10_global_cap() {
    let w = World::new();
    *w.tuning.borrow_mut() = None;
    w.connected.set(true);
    let d = developer(&w);
    assert_eq!(d.tuning().max_per_month, 40_000);
    // A 3×3 zone: zone budget 9 × 5000 = 45000 > 40000; heap capacity 67500.
    zone(&d, rect(10, 10, 12, 12), 14);
    w.supply.set(100_000);
    month(&d);
    assert_eq!(*w.put_in.borrow(), [40_000, 0, 0, 0]);
    assert_eq!(w.supply.get(), 60_000);
}

/// Test 11: a disconnected zone takes nothing, still decays, gets a zot
/// `0x107A0000` at the first mask cell spiralling from the centre, 3.0 m
/// above the terrain, and counts for nothing; connecting removes the zot at
/// the next month.
#[test]
fn test_11_disconnected() {
    let w = World::new();
    let d = developer(&w);
    zone(&d, rect(10, 20, 12, 22), 14);
    let z = only_zone(&d);
    z.set_heap(0, heap(0, 0, true, true, 67_500, 10_000));
    w.supply.set(50_000);
    month(&d);
    assert!(!z.connected());
    assert_eq!(z.heaps()[0].contained, 9300);
    assert!(w.put_in.borrow().iter().all(|&n| n == 0));
    assert_eq!(w.supply.get(), 50_000);
    // The centre cell (11, 21) is a mask cell: x = 11·16 + 8, z = 21·16 + 8.
    let zot = z.zot().expect("a zot");
    assert_eq!(w.zots.borrow().get(&zot), Some(&[184.0, 273.0, 344.0]));
    assert_eq!(d.state().capacity_total, 0);
    assert_eq!(d.state().used_total, 0);
    assert_eq!(*w.garbage_capacity.borrow(), [0]);
    // A second disconnected month keeps the one zot.
    month(&d);
    assert_eq!(z.zot(), Some(zot));
    // Connected: the zot is removed and released, and the zone counts again.
    w.connected.set(true);
    month(&d);
    assert_eq!(z.zot(), None);
    assert_eq!(*w.removed_props.borrow(), [zot]);
    assert!(w.released.borrow().contains(&zot));
    assert_eq!(d.state().capacity_total, 270_000);
}

/// LG13 (§4.4.9, `0x006BB766`–`0x006BB798`): an initialised zone's
/// `Shutdown` removes its zot through the prop manager (`RemovePropA`) and
/// then releases it. The developer's `Shutdown` runs only once, and only
/// when initialised (§1.2).
#[test]
fn shutdown_removes_the_zot_prop_then_releases_it() {
    let w = World::new();
    let d = developer(&w);
    zone(&d, rect(10, 20, 12, 22), 14);
    let z = only_zone(&d);
    assert!(z.is_initialised());
    month(&d);
    let zot = z.zot().expect("a zot");
    assert!(w.removed_props.borrow().is_empty());
    assert!(d.shutdown());
    assert_eq!(*w.removed_props.borrow(), [zot]);
    assert_eq!(w.released.borrow().iter().filter(|&&t| t == zot).count(), 1);
    assert!(w.zots.borrow().is_empty());
    assert_eq!(d.zone_count(), 0);
    assert!(!d.is_initialised());
    // A second Shutdown does nothing.
    let released = w.released.borrow().len();
    assert!(d.shutdown());
    assert_eq!(w.released.borrow().len(), released);
    // A developer never initialised: Shutdown returns true at once.
    let w2 = World::new();
    let d2 = LandfillDeveloper::new(w2.clone(), 2);
    assert!(d2.shutdown());
    assert!(w2.released.borrow().is_empty());
}

/// Test 11 (spiral, §4.7.5 example 1, LF4): a 3 × 3 zone over
/// `(10..12, 20..22)` with its centre `(11, 21)` dezoned. Ring 1 starts at
/// `(cx+1, cz)`, so the zot goes to `(12, 21)` (a top-left-corner order would
/// give `(10, 20)`).
#[test]
fn test_11_zot_spiral_skips_a_cleared_centre() {
    let w = World::new();
    let d = developer(&w);
    zone(&d, rect(10, 20, 12, 22), 14);
    let z = only_zone(&d);
    let mut region = z.region();
    region.set(11, 21, false);
    z.set_region(region);
    month(&d);
    let zot = z.zot().unwrap();
    assert_eq!(
        w.zots.borrow().get(&zot),
        Some(&[12.0 * 16.0 + 8.0, 273.0, 21.0 * 16.0 + 8.0])
    );
}

/// Test 11 (spiral, §4.7.5 example 2, LF4): a zone over `x 10..14, z 20..22`
/// with only `(10, 20)` and `(14, 22)` set. Ring 1 has no set cell; ring 2
/// clipped to the rect is `(14, 20)`, `(14, 21)`, `(14, 22)`, `(10, 22)`,
/// `(10, 21)`, `(10, 20)`, so the zot goes to `(14, 22)`.
#[test]
fn test_11_zot_spiral_clipped_ring_two() {
    let w = World::new();
    let d = developer(&w);
    zone(&d, rect(10, 20, 14, 22), 14);
    let z = only_zone(&d);
    let mut region = z.region();
    for x in 10..=14 {
        for zz in 20..=22 {
            region.set(x, zz, (x, zz) == (10, 20) || (x, zz) == (14, 22));
        }
    }
    z.set_region(region);
    month(&d);
    let zot = z.zot().unwrap();
    assert_eq!(
        w.zots.borrow().get(&zot),
        Some(&[14.0 * 16.0 + 8.0, 273.0, 22.0 * 16.0 + 8.0])
    );
}

/// Test 12: random draws. A 3×3 zone (2×2 heaps) with heaps (0,0), (1,0),
/// (0,1) active; heap (1,1) becomes active → its redraw visits (1,1), (2,1),
/// (1,2), (2,2) and only (1,1) has mask 15 → exactly one `rand()`, tile
/// `0x02FA0000`, orientation `2 × (41 & 3) = 2` with the CRT at state 1.
/// Heap (0,0) becoming active first draws nothing.
#[test]
fn test_12_random_draws() {
    let w = World::new();
    let d = developer(&w);
    zone(&d, rect(10, 20, 12, 22), 14);
    let z = only_zone(&d);
    let c = z.heaps()[0].capacity;
    z.set_heap(0, heap(0, 0, true, true, c, 1));
    z.set_heap(1, heap(1, 0, true, true, c, 1));
    z.set_heap(2, heap(0, 1, true, true, c, 1));
    z.set_heap(3, heap(1, 1, true, true, c, 1));
    let mut rng = CrtRand::default();
    z.update_surrounding_tiles(3, &mut rng);
    // Exactly one draw: the state moved once from 1.
    let mut once = CrtRand::default();
    assert_eq!(once.rand(), 41);
    assert_eq!(rng, once);
    assert_eq!(w.tile_at(11, 21), Some((tile::INTERIOR, 2, 270.0)));
    assert_eq!(w.tile_at(12, 21), Some((tile::EDGE, 3, 270.0)));
    assert_eq!(w.tile_at(11, 22), Some((tile::EDGE, 0, 270.0)));
    assert_eq!(w.tile_at(12, 22), Some((tile::CORNER, 3, 270.0)));

    // The same zone with heap (0,0) becoming active first: no draw.
    let w = World::new();
    let d = developer(&w);
    zone(&d, rect(10, 20, 12, 22), 14);
    let z = only_zone(&d);
    z.set_heap(0, heap(0, 0, true, true, c, 1));
    let mut rng = CrtRand::default();
    z.update_surrounding_tiles(0, &mut rng);
    assert_eq!(rng, CrtRand::default());

    // In play heaps fill in vector order: a 3×3 zone takes at most 45000 a
    // month and a heap holds 67500, so the first month turns one heap active
    // and draws nothing.
    let w = World::new();
    w.connected.set(true);
    let d = developer(&w);
    zone(&d, rect(10, 20, 12, 22), 14);
    w.supply.set(1_000_000);
    month(&d);
    let z = only_zone(&d);
    let active: Vec<bool> = z.heaps().iter().map(|h| h.active).collect();
    assert_eq!(active, [true, false, false, false]);
    assert_eq!(d.crt(), CrtRand::default());
}

/// Test 13: redistribution. Three valid heaps of capacity 100 with 100, 50,
/// 0 (total 150) → share 100 → 100, 50, 0.
#[test]
fn test_13_redistribution() {
    let w = World::new();
    let d = developer(&w);
    zone(&d, rect(10, 10, 13, 11), 14);
    let z = only_zone(&d);
    assert_eq!((z.w(), z.h()), (3, 1));
    z.set_heaps(vec![
        heap(0, 0, true, true, 100, 100),
        heap(1, 0, true, true, 100, 50),
        heap(2, 0, true, false, 100, 0),
    ]);
    z.redistribute_garbage();
    let got: Vec<u32> = z.heaps().iter().map(|h| h.contained).collect();
    assert_eq!(got, [100, 50, 0]);
    // The other order spreads it: 0, 50, 100 → 100, 50, 0 as well.
    z.set_heaps(vec![
        heap(0, 0, true, false, 100, 0),
        heap(1, 0, true, true, 100, 50),
        heap(2, 0, true, true, 100, 100),
    ]);
    z.redistribute_garbage();
    let got: Vec<u32> = z.heaps().iter().map(|h| h.contained).collect();
    assert_eq!(got, [100, 50, 0]);
}

/// Test 14: dezoning one corner cell of a full 3×3 zone invalidates the
/// corner heap; `RecalculateCapacity` with 3 valid heaps gives
/// `c = round(240000 / 3) = 80000`, and the invalid heap's garbage goes back
/// to the supply.
#[test]
fn test_14_spill_on_dezone() {
    let w = World::new();
    let d = developer(&w);
    zone(&d, rect(10, 10, 12, 12), 14);
    let z = only_zone(&d);
    for i in 0..4 {
        let h = z.heaps()[i];
        z.set_heap(i, Heap { contained: 67_500, ..h });
    }
    zone(&d, rect(10, 10, 10, 10), 0);
    let caps: Vec<(bool, u32, u32)> = z.heaps().iter().map(|h| (h.valid, h.capacity, h.contained)).collect();
    assert_eq!(
        caps,
        [
            (false, 0, 0),
            (true, 80_000, 67_500),
            (true, 80_000, 67_500),
            (true, 80_000, 67_500)
        ]
    );
    assert_eq!(*w.supply_deltas.borrow(), [67_500]);
    // `changeZone = false`: the zone manager already rezoned the cell.
    assert!(w.placed.borrow().is_empty());
    assert!(!z.is_mask_cell(10, 10));
    assert_eq!(*w.redisplayed.borrow(), [rect(10, 10, 10, 10)]);
}

/// Test 15: dezoning the middle column of a 3×3 zone leaves two 1×3 strips
/// with no valid heap → sticky flag → the remaining 6 cells are rezoned to
/// type 0 and the zone is removed.
#[test]
fn test_15_narrow_remainder() {
    let w = World::new();
    let d = developer(&w);
    zone(&d, rect(10, 10, 12, 12), 14);
    zone(&d, rect(11, 10, 11, 12), 0);
    assert_eq!(d.zone_count(), 0);
    let placed: Vec<(CellRect, i32)> = w.placed.borrow().clone();
    let want: Vec<(CellRect, i32)> = [(10, 10), (10, 11), (10, 12), (12, 10), (12, 11), (12, 12)]
        .iter()
        .map(|&(x, z)| (rect(x, z, x, z), 0))
        .collect();
    assert_eq!(placed, want);
    // No re-entrancy: delivering the posted zoning messages afterwards finds no
    // zone and changes nothing.
    for (r, t) in want {
        zone(&d, r, t);
    }
    assert_eq!(d.zone_count(), 0);
}

/// Test 16: zoning a 2×2 landfill diagonally touching an existing 2×2 zone
/// extends that zone: the union rect, the 8 mask cells, the heaps rebuilt.
#[test]
fn test_16_adjacency_merge() {
    let w = World::new();
    let d = developer(&w);
    zone(&d, rect(10, 10, 11, 11), 14);
    zone(&d, rect(12, 12, 13, 13), 14);
    let z = only_zone(&d);
    assert_eq!(z.rect(), rect(10, 10, 13, 13));
    assert_eq!(z.cell_count(), 8);
    assert_eq!((z.w(), z.h()), (3, 3));
    let valid: Vec<usize> = (0..9).filter(|&i| z.heaps()[i].valid).collect();
    // Only the two 2×2 blocks keep a valid heap: (0,0) and (2,2).
    assert_eq!(valid, [0, 8]);
    // 8 cells over 2 valid heaps: c = 120000.
    assert_eq!(z.heaps()[0].capacity, 120_000);
    assert_eq!(z.heaps()[8].capacity, 120_000);
}

/// Test 17: the §6.3 bytes reproduce the zone exactly, except that
/// `levelAltitude` becomes the centre altitude and a dezoned cell of the last
/// row or column comes back zoned.
#[test]
fn test_17_save_round_trip() {
    let bytes: Vec<u8> = [
        "04 00 0A 00 00 00 14 00 00 00 0B 00 00 00 15 00 00 00 01",
        "01 00 00 00 01 00 00 00",
        "02 00 00 00 00 00 00 00 00 00 01 00 01 00 10 27 00 00 C0 D4 01 00",
        "01 00 00 00 00",
    ]
    .join(" ")
    .split_whitespace()
    .map(|b| u8::from_str_radix(b, 16).unwrap())
    .collect();
    let w = World::new();
    w.set_alt(10, 20, 281.0);
    let z = LandfillZone::empty(w.clone());
    assert!(z.read(&mut ByteReader::new(&bytes)));
    assert_eq!(z.rect(), rect(10, 20, 11, 21));
    assert_eq!(z.cell_count(), 4);
    assert_eq!(z.heaps(), [heap(0, 0, true, true, 120_000, 10_000)]);
    assert!(z.connected());
    assert_eq!(z.zot(), None);
    let mut out = ByteWriter::new();
    assert!(z.write(&mut out));
    assert_eq!(out.bytes(), &bytes[..]);
    // levelAltitude is not saved: Init takes the centre cell's altitude.
    assert_eq!(z.level_altitude(), 0.0);
    z.init();
    assert_eq!(z.level_altitude(), 281.0);

    // A dezoned cell in the last column comes back as zoned.
    let d = developer(&w);
    zone(&d, rect(10, 20, 12, 22), 14);
    let z = only_zone(&d);
    z.dezone_cell(12, 21, false, false);
    z.dezone_cell(11, 21, false, false);
    assert_eq!(z.cell_count(), 7);
    let mut out = ByteWriter::new();
    assert!(d.write(&mut out));
    let d2 = LandfillDeveloper::new(w.clone(), 2);
    assert!(d2.read(&mut ByteReader::new(out.bytes())));
    let z2 = only_zone(&d2);
    assert!(z2.is_mask_cell(12, 21));
    assert!(!z2.is_mask_cell(11, 21));
    assert_eq!(z2.cell_count(), 8);
    assert_eq!(z2.heaps(), z.heaps());
    assert!(d2.state().loaded);
}

/// Zone `Init` (§4.4.9, LF2): only an ordered equal compare with +0.0
/// recomputes `levelAltitude`. The spec's example: a zone over x 3..6,
/// z 10..10 reaching `Init` with −0.0 gets the altitude at the centre of cell
/// (4, 10); a NaN and a denormal are kept.
#[test]
fn zone_init_level_altitude_exact_zero() {
    let w = World::new();
    w.set_alt(4, 10, 283.0);
    let fresh = |v: f32| {
        let z = LandfillZone::new(w.clone(), CellRegion::all_set(rect(3, 10, 6, 10)));
        z.set_level_altitude(v);
        z.init();
        z.level_altitude()
    };
    assert_eq!(fresh(-0.0), 283.0);
    assert_eq!(fresh(0.0), 283.0);
    assert!(fresh(f32::NAN).is_nan());
    let denormal = f32::from_bits(1);
    assert_eq!(fresh(denormal).to_bits(), 1);
    assert_eq!(fresh(270.5), 270.5);
}

/// Test 18: `usedTotal` 150 over `capacityTotal` 100 → `GetLandfillUsage` →
/// (100, 100).
#[test]
fn test_18_usage_clamp() {
    let w = World::new();
    let d = developer(&w);
    zone(&d, rect(10, 10, 11, 11), 14);
    let z = only_zone(&d);
    z.set_connected(true);
    z.set_heap(0, heap(0, 0, true, true, 100, 150));
    d.update_capacity_values();
    assert_eq!((d.state().used_total, d.state().capacity_total), (150, 100));
    assert_eq!(d.get_landfill_usage(), (100, 100));
}

/// Test 19: `GetLandfillZoneFromCell` is half-open: rect (10, 20, 12, 22)
/// answers (10, 20) and (11, 21), not (12, 21) or (11, 22).
#[test]
fn test_19_zone_from_cell() {
    let w = World::new();
    let d = developer(&w);
    zone(&d, rect(10, 20, 12, 22), 14);
    let z = only_zone(&d);
    for (x, zc) in [(10, 20), (11, 21)] {
        assert!(Rc::ptr_eq(&d.get_landfill_zone_from_cell(x, zc).unwrap(), &z));
    }
    for (x, zc) in [(12, 21), (11, 22), (9, 20)] {
        assert!(d.get_landfill_zone_from_cell(x, zc).is_none());
    }
}

/// Test 20: efficiency. Maintenance 40 with current 48 → 1.2; maintenance 0
/// → 1.0.
#[test]
fn test_20_efficiency() {
    let w = World::new();
    let d = developer(&w);
    assert_eq!(d.get_efficiency(), 1.0);
    w.connected.set(true);
    zone(&d, rect(10, 20, 11, 21), 14);
    w.supply.set(10_000);
    month(&d);
    assert_eq!(d.get_maintenance_costs(), 40);
    w.current.set(48);
    assert_eq!(d.get_current_budget(), 48);
    assert_eq!(d.get_efficiency(), 1.2f32);
}

/// Test 21: a tile at y = 270 with the terrain raised to 271 (reason 1) moves
/// to 271; raised by 0.2 it stays; an empty cell in the rect is dezoned and
/// rezoned to 0.
#[test]
fn test_21_terrain_move() {
    let w = World::new();
    w.connected.set(true);
    let d = developer(&w);
    zone(&d, rect(10, 20, 12, 22), 14);
    w.supply.set(10_000);
    month(&d);
    assert_eq!(w.tile_at(10, 20).map(|t| t.2), Some(270.0));
    // Raised to 271.
    w.set_alt(10, 20, 271.0);
    terrain(&d, rect(10, 20, 10, 20), 1);
    assert_eq!(w.tile_at(10, 20).map(|t| t.2), Some(271.0));
    let token = w.occupant_at(10, 20, OCCUPANT_TYPE_LANDFILL).unwrap();
    assert_eq!(*w.reindexed.borrow(), [token]);
    assert_eq!(*w.occupant_messages.borrow(), [(token, msg::OCCUPANT_CHANGED)]);
    // Raised by 0.2: below the 0.25 threshold.
    w.set_alt(11, 20, 270.2);
    terrain(&d, rect(11, 20, 11, 20), 1);
    assert_eq!(w.tile_at(11, 20).map(|t| t.2), Some(270.0));
    assert!(w.placed.borrow().is_empty());
    // (12, 22) has no tile (mask 0): any terraforming dezones it.
    terrain(&d, rect(12, 22, 12, 22), 1);
    assert_eq!(*w.placed.borrow(), [(rect(12, 22, 12, 22), 0)]);
    assert!(!only_zone(&d).is_mask_cell(12, 22));
    // A tile below sea level moves to the sea level.
    w.set_alt(11, 21, 240.0);
    terrain(&d, rect(11, 21, 11, 21), 2);
    assert_eq!(w.tile_at(11, 21).map(|t| t.2), Some(250.0));
}

/// Test 22: `AttemptAreaLevelling` with stubbed slots (average 272.0, slope
/// 3.0, `ModifyTerrainHeight` recording).
#[test]
fn test_22_levelling_target_and_refusal() {
    let w = World::new();
    let r = rect(20, 20, 21, 21);
    let want = ModifyTerrainHeight {
        first: true,
        second: true,
        x1: 20,
        z1: 20,
        x2: 22,
        z2: 22,
        altitude: 272.0,
        lot: 0,
        reason: 5,
        cost_out: None,
    };
    // FLT_MAX: one call, the vertex average, true.
    assert!(attempt_area_levelling(&*w, r, 0.0, true, true, FLT_MAX, 0));
    assert_eq!(*w.modify.borrow(), [want]);
    // The second argument is inert.
    for f in [272.0f32, 0.0, -1.0] {
        w.modify.borrow_mut().clear();
        assert!(attempt_area_levelling(&*w, r, f, true, true, FLT_MAX, 0));
        assert_eq!(*w.modify.borrow(), [want]);
    }
    // Slope 41 over the tuning's 40: false, nothing levelled.
    w.slope.set(41.0);
    w.modify.borrow_mut().clear();
    assert!(!attempt_area_levelling(&*w, r, 0.0, true, true, FLT_MAX, 0));
    assert!(w.modify.borrow().is_empty());
    // A NaN maxAreaSlope takes the FLT_MAX branch too.
    assert!(!attempt_area_levelling(&*w, r, 0.0, true, true, f32::NAN, 0));
    assert!(w.modify.borrow().is_empty());
    // maxAreaSlope 5000 ≥ 1000: the slope test is skipped.
    w.slope.set(9999.0);
    assert!(attempt_area_levelling(&*w, r, 0.0, true, true, 5000.0, 0));
    assert_eq!(w.modify.borrow().len(), 1);
    // A finite maxAreaSlope ≠ FLT_MAX is used as is, not the tuning's 40.
    w.slope.set(45.0);
    w.modify.borrow_mut().clear();
    assert!(attempt_area_levelling(&*w, r, 0.0, true, true, 50.0, 0));
    assert!(!attempt_area_levelling(&*w, r, 0.0, true, true, 44.0, 0));
    assert_eq!(w.modify.borrow().len(), 1);
    // A NaN slope does not block.
    w.slope.set(f32::NAN);
    assert!(attempt_area_levelling(&*w, r, 0.0, true, true, FLT_MAX, 0));

    // Through the zone: §4.4.1 step 3 on test 3's terrain (variance 20 > 5)
    // levels the rect through slot 21 with (rect, 0.0, true, true, FLT_MAX).
    let w = World::new();
    w.set_alt(20, 20, 270.0);
    w.set_alt(20, 21, 272.0);
    w.set_alt(21, 20, 280.0);
    w.set_alt(21, 21, 290.0);
    let d = developer(&w);
    zone(&d, r, 14);
    assert_eq!(*w.levelling.borrow(), [(r, 0.0, true, true, FLT_MAX, true)]);
    assert_eq!(*w.modify.borrow(), [want]);
    // levelAltitude is the caller's cell-centre mean, not the vertex average.
    assert_eq!(only_zone(&d).level_altitude(), 278.0);

    // Refused (slope 41): no levelling, but the zone is still created and
    // populated, and levelAltitude is still the mean.
    let w = World::new();
    w.set_alt(20, 20, 270.0);
    w.set_alt(20, 21, 272.0);
    w.set_alt(21, 20, 280.0);
    w.set_alt(21, 21, 290.0);
    w.slope.set(41.0);
    let d = developer(&w);
    zone(&d, r, 14);
    assert_eq!(*w.levelling.borrow(), [(r, 0.0, true, true, FLT_MAX, false)]);
    assert!(w.modify.borrow().is_empty());
    let z = only_zone(&d);
    assert_eq!(z.level_altitude(), 278.0);
    assert_eq!(z.cell_count(), 4);
    assert!(w.occupant_at(20, 20, OCCUPANT_TYPE_BASE_TEXTURE).is_some());
}

/// `AddRegionToZone` step 3 (§4.4.4, LF5): each region mask cell, x outer
/// and z inner, calls `LevelRect((x, z, x+1, z+1))`; the rect is inclusive,
/// so `ModifyTerrainHeight` levels the 2×2-cell block (vertices `x..x+2 ×
/// z..z+2`), one cell past the region's +x and +z edges.
#[test]
fn add_region_levels_a_two_by_two_block_per_cell() {
    let w = World::new();
    let d = developer(&w);
    zone(&d, rect(10, 20, 11, 21), 14);
    let z = only_zone(&d);
    z.set_level_altitude(275.0);
    w.levelling.borrow_mut().clear();
    w.modify.borrow_mut().clear();
    z.add_region_to_zone(&CellRegion::all_set(rect(12, 20, 13, 21)));
    let cells = [(12, 20), (12, 21), (13, 20), (13, 21)];
    let calls: Vec<CellRect> = w.levelling.borrow().iter().map(|c| c.0).collect();
    let want: Vec<CellRect> = cells.iter().map(|&(x, z)| rect(x, z, x + 1, z + 1)).collect();
    assert_eq!(calls, want);
    let blocks: Vec<(i32, i32, i32, i32)> = w.modify.borrow().iter().map(|m| (m.x1, m.z1, m.x2, m.z2)).collect();
    let want: Vec<(i32, i32, i32, i32)> = cells.iter().map(|&(x, z)| (x, z, x + 2, z + 2)).collect();
    assert_eq!(blocks, want);
    assert!(w.modify.borrow().iter().all(|m| m.first && m.second && m.reason == 5));
    assert_eq!(z.rect(), rect(10, 20, 13, 21));
}

/// Test 23: reason 5 is ignored; reasons 1, 2 and 3 dezone the empty cells.
#[test]
fn test_23_reason_5_is_ignored() {
    for reason in [5u32, 1, 2, 3, 0, 4] {
        let w = World::new();
        w.connected.set(true);
        let d = developer(&w);
        zone(&d, rect(10, 20, 12, 22), 14);
        w.supply.set(10_000);
        month(&d);
        let z = only_zone(&d);
        let before = (z.region(), z.heaps(), w.tile_at(10, 20));
        terrain(&d, rect(10, 20, 12, 22), reason);
        let tools = matches!(reason, 1..=3);
        if tools {
            // The five cells without a tile are dezoned; the tiled ones stay.
            let placed: Vec<(i32, i32)> = w.placed.borrow().iter().map(|(r, _)| (r.x1, r.z1)).collect();
            assert_eq!(
                placed,
                [(10, 22), (11, 22), (12, 20), (12, 21), (12, 22)],
                "reason {reason}"
            );
            assert_eq!(z.cell_count(), 4);
        } else {
            assert!(w.placed.borrow().is_empty(), "reason {reason}");
            assert_eq!((z.region(), z.heaps(), w.tile_at(10, 20)), before, "reason {reason}");
        }
    }
}

// ---- lifecycle and messages (§1, §3) --------------------------------------------------------------

#[test]
fn init_registers_the_month_agent_and_five_messages() {
    let w = World::new();
    let d = LandfillDeveloper::new(w.clone(), 1);
    assert!(d.init());
    assert!(d.init());
    assert_eq!(*w.agents.borrow(), ["cSC4LandfillDeveloper/6/2"]);
    let subs: Vec<u32> = w.subscriptions.borrow().iter().copied().collect();
    let mut want = msg::SUBSCRIPTIONS.to_vec();
    want.sort_unstable();
    assert_eq!(subs, want);
    // The department arrives with PostCityInit, not Init.
    assert_eq!(d.state().department, None);
    d.do_message(&LandfillMessage::of_type(msg::POST_CITY_INIT));
    assert_eq!(d.state().department, Some(SANITATION));
    assert!(d.shutdown());
    assert!(w.subscriptions.borrow().is_empty());
    assert!(w.agents.borrow().is_empty());
    assert!(w.released.borrow().contains(&SANITATION));
}

#[test]
fn query_interface_answers_four_iids() {
    for iid in [
        IID_UNKNOWN,
        IID_LANDFILL_DEVELOPER,
        IID_MESSAGE_TARGET2,
        IID_SERIALIZABLE,
    ] {
        assert!(answers_iid(iid));
    }
    assert!(!answers_iid(0x2A15_F262));
}

#[test]
fn post_city_init_after_a_load_recomputes_the_totals() {
    let w = World::new();
    w.connected.set(true);
    let d = developer(&w);
    zone(&d, rect(10, 20, 11, 21), 14);
    w.supply.set(10_000);
    month(&d);
    only_zone(&d).set_connected(true);
    let mut out = ByteWriter::new();
    d.write(&mut out);
    let d2 = LandfillDeveloper::new(w.clone(), 2);
    d2.init();
    assert!(d2.read(&mut ByteReader::new(out.bytes())));
    assert_eq!(d2.state().capacity_total, 0);
    d2.do_message(&LandfillMessage::of_type(msg::POST_CITY_INIT));
    // Maintenance (4 tiles × 10), the capacity totals and the zone Init.
    assert_eq!(d2.get_maintenance_costs(), 40);
    assert_eq!(d2.get_landfill_usage(), (10_000, 120_000));
    assert!(only_zone(&d2).is_initialised());
    // A wrong version fails the read but still sets `loaded`.
    let d3 = LandfillDeveloper::new(w.clone(), 3);
    assert!(!d3.read(&mut ByteReader::new(&[2, 0, 0, 0, 0, 0])));
    assert!(d3.state().loaded);
}

#[test]
fn funds_changed_and_view_level_gate_the_effects() {
    let w = World::new();
    let d = LandfillDeveloper::new(w.clone(), 1);
    d.init();
    zone(&d, rect(10, 20, 11, 21), 14);
    // No department yet: FundsChanged does nothing.
    d.do_message(&LandfillMessage::of_type(msg::FUNDS_CHANGED));
    assert!(w.trucks.borrow().is_empty());
    d.do_message(&LandfillMessage::of_type(msg::POST_CITY_INIT));
    w.funding.set(0.0);
    d.do_message(&LandfillMessage::of_type(msg::FUNDS_CHANGED));
    let truck = only_zone(&d).effects().truck.unwrap();
    assert_eq!(*w.trucks.borrow(), [(truck, false)]);
    // View level 1 starts the seagulls once; leaving level 1 stops them once.
    let gull = only_zone(&d).effects().seagull.unwrap();
    w.view_level.set(1);
    d.do_message(&LandfillMessage::of_type(msg::PRE_VIEW_LEVEL_CHANGE));
    d.do_message(&LandfillMessage::of_type(msg::PRE_VIEW_LEVEL_CHANGE));
    w.view_level.set(2);
    d.do_message(&LandfillMessage::of_type(msg::PRE_VIEW_LEVEL_CHANGE));
    d.do_message(&LandfillMessage::of_type(msg::PRE_VIEW_LEVEL_CHANGE));
    assert_eq!(*w.seagull.borrow(), [(gull, true), (gull, false)]);
}

#[test]
fn empty_list_month_records_stale_totals_and_closes_the_valve() {
    let w = World::new();
    w.uncollected.set(-5);
    w.supply.set(300);
    let d = developer(&w);
    month(&d);
    assert_eq!(
        *w.history.borrow(),
        [(history::CAPACITY, 0), (history::TOTAL_GARBAGE, 300)]
    );
    assert_eq!(*w.full_expenses.borrow(), [0]);
    assert_eq!(*w.garbage_capacity.borrow(), [0]);
}

#[test]
fn populate_zone_drops_roads_and_foreign_zones_without_invalidating() {
    let w = World::new();
    w.networks.borrow_mut().insert((11, 11));
    w.zone_types.borrow_mut().insert((10, 11), 1);
    let d = developer(&w);
    zone(&d, rect(10, 10, 12, 12), 14);
    let z = only_zone(&d);
    assert!(!z.is_mask_cell(11, 11));
    assert!(!z.is_mask_cell(10, 11));
    // Quirk 5: the dropped cells' heaps stay valid.
    assert_eq!(z.num_valid_heaps(), 4);
    assert_eq!(z.cell_count(), 7);
}

#[test]
fn is_zone_connected_asks_every_type_when_nothing_connects() {
    let w = World::new();
    let d = developer(&w);
    zone(&d, rect(10, 10, 11, 11), 14);
    w.count_calls.set(0);
    assert!(!only_zone(&d).is_zone_connected());
    assert_eq!(w.count_calls.get(), 12);
    w.connected.set(true);
    w.count_calls.set(0);
    assert!(only_zone(&d).is_zone_connected());
    assert_eq!(w.count_calls.get(), 1);
}

#[test]
fn a_bad_heap_version_appends_the_previous_heap_and_fails() {
    // A 3×2 zone record (W = 2, H = 1) whose second heap has version 3.
    let mut out = ByteWriter::new();
    out.put_u16(4);
    for v in [0, 0, 2, 1] {
        out.put_i32(v);
    }
    for _ in 0..2 {
        out.put_u8(1);
    }
    out.put_i32(2);
    out.put_i32(1);
    out.put_u16(2);
    for v in [0u32, 0] {
        out.put_u32(v);
    }
    out.put_u16(1);
    out.put_u16(1);
    out.put_u32(77);
    out.put_u32(99);
    out.put_u16(3);
    let w = World::new();
    let z = LandfillZone::empty(w.clone());
    assert!(!z.read(&mut ByteReader::new(out.bytes())));
    assert_eq!(
        z.heaps(),
        [heap(0, 0, true, true, 99, 77), heap(0, 0, true, true, 99, 77)]
    );
}
