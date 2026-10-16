// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops `gdev_…` for `fc4_core::growth_developer`
//! (`docs/specs/growth-developer.md`): a scripted world implementing
//! `GrowthDeveloperHost` (services, the message server and simulator, the
//! date, source maps, demand nodes and neighbours, the BDS's answers, the
//! tract developer, zone types, lots and special-building occupants, the
//! time-slice clock) and ops that drive one or more growth developers sharing
//! one `GrowthStatics`. See README.md ("Growth developer ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::rc::{Rc, Weak};

use fc4_core::development::{ConstructionProfile, GrowthItem, GrowthProfile};
use fc4_core::grid::{create_for_clsid, CityDims, GridElem, GridProvider, ScanState, SimGrid, TractGrid};
use fc4_core::growth_developer::{
    self as gd, msg, query_interface, shipped_exemplar, Curve, DevGrid, GridHandle, GrowthDeveloper,
    GrowthDeveloperHost, GrowthMessage, GrowthStatics, Service, SourceMap, CLSID_GROWTH_DEVELOPER, SLOTS,
};
use fc4_core::property::PropertyHolder;
use fc4_core::rng::FixedSeeds;
use fc4_core::stl::stl_sort;
use fc4_core::store::{load_store, save_store, LoadRegistry, Package};
use fc4_core::stream::{ByteReader, ByteWriter, OutStream, RefValue, SaveObject, SaveRegistry};
use fc4_core::variant::Variant;

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, f32_of, i64_of, jf, ji, opt, str_of, u32_of, variant_of, EventLog, R,
};

/// One source map of the world.
#[derive(Clone)]
enum Src {
    U8(Rc<SimGrid<u8>>),
    I8(Rc<SimGrid<i8>>),
    I16(Rc<SimGrid<i16>>),
    F32(Rc<SimGrid<f32>>),
}

#[derive(Clone, Default)]
struct Occ {
    props: PropertyHolder,
    pos: [f32; 3],
}

#[derive(Clone, Copy, Default)]
struct Lot {
    building_type: u32,
    capacity: u16,
    size: (u8, u8),
    rect: [i32; 4],
    location: (i32, i32),
}

#[derive(Clone)]
struct SavedCity {
    pkg: Package,
    root: Vec<u8>,
}

pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    devs: RefCell<BTreeMap<String, Rc<GrowthDeveloper>>>,
    current: RefCell<String>,
    statics: RefCell<Rc<GrowthStatics>>,
    next_token: Cell<u64>,
    labels: RefCell<BTreeMap<String, u64>>,
    names: RefCell<BTreeMap<u64, String>>,
    refs: RefCell<BTreeMap<u64, i64>>,
    cells: Cell<(i32, i32)>,
    date: Cell<u32>,
    wall_seed: Cell<u32>,
    missing_services: RefCell<Vec<Service>>,
    no_message_server: Cell<bool>,
    no_simulator: Cell<bool>,
    no_grids: Cell<bool>,
    log_population: Cell<bool>,
    log_zone: Cell<bool>,
    log_supply: Cell<bool>,
    tract_developer: Cell<Option<u64>>,
    multiplier: Cell<f32>,
    connected: Cell<(u32, u32)>,
    /// (id, index) → node token.
    demand: RefCell<BTreeMap<(u32, u32), u64>>,
    supply: RefCell<BTreeMap<u64, f32>>,
    active: RefCell<BTreeMap<u64, f32>>,
    /// Running `AddToDemandValue` totals per node.
    demand_added: RefCell<BTreeMap<u64, f64>>,
    specials: RefCell<Vec<u64>>,
    occs: RefCell<BTreeMap<u64, Occ>>,
    lots: RefCell<BTreeMap<u64, Lot>>,
    max_capacity: RefCell<BTreeMap<i32, f32>>,
    purposes: RefCell<BTreeMap<u32, u32>>,
    zones: RefCell<BTreeMap<(i32, i32), i8>>,
    sources: RefCell<HashMap<SourceMap, Src>>,
    /// `limit_timer_expired` answers false this many times after each
    /// `limit_timer_start` (None: never expires).
    timer_checks: Cell<Option<u32>>,
    timer_left: Cell<u32>,
    grids_u8: RefCell<Vec<Rc<SimGrid<u8>>>>,
    grids_i16: RefCell<Vec<Rc<SimGrid<i16>>>>,
    grids_u16: RefCell<Vec<Rc<SimGrid<u16>>>>,
    saves: RefCell<BTreeMap<String, SavedCity>>,
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            me: RefCell::new(Weak::new()),
            events: EventLog::default(),
            devs: RefCell::new(BTreeMap::new()),
            current: RefCell::new("dev".into()),
            statics: RefCell::new(GrowthStatics::new(0, &seeds(1))),
            next_token: Cell::new(0x100),
            labels: RefCell::new(BTreeMap::new()),
            names: RefCell::new(BTreeMap::new()),
            refs: RefCell::new(BTreeMap::new()),
            cells: Cell::new((64, 64)),
            date: Cell::new(0),
            wall_seed: Cell::new(1),
            missing_services: RefCell::new(Vec::new()),
            no_message_server: Cell::new(false),
            no_simulator: Cell::new(false),
            no_grids: Cell::new(false),
            log_population: Cell::new(false),
            log_zone: Cell::new(false),
            log_supply: Cell::new(true),
            tract_developer: Cell::new(None),
            multiplier: Cell::new(1.0),
            connected: Cell::new((0, 0)),
            demand: RefCell::new(BTreeMap::new()),
            supply: RefCell::new(BTreeMap::new()),
            active: RefCell::new(BTreeMap::new()),
            demand_added: RefCell::new(BTreeMap::new()),
            specials: RefCell::new(Vec::new()),
            occs: RefCell::new(BTreeMap::new()),
            lots: RefCell::new(BTreeMap::new()),
            max_capacity: RefCell::new(BTreeMap::new()),
            purposes: RefCell::new(BTreeMap::new()),
            zones: RefCell::new(BTreeMap::new()),
            sources: RefCell::new(HashMap::new()),
            timer_checks: Cell::new(None),
            timer_left: Cell::new(0),
            grids_u8: RefCell::new(Vec::new()),
            grids_i16: RefCell::new(Vec::new()),
            grids_u16: RefCell::new(Vec::new()),
            saves: RefCell::new(BTreeMap::new()),
        });
        let td = w.token("tract_developer");
        w.tract_developer.set(Some(td));
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn rc(&self) -> Rc<World> {
        self.me.borrow().upgrade().expect("world alive")
    }

    fn token(&self, label: &str) -> u64 {
        let t = self.next_token.get();
        self.next_token.set(t + 1);
        self.names.borrow_mut().insert(t, label.to_owned());
        self.labels.borrow_mut().insert(label.to_owned(), t);
        t
    }

    /// The token of `label`, created on first use.
    fn token_of(&self, label: &str) -> u64 {
        let found = self.labels.borrow().get(label).copied();
        found.unwrap_or_else(|| self.token(label))
    }

    fn find(&self, label: &str) -> R<u64> {
        self.labels
            .borrow()
            .get(label)
            .copied()
            .ok_or_else(|| format!("unknown object '{label}'"))
    }

    fn name(&self, t: u64) -> Json {
        Json::Str(self.names.borrow().get(&t).cloned().unwrap_or_else(|| format!("#{t}")))
    }

    fn opt_name(&self, t: Option<u64>) -> Json {
        t.map_or(Json::Null, |t| self.name(t))
    }

    fn dims(&self) -> CityDims {
        let (x, z) = self.cells.get();
        CityDims {
            cells_x: x,
            cells_z: z,
            ..CityDims::square(1)
        }
    }

    /// The developer named by `dev` (default: the current one).
    fn dev(&self, op: &Json) -> R<Rc<GrowthDeveloper>> {
        let label = match opt(op, "dev") {
            Some(v) => str_of(v, "dev")?.to_owned(),
            None => self.current.borrow().clone(),
        };
        self.devs
            .borrow()
            .get(&label)
            .cloned()
            .ok_or_else(|| format!("no developer '{label}' (gdev_create)"))
    }

    fn statics(&self) -> Rc<GrowthStatics> {
        self.statics.borrow().clone()
    }

    fn handle_name(&self, h: GridHandle<'_>) -> Json {
        fn find<T: GridElem>(list: &[Rc<SimGrid<T>>], g: &Rc<dyn TractGrid<T>>) -> Json {
            let p = Rc::as_ptr(g) as *const ();
            list.iter()
                .find(|s| Rc::as_ptr(s) as *const () == p)
                .map_or(Json::Str("?".into()), |s| hex(s.instance_id()))
        }
        match h {
            GridHandle::U8(g) => find(&self.grids_u8.borrow(), g),
            GridHandle::I16(g) => find(&self.grids_i16.borrow(), g),
            GridHandle::U16(g) => find(&self.grids_u16.borrow(), g),
        }
    }

    fn resize<T: GridElem>(&self, list: &[Rc<SimGrid<T>>], g: &Rc<dyn TractGrid<T>>, s: i32) {
        let p = Rc::as_ptr(g) as *const ();
        if let Some(sg) = list.iter().find(|x| Rc::as_ptr(x) as *const () == p) {
            sg.set_tract_size(&self.dims(), s);
        }
    }
}

fn seeds(wall: u32) -> FixedSeeds {
    FixedSeeds {
        wall_clock: wall,
        process_clock: 0,
    }
}

fn hex(v: u32) -> Json {
    Json::Str(format!("0x{v:08X}"))
}

fn types_json(t: &[u32]) -> Json {
    Json::Array(t.iter().map(|&v| hex(v)).collect())
}

fn service_name(s: Service) -> &'static str {
    match s {
        Service::City => "city",
        Service::Bds => "bds",
        Service::OccupantManager => "occupant_manager",
        Service::ZoneManager => "zone_manager",
        Service::LandValue => "land_value",
        Service::Traffic => "traffic",
    }
}

fn service_of(name: &str) -> R<Service> {
    Service::ACQUIRE
        .into_iter()
        .find(|s| service_name(*s) == name)
        .ok_or_else(|| format!("unknown service '{name}'"))
}

fn dev_grid_name(g: DevGrid) -> &'static str {
    match g {
        DevGrid::ZonedCount => "zoned",
        DevGrid::Working => "working",
        DevGrid::Desirability => "desirability",
        DevGrid::Population => "population",
        DevGrid::Capacity => "capacity",
        DevGrid::MaxCapacity => "max_capacity",
    }
}

fn curve_name(c: Curve) -> &'static str {
    match c {
        Curve::LandValue => "land_value",
        Curve::Slope => "slope",
        Curve::ProximityR1 => "r1",
        Curve::ProximityR2 => "r2",
        Curve::ProximityR3 => "r3",
        Curve::ProximityCO2 => "co2",
        Curve::ProximityCO3 => "co3",
        Curve::Park => "park",
        Curve::Air => "air",
        Curve::Water => "water",
        Curve::Garbage => "garbage",
        Curve::Radiation => "radiation",
        Curve::School => "school",
        Curve::Hospital => "hospital",
        Curve::Crime => "crime",
        Curve::Commute => "commute",
        Curve::Traffic => "traffic",
        Curve::Landmark => "landmark",
        Curve::Transient => "transient",
    }
}

/// A source map from `map` and `index?`.
fn source_of(op: &Json) -> R<SourceMap> {
    let name = str_of(field(op, "map")?, "map")?;
    let index = || -> R<u32> { u32_of(field(op, "index")?, "index") };
    Ok(match name {
        "intrinsic_land_value" => SourceMap::IntrinsicLandValue,
        "total_land_value" => SourceMap::TotalLandValue,
        "slope" => SourceMap::Slope,
        "r_proximity" => SourceMap::ResidentialProximity(index()?),
        "co_proximity" => SourceMap::CommercialProximity(index()?),
        "park" => SourceMap::Park,
        "pollution" => SourceMap::Pollution(index()?),
        "school" => SourceMap::SchoolGrade,
        "hospital" => SourceMap::HospitalGrade,
        "crime" => SourceMap::Crime,
        "trip_length" => SourceMap::TripLength,
        "commercial_traffic" => SourceMap::CommercialTraffic,
        "landmark" => SourceMap::Landmark,
        "transient" => SourceMap::TransientAura,
        _ => return Err(format!("unknown source map '{name}'")),
    })
}

/// The element type of a source map (§4.3).
fn source_kind(m: SourceMap) -> &'static str {
    match m {
        SourceMap::Slope | SourceMap::SchoolGrade | SourceMap::HospitalGrade => "f32",
        SourceMap::Park | SourceMap::Pollution(_) | SourceMap::Landmark => "i16",
        SourceMap::TransientAura => "i8",
        _ => "u8",
    }
}

fn int_field(e: &Json, k: &str) -> R<i32> {
    Ok(i64_of(field(e, k)?, k)? as i32)
}

fn opt_i32(e: &Json, k: &str, default: i32) -> R<i32> {
    opt(e, k).map_or(Ok(default), |v| Ok(i64_of(v, k)? as i32))
}

fn opt_u32(e: &Json, k: &str, default: u32) -> R<u32> {
    opt(e, k).map_or(Ok(default), |v| u32_of(v, k))
}

fn opt_bool(e: &Json, k: &str, default: bool) -> R<bool> {
    opt(e, k).map_or(Ok(default), |v| bool_of(v, k))
}

fn ints<const N: usize>(v: &Json, k: &str) -> R<[i32; N]> {
    let a = arr(v, k)?;
    if a.len() != N {
        return Err(format!("'{k}' must have {N} elements"));
    }
    let mut out = [0i32; N];
    for (o, j) in out.iter_mut().zip(a) {
        *o = i64_of(j, k)? as i32;
    }
    Ok(out)
}

fn items_json(items: &[GrowthItem]) -> Json {
    Json::Array(
        items
            .iter()
            .map(|i| Json::Array(vec![ji(i.x), ji(i.z), ji(i.max_stage)]))
            .collect(),
    )
}

fn items_of(v: &Json, k: &str) -> R<Vec<GrowthItem>> {
    arr(v, k)?
        .iter()
        .map(|e| {
            let a = arr(e, k)?;
            let g = |i: usize| a.get(i).map_or(Ok(1), |j| i64_of(j, k).map(|v| v as i32));
            Ok(GrowthItem {
                x: g(0)?,
                z: g(1)?,
                max_stage: g(2)?,
            })
        })
        .collect()
}

/// Writes `fill` and `tracts` (`[[x, z, v], …]`) into a grid.
fn fill_grid<T: GridElem>(g: &dyn TractGrid<T>, op: &Json, conv: &dyn Fn(&Json) -> R<T>) -> R<()> {
    if let Some(v) = opt(op, "fill") {
        g.fill(conv(v)?);
    }
    if let Some(v) = opt(op, "tracts") {
        for t in arr(v, "tracts")? {
            let a = arr(t, "tracts")?;
            if a.len() != 3 {
                return Err("a tract is [x, z, v]".into());
            }
            g.set(i64_of(&a[0], "x")? as i32, i64_of(&a[1], "z")? as i32, conv(&a[2])?);
        }
    }
    Ok(())
}

fn conv_int<T: GridElem>(j: &Json) -> R<T> {
    Ok(T::from_mean(i64_of(j, "value")? as i32))
}

/// Checks `instance?`, `size?`, `counts?`, `all?`, `tracts?` and
/// `nonzero?` of a grid; `json` converts a value for comparison.
fn check_grid<T: GridElem>(g: &dyn TractGrid<T>, instance: Option<u32>, op: &Json, json: &dyn Fn(T) -> Json) -> R<()> {
    let geo = g.geometry();
    if let Some(v) = opt(op, "instance") {
        let want = u32_of(v, "instance")?;
        if instance != Some(want) {
            return Err(format!("instance {instance:X?}, expected {want:08X}"));
        }
    }
    if let Some(v) = opt(op, "size") {
        if i64::from(geo.tract_size) != i64_of(v, "size")? {
            return Err(format!("tract size {}", geo.tract_size));
        }
    }
    if let Some(v) = opt(op, "counts") {
        let [x, z] = ints::<2>(v, "counts")?;
        if (geo.count_x, geo.count_z) != (x, z) {
            return Err(format!("counts {} × {}", geo.count_x, geo.count_z));
        }
    }
    let (cx, cz) = (geo.count_x.clamp(0, 4096), geo.count_z.clamp(0, 4096));
    if let Some(want) = opt(op, "all") {
        for x in 0..cx {
            for z in 0..cz {
                let got = json(g.get(x, z));
                if !scalar::matches(&got, want, false) {
                    return Err(format!(
                        "tract ({x}, {z}) = {}, expected {}",
                        scalar::show(&got),
                        scalar::show(want)
                    ));
                }
            }
        }
    }
    if let Some(v) = opt(op, "tracts") {
        let mut errors = Vec::new();
        for t in arr(v, "tracts")? {
            let a = arr(t, "tracts")?;
            if a.len() != 3 {
                return Err("a tract is [x, z, v]".into());
            }
            let (x, z) = (i64_of(&a[0], "x")? as i32, i64_of(&a[1], "z")? as i32);
            let got = json(g.get(x, z));
            if !scalar::matches(&got, &a[2], false) {
                errors.push(format!(
                    "({x}, {z}) = {}, expected {}",
                    scalar::show(&got),
                    scalar::show(&a[2])
                ));
            }
        }
        if !errors.is_empty() {
            return Err(errors.join("; "));
        }
    }
    if let Some(v) = opt(op, "nonzero") {
        let mut n = 0i64;
        for x in 0..cx {
            for z in 0..cz {
                if json(g.get(x, z)) != ji(0) && json(g.get(x, z)) != jf(0.0) {
                    n += 1;
                }
            }
        }
        if n != i64_of(v, "nonzero")? {
            return Err(format!("{n} non-zero tracts"));
        }
    }
    Ok(())
}

// ---- the host -------------------------------------------------------------------------------------

macro_rules! provider {
    ($t:ty, $list:ident) => {
        impl GridProvider<$t> for World {
            fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<$t>>> {
                self.events.push(
                    "create_grid",
                    vec![("instance", hex(instance)), ("size", ji(tract_size))],
                );
                if self.no_grids.get() {
                    return None;
                }
                let g = SimGrid::<$t>::create(&self.dims(), instance, tract_size);
                self.$list.borrow_mut().push(g.clone());
                Some(g as Rc<dyn TractGrid<$t>>)
            }

            fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<$t>>> {
                let g = r.object::<SimGrid<$t>>()?;
                self.$list.borrow_mut().push(g.clone());
                Some(g as Rc<dyn TractGrid<$t>>)
            }
        }
    };
}

provider!(u8, grids_u8);
provider!(i16, grids_i16);
provider!(u16, grids_u16);

impl GrowthDeveloperHost for World {
    fn acquire_service(&self, s: Service) -> bool {
        let ok = !self.missing_services.borrow().contains(&s);
        self.events.push(
            "acquire_service",
            vec![("service", Json::Str(service_name(s).into())), ("ok", Json::Bool(ok))],
        );
        ok
    }
    fn release_service(&self, s: Service) {
        self.events
            .push("release_service", vec![("service", Json::Str(service_name(s).into()))]);
    }
    fn acquire_message_server(&self) -> bool {
        let ok = !self.no_message_server.get();
        self.events.push("acquire_message_server", vec![("ok", Json::Bool(ok))]);
        ok
    }
    fn release_message_server(&self) {
        self.events.push("release_message_server", vec![]);
    }
    fn subscribe(&self, t: &[u32]) {
        self.events.push("subscribe", vec![("types", types_json(t))]);
    }
    fn unsubscribe(&self, t: &[u32]) {
        self.events.push("unsubscribe", vec![("types", types_json(t))]);
    }
    fn simulator_present(&self) -> bool {
        !self.no_simulator.get()
    }
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        self.events.push(
            "agent",
            vec![
                ("level", ji(level)),
                ("name", Json::Str(name.into())),
                ("flags", ji(flags)),
            ],
        );
    }
    fn remove_agent(&self) {
        self.events.push("remove_agent", vec![]);
    }
    fn sim_date_number(&self) -> u32 {
        self.date.get()
    }
    fn grid_set_tract_size(&self, which: DevGrid, grid: GridHandle<'_>, s: i32) {
        self.events.push(
            "set_tract_size",
            vec![
                ("which", Json::Str(dev_grid_name(which).into())),
                ("grid", self.handle_name(grid)),
                ("size", ji(s)),
            ],
        );
        match grid {
            GridHandle::U8(g) => self.resize(&self.grids_u8.borrow(), g, s),
            GridHandle::I16(g) => self.resize(&self.grids_i16.borrow(), g, s),
            GridHandle::U16(g) => self.resize(&self.grids_u16.borrow(), g, s),
        }
    }
    fn grid_shutdown(&self, which: DevGrid, grid: GridHandle<'_>) {
        self.events.push(
            "grid_shutdown",
            vec![
                ("which", Json::Str(dev_grid_name(which).into())),
                ("grid", self.handle_name(grid)),
            ],
        );
    }
    fn add_ref(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) += 1;
        self.events.push("add_ref", vec![("object", self.name(token))]);
    }
    fn release(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) -= 1;
        self.events.push("release", vec![("object", self.name(token))]);
    }
    fn get_demand(&self, id: u32, index: u32) -> Option<u64> {
        let node = self.demand.borrow().get(&(id, index)).copied();
        if self.log_supply.get() {
            self.events.push(
                "get_demand",
                vec![("id", hex(id)), ("index", hex(index)), ("node", self.opt_name(node))],
            );
        }
        node
    }
    fn query_supply(&self, node: u64) -> f32 {
        if self.log_supply.get() {
            self.events.push("query_supply", vec![("node", self.name(node))]);
        }
        self.supply.borrow().get(&node).copied().unwrap_or(0.0)
    }
    fn add_to_demand_value(&self, node: u64, v: f32) {
        *self.demand_added.borrow_mut().entry(node).or_insert(0.0) += f64::from(v);
        self.events.push(
            "add_to_demand",
            vec![("node", self.name(node)), ("value", jf(v)), ("bits", hex(v.to_bits()))],
        );
    }
    fn active_demand(&self, node: u64) -> f32 {
        self.events.push("active_demand", vec![("node", self.name(node))]);
        self.active.borrow().get(&node).copied().unwrap_or(0.0)
    }
    fn connected_cities(&self) -> u32 {
        if self.log_supply.get() {
            self.events.push("connected_cities", vec![]);
        }
        self.connected.get().0
    }
    fn all_connected_cities(&self) -> u32 {
        if self.log_supply.get() {
            self.events.push("all_connected_cities", vec![]);
        }
        self.connected.get().1
    }
    fn stage_population_multiplier(&self) -> f32 {
        if self.log_supply.get() {
            self.events.push("stage_multiplier", vec![]);
        }
        self.multiplier.get()
    }
    fn tract_developer(&self) -> Option<u64> {
        self.events.push("tract_developer", vec![]);
        self.tract_developer.get()
    }
    fn special_buildings(&self) -> Vec<u64> {
        self.events.push("special_buildings", vec![]);
        self.specials.borrow().clone()
    }
    fn max_capacity(&self, zone_type: i32, purpose: u32, wealth: u32) -> f32 {
        self.events.push(
            "max_capacity",
            vec![
                ("zone", ji(zone_type)),
                ("purpose", ji(purpose)),
                ("wealth", ji(wealth)),
            ],
        );
        self.max_capacity.borrow().get(&zone_type).copied().unwrap_or(0.0)
    }
    fn building_purpose(&self, building_type: u32) -> u32 {
        self.events
            .push("building_purpose", vec![("building_type", hex(building_type))]);
        self.purposes.borrow().get(&building_type).copied().unwrap_or(0)
    }
    fn update_population(&self, td: u64, p: &GrowthProfile) {
        if self.log_population.get() {
            self.events.push(
                "update_population",
                vec![
                    ("tract_developer", self.name(td)),
                    ("developer", self.name(p.developer)),
                    ("x", ji(p.x)),
                    ("z", ji(p.z)),
                    ("stage", ji(p.stage)),
                ],
            );
        }
    }
    fn zone_type(&self, x: i32, z: i32) -> i8 {
        if self.log_zone.get() {
            self.events.push("zone_type", vec![("x", ji(x)), ("z", ji(z))]);
        }
        self.zones.borrow().get(&(x, z)).copied().unwrap_or(0)
    }
    fn lot_building_type(&self, lot: u64, flag: bool) -> u32 {
        self.events.push(
            "lot_building_type",
            vec![("lot", self.name(lot)), ("flag", Json::Bool(flag))],
        );
        self.lots.borrow().get(&lot).map_or(0, |l| l.building_type)
    }
    fn lot_capacity(&self, lot: u64, developer_type: u32, flag: bool) -> u16 {
        self.events.push(
            "lot_capacity",
            vec![
                ("lot", self.name(lot)),
                ("developer_type", hex(developer_type)),
                ("flag", Json::Bool(flag)),
            ],
        );
        self.lots.borrow().get(&lot).map_or(0, |l| l.capacity)
    }
    fn lot_size(&self, lot: u64) -> (u8, u8) {
        self.events.push("lot_size", vec![("lot", self.name(lot))]);
        self.lots.borrow().get(&lot).map_or((0, 0), |l| l.size)
    }
    fn lot_bounding_rect(&self, lot: u64) -> [i32; 4] {
        self.events.push("lot_rect", vec![("lot", self.name(lot))]);
        self.lots.borrow().get(&lot).map_or([0; 4], |l| l.rect)
    }
    fn lot_location(&self, lot: u64) -> (i32, i32) {
        self.events.push("lot_location", vec![("lot", self.name(lot))]);
        self.lots.borrow().get(&lot).map_or((0, 0), |l| l.location)
    }
    fn occupant_property(&self, occ: u64, id: u32) -> Option<Variant> {
        self.occs.borrow().get(&occ)?.props.get(id).cloned()
    }
    fn occupant_position(&self, occ: u64) -> [f32; 3] {
        self.occs.borrow().get(&occ).map_or([0.0; 3], |o| o.pos)
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        ((x / 16.0).floor() as i32, (z / 16.0).floor() as i32)
    }
    fn source_u8(&self, map: SourceMap) -> Option<Rc<dyn TractGrid<u8>>> {
        match self.sources.borrow().get(&map)? {
            Src::U8(g) => Some(g.clone() as Rc<dyn TractGrid<u8>>),
            _ => None,
        }
    }
    fn source_i8(&self, map: SourceMap) -> Option<Rc<dyn TractGrid<i8>>> {
        match self.sources.borrow().get(&map)? {
            Src::I8(g) => Some(g.clone() as Rc<dyn TractGrid<i8>>),
            _ => None,
        }
    }
    fn source_i16(&self, map: SourceMap) -> Option<Rc<dyn TractGrid<i16>>> {
        match self.sources.borrow().get(&map)? {
            Src::I16(g) => Some(g.clone() as Rc<dyn TractGrid<i16>>),
            _ => None,
        }
    }
    fn source_f32(&self, map: SourceMap) -> Option<Rc<dyn TractGrid<f32>>> {
        match self.sources.borrow().get(&map)? {
            Src::F32(g) => Some(g.clone() as Rc<dyn TractGrid<f32>>),
            _ => None,
        }
    }
    fn limit_timer_start(&self, ms: u32) {
        self.events.push("timer_start", vec![("ms", ji(ms))]);
        self.timer_left.set(self.timer_checks.get().unwrap_or(0));
    }
    fn limit_timer_expired(&self) -> bool {
        if self.timer_checks.get().is_none() {
            return false;
        }
        let left = self.timer_left.get();
        if left == 0 {
            return true;
        }
        self.timer_left.set(left - 1);
        false
    }
    fn session_seed(&self) -> u32 {
        self.events.push("session_seed", vec![]);
        self.wall_seed.get()
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "growth_developer", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("gdev_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "source" => source(&w, op),
        "statics" => statics(&w, op),
        "occupant" => occupant(&w, op),
        "specials" => specials(&w, op),
        "lot" => lot(&w, op),
        "create" => create(&w, op),
        "init" => init(&w, op),
        "shutdown" => w.dev(op).and_then(|d| check_expect(op, Json::Bool(d.shutdown()))),
        "idle" => idle(&w, op),
        "update" => update(&w, op),
        "message" => message(&w, op),
        "call" => call(&w, op),
        "set" => set(&w, op),
        "expect" => expect(&w, op),
        "expect_grid" => expect_grid(&w, op),
        "expect_refs" => expect_refs(&w, op),
        "expect_demand" => expect_demand(&w, op),
        "expect_salts" => expect_salts(&w, op),
        "expect_events" => w.events.expect_events(op),
        "expect_event_count" => w.events.expect_count(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }
        "sort" => sort(op),
        "save" => save(&w, op),
        "load" => load(&w, op),
        "expect_save" => expect_save(&w, op),
        "patch" => patch(&w, op),
        "expect_payload" => expect_payload(&w, op),
        "read" => read(&w, op),
        _ => Err(format!("unknown op '{name}'")),
    })
}

// ---- environment ----------------------------------------------------------------------------------

/// `gdev_env`: the scripted world (see README).
fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "cells") {
        let [x, z] = ints::<2>(v, "cells")?;
        if !(0..=4096).contains(&x) || !(0..=4096).contains(&z) {
            return Err("cells must be 0..=4096".into());
        }
        w.cells.set((x, z));
    }
    if let Some(v) = opt(op, "date") {
        w.date.set(u32_of(v, "date")?);
    }
    if let Some(v) = opt(op, "wall_seed") {
        w.wall_seed.set(u32_of(v, "wall_seed")?);
    }
    if let Some(v) = op.get("missing_services") {
        let mut m = Vec::new();
        for s in arr(v, "missing_services")? {
            m.push(service_of(str_of(s, "service")?)?);
        }
        *w.missing_services.borrow_mut() = m;
    }
    for (k, c) in [
        ("no_message_server", &w.no_message_server),
        ("no_simulator", &w.no_simulator),
        ("no_grids", &w.no_grids),
        ("log_population", &w.log_population),
        ("log_zone", &w.log_zone),
        ("log_supply", &w.log_supply),
    ] {
        if let Some(v) = opt(op, k) {
            c.set(bool_of(v, k)?);
        }
    }
    if let Some(v) = op.get("tract_developer") {
        let on = !matches!(v, Json::Null) && bool_of(v, "tract_developer")?;
        w.tract_developer.set(on.then(|| w.token_of("tract_developer")));
    }
    if let Some(v) = opt(op, "multiplier") {
        w.multiplier.set(f32_of(v, "multiplier")?);
    }
    if let Some(v) = opt(op, "connected") {
        let [a, b] = ints::<2>(v, "connected")?;
        w.connected.set((a as u32, b as u32));
    }
    if opt_bool(op, "clear_demand", false)? {
        w.demand.borrow_mut().clear();
    }
    if let Some(v) = opt(op, "demand") {
        for e in arr(v, "demand")? {
            let id = u32_of(field(e, "id")?, "id")?;
            let index = opt_u32(e, "index", 0)?;
            let label = match opt(e, "label") {
                Some(l) => str_of(l, "label")?.to_owned(),
                None => format!("demand_{id:X}_{index:X}"),
            };
            let t = w.token_of(&label);
            w.demand.borrow_mut().insert((id, index), t);
            if let Some(s) = opt(e, "supply") {
                w.supply.borrow_mut().insert(t, f32_of(s, "supply")?);
            }
            if let Some(s) = opt(e, "active") {
                w.active.borrow_mut().insert(t, f32_of(s, "active")?);
            }
        }
    }
    if let Some(v) = op.get("timer_checks") {
        w.timer_checks.set(match v {
            Json::Null => None,
            v => Some(u32_of(v, "timer_checks")?),
        });
    }
    if let Some(v) = opt(op, "max_capacity") {
        for (k, c) in v.as_object().ok_or("max_capacity must be an object")? {
            let z = Json::Str(k.clone()).as_i64().ok_or("bad zone type")? as i32;
            w.max_capacity.borrow_mut().insert(z, f32_of(c, "max_capacity")?);
        }
    }
    if let Some(v) = opt(op, "purposes") {
        for (k, p) in v.as_object().ok_or("purposes must be an object")? {
            let t = Json::Str(k.clone()).as_i64().ok_or("bad building type")? as u32;
            w.purposes.borrow_mut().insert(t, u32_of(p, "purpose")?);
        }
    }
    if let Some(v) = opt(op, "zones") {
        for e in arr(v, "zones")? {
            let x = int_field(e, "x")?;
            let z = int_field(e, "z")?;
            let wd = opt_i32(e, "w", 1)?;
            let dp = opt_i32(e, "d", 1)?;
            if !(1..=4096).contains(&wd) || !(1..=4096).contains(&dp) {
                return Err("zone rectangle size must be 1..=4096".into());
            }
            let t = int_field(e, "zone")? as i8;
            for cz in z..z + dp {
                for cx in x..x + wd {
                    w.zones.borrow_mut().insert((cx, cz), t);
                }
            }
        }
    }
    Ok(())
}

/// `gdev_source`: `map`, `index?`, `size` (tract size), `fill?`,
/// `tracts? [[x, z, v]]`; `none: true` removes the map.
fn source(w: &World, op: &Json) -> R<()> {
    let m = source_of(op)?;
    if opt_bool(op, "none", false)? {
        w.sources.borrow_mut().remove(&m);
        return Ok(());
    }
    let size = int_field(op, "size")?;
    let dims = w.dims();
    let src = match source_kind(m) {
        "u8" => {
            let g = SimGrid::<u8>::create(&dims, 0, size);
            fill_grid(&*g, op, &conv_int::<u8>)?;
            Src::U8(g)
        }
        "i8" => {
            let g = SimGrid::<i8>::create(&dims, 0, size);
            fill_grid(&*g, op, &conv_int::<i8>)?;
            Src::I8(g)
        }
        "i16" => {
            let g = SimGrid::<i16>::create(&dims, 0, size);
            fill_grid(&*g, op, &conv_int::<i16>)?;
            Src::I16(g)
        }
        _ => {
            let g = SimGrid::<f32>::create(&dims, 0, size);
            fill_grid(&*g, op, &|j| f32_of(j, "value"))?;
            Src::F32(g)
        }
    };
    w.sources.borrow_mut().insert(m, src);
    Ok(())
}

/// `gdev_statics`: `seed?` (reseeds the shared generator), `session?` (with
/// `fresh`: `GrowthStatics::from_session`, foundations §6.3), `slice_step?`,
/// `misses?`, `fresh?` (a new shared object for developers created later).
fn statics(w: &World, op: &Json) -> R<()> {
    let s = seeds(w.wall_seed.get());
    if opt_bool(op, "fresh", false)? {
        *w.statics.borrow_mut() = match opt(op, "session") {
            Some(v) => GrowthStatics::from_session(u32_of(v, "session")?, &s),
            None => GrowthStatics::new(opt_u32(op, "seed", 0)?, &s),
        };
    } else if opt(op, "session").is_some() {
        return Err("'session' needs 'fresh': true".into());
    } else if let Some(v) = opt(op, "seed") {
        w.statics().reseed(u32_of(v, "seed")?, &s);
    }
    let st = w.statics();
    let mut step = st.slice_step();
    if let Some(v) = opt(op, "slice_step") {
        step.step = u32_of(v, "slice_step")?;
    }
    if let Some(v) = opt(op, "misses") {
        step.misses = i64_of(v, "misses")? as i32;
    }
    st.set_slice_step(step);
    Ok(())
}

/// `gdev_occupant`: `label`, `props?`, `position? [x, y, z]` (or `cell? [x,
/// z]`: the cell's centre).
fn occupant(w: &World, op: &Json) -> R<()> {
    let label = str_of(field(op, "label")?, "label")?;
    let t = w.token_of(label);
    let mut o = w.occs.borrow().get(&t).cloned().unwrap_or_default();
    if let Some(v) = opt(op, "props") {
        apply_props(&mut o.props, v)?;
    }
    if let Some(v) = opt(op, "position") {
        let a = arr(v, "position")?;
        if a.len() != 3 {
            return Err("position is [x, y, z]".into());
        }
        for (i, j) in a.iter().enumerate() {
            o.pos[i] = f32_of(j, "position")?;
        }
    }
    if let Some(v) = opt(op, "cell") {
        let [x, z] = ints::<2>(v, "cell")?;
        o.pos = [x as f32 * 16.0 + 8.0, 0.0, z as f32 * 16.0 + 8.0];
    }
    w.occs.borrow_mut().insert(t, o);
    Ok(())
}

/// `gdev_specials`: `list` (occupant labels): the BDS's special buildings.
fn specials(w: &World, op: &Json) -> R<()> {
    let mut l = Vec::new();
    for e in arr(field(op, "list")?, "list")? {
        l.push(w.token_of(str_of(e, "list")?));
    }
    *w.specials.borrow_mut() = l;
    Ok(())
}

/// `gdev_lot`: `label`, `building_type?`, `capacity?`, `size? [w, d]`,
/// `rect? [x0, z0, x1, z1]`, `location? [x, z]`.
fn lot(w: &World, op: &Json) -> R<()> {
    let label = str_of(field(op, "label")?, "label")?;
    let t = w.token_of(label);
    let mut l = w.lots.borrow().get(&t).copied().unwrap_or_default();
    if let Some(v) = opt(op, "building_type") {
        l.building_type = u32_of(v, "building_type")?;
    }
    if let Some(v) = opt(op, "capacity") {
        l.capacity = u32_of(v, "capacity")? as u16;
    }
    if let Some(v) = opt(op, "size") {
        let [a, b] = ints::<2>(v, "size")?;
        l.size = (a as u8, b as u8);
    }
    if let Some(v) = opt(op, "rect") {
        l.rect = ints::<4>(v, "rect")?;
    }
    if let Some(v) = opt(op, "location") {
        let [a, b] = ints::<2>(v, "location")?;
        l.location = (a, b);
    }
    w.lots.borrow_mut().insert(t, l);
    Ok(())
}

/// Adds (or, with `{"remove": true}`, removes) properties.
fn apply_props(h: &mut PropertyHolder, v: &Json) -> R<()> {
    for (k, val) in v.as_object().ok_or("props must be an object")? {
        let id = Json::Str(k.clone())
            .as_i64()
            .ok_or_else(|| format!("bad property ID '{k}'"))? as u32;
        if val.get("remove").is_some() {
            h.remove(id);
        } else {
            h.add(id, variant_of(val).map_err(|e| format!("property {k}: {e}"))?);
        }
    }
    Ok(())
}

// ---- lifecycle and driving ------------------------------------------------------------------------

/// `gdev_create`: `label?` ("dev"): a constructed developer sharing the
/// world's statics; becomes the current developer.
fn create(w: &World, op: &Json) -> R<()> {
    let label = opt(op, "label").map_or(Ok("dev"), |v| str_of(v, "label"))?.to_owned();
    let t = w.token(&label);
    let host = w.rc() as Rc<dyn GrowthDeveloperHost>;
    // `add_ref` (default true): `CreateDeveloper`'s constructor + `AddRef`.
    let d = if opt_bool(op, "add_ref", true)? {
        GrowthDeveloper::create(host, w.statics(), t)
    } else {
        GrowthDeveloper::new(host, w.statics(), t)
    };
    w.devs.borrow_mut().insert(label.clone(), d);
    *w.current.borrow_mut() = label;
    Ok(())
}

/// `gdev_init`: `exemplar?` (a shipped developer type; default an empty
/// exemplar), `props?`, `null?`, `expect?`.
fn init(w: &World, op: &Json) -> R<()> {
    let d = w.dev(op)?;
    let holder = if opt_bool(op, "null", false)? {
        None
    } else {
        let mut h = match opt(op, "exemplar") {
            Some(v) => {
                let t = u32_of(v, "exemplar")?;
                shipped_exemplar(t).ok_or_else(|| format!("no shipped exemplar {t:#X}"))?
            }
            None => PropertyHolder::new(),
        };
        if let Some(v) = opt(op, "props") {
            apply_props(&mut h, v)?;
        }
        Some(Rc::new(h))
    };
    check_expect(op, Json::Bool(d.init(holder)))
}

/// `gdev_idle`: `count?` (1), `ms?` (5): SimIdle messages.
fn idle(w: &World, op: &Json) -> R<()> {
    let d = w.dev(op)?;
    let count = opt_i32(op, "count", 1)?;
    if !(1..=1_000_000).contains(&count) {
        return Err("'count' must be 1..=1000000".into());
    }
    let ms = opt_u32(op, "ms", 5)?;
    let mut r = true;
    for _ in 0..count {
        let m = GrowthMessage {
            data3: ms,
            ..GrowthMessage::of_type(msg::SIM_IDLE)
        };
        r = d.handle_message(&m);
    }
    check_expect(op, Json::Bool(r))
}

/// `gdev_update`: `StartMapUpdate`, then `DoIdle(ms?)` until the update has
/// finished; `calls?` checks the number of `DoIdle` calls.
fn update(w: &World, op: &Json) -> R<()> {
    let d = w.dev(op)?;
    let ms = opt_u32(op, "ms", 5)?;
    d.start_map_update();
    let mut calls = 0i64;
    while !d.is_map_update_finished() {
        d.do_idle(ms);
        calls += 1;
        if calls > 10_000_000 {
            return Err(format!("update stuck in state {}", d.data().update_state));
        }
    }
    if let Some(v) = opt(op, "calls") {
        if calls != i64_of(v, "calls")? {
            return Err(format!("{calls} DoIdle calls"));
        }
    }
    Ok(())
}

fn msg_type(j: &Json) -> R<u32> {
    if let Some(s) = j.as_str() {
        match s {
            "SimIdle" => return Ok(msg::SIM_IDLE),
            "SimNewDay" => return Ok(msg::SIM_NEW_DAY),
            "ZoningChange" => return Ok(msg::ZONING_CHANGE),
            "StateChanged" => return Ok(msg::STATE_CHANGED),
            _ => {}
        }
    }
    u32_of(j, "type")
}

/// `gdev_message`: `type`, `data1..4?`, `rect? [x0, z0, x1, z1]`, `lot?`,
/// `expect?`.
fn message(w: &World, op: &Json) -> R<()> {
    let d = w.dev(op)?;
    let mut m = GrowthMessage::of_type(msg_type(field(op, "type")?)?);
    m.data1 = opt_u32(op, "data1", 0)?;
    m.data2 = opt_u32(op, "data2", 0)?;
    m.data3 = opt_u32(op, "data3", 0)?;
    m.data4 = opt_u32(op, "data4", 0)?;
    if let Some(v) = opt(op, "rect") {
        m.rect = ints::<4>(v, "rect")?;
    }
    if let Some(v) = opt(op, "lot") {
        m.unknown = Some(w.find(str_of(v, "lot")?)?);
    }
    check_expect(op, Json::Bool(d.handle_message(&m)))
}

fn a_i32(op: &Json, i: usize) -> R<i32> {
    Ok(i64_of(arg(op, i)?, "argument")? as i32)
}

fn a_u32(op: &Json, i: usize) -> R<u32> {
    u32_of(arg(op, i)?, "argument")
}

fn a_f32(op: &Json, i: usize) -> R<f32> {
    f32_of(arg(op, i)?, "argument")
}

fn float_result(op: &Json, v: f32) -> R<()> {
    if let Some(b) = opt(op, "expect_bits") {
        let want = u32_of(b, "expect_bits")?;
        if v.to_bits() != want {
            return Err(format!("bits 0x{:08X}, expected 0x{want:08X} (value {v})", v.to_bits()));
        }
    }
    check_expect(op, jf(v))
}

/// `gdev_call`: `method`, `args?`, `expect?` (and `expect_bits?` for float
/// results): the §10 slots and the pure helpers.
fn call(w: &World, op: &Json) -> R<()> {
    let method = str_of(field(op, "method")?, "method")?;
    // Pure helpers first (no developer needed).
    let pure = match method {
        "population_of" => Some(ji(gd::population_of(a_i32(op, 0)? as i16, a_u32(op, 1)? as u16))),
        "desirability_fraction" => return float_result(op, gd::desirability_fraction(a_i32(op, 0)? as i16)),
        "add_curve" => Some(ji(gd::add_curve(a_i32(op, 0)? as i16, a_f32(op, 1)?))),
        "commute_add" => Some(ji(gd::commute_add(a_i32(op, 0)? as i16, a_f32(op, 1)?))),
        "phantom_term" => {
            Some(gd::phantom_term(a_i32(op, 0)?, a_i32(op, 1)? as i16, a_i32(op, 2)? as i16).map_or(Json::Null, ji))
        }
        "round_capacity" => Some(ji(gd::round_capacity(a_f32(op, 0)?))),
        "split_capacity" => Some(
            gd::split_capacity(a_i32(op, 0)?, a_i32(op, 1)?)
                .map_or(Json::Null, |(q, r)| Json::Array(vec![ji(q), ji(r)])),
        ),
        "decline_key_seed" => Some(ji(gd::decline_key_seed(a_i32(op, 0)?, a_i32(op, 1)?, a_u32(op, 2)?))),
        "decline_key" => {
            let wall = w.wall_seed.get();
            Some(ji(gd::decline_key(a_i32(op, 0)?, a_i32(op, 1)?, a_u32(op, 2)?, wall)))
        }
        "style_fraction" => return float_result(op, gd::style_fraction(a_u32(op, 0)?)),
        "query_interface" => Some(match query_interface(a_u32(op, 0)?) {
            Some(i) => Json::Str(format!("{i:?}")),
            None => Json::Null,
        }),
        "slot" => {
            let n = a_u32(op, 0)?;
            let r = SLOTS.iter().find(|r| r.0 == n).ok_or("no such slot")?;
            Some(Json::Array(vec![
                Json::Str(r.1.into()),
                ji(r.2),
                Json::Str(format!("{:?}", r.3)),
            ]))
        }
        "draw_salt" => Some(ji(w.statics().draw_salt())),
        _ => None,
    };
    if let Some(got) = pure {
        return check_expect(op, got);
    }
    let d = w.dev(op)?;
    let got = match method {
        "add_ref" => ji(d.add_ref()),
        "release" => ji(d.release()),
        "target_zone" => ji(d.target_zone()),
        "developer_type" => ji(d.developer_type()),
        "sub_type" => ji(d.sub_type()),
        "purpose" => ji(d.purpose()),
        "wealth" => ji(d.wealth()),
        "construction_info" => {
            // The caller's profile starts with `args[0]` as its zone code.
            let mut p = ConstructionProfile {
                zone: op.get("args").map_or(Ok(-1), |_| a_i32(op, 0))?,
                unknown2: -1,
                ..ConstructionProfile::default()
            };
            d.construction_info(&mut p);
            Json::Object(vec![
                ("zone".into(), ji(p.zone)),
                ("unknown2".into(), ji(p.unknown2)),
                ("wealth".into(), ji(p.wealth)),
                ("purpose".into(), ji(p.purpose)),
                ("land_value_min".into(), ji(p.land_value_min)),
                ("land_value_max".into(), ji(p.land_value_max)),
            ])
        }
        "lists_valid" => Json::Bool(d.lists_valid()),
        "growth_list" => items_json(&d.growth_list()),
        "decline_list" => items_json(&d.decline_list()),
        "growth_items" => items_json(&d.growth_items()),
        "decline_items" => items_json(&d.decline_items()),
        "desirability_map" => grid_json(w, d.desirability_map().map(|g| inst_i16(w, &g))),
        "capacity_map" => grid_json(w, d.capacity_map().map(|g| inst_u16(w, &g))),
        "population_map" => grid_json(w, d.population_map().map(|g| inst_i16(w, &g))),
        "max_capacity_map" => grid_json(w, d.max_capacity_map().map(|g| inst_u16(w, &g))),
        "total_capacity" => ji(d.total_capacity()),
        "source_demand" => w.opt_name(d.source_demand()),
        "source_active_demand" => match d.source_active_demand() {
            Some(v) => return float_result(op, v),
            None => Json::Null,
        },
        "caps" => Json::Array(d.caps().iter().map(|&c| ji(c)).collect()),
        "desirability_threshold" => ji(d.desirability_threshold()),
        "abandonment_threshold" => return float_result(op, d.abandonment_threshold(a_i32(op, 0)?)),
        "monopoly_use" => ji(d.monopoly_use()),
        "style_percentages" => styles_json(&d.style_percentages()),
        "start_map_update" => {
            d.start_map_update();
            Json::Null
        }
        "is_map_update_finished" => Json::Bool(d.is_map_update_finished()),
        "estimate_desirability" => ji(d.estimate_desirability(a_i32(op, 0)?, a_i32(op, 1)?)),
        "subjective_thresholds" => {
            let (a, b) = d.subjective_thresholds(a_i32(op, 0)?);
            Json::Array(vec![ji(a), ji(b)])
        }
        "new_day_finished" => Json::Bool(d.new_day_finished()),
        "city_size" => return float_result(op, d.city_size()),
        "update_max_capacity" => {
            d.update_max_capacity(ints::<4>(arg(op, 0)?, "rect")?);
            Json::Null
        }
        "update_population_count" => {
            let lot = w.find(str_of(arg(op, 0)?, "lot")?)?;
            d.update_population_count(lot, a_u32(op, 1)?, bool_of(arg(op, 2)?, "add")?);
            Json::Null
        }
        "do_idle" => {
            d.do_idle(a_u32(op, 0)?);
            Json::Null
        }
        _ => return Err(format!("unknown method '{method}'")),
    };
    check_expect(op, got)
}

fn inst_i16(w: &World, g: &Rc<dyn TractGrid<i16>>) -> Json {
    w.handle_name(GridHandle::I16(g))
}

fn inst_u16(w: &World, g: &Rc<dyn TractGrid<u16>>) -> Json {
    w.handle_name(GridHandle::U16(g))
}

fn grid_json(_w: &World, g: Option<Json>) -> Json {
    g.unwrap_or(Json::Null)
}

fn styles_json(m: &BTreeMap<u32, f32>) -> Json {
    Json::Object(m.iter().map(|(k, v)| (format!("0x{k:X}"), jf(*v))).collect())
}

/// `gdev_set`: `state?` (`update_state`, `start_day`, `last_phantom_sum`,
/// `total_capacity`, `growth`, `decline`, `ref_count`), `grid?` with
/// `fill?` / `tracts?`.
fn set(w: &World, op: &Json) -> R<()> {
    let d = w.dev(op)?;
    if let Some(s) = opt(op, "state") {
        for (k, v) in s.as_object().ok_or("state must be an object")? {
            let mut g = d.data_mut();
            match k.as_str() {
                "update_state" => g.update_state = u32_of(v, k)?,
                "start_day" => g.start_day = u32_of(v, k)?,
                "last_phantom_sum" => g.last_phantom_sum = i64_of(v, k)? as i32,
                "total_capacity" => g.total_capacity = i64_of(v, k)? as i32,
                "ref_count" => g.ref_count = u32_of(v, k)?,
                "growth" => g.growth = items_of(v, k)?,
                "decline" => g.decline = items_of(v, k)?,
                _ => return Err(format!("unknown state field '{k}'")),
            }
        }
    }
    if let Some(v) = opt(op, "grid") {
        let name = str_of(v, "grid")?;
        match name {
            "zoned" => fill_grid(&*d.zoned_count_map().ok_or("no grid")?, op, &conv_int::<u8>)?,
            "working" => fill_grid(&*d.working_map().ok_or("no grid")?, op, &conv_int::<i16>)?,
            "desirability" => fill_grid(&*d.desirability_map().ok_or("no grid")?, op, &conv_int::<i16>)?,
            "population" => fill_grid(&*d.population_map().ok_or("no grid")?, op, &conv_int::<i16>)?,
            "capacity" => fill_grid(&*d.capacity_map().ok_or("no grid")?, op, &conv_int::<u16>)?,
            "max_capacity" => fill_grid(&*d.max_capacity_map().ok_or("no grid")?, op, &conv_int::<u16>)?,
            _ => return Err(format!("unknown grid '{name}'")),
        }
    }
    Ok(())
}

/// `gdev_expect_grid`: `grid`, then the checks of [`check_grid`];
/// `exists: false` checks that the grid is null.
fn expect_grid(w: &World, op: &Json) -> R<()> {
    let d = w.dev(op)?;
    let name = str_of(field(op, "grid")?, "grid")?;
    let want_exists = opt_bool(op, "exists", true)?;
    macro_rules! go {
        ($g:expr, $h:ident, $t:ty) => {{
            let g = $g;
            if g.is_some() != want_exists {
                return Err(format!("grid {name} exists = {}", g.is_some()));
            }
            let Some(g) = g else { return Ok(()) };
            let inst = w
                .handle_name(GridHandle::$h(&g))
                .as_str()
                .and_then(|s| Json::Str(s.into()).as_i64());
            check_grid::<$t>(&*g, inst.map(|v| v as u32), op, &|v| ji(v))
        }};
    }
    match name {
        "zoned" => go!(d.zoned_count_map(), U8, u8),
        "working" => go!(d.working_map(), I16, i16),
        "desirability" => go!(d.desirability_map(), I16, i16),
        "population" => go!(d.population_map(), I16, i16),
        "capacity" => go!(d.capacity_map(), U16, u16),
        "max_capacity" => go!(d.max_capacity_map(), U16, u16),
        _ => Err(format!("unknown grid '{name}'")),
    }
}

/// `gdev_expect`: `state` (a partial object of the fields below).
fn expect(w: &World, op: &Json) -> R<()> {
    let d = w.dev(op)?;
    let st = w.statics();
    let g = d.data().clone();
    let t = &g.tuning;
    let it = g.iterator.as_ref().map_or(Json::Null, |i| {
        Json::Object(vec![
            ("x".into(), ji(i.x)),
            ("z".into(), ji(i.z)),
            ("rect".into(), Json::Array(vec![ji(i.x0), ji(i.z0), ji(i.x1), ji(i.z1)])),
            (
                "state".into(),
                Json::Str(
                    match i.state {
                        ScanState::Fresh => "fresh",
                        ScanState::Running => "running",
                        ScanState::Done => "done",
                    }
                    .into(),
                ),
            ),
        ])
    });
    let grids = {
        let mut kv = Vec::new();
        let names = [
            ("zoned", d.zoned_count_map().map(|g| w.handle_name(GridHandle::U8(&g)))),
            ("working", d.working_map().map(|g| inst_i16(w, &g))),
            ("desirability", d.desirability_map().map(|g| inst_i16(w, &g))),
            ("population", d.population_map().map(|g| inst_i16(w, &g))),
            ("capacity", d.capacity_map().map(|g| inst_u16(w, &g))),
            ("max_capacity", d.max_capacity_map().map(|g| inst_u16(w, &g))),
        ];
        for (k, v) in names {
            kv.push((k.to_owned(), v.unwrap_or(Json::Null)));
        }
        Json::Object(kv)
    };
    let step = st.slice_step();
    let got: Vec<(String, Json)> = vec![
        ("initialized".into(), Json::Bool(g.initialized)),
        ("ref_count".into(), ji(g.ref_count)),
        ("destroyed".into(), Json::Bool(g.destroyed)),
        ("holder".into(), Json::Bool(g.holder.is_some())),
        (
            "services".into(),
            Json::Array(g.services.iter().map(|s| Json::Str(service_name(*s).into())).collect()),
        ),
        ("message_server".into(), Json::Bool(g.message_server)),
        ("update_state".into(), ji(g.update_state)),
        ("start_day".into(), ji(g.start_day)),
        ("iterator".into(), it),
        (
            "specials".into(),
            Json::Array(g.specials.iter().map(|&o| w.name(o)).collect()),
        ),
        ("special_cursor".into(), ji(g.special_cursor as i64)),
        ("growth".into(), items_json(&g.growth)),
        ("decline".into(), items_json(&g.decline)),
        ("last_phantom_sum".into(), ji(g.last_phantom_sum)),
        ("total_capacity".into(), ji(g.total_capacity)),
        ("source_demand".into(), w.opt_name(g.source_demand)),
        ("last_salt".into(), g.last_salt.map_or(Json::Null, ji)),
        ("city_size".into(), jf(g.last_city_size)),
        ("city_size_bits".into(), ji(g.last_city_size.to_bits())),
        ("grids".into(), grids),
        ("rng_lcg".into(), ji(st.rng_state())),
        ("draws".into(), ji(st.draws() as i64)),
        ("slice_step".into(), ji(step.step)),
        ("slice_misses".into(), ji(step.misses)),
        ("developer_type".into(), ji(t.developer_type)),
        ("target_zone".into(), ji(t.target_zone)),
        ("purpose".into(), ji(t.purpose)),
        ("wealth".into(), ji(t.wealth)),
        ("land_value_min".into(), ji(t.land_value_min)),
        ("land_value_max".into(), ji(t.land_value_max)),
        ("use_intrinsic".into(), Json::Bool(t.use_intrinsic_land_value)),
        ("tract_size".into(), ji(t.tract_size)),
        ("baseline".into(), ji(t.baseline)),
        ("growth_threshold".into(), ji(t.growth_threshold)),
        ("decline_threshold".into(), ji(t.decline_threshold)),
        ("phantom_demand".into(), Json::Bool(t.phantom_demand)),
        (
            "caps".into(),
            Json::Array(vec![ji(t.power_cap), ji(t.water_cap), ji(t.fire_cap), ji(t.park_cap)]),
        ),
        ("monopoly_use".into(), ji(t.monopoly_use)),
        ("stage_count".into(), ji(t.stage_curves.len() as i64)),
        (
            "curves".into(),
            Json::Array(
                Curve::ALL
                    .into_iter()
                    .filter(|c| t.curve(*c).is_some())
                    .map(|c| Json::Str(curve_name(c).into()))
                    .collect(),
            ),
        ),
        ("styles".into(), styles_json(&t.styles)),
        (
            "abandonment".into(),
            Json::Object(t.abandonment.iter().map(|(k, v)| (k.to_string(), jf(*v))).collect()),
        ),
        (
            "subjective_min".into(),
            Json::Array(t.subjective_min.iter().map(|&v| ji(v)).collect()),
        ),
        (
            "subjective_max".into(),
            Json::Array(t.subjective_max.iter().map(|&v| ji(v)).collect()),
        ),
    ];
    if let Some(want) = opt(op, "state") {
        check_partial_state(&got, want)?;
    }
    // `curve_eval? [{curve, x, y}]` and `stage_eval? [{stage, x, y}]`.
    if let Some(v) = opt(op, "curve_eval") {
        for e in arr(v, "curve_eval")? {
            let name = str_of(field(e, "curve")?, "curve")?;
            let c = Curve::ALL
                .into_iter()
                .find(|c| curve_name(*c) == name)
                .ok_or_else(|| format!("unknown curve '{name}'"))?;
            let curve = t.curve(c).ok_or_else(|| format!("curve {name} absent"))?;
            let y = curve.eval(f32_of(field(e, "x")?, "x")?);
            if !scalar::matches(&jf(y), field(e, "y")?, false) {
                return Err(format!("curve {name}: got {y}"));
            }
        }
    }
    if let Some(v) = opt(op, "stage_eval") {
        for e in arr(v, "stage_eval")? {
            let i = i64_of(field(e, "stage")?, "stage")? as usize;
            let curve = t
                .stage_curves
                .get(i.wrapping_sub(1))
                .ok_or_else(|| format!("no stage curve {i}"))?;
            let y = curve.eval(f32_of(field(e, "x")?, "x")?);
            if !scalar::matches(&jf(y), field(e, "y")?, false) {
                return Err(format!("stage curve {i}: got {y}"));
            }
            if let Some(b) = opt(e, "bits") {
                if y.to_bits() != u32_of(b, "bits")? {
                    return Err(format!("stage curve {i}: bits 0x{:08X}", y.to_bits()));
                }
            }
        }
    }
    Ok(())
}

/// Like [`check_state`], with the `grids` and `iterator` objects matched
/// partially (only the keys given).
fn check_partial_state(got: &[(String, Json)], want: &Json) -> R<()> {
    let want = want.as_object().ok_or("'state' must be an object")?;
    let mut errors = Vec::new();
    for (k, v) in want {
        let partial = k == "grids" || k == "iterator";
        match got.iter().find(|(gk, _)| gk == k) {
            None => errors.push(format!("unknown state field '{k}'")),
            Some((_, g)) => {
                if !scalar::matches(g, v, partial) {
                    errors.push(format!("{k}: got {}, expected {}", scalar::show(g), scalar::show(v)));
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

/// `gdev_expect_refs`: `{label: count}` (references the developers hold).
fn expect_refs(w: &World, op: &Json) -> R<()> {
    for (k, want) in field(op, "refs")?.as_object().ok_or("refs must be an object")? {
        let t = w.find(k)?;
        let got = w.refs.borrow().get(&t).copied().unwrap_or(0);
        if got != i64_of(want, k)? {
            return Err(format!("{k}: {got} references"));
        }
    }
    Ok(())
}

/// `gdev_expect_demand`: `{label: total}` (the sum of `AddToDemandValue`).
fn expect_demand(w: &World, op: &Json) -> R<()> {
    for (k, want) in field(op, "totals")?.as_object().ok_or("totals must be an object")? {
        let t = w.find(k)?;
        let got = w.demand_added.borrow().get(&t).copied().unwrap_or(0.0);
        if !scalar::matches(&Json::Float(got), want, false) {
            return Err(format!("{k}: total {got}"));
        }
    }
    Ok(())
}

/// `gdev_sort`: `keys` (integers), `order?` (`"desc"`, default, or
/// `"asc"`), `expect` (the original indices in sorted order): the game's
/// `std::sort` on its own.
fn sort(op: &Json) -> R<()> {
    let keys: Vec<i64> = arr(field(op, "keys")?, "keys")?
        .iter()
        .map(|k| i64_of(k, "keys"))
        .collect::<R<_>>()?;
    let mut v: Vec<(i64, usize)> = keys.into_iter().zip(0..).collect();
    let desc = opt(op, "order").map_or(Ok("desc"), |o| str_of(o, "order"))? == "desc";
    stl_sort(&mut v, &mut |a, b| if desc { a.0 > b.0 } else { a.0 < b.0 });
    check_expect(op, Json::Array(v.iter().map(|e| ji(e.1 as i64)).collect()))
}

// ---- persistence ----------------------------------------------------------------------------------

fn slot(op: &Json) -> R<String> {
    Ok(str_of(field(op, "slot")?, "slot")?.to_owned())
}

fn save_now(w: &World, op: &Json) -> R<SavedCity> {
    let d = w.dev(op)?;
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut root = ByteWriter::with_registry(reg.clone());
    root.put_ref(&d.save_ref());
    let mut pkg = Package::new();
    let rep = save_store(&reg, &mut pkg);
    if rep.aborted || rep.entries_discarded != 0 {
        return Err(format!("save failed: {rep:?}"));
    }
    Ok(SavedCity {
        pkg,
        root: root.into_bytes(),
    })
}

/// `gdev_save`: `slot`, `dev?`.
fn save(w: &World, op: &Json) -> R<()> {
    let s = save_now(w, op)?;
    w.saves.borrow_mut().insert(slot(op)?, s);
    Ok(())
}

/// `gdev_load`: `slot`, `label?` (the loaded developer's label, default
/// "dev"), `expect?` (a clean store load, default true). The loaded
/// developer (reference count 0, not initialised) becomes current.
fn load(w: &World, op: &Json) -> R<()> {
    let saved = w.saves.borrow().get(&slot(op)?).cloned().ok_or("no such slot")?;
    let label = opt(op, "label").map_or(Ok("dev"), |v| str_of(v, "label"))?.to_owned();
    let me = w.rc();
    let statics = w.statics();
    let new_label = label.clone();
    let mut factory = |class: u32, _object_id: u32| -> Option<RefValue> {
        match class {
            CLSID_GROWTH_DEVELOPER => {
                let t = me.token(&new_label);
                Some(RefValue::Object(GrowthDeveloper::new(
                    me.clone() as Rc<dyn GrowthDeveloperHost>,
                    statics.clone(),
                    t,
                ) as Rc<dyn SaveObject>))
            }
            c => create_for_clsid(c).map(RefValue::Object),
        }
    };
    let mut reg = LoadRegistry::new(BTreeMap::new(), Box::new(&mut factory));
    let report = load_store(&saved.pkg, &mut reg);
    let ok = report.index_ok && report.failed_classes.is_empty();
    let mut r = ByteReader::with_resolver(&saved.root, &mut reg);
    let root = fc4_core::stream::InStream::get_ref(&mut r);
    let want = opt(op, "expect").map_or(Ok(true), |v| bool_of(v, "expect"))?;
    if ok != want {
        return Err(format!("store load clean = {ok}, expected {want}"));
    }
    if let Some(d) = root.as_ref().and_then(GrowthDeveloper::from_save_ref) {
        w.devs.borrow_mut().insert(label.clone(), d);
        *w.current.borrow_mut() = label;
    } else if want {
        return Err("the root did not resolve to a growth developer".into());
    }
    Ok(())
}

/// `gdev_expect_salts`: `devs` (labels), `salts` (a list): the developers'
/// last salts, as a multiset, are contained in `salts` (each listed salt
/// used at most once). The [compat-limit] check: which industrial developer
/// got which salt is not pinned, only that the salts handed out are the
/// drawn ones.
fn expect_salts(w: &World, op: &Json) -> R<()> {
    let mut pool = Vec::new();
    for v in arr(field(op, "salts")?, "salts")? {
        pool.push(u32_of(v, "salts")?);
    }
    for v in arr(field(op, "devs")?, "devs")? {
        let label = str_of(v, "devs")?;
        let d = w
            .devs
            .borrow()
            .get(label)
            .cloned()
            .ok_or_else(|| format!("no developer '{label}'"))?;
        let salt = d
            .data()
            .last_salt
            .ok_or_else(|| format!("developer '{label}' has drawn no salt"))?;
        let i = pool
            .iter()
            .position(|&s| s == salt)
            .ok_or_else(|| format!("developer '{label}' salt {salt:#010X} is not among the remaining {pool:X?}"))?;
        pool.swap_remove(i);
    }
    Ok(())
}

fn expect_save(w: &World, op: &Json) -> R<()> {
    let want = w.saves.borrow().get(&slot(op)?).cloned().ok_or("no such slot")?;
    let got = save_now(w, op)?;
    scalar::same_save(
        &scalar::Saved {
            pkg: got.pkg,
            root: got.root,
        },
        &scalar::Saved {
            pkg: want.pkg,
            root: want.root,
        },
    )
}

fn patch(w: &World, op: &Json) -> R<()> {
    let mut saves = w.saves.borrow_mut();
    let s = saves.get_mut(&slot(op)?).ok_or("no such slot")?;
    let mut tmp = scalar::Saved {
        pkg: std::mem::take(&mut s.pkg),
        root: Vec::new(),
    };
    let r = scalar::patch(&mut tmp, CLSID_GROWTH_DEVELOPER, op);
    s.pkg = tmp.pkg;
    r
}

/// `gdev_expect_payload`: `slot`, `length?`, `hex?`, `at? [{offset, hex}]`.
fn expect_payload(w: &World, op: &Json) -> R<()> {
    let saves = w.saves.borrow();
    let s = saves.get(&slot(op)?).ok_or("no such slot")?;
    let tmp = scalar::Saved {
        pkg: s.pkg.clone(),
        root: Vec::new(),
    };
    let got = scalar::entry_payload(&tmp, CLSID_GROWTH_DEVELOPER, 0)?;
    if let Some(h) = opt(op, "hex") {
        let want = hex_decode(str_of(h, "hex")?)?;
        if got != want {
            return Err(format!("payload differs:\n    got      {}", hex_encode(&got)));
        }
    }
    if let Some(v) = opt(op, "at") {
        for e in arr(v, "at")? {
            let off = i64_of(field(e, "offset")?, "offset")? as usize;
            let want = hex_decode(str_of(field(e, "hex")?, "hex")?)?;
            let slice = got.get(off..off + want.len()).ok_or("slice outside the payload")?;
            if slice != want.as_slice() {
                return Err(format!(
                    "payload at {off}: got {}, expected {}",
                    hex_encode(slice),
                    hex_encode(&want)
                ));
            }
        }
    }
    if let Some(v) = opt(op, "length") {
        if got.len() as i64 != i64_of(v, "length")? {
            return Err(format!("payload length {}", got.len()));
        }
    }
    Ok(())
}

/// `gdev_read`: `hex` (a record with null references only), `expect?`
/// (the `Read` result), `dev?`.
fn read(w: &World, op: &Json) -> R<()> {
    let d = w.dev(op)?;
    let bytes = hex_decode(str_of(field(op, "hex")?, "hex")?)?;
    let mut r = ByteReader::new(&bytes);
    let ok = SaveObject::read(&*d, &mut r);
    check_expect(op, Json::Bool(ok))
}
