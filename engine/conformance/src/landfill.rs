// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops `landfill_…` for `fc4_core::landfill`
//! (`docs/specs/landfill.md`): a scripted city (terrain, pollution counters,
//! the sanitation department, a zone map, occupants and props, connectivity,
//! the lot developer's levelling slots stubbed as §11 test 22 asks) that
//! implements [`LandfillHost`], an event log of the host calls with effects,
//! a queue of the messages the host posts (`PlaceZone`'s `0x07699455`,
//! the levelling's `0x278E6003` with `data4 = 5`), and ops that drive the
//! developer and its zones. See README.md ("Landfill ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::{Rc, Weak};

use fc4_core::landfill::{
    attempt_area_levelling, budget_ids, msg, shipped_tile_costs, shipped_tile_exemplar, shipped_tuning, tile, CellRect,
    CellRegion, CrtRand, Heap, LandfillDeveloper, LandfillHost, LandfillMessage, LandfillOccupant, LandfillZone,
    LevellingSlots, ModifyTerrainHeight, ZoneEffects, FLT_MAX, IID_OCCUPANT, NETWORK_TILE_MASK,
    OCCUPANT_TYPE_BASE_TEXTURE, OCCUPANT_TYPE_FLORA, OCCUPANT_TYPE_LANDFILL, PROP_ITEM_COST, SHIPPED_MAX_AREA_SLOPE,
    ZOT_PROP,
};
use fc4_core::property::PropertyHolder;
use fc4_core::stream::{ByteReader, ByteWriter, RefValue};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{boolean, field, HostRef};
use crate::scalar::{
    self, arr, bool_of, check_expect, check_state, f32_of, holder_of, i64_of, jf, ji, opt, show, str_of, u32_of,
    EventLog, R,
};

/// Cells are 16 m (`CellCenterToPosition` = `16·x + 8`).
const CELL: f32 = 16.0;
/// The sanitation department's token.
const SANITATION: u64 = 0x5A;

// ---- world -------------------------------------------------------------------------------------

/// An occupant the developer sees: a landfill tile (a core object), a base
/// texture or a flora occupant.
#[derive(Clone)]
struct Occ {
    kind: u32,
    x: i32,
    z: i32,
    tile: Option<Rc<LandfillOccupant>>,
}

pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    dev: RefCell<Option<Rc<LandfillDeveloper>>>,
    next_dev: Cell<u64>,
    // City and terrain.
    cells: Cell<(i32, i32)>,
    base_alt: Cell<f32>,
    alt: RefCell<BTreeMap<(i32, i32), f32>>,
    sea: Cell<f32>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    // Pollution.
    supply: Cell<u32>,
    tile_cap: Cell<u32>,
    uncollected: Cell<i32>,
    active_landfill: RefCell<BTreeSet<(i32, i32)>>,
    // Budget.
    department: Cell<bool>,
    funding: Cell<f32>,
    current: Cell<i64>,
    // Zone manager.
    zone_types: RefCell<BTreeMap<(i32, i32), i32>>,
    default_zone: Cell<i32>,
    // Occupants and props.
    occupants: RefCell<BTreeMap<u64, Occ>>,
    zots: RefCell<BTreeMap<u64, [f32; 3]>>,
    next: Cell<u64>,
    // Traffic.
    connected: Cell<bool>,
    networks: RefCell<BTreeSet<(i32, i32)>>,
    // The lot developer's levelling slots (§4.5.3).
    avg: Cell<Option<f32>>,
    slope: Cell<f32>,
    tuning_slope: Cell<f32>,
    apply_levelling: Cell<bool>,
    // View.
    view_level: Cell<u32>,
    // Messages the host posted, delivered by `landfill_deliver`.
    posted: RefCell<Vec<LandfillMessage>>,
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            me: RefCell::new(Weak::new()),
            events: EventLog::default(),
            dev: RefCell::new(None),
            next_dev: Cell::new(1),
            cells: Cell::new((64, 64)),
            base_alt: Cell::new(270.0),
            alt: RefCell::new(BTreeMap::new()),
            sea: Cell::new(250.0),
            tuning: RefCell::new(Some(Rc::new(shipped_tuning()))),
            supply: Cell::new(0),
            tile_cap: Cell::new(30000),
            uncollected: Cell::new(0),
            active_landfill: RefCell::new(BTreeSet::new()),
            department: Cell::new(true),
            funding: Cell::new(1.0),
            current: Cell::new(0),
            zone_types: RefCell::new(BTreeMap::new()),
            default_zone: Cell::new(14),
            occupants: RefCell::new(BTreeMap::new()),
            zots: RefCell::new(BTreeMap::new()),
            next: Cell::new(0x1000),
            connected: Cell::new(false),
            networks: RefCell::new(BTreeSet::new()),
            avg: Cell::new(None),
            slope: Cell::new(0.0),
            tuning_slope: Cell::new(SHIPPED_MAX_AREA_SLOPE),
            apply_levelling: Cell::new(true),
            view_level: Cell::new(1),
            posted: RefCell::new(Vec::new()),
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn rc(&self) -> R<Rc<World>> {
        self.me.borrow().upgrade().ok_or_else(|| "world dropped".into())
    }

    fn dev(&self) -> R<Rc<LandfillDeveloper>> {
        self.dev
            .borrow()
            .clone()
            .ok_or_else(|| "no developer (use landfill_create)".into())
    }

    fn zone(&self, op: &Json) -> R<Rc<LandfillZone>> {
        let i = opt(op, "zone").map_or(Ok(0), |v| i64_of(v, "zone"))? as usize;
        self.dev()?
            .zone_list()
            .get(i)
            .cloned()
            .ok_or_else(|| format!("no zone {i}"))
    }

    fn token(&self) -> u64 {
        let t = self.next.get();
        self.next.set(t + 1);
        t
    }

    fn altitude(&self, x: i32, z: i32) -> f32 {
        self.alt.borrow().get(&(x, z)).copied().unwrap_or(self.base_alt.get())
    }

    fn occupant_of(&self, x: i32, z: i32, kind: u32) -> Option<u64> {
        self.occupants
            .borrow()
            .iter()
            .find(|(_, o)| o.kind == kind && o.x == x && o.z == z)
            .map(|(t, _)| *t)
    }

    fn tile_of(&self, occupant: u64) -> Option<(i32, i32, Rc<LandfillOccupant>)> {
        let o = self.occupants.borrow().get(&occupant).cloned()?;
        Some((o.x, o.z, o.tile?))
    }

    /// Every landfill tile as `[x, z, tile, orientation, y]`, by (z, x).
    fn tiles(&self) -> Json {
        let mut v: Vec<(i32, i32, Json)> = self
            .occupants
            .borrow()
            .values()
            .filter_map(|o| {
                let t = o.tile.as_ref()?;
                Some((
                    o.z,
                    o.x,
                    Json::Array(vec![
                        ji(o.x),
                        ji(o.z),
                        ji(t.tile_id()),
                        ji(t.orientation()),
                        jf(t.position()[1]),
                    ]),
                ))
            })
            .collect();
        v.sort_by_key(|e| (e.0, e.1));
        Json::Array(v.into_iter().map(|e| e.2).collect())
    }
}

fn jrect(r: CellRect) -> Json {
    Json::Array(vec![ji(r.x1), ji(r.z1), ji(r.x2), ji(r.z2)])
}

/// A float that may be `FLT_MAX` or `NaN`, as JSON (`"FLT_MAX"`, `"NaN"`).
fn jspecial(v: f32) -> Json {
    if v.is_nan() {
        Json::Str("NaN".into())
    } else if v == FLT_MAX {
        Json::Str("FLT_MAX".into())
    } else {
        jf(v)
    }
}

fn special_of(j: &Json, what: &str) -> R<f32> {
    match j.as_str() {
        Some("FLT_MAX") => Ok(FLT_MAX),
        Some("NaN") => Ok(f32::NAN),
        _ => f32_of(j, what),
    }
}

impl LevellingSlots for World {
    /// The stubbed vertex average, or the mean of the cell altitudes over the
    /// rect when none is set.
    fn average_area_altitude(&self, rect: CellRect, _with_water: bool) -> f32 {
        if let Some(a) = self.avg.get() {
            return a;
        }
        let (mut sum, mut n) = (0.0f64, 0.0f64);
        for z in rect.z1..=rect.z2 {
            for x in rect.x1..=rect.x2 {
                sum += f64::from(self.altitude(x, z));
                n += 1.0;
            }
        }
        if n == 0.0 {
            0.0
        } else {
            (sum / n) as f32
        }
    }
    fn area_slope(&self, _rect: CellRect, _with_water: bool) -> f32 {
        self.slope.get()
    }
    fn max_area_slope_allowed_for_levelling(&self) -> f32 {
        self.tuning_slope.get()
    }
    /// Records the call; with `apply_levelling` it sets the cells of
    /// `[x1, x2) × [z1, z2)` to the altitude and posts `0x278E6003` with the
    /// reason as `data4` (the terrain's `RedisplayTerrain`, §4.5.3).
    fn modify_terrain_height(&self, c: ModifyTerrainHeight) -> bool {
        self.events.push(
            "modify_terrain",
            vec![
                ("first", Json::Bool(c.first)),
                ("second", Json::Bool(c.second)),
                ("rect", Json::Array(vec![ji(c.x1), ji(c.z1), ji(c.x2), ji(c.z2)])),
                ("altitude", jf(c.altitude)),
                ("lot", ji(c.lot as i64)),
                ("reason", ji(c.reason)),
                ("cost_out", Json::Bool(c.cost_out.is_some())),
            ],
        );
        if self.apply_levelling.get() {
            for z in c.z1..c.z2 {
                for x in c.x1..c.x2 {
                    self.alt.borrow_mut().insert((x, z), c.altitude);
                }
            }
            let mut m = LandfillMessage::of_type(msg::TERRAIN_MODIFIED);
            m.data1 = ((c.x1 as u32 + 1) << 16) | (c.x2 - 1) as u32;
            m.data2 = ((c.z1 as u32 + 1) << 16) | (c.z2 - 1) as u32;
            m.data4 = c.reason;
            self.posted.borrow_mut().push(m);
        }
        true
    }
}

impl LandfillHost for World {
    fn acquire_services(&self) {}
    fn release_services(&self) {}
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        self.events.push(
            "add_agent",
            vec![
                ("name", Json::Str(name.into())),
                ("level", ji(level)),
                ("flags", ji(flags)),
            ],
        );
    }
    fn remove_agent(&self) {
        self.events.push("remove_agent", vec![]);
    }
    fn subscribe(&self, msg_type: u32) {
        self.events.push("subscribe", vec![("type", ji(msg_type))]);
    }
    fn unsubscribe(&self, msg_type: u32) {
        self.events.push("unsubscribe", vec![("type", ji(msg_type))]);
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
    fn set_altitude(&self, rect: CellRect, altitude: f32) {
        self.events
            .push("set_altitude", vec![("rect", jrect(rect)), ("altitude", jf(altitude))]);
        for z in rect.z1..=rect.z2 {
            for x in rect.x1..=rect.x2 {
                self.alt.borrow_mut().insert((x, z), altitude);
            }
        }
    }
    fn redisplay_terrain(&self, rect: CellRect) {
        self.events.push("redisplay", vec![("rect", jrect(rect))]);
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
        self.events.push("supply", vec![("delta", ji(delta))]);
        self.supply.set(self.supply.get().wrapping_add(delta as u32));
    }
    fn set_garbage_capacity(&self, available: i32) {
        self.events.push("garbage_capacity", vec![("value", ji(available))]);
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
        self.events.push("put_in", vec![("value", ji(n))]);
    }
    fn get_department_budget(&self, department: u32) -> Option<u64> {
        (department == budget_ids::SANITATION_DEPARTMENT && self.department.get()).then_some(SANITATION)
    }
    fn add_ref(&self, _token: u64) {}
    fn release(&self, token: u64) {
        self.events.push("release", vec![("token", ji(token as i64))]);
    }
    fn department_funding_percentage(&self, _department: u64) -> f32 {
        self.funding.get()
    }
    fn line_item_current_expenses(&self, _department: u64, line: u32) -> Option<i64> {
        (line == budget_ids::LANDFILL_LINE).then_some(self.current.get())
    }
    fn set_line_item_full_expenses(&self, _department: u64, line: u32, value: i64) {
        self.events
            .push("full_expenses", vec![("line", ji(line)), ("value", ji(value))]);
    }
    fn occupant_item_cost(&self, occupant: u64, property: u32) -> Option<i64> {
        if property != PROP_ITEM_COST {
            return None;
        }
        let (_, _, t) = self.tile_of(occupant)?;
        let id = t.tile_id();
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
    /// Rezones the mask cells and posts the zoning message (§4.4.8: no
    /// re-entrancy; `landfill_deliver` delivers it).
    fn place_zone(&self, region: &CellRegion, zone_type: i32) -> bool {
        self.events.push(
            "place_zone",
            vec![("rect", jrect(region.rect())), ("type", ji(zone_type))],
        );
        for (x, z) in region.mask_cells() {
            self.zone_types.borrow_mut().insert((x, z), zone_type);
        }
        let mut m = LandfillMessage::of_type(msg::ZONING_CHANGE);
        m.data3 = zone_type as u32;
        m.region = Some(region.clone());
        self.posted.borrow_mut().push(m);
        true
    }
    fn occupant_at(&self, x: i32, z: i32, occupant_type: u32) -> Option<u64> {
        self.occupant_of(x, z, occupant_type)
    }
    fn insert_landfill_occupant(&self, occupant: &Rc<LandfillOccupant>, x: i32, z: i32) -> bool {
        let t = self.token();
        occupant.set_token(t);
        self.events.push(
            "tile",
            vec![
                ("x", ji(x)),
                ("z", ji(z)),
                ("tile", ji(occupant.tile_id())),
                ("orientation", ji(occupant.orientation())),
                ("y", jf(occupant.position()[1])),
            ],
        );
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
        let o = self.occupants.borrow_mut().remove(&occupant);
        if let Some(o) = &o {
            if o.tile.is_some() {
                self.events.push("tile_remove", vec![("x", ji(o.x)), ("z", ji(o.z))]);
            }
        }
        o.is_some()
    }
    fn occupant_changed(&self, _occupant: u64) {}
    fn occupant_position(&self, occupant: u64) -> [f32; 3] {
        self.tile_of(occupant).map_or([0.0; 3], |t| t.2.position())
    }
    fn occupant_set_position(&self, occupant: u64, position: [f32; 3]) {
        if let Some((x, z, t)) = self.tile_of(occupant) {
            self.events
                .push("tile_move", vec![("x", ji(x)), ("z", ji(z)), ("y", jf(position[1]))]);
            t.set_position(position);
        }
    }
    fn occupant_set_removal_flags(&self, _occupant: u64, _flags: u32) {}
    fn occupant_post_message(&self, _occupant: u64, _message: u32) {}
    fn is_landfill_occupant(&self, occupant: u64) -> bool {
        self.tile_of(occupant).is_some()
    }
    fn is_empty_landfill_tile(&self, occupant: u64) -> bool {
        self.tile_of(occupant).is_some_and(|t| t.2.is_empty_landfill_tile())
    }
    fn remove_incompatible_occupants(&self, x: i32, z: i32) {
        self.occupants
            .borrow_mut()
            .retain(|_, o| !(o.kind == OCCUPANT_TYPE_FLORA && o.x == x && o.z == z));
    }
    fn set_empty_cell_texture(&self, x: i32, z: i32) {
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
    fn remove_base_texture(&self, x: i32, z: i32) {
        self.occupants
            .borrow_mut()
            .retain(|_, o| !(o.kind == OCCUPANT_TYPE_BASE_TEXTURE && o.x == x && o.z == z));
    }
    fn add_lot_prop(&self, prop_type: u32, position: [f32; 3], _orientation: i32) -> Option<u64> {
        if prop_type != ZOT_PROP {
            return None;
        }
        let t = self.token();
        self.events.push(
            "zot_add",
            vec![("pos", Json::Array(position.iter().map(|&v| jf(v)).collect()))],
        );
        self.zots.borrow_mut().insert(t, position);
        Some(t)
    }
    fn remove_prop(&self, occupant: u64) -> bool {
        let removed = self.zots.borrow_mut().remove(&occupant).is_some();
        if removed {
            self.events.push("zot_remove", vec![]);
        }
        removed
    }
    fn zot_orientation(&self) -> i32 {
        0
    }
    fn find_network_tile(&self, x: i32, z: i32, flag_mask: u32, flag: bool) -> Option<u64> {
        (flag_mask == NETWORK_TILE_MASK && flag && self.networks.borrow().contains(&(x, z))).then_some(1)
    }
    fn subnetworks_in_region(&self, _region: &CellRegion) -> Vec<u32> {
        if self.connected.get() {
            vec![1]
        } else {
            Vec::new()
        }
    }
    fn occupant_count_for_all_subnetworks(&self, developer_type: u32) -> Vec<u32> {
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
        self.events.push(
            "levelling",
            vec![
                ("rect", jrect(rect)),
                ("f", jspecial(f)),
                ("with_water", Json::Bool(with_water)),
                ("water_flag", Json::Bool(water_flag)),
                ("max", jspecial(max_area_slope)),
                ("result", Json::Bool(r)),
            ],
        );
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
    fn set_truck_enabled(&self, _truck: u64, enabled: bool) {
        self.events.push("truck", vec![("enabled", Json::Bool(enabled))]);
    }
    fn start_seagull(&self, _seagull: u64, _flag: u32) {
        self.events.push("seagull", vec![("on", Json::Bool(true))]);
    }
    fn stop_seagull(&self, _seagull: u64, _flag: u32) {
        self.events.push("seagull", vec![("on", Json::Bool(false))]);
    }
    fn send_history(&self, series: u32, value: u32) {
        self.events
            .push("history", vec![("series", ji(series)), ("value", ji(value))]);
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

// ---- ops ---------------------------------------------------------------------------------------

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "landfill", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("landfill_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "altitude" => altitude(&w, op),
        "zone_types" => zone_types(&w, op),
        "create" => create(&w, op),
        "shutdown" => shutdown(&w),
        "zone" => zone(&w, op),
        "message" => message(&w, op),
        "month" => month(&w, op),
        "terrain" => terrain(&w, op),
        "deliver" => deliver(&w, op),
        "heaps" => heaps(&w, op),
        "set" => set(&w, op),
        "dezone_cell" => dezone_cell(&w, op),
        "decay" => decay(&w, op),
        "redistribute" => w.zone(op).map(|z| z.redistribute_garbage()),
        "update_tiles" => update_tiles(&w, op),
        "levelling" => levelling(&w, op),
        "call" => call(&w, op),
        "expect" => expect(&w, op),
        "record" => record(&w, op),
        "zone_record" => zone_record(&w, op),
        "reload" => reload(&w, op),
        "expect_events" => w.events.expect_events(op),
        "expect_event_count" => w.events.expect_count(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }
        _ => Err(format!("unknown op '{name}'")),
    })
}

fn rect_of(j: &Json) -> R<CellRect> {
    let a = arr(j, "rect")?;
    if a.len() != 4 {
        return Err("'rect' must be [x1, z1, x2, z2]".into());
    }
    Ok(CellRect::new(
        i64_of(&a[0], "x1")? as i32,
        i64_of(&a[1], "z1")? as i32,
        i64_of(&a[2], "x2")? as i32,
        i64_of(&a[3], "z2")? as i32,
    ))
}

fn ints<const N: usize>(j: &Json, what: &str) -> R<[i64; N]> {
    let a = arr(j, what)?;
    if a.len() != N {
        return Err(format!("'{what}' must have {N} elements"));
    }
    let mut out = [0i64; N];
    for (o, v) in out.iter_mut().zip(a) {
        *o = scalar::num(v).ok_or_else(|| format!("'{what}' must hold numbers"))? as i64;
    }
    Ok(out)
}

/// `landfill_env`: `cells` ([w, h]), `sea`, `altitude` (the flat ground),
/// `tuning` ("shipped", null for the code defaults, or a property map),
/// `supply`, `tile_capacity`, `uncollected`, `funding`, `current` (the
/// line item's current expenses), `department` (bool), `connected`,
/// `networks` ([[x, z], …]), `zone_type` (the default zone type),
/// `view_level`, `levelling` ({`avg`, `slope`, `tuning`, `apply`}).
fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "cells") {
        let [x, z] = ints::<2>(v, "cells")?;
        w.cells.set((x as i32, z as i32));
    }
    if let Some(v) = opt(op, "sea") {
        w.sea.set(f32_of(v, "sea")?);
    }
    if let Some(v) = opt(op, "altitude") {
        w.base_alt.set(f32_of(v, "altitude")?);
    }
    if let Some(v) = op.get("tuning") {
        *w.tuning.borrow_mut() = match v {
            Json::Null => None,
            Json::Str(s) if s == "shipped" => Some(Rc::new(shipped_tuning())),
            other => Some(Rc::new(holder_of(other)?)),
        };
    }
    if let Some(v) = opt(op, "supply") {
        w.supply.set(u32_of(v, "supply")?);
    }
    if let Some(v) = opt(op, "tile_capacity") {
        w.tile_cap.set(u32_of(v, "tile_capacity")?);
    }
    if let Some(v) = opt(op, "uncollected") {
        w.uncollected.set(i64_of(v, "uncollected")? as i32);
    }
    if let Some(v) = opt(op, "funding") {
        w.funding.set(f32_of(v, "funding")?);
    }
    if let Some(v) = opt(op, "current") {
        w.current.set(i64_of(v, "current")?);
    }
    if let Some(v) = opt(op, "department") {
        w.department.set(bool_of(v, "department")?);
    }
    if let Some(v) = opt(op, "connected") {
        w.connected.set(bool_of(v, "connected")?);
    }
    if let Some(v) = opt(op, "networks") {
        let mut n = w.networks.borrow_mut();
        n.clear();
        for c in arr(v, "networks")? {
            let [x, z] = ints::<2>(c, "networks")?;
            n.insert((x as i32, z as i32));
        }
    }
    if let Some(v) = opt(op, "zone_type") {
        w.default_zone.set(i64_of(v, "zone_type")? as i32);
    }
    if let Some(v) = opt(op, "view_level") {
        w.view_level.set(u32_of(v, "view_level")?);
    }
    if let Some(l) = opt(op, "levelling") {
        if let Some(v) = l.get("avg") {
            w.avg.set(match v {
                Json::Null => None,
                v => Some(f32_of(v, "avg")?),
            });
        }
        if let Some(v) = opt(l, "slope") {
            w.slope.set(special_of(v, "slope")?);
        }
        if let Some(v) = opt(l, "tuning") {
            w.tuning_slope.set(special_of(v, "tuning")?);
        }
        if let Some(v) = opt(l, "apply") {
            w.apply_levelling.set(bool_of(v, "apply")?);
        }
    }
    Ok(())
}

/// `landfill_altitude`: `cells` ([[x, z, altitude], …]) and/or `rect` with
/// `value`.
fn altitude(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "cells") {
        for c in arr(v, "cells")? {
            let a = arr(c, "cell")?;
            if a.len() != 3 {
                return Err("a cell is [x, z, altitude]".into());
            }
            let (x, z) = (i64_of(&a[0], "x")? as i32, i64_of(&a[1], "z")? as i32);
            w.alt.borrow_mut().insert((x, z), f32_of(&a[2], "altitude")?);
        }
    }
    if let Some(r) = opt(op, "rect") {
        let r = rect_of(r)?;
        let a = f32_of(field(op, "value")?, "value")?;
        for z in r.z1..=r.z2 {
            for x in r.x1..=r.x2 {
                w.alt.borrow_mut().insert((x, z), a);
            }
        }
    }
    Ok(())
}

/// `landfill_zone_types`: `cells` ([[x, z, type], …]).
fn zone_types(w: &World, op: &Json) -> R<()> {
    for c in arr(field(op, "cells")?, "cells")? {
        let [x, z, t] = ints::<3>(c, "cell")?;
        w.zone_types.borrow_mut().insert((x as i32, z as i32), t as i32);
    }
    Ok(())
}

/// `landfill_create`: a developer, `Init`, then PostCityInit unless
/// `post_city_init` is false; `expect` = Init's result.
fn create(w: &World, op: &Json) -> R<()> {
    let token = w.next_dev.get();
    w.next_dev.set(token + 1);
    let d = LandfillDeveloper::new(w.rc()? as Rc<dyn LandfillHost>, token);
    *w.dev.borrow_mut() = Some(d.clone());
    let ok = d.init();
    check_expect(op, Json::Bool(ok))?;
    if opt(op, "post_city_init").map_or(Ok(true), |v| bool_of(v, "post_city_init"))? {
        d.do_message(&LandfillMessage::of_type(msg::POST_CITY_INIT));
    }
    Ok(())
}

fn shutdown(w: &World) -> R<()> {
    w.dev()?.shutdown();
    Ok(())
}

/// `landfill_zone`: delivers the zoning message for `rect` (all cells) with
/// `type` (default 14).
fn zone(w: &World, op: &Json) -> R<()> {
    let r = rect_of(field(op, "rect")?)?;
    let t = opt(op, "type").map_or(Ok(14), |v| i64_of(v, "type"))? as i32;
    let mut m = LandfillMessage::of_type(msg::ZONING_CHANGE);
    m.data3 = t as u32;
    m.region = Some(CellRegion::all_set(r));
    let ok = w.dev()?.do_message(&m);
    check_expect(op, Json::Bool(ok))
}

/// `landfill_message`: `type`, `data1`…`data4`, optional `rect` (the
/// region); `expect` = DoMessage's result.
fn message(w: &World, op: &Json) -> R<()> {
    let mut m = LandfillMessage::of_type(u32_of(field(op, "type")?, "type")?);
    for (k, slot) in [
        ("data1", &mut m.data1),
        ("data2", &mut m.data2),
        ("data3", &mut m.data3),
        ("data4", &mut m.data4),
    ] {
        if let Some(v) = opt(op, k) {
            *slot = u32_of(v, k)?;
        }
    }
    if let Some(r) = opt(op, "rect") {
        m.region = Some(CellRegion::all_set(rect_of(r)?));
    }
    let ok = w.dev()?.do_message(&m);
    check_expect(op, Json::Bool(ok))
}

/// `landfill_month`: `count` SimNewMonth messages (default 1).
fn month(w: &World, op: &Json) -> R<()> {
    let n = opt(op, "count").map_or(Ok(1), |v| i64_of(v, "count"))?;
    let d = w.dev()?;
    for _ in 0..n {
        d.do_message(&LandfillMessage::of_type(msg::SIM_NEW_MONTH));
    }
    Ok(())
}

/// `landfill_terrain`: optionally sets every cell of `rect` to `altitude`,
/// then delivers `0x278E6003` over `rect` with `data4 = reason` (default 1).
fn terrain(w: &World, op: &Json) -> R<()> {
    let r = rect_of(field(op, "rect")?)?;
    if let Some(v) = opt(op, "altitude") {
        let a = f32_of(v, "altitude")?;
        for z in r.z1..=r.z2 {
            for x in r.x1..=r.x2 {
                w.alt.borrow_mut().insert((x, z), a);
            }
        }
    }
    let mut m = LandfillMessage::of_type(msg::TERRAIN_MODIFIED);
    m.data1 = ((r.x1 as u32 + 1) << 16) | r.x2 as u32;
    m.data2 = ((r.z1 as u32 + 1) << 16) | r.z2 as u32;
    m.data4 = opt(op, "reason").map_or(Ok(1), |v| u32_of(v, "reason"))?;
    w.dev()?.do_message(&m);
    Ok(())
}

/// `landfill_deliver`: delivers the messages the host posted, in order,
/// until none are left (`count` = how many were delivered).
fn deliver(w: &World, op: &Json) -> R<()> {
    let d = w.dev()?;
    let mut n = 0i64;
    loop {
        let next = {
            let mut q = w.posted.borrow_mut();
            if q.is_empty() {
                break;
            }
            q.remove(0)
        };
        d.do_message(&next);
        n += 1;
        if n > 10_000 {
            return Err("message loop".into());
        }
    }
    if let Some(v) = opt(op, "count") {
        let want = i64_of(v, "count")?;
        if n != want {
            return Err(format!("delivered {n} messages, expected {want}"));
        }
    }
    Ok(())
}

fn heap_of(j: &Json) -> R<Heap> {
    let a = arr(j, "heap")?;
    if a.len() != 6 {
        return Err("a heap is [hx, hz, valid, active, capacity, contained]".into());
    }
    Ok(Heap {
        hx: u32_of(&a[0], "hx")?,
        hz: u32_of(&a[1], "hz")?,
        valid: bool_of(&a[2], "valid")?,
        active: bool_of(&a[3], "active")?,
        capacity: u32_of(&a[4], "capacity")?,
        contained: u32_of(&a[5], "contained")?,
    })
}

fn jheap(h: &Heap) -> Json {
    Json::Array(vec![
        ji(h.hx),
        ji(h.hz),
        Json::Bool(h.valid),
        Json::Bool(h.active),
        ji(h.capacity),
        ji(h.contained),
    ])
}

/// `landfill_heaps`: `zone`, and `heaps` (replaces the vector) or `index`
/// with `heap` (one heap), or `contained` (sets every heap's contained).
fn heaps(w: &World, op: &Json) -> R<()> {
    let z = w.zone(op)?;
    if let Some(v) = opt(op, "heaps") {
        z.set_heaps(arr(v, "heaps")?.iter().map(heap_of).collect::<R<Vec<_>>>()?);
    }
    if let Some(v) = opt(op, "heap") {
        let i = i64_of(field(op, "index")?, "index")? as usize;
        z.set_heap(i, heap_of(v)?);
    }
    if let Some(v) = opt(op, "contained") {
        let c = u32_of(v, "contained")?;
        for (i, h) in z.heaps().into_iter().enumerate() {
            z.set_heap(i, Heap { contained: c, ..h });
        }
    }
    Ok(())
}

/// `landfill_set`: `zone`, `level_altitude`, `connected`, `clear_cells`
/// ([[x, z], …] cleared from the mask).
fn set(w: &World, op: &Json) -> R<()> {
    let z = w.zone(op)?;
    if let Some(v) = opt(op, "level_altitude") {
        z.set_level_altitude(f32_of(v, "level_altitude")?);
    }
    if let Some(v) = opt(op, "connected") {
        z.set_connected(bool_of(v, "connected")?);
    }
    if let Some(v) = opt(op, "clear_cells") {
        let mut region = z.region();
        for c in arr(v, "clear_cells")? {
            let [x, zc] = ints::<2>(c, "cell")?;
            region.set(x as i32, zc as i32, false);
        }
        z.set_region(region);
    }
    Ok(())
}

/// `landfill_dezone_cell`: `zone`, `cell` ([x, z]), `change_zone`,
/// `remove_texture` (both default false).
fn dezone_cell(w: &World, op: &Json) -> R<()> {
    let z = w.zone(op)?;
    let [x, zc] = ints::<2>(field(op, "cell")?, "cell")?;
    let change = opt(op, "change_zone").map_or(Ok(false), |v| bool_of(v, "change_zone"))?;
    let texture = opt(op, "remove_texture").map_or(Ok(false), |v| bool_of(v, "remove_texture"))?;
    z.dezone_cell(x as i32, zc as i32, change, texture);
    Ok(())
}

/// `landfill_decay`: `zone`, `index` (the developer's tuning).
fn decay(w: &World, op: &Json) -> R<()> {
    let z = w.zone(op)?;
    let t = w.dev()?.tuning();
    let i = i64_of(field(op, "index")?, "index")? as usize;
    z.decay_garbage(i, t.decay_rate, t.max_decay_per_zone);
    Ok(())
}

/// `landfill_update_tiles`: `zone`, `index` (the heap that became active),
/// with a CRT seeded to `seed` (default 1); `draws` = the expected number of
/// `rand()` calls.
fn update_tiles(w: &World, op: &Json) -> R<()> {
    let z = w.zone(op)?;
    let seed = opt(op, "seed").map_or(Ok(1), |v| u32_of(v, "seed"))?;
    let start = CrtRand::new(seed);
    let mut rng = start;
    z.update_surrounding_tiles(i64_of(field(op, "index")?, "index")? as i32, &mut rng);
    if let Some(v) = opt(op, "draws") {
        let want = i64_of(v, "draws")?;
        let mut probe = start;
        let mut n = 0i64;
        while probe != rng {
            probe.rand();
            n += 1;
            if n > 1000 {
                return Err("more than 1000 draws".into());
            }
        }
        if n != want {
            return Err(format!("{n} rand() draws, expected {want}"));
        }
    }
    Ok(())
}

/// `landfill_levelling`: calls `AttemptAreaLevelling` (§4.5.3) over the
/// world's stubbed slots: `args` = [rect, f, withWater, waterFlag,
/// maxAreaSlope] (`"FLT_MAX"` and `"NaN"` allowed), `expect` = the result.
fn levelling(w: &World, op: &Json) -> R<()> {
    let a = arr(field(op, "args")?, "args")?;
    if a.len() != 5 {
        return Err("args: [rect, f, withWater, waterFlag, maxAreaSlope]".into());
    }
    let r = attempt_area_levelling(
        w,
        rect_of(&a[0])?,
        special_of(&a[1], "f")?,
        bool_of(&a[2], "withWater")?,
        bool_of(&a[3], "waterFlag")?,
        special_of(&a[4], "maxAreaSlope")?,
        0,
    );
    check_expect(op, Json::Bool(r))
}

/// `landfill_call`: `method` with `args`, `expect` = the result.
fn call(w: &World, op: &Json) -> R<()> {
    let d = w.dev()?;
    let method = str_of(field(op, "method")?, "method")?;
    let arg = |i: usize| scalar::arg(op, i);
    let got = match method {
        "get_zoning_cost" => {
            let r = rect_of(arg(0)?)?;
            let cost = d.get_zoning_cost(&CellRegion::all_set(r));
            Json::Array(vec![ji(cost as u32), ji((cost >> 32) as u32)])
        }
        "get_landfill_usage" => {
            let (u, c) = d.get_landfill_usage();
            Json::Array(vec![ji(u), ji(c)])
        }
        "get_landfill_tile_count" => ji(d.get_landfill_tile_count()),
        "get_maintenance_costs" => ji(d.get_maintenance_costs()),
        "get_current_budget" => ji(d.get_current_budget()),
        "get_efficiency" => jf(d.get_efficiency()),
        "update_capacity_values" => {
            d.update_capacity_values();
            Json::Null
        }
        "get_landfill_zone_from_cell" => {
            let [x, z] = ints::<2>(arg(0)?, "cell")?;
            match d.get_landfill_zone_from_cell(x as i32, z as i32) {
                None => Json::Null,
                Some(found) => ji(d
                    .zone_list()
                    .iter()
                    .position(|z| Rc::ptr_eq(z, &found))
                    .map_or(-1, |i| i as i64)),
            }
        }
        "recalculate_capacity" => ji(w.zone(op)?.recalculate_capacity(w.tile_cap.get())),
        "get_capacity" => {
            let (n, cap, used) = w.zone(op)?.get_capacity();
            Json::Array(vec![ji(n), ji(cap), ji(used)])
        }
        "is_zone_connected" => Json::Bool(w.zone(op)?.is_zone_connected()),
        other => return Err(format!("unknown method '{other}'")),
    };
    check_expect(op, got)
}

fn zone_state(w: &World, z: &LandfillZone) -> Vec<(String, Json)> {
    let zot = z
        .zot()
        .and_then(|t| w.zots.borrow().get(&t).copied())
        .map_or(Json::Null, |p| Json::Array(p.iter().map(|&v| jf(v)).collect()));
    let region = z.region();
    let mut mask: Vec<Json> = Vec::new();
    for (x, zc) in region.mask_cells() {
        mask.push(Json::Array(vec![ji(x), ji(zc)]));
    }
    vec![
        ("rect".into(), jrect(z.rect())),
        ("cells".into(), ji(z.cell_count())),
        ("mask".into(), Json::Array(mask)),
        ("w".into(), ji(z.w())),
        ("h".into(), ji(z.h())),
        ("heaps".into(), Json::Array(z.heaps().iter().map(jheap).collect())),
        ("valid_heaps".into(), ji(z.num_valid_heaps())),
        ("connected".into(), Json::Bool(z.connected())),
        ("zot".into(), zot),
        ("level_altitude".into(), jf(z.level_altitude())),
        ("completely_dezoned".into(), Json::Bool(z.is_completely_dezoned())),
    ]
}

/// `landfill_expect`: `state` (developer and world fields) and/or
/// `zone_state` (fields of zone `zone`). See README.md for the fields.
fn expect(w: &World, op: &Json) -> R<()> {
    if let Some(want) = opt(op, "state") {
        let mut got: Vec<(String, Json)> = vec![
            ("supply".into(), ji(w.supply.get())),
            ("tiles".into(), w.tiles()),
            (
                "active_landfill_cells".into(),
                ji(w.active_landfill.borrow().len() as i64),
            ),
            ("zots".into(), ji(w.zots.borrow().len() as i64)),
            ("posted".into(), ji(w.posted.borrow().len() as i64)),
        ];
        if let Ok(d) = w.dev() {
            let s = d.state();
            got.extend([
                ("zones".to_owned(), ji(d.zone_count() as i64)),
                ("maintenance".to_owned(), ji(s.maintenance)),
                ("capacity_total".to_owned(), ji(s.capacity_total)),
                ("used_total".to_owned(), ji(s.used_total)),
                ("loaded".to_owned(), Json::Bool(s.loaded)),
                ("department".to_owned(), Json::Bool(s.department.is_some())),
                ("crt_state".to_owned(), ji(d.crt().state())),
            ]);
        }
        check_state(&got, want)?;
    }
    if let Some(want) = opt(op, "zone_state") {
        let z = w.zone(op)?;
        check_state(&zone_state(w, &z), want)?;
    }
    if let Some(v) = opt(op, "zone_types") {
        for c in arr(v, "zone_types")? {
            let [x, z, t] = ints::<3>(c, "zone_types")?;
            let got = w.zone_type(x as i32, z as i32);
            if i64::from(got) != t {
                return Err(format!("zone type at ({x}, {z}) is {got}, expected {t}"));
            }
        }
    }
    Ok(())
}

/// `landfill_record`: the developer's record (§6) as `hex`, or its first
/// bytes as `hex_prefix`.
fn record(w: &World, op: &Json) -> R<()> {
    let got = scalar::record_bytes(&*w.dev()?);
    if let Some(v) = opt(op, "hex") {
        let want = hex_decode(str_of(v, "hex")?)?;
        if got != want {
            return Err(format!(
                "record differs:\n    got      {}\n    expected {}",
                hex_encode(&got),
                hex_encode(&want)
            ));
        }
    }
    if let Some(v) = opt(op, "hex_prefix") {
        let want = hex_decode(str_of(v, "hex_prefix")?)?;
        if got.len() < want.len() || got[..want.len()] != want[..] {
            return Err(format!(
                "record prefix differs:\n    got      {}\n    expected {}",
                hex_encode(&got[..got.len().min(want.len() + 8)]),
                hex_encode(&want)
            ));
        }
    }
    Ok(())
}

/// `landfill_zone_record`: reads `read` (a §6.3 zone record) into a fresh
/// zone (`expect` = the result); with `rewrite` the zone's write must
/// reproduce the bytes; with `init` the zone is initialised; `zone_state`
/// checks its fields.
fn zone_record(w: &World, op: &Json) -> R<()> {
    let bytes = hex_decode(str_of(field(op, "read")?, "read")?)?;
    let z = LandfillZone::empty(w.rc()? as Rc<dyn LandfillHost>);
    let ok = z.read(&mut ByteReader::new(&bytes));
    check_expect(op, Json::Bool(ok))?;
    if opt(op, "rewrite").map_or(Ok(false), |v| bool_of(v, "rewrite"))? {
        let mut out = ByteWriter::new();
        z.write(&mut out);
        if out.bytes() != &bytes[..] {
            return Err(format!(
                "rewrite differs:\n    got      {}\n    expected {}",
                hex_encode(out.bytes()),
                hex_encode(&bytes)
            ));
        }
    }
    if opt(op, "init").map_or(Ok(false), |v| bool_of(v, "init"))? {
        z.init();
    }
    if let Some(want) = opt(op, "zone_state") {
        check_state(&zone_state(w, &z), want)?;
    }
    Ok(())
}

/// `landfill_reload`: writes the developer's record and reads it into a new
/// developer that replaces it (`expect` = the read result, default true);
/// PostCityInit follows when `post_city_init` is true. Zones with a zot
/// cannot be reloaded (the record holds a reference).
fn reload(w: &World, op: &Json) -> R<()> {
    let mut out = ByteWriter::new();
    if !w.dev()?.write(&mut out) {
        return Err("write failed (a zot reference?)".into());
    }
    let token = w.next_dev.get();
    w.next_dev.set(token + 1);
    let d = LandfillDeveloper::new(w.rc()? as Rc<dyn LandfillHost>, token);
    d.init();
    let ok = d.read(&mut ByteReader::new(out.bytes()));
    let want = opt(op, "expect").map_or(Ok(true), |v| boolean(v).ok_or("'expect' must be a boolean"))?;
    if ok != want {
        return Err(format!("read returned {ok}, expected {}", show(&Json::Bool(want))));
    }
    *w.dev.borrow_mut() = Some(d.clone());
    if opt(op, "post_city_init").map_or(Ok(false), |v| bool_of(v, "post_city_init"))? {
        d.do_message(&LandfillMessage::of_type(msg::POST_CITY_INIT));
    }
    Ok(())
}
