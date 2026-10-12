// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops `bds_…` for `fc4_core::development_simulator`
//! (`docs/specs/building-development.md`): a scripted city (exemplars, lot
//! configurations, growth developers with scripted demand and lists, a
//! scripted tract developer, lots, cells, occupants), the process-wide
//! statics, and ops that drive the BDS. Host calls with effects are logged
//! as events; the generator's draws are logged as `draw` events at the
//! moment they happen, so vectors see their interleaving with the tract
//! developer's calls. See README.md ("Building development ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::{Rc, Weak};

use fc4_core::demand::{DemandNode, DemandPoster};
use fc4_core::development::{ConstructionProfile, GrowthItem, GrowthProfile, Outcome};
use fc4_core::development_simulator::{
    msg, query_interface, AdviceSubject, BdsInterface, BdsProcess, BuildingConfigObject, CellRect, CommandParam,
    DeveloperId, DevelopmentHost, DevelopmentSim, GrowthDevelopers, LotConfigInfo, LotId, Message, OccupantId,
    PlanEntry, SharedProcess, Vec3,
};
use fc4_core::grid::{CityDims, GridProvider, SimGrid, TractGrid};
use fc4_core::property::PropertyHolder;
use fc4_core::rng::FixedSeeds;
use fc4_core::stream::{ByteReader, ByteWriter, OutStream, RefResolver, RefValue, ResKey, SaveObject};
use fc4_core::variant::{mem, Elem, Variant};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, check_state, f32_of, holder_of, i64_of, jf, ji, opt, str_of, u32_of,
    EventLog, R,
};

/// The class ID the fake world gives growth developers in saves (the real
/// one, growth-developer §9).
const DEVELOPER_CLASS: u32 = 0x0990_C138;
/// The class ID of the fake occupants.
const OCCUPANT_CLASS: u32 = 0x0CC0_0001;

/// A growth developer's scripted behaviour, by developer type.
#[derive(Clone)]
struct DevSpec {
    /// `None`: no source node.
    ad: Option<f32>,
    purpose: Option<u32>,
    sub_type: Option<u32>,
    valid: bool,
    growth: Vec<GrowthItem>,
    decline: Vec<GrowthItem>,
    /// `IsMapUpdateFinished` answers false this many times after each
    /// `StartMapUpdate`.
    finish_after: u32,
}

impl Default for DevSpec {
    fn default() -> Self {
        DevSpec {
            ad: Some(0.0),
            purpose: None,
            sub_type: None,
            valid: true,
            growth: Vec::new(),
            decline: Vec::new(),
            finish_after: 0,
        }
    }
}

/// A developer object.
#[derive(Clone, Default)]
struct DevObj {
    dev_type: u32,
    refs: i64,
    busy: u32,
}

#[derive(Clone, Default)]
struct Lot {
    building_type: u32,
    rect: CellRect,
    stage: Option<u8>,
    building: Option<OccupantId>,
    state: u32,
    center: Vec3,
    slope: f32,
    dev_slope: f32,
    demolishable: bool,
}

#[derive(Clone, Copy, Default)]
struct CellInfo {
    zone: i8,
    powered: bool,
    lv_type: u32,
    lv: u8,
    lot: Option<LotId>,
}

#[derive(Clone, Default)]
struct Occ {
    groups: Vec<u32>,
    preview: bool,
    pos: Vec3,
    bbox: (Vec3, Vec3),
    cost: Option<i64>,
    refs: i64,
    load_fails: bool,
}

pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    process: RefCell<Option<SharedProcess>>,
    sim: RefCell<Option<Rc<DevelopmentSim>>>,
    next_token: Cell<u64>,
    day: Cell<u32>,
    difficulty: Cell<u32>,
    cell_widths: Cell<(f32, f32)>,
    cell_counts: Cell<(i32, i32)>,
    altitude: Cell<f32>,
    area_altitude: Cell<f32>,
    exemplars: RefCell<Vec<(ResKey, Rc<PropertyHolder>)>>,
    lot_configs: RefCell<Vec<LotConfigInfo>>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    census: RefCell<BTreeMap<u32, f32>>,
    specs: RefCell<BTreeMap<u32, DevSpec>>,
    devs: RefCell<BTreeMap<u64, DevObj>>,
    grow_returns: RefCell<VecDeque<i32>>,
    grow_default: Cell<i32>,
    decline_returns: RefCell<VecDeque<i32>>,
    decline_default: Cell<i32>,
    lots: RefCell<BTreeMap<u64, Lot>>,
    cells: RefCell<BTreeMap<(i32, i32), CellInfo>>,
    cell_default: Cell<CellInfo>,
    occupants: RefCell<BTreeMap<u64, Occ>>,
    building_cells: RefCell<BTreeSet<(i32, i32)>>,
    occupant_manager: Cell<bool>,
    insert_ok: Cell<bool>,
    funds: Cell<i64>,
    demolish_cost: Cell<i64>,
    saves: RefCell<BTreeMap<String, Vec<u8>>>,
    /// The building maps this world created (newest last).
    grids: RefCell<Vec<Rc<SimGrid<u8>>>>,
    /// Object ID → object, for references written by `bds_save`.
    saved_objects: RefCell<BTreeMap<u32, RefValue>>,
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            me: RefCell::new(Weak::new()),
            events: EventLog::default(),
            process: RefCell::new(None),
            sim: RefCell::new(None),
            next_token: Cell::new(0x100),
            day: Cell::new(1000),
            difficulty: Cell::new(1),
            cell_widths: Cell::new((16.0, 16.0)),
            cell_counts: Cell::new((64, 64)),
            altitude: Cell::new(300.0),
            area_altitude: Cell::new(250.0),
            exemplars: RefCell::new(Vec::new()),
            lot_configs: RefCell::new(Vec::new()),
            tuning: RefCell::new(None),
            census: RefCell::new(BTreeMap::new()),
            specs: RefCell::new(BTreeMap::new()),
            devs: RefCell::new(BTreeMap::new()),
            grow_returns: RefCell::new(VecDeque::new()),
            grow_default: Cell::new(0),
            decline_returns: RefCell::new(VecDeque::new()),
            decline_default: Cell::new(0),
            lots: RefCell::new(BTreeMap::new()),
            cells: RefCell::new(BTreeMap::new()),
            cell_default: Cell::new(CellInfo::default()),
            occupants: RefCell::new(BTreeMap::new()),
            building_cells: RefCell::new(BTreeSet::new()),
            occupant_manager: Cell::new(true),
            insert_ok: Cell::new(true),
            funds: Cell::new(100_000),
            demolish_cost: Cell::new(0),
            saves: RefCell::new(BTreeMap::new()),
            grids: RefCell::new(Vec::new()),
            saved_objects: RefCell::new(BTreeMap::new()),
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn rc(&self) -> Rc<World> {
        self.me.borrow().upgrade().expect("world alive")
    }

    fn token(&self) -> u64 {
        let t = self.next_token.get();
        self.next_token.set(t + 1);
        t
    }

    fn sim(&self) -> R<Rc<DevelopmentSim>> {
        self.sim.borrow().clone().ok_or_else(|| "no BDS (bds_create)".into())
    }

    fn process(&self) -> SharedProcess {
        if let Some(p) = self.process.borrow().clone() {
            return p;
        }
        let p = new_process(0);
        *self.process.borrow_mut() = Some(p.clone());
        p
    }

    /// Moves the generator's recorded draws into the event log.
    fn flush_draws(&self) {
        let Some(p) = self.process.borrow().clone() else { return };
        for d in p.take_trace() {
            self.events.push(
                "draw",
                vec![
                    ("n", ji(d.n)),
                    ("value", ji(d.value)),
                    ("stepped", Json::Bool(d.stepped)),
                ],
            );
        }
    }

    fn log(&self, kind: &str, fields: Vec<(&str, Json)>) {
        self.flush_draws();
        self.events.push(kind, fields);
    }

    fn dev_type(&self, dev: DeveloperId) -> u32 {
        self.devs.borrow().get(&dev).map_or(0, |d| d.dev_type)
    }

    fn spec(&self, dev: DeveloperId) -> DevSpec {
        let t = self.dev_type(dev);
        self.specs.borrow().get(&t).cloned().unwrap_or_default()
    }

    fn cell(&self, x: i32, z: i32) -> CellInfo {
        self.cells
            .borrow()
            .get(&(x, z))
            .copied()
            .unwrap_or(self.cell_default.get())
    }

    fn lot(&self, lot: LotId) -> Lot {
        self.lots.borrow().get(&lot).cloned().unwrap_or_default()
    }

    fn occ(&self, o: OccupantId) -> Occ {
        self.occupants.borrow().get(&o).cloned().unwrap_or_default()
    }

    fn occ_refs(&self, o: OccupantId, delta: i64) {
        self.occupants.borrow_mut().entry(o).or_default().refs += delta;
    }
}

const PROCESS_SEEDS: FixedSeeds = FixedSeeds {
    wall_clock: 1,
    process_clock: 1,
};

fn new_process(seed: u32) -> SharedProcess {
    traced(BdsProcess::new(seed, &PROCESS_SEEDS))
}

fn traced(p: SharedProcess) -> SharedProcess {
    p.enable_trace();
    p
}

fn profile_json(p: &GrowthProfile) -> Vec<(&'static str, Json)> {
    vec![
        ("dev", ji(p.developer as i64)),
        ("x", ji(p.x)),
        ("z", ji(p.z)),
        ("stage", ji(p.stage)),
    ]
}

fn vec3_json(v: Vec3) -> Json {
    Json::Array(v.iter().map(|&x| jf(x)).collect())
}

fn rect_json(r: CellRect) -> Json {
    Json::Array(vec![ji(r.0), ji(r.1), ji(r.2), ji(r.3)])
}

impl DemandPoster for World {
    fn active_demand_changed(&self, node: u64, key: u32, data2: i32) {
        self.log(
            "active_demand_changed",
            vec![("node", ji(node as i64)), ("key", ji(key)), ("data2", ji(data2))],
        );
    }
}

impl GridProvider<u8> for World {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        let (cx, cz) = self.cell_counts.get();
        let (wx, wz) = self.cell_widths.get();
        let dims = CityDims {
            cells_x: cx,
            cells_z: cz,
            cell_width_x: wx,
            cell_width_z: wz,
        };
        let g = SimGrid::<u8>::create(&dims, instance, tract_size);
        let t = self.token();
        g.set_host_token(t);
        self.grids.borrow_mut().push(g.clone());
        self.log(
            "create_grid",
            vec![
                ("instance", ji(instance)),
                ("tract_size", ji(tract_size)),
                ("id", ji(t as i64)),
            ],
        );
        Some(g as Rc<dyn TractGrid<u8>>)
    }
}

impl DevelopmentHost for World {
    fn sim_day(&self) -> u32 {
        self.day.get()
    }
    fn difficulty(&self) -> u32 {
        self.difficulty.get()
    }
    fn cell_widths(&self) -> (f32, f32) {
        self.cell_widths.get()
    }
    fn terrain_cell_counts(&self) -> (i32, i32) {
        self.cell_counts.get()
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        let (wx, wz) = self.cell_widths.get();
        ((x / wx).floor() as i32, (z / wz).floor() as i32)
    }
    fn cell_center(&self, cx: i32, cz: i32) -> (f32, f32) {
        let (wx, wz) = self.cell_widths.get();
        ((cx as f32 + 0.5) * wx, (cz as f32 + 0.5) * wz)
    }
    fn altitude(&self, _x: f32, _z: f32) -> f32 {
        self.altitude.get()
    }

    fn exemplars(&self) -> Vec<(ResKey, Rc<PropertyHolder>)> {
        self.exemplars.borrow().clone()
    }
    fn exemplar_string(&self, holder: &PropertyHolder, id: u32) -> Option<Vec<u8>> {
        holder.get(id).and_then(Variant::bytes)
    }
    fn lot_configurations(&self) -> Vec<LotConfigInfo> {
        self.lot_configs.borrow().clone()
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }

    fn register_static_cleanup(&self) {
        self.log("static_cleanup", vec![]);
    }
    fn post_building_types_updated(&self, count: u32) {
        self.log(
            "post",
            vec![("type", ji(msg::BUILDING_TYPES_UPDATED)), ("data2", ji(count))],
        );
    }
    fn subscribe(&self, msg_types: &[u32]) {
        let types = Json::Array(msg_types.iter().map(|&t| ji(t)).collect());
        self.log("subscribe", vec![("types", types)]);
    }
    fn unsubscribe(&self, msg_types: &[u32]) {
        let types = Json::Array(msg_types.iter().map(|&t| ji(t)).collect());
        self.log("unsubscribe", vec![("types", types)]);
    }
    fn register_command(&self, id: u32, name: &str) {
        self.log(
            "register_command",
            vec![("id", ji(id)), ("name", Json::Str(name.into()))],
        );
    }
    fn unregister_command(&self, id: u32) {
        self.log("unregister_command", vec![("id", ji(id))]);
    }
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        self.log(
            "add_agent",
            vec![
                ("level", ji(level)),
                ("name", Json::Str(name.into())),
                ("flags", ji(flags)),
            ],
        );
    }
    fn remove_agent(&self) {
        self.log("remove_agent", vec![]);
    }
    fn register_cheat(&self, id: u32, name: &str) {
        self.log("register_cheat", vec![("id", ji(id)), ("name", Json::Str(name.into()))]);
    }
    fn unregister_cheat(&self, id: u32) {
        self.log("unregister_cheat", vec![("id", ji(id))]);
    }
    fn acquire_services(&self) {
        self.log("acquire_services", vec![]);
    }
    fn release_services(&self) {
        self.log("release_services", vec![]);
    }

    fn census_supply(&self, id: u32) -> Option<f32> {
        self.census.borrow().get(&id).copied()
    }
    fn demand_active_value(&self, node: u64) -> f32 {
        self.spec(node).ad.unwrap_or(0.0)
    }
    fn create_demand_node(&self) -> Option<Rc<DemandNode>> {
        let t = self.token();
        self.log("create_node", vec![("id", ji(t as i64))]);
        Some(DemandNode::new(self.rc() as Rc<dyn DemandPoster>, t))
    }

    fn tract_grow(&self, profile: &GrowthProfile, outcomes: &mut Vec<Outcome>, flag: &mut bool) -> i32 {
        let built = self
            .grow_returns
            .borrow_mut()
            .pop_front()
            .unwrap_or(self.grow_default.get());
        let mut f = profile_json(profile);
        f.push(("type", ji(self.dev_type(profile.developer))));
        f.push(("outcomes_in", ji(outcomes.len() as i64)));
        f.push(("flag_in", Json::Bool(*flag)));
        f.push(("built", ji(built)));
        self.log("tract_grow", f);
        outcomes.push(if built > 0 {
            Outcome::Success
        } else {
            Outcome::UnknownFailure
        });
        built
    }
    fn tract_decline(&self, profile: &GrowthProfile) -> i32 {
        let result = self
            .decline_returns
            .borrow_mut()
            .pop_front()
            .unwrap_or(self.decline_default.get());
        let mut f = profile_json(profile);
        f.push(("type", ji(self.dev_type(profile.developer))));
        f.push(("result", ji(result)));
        self.log("tract_decline", f);
        result
    }

    fn lot_building_type(&self, lot: LotId) -> u32 {
        self.lot(lot).building_type
    }
    fn lot_bounding_rect(&self, lot: LotId) -> CellRect {
        self.lot(lot).rect
    }
    fn lot_growth_stage(&self, lot: LotId) -> Option<u8> {
        self.lot(lot).stage
    }
    fn lot_building(&self, lot: LotId) -> Option<OccupantId> {
        self.lot(lot).building
    }
    fn lot_state(&self, lot: LotId) -> u32 {
        self.lot(lot).state
    }
    fn lot_center(&self, lot: LotId) -> Vec3 {
        self.lot(lot).center
    }
    fn lot_slope(&self, lot: LotId) -> f32 {
        self.lot(lot).slope
    }
    fn lot_demolishable(&self, lot: LotId) -> bool {
        self.lot(lot).demolishable
    }
    fn lot_at(&self, x: i32, z: i32) -> Option<LotId> {
        self.cell(x, z).lot
    }
    fn lots_in_cell_rect(&self, rect: CellRect) -> Vec<LotId> {
        self.lots
            .borrow()
            .iter()
            .filter(|(_, l)| l.rect.0 <= rect.2 && rect.0 <= l.rect.2 && l.rect.1 <= rect.3 && rect.1 <= l.rect.3)
            .map(|(&id, _)| id)
            .collect()
    }
    fn lot_developer_slope(&self, lot: LotId) -> f32 {
        self.lot(lot).dev_slope
    }
    fn update_lot_elements(&self, lot: LotId, had_foundation: bool) {
        self.log(
            "update_lot_elements",
            vec![("lot", ji(lot as i64)), ("had_foundation", Json::Bool(had_foundation))],
        );
    }
    fn area_altitude(&self, rect: [f32; 4]) -> f32 {
        self.log(
            "area_altitude",
            vec![("rect", Json::Array(rect.iter().map(|&v| jf(v)).collect()))],
        );
        self.area_altitude.get()
    }

    fn zone_type(&self, x: i32, z: i32) -> i8 {
        self.cell(x, z).zone
    }
    fn dezone(&self, rect: CellRect, flag: bool) {
        self.log("dezone", vec![("rect", rect_json(rect)), ("flag", Json::Bool(flag))]);
    }
    fn powered(&self, x: i32, z: i32) -> bool {
        self.cell(x, z).powered
    }
    fn land_value_type(&self, x: i32, z: i32) -> u32 {
        self.cell(x, z).lv_type
    }
    fn land_value(&self, x: i32, z: i32) -> u8 {
        self.cell(x, z).lv
    }
    fn demolish_lots(&self, lots: &[LotId]) -> i64 {
        let cost = self.demolish_cost.get();
        self.log(
            "demolish_lots",
            vec![
                ("lots", Json::Array(lots.iter().map(|&l| ji(l as i64)).collect())),
                ("cost", ji(cost)),
            ],
        );
        cost
    }
    fn total_funds(&self) -> i64 {
        self.funds.get()
    }
    fn set_total_funds(&self, v: i64) {
        self.funds.set(v);
        self.log("set_funds", vec![("value", ji(v))]);
    }

    fn has_occupant_manager(&self) -> bool {
        self.occupant_manager.get()
    }
    fn insert_occupant(&self, occ: OccupantId) -> bool {
        let ok = self.insert_ok.get();
        if ok {
            self.occ_refs(occ, 1);
        }
        self.log(
            "insert_occupant",
            vec![("occ", ji(occ as i64)), ("result", Json::Bool(ok))],
        );
        ok
    }
    fn remove_occupant(&self, occ: OccupantId) -> bool {
        self.log("remove_occupant", vec![("occ", ji(occ as i64))]);
        true
    }
    fn building_occupant_at_cell(&self, cx: i32, cz: i32) -> bool {
        self.building_cells.borrow().contains(&(cx, cz))
    }
    fn occupant_has_property(&self, occ: OccupantId, id: u32) -> bool {
        id == fc4_core::development_simulator::prop::PREVIEW_OCCUPANT && self.occ(occ).preview
    }
    fn occupant_property_i64(&self, occ: OccupantId, id: u32) -> Option<i64> {
        if id == fc4_core::development_simulator::prop::DEMOLITION_COST {
            self.occ(occ).cost
        } else {
            None
        }
    }
    fn occupant_in_group(&self, occ: OccupantId, group: u32) -> bool {
        self.occ(occ).groups.contains(&group)
    }
    fn occupant_position(&self, occ: OccupantId) -> Vec3 {
        self.occ(occ).pos
    }
    fn create_building_occupant(&self) -> Option<OccupantId> {
        let t = self.token();
        let template = self.occupants.borrow().get(&0).cloned().unwrap_or_default();
        self.occupants.borrow_mut().insert(t, Occ { refs: 1, ..template });
        self.log("create_occupant", vec![("occ", ji(t as i64))]);
        Some(t)
    }
    fn set_building_type(&self, occ: OccupantId, t: u32) {
        self.log("set_building_type", vec![("occ", ji(occ as i64)), ("type", ji(t))]);
    }
    fn load_building_occupant(&self, occ: OccupantId, t: u32) -> bool {
        let ok = !self.occ(occ).load_fails;
        self.log(
            "load_occupant",
            vec![("occ", ji(occ as i64)), ("type", ji(t)), ("result", Json::Bool(ok))],
        );
        ok
    }
    fn occupant_bounding_box(&self, occ: OccupantId) -> (Vec3, Vec3) {
        self.occ(occ).bbox
    }
    fn set_occupant_bounding_box(&self, occ: OccupantId, min: Vec3, max: Vec3) {
        if let Some(o) = self.occupants.borrow_mut().get_mut(&occ) {
            o.bbox = (min, max);
        }
        self.log(
            "set_bbox",
            vec![
                ("occ", ji(occ as i64)),
                ("min", vec3_json(min)),
                ("max", vec3_json(max)),
            ],
        );
    }
    fn set_occupant_position(&self, occ: OccupantId, pos: Vec3) {
        if let Some(o) = self.occupants.borrow_mut().get_mut(&occ) {
            o.pos = pos;
        }
        self.log("set_position", vec![("occ", ji(occ as i64)), ("pos", vec3_json(pos))]);
    }
    fn set_occupant_orientation(&self, occ: OccupantId, orientation: i32) {
        self.log(
            "set_orientation",
            vec![("occ", ji(occ as i64)), ("orientation", ji(orientation))],
        );
    }
    fn init_occupant(&self, occ: OccupantId) {
        self.log("init_occupant", vec![("occ", ji(occ as i64))]);
    }
    fn add_ref(&self, occ: OccupantId) {
        self.occ_refs(occ, 1);
        self.log("add_ref", vec![("occ", ji(occ as i64))]);
    }
    fn release(&self, occ: OccupantId) {
        self.occ_refs(occ, -1);
        self.log("release", vec![("occ", ji(occ as i64))]);
    }
    fn object_ref(&self, occ: OccupantId) -> RefValue {
        RefValue::Host {
            token: occ,
            class_id: OCCUPANT_CLASS,
        }
    }
    fn resolve_occupant(&self, r: &RefValue) -> Option<OccupantId> {
        match r {
            RefValue::Host { token, .. } => {
                self.occ_refs(*token, 1);
                Some(*token)
            }
            _ => None,
        }
    }

    fn push_advisor_event(&self, advisor: u32, event: u32, discarded: &AdviceSubject) {
        self.log(
            "advisor_event",
            vec![
                ("advisor", ji(advisor)),
                ("id", ji(event)),
                ("subject", Json::Null),
                (
                    "discarded",
                    Json::Object(vec![
                        ("x".into(), jf(discarded.x)),
                        ("z".into(), jf(discarded.z)),
                        ("lot".into(), ji(discarded.lot as i64)),
                        ("data1".into(), ji(discarded.data1)),
                    ]),
                ),
            ],
        );
    }
}

impl GrowthDevelopers for World {
    fn create_developer(&self, _exemplar: &Rc<PropertyHolder>) -> Option<DeveloperId> {
        let t = self.token();
        self.devs.borrow_mut().insert(
            t,
            DevObj {
                dev_type: 0,
                refs: 1,
                busy: 0,
            },
        );
        self.log("create_developer", vec![("dev", ji(t as i64))]);
        Some(t)
    }
    fn developer_init(&self, dev: DeveloperId, exemplar: &Rc<PropertyHolder>) -> bool {
        let t = fc4_core::props::u32_of(exemplar, 0x21).unwrap_or(0);
        if let Some(d) = self.devs.borrow_mut().get_mut(&dev) {
            d.dev_type = t;
        }
        self.log("developer_init", vec![("dev", ji(dev as i64)), ("type", ji(t))]);
        true
    }
    fn developer_shutdown(&self, dev: DeveloperId) -> bool {
        self.log("developer_shutdown", vec![("dev", ji(dev as i64))]);
        true
    }
    fn developer_release(&self, dev: DeveloperId) {
        if let Some(d) = self.devs.borrow_mut().get_mut(&dev) {
            d.refs -= 1;
        }
        self.log("developer_release", vec![("dev", ji(dev as i64))]);
    }
    fn developer_sub_type(&self, dev: DeveloperId) -> u32 {
        self.spec(dev).sub_type.unwrap_or_else(|| self.dev_type(dev))
    }
    fn developer_purpose(&self, dev: DeveloperId) -> u32 {
        let t = self.dev_type(dev);
        self.spec(dev).purpose.unwrap_or(match t >> 8 {
            0x10 => 1,
            0x31 => 2,
            0x33 => 3,
            0x41 => 5,
            0x42 => 6,
            0x43 => 7,
            0x44 => 8,
            _ => 0,
        })
    }
    fn developer_lists_valid(&self, dev: DeveloperId) -> bool {
        self.spec(dev).valid
    }
    fn developer_growth_list(&self, dev: DeveloperId) -> Vec<GrowthItem> {
        self.spec(dev).growth
    }
    fn developer_decline_list(&self, dev: DeveloperId) -> Vec<GrowthItem> {
        self.spec(dev).decline
    }
    /// The fake source node of a developer is named by the developer's own
    /// token; it exists when the spec's `ad` is set.
    fn developer_source_demand(&self, dev: DeveloperId) -> Option<u64> {
        self.spec(dev).ad.map(|_| dev)
    }
    fn developer_start_map_update(&self, dev: DeveloperId) {
        let n = self.spec(dev).finish_after;
        if let Some(d) = self.devs.borrow_mut().get_mut(&dev) {
            d.busy = n;
        }
        self.log("start_map_update", vec![("dev", ji(dev as i64))]);
    }
    fn developer_map_update_finished(&self, dev: DeveloperId) -> bool {
        let done = {
            let mut devs = self.devs.borrow_mut();
            match devs.get_mut(&dev) {
                Some(d) if d.busy > 0 => {
                    d.busy -= 1;
                    false
                }
                _ => true,
            }
        };
        self.log(
            "map_update_finished",
            vec![("dev", ji(dev as i64)), ("result", Json::Bool(done))],
        );
        done
    }
    fn developer_ref(&self, dev: DeveloperId) -> RefValue {
        RefValue::Host {
            token: dev,
            class_id: DEVELOPER_CLASS,
        }
    }
    fn resolve_developer(&self, r: &RefValue) -> Option<DeveloperId> {
        match r {
            RefValue::Host { token, .. } => {
                let mut devs = self.devs.borrow_mut();
                let d = devs.get_mut(token)?;
                d.refs += 1;
                Some(*token)
            }
            _ => None,
        }
    }
}

// ---- save streams -----------------------------------------------------------------------------------

/// A byte stream whose references carry the objects' own host IDs
/// (foundations §2.7: `objectID, classID`): host objects their token, core
/// objects their `host_token()`. Every written object is remembered for
/// `bds_load`.
struct IdWriter<'a> {
    inner: ByteWriter,
    objects: &'a RefCell<BTreeMap<u32, RefValue>>,
}

impl OutStream for IdWriter<'_> {
    fn put_u8(&mut self, v: u8) -> bool {
        self.inner.put_u8(v)
    }
    fn put_u16(&mut self, v: u16) -> bool {
        self.inner.put_u16(v)
    }
    fn put_u32(&mut self, v: u32) -> bool {
        self.inner.put_u32(v)
    }
    fn put_u64(&mut self, v: u64) -> bool {
        self.inner.put_u64(v)
    }
    fn put_f32(&mut self, v: f32) -> bool {
        self.inner.put_f32(v)
    }
    fn put_f64(&mut self, v: f64) -> bool {
        self.inner.put_f64(v)
    }
    fn put_bytes(&mut self, v: &[u8]) -> bool {
        self.inner.put_bytes(v)
    }
    fn put_str(&mut self, v: &[u8]) -> bool {
        self.inner.put_str(v)
    }
    fn put_ref(&mut self, r: &RefValue) -> bool {
        let (id, class) = match r {
            RefValue::Null => return self.inner.put_u32(0),
            RefValue::Host { token, class_id } => (*token as u32, *class_id),
            RefValue::Object(o) => (o.host_token() as u32, o.class_id()),
        };
        self.objects.borrow_mut().insert(id, r.clone());
        self.inner.put_u32(id) && self.inner.put_u32(class)
    }
    fn error(&self) -> i32 {
        self.inner.error()
    }
    fn set_error(&mut self) {
        self.inner.set_error();
    }
}

struct IdResolver<'a> {
    objects: &'a RefCell<BTreeMap<u32, RefValue>>,
}

/// Objects written by `bds_save` resolve to themselves; any other ID
/// resolves to the host object with that token (which the world's
/// `resolve_*` calls may not know: unresolved).
impl RefResolver for IdResolver<'_> {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        let known = self.objects.borrow().get(&object_id).cloned();
        known.or(Some(RefValue::Host {
            token: u64::from(object_id),
            class_id,
        }))
    }
}

// ---- ops ----------------------------------------------------------------------------------------------

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "development_simulator", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("bds_")?;
    let w = world(host);
    let r = match rest {
        "process" => (|| {
            let p = match opt(op, "session") {
                Some(s) => traced(BdsProcess::from_session(u32_of(s, "session")?, &PROCESS_SEEDS)),
                None => new_process(opt(op, "seed").map_or(Ok(0), |s| u32_of(s, "seed"))?),
            };
            w.flush_draws();
            *w.process.borrow_mut() = Some(p);
            Ok(())
        })(),
        "world" => set_world(&w, op),
        "exemplars" => exemplars(&w, op),
        "lot_configs" => lot_configs(&w, op),
        "developers" => developers(&w, op),
        "tract" => tract(&w, op),
        "lots" => lots(&w, op),
        "occupants" => occupants(&w, op),
        "create" => (|| {
            let s = DevelopmentSim::new(
                w.rc() as Rc<dyn DevelopmentHost>,
                w.rc() as Rc<dyn GrowthDevelopers>,
                w.process(),
                w.token(),
            );
            if let Some(n) = opt(op, "grow_budget") {
                s.set_grow_attempt_budget(u32_of(n, "grow_budget")?);
            }
            s.add_ref();
            *w.sim.borrow_mut() = Some(s);
            Ok(())
        })(),
        "grow_budget" => (|| {
            let s = w.sim()?;
            s.set_grow_attempt_budget(u32_of(field(op, "value")?, "value")?);
            Ok(())
        })(),
        "init" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.init()))),
        "shutdown" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.shutdown()))),
        "release" => w.sim().and_then(|s| check_expect(op, ji(s.release()))),
        "add_ref" => w.sim().and_then(|s| check_expect(op, ji(s.add_ref()))),
        "message" => message(&w, op),
        "day" => (|| {
            let s = w.sim()?;
            let n = op.get("count").and_then(Json::as_i64).unwrap_or(1);
            for _ in 0..n {
                s.do_message(&Message::of_type(msg::SIM_NEW_DAY));
                w.day.set(w.day.get().wrapping_add(1));
            }
            Ok(())
        })(),
        "call" => call(&w, op),
        "set" => set(&w, op),
        "expect" => expect(&w, op),
        "expect_map" => expect_map(&w, op),
        "expect_refs" => expect_refs(&w, op),
        "expect_events" => {
            w.flush_draws();
            w.events.expect_events(op)
        }
        "expect_event_count" => {
            w.flush_draws();
            w.events.expect_count(op)
        }
        "clear_events" => {
            w.flush_draws();
            w.events.clear();
            Ok(())
        }
        "save" => save(&w, op),
        "expect_record" => expect_record(&w, op),
        "patch" => patch(&w, op),
        "load" => load(&w, op),
        _ => Err(format!("unknown op '{name}'")),
    };
    w.flush_draws();
    Some(r)
}

fn i32_of(j: &Json, what: &str) -> R<i32> {
    Ok(i64_of(j, what)? as i32)
}

fn pair_f(j: &Json, what: &str) -> R<(f32, f32)> {
    let a = arr(j, what)?;
    match a {
        [x, z] => Ok((f32_of(x, what)?, f32_of(z, what)?)),
        _ => Err(format!("'{what}' must be a pair")),
    }
}

fn vec3(j: &Json, what: &str) -> R<Vec3> {
    let a = arr(j, what)?;
    match a {
        [x, y, z] => Ok([f32_of(x, what)?, f32_of(y, what)?, f32_of(z, what)?]),
        _ => Err(format!("'{what}' must be [x, y, z]")),
    }
}

fn rect_of(j: &Json, what: &str) -> R<CellRect> {
    let a = arr(j, what)?;
    match a {
        [a, b, c, d] => Ok((i32_of(a, what)?, i32_of(b, what)?, i32_of(c, what)?, i32_of(d, what)?)),
        _ => Err(format!("'{what}' must be [x1, z1, x2, z2]")),
    }
}

fn items_of(j: &Json, what: &str) -> R<Vec<GrowthItem>> {
    arr(j, what)?
        .iter()
        .map(|it| {
            let a = arr(it, what)?;
            Ok(GrowthItem {
                x: i32_of(a.first().ok_or("item x")?, what)?,
                z: i32_of(a.get(1).ok_or("item z")?, what)?,
                max_stage: a.get(2).map_or(Ok(1), |s| i32_of(s, what))?,
            })
        })
        .collect()
}

/// `bds_world`: day, difficulty, cell_widths, cell_counts, altitude,
/// area_altitude, census {id: value}, tuning (property map, or null for
/// none, or "shipped"), funds, demolish_cost, occupant_manager, insert_ok.
fn set_world(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "day") {
        w.day.set(u32_of(v, "day")?);
    }
    if let Some(v) = opt(op, "next_token") {
        w.next_token.set(u64::from(u32_of(v, "next_token")?));
    }
    if let Some(v) = opt(op, "difficulty") {
        w.difficulty.set(u32_of(v, "difficulty")?);
    }
    if let Some(v) = opt(op, "cell_widths") {
        w.cell_widths.set(pair_f(v, "cell_widths")?);
    }
    if let Some(v) = opt(op, "cell_counts") {
        let (x, z) = pair_f(v, "cell_counts")?;
        w.cell_counts.set((x as i32, z as i32));
    }
    if let Some(v) = opt(op, "altitude") {
        w.altitude.set(f32_of(v, "altitude")?);
    }
    if let Some(v) = opt(op, "area_altitude") {
        w.area_altitude.set(f32_of(v, "area_altitude")?);
    }
    if let Some(v) = opt(op, "census") {
        let mut c = w.census.borrow_mut();
        c.clear();
        for (k, val) in v.as_object().ok_or("census must be an object")? {
            let id = Json::Str(k.clone()).as_i64().ok_or("census id")? as u32;
            c.insert(id, f32_of(val, "census")?);
        }
    }
    if let Some(v) = op.get("tuning") {
        *w.tuning.borrow_mut() = match v {
            Json::Null => None,
            Json::Str(s) if s == "shipped" => Some(Rc::new(shipped_tuning())),
            other => Some(Rc::new(holder_of(other)?)),
        };
    }
    if let Some(v) = opt(op, "funds") {
        w.funds.set(i64_of(v, "funds")?);
    }
    if let Some(v) = opt(op, "demolish_cost") {
        w.demolish_cost.set(i64_of(v, "demolish_cost")?);
    }
    if let Some(v) = opt(op, "occupant_manager") {
        w.occupant_manager.set(bool_of(v, "occupant_manager")?);
    }
    if let Some(v) = opt(op, "insert_ok") {
        w.insert_ok.set(bool_of(v, "insert_ok")?);
    }
    Ok(())
}

/// The shipped "simulators" exemplar values the BDS reads (§4.13).
fn shipped_tuning() -> PropertyHolder {
    let mut h = PropertyHolder::new();
    h.add(
        0x47E2_C380,
        Variant::array(mem::UINT32, vec![Elem::U32(8192), Elem::U32(20480)]),
    );
    h.add(
        0x47E2_C383,
        Variant::array(mem::FLOAT32, vec![Elem::F32(1.2), Elem::F32(1.1), Elem::F32(1.0)]),
    );
    h
}

/// `bds_exemplars {list: […], append}`: each entry is a building
/// `{building: inst, purpose, wealth, capacity, families, size, name,
/// localized, group}`, a developer `{developer: type}`, a foundation
/// `{foundation: inst}`, a wall `{wall: inst}`, or a raw
/// `{instance, group, props}`; extra `props` merge in.
fn exemplars(w: &World, op: &Json) -> R<()> {
    let append = opt(op, "append").map_or(Ok(false), |v| bool_of(v, "append"))?;
    let mut out = Vec::new();
    for e in arr(field(op, "list")?, "list")? {
        let mut h = match opt(e, "props") {
            Some(p) => holder_of(p)?,
            None => PropertyHolder::new(),
        };
        let u32v = |v: u32| Variant::Scalar(Elem::U32(v));
        let (group, instance) = if let Some(b) = opt(e, "building") {
            let inst = u32_of(b, "building")?;
            h.add(0x10, u32v(2));
            let purpose = opt(e, "purpose").map_or(Ok(0), |v| u32_of(v, "purpose"))?;
            let wealth = opt(e, "wealth").map_or(Ok(0), |v| u32_of(v, "wealth"))?;
            h.add(0x2781_2833, Variant::Scalar(Elem::U8(purpose as u8)));
            h.add(0x2781_2832, Variant::Scalar(Elem::U8(wealth as u8)));
            if let Some(c) = opt(e, "capacity") {
                let dt = fc4_core::development::developer_type(purpose, wealth);
                h.add(
                    0x2781_2834,
                    Variant::array(mem::UINT32, vec![Elem::U32(dt), Elem::U32(u32_of(c, "capacity")?)]),
                );
            }
            if let Some(f) = opt(e, "families") {
                let items = arr(f, "families")?
                    .iter()
                    .map(|x| u32_of(x, "families").map(Elem::U32))
                    .collect::<R<Vec<_>>>()?;
                h.add(0x2781_2870, Variant::array(mem::UINT32, items));
            }
            if let Some(s) = opt(e, "size") {
                let v = vec3(s, "size")?;
                h.add(
                    0x2781_2810,
                    Variant::array(mem::FLOAT32, v.iter().map(|&x| Elem::F32(x)).collect()),
                );
            }
            if let Some(n) = opt(e, "name") {
                h.add(0x20, Variant::string(str_of(n, "name")?.as_bytes()));
            }
            if let Some(n) = opt(e, "localized") {
                h.add(0x8A41_6A99, Variant::string(str_of(n, "localized")?.as_bytes()));
            }
            (0x1000_0000, inst)
        } else if let Some(d) = opt(e, "developer") {
            let t = u32_of(d, "developer")?;
            h.add(0x10, u32v(4));
            if t != 0 {
                h.add(0x21, u32v(t));
            }
            (0x67CD_5FA1, t)
        } else if let Some(f) = opt(e, "foundation") {
            h.add(0x10, u32v(0x11));
            (0x2000_0000, u32_of(f, "foundation")?)
        } else if let Some(f) = opt(e, "wall") {
            h.add(0x10, u32v(0x15));
            (0x3000_0000, u32_of(f, "wall")?)
        } else {
            (
                opt(e, "group").map_or(Ok(0), |v| u32_of(v, "group"))?,
                u32_of(field(e, "instance")?, "instance")?,
            )
        };
        let group = opt(e, "group").map_or(Ok(group), |v| u32_of(v, "group"))?;
        let key = ResKey {
            type_id: 0x6534_284A,
            group,
            instance,
        };
        out.push((key, Rc::new(h)));
    }
    let mut ex = w.exemplars.borrow_mut();
    if !append {
        ex.clear();
    }
    ex.extend(out);
    Ok(())
}

/// `bds_lot_configs {list: [{size: [w, h], zones: [z…], ids: [...],
/// orientation, pos: [x, z], no_object}]}`.
fn lot_configs(w: &World, op: &Json) -> R<()> {
    let mut out = Vec::new();
    for c in arr(field(op, "list")?, "list")? {
        let (sw, sh) = pair_f(field(c, "size")?, "size")?;
        let mut zones = 0u16;
        for z in arr(field(c, "zones")?, "zones")? {
            zones |= 1 << (u32_of(z, "zones")? & 15);
        }
        let no_object = opt(c, "no_object").map_or(Ok(false), |v| bool_of(v, "no_object"))?;
        let primary = if no_object {
            None
        } else {
            let (x, z) = match opt(c, "pos") {
                Some(p) => pair_f(p, "pos")?,
                None => (sw * 8.0, sh * 8.0),
            };
            Some(BuildingConfigObject {
                ids: arr(field(c, "ids")?, "ids")?
                    .iter()
                    .map(|v| u32_of(v, "ids"))
                    .collect::<R<Vec<_>>>()?,
                orientation: opt(c, "orientation").map_or(Ok(0), |v| u32_of(v, "orientation"))?,
                x,
                z,
            })
        };
        out.push(LotConfigInfo {
            size: (sw as u32, sh as u32),
            zones,
            primary,
        });
    }
    *w.lot_configs.borrow_mut() = out;
    Ok(())
}

/// `bds_developers {"0x1010": {ad, no_node, purpose, sub_type, valid,
/// growth, decline, finish_after}, …}`: merged into the scripted specs.
fn developers(w: &World, op: &Json) -> R<()> {
    let devs = field(op, "devs")?.as_object().ok_or("devs must be an object")?;
    let mut specs = w.specs.borrow_mut();
    for (k, v) in devs {
        let t = Json::Str(k.clone()).as_i64().ok_or("developer type")? as u32;
        let s = specs.entry(t).or_default();
        if let Some(a) = opt(v, "ad") {
            s.ad = Some(f32_of(a, "ad")?);
        }
        if opt(v, "no_node").is_some() {
            s.ad = None;
        }
        if let Some(p) = opt(v, "purpose") {
            s.purpose = Some(u32_of(p, "purpose")?);
        }
        if let Some(p) = opt(v, "sub_type") {
            s.sub_type = Some(u32_of(p, "sub_type")?);
        }
        if let Some(p) = opt(v, "valid") {
            s.valid = bool_of(p, "valid")?;
        }
        if let Some(p) = opt(v, "growth") {
            s.growth = items_of(p, "growth")?;
        }
        if let Some(p) = opt(v, "decline") {
            s.decline = items_of(p, "decline")?;
        }
        if let Some(p) = opt(v, "finish_after") {
            s.finish_after = u32_of(p, "finish_after")?;
        }
    }
    Ok(())
}

/// `bds_tract {grow: [built…], grow_default, decline: [result…],
/// decline_default}`: queued returns of the tract developer.
fn tract(w: &World, op: &Json) -> R<()> {
    let ints = |k: &str| -> R<Option<VecDeque<i32>>> {
        match opt(op, k) {
            None => Ok(None),
            Some(v) => Ok(Some(arr(v, k)?.iter().map(|x| i32_of(x, k)).collect::<R<_>>()?)),
        }
    };
    if let Some(q) = ints("grow")? {
        *w.grow_returns.borrow_mut() = q;
    }
    if let Some(q) = ints("decline")? {
        *w.decline_returns.borrow_mut() = q;
    }
    if let Some(v) = opt(op, "grow_default") {
        w.grow_default.set(i32_of(v, "grow_default")?);
    }
    if let Some(v) = opt(op, "decline_default") {
        w.decline_default.set(i32_of(v, "decline_default")?);
    }
    Ok(())
}

fn cell_info(j: &Json, base: CellInfo) -> R<CellInfo> {
    let mut c = base;
    if let Some(v) = opt(j, "zone") {
        c.zone = i64_of(v, "zone")? as i8;
    }
    if let Some(v) = opt(j, "powered") {
        c.powered = bool_of(v, "powered")?;
    }
    if let Some(v) = opt(j, "lv_type") {
        c.lv_type = u32_of(v, "lv_type")?;
    }
    if let Some(v) = opt(j, "lv") {
        c.lv = u32_of(v, "lv")? as u8;
    }
    if let Some(v) = j.get("lot") {
        c.lot = match v {
            Json::Null => None,
            other => Some(u32_of(other, "lot")? as u64),
        };
    }
    Ok(c)
}

/// `bds_lots {lots: [{id, building_type, rect, stage, building, state,
/// center, slope, dev_slope, demolishable}], cells: [{x, z, …}],
/// cell_default: {zone, powered, lv_type, lv, lot}}`.
fn lots(w: &World, op: &Json) -> R<()> {
    if let Some(list) = opt(op, "lots") {
        let mut lots = w.lots.borrow_mut();
        for l in arr(list, "lots")? {
            let id = u32_of(field(l, "id")?, "id")? as u64;
            let mut lot = lots.get(&id).cloned().unwrap_or_default();
            if let Some(v) = opt(l, "building_type") {
                lot.building_type = u32_of(v, "building_type")?;
            }
            if let Some(v) = opt(l, "rect") {
                lot.rect = rect_of(v, "rect")?;
            }
            if let Some(v) = l.get("stage") {
                lot.stage = match v {
                    Json::Null => None,
                    other => Some(u32_of(other, "stage")? as u8),
                };
            }
            if let Some(v) = l.get("building") {
                lot.building = match v {
                    Json::Null => None,
                    other => Some(u32_of(other, "building")? as u64),
                };
            }
            if let Some(v) = opt(l, "state") {
                lot.state = u32_of(v, "state")?;
            }
            if let Some(v) = opt(l, "center") {
                lot.center = vec3(v, "center")?;
            }
            if let Some(v) = opt(l, "slope") {
                lot.slope = f32_of(v, "slope")?;
            }
            if let Some(v) = opt(l, "dev_slope") {
                lot.dev_slope = f32_of(v, "dev_slope")?;
            }
            if let Some(v) = opt(l, "demolishable") {
                lot.demolishable = bool_of(v, "demolishable")?;
            }
            lots.insert(id, lot);
        }
    }
    if let Some(d) = opt(op, "cell_default") {
        w.cell_default.set(cell_info(d, w.cell_default.get())?);
    }
    if let Some(list) = opt(op, "cells") {
        for c in arr(list, "cells")? {
            let (x, z) = (i32_of(field(c, "x")?, "x")?, i32_of(field(c, "z")?, "z")?);
            let base = w.cell(x, z);
            let info = cell_info(c, base)?;
            w.cells.borrow_mut().insert((x, z), info);
        }
    }
    Ok(())
}

/// `bds_occupants {list: [{id, groups, preview, pos, bbox: [min, max],
/// cost, load_fails}], building_cells: [[x, z]…]}`. Occupant 0 is the
/// template for created building occupants.
fn occupants(w: &World, op: &Json) -> R<()> {
    if let Some(list) = opt(op, "list") {
        let mut occs = w.occupants.borrow_mut();
        for o in arr(list, "list")? {
            let id = u32_of(field(o, "id")?, "id")? as u64;
            let mut occ = occs.get(&id).cloned().unwrap_or_default();
            if let Some(v) = opt(o, "groups") {
                occ.groups = arr(v, "groups")?
                    .iter()
                    .map(|g| u32_of(g, "groups"))
                    .collect::<R<_>>()?;
            }
            if let Some(v) = opt(o, "preview") {
                occ.preview = bool_of(v, "preview")?;
            }
            if let Some(v) = opt(o, "pos") {
                occ.pos = vec3(v, "pos")?;
            }
            if let Some(v) = opt(o, "bbox") {
                let a = arr(v, "bbox")?;
                if a.len() != 2 {
                    return Err("bbox must be [min, max]".into());
                }
                occ.bbox = (vec3(&a[0], "bbox")?, vec3(&a[1], "bbox")?);
            }
            if let Some(v) = opt(o, "cost") {
                occ.cost = Some(i64_of(v, "cost")?);
            }
            if let Some(v) = opt(o, "load_fails") {
                occ.load_fails = bool_of(v, "load_fails")?;
            }
            occs.insert(id, occ);
        }
    }
    if let Some(list) = opt(op, "building_cells") {
        let mut cells = w.building_cells.borrow_mut();
        cells.clear();
        for c in arr(list, "building_cells")? {
            let a = arr(c, "building_cells")?;
            cells.insert((
                i32_of(a.first().ok_or("cell x")?, "x")?,
                i32_of(a.get(1).ok_or("cell z")?, "z")?,
            ));
        }
    }
    Ok(())
}

fn msg_type(j: &Json) -> R<u32> {
    if let Some(s) = j.as_str() {
        let named = match s {
            "SimBegin" => Some(msg::SIM_BEGIN),
            "SimNewDay" => Some(msg::SIM_NEW_DAY),
            "SimNewMonth" => Some(msg::SIM_NEW_MONTH),
            "SimIdle" => Some(msg::SIM_IDLE),
            "Cheat" => Some(msg::CHEAT),
            "PlaceBuilding" => Some(msg::PLACE_BUILDING),
            "TerrainChanged" => Some(msg::TERRAIN_CHANGED),
            "OccupantInserted" => Some(msg::OCCUPANT_INSERTED),
            "OccupantRemoved" => Some(msg::OCCUPANT_REMOVED),
            "StateChanged" => Some(msg::STATE_CHANGED),
            _ => None,
        };
        if let Some(t) = named {
            return Ok(t);
        }
    }
    u32_of(j, "type")
}

/// `bds_message {type, data1…data4, occupant, lot, params: [[u32, f32]…],
/// expect}`.
fn message(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let d = |k: &str| opt(op, k).map_or(Ok(0), |v| u32_of(v, k));
    let mut params = Vec::new();
    if let Some(p) = opt(op, "params") {
        for v in arr(p, "params")? {
            let a = arr(v, "params")?;
            params.push(CommandParam {
                as_u32: u32_of(a.first().ok_or("param")?, "params")?,
                as_f32: a.get(1).map_or(Ok(0.0), |x| f32_of(x, "params"))?,
            });
        }
    }
    let m = Message {
        msg_type: msg_type(field(op, "type")?)?,
        data1: d("data1")?,
        data2: d("data2")?,
        data3: d("data3")?,
        data4: d("data4")?,
        occupant: opt(op, "occupant")
            .map(|v| u32_of(v, "occupant"))
            .transpose()?
            .map(u64::from),
        lot: opt(op, "lot").map(|v| u32_of(v, "lot")).transpose()?.map(u64::from),
        params,
    };
    check_expect(op, Json::Bool(s.do_message(&m)))
}

fn u32_list(j: &Json, what: &str) -> R<Vec<u32>> {
    arr(j, what)?.iter().map(|v| u32_of(v, what)).collect()
}

fn list_json(v: &[u32]) -> Json {
    Json::Array(v.iter().map(|&x| ji(x)).collect())
}

fn key_json(k: Option<ResKey>) -> Json {
    match k {
        None => Json::Null,
        Some(k) => Json::Array(vec![ji(k.type_id), ji(k.group), ji(k.instance)]),
    }
}

fn construction_profile(j: &Json) -> R<ConstructionProfile> {
    let g = |k: &str| opt(j, k).map_or(Ok(0), |v| i64_of(v, k));
    Ok(ConstructionProfile {
        zone: g("zone")? as i32,
        unknown2: 0,
        wealth: g("wealth")? as u32,
        purpose: g("purpose")? as u32,
        land_value_min: g("lv_min")? as u8,
        land_value_max: g("lv_max")? as u8,
        building_type: g("type")? as u32,
        x: g("x")? as i32,
        z: g("z")? as i32,
    })
}

fn opt_occ(o: Option<OccupantId>) -> Json {
    o.map_or(Json::Null, |v| ji(v as i64))
}

/// `bds_call {method, args, expect}` (see README).
fn call(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let method = str_of(field(op, "method")?, "method")?;
    let a_u32 = |i: usize| -> R<u32> { u32_of(arg(op, i)?, "args") };
    let a_i32 = |i: usize| -> R<i32> { i32_of(arg(op, i)?, "args") };
    let vec_arg = |i: usize| -> R<Vec<u32>> {
        match op.get("args").and_then(Json::as_array).and_then(|a| a.get(i)) {
            None | Some(Json::Null) => Ok(Vec::new()),
            Some(v) => u32_list(v, "args"),
        }
    };
    let got = match method {
        "grow" => {
            s.grow();
            Json::Null
        }
        "plan_decline" => {
            s.plan_decline();
            Json::Null
        }
        "execute_decline_plan" => {
            s.execute_decline_plan();
            Json::Null
        }
        "do_idle" => {
            s.do_idle();
            Json::Null
        }
        "monthly" => {
            s.do_monthly_simulation();
            Json::Null
        }
        "daily" => {
            s.do_daily_simulation();
            Json::Null
        }
        "read_tuning" => {
            s.read_tuning();
            Json::Null
        }
        "total_resources" => ji(s.total_construction_resources()),
        "free_resources" => ji(s.free_construction_resources()),
        "claim" => Json::Bool(s.claim_construction_resources(a_u32(0)? as u16)),
        "release" => Json::Bool(s.release_construction_resources(a_u32(0)? as u16)),
        "is_building_type_valid" => Json::Bool(s.is_building_type_valid(a_u32(0)?)),
        "is_building_family_type_valid" => Json::Bool(s.is_building_family_type_valid(a_u32(0)?)),
        "is_building_in_family" => Json::Bool(s.is_building_in_family(a_u32(0)?, a_u32(1)?)),
        "types_by_family" => {
            let mut v = vec_arg(1)?;
            let n = s.building_types_by_family(&mut v, a_u32(0)?);
            Json::Object(vec![("count".into(), ji(n)), ("vector".into(), list_json(&v))])
        }
        "types_by_family_set" => {
            let mut set: BTreeSet<u32> = vec_arg(1)?.into_iter().collect();
            let n = s.building_types_by_family_set(&mut set, a_u32(0)?);
            let v: Vec<u32> = set.into_iter().collect();
            Json::Object(vec![("count".into(), ji(n)), ("set".into(), list_json(&v))])
        }
        "types_by_filter" => {
            let allowed: Option<Vec<u32>> = match arg(op, 0)? {
                Json::Null => None,
                v => Some(u32_list(v, "args")?),
            };
            let mut set: BTreeSet<u32> = vec_arg(1)?.into_iter().collect();
            let f = allowed.map(|a| move |t: u32| a.contains(&t));
            let n = match &f {
                Some(f) => s.building_types_by_filter(&mut set, Some(f)),
                None => s.building_types_by_filter(&mut set, None),
            };
            let v: Vec<u32> = set.into_iter().collect();
            Json::Object(vec![("count".into(), ji(n)), ("set".into(), list_json(&v))])
        }
        "profile" | "profile_ptr" => {
            let p = if method == "profile" {
                Some(s.building_profile(a_u32(0)?))
            } else {
                s.building_profile_ptr(a_u32(0)?)
            };
            match p {
                None => Json::Null,
                Some(p) => Json::Object(vec![
                    ("purpose".into(), ji(p.purpose)),
                    ("wealth".into(), ji(p.wealth)),
                    ("capacity".into(), ji(p.capacity)),
                    ("size".into(), vec3_json(p.occupant_size)),
                ]),
            }
        }
        "building_key" => key_json(s.building_key_from_type(a_u32(0)?)),
        "foundation_key" => key_json(s.foundation_key_from_type(a_u32(0)?)),
        "wall_key" => key_json(s.retaining_wall_key_from_type(a_u32(0)?)),
        "exemplar_name" => Json::Str(String::from_utf8_lossy(&s.exemplar_name(a_u32(0)?)).into_owned()),
        "building_name" => Json::Str(String::from_utf8_lossy(&s.building_name(a_u32(0)?)).into_owned()),
        "all_building_types" | "all_family_types" | "all_foundation_types" | "all_wall_types" => {
            let mut v = vec_arg(0)?;
            let n = match method {
                "all_building_types" => s.all_building_types(&mut v),
                "all_family_types" => s.all_building_family_types(&mut v),
                "all_foundation_types" => s.all_foundation_types(&mut v),
                _ => s.all_lot_retaining_wall_types(&mut v),
            };
            Json::Object(vec![("count".into(), ji(n)), ("vector".into(), list_json(&v))])
        }
        "all_building_types_array" => {
            let mut n = a_u32(0)?;
            let len = match arg(op, 1)? {
                Json::Null => None,
                v => Some(u32_of(v, "args")? as usize),
            };
            let mut buf = vec![0u32; len.unwrap_or(0)];
            let ret = s.all_building_types_array(len.map(|_| buf.as_mut_slice()), &mut n);
            Json::Object(vec![
                ("ret".into(), ji(ret)),
                ("n".into(), ji(n)),
                ("array".into(), list_json(&buf)),
            ])
        }
        "developer_type" => ji(s.developer_type(a_u32(0)?, a_u32(1)?)),
        "wealth" => ji(s.wealth(a_u32(0)?)),
        "developer" => opt_occ(s.developer(a_u32(0)?)),
        "developer_by" => opt_occ(s.developer_by(a_u32(0)?, a_u32(1)?)),
        "developers" => Json::Object(
            s.developers()
                .iter()
                .map(|(t, d)| (format!("0x{t:X}"), ji(*d as i64)))
                .collect(),
        ),
        "map_key" => ji(s.building_map_key(a_u32(0)?, a_u32(1)?)),
        "map_key_of_type" => ji(s.building_map_key_of_type(a_u32(0)?)),
        "max_capacity" => jf(s.max_capacity(a_u32(0)?, a_u32(1)?, a_u32(2)?)),
        "density" => Json::Array(
            s.process()
                .catalogue()
                .density_table()
                .iter()
                .map(|(&(z, k), &v)| Json::Array(vec![ji(z), ji(k), jf(v)]))
                .collect(),
        ),
        "day_of_last_build" => ji(s.day_of_last_successful_build(a_u32(0)?)),
        "stage_multiplier" => jf(s.stage_population_multiplier()),
        "cheats" => Json::Object(vec![
            ("plumbing".into(), Json::Bool(s.cheat_ignore_plumbing())),
            ("power".into(), Json::Bool(s.cheat_ignore_power())),
            ("caps".into(), Json::Bool(s.cheat_ignore_caps())),
            ("debug_growth".into(), Json::Bool(s.cheat_debug_growth())),
            ("coffee_bar".into(), Json::Bool(s.cheat_coffee_bar())),
        ]),
        "suitable" => {
            let p = construction_profile(arg(op, 0)?)?;
            let mut reason = -1;
            let ok = s.is_suitable_for_construction(&p, &mut reason);
            Json::Object(vec![("result".into(), Json::Bool(ok)), ("reason".into(), ji(reason))])
        }
        "location_ok" => Json::Bool(s.location_is_ok_for_new_building(
            a_u32(0)?,
            f32_of(arg(op, 1)?, "args")?,
            f32_of(arg(op, 2)?, "args")?,
        )),
        "add_occupant" => opt_occ(s.add_new_building_occupant(a_u32(0)?, vec3(arg(op, 1)?, "args")?, a_i32(2)?)),
        "add_occupant_at_cell" => {
            opt_occ(s.add_new_building_occupant_at_cell(a_u32(0)?, a_i32(1)?, a_i32(2)?, a_i32(3)?))
        }
        "plop" => opt_occ(s.plop_building(&construction_profile(arg(op, 0)?)?)),
        "insert_occupant" => Json::Bool(s.insert_building_occupant(u64::from(a_u32(0)?))),
        "demolish_foundation" => Json::Bool(s.demolish_foundation_occupant(u64::from(a_u32(0)?))),
        "demolition_cost" => ji(s.cost_of_building_demolition(u64::from(a_u32(0)?))),
        "foundation_cost" => ji(s.cost_of_foundation_demolition(u64::from(a_u32(0)?))),
        "building_changed" => {
            s.do_building_changed(u64::from(a_u32(0)?), a_u32(1)?, bool_of(arg(op, 2)?, "args")?);
            Json::Null
        }
        "terrain_changed" => {
            s.do_terrain_changed(rect_of(arg(op, 0)?, "args")?, a_u32(1)?);
            Json::Null
        }
        "unpack_terrain_rect" => {
            let r = fc4_core::development_simulator::unpack_terrain_rect(a_u32(0)?, a_u32(1)?, (a_i32(2)?, a_i32(3)?));
            rect_json(r)
        }
        "query_interface" => match query_interface(a_u32(0)?) {
            None => Json::Null,
            Some(BdsInterface::Primary) => Json::Str("primary".into()),
            Some(BdsInterface::MessageTarget) => Json::Str("message_target".into()),
            Some(BdsInterface::MessageTarget2) => Json::Str("message_target2".into()),
            Some(BdsInterface::Serializable) => Json::Str("serializable".into()),
        },
        "class_id" => ji(s.class_id()),
        "special_buildings" => Json::Array(s.special_buildings().iter().map(|&o| ji(o as i64)).collect()),
        "app_shutdown" => {
            s.process().app_shutdown();
            Json::Null
        }
        other => return Err(format!("unknown bds method '{other}'")),
    };
    check_expect(op, got)
}

fn plan_json(p: &[PlanEntry]) -> Json {
    Json::Array(
        p.iter()
            .map(|e| Json::Array(vec![ji(e.day), ji(e.x), ji(e.z), ji(e.developer as i64), ji(e.stage)]))
            .collect(),
    )
}

fn type_map<V: Copy>(m: &BTreeMap<u32, V>, f: impl Fn(V) -> Json) -> Json {
    Json::Object(m.iter().map(|(t, v)| (format!("0x{t:X}"), f(*v))).collect())
}

/// `bds_expect {state: {…}}`: a partial state check.
fn expect(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let d = s.snapshot();
    let p = s.process();
    let node = s.construction_node();
    let marks = Json::Object(
        p.marks()
            .iter()
            .map(|(t, cells)| {
                (
                    format!("0x{t:X}"),
                    Json::Array(cells.iter().map(|&(x, z)| Json::Array(vec![ji(x), ji(z)])).collect()),
                )
            })
            .collect(),
    );
    let got = vec![
        ("initialized".to_owned(), Json::Bool(d.initialized)),
        ("loaded_from_save".to_owned(), Json::Bool(d.loaded_from_save)),
        ("destroyed".to_owned(), Json::Bool(d.destroyed)),
        ("ref_count".to_owned(), ji(d.ref_count)),
        ("state".to_owned(), ji(d.state)),
        ("cursor".to_owned(), d.cursor.map_or(Json::Null, ji)),
        ("developers".to_owned(), type_map(&d.developers, |v| ji(v as i64))),
        ("highest_stage".to_owned(), type_map(&d.highest_stage, ji)),
        ("last_build".to_owned(), type_map(&d.last_successful_build, ji)),
        (
            "special_buildings".to_owned(),
            Json::Array(d.special_buildings.iter().map(|&o| ji(o as i64)).collect()),
        ),
        ("decline_plan".to_owned(), plan_json(&d.decline_plan)),
        ("planned_growth".to_owned(), ji(d.planned_growth.len() as i64)),
        ("grow_budget".to_owned(), ji(s.grow_attempt_budget())),
        ("cr_min".to_owned(), ji(d.tuning.cr_min)),
        ("cr_max".to_owned(), ji(d.tuning.cr_max)),
        ("cr_per_resident".to_owned(), jf(d.tuning.cr_per_resident)),
        ("bias".to_owned(), ji(d.tuning.bias)),
        (
            "stage_multiplier".to_owned(),
            Json::Array(d.tuning.stage_multiplier.iter().map(|&v| jf(v)).collect()),
        ),
        ("ignore_plumbing".to_owned(), Json::Bool(d.cheat_ignore_plumbing)),
        ("ignore_power".to_owned(), Json::Bool(d.cheat_ignore_power)),
        ("ignore_caps".to_owned(), Json::Bool(d.cheat_ignore_caps)),
        ("coffee_bar".to_owned(), Json::Bool(d.cheat_coffee_bar)),
        (
            "cell_counts".to_owned(),
            Json::Array(vec![ji(d.cell_counts.0), ji(d.cell_counts.1)]),
        ),
        ("has_map".to_owned(), Json::Bool(s.building_map().is_some())),
        (
            "map_id".to_owned(),
            s.building_map().map_or(Json::Null, |g| match g.save_ref() {
                RefValue::Object(o) => ji(o.host_token() as i64),
                _ => Json::Null,
            }),
        ),
        (
            "node_id".to_owned(),
            node.as_ref().map_or(Json::Null, |n| ji(n.token() as i64)),
        ),
        (
            "supply".to_owned(),
            node.as_ref().map_or(Json::Null, |n| jf(n.supply())),
        ),
        (
            "demand".to_owned(),
            node.as_ref().map_or(Json::Null, |n| jf(n.demand())),
        ),
        ("marks".to_owned(), marks),
        ("steps".to_owned(), ji(p.steps() as i64)),
        ("rng_state".to_owned(), ji(p.rng_state())),
        ("catalogue_loaded".to_owned(), Json::Bool(p.catalogue().is_loaded())),
        ("building_count".to_owned(), ji(p.catalogue().building_count() as i64)),
        ("funds".to_owned(), ji(w.funds.get())),
        ("day".to_owned(), ji(w.day.get())),
    ];
    check_state(&got, field(op, "state")?)
}

/// `bds_expect_map {cells: [[x, z, value]…]}` on the building map.
fn expect_map(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let g = s.building_map().ok_or("no building map")?;
    let mut errors = Vec::new();
    for c in arr(field(op, "cells")?, "cells")? {
        let a = arr(c, "cells")?;
        let (x, z, v) = match a {
            [x, z, v] => (i32_of(x, "x")?, i32_of(z, "z")?, u32_of(v, "v")?),
            _ => return Err("cells entries are [x, z, value]".into()),
        };
        let got = g.get(x, z);
        if u32::from(got) != v {
            errors.push(format!("({x}, {z}): got {got}, expected {v}"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

/// `bds_expect_refs {developers: {token: refs}, occupants: {id: refs}}`.
fn expect_refs(w: &World, op: &Json) -> R<()> {
    let mut errors = Vec::new();
    if let Some(m) = opt(op, "developers") {
        for (k, v) in m.as_object().ok_or("developers must be an object")? {
            let t = Json::Str(k.clone()).as_i64().ok_or("token")? as u64;
            let got = w.devs.borrow().get(&t).map_or(0, |d| d.refs);
            if got != i64_of(v, "refs")? {
                errors.push(format!("developer {k}: {got} references"));
            }
        }
    }
    if let Some(m) = opt(op, "occupants") {
        for (k, v) in m.as_object().ok_or("occupants must be an object")? {
            let t = Json::Str(k.clone()).as_i64().ok_or("id")? as u64;
            let got = w.occupants.borrow().get(&t).map_or(0, |o| o.refs);
            if got != i64_of(v, "refs")? {
                errors.push(format!("occupant {k}: {got} references"));
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

/// `bds_set {state: {…}}`: state, decline_plan [[day, x, z, dev, stage]…],
/// supply, demand, ignore_plumbing/power/caps, coffee_bar, bias,
/// loaded_from_save, special_buildings.
fn set(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let st = field(op, "state")?.as_object().ok_or("state must be an object")?;
    for (k, v) in st {
        match k.as_str() {
            "supply" | "demand" => {
                let n = s.construction_node().ok_or("no construction node")?;
                if k == "supply" {
                    n.set_supply(f32_of(v, k)?);
                } else {
                    n.set_demand(f32_of(v, k)?);
                }
            }
            "map_id" => {
                let map = s.building_map().ok_or("no building map")?;
                let ours = w
                    .grids
                    .borrow()
                    .iter()
                    .find(|g| {
                        let mine: Rc<dyn TractGrid<u8>> = (*g).clone();
                        Rc::ptr_eq(&mine, &map)
                    })
                    .cloned();
                ours.ok_or("the building map is not the world's")?
                    .set_host_token(u64::from(u32_of(v, k)?));
            }
            "node_id" => {
                let n = s.construction_node().ok_or("no construction node")?;
                n.set_token(u64::from(u32_of(v, k)?));
            }
            "decline_plan" => {
                let mut plan = Vec::new();
                for e in arr(v, k)? {
                    let a = arr(e, k)?;
                    let g = |i: usize| -> R<i64> { i64_of(a.get(i).ok_or("plan entry")?, k) };
                    plan.push(PlanEntry {
                        day: g(0)? as u32,
                        x: g(1)? as i32,
                        z: g(2)? as i32,
                        developer: g(3)? as u64,
                        stage: g(4).unwrap_or(0) as u8,
                    });
                }
                s.data_mut().decline_plan = plan;
            }
            _ => {
                let mut d = s.data_mut();
                match k.as_str() {
                    "state" => d.state = u32_of(v, k)?,
                    "ignore_plumbing" => d.cheat_ignore_plumbing = bool_of(v, k)?,
                    "ignore_power" => d.cheat_ignore_power = bool_of(v, k)?,
                    "ignore_caps" => d.cheat_ignore_caps = bool_of(v, k)?,
                    "coffee_bar" => d.cheat_coffee_bar = bool_of(v, k)?,
                    "bias" => d.tuning.bias = i64_of(v, k)? as i32,
                    "loaded_from_save" => d.loaded_from_save = bool_of(v, k)?,
                    "special_buildings" => {
                        d.special_buildings = u32_list(v, k)?.into_iter().map(u64::from).collect();
                    }
                    "highest_stage" => {
                        for (t, sv) in v.as_object().ok_or("highest_stage must be an object")? {
                            let t = Json::Str(t.clone()).as_i64().ok_or("type")? as u32;
                            d.highest_stage.insert(t, u32_of(sv, k)? as u8);
                        }
                    }
                    other => return Err(format!("unknown settable field '{other}'")),
                }
            }
        }
    }
    Ok(())
}

/// `bds_save {slot}`: the BDS record (§6) with references written as the
/// objects' host IDs.
fn save(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let slot = str_of(field(op, "slot")?, "slot")?.to_owned();
    let mut out = IdWriter {
        inner: ByteWriter::new(),
        objects: &w.saved_objects,
    };
    let ok = s.write(&mut out);
    check_expect(op, Json::Bool(ok))?;
    w.saves.borrow_mut().insert(slot, out.inner.into_bytes());
    Ok(())
}

/// `bds_expect_record {slot, hex}` (whole record) or `{slot, length, at:
/// [{offset, hex}]}`.
fn expect_record(w: &World, op: &Json) -> R<()> {
    let slot = str_of(field(op, "slot")?, "slot")?;
    let bytes = w.saves.borrow().get(slot).cloned().ok_or("no such slot")?;
    if let Some(h) = opt(op, "hex") {
        let want = hex_decode(str_of(h, "hex")?)?;
        if bytes != want {
            return Err(format!(
                "record differs:\n  got      {}\n  expected {}",
                hex_encode(&bytes),
                hex_encode(&want)
            ));
        }
    }
    if let Some(n) = opt(op, "length") {
        if bytes.len() as i64 != i64_of(n, "length")? {
            return Err(format!("record length {}", bytes.len()));
        }
    }
    if let Some(at) = opt(op, "at") {
        for e in arr(at, "at")? {
            let off = i64_of(field(e, "offset")?, "offset")? as usize;
            let want = hex_decode(str_of(field(e, "hex")?, "hex")?)?;
            let got = bytes.get(off..off + want.len()).ok_or("offset beyond the record")?;
            if got != want.as_slice() {
                return Err(format!(
                    "at {off}: got {}, expected {}",
                    hex_encode(got),
                    hex_encode(&want)
                ));
            }
        }
    }
    Ok(())
}

/// `bds_patch {slot, offset, hex}` (overwrite) or `{slot, truncate}`.
fn patch(w: &World, op: &Json) -> R<()> {
    let slot = str_of(field(op, "slot")?, "slot")?;
    let mut saves = w.saves.borrow_mut();
    let bytes = saves.get_mut(slot).ok_or("no such slot")?;
    if let Some(n) = opt(op, "truncate") {
        bytes.truncate(i64_of(n, "truncate")? as usize);
        return Ok(());
    }
    let off = i64_of(field(op, "offset")?, "offset")? as usize;
    let new = hex_decode(str_of(field(op, "hex")?, "hex")?)?;
    let dst = bytes.get_mut(off..off + new.len()).ok_or("patch beyond the record")?;
    dst.copy_from_slice(&new);
    Ok(())
}

/// `bds_load {slot | hex, expect, consumed}`: `Read` into the current BDS.
fn load(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let bytes = match opt(op, "hex") {
        Some(h) => hex_decode(str_of(h, "hex")?)?,
        None => {
            let slot = str_of(field(op, "slot")?, "slot")?;
            w.saves.borrow().get(slot).cloned().ok_or("no such slot")?
        }
    };
    let mut resolver = IdResolver {
        objects: &w.saved_objects,
    };
    let mut r = ByteReader::with_resolver(&bytes, &mut resolver);
    let ok = s.read(&mut r);
    check_expect(op, Json::Bool(ok))?;
    if let Some(c) = opt(op, "consumed") {
        let consumed = bytes.len() - r.remaining();
        if consumed as i64 != i64_of(c, "consumed")? {
            return Err(format!("consumed {consumed} bytes"));
        }
    }
    Ok(())
}
