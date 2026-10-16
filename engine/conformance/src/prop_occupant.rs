// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/prop-occupant.md`: one prop occupant (or
//! one visual-effect prop), a fake game behind
//! [`fc4_core::prop_occupant::PropOccupantHost`] and
//! [`fc4_core::prop_occupant::EffectHost`], and ops that drive the 24 slots,
//! the save record of §7, the PV2 tag rule of §5.3 and the class-object store
//! framing. See README.md ("Prop occupant ops").
//!
//! Op names are `po_*`. The record ops take and return hex, so a vector can
//! pin a corpus record byte for byte.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use fc4_core::automata::{OccupantBase, OCCUPANT_BASE_VERSION};
use fc4_core::prop_occupant::{
    add_usage, classify, read_effect_record, read_prop_record, shared_type_table, slot_channel, slot_spec,
    write_effect_record, write_prop_record, AddKind, BlockSlots, DateRange, EffectHost, EffectRecord, EffectRecordHead,
    PropOccupant, PropOccupantHost, PropRecord, ReadOutcome, RetChannel, SharedPropTypeTable, TagMatch, TagTally,
    VisualEffectProp, CLSID_PROP_OCCUPANT, EFFECT_RECORD_VERSION, LEGACY_TIMER_BITS, MSG_CREATE_OCCUPANT,
    MSG_PROP_STATE_CHANGED, NO_LOT_TAG, OBJECT_SIZE, OBJECT_SIZE_BURNT_BUILDING, OBJECT_SIZE_TERRAIN_DECAL,
    OBJECT_SIZE_VISUAL_EFFECT, OCCUPANT_TYPE_PROP, PROP_RECORD_VERSION, VTABLE_ADDR, VTABLE_LEN, VTABLE_NEXT_ADDR,
};
use fc4_core::store::{crc32_mpeg2, load_store, save_store, LoadRegistry, Package};
use fc4_core::stream::{ByteReader, ByteWriter, InStream, OutStream, RefValue, SaveObject, SaveRegistry};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{boolean, field, HostRef};
use crate::scalar::{self, arg, arr, check_expect, check_state, f32_of, jf, ji, opt, str_of, u32_of, R};

// ---- the fake game ----------------------------------------------------------

/// Everything the occupant reaches outside itself (§3, §6.1, `nt` §13.1).
#[derive(Default)]
struct Game {
    in_city: Cell<bool>,
    model_state_property: Cell<Option<bool>>,
    messages: RefCell<Vec<(u32, u32)>>,
    model_states: RefCell<Vec<u8>>,
    clock_entries: RefCell<Vec<(u8, u8)>>,
    draws: RefCell<Vec<u8>>,
    draws_made: Cell<usize>,
    effect_handles: RefCell<Vec<u64>>,
    effect_calls: RefCell<Vec<u64>>,
    removed: Cell<u32>,
}

impl Game {
    /// The messages posted, as `[type, data1]` pairs.
    fn messages_json(&self) -> Json {
        Json::Array(
            self.messages
                .borrow()
                .iter()
                .map(|(t, d)| Json::Array(vec![ji(*t), ji(*d)]))
                .collect(),
        )
    }

    /// The clock entries registered, as `[start, end]` pairs.
    fn clock_json(&self) -> Json {
        Json::Array(
            self.clock_entries
                .borrow()
                .iter()
                .map(|(a, b)| Json::Array(vec![ji(*a), ji(*b)]))
                .collect(),
        )
    }
}

impl PropOccupantHost for Game {
    fn is_in_city(&self) -> bool {
        self.in_city.get()
    }

    fn post_occupant_message(&self, msg_type: u32, data1: u32) {
        self.messages.borrow_mut().push((msg_type, data1));
    }

    fn has_model_state_property(&self) -> Option<bool> {
        self.model_state_property.get()
    }

    fn set_model_state(&self, state: u8) {
        self.model_states.borrow_mut().push(state);
    }

    fn set_clock_entry(&self, start_tenths: u8, end_tenths: u8) {
        // (0, 0) is the clear (`nt` §13.1 slot 13); anything else registers.
        let mut c = self.clock_entries.borrow_mut();
        if start_tenths == 0 && end_tenths == 0 {
            c.clear();
        } else {
            c.push((start_tenths, end_tenths));
        }
    }

    fn draw_chance(&self) -> u8 {
        self.draws_made.set(self.draws_made.get() + 1);
        let mut d = self.draws.borrow_mut();
        if d.is_empty() {
            50
        } else {
            d.remove(0)
        }
    }

    fn remove_from_city(&self) -> bool {
        if self.in_city.get() {
            self.in_city.set(false);
            self.removed.set(self.removed.get() + 1);
            true
        } else {
            false
        }
    }
}

impl EffectHost for Game {
    fn restart_effect(&self, _id: u32, _name: &[u8]) -> Option<u64> {
        let h = 0xE000 + self.effect_handles.borrow().len() as u64 + 1;
        self.effect_handles.borrow_mut().push(h);
        Some(h)
    }

    fn effect_slot3(&self, handle: u64) {
        self.effect_calls.borrow_mut().push(handle);
    }

    fn effect_slot4(&self, handle: u64) {
        self.effect_calls.borrow_mut().push(handle);
    }

    fn flag2(&self) -> bool {
        self.in_city.get()
    }
}

/// An [`EffectHost`] whose `restart_effect` makes nothing, so `StartEffect`
/// answers false (`nt` §13.2).
struct Effectless {
    game: Game,
}

impl EffectHost for Effectless {
    fn restart_effect(&self, _id: u32, _name: &[u8]) -> Option<u64> {
        None
    }

    fn effect_slot3(&self, handle: u64) {
        self.game.effect_slot3(handle);
    }

    fn effect_slot4(&self, handle: u64) {
        self.game.effect_slot4(handle);
    }

    fn flag2(&self) -> bool {
        self.game.flag2()
    }
}

// ---- the world --------------------------------------------------------------

/// The per-vector world: one prop, one effect prop, the shared type table and
/// the fake game.
#[derive(Default)]
struct World {
    types: RefCell<Option<SharedPropTypeTable>>,
    prop: RefCell<Option<PropOccupant>>,
    record: RefCell<Option<PropRecord>>,
    effect: RefCell<Option<VisualEffectProp>>,
    effect_record: RefCell<Option<EffectRecord>>,
    game: Game,
    /// Whether the next `SetEffectType` finds no effect object.
    no_effect: Cell<bool>,
    /// The object IDs of a configuration, for the PV2 ops.
    config: RefCell<Option<Vec<u32>>>,
    /// Whether the configuration's array is unreadable.
    config_unknown: Cell<bool>,
    block: RefCell<BlockSlots>,
    tally: RefCell<TagTally>,
}

impl World {
    fn new() -> Rc<World> {
        Rc::new(World {
            types: RefCell::new(Some(shared_type_table())),
            ..World::default()
        })
    }

    fn types(&self) -> SharedPropTypeTable {
        self.types.borrow().clone().unwrap_or_else(shared_type_table)
    }

    fn ensure_prop(&self) {
        if self.prop.borrow().is_none() {
            *self.prop.borrow_mut() = Some(PropOccupant::new(self.types()));
        }
    }

    fn ensure_record(&self) {
        if self.record.borrow().is_none() {
            *self.record.borrow_mut() = Some(PropRecord::default());
        }
    }

    fn ensure_effect(&self) {
        if self.effect.borrow().is_none() {
            *self.effect.borrow_mut() = Some(VisualEffectProp::new(self.types()));
        }
    }

    /// A one-line state view of the current prop (or record).
    fn state(&self) -> Vec<(String, Json)> {
        let mut v: Vec<(String, Json)> = Vec::new();
        if let Some(p) = self.prop.borrow().as_ref() {
            let (lo, hi) = p.bounding_box();
            v.push(("state".into(), ji(p.get_state())));
            v.push(("chance".into(), ji(p.get_random_chance())));
            v.push((
                "tod_bytes".into(),
                Json::Array(vec![ji(p.time_of_day_bytes().0), ji(p.time_of_day_bytes().1)]),
            ));
            let (a, b) = p.get_time_of_day_range();
            v.push(("tod".into(), Json::Array(vec![jf(a), jf(b)])));
            v.push(("timer_bits".into(), ji(p.get_prop_timer_bitfield())));
            v.push(("lot_tag".into(), ji(p.lot_tag())));
            v.push(("has_lot_tag".into(), Json::Bool(p.has_lot_tag())));
            v.push(("prop_type".into(), ji(p.get_prop_type())));
            v.push(("type_index".into(), ji(p.prop_type_index())));
            v.push(("orientation".into(), ji(p.get_orientation())));
            v.push(("usage".into(), ji(p.get_prop_usage())));
            v.push((
                "bbox".into(),
                Json::Array(vec![
                    Json::Array(lo.iter().map(|x| jf(*x)).collect()),
                    Json::Array(hi.iter().map(|x| jf(*x)).collect()),
                ]),
            ));
            v.push(("refcount".into(), ji(p.refcount())));
            v.push(("loaded".into(), Json::Bool(p.is_loaded())));
            v.push(("has_date_range".into(), Json::Bool(p.has_date_range())));
            v.push(("occupant_type".into(), ji(p.occupant_type())));
            v.push(("object_size".into(), ji(p.object_size() as i64)));
            match p.simulator_date_range() {
                Some(r) => v.push((
                    "date_range".into(),
                    Json::Array(vec![ji(r.interval), ji(r.duration), ji(r.start_day), ji(r.end_day)]),
                )),
                None => v.push(("date_range".into(), Json::Null)),
            }
        }
        if let Some(e) = self.effect.borrow().as_ref() {
            v.push(("effect_id".into(), ji(e.effect_id())));
            v.push(("effect_name".into(), Json::Str(hex_encode(&e.effect_name()))));
            v.push(("effect_started".into(), Json::Bool(e.is_started())));
            v.push(("effect_live".into(), Json::Bool(e.is_live())));
            v.push(("effect_ctor_flag".into(), ji(e.ctor_flag())));
            v.push((
                "effect_scratch".into(),
                Json::Array(vec![ji(e.scratch()[0]), ji(e.scratch()[1])]),
            ));
        }
        v.push(("messages".into(), self.game.messages_json()));
        v.push(("clock".into(), self.game.clock_json()));
        v.push(("draws".into(), ji(self.game.draws_made.get() as i64)));
        v.push(("removed".into(), ji(self.game.removed.get())));
        v.push((
            "model_states".into(),
            Json::Array(self.game.model_states.borrow().iter().map(|s| ji(*s)).collect()),
        ));
        v.push(("effect_calls".into(), ji(self.game.effect_calls.borrow().len() as i64)));
        v
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "prop_occupant", World::new)
}

// ---- helpers ----------------------------------------------------------------

fn channel_name(c: RetChannel) -> &'static str {
    match c {
        RetChannel::Eax => "eax",
        RetChannel::Al => "al",
        RetChannel::Void => "void",
        RetChannel::ViaCall => "via-call",
        RetChannel::EaxOrVoid => "eax-or-void",
    }
}

/// A JSON number as a u32, accepting `"0x…"` strings.
fn u32_arg(op: &Json, i: usize) -> R<u32> {
    u32_of(arg(op, i)?, &format!("argument {i}"))
}

fn f32_arg(op: &Json, i: usize) -> R<f32> {
    f32_of(arg(op, i)?, &format!("argument {i}"))
}

fn str_arg(op: &Json, i: usize) -> R<&str> {
    str_of(arg(op, i)?, &format!("argument {i}"))
}

fn bool_arg(op: &Json, i: usize) -> R<bool> {
    match boolean(arg(op, i)?) {
        Some(b) => Ok(b),
        None => Err(format!("argument {i} must be a boolean")),
    }
}

/// A three-float array argument.
fn vec3_arg(op: &Json, i: usize) -> R<[f32; 3]> {
    let a = arr(arg(op, i)?, &format!("argument {i}"))?;
    if a.len() != 3 {
        return Err(format!("argument {i} must be three floats"));
    }
    Ok([f32_of(&a[0], "x")?, f32_of(&a[1], "y")?, f32_of(&a[2], "z")?])
}

/// The game for an op that drives a slot needing the host seam.
fn game_of(w: &Rc<World>) -> &Game {
    &w.game
}

// ---- operations -------------------------------------------------------------

fn create(w: &Rc<World>, op: &Json) -> R<()> {
    *w.prop.borrow_mut() = Some(PropOccupant::new(w.types()));
    if let Some(v) = opt(op, "loaded") {
        if let Some(b) = boolean(v) {
            w.prop.borrow().as_ref().unwrap().set_loaded(b);
        }
    }
    Ok(())
}

fn effect_create(w: &Rc<World>, op: &Json) -> R<()> {
    *w.effect.borrow_mut() = Some(VisualEffectProp::new(w.types()));
    if let Some(v) = opt(op, "no_effect") {
        if boolean(v) == Some(true) {
            w.no_effect.set(true);
        }
    }
    Ok(())
}

fn po_call(w: &Rc<World>, op: &Json) -> R<()> {
    w.ensure_prop();
    let method = str_of(field(op, "method")?, "method")?;
    let g = game_of(w);
    let got = match method {
        // -- QueryInterface and the sub-objects (§1.1, §2.3) --
        "query_interface" => match PropOccupant::query_interface(u32_arg(op, 0)?) {
            Some(off) => ji(off as i64),
            None => Json::Null,
        },
        "query_interface_base" => match PropOccupant::query_interface_base(u32_arg(op, 0)?) {
            Some(off) => ji(off as i64),
            None => Json::Null,
        },
        "as_occupant_offset" => match PropOccupant::as_occupant_offset(Some(0)) {
            Some(off) => ji(off as i64),
            None => Json::Null,
        },
        "as_occupant_offset_null" => match PropOccupant::as_occupant_offset(None) {
            Some(off) => ji(off as i64),
            None => Json::Null,
        },

        // -- reference counting (§2.2, quirk 10) --
        "add_ref" => ji(w.prop.borrow().as_ref().unwrap().add_ref()),
        "release" => ji(w.prop.borrow().as_ref().unwrap().release()),
        "refcount" => ji(w.prop.borrow().as_ref().unwrap().refcount()),

        // -- slot 4: the bounding box (§2.2, §5.2) --
        "set_bounding_box" => {
            let lo = vec3_arg(op, 0)?;
            let hi = vec3_arg(op, 1)?;
            w.prop.borrow().as_ref().unwrap().set_bounding_box(lo, hi);
            Json::Null
        }
        "bounding_box" => {
            let (lo, hi) = w.prop.borrow().as_ref().unwrap().bounding_box();
            Json::Array(vec![
                Json::Array(lo.iter().map(|x| jf(*x)).collect()),
                Json::Array(hi.iter().map(|x| jf(*x)).collect()),
            ])
        }
        "set_bounding_box_from_size" => {
            let size: Option<Vec<f32>> = match opt(op, "size") {
                None => match arg(op, 0) {
                    Ok(j) if j.as_array().is_some() => Some(
                        arr(j, "size")?
                            .iter()
                            .map(|x| f32_of(x, "size"))
                            .collect::<R<Vec<f32>>>()?,
                    ),
                    _ => None,
                },
                Some(j) => Some(
                    arr(j, "size")?
                        .iter()
                        .map(|x| f32_of(x, "size"))
                        .collect::<R<Vec<f32>>>()?,
                ),
            };
            w.prop
                .borrow_mut()
                .as_mut()
                .unwrap()
                .set_bounding_box_from_size(size.as_deref());
            Json::Null
        }

        // -- slots 5 and 6 (§4.1, quirk 9) --
        "slot5_stub" => ji(w.prop.borrow().as_ref().unwrap().slot5_stub()),
        "slot6_noop" => {
            w.prop
                .borrow()
                .as_ref()
                .unwrap()
                .slot6_noop(u32_arg(op, 0).unwrap_or(0));
            Json::Null
        }

        // -- slots 7 and 8: the prop type (§5.1) --
        "get_prop_type" => ji(w.prop.borrow().as_ref().unwrap().get_prop_type()),
        "set_prop_type" => {
            let id = u32_arg(op, 0)?;
            w.prop.borrow_mut().as_mut().unwrap().set_prop_type(id);
            Json::Null
        }
        "prop_type_index" => ji(w.prop.borrow().as_ref().unwrap().prop_type_index()),
        "type_table_len" => ji(w.types().borrow().len() as i64),

        // -- slots 9 and 10: orientation (§5.2) --
        "get_orientation" => ji(w.prop.borrow().as_ref().unwrap().get_orientation()),
        "set_orientation" => Json::Bool(w.prop.borrow().as_ref().unwrap().set_orientation(i32_of_arg(op, 0)?)),

        // -- slots 11 and 12: state (nt §13.1) --
        "get_state" => ji(w.prop.borrow().as_ref().unwrap().get_state()),
        "set_state" => {
            let s = u32_arg(op, 0)?;
            w.prop.borrow().as_ref().unwrap().set_state(s as u8, g);
            Json::Null
        }
        "restore_state" => {
            let s = u32_arg(op, 0)?;
            w.prop.borrow().as_ref().unwrap().restore_state(s as u8);
            Json::Null
        }

        // -- slots 13 and 14: time of day (§8 item 3) --
        "set_time_of_day_range" => {
            let a = f32_arg(op, 0)?;
            let b = f32_arg(op, 1)?;
            w.prop.borrow().as_ref().unwrap().set_time_of_day_range(a, b, g);
            Json::Null
        }
        "get_time_of_day_range" => {
            let (a, b) = w.prop.borrow().as_ref().unwrap().get_time_of_day_range();
            Json::Array(vec![jf(a), jf(b)])
        }
        "time_of_day_bytes" => {
            let (a, b) = w.prop.borrow().as_ref().unwrap().time_of_day_bytes();
            Json::Array(vec![ji(a), ji(b)])
        }

        // -- slots 15 and 16: the date range (§2.2) --
        "set_simulator_date_range" => {
            let a = u32_arg(op, 0)?;
            let b = u32_arg(op, 1)?;
            let c = u32_arg(op, 2)?;
            let d = u32_arg(op, 3)?;
            w.prop.borrow().as_ref().unwrap().set_simulator_date_range(DateRange {
                interval: a,
                duration: b,
                start_day: c,
                end_day: d,
            });
            Json::Null
        }
        "simulator_date_range" => match w.prop.borrow().as_ref().unwrap().simulator_date_range() {
            Some(r) => Json::Array(vec![ji(r.interval), ji(r.duration), ji(r.start_day), ji(r.end_day)]),
            None => Json::Null,
        },
        "tick_simulator_date" => {
            let d = u32_arg(op, 0)?;
            w.prop.borrow().as_ref().unwrap().tick_simulator_date(d);
            Json::Null
        }

        // -- slots 17 and 18: the random chance (§4.1) --
        "set_random_chance" => {
            let c = u32_arg(op, 0)?;
            w.prop.borrow().as_ref().unwrap().set_random_chance(c as u8);
            Json::Null
        }
        "get_random_chance" => ji(w.prop.borrow().as_ref().unwrap().get_random_chance()),

        // -- slots 19, 20, 21: the timer bits (nt §13.1) --
        "get_prop_timer_bitfield" => ji(w.prop.borrow().as_ref().unwrap().get_prop_timer_bitfield()),
        "get_prop_timer_state" => {
            let m = u32_arg(op, 0)?;
            Json::Bool(w.prop.borrow().as_ref().unwrap().get_prop_timer_state(m as u8))
        }
        "set_prop_timer_state" => {
            let m = u32_arg(op, 0)?;
            let on = bool_arg(op, 1)?;
            w.prop.borrow().as_ref().unwrap().set_prop_timer_state(m as u8, on, g);
            Json::Null
        }

        // -- slots 22 and 23: the prop usage (§2.2, §3) --
        "get_prop_usage" => ji(w.prop.borrow().as_ref().unwrap().get_prop_usage()),
        "set_prop_usage" => {
            let u = u32_arg(op, 0)?;
            w.prop.borrow().as_ref().unwrap().set_prop_usage(u as u8);
            Json::Null
        }
        "add_usage" => {
            let kind = match str_arg(op, 0)? {
                "city" => AddKind::City,
                "lot" => AddKind::Lot,
                "building" => AddKind::Building,
                "street" => AddKind::Street,
                other => return Err(format!("unknown Add*Prop kind '{other}'")),
            };
            ji(add_usage(kind))
        }

        // -- the lot tag (§5.3) --
        "lot_tag" => ji(w.prop.borrow().as_ref().unwrap().lot_tag()),
        "has_lot_tag" => Json::Bool(w.prop.borrow().as_ref().unwrap().has_lot_tag()),
        "set_lot_tag" => {
            let t = u32_arg(op, 0)?;
            w.prop.borrow().as_ref().unwrap().set_lot_tag(t);
            Json::Null
        }

        // -- load bookkeeping (lot-records.md §2.4) --
        "init_after_load" => Json::Bool(w.prop.borrow().as_ref().unwrap().init_after_load(g)),
        "shutdown" => Json::Bool(w.prop.borrow().as_ref().unwrap().shutdown(g)),
        "set_loaded" => {
            let b = bool_arg(op, 0)?;
            w.prop.borrow().as_ref().unwrap().set_loaded(b);
            Json::Null
        }

        // -- the slot table (§4.1) --
        "vtable_len" => ji(VTABLE_LEN as i64),
        "slot_channel" => {
            let i = u32_arg(op, 0)? as usize;
            match slot_channel(i) {
                Some(c) => Json::Str(channel_name(c).into()),
                None => Json::Null,
            }
        }
        "slot_method" => {
            let i = u32_arg(op, 0)? as usize;
            match slot_spec(i) {
                Some(s) => Json::Str(s.method.into()),
                None => Json::Null,
            }
        }
        "slot_address" => {
            let i = u32_arg(op, 0)? as usize;
            match slot_spec(i) {
                Some(s) => ji(s.address),
                None => Json::Null,
            }
        }
        "slot_offset" => {
            let i = u32_arg(op, 0)? as usize;
            match slot_spec(i) {
                Some(s) => ji(s.offset),
                None => Json::Null,
            }
        }
        "slot_args" => {
            let i = u32_arg(op, 0)? as usize;
            match slot_spec(i) {
                Some(s) => Json::Array(vec![ji(s.callee_pop), ji(s.arg_dwords)]),
                None => Json::Null,
            }
        }
        "vtable_addr" => ji(VTABLE_ADDR),
        "vtable_next_addr" => ji(VTABLE_NEXT_ADDR),

        other => return Err(format!("unknown prop occupant method '{other}'")),
    };
    check_expect(op, got)
}

/// `set_orientation` takes a signed value; `u32_of` refuses negatives.
fn i32_of_arg(op: &Json, i: usize) -> R<i32> {
    let j = arg(op, i)?;
    j.as_i64()
        .map(|v| v as i32)
        .ok_or_else(|| format!("argument {i} must be an integer"))
}

fn po_expect(w: &Rc<World>, op: &Json) -> R<()> {
    check_state(&w.state(), field(op, "state")?)
}

/// A fresh default record (§7), replacing whatever was there.
fn po_record_create(w: &Rc<World>, _op: &Json) -> R<()> {
    *w.record.borrow_mut() = Some(PropRecord::default());
    Ok(())
}

/// Builds the current record from the live prop and the constructor's
/// occupant base (§7), so `po_write` emits what a save would.
fn po_record_from_prop(w: &Rc<World>, _op: &Json) -> R<()> {
    w.ensure_prop();
    let base = OccupantBase::new(OCCUPANT_TYPE_PROP);
    let rec = PropRecord::of(w.prop.borrow().as_ref().unwrap(), base);
    *w.record.borrow_mut() = Some(rec);
    Ok(())
}

fn po_read(w: &Rc<World>, op: &Json) -> R<()> {
    let bytes = hex_decode(str_of(field(op, "hex")?, "hex")?)?;
    let mut r = ByteReader::new(&bytes);
    let got = match read_prop_record(&mut r) {
        ReadOutcome::Ok(rec) => {
            *w.record.borrow_mut() = Some(rec.clone());
            if opt(op, "apply").and_then(boolean) == Some(true) {
                w.ensure_prop();
                rec.apply(w.prop.borrow_mut().as_mut().unwrap());
            }
            Json::Str("ok".into())
        }
        ReadOutcome::BaseFailed => Json::Str("base-failed".into()),
        ReadOutcome::Truncated => Json::Str("truncated".into()),
    };
    check_expect(op, got)
}

fn po_write(w: &Rc<World>, op: &Json) -> R<()> {
    w.ensure_record();
    let rec = w.record.borrow().clone().unwrap();
    let mut out = ByteWriter::new();
    if !write_prop_record(&rec, &mut out) {
        return Err("the record write failed".into());
    }
    let hex = hex_encode(&out.into_bytes());
    if let Some(want) = opt(op, "hex") {
        let want = hex_decode(str_of(want, "hex")?)?;
        let got = hex_decode(&hex)?;
        if got != want {
            return Err(format!(
                "record bytes differ: got {}, expected {}",
                hex_encode(&got),
                hex_encode(&want)
            ));
        }
    }
    check_expect(op, Json::Str(hex))
}

fn po_record_field(w: &Rc<World>, op: &Json) -> R<()> {
    w.ensure_record();
    let name = str_of(field(op, "name")?, "name")?;
    let rec = w.record.borrow();
    let rec = rec.as_ref().unwrap();
    let got = match name {
        "version" => ji(rec.version),
        "version_class" => Json::Str(
            (if rec.version_class().has_timer_bits() {
                "with-timer-bits"
            } else {
                "without-timer-bits"
            })
            .into(),
        ),
        "effective_timer_bits" => ji(rec.effective_timer_bits()),
        "prop_type" => ji(rec.prop_type),
        "orientation" => ji(rec.orientation),
        "state" => ji(rec.state),
        "chance" => ji(rec.chance),
        "prop_usage" => ji(rec.prop_usage),
        "lot_tag" => ji(rec.lot_tag),
        "timer_bits" => ji(rec.timer_bits),
        "time_of_day" => Json::Array(vec![ji(rec.time_of_day.0), ji(rec.time_of_day.1)]),
        "bounds" => Json::Array(vec![
            Json::Array(rec.bounds.0.iter().map(|x| jf(*x)).collect()),
            Json::Array(rec.bounds.1.iter().map(|x| jf(*x)).collect()),
        ]),
        "has_date_range" => Json::Bool(rec.date_range.is_some()),
        "occupant_base_version" => ji(OCCUPANT_BASE_VERSION),
        "occupant_type" => ji(rec.base.occupant_type),
        "base_status" => ji(rec.base.status),
        "base_flags" => ji(rec.base.flags),
        other => return Err(format!("unknown record field '{other}'")),
    };
    check_expect(op, got)
}

fn po_effect_call(w: &Rc<World>, op: &Json) -> R<()> {
    w.ensure_effect();
    let method = str_of(field(op, "method")?, "method")?;
    let no_effect = w.no_effect.get();
    let got = match method {
        "query_interface" => match VisualEffectProp::query_interface(u32_arg(op, 0)?) {
            Some(off) => ji(off as i64),
            None => Json::Null,
        },
        "as_occupant_offset" => ji(VisualEffectProp::as_occupant_offset() as i64),
        "as_prop_occupant_offset" => ji(VisualEffectProp::as_prop_occupant_offset() as i64),
        "object_size" => ji(w.effect.borrow().as_ref().unwrap().object_size() as i64),
        "class_id" => ji(w.effect.borrow().as_ref().unwrap().class_id()),
        "set_effect_type_id" => {
            let id = u32_arg(op, 0)?;
            let e = w.effect.borrow();
            let e = e.as_ref().unwrap();
            if no_effect {
                let g = Effectless { game: Game::default() };
                Json::Bool(e.set_effect_type_id(id, &g))
            } else {
                Json::Bool(e.set_effect_type_id(id, &w.game))
            }
        }
        "set_effect_type_name" => {
            let name = str_arg(op, 0)?.as_bytes().to_vec();
            let e = w.effect.borrow();
            let e = e.as_ref().unwrap();
            if no_effect {
                let g = Effectless { game: Game::default() };
                Json::Bool(e.set_effect_type_name(&name, &g))
            } else {
                Json::Bool(e.set_effect_type_name(&name, &w.game))
            }
        }
        "start_effect" => {
            let b = bool_arg(op, 0)?;
            let e = w.effect.borrow();
            let e = e.as_ref().unwrap();
            if no_effect {
                let g = Effectless { game: Game::default() };
                Json::Bool(e.start_effect(b, &g))
            } else {
                Json::Bool(e.start_effect(b, &w.game))
            }
        }
        "stop_effect" => {
            let b = bool_arg(op, 0)?;
            let e = w.effect.borrow();
            let e = e.as_ref().unwrap();
            Json::Bool(e.stop_effect(b, &w.game))
        }
        "effect_handle" => ji(w.effect.borrow().as_ref().unwrap().effect_handle() as i64),
        "effect_id" => ji(w.effect.borrow().as_ref().unwrap().effect_id()),
        "effect_name" => Json::Str(hex_encode(&w.effect.borrow().as_ref().unwrap().effect_name())),
        "is_started" => Json::Bool(w.effect.borrow().as_ref().unwrap().is_started()),
        "is_live" => Json::Bool(w.effect.borrow().as_ref().unwrap().is_live()),
        "should_drop_name_on_write" => Json::Bool(w.effect.borrow().as_ref().unwrap().should_drop_name_on_write()),
        other => return Err(format!("unknown visual-effect method '{other}'")),
    };
    check_expect(op, got)
}

fn po_effect_read(w: &Rc<World>, op: &Json) -> R<()> {
    let bytes = hex_decode(str_of(field(op, "hex")?, "hex")?)?;
    let mut r = ByteReader::new(&bytes);
    let got = match read_effect_record(&mut r) {
        Some(rec) => {
            let hex_name = hex_encode(&rec.name);
            let trailer = rec.trailer;
            let head = rec.head;
            *w.effect_record.borrow_mut() = Some(rec);
            Json::Array(vec![
                Json::Str("ok".into()),
                Json::Str(hex_name),
                ji(trailer),
                ji(head.byte0),
                ji(head.byte1),
            ])
        }
        None => Json::Str("failed".into()),
    };
    check_expect(op, got)
}

fn po_effect_write(w: &Rc<World>, op: &Json) -> R<()> {
    w.ensure_effect();
    let live = w.effect.borrow().as_ref().unwrap().is_live();
    let rec = w.effect_record.borrow().clone().unwrap_or(EffectRecord {
        version: EFFECT_RECORD_VERSION,
        prop: PropRecord::default(),
        head: EffectRecordHead { byte0: 1, byte1: 0 },
        name: Vec::new(),
        trailer: 0,
    });
    let mut out = ByteWriter::new();
    if !write_effect_record(&rec, live, &mut out) {
        return Err("the effect record write failed".into());
    }
    let hex = hex_encode(&out.into_bytes());
    if let Some(want) = opt(op, "hex") {
        let want = hex_decode(str_of(want, "hex")?)?;
        let got = hex_decode(&hex)?;
        if got != want {
            return Err(format!(
                "effect record bytes differ: got {}, expected {}",
                hex_encode(&got),
                hex_encode(&want)
            ));
        }
    }
    check_expect(op, Json::Str(hex))
}

/// The constants of §1 and §2.2 that a vector pins down (test 2, test 3).
fn po_const(op: &Json) -> R<()> {
    let name = str_of(field(op, "name")?, "name")?;
    let got = match name {
        "clsid_prop_occupant" => ji(CLSID_PROP_OCCUPANT),
        "occupant_type" => ji(fc4_core::prop_occupant::OCCUPANT_TYPE_PROP),
        "object_size" => ji(OBJECT_SIZE as i64),
        "object_size_terrain_decal" => ji(OBJECT_SIZE_TERRAIN_DECAL as i64),
        "object_size_visual_effect" => ji(OBJECT_SIZE_VISUAL_EFFECT as i64),
        "object_size_burnt_building" => ji(OBJECT_SIZE_BURNT_BUILDING as i64),
        "msg_create_occupant" => ji(MSG_CREATE_OCCUPANT),
        "msg_prop_state_changed" => ji(MSG_PROP_STATE_CHANGED),
        "no_lot_tag" => ji(NO_LOT_TAG),
        "prop_record_version" => ji(PROP_RECORD_VERSION),
        "effect_record_version" => ji(EFFECT_RECORD_VERSION),
        "legacy_timer_bits" => ji(LEGACY_TIMER_BITS),
        "occupant_base_version" => ji(OCCUPANT_BASE_VERSION),
        "vtable_addr" => ji(VTABLE_ADDR),
        "vtable_next_addr" => ji(VTABLE_NEXT_ADDR),
        "vtable_len" => ji(VTABLE_LEN as i64),
        other => return Err(format!("unknown prop occupant constant '{other}'")),
    };
    check_expect(op, got)
}

fn po_game(w: &Rc<World>, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "in_city") {
        if let Some(b) = boolean(v) {
            w.game.in_city.set(b);
        }
    }
    if let Some(v) = opt(op, "model_state_property") {
        w.game.model_state_property.set(match v {
            Json::Null => None,
            other => Some(boolean(other).unwrap_or(false)),
        });
    }
    if let Some(Json::Array(a)) = opt(op, "draws") {
        let mut d = w.game.draws.borrow_mut();
        d.clear();
        for x in a {
            d.push(u32_of(x, "draws")? as u8);
        }
    }
    if let Some(v) = opt(op, "flag2") {
        if let Some(b) = boolean(v) {
            w.game.in_city.set(b);
        }
    }
    if let Some(v) = opt(op, "clear") {
        if boolean(v) == Some(true) {
            w.game.messages.borrow_mut().clear();
            w.game.clock_entries.borrow_mut().clear();
            w.game.model_states.borrow_mut().clear();
            w.game.effect_calls.borrow_mut().clear();
            w.game.draws_made.set(0);
            w.game.removed.set(0);
        }
    }
    Ok(())
}

fn po_config(w: &Rc<World>, op: &Json) -> R<()> {
    // `ids` present and an array is the configuration's object IDs; `ids` null
    // (or absent together with `unknown`) is "no readable array", and
    // `unknown: true` then distinguishes the two corpus categories.
    let unknown = boolean(opt(op, "unknown").unwrap_or(&Json::Null)).unwrap_or(false);
    match opt(op, "ids").and_then(Json::as_array) {
        Some(a) => {
            let mut ids = Vec::new();
            for x in a {
                ids.push(u32_of(x, "ids")?);
            }
            *w.config.borrow_mut() = Some(ids);
            w.config_unknown.set(false);
        }
        None => {
            *w.config.borrow_mut() = None;
            w.config_unknown.set(unknown);
        }
    }
    Ok(())
}

fn po_tag(w: &Rc<World>, op: &Json) -> R<()> {
    let tag = u32_arg(op, 0)?;
    let ids = w.config.borrow().clone();
    let unknown = w.config_unknown.get();
    let outcome = classify(tag, ids.as_deref(), unknown);
    let mut block = w.block.borrow_mut();
    block.see_tag(outcome);
    if let Some(m) = outcome {
        w.tally.borrow_mut().add(m);
    }
    let got = match outcome {
        None => Json::Null,
        Some(TagMatch::Matched { object_index }) => {
            Json::Array(vec![Json::Str("matched".into()), ji(object_index as i64)])
        }
        Some(TagMatch::ConfigLacksTag) => Json::Str("config-lacks-tag".into()),
        Some(TagMatch::NoConfiguration) => Json::Str("no-configuration".into()),
        Some(TagMatch::UnknownConfiguration) => Json::Str("unknown-configuration".into()),
    };
    check_expect(op, got)
}

fn po_block(w: &Rc<World>, op: &Json) -> R<()> {
    let method = str_of(field(op, "method")?, "method")?;
    let got = match method {
        "keep" => {
            w.block.borrow_mut().keep();
            Json::Null
        }
        "kept" => ji(w.block.borrow().kept()),
        "unmatched" => ji(w.block.borrow().unmatched() as i64),
        "slots_with" => {
            let open = u32_arg(op, 0)?;
            ji(w.block.borrow().slots_with(open))
        }
        "reset" => {
            *w.block.borrow_mut() = BlockSlots::new();
            Json::Null
        }
        other => return Err(format!("unknown block method '{other}'")),
    };
    check_expect(op, got)
}

fn po_tally(w: &Rc<World>, op: &Json) -> R<()> {
    let method = str_of(field(op, "method")?, "method")?;
    let t = w.tally.borrow();
    let got = match method {
        "tagged" => ji(t.tagged as i64),
        "matched" => ji(t.matched as i64),
        "config_lacks_tag" => ji(t.config_lacks_tag as i64),
        "no_configuration" => ji(t.no_configuration as i64),
        "unknown_configuration" => ji(t.unknown_configuration as i64),
        "unmatched" => ji(t.unmatched() as i64),
        "unmatched_percent" => jf(t.unmatched_percent() as f32),
        other => return Err(format!("unknown tally method '{other}'")),
    };
    check_expect(op, got)
}

fn po_store(_w: &Rc<World>, op: &Json) -> R<()> {
    let method = str_of(field(op, "method")?, "method")?;

    /// A test object whose payload is one prop record.
    struct Saved(RefCell<PropRecord>);

    impl SaveObject for Saved {
        fn class_id(&self) -> u32 {
            CLSID_PROP_OCCUPANT
        }

        fn write(&self, out: &mut dyn OutStream) -> bool {
            write_prop_record(&self.0.borrow(), out)
        }

        fn read(&self, inp: &mut dyn InStream) -> bool {
            match read_prop_record(inp) {
                ReadOutcome::Ok(r) => {
                    *self.0.borrow_mut() = r;
                    true
                }
                _ => false,
            }
        }

        fn as_any_rc(self: Rc<Self>) -> Rc<dyn std::any::Any> {
            self
        }
    }

    let got = match method {
        "write" => {
            let n = u32_arg(op, 0)? as usize;
            let registry = Rc::new(RefCell::new(SaveRegistry::new()));
            for i in 0..n {
                let rec = PropRecord {
                    prop_type: 0x1000 + i as u32,
                    ..PropRecord::default()
                };
                let obj: Rc<dyn SaveObject> = Rc::new(Saved(RefCell::new(rec)));
                registry.borrow_mut().register(&RefValue::Object(obj.clone()));
            }
            let mut pkg: Package = Package::new();
            let report = save_store(&registry, &mut pkg);
            let key = (CLSID_PROP_OCCUPANT, 0x299B_2D1B, 0);
            let bytes = match pkg.get(&key) {
                Some(b) => b.clone(),
                None => return Err("the class record is missing".into()),
            };
            // Parse the framing: exactly N entries, no trailing bytes, each CRC.
            let mut pos = 0usize;
            let mut count = 0usize;
            let mut crcs_ok = 0usize;
            while pos < bytes.len() {
                if pos + 12 > bytes.len() {
                    return Err(format!("a truncated entry at {pos}"));
                }
                let entry_size = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
                let crc = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap());
                if entry_size < 12 || pos + entry_size > bytes.len() {
                    return Err(format!("bad entrySize {entry_size} at {pos}"));
                }
                let span = (entry_size - 8).min(fc4_core::store::CRC_LIMIT);
                if crc32_mpeg2(&bytes[pos + 8..pos + 8 + span]) == crc {
                    crcs_ok += 1;
                }
                pos += entry_size;
                count += 1;
            }
            if pos != bytes.len() {
                return Err("trailing bytes after the last entry".into());
            }
            let factory = |_c: u32, _id: u32| -> Option<RefValue> {
                Some(RefValue::Object(Rc::new(Saved(RefCell::new(PropRecord::default())))))
            };
            let mut index = std::collections::BTreeMap::new();
            index.insert(CLSID_PROP_OCCUPANT, n as u32);
            let mut reg = LoadRegistry::new(index, Box::new(factory));
            let load_report = load_store(&pkg, &mut reg);
            Json::Array(vec![
                ji(report.entries_written),
                ji(count as i64),
                ji(crcs_ok as i64),
                ji(load_report.entries_read),
                Json::Bool(load_report.index_ok),
            ])
        }
        other => return Err(format!("unknown store method '{other}'")),
    };
    check_expect(op, got)
}

// ---- dispatch ---------------------------------------------------------------

/// Handles the `po_*` ops of this module; `None` when the name is not ours.
pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("po_")?;
    let w = world(host);
    Some(match rest {
        "create" => create(&w, op),
        "effect_create" => effect_create(&w, op),
        "call" => po_call(&w, op),
        "expect" => po_expect(&w, op),
        "const" => po_const(op),
        "game" => po_game(&w, op),
        "record_create" => po_record_create(&w, op),
        "record_from_prop" => po_record_from_prop(&w, op),
        "read" => po_read(&w, op),
        "write" => po_write(&w, op),
        "record_field" => po_record_field(&w, op),
        "effect_call" => po_effect_call(&w, op),
        "effect_read" => po_effect_read(&w, op),
        "effect_write" => po_effect_write(&w, op),
        "config" => po_config(&w, op),
        "tag" => po_tag(&w, op),
        "block" => po_block(&w, op),
        "tally" => po_tally(&w, op),
        "store" => po_store(&w, op),
        other => return Some(Err(format!("unknown prop occupant op 'po_{other}'"))),
    })
}
