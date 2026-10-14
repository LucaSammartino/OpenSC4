// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops `ldev_…` for `fc4_core::lot_developer`
//! (`docs/specs/lot-developer.md`): a scripted world (lots, lot
//! configurations and their objects, buildings and other occupants, growth
//! developers, business units, cell grids, terrain, resources, cheats)
//! implementing `LotDeveloperHost`, an event log of the host calls that
//! change something, the draw trace of the shared statics, and ops that
//! drive the lot developer. See README.md ("Lot developer ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::{Rc, Weak};

use fc4_core::development::BuildingProfile;
use fc4_core::lot_developer::{
    answers_iid, is_empty_lot_suitable_for_zot, lot_prop, lot_state, reason, rubble_count,
    BurntBuilding, CellRect, CfgObject, Cheat, Draw, DrawKind, LotDevMessage, LotDeveloper, LotDeveloperHost,
    LotDeveloperStatics, SharedStatics, TerrainView, TextureRecord, Tuning, Vec3, FLT_MAX, IID_BUILDING_OCCUPANT,
    OCCUPANT_TYPE_BASE_TEXTURE, OCCUPANT_TYPE_BUILDING, OCCUPANT_TYPE_FOUNDATION, OCCUPANT_TYPE_PROP,
};
use fc4_core::x87::{ftol32, r24};
use fc4_core::property::PropertyHolder;
use fc4_core::rng::{CrtRand, FixedSeeds, RzRandom};
use fc4_core::stream::{ByteReader, ByteWriter, RefResolver, RefValue, SaveRegistry};
use fc4_core::variant::{mem, Elem, Variant};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{boolean, field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, f32_of, i64_of, ji, matches, opt, show, str_of, u32_of, EventLog, R,
};

const SEEDS: FixedSeeds = FixedSeeds {
    wall_clock: 1,
    process_clock: 1,
};

/// The default file-RNG seed for the vectors (§16 "seeded explicitly").
const DEFAULT_SEED: u32 = 12345;

// ---- world -------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Lot {
    x: i32,
    z: i32,
    w: u32,
    h: u32,
    zone: i32,
    state: u32,
    livable: bool,
    condition: u32,
    days: u32,
    building_type: u32,
    building: Option<u64>,
    cfg: Option<u64>,
    wealth: u32,
    occupancy: f32,
    needs_power: bool,
    has_power: bool,
    road: bool,
    job: bool,
    organic: bool,
    flags: u32,
    historical: bool,
    initialized: bool,
    travel_jobs: [f32; 4],
    designate: Option<u64>,
    unit: Option<u64>,
    available: u16,
    props: BTreeMap<u32, u32>,
    /// Lot slots 16 / 17 (§17.5, §17.6).
    altitude: f32,
    /// Lot slot 23 (§17.6, §17.7).
    facing: i32,
    /// Lot slot 68 `GetVerticesAreLocked` (§17.3).
    locked: bool,
}

impl Default for Lot {
    fn default() -> Lot {
        Lot {
            x: 0,
            z: 0,
            w: 1,
            h: 1,
            zone: 1,
            state: lot_state::READY,
            livable: true,
            condition: 0,
            days: 0,
            building_type: 0,
            building: None,
            cfg: None,
            wealth: 1,
            occupancy: 1.0,
            needs_power: true,
            has_power: false,
            road: true,
            job: true,
            organic: true,
            flags: 0,
            historical: false,
            initialized: true,
            travel_jobs: [0.0, 1.0, 0.0, 0.0],
            designate: None,
            unit: None,
            available: 10,
            props: BTreeMap::new(),
            altitude: 0.0,
            facing: 0,
            locked: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct Cfg {
    id: u32,
    w: u32,
    h: u32,
    stage: u8,
    water: bool,
    max_slope: f32,
    objects: Vec<CfgObject>,
    primary_building: Option<CfgObject>,
    primary_foundation: Option<CfgObject>,
    water_objects: bool,
}

#[derive(Clone, Debug)]
struct Occ {
    bbox: (Vec3, Vec3),
    cells: CellRect,
    group: u32,
    lit: bool,
    is_building: bool,
    position: Vec3,
    orientation: u32,
    props: BTreeMap<u32, u32>,
    /// For `Light`: whether the prop carries a true `0x4A9F188B`.
    night_light: bool,
    /// The lot this prop belongs to (for `GetLotProps`).
    lot: Option<u64>,
    /// `Some(kind)` for a foundation occupant (§17.5).
    foundation_kind: Option<i32>,
    foundation_type: u32,
    /// The occupant type, when not implied (building, foundation, prop).
    occ_type: Option<u32>,
    /// Occupant slot 32 `IsFlagSet(1)`: inserted in the occupant manager.
    inserted: bool,
    /// Listed by the demolition filter as immovable (§17.3).
    immovable: bool,
}

impl Default for Occ {
    fn default() -> Occ {
        Occ {
            bbox: ([0.0; 3], [0.0; 3]),
            cells: CellRect::new(0, 0, 0, 0),
            group: 0,
            lit: false,
            is_building: false,
            position: [0.0; 3],
            orientation: 0,
            props: BTreeMap::new(),
            night_light: false,
            lot: None,
            foundation_kind: None,
            foundation_type: 0,
            occ_type: None,
            inserted: true,
            immovable: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct Profile {
    purpose: u8,
    wealth: u8,
    capacity: u16,
}

#[derive(Clone, Debug, Default)]
struct Dev {
    key: u32,
    purpose: u32,
    desirability: i16,
    water_cap: u8,
}

pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    sim: RefCell<Option<LotDeveloper>>,
    statics: RefCell<Option<SharedStatics>>,
    lots: RefCell<BTreeMap<u64, Lot>>,
    order: RefCell<Vec<u64>>,
    cfgs: RefCell<BTreeMap<u64, Cfg>>,
    occs: RefCell<BTreeMap<u64, Occ>>,
    profiles: RefCell<BTreeMap<u32, Profile>>,
    families: RefCell<BTreeMap<u32, Vec<u32>>>,
    valid_building: RefCell<BTreeSet<u32>>,
    valid_flora: RefCell<BTreeSet<u32>>,
    devs: RefCell<BTreeMap<u64, Dev>>,
    units: RefCell<BTreeMap<u64, Vec<u64>>>,
    textures: RefCell<Vec<TextureRecord>>,
    resources: RefCell<BTreeSet<u32>>,
    powered: RefCell<BTreeMap<(i32, i32), bool>>,
    watered: RefCell<BTreeMap<(i32, i32), bool>>,
    trip: RefCell<BTreeMap<(i32, i32), u8>>,
    terrain_alt: RefCell<BTreeMap<i32, f32>>,
    terrain_water: RefCell<BTreeMap<i32, f32>>,
    /// The altitude of every vertex not in `terrain_alt` (default 100).
    flat_alt: Cell<f32>,
    /// `GetAltitude(x, z)` / `GetWaterLevel(x, z)` at given positions (by
    /// the bits of x and z); other positions interpolate the vertices / 0.
    pos_alt: RefCell<BTreeMap<(u32, u32), f32>>,
    pos_water: RefCell<BTreeMap<(u32, u32), f32>>,
    water_vertices: RefCell<BTreeSet<(i32, i32)>>,
    water_cells: RefCell<BTreeSet<(i32, i32)>>,
    /// Log every terrain read as an event (`alt_vertex`, `water_vertex`,
    /// `alt_at`, `water_at`).
    trace_terrain: Cell<bool>,
    /// The lot developer's terrain view while it is busy, for the BDS model
    /// (§17.5.3 calls back slot 17).
    view: RefCell<Option<TerrainView>>,
    zone_color: Cell<u32>,
    zone_alpha: Cell<u8>,
    base_texture_fails: Cell<bool>,
    foundation_fails: Cell<bool>,
    surrounding: RefCell<BTreeMap<u64, Vec<u64>>>,
    surrounding_rect: RefCell<Vec<u64>>,
    lots_in_rect: RefCell<Vec<u64>>,
    iters: RefCell<BTreeMap<u64, Cell<usize>>>,
    next_token: Cell<u64>,
    iter_next: Cell<u64>,
    // world state
    city_cells: Cell<(i32, i32)>,
    cell_widths: Cell<(f32, f32)>,
    terrain_cells: Cell<(i32, i32)>,
    terrain_width: Cell<f32>,
    sim_date: Cell<u32>,
    time_mode: Cell<u32>,
    view_rotation: Cell<i32>,
    jobs_demand: Cell<Option<f32>>,
    cheats: Cell<[bool; 3]>,
    no_services: Cell<bool>,
    no_simulator: Cell<bool>,
    no_message_server: Cell<bool>,
    claim_resources: Cell<bool>,
    add_building_fails: Cell<bool>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            me: RefCell::new(Weak::new()),
            events: EventLog::default(),
            sim: RefCell::new(None),
            statics: RefCell::new(None),
            lots: RefCell::new(BTreeMap::new()),
            order: RefCell::new(Vec::new()),
            cfgs: RefCell::new(BTreeMap::new()),
            occs: RefCell::new(BTreeMap::new()),
            profiles: RefCell::new(BTreeMap::new()),
            families: RefCell::new(BTreeMap::new()),
            valid_building: RefCell::new(BTreeSet::new()),
            valid_flora: RefCell::new(BTreeSet::new()),
            devs: RefCell::new(BTreeMap::new()),
            units: RefCell::new(BTreeMap::new()),
            textures: RefCell::new(Vec::new()),
            resources: RefCell::new(BTreeSet::new()),
            powered: RefCell::new(BTreeMap::new()),
            watered: RefCell::new(BTreeMap::new()),
            trip: RefCell::new(BTreeMap::new()),
            terrain_alt: RefCell::new(BTreeMap::new()),
            terrain_water: RefCell::new(BTreeMap::new()),
            flat_alt: Cell::new(100.0),
            pos_alt: RefCell::new(BTreeMap::new()),
            pos_water: RefCell::new(BTreeMap::new()),
            water_vertices: RefCell::new(BTreeSet::new()),
            water_cells: RefCell::new(BTreeSet::new()),
            trace_terrain: Cell::new(false),
            view: RefCell::new(None),
            zone_color: Cell::new(0xFFB2_3300),
            zone_alpha: Cell::new(0xAA),
            base_texture_fails: Cell::new(false),
            foundation_fails: Cell::new(false),
            surrounding: RefCell::new(BTreeMap::new()),
            surrounding_rect: RefCell::new(Vec::new()),
            lots_in_rect: RefCell::new(Vec::new()),
            iters: RefCell::new(BTreeMap::new()),
            next_token: Cell::new(1),
            iter_next: Cell::new(0x9000),
            city_cells: Cell::new((64, 64)),
            cell_widths: Cell::new((16.0, 16.0)),
            terrain_cells: Cell::new((64, 64)),
            terrain_width: Cell::new(16.0),
            sim_date: Cell::new(0),
            time_mode: Cell::new(0),
            view_rotation: Cell::new(0),
            jobs_demand: Cell::new(None),
            cheats: Cell::new([false; 3]),
            no_services: Cell::new(false),
            no_simulator: Cell::new(false),
            no_message_server: Cell::new(false),
            claim_resources: Cell::new(true),
            add_building_fails: Cell::new(false),
            tuning: RefCell::new(Some(Rc::new(Tuning::shipped_exemplar()))),
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

    fn with_sim<T>(&self, f: impl FnOnce(&mut LotDeveloper) -> R<T>) -> R<T> {
        let mut s = self
            .sim
            .borrow_mut()
            .take()
            .ok_or("no lot developer (use ldev_create)")?;
        *self.view.borrow_mut() = Some(s.terrain_view());
        let r = f(&mut s);
        *self.sim.borrow_mut() = Some(s);
        r
    }

    // ---- terrain model ----

    fn verts_x(&self) -> i32 {
        self.terrain_cells.get().0 + 1
    }

    fn vertex_alt(&self, x: i32, z: i32) -> f32 {
        let i = x.wrapping_add(z.wrapping_mul(self.verts_x()));
        self.terrain_alt.borrow().get(&i).copied().unwrap_or(self.flat_alt.get())
    }

    fn set_vertex_alt(&self, x: i32, z: i32, h: f32) {
        let i = x.wrapping_add(z.wrapping_mul(self.verts_x()));
        self.terrain_alt.borrow_mut().insert(i, h);
    }

    /// `GetAltitude(x, z)` where no position is scripted: the vertices
    /// interpolated bilinearly (flat terrain stays flat).
    fn interpolated_alt(&self, x: f32, z: f32) -> f32 {
        let cw = f64::from(self.terrain_width.get());
        let fx = f64::from(x) / cw;
        let fz = f64::from(z) / cw;
        let (ix, iz) = (fx.floor(), fz.floor());
        let (tx, tz) = (fx - ix, fz - iz);
        let (ix, iz) = (ix as i32, iz as i32);
        let a = |dx: i32, dz: i32| f64::from(self.vertex_alt(ix + dx, iz + dz));
        let v = a(0, 0) * (1.0 - tx) * (1.0 - tz) + a(1, 0) * tx * (1.0 - tz) + a(0, 1) * (1.0 - tx) * tz + a(1, 1) * tx * tz;
        v as f32
    }

    fn trace(&self, kind: &str, fields: Vec<(&str, Json)>) {
        if self.trace_terrain.get() {
            self.events.push(kind, fields);
        }
    }

    fn lot_rect(l: &Lot) -> CellRect {
        CellRect::new(l.x, l.z, l.x + l.w as i32 - 1, l.z + l.h as i32 - 1)
    }

    /// A lot other than `exempt` with locked vertices on a cell of `r`.
    fn locked_lot_in(&self, r: CellRect, exempt: Option<u64>) -> bool {
        self.lots
            .borrow()
            .iter()
            .any(|(&t, l)| l.locked && Some(t) != exempt && overlaps(Self::lot_rect(l), r))
    }

    /// The bounding boxes of the immovable occupants on the cells of `r`.
    fn immovable_in(&self, r: CellRect) -> Vec<(Vec3, Vec3)> {
        self.occs
            .borrow()
            .values()
            .filter(|o| o.immovable && overlaps(o.cells, r))
            .map(|o| o.bbox)
            .collect()
    }

    fn type_of(o: &Occ) -> u32 {
        if let Some(t) = o.occ_type {
            t
        } else if o.foundation_kind.is_some() {
            OCCUPANT_TYPE_FOUNDATION
        } else if o.is_building {
            OCCUPANT_TYPE_BUILDING
        } else {
            OCCUPANT_TYPE_PROP
        }
    }

    fn lot<T: Default>(&self, l: u64, f: impl FnOnce(&Lot) -> T) -> T {
        self.lots.borrow().get(&l).map(f).unwrap_or_default()
    }

    fn lot_mut(&self, l: u64, f: impl FnOnce(&mut Lot)) {
        if let Some(lot) = self.lots.borrow_mut().get_mut(&l) {
            f(lot);
        }
    }

    fn occ<T: Default>(&self, o: u64, f: impl FnOnce(&Occ) -> T) -> T {
        self.occs.borrow().get(&o).map(f).unwrap_or_default()
    }

    fn occ_mut(&self, o: u64, f: impl FnOnce(&mut Occ)) {
        if let Some(occ) = self.occs.borrow_mut().get_mut(&o) {
            f(occ);
        }
    }
}

fn overlaps(a: CellRect, b: CellRect) -> bool {
    a.x1 <= b.x2 && b.x1 <= a.x2 && a.z1 <= b.z2 && b.z1 <= a.z2
}

fn pos_key(x: f32, z: f32) -> (u32, u32) {
    (x.to_bits(), z.to_bits())
}

fn jf(v: f32) -> Json {
    Json::Float(f64::from(v))
}

fn rect_json(r: CellRect) -> Json {
    Json::Array(vec![ji(r.x1), ji(r.z1), ji(r.x2), ji(r.z2)])
}

fn pos_json(p: Vec3) -> Json {
    Json::Array(vec![
        Json::Float(f64::from(p[0])),
        Json::Float(f64::from(p[1])),
        Json::Float(f64::from(p[2])),
    ])
}

impl LotDeveloperHost for World {
    fn acquire_services(&self) -> bool {
        self.events.push("acquire_services", vec![]);
        !self.no_services.get()
    }
    fn release_services(&self) {
        self.events.push("release_services", vec![]);
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn has_simulator(&self) -> bool {
        !self.no_simulator.get()
    }
    fn add_agent(&self, level: u32, message: u32) {
        self.events
            .push("add_agent", vec![("level", ji(level)), ("message", ji(message))]);
    }
    fn remove_agent(&self) {
        self.events.push("remove_agent", vec![]);
    }
    fn acquire_message_server(&self) -> bool {
        self.events.push("acquire_message_server", vec![]);
        !self.no_message_server.get()
    }
    fn release_message_server(&self) {
        self.events.push("release_message_server", vec![]);
    }
    fn subscribe(&self, message: u32) {
        self.events.push("subscribe", vec![("message", ji(message))]);
    }
    fn unsubscribe(&self, message: u32) {
        self.events.push("unsubscribe", vec![("message", ji(message))]);
    }
    fn register_animation_tick(&self) {
        self.events.push("register_tick", vec![]);
    }
    fn unregister_animation_tick(&self) {
        self.events.push("unregister_tick", vec![]);
    }
    fn post_message(&self, msg_type: u32, data1: u32, lot: Option<u64>) {
        self.events.push(
            "post",
            vec![
                ("message", ji(msg_type)),
                ("data1", ji(data1)),
                ("lot", lot.map_or(Json::Null, |l| ji(l as i64))),
            ],
        );
    }

    fn city_cell_counts(&self) -> (i32, i32) {
        self.city_cells.get()
    }
    fn city_cell_widths(&self) -> (f32, f32) {
        self.cell_widths.get()
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (bool, i32, i32) {
        let (cx, cz) = self.cell_widths.get();
        (true, (x / cx).floor() as i32, (z / cz).floor() as i32)
    }
    fn cell_corner_to_position(&self, x: i32, z: i32) -> (f32, f32) {
        let (cx, cz) = self.cell_widths.get();
        (x as f32 * cx, z as f32 * cz)
    }
    fn cell_center_to_position(&self, x: i32, z: i32) -> (f32, f32) {
        let (cx, cz) = self.cell_widths.get();
        ((x as f32 + 0.5) * cx, (z as f32 + 0.5) * cz)
    }
    fn sim_date(&self) -> u32 {
        self.sim_date.get()
    }
    fn terrain_cell_counts(&self) -> (i32, i32) {
        self.terrain_cells.get()
    }
    fn terrain_cell_width(&self) -> f32 {
        self.terrain_width.get()
    }
    fn terrain_altitude(&self, index: i32) -> f32 {
        self.trace("alt_vertex", vec![("index", ji(index))]);
        self.terrain_alt.borrow().get(&index).copied().unwrap_or(self.flat_alt.get())
    }
    fn terrain_water_level(&self, index: i32) -> f32 {
        self.trace("water_vertex", vec![("index", ji(index))]);
        self.terrain_water.borrow().get(&index).copied().unwrap_or(0.0)
    }
    fn terrain_altitude_at(&self, x: f32, z: f32) -> f32 {
        self.trace("alt_at", vec![("x", jf(x)), ("z", jf(z))]);
        let p = self.pos_alt.borrow().get(&pos_key(x, z)).copied();
        p.unwrap_or_else(|| self.interpolated_alt(x, z))
    }
    fn terrain_water_level_at(&self, x: f32, z: f32) -> f32 {
        self.trace("water_at", vec![("x", jf(x)), ("z", jf(z))]);
        self.pos_water.borrow().get(&pos_key(x, z)).copied().unwrap_or(0.0)
    }
    fn terrain_is_water_vertex(&self, x: i32, z: i32) -> bool {
        self.water_vertices.borrow().contains(&(x, z))
    }
    fn terrain_cell_has_water(&self, x: i32, z: i32) -> bool {
        self.water_cells.borrow().contains(&(x, z))
    }
    fn zone_drag_color(&self, _zone: i32) -> u32 {
        self.zone_color.get()
    }
    fn zone_display_alpha(&self) -> u8 {
        self.zone_alpha.get()
    }
    fn view_rotation(&self) -> i32 {
        self.view_rotation.get()
    }
    fn visual_time_mode(&self) -> u32 {
        self.time_mode.get()
    }
    fn resource_exists(&self, _t: u32, _g: u32, instance: u32) -> bool {
        self.resources.borrow().contains(&instance)
    }

    fn lot_count(&self) -> i32 {
        self.order.borrow().len() as i32
    }
    fn safe_lot_iterator(&self) -> u64 {
        let t = self.iter_next.get();
        self.iter_next.set(t + 1);
        self.iters.borrow_mut().insert(t, Cell::new(0));
        t
    }
    fn safe_lot_iterator_rewind(&self, it: u64) {
        if let Some(c) = self.iters.borrow().get(&it) {
            c.set(0);
        }
    }
    fn safe_lot_iterator_next(&self, it: u64) -> Option<u64> {
        let idx = self.iters.borrow().get(&it)?.get();
        let lot = *self.order.borrow().get(idx)?;
        self.iters.borrow().get(&it)?.set(idx + 1);
        Some(lot)
    }
    fn safe_lot_iterator_free(&self, it: u64) {
        self.iters.borrow_mut().remove(&it);
    }
    fn get_lot(&self, x: i32, z: i32, origin: bool) -> Option<u64> {
        // Lot manager slot 6: the lot on cell (x, z) (its origin only when
        // `origin`), first in lot-list order.
        let order = self.order.borrow();
        let lots = self.lots.borrow();
        let cell = CellRect::new(x, z, x, z);
        order.iter().copied().find(|&t| {
            lots.get(&t).is_some_and(|l| {
                (l.x == x && l.z == z) || (!origin && overlaps(World::lot_rect(l), cell))
            })
        })
    }
    fn lots_in_cell_rect(&self, _rect: CellRect, _origin: bool) -> Vec<u64> {
        self.lots_in_rect.borrow().clone()
    }
    fn lots_surrounding_lot(&self, lot: u64, list: &mut Vec<u64>, _flags: u32) {
        if let Some(v) = self.surrounding.borrow().get(&lot) {
            for &n in v {
                if !list.contains(&n) {
                    list.push(n);
                }
            }
        }
    }
    fn lots_surrounding_rect(&self, _rect: CellRect, list: &mut Vec<u64>, _flags: u32) {
        for &n in self.surrounding_rect.borrow().iter() {
            if !list.contains(&n) {
                list.push(n);
            }
        }
    }
    fn delete_lot(&self, lot: u64) -> bool {
        self.events.push("delete_lot", vec![("lot", ji(lot as i64))]);
        self.order.borrow_mut().retain(|&l| l != lot);
        self.lots.borrow_mut().remove(&lot);
        true
    }

    fn lot_is_initialized(&self, lot: u64) -> bool {
        self.lot(lot, |l| l.initialized)
    }
    fn lot_location(&self, lot: u64) -> (i32, i32) {
        self.lot(lot, |l| (l.x, l.z))
    }
    fn lot_size(&self, lot: u64) -> (u32, u32) {
        self.lot(lot, |l| (l.w, l.h))
    }
    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        self.lot(lot, |l| {
            CellRect::new(l.x, l.z, l.x + l.w as i32 - 1, l.z + l.h as i32 - 1)
        })
    }
    fn lot_center(&self, lot: u64) -> Vec3 {
        // Lot slot 15: `(8·(x1+x2+1), altitude, 8·(z1+z2+1))` (§17.5.4).
        self.lot(lot, |l| {
            let r = World::lot_rect(l);
            [
                8.0 * (r.x1 + r.x2 + 1) as f32,
                l.altitude,
                8.0 * (r.z1 + r.z2 + 1) as f32,
            ]
        })
    }
    fn lot_zone_type(&self, lot: u64) -> i32 {
        self.lot(lot, |l| l.zone)
    }
    fn lot_altitude(&self, lot: u64) -> f32 {
        self.lot(lot, |l| l.altitude)
    }
    fn lot_set_altitude(&self, lot: u64, altitude: f32) {
        self.lot_mut(lot, |l| l.altitude = altitude);
    }
    fn lot_facing(&self, lot: u64) -> i32 {
        self.lot(lot, |l| l.facing)
    }
    fn lot_flags(&self, lot: u64) -> u32 {
        self.lot(lot, |l| l.flags)
    }
    fn lot_set_flags(&self, lot: u64, flags: u32) {
        self.lot_mut(lot, |l| l.flags = flags);
    }
    fn lot_set_vertices_locked(&self, lot: u64, locked: bool) {
        self.events.push(
            "vertices_locked",
            vec![("lot", ji(lot as i64)), ("locked", Json::Bool(locked))],
        );
        self.lot_mut(lot, |l| l.locked = locked);
    }
    fn lot_update_slope(&self, lot: u64) {
        self.events.push("update_slope", vec![("lot", ji(lot as i64))]);
    }
    fn lot_state(&self, lot: u64) -> u32 {
        self.lot(lot, |l| l.state)
    }
    fn lot_set_state(&self, lot: u64, state: u32, wealth: u32, why: u32) -> bool {
        self.events.push(
            "set_state",
            vec![
                ("lot", ji(lot as i64)),
                ("state", ji(state)),
                ("wealth", ji(wealth)),
                ("reason", ji(why)),
            ],
        );
        self.lot_mut(lot, |l| l.state = state);
        true
    }
    fn lot_condition(&self, lot: u64) -> u32 {
        self.lot(lot, |l| l.condition)
    }
    fn lot_set_condition(&self, lot: u64, condition: u32) {
        self.events.push(
            "set_condition",
            vec![("lot", ji(lot as i64)), ("condition", ji(condition))],
        );
        self.lot_mut(lot, |l| l.condition = condition);
    }
    fn lot_is_livable(&self, lot: u64) -> bool {
        self.lot(lot, |l| l.livable)
    }
    fn lot_days_since_state_change(&self, lot: u64) -> u32 {
        self.lot(lot, |l| l.days)
    }
    fn lot_building_type(&self, lot: u64, _flag: bool) -> u32 {
        self.lot(lot, |l| l.building_type)
    }
    fn lot_set_building_type(&self, lot: u64, building: u32) {
        self.lot_mut(lot, |l| l.building_type = building);
    }
    fn lot_building(&self, lot: u64) -> Option<u64> {
        self.lot(lot, |l| l.building)
    }
    fn lot_configuration(&self, lot: u64) -> Option<u64> {
        self.lot(lot, |l| l.cfg)
    }
    fn lot_set_configuration(&self, lot: u64, cfg: Option<u64>) {
        self.lot_mut(lot, |l| l.cfg = cfg);
    }
    fn lot_occupant_wealth(&self, lot: u64) -> u32 {
        self.lot(lot, |l| l.wealth)
    }
    fn lot_occupancy(&self, lot: u64) -> f32 {
        self.lot(lot, |l| l.occupancy)
    }
    fn lot_needs_power(&self, lot: u64) -> bool {
        self.lot(lot, |l| l.needs_power)
    }
    fn lot_has_power(&self, lot: u64) -> bool {
        self.lot(lot, |l| l.has_power)
    }
    fn lot_set_has_power(&self, lot: u64, powered: bool) {
        self.lot_mut(lot, |l| l.has_power = powered);
    }
    fn lot_has_job_access(&self, lot: u64) -> bool {
        self.lot(lot, |l| l.job)
    }
    fn lot_has_road_access(&self, lot: u64) -> bool {
        self.lot(lot, |l| l.road)
    }
    fn lot_has_organic_road_access(&self, lot: u64, _purpose: u32) -> bool {
        self.lot(lot, |l| l.organic)
    }
    fn lot_update_road_access(&self, lot: u64) {
        self.events.push("update_road_access", vec![("lot", ji(lot as i64))]);
    }
    fn lot_travel_jobs(&self, lot: u64, wealth: u32) -> f32 {
        self.lot(lot, |l| l.travel_jobs[wealth.min(3) as usize])
    }
    fn lot_travel_designate(&self, lot: u64) -> Option<u64> {
        self.lot(lot, |l| l.designate)
    }
    fn lot_set_travel_designate(&self, lot: u64, designate: Option<u64>) {
        self.lot_mut(lot, |l| l.designate = designate);
    }
    fn lot_unit(&self, lot: u64) -> Option<u64> {
        self.lot(lot, |l| l.unit)
    }
    fn lot_reset_travel_population(&self, lot: u64) {
        self.events.push("reset_travel_pop", vec![("lot", ji(lot as i64))]);
    }
    fn lot_add_travel_population(&self, lot: u64, from: u64) {
        self.events.push(
            "add_travel_pop",
            vec![("lot", ji(lot as i64)), ("from", ji(from as i64))],
        );
    }
    fn lot_is_historical(&self, lot: u64) -> bool {
        self.lot(lot, |l| l.historical)
    }
    fn lot_set_historical(&self, lot: u64, historical: bool) {
        self.lot_mut(lot, |l| l.historical = historical);
    }
    fn lot_available_capacity(&self, lot: u64, _dev: u32, _flag: bool) -> u16 {
        self.lot(lot, |l| l.available)
    }
    fn lot_set_population(&self, lot: u64, dev: u32, population: u16) -> bool {
        self.events.push(
            "set_population",
            vec![("lot", ji(lot as i64)), ("dev", ji(dev)), ("n", ji(population))],
        );
        true
    }
    fn lot_property(&self, lot: u64, id: u32) -> Option<u32> {
        self.lots.borrow().get(&lot).and_then(|l| l.props.get(&id).copied())
    }
    fn lot_set_property(&self, lot: u64, id: u32, value: u32) {
        self.lot_mut(lot, |l| {
            l.props.insert(id, value);
        });
    }

    fn cfg_id(&self, cfg: u64) -> u32 {
        self.cfgs.borrow().get(&cfg).map_or(0, |c| c.id)
    }
    fn cfg_size(&self, cfg: u64) -> (u32, u32) {
        self.cfgs.borrow().get(&cfg).map_or((0, 0), |c| (c.w, c.h))
    }
    fn cfg_max_slope_before_foundation(&self, cfg: u64) -> f32 {
        self.cfgs.borrow().get(&cfg).map_or(0.0, |c| c.max_slope)
    }
    fn cfg_growth_stage(&self, cfg: u64) -> u8 {
        self.cfgs.borrow().get(&cfg).map_or(0, |c| c.stage)
    }
    fn cfg_is_compatible_with_water(&self, cfg: u64) -> bool {
        self.cfgs.borrow().get(&cfg).is_some_and(|c| c.water)
    }
    fn cfg_objects(&self, cfg: u64) -> Vec<CfgObject> {
        self.cfgs.borrow().get(&cfg).map_or(Vec::new(), |c| c.objects.clone())
    }
    fn cfg_primary_building_object(&self, cfg: u64) -> Option<CfgObject> {
        self.cfgs.borrow().get(&cfg).and_then(|c| c.primary_building.clone())
    }
    fn cfg_primary_foundation_object(&self, cfg: u64) -> Option<CfgObject> {
        self.cfgs.borrow().get(&cfg).and_then(|c| c.primary_foundation.clone())
    }
    fn cfg_has_objects_of_type(&self, cfg: u64, object_type: u32) -> bool {
        if object_type == fc4_core::lot_developer::object_type::WATER {
            return self.cfgs.borrow().get(&cfg).is_some_and(|c| c.water_objects);
        }
        self.cfgs
            .borrow()
            .get(&cfg)
            .is_some_and(|c| c.objects.iter().any(|o| o.object_type == object_type))
    }
    fn get_lot_configuration(&self, id: u32) -> Option<u64> {
        self.cfgs.borrow().iter().find(|(_, c)| c.id == id).map(|(&t, _)| t)
    }
    fn object_id_array(&self, cfg: u64, index: usize) -> Vec<u32> {
        self.cfgs
            .borrow()
            .get(&cfg)
            .and_then(|c| c.objects.get(index))
            .map_or(Vec::new(), |o| o.ids.clone())
    }
    fn first_building(&self, lot: u64) -> Option<u64> {
        self.lot(lot, |l| l.building)
    }
    fn update_lot_retaining_walls(&self, lot: u64) -> bool {
        self.events
            .push("update_retaining_walls", vec![("lot", ji(lot as i64))]);
        true
    }
    fn update_lot_object_altitudes(&self, lot: u64) {
        self.events
            .push("update_object_altitudes", vec![("lot", ji(lot as i64))]);
    }
    fn update_lot_base_texture_occupant(&self, lot: u64, had: bool) -> bool {
        self.events.push(
            "update_base_texture_occupant",
            vec![("lot", ji(lot as i64)), ("had_foundation", Json::Bool(had))],
        );
        true
    }
    fn base_texture_occupant(&self, lot: u64, _create: bool) -> Option<u64> {
        let own = self
            .occs
            .borrow()
            .iter()
            .find(|(_, o)| o.occ_type == Some(OCCUPANT_TYPE_BASE_TEXTURE) && o.lot == Some(lot))
            .map(|(&t, _)| t);
        if own.is_some() {
            return own;
        }
        (!self.base_texture_fails.get()).then_some(0xB0)
    }
    fn base_texture_set(&self, occ: u64, records: &[TextureRecord]) {
        self.events.push(
            "set_textures",
            vec![("occ", ji(occ as i64)), ("n", ji(records.len() as i64))],
        );
        for r in records {
            self.events.push(
                "set_texture",
                vec![
                    ("texture", ji(r.texture)),
                    ("x", ji(r.x)),
                    ("z", ji(r.z)),
                    ("orientation", ji(r.orientation)),
                    ("byte3", ji(r.byte3)),
                    ("extra", Json::Array(r.extra.iter().map(|&b| ji(b)).collect())),
                    ("frame", ji(r.frame)),
                ],
            );
        }
        self.textures.borrow_mut().extend_from_slice(records);
    }
    fn add_retaining_wall_occupant(&self, lot: u64, flag: u32) {
        self.events
            .push("retaining_wall", vec![("lot", ji(lot as i64)), ("flag", ji(flag))]);
    }
    fn update_surrounding_retaining_walls(&self, lot: u64) {
        self.events
            .push("update_surrounding_walls", vec![("lot", ji(lot as i64))]);
    }
    fn add_network_connection_nodes(&self, lot: u64, _cfg: u64) {
        self.events.push("add_network_nodes", vec![("lot", ji(lot as i64))]);
    }
    fn remove_network_connection_nodes(&self, lot: u64) {
        self.events.push("remove_network_nodes", vec![("lot", ji(lot as i64))]);
    }
    fn start_construction_occupant(&self, lot: u64) {
        self.events.push("construction_occupant", vec![("lot", ji(lot as i64))]);
    }

    fn occupant_as_building(&self, occ: u64) -> Option<u64> {
        self.occs.borrow().get(&occ).and_then(|o| o.is_building.then_some(occ))
    }
    fn building_add_ref(&self, _b: u64) {}
    fn building_release(&self, _b: u64) {}
    fn building_set_lit(&self, b: u64, lit: bool) {
        self.events
            .push("set_lit", vec![("building", ji(b as i64)), ("lit", Json::Bool(lit))]);
        self.occ_mut(b, |o| o.lit = lit);
    }
    fn occupant_type(&self, occ: u64) -> u32 {
        self.occs.borrow().get(&occ).map_or(0, World::type_of)
    }
    fn occupant_position(&self, occ: u64) -> Vec3 {
        self.occ(occ, |o| o.position)
    }
    fn occupant_bounding_box(&self, occ: u64) -> (Vec3, Vec3) {
        self.occ(occ, |o| o.bbox)
    }
    fn occupant_city_cells(&self, occ: u64) -> CellRect {
        self.occ(occ, |o| o.cells)
    }
    fn occupant_in_group(&self, occ: u64, group: u32) -> bool {
        self.occ(occ, |o| o.group == group)
    }
    fn occupant_post_message(&self, occ: u64, message: u32) {
        self.events
            .push("occ_post", vec![("occ", ji(occ as i64)), ("message", ji(message))]);
    }
    fn occupant_set_removal_flags(&self, occ: u64, flags: u32) {
        self.events
            .push("removal_flags", vec![("occ", ji(occ as i64)), ("flags", ji(flags))]);
    }
    fn occupant_set_lot_tag(&self, occ: u64, tag: u32) {
        self.events
            .push("set_lot_tag", vec![("occ", ji(occ as i64)), ("tag", ji(tag))]);
    }
    fn occupant_property(&self, occ: u64, id: u32) -> Option<u32> {
        self.occs.borrow().get(&occ).and_then(|o| o.props.get(&id).copied())
    }
    fn occupant_set_property(&self, occ: u64, id: u32, value: u32) {
        self.occ_mut(occ, |o| {
            o.props.insert(id, value);
        });
    }
    fn occupant_init(&self, _occ: u64) {}
    fn occupant_set_flag(&self, _occ: u64, _flag: u32) {}
    fn occupant_is_flag_set(&self, occ: u64, flag: u32) -> bool {
        flag == 1 && self.occs.borrow().get(&occ).map_or(true, |o| o.inserted)
    }
    fn occupant_shutdown(&self, occ: u64) {
        self.events.push("shutdown_occupant", vec![("occ", ji(occ as i64))]);
    }
    fn occupant_set_orientation(&self, occ: u64, orientation: u32) {
        self.occ_mut(occ, |o| o.orientation = orientation);
    }
    fn occupant_set_position(&self, occ: u64, pos: Vec3) {
        self.occ_mut(occ, |o| o.position = pos);
    }
    fn occupant_insert(&self, occ: u64) {
        self.events.push("insert_occupant", vec![("occ", ji(occ as i64))]);
    }
    fn occupant_remove(&self, occ: u64, a: u32, flags: u32) {
        self.events.push(
            "remove_occupant",
            vec![("occ", ji(occ as i64)), ("a", ji(a)), ("flags", ji(flags))],
        );
        self.occs.borrow_mut().remove(&occ);
    }
    fn occupant_changed(&self, occ: u64) {
        self.events.push("occupant_changed", vec![("occ", ji(occ as i64))]);
    }
    fn occupants_in_bbox(&self, _min: Vec3, _max: Vec3) -> Vec<u64> {
        Vec::new()
    }
    fn occupants_in_cells(&self, rect: CellRect, t: u32) -> Vec<u64> {
        if t == OCCUPANT_TYPE_FOUNDATION || t == OCCUPANT_TYPE_BASE_TEXTURE {
            // Slot 41 with a type filter, in list (token) order.
            return self
                .occs
                .borrow()
                .iter()
                .filter(|(_, o)| World::type_of(o) == t && overlaps(o.cells, rect))
                .map(|(&k, _)| k)
                .collect();
        }
        // The night-light props of the lot the building is on.
        self.occs
            .borrow()
            .iter()
            .filter(|(_, o)| o.night_light)
            .map(|(&t, _)| t)
            .collect()
    }
    fn occupants_in_box(&self, min: Vec3, max: Vec3, t: u32) -> Vec<u64> {
        // Slot 37: the occupants of type `t` whose bounding box meets the
        // closed box (§16 test 34 *[inferred]*).
        self.occs
            .borrow()
            .iter()
            .filter(|(_, o)| {
                let (a, b) = o.bbox;
                World::type_of(o) == t && (0..3).all(|i| a[i] <= max[i] && min[i] <= b[i])
            })
            .map(|(&k, _)| k)
            .collect()
    }
    fn building_orientation(&self, b: u64) -> i32 {
        self.occ(b, |o| o.orientation as i32)
    }
    fn foundation_kind(&self, occ: u64) -> Option<i32> {
        self.occs.borrow().get(&occ).and_then(|o| o.foundation_kind)
    }
    fn foundation_type(&self, occ: u64) -> u32 {
        self.occ(occ, |o| o.foundation_type)
    }
    fn add_new_foundation_occupant(
        &self,
        foundation_type: u32,
        kind: i32,
        pos: &mut Vec3,
        dx: f32,
        dz: f32,
        orient: i32,
        bottom: f32,
    ) -> bool {
        // The BDS model of §17.5.3.
        let mut ev = vec![
            ("type", ji(foundation_type)),
            ("kind", ji(kind)),
            ("pos", pos_json(*pos)),
            ("dx", jf(dx)),
            ("dz", jf(dz)),
            ("orient", ji(orient)),
            ("bottom", jf(bottom)),
        ];
        // 2.
        if self.foundation_fails.get() {
            self.events.push("add_foundation", ev);
            return false;
        }
        // 3.
        let hx = f64::from(dx) * 0.5;
        let hz = f64::from(dz) * 0.5;
        let p = |v: f32, h: f64, sign: f64, inset: f64| r24(r24(f64::from(v) + sign * h) + inset);
        let cell = |v: f64| ftol32(v * 0.0625);
        let (cx, cz) = self.city_cells.get();
        let c = CellRect::new(
            cell(p(pos[0], hx, -1.0, 0.5)).max(0),
            cell(p(pos[2], hz, -1.0, 0.5)).max(0),
            cell(p(pos[0], hx, 1.0, -0.5)).min(cx - 1),
            cell(p(pos[2], hz, 1.0, -0.5)).min(cz - 1),
        );
        let view = self.view.borrow().clone().unwrap_or_default();
        // 4.
        if pos[1] < 0.0 {
            pos[1] = view.lowest_or_highest_area_altitude(self, c, false, true);
        }
        // 5.
        let bot = if bottom < 0.0 || bottom.is_nan() {
            view.lowest_or_highest_area_altitude(self, c, true, true)
        } else {
            bottom
        };
        // 6.
        let mut depth = r24(f64::from(pos[1]) - f64::from(bot)) as f32;
        if depth < 0.5 {
            depth = 0.5;
        }
        // 7.
        let t = self.token();
        let o = orient.clamp(0, 3);
        self.occs.borrow_mut().insert(
            t,
            Occ {
                bbox: (
                    [(f64::from(pos[0]) - hx) as f32, bot, (f64::from(pos[2]) - hz) as f32],
                    [(f64::from(pos[0]) + hx) as f32, pos[1], (f64::from(pos[2]) + hz) as f32],
                ),
                cells: c,
                position: *pos,
                orientation: o as u32,
                foundation_kind: Some(kind),
                foundation_type,
                ..Occ::default()
            },
        );
        ev.extend([
            ("occ", ji(t as i64)),
            ("top", jf(pos[1])),
            ("size", pos_json([dx, depth, dz])),
            ("cells", rect_json(c)),
        ]);
        self.events.push("add_foundation", ev);
        true
    }

    fn prop_create(&self, prop_type: u32) -> Option<u64> {
        let t = self.token();
        self.occs.borrow_mut().insert(t, Occ::default());
        self.events
            .push("create_prop", vec![("occ", ji(t as i64)), ("type", ji(prop_type))]);
        Some(t)
    }
    fn lot_props(&self, lot: u64) -> Vec<u64> {
        // `GetLotProps` (prop manager slot 24) appends the lot's props and
        // then erases every element whose position is outside the lot's
        // world rectangle (§15.2). A prop created by `AddLotProp(type, pos,
        // orient)` therefore belongs to the lot its position falls in; the
        // explicit `lot` tag covers props a vector places by hand.
        let (x, z, w, h) = self.lot(lot, |l| (l.x, l.z, l.w, l.h));
        let x0 = f64::from(x) * 16.0;
        let z0 = f64::from(z) * 16.0;
        let x1 = f64::from(x + w as i32) * 16.0;
        let z1 = f64::from(z + h as i32) * 16.0;
        self.occs
            .borrow()
            .iter()
            .filter(|(_, o)| {
                o.lot == Some(lot)
                    || (f64::from(o.position[0]) >= x0
                        && f64::from(o.position[0]) < x1
                        && f64::from(o.position[2]) >= z0
                        && f64::from(o.position[2]) < z1)
            })
            .map(|(&t, _)| t)
            .collect()
    }
    fn prop_add_to_lot(&self, lot: u64, occ: u64) -> bool {
        let (pos, orient) = self.occ(occ, |o| (o.position, o.orientation));
        self.events.push(
            "add_lot_prop",
            vec![
                ("lot", ji(lot as i64)),
                ("occ", ji(occ as i64)),
                ("orientation", ji(orient)),
                ("pos", pos_json(pos)),
            ],
        );
        self.occ_mut(occ, |o| o.lot = Some(lot));
        true
    }
    fn prop_add(&self, prop_type: u32, pos: Vec3, orientation: i32) -> Option<u64> {
        let t = self.token();
        self.occs.borrow_mut().insert(
            t,
            Occ {
                position: pos,
                orientation: orientation as u32,
                ..Occ::default()
            },
        );
        self.events.push(
            "prop_add",
            vec![
                ("occ", ji(t as i64)),
                ("type", ji(prop_type)),
                ("pos", pos_json(pos)),
                ("orientation", ji(orientation)),
            ],
        );
        Some(t)
    }
    fn remove_lot_props(&self, lot: u64) {
        self.events.push("remove_lot_props", vec![("lot", ji(lot as i64))]);
        self.occs.borrow_mut().retain(|_, o| o.lot != Some(lot));
    }
    fn prop_remove(&self, occ: u64) -> bool {
        self.events.push("prop_remove", vec![("occ", ji(occ as i64))]);
        self.occs.borrow_mut().remove(&occ);
        true
    }
    fn remove_building_props(&self, b: u64) {
        self.events
            .push("remove_building_props", vec![("building", ji(b as i64))]);
    }
    fn make_burnt_building(&self, building: u64) -> Option<BurntBuilding> {
        let (min, max) = self.occupant_bounding_box(building);
        Some(BurntBuilding {
            occupant: building,
            min,
            max,
        })
    }
    fn add_rubble_decal(&self, _lot: u64, _b: &BurntBuilding, instance: u32, size: f32) {
        self.events.push(
            "rubble_decal",
            vec![("instance", ji(instance)), ("size", Json::Float(f64::from(size)))],
        );
    }

    fn building_profile(&self, building: u32) -> BuildingProfile {
        self.profiles
            .borrow()
            .get(&building)
            .map_or(BuildingProfile::default(), |p| BuildingProfile {
                purpose: p.purpose,
                wealth: p.wealth,
                capacity: p.capacity,
                ..BuildingProfile::default()
            })
    }
    fn developer_type(&self, purpose: u32, wealth: u32) -> u32 {
        (purpose << 8) | wealth
    }
    fn developer_of_type(&self, dev_type: u32) -> Option<u64> {
        self.devs
            .borrow()
            .iter()
            .find(|(_, d)| d.key == dev_type)
            .map(|(&t, _)| t)
    }
    fn wealth_of_type(&self, dev_type: u32) -> u32 {
        dev_type & 0xFF
    }
    fn developer_for(&self, purpose: u32, wealth: u32) -> Option<u64> {
        let key = (purpose << 8) | wealth;
        self.devs.borrow().iter().find(|(_, d)| d.key == key).map(|(&t, _)| t)
    }
    fn is_building_type(&self, id: u32) -> bool {
        self.valid_building.borrow().contains(&id)
    }
    fn building_types_by_family(&self, family: u32) -> Vec<u32> {
        self.families.borrow().get(&family).cloned().unwrap_or_default()
    }
    fn add_new_building_occupant(&self, building: u32, _pos: Vec3, _o: i32) -> Option<u64> {
        if self.add_building_fails.get() {
            return None;
        }
        let t = self.token();
        self.occs.borrow_mut().insert(
            t,
            Occ {
                is_building: true,
                bbox: ([0.0, 0.0, 0.0], [16.0, 10.0, 16.0]),
                cells: CellRect::new(0, 0, 0, 0),
                ..Occ::default()
            },
        );
        self.events
            .push("add_building", vec![("occ", ji(t as i64)), ("type", ji(building))]);
        Some(t)
    }
    fn claim_construction_resources(&self, amount: u16) -> bool {
        let ok = self.claim_resources.get();
        self.events
            .push("claim", vec![("amount", ji(amount)), ("ok", Json::Bool(ok))]);
        ok
    }
    fn cheat(&self, cheat: Cheat) -> bool {
        let c = self.cheats.get();
        match cheat {
            Cheat::IgnorePlumbing => c[0],
            Cheat::IgnorePower => c[1],
            Cheat::CoffeeBar => c[2],
        }
    }

    fn dev_purpose(&self, dev: u64) -> u32 {
        self.devs.borrow().get(&dev).map_or(0, |d| d.purpose)
    }
    fn dev_desirability(&self, dev: u64, _x: i32, _z: i32) -> i16 {
        self.devs.borrow().get(&dev).map_or(0, |d| d.desirability)
    }
    fn dev_water_supply_cap(&self, dev: u64) -> u8 {
        self.devs.borrow().get(&dev).map_or(0, |d| d.water_cap)
    }

    fn is_powered(&self, x: i32, z: i32) -> bool {
        self.powered.borrow().get(&(x, z)).copied().unwrap_or(false)
    }
    fn is_cell_watered(&self, x: i32, z: i32) -> bool {
        self.watered.borrow().get(&(x, z)).copied().unwrap_or(false)
    }
    fn trip_length(&self, x: i32, z: i32) -> u8 {
        self.trip.borrow().get(&(x, z)).copied().unwrap_or(0)
    }
    fn jobs_demand(&self) -> Option<f32> {
        self.jobs_demand.get()
    }

    fn flora_type_valid(&self, id: u32) -> bool {
        self.valid_flora.borrow().contains(&id)
    }
    fn flora_types_by_family(&self, id: u32) -> Option<Vec<u32>> {
        self.families.borrow().get(&id).cloned()
    }
    fn flora_add(&self, flora_type: u32, _pos: Vec3) -> Option<u64> {
        let t = self.token();
        self.occs.borrow_mut().insert(t, Occ::default());
        self.events
            .push("flora_add", vec![("occ", ji(t as i64)), ("type", ji(flora_type))]);
        Some(t)
    }
    fn flora_demolish_in_cell_area(&self, _rect: CellRect) {
        self.events.push("flora_demolish", vec![]);
    }

    fn demolish_occupant(&self, occ: u64) -> bool {
        self.events.push("demolish_occupant", vec![("occ", ji(occ as i64))]);
        true
    }
    fn demolish_lot(&self, lot: u64) -> bool {
        self.events.push("demolish_lot", vec![("lot", ji(lot as i64))]);
        true
    }
    fn modify_terrain_height(
        &self,
        per_vertex: bool,
        x0: i32,
        z0: i32,
        x1: i32,
        z1: i32,
        h: f32,
        exempt: Option<u64>,
    ) -> i32 {
        // The demolition model of §17.3 (doIt 1, reason 5, cost null).
        self.events.push(
            "modify_terrain",
            vec![
                ("per_vertex", Json::Bool(per_vertex)),
                ("rect", rect_json(CellRect::new(x0, z0, x1, z1))),
                ("h", jf(h)),
                ("exempt", exempt.map_or(Json::Null, |l| ji(l as i64))),
            ],
        );
        let (cx, cz) = self.city_cells.get();
        let q = CellRect::new((x0 - 1).max(0), (z0 - 1).max(0), x1.min(cx - 1), z1.min(cz - 1));
        let immovable = self.immovable_in(q);
        let diff = |x: i32, z: i32| (f64::from(self.vertex_alt(x, z)) - f64::from(h)).abs();
        let mut n = 0;
        if per_vertex {
            for x in x0..=x1 {
                for z in z0..=z1 {
                    if self.water_vertices.borrow().contains(&(x, z)) || diff(x, z) < 0.05 {
                        continue;
                    }
                    if self.locked_lot_in(CellRect::new(x - 1, z - 1, x, z), exempt) {
                        continue;
                    }
                    let (lo_x, hi_x) = (16.0 * x as f32 - 14.5, 16.0 * x as f32 + 14.5);
                    let (lo_z, hi_z) = (16.0 * z as f32 - 14.5, 16.0 * z as f32 + 14.5);
                    if immovable
                        .iter()
                        .any(|(a, b)| a[0] <= hi_x && lo_x <= b[0] && a[2] <= hi_z && lo_z <= b[2])
                    {
                        continue;
                    }
                    n += 1;
                    self.set_vertex_alt(x, z, h);
                    self.events
                        .push("set_altitude", vec![("x", ji(x)), ("z", ji(z)), ("h", jf(h))]);
                }
            }
            if n > 0 {
                self.events.push(
                    "redisplay",
                    vec![
                        ("rect", rect_json(CellRect::new(q.x1, q.z1, q.x2 + 1, q.z2 + 1))),
                        ("reason", ji(5)),
                    ],
                );
            }
            return n;
        }
        if !immovable.is_empty() {
            return 0;
        }
        for x in x0..=x1 {
            for z in z0..=z1 {
                if diff(x, z) > 0.05 {
                    n += 1;
                }
            }
        }
        if n == 0 {
            return 0;
        }
        if !self.locked_lot_in(q, exempt) {
            let r = CellRect::new(x0.max(0), z0.max(0), x1.min(cx), z1.min(cz));
            for x in r.x1..=r.x2 {
                for z in r.z1..=r.z2 {
                    self.set_vertex_alt(x, z, h);
                }
            }
            self.events.push(
                "set_altitude_rect",
                vec![("rect", rect_json(r)), ("h", jf(h)), ("reason", ji(5))],
            );
            self.events
                .push("redisplay", vec![("rect", rect_json(r)), ("reason", ji(5))]);
        }
        n
    }

    fn unit_members(&self, unit: u64) -> Vec<u64> {
        self.units.borrow().get(&unit).cloned().unwrap_or_default()
    }
    fn unit_create(&self) -> u64 {
        let t = self.token();
        self.units.borrow_mut().insert(t, Vec::new());
        self.events.push("unit_create", vec![("unit", ji(t as i64))]);
        t
    }
    fn unit_add(&self, unit: u64, lot: u64) -> bool {
        self.units.borrow_mut().entry(unit).or_default().push(lot);
        self.lot_mut(lot, |l| l.unit = Some(unit));
        self.events
            .push("unit_add", vec![("unit", ji(unit as i64)), ("lot", ji(lot as i64))]);
        true
    }
    fn unit_remove(&self, unit: u64, lot: u64) -> bool {
        let removed = self
            .units
            .borrow_mut()
            .get_mut(&unit)
            .is_some_and(|v| v.contains(&lot).then(|| v.retain(|&l| l != lot)).is_some());
        if removed {
            self.lot_mut(lot, |l| l.unit = None);
        }
        removed
    }

    fn building_ref(&self, building: u64) -> RefValue {
        RefValue::Host {
            token: building,
            class_id: IID_BUILDING_OCCUPANT,
        }
    }
    fn resolve_building(&self, r: &RefValue) -> Option<u64> {
        match r {
            RefValue::Host { token, class_id } if *class_id == IID_BUILDING_OCCUPANT => Some(*token),
            _ => None,
        }
    }
}

// ---- dispatch ----------------------------------------------------------------------------------

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "lot_developer", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("ldev_")?;
    let w = world(host);
    Some(match rest {
        "world" => set_world(&w, op),
        "lot" => set_lot(&w, op),
        "clear_lots" => {
            w.lots.borrow_mut().clear();
            w.order.borrow_mut().clear();
            Ok(())
        }
        "make_lots" => make_lots(&w, op),
        "make_buildings" => make_buildings(&w, op),
        "clear_buildings" => clear_buildings(&w),
        "cfg" => set_cfg(&w, op),
        "occ" => set_occ(&w, op),
        "profile" => set_profile(&w, op),
        "family" => set_family(&w, op),
        "valid" => set_valid(&w, op),
        "dev" => set_dev(&w, op),
        "unit" => set_unit(&w, op),
        "cells" => set_cells(&w, op),
        "terrain" => set_terrain(&w, op),
        "resource" => set_resource(&w, op),
        "surrounding" => set_surrounding(&w, op),
        "tuning" => set_tuning(&w, op),
        "create" => create(&w, op),
        "read_tuning" => w.with_sim(|s| {
            s.read_tuning();
            Ok(())
        }),
        "rng" => rng(&w, op),
        "init" => w.with_sim(|s| check_expect(op, Json::Bool(s.init()))),
        "shutdown" => w.with_sim(|s| check_expect(op, Json::Bool(s.shutdown()))),
        "message" => message(&w, op),
        "set" => set_state(&w, op),
        "daily" => w.with_sim(|s| check_expect(op, Json::Bool(s.do_daily_simulation()))),
        "per_day" => w.with_sim(|s| check_expect(op, ji(s.per_day()))),
        "update_lot" => update_lot(&w, op),
        "test_desirability" => test_desirability(&w, op),
        "test_power" => test_call(&w, op, |s, l, x, z| ji(s.test_power(l, x, z))),
        "test_water" => test_call(&w, op, |s, l, x, z| Json::Bool(s.test_water(l, x, z))),
        "test_travel" => test_travel(&w, op),
        "test_condition" => test_call(&w, op, |s, l, x, z| ji(s.test_condition(l, x, z))),
        "coffee_bar" => call_lot(&w, op, |s, l| {
            s.test_coffee_bar(l);
            Ok(())
        }),
        "add_zot" => add_zot(&w, op),
        "remove_zot" => remove_zot(&w, op),
        "pick_unit" => call_lot(&w, op, |s, l| {
            s.pick_unit_to_join(l);
            Ok(())
        }),
        "needs_anchor" => needs_anchor(&w, op),
        "remove_from_unit" => call_lot_expect(&w, op, |s, l| Json::Bool(s.remove_lot_from_its_business_unit(l))),
        "update_designates" => update_designates(&w, op),
        "update_populations" => update_populations(&w, op),
        "start_construction" => start_construction(&w, op),
        "construct_default" => construct_default(&w, op),
        "abandon" => abandon(&w, op),
        "reoccupy" => reoccupy(&w, op),
        "change_building" => change_building(&w, op),
        "update_lot_elements" => update_lot_elements(&w, op),
        "explode_lot" => explode_lot(&w, op),
        "add_rubble" => add_rubble(&w, op),
        "start_demolish" => call_lot_expect(&w, op, |s, l| Json::Bool(s.start_demolish_lot(l))),
        "end_demolish" => call_lot_expect(&w, op, |s, l| Json::Bool(s.end_demolish_lot(l))),
        "clear_lot" => call_lot(&w, op, |s, l| {
            s.clear_lot(l);
            Ok(())
        }),
        "power_changed" => power_changed(&w, op),
        "set_normal_texture" => set_normal_texture(&w, op),
        "set_force_empty" => set_force_empty(&w, op),
        "anim_tick" => anim_tick(&w, op),
        "light" => light(&w, op),
        "update_all" => update_all(&w, op),
        "area_slope" => area_slope(&w, op),
        "dev_cost" => dev_cost(&w, op),
        "altitude" => altitude(&w, op),
        "area_developed" => area_developed(&w, op),
        "area_altitude" => area_altitude(&w, op),
        "altitude_f" => altitude_f(&w, op),
        "level_internal" => level_internal(&w, op),
        "level_bordering" => level_bordering(&w, op),
        "attempt_levelling" => attempt_levelling(&w, op),
        "object_position" => object_position(&w, op),
        "position_for_building" => position_for_building(&w, op),
        "boundary_texture" => boundary_texture(&w, op),
        "set_empty_texture" => call_lot(&w, op, |s, l| {
            s.set_empty_lot_base_texture(l);
            Ok(())
        }),
        "foundation" => foundation(&w, op),
        "remove_foundation" => remove_foundation(&w, op),
        "add_lot_foundation" => add_lot_foundation(&w, op),
        "update_lot_foundation" => call_lot_expect(&w, op, |s, l| Json::Bool(s.update_lot_foundation(l))),
        "foundation_required" => foundation_required(&w, op),
        "add_building_foundation" => add_building_foundation(&w, op),
        "expect_terrain" => expect_terrain(&w, op),
        "expect_occ" => expect_occ(&w, op),
        "save" => save(&w, op),
        "load" => load(&w, op),
        "expect" => expect(&w, op),
        "expect_lot" => expect_lot(&w, op),
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

// ---- helpers -----------------------------------------------------------------------------------

fn i32_of(j: &Json, what: &str) -> R<i32> {
    Ok(i64_of(j, what)? as i32)
}

fn u32_list(j: &Json, what: &str) -> R<Vec<u32>> {
    arr(j, what)?.iter().map(|v| u32_of(v, what)).collect()
}

fn i32_list(j: &Json, what: &str) -> R<Vec<i32>> {
    arr(j, what)?.iter().map(|v| i32_of(v, what)).collect()
}

fn pair(j: &Json, what: &str) -> R<(i32, i32)> {
    let a = arr(j, what)?;
    if a.len() != 2 {
        return Err(format!("'{what}' must be [x, z]"));
    }
    Ok((i32_of(&a[0], what)?, i32_of(&a[1], what)?))
}

fn rect_of(j: &Json, what: &str) -> R<CellRect> {
    let a = arr(j, what)?;
    if a.len() != 4 {
        return Err(format!("'{what}' must be [x1, z1, x2, z2]"));
    }
    Ok(CellRect::new(
        i32_of(&a[0], what)?,
        i32_of(&a[1], what)?,
        i32_of(&a[2], what)?,
        i32_of(&a[3], what)?,
    ))
}

fn vec3_of(j: &Json, what: &str) -> R<Vec3> {
    let a = arr(j, what)?;
    if a.len() != 3 {
        return Err(format!("'{what}' must be [x, y, z]"));
    }
    Ok([f32_of(&a[0], what)?, f32_of(&a[1], what)?, f32_of(&a[2], what)?])
}

fn token_of(op: &Json, key: &str) -> R<u64> {
    Ok(i64_of(field(op, key)?, key)? as u64)
}

fn opt_token(op: &Json, key: &str) -> R<Option<u64>> {
    match opt(op, key) {
        Some(v) => Ok(Some(i64_of(v, key)? as u64)),
        None => Ok(None),
    }
}

// ---- world ops ---------------------------------------------------------------------------------

/// `ldev_world`: `city [x, z]`, `cell_widths [x, z]`, `terrain_cells [x, z]`,
/// `terrain_width`, `sim_date`, `time_mode`, `view_rotation`, `jobs_demand`
/// (number, "nan" or null), `cheats {ignore_plumbing, ignore_power,
/// coffee_bar}`, `no_services`, `no_simulator`, `no_message_server`,
/// `claim_resources`, `add_building_fails`, `zone_color`, `zone_alpha`,
/// `base_texture_fails`, `foundation_fails`, `trace_terrain`.
fn set_world(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "zone_color") {
        w.zone_color.set(u32_of(v, "zone_color")?);
    }
    if let Some(v) = opt(op, "zone_alpha") {
        w.zone_alpha.set(u32_of(v, "zone_alpha")? as u8);
    }
    if let Some(c) = opt(op, "city") {
        w.city_cells.set(pair(c, "city")?);
    }
    if let Some(c) = opt(op, "cell_widths") {
        let a = arr(c, "cell_widths")?;
        w.cell_widths.set((f32_of(&a[0], "cw")?, f32_of(&a[1], "cw")?));
    }
    if let Some(c) = opt(op, "terrain_cells") {
        w.terrain_cells.set(pair(c, "terrain_cells")?);
    }
    if let Some(v) = opt(op, "terrain_width") {
        w.terrain_width.set(f32_of(v, "terrain_width")?);
    }
    if let Some(v) = opt(op, "sim_date") {
        w.sim_date.set(u32_of(v, "sim_date")?);
    }
    if let Some(v) = opt(op, "time_mode") {
        w.time_mode.set(u32_of(v, "time_mode")?);
    }
    if let Some(v) = opt(op, "view_rotation") {
        w.view_rotation.set(i32_of(v, "view_rotation")?);
    }
    if let Some(v) = opt(op, "jobs_demand") {
        w.jobs_demand.set(match v {
            Json::Null => None,
            Json::Str(s) if s.eq_ignore_ascii_case("nan") => Some(f32::NAN),
            other => Some(f32_of(other, "jobs_demand")?),
        });
    }
    if let Some(c) = opt(op, "cheats") {
        let mut ch = w.cheats.get();
        for (k, v) in c.as_object().ok_or("'cheats' must be an object")? {
            let i = match k.as_str() {
                "ignore_plumbing" => 0,
                "ignore_power" => 1,
                "coffee_bar" => 2,
                other => return Err(format!("unknown cheat '{other}'")),
            };
            ch[i] = bool_of(v, k)?;
        }
        w.cheats.set(ch);
    }
    for (k, cell) in [
        ("no_services", &w.no_services),
        ("no_simulator", &w.no_simulator),
        ("no_message_server", &w.no_message_server),
        ("claim_resources", &w.claim_resources),
        ("add_building_fails", &w.add_building_fails),
        ("base_texture_fails", &w.base_texture_fails),
        ("foundation_fails", &w.foundation_fails),
        ("trace_terrain", &w.trace_terrain),
    ] {
        if let Some(v) = opt(op, k) {
            cell.set(bool_of(v, k)?);
        }
    }
    Ok(())
}

/// `ldev_lot`: `id` (assigned when absent), `x`, `z`, `w`, `h`, `zone`,
/// `state`, `livable`, `condition`, `days`, `building_type`, `building`,
/// `cfg`, `wealth`, `occupancy`, `needs_power`, `has_power`, `road`, `job`,
/// `organic`, `flags`, `historical`, `initialized`, `travel_jobs [w0..w3]`,
/// `designate`, `unit`, `available`, `props {hexId: value}`, `counter
/// {name: value}` (low_desirability, no_power, no_water, travel, zot),
/// `order` (append to the lot-list order; default true for a new lot),
/// `altitude`, `facing`, `locked`.
fn set_lot(w: &World, op: &Json) -> R<()> {
    let id = opt_token(op, "id")?.unwrap_or_else(|| w.token());
    let mut l = w.lots.borrow().get(&id).cloned().unwrap_or_default();
    let new = !w.lots.borrow().contains_key(&id);
    for (k, v) in op.as_object().ok_or("op must be an object")? {
        match k.as_str() {
            "op" | "id" | "description" | "note" | "order" => {}
            "x" => l.x = i32_of(v, k)?,
            "z" => l.z = i32_of(v, k)?,
            "w" => l.w = u32_of(v, k)?,
            "h" => l.h = u32_of(v, k)?,
            "zone" => l.zone = i32_of(v, k)?,
            "state" => l.state = u32_of(v, k)?,
            "livable" => l.livable = bool_of(v, k)?,
            "condition" => l.condition = u32_of(v, k)?,
            "days" => l.days = u32_of(v, k)?,
            "building_type" => l.building_type = u32_of(v, k)?,
            "building" => l.building = Some(i64_of(v, k)? as u64),
            "no_building" => l.building = None,
            "cfg" => l.cfg = Some(i64_of(v, k)? as u64),
            "no_cfg" => l.cfg = None,
            "wealth" => l.wealth = u32_of(v, k)?,
            "occupancy" => l.occupancy = f32_of(v, k)?,
            "needs_power" => l.needs_power = bool_of(v, k)?,
            "has_power" => l.has_power = bool_of(v, k)?,
            "road" => l.road = bool_of(v, k)?,
            "job" => l.job = bool_of(v, k)?,
            "organic" => l.organic = bool_of(v, k)?,
            "flags" => l.flags = u32_of(v, k)?,
            "historical" => l.historical = bool_of(v, k)?,
            "initialized" => l.initialized = bool_of(v, k)?,
            "available" => l.available = u32_of(v, k)? as u16,
            "altitude" => l.altitude = f32_of(v, k)?,
            "facing" => l.facing = i32_of(v, k)?,
            "locked" => l.locked = bool_of(v, k)?,
            "travel_jobs" => {
                let a = arr(v, k)?;
                for (o, x) in l.travel_jobs.iter_mut().zip(a) {
                    *o = f32_of(x, k)?;
                }
            }
            "designate" => l.designate = Some(i64_of(v, k)? as u64),
            "no_designate" => l.designate = None,
            "unit" => l.unit = Some(i64_of(v, k)? as u64),
            "no_unit" => l.unit = None,
            "props" => {
                for (pk, pv) in v.as_object().ok_or("'props' must be an object")? {
                    let id2 =
                        u32::from_str_radix(pk.trim_start_matches("0x"), 16).map_err(|_| format!("bad prop '{pk}'"))?;
                    l.props.insert(id2, u32_of(pv, pk)?);
                }
            }
            "counter" => {
                for (ck, cv) in v.as_object().ok_or("'counter' must be an object")? {
                    let id2 = match ck.as_str() {
                        "low_desirability" => lot_prop::LOW_DESIRABILITY,
                        "no_power" => lot_prop::NO_POWER,
                        "no_water" => lot_prop::NO_WATER,
                        "travel" => lot_prop::TRAVEL,
                        "zot" => lot_prop::ZOT,
                        other => return Err(format!("unknown counter '{other}'")),
                    };
                    l.props.insert(id2, u32_of(cv, ck)?);
                }
            }
            other => return Err(format!("unknown lot field '{other}'")),
        }
    }
    let add_order = opt(op, "order").and_then(boolean).unwrap_or(new);
    w.lots.borrow_mut().insert(id, l);
    if add_order && !w.order.borrow().contains(&id) {
        w.order.borrow_mut().push(id);
    }
    Ok(())
}

fn cfg_object_of(j: &Json) -> R<CfgObject> {
    let mut o = CfgObject::default();
    for (k, v) in j.as_object().ok_or("object must be an object")? {
        match k.as_str() {
            "type" => o.object_type = u32_of(v, k)?,
            "flags" => o.flags = u32_of(v, k)? as u8,
            "ids" => o.ids = u32_list(v, k)?,
            "tag" => o.tag = u32_of(v, k)?,
            "orientation" => o.orientation = u32_of(v, k)? as u8,
            "byte3" => o.byte3 = u32_of(v, k)? as u8,
            "offset" => o.offset = vec3_of(v, k)?,
            "footprint_min" => {
                let a = arr(v, k)?;
                o.footprint_min = [f32_of(&a[0], k)?, f32_of(&a[1], k)?];
            }
            "footprint_max" => {
                let a = arr(v, k)?;
                o.footprint_max = [f32_of(&a[0], k)?, f32_of(&a[1], k)?];
            }
            "foundation" => o.foundation = u32_of(v, k)?,
            other => return Err(format!("unknown object field '{other}'")),
        }
    }
    Ok(o)
}

/// A primary object: `null` (none), an ID list, or an object record.
fn primary_object_of(v: &Json, k: &str) -> R<Option<CfgObject>> {
    Ok(match v {
        Json::Null => None,
        Json::Array(_) => Some(CfgObject {
            ids: u32_list(v, k)?,
            ..CfgObject::default()
        }),
        _ => Some(cfg_object_of(v)?),
    })
}

/// `ldev_make_lots`: `count` empty lots in state 1 (READY) laid out along
/// `(x, 0)`, appended to the lot-list order in creation order. Each is an
/// empty lot with no building, so `UpdateLot` runs only the business-unit
/// step (one `reset_travel_pop` event per visit).
fn make_lots(w: &World, op: &Json) -> R<()> {
    let n = i64_of(field(op, "count")?, "count")?;
    for i in 0..n {
        let l = Lot {
            x: i as i32,
            z: 0,
            state: lot_state::READY,
            ..Lot::default()
        };
        let t = w.token();
        w.lots.borrow_mut().insert(t, l);
        w.order.borrow_mut().push(t);
    }
    Ok(())
}

/// `ldev_make_buildings`: `count` building occupants, tokens 1..count, each
/// placed on its own lot at `(i, 0)` with the building's cells at that lot,
/// and delivered to the lot developer as an `0x99EF1142` occupant-inserted
/// message so it joins the night deque (§8.1). Use `ldev_create` and
/// `ldev_init` first.
fn make_buildings(w: &World, op: &Json) -> R<()> {
    let n = i64_of(field(op, "count")?, "count")?;
    for i in 0..n {
        let id = (i + 1) as u64;
        w.occs.borrow_mut().insert(
            id,
            Occ {
                is_building: true,
                lit: false,
                cells: CellRect::new(i as i32, 0, i as i32, 0),
                ..Occ::default()
            },
        );
        let l = Lot {
            x: i as i32,
            z: 0,
            building: Some(id),
            ..Lot::default()
        };
        w.lots.borrow_mut().insert(id, l);
        w.order.borrow_mut().push(id);
        let m = LotDevMessage {
            msg_type: fc4_core::lot_developer::msg::OCCUPANT_INSERTED,
            data3: fc4_core::lot_developer::OCCUPANT_TYPE_BUILDING,
            void1: Some(id),
            ..LotDevMessage::default()
        };
        w.with_sim(|s| {
            s.do_message(&m);
            Ok(())
        })?;
    }
    Ok(())
}

/// `ldev_clear_buildings`: empties the night deque through `0x99EF1143`
/// occupant-removed messages, clearing the scripted buildings and lots.
fn clear_buildings(w: &World) -> R<()> {
    let ids: Vec<u64> = w
        .occs
        .borrow()
        .iter()
        .filter(|(_, o)| o.is_building)
        .map(|(&t, _)| t)
        .collect();
    for id in ids {
        let m = LotDevMessage {
            msg_type: fc4_core::lot_developer::msg::OCCUPANT_REMOVED,
            data3: fc4_core::lot_developer::OCCUPANT_TYPE_BUILDING,
            void1: Some(id),
            ..LotDevMessage::default()
        };
        w.with_sim(|s| {
            s.do_message(&m);
            Ok(())
        })?;
    }
    w.occs.borrow_mut().clear();
    w.lots.borrow_mut().clear();
    w.order.borrow_mut().clear();
    Ok(())
}

/// `ldev_cfg`: `id` (token), `cfg_id` (the configuration's own ID), `w`, `h`,
/// `stage`, `water`, `max_slope`, `objects [{type, flags, ids, tag,
/// orientation, byte3, offset, footprint_min, footprint_max, foundation}]`,
/// `primary_building` / `primary_foundation` (an object, or its ID list;
/// null removes it), `water_objects`.
fn set_cfg(w: &World, op: &Json) -> R<()> {
    let id = token_of(op, "id")?;
    let mut c = w.cfgs.borrow().get(&id).cloned().unwrap_or(Cfg {
        id: id as u32,
        w: 1,
        h: 1,
        ..Cfg::default()
    });
    for (k, v) in op.as_object().ok_or("op must be an object")? {
        match k.as_str() {
            "op" | "id" | "description" | "note" => {}
            "cfg_id" => c.id = u32_of(v, k)?,
            "w" => c.w = u32_of(v, k)?,
            "h" => c.h = u32_of(v, k)?,
            "stage" => c.stage = u32_of(v, k)? as u8,
            "water" => c.water = bool_of(v, k)?,
            "max_slope" => c.max_slope = f32_of(v, k)?,
            "objects" => c.objects = arr(v, k)?.iter().map(cfg_object_of).collect::<R<_>>()?,
            "primary_building" => c.primary_building = primary_object_of(v, k)?,
            "primary_foundation" => c.primary_foundation = primary_object_of(v, k)?,
            "water_objects" => c.water_objects = bool_of(v, k)?,
            other => return Err(format!("unknown cfg field '{other}'")),
        }
    }
    w.cfgs.borrow_mut().insert(id, c);
    Ok(())
}

/// `ldev_occ`: `id` (token), `is_building`, `bbox [[minx,miny,minz],
/// [maxx,maxy,maxz]]`, `cells [x1,z1,x2,z2]`, `group`, `lit`, `position`,
/// `orientation`, `night_light`, `lot`, `props {hexId: value}`,
/// `foundation_kind`, `foundation_type`, `type` (the occupant type),
/// `inserted`, `immovable`.
fn set_occ(w: &World, op: &Json) -> R<()> {
    let id = token_of(op, "id")?;
    let mut o = w.occs.borrow().get(&id).cloned().unwrap_or_default();
    for (k, v) in op.as_object().ok_or("op must be an object")? {
        match k.as_str() {
            "op" | "id" | "description" | "note" => {}
            "is_building" => o.is_building = bool_of(v, k)?,
            "bbox" => {
                let a = arr(v, k)?;
                o.bbox = (vec3_of(&a[0], "bbox")?, vec3_of(&a[1], "bbox")?);
            }
            "cells" => o.cells = rect_of(v, k)?,
            "group" => o.group = u32_of(v, k)?,
            "lit" => o.lit = bool_of(v, k)?,
            "position" => o.position = vec3_of(v, k)?,
            "orientation" => o.orientation = u32_of(v, k)?,
            "night_light" => o.night_light = bool_of(v, k)?,
            "lot" => o.lot = Some(i64_of(v, k)? as u64),
            "props" => {
                for (pk, pv) in v.as_object().ok_or("'props' must be an object")? {
                    let id2 =
                        u32::from_str_radix(pk.trim_start_matches("0x"), 16).map_err(|_| format!("bad prop '{pk}'"))?;
                    o.props.insert(id2, u32_of(pv, pk)?);
                }
            }
            "foundation_kind" => o.foundation_kind = Some(i32_of(v, k)?),
            "foundation_type" => o.foundation_type = u32_of(v, k)?,
            "type" => o.occ_type = Some(u32_of(v, k)?),
            "inserted" => o.inserted = bool_of(v, k)?,
            "immovable" => o.immovable = bool_of(v, k)?,
            other => return Err(format!("unknown occ field '{other}'")),
        }
    }
    w.occs.borrow_mut().insert(id, o);
    Ok(())
}

/// `ldev_profile`: `building` (the building type), `purpose`, `wealth`,
/// `capacity`.
fn set_profile(w: &World, op: &Json) -> R<()> {
    let b = u32_of(field(op, "building")?, "building")?;
    let mut p = w.profiles.borrow().get(&b).cloned().unwrap_or_default();
    if let Some(v) = opt(op, "purpose") {
        p.purpose = u32_of(v, "purpose")? as u8;
    }
    if let Some(v) = opt(op, "wealth") {
        p.wealth = u32_of(v, "wealth")? as u8;
    }
    if let Some(v) = opt(op, "capacity") {
        p.capacity = u32_of(v, "capacity")? as u16;
    }
    w.profiles.borrow_mut().insert(b, p);
    Ok(())
}

/// `ldev_family`: `family`, `members [ids]`.
fn set_family(w: &World, op: &Json) -> R<()> {
    let f = u32_of(field(op, "family")?, "family")?;
    let m = u32_list(field(op, "members")?, "members")?;
    w.families.borrow_mut().insert(f, m);
    Ok(())
}

/// `ldev_valid`: `building [ids]` and/or `flora [ids]` marked valid.
fn set_valid(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "building") {
        for id in u32_list(v, "building")? {
            w.valid_building.borrow_mut().insert(id);
        }
    }
    if let Some(v) = opt(op, "flora") {
        for id in u32_list(v, "flora")? {
            w.valid_flora.borrow_mut().insert(id);
        }
    }
    Ok(())
}

/// `ldev_dev`: `id` (token), `key` (`(purpose << 8) | wealth`; computed from
/// `purpose`/`wealth` when absent), `purpose`, `desirability`, `water_cap`.
fn set_dev(w: &World, op: &Json) -> R<()> {
    let id = token_of(op, "id")?;
    let mut d = w.devs.borrow().get(&id).cloned().unwrap_or_default();
    if let Some(v) = opt(op, "purpose") {
        d.purpose = u32_of(v, "purpose")?;
    }
    if let Some(v) = opt(op, "desirability") {
        d.desirability = i32_of(v, "desirability")? as i16;
    }
    if let Some(v) = opt(op, "water_cap") {
        d.water_cap = u32_of(v, "water_cap")? as u8;
    }
    // The key defaults to the developer type `(purpose << 8) | wealth`; when
    // an existing developer is updated, an omitted purpose or wealth keeps
    // the value already in its key.
    let cur_purpose = d.key >> 8;
    let cur_wealth = d.key & 0xFF;
    d.key = match opt(op, "key") {
        Some(v) => u32_of(v, "key")?,
        None => {
            let purpose = opt(op, "purpose").map_or(Ok(cur_purpose), |v| u32_of(v, "purpose"))?;
            let wealth = opt(op, "wealth").map_or(Ok(cur_wealth), |v| u32_of(v, "wealth"))?;
            (purpose << 8) | wealth
        }
    };
    w.devs.borrow_mut().insert(id, d);
    Ok(())
}

/// `ldev_unit`: `id` (token), `members [lots]` (also sets each lot's unit).
fn set_unit(w: &World, op: &Json) -> R<()> {
    let id = token_of(op, "id")?;
    let members = opt(op, "members").map_or(Ok(Vec::new()), |v| {
        arr(v, "members")?
            .iter()
            .map(|x| Ok(i64_of(x, "member")? as u64))
            .collect::<R<Vec<u64>>>()
    })?;
    for &m in &members {
        w.lot_mut(m, |l| l.unit = Some(id));
    }
    w.units.borrow_mut().insert(id, members);
    Ok(())
}

/// `ldev_cells`: `powered [[x, z, v]]`, `watered [[x, z, v]]`,
/// `trip [[x, z, v]]`. A value of 1 (or any nonzero for powered / watered)
/// sets the cell.
fn set_cells(w: &World, op: &Json) -> R<()> {
    for (k, map) in [("powered", &w.powered), ("watered", &w.watered)] {
        if let Some(v) = opt(op, k) {
            for e in arr(v, k)? {
                let t = i32_list(e, k)?;
                map.borrow_mut()
                    .insert((t[0], t[1]), t.get(2).copied().unwrap_or(1) != 0);
            }
        }
    }
    if let Some(v) = opt(op, "trip") {
        for e in arr(v, "trip")? {
            let t = i32_list(e, "trip")?;
            w.trip
                .borrow_mut()
                .insert((t[0], t[1]), t.get(2).copied().unwrap_or(0) as u8);
        }
    }
    Ok(())
}

/// `ldev_terrain`: `alt [[index, value]]` (vertex index → altitude),
/// `water [[index, value]]`, `flat` (every other vertex; default 100),
/// `vertex [[x, z, value]]` (by vertex, index `x + z·(terrain cells x + 1)`),
/// `pos [[x, z, value]]` (`GetAltitude(x, z)`; other positions interpolate
/// the vertices), `water_pos [[x, z, value]]` (`GetWaterLevel(x, z)`, else
/// 0), `water_vertices [[x, z]]`, `water_cells [[x, z]]`; `reset: true`
/// first clears everything scripted (not `flat`).
fn set_terrain(w: &World, op: &Json) -> R<()> {
    if opt(op, "reset").and_then(boolean).unwrap_or(false) {
        w.terrain_alt.borrow_mut().clear();
        w.terrain_water.borrow_mut().clear();
        w.pos_alt.borrow_mut().clear();
        w.pos_water.borrow_mut().clear();
        w.water_vertices.borrow_mut().clear();
        w.water_cells.borrow_mut().clear();
    }
    if let Some(v) = opt(op, "flat") {
        w.flat_alt.set(f32_of(v, "flat")?);
    }
    if let Some(v) = opt(op, "vertex") {
        for e in arr(v, "vertex")? {
            let a = arr(e, "vertex")?;
            w.set_vertex_alt(i32_of(&a[0], "x")?, i32_of(&a[1], "z")?, f32_of(&a[2], "value")?);
        }
    }
    for (k, map) in [("pos", &w.pos_alt), ("water_pos", &w.pos_water)] {
        if let Some(v) = opt(op, k) {
            for e in arr(v, k)? {
                let a = arr(e, k)?;
                map.borrow_mut()
                    .insert(pos_key(f32_of(&a[0], "x")?, f32_of(&a[1], "z")?), f32_of(&a[2], "value")?);
            }
        }
    }
    for (k, set) in [("water_vertices", &w.water_vertices), ("water_cells", &w.water_cells)] {
        if let Some(v) = opt(op, k) {
            for e in arr(v, k)? {
                let t = pair(e, k)?;
                set.borrow_mut().insert(t);
            }
        }
    }
    if let Some(v) = opt(op, "alt") {
        for e in arr(v, "alt")? {
            let a = e.as_array().ok_or("'alt' entries are [index, value]")?;
            let idx = i32_of(&a[0], "index")?;
            let val = f32_of(&a[1], "value")?;
            w.terrain_alt.borrow_mut().insert(idx, val);
        }
    }
    if let Some(v) = opt(op, "water") {
        for e in arr(v, "water")? {
            let a = e.as_array().ok_or("'water' entries are [index, value]")?;
            let idx = i32_of(&a[0], "index")?;
            let val = f32_of(&a[1], "value")?;
            w.terrain_water.borrow_mut().insert(idx, val);
        }
    }
    Ok(())
}

/// `ldev_resource`: `instances [ids]` that exist as base-texture resources.
/// `replace: true` clears the set first (the default adds to it).
fn set_resource(w: &World, op: &Json) -> R<()> {
    if opt(op, "replace").and_then(boolean).unwrap_or(false) {
        w.resources.borrow_mut().clear();
    }
    for id in u32_list(field(op, "instances")?, "instances")? {
        w.resources.borrow_mut().insert(id);
    }
    Ok(())
}

/// `ldev_surrounding`: `lot {token: [neighbours]}`, `rect [lots]`,
/// `in_rect [lots]` (what `GetLotsInCellRect` returns).
fn set_surrounding(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "lot") {
        for (k, mv) in v.as_object().ok_or("'lot' must be an object")? {
            let lot = k.parse::<u64>().map_err(|_| format!("bad lot token '{k}'"))?;
            let members = arr(mv, k)?
                .iter()
                .map(|x| Ok(i64_of(x, "n")? as u64))
                .collect::<R<Vec<u64>>>()?;
            w.surrounding.borrow_mut().insert(lot, members);
        }
    }
    if let Some(v) = opt(op, "rect") {
        *w.surrounding_rect.borrow_mut() = arr(v, "rect")?
            .iter()
            .map(|x| Ok(i64_of(x, "n")? as u64))
            .collect::<R<_>>()?;
    }
    if let Some(v) = opt(op, "in_rect") {
        *w.lots_in_rect.borrow_mut() = arr(v, "in_rect")?
            .iter()
            .map(|x| Ok(i64_of(x, "n")? as u64))
            .collect::<R<_>>()?;
    }
    Ok(())
}

/// `ldev_tuning`: `none` (no exemplar), `shipped` (the shipped exemplar),
/// or explicit properties (`stay_new`, `thresholds [a, b, c]`, `night_acc`,
/// `night_batch`, `max_slope_flatness`, `auto_level_delta`,
/// `max_area_slope`, `default_lot_foundation`, `cost_curve [[x, y], …]`)
/// on an empty exemplar, or on the shipped one with `from_shipped: true`.
fn set_tuning(w: &World, op: &Json) -> R<()> {
    if opt(op, "none").and_then(boolean).unwrap_or(false) {
        *w.tuning.borrow_mut() = None;
        return Ok(());
    }
    if opt(op, "shipped").and_then(boolean).unwrap_or(false) {
        *w.tuning.borrow_mut() = Some(Rc::new(Tuning::shipped_exemplar()));
        return Ok(());
    }
    let mut h = if opt(op, "from_shipped").and_then(boolean).unwrap_or(false) {
        Tuning::shipped_exemplar()
    } else {
        PropertyHolder::new()
    };
    for (k, id) in [
        ("auto_level_delta", fc4_core::lot_developer::prop::AUTO_LEVEL_INTERNAL_ALTITUDE_DELTA),
        ("max_area_slope", fc4_core::lot_developer::prop::MAX_AREA_SLOPE_FOR_LEVELLING),
    ] {
        if let Some(v) = opt(op, k) {
            h.add(id, Variant::Scalar(Elem::F32(f32_of(v, k)?)));
        }
    }
    if let Some(v) = opt(op, "default_lot_foundation") {
        h.add(
            fc4_core::lot_developer::prop::DEFAULT_LOT_FOUNDATION_TYPE,
            Variant::Scalar(Elem::U32(u32_of(v, "default_lot_foundation")?)),
        );
    }
    if let Some(v) = opt(op, "stay_new") {
        h.add(
            fc4_core::lot_developer::prop::STAY_NEW_TIME,
            Variant::Scalar(Elem::U16(u32_of(v, "stay_new")? as u16)),
        );
    }
    if let Some(v) = opt(op, "thresholds") {
        let a = arr(v, "thresholds")?;
        let elems: Vec<Elem> = a.iter().map(|x| Ok(Elem::F32(f32_of(x, "t")?))).collect::<R<_>>()?;
        h.add(
            fc4_core::lot_developer::prop::OCCUPANCY_THRESHOLDS,
            Variant::array(mem::FLOAT32, elems),
        );
    }
    if let Some(v) = opt(op, "night_acc") {
        h.add(
            fc4_core::lot_developer::prop::NIGHT_UPDATE_INTERVAL,
            Variant::Scalar(Elem::U32(u32_of(v, "night_acc")?)),
        );
    }
    if let Some(v) = opt(op, "night_batch") {
        h.add(
            fc4_core::lot_developer::prop::NIGHT_UPDATE_COUNT,
            Variant::Scalar(Elem::U32(u32_of(v, "night_batch")?)),
        );
    }
    if let Some(v) = opt(op, "max_slope_flatness") {
        h.add(
            fc4_core::lot_developer::prop::MAX_SLOPE_FOR_FLATNESS,
            Variant::Scalar(Elem::F32(f32_of(v, "max_slope_flatness")?)),
        );
    }
    if let Some(v) = opt(op, "cost_curve") {
        let flat: Vec<Elem> = arr(v, "cost_curve")?
            .iter()
            .flat_map(|pt| {
                let a = pt.as_array().unwrap();
                vec![
                    Elem::F32(a[0].as_f64().unwrap() as f32),
                    Elem::F32(a[1].as_f64().unwrap() as f32),
                ]
            })
            .collect();
        h.add(
            fc4_core::lot_developer::prop::COST_VS_SLOPE,
            Variant::array(mem::FLOAT32, flat),
        );
    }
    *w.tuning.borrow_mut() = Some(Rc::new(h));
    Ok(())
}

// ---- lifecycle / rng ---------------------------------------------------------------------------

fn create(w: &World, op: &Json) -> R<()> {
    let seed = opt(op, "seed").map_or(Ok(DEFAULT_SEED), |v| u32_of(v, "seed"))?;
    let statics = match opt(op, "session") {
        Some(v) => LotDeveloperStatics::from_session(u32_of(v, "session")?, SEEDS),
        None => LotDeveloperStatics::new(seed, SEEDS),
    };
    if let Some(c) = opt(op, "crt_seed") {
        statics.borrow_mut().crt = CrtRand::new(u32_of(c, "crt_seed")?);
    }
    if opt(op, "trace").and_then(boolean).unwrap_or(true) {
        statics.borrow_mut().trace = Some(Vec::new());
    }
    let s = LotDeveloper::new(w.rc()? as Rc<dyn LotDeveloperHost>, statics.clone());
    *w.statics.borrow_mut() = Some(statics);
    *w.sim.borrow_mut() = Some(s);
    Ok(())
}

/// `ldev_rng`: `seed` (`Seed(s)` of the file-local generator), `lcg` / `mul`
/// (raw states), `crt` (the `rand()` state).
fn rng(w: &World, op: &Json) -> R<()> {
    let st = w.statics.borrow().clone().ok_or("no statics (use ldev_create)")?;
    let mut s = st.borrow_mut();
    if let Some(v) = opt(op, "seed") {
        s.rng = RzRandom::new(u32_of(v, "seed")?, &SEEDS);
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

// ---- calls -------------------------------------------------------------------------------------

fn message_of(op: &Json) -> R<LotDevMessage> {
    let mut m = LotDevMessage::of_type(u32_of(field(op, "msg")?, "msg")?);
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
    if let Some(v) = opt(op, "void1") {
        m.void1 = Some(i64_of(v, "void1")? as u64);
    }
    if let Some(v) = opt(op, "lot") {
        m.lot = Some(i64_of(v, "lot")? as u64);
    }
    Ok(m)
}

/// `ldev_message`: a `LotDevMessage`; `expect` (the `DoMessage` result).
fn message(w: &World, op: &Json) -> R<()> {
    let m = message_of(op)?;
    w.with_sim(|s| check_expect(op, Json::Bool(s.do_message(&m))))
}

/// `ldev_set`: `daily_state`, `visited`, `net_changed`, `unit_changed`,
/// `no_zot_until_day`, `can_check_commute`, `night_mode`, `night_acc`,
/// `night_batch`, `night_cursor`, `lit_flag`, `force_empty`,
/// `daily_work_budget`.
fn set_state(w: &World, op: &Json) -> R<()> {
    w.with_sim(|s| {
        if let Some(v) = opt(op, "daily_state") {
            s.set_daily_state(u32_of(v, "daily_state")?);
        }
        if let Some(v) = opt(op, "visited") {
            s.set_visited(i32_of(v, "visited")?);
        }
        if let Some(v) = opt(op, "net_changed") {
            s.set_net_changed(bool_of(v, "net_changed")?);
        }
        if let Some(v) = opt(op, "unit_changed") {
            s.set_unit_changed(bool_of(v, "unit_changed")?);
        }
        if let Some(v) = opt(op, "no_zot_until_day") {
            s.set_no_zot_until_day(u32_of(v, "no_zot_until_day")?);
        }
        if let Some(v) = opt(op, "can_check_commute") {
            s.set_can_check_commute(bool_of(v, "can_check_commute")?);
        }
        if let Some(v) = opt(op, "night_mode") {
            s.set_night_mode(u32_of(v, "night_mode")?);
        }
        if let Some(v) = opt(op, "night_acc") {
            s.set_night_acc(u32_of(v, "night_acc")?);
        }
        if let Some(v) = opt(op, "night_batch") {
            s.set_night_batch(u32_of(v, "night_batch")?);
        }
        if let Some(v) = opt(op, "night_cursor") {
            s.set_night_cursor(i64_of(v, "night_cursor")? as usize);
        }
        if let Some(v) = opt(op, "lit_flag") {
            s.set_lit_flag(bool_of(v, "lit_flag")?);
        }
        if let Some(v) = opt(op, "daily_work_budget") {
            s.set_daily_work_budget(u32_of(v, "daily_work_budget")?);
        }
        if let Some(v) = opt(op, "force_empty") {
            // Set the flag without the walk (the walk is `ldev_set_force_empty`).
            s.set_force_empty_direct(bool_of(v, "force_empty")?);
        }
        Ok(())
    })
}

fn update_lot(w: &World, op: &Json) -> R<()> {
    let lot = token_of(op, "lot")?;
    w.with_sim(|s| check_expect(op, Json::Bool(s.update_lot(lot))))
}

fn test_desirability(w: &World, op: &Json) -> R<()> {
    let lot = token_of(op, "lot")?;
    let (x, z) = pair(field(op, "at")?, "at")?;
    w.with_sim(|s| check_expect(op, Json::Bool(s.test_desirability(lot, x, z))))
}

fn test_call(w: &World, op: &Json, f: impl FnOnce(&mut LotDeveloper, u64, i32, i32) -> Json) -> R<()> {
    let lot = token_of(op, "lot")?;
    let (x, z) = pair(field(op, "at")?, "at")?;
    w.with_sim(|s| check_expect(op, f(s, lot, x, z)))
}

fn test_travel(w: &World, op: &Json) -> R<()> {
    let lot = token_of(op, "lot")?;
    let (x, z) = pair(field(op, "at")?, "at")?;
    let purpose = opt(op, "purpose").map_or(Ok(0u32), |v| u32_of(v, "purpose"))? as u8;
    let wealth = opt(op, "wealth").map_or(Ok(0u32), |v| u32_of(v, "wealth"))? as u8;
    w.with_sim(|s| {
        let p = BuildingProfile::basic(purpose, wealth, 0);
        check_expect(op, Json::Bool(s.test_travel(lot, &p, x, z)))
    })
}

fn call_lot(w: &World, op: &Json, f: impl FnOnce(&mut LotDeveloper, u64) -> R<()>) -> R<()> {
    let lot = token_of(op, "lot")?;
    w.with_sim(|s| f(s, lot))
}

fn call_lot_expect(w: &World, op: &Json, f: impl FnOnce(&mut LotDeveloper, u64) -> Json) -> R<()> {
    let lot = token_of(op, "lot")?;
    w.with_sim(|s| check_expect(op, f(s, lot)))
}

fn add_zot(w: &World, op: &Json) -> R<()> {
    let lot = token_of(op, "lot")?;
    let prop_type = u32_of(field(op, "prop")?, "prop")?;
    let which = u32_of(field(op, "zot")?, "zot")?;
    w.with_sim(|s| {
        s.add_building_zot_prop(lot, prop_type, which);
        Ok(())
    })
}

fn remove_zot(w: &World, op: &Json) -> R<()> {
    let lot = token_of(op, "lot")?;
    let which = opt(op, "zot").map_or(Ok(0u32), |v| u32_of(v, "zot"))?;
    w.with_sim(|s| {
        if which == 0 && opt(op, "zot").is_none() {
            // slot 11: remove whatever zot is shown.
            let r = s.remove_zot(lot);
            return check_expect(op, Json::Bool(r));
        }
        s.remove_building_zot_prop(lot, which);
        check_expect(op, Json::Bool(true))
    })
}

fn needs_anchor(w: &World, op: &Json) -> R<()> {
    let rect = rect_of(field(op, "rect")?, "rect")?;
    let purpose = i32_of(field(op, "purpose")?, "purpose")?;
    w.with_sim(|s| check_expect(op, Json::Bool(s.needs_anchor_building(rect, purpose))))
}

/// `ldev_update_designates`: slot 34 `UpdateTravelDesignates(u&)` (§5.7).
/// `unit` is the unit reference; `expect` checks the resulting reference
/// (null when the dissolution left none).
fn update_designates(w: &World, op: &Json) -> R<()> {
    let mut unit = opt_token(op, "unit")?;
    w.with_sim(|s| {
        s.update_travel_designates(&mut unit);
        Ok(())
    })?;
    if op.get("expect").is_some() {
        let got = unit.map_or(Json::Null, |u| ji(u as i64));
        return check_expect(op, got);
    }
    Ok(())
}

/// `ldev_update_populations`: slot 35 `UpdateTravelPopulations(u)` (§5.7).
fn update_populations(w: &World, op: &Json) -> R<()> {
    let unit = token_of(op, "unit")?;
    w.with_sim(|s| {
        s.update_travel_populations(unit);
        Ok(())
    })
}

fn start_construction(w: &World, op: &Json) -> R<()> {
    let lot = token_of(op, "lot")?;
    let cfg = opt_token(op, "cfg")?;
    let btype = u32_of(field(op, "type")?, "type")?;
    let animate = opt(op, "animate").and_then(boolean).unwrap_or(false);
    w.with_sim(|s| check_expect(op, Json::Bool(s.start_construction(lot, cfg, btype, animate))))
}

fn construct_default(w: &World, op: &Json) -> R<()> {
    let lot = token_of(op, "lot")?;
    let cfg = opt_token(op, "cfg")?;
    let btype = opt(op, "type").map_or(Ok(0u32), |v| u32_of(v, "type"))?;
    let animate = opt(op, "animate").and_then(boolean).unwrap_or(false);
    w.with_sim(|s| check_expect(op, Json::Bool(s.construct_default_lot(lot, cfg, btype, animate))))
}

fn abandon(w: &World, op: &Json) -> R<()> {
    let lot = token_of(op, "lot")?;
    let why = u32_of(field(op, "reason")?, "reason")?;
    w.with_sim(|s| check_expect(op, Json::Bool(s.abandon_building(lot, why))))
}

fn reoccupy(w: &World, op: &Json) -> R<()> {
    let lot = token_of(op, "lot")?;
    let dev = u32_of(field(op, "dev")?, "dev")?;
    w.with_sim(|s| check_expect(op, Json::Bool(s.reoccupy_building(lot, dev))))
}

fn change_building(w: &World, op: &Json) -> R<()> {
    let lot = token_of(op, "lot")?;
    let btype = u32_of(field(op, "type")?, "type")?;
    w.with_sim(|s| check_expect(op, Json::Bool(s.change_building(lot, btype))))
}

/// `ldev_update_lot_elements`: `lot`, `kind` (the whole 32-bit terrain-change
/// kind; §6.4, §16 test 27); `expect` (always true).
fn update_lot_elements(w: &World, op: &Json) -> R<()> {
    let lot = token_of(op, "lot")?;
    let kind = u32_of(field(op, "kind")?, "kind")?;
    w.with_sim(|s| check_expect(op, Json::Bool(s.update_lot_elements(lot, kind))))
}

fn explode_lot(w: &World, op: &Json) -> R<()> {
    let lot = token_of(op, "lot")?;
    let burnt = opt(op, "burnt").and_then(boolean).unwrap_or(false);
    w.with_sim(|s| check_expect(op, Json::Bool(s.explode_lot(lot, burnt))))
}

fn add_rubble(w: &World, op: &Json) -> R<()> {
    let lot = token_of(op, "lot")?;
    let min = vec3_of(field(op, "min")?, "min")?;
    let max = vec3_of(field(op, "max")?, "max")?;
    let burnt = opt(op, "burnt").and_then(boolean).unwrap_or(false);
    w.with_sim(|s| {
        s.add_lot_rubble_props(lot, min, max, burnt);
        Ok(())
    })
}

fn power_changed(w: &World, op: &Json) -> R<()> {
    let rect = rect_of(field(op, "rect")?, "rect")?;
    w.with_sim(|s| {
        s.do_power_changed(rect);
        Ok(())
    })
}

fn set_normal_texture(w: &World, op: &Json) -> R<()> {
    let lot = token_of(op, "lot")?;
    let cfg = opt_token(op, "cfg")?;
    let flag = opt(op, "flag").and_then(boolean).unwrap_or(true);
    w.with_sim(|s| {
        s.set_normal_lot_base_texture(lot, cfg, flag);
        Ok(())
    })
}

fn set_force_empty(w: &World, op: &Json) -> R<()> {
    let f = bool_of(field(op, "value")?, "value")?;
    w.with_sim(|s| {
        s.set_force_empty_lot_base_texture(f);
        Ok(())
    })
}

fn anim_tick(w: &World, op: &Json) -> R<()> {
    let ms = u32_of(field(op, "ms")?, "ms")?;
    w.with_sim(|s| {
        s.on_animation_tick(ms);
        Ok(())
    })
}

fn light(w: &World, op: &Json) -> R<()> {
    let b = token_of(op, "building")?;
    let is_night = opt(op, "night").and_then(boolean).unwrap_or(true);
    w.with_sim(|s| {
        s.light(b, is_night);
        Ok(())
    })
}

fn update_all(w: &World, op: &Json) -> R<()> {
    let mode = u32_of(field(op, "mode")?, "mode")?;
    w.with_sim(|s| {
        s.update_all(mode);
        Ok(())
    })
}

fn area_slope(w: &World, op: &Json) -> R<()> {
    let rect = rect_of(field(op, "rect")?, "rect")?;
    let water = opt(op, "water").and_then(boolean).unwrap_or(true);
    w.with_sim(|s| check_expect(op, Json::Float(f64::from(s.get_area_slope(rect, water)))))
}

fn dev_cost(w: &World, op: &Json) -> R<()> {
    let rect = rect_of(field(op, "rect")?, "rect")?;
    w.with_sim(|s| check_expect(op, ji(s.get_lot_development_cost(rect))))
}

fn altitude(w: &World, op: &Json) -> R<()> {
    let (x, z) = pair(field(op, "at")?, "at")?;
    let water = opt(op, "water").and_then(boolean).unwrap_or(false);
    w.with_sim(|s| {
        check_expect(
            op,
            Json::Float(f64::from(s.get_terrain_altitude_by_vertex(x, z, water))),
        )
    })
}

/// `ldev_area_developed`: slots 23 / 24, both always false (§11.1). `rect`
/// for slot 23, `lot` for slot 24.
fn area_developed(w: &World, op: &Json) -> R<()> {
    w.with_sim(|s| {
        let got = match opt(op, "rect") {
            Some(r) => s.is_area_developed(rect_of(r, "rect")?),
            None => s.is_lot_developed(token_of(op, "lot")?),
        };
        check_expect(op, Json::Bool(got))
    })
}

// ---- geometry (§17) ----------------------------------------------------------------------------

/// A float argument: a number, `"flt_max"`, `"-flt_max"` or `"nan"`.
fn float_arg(j: &Json, what: &str) -> R<f32> {
    match j {
        Json::Str(s) if s.eq_ignore_ascii_case("flt_max") => Ok(FLT_MAX),
        Json::Str(s) if s.eq_ignore_ascii_case("-flt_max") => Ok(-FLT_MAX),
        Json::Str(s) if s.eq_ignore_ascii_case("nan") => Ok(f32::NAN),
        other => f32_of(other, what),
    }
}

/// A float result, `"flt_max"` / `"-flt_max"` / `"nan"` for those values.
fn float_json(v: f32) -> Json {
    if v.is_nan() {
        Json::Str("nan".into())
    } else if v == FLT_MAX {
        Json::Str("flt_max".into())
    } else if v == -FLT_MAX {
        Json::Str("-flt_max".into())
    } else {
        jf(v)
    }
}

/// `expect_bits`: the float32 result's exact bit pattern.
fn check_bits(op: &Json, v: f32) -> R<()> {
    if let Some(b) = opt(op, "expect_bits") {
        let want = u32_of(b, "expect_bits")?;
        if v.to_bits() != want {
            return Err(format!("returned {v} ({:#010X}), expected bits {want:#010X}", v.to_bits()));
        }
    }
    Ok(())
}

/// An optional token that may be `null`.
fn opt_token_null(op: &Json, key: &str) -> R<Option<u64>> {
    match opt(op, key) {
        None | Some(Json::Null) => Ok(None),
        Some(v) => Ok(Some(i64_of(v, key)? as u64)),
    }
}

fn flag(op: &Json, key: &str, default: bool) -> bool {
    opt(op, key).and_then(boolean).unwrap_or(default)
}

/// `ldev_area_altitude`: `rect` (cells), `water` (default true); slot 17
/// with `lowest` (default false), or slot 18 with `average: true`;
/// `expect` (a number, `"flt_max"`, `"-flt_max"` or `"nan"`), `expect_bits`.
fn area_altitude(w: &World, op: &Json) -> R<()> {
    let rect = rect_of(field(op, "rect")?, "rect")?;
    let water = flag(op, "water", true);
    let lowest = flag(op, "lowest", false);
    let average = flag(op, "average", false);
    w.with_sim(|s| {
        let v = if average {
            s.average_area_altitude(rect, water)
        } else {
            s.lowest_or_highest_area_altitude(rect, lowest, water)
        };
        check_bits(op, v)?;
        check_expect(op, float_json(v))
    })
}

/// `ldev_altitude_f`: slot 16, `rect [fx1, fz1, fx2, fz2]` (metres),
/// `lowest`, `water`; `expect`.
fn altitude_f(w: &World, op: &Json) -> R<()> {
    let a = arr(field(op, "rect")?, "rect")?;
    if a.len() != 4 {
        return Err("'rect' must be [fx1, fz1, fx2, fz2]".into());
    }
    let rect = [
        float_arg(&a[0], "rect")?,
        float_arg(&a[1], "rect")?,
        float_arg(&a[2], "rect")?,
        float_arg(&a[3], "rect")?,
    ];
    let lowest = flag(op, "lowest", false);
    let water = flag(op, "water", false);
    w.with_sim(|s| {
        let v = s.lowest_or_highest_area_altitude_f(rect, lowest, water);
        check_bits(op, v)?;
        check_expect(op, float_json(v))
    })
}

/// `ldev_level_internal`: slot 19, `rect`, `target`, `skip_water`, `force`,
/// `lot` (or null); `expect` (the call count).
fn level_internal(w: &World, op: &Json) -> R<()> {
    let rect = rect_of(field(op, "rect")?, "rect")?;
    let target = float_arg(field(op, "target")?, "target")?;
    let skip_water = flag(op, "skip_water", true);
    let force = flag(op, "force", false);
    let lot = opt_token_null(op, "lot")?;
    w.with_sim(|s| check_expect(op, ji(s.level_area_internal_vertices(rect, target, skip_water, force, lot))))
}

/// `ldev_level_bordering`: slot 20, `rect`, `h`, `lot`; `expect`.
fn level_bordering(w: &World, op: &Json) -> R<()> {
    let rect = rect_of(field(op, "rect")?, "rect")?;
    let h = float_arg(field(op, "h")?, "h")?;
    let lot = opt_token_null(op, "lot")?;
    w.with_sim(|s| check_expect(op, ji(s.level_bordering_vertices(rect, h, lot))))
}

/// `ldev_attempt_levelling`: slot 21, `rect`, `water`, `per_vertex`,
/// `max_slope` (default `"flt_max"`), `lot`; `expect`. The unused second
/// argument is not modelled.
fn attempt_levelling(w: &World, op: &Json) -> R<()> {
    let rect = rect_of(field(op, "rect")?, "rect")?;
    let water = flag(op, "water", true);
    let per_vertex = flag(op, "per_vertex", true);
    let m = opt(op, "max_slope").map_or(Ok(FLT_MAX), |v| float_arg(v, "max_slope"))?;
    let lot = opt_token_null(op, "lot")?;
    w.with_sim(|s| check_expect(op, ji(s.attempt_area_levelling(rect, water, per_vertex, m, lot))))
}

fn position_json(pos: Vec3, orient: i32) -> Json {
    Json::Object(vec![("pos".into(), pos_json(pos)), ("orient".into(), ji(orient))])
}

/// `ldev_object_position`: slot 39, `lot`, `cfg` (or null), `object {…}`
/// (a configuration object record); `expect {pos, orient}`.
fn object_position(w: &World, op: &Json) -> R<()> {
    let lot = token_of(op, "lot")?;
    let cfg = opt_token_null(op, "cfg")?;
    let obj = cfg_object_of(field(op, "object")?)?;
    w.with_sim(|s| {
        let (pos, orient) = s.object_position(lot, cfg, &obj);
        check_expect(op, position_json(pos, orient))
    })
}

/// `ldev_position_for_building`: slot 40, `lot`; `expect {pos, orient}` or
/// null (EAX 0, nothing written).
fn position_for_building(w: &World, op: &Json) -> R<()> {
    let lot = token_of(op, "lot")?;
    w.with_sim(|s| {
        let got = s
            .position_for_building(lot)
            .map_or(Json::Null, |(pos, orient)| position_json(pos, orient));
        check_expect(op, got)
    })
}

/// `ldev_boundary_texture`: slot 42, `sides [s0, s1, s2, s3]`; `expect
/// {texture, rotation, flip}` (null when the boundary map is empty).
fn boundary_texture(w: &World, op: &Json) -> R<()> {
    let a = i32_list(field(op, "sides")?, "sides")?;
    if a.len() != 4 {
        return Err("'sides' must have four entries".into());
    }
    w.with_sim(|s| {
        let got = s.boundary_lot_texture([a[0], a[1], a[2], a[3]]).map_or(Json::Null, |r| {
            Json::Object(vec![
                ("texture".into(), ji(r.texture)),
                ("rotation".into(), ji(r.rotation)),
                ("flip".into(), Json::Bool(r.flip)),
            ])
        });
        check_expect(op, got)
    })
}

/// `ldev_foundation`: a lookup; `occupant` (slot 48), `lot` (slot 51),
/// `cell [x, z]` with `kind` (slot 49), or `position [x, z]` with `kind`
/// (slot 50); `expect` (the foundation token or null).
fn foundation(w: &World, op: &Json) -> R<()> {
    w.with_sim(|s| {
        let got = if let Some(v) = opt(op, "occupant") {
            s.foundation_by_occupant(i64_of(v, "occupant")? as u64)
        } else if let Some(v) = opt(op, "lot") {
            s.lot_foundation(i64_of(v, "lot")? as u64)
        } else if let Some(v) = opt(op, "cell") {
            let (x, z) = pair(v, "cell")?;
            s.foundation_by_cell(i32_of(field(op, "kind")?, "kind")?, x, z)
        } else {
            let a = arr(field(op, "position")?, "position")?;
            s.foundation_by_position(
                i32_of(field(op, "kind")?, "kind")?,
                f32_of(&a[0], "x")?,
                f32_of(&a[1], "z")?,
            )
        };
        check_expect(op, got.map_or(Json::Null, |t| ji(t as i64)))
    })
}

/// `ldev_remove_foundation`: `position [x, z]` with `kind` (slot 52),
/// `cell [x, z]` (slot 53) or `occupant` (slot 60); `expect` (AL).
fn remove_foundation(w: &World, op: &Json) -> R<()> {
    w.with_sim(|s| {
        let got = if let Some(v) = opt(op, "occupant") {
            s.remove_building_foundation(i64_of(v, "occupant")? as u64)
        } else if let Some(v) = opt(op, "cell") {
            let (x, z) = pair(v, "cell")?;
            s.remove_lot_foundation_at(x, z)
        } else {
            let a = arr(field(op, "position")?, "position")?;
            s.remove_foundation(
                i32_of(field(op, "kind")?, "kind")?,
                f32_of(&a[0], "x")?,
                f32_of(&a[1], "z")?,
            )
        };
        check_expect(op, Json::Bool(got))
    })
}

/// `ldev_add_lot_foundation`: `lot`, then `type` (slot 55) or `cfg` (slot
/// 54; null allowed); `use_lot_altitude`, `level`, `walls`; `expect` (AL).
fn add_lot_foundation(w: &World, op: &Json) -> R<()> {
    let lot = token_of(op, "lot")?;
    let use_alt = flag(op, "use_lot_altitude", false);
    let level = flag(op, "level", false);
    let walls = flag(op, "walls", false);
    let ftype = opt(op, "type").map(|v| u32_of(v, "type")).transpose()?;
    let cfg = opt_token_null(op, "cfg")?;
    w.with_sim(|s| {
        let got = match ftype {
            Some(t) => s.add_lot_foundation_type(lot, t, use_alt, level, walls),
            None => s.add_lot_foundation(lot, cfg, use_alt, level, walls),
        };
        check_expect(op, Json::Bool(got))
    })
}

/// `ldev_foundation_required`: `occupant` (slot 56) or `min`, `max` (slot
/// 57); `expect`.
fn foundation_required(w: &World, op: &Json) -> R<()> {
    let occ = opt_token(op, "occupant")?;
    let bx = match occ {
        Some(_) => None,
        None => Some((vec3_of(field(op, "min")?, "min")?, vec3_of(field(op, "max")?, "max")?)),
    };
    w.with_sim(|s| {
        let got = match (occ, bx) {
            (Some(o), _) => s.is_occupant_foundation_required(o),
            (None, Some((min, max))) => s.is_box_foundation_required(min, max),
            _ => unreachable!(),
        };
        check_expect(op, Json::Bool(got))
    })
}

/// `ldev_add_building_foundation`: `building`, then `cfg` (slot 58) or
/// `type` (slot 59); `expect` (AL).
fn add_building_foundation(w: &World, op: &Json) -> R<()> {
    let b = token_of(op, "building")?;
    let ftype = opt(op, "type").map(|v| u32_of(v, "type")).transpose()?;
    let cfg = opt_token(op, "cfg")?;
    w.with_sim(|s| {
        let got = match (ftype, cfg) {
            (Some(t), _) => s.add_building_foundation_type(b, t),
            (None, Some(c)) => s.add_building_foundation(b, c),
            _ => return Err("need 'cfg' or 'type'".into()),
        };
        check_expect(op, Json::Bool(got))
    })
}

/// `ldev_expect_terrain`: `vertex [[x, z, value]]`, the vertex altitudes.
fn expect_terrain(w: &World, op: &Json) -> R<()> {
    for e in arr(field(op, "vertex")?, "vertex")? {
        let a = arr(e, "vertex")?;
        let (x, z) = (i32_of(&a[0], "x")?, i32_of(&a[1], "z")?);
        let got = jf(w.vertex_alt(x, z));
        if !matches(&got, &a[2], false) {
            return Err(format!("vertex ({x}, {z}): got {}, expected {}", show(&got), show(&a[2])));
        }
    }
    Ok(())
}

/// `ldev_expect_occ`: `id`; any of `exists`, `position`, `bbox`, `cells`,
/// `orientation`, `foundation_kind`, `foundation_type`.
fn expect_occ(w: &World, op: &Json) -> R<()> {
    let id = token_of(op, "id")?;
    let occs = w.occs.borrow();
    let o = occs.get(&id);
    if let Some(e) = opt(op, "exists") {
        if bool_of(e, "exists")? != o.is_some() {
            return Err(format!("occupant {id} exists: {}", o.is_some()));
        }
    }
    let Some(o) = o else {
        return Ok(());
    };
    let got = vec![
        ("exists".to_owned(), Json::Bool(true)),
        ("position".to_owned(), pos_json(o.position)),
        ("bbox".to_owned(), Json::Array(vec![pos_json(o.bbox.0), pos_json(o.bbox.1)])),
        ("cells".to_owned(), rect_json(o.cells)),
        ("orientation".to_owned(), ji(o.orientation)),
        ("foundation_kind".to_owned(), o.foundation_kind.map_or(Json::Null, ji)),
        ("foundation_type".to_owned(), ji(o.foundation_type)),
    ];
    let mut want = Vec::new();
    for (k, v) in op.as_object().ok_or("op must be an object")? {
        if !matches!(k.as_str(), "op" | "id" | "description" | "note") {
            want.push((k.clone(), v.clone()));
        }
    }
    scalar::check_state(&got, &Json::Object(want))
}

// ---- persistence -------------------------------------------------------------------------------

fn save(w: &World, op: &Json) -> R<()> {
    w.with_sim(|s| {
        let registry = Rc::new(RefCell::new(SaveRegistry::new()));
        let mut b = ByteWriter::with_registry(registry);
        let ok = s.write(&mut b);
        if let Some(v) = opt(op, "ok") {
            if bool_of(v, "ok")? != ok {
                return Err(format!("write returned {ok}"));
            }
        }
        check_expect(op, Json::Str(hex_encode(b.bytes())))
    })
}

struct HostResolver;
impl RefResolver for HostResolver {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        if class_id == IID_BUILDING_OCCUPANT {
            Some(RefValue::Host {
                token: u64::from(object_id),
                class_id,
            })
        } else {
            None
        }
    }
}

fn load(w: &World, op: &Json) -> R<()> {
    let bytes = hex_decode(str_of(field(op, "hex")?, "hex")?)?;
    w.with_sim(|s| {
        let mut resolver = HostResolver;
        let ok = s.read(&mut ByteReader::with_resolver(&bytes, &mut resolver));
        check_expect(op, Json::Bool(ok))
    })
}

// ---- checks ------------------------------------------------------------------------------------

/// `ldev_expect`: any of `initialised`, `daily_state`, `visited`,
/// `net_changed`, `unit_changed`, `net_snap`, `unit_snap`, `zots_allowed`,
/// `no_zot_until_day`, `can_check_commute`, `force_empty`, `night_mode`,
/// `night_acc`, `night_batch`, `night_cursor`, `lit_flag`, `verts_x`,
/// `extent_x`, `extent_z`, `night` (the deque's tokens, `null` for a null
/// slot), `night_len`, `per_day`, `cell_counts`, `lcg`, `mul`, `crt`,
/// `tuning {field: value}`.
fn expect(w: &World, op: &Json) -> R<()> {
    let statics = w.statics.borrow().clone();
    w.with_sim(|s| {
        let mut got: Vec<(String, Json)> = Vec::new();
        let mut put = |k: &str, v: Json| got.push((k.to_owned(), v));
        put("initialised", Json::Bool(s.is_initialised()));
        put("daily_state", ji(s.daily_state()));
        put("visited", ji(s.visited()));
        put("net_changed", Json::Bool(s.net_changed()));
        put("unit_changed", Json::Bool(s.unit_changed()));
        put("net_snap", Json::Bool(s.net_snap()));
        put("unit_snap", Json::Bool(s.unit_snap()));
        put("zots_allowed", Json::Bool(s.zots_allowed()));
        put("no_zot_until_day", ji(s.no_zot_until_day()));
        put("can_check_commute", Json::Bool(s.can_check_commute()));
        put("force_empty", Json::Bool(s.force_empty()));
        put("night_mode", ji(s.night_mode()));
        put("night_acc", ji(s.night_acc()));
        put("night_batch", ji(s.night_batch()));
        put("night_cursor", ji(s.night_cursor() as i64));
        put("lit_flag", Json::Bool(s.lit_flag()));
        put("verts_x", ji(s.verts_x()));
        put("extent_x", Json::Float(f64::from(s.extent_x())));
        put("extent_z", Json::Float(f64::from(s.extent_z())));
        put(
            "night",
            Json::Array(
                s.night()
                    .iter()
                    .map(|e| e.map_or(Json::Null, |b| ji(b as i64)))
                    .collect(),
            ),
        );
        put("night_len", ji(s.night_len() as i64));
        put("per_day", ji(s.per_day()));
        let (cx, cz) = s.cell_counts();
        put("cell_counts", Json::Array(vec![ji(cx), ji(cz)]));
        if let Some(st) = &statics {
            let st = st.borrow();
            put("lcg", ji(st.rng.lcg_state()));
            put("mul", ji(st.rng.mul_state()));
            put("crt", ji(st.crt.state()));
        }
        let t = s.tuning();
        put(
            "tuning",
            Json::Object(vec![
                ("stay_new_time".into(), ji(t.stay_new_time)),
                (
                    "thresholds".into(),
                    Json::Array(t.thresholds.iter().map(|&v| Json::Float(f64::from(v))).collect()),
                ),
                (
                    "max_slope_for_flatness".into(),
                    Json::Float(f64::from(t.max_slope_for_flatness)),
                ),
                (
                    "max_area_slope_for_levelling".into(),
                    Json::Float(f64::from(t.max_area_slope_for_levelling)),
                ),
                ("default_lot_foundation_type".into(), ji(t.default_lot_foundation_type)),
                ("attempt_lot_levelling".into(), Json::Bool(t.attempt_lot_levelling)),
                (
                    "attempt_internal_lot_levelling".into(),
                    Json::Bool(t.attempt_internal_lot_levelling),
                ),
            ]),
        );
        let want = op.as_object().ok_or("op must be an object")?;
        let mut errors = Vec::new();
        for (k, v) in want {
            if k == "op" || k == "description" || k == "note" {
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

/// `ldev_expect_lot`: `lot`; any of `exists`, `state`, `condition`, `zone`,
/// `building_type`, `wealth`, `flags`, `historical`, `has_power`, `cfg`,
/// `building`, `unit`, `designate`, `rect`, `altitude`, `facing`,
/// `locked`, `prop {name: value}` (the lot counters and the zot).
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
        ("exists".to_owned(), Json::Bool(true)),
        ("state".to_owned(), ji(l.state)),
        ("condition".to_owned(), ji(l.condition)),
        ("zone".to_owned(), ji(l.zone)),
        ("building_type".to_owned(), ji(l.building_type)),
        ("wealth".to_owned(), ji(l.wealth)),
        ("flags".to_owned(), ji(l.flags)),
        ("historical".to_owned(), Json::Bool(l.historical)),
        ("has_power".to_owned(), Json::Bool(l.has_power)),
        ("cfg".to_owned(), l.cfg.map_or(Json::Null, |c| ji(c as i64))),
        ("building".to_owned(), l.building.map_or(Json::Null, |b| ji(b as i64))),
        ("unit".to_owned(), l.unit.map_or(Json::Null, |u| ji(u as i64))),
        ("designate".to_owned(), l.designate.map_or(Json::Null, |d| ji(d as i64))),
        ("altitude".to_owned(), jf(l.altitude)),
        ("facing".to_owned(), ji(l.facing)),
        ("locked".to_owned(), Json::Bool(l.locked)),
        (
            "rect".to_owned(),
            rect_json(CellRect::new(l.x, l.z, l.x + l.w as i32 - 1, l.z + l.h as i32 - 1)),
        ),
    ];
    if let Some(p) = opt(op, "prop") {
        let mut pj = Vec::new();
        for (k, v) in p.as_object().ok_or("'prop' must be an object")? {
            let pid = match k.as_str() {
                "low_desirability" => lot_prop::LOW_DESIRABILITY,
                "no_power" => lot_prop::NO_POWER,
                "no_water" => lot_prop::NO_WATER,
                "travel" => lot_prop::TRAVEL,
                "zot" => lot_prop::ZOT,
                other => u32::from_str_radix(other.trim_start_matches("0x"), 16)
                    .map_err(|_| format!("bad prop '{other}'"))?,
            };
            let cur = l.props.get(&pid).copied();
            let gotv = cur.map_or(Json::Null, ji);
            if !matches(&gotv, v, false) {
                return Err(format!("prop {k}: got {}, expected {}", show(&gotv), show(v)));
            }
            pj.push((k.clone(), gotv));
        }
        got.push(("prop".to_owned(), Json::Object(pj)));
    }
    let mut want = Vec::new();
    for (k, v) in op.as_object().ok_or("op must be an object")? {
        if !matches!(k.as_str(), "op" | "lot" | "description" | "note") {
            want.push((k.clone(), v.clone()));
        }
    }
    scalar::check_state(&got, &Json::Object(want))
}

/// `ldev_expect_draws`: the draws traced since the last check, one entry
/// each: `"R1"` (kind only) or `["R1", arg, value]` (value compared
/// exactly); `keep: true` does not clear the trace.
fn expect_draws(w: &World, op: &Json) -> R<()> {
    let st = w.statics.borrow().clone().ok_or("no statics (use ldev_create)")?;
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
                let k = str_of(&a[0], "kind")?;
                let n = u32_of(&a[1], "arg")?;
                let v = a[2].as_f64().ok_or("draw value must be a number")?;
                g.kind.name() == k && g.arg == n && (g.value - v).abs() < 1e-9
            }
            _ => return Err("draw entries are \"Rn\" or [\"Rn\", arg, value]".into()),
        };
        if !ok {
            return Err(format!("draw {i} differs: got {}", show_got()));
        }
    }
    let _ = DrawKind::R1;
    Ok(())
}

/// `ldev_eval`: a pure function. `fn` one of
/// `is_empty_lot_suitable_for_zot [zone, x, z]`, `desirability_tolerance
/// [x, z]`, `answers_iid [iid]`, `rubble_count [minx, miny, minz, maxx,
/// maxy, maxz]` (→ `{count, small, large}`), `area_slope_of` is on the sim
/// (`ldev_area_slope`).
fn eval(op: &Json) -> R<()> {
    let f = str_of(field(op, "fn")?, "fn")?;
    let got = match f {
        "is_empty_lot_suitable_for_zot" => Json::Bool(is_empty_lot_suitable_for_zot(
            i32_of(arg(op, 0)?, "zone")?,
            i32_of(arg(op, 1)?, "x")?,
            i32_of(arg(op, 2)?, "z")?,
        )),
        "desirability_tolerance" => ji(LotDeveloper::desirability_tolerance(
            i32_of(arg(op, 0)?, "x")?,
            i32_of(arg(op, 1)?, "z")?,
            &SEEDS,
        )),
        "answers_iid" => Json::Bool(answers_iid(u32_of(arg(op, 0)?, "iid")?)),
        "rubble_count" => {
            let min = [
                f32_of(arg(op, 0)?, "x")?,
                f32_of(arg(op, 1)?, "y")?,
                f32_of(arg(op, 2)?, "z")?,
            ];
            let max = [
                f32_of(arg(op, 3)?, "x")?,
                f32_of(arg(op, 4)?, "y")?,
                f32_of(arg(op, 5)?, "z")?,
            ];
            let s = rubble_count(min, max);
            Json::Object(vec![
                ("count".into(), ji(s.count)),
                ("small".into(), Json::Bool(s.small)),
                ("large".into(), Json::Bool(s.large)),
            ])
        }
        "reason_const" => ji(match str_of(arg(op, 0)?, "name")? {
            "low_desirability" => reason::LOW_DESIRABILITY,
            "no_power" => reason::NO_POWER,
            "no_water" => reason::NO_WATER,
            "commute" => reason::COMMUTE,
            other => return Err(format!("unknown reason '{other}'")),
        }),
        other => return Err(format!("unknown eval fn '{other}'")),
    };
    check_expect(op, got)
}
