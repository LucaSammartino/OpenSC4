// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/flora.md`: a scripted city implementing
//! `FloraHost` (city size and cells, the clock, a terrain with ground,
//! normal, lot and moisture rectangles, an occupant manager holding
//! occupants with types, positions, boxes and flora values, flora
//! exemplars, the tuning exemplar, the obliteration segments), a scriptable
//! generator, a process-wide flora registry, and ops that drive the flora
//! simulator. See README.md ("Flora ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::rc::{Rc, Weak};

use fc4_core::flora::{
    advance_cluster_type, blocks_flora, blocks_per_month, elevation_index, msg, occupant_type, prop, query_interface,
    scale_wild_seeds, table_byte, CellRect, CommandParam, FloraHost, FloraInterface, FloraOccupant, FloraRandom,
    FloraRecord, FloraRegistry, FloraRng, FloraSim, Message, OccupantFilter, OccupantId, RngState, SegmentId,
    SharedRegistry, Vec3, CLSID_FLORA_SIMULATOR, IID_OCCUPANT,
};
use fc4_core::grid::IteratorState;
use fc4_core::property::PropertyHolder;
use fc4_core::rng::{CrtRand, FixedSeeds};
use fc4_core::store::{load_store, save_store, LoadRegistry, Package};
use fc4_core::stream::{ByteReader, ByteWriter, OutStream, RefValue, ResKey, SaveObject, SaveRegistry};
use fc4_core::variant::{mem, Elem, Variant};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, check_state, f32_of, i64_of, jf, ji, opt, str_of, u32_of, EventLog, R,
};

/// Test-only class ID of the occupants a save refers to.
const OCCUPANT_CLASS: u32 = 0x0CC0_0001;
/// The first ID of occupants the simulator creates.
const FIRST_CREATED: u64 = 1000;
/// The cell width of the scripted city (metres).
const CELL: f32 = 16.0;

/// The 14 float names of property `0x083DC35B`, in order.
const FLOAT_NAMES: [&str; 14] = [
    "maturity",
    "seeding_age",
    "interval",
    "seeds",
    "spread",
    "lifespan",
    "max_age",
    "slope",
    "min_score",
    "probability",
    "half_width",
    "unused",
    "clearance",
    "height",
];

#[derive(Clone, Debug, Default)]
struct Occ {
    t: u32,
    pos: [f32; 3],
    bbox: Option<(Vec3, Vec3)>,
    in_manager: bool,
    preview: bool,
    network: Option<u32>,
    flora: Option<FloraOccupant>,
    /// The record type the occupant holds (`SetStaticFloraData`).
    record: Option<u32>,
}

#[derive(Clone, Debug)]
enum SegRec {
    U32(u32),
    Occ(Occ),
}

type Rect = [f32; 4];

#[derive(Clone)]
struct SavedCity {
    pkg: Package,
    root: Vec<u8>,
    hosts: HashMap<u32, u64>,
}

#[derive(Default)]
pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    sim: RefCell<Option<Rc<FloraSim>>>,
    registry: RefCell<Option<SharedRegistry>>,
    next_token: Cell<u64>,
    saves: RefCell<BTreeMap<String, SavedCity>>,
    exists: Cell<bool>,
    size: Cell<(f32, f32)>,
    cells: Cell<(i32, i32)>,
    established: Cell<bool>,
    date: Cell<u32>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    exemplars: RefCell<Vec<(ResKey, Rc<PropertyHolder>)>>,
    spec_segment: Cell<Option<u64>>,
    terrain_init: Cell<bool>,
    sea: Cell<f32>,
    ground: Cell<f32>,
    grounds: RefCell<Vec<(Rect, f32)>>,
    normal_y: Cell<f32>,
    normals: RefCell<Vec<(Rect, f32)>>,
    lots: RefCell<Vec<Rect>>,
    moisture: Cell<u32>,
    moistures: RefCell<Vec<(Rect, u32)>>,
    occs: RefCell<BTreeMap<u64, Occ>>,
    next_occ: Cell<u64>,
    manager: Cell<bool>,
    insert_ok: Cell<bool>,
    refs: RefCell<BTreeMap<u64, i64>>,
    segments: RefCell<BTreeMap<u64, BTreeMap<ResKey, SegRec>>>,
    crt_mark: Cell<CrtRand>,
}

fn in_rect(r: &Rect, x: f32, z: f32) -> bool {
    r[0] <= x && x <= r[2] && r[1] <= z && z <= r[3]
}

/// The scripted occupant manager's box query (`GetOccupantsByBBox`): closed
/// boxes, so point footprints and flat query boxes are found. This is the
/// test host's own query, not the simulator's `BoundingBoxesIntersect`
/// (strict, `boxes_intersect`), which the simulator applies on top (§6.4).
fn query_meets(a: (Vec3, Vec3), b: (Vec3, Vec3)) -> bool {
    (0..3).all(|i| a.0[i] <= b.1[i] && b.0[i] <= a.1[i])
}

/// A scripted generator: queued integers (range / uniform u32 results),
/// queued uniform doubles `u` (range results are computed from `u` with
/// the 24-bit formula of foundations §6.1) and queued Gaussians; every draw
/// is logged.
struct Script {
    world: Weak<World>,
    ints: VecDeque<i64>,
    doubles: VecDeque<f64>,
    gauss: VecDeque<f64>,
    state: RngState,
}

impl Script {
    fn log(&self, fields: Vec<(&str, Json)>) {
        if let Some(w) = self.world.upgrade() {
            w.events.push("rng", fields);
        }
    }
}

impl FloraRandom for Script {
    fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let v = self.ints.pop_front().map_or(lo, |v| v as i32);
        self.log(vec![
            ("method", Json::Str("range_i32".into())),
            ("lo", ji(lo)),
            ("hi", ji(hi)),
            ("value", ji(v)),
        ]);
        v
    }
    fn uniform_u32(&mut self, n: u32) -> u32 {
        let v = self.ints.pop_front().map_or(0, |v| v as u32);
        self.log(vec![
            ("method", Json::Str("uniform_u32".into())),
            ("n", ji(n)),
            ("value", ji(v)),
        ]);
        v
    }
    fn uniform_f64(&mut self) -> f64 {
        let u = self.doubles.pop_front().unwrap_or(0.5);
        self.log(vec![("method", Json::Str("uniform".into())), ("value", Json::Float(u))]);
        u
    }
    fn range_f64(&mut self, a: f64, b: f64) -> f64 {
        let u = self.doubles.pop_front().unwrap_or(0.5);
        let d = (b - a) as f32;
        let p = (u * f64::from(d)) as f32;
        let v = f64::from((f64::from(p) + a) as f32);
        self.log(vec![
            ("method", Json::Str("range_f64".into())),
            ("a", Json::Float(a)),
            ("b", Json::Float(b)),
            ("value", Json::Float(v)),
        ]);
        v
    }
    fn gaussian(&mut self) -> f64 {
        let g = self.gauss.pop_front().unwrap_or(0.0);
        self.log(vec![
            ("method", Json::Str("gaussian".into())),
            ("value", Json::Float(g)),
        ]);
        g
    }
    fn state(&self) -> RngState {
        self.state
    }
    fn set_state(&mut self, s: RngState) {
        self.state = s;
    }
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            next_token: Cell::new(0x100),
            exists: Cell::new(true),
            size: Cell::new((1024.0, 1024.0)),
            cells: Cell::new((64, 64)),
            terrain_init: Cell::new(true),
            sea: Cell::new(250.0),
            ground: Cell::new(260.0),
            normal_y: Cell::new(1.0),
            moisture: Cell::new(8),
            next_occ: Cell::new(FIRST_CREATED),
            manager: Cell::new(true),
            insert_ok: Cell::new(true),
            ..Default::default()
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        *w.tuning.borrow_mut() = Some(Rc::new(shipped_tuning()));
        w
    }

    fn rc(&self) -> Rc<World> {
        self.me.borrow().upgrade().expect("world alive")
    }

    fn sim(&self) -> R<Rc<FloraSim>> {
        self.sim
            .borrow()
            .clone()
            .ok_or_else(|| "no flora simulator (flora_create)".into())
    }

    fn registry(&self) -> SharedRegistry {
        self.registry
            .borrow_mut()
            .get_or_insert_with(FloraRegistry::shared)
            .clone()
    }

    fn token(&self) -> u64 {
        let t = self.next_token.get();
        self.next_token.set(t + 1);
        t
    }

    fn rng(&self, spec: Option<&Json>) -> R<Box<dyn FloraRandom>> {
        let me = self.me.borrow().clone();
        let mut s = Script {
            world: me,
            ints: VecDeque::new(),
            doubles: VecDeque::new(),
            gauss: VecDeque::new(),
            state: RngState::default(),
        };
        let Some(spec) = spec else { return Ok(Box::new(s)) };
        if let Some(seed) = opt(spec, "seed") {
            let seeds = FixedSeeds {
                wall_clock: 0,
                process_clock: opt(spec, "process_clock").map_or(Ok(1), |v| u32_of(v, "process_clock"))?,
            };
            return Ok(Box::new(FloraRng::with_seed(u32_of(seed, "seed")?, &seeds)));
        }
        let nums = |k: &str| -> R<VecDeque<f64>> {
            match opt(spec, k) {
                None => Ok(VecDeque::new()),
                Some(v) => arr(v, k)?
                    .iter()
                    .map(|x| scalar::num(x).ok_or_else(|| format!("{k}: number")))
                    .collect(),
            }
        };
        s.ints = nums("ints")?.into_iter().map(|v| v as i64).collect();
        s.doubles = nums("doubles")?;
        s.gauss = nums("gauss")?;
        Ok(Box::new(s))
    }

    fn occ<T>(&self, id: u64, f: impl FnOnce(&Occ) -> T) -> Option<T> {
        self.occs.borrow().get(&id).map(f)
    }

    fn occ_mut(&self, id: u64, f: impl FnOnce(&mut Occ)) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&id) {
            f(o);
        }
    }

    /// The bounding box: explicit, or the position ± the record's half width
    /// (default 1) and height (default 3) above it.
    fn bbox(&self, o: &Occ) -> (Vec3, Vec3) {
        if let Some(b) = o.bbox {
            return b;
        }
        let rec = o.record.and_then(|t| self.registry().borrow().get(t));
        let (hw, ht) = rec.map_or((1.0, 3.0), |r| (r.half_width, r.height));
        let p = o.pos;
        ([p[0] - hw, p[1], p[2] - hw], [p[0] + hw, p[1] + ht, p[2] + hw])
    }

    fn ground_at(&self, x: f32, z: f32) -> f32 {
        let g = self.grounds.borrow();
        g.iter()
            .rev()
            .find(|(r, _)| in_rect(r, x, z))
            .map_or(self.ground.get(), |e| e.1)
    }

    fn ev(&self, kind: &str, occ: OccupantId, extra: Vec<(&str, Json)>) {
        let mut f = vec![("occupant", ji(occ as i64))];
        f.extend(extra);
        self.events.push(kind, f);
    }

    fn new_occupant(&self, o: Occ) -> u64 {
        let id = self.next_occ.get();
        self.next_occ.set(id + 1);
        self.occs.borrow_mut().insert(id, o);
        self.refs.borrow_mut().insert(id, 1);
        id
    }
}

/// The shipped "Flora Tuning Parameters" (§7).
fn shipped_tuning() -> PropertyHolder {
    let mut h = PropertyHolder::new();
    h.add(prop::BURN_RATE, Variant::Scalar(Elem::F32(1.0)));
    h.add(prop::BURNT_REMOVAL_STEP, Variant::Scalar(Elem::F32(1.0)));
    h.add(prop::PERIOD, Variant::Scalar(Elem::U32(24)));
    h.add(prop::WILD_SEEDS, Variant::Scalar(Elem::U32(0)));
    h.add(prop::ELEVATION_SCALE, Variant::Scalar(Elem::F32(800.0)));
    h
}

impl FloraHost for World {
    fn city_exists(&self) -> bool {
        self.exists.get()
    }
    fn city_size(&self) -> (f32, f32) {
        self.size.get()
    }
    fn city_cell_counts(&self) -> (i32, i32) {
        self.cells.get()
    }
    fn city_established(&self) -> bool {
        self.established.get()
    }
    fn sim_date_number(&self) -> u32 {
        self.date.get()
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn flora_exemplars(&self) -> Vec<(ResKey, Rc<PropertyHolder>)> {
        self.exemplars.borrow().clone()
    }
    fn obliterated_segment(&self) -> Option<SegmentId> {
        self.spec_segment.get()
    }

    fn register_app_shutdown_callback(&self) {
        self.events.push("app_callback", vec![]);
    }
    fn subscribe(&self, msg_types: &[u32]) {
        let t = msg_types.iter().map(|&v| ji(v)).collect();
        self.events.push("subscribe", vec![("types", Json::Array(t))]);
    }
    fn unsubscribe(&self, msg_types: &[u32]) {
        let t = msg_types.iter().map(|&v| ji(v)).collect();
        self.events.push("unsubscribe", vec![("types", Json::Array(t))]);
    }
    fn register_command(&self, id: u32, name: &str) {
        self.events.push(
            "register_command",
            vec![("id", ji(id)), ("name", Json::Str(name.into()))],
        );
    }
    fn unregister_command(&self, id: u32) {
        self.events.push("unregister_command", vec![("id", ji(id))]);
    }
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        self.events.push(
            "add_agent",
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
    fn acquire_services(&self) {
        self.events.push("acquire_services", vec![]);
    }
    fn release_services(&self) {
        self.events.push("release_services", vec![]);
    }
    fn register_cheat(&self, id: u32, name: &str) {
        self.events
            .push("register_cheat", vec![("id", ji(id)), ("name", Json::Str(name.into()))]);
    }
    fn unregister_cheat(&self, id: u32) {
        self.events.push("unregister_cheat", vec![("id", ji(id))]);
    }
    fn post(&self, msg_type: u32, data2: u32) {
        self.events
            .push("post", vec![("type", ji(msg_type)), ("data2", ji(data2))]);
    }

    fn terrain_initialized(&self) -> bool {
        self.terrain_init.get()
    }
    fn altitude(&self, x: f32, z: f32) -> f32 {
        self.ground_at(x, z)
    }
    fn sea_level(&self) -> f32 {
        self.sea.get()
    }
    fn face_normal(&self, x: f32, z: f32) -> Vec3 {
        let n = self.normals.borrow();
        let y = n
            .iter()
            .rev()
            .find(|(r, _)| in_rect(r, x, z))
            .map_or(self.normal_y.get(), |e| e.1);
        [0.0, y, 0.0]
    }
    fn lot_savvy_altitude(&self, x: f32, z: f32) -> (f32, bool) {
        let on_lot = self.lots.borrow().iter().any(|r| in_rect(r, x, z));
        (self.ground_at(x, z), on_lot)
    }
    fn moisture_type(&self, x: f32, z: f32) -> u32 {
        let m = self.moistures.borrow();
        m.iter()
            .rev()
            .find(|(r, _)| in_rect(r, x, z))
            .map_or(self.moisture.get(), |e| e.1)
    }

    fn occupant_type(&self, occ: OccupantId) -> u32 {
        self.occ(occ, |o| o.t).unwrap_or(0)
    }
    fn occupant_has_property(&self, occ: OccupantId, id: u32) -> bool {
        id == prop::PREVIEW_OCCUPANT && self.occ(occ, |o| o.preview).unwrap_or(false)
    }
    fn occupant_position(&self, occ: OccupantId) -> Vec3 {
        self.occ(occ, |o| o.pos).unwrap_or([0.0; 3])
    }
    fn set_occupant_position(&self, occ: OccupantId, pos: Vec3) {
        self.ev(
            "set_position",
            occ,
            vec![("pos", Json::Array(pos.iter().map(|&v| jf(v)).collect()))],
        );
        self.occ_mut(occ, |o| {
            if let Some((mn, mx)) = o.bbox.as_mut() {
                for i in 0..3 {
                    mn[i] += pos[i] - o.pos[i];
                    mx[i] += pos[i] - o.pos[i];
                }
            }
            o.pos = pos;
        });
    }
    fn occupant_bounding_box(&self, occ: OccupantId) -> (Vec3, Vec3) {
        let o = self.occs.borrow().get(&occ).cloned();
        o.map_or(([0.0; 3], [0.0; 3]), |o| self.bbox(&o))
    }
    fn occupant_in_manager(&self, occ: OccupantId) -> bool {
        self.occ(occ, |o| o.in_manager).unwrap_or(false)
    }
    fn set_occupant_in_manager(&self, occ: OccupantId, on: bool) {
        self.ev("set_flag", occ, vec![("on", Json::Bool(on))]);
        self.occ_mut(occ, |o| o.in_manager = on);
    }
    fn network_flags(&self, occ: OccupantId) -> Option<u32> {
        self.occ(occ, |o| o.network).flatten()
    }
    fn flora_occupant(&self, occ: OccupantId) -> Option<FloraOccupant> {
        self.occ(occ, |o| o.flora).flatten()
    }
    fn set_flora_type(&self, occ: OccupantId, flora_type: u32) {
        self.ev("set_type", occ, vec![("value", ji(flora_type))]);
        self.occ_mut(occ, |o| {
            if let Some(f) = o.flora.as_mut() {
                f.flora_type = flora_type;
            }
        });
    }
    fn set_birth_date(&self, occ: OccupantId, day: u32) {
        self.ev("set_birth", occ, vec![("value", ji(day))]);
        self.occ_mut(occ, |o| {
            if let Some(f) = o.flora.as_mut() {
                f.birth_date = day;
            }
        });
    }
    fn set_last_seeding_date(&self, occ: OccupantId, day: u32) {
        self.ev("set_last_seed", occ, vec![("value", ji(day))]);
        self.occ_mut(occ, |o| {
            if let Some(f) = o.flora.as_mut() {
                f.last_seeding_date = day;
            }
        });
    }
    fn set_flora_state(&self, occ: OccupantId, state: i8) {
        self.ev("set_state", occ, vec![("value", ji(state))]);
        self.occ_mut(occ, |o| {
            if let Some(f) = o.flora.as_mut() {
                f.state = state;
            }
        });
    }
    fn set_flora_orientation(&self, occ: OccupantId, orientation: u32) {
        self.ev("set_orientation", occ, vec![("value", ji(orientation))]);
        self.occ_mut(occ, |o| {
            if let Some(f) = o.flora.as_mut() {
                f.orientation = orientation as u8;
            }
        });
    }
    fn set_flora_record(&self, occ: OccupantId, record: Option<Rc<FloraRecord>>) {
        let t = record.as_ref().map(|r| r.type_id);
        self.ev("set_record", occ, vec![("record", t.map_or(Json::Null, ji))]);
        self.occ_mut(occ, |o| o.record = t);
    }
    fn post_occupant_change(&self, occ: OccupantId) {
        self.ev("change", occ, vec![]);
    }
    fn create_flora_occupant(&self) -> Option<OccupantId> {
        let id = self.new_occupant(Occ {
            t: occupant_type::FLORA,
            flora: Some(FloraOccupant::default()),
            ..Occ::default()
        });
        self.ev("create", id, vec![]);
        Some(id)
    }
    fn init_occupant(&self, occ: OccupantId) {
        self.ev("init", occ, vec![]);
    }
    fn add_ref(&self, occ: OccupantId) {
        *self.refs.borrow_mut().entry(occ).or_insert(0) += 1;
    }
    fn release(&self, occ: OccupantId) {
        *self.refs.borrow_mut().entry(occ).or_insert(0) -= 1;
    }
    fn object_ref(&self, occ: OccupantId) -> RefValue {
        RefValue::Host {
            token: occ,
            class_id: OCCUPANT_CLASS,
        }
    }
    fn resolve_ref(&self, r: &RefValue, iid: u32) -> Option<OccupantId> {
        match r {
            RefValue::Host { token, class_id } if *class_id == OCCUPANT_CLASS && iid == IID_OCCUPANT => Some(*token),
            RefValue::Host { token, .. } => {
                self.release(*token);
                None
            }
            _ => None,
        }
    }

    fn has_occupant_manager(&self) -> bool {
        self.manager.get()
    }
    fn insert_occupant(&self, occ: OccupantId) -> bool {
        self.ev("insert", occ, vec![]);
        if !self.insert_ok.get() {
            return false;
        }
        self.occ_mut(occ, |o| o.in_manager = true);
        self.add_ref(occ);
        true
    }
    fn remove_occupant(&self, occ: OccupantId) -> bool {
        self.ev("remove", occ, vec![]);
        let was = self.occ(occ, |o| o.in_manager).unwrap_or(false);
        if was {
            self.occ_mut(occ, |o| o.in_manager = false);
            self.release(occ);
        }
        true
    }
    fn move_occupant(&self, occ: OccupantId) {
        self.ev("move", occ, vec![]);
    }
    fn occupants_in_bbox(&self, min: Vec3, max: Vec3, filter: OccupantFilter) -> Vec<OccupantId> {
        let occs: Vec<(u64, Occ)> = self.occs.borrow().iter().map(|(&k, v)| (k, v.clone())).collect();
        occs.into_iter()
            .filter(|(_, o)| {
                o.in_manager
                    && match filter {
                        OccupantFilter::Type(t) => o.t == t,
                        OccupantFilter::Blocking => blocks_flora(o.t, o.network),
                    }
                    && query_meets(self.bbox(o), (min, max))
            })
            .map(|(k, _)| k)
            .collect()
    }
    fn occupants_in_cells(&self, rect: Option<CellRect>, occ_type: u32) -> Vec<OccupantId> {
        self.occs
            .borrow()
            .iter()
            .filter(|(_, o)| {
                let cx = (o.pos[0] / CELL).floor() as i32;
                let cz = (o.pos[2] / CELL).floor() as i32;
                o.in_manager
                    && o.t == occ_type
                    && rect.map_or(true, |(x0, z0, x1, z1)| x0 <= cx && cx <= x1 && z0 <= cz && cz <= z1)
            })
            .map(|(&k, _)| k)
            .collect()
    }
    fn occupants_in_manager_cells(&self, occ_type: u32) -> Vec<OccupantId> {
        self.occupants_in_cells(None, occ_type)
    }

    fn segment_write_u32(&self, seg: SegmentId, key: ResKey, value: u32) -> bool {
        self.events.push(
            "seg_write",
            vec![
                ("segment", ji(seg as i64)),
                ("key", key_json(key)),
                ("value", ji(value)),
            ],
        );
        self.segments
            .borrow_mut()
            .entry(seg)
            .or_default()
            .insert(key, SegRec::U32(value));
        true
    }
    fn segment_read_u32(&self, seg: SegmentId, key: ResKey) -> Option<u32> {
        match self.segments.borrow().get(&seg)?.get(&key)? {
            SegRec::U32(v) => Some(*v),
            SegRec::Occ(_) => None,
        }
    }
    fn segment_write_occupant(&self, seg: SegmentId, key: ResKey, occ: OccupantId) -> bool {
        let Some(o) = self.occs.borrow().get(&occ).cloned() else {
            return false;
        };
        self.events.push(
            "seg_write",
            vec![
                ("segment", ji(seg as i64)),
                ("key", key_json(key)),
                ("occupant", ji(occ as i64)),
                ("in_manager", Json::Bool(o.in_manager)),
            ],
        );
        self.segments
            .borrow_mut()
            .entry(seg)
            .or_default()
            .insert(key, SegRec::Occ(o));
        true
    }
    fn segment_read_flora_occupant(&self, seg: SegmentId, key: ResKey) -> Option<OccupantId> {
        let rec = self.segments.borrow().get(&seg)?.get(&key)?.clone();
        let mut o = match rec {
            SegRec::Occ(o) => o,
            SegRec::U32(_) => Occ {
                t: occupant_type::FLORA,
                flora: Some(FloraOccupant::default()),
                ..Occ::default()
            },
        };
        o.in_manager = false;
        let id = self.new_occupant(o);
        self.ev("read_occupant", id, vec![("key", key_json(key))]);
        Some(id)
    }
}

fn key_json(k: ResKey) -> Json {
    Json::Array(vec![ji(k.type_id), ji(k.group), ji(k.instance)])
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "flora", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("flora_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "types" => types(&w, op),
        "tuning" => tuning(&w, op),
        "registry" => registry_op(&w, op),
        "create" => (|| {
            let rng = w.rng(opt(op, "rng"))?;
            let s = FloraSim::new(w.rc() as Rc<dyn FloraHost>, w.registry(), w.token(), rng);
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
        "days" => w.sim().map(|s| {
            let n = op.get("count").and_then(Json::as_i64).unwrap_or(1);
            for _ in 0..n {
                s.do_message(&Message::of_type(msg::SIM_NEW_DAY));
            }
        }),
        "call" => call(&w, op),
        "set" => set(&w, op),
        "expect" => expect(&w, op),
        "expect_occupant" => expect_occupant(&w, op),
        "expect_record" => expect_record(&w, op),
        "expect_registry" => expect_registry(&w, op),
        "expect_rand" => expect_rand(&w, op),
        "expect_refs" => expect_refs(&w, op),
        "expect_segment" => expect_segment(&w, op),
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
        "read" => read_record(&w, op),
        _ => Err(format!("unknown op '{name}'")),
    })
}

// ---- environment ----------------------------------------------------------------------------------

fn i32_of(j: &Json, what: &str) -> R<i32> {
    Ok(i64_of(j, what)? as i32)
}

fn vec3(j: &Json, what: &str) -> R<Vec3> {
    let a = arr(j, what)?;
    if a.len() != 3 {
        return Err(format!("'{what}' must be [x, y, z]"));
    }
    Ok([f32_of(&a[0], what)?, f32_of(&a[1], what)?, f32_of(&a[2], what)?])
}

fn vec3_json(v: Vec3) -> Json {
    Json::Array(v.iter().map(|&x| jf(x)).collect())
}

fn pair_f(j: &Json, what: &str) -> R<(f32, f32)> {
    let a = arr(j, what)?;
    if a.len() != 2 {
        return Err(format!("'{what}' must be a pair"));
    }
    Ok((f32_of(&a[0], what)?, f32_of(&a[1], what)?))
}

fn rect4(j: &Json, what: &str) -> R<Rect> {
    let a = arr(j, what)?;
    if a.len() < 4 {
        return Err(format!("'{what}' entries start with [x0, z0, x1, z1]"));
    }
    Ok([
        f32_of(&a[0], what)?,
        f32_of(&a[1], what)?,
        f32_of(&a[2], what)?,
        f32_of(&a[3], what)?,
    ])
}

fn cell_rect(j: &Json, what: &str) -> R<CellRect> {
    let a = arr(j, what)?;
    if a.len() != 4 {
        return Err(format!("'{what}' must be [x0, z0, x1, z1]"));
    }
    Ok((
        i32_of(&a[0], what)?,
        i32_of(&a[1], what)?,
        i32_of(&a[2], what)?,
        i32_of(&a[3], what)?,
    ))
}

fn type_of(j: &Json) -> R<u32> {
    Ok(match j.as_str() {
        Some("flora") => occupant_type::FLORA,
        Some("building") => occupant_type::BUILDING,
        Some("prop") => occupant_type::PROP,
        Some("landfill") => occupant_type::LANDFILL,
        Some("network") => occupant_type::NETWORK,
        _ => u32_of(j, "type")?,
    })
}

fn occupant_spec(e: &Json) -> R<(u64, Occ)> {
    let id = u64::from(u32_of(field(e, "id")?, "id")?);
    let t = type_of(field(e, "type")?)?;
    let pos = opt(e, "pos").map_or(Ok([0.0; 3]), |p| vec3(p, "pos"))?;
    let bbox = match opt(e, "bbox") {
        Some(b) => {
            let a = arr(b, "bbox")?;
            if a.len() != 2 {
                return Err("'bbox' must be [[x, y, z], [x, y, z]]".into());
            }
            Some((vec3(&a[0], "bbox")?, vec3(&a[1], "bbox")?))
        }
        None => None,
    };
    let flora = if t == occupant_type::FLORA || opt(e, "flora_type").is_some() {
        let g = |k: &str| opt(e, k).map_or(Ok(0), |v| i64_of(v, k));
        Some(FloraOccupant {
            flora_type: g("flora_type")? as u32,
            birth_date: g("birth")? as u32,
            last_seeding_date: g("last_seed")? as u32,
            state: g("state")? as i8,
            orientation: g("orientation")? as u8,
        })
    } else {
        None
    };
    let record = match e.get("record") {
        Some(Json::Null) => None,
        Some(v) => Some(u32_of(v, "record")?),
        None => flora.map(|f| f.flora_type),
    };
    Ok((
        id,
        Occ {
            t,
            pos,
            bbox,
            in_manager: opt(e, "in_manager").map_or(Ok(true), |v| bool_of(v, "in_manager"))?,
            preview: opt(e, "preview").map_or(Ok(false), |v| bool_of(v, "preview"))?,
            network: opt(e, "network").map(|v| u32_of(v, "network")).transpose()?,
            flora,
            record,
        },
    ))
}

fn rects_with<T>(j: &Json, what: &str, val: impl Fn(&Json) -> R<T>) -> R<Vec<(Rect, T)>> {
    arr(j, what)?
        .iter()
        .map(|e| {
            let a = arr(e, what)?;
            if a.len() != 5 {
                return Err(format!("'{what}' entries are [x0, z0, x1, z1, value]"));
            }
            Ok((rect4(e, what)?, val(&a[4])?))
        })
        .collect()
}

fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "exists") {
        w.exists.set(bool_of(v, "exists")?);
    }
    if let Some(v) = opt(op, "size") {
        w.size.set(pair_f(v, "size")?);
    }
    if let Some(v) = opt(op, "cells") {
        let (x, z) = pair_f(v, "cells")?;
        w.cells.set((x as i32, z as i32));
    }
    if let Some(v) = opt(op, "established") {
        w.established.set(bool_of(v, "established")?);
    }
    if let Some(v) = opt(op, "date") {
        w.date.set(u32_of(v, "date")?);
    }
    if let Some(v) = opt(op, "terrain_initialized") {
        w.terrain_init.set(bool_of(v, "terrain_initialized")?);
    }
    if let Some(v) = opt(op, "sea") {
        w.sea.set(f32_of(v, "sea")?);
    }
    if let Some(v) = opt(op, "ground") {
        w.ground.set(f32_of(v, "ground")?);
    }
    if let Some(v) = opt(op, "grounds") {
        *w.grounds.borrow_mut() = rects_with(v, "grounds", |x| f32_of(x, "grounds"))?;
    }
    if let Some(v) = opt(op, "normal_y") {
        w.normal_y.set(f32_of(v, "normal_y")?);
    }
    if let Some(v) = opt(op, "normals") {
        *w.normals.borrow_mut() = rects_with(v, "normals", |x| f32_of(x, "normals"))?;
    }
    if let Some(v) = opt(op, "lots") {
        *w.lots.borrow_mut() = arr(v, "lots")?.iter().map(|r| rect4(r, "lots")).collect::<R<_>>()?;
    }
    if let Some(v) = opt(op, "moisture") {
        w.moisture.set(u32_of(v, "moisture")?);
    }
    if let Some(v) = opt(op, "moistures") {
        *w.moistures.borrow_mut() = rects_with(v, "moistures", |x| u32_of(x, "moistures"))?;
    }
    if let Some(v) = opt(op, "manager") {
        w.manager.set(bool_of(v, "manager")?);
    }
    if let Some(v) = opt(op, "insert_ok") {
        w.insert_ok.set(bool_of(v, "insert_ok")?);
    }
    if let Some(v) = op.get("segment") {
        w.spec_segment.set(match v {
            Json::Null => None,
            v => Some(u64::from(u32_of(v, "segment")?)),
        });
    }
    if let Some(v) = opt(op, "occupants") {
        for e in arr(v, "occupants")? {
            let (id, o) = occupant_spec(e)?;
            let inside = o.in_manager;
            w.occs.borrow_mut().insert(id, o);
            w.refs.borrow_mut().insert(id, i64::from(inside));
        }
    }
    if let Some(v) = opt(op, "forget") {
        for e in arr(v, "forget")? {
            w.occs.borrow_mut().remove(&u64::from(u32_of(e, "forget")?));
        }
    }
    Ok(())
}

/// A flora exemplar from a type spec (see README.md, `flora_types`).
fn exemplar_of(e: &Json) -> R<PropertyHolder> {
    let mut h = PropertyHolder::new();
    let u = |v: u32| Variant::Scalar(Elem::U32(v));
    let et = opt(e, "exemplar_type").map_or(Ok(0x0F), |v| u32_of(v, "exemplar_type"))?;
    h.add(prop::EXEMPLAR_TYPE, u(et));
    if let Some(t) = opt(e, "type") {
        h.add(prop::TYPE_ID, u(u32_of(t, "type")?));
    }
    if let Some(v) = opt(e, "next") {
        h.add(prop::NEXT_CLUSTER_TYPE, u(u32_of(v, "next")?));
    }
    if opt(e, "water").map_or(Ok(false), |v| bool_of(v, "water"))? {
        h.add(prop::WATER, Variant::Scalar(Elem::Bool(true)));
    }
    let mut floats = FloraRecord::new(0).floats().to_vec();
    let mut any = false;
    for (i, name) in FLOAT_NAMES.iter().enumerate() {
        if let Some(v) = opt(e, name) {
            floats[i] = f32_of(v, name)?;
            any = true;
        }
    }
    if let Some(v) = opt(e, "floats") {
        floats = arr(v, "floats")?
            .iter()
            .map(|x| f32_of(x, "floats"))
            .collect::<R<_>>()?;
        any = true;
    }
    if any {
        h.add(
            prop::FLOATS,
            Variant::array(mem::FLOAT32, floats.into_iter().map(Elem::F32).collect()),
        );
    }
    for (key, id) in [
        ("cluster_radius", prop::CLUSTER_RADIUS),
        ("cluster_height", prop::CLUSTER_HEIGHT),
    ] {
        if let Some(v) = opt(e, key) {
            h.add(id, Variant::Scalar(Elem::F32(f32_of(v, key)?)));
        }
    }
    let fill = opt(e, "table").map(|v| f32_of(v, "table")).transpose()?;
    let cells = opt(e, "table_cells");
    if fill.is_some() || cells.is_some() {
        let mut t = vec![0.0f32; 256];
        for (i, x) in t.iter_mut().enumerate() {
            let (em, mm) = (i / 16, i % 16);
            *x = match fill {
                Some(f) => f,
                None if (5..=12).contains(&em) && (5..=12).contains(&mm) => 0.6,
                None => 0.0,
            };
        }
        if let Some(c) = cells {
            for entry in arr(c, "table_cells")? {
                let a = arr(entry, "table_cells")?;
                if a.len() != 3 {
                    return Err("'table_cells' entries are [e, m, value]".into());
                }
                let i = (i64_of(&a[0], "e")? * 16 + i64_of(&a[1], "m")?) as usize;
                *t.get_mut(i).ok_or("table index")? = f32_of(&a[2], "value")?;
            }
        }
        h.add(
            prop::TABLE,
            Variant::array(mem::FLOAT32, t.into_iter().map(Elem::F32).collect()),
        );
    }
    if let Some(v) = opt(e, "never_auto") {
        h.add(prop::NEVER_AUTO, Variant::Scalar(Elem::Bool(bool_of(v, "never_auto")?)));
    }
    if let Some(v) = opt(e, "flag_switch") {
        h.add(
            prop::FLAG_SWITCH,
            Variant::Scalar(Elem::Bool(bool_of(v, "flag_switch")?)),
        );
    }
    if let Some(v) = opt(e, "families") {
        let f = arr(v, "families")?
            .iter()
            .map(|x| u32_of(x, "families").map(Elem::U32))
            .collect::<R<_>>()?;
        h.add(prop::FAMILIES, Variant::array(mem::UINT32, f));
    }
    if let Some(v) = opt(e, "plop_cost") {
        h.add(prop::PLOP_COST, Variant::Scalar(Elem::I64(i64_of(v, "plop_cost")?)));
    }
    if let Some(p) = opt(e, "props") {
        let extra = scalar::holder_of(p)?;
        for id in extra.ids() {
            if let Some(v) = extra.get(id) {
                h.add(id, v.clone());
            }
        }
    }
    Ok(h)
}

/// `flora_types`: `types` (exemplar specs, in enumeration order), `append?`.
fn types(w: &World, op: &Json) -> R<()> {
    let mut list = Vec::new();
    for (i, e) in arr(field(op, "types")?, "types")?.iter().enumerate() {
        let key = ResKey {
            type_id: 0x6534_284A,
            group: 0x7A4A_0000,
            instance: i as u32,
        };
        list.push((key, Rc::new(exemplar_of(e)?)));
    }
    let append = opt(op, "append").map_or(Ok(false), |v| bool_of(v, "append"))?;
    let mut ex = w.exemplars.borrow_mut();
    if !append {
        ex.clear();
    }
    ex.extend(list);
    Ok(())
}

fn tuning(w: &World, op: &Json) -> R<()> {
    if opt(op, "none").and_then(crate::runner::boolean).unwrap_or(false) {
        *w.tuning.borrow_mut() = None;
        return Ok(());
    }
    let mut h = if opt(op, "shipped").map_or(Ok(true), |v| bool_of(v, "shipped"))? {
        shipped_tuning()
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

/// `flora_registry`: `reset` (a new process), `app_shutdown`, `default_flags`.
fn registry_op(w: &World, op: &Json) -> R<()> {
    if opt(op, "reset").map_or(Ok(false), |v| bool_of(v, "reset"))? {
        *w.registry.borrow_mut() = Some(FloraRegistry::shared());
    }
    if let Some(v) = opt(op, "default_flags") {
        w.registry()
            .borrow_mut()
            .set_default_record_flags(u32_of(v, "default_flags")? as u8);
    }
    if opt(op, "app_shutdown").map_or(Ok(false), |v| bool_of(v, "app_shutdown"))? {
        w.registry().borrow_mut().app_shutdown();
    }
    Ok(())
}

// ---- calls ------------------------------------------------------------------------------------------

fn msg_type(j: &Json) -> R<u32> {
    Ok(match j.as_str() {
        Some("SimNewDay") => msg::SIM_NEW_DAY,
        Some("SimNewMonth") => msg::SIM_NEW_MONTH,
        Some("InsertOccupant") => msg::INSERT_OCCUPANT,
        Some("RemoveOccupant") => msg::REMOVE_OCCUPANT,
        Some("TerrainHeightsChanged") => msg::TERRAIN_HEIGHTS_CHANGED,
        Some("ExemplarChanged") => msg::EXEMPLAR_CHANGED,
        Some("CityEstablished") => msg::CITY_ESTABLISHED,
        Some("CityObliterated") => msg::CITY_OBLITERATED,
        Some("PlaceFlora") => msg::PLACE_FLORA,
        Some("Cheat") => msg::CHEAT,
        _ => u32_of(j, "type")?,
    })
}

fn message(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let d = |k: &str| opt(op, k).map_or(Ok(0), |v| u32_of(v, k));
    let params = match opt(op, "params") {
        Some(p) => arr(p, "params")?
            .iter()
            .map(|e| {
                let a = arr(e, "params")?;
                if a.len() != 2 {
                    return Err("'params' entries are [as_u32, as_f32]".to_string());
                }
                Ok(CommandParam {
                    as_u32: u32_of(&a[0], "params")?,
                    as_f32: f32_of(&a[1], "params")?,
                })
            })
            .collect::<R<Vec<_>>>()?,
        None => Vec::new(),
    };
    let m = Message {
        msg_type: msg_type(field(op, "type")?)?,
        data1: d("data1")?,
        data2: d("data2")?,
        data3: match opt(op, "data3") {
            Some(v) if v.as_str().is_some() => type_of(v)?,
            Some(v) => u32_of(v, "data3")?,
            None => 0,
        },
        occupant: opt(op, "occupant")
            .map(|v| u32_of(v, "occupant"))
            .transpose()?
            .map(u64::from),
        text: opt(op, "text")
            .map(|v| str_of(v, "text").map(str::to_owned))
            .transpose()?,
        params,
    };
    check_expect(op, Json::Bool(s.do_message(&m)))
}

fn a_f(op: &Json, i: usize) -> R<f32> {
    f32_of(arg(op, i)?, "argument")
}

fn a_u(op: &Json, i: usize) -> R<u32> {
    u32_of(arg(op, i)?, "argument")
}

fn a_b(op: &Json, i: usize) -> R<bool> {
    bool_of(arg(op, i)?, "argument")
}

fn opt_arg(op: &Json, i: usize) -> Option<&Json> {
    op.get("args")
        .and_then(Json::as_array)
        .and_then(|a| a.get(i))
        .filter(|v| !matches!(v, Json::Null))
}

fn ids_json(v: &[u32]) -> Json {
    Json::Array(v.iter().map(|&x| ji(x)).collect())
}

fn sorted_set(s: &HashSet<u32>) -> Json {
    let mut v: Vec<u32> = s.iter().copied().collect();
    v.sort_unstable();
    ids_json(&v)
}

fn call(w: &World, op: &Json) -> R<()> {
    let method = str_of(field(op, "method")?, "method")?;
    // Pure helpers first (no simulator needed).
    let got = match method {
        "elevation_index" => Some(ji(
            elevation_index(a_f(op, 0)?, a_f(op, 1)?, a_b(op, 2)?, a_f(op, 3)?) as i64
        )),
        "table_byte" => Some(ji(table_byte(a_f(op, 0)?))),
        "blocks_per_month" => Some(ji(blocks_per_month(
            i32_of(arg(op, 0)?, "x")?,
            i32_of(arg(op, 1)?, "z")?,
            a_u(op, 2)?,
        ))),
        "scale_wild_seeds" => Some(ji(scale_wild_seeds(a_u(op, 0)?, a_f(op, 1)?, a_f(op, 2)?))),
        "blocks_flora" => Some(Json::Bool(blocks_flora(
            type_of(arg(op, 0)?)?,
            opt_arg(op, 1).map(|v| u32_of(v, "flags")).transpose()?,
        ))),
        "query_interface" => Some(match query_interface(a_u(op, 0)?) {
            Some(FloraInterface::Primary) => Json::Str("primary".into()),
            Some(FloraInterface::MessageTarget) => Json::Str("message_target".into()),
            Some(FloraInterface::Serializable) => Json::Str("serializable".into()),
            None => Json::Bool(false),
        }),
        "advance_cluster_type" => Some(ji(advance_cluster_type(
            &w.registry().borrow(),
            a_u(op, 0)?,
            a_u(op, 1)?,
        ))),
        _ => None,
    };
    if let Some(g) = got {
        return check_expect(op, g);
    }
    let s = w.sim()?;
    let occ_arg = |i: usize| -> R<u64> { Ok(u64::from(a_u(op, i)?)) };
    let got = match method {
        "elevation_index_at" => ji(s.elevation_index_at(a_f(op, 0)?, a_f(op, 1)?, a_b(op, 2)?) as i64),
        "score" => jf(s.score(a_u(op, 0)?, a_f(op, 1)?, a_f(op, 2)?)),
        "best_type" => {
            let (t, sc) = s.best_type(a_f(op, 0)?, a_f(op, 1)?);
            Json::Array(vec![ji(t), jf(sc)])
        }
        "location_ok" => Json::Bool(s.location_ok(
            a_u(op, 0)?,
            a_f(op, 1)?,
            a_f(op, 2)?,
            opt_arg(op, 3).map(|v| u32_of(v, "exclude")).transpose()?.map(u64::from),
        )),
        "add_new" => {
            let mut pos = vec3(arg(op, 1)?, "pos")?;
            let r = s.add_new_flora_occupant(a_u(op, 0)?, &mut pos, a_b(op, 2)?);
            Json::Object(vec![
                ("occupant".into(), r.map_or(Json::Null, |o| ji(o as i64))),
                ("pos".into(), vec3_json(pos)),
            ])
        }
        "add_flora_occupant" => Json::Bool(s.add_flora_occupant(occ_arg(0)?)),
        "drop_seed" => Json::Bool(s.drop_seed(a_u(op, 0)?, a_f(op, 1)?, a_f(op, 2)?)),
        "simulate_existing" => {
            let o = occ_arg(0)?;
            let fo = w.occ(o, |x| x.flora).flatten().ok_or("not a flora occupant")?;
            s.simulate_existing(o, fo);
            Json::Null
        }
        "monthly" => {
            s.monthly();
            Json::Null
        }
        "wild_seeds" => {
            s.wild_seeds();
            Json::Null
        }
        "daily" => {
            s.daily();
            Json::Null
        }
        "kill" => Json::Bool(s.kill_flora_occupant(occ_arg(0)?)),
        "occupant_inserted" => {
            s.occupant_inserted(occ_arg(0)?, type_of(arg(op, 1)?)?);
            Json::Null
        }
        "terrain_change" => {
            s.terrain_altitude_change(cell_rect(arg(op, 0)?, "rect")?);
            Json::Null
        }
        "demolish" => Json::Bool(s.demolish_flora_occupant(occ_arg(0)?, 7)),
        "demolish_cells" => Json::Bool(s.demolish_in_cell_area(cell_rect(arg(op, 0)?, "rect")?, 7)),
        "demolish_bbox" => {
            Json::Bool(s.demolish_in_bounding_box(vec3(arg(op, 0)?, "min")?, vec3(arg(op, 1)?, "max")?, 7))
        }
        "cost" => ji(s.cost_of_occupant_demolition(occ_arg(0)?, 0)),
        "enable_flora" => {
            s.enable_flora(a_b(op, 0)?);
            Json::Null
        }
        "blast" => {
            s.blast(a_u(op, 0)?, a_u(op, 1)?, a_u(op, 2)?);
            Json::Null
        }
        "static_data" => s.static_flora_data(a_u(op, 0)?).map_or(Json::Null, |r| ji(r.type_id)),
        "all_types" => ids_json(&s.all_flora_types()),
        "all_types_into" => {
            // args: [buffer length or null, count]
            let mut count = a_u(op, 1)?;
            let (ret, copied) = match opt_arg(op, 0) {
                None => (s.all_flora_types_into(None, &mut count), Vec::new()),
                Some(len) => {
                    let mut buf = vec![0u32; u32_of(len, "buffer")? as usize];
                    let r = s.all_flora_types_into(Some(&mut buf), &mut count);
                    buf.truncate(r as usize);
                    (r, buf)
                }
            };
            Json::Object(vec![
                ("ret".into(), ji(ret)),
                ("count".into(), ji(count)),
                ("copied".into(), ids_json(&copied)),
            ])
        }
        "family_types" => ids_json(&s.all_flora_family_types()),
        "is_valid" => Json::Bool(s.is_flora_type_valid(a_u(op, 0)?)),
        "is_family_valid" => Json::Bool(s.is_flora_family_type_valid(a_u(op, 0)?)),
        "types_by_filter" => {
            let mut set = HashSet::new();
            let ret = match opt_arg(op, 0) {
                None => s.flora_types_by_filter(&mut set, None),
                Some(v) => {
                    let ok: HashSet<u32> = arr(v, "accepted")?
                        .iter()
                        .map(|x| u32_of(x, "accepted"))
                        .collect::<R<_>>()?;
                    let f = |t: u32| ok.contains(&t);
                    s.flora_types_by_filter(&mut set, Some(&f))
                }
            };
            Json::Object(vec![("ret".into(), ji(ret)), ("set".into(), sorted_set(&set))])
        }
        "types_by_family" => {
            let mut set = HashSet::new();
            let ret = s.flora_types_by_family(&mut set, a_u(op, 0)?);
            Json::Object(vec![("ret".into(), ji(ret)), ("set".into(), sorted_set(&set))])
        }
        "add_static" => match opt_arg(op, 0) {
            None => Json::Bool(s.add_static_flora_data_from(None)),
            Some(spec) => Json::Bool(s.add_static_flora_data_from(Some(&exemplar_of(spec)?))),
        },
        "save_obliterated" => Json::Bool(s.save_obliterated(u64::from(a_u(op, 0)?))),
        "restore_obliterated" => Json::Bool(s.restore_obliterated(u64::from(a_u(op, 0)?))),
        "read_tuning" => {
            s.read_tuning();
            Json::Null
        }
        "exemplar_changed" => {
            s.exemplar_changed(a_u(op, 0)?);
            Json::Null
        }
        "add_ref" => ji(s.add_ref()),
        "release" => ji(s.release()),
        "draw_uniform" => Json::Float(s.draw_uniform()),
        other => return Err(format!("unknown method '{other}'")),
    };
    check_expect(op, got)
}

fn set(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let Some(state) = opt(op, "state") else { return Ok(()) };
    let mut d = s.data_mut();
    for (k, v) in state.as_object().ok_or("'state' must be an object")? {
        match k.as_str() {
            "enabled" => d.enabled = bool_of(v, k)?,
            "propagation" => d.propagation = bool_of(v, k)?,
            "propagation_forced" => d.propagation_forced = bool_of(v, k)?,
            "preserve" => d.preserve = bool_of(v, k)?,
            "size" => (d.size_x, d.size_z) = pair_f(v, k)?,
            "cells" => {
                let (x, z) = pair_f(v, k)?;
                (d.cells_x, d.cells_z) = (x as i32, z as i32);
            }
            "today" => d.today = u32_of(v, k)?,
            "period" => d.period = u32_of(v, k)?,
            "blocks_per_month" => d.blocks_per_month = u32_of(v, k)?,
            "wild_seeds_per_month" => d.wild_seeds_per_month = u32_of(v, k)?,
            "keep_iterator_flag" => d.keep_iterator_flag = u32_of(v, k)? as u8,
            "elevation_scale" => d.elevation_scale = f32_of(v, k)?,
            "burn_accumulator" => d.burn_accumulator = f32_of(v, k)?,
            "burnt_removal_accumulator" => d.burnt_removal_accumulator = f32_of(v, k)?,
            "burn_rate" => d.burn_rate = f32_of(v, k)?,
            "burnt_removal_step" => d.burnt_removal_step = f32_of(v, k)?,
            other => return Err(format!("unknown state field '{other}'")),
        }
    }
    Ok(())
}

fn expect(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let d = s.snapshot();
    let it = &d.iterator;
    let state = match it.state {
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
        ("table_len".into(), ji(it.table.len() as i64)),
        ("index".into(), ji(it.index)),
        ("current".into(), ji(it.current)),
        ("step".into(), ji(it.step)),
    ]);
    let rs = s.rng_state();
    let got: Vec<(String, Json)> = vec![
        ("initialized".into(), Json::Bool(d.initialized)),
        ("ref_count".into(), ji(d.ref_count)),
        ("destroyed".into(), Json::Bool(d.destroyed)),
        ("enabled".into(), Json::Bool(d.enabled)),
        ("propagation".into(), Json::Bool(d.propagation)),
        ("propagation_forced".into(), Json::Bool(d.propagation_forced)),
        ("preserve".into(), Json::Bool(d.preserve)),
        ("size".into(), Json::Array(vec![jf(d.size_x), jf(d.size_z)])),
        ("cells".into(), Json::Array(vec![ji(d.cells_x), ji(d.cells_z)])),
        ("today".into(), ji(d.today)),
        ("period".into(), ji(d.period)),
        ("blocks_per_month".into(), ji(d.blocks_per_month)),
        ("wild_seeds_per_month".into(), ji(d.wild_seeds_per_month)),
        ("keep_iterator_flag".into(), ji(d.keep_iterator_flag)),
        ("elevation_scale".into(), jf(d.elevation_scale)),
        (
            "burnt".into(),
            Json::Array(d.burnt.iter().map(|&o| ji(o as i64)).collect()),
        ),
        ("burn_accumulator".into(), jf(d.burn_accumulator)),
        ("burnt_removal_accumulator".into(), jf(d.burnt_removal_accumulator)),
        ("burn_rate".into(), jf(d.burn_rate)),
        ("burnt_removal_step".into(), jf(d.burnt_removal_step)),
        ("simulator_held".into(), Json::Bool(d.simulator_held)),
        (
            "rng".into(),
            Json::Array(vec![
                ji(rs.lcg),
                ji(rs.mul),
                ji(rs.has_cached_gaussian),
                Json::Float(rs.cached_gaussian),
            ]),
        ),
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

/// `flora_expect_occupant`: `occupant`, then any of `exists`, `in_manager`,
/// `flora_type`, `birth`, `last_seed`, `state`, `orientation`, `record`
/// (type or null), `pos`, `type`.
fn expect_occupant(w: &World, op: &Json) -> R<()> {
    let id = u64::from(u32_of(field(op, "occupant")?, "occupant")?);
    let o = w.occs.borrow().get(&id).cloned();
    let mut got: Vec<(String, Json)> = vec![("exists".into(), Json::Bool(o.is_some()))];
    if let Some(o) = &o {
        let f = o.flora.unwrap_or_default();
        got.extend([
            ("in_manager".into(), Json::Bool(o.in_manager)),
            ("type".into(), ji(o.t)),
            ("flora_type".into(), ji(f.flora_type)),
            ("birth".into(), ji(f.birth_date)),
            ("last_seed".into(), ji(f.last_seeding_date)),
            ("state".into(), ji(f.state)),
            ("orientation".into(), ji(f.orientation)),
            ("record".into(), o.record.map_or(Json::Null, ji)),
            ("pos".into(), vec3_json(o.pos)),
        ]);
    }
    let mut want = op.clone();
    if let Json::Object(kv) = &mut want {
        kv.retain(|(k, _)| k != "op" && k != "occupant" && k != "note");
    }
    check_state(&got, &want)
}

/// `flora_expect_record`: `type`, then any of `exists`, `next`,
/// `seed_type`, `water`, `flags`, the float names, `cluster_radius`,
/// `cluster_height`, `table` (`[[e, m, byte]]`), `unnamed`, `families`,
/// `plop_cost`.
fn expect_record(w: &World, op: &Json) -> R<()> {
    let t = u32_of(field(op, "type")?, "type")?;
    let rec = w.registry().borrow().get(t);
    let mut got: Vec<(String, Json)> = vec![("exists".into(), Json::Bool(rec.is_some()))];
    if let Some(r) = &rec {
        got.extend([
            ("next".into(), ji(r.next_type)),
            ("seed_type".into(), ji(r.seed_type.get())),
            ("water".into(), Json::Bool(r.water)),
            ("flags".into(), ji(r.flags)),
            ("cluster_radius".into(), jf(r.cluster_radius)),
            ("cluster_height".into(), jf(r.cluster_height)),
            ("unnamed".into(), ji(r.unnamed)),
            ("families".into(), ids_json(&r.families)),
            ("plop_cost".into(), ji(r.plop_cost)),
        ]);
        for (name, v) in FLOAT_NAMES.iter().zip(r.floats()) {
            got.push(((*name).into(), jf(v)));
        }
    }
    let mut want = op.clone();
    let mut table_want = None;
    if let Json::Object(kv) = &mut want {
        if let Some(p) = kv.iter().position(|(k, _)| k == "table") {
            table_want = Some(kv.remove(p).1);
        }
        kv.retain(|(k, _)| k != "op" && k != "type" && k != "note");
    }
    if let Some(tw) = table_want {
        let r = rec.as_ref().ok_or("no such record")?;
        for e in arr(&tw, "table")? {
            let a = arr(e, "table")?;
            if a.len() != 3 {
                return Err("'table' entries are [e, m, byte]".into());
            }
            let i = (i64_of(&a[0], "e")? * 16 + i64_of(&a[1], "m")?) as usize;
            let want = i64_of(&a[2], "byte")?;
            let got = i64::from(*r.table.get(i).ok_or("table index")?);
            if got != want {
                return Err(format!("table[{i}] = {got}, expected {want}"));
            }
        }
    }
    check_state(&got, &want)
}

/// `flora_expect_registry`: any of `types` (the type array), `keys` (the map
/// keys, ascending), `families` (family IDs), `family` (`{id: [types]}`),
/// `loaded`.
fn expect_registry(w: &World, op: &Json) -> R<()> {
    let reg = w.registry();
    let r = reg.borrow();
    let mut fam = Vec::new();
    for id in r.family_ids() {
        fam.push((id.to_string(), ids_json(r.family(id).unwrap_or(&[]))));
    }
    let got: Vec<(String, Json)> = vec![
        ("types".into(), ids_json(r.types())),
        ("keys".into(), ids_json(&r.keys())),
        ("families".into(), ids_json(&r.family_ids())),
        ("family".into(), Json::Object(fam)),
        ("loaded".into(), Json::Bool(r.is_loaded())),
        ("default_flags".into(), ji(r.default_record_flags())),
    ];
    let mut want = op.clone();
    if let Json::Object(kv) = &mut want {
        kv.retain(|(k, _)| k != "op" && k != "note");
    }
    check_state(&got, &want)
}

/// `flora_expect_rand`: `calls` since the simulator was created or loaded,
/// or since the last check.
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

/// `flora_expect_refs`: `refs` (`{"id": count}`).
fn expect_refs(w: &World, op: &Json) -> R<()> {
    let refs = w.refs.borrow();
    for (k, v) in field(op, "refs")?.as_object().ok_or("'refs' must be an object")? {
        let id = Json::Str(k.clone()).as_i64().ok_or("bad occupant id")? as u64;
        let got = refs.get(&id).copied().unwrap_or(0);
        let want = i64_of(v, "refs")?;
        if got != want {
            return Err(format!("occupant {id}: {got} references, expected {want}"));
        }
    }
    Ok(())
}

/// `flora_expect_segment`: `segment`, `count?` (records), `records?`
/// (`[{key: [t, g, i], u32? | flora_type? …}]`).
fn expect_segment(w: &World, op: &Json) -> R<()> {
    let seg = u64::from(u32_of(field(op, "segment")?, "segment")?);
    let segs = w.segments.borrow();
    let empty = BTreeMap::new();
    let recs = segs.get(&seg).unwrap_or(&empty);
    if let Some(c) = opt(op, "count") {
        if recs.len() as i64 != i64_of(c, "count")? {
            return Err(format!("{} records", recs.len()));
        }
    }
    if let Some(list) = opt(op, "records") {
        for e in arr(list, "records")? {
            let k = arr(field(e, "key")?, "key")?;
            if k.len() != 3 {
                return Err("'key' must be [type, group, instance]".into());
            }
            let key = ResKey {
                type_id: u32_of(&k[0], "key")?,
                group: u32_of(&k[1], "key")?,
                instance: u32_of(&k[2], "key")?,
            };
            let rec = recs.get(&key).ok_or_else(|| format!("no record {key:?}"))?;
            match rec {
                SegRec::U32(v) => {
                    if let Some(want) = opt(e, "u32") {
                        if i64::from(*v) != i64_of(want, "u32")? {
                            return Err(format!("record {key:?} = {v}"));
                        }
                    }
                }
                SegRec::Occ(o) => {
                    if let Some(want) = opt(e, "flora_type") {
                        let got = o.flora.map_or(0, |f| f.flora_type);
                        if i64::from(got) != i64_of(want, "flora_type")? {
                            return Err(format!("record {key:?} flora type {got}"));
                        }
                    }
                    if opt(e, "u32").is_some() {
                        return Err(format!("record {key:?} holds an occupant"));
                    }
                }
            }
        }
    }
    Ok(())
}

// ---- persistence ------------------------------------------------------------------------------------

fn slot(op: &Json) -> R<String> {
    Ok(str_of(field(op, "slot")?, "slot")?.to_owned())
}

fn save_now(w: &World) -> R<SavedCity> {
    let s = w.sim()?;
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut root = ByteWriter::with_registry(reg.clone());
    root.put_ref(&RefValue::Object(s as Rc<dyn SaveObject>));
    let mut pkg = Package::new();
    let rep = save_store(&reg, &mut pkg);
    if rep.aborted || rep.entries_discarded != 0 {
        return Err(format!("save failed: {rep:?}"));
    }
    let hosts = reg
        .borrow()
        .entries
        .iter()
        .filter(|e| e.2.is_none())
        .map(|e| (e.1, e.3))
        .collect();
    Ok(SavedCity {
        pkg,
        root: root.into_bytes(),
        hosts,
    })
}

fn save(w: &World, op: &Json) -> R<()> {
    let s = save_now(w)?;
    w.saves.borrow_mut().insert(slot(op)?, s);
    Ok(())
}

/// `flora_load`: `slot`, `rng?`, `expect?` (clean store load), `crt_seed?`.
fn load(w: &World, op: &Json) -> R<()> {
    let saved = w.saves.borrow().get(&slot(op)?).cloned().ok_or("no such slot")?;
    let me = w.rc();
    let mut rng = Some(w.rng(opt(op, "rng"))?);
    let mut factory = |class: u32, object_id: u32| -> Option<RefValue> {
        match class {
            CLSID_FLORA_SIMULATOR => {
                let r = rng.take()?;
                let s = FloraSim::new(me.clone() as Rc<dyn FloraHost>, me.registry(), me.token(), r);
                Some(RefValue::Object(s as Rc<dyn SaveObject>))
            }
            OCCUPANT_CLASS => {
                let token = *saved.hosts.get(&object_id)?;
                me.add_ref(token);
                Some(RefValue::Host { token, class_id: class })
            }
            _ => None,
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
    let sim = root
        .as_ref()
        .and_then(|v| v.object::<FloraSim>())
        .ok_or("the root did not resolve to a flora simulator")?;
    if let Some(seed) = opt(op, "crt_seed") {
        sim.set_crt(CrtRand::new(u32_of(seed, "crt_seed")?));
    }
    w.crt_mark.set(sim.crt());
    *w.sim.borrow_mut() = Some(sim);
    Ok(())
}

fn expect_save(w: &World, op: &Json) -> R<()> {
    let want = w.saves.borrow().get(&slot(op)?).cloned().ok_or("no such slot")?;
    let got = save_now(w)?;
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
    let r = scalar::patch(&mut tmp, CLSID_FLORA_SIMULATOR, op);
    s.pkg = tmp.pkg;
    r
}

fn payload_of(w: &World, op: &Json) -> R<Vec<u8>> {
    let saves = w.saves.borrow();
    let s = saves.get(&slot(op)?).ok_or("no such slot")?;
    let tmp = scalar::Saved {
        pkg: s.pkg.clone(),
        root: Vec::new(),
    };
    scalar::entry_payload(&tmp, CLSID_FLORA_SIMULATOR, 0)
}

/// `flora_read`: calls the current simulator's `Read` directly on a record
/// payload (`slot`, or `hex`) without a reference resolver; `expect` is its
/// result; `consumed?` the bytes it read.
fn read_record(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let payload = match opt(op, "hex") {
        Some(h) => hex_decode(str_of(h, "hex")?)?,
        None => payload_of(w, op)?,
    };
    let mut r = ByteReader::new(&payload);
    check_expect(op, Json::Bool(SaveObject::read(&*s, &mut r)))?;
    if let Some(c) = opt(op, "consumed") {
        if r.position() as i64 != i64_of(c, "consumed")? {
            return Err(format!("Read consumed {} bytes", r.position()));
        }
    }
    Ok(())
}

/// `flora_expect_payload`: `slot`, `hex?`, `at?` (`[{offset, hex}]`),
/// `length?`.
fn expect_payload(w: &World, op: &Json) -> R<()> {
    let got = payload_of(w, op)?;
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
