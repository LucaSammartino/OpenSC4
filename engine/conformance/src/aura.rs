// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/aura.md`: a scripted city (lots,
//! occupants, budget, ordinances, the grids of other simulators), a
//! scriptable random generator, and ops that drive the aura simulator. See
//! README.md ("Aura ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::rc::{Rc, Weak};

use fc4_core::aura::{
    self, query_interface, shipped_tuning, AuraHost, AuraInterface, AuraMessage, AuraRandom, AuraRng, AuraSim, Hotspot,
    IteratedLot, LotInfo, LotIterator, SliceStep, Strike, CLSID_AURA_SIMULATOR, SLOT_TABLE,
};
use fc4_core::grid::{create_for_clsid, CityDims, GridElem, GridProvider, SimGrid, TractGrid};
use fc4_core::property::PropertyHolder;
use fc4_core::rng::FixedSeeds;
use fc4_core::stream::{RefValue, SaveObject};
use fc4_core::variant::Elem;

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, check_state, f32_of, i64_of, jf, ji, num, opt, show, str_of, u32_of,
    EventLog, Saved, R,
};

// ---- the scripted generator -------------------------------------------------------------------------

struct Stub {
    world: Weak<World>,
    uniform: VecDeque<f64>,
    ints: VecDeque<u32>,
    last: (f64, u32),
}

impl AuraRandom for Stub {
    fn range_f64(&mut self, a: f64, b: f64) -> f64 {
        let u = self.uniform.pop_front().unwrap_or(self.last.0);
        self.last.0 = u;
        let v = a + u * (b - a);
        if let Some(w) = self.world.upgrade() {
            w.events.push(
                "rng",
                vec![("kind", Json::Str("double".into())), ("value", Json::Float(v))],
            );
        }
        v
    }
    fn uniform_u32(&mut self, n: u32) -> u32 {
        let i = self.ints.pop_front().unwrap_or(self.last.1);
        self.last.1 = i;
        if let Some(w) = self.world.upgrade() {
            w.events.push(
                "rng",
                vec![("kind", Json::Str("int".into())), ("n", ji(n)), ("value", ji(i))],
            );
        }
        i
    }
}

/// A real generator that logs its draws.
struct Logged {
    world: Weak<World>,
    rng: AuraRng,
}

impl AuraRandom for Logged {
    fn range_f64(&mut self, a: f64, b: f64) -> f64 {
        let v = self.rng.range_f64(a, b);
        if let Some(w) = self.world.upgrade() {
            w.events.push(
                "rng",
                vec![("kind", Json::Str("double".into())), ("value", Json::Float(v))],
            );
        }
        v
    }
    fn uniform_u32(&mut self, n: u32) -> u32 {
        let v = self.rng.uniform_u32(n);
        if let Some(w) = self.world.upgrade() {
            w.events.push(
                "rng",
                vec![("kind", Json::Str("int".into())), ("n", ji(n)), ("value", ji(v))],
            );
        }
        v
    }
}

// ---- the world --------------------------------------------------------------------------------------

#[derive(Clone, Default)]
struct Occupant {
    kind: u32,
    pos: [f32; 3],
    props: BTreeMap<u32, Vec<i32>>,
    dept: Option<u32>,
}

/// The grids of other simulators.
#[derive(Default)]
struct Sources {
    pollution: [Option<Rc<SimGrid<i16>>>; 4],
    school: Option<Rc<SimGrid<f32>>>,
    hospital: Option<Rc<SimGrid<f32>>>,
    crime: Option<Rc<SimGrid<u8>>>,
    trip: Option<Rc<SimGrid<u8>>>,
    congestion: Option<Rc<SimGrid<u8>>>,
    pop: [Option<Rc<SimGrid<u16>>>; 3],
}

#[derive(Default)]
pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    sim: RefCell<Option<Rc<AuraSim>>>,
    next_token: Cell<u64>,
    saves: RefCell<BTreeMap<String, Saved>>,
    city: Cell<Option<CityDims>>,
    date: Cell<u32>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    lots: RefCell<Vec<IteratedLot>>,
    occupants: RefCell<BTreeMap<u64, Occupant>>,
    funding: RefCell<BTreeMap<u32, f32>>,
    tax: RefCell<[f32; 12]>,
    offset: Cell<i32>,
    land_value_type: Cell<i32>,
    timer_expired: Cell<bool>,
    /// Clock reads that answer "not expired" after each timer start before
    /// the timer expires (`None`: `timer_expired` answers every read).
    timer_budget: Cell<Option<u32>>,
    /// Clock reads since the last timer start.
    timer_reads: Cell<u32>,
    /// Log every clock read as a `timer_read` event.
    log_timer: Cell<bool>,
    /// Lots taken from the current lot iterator.
    lots_taken: Cell<u32>,
    registry_ok: Cell<bool>,
    sources: RefCell<Sources>,
    seed: Cell<u32>,
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            next_token: Cell::new(0x100),
            date: Cell::new(1000),
            tuning: RefCell::new(Some(Rc::new(shipped_tuning()))),
            land_value_type: Cell::new(1),
            timer_expired: Cell::new(false),
            registry_ok: Cell::new(true),
            seed: Cell::new(1),
            ..Default::default()
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn rc(&self) -> Rc<World> {
        self.me.borrow().upgrade().expect("world alive")
    }

    fn city(&self) -> CityDims {
        self.city.get().unwrap_or(CityDims::square(64))
    }

    fn sim(&self) -> R<Rc<AuraSim>> {
        self.sim
            .borrow()
            .clone()
            .ok_or_else(|| "no aura simulator (aura_create)".into())
    }

    fn token(&self) -> u64 {
        let t = self.next_token.get();
        self.next_token.set(t + 1);
        t
    }

    fn log(&self, kind: &str, fields: Vec<(&str, Json)>) {
        self.events.push(kind, fields);
    }

    fn rng(&self, spec: Option<&Json>) -> R<Box<dyn AuraRandom>> {
        let seeds = FixedSeeds {
            wall_clock: self.seed.get(),
            process_clock: 1,
        };
        let me = self.me.borrow().clone();
        let Some(spec) = spec else {
            return Ok(Box::new(Logged {
                world: me,
                rng: AuraRng::new(&seeds),
            }));
        };
        if let Some(s) = opt(spec, "seed") {
            return Ok(Box::new(Logged {
                world: me,
                rng: AuraRng::with_seed(u32_of(s, "seed")?, &seeds),
            }));
        }
        let stub = field(spec, "stub")?;
        let uniform = opt(stub, "uniform").map_or(Ok(VecDeque::new()), |v| {
            arr(v, "uniform")?
                .iter()
                .map(|x| num(x).ok_or_else(|| "uniform: number".to_string()))
                .collect()
        })?;
        let ints = opt(stub, "int").map_or(Ok(VecDeque::new()), |v| {
            arr(v, "int")?.iter().map(|x| u32_of(x, "int")).collect()
        })?;
        Ok(Box::new(Stub {
            world: me,
            uniform,
            ints,
            last: (0.5, 0),
        }))
    }
}

impl GridProvider<i8> for World {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<i8>>> {
        self.log(
            "create_grid",
            vec![
                ("type", Json::Str("i8".into())),
                ("instance", ji(instance)),
                ("size", ji(tract_size)),
            ],
        );
        Some(SimGrid::<i8>::create(&self.city(), instance, tract_size) as Rc<dyn TractGrid<i8>>)
    }
}

impl GridProvider<i16> for World {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<i16>>> {
        self.log(
            "create_grid",
            vec![
                ("type", Json::Str("i16".into())),
                ("instance", ji(instance)),
                ("size", ji(tract_size)),
            ],
        );
        Some(SimGrid::<i16>::create(&self.city(), instance, tract_size) as Rc<dyn TractGrid<i16>>)
    }
}

/// A snapshot of the world's lots at the time the iterator was taken.
struct Lots(std::vec::IntoIter<IteratedLot>, Weak<World>);

impl LotIterator for Lots {
    fn next_lot(&mut self) -> Option<IteratedLot> {
        let l = self.0.next();
        if let (Some(_), Some(w)) = (&l, self.1.upgrade()) {
            w.lots_taken.set(w.lots_taken.get() + 1);
        }
        l
    }
}

fn dyn_grid<T: GridElem>(g: &Option<Rc<SimGrid<T>>>) -> Option<Rc<dyn TractGrid<T>>> {
    g.clone().map(|g| g as Rc<dyn TractGrid<T>>)
}

impl AuraHost for World {
    fn cell_width(&self) -> (f32, f32) {
        let c = self.city();
        (c.cell_width_x, c.cell_width_z)
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        let c = self.city();
        ((x / c.cell_width_x).floor() as i32, (z / c.cell_width_z).floor() as i32)
    }
    fn sim_date_number(&self) -> u32 {
        self.date.get()
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn publish_base_aura(&self, base: [i32; 3]) {
        self.log(
            "base_aura",
            vec![("values", Json::Array(base.iter().map(|&v| ji(v)).collect()))],
        );
    }
    fn lot_count(&self) -> u32 {
        self.lots.borrow().len() as u32
    }
    fn lot_iterator(&self) -> Box<dyn LotIterator> {
        self.log("lot_iterator", vec![]);
        self.lots_taken.set(0);
        Box::new(Lots(self.lots.borrow().clone().into_iter(), self.me.borrow().clone()))
    }
    fn lot_at(&self, x: i32, z: i32) -> Option<LotInfo> {
        self.lots
            .borrow()
            .iter()
            .map(|l| l.info)
            .find(|r| (r.x1..=r.x2).contains(&x) && (r.z1..=r.z2).contains(&z))
    }
    fn land_value_type(&self, _x: i32, _z: i32) -> i32 {
        self.land_value_type.get()
    }
    fn occupant_has_property(&self, occ: u64, prop: u32) -> bool {
        self.occupants
            .borrow()
            .get(&occ)
            .is_some_and(|o| o.props.contains_key(&prop))
    }
    fn occupant_sint32s(&self, occ: u64, prop: u32) -> Option<Vec<i32>> {
        self.occupants.borrow().get(&occ)?.props.get(&prop).cloned()
    }
    fn occupant_type(&self, occ: u64) -> u32 {
        self.occupants.borrow().get(&occ).map_or(0, |o| o.kind)
    }
    fn occupant_position(&self, occ: u64) -> [f32; 3] {
        self.occupants.borrow().get(&occ).map_or([0.0; 3], |o| o.pos)
    }
    fn budget_item_department(&self, occ: u64, _purpose: u32) -> Option<u32> {
        self.occupants.borrow().get(&occ)?.dept
    }
    fn department_funding(&self, dept: u32) -> Option<f32> {
        self.funding.borrow().get(&dept).copied()
    }
    fn tax_rate(&self, index: i32) -> f32 {
        self.tax.borrow().get(index as usize).copied().unwrap_or(0.0)
    }
    fn cumulative_offset(&self, _prop: u32, _index: u32) -> i32 {
        self.offset.get()
    }
    fn pollution_grid(&self, kind: i32) -> Option<Rc<dyn TractGrid<i16>>> {
        dyn_grid(self.sources.borrow().pollution.get(kind as usize)?)
    }
    fn school_grade_map(&self) -> Option<Rc<dyn TractGrid<f32>>> {
        dyn_grid(&self.sources.borrow().school)
    }
    fn hospital_grade_map(&self) -> Option<Rc<dyn TractGrid<f32>>> {
        dyn_grid(&self.sources.borrow().hospital)
    }
    fn crime_grid(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        dyn_grid(&self.sources.borrow().crime)
    }
    fn trip_length_map(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        dyn_grid(&self.sources.borrow().trip)
    }
    fn congestion_map(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        dyn_grid(&self.sources.borrow().congestion)
    }
    fn population_grids(&self) -> Option<[Rc<dyn TractGrid<u16>>; 3]> {
        let s = self.sources.borrow();
        Some([dyn_grid(&s.pop[0])?, dyn_grid(&s.pop[1])?, dyn_grid(&s.pop[2])?])
    }
    fn record_history(&self, series: u32, value: i32) {
        self.log("history", vec![("series", ji(series)), ("value", ji(value))]);
    }
    fn attempt_disaster_start(&self, manager: u32, x: i32, z: i32) {
        self.log("disaster", vec![("manager", ji(manager)), ("x", ji(x)), ("z", ji(z))]);
    }
    fn end_strike(&self, strike: Strike) {
        let name = match strike {
            Strike::Police => "police",
            Strike::Fire => "fire",
            Strike::Health => "health",
            Strike::School => "school",
        };
        self.log("strike", vec![("service", Json::Str(name.into()))]);
    }
    fn post(&self, msg_type: u32) {
        self.log("post", vec![("type", ji(msg_type))]);
    }
    fn limit_timer_start(&self, ms: u32) {
        self.timer_reads.set(0);
        self.log("timer_start", vec![("ms", ji(ms))]);
    }
    fn limit_timer_expired(&self) -> bool {
        let n = self.timer_reads.get();
        self.timer_reads.set(n + 1);
        if self.log_timer.get() {
            self.log("timer_read", vec![("lots_taken", ji(self.lots_taken.get()))]);
        }
        match self.timer_budget.get() {
            Some(b) => n >= b,
            None => self.timer_expired.get(),
        }
    }
    fn subscribe(&self, msgs: &[u32]) {
        self.log(
            "subscribe",
            vec![("types", Json::Array(msgs.iter().map(|&m| ji(m)).collect()))],
        );
    }
    fn unsubscribe(&self, msgs: &[u32]) {
        self.log(
            "unsubscribe",
            vec![("types", Json::Array(msgs.iter().map(|&m| ji(m)).collect()))],
        );
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
    fn registry_add(&self) -> bool {
        self.log("registry_add", vec![]);
        self.registry_ok.get()
    }
    fn registry_remove(&self) {
        self.log("registry_remove", vec![]);
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "aura", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("aura_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "create" => (|| {
            if let Some(s) = opt(op, "seed") {
                w.seed.set(u32_of(s, "seed")?);
            }
            let rng = w.rng(opt(op, "rng"))?;
            *w.sim.borrow_mut() = Some(AuraSim::new(w.rc() as Rc<dyn AuraHost>, w.token(), rng));
            Ok(())
        })(),
        "rng" => w.sim().and_then(|s| {
            s.set_random(w.rng(Some(op))?);
            Ok(())
        }),
        "init" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.init()))),
        "shutdown" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.shutdown()))),
        "message" => message(&w, op),
        "idle" => idle(&w, op),
        "run" => run(&w, op),
        "call" => call(&w, op),
        "set" => set(&w, op),
        "expect" => expect(&w, op),
        "set_grid" => set_grid(&w, op),
        "expect_grid" => expect_grid(&w, op),
        "expect_events" => w.events.expect_events(op),
        "expect_event_count" => w.events.expect_count(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }
        "save" => save(&w, op),
        "load" => load(&w, op),
        "expect_save" => expect_save(&w, op),
        "patch" => (|| {
            let mut saves = w.saves.borrow_mut();
            let s = saves.get_mut(&slot(op)?).ok_or("no such slot")?;
            scalar::patch(s, CLSID_AURA_SIMULATOR, op)
        })(),
        "expect_payload" => expect_payload(&w, op),
        _ => Err(format!("unknown op '{name}'")),
    })
}

// ---- environment ------------------------------------------------------------------------------------

fn pair_i(v: &Json, k: &str) -> R<(i32, i32)> {
    let a = arr(v, k)?;
    if a.len() != 2 {
        return Err(format!("'{k}' must have 2 elements"));
    }
    Ok((i64_of(&a[0], k)? as i32, i64_of(&a[1], k)? as i32))
}

fn lot_of(j: &Json) -> R<IteratedLot> {
    let r = arr(field(j, "rect")?, "rect")?;
    if r.len() != 4 {
        return Err("'rect' must be [x1, z1, x2, z2]".into());
    }
    let c = |i: usize| i64_of(&r[i], "rect").map(|v| v as i32);
    let building = match j.get("building") {
        None => Some(0),
        Some(Json::Null) => None,
        Some(v) => Some(u32_of(v, "building")? as u64),
    };
    let mut populations = Vec::new();
    if let Some(p) = opt(j, "pops") {
        for e in arr(p, "pops")? {
            let e = arr(e, "pops")?;
            populations.push((u32_of(&e[0], "type")?, i64_of(&e[1], "count")? as i32));
        }
    }
    Ok(IteratedLot {
        info: LotInfo {
            x1: c(0)?,
            z1: c(1)?,
            x2: c(2)?,
            z2: c(3)?,
            zone_type: opt(j, "zone").map_or(Ok(1), |v| u32_of(v, "zone"))?,
        },
        building,
        occupant_wealth: opt(j, "wealth").map_or(Ok(1), |v| i64_of(v, "wealth"))? as i32,
        populations,
    })
}

fn occupant_of(j: &Json) -> R<Occupant> {
    let mut o = Occupant {
        kind: opt(j, "type").map_or(Ok(0), |v| u32_of(v, "type"))?,
        dept: opt(j, "dept").map(|v| u32_of(v, "dept")).transpose()?,
        ..Default::default()
    };
    if let Some(p) = opt(j, "pos") {
        let a = arr(p, "pos")?;
        for (i, v) in a.iter().take(3).enumerate() {
            o.pos[i] = f32_of(v, "pos")?;
        }
    }
    if let Some(p) = opt(j, "props") {
        for (k, v) in p.as_object().ok_or("props must be an object")? {
            let id = Json::Str(k.clone()).as_i64().ok_or("bad property id")? as u32;
            let vals = arr(v, k)?
                .iter()
                .map(|x| i64_of(x, k).map(|i| i as i32))
                .collect::<R<Vec<i32>>>()?;
            o.props.insert(id, vals);
        }
    }
    Ok(o)
}

/// A core grid of the world's city filled from `{size?, fill?, cells?}`.
fn source<T: GridElem>(city: &CityDims, j: &Json) -> R<Option<Rc<SimGrid<T>>>> {
    if matches!(j, Json::Null) {
        return Ok(None);
    }
    let size = opt(j, "size").map_or(Ok(4), |v| i64_of(v, "size"))? as i32;
    let g = SimGrid::<T>::create(city, 0, size);
    if let Some(v) = opt(j, "fill") {
        g.fill(value_of::<T>(v)?);
    }
    write_cells(&*g, j)?;
    Ok(Some(g))
}

fn value_of<T: GridElem>(j: &Json) -> R<T> {
    num(j)
        .map(T::from_f64)
        .ok_or_else(|| format!("bad grid value {}", show(j)))
}

fn write_cells<T: GridElem>(g: &dyn TractGrid<T>, j: &Json) -> R<()> {
    if let Some(c) = opt(j, "cells") {
        for e in arr(c, "cells")? {
            let e = arr(e, "cells")?;
            if e.len() != 3 {
                return Err("a cell is [x, z, v]".into());
            }
            g.set(
                i64_of(&e[0], "x")? as i32,
                i64_of(&e[1], "z")? as i32,
                value_of::<T>(&e[2])?,
            );
        }
    }
    Ok(())
}

fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "cells") {
        let (x, z) = pair_i(v, "cells")?;
        let mut c = w.city();
        c.cells_x = x;
        c.cells_z = z;
        w.city.set(Some(c));
    }
    if let Some(v) = opt(op, "cell_width") {
        let a = arr(v, "cell_width")?;
        let mut c = w.city();
        c.cell_width_x = f32_of(&a[0], "cell_width")?;
        c.cell_width_z = f32_of(a.get(1).unwrap_or(&a[0]), "cell_width")?;
        w.city.set(Some(c));
    }
    if let Some(v) = opt(op, "date") {
        w.date.set(u32_of(v, "date")?);
    }
    if let Some(v) = op.get("tuning") {
        *w.tuning.borrow_mut() = match v {
            Json::Null => None,
            Json::Str(s) if s == "shipped" => Some(Rc::new(shipped_tuning())),
            Json::Str(s) if s == "empty" => Some(Rc::new(PropertyHolder::new())),
            other => {
                return Err(format!(
                    "tuning must be \"shipped\", \"empty\" or null, not {}",
                    show(other)
                ))
            }
        };
    }
    if let Some(p) = opt(op, "tuning_props") {
        let extra = scalar::holder_of(p)?;
        let mut h = w.tuning.borrow().as_deref().cloned().unwrap_or_default();
        for id in extra.ids() {
            if let Some(v) = extra.get(id) {
                h.add(id, v.clone());
            }
        }
        *w.tuning.borrow_mut() = Some(Rc::new(h));
    }
    if let Some(r) = opt(op, "tuning_remove") {
        let mut h = w.tuning.borrow().as_deref().cloned().unwrap_or_default();
        for id in arr(r, "tuning_remove")? {
            h.remove(u32_of(id, "tuning_remove")?);
        }
        *w.tuning.borrow_mut() = Some(Rc::new(h));
    }
    if let Some(v) = opt(op, "funding") {
        for (k, f) in v.as_object().ok_or("funding must be an object")? {
            let id = Json::Str(k.clone()).as_i64().ok_or("bad department id")? as u32;
            match f {
                Json::Null => {
                    w.funding.borrow_mut().remove(&id);
                }
                Json::Str(s) if s == "nan" => {
                    w.funding.borrow_mut().insert(id, f32::NAN);
                }
                f => {
                    w.funding.borrow_mut().insert(id, f32_of(f, "funding")?);
                }
            }
        }
    }
    if let Some(v) = opt(op, "tax") {
        for (k, r) in v.as_object().ok_or("tax must be an object {index: rate}")? {
            let i = k.parse::<usize>().map_err(|_| "bad tax index")?;
            if let Some(slot) = w.tax.borrow_mut().get_mut(i) {
                *slot = f32_of(r, "tax")?;
            }
        }
    }
    if let Some(v) = opt(op, "ordinance_offset") {
        w.offset.set(i64_of(v, "ordinance_offset")? as i32);
    }
    if let Some(v) = opt(op, "land_value_type") {
        w.land_value_type.set(i64_of(v, "land_value_type")? as i32);
    }
    if let Some(v) = opt(op, "timer_expired") {
        w.timer_expired.set(bool_of(v, "timer_expired")?);
    }
    if let Some(v) = op.get("timer_reads_before_expiry") {
        w.timer_budget.set(match v {
            Json::Null => None,
            v => Some(u32_of(v, "timer_reads_before_expiry")?),
        });
    }
    if let Some(v) = opt(op, "log_timer_reads") {
        w.log_timer.set(bool_of(v, "log_timer_reads")?);
    }
    if let Some(v) = opt(op, "registry_ok") {
        w.registry_ok.set(bool_of(v, "registry_ok")?);
    }
    if let Some(v) = op.get("lots") {
        let lots = match v {
            Json::Null => Vec::new(),
            v => arr(v, "lots")?.iter().map(lot_of).collect::<R<Vec<_>>>()?,
        };
        *w.lots.borrow_mut() = lots;
    }
    if let Some(v) = opt(op, "occupants") {
        for (k, o) in v.as_object().ok_or("occupants must be an object")? {
            let id = Json::Str(k.clone()).as_i64().ok_or("bad occupant id")? as u64;
            match o {
                Json::Null => {
                    w.occupants.borrow_mut().remove(&id);
                }
                o => {
                    w.occupants.borrow_mut().insert(id, occupant_of(o)?);
                }
            }
        }
    }
    if let Some(v) = opt(op, "grids") {
        let city = w.city();
        let mut s = w.sources.borrow_mut();
        for (k, g) in v.as_object().ok_or("grids must be an object")? {
            match k.as_str() {
                "crime" => s.crime = source(&city, g)?,
                "trip" => s.trip = source(&city, g)?,
                "congestion" => s.congestion = source(&city, g)?,
                "school" => s.school = source(&city, g)?,
                "hospital" => s.hospital = source(&city, g)?,
                "air" => s.pollution[0] = source(&city, g)?,
                "water" => s.pollution[1] = source(&city, g)?,
                "garbage" => s.pollution[2] = source(&city, g)?,
                "radiation" => s.pollution[3] = source(&city, g)?,
                "pop1" => s.pop[0] = source(&city, g)?,
                "pop2" => s.pop[1] = source(&city, g)?,
                "pop3" => s.pop[2] = source(&city, g)?,
                other => return Err(format!("unknown source grid '{other}'")),
            }
        }
    }
    Ok(())
}

// ---- calls ------------------------------------------------------------------------------------------

fn msg_type(j: &Json) -> R<u32> {
    Ok(match j.as_str() {
        Some("SimBegin") => aura::msg::SIM_BEGIN,
        Some("SimNewDay") => aura::msg::SIM_NEW_DAY,
        Some("SimNewMonth") => aura::msg::SIM_NEW_MONTH,
        Some("SimIdle") => aura::msg::SIM_IDLE,
        Some("PostCityInit") => aura::msg::POST_CITY_INIT,
        Some("OccupantInserted") => aura::msg::OCCUPANT_INSERTED,
        Some("OccupantRemoved") => aura::msg::OCCUPANT_REMOVED,
        Some("OrdinanceUpdated") => aura::msg::ORDINANCE_UPDATED,
        Some("ExemplarChanged") => aura::msg::EXEMPLAR_CHANGED,
        _ => u32_of(j, "type")?,
    })
}

fn message(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let d = |k: &str| opt(op, k).map_or(Ok(0), |v| u32_of(v, k));
    let m = AuraMessage {
        msg_type: msg_type(field(op, "type")?)?,
        data1: d("data1")?,
        data2: d("data2")?,
        data3: d("data3")?,
        void1: opt(op, "occupant").map_or(Ok(0), |v| u32_of(v, "occupant"))? as u64,
    };
    check_expect(op, Json::Bool(s.do_message(&m)))
}

/// `aura_idle`: `count` × `DoIdle(ms)`, then `expect_state`.
fn idle(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let ms = opt(op, "ms").map_or(Ok(5), |v| u32_of(v, "ms"))?;
    let count = opt(op, "count").map_or(Ok(1), |v| i64_of(v, "count"))?;
    for _ in 0..count {
        s.do_idle(ms);
    }
    if let Some(e) = opt(op, "expect_state") {
        let got = s.data().state;
        if i64::from(got) != i64_of(e, "expect_state")? {
            return Err(format!("state {got}, expected {}", show(e)));
        }
    }
    Ok(())
}

/// `aura_run`: `DoIdle(ms)` until the state is 0 (at most `max` calls).
fn run(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let ms = opt(op, "ms").map_or(Ok(5), |v| u32_of(v, "ms"))?;
    let max = opt(op, "max").map_or(Ok(1000), |v| i64_of(v, "max"))?;
    let mut n = 0i64;
    while s.data().state != 0 {
        if n >= max {
            return Err(format!("no end of cycle after {max} calls"));
        }
        s.do_idle(ms);
        n += 1;
    }
    if let Some(e) = opt(op, "expect_calls") {
        if n != i64_of(e, "expect_calls")? {
            return Err(format!("{n} calls, expected {}", show(e)));
        }
    }
    Ok(())
}

fn a_f32(op: &Json, i: usize) -> R<f32> {
    f32_of(arg(op, i)?, "argument")
}

fn a_i32(op: &Json, i: usize) -> R<i32> {
    Ok(i64_of(arg(op, i)?, "argument")? as i32)
}

fn a_u32(op: &Json, i: usize) -> R<u32> {
    u32_of(arg(op, i)?, "argument")
}

fn elem_of(j: &Json) -> R<Elem> {
    Ok(match j {
        Json::Float(f) => Elem::F32(*f as f32),
        j => Elem::I64(i64_of(j, "parameter")?),
    })
}

fn call(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let m = str_of(field(op, "method")?, "method")?;
    let got = match m {
        "add_transient_effect" => {
            s.add_transient_effect(a_f32(op, 0)?, a_f32(op, 1)?, a_f32(op, 2)?, a_f32(op, 3)?);
            Json::Null
        }
        "add_transient_effect_type" => {
            s.add_transient_effect_type(a_f32(op, 0)?, a_f32(op, 1)?, a_u32(op, 2)?);
            Json::Null
        }
        "aura_value" => ji(s.aura_value(a_i32(op, 0)?, a_i32(op, 1)?)),
        "mayor_rating" => ji(s.mayor_rating()),
        "max_mayor_rating" | "min_mayor_rating" => {
            let (r, (x, z)) = if m == "max_mayor_rating" {
                s.max_mayor_rating()
            } else {
                s.min_mayor_rating()
            };
            Json::Array(vec![ji(r), ji(x), ji(z)])
        }
        "bust_strike" => {
            s.bust_strike(a_u32(op, 0)?);
            Json::Null
        }
        "registry" => {
            let name = str_of(field(op, "command")?, "command")?;
            let params = opt(op, "args").map_or(Ok(Vec::new()), |a| {
                arr(a, "args")?.iter().map(elem_of).collect::<R<Vec<_>>>()
            })?;
            Json::Bool(match name {
                aura::REGISTRY_ADD_TRANSIENT_EFFECT => s.registry_add_transient_effect(&params),
                aura::REGISTRY_END_STRIKE => s.registry_end_strike(&params),
                other => return Err(format!("unknown registry command '{other}'")),
            })
        }
        "occupant_changed" => {
            s.occupant_changed(a_u32(op, 0)? as u64, bool_of(arg(op, 1)?, "add")?);
            Json::Null
        }
        "centre_cell" => {
            let (x, z) = s.centre_cell(a_f32(op, 0)?, a_f32(op, 1)?);
            Json::Array(vec![ji(x), ji(z)])
        }
        "lot_base_aura" => ji(s.lot_base_aura(&lot_of(arg(op, 0)?)?)),
        "tax_change" => {
            s.tax_change();
            Json::Null
        }
        "read_tuning" => {
            s.read_tuning();
            Json::Null
        }
        "query_interface" => match query_interface(a_u32(op, 0)?) {
            Some(AuraInterface::Primary) => Json::Str("primary".into()),
            Some(AuraInterface::MessageTarget) => Json::Str("message_target".into()),
            Some(AuraInterface::Serializable) => Json::Str("serializable".into()),
            None => Json::Bool(false),
        },
        "class_id" => ji(s.class_id()),
        "slot_pops" => {
            let slot = a_u32(op, 0)?;
            let e = SLOT_TABLE
                .iter()
                .find(|e| e.0 == slot)
                .ok_or_else(|| format!("no slot {slot}"))?;
            ji(e.2)
        }
        other => return Err(format!("unknown aura method '{other}'")),
    };
    check_expect(op, got)
}

// ---- state ------------------------------------------------------------------------------------------

fn i32s(v: &Json, k: &str) -> R<Vec<i32>> {
    arr(v, k)?.iter().map(|x| i64_of(x, k).map(|i| i as i32)).collect()
}

fn triples(v: &Json, k: &str) -> R<Vec<[i32; 3]>> {
    arr(v, k)?
        .iter()
        .map(|e| {
            let t = i32s(e, k)?;
            if t.len() != 3 {
                return Err(format!("'{k}' entries are [x, z, w]"));
            }
            Ok([t[0], t[1], t[2]])
        })
        .collect()
}

fn three(v: &Json, k: &str) -> R<[i32; 3]> {
    let t = i32s(v, k)?;
    t.try_into().map_err(|_| format!("'{k}' must have 3 values"))
}

/// `aura_set`: pokes state fields (test setup).
fn set(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let state = field(op, "state")?.as_object().ok_or("state must be an object")?;
    for (k, v) in state {
        if k == "slice_step" {
            let t = i32s(v, k)?;
            s.set_slice_step(SliceStep {
                step: t.first().copied().unwrap_or(1024) as u32,
                misses: t.get(1).copied().unwrap_or(0),
            });
            continue;
        }
        let mut d = s.data_mut();
        let int = || i64_of(v, k).map(|i| i as i32);
        let fl = || f32_of(v, k);
        match k.as_str() {
            "state" => d.state = int()?,
            "cycle_start_day" => d.cycle_start_day = u32_of(v, k)?,
            "department_cursor" => {
                d.department_cursor = match v {
                    Json::Null => None,
                    v => Some(u32_of(v, k)?),
                }
            }
            "mayor_rating" => d.mayor_rating = int()?,
            "unused_rating" => d.unused_rating = int()?,
            "global_effect" => d.global_effect = int()?,
            "effective_base_aura" => d.effective_base_aura = three(v, k)?,
            "base_aura" => d.tuning.base_aura = three(v, k)?,
            "hotspot" => d.tuning.hotspot = three(v, k)?,
            "long_decay" => d.tuning.long_decay = fl()?,
            "short_decay" => d.tuning.short_decay = fl()?,
            "max_rating" => d.max_rating = int()? as i16,
            "min_rating" => d.min_rating = int()? as i16,
            "max_location" => {
                let t = i32s(v, k)?;
                d.max_location = (t[0], t[1]);
            }
            "min_location" => {
                let t = i32s(v, k)?;
                d.min_location = (t[0], t[1]);
            }
            "low_tracts" => d.low_tracts = triples(v, k)?.into_iter().map(|t| (t[0], t[1], t[2])).collect(),
            "hotspots" => {
                d.hotspots = triples(v, k)?
                    .into_iter()
                    .map(|t| Hotspot {
                        x: t[0],
                        z: t[1],
                        weight: t[2],
                    })
                    .collect()
            }
            "re_add_landmarks" => d.re_add_landmarks = bool_of(v, k)?,
            "previous_tax_rate" => {
                let a = arr(v, k)?;
                for (i, x) in a.iter().take(12).enumerate() {
                    d.previous_tax_rate[i] = f32_of(x, k)?;
                }
            }
            other => return Err(format!("unknown aura state field '{other}'")),
        }
    }
    Ok(())
}

fn tri_json(t: &[(i32, i32, i32)]) -> Json {
    Json::Array(
        t.iter()
            .map(|&(a, b, c)| Json::Array(vec![ji(a), ji(b), ji(c)]))
            .collect(),
    )
}

fn expect(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let d = s.data().clone();
    let t = &d.tuning;
    let curve = |c: &Option<fc4_core::curve::ResponseCurve>| Json::Bool(c.is_some());
    let arr3 = |a: [i32; 3]| Json::Array(a.iter().map(|&v| ji(v)).collect());
    let arr4 = |a: [f32; 4]| Json::Array(a.iter().map(|&v| jf(v)).collect());
    let slice = s.slice_step();
    let got: Vec<(String, Json)> = vec![
        ("initialized".into(), Json::Bool(d.initialized)),
        ("state".into(), ji(d.state)),
        ("cycle_start_day".into(), ji(d.cycle_start_day)),
        ("department_cursor".into(), d.department_cursor.map_or(Json::Null, ji)),
        ("mayor_rating".into(), ji(d.mayor_rating)),
        ("unused_rating".into(), ji(d.unused_rating)),
        ("global_effect".into(), ji(d.global_effect)),
        (
            "previous_tax_rate".into(),
            Json::Array(d.previous_tax_rate.iter().map(|&v| jf(v)).collect()),
        ),
        ("effective_base_aura".into(), arr3(d.effective_base_aura)),
        ("max_rating".into(), ji(d.max_rating)),
        (
            "max_location".into(),
            Json::Array(vec![ji(d.max_location.0), ji(d.max_location.1)]),
        ),
        ("min_rating".into(), ji(d.min_rating)),
        (
            "min_location".into(),
            Json::Array(vec![ji(d.min_location.0), ji(d.min_location.1)]),
        ),
        ("low_tracts".into(), tri_json(&d.low_tracts)),
        (
            "hotspots".into(),
            tri_json(&d.hotspots.iter().map(|h| (h.x, h.z, h.weight)).collect::<Vec<_>>()),
        ),
        ("riot_roll".into(), jf(d.riot_roll)),
        ("re_add_landmarks".into(), Json::Bool(d.re_add_landmarks)),
        ("long_decay".into(), jf(t.long_decay)),
        ("short_decay".into(), jf(t.short_decay)),
        ("base_aura".into(), arr3(t.base_aura)),
        ("hotspot".into(), arr3(t.hotspot)),
        ("transient_magnitude".into(), arr4(t.transient_magnitude)),
        ("transient_radius".into(), arr4(t.transient_radius)),
        (
            "curves".into(),
            Json::Object(vec![
                ("park".into(), curve(&t.park)),
                ("air".into(), curve(&t.pollution[0])),
                ("water".into(), curve(&t.pollution[1])),
                ("garbage".into(), curve(&t.pollution[2])),
                ("radiation".into(), curve(&t.pollution[3])),
                ("school".into(), curve(&t.school)),
                ("hospital".into(), curve(&t.hospital)),
                ("crime".into(), curve(&t.crime)),
                ("trip".into(), curve(&t.trip)),
                ("congestion".into(), curve(&t.congestion)),
                ("tax_change".into(), curve(&t.tax_change)),
                ("value_wealth".into(), curve(&t.value_wealth)),
                ("riot".into(), curve(&t.riot)),
            ]),
        ),
        (
            "departments".into(),
            Json::Array(s.department_ids().into_iter().map(ji).collect()),
        ),
        ("lot_iterator".into(), Json::Bool(s.has_lot_iterator())),
        ("slice_step".into(), Json::Array(vec![ji(slice.step), ji(slice.misses)])),
    ];
    let want = field(op, "state")?;
    // `curves` is compared partially.
    if let Some(c) = want.get("curves") {
        let g = &got.iter().find(|(k, _)| k == "curves").expect("curves").1;
        if !scalar::matches(g, c, true) {
            return Err(format!("curves: got {}, expected {}", show(g), show(c)));
        }
    }
    let rest: Vec<(String, Json)> = want
        .as_object()
        .ok_or("state must be an object")?
        .iter()
        .filter(|(k, _)| k != "curves")
        .cloned()
        .collect();
    check_state(&got, &Json::Object(rest))
}

// ---- grids ------------------------------------------------------------------------------------------

enum AuraGrid {
    I8(Rc<dyn TractGrid<i8>>),
    I16(Rc<dyn TractGrid<i16>>),
}

fn grid_by_name(s: &AuraSim, name: &str) -> R<Option<AuraGrid>> {
    if let Some(id) = name.strip_prefix("dept:") {
        let id = Json::Str(id.into()).as_i64().ok_or("bad department id")? as u32;
        return Ok(s.department_grid(id).map(AuraGrid::I16));
    }
    Ok(match name {
        "aura" => s.aura_grid().map(AuraGrid::I8),
        "base" => s.base_grid().map(AuraGrid::I8),
        "transient" => s.transient_grid().map(AuraGrid::I8),
        "work" => s.work_grid().map(AuraGrid::I8),
        "building_effect" => s.building_effect_grid().map(AuraGrid::I8),
        "park_map" => s.park_map().map(AuraGrid::I16),
        "raw_park" => s.raw_park_grid().map(AuraGrid::I16),
        "landmark_map" => s.landmark_map().map(AuraGrid::I16),
        "landmark_work" => s.landmark_work_grid().map(AuraGrid::I16),
        other => return Err(format!("unknown aura grid '{other}'")),
    })
}

/// `aura_set_grid`: `grid`, `fill?`, `cells?`. A missing department grid
/// (`dept:ID`) is created through the occupant path's rule (int16, S = 4,
/// instance = ID, filled with 0) as test setup.
fn set_grid(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let name = str_of(field(op, "grid")?, "grid")?;
    let g = match grid_by_name(&s, name)? {
        Some(g) => g,
        None => {
            let id = name
                .strip_prefix("dept:")
                .and_then(|v| Json::Str(v.into()).as_i64())
                .ok_or_else(|| format!("grid '{name}' does not exist"))? as u32;
            // Through a landmark occupant (§7.2) with no cone.
            let tok = w.token();
            let mut o = Occupant {
                dept: Some(id),
                ..Default::default()
            };
            o.props.insert(aura::ids::PROP_LANDMARK_EFFECT, vec![0, 0]);
            w.occupants.borrow_mut().insert(tok, o);
            s.occupant_changed(tok, true);
            w.occupants.borrow_mut().remove(&tok);
            grid_by_name(&s, name)?.ok_or("department grid not created")?
        }
    };
    fn fill<T: GridElem>(g: &dyn TractGrid<T>, op: &Json) -> R<()> {
        if let Some(v) = opt(op, "fill") {
            g.fill(value_of::<T>(v)?);
        }
        write_cells(g, op)
    }
    match g {
        AuraGrid::I8(g) => fill(&*g, op),
        AuraGrid::I16(g) => fill(&*g, op),
    }
}

/// `aura_expect_grid`: `grid`, any of `exists`, `instance`, `tract_size`,
/// `counts`, `cells` (`[[x, z, v]]`), `others` (every unlisted tract).
fn expect_grid(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let name = str_of(field(op, "grid")?, "grid")?;
    let g = grid_by_name(&s, name)?;
    if let Some(e) = opt(op, "exists") {
        if g.is_some() != bool_of(e, "exists")? {
            return Err(format!("grid '{name}' exists = {}", g.is_some()));
        }
    }
    let Some(g) = g else {
        return if opt(op, "exists").is_some() {
            Ok(())
        } else {
            Err(format!("grid '{name}' does not exist"))
        };
    };
    fn check<T: GridElem>(name: &str, g: &dyn TractGrid<T>, op: &Json) -> R<()> {
        let geo = g.geometry();
        if let Some(v) = opt(op, "tract_size") {
            if i64::from(geo.tract_size) != i64_of(v, "tract_size")? {
                return Err(format!("{name}: tract size {}", geo.tract_size));
            }
        }
        if let Some(v) = opt(op, "counts") {
            let (x, z) = pair_i(v, "counts")?;
            if (geo.count_x, geo.count_z) != (x, z) {
                return Err(format!("{name}: counts {} × {}", geo.count_x, geo.count_z));
            }
        }
        let mut listed = std::collections::BTreeSet::new();
        let mut errors = Vec::new();
        if let Some(c) = opt(op, "cells") {
            for e in arr(c, "cells")? {
                let e = arr(e, "cells")?;
                let (x, z) = (i64_of(&e[0], "x")? as i32, i64_of(&e[1], "z")? as i32);
                listed.insert((x, z));
                let got = g.get(x, z).to_f64();
                let want = num(&e[2]).ok_or("bad cell value")?;
                if !scalar::close(got, want) {
                    errors.push(format!("({x}, {z}) = {got}, expected {want}"));
                }
            }
        }
        if let Some(o) = opt(op, "others") {
            let want = num(o).ok_or("bad 'others' value")?;
            'outer: for x in 0..geo.count_x.clamp(0, 4096) {
                for z in 0..geo.count_z.clamp(0, 4096) {
                    if listed.contains(&(x, z)) {
                        continue;
                    }
                    let got = g.get(x, z).to_f64();
                    if !scalar::close(got, want) {
                        errors.push(format!("({x}, {z}) = {got}, expected {want} (others)"));
                        if errors.len() > 8 {
                            break 'outer;
                        }
                    }
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(format!("{name}: {}", errors.join("; ")))
        }
    }
    match g {
        AuraGrid::I8(g) => check(name, &*g, op),
        AuraGrid::I16(g) => check(name, &*g, op),
    }
}

// ---- persistence --------------------------------------------------------------------------------------

fn slot(op: &Json) -> R<String> {
    Ok(str_of(field(op, "slot")?, "slot")?.to_owned())
}

fn save_now(w: &World) -> R<Saved> {
    let s = w.sim()?;
    scalar::save(&[RefValue::Object(s as Rc<dyn SaveObject>)])
}

fn save(w: &World, op: &Json) -> R<()> {
    let s = save_now(w)?;
    w.saves.borrow_mut().insert(slot(op)?, s);
    Ok(())
}

/// `aura_load`: a fresh simulator and core grids from the store; `expect`
/// is whether the aura record loaded (default true).
fn load(w: &World, op: &Json) -> R<()> {
    let name = slot(op)?;
    let saved = w.saves.borrow().get(&name).cloned().ok_or("no such slot")?;
    let me = w.rc();
    let into = opt(op, "into_current").map_or(Ok(false), |v| bool_of(v, "into_current"))?;
    let current = if into { Some(w.sim()?) } else { None };
    let mut rng = Some(w.rng(opt(op, "rng"))?);
    let mut factory = |class: u32, _id: u32| -> Option<RefValue> {
        if class == CLSID_AURA_SIMULATOR {
            if let Some(c) = &current {
                return Some(RefValue::Object(c.clone() as Rc<dyn SaveObject>));
            }
            let r = rng.take()?;
            return Some(RefValue::Object(
                AuraSim::new(me.clone() as Rc<dyn AuraHost>, me.token(), r) as Rc<dyn SaveObject>,
            ));
        }
        create_for_clsid(class).map(RefValue::Object)
    };
    let (ok, roots) = scalar::load(&saved, &mut factory);
    let want = opt(op, "expect").map_or(Ok(true), |v| bool_of(v, "expect"))?;
    if ok != want {
        return Err(format!("store load clean = {ok}, expected {want}"));
    }
    if !ok || into {
        return Ok(());
    }
    let sim = roots
        .first()
        .and_then(|r| r.object::<AuraSim>())
        .ok_or("the root did not resolve to an aura simulator")?;
    *w.sim.borrow_mut() = Some(sim);
    Ok(())
}

fn expect_save(w: &World, op: &Json) -> R<()> {
    let want = w.saves.borrow().get(&slot(op)?).cloned().ok_or("no such slot")?;
    scalar::same_save(&save_now(w)?, &want)
}

fn expect_payload(w: &World, op: &Json) -> R<()> {
    let saves = w.saves.borrow();
    let s = saves.get(&slot(op)?).ok_or("no such slot")?;
    let got = scalar::entry_payload(s, CLSID_AURA_SIMULATOR, 0)?;
    if let Some(h) = opt(op, "hex") {
        let want = hex_decode(str_of(h, "hex")?)?;
        if got != want {
            return Err(format!(
                "payload differs:\n    got      {}\n    expected {}",
                hex_encode(&got),
                hex_encode(&want)
            ));
        }
    }
    if let Some(n) = opt(op, "size") {
        if got.len() as i64 != i64_of(n, "size")? {
            return Err(format!("payload size {}", got.len()));
        }
    }
    Ok(())
}
