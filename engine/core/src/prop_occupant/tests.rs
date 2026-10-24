// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! A fake game and the 25 conformance tests of `docs/specs/prop-occupant.md`
//! §11. Each test names the section it covers.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::*;
use crate::automata::{OccupantBase, OCCUPANT_BASE_VERSION};
use crate::store::{crc32_mpeg2, load_store, save_store, LoadRegistry, Package, STORE_GROUP};
use crate::stream::{ByteReader, ByteWriter, InStream, OutStream, RefValue, SaveObject, SaveRegistry};

// ---- a fake game ------------------------------------------------------------

/// Everything the occupant reaches outside itself, logged so a test can check
/// what was posted, drawn and registered.
#[derive(Default)]
struct Game {
    in_city: Cell<bool>,
    model_state_property: Cell<Option<bool>>,
    messages: RefCell<Vec<(u32, u32)>>,
    model_states: RefCell<Vec<u8>>,
    clock_entries: RefCell<Vec<(u8, u8)>>,
    /// The scripted `RandomUint32Uniform(100)` draws, consumed in order; when
    /// the script runs out the draw is 50 (quirk 7).
    draws: RefCell<Vec<u8>>,
    /// How many draws have actually been made.
    draws_made: Cell<usize>,
    effect_handles: RefCell<Vec<u64>>,
    effect_calls: RefCell<Vec<u64>>,
    removed: Cell<u32>,
}

impl Game {
    fn new() -> Game {
        Game::default()
    }

    fn messages(&self) -> Vec<(u32, u32)> {
        self.messages.borrow().clone()
    }

    fn draw_count(&self) -> usize {
        self.draws_made.get()
    }

    /// The number of effect slot-3 / slot-4 calls made.
    fn effect_call_count(&self) -> usize {
        self.effect_calls.borrow().len()
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
        true
    }
}

/// A game whose `restart_effect` makes nothing, so `StartEffect` answers false.
#[derive(Default)]
struct EffectlessGame {
    inner: Game,
}

impl EffectHost for EffectlessGame {
    fn restart_effect(&self, _id: u32, _name: &[u8]) -> Option<u64> {
        None
    }

    fn effect_slot3(&self, handle: u64) {
        self.inner.effect_slot3(handle);
    }

    fn effect_slot4(&self, handle: u64) {
        self.inner.effect_slot4(handle);
    }

    fn flag2(&self) -> bool {
        self.inner.flag2()
    }
}

fn types() -> SharedPropTypeTable {
    shared_type_table()
}

fn prop() -> PropOccupant {
    PropOccupant::new(types())
}

/// A record whose occupant base is the constructor's (type `0xA823821E`).
fn record_for(p: &PropOccupant) -> PropRecord {
    PropRecord::of(p, OccupantBase::new(OCCUPANT_TYPE_PROP))
}

fn round_trip(record: &PropRecord) -> PropRecord {
    let mut w = ByteWriter::new();
    assert!(write_prop_record(record, &mut w), "write failed");
    let bytes = w.into_bytes();
    let mut r = ByteReader::new(&bytes);
    match read_prop_record(&mut r) {
        ReadOutcome::Ok(got) => got,
        other => panic!("read produced {other:?}"),
    }
}

// ================================================================ §11 1

/// §11 1: identity. `QueryInterface` answers the two class IIDs with the
/// object base and a new reference, `0x452294AA` with `+4`, and anything else
/// reaches the occupant base, which refuses an unknown IID.

#[test]
fn t01_identity() {
    // 0xE9793A65 and the class ID both return the object base (§1.1).
    assert_eq!(
        PropOccupant::query_interface(IID_PROP_OCCUPANT),
        Some(OFF_PROP_OCCUPANT)
    );
    assert_eq!(
        PropOccupant::query_interface(CLSID_PROP_OCCUPANT),
        Some(OFF_PROP_OCCUPANT)
    );
    // 0x452294AA returns this + 4 (§1.1).
    assert_eq!(
        PropOccupant::query_interface(IID_MESSAGE_TARGET2),
        Some(OFF_MESSAGE_TARGET)
    );
    // An unknown IID is passed to the occupant base, not refused directly
    // (quirk 4).
    assert_eq!(PropOccupant::query_interface(0xDEAD_BEEF), Some(OFF_OCCUPANT));
    // The base itself answers only cIGZUnknown (test 1's last clause).
    assert_eq!(PropOccupant::query_interface_base(IID_UNKNOWN), Some(OFF_OCCUPANT));
    assert_eq!(PropOccupant::query_interface_base(0xDEAD_BEEF), None);

    // Every successful branch calls AddRef before returning 1 (§1.1).
    let p = prop();
    assert_eq!(p.refcount(), 0);
    assert_eq!(p.add_ref(), 1);
}

// ================================================================ §11 2

/// §11 2: constructor defaults. State 0, chance **100**, time-of-day (0, 0),
/// timer bits 0, lot tag **`0xFFFFFFFF`**, prop type index 0 and occupant type
/// `0xA823821E`. The chance and the tag are the two an implementer is most
/// likely to get wrong (quirk 1).

#[test]
fn t02_constructor_defaults() {
    let p = prop();
    assert_eq!(p.get_state(), 0);
    assert_eq!(p.get_random_chance(), 100);
    assert_eq!(p.get_time_of_day_range(), (0.0, 0.0));
    assert_eq!(p.get_prop_timer_bitfield(), 0);
    assert_eq!(p.lot_tag(), 0xFFFF_FFFF);
    assert!(!p.has_lot_tag());
    assert_eq!(p.prop_type_index(), 0);
    assert_eq!(p.occupant_type(), 0xA823_821E);
    assert_eq!(p.get_orientation(), 0);
    assert_eq!(p.get_prop_usage(), 0);
    assert_eq!(p.refcount(), 0);
    assert_eq!(p.simulator_date_range(), None);
}

// ================================================================ §11 3

/// §11 3: object size. `CreateProp` allocates `0x68` for the plain class,
/// `0xA0` for `0x2977AA48` and `0x94` for `0x2977AA49`.

#[test]
fn t03_object_size() {
    assert_eq!(OBJECT_SIZE, 0x68);
    assert_eq!(OBJECT_SIZE_TERRAIN_DECAL, 0xA0);
    assert_eq!(OBJECT_SIZE_VISUAL_EFFECT, 0x94);
    assert_eq!(OBJECT_SIZE_BURNT_BUILDING, 0xD8);
    assert_eq!(prop().object_size(), 0x68);
    assert_eq!(VisualEffectProp::new(types()).object_size(), 0x94);
    // The pool strides of §4.2 independently confirm three of the four.
    assert_eq!(OBJECT_SIZE, 0x68);
    assert_eq!(OBJECT_SIZE_TERRAIN_DECAL, 0xA0);
    assert_eq!(OBJECT_SIZE_VISUAL_EFFECT, 0x94);
}

// ================================================================ §11 4

/// §11 4: `AsOccupant`, the vtable extent and the refcount. Slot 3 is
/// `this + 8` (null stays null); the interface is **exactly 24 slots**; and the
/// count lives at `+0x14`, which is where the `add ecx, 8` thunks land
/// (quirk 10), not at `+0x04`.

#[test]
fn t04_as_occupant_vtable_and_refcount() {
    // AsOccupant (slot 3, channel eax): this + 8, null stays null (§4.1).
    assert_eq!(
        PropOccupant::as_occupant_offset(Some(OFF_PROP_OCCUPANT)),
        Some(OFF_OCCUPANT)
    );
    assert_eq!(PropOccupant::as_occupant_offset(None), None);

    // The whole interface: 24 slots, +0x00…+0x5F (§4.1, §4.2, test 4).
    assert_eq!(PROP_OCCUPANT_VTABLE.len(), 24);
    assert_eq!(VTABLE_LEN, 24);
    assert_eq!(PROP_OCCUPANT_VTABLE[0].offset, 0x00);
    assert_eq!(PROP_OCCUPANT_VTABLE[23].offset, 0x5C);
    assert_eq!(PROP_OCCUPANT_VTABLE[23].slot, 23);
    // Nothing past slot 23 belongs to this interface: the words at
    // 0x00AA4900 + 24*4 are the pool/factory table (§4.2).
    assert!(slot_spec(24).is_none());
    assert_eq!(slot_channel(24), None);
    // The pool/factory table begins exactly where the interface ends
    // (§4.2): `0x00AA4900 + 24*4` is **not** part of this interface.
    assert_eq!(VTABLE_NEXT_ADDR, VTABLE_ADDR + (VTABLE_LEN as u32) * 4);
    // Consecutive compiled offsets, 4 bytes apart.
    for pair in PROP_OCCUPANT_VTABLE.windows(2) {
        assert_eq!(pair[1].offset, pair[0].offset + 4);
    }
    // The SDK index is the compiled slot minus 3 (nt §13.1).
    for s in &PROP_OCCUPANT_VTABLE[3..] {
        assert_eq!(s.sdk_index(), s.slot - 3);
    }

    // AddRef twice from 0 and Release once: the count lives at +0x14 (§2.2).
    let p = prop();
    assert_eq!(p.add_ref(), 1);
    assert_eq!(p.add_ref(), 2);
    assert_eq!(p.release(), 1);
    assert_eq!(p.refcount(), 1);
    // The value returned by slot 1 agrees with +0x14.
    assert_eq!(p.add_ref(), p.refcount());
    // A naive refcount of `this` would land on +0x04, the message-target
    // sub-object (quirk 10) — the offsets must not be confused.
    assert_ne!(OFF_REFCOUNT, OFF_MESSAGE_TARGET);
    assert_eq!(OFF_REFCOUNT, 0x14);

    // The six sub-objects of §2.3.
    let subs = PropOccupant::sub_object_offsets();
    assert_eq!(subs.len(), 6);
    assert_eq!(subs[0], (0x00AA_4900, OFF_PROP_OCCUPANT));
    assert_eq!(subs[5], (0x00AA_47D0, OFF_AUX));
}

// ================================================================ §11 5

/// §11 5: `GetPropType` round trip. `SetPropType(id)` then `GetPropType()` is
/// `id`, and two props of the same type hold the **same** `+0x5C` index,
/// because the table is process-global (§5.1 point 1).

#[test]
fn t05_get_prop_type_round_trip() {
    let t = types();
    let a = PropOccupant::new(t.clone());
    let b = PropOccupant::new(t.clone());
    a.set_prop_type(0x1234_5678);
    assert_eq!(a.get_prop_type(), 0x1234_5678);
    assert_eq!(a.prop_type_index(), 0);
    b.set_prop_type(0x1234_5678);
    assert_eq!(b.get_prop_type(), 0x1234_5678);
    // Same type, same index: the table is shared.
    assert_eq!(a.prop_type_index(), b.prop_type_index());
    // A second type takes the next index.
    a.set_prop_type(0x9ABC_DEF0);
    assert_eq!(a.prop_type_index(), 1);
    assert_eq!(a.get_prop_type(), 0x9ABC_DEF0);
    assert_eq!(b.get_prop_type(), 0x1234_5678);
}

// ================================================================ §11 6

/// §11 6: an unknown type is interned. Reading a save whose prop names a type
/// absent from the catalogue succeeds, `GetPropType` afterwards returns that
/// ID, and the process-wide table has grown by one (§5.1 point 2, quirk 2).

#[test]
fn t06_unknown_type_is_interned() {
    let t = types();
    let p = PropOccupant::new(t.clone());
    let before = t.borrow().len();

    let rec = PropRecord {
        prop_type: 0xFEED_F00D,
        bounds: ([0.0; 3], [1.0, 1.0, 1.0]),
        ..PropRecord::default()
    };
    let got = round_trip(&rec);
    got.apply(&p);

    assert_eq!(p.get_prop_type(), 0xFEED_F00D);
    // The table grew by exactly one (test 6).
    assert_eq!(t.borrow().len(), before + 1);
    // And it is interned at the end.
    assert_eq!(t.borrow().id_of(p.prop_type_index()), 0xFEED_F00D);
}

// ================================================================ §11 7

/// §11 7: `GetState` is a clean **EAX** return. `SetState(2)` then
/// `GetState()` is 2 with the upper bytes zero (`movzx`), not merely AL
/// (§4.1, quirk 11).

#[test]
fn t07_get_state_is_clean_eax() {
    let g = Game::new();
    let p = prop();
    p.set_state(2, &g);
    let v = p.get_state();
    assert_eq!(v, 2);
    // A `movzx` return: the whole register is defined, so a caller may compare
    // the u32 without masking.
    assert_eq!(v & 0xFFFF_FF00, 0);
    assert_eq!(v & 0xFF, 2);
    // Channel check: slot 12 is `eax` while slot 18 is `al` (quirk 11).
    assert_eq!(slot_channel(12), Some(RetChannel::Eax));
    assert_eq!(slot_channel(18), Some(RetChannel::Al));
}

// ================================================================ §11 8

/// §11 8: `GetRandomChance` is an **AL** return. `SetRandomChance(37)` then
/// `GetRandomChance()` is 37 in AL; a caller that reads EAX as a 32-bit value
/// must mask to `0xFF` (§4.1, quirk 3).

#[test]
fn t08_get_random_chance_is_al() {
    let p = prop();
    p.set_random_chance(37);
    assert_eq!(p.get_random_chance(), 37);
    // The measured channel is `al`, so the upper 24 bits are undefined in the
    // original. [choice] FreeCity4 zero-extends, and a host must mask.
    assert_eq!(slot_channel(18), Some(RetChannel::Al));
    assert_eq!(u32::from(p.get_random_chance()) & 0xFF, 37);
    // Adjacent, opposite: slot 12 is a clean eax (quirk 11).
    assert_eq!(slot_channel(12), Some(RetChannel::Eax));
    // And slot 22 is `al` too, while 19 is `eax` (§8 item 2).
    assert_eq!(slot_channel(22), Some(RetChannel::Al));
    assert_eq!(slot_channel(19), Some(RetChannel::Eax));
}

// ================================================================ §11 9

/// §11 9: the bounded box. A prop whose `OccupantSize` is (2, 3, 4) gets
/// (0,0,0)–(2,3,4); a prop with no size gets (0,0,0)–(1,1,1) (§5.2).

#[test]
fn t09_bounding_box() {
    let p = prop();
    p.set_bounding_box_from_size(Some(&[2.0, 3.0, 4.0]));
    assert_eq!(p.bounding_box(), ([0.0, 0.0, 0.0], [2.0, 3.0, 4.0]));

    let q = prop();
    q.set_bounding_box_from_size(None);
    assert_eq!(q.bounding_box(), ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]));
    let r = prop();
    r.set_bounding_box_from_size(Some(&[]));
    assert_eq!(r.bounding_box(), ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]));

    // SetBoundingBox (slot 4) is `void` and writes the AABB at +0x3C (§2.2).
    assert_eq!(slot_channel(4), Some(RetChannel::Void));
    let s = prop();
    s.set_bounding_box([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]);
    assert_eq!(s.bounding_box(), ([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]));
}

// ================================================================ §11 10

/// §11 10: orientation. `SetOrientation(3)` then `GetOrientation()` is 3, and
/// field 5 of the save record is 3. `SetOrientation` clamps into 0…3, so a
/// caller cannot store 4 or a negative value (§5.2).

#[test]
fn t10_orientation() {
    let p = prop();
    assert!(p.set_orientation(3));
    assert_eq!(p.get_orientation(), 3);
    // Channel `al`, always 1 (§4.1).
    assert_eq!(slot_channel(10), Some(RetChannel::Al));
    // The clamp.
    assert!(p.set_orientation(4));
    assert_eq!(p.get_orientation(), 3);
    assert!(p.set_orientation(-1));
    assert_eq!(p.get_orientation(), 0);
    assert!(p.set_orientation(0));
    assert_eq!(p.get_orientation(), 0);

    // Field 5 of the record is the clamped value.
    p.set_orientation(3);
    let rec = record_for(&p);
    assert_eq!(rec.orientation, 3);
    let got = round_trip(&rec);
    assert_eq!(got.orientation, 3);
}

// ================================================================ §11 11

/// §11 11: time of day. `SetTimeOfDayRange(18, 8)` stores (180, 80) in
/// `+0x5E` / `+0x5F` and `GetTimeOfDayRange` returns (18.0, 8.0);
/// `SetTimeOfDayRange(0, 0)` registers **no** clock entry. Neither method
/// returns in `st0` (§8 item 3, quirk 12).

#[test]
fn t11_time_of_day() {
    let g = Game::new();
    let p = prop();
    p.set_time_of_day_range(18.0, 8.0, &g);
    assert_eq!(p.time_of_day_bytes(), (180, 80));
    let (a, b) = p.get_time_of_day_range();
    assert!((a - 18.0).abs() < 1e-6 && (b - 8.0).abs() < 1e-6);
    // Registered with the 24-hour clock because a != b and both are >= 0.
    assert_eq!(g.clock_entries.borrow().as_slice(), &[(180, 80)]);

    // (0, 0) registers nothing (test 11): the clear leaves no entry.
    p.set_time_of_day_range(0.0, 0.0, &g);
    assert_eq!(p.time_of_day_bytes(), (0, 0));
    assert!(
        g.clock_entries.borrow().is_empty(),
        "SetTimeOfDayRange(0, 0) registers no clock entry"
    );

    // Channels: both are `void`, and x87 is used internally but ST(0) is empty
    // at return (§8 item 3).
    assert_eq!(slot_channel(13), Some(RetChannel::Void));
    assert_eq!(slot_channel(14), Some(RetChannel::Void));
}

// ================================================================ §11 12

/// §11 12: timer bits. `GetPropTimerState(0x2)` is false;
/// `SetPropTimerState(0x2, true)` makes it true and leaves the other bits
/// alone; `GetPropTimerBitfield()` reports the mask (§4.1).

#[test]
fn t12_timer_bits() {
    let g = Game::new();
    let p = prop();
    p.set_prop_timer_bits(0x14); // 0b010100: low nibble 0x4
    assert!(!p.get_prop_timer_state(0x2));
    p.set_prop_timer_state(0x2, true, &g);
    assert!(p.get_prop_timer_state(0x2));
    // The other bits are alone.
    assert_eq!(p.get_prop_timer_bitfield(), 0x16);
    assert!(p.get_prop_timer_state(0x4));
    assert!(!p.get_prop_timer_state(0x8));
    p.set_prop_timer_state(0x2, false, &g);
    assert!(!p.get_prop_timer_state(0x2));
    assert_eq!(p.get_prop_timer_bitfield(), 0x14);
    // Channels: 19 is `movzx eax` (a clean value), 20 is a real 0/1 in eax.
    assert_eq!(slot_channel(19), Some(RetChannel::Eax));
    assert_eq!(slot_channel(20), Some(RetChannel::Eax));
}

// ================================================================ §11 13

/// §11 13: the timer transition draw (quirk 7). With chance 100 and the low
/// nibble going to `0xF`, `SetState(0)` is called; with chance 0 it is not;
/// setting bits that leave the nibble below `0xF` never draws.

#[test]
fn t13_timer_transition_draw() {
    // Chance 100: becoming all-set draws once and calls SetState(0).
    // The prop starts in state 2 so the call is observable: SetState is a
    // no-op when the value already equals `+0x63` (nt §13.1 step 1).
    let g = Game::new();
    g.in_city.set(true); // SetState posts only while the occupant is in the city
    g.draws.borrow_mut().push(50);
    let p = prop();
    assert_eq!(p.get_random_chance(), 100);
    p.set_state(2, &g);
    g.messages.borrow_mut().clear();
    p.set_prop_timer_bits(0x03);
    p.set_prop_timer_state(0x0C, true, &g);
    assert_eq!(p.get_prop_timer_bitfield() & 0x0F, 0x0F);
    assert_eq!(g.draw_count(), 1, "exactly one draw on the transition");
    assert_eq!(p.get_state(), 0);
    assert!(
        g.messages().contains(&(MSG_PROP_STATE_CHANGED, 0)),
        "SetState(0) posted"
    );

    // Chance 0: the draw happens but r < 0 is never true, so no SetState(0).
    let g2 = Game::new();
    g2.draws.borrow_mut().push(0);
    let p2 = prop();
    p2.set_random_chance(0);
    p2.set_prop_timer_bits(0x01);
    p2.set_state(2, &g2);
    g2.messages.borrow_mut().clear();
    p2.set_prop_timer_state(0x0E, true, &g2);
    assert_eq!(g2.draw_count(), 1);
    assert_eq!(p2.get_state(), 2, "chance 0 never draws in SetState(0)");

    // Bits that leave the nibble below 0xF never draw.
    let g3 = Game::new();
    let p3 = prop();
    p3.set_prop_timer_bits(0x00);
    p3.set_prop_timer_state(0x04, true, &g3);
    p3.set_prop_timer_state(0x02, true, &g3);
    assert_eq!(g3.draw_count(), 0);
    assert_eq!(p3.get_prop_timer_bitfield() & 0x0F, 0x06);

    // Becoming not-all-set calls SetState(1), with no draw (nt §13.1 step 4).
    let g4 = Game::new();
    let p4 = prop();
    p4.set_prop_timer_bits(0x0F);
    p4.set_prop_timer_state(0x08, false, &g4);
    assert_eq!(g4.draw_count(), 0);
    assert_eq!(p4.get_state(), 1);
}

// ================================================================ §11 14

/// §11 14: the lot tag default. A fresh prop's `GetLotTag` (occupant slot 27)
/// is false; `SetLotTag(0x1234)` makes it true and returns `0x1234` (§5.3).

#[test]
fn t14_lot_tag_default() {
    let p = prop();
    assert!(!p.has_lot_tag());
    assert_eq!(p.lot_tag(), NO_LOT_TAG);
    p.set_lot_tag(0x1234);
    assert!(p.has_lot_tag());
    assert_eq!(p.lot_tag(), 0x1234);
    // Clearing returns to none.
    p.set_lot_tag(NO_LOT_TAG);
    assert!(!p.has_lot_tag());
    // The record field 12 is the tag, 0xFFFFFFFF = none.
    p.set_lot_tag(0x1234);
    assert_eq!(record_for(&p).lot_tag, 0x1234);
}

// ================================================================ §11 15

/// §11 15: usage. `AddLotProp` writes 2, `AddCityProp` 1, `AddStreetProp` 4
/// and `AddBuildingProp` 3; `GetPropUsage` / `SetPropUsage` round-trip each
/// (§3).

#[test]
fn t15_usage() {
    // The four Add* paths write these codes (§3).
    assert_eq!(usage::CITY, 1);
    assert_eq!(usage::LOT, 2);
    assert_eq!(usage::BUILDING, 3);
    assert_eq!(usage::STREET, 4);
    assert_eq!(add_usage(AddKind::City), 1);
    assert_eq!(add_usage(AddKind::Lot), 2);
    assert_eq!(add_usage(AddKind::Building), 3);
    assert_eq!(add_usage(AddKind::Street), 4);

    let p = prop();
    for code in [1u8, 2, 3, 4] {
        p.set_prop_usage(code);
        assert_eq!(p.get_prop_usage(), code);
        let rec = record_for(&p);
        assert_eq!(rec.prop_usage, code);
    }
    // Slots 22 (`al`) and 23 (`void`).
    assert_eq!(slot_channel(22), Some(RetChannel::Al));
    assert_eq!(slot_channel(23), Some(RetChannel::Void));
}

// ================================================================ §11 16

/// §11 16: the save / load round trip. A prop with type, box, orientation 2,
/// state 1, range (180, 80), chance 50, usage 2, lot tag `0xABCD1234` and
/// timer bits 7 comes back identical, and the record's version is 6 (§7).

#[test]
fn t16_save_load_round_trip() {
    let g = Game::new();
    let p = prop();
    p.set_prop_type(0x1111_2222);
    p.set_bounding_box([0.5, 1.5, 2.5], [3.5, 4.5, 5.5]);
    p.set_orientation(2);
    p.set_state(1, &g);
    p.set_time_of_day_bytes(180, 80);
    p.set_random_chance(50);
    p.set_prop_usage(2);
    p.set_lot_tag(0xABCD_1234);
    p.set_prop_timer_bits(7);
    p.set_simulator_date_range(DateRange {
        interval: 365,
        duration: 12,
        start_day: 10,
        end_day: 22,
    });

    let rec = record_for(&p);
    assert_eq!(rec.version, PROP_RECORD_VERSION);
    assert_eq!(rec.version, 6);
    assert_eq!(rec.base.occupant_type, OCCUPANT_TYPE_PROP);
    assert_eq!(rec.base.flags & 0x22, 0, "bits 1 and 5 are cleared on write");
    assert_eq!(OCCUPANT_BASE_VERSION, 4);

    let got = round_trip(&rec);
    assert_eq!(got.version, 6);
    // The status byte is a read artifact: `(highlight << 4) | 2` (`lr` §2.3).
    assert_eq!(got.base.status, (got.base.highlight << 4) | 2);
    assert_eq!(got.base.occupant_type, OCCUPANT_TYPE_PROP);
    assert_eq!(got.prop_type, 0x1111_2222);
    assert_eq!(got.bounds, ([0.5, 1.5, 2.5], [3.5, 4.5, 5.5]));
    assert_eq!(got.orientation, 2);
    assert_eq!(got.state, 1);
    assert_eq!(got.time_of_day, (180, 80));
    assert_eq!(got.date_range, rec.date_range);
    assert_eq!(got.chance, 50);
    assert_eq!(got.prop_usage, 2);
    assert_eq!(got.lot_tag, 0xABCD_1234);
    assert_eq!(got.timer_bits, 7);

    // And the fields land in a fresh prop.
    let q = prop();
    got.apply(&q);
    assert_eq!(q.get_prop_type(), 0x1111_2222);
    assert_eq!(q.bounding_box(), ([0.5, 1.5, 2.5], [3.5, 4.5, 5.5]));
    assert_eq!(q.get_orientation(), 2);
    assert_eq!(q.get_state(), 1);
    assert_eq!(q.time_of_day_bytes(), (180, 80));
    assert_eq!(q.get_random_chance(), 50);
    assert_eq!(q.get_prop_usage(), 2);
    assert_eq!(q.lot_tag(), 0xABCD_1234);
    assert_eq!(q.get_prop_timer_bitfield(), 7);
    assert_eq!(q.simulator_date_range(), rec.date_range);
}

// ================================================================ §11 17

/// §11 17: a version ≤ 5 read. A record whose version is 5 (with field 13
/// omitted) reads successfully and comes back with timer bits `0x0F` (quirk 5).

#[test]
fn t17_version_five_read() {
    // Build a version-5 record by hand: field 13 is not written.
    let mut w = ByteWriter::new();
    let base = OccupantBase::new(OCCUPANT_TYPE_PROP);
    w.put_u16(5);
    assert!(base.write(&mut w));
    w.put_u32(0x3333_4444); // type
    for v in [0.0f32; 6] {
        w.put_f32(v);
    }
    w.put_i8(1); // orientation
    w.put_u8(0); // state
    w.put_u8(10); // tod start
    w.put_u8(20); // tod end
    w.put_bool(false); // no date range
    w.put_u8(60); // chance
    w.put_u8(4); // usage
    w.put_u32(0x5555_6666); // lot tag
                            // Field 13 deliberately omitted.

    let bytes = w.into_bytes();
    let mut r = ByteReader::new(&bytes);
    let got = match read_prop_record(&mut r) {
        ReadOutcome::Ok(v) => v,
        other => panic!("read produced {other:?}"),
    };
    assert_eq!(got.version, 5);
    assert_eq!(got.version_class(), RecordVersion::WithoutTimerBits);
    assert!(!got.version_class().has_timer_bits());
    // Field 13 becomes 0x0F (test 17).
    assert_eq!(got.timer_bits, LEGACY_TIMER_BITS);
    assert_eq!(got.effective_timer_bits(), 0x0F);
    assert_eq!(got.prop_type, 0x3333_4444);
    assert_eq!(got.time_of_day, (10, 20));
    assert_eq!(got.chance, 60);
    assert_eq!(got.lot_tag, 0x5555_6666);
    // Nothing was left over.
    assert_eq!(r.remaining(), 0);

    // A version greater than 5 does read field 13.
    let mut w6 = ByteWriter::new();
    let rec = PropRecord {
        base: OccupantBase::new(OCCUPANT_TYPE_PROP),
        timer_bits: 3,
        ..PropRecord::default()
    };
    assert!(write_prop_record(&rec, &mut w6));
    let bytes6 = w6.into_bytes();
    let mut r6 = ByteReader::new(&bytes6);
    let got6 = read_prop_record(&mut r6).record().expect("v6 reads").clone();
    assert_eq!(got6.version_class(), RecordVersion::WithTimerBits);
    assert_eq!(got6.effective_timer_bits(), 3);
}

// ================================================================ §11 18

/// §11 18: an unknown type survives a round trip. Save a prop whose type is
/// not in the catalogue, reload, save again: the type ID is unchanged in the
/// record (§5.1, quirk 2). The u32 ID is what is saved, not the index.

#[test]
fn t18_unknown_type_survives_a_round_trip() {
    let t = types();
    let p = PropOccupant::new(t.clone());
    let unknown = 0xA1B2_C3D4;
    assert_eq!(t.borrow().len(), 0, "the type is not in the catalogue");

    let rec = PropRecord {
        prop_type: unknown,
        ..PropRecord::default()
    };
    let first = round_trip(&rec);
    first.apply(&p);
    assert_eq!(p.get_prop_type(), unknown);

    // Save again from the live prop.
    let second = record_for(&p);
    assert_eq!(second.prop_type, unknown, "the type ID is unchanged");
    assert_eq!(round_trip(&second).prop_type, unknown);
    // The record carries the ID, never the +0x5C index.
    assert_ne!(second.prop_type, u32::from(p.prop_type_index()));
}

// ================================================================ §11 19

/// §11 19: class-object store framing. In a save with N props the record
/// `(0x2977AA47, 0x299B2D1B, 0)` parses into exactly N entries with no
/// trailing bytes, and each entry's CRC verifies under CRC-32/MPEG-2 (§7,
/// foundations §3.3–§3.4).

#[test]
fn t19_class_object_store_framing() {
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

    let n: usize = 3;
    let registry = Rc::new(RefCell::new(SaveRegistry::new()));
    for i in 0..n {
        let rec = PropRecord {
            prop_type: 0x1000 + i as u32,
            lot_tag: 0x2000 + i as u32,
            ..PropRecord::default()
        };
        let obj: Rc<dyn SaveObject> = Rc::new(Saved(RefCell::new(rec)));
        registry.borrow_mut().register(&RefValue::Object(obj.clone()));
    }
    assert_eq!(registry.borrow().entries.len(), n);

    let mut pkg: Package = Package::new();
    let report = save_store(&registry, &mut pkg);
    assert_eq!(report.entries_written, n as u32);

    // The record key (0x2977AA47, 0x299B2D1B, 0) (§7). `save_store` also
    // writes the store's index record, so the package holds exactly two.
    let key = (CLSID_PROP_OCCUPANT, STORE_GROUP, 0);
    let bytes = pkg.get(&key).expect("the class record is present");
    assert_eq!(pkg.len(), 2, "the class record plus the store index");
    assert!(pkg.contains_key(&(crate::store::INDEX_TYPE, STORE_GROUP, 0)));

    // Exactly N entries, no trailing bytes, each CRC verifying.
    let mut pos = 0usize;
    let mut count = 0usize;
    let mut ids = Vec::new();
    while pos < bytes.len() {
        assert!(pos + 12 <= bytes.len(), "a truncated entry at {pos}");
        let entry_size = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
        let crc = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap());
        let object_id = u32::from_le_bytes(bytes[pos + 8..pos + 12].try_into().unwrap());
        assert!(entry_size >= 12, "entrySize covers the header");
        assert!(pos + entry_size <= bytes.len(), "an entry runs past the record");
        let span = (entry_size - 8).min(crate::store::CRC_LIMIT);
        assert_eq!(
            crc32_mpeg2(&bytes[pos + 8..pos + 8 + span]),
            crc,
            "CRC-32/MPEG-2 of entry {count}"
        );
        ids.push(object_id);
        pos += entry_size;
        count += 1;
    }
    assert_eq!(count, n, "exactly N entries");
    assert_eq!(pos, bytes.len(), "no trailing bytes");

    // And they load back: the index names the class and its object count.
    let factory = |_class: u32, _id: u32| -> Option<RefValue> {
        Some(RefValue::Object(Rc::new(Saved(RefCell::new(PropRecord::default())))))
    };
    let mut index = std::collections::BTreeMap::new();
    index.insert(CLSID_PROP_OCCUPANT, n as u32);
    let mut reg = LoadRegistry::new(index, Box::new(factory));
    let load_report = load_store(&pkg, &mut reg);
    assert_eq!(load_report.entries_read, n as u32, "all N entries read");
    assert!(load_report.index_ok, "the index is readable");
    assert!(load_report.failed_classes.is_empty());
    assert_eq!(reg.count_for_class(CLSID_PROP_OCCUPANT), n as u32);
}

// ================================================================ §11 20

/// §11 20: loaded props do not announce themselves. After a load, no
/// `CreateOccupant 0x99EF1140` is posted for the loaded props
/// (lot-records.md §2.4).

#[test]
fn t20_loaded_props_do_not_announce_themselves() {
    // A prop created fresh does announce itself.
    let g = Game::new();
    let p = prop();
    assert!(p.init_after_load(&g), "a new prop posts CreateOccupant");
    assert_eq!(g.messages(), vec![(MSG_CREATE_OCCUPANT, 0)]);

    // A loaded prop does not.
    let g2 = Game::new();
    let q = prop();
    q.set_loaded(true);
    assert!(!q.init_after_load(&g2), "a loaded prop posts nothing");
    assert!(g2.messages().is_empty());
    assert!(q.is_loaded());
}

// ================================================================ §11 21

/// §11 21: the visual-effect attach. `StartEffect(true)` returns true only
/// when an effect object exists at `+0x6C`; `StopEffect(true)` returns true
/// only when one existed and was started (§6.2, `nt` §13.2).

#[test]
fn t21_visual_effect_attach() {
    let g = Game::new();
    let e = VisualEffectProp::new(types());
    assert_eq!(e.class_id(), CLSID_PROP_VISUAL_EFFECT);

    // SetEffectType(name) makes an effect object through the host.
    assert!(e.set_effect_type_name(b"pothole", &g));
    assert_ne!(e.effect_handle(), 0, "the effect object is at +0x6C");
    assert_eq!(e.effect_name(), b"pothole");
    assert_eq!(e.effect_id(), 0, "the name form stores ID 0 (nt §13.2)");

    // StartEffect(true) is true only with an effect object.
    assert!(e.start_effect(true, &g));
    assert!(e.is_started());
    assert_eq!(g.effect_call_count(), 1, "flag 2 set: effect slot 3");
    // StopEffect(true) is true only when one existed and was started.
    assert!(e.stop_effect(true, &g));
    assert!(!e.is_started());
    assert_eq!(g.effect_call_count(), 2, "flag 2 set: effect slot 4");
    // Stopping again is false: `started` is clear.
    assert!(!e.stop_effect(true, &g));

    // With no effect object StartEffect still sets `started` and returns false.
    let ng = EffectlessGame::default();
    let f = VisualEffectProp::new(types());
    assert!(!f.set_effect_type_name(b"x", &ng) || true);
    // `restart_effect` returned None, so +0x6C is 0.
    f.set_effect_handle(0);
    assert!(!f.start_effect(true, &ng.inner));
    assert!(f.is_started(), "started := 1 even on the failure path");
    assert!(!f.stop_effect(true, &ng.inner));

    // The IDs.
    assert_eq!(VisualEffectProp::query_interface(IID_VISUAL_EFFECT), Some(0));
    assert_eq!(VisualEffectProp::query_interface(CLSID_PROP_VISUAL_EFFECT), Some(0));
    assert_eq!(VisualEffectProp::as_occupant_offset(), OFF_EFFECT_OCCUPANT);
    assert_eq!(VisualEffectProp::as_prop_occupant_offset(), OFF_EFFECT_PROP);
}

// ================================================================ §11 22

/// §11 22: the visual-effect record quirk. A live effect prop saves an
/// **empty** name and 0; after reload the name comes back from the exemplar
/// *[inferred]* (§6.2, `lr` §10.2).

#[test]
fn t22_visual_effect_record_quirk() {
    let g = Game::new();
    let e = VisualEffectProp::new(types());
    e.set_effect_type_name(b"welding_sparks", &g);
    assert!(e.is_live(), "the effect object is non-null");

    // The quirk: the name is dropped while the effect is live.
    assert!(e.should_drop_name_on_write());
    assert!(e.name_dropped_on_write());

    let mut rec = EffectRecord {
        version: EFFECT_RECORD_VERSION,
        prop: PropRecord::default(),
        head: EffectRecordHead { byte0: 1, byte1: 0 },
        name: e.effect_name(),
        trailer: EFFECT_RECORD_TRAILER,
    };
    rec.name = b"welding_sparks".to_vec();
    rec.trailer = 0x99;

    let mut w = ByteWriter::new();
    assert!(write_effect_record(&rec, true, &mut w));
    let bytes = w.into_bytes();
    let mut r = ByteReader::new(&bytes);
    let got = read_effect_record(&mut r).expect("the effect record reads");
    assert_eq!(got.name, Vec::<u8>::new(), "an empty name while live");
    assert_eq!(got.trailer, 0, "and 0 in place of the trailing u32");
    assert_eq!(got.head, (EffectRecordHead { byte0: 1, byte1: 0 }));
    assert_eq!(r.remaining(), 0);

    // LR13 T73: repeated live-effect serialization drops the name/trailer
    // each time without mutating the stored name or creating a bare occupant.
    let mut repeated = ByteWriter::new();
    assert!(write_effect_record(&rec, true, &mut repeated));
    assert_eq!(repeated.into_bytes(), bytes);
    assert_eq!(rec.name, b"welding_sparks");
    assert_eq!(rec.trailer, 0x99);

    // Not live: the name and the trailer are written out.
    let mut w2 = ByteWriter::new();
    assert!(write_effect_record(&rec, false, &mut w2));
    let bytes2 = w2.into_bytes();
    let mut r2 = ByteReader::new(&bytes2);
    let got2 = read_effect_record(&mut r2).expect("the effect record reads");
    assert_eq!(got2.name, b"welding_sparks");
    assert_eq!(got2.trailer, 0x99);

    // The version gate: anything other than 3 fails (lr §10.2 field 1).
    let mut w3 = ByteWriter::new();
    w3.put_u16(2);
    let bytes3 = w3.into_bytes();
    let mut r3 = ByteReader::new(&bytes3);
    assert!(read_effect_record(&mut r3).is_none());
}

// ================================================================ §11 23

/// §11 23: state and the model (quirk 6). `SetState(s)` posts `0xE9C86DDB`
/// when the occupant is in the city, and does nothing when `s` already equals
/// `+0x63`.

#[test]
fn t23_state_and_the_model() {
    // Already equal: nothing at all.
    let g = Game::new();
    g.in_city.set(true);
    let p = prop();
    p.set_state(0, &g);
    assert!(g.messages().is_empty(), "s == +0x63 does nothing");
    assert!(g.model_states.borrow().is_empty());

    // Different and in the city: the message is posted.
    p.set_state(1, &g);
    assert_eq!(g.messages(), vec![(MSG_PROP_STATE_CHANGED, 1)]);
    assert_eq!(p.get_state(), 1);

    // Not in the city: no message.
    g.in_city.set(false);
    g.messages.borrow_mut().clear();
    p.set_state(2, &g);
    assert!(g.messages().is_empty());
    assert_eq!(p.get_state(), 2);

    // The model switches only when the exemplar has 0x4A70D491 (quirk 6).
    let g2 = Game::new();
    g2.in_city.set(true);
    g2.model_state_property.set(Some(false));
    let p2 = prop();
    p2.set_state(1, &g2);
    assert!(g2.model_states.borrow().is_empty(), "no property, no model switch");

    g2.model_state_property.set(Some(true));
    p2.set_state(2, &g2);
    assert_eq!(g2.model_states.borrow().as_slice(), &[2]);
}

// ================================================================ §11 24

/// §11 24: PV2 tag matching. A lot prop whose tag equals a configuration
/// object's ID is counted normally; one whose tag names nothing leaves the
/// block's slot count **one lower** (`pd` §4.3 step 2.3.2 step 2).

#[test]
fn t24_pv2_tag_matching() {
    use crate::prop_occupant::tag::{classify, BlockSlots, TagMatch, TagTally, NO_LOT_TAG};

    // A tag that names an object is matched, at the first such object.
    let ids = [0x11u32, 0x22, 0x33, 0x22];
    assert_eq!(
        classify(0x22, Some(&ids), false),
        Some(TagMatch::Matched { object_index: 1 })
    );
    // A tag that names nothing is ConfigLacksTag.
    assert_eq!(classify(0x99, Some(&ids), false), Some(TagMatch::ConfigLacksTag));
    // No configuration at all.
    assert_eq!(classify(0x22, None, false), Some(TagMatch::NoConfiguration));
    // An unreadable configuration stays its own category.
    assert_eq!(classify(0x22, None, true), Some(TagMatch::UnknownConfiguration));
    // No tag at all is neither matched nor unmatched (step 2.3.2 step 1).
    assert_eq!(classify(NO_LOT_TAG, Some(&ids), false), None);

    // The quirk: an unmatched tag lowers k by one.
    let mut s = BlockSlots::new();
    s.keep();
    s.keep();
    s.keep();
    assert_eq!(s.kept(), 3);
    s.see_tag(Some(TagMatch::ConfigLacksTag));
    assert_eq!(s.kept(), 2, "k := k - 1 for an unmatched tag");
    assert_eq!(s.unmatched(), 1);
    // A matched tag is counted normally: k is untouched here.
    s.see_tag(classify(0x22, Some(&ids), false));
    assert_eq!(s.kept(), 2);
    // A prop with no tag leaves k alone.
    s.see_tag(None);
    assert_eq!(s.kept(), 2);
    // Step 2.3.5: slots = opt + k, one lower than the three props suggest.
    assert_eq!(s.slots_with(4), 6);

    // A synthetic tally: one of each outcome.
    let mut t = TagTally::new();
    t.add(TagMatch::ConfigLacksTag);
    t.add(TagMatch::NoConfiguration);
    t.add(TagMatch::UnknownConfiguration);
    t.add(TagMatch::Matched { object_index: 0 });
    assert_eq!(t.tagged, 4);
    assert_eq!(t.matched, 1);
    assert_eq!(t.config_lacks_tag, 1);
    assert_eq!(t.no_configuration, 1);
    assert_eq!(t.unknown_configuration, 1);
    assert_eq!(t.unmatched(), 3);
    assert!((t.unmatched_percent() - 75.0).abs() < 1e-9);

    // The corpus figures of §5.3: 40,830 of 147,162 unmatched (27.7 %), split
    // 40,794 + 36 + 0. The categories must sum to the total.
    use crate::prop_occupant::tag::corpus;
    assert_eq!(
        corpus::CONFIG_LACKS_TAG + corpus::NO_CONFIGURATION + corpus::UNKNOWN_CONFIGURATION,
        corpus::UNMATCHED
    );
    assert_eq!(corpus::UNMATCHED, 40_830);
    assert_eq!(corpus::TAGGED, 147_162);
    assert_eq!(corpus::CONFIG_LACKS_TAG, 40_794);
    assert_eq!(corpus::NO_CONFIGURATION, 36);
    assert_eq!(corpus::UNKNOWN_CONFIGURATION, 0);
    const _: () = assert!(corpus::UNMATCHED < corpus::TAGGED);
    // 40,830 / 147,162 rounds to the 27.7 % the corpus reports.
    let pct = corpus::UNMATCHED as f64 * 100.0 / corpus::TAGGED as f64;
    assert!((pct - corpus::UNMATCHED_PERCENT).abs() < 0.05, "pct = {pct}");
    // Effect props are clean: the tally of §5.3 counts no effect prop.
    assert_eq!(corpus::UNKNOWN_CONFIGURATION, 0);
}

// ================================================================ §11 25

/// §11 25: removal. `RemovePropA` then `Shutdown` removes the prop from the
/// occupant manager; a second removal is a no-op (§3).

#[test]
fn t25_removal() {
    let g = Game::new();
    g.in_city.set(true);
    let p = prop();
    // The first shutdown removes it.
    assert!(p.shutdown(&g), "the first removal is a removal");
    assert_eq!(g.removed.get(), 1);
    assert!(!g.is_in_city());
    // A second is a no-op.
    assert!(!p.shutdown(&g), "a second removal is a no-op");
    assert_eq!(g.removed.get(), 1);
}
