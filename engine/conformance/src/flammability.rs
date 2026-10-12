// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/flammability.md`: a scripted city
//! (occupants with types, flammability, lots and bounds; lots with
//! conditions and buildings; watered and zoned cells; ordinance factor,
//! season, tutorial, fire manager), a scriptable fire roll, grids created
//! through the host, and ops that drive the flammability simulator. See
//! README.md ("Flammability ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::{Rc, Weak};

use fc4_core::curve::ResponseCurve;
use fc4_core::flammability::{
    answers_iid, compute_hotspots, daily_value, msg, occupant_raw_value, occupant_type, prop, CellRect,
    FlammabilityHost, FlammabilityRandom, FlammabilityRng, FlammabilitySim, Hotspot, LotId, Message, OccupantId,
    Tuning, CLSID_FLAMMABILITY_SIMULATOR,
};
use fc4_core::grid::{create_for_clsid, CityDims, GridProvider, IteratorState, SimGrid, TractGrid};
use fc4_core::property::PropertyHolder;
use fc4_core::rng::{CrtRand, FixedSeeds};
use fc4_core::stream::{RefValue, SaveObject};
use fc4_core::variant::{Elem, Variant};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, check_state, f32_of, i64_of, jf, ji, opt, str_of, u32_of, EventLog, Saved, R,
};

#[derive(Clone)]
struct Occ {
    t: u32,
    flam: Option<Variant>,
    preview: bool,
    lot: Option<LotId>,
    rect: CellRect,
    pos: [f32; 3],
}

#[derive(Default)]
pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    sim: RefCell<Option<Rc<FlammabilitySim>>>,
    next_token: Cell<u64>,
    saves: RefCell<BTreeMap<String, Saved>>,
    cells: Cell<(i32, i32)>,
    occs: RefCell<BTreeMap<u64, Occ>>,
    unlisted: RefCell<BTreeSet<u64>>,
    lots: RefCell<BTreeMap<u64, (u8, Option<u64>)>>,
    random_lot: Cell<Option<u64>>,
    factor: Cell<Option<f32>>,
    season: Cell<Option<i32>>,
    unwatered_all: Cell<bool>,
    unwatered: RefCell<BTreeSet<(i32, i32)>>,
    zoned_all: Cell<bool>,
    zoned: RefCell<BTreeSet<(i32, i32)>>,
    tutorial: Cell<bool>,
    no_fire_manager: Cell<bool>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    /// Zone-type queries per tract (one per daily visit).
    visits: RefCell<BTreeMap<(i32, i32), u32>>,
    /// The grids the host made or loaded: (instance, grid).
    grids: RefCell<Vec<Rc<SimGrid<u8>>>>,
    crt_mark: Cell<CrtRand>,
    /// Messages posted so far.
    posts: Cell<u64>,
}

/// A scripted fire roll: queued values of `r` (the result of
/// `RandomDoubleRangeUniform(0, 100)`), the last one repeating.
struct Script {
    world: Weak<World>,
    queue: VecDeque<f64>,
    last: f64,
}

impl FlammabilityRandom for Script {
    fn range_f64(&mut self, a: f64, b: f64) -> f64 {
        let r = self.queue.pop_front().unwrap_or(self.last);
        self.last = r;
        if let Some(w) = self.world.upgrade() {
            w.events.push(
                "rng",
                vec![("value", Json::Float(r)), ("a", Json::Float(a)), ("b", Json::Float(b))],
            );
        }
        r
    }
}

/// The real generator, with its draws logged.
struct Logged {
    world: Weak<World>,
    rng: FlammabilityRng,
}

impl FlammabilityRandom for Logged {
    fn range_f64(&mut self, a: f64, b: f64) -> f64 {
        let r = self.rng.range_f64(a, b);
        if let Some(w) = self.world.upgrade() {
            w.events.push(
                "rng",
                vec![("value", Json::Float(r)), ("a", Json::Float(a)), ("b", Json::Float(b))],
            );
        }
        r
    }
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            next_token: Cell::new(0x100),
            cells: Cell::new((64, 64)),
            factor: Cell::new(Some(1.0)),
            season: Cell::new(Some(0)),
            ..Default::default()
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn rc(&self) -> Rc<World> {
        self.me.borrow().upgrade().expect("world alive")
    }

    fn sim(&self) -> R<Rc<FlammabilitySim>> {
        self.sim
            .borrow()
            .clone()
            .ok_or_else(|| "no flammability simulator (flam_create)".into())
    }

    fn token(&self) -> u64 {
        let t = self.next_token.get();
        self.next_token.set(t + 1);
        t
    }

    fn rng(&self, spec: Option<&Json>) -> R<Box<dyn FlammabilityRandom>> {
        let me = self.me.borrow().clone();
        let Some(spec) = spec else {
            return Ok(Box::new(Script {
                world: me,
                queue: VecDeque::new(),
                last: 50.0,
            }));
        };
        if let Some(c) = opt(spec, "process_clock") {
            let seeds = FixedSeeds {
                wall_clock: 0,
                process_clock: u32_of(c, "process_clock")?,
            };
            return Ok(Box::new(Logged {
                world: me,
                rng: FlammabilityRng::new(&seeds),
            }));
        }
        let queue = arr(field(spec, "r")?, "r")?
            .iter()
            .map(|v| scalar::num(v).ok_or_else(|| "r: number".to_string()))
            .collect::<R<VecDeque<f64>>>()?;
        Ok(Box::new(Script {
            world: me,
            queue,
            last: 50.0,
        }))
    }

    /// The instance ID of a grid the host made or loaded (`None` if unknown).
    fn instance_of(&self, g: &Rc<dyn TractGrid<u8>>) -> Option<u32> {
        let p = Rc::as_ptr(g) as *const u8;
        self.grids
            .borrow()
            .iter()
            .find(|s| Rc::as_ptr(s) as *const u8 == p)
            .map(|s| s.instance_id())
    }
}

impl GridProvider<u8> for World {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        let (x, z) = self.cells.get();
        let city = CityDims {
            cell_count_x: x,
            cell_count_z: z,
            ..CityDims::square(1)
        };
        self.events.push(
            "create_grid",
            vec![("instance", ji(instance)), ("size", ji(tract_size))],
        );
        let g = SimGrid::<u8>::create(&city, instance, tract_size);
        self.grids.borrow_mut().push(g.clone());
        Some(g as Rc<dyn TractGrid<u8>>)
    }
}

impl FlammabilityHost for World {
    fn city_cell_counts(&self) -> (i32, i32) {
        self.cells.get()
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        ((x / 16.0).floor() as i32, (z / 16.0).floor() as i32)
    }
    fn is_tutorial(&self) -> bool {
        self.tutorial.get()
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn occupant_type(&self, occ: OccupantId) -> u32 {
        self.occs.borrow().get(&occ).map_or(0, |o| o.t)
    }
    fn occupant_property(&self, occ: OccupantId, id: u32) -> Option<Variant> {
        let occs = self.occs.borrow();
        let o = occs.get(&occ)?;
        match id {
            prop::FLAMMABILITY => o.flam.clone(),
            prop::PREVIEW_OCCUPANT if o.preview => Some(Variant::Scalar(Elem::Bool(true))),
            _ => None,
        }
    }
    fn occupant_bounds(&self, occ: OccupantId) -> CellRect {
        self.occs.borrow().get(&occ).map_or((0, 0, -1, -1), |o| o.rect)
    }
    fn occupant_position(&self, occ: OccupantId) -> [f32; 3] {
        self.occs.borrow().get(&occ).map_or([0.0; 3], |o| o.pos)
    }
    fn occupants_in_cells(&self, (x0, z0, x1, z1): CellRect) -> Vec<OccupantId> {
        let accepted = [occupant_type::BUILDING, occupant_type::FLORA, occupant_type::PROP];
        let unlisted = self.unlisted.borrow();
        self.occs
            .borrow()
            .iter()
            .filter(|(id, o)| {
                !unlisted.contains(id)
                    && accepted.contains(&o.t)
                    && o.rect.0 <= x1
                    && o.rect.2 >= x0
                    && o.rect.1 <= z1
                    && o.rect.3 >= z0
            })
            .map(|(&id, _)| id)
            .collect()
    }
    fn occupant_lot(&self, occ: OccupantId) -> Option<LotId> {
        self.occs.borrow().get(&occ).and_then(|o| o.lot)
    }
    fn lot_condition(&self, lot: LotId) -> u8 {
        self.lots.borrow().get(&lot).map_or(0, |l| l.0)
    }
    fn lot_building(&self, lot: LotId) -> Option<OccupantId> {
        self.lots.borrow().get(&lot).and_then(|l| l.1)
    }
    fn random_lot(&self) -> Option<LotId> {
        let l = self.random_lot.get();
        self.events
            .push("random_lot", vec![("lot", l.map_or(Json::Null, |v| ji(v as i64)))]);
        l
    }
    fn cumulative_factor(&self, _prop: u32, _index: u32) -> Option<f32> {
        self.factor.get()
    }
    fn current_season(&self) -> Option<i32> {
        self.season.get()
    }
    fn is_cell_watered(&self, x: i32, z: i32) -> bool {
        !(self.unwatered_all.get() || self.unwatered.borrow().contains(&(x, z)))
    }
    fn zone_type(&self, x: i32, z: i32) -> u32 {
        *self.visits.borrow_mut().entry((x, z)).or_default() += 1;
        u32::from(self.zoned_all.get() || self.zoned.borrow().contains(&(x, z)))
    }
    fn record_history(&self, history_type: u32, value: i32) {
        self.events
            .push("history", vec![("type", ji(history_type)), ("value", ji(value))]);
    }
    fn ignite_cell(&self, x: u32, z: u32, use_probability: bool, teleport: bool, fire_iid: u32) -> bool {
        if self.no_fire_manager.get() {
            return false;
        }
        self.events.push(
            "ignite",
            vec![
                ("x", ji(x)),
                ("z", ji(z)),
                ("use_probability", Json::Bool(use_probability)),
                ("teleport", Json::Bool(teleport)),
                ("fire_iid", ji(fire_iid)),
            ],
        );
        true
    }
    fn set_fire_cause(&self, cause: u32) {
        self.events.push("cause", vec![("cause", ji(cause))]);
    }
    fn post(&self, msg_type: u32) {
        self.posts.set(self.posts.get() + 1);
        let name = match msg_type {
            msg::SIM_CYCLE_COMPLETE => Json::Str("SimCycleComplete".into()),
            t => ji(t),
        };
        self.events.push("post", vec![("type", name)]);
    }
    fn grid_shutdown(&self, grid: &Rc<dyn TractGrid<u8>>) {
        let inst = self.instance_of(grid).map_or(Json::Null, ji);
        self.events.push("grid_shutdown", vec![("instance", inst)]);
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "flammability", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("flam_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "tuning" => tuning(&w, op),
        "create" => (|| {
            let rng = w.rng(opt(op, "rng"))?;
            let s = FlammabilitySim::new(w.rc() as Rc<dyn FlammabilityHost>, w.token(), rng);
            if let Some(seed) = opt(op, "crt_seed") {
                s.set_crt(CrtRand::new(u32_of(seed, "crt_seed")?));
            }
            w.crt_mark.set(s.crt());
            *w.sim.borrow_mut() = Some(s);
            Ok(())
        })(),
        "rng" => w.sim().and_then(|s| {
            s.set_random(w.rng(Some(op))?);
            Ok(())
        }),
        "init" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.init()))),
        "shutdown" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.shutdown()))),
        "message" => message(&w, op),
        "days" => days(&w, op),
        "call" => call(&w, op),
        "set" => set(&w, op),
        "expect" => expect(&w, op),
        "expect_grid" => expect_grid(&w, op),
        "expect_rand" => expect_rand(&w, op),
        "expect_visits" => expect_visits(&w, op),
        "expect_events" => w.events.expect_events(op),
        "expect_event_count" => w.events.expect_count(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }
        "save" => save(&w, op),
        "load" => load(&w, op),
        "expect_save" => expect_save(&w, op),
        "patch" => patch(&w, op),
        "expect_payload" => expect_payload(&w, op),
        _ => Err(format!("unknown op '{name}'")),
    })
}

// ---- environment ----------------------------------------------------------------------------------

fn i32_of(j: &Json, what: &str) -> R<i32> {
    Ok(i64_of(j, what)? as i32)
}

fn pair(j: &Json, what: &str) -> R<(i32, i32)> {
    let a = arr(j, what)?;
    if a.len() != 2 {
        return Err(format!("'{what}' must be [x, z]"));
    }
    Ok((i32_of(&a[0], what)?, i32_of(&a[1], what)?))
}

fn cell_set(j: &Json, what: &str) -> R<BTreeSet<(i32, i32)>> {
    arr(j, what)?.iter().map(|c| pair(c, what)).collect()
}

fn type_of(j: &Json) -> R<u32> {
    Ok(match j.as_str() {
        Some("building") => occupant_type::BUILDING,
        Some("prop") => occupant_type::PROP,
        Some("flora") => occupant_type::FLORA,
        _ => u32_of(j, "type")?,
    })
}

fn occupant(e: &Json) -> R<(u64, Occ)> {
    let id = u32_of(field(e, "id")?, "id")? as u64;
    let rect = if let Some(r) = opt(e, "rect") {
        let a = arr(r, "rect")?;
        if a.len() != 4 {
            return Err("'rect' must be [x0, z0, x1, z1]".into());
        }
        (
            i32_of(&a[0], "rect")?,
            i32_of(&a[1], "rect")?,
            i32_of(&a[2], "rect")?,
            i32_of(&a[3], "rect")?,
        )
    } else {
        let (x, z) = pair(field(e, "cell")?, "cell")?;
        (x, z, x, z)
    };
    let flam = match e.get("flam") {
        None | Some(Json::Null) => None,
        Some(v @ Json::Object(_)) => Some(scalar::variant_of(v)?),
        Some(v) => Some(Variant::Scalar(Elem::U32(u32_of(v, "flam")?))),
    };
    let pos = match opt(e, "pos") {
        Some(p) => {
            let a = arr(p, "pos")?;
            [f32_of(&a[0], "pos")?, f32_of(&a[1], "pos")?, f32_of(&a[2], "pos")?]
        }
        None => [0.0; 3],
    };
    Ok((
        id,
        Occ {
            t: type_of(field(e, "type")?)?,
            flam,
            preview: opt(e, "preview").map_or(Ok(false), |v| bool_of(v, "preview"))?,
            lot: opt(e, "lot").map(|v| u32_of(v, "lot")).transpose()?.map(u64::from),
            rect,
            pos,
        },
    ))
}

fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "cells") {
        w.cells.set(pair(v, "cells")?);
    }
    if let Some(v) = opt(op, "occupants") {
        for e in arr(v, "occupants")? {
            let (id, o) = occupant(e)?;
            w.occs.borrow_mut().insert(id, o);
            w.unlisted.borrow_mut().remove(&id);
        }
    }
    if let Some(v) = opt(op, "unlist") {
        for e in arr(v, "unlist")? {
            w.unlisted.borrow_mut().insert(u64::from(u32_of(e, "unlist")?));
        }
    }
    if let Some(v) = opt(op, "forget") {
        for e in arr(v, "forget")? {
            w.occs.borrow_mut().remove(&u64::from(u32_of(e, "forget")?));
        }
    }
    if let Some(v) = opt(op, "lots") {
        for e in arr(v, "lots")? {
            let id = u64::from(u32_of(field(e, "id")?, "id")?);
            let cond = opt(e, "condition").map_or(Ok(0), |c| u32_of(c, "condition"))? as u8;
            let b = opt(e, "building")
                .map(|b| u32_of(b, "building"))
                .transpose()?
                .map(u64::from);
            w.lots.borrow_mut().insert(id, (cond, b));
        }
    }
    if let Some(v) = op.get("random_lot") {
        w.random_lot.set(match v {
            Json::Null => None,
            v => Some(u64::from(u32_of(v, "random_lot")?)),
        });
    }
    if let Some(v) = op.get("factor") {
        w.factor.set(match v {
            Json::Null => None,
            v => Some(f32_of(v, "factor")?),
        });
    }
    if let Some(v) = op.get("season") {
        w.season.set(match v {
            Json::Null => None,
            v => Some(i32_of(v, "season")?),
        });
    }
    if let Some(v) = opt(op, "unwatered") {
        *w.unwatered.borrow_mut() = cell_set(v, "unwatered")?;
    }
    if let Some(v) = opt(op, "unwatered_all") {
        w.unwatered_all.set(bool_of(v, "unwatered_all")?);
    }
    if let Some(v) = opt(op, "zoned") {
        *w.zoned.borrow_mut() = cell_set(v, "zoned")?;
    }
    if let Some(v) = opt(op, "zoned_all") {
        w.zoned_all.set(bool_of(v, "zoned_all")?);
    }
    if let Some(v) = opt(op, "tutorial") {
        w.tutorial.set(bool_of(v, "tutorial")?);
    }
    if let Some(v) = opt(op, "fire_manager") {
        w.no_fire_manager.set(!bool_of(v, "fire_manager")?);
    }
    Ok(())
}

/// `flam_tuning`: `shipped` (the §8 shipped values), `props` (merged over
/// them when `shipped`), or `none` (no exemplar).
fn tuning(w: &World, op: &Json) -> R<()> {
    if opt(op, "none").and_then(crate::runner::boolean).unwrap_or(false) {
        *w.tuning.borrow_mut() = None;
        return Ok(());
    }
    let mut h = if opt(op, "shipped").and_then(crate::runner::boolean).unwrap_or(false) {
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

// ---- calls ------------------------------------------------------------------------------------------

fn msg_type(j: &Json) -> R<u32> {
    Ok(match j.as_str() {
        Some("SimNewDay") => msg::SIM_NEW_DAY,
        Some("SimBegin") => msg::SIM_BEGIN,
        Some("InsertOccupant") => msg::INSERT_OCCUPANT,
        Some("RemoveOccupant") => msg::REMOVE_OCCUPANT,
        Some("ChangeOccupant") => msg::CHANGE_OCCUPANT,
        Some("SeasonChange") => msg::SEASON_CHANGE,
        Some("OrdinanceUpdated") => msg::ORDINANCE_UPDATED,
        Some("ConditionChanged") => msg::CONDITION_CHANGED,
        Some("ExemplarChanged") => msg::EXEMPLAR_CHANGED,
        _ => u32_of(j, "type")?,
    })
}

fn message(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let m = Message {
        msg_type: msg_type(field(op, "type")?)?,
        occupant: opt(op, "occupant")
            .map(|v| u32_of(v, "occupant"))
            .transpose()?
            .map(u64::from),
        lot: opt(op, "lot").map(|v| u32_of(v, "lot")).transpose()?.map(u64::from),
    };
    check_expect(op, Json::Bool(s.do_message(&m)))
}

/// `flam_days`: `count` SimNewDay messages; `ends?` lists the days (1-based
/// within this op) on which SimCycleComplete was posted. Clears nothing.
fn days(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let count = i64_of(field(op, "count")?, "count")?;
    let want = opt(op, "ends").map(|v| arr(v, "ends")).transpose()?;
    let mut ends = Vec::new();
    for d in 1..=count {
        let before = w.posts.get();
        s.do_message(&Message::of_type(msg::SIM_NEW_DAY));
        for _ in before..w.posts.get() {
            ends.push(ji(d));
        }
    }
    if let Some(want) = want {
        let got = Json::Array(ends);
        if !scalar::matches(&got, &Json::Array(want.to_vec()), false) {
            return Err(format!(
                "cycle ends on days {}, expected {}",
                scalar::show(&got),
                scalar::show(&Json::Array(want.to_vec()))
            ));
        }
    }
    Ok(())
}

fn a_i32(op: &Json, i: usize) -> R<i32> {
    i32_of(arg(op, i)?, "argument")
}

fn hotspot_json(h: &Hotspot) -> Json {
    Json::Array(vec![ji(h.x), ji(h.z), ji(h.weight)])
}

fn hotspots_json(v: &[Hotspot]) -> Json {
    Json::Array(v.iter().map(hotspot_json).collect())
}

fn hotspots_of(j: &Json, what: &str) -> R<Vec<Hotspot>> {
    arr(j, what)?
        .iter()
        .map(|h| {
            let a = arr(h, what)?;
            if a.len() != 3 {
                return Err(format!("'{what}' entries are [x, z, weight]"));
            }
            Ok(Hotspot::new(
                i32_of(&a[0], what)?,
                i32_of(&a[1], what)?,
                i32_of(&a[2], what)?,
            ))
        })
        .collect()
}

fn call(w: &World, op: &Json) -> R<()> {
    let method = str_of(field(op, "method")?, "method")?;
    // Pure helpers first (no simulator needed).
    let got = match method {
        "occupant_raw_value" => Some(ji(occupant_raw_value(
            i64_of(arg(op, 0)?, "p")?,
            f32_of(arg(op, 1)?, "m")?,
        ))),
        "daily_value" => Some(ji(daily_value(
            u32_of(arg(op, 0)?, "r")? as u8,
            f32_of(arg(op, 1)?, "g")?,
            bool_of(arg(op, 2)?, "watered")?,
            f32_of(arg(op, 3)?, "no_water")?,
        ))),
        "global_multiplier" => {
            let f = match arg(op, 0)? {
                Json::Null => None,
                v => Some(f32_of(v, "factor")?),
            };
            let season = match arg(op, 1)? {
                Json::Null => None,
                v => Some(i32_of(v, "season")?),
            };
            Some(jf(fc4_core::flammability::global_multiplier(
                f,
                season,
                f32_of(arg(op, 2)?, "summer")?,
            )))
        }
        "compute_hotspots" => Some(hotspots_json(&compute_hotspots(
            &hotspots_of(arg(op, 0)?, "candidates")?,
            a_i32(op, 1)?,
            a_i32(op, 2)?,
        ))),
        "query_interface" => Some(Json::Bool(answers_iid(u32_of(arg(op, 0)?, "iid")?))),
        _ => None,
    };
    if let Some(g) = got {
        return check_expect(op, g);
    }
    let s = w.sim()?;
    let got = match method {
        "occupant_raw" => ji(s.occupant_raw(u64::from(u32_of(arg(op, 0)?, "occupant")?))),
        "tract_raw" => ji(s.tract_raw(a_i32(op, 0)?, a_i32(op, 1)?)),
        "occupant_update" => {
            s.occupant_update(
                u64::from(u32_of(arg(op, 0)?, "occupant")?),
                bool_of(arg(op, 1)?, "add")?,
            );
            Json::Null
        }
        "compute_global_multiplier" => jf(s.compute_global_multiplier()),
        "is_cell_flammable" => Json::Bool(s.is_cell_flammable(a_i32(op, 0)?, a_i32(op, 1)?)),
        "hotspot" => match s.flammability_hotspot() {
            Some((v, x, z)) => Json::Array(vec![ji(v), ji(x), ji(z)]),
            None => Json::Bool(false),
        },
        "average" => ji(s.average_flammability()),
        "flammability_grid" => match s.adjusted_grid() {
            Some(g) => w.instance_of(&g).map_or(Json::Null, ji),
            None => Json::Null,
        },
        "daily" => {
            s.do_daily_simulation();
            Json::Null
        }
        "end_of_cycle" => {
            s.end_of_cycle();
            Json::Null
        }
        "compute_hotspot_list" => {
            s.compute_hotspot_list();
            hotspots_json(&s.snapshot().hotspots)
        }
        "fire_roll" => {
            s.fire_roll();
            Json::Null
        }
        "curve" => {
            let d = s.snapshot();
            match d.tuning.hotspot_fire_curve {
                Some(c) => jf(c.eval(f32_of(arg(op, 0)?, "x")?)),
                None => Json::Null,
            }
        }
        "add_ref" => ji(s.add_ref()),
        "release" => ji(s.release()),
        other => return Err(format!("unknown method '{other}'")),
    };
    check_expect(op, got)
}

fn grid_of(s: &FlammabilitySim, name: &str) -> R<Option<Rc<dyn TractGrid<u8>>>> {
    match name {
        "raw" => Ok(s.raw_grid()),
        "adjusted" => Ok(s.adjusted_grid()),
        other => Err(format!("unknown grid '{other}' (raw, adjusted)")),
    }
}

fn cells_of(j: &Json, what: &str) -> R<Vec<(i32, i32, i64)>> {
    arr(j, what)?
        .iter()
        .map(|c| {
            let a = arr(c, what)?;
            if a.len() != 3 {
                return Err(format!("'{what}' entries are [x, z, v]"));
            }
            Ok((i32_of(&a[0], what)?, i32_of(&a[1], what)?, i64_of(&a[2], what)?))
        })
        .collect()
}

fn set(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    for name in ["raw", "adjusted"] {
        let fill = opt(op, &format!("{name}_fill"));
        let cells = opt(op, name);
        if fill.is_none() && cells.is_none() {
            continue;
        }
        let g = grid_of(&s, name)?.ok_or_else(|| format!("no {name} grid"))?;
        if let Some(v) = fill {
            g.fill(u32_of(v, "fill")? as u8);
        }
        if let Some(c) = cells {
            for (x, z, v) in cells_of(c, name)? {
                g.set(x, z, v as u8);
            }
        }
    }
    let Some(state) = opt(op, "state") else { return Ok(()) };
    let mut d = s.data_mut();
    for (k, v) in state.as_object().ok_or("'state' must be an object")? {
        match k.as_str() {
            "global_multiplier" => d.global_multiplier = f32_of(v, k)?,
            "average" => d.average = u32_of(v, k)? as u8,
            "zoned_sum" => d.zoned_sum = i32_of(v, k)?,
            "zoned_count" => d.zoned_count = i32_of(v, k)?,
            "tracts_per_day" => d.tracts_per_day = i32_of(v, k)?,
            "candidates" => d.candidates = hotspots_of(v, k)?,
            "hotspots" => d.hotspots = hotspots_of(v, k)?,
            "summer_multiplier" => d.tuning.summer_multiplier = f32_of(v, k)?,
            "abandonment_multiplier" => d.tuning.abandonment_multiplier = f32_of(v, k)?,
            "no_water_multiplier" => d.tuning.no_water_multiplier = f32_of(v, k)?,
            "hotspot_threshold" => d.tuning.hotspot_threshold = i32_of(v, k)?,
            "hotspot_radius" => d.tuning.hotspot_radius = i32_of(v, k)?,
            "hotspot_min_weight" => d.tuning.hotspot_min_weight = i32_of(v, k)?,
            "random_fire_probability" => d.tuning.random_fire_probability = f32_of(v, k)?,
            "curve" => {
                d.tuning.hotspot_fire_curve = match v {
                    Json::Null => None,
                    v => {
                        let flat = arr(v, k)?.iter().map(|x| f32_of(x, k)).collect::<R<Vec<f32>>>()?;
                        ResponseCurve::from_flat(&flat)
                    }
                }
            }
            other => return Err(format!("unknown state field '{other}'")),
        }
    }
    Ok(())
}

fn expect(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let d = s.snapshot();
    let t = &d.tuning;
    let it = &d.iterator;
    let state = match it.state {
        IteratorState::NotStarted => "not_started",
        IteratorState::Running => "running",
        IteratorState::Exhausted => "exhausted",
    };
    let iterator = Json::Object(vec![
        ("state".into(), Json::Str(state.into())),
        ("mode".into(), ji(it.mode)),
        ("dirty".into(), ji(it.dirty)),
        (
            "rect".into(),
            Json::Array(vec![ji(it.x0), ji(it.z0), ji(it.x1), ji(it.z1)]),
        ),
        ("count".into(), ji(it.count)),
        ("index".into(), ji(it.index)),
        ("current".into(), ji(it.current)),
        ("step".into(), ji(it.step)),
    ]);
    let got: Vec<(String, Json)> = vec![
        ("initialized".into(), Json::Bool(d.initialized)),
        ("ref_count".into(), ji(d.ref_count)),
        ("destroyed".into(), Json::Bool(d.destroyed)),
        ("count_x".into(), ji(d.count_x)),
        ("count_z".into(), ji(d.count_z)),
        ("average".into(), ji(d.average)),
        ("global_multiplier".into(), jf(d.global_multiplier)),
        ("zoned_sum".into(), ji(d.zoned_sum)),
        ("zoned_count".into(), ji(d.zoned_count)),
        ("candidates".into(), hotspots_json(&d.candidates)),
        ("hotspots".into(), hotspots_json(&d.hotspots)),
        ("tracts_per_day".into(), ji(d.tracts_per_day)),
        ("summer_multiplier".into(), jf(t.summer_multiplier)),
        ("abandonment_multiplier".into(), jf(t.abandonment_multiplier)),
        ("no_water_multiplier".into(), jf(t.no_water_multiplier)),
        ("tract_size".into(), ji(t.tract_size)),
        ("cycle_period".into(), ji(t.cycle_period)),
        ("hotspot_threshold".into(), ji(t.hotspot_threshold)),
        ("hotspot_radius".into(), ji(t.hotspot_radius)),
        ("hotspot_min_weight".into(), ji(t.hotspot_min_weight)),
        ("random_fire_probability".into(), jf(t.random_fire_probability)),
        ("has_curve".into(), Json::Bool(t.hotspot_fire_curve.is_some())),
        ("has_adjusted".into(), Json::Bool(s.adjusted_grid().is_some())),
        ("has_raw".into(), Json::Bool(s.raw_grid().is_some())),
        ("iterator".into(), iterator),
    ];
    // The iterator matches partially.
    let mut want = field(op, "state")?.clone();
    if let Json::Object(kv) = &mut want {
        if let Some(pos) = kv.iter().position(|(k, _)| k == "iterator") {
            let (_, wi) = kv.remove(pos);
            let gi = &got.iter().find(|(k, _)| k == "iterator").expect("iterator").1;
            if !scalar::matches(gi, &wi, true) {
                return Err(format!(
                    "iterator: got {}, expected {}",
                    scalar::show(gi),
                    scalar::show(&wi)
                ));
            }
        }
    }
    check_state(&got, &want)
}

/// `flam_expect_grid`: `grid` (`raw`, `adjusted`), any of `counts`
/// (`[x, z]`), `tract_size`, `instance`, `cells` (`[[x, z, v], …]`),
/// `others` (every unlisted tract), `all`.
fn expect_grid(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let name = str_of(field(op, "grid")?, "grid")?;
    let g = grid_of(&s, name)?.ok_or_else(|| format!("no {name} grid"))?;
    let geo = g.geometry();
    if let Some(v) = opt(op, "counts") {
        let (x, z) = pair(v, "counts")?;
        if (geo.count_x, geo.count_z) != (x, z) {
            return Err(format!(
                "counts ({}, {}), expected ({x}, {z})",
                geo.count_x, geo.count_z
            ));
        }
    }
    if let Some(v) = opt(op, "tract_size") {
        if geo.size != i32_of(v, "tract_size")? {
            return Err(format!("tract size {}", geo.size));
        }
    }
    if let Some(v) = opt(op, "instance") {
        let want = u32_of(v, "instance")?;
        if w.instance_of(&g) != Some(want) {
            return Err(format!("instance {:?}, expected {want:#X}", w.instance_of(&g)));
        }
    }
    let mut listed = BTreeSet::new();
    let mut errors = Vec::new();
    if let Some(c) = opt(op, "cells") {
        for (x, z, v) in cells_of(c, "cells")? {
            listed.insert((x, z));
            let got = g.get(x, z);
            if i64::from(got) != v {
                errors.push(format!("({x}, {z}) = {got}, expected {v}"));
            }
        }
    }
    let others = opt(op, "others").or_else(|| opt(op, "all"));
    if let Some(o) = others {
        let want = i64_of(o, "others")?;
        for x in 0..geo.count_x {
            for z in 0..geo.count_z {
                if !listed.contains(&(x, z)) && i64::from(g.get(x, z)) != want {
                    errors.push(format!("({x}, {z}) = {}, expected {want}", g.get(x, z)));
                    if errors.len() > 8 {
                        return Err(errors.join("; "));
                    }
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

/// `flam_expect_rand`: `calls` = the number of `rand()` calls since the
/// simulator was created or since the last `flam_expect_rand`.
fn expect_rand(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let now = s.crt();
    let mut c = w.crt_mark.get();
    let mut n = 0i64;
    while c != now {
        c.rand();
        n += 1;
        if n > 10_000_000 {
            return Err("rand() state not reachable from the mark".into());
        }
    }
    w.crt_mark.set(now);
    let want = i64_of(field(op, "calls")?, "calls")?;
    if n != want {
        return Err(format!("{n} rand() calls, expected {want}"));
    }
    Ok(())
}

/// `flam_expect_visits`: zone-type queries (one per daily visit) since the
/// last check: `all` (every tract of the city exactly that many times),
/// `cells` (`[[x, z, n], …]`), `total`. Clears the counters.
fn expect_visits(w: &World, op: &Json) -> R<()> {
    let v = std::mem::take(&mut *w.visits.borrow_mut());
    if let Some(t) = opt(op, "total") {
        let total: i64 = v.values().map(|&n| i64::from(n)).sum();
        if total != i64_of(t, "total")? {
            return Err(format!("{total} visits"));
        }
    }
    if let Some(a) = opt(op, "all") {
        let want = i64_of(a, "all")?;
        let (cx, cz) = w.cells.get();
        if v.len() as i64 != i64::from(cx) * i64::from(cz) {
            return Err(format!("{} tracts visited, expected {}", v.len(), cx * cz));
        }
        if let Some(((x, z), n)) = v.iter().find(|(_, &n)| i64::from(n) != want) {
            return Err(format!("tract ({x}, {z}) visited {n} times, expected {want}"));
        }
    }
    if let Some(c) = opt(op, "cells") {
        for (x, z, n) in cells_of(c, "cells")? {
            let got = v.get(&(x, z)).copied().unwrap_or(0);
            if i64::from(got) != n {
                return Err(format!("tract ({x}, {z}) visited {got} times, expected {n}"));
            }
        }
    }
    Ok(())
}

// ---- persistence ------------------------------------------------------------------------------------

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

fn load(w: &World, op: &Json) -> R<()> {
    let name = slot(op)?;
    let saved = w.saves.borrow().get(&name).cloned().ok_or("no such slot")?;
    let me = w.rc();
    let mut rng = Some(w.rng(opt(op, "rng"))?);
    let mut factory = |class: u32, _id: u32| -> Option<RefValue> {
        if class == CLSID_FLAMMABILITY_SIMULATOR {
            let r = rng.take()?;
            let s = FlammabilitySim::new(me.clone() as Rc<dyn FlammabilityHost>, me.token(), r);
            return Some(RefValue::Object(s as Rc<dyn SaveObject>));
        }
        let o = create_for_clsid(class)?;
        if let Ok(g) = o.clone().as_any_rc().downcast::<SimGrid<u8>>() {
            me.grids.borrow_mut().push(g);
        }
        Some(RefValue::Object(o))
    };
    let (ok, roots) = scalar::load(&saved, &mut factory);
    let want = opt(op, "expect").map_or(Ok(true), |v| bool_of(v, "expect"))?;
    if ok != want {
        return Err(format!("store load clean = {ok}, expected {want}"));
    }
    let sim = roots
        .first()
        .and_then(|r| r.object::<FlammabilitySim>())
        .ok_or("the root did not resolve to a flammability simulator")?;
    w.crt_mark.set(sim.crt());
    *w.sim.borrow_mut() = Some(sim);
    Ok(())
}

fn expect_save(w: &World, op: &Json) -> R<()> {
    let want = w.saves.borrow().get(&slot(op)?).cloned().ok_or("no such slot")?;
    scalar::same_save(&save_now(w)?, &want)
}

fn patch(w: &World, op: &Json) -> R<()> {
    let mut saves = w.saves.borrow_mut();
    let s = saves.get_mut(&slot(op)?).ok_or("no such slot")?;
    let class = opt(op, "class").map_or(Ok(CLSID_FLAMMABILITY_SIMULATOR), |c| u32_of(c, "class"))?;
    scalar::patch(s, class, op)
}

fn expect_payload(w: &World, op: &Json) -> R<()> {
    let saves = w.saves.borrow();
    let s = saves.get(&slot(op)?).ok_or("no such slot")?;
    let got = scalar::entry_payload(s, CLSID_FLAMMABILITY_SIMULATOR, 0)?;
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
