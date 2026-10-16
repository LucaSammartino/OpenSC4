// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/prop-lotconfig.md`: the prop manager
//! (`pm_…`, Part A) and the lot configuration manager (`lcm_…`, Part B),
//! each over one scripted city. See README.md ("Prop and lot configuration
//! manager ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use fc4_core::lot_config_manager::{
    self as lcm, lot_object_array, BuildingInfo, LcmService, LotConfigCatalogue, LotConfigHost, LotConfigManager,
    LotConfigObject, SharedLotConfigCatalogue, LOT_CONFIG_GROUP,
};
use fc4_core::prop_manager::{
    self as pm, BuildingProps, BuiltinProp, CellRect, PropCatalogue, PropHost, PropManager, PropProfile, PropService,
    SharedPropCatalogue, FLAG_INSERTED, FLAG_SHOWN,
};
use fc4_core::property::PropertyHolder;
use fc4_core::props::{EXEMPLAR_TYPE, PROP_EXEMPLAR_TYPE};
use fc4_core::stl::StlHashSet;
use fc4_core::stream::{ByteReader, ByteWriter, RefResolver, RefValue, ResKey, SaveObject, SaveRegistry};
use fc4_core::variant::{mem, Elem, Variant};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, check_state, f32_of, holder_of, jf, ji, opt, str_of, u32_of, EventLog, R,
};

/// The services' tokens and the classes their references are written with.
const PM_OM: u64 = 0x50;
const PM_LM: u64 = 0x51;
const PM_RM: u64 = 0x52;
const PM_VF: u64 = 0x53;
const PM_ME: u64 = 0xAAAA;
const LCM_BDS: u64 = 0x50;
const LCM_FLORA: u64 = 0x51;
const LCM_PM: u64 = 0x52;
const LCM_ME: u64 = 0xAAAA;

fn u32s(j: &Json, what: &str) -> R<Vec<u32>> {
    arr(j, what)?.iter().map(|v| u32_of(v, what)).collect()
}

fn f32s(j: &Json, what: &str) -> R<Vec<f32>> {
    arr(j, what)?.iter().map(|v| f32_of(v, what)).collect()
}

fn f3(j: &Json, what: &str) -> R<[f32; 3]> {
    let v = f32s(j, what)?;
    match v[..] {
        [a, b, c] => Ok([a, b, c]),
        _ => Err(format!("'{what}' must have three numbers")),
    }
}

fn tok(j: &Json, what: &str) -> R<u64> {
    j.as_i64()
        .map(|v| v as u64)
        .ok_or_else(|| format!("'{what}' must be a token"))
}

fn ju(v: u32) -> Json {
    ji(i64::from(v))
}

fn jt(v: u64) -> Json {
    ji(v as i64)
}

fn jlist(v: &[u64]) -> Json {
    Json::Array(v.iter().map(|&t| jt(t)).collect())
}

fn jids(v: &[u32]) -> Json {
    Json::Array(v.iter().map(|&t| ju(t)).collect())
}

fn obj(kv: Vec<(&str, Json)>) -> Json {
    Json::Object(kv.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}

fn u32_array(id: u32, v: &[u32]) -> (u32, Variant) {
    (
        id,
        Variant::array(mem::UINT32, v.iter().map(|&x| Elem::U32(x)).collect()),
    )
}

/// Resolves object `n` of a record to the token `objects[n]` (default
/// `0x4F + n`).
struct Resolver(BTreeMap<u32, u64>);

impl RefResolver for Resolver {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        let token = self.0.get(&object_id).copied().unwrap_or(0x4F + u64::from(object_id));
        Some(RefValue::Host { token, class_id })
    }
}

fn resolver(op: &Json) -> R<Resolver> {
    let mut m = BTreeMap::new();
    if let Some(o) = opt(op, "objects") {
        for (k, v) in o.as_object().ok_or("'objects' must be an object")? {
            let id = Json::Str(k.clone()).as_i64().ok_or("bad object ID")? as u32;
            m.insert(id, tok(v, "objects")?);
        }
    }
    Ok(Resolver(m))
}

fn write_record(obj: &dyn SaveObject) -> (bool, Vec<u8>) {
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut w = ByteWriter::with_registry(reg);
    let ok = obj.write(&mut w);
    (ok, w.into_bytes())
}

fn expect_hex(op: &Json, got: &[u8]) -> R<()> {
    let want = hex_decode(str_of(field(op, "hex")?, "hex")?)?;
    if got != want {
        return Err(format!("record {}, expected {}", hex_encode(got), hex_encode(&want)));
    }
    Ok(())
}

fn refs_check(op: &Json, refs: &BTreeMap<u64, i64>, label: &dyn Fn(&str) -> Option<u64>) -> R<()> {
    let want = field(op, "refs")?.as_object().ok_or("'refs' must be an object")?;
    for (k, v) in want {
        let t = match Json::Str(k.clone()).as_i64() {
            Some(t) => t as u64,
            None => label(k).ok_or_else(|| format!("unknown label '{k}'"))?,
        };
        let got = refs.get(&t).copied().unwrap_or(0);
        let w = v.as_i64().ok_or("reference counts must be integers")?;
        if got != w {
            return Err(format!("references of {k}: {got}, expected {w}"));
        }
    }
    Ok(())
}

// ==== Part A: the prop manager ======================================================================

#[derive(Clone, Debug, Default)]
struct Occ {
    flags: u32,
    pos: [f32; 3],
    usage: u32,
    prop_type: u32,
    bbox: Option<([f32; 3], [f32; 3])>,
    builtin: Option<BuiltinProp>,
    class_id: u32,
    inits: u32,
    shutdowns: u32,
}

/// Buildings: token → (cell, props property).
type Buildings = BTreeMap<u64, ((i32, i32), BuildingProps)>;
/// Per configuration ID: (families, size).
type Meta = BTreeMap<u32, (Vec<u32>, (u32, u32))>;
/// Building type → (capacity, footprint, exemplar).
type BuildingTypes = BTreeMap<u32, (u16, Option<(f32, f32)>, Option<Rc<PropertyHolder>>)>;

#[derive(Default)]
struct PmWorld {
    events: EventLog,
    refs: RefCell<BTreeMap<u64, i64>>,
    exemplars: RefCell<Vec<(ResKey, Rc<PropertyHolder>)>>,
    no_resource_manager: Cell<bool>,
    no_sink: Cell<bool>,
    no_services: Cell<bool>,
    gzcom_classes: RefCell<BTreeSet<u32>>,
    next: Cell<u64>,
    occs: RefCell<BTreeMap<u64, Occ>>,
    lots: RefCell<BTreeMap<u64, CellRect>>,
    buildings: RefCell<Buildings>,
    catalogue: RefCell<Option<SharedPropCatalogue>>,
    manager: RefCell<Option<Rc<PropManager>>>,
}

impl PmWorld {
    fn new() -> Rc<PmWorld> {
        let w = Rc::new(PmWorld::default());
        w.next.set(0x1000);
        w
    }
    fn bump(&self, t: u64, d: i64) {
        *self.refs.borrow_mut().entry(t).or_insert(0) += d;
    }
    fn new_token(&self) -> u64 {
        let t = self.next.get();
        self.next.set(t + 1);
        t
    }
    fn occ(&self, t: u64) -> Occ {
        self.occs.borrow().get(&t).cloned().unwrap_or_default()
    }
    fn with_occ(&self, t: u64, f: impl FnOnce(&mut Occ)) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&t) {
            f(o);
        }
    }
    fn inserted_with_usage(&self, u: u32) -> Vec<u64> {
        self.occs
            .borrow()
            .iter()
            .filter(|(_, o)| o.flags & FLAG_INSERTED != 0 && o.usage == u)
            .map(|(t, _)| *t)
            .collect()
    }
    fn m(&self) -> R<Rc<PropManager>> {
        self.manager
            .borrow()
            .clone()
            .ok_or_else(|| "no prop manager (pm_create)".into())
    }
}

fn cell_of(x: f32, z: f32) -> (i32, i32) {
    ((x / 16.0).floor() as i32, (z / 16.0).floor() as i32)
}

impl PropHost for PmWorld {
    fn exemplar_keys(&self) -> Option<Vec<ResKey>> {
        if self.no_resource_manager.get() {
            return None;
        }
        Some(self.exemplars.borrow().iter().map(|(k, _)| *k).collect())
    }
    fn load_exemplar(&self, key: ResKey) -> Option<Rc<PropertyHolder>> {
        self.exemplars
            .borrow()
            .iter()
            .rev()
            .find(|(k, _)| *k == key)
            .map(|(_, h)| h.clone())
    }
    fn register_shutdown_callback(&self) {
        self.events.push("shutdown_callback", vec![]);
    }
    fn post_exemplar_data_updated(&self, manager: u64, data2: u32) {
        self.events
            .push("post", vec![("manager", jt(manager)), ("data2", ju(data2))]);
    }
    fn register_command(&self, id: u32) {
        self.events.push("command", vec![("id", ju(id))]);
    }
    fn unregister_command(&self, id: u32) {
        self.events.push("uncommand", vec![("id", ju(id))]);
    }
    fn subscribe(&self, m: u32) {
        self.events.push("subscribe", vec![("type", ju(m))]);
    }
    fn unsubscribe(&self, m: u32) {
        self.events.push("unsubscribe", vec![("type", ju(m))]);
    }
    fn acquire_service(&self, s: PropService) -> Option<u64> {
        if self.no_services.get() {
            return None;
        }
        let t = match s {
            PropService::OccupantManager => PM_OM,
            PropService::ResourceManager => PM_RM,
            PropService::LotManager => PM_LM,
        };
        self.bump(t, 1);
        Some(t)
    }
    fn view_filter(&self) -> Option<u64> {
        self.bump(PM_VF, 1);
        Some(PM_VF)
    }
    fn add_ref(&self, t: u64) {
        self.bump(t, 1);
    }
    fn release(&self, t: u64) {
        self.bump(t, -1);
    }
    fn create_prop_instance(&self, class_id: u32) -> Option<u64> {
        if !self.gzcom_classes.borrow().contains(&class_id) {
            return None;
        }
        let t = self.new_token();
        self.occs.borrow_mut().insert(
            t,
            Occ {
                class_id,
                ..Occ::default()
            },
        );
        self.bump(t, 1);
        Some(t)
    }
    fn create_builtin_prop(&self, kind: BuiltinProp) -> u64 {
        let t = self.new_token();
        self.occs.borrow_mut().insert(
            t,
            Occ {
                builtin: Some(kind),
                ..Occ::default()
            },
        );
        self.bump(t, 1);
        t
    }
    fn set_prop_type(&self, p: u64, id: u32) {
        self.with_occ(p, |o| o.prop_type = id);
    }
    fn attach_exemplar(&self, _p: u64, key: ResKey, _global: bool) -> Option<Rc<PropertyHolder>> {
        if self.no_sink.get() {
            return None;
        }
        self.load_exemplar(key)
    }
    fn set_bounding_box(&self, p: u64, min: [f32; 3], max: [f32; 3]) {
        self.with_occ(p, |o| o.bbox = Some((min, max)));
    }
    fn occupant_flag(&self, occ: u64, flag: u32) -> bool {
        self.occ(occ).flags & flag != 0
    }
    fn occupant_init(&self, occ: u64) -> bool {
        self.with_occ(occ, |o| o.inits += 1);
        true
    }
    fn occupant_shutdown(&self, occ: u64) {
        self.with_occ(occ, |o| o.shutdowns += 1);
    }
    fn set_prop_usage(&self, occ: u64, u: u32) {
        self.with_occ(occ, |o| o.usage = u);
    }
    fn set_position(&self, occ: u64, pos: [f32; 3]) {
        self.with_occ(occ, |o| o.pos = pos);
    }
    fn set_orientation(&self, _occ: u64, _o: i32) {}
    fn position(&self, occ: u64) -> [f32; 3] {
        self.occ(occ).pos
    }
    fn hide(&self, occ: u64) {
        self.with_occ(occ, |o| o.flags &= !FLAG_SHOWN);
        self.events.push("hide", vec![("occupant", jt(occ))]);
    }
    fn insert_occupant(&self, occ: u64) -> bool {
        self.with_occ(occ, |o| o.flags |= FLAG_INSERTED);
        self.bump(occ, 1);
        true
    }
    fn remove_occupant(&self, occ: u64) {
        self.with_occ(occ, |o| o.flags &= !FLAG_INSERTED);
        self.bump(occ, -1);
        self.events.push("remove", vec![("occupant", jt(occ))]);
    }
    fn occupants_in_box(&self, min: [f32; 3], max: [f32; 3], u: u32) -> Option<Vec<u64>> {
        let v: Vec<u64> = self
            .inserted_with_usage(u)
            .into_iter()
            .filter(|t| {
                let p = self.occ(*t).pos;
                min[0] <= p[0] && p[0] <= max[0] && min[2] <= p[2] && p[2] <= max[2]
            })
            .collect();
        for t in &v {
            self.bump(*t, 1);
        }
        Some(v)
    }
    fn occupants_in_cell_rect(&self, r: CellRect, u: u32) -> Option<Vec<u64>> {
        // The cells around the rectangle too: an occupant manager box query
        // also returns neighbours that touch it.
        let v: Vec<u64> = self
            .inserted_with_usage(u)
            .into_iter()
            .filter(|t| {
                let p = self.occ(*t).pos;
                let (cx, cz) = cell_of(p[0], p[2]);
                r.0 - 1 <= cx && cx <= r.2 + 1 && r.1 - 1 <= cz && cz <= r.3 + 1
            })
            .collect();
        for t in &v {
            self.bump(*t, 1);
        }
        Some(v)
    }
    fn building_at_cell(&self, x: i32, z: i32) -> Option<u64> {
        let b = self
            .buildings
            .borrow()
            .iter()
            .find(|(_, (c, _))| *c == (x, z))
            .map(|(t, _)| *t)?;
        self.bump(b, 1);
        Some(b)
    }
    fn lot_at_cell(&self, x: i32, z: i32) -> Option<u64> {
        self.lots
            .borrow()
            .iter()
            .find(|(_, r)| r.0 <= x && x <= r.2 && r.1 <= z && z <= r.3)
            .map(|(t, _)| *t)
    }
    fn lot_cell_rect(&self, lot: u64) -> CellRect {
        self.lots.borrow().get(&lot).copied().unwrap_or_default()
    }
    fn cell_to_world(&self, x: i32, z: i32) -> (f32, f32) {
        (x as f32 * 16.0, z as f32 * 16.0)
    }
    fn world_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        cell_of(x, z)
    }
    fn building_props(&self, b: u64) -> BuildingProps {
        self.buildings
            .borrow()
            .get(&b)
            .map_or(BuildingProps::NoHolder, |(_, p)| p.clone())
    }
    fn set_building_props(&self, b: u64, props: &[u64]) {
        if let Some((_, p)) = self.buildings.borrow_mut().get_mut(&b) {
            *p = BuildingProps::Present(props.to_vec());
        }
    }
    fn add_building_props_property(&self, b: u64, props: &[u64]) {
        self.events.push("add_property", vec![("building", jt(b))]);
        if let Some((_, p)) = self.buildings.borrow_mut().get_mut(&b) {
            *p = BuildingProps::Present(props.to_vec());
        }
    }
    fn remove_building_props_property(&self, b: u64) {
        self.events.push("remove_property", vec![("building", jt(b))]);
        if let Some((_, p)) = self.buildings.borrow_mut().get_mut(&b) {
            *p = BuildingProps::Absent;
        }
    }
    fn create_text_decal(&self) -> Option<u64> {
        let t = self.new_token();
        self.occs.borrow_mut().insert(t, Occ::default());
        self.bump(t, 1);
        Some(t)
    }
    fn terrain_altitude(&self, _x: f32, _z: f32) -> f32 {
        0.0
    }
    fn decal_set_text(&self, _d: u64, _text: &str) {}
    fn decal_slot8(&self, _d: u64, _v: [f32; 3]) {}
    fn decal_slot10(&self, _d: u64, _v: f32) {}
    fn decal_slot6(&self, _d: u64, _v: f32) {}
    fn object_ref(&self, t: u64) -> RefValue {
        let class_id = match t {
            PM_OM => 0x098F_964D,
            PM_LM => 0x498F_9B01,
            _ => 0,
        };
        RefValue::Host { token: t, class_id }
    }
    fn resolve_ref(&self, r: &RefValue, _iid: u32) -> Option<u64> {
        match r {
            RefValue::Host { token, .. } => {
                self.bump(*token, 1);
                Some(*token)
            }
            _ => None,
        }
    }
}

fn pm_world(host: &HostRef) -> Rc<PmWorld> {
    scalar::world(host, "prop_manager", PmWorld::new)
}

fn profile_json(p: &PropProfile) -> Json {
    let s = p.size();
    obj(vec![
        ("class", ju(p.class_id())),
        ("size", Json::Array(s.iter().map(|&v| jf(v)).collect())),
        ("satisfaction", ju(p.satisfaction())),
        ("wealth", ju(u32::from(p.wealth()))),
        ("crime", ju(p.crime_effect())),
    ])
}

fn builtin_name(b: Option<BuiltinProp>) -> Json {
    match b {
        None => Json::Null,
        Some(BuiltinProp::Plain) => Json::Str("plain".into()),
        Some(BuiltinProp::TerrainDecal) => Json::Str("terrain_decal".into()),
        Some(BuiltinProp::VisualEffect) => Json::Str("visual_effect".into()),
    }
}

fn pm_exemplar(w: &PmWorld, op: &Json) -> R<()> {
    let instance = u32_of(field(op, "instance")?, "instance")?;
    let mut h = match opt(op, "properties") {
        Some(p) => holder_of(p)?,
        None => PropertyHolder::new(),
    };
    let ty = match opt(op, "exemplar_type") {
        Some(t) => u32_of(t, "exemplar_type")?,
        None => pm::EXEMPLAR_TYPE_PROP,
    };
    h.add(PROP_EXEMPLAR_TYPE, Variant::Scalar(Elem::U32(ty)));
    if let Some(f) = opt(op, "families") {
        let (id, v) = u32_array(pm::prop::FAMILIES, &u32s(f, "families")?);
        h.add(id, v);
    }
    if let Some(s) = opt(op, "size") {
        let v = f32s(s, "size")?;
        h.add(
            pm::prop::OCCUPANT_SIZE,
            Variant::array(mem::FLOAT32, v.iter().map(|&x| Elem::F32(x)).collect()),
        );
    }
    if let Some(c) = opt(op, "class") {
        h.add(
            pm::prop::OCCUPANT_CLASS,
            Variant::Scalar(Elem::U32(u32_of(c, "class")?)),
        );
    }
    let key = ResKey {
        type_id: EXEMPLAR_TYPE,
        group: match opt(op, "group") {
            Some(g) => u32_of(g, "group")?,
            None => 0x1234,
        },
        instance,
    };
    w.exemplars.borrow_mut().push((key, Rc::new(h)));
    Ok(())
}

fn pm_env(w: &PmWorld, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "no_resource_manager") {
        w.no_resource_manager.set(bool_of(v, "no_resource_manager")?);
    }
    if let Some(v) = opt(op, "no_sink") {
        w.no_sink.set(bool_of(v, "no_sink")?);
    }
    if let Some(v) = opt(op, "no_services") {
        w.no_services.set(bool_of(v, "no_services")?);
    }
    if let Some(v) = opt(op, "gzcom_classes") {
        *w.gzcom_classes.borrow_mut() = u32s(v, "gzcom_classes")?.into_iter().collect();
    }
    Ok(())
}

fn pm_occupant(w: &PmWorld, op: &Json) -> R<()> {
    let t = tok(field(op, "occupant")?, "occupant")?;
    let pos = match opt(op, "pos") {
        Some(p) => f3(p, "pos")?,
        None => [0.0; 3],
    };
    let usage = match opt(op, "usage") {
        Some(u) => u32_of(u, "usage")?,
        None => 0,
    };
    let inserted = match opt(op, "inserted") {
        Some(v) => bool_of(v, "inserted")?,
        None => false,
    };
    let shown = match opt(op, "shown") {
        Some(v) => bool_of(v, "shown")?,
        None => false,
    };
    let flags = if inserted { FLAG_INSERTED } else { 0 } | if shown { FLAG_SHOWN } else { 0 };
    w.occs.borrow_mut().insert(
        t,
        Occ {
            pos,
            usage,
            flags,
            ..Occ::default()
        },
    );
    if let Some(n) = opt(op, "refs") {
        w.refs
            .borrow_mut()
            .insert(t, n.as_i64().ok_or("'refs' must be an integer")?);
    }
    Ok(())
}

fn pm_building(w: &PmWorld, op: &Json) -> R<()> {
    let b = tok(field(op, "building")?, "building")?;
    let c = u32s(field(op, "cell")?, "cell")?;
    let props = match op.get("props") {
        None | Some(Json::Null) => BuildingProps::NoHolder,
        Some(Json::Str(s)) if s == "absent" => BuildingProps::Absent,
        Some(v) => BuildingProps::Present(arr(v, "props")?.iter().map(|t| tok(t, "props")).collect::<R<_>>()?),
    };
    w.buildings.borrow_mut().insert(b, ((c[0] as i32, c[1] as i32), props));
    Ok(())
}

fn building_props_json(p: &BuildingProps) -> Json {
    match p {
        BuildingProps::NoHolder => Json::Null,
        BuildingProps::Absent => Json::Str("absent".into()),
        BuildingProps::Present(v) => jlist(v),
    }
}

fn opt_tok(j: &Json, what: &str) -> R<Option<u64>> {
    match j {
        Json::Null => Ok(None),
        v => tok(v, what).map(Some),
    }
}

fn pm_call(w: &PmWorld, op: &Json) -> R<()> {
    let m = w.m()?;
    let method = str_of(field(op, "method")?, "method")?;
    let mut list: Vec<u64> = match opt(op, "list") {
        Some(l) => arr(l, "list")?.iter().map(|t| tok(t, "list")).collect::<R<_>>()?,
        None => Vec::new(),
    };
    let a_u32 = |i| arg(op, i).and_then(|v| u32_of(v, "argument"));
    let a_i32 = |i| a_u32(i).map(|v| v as i32);
    let a_tok = |i| arg(op, i).and_then(|v| tok(v, "argument"));
    let a_opt = |i| arg(op, i).and_then(|v| opt_tok(v, "argument"));
    let with_list = |r: bool, l: &[u64]| obj(vec![("result", Json::Bool(r)), ("list", jlist(l))]);
    let got = match method {
        "is_prop_type_valid" => Json::Bool(m.is_prop_type_valid(a_u32(0)?)),
        "is_prop_family_type_valid" => Json::Bool(m.is_prop_family_type_valid(a_u32(0)?)),
        "is_prop_in_family" => Json::Bool(m.is_prop_in_family(a_u32(0)?, a_u32(1)?)),
        "prop_types_by_family" => {
            let mut v: Vec<u32> = match opt(op, "vec") {
                Some(v) => u32s(v, "vec")?,
                None => Vec::new(),
            };
            let n = m.prop_types_by_family(&mut v, a_u32(0)?);
            obj(vec![("result", ju(n)), ("vec", jids(&v))])
        }
        "all_prop_types" => {
            let mut v = m.all_prop_types();
            v.sort_unstable();
            jids(&v)
        }
        "all_prop_family_types" => {
            let mut v = m.all_prop_family_types();
            v.sort_unstable();
            jids(&v)
        }
        "prop_key_from_type" => match m.prop_key_from_type(a_u32(0)?) {
            Some(k) => jids(&[k.type_id, k.group, k.instance]),
            None => Json::Null,
        },
        "prop_name" => Json::Str(m.prop_name(a_u32(0)?)),
        "prop_profile" => profile_json(&m.prop_profile(a_u32(0)?)),
        "prop_profile_ptr" => m.prop_profile_ptr(a_u32(0)?).map_or(Json::Null, |p| profile_json(&p)),
        "create_prop" => m.create_prop(a_u32(0)?).map_or(Json::Null, jt),
        "add_city_prop" => Json::Bool(m.add_city_prop(a_tok(0)?)),
        "add_lot_prop" => Json::Bool(m.add_lot_prop(a_opt(0)?, a_tok(1)?)),
        "add_building_prop" => Json::Bool(m.add_building_prop(a_tok(0)?, a_tok(1)?)),
        "get_lot_props" => {
            let r = m.get_lot_props(&mut list, a_opt(0)?);
            with_list(r, &list)
        }
        "get_lot_props_at" => {
            let r = m.get_lot_props_at(&mut list, a_i32(0)?, a_i32(1)?);
            with_list(r, &list)
        }
        "get_street_props_at" => {
            let r = m.get_street_props_at(&mut list, a_i32(0)?, a_i32(1)?);
            with_list(r, &list)
        }
        "get_building_props" => {
            let r = m.get_building_props(&mut list, a_opt(0)?);
            with_list(r, &list)
        }
        "get_building_props_at" => {
            let r = m.get_building_props_at(&mut list, a_i32(0)?, a_i32(1)?);
            with_list(r, &list)
        }
        "remove_building_prop" => Json::Bool(m.remove_building_prop(a_opt(0)?, a_tok(1)?)),
        "remove_building_props" => Json::Bool(m.remove_building_props(a_opt(0)?)),
        "remove_lot_props" => Json::Bool(m.remove_lot_props(a_opt(0)?)),
        "remove_prop" => Json::Bool(m.remove_prop(a_tok(0)?)),
        "remove_street_props_from_tile" => {
            m.remove_street_props_from_tile(a_i32(0)?, a_i32(1)?);
            Json::Null
        }
        other => return Err(format!("unknown prop manager method '{other}'")),
    };
    check_expect(op, got)
}

fn pm_expect_occupant(w: &PmWorld, op: &Json) -> R<()> {
    let t = tok(field(op, "occupant")?, "occupant")?;
    let o = w
        .occs
        .borrow()
        .get(&t)
        .cloned()
        .ok_or_else(|| format!("no occupant {t:#X}"))?;
    let bbox = o.bbox.map_or(Json::Null, |(a, b)| {
        Json::Array(vec![
            Json::Array(a.iter().map(|&v| jf(v)).collect()),
            Json::Array(b.iter().map(|&v| jf(v)).collect()),
        ])
    });
    let got = vec![
        ("prop_type".to_owned(), ju(o.prop_type)),
        ("bbox".to_owned(), bbox),
        ("builtin".to_owned(), builtin_name(o.builtin)),
        ("class".to_owned(), ju(o.class_id)),
        ("usage".to_owned(), ju(o.usage)),
        ("inserted".to_owned(), Json::Bool(o.flags & FLAG_INSERTED != 0)),
        ("shown".to_owned(), Json::Bool(o.flags & FLAG_SHOWN != 0)),
        ("inits".to_owned(), ju(o.inits)),
        ("shutdowns".to_owned(), ju(o.shutdowns)),
        ("pos".to_owned(), Json::Array(o.pos.iter().map(|&v| jf(v)).collect())),
    ];
    check_state(&got, field(op, "state")?)
}

fn pm_expect(w: &PmWorld, op: &Json) -> R<()> {
    let s = w.m()?.snapshot();
    let o = |t: Option<u64>| t.map_or(Json::Null, jt);
    let (types, families, profiles) = w.m()?.catalogue().borrow().counts();
    let got = vec![
        ("initialized".to_owned(), Json::Bool(s.initialized)),
        ("occupant_manager".to_owned(), o(s.occupant_manager)),
        ("resource_manager".to_owned(), o(s.resource_manager)),
        ("lot_manager".to_owned(), o(s.lot_manager)),
        ("view_filter".to_owned(), o(s.view_filter)),
        ("filter".to_owned(), Json::Bool(s.filter)),
        ("types".to_owned(), ji(types as i64)),
        ("families".to_owned(), ji(families as i64)),
        ("profiles".to_owned(), ji(profiles as i64)),
    ];
    check_state(&got, field(op, "state")?)
}

fn pm_exec(host: &HostRef, rest: &str, op: &Json) -> R<()> {
    let w = pm_world(host);
    match rest {
        "env" => pm_env(&w, op),
        "exemplar" => pm_exemplar(&w, op),
        "occupant" => pm_occupant(&w, op),
        "lot" => {
            let l = tok(field(op, "lot")?, "lot")?;
            let r = u32s(field(op, "rect")?, "rect")?;
            if r.len() != 4 {
                return Err("'rect' must have four cells".into());
            }
            w.lots
                .borrow_mut()
                .insert(l, (r[0] as i32, r[1] as i32, r[2] as i32, r[3] as i32));
            Ok(())
        }
        "building" => pm_building(&w, op),
        "create" => {
            let keep = match opt(op, "same_catalogue") {
                Some(v) => bool_of(v, "same_catalogue")?,
                None => false,
            };
            let cat = match (keep, w.catalogue.borrow().clone()) {
                (true, Some(c)) => c,
                _ => PropCatalogue::shared(),
            };
            *w.catalogue.borrow_mut() = Some(cat.clone());
            *w.manager.borrow_mut() = Some(PropManager::new(w.clone(), cat, PM_ME));
            Ok(())
        }
        "init" => check_expect(op, Json::Bool(w.m()?.init())),
        "shutdown" => check_expect(op, Json::Bool(w.m()?.shutdown())),
        "call" => pm_call(&w, op),
        "expect" => pm_expect(&w, op),
        "expect_occupant" => pm_expect_occupant(&w, op),
        "expect_building" => {
            let b = tok(field(op, "building")?, "building")?;
            let got = building_props_json(&w.building_props(b));
            let want = field(op, "props")?;
            if scalar::matches(&got, want, false) {
                Ok(())
            } else {
                Err(format!(
                    "building props {}, expected {}",
                    scalar::show(&got),
                    scalar::show(want)
                ))
            }
        }
        "expect_refs" => refs_check(op, &w.refs.borrow(), &|_| None),
        "record" => {
            let (ok, bytes) = write_record(&*w.m()?);
            if !ok {
                return Err("Write returned false".into());
            }
            expect_hex(op, &bytes)
        }
        "read" => {
            let bytes = hex_decode(str_of(field(op, "hex")?, "hex")?)?;
            let mut res = resolver(op)?;
            let ok = w.m()?.read(&mut ByteReader::with_resolver(&bytes, &mut res));
            check_expect(op, Json::Bool(ok))
        }
        "expect_events" => w.events.expect_events(op),
        "expect_event_count" => w.events.expect_count(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }
        other => Err(format!("unknown op 'pm_{other}'")),
    }
}

// ==== Part B: the lot configuration manager =======================================================

#[derive(Clone, Debug, Default)]
struct Cfg {
    id: u32,
    families: Vec<u32>,
    size: (u32, u32),
    objects: Vec<LotConfigObject>,
    info: Option<BuildingInfo>,
    walls: Vec<u32>,
    generic_for: Option<u32>,
}

#[derive(Default)]
struct LcmWorld {
    events: EventLog,
    refs: RefCell<BTreeMap<u64, i64>>,
    detail: Cell<u32>,
    keys: RefCell<Vec<(ResKey, Rc<PropertyHolder>)>>,
    meta: RefCell<Meta>,
    no_resource_manager: Cell<bool>,
    next: Cell<u64>,
    serial: Cell<u32>,
    configs: RefCell<BTreeMap<u64, Cfg>>,
    labels: RefCell<BTreeMap<String, u64>>,
    filters: RefCell<BTreeMap<u64, BTreeSet<u32>>>,
    building_types: RefCell<BuildingTypes>,
    building_families: RefCell<BTreeMap<u32, Vec<u32>>>,
    flora_types: RefCell<BTreeMap<u32, f32>>,
    flora_families: RefCell<BTreeMap<u32, Vec<u32>>>,
    prop_types: RefCell<BTreeMap<u32, (f32, f32)>>,
    prop_families: RefCell<BTreeMap<u32, Vec<u32>>>,
    walls_valid: RefCell<BTreeSet<u32>>,
    catalogue: RefCell<Option<SharedLotConfigCatalogue>>,
    manager: RefCell<Option<Rc<LotConfigManager>>>,
}

impl LcmWorld {
    fn new() -> Rc<LcmWorld> {
        let w = Rc::new(LcmWorld::default());
        w.next.set(0x1000);
        w.detail.set(3);
        w
    }
    fn bump(&self, t: u64, d: i64) {
        *self.refs.borrow_mut().entry(t).or_insert(0) += d;
    }
    fn new_token(&self) -> u64 {
        let t = self.next.get();
        self.next.set(t + 1);
        t
    }
    fn cfg(&self, t: u64) -> Cfg {
        self.configs.borrow().get(&t).cloned().unwrap_or_default()
    }
    fn with_cfg(&self, t: u64, f: impl FnOnce(&mut Cfg)) {
        if let Some(c) = self.configs.borrow_mut().get_mut(&t) {
            f(c);
        }
    }
    fn m(&self) -> R<Rc<LotConfigManager>> {
        self.manager
            .borrow()
            .clone()
            .ok_or_else(|| "no lot configuration manager (lcm_create)".into())
    }
    fn label(&self, l: &str) -> Option<u64> {
        self.labels.borrow().get(l).copied()
    }
    /// A configuration argument: a label, a token or null.
    fn cfg_arg(&self, j: &Json) -> R<Option<u64>> {
        match j {
            Json::Null => Ok(None),
            Json::Str(s) if Json::Str(s.clone()).as_i64().is_none() => self
                .label(s)
                .map(Some)
                .ok_or_else(|| format!("unknown configuration '{s}'")),
            v => tok(v, "configuration").map(Some),
        }
    }
    fn ids_of(&self, tokens: &[u64]) -> Json {
        jids(&tokens.iter().map(|&t| self.cfg(t).id).collect::<Vec<_>>())
    }
}

impl LotConfigHost for LcmWorld {
    fn detail_level(&self) -> u32 {
        self.detail.get()
    }
    fn exemplar_keys(&self) -> Option<Vec<ResKey>> {
        if self.no_resource_manager.get() {
            return None;
        }
        Some(self.keys.borrow().iter().map(|(k, _)| *k).collect())
    }
    fn register_shutdown_callback(&self) {
        self.events.push("register_callback", vec![]);
    }
    fn unregister_shutdown_callback(&self) {
        self.events.push("unregister_callback", vec![]);
    }
    fn create_configuration(&self) -> u64 {
        let t = self.new_token();
        self.bump(t, 1);
        self.configs.borrow_mut().insert(t, Cfg::default());
        t
    }
    fn load_configuration(&self, cfg: u64, key: ResKey, detail: u32) -> bool {
        let h = self
            .keys
            .borrow()
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, h)| h.clone());
        let Some(h) = h else {
            return false;
        };
        let mut serial = self.serial.get();
        let objects = lot_object_array(&h, detail, &mut serial);
        self.serial.set(serial);
        let (families, size) = self.meta.borrow().get(&key.instance).cloned().unwrap_or_default();
        self.with_cfg(cfg, |c| {
            c.id = key.instance;
            c.families = families;
            c.size = size;
            c.objects = objects;
        });
        true
    }
    fn config_id(&self, cfg: u64) -> u32 {
        self.cfg(cfg).id
    }
    fn config_belongs_to_family(&self, cfg: u64, family: u32) -> bool {
        self.cfg(cfg).families.contains(&family)
    }
    fn config_families(&self, cfg: u64) -> Vec<u32> {
        self.cfg(cfg).families
    }
    fn config_size(&self, cfg: u64) -> (u32, u32) {
        self.cfg(cfg).size
    }
    fn config_objects(&self, cfg: u64) -> Vec<LotConfigObject> {
        self.cfg(cfg).objects
    }
    fn set_config_objects(&self, cfg: u64, objects: Vec<LotConfigObject>) {
        self.with_cfg(cfg, |c| c.objects = objects);
    }
    fn config_primary_building(&self, cfg: u64) -> Option<LotConfigObject> {
        self.cfg(cfg)
            .objects
            .into_iter()
            .find(|o| u32::from(o.kind) == lcm::kind::BUILDING)
    }
    fn set_config_building_info(&self, cfg: u64, info: &BuildingInfo) {
        self.with_cfg(cfg, |c| c.info = Some(info.clone()));
    }
    fn config_retaining_walls(&self, cfg: u64) -> Vec<u32> {
        self.cfg(cfg).walls
    }
    fn set_config_retaining_walls(&self, cfg: u64, ids: Vec<u32>) {
        self.with_cfg(cfg, |c| c.walls = ids);
    }
    fn is_retaining_wall_valid(&self, id: u32) -> bool {
        self.walls_valid.borrow().contains(&id)
    }
    fn create_generic_configuration(&self, ty: u32, size: (u32, u32), objects: Vec<LotConfigObject>) -> Option<u64> {
        let t = self.new_token();
        self.bump(t, 1);
        self.configs.borrow_mut().insert(
            t,
            Cfg {
                size,
                objects,
                generic_for: Some(ty),
                ..Cfg::default()
            },
        );
        Some(t)
    }
    fn acquire_service(&self, service: LcmService) -> Option<u64> {
        let t = match service {
            LcmService::BuildingDevelopment => LCM_BDS,
            LcmService::Flora => LCM_FLORA,
            LcmService::PropManager => LCM_PM,
        };
        self.bump(t, 1);
        Some(t)
    }
    fn subscribe(&self, m: u32) {
        self.events.push("subscribe", vec![("type", ju(m))]);
    }
    fn unsubscribe(&self, m: u32) {
        self.events.push("unsubscribe", vec![("type", ju(m))]);
    }
    fn add_ref(&self, t: u64) {
        self.bump(t, 1);
    }
    fn release(&self, t: u64) {
        self.bump(t, -1);
    }
    fn filter_matches(&self, filter: u64, cfg: u64) -> bool {
        let id = self.cfg(cfg).id;
        self.filters.borrow().get(&filter).is_some_and(|s| s.contains(&id))
    }
    fn building_exemplar(&self, ty: u32) -> Option<Rc<PropertyHolder>> {
        self.building_types.borrow().get(&ty).and_then(|t| t.2.clone())
    }
    fn is_building_type_valid(&self, id: u32) -> bool {
        self.building_types.borrow().contains_key(&id)
    }
    fn is_building_family_type_valid(&self, id: u32) -> bool {
        self.is_building_type_valid(id) || self.building_families.borrow().contains_key(&id)
    }
    fn building_types_by_family(&self, vec: &mut Vec<u32>, id: u32) -> u32 {
        if let Some(f) = self.building_families.borrow().get(&id) {
            *vec = f.clone();
            return f.len() as u32;
        }
        if self.is_building_type_valid(id) {
            *vec = vec![id];
        }
        0
    }
    fn building_types_into_set(&self, set: &mut StlHashSet, id: u32) -> u32 {
        let f = self.building_families.borrow().get(&id).cloned().unwrap_or_default();
        for &t in &f {
            set.insert(t);
        }
        f.len() as u32
    }
    fn is_building_in_family(&self, ty: u32, family: u32) -> bool {
        self.building_families
            .borrow()
            .get(&family)
            .is_some_and(|f| f.contains(&ty))
    }
    fn building_capacity(&self, ty: u32) -> u16 {
        self.building_types.borrow().get(&ty).map_or(0, |t| t.0)
    }
    fn building_footprint(&self, ty: u32) -> Option<(f32, f32)> {
        self.building_types.borrow().get(&ty).and_then(|t| t.1)
    }
    fn flora_radius(&self, id: u32) -> Option<f32> {
        self.flora_types.borrow().get(&id).copied()
    }
    fn is_flora_type_valid(&self, id: u32) -> bool {
        self.flora_types.borrow().contains_key(&id)
    }
    fn flora_types_into_set(&self, set: &mut StlHashSet, id: u32) -> u32 {
        let f = self.flora_families.borrow().get(&id).cloned().unwrap_or_default();
        for &t in &f {
            set.insert(t);
        }
        f.len() as u32
    }
    fn is_prop_type_valid(&self, id: u32) -> bool {
        self.prop_types.borrow().contains_key(&id)
    }
    fn prop_types_by_family(&self, vec: &mut Vec<u32>, id: u32) -> u32 {
        if let Some(f) = self.prop_families.borrow().get(&id) {
            *vec = f.clone();
            return f.len() as u32;
        }
        if self.is_prop_type_valid(id) {
            *vec = vec![id];
        }
        0
    }
    fn is_prop_family_type_valid(&self, id: u32) -> bool {
        self.is_prop_type_valid(id) || self.prop_families.borrow().contains_key(&id)
    }
    fn prop_size(&self, id: u32) -> Option<(f32, f32)> {
        self.prop_types.borrow().get(&id).copied()
    }
    fn object_ref(&self, t: u64) -> RefValue {
        let class_id = match t {
            LCM_BDS => 0x8990_BE7A,
            LCM_FLORA => 0xC990_C034,
            LCM_PM => 0xA990_BD10,
            _ => 0,
        };
        RefValue::Host { token: t, class_id }
    }
    fn resolve_ref(&self, r: &RefValue, _iid: u32) -> Option<u64> {
        match r {
            RefValue::Host { token, .. } => {
                self.bump(*token, 1);
                Some(*token)
            }
            _ => None,
        }
    }
}

fn lcm_world(host: &HostRef) -> Rc<LcmWorld> {
    scalar::world(host, "lot_config_manager", LcmWorld::new)
}

fn lot_object_holder(op: &Json) -> R<PropertyHolder> {
    let mut h = PropertyHolder::new();
    match op.get("version") {
        Some(Json::Null) => {}
        Some(v) => h.add(lcm::prop::VERSION, Variant::Scalar(Elem::U32(u32_of(v, "version")?))),
        None => h.add(lcm::prop::VERSION, Variant::Scalar(Elem::U32(2))),
    }
    for (i, v) in arr(field(op, "objects")?, "objects")?.iter().enumerate() {
        let (id, var) = u32_array(lcm::prop::LOT_OBJECT_FIRST + i as u32, &u32s(v, "objects")?);
        h.add(id, var);
    }
    Ok(h)
}

/// A configuration object: `{kind, ids, rect?: [min_x, min_z, max_x, max_z],
/// centre?: [x, z], orientation?}`.
fn config_object(j: &Json) -> R<LotConfigObject> {
    let mut o = LotConfigObject {
        kind: u32_of(field(j, "kind")?, "kind")? as u8,
        ids: match opt(j, "ids") {
            Some(v) => u32s(v, "ids")?,
            None => Vec::new(),
        },
        ..LotConfigObject::default()
    };
    if let Some(r) = opt(j, "rect") {
        let r = f32s(r, "rect")?;
        (o.min_x, o.min_z, o.max_x, o.max_z) = (r[0], r[1], r[2], r[3]);
    }
    if let Some(c) = opt(j, "centre") {
        let c = f32s(c, "centre")?;
        o.offset = [c[0], 0.0, c[1]];
    }
    if let Some(v) = opt(j, "orientation") {
        o.orientation = u32_of(v, "orientation")? as u8;
    }
    Ok(o)
}

fn object_json(o: &LotConfigObject) -> Json {
    obj(vec![
        ("kind", ju(u32::from(o.kind))),
        ("flags", ju(u32::from(o.flags))),
        ("ids", jids(&o.ids)),
        (
            "rect",
            Json::Array([o.min_x, o.min_z, o.max_x, o.max_z].iter().map(|&v| jf(v)).collect()),
        ),
        ("centre", Json::Array(vec![jf(o.offset[0]), jf(o.offset[2])])),
        ("orientation", ju(u32::from(o.orientation))),
    ])
}

fn rect_json(r: [f32; 4]) -> Json {
    Json::Array(r.iter().map(|&v| jf(v)).collect())
}

fn info_json(i: &Option<BuildingInfo>) -> Json {
    match i {
        None => Json::Null,
        Some(i) => obj(vec![
            ("min_capacity", ju(u32::from(i.min_capacity))),
            ("max_capacity", ju(u32::from(i.max_capacity))),
            ("groups", jids(&i.groups)),
            ("non_monopoly", Json::Bool(i.non_monopoly)),
        ]),
    }
}

fn lcm_types(w: &LcmWorld, kind: &str, op: &Json) -> R<()> {
    let id = u32_of(field(op, "type")?, "type")?;
    match kind {
        "building_type" => {
            let capacity = match opt(op, "capacity") {
                Some(v) => u32_of(v, "capacity")? as u16,
                None => 0,
            };
            let footprint = match opt(op, "footprint") {
                Some(v) => {
                    let f = f32s(v, "footprint")?;
                    Some((f[0], f[1]))
                }
                None => None,
            };
            let ex = match opt(op, "exemplar") {
                Some(p) => Some(Rc::new(holder_of(p)?)),
                None => None,
            };
            w.building_types.borrow_mut().insert(id, (capacity, footprint, ex));
        }
        "prop_type" => {
            let s = f32s(field(op, "size")?, "size")?;
            w.prop_types.borrow_mut().insert(id, (s[0], s[1]));
        }
        "flora_type" => {
            w.flora_types
                .borrow_mut()
                .insert(id, f32_of(field(op, "radius")?, "radius")?);
        }
        family => {
            let types = u32s(field(op, "types")?, "types")?;
            let map = match family {
                "building_family" => &w.building_families,
                "prop_family" => &w.prop_families,
                _ => &w.flora_families,
            };
            map.borrow_mut().insert(id, types);
        }
    }
    Ok(())
}

fn lcm_call(w: &LcmWorld, op: &Json) -> R<()> {
    let m = w.m()?;
    let method = str_of(field(op, "method")?, "method")?;
    let a_u32 = |i| arg(op, i).and_then(|v| u32_of(v, "argument"));
    let a_cfg = |i| arg(op, i).and_then(|v| w.cfg_arg(v));
    let a_ids = |i| arg(op, i).and_then(|v| u32s(v, "argument"));
    let a_filter = |i| {
        arg(op, i).and_then(|v| match v {
            Json::Null => Ok(None),
            v => tok(v, "filter").map(Some),
        })
    };
    let got = match method {
        "configuration_count" => ji(m.configuration_map().len() as i64),
        "object_count" => ji(m
            .configuration_map()
            .iter()
            .map(|&(_, t)| w.cfg(t).objects.len() as i64)
            .sum::<i64>()),
        "configuration_ids" => {
            let mut v: Vec<u32> = m.configuration_map().iter().map(|&(id, _)| id).collect();
            v.sort_unstable();
            jids(&v)
        }
        "family_id_set" => {
            let mut v = m.family_id_set();
            v.sort_unstable();
            jids(&v)
        }
        "configuration" => m.configuration(a_u32(0)?).map_or(Json::Null, |t| ju(w.cfg(t).id)),
        "add_configuration" => {
            let c = a_cfg(0)?.ok_or("add_configuration needs a configuration")?;
            Json::Bool(m.add_configuration(c, crate::runner::boolean(arg(op, 1)?).unwrap_or(false)))
        }
        "remove_configuration" => Json::Bool(m.remove_configuration(a_u32(0)?)),
        "configurations_by_family" => w.ids_of(&m.configurations_by_family(a_u32(0)?)),
        "configurations_by_size" => w.ids_of(&m.configurations_by_size(a_u32(0)?, a_u32(1)?)),
        "configurations_by_filter" => w.ids_of(&m.configurations_by_filter(a_filter(0)?)),
        "ids_by_family" => jids(&m.ids_by_family(a_u32(0)?)),
        "ids_by_size" => jids(&m.ids_by_size(a_u32(0)?, a_u32(1)?)),
        "ids_by_filter" => jids(&m.ids_by_filter(a_filter(0)?)),
        "register_filter" => {
            m.register_filter(a_filter(0)?.ok_or("a filter token")?);
            Json::Null
        }
        "unregister_filter" => {
            m.unregister_filter(a_filter(0)?.ok_or("a filter token")?);
            Json::Null
        }
        "flush" => {
            m.flush();
            Json::Null
        }
        "has_building_family" => Json::Bool(m.has_building_family(a_cfg(0)?, a_u32(1)?).is_some()),
        "has_building_type" => Json::Bool(m.has_building_type(a_cfg(0)?, a_u32(1)?).is_some()),
        "object_id_array" => {
            let o = LotConfigObject {
                kind: a_u32(0)? as u8,
                ids: a_ids(1)?,
                ..LotConfigObject::default()
            };
            jids(&m.object_id_array(Some(&o)))
        }
        "footprint_props" => rect_json(m.footprint_props(&a_ids(0)?)),
        "footprint_buildings" => rect_json(m.footprint_buildings(&a_ids(0)?)),
        "footprint_flora" => rect_json(m.footprint_flora(&a_ids(0)?)),
        "post_city_init" => Json::Bool(m.do_message(lcm::MSG_POST_CITY_INIT)),
        "fix" => ju(m.fix(a_cfg(0)?)),
        "create_generic" => match m.create_generic(a_u32(0)?) {
            None => Json::Null,
            Some(t) => {
                if let Some(l) = opt(op, "as") {
                    w.labels.borrow_mut().insert(str_of(l, "as")?.to_owned(), t);
                }
                let c = w.cfg(t);
                obj(vec![
                    ("size", jids(&[c.size.0, c.size.1])),
                    ("objects", Json::Array(c.objects.iter().map(object_json).collect())),
                ])
            }
        },
        other => return Err(format!("unknown lot configuration manager method '{other}'")),
    };
    check_expect(op, got)
}

fn lcm_expect(w: &LcmWorld, op: &Json) -> R<()> {
    let m = w.m()?;
    let s = m.snapshot();
    let o = |t: Option<u64>| t.map_or(Json::Null, jt);
    let cat = m.catalogue();
    let cat = cat.borrow();
    let got = vec![
        ("initialized".to_owned(), Json::Bool(s.initialized)),
        ("building_development".to_owned(), o(s.building_development)),
        ("flora".to_owned(), o(s.flora)),
        ("prop_manager".to_owned(), o(s.prop_manager)),
        ("filters".to_owned(), ji(s.filters.len() as i64)),
        ("loaded".to_owned(), Json::Bool(cat.is_loaded())),
        ("detail".to_owned(), ju(cat.detail())),
    ];
    check_state(&got, field(op, "state")?)
}

fn lcm_expect_config(w: &LcmWorld, op: &Json) -> R<()> {
    let t = w
        .cfg_arg(field(op, "configuration")?)?
        .ok_or("'configuration' must name one")?;
    let c = w.configs.borrow().get(&t).cloned().ok_or("no such configuration")?;
    let got = vec![
        ("id".to_owned(), ju(c.id)),
        ("size".to_owned(), jids(&[c.size.0, c.size.1])),
        (
            "objects".to_owned(),
            Json::Array(c.objects.iter().map(object_json).collect()),
        ),
        ("object_count".to_owned(), ji(c.objects.len() as i64)),
        ("info".to_owned(), info_json(&c.info)),
        ("walls".to_owned(), jids(&c.walls)),
        ("generic_for".to_owned(), c.generic_for.map_or(Json::Null, ju)),
    ];
    check_state(&got, field(op, "state")?)
}

fn lcm_exec(host: &HostRef, rest: &str, op: &Json) -> R<()> {
    let w = lcm_world(host);
    match rest {
        "env" => {
            if let Some(v) = opt(op, "detail") {
                w.detail.set(u32_of(v, "detail")?);
            }
            if let Some(v) = opt(op, "no_resource_manager") {
                w.no_resource_manager.set(bool_of(v, "no_resource_manager")?);
            }
            if let Some(v) = opt(op, "valid_walls") {
                *w.walls_valid.borrow_mut() = u32s(v, "valid_walls")?.into_iter().collect();
            }
            Ok(())
        }
        "lot_exemplar" => {
            let id = u32_of(field(op, "id")?, "id")?;
            let h = lot_object_holder(op)?;
            let key = ResKey {
                type_id: EXEMPLAR_TYPE,
                group: LOT_CONFIG_GROUP,
                instance: id,
            };
            w.keys.borrow_mut().push((key, Rc::new(h)));
            let families = match opt(op, "families") {
                Some(v) => u32s(v, "families")?,
                None => Vec::new(),
            };
            let size = match opt(op, "size") {
                Some(v) => {
                    let s = u32s(v, "size")?;
                    (s[0], s[1])
                }
                None => (1, 1),
            };
            w.meta.borrow_mut().insert(id, (families, size));
            Ok(())
        }
        "remove_exemplar" => {
            let id = u32_of(field(op, "id")?, "id")?;
            w.keys.borrow_mut().retain(|(k, _)| k.instance != id);
            Ok(())
        }
        "config" => {
            let label = str_of(field(op, "label")?, "label")?.to_owned();
            let t = w.new_token();
            w.bump(t, 1);
            let objects = match opt(op, "objects") {
                Some(v) => arr(v, "objects")?.iter().map(config_object).collect::<R<_>>()?,
                None => Vec::new(),
            };
            let size = match opt(op, "size") {
                Some(v) => {
                    let s = u32s(v, "size")?;
                    (s[0], s[1])
                }
                None => (1, 1),
            };
            let walls = match opt(op, "walls") {
                Some(v) => u32s(v, "walls")?,
                None => Vec::new(),
            };
            w.configs.borrow_mut().insert(
                t,
                Cfg {
                    id: u32_of(field(op, "id")?, "id")?,
                    families: match opt(op, "families") {
                        Some(v) => u32s(v, "families")?,
                        None => Vec::new(),
                    },
                    size,
                    objects,
                    walls,
                    ..Cfg::default()
                },
            );
            w.labels.borrow_mut().insert(label, t);
            Ok(())
        }
        "set_objects" => {
            let t = w
                .cfg_arg(field(op, "configuration")?)?
                .ok_or("'configuration' must name one")?;
            let objects = arr(field(op, "objects")?, "objects")?
                .iter()
                .map(config_object)
                .collect::<R<Vec<_>>>()?;
            w.with_cfg(t, |c| c.objects = objects);
            Ok(())
        }
        k @ ("building_type" | "building_family" | "prop_type" | "prop_family" | "flora_type" | "flora_family") => {
            lcm_types(&w, k, op)
        }
        "filter" => {
            let f = tok(field(op, "filter")?, "filter")?;
            let ids = u32s(field(op, "ids")?, "ids")?;
            w.filters.borrow_mut().insert(f, ids.into_iter().collect());
            Ok(())
        }
        "objects" => {
            let h = lot_object_holder(op)?;
            let d = u32_of(field(op, "detail")?, "detail")?;
            let mut serial = 0;
            let got: Vec<Json> = lot_object_array(&h, d, &mut serial)
                .iter()
                .map(|o| Json::Array(vec![ju(u32::from(o.kind)), ju(u32::from(o.flags))]))
                .collect();
            check_expect(op, Json::Array(got))
        }
        "lod" => {
            let v1 = u32_of(field(op, "v1")?, "v1")?;
            check_expect(op, ju(lcm::lod_of(v1)))
        }
        "create" => {
            let keep = match opt(op, "same_catalogue") {
                Some(v) => bool_of(v, "same_catalogue")?,
                None => false,
            };
            let cat = match (keep, w.catalogue.borrow().clone()) {
                (true, Some(c)) => c,
                _ => LotConfigCatalogue::shared(),
            };
            *w.catalogue.borrow_mut() = Some(cat.clone());
            *w.manager.borrow_mut() = Some(LotConfigManager::new(w.clone(), cat, LCM_ME));
            Ok(())
        }
        "init" => check_expect(op, Json::Bool(w.m()?.init())),
        "shutdown" => check_expect(op, Json::Bool(w.m()?.shutdown())),
        "unload" => {
            w.m()?.unload();
            Ok(())
        }
        "remember" => {
            let id = u32_of(field(op, "id")?, "id")?;
            let t = w
                .m()?
                .configuration(id)
                .ok_or_else(|| format!("no configuration {id:#X}"))?;
            w.labels
                .borrow_mut()
                .insert(str_of(field(op, "as")?, "as")?.to_owned(), t);
            Ok(())
        }
        "expect_same" => {
            let id = u32_of(field(op, "id")?, "id")?;
            let t = w.m()?.configuration(id);
            let l = str_of(field(op, "as")?, "as")?;
            let want = bool_of(field(op, "same")?, "same")?;
            if (t.is_some() && t == w.label(l)) != want {
                return Err(format!("configuration {id:#X} same as '{l}': expected {want}"));
            }
            Ok(())
        }
        "call" => lcm_call(&w, op),
        "expect" => lcm_expect(&w, op),
        "expect_config" => lcm_expect_config(&w, op),
        "expect_refs" => refs_check(op, &w.refs.borrow(), &|l| w.label(l)),
        "record" => {
            let (ok, bytes) = write_record(&*w.m()?);
            if !ok {
                return Err("Write returned false".into());
            }
            expect_hex(op, &bytes)
        }
        "read" => {
            let bytes = hex_decode(str_of(field(op, "hex")?, "hex")?)?;
            let mut res = resolver(op)?;
            let ok = w.m()?.read(&mut ByteReader::with_resolver(&bytes, &mut res));
            check_expect(op, Json::Bool(ok))
        }
        "expect_events" => w.events.expect_events(op),
        "expect_event_count" => w.events.expect_count(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }
        other => Err(format!("unknown op 'lcm_{other}'")),
    }
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    if let Some(rest) = name.strip_prefix("pm_") {
        return Some(pm_exec(host, rest, op));
    }
    if let Some(rest) = name.strip_prefix("lcm_") {
        return Some(lcm_exec(host, rest, op));
    }
    None
}
