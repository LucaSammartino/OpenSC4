// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The prop occupant (`cSC4PropOccupant`, class `0x2977AA47`) through the C
//! ABI: `docs/specs/prop-occupant.md`. The contract is `include/fc4.h`.
//!
//! Three opaque handles cross the boundary: the process-wide prop type table of
//! §5.1, one prop occupant, and one visual-effect prop. The slot table of §4.1
//! is queryable so a C host can read each slot's **measured return channel**
//! rather than the `eax-or-void` the ABI rows carry.

use std::ffi::c_void;

use fc4_core::automata::{OccupantBase, OCCUPANT_BASE_VERSION};
use fc4_core::prop_occupant::{
    add_usage, classify, read_effect_record, read_prop_record, shared_type_table, slot_channel, slot_spec,
    write_effect_record, write_prop_record, AddKind, DateRange, EffectHost, EffectRecord, EffectRecordHead,
    PropOccupant, PropOccupantHost, PropRecord, ReadOutcome, RetChannel, SharedPropTypeTable, TagMatch,
    VisualEffectProp, CLSID_PROP_OCCUPANT, EFFECT_RECORD_VERSION, LEGACY_TIMER_BITS, NO_LOT_TAG, OBJECT_SIZE,
    OBJECT_SIZE_BURNT_BUILDING, OBJECT_SIZE_TERRAIN_DECAL, OBJECT_SIZE_VISUAL_EFFECT, OCCUPANT_TYPE_PROP,
    PROP_RECORD_VERSION, VTABLE_LEN,
};
use fc4_core::stream::{ByteReader, ByteWriter};

use crate::guard;

/// Opaque handle for the process-wide prop type table (§5.1).
#[repr(C)]
pub struct fc4_prop_type_table {
    _private: [u8; 0],
}

/// Opaque handle for one `cSC4PropOccupant`.
#[repr(C)]
pub struct fc4_prop_occupant {
    _private: [u8; 0],
}

/// Opaque handle for one `cSC4PropOccupantVisualEffect` (§6.2).
#[repr(C)]
pub struct fc4_prop_effect {
    _private: [u8; 0],
}

/// What the occupant reaches outside itself (§3, §6.1, `nt` §13.1).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_prop_occupant_host {
    /// `sizeof(fc4_prop_occupant_host)` as the caller compiled it.
    pub struct_size: u32,
    /// Opaque caller state passed back to every callback.
    pub user: *mut c_void,
    /// Whether the occupant is in the city. `SetState` posts only when true.
    pub is_in_city: Option<unsafe extern "C" fn(user: *mut c_void) -> i32>,
    /// Post an occupant message carrying this prop and `data1`.
    pub post_occupant_message: Option<unsafe extern "C" fn(user: *mut c_void, msg_type: u32, data1: u32)>,
    /// Whether the exemplar has property `0x4A70D491`: 1 true, 0 false,
    /// **-1 no exemplar is attached** (quirk 6).
    pub has_model_state_property: Option<unsafe extern "C" fn(user: *mut c_void) -> i32>,
    /// Switch the drawn model instances to `state`.
    pub set_model_state: Option<unsafe extern "C" fn(user: *mut c_void, state: u8)>,
    /// Register or clear the 24-hour clock entry; `(0, 0)` clears it.
    pub set_clock_entry: Option<unsafe extern "C" fn(user: *mut c_void, start: u8, end: u8)>,
    /// One draw from the prop's own generator, uniform in `0..100`.
    pub draw_chance: Option<unsafe extern "C" fn(user: *mut c_void) -> u8>,
    /// Remove this prop from the occupant manager; 1 when it was there.
    pub remove_from_city: Option<unsafe extern "C" fn(user: *mut c_void) -> i32>,
}

/// What the visual-effect prop reaches outside itself (§6.2).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_prop_effect_host {
    /// `sizeof(fc4_prop_effect_host)` as the caller compiled it.
    pub struct_size: u32,
    /// Opaque caller state passed back to every callback.
    pub user: *mut c_void,
    /// (Re)create the effect object; returns the handle or 0.
    pub restart_effect: Option<unsafe extern "C" fn(user: *mut c_void, id: u32, name: *const u8, name_len: u32) -> u64>,
    /// The effect's slot 3 (start).
    pub effect_slot3: Option<unsafe extern "C" fn(user: *mut c_void, handle: u64)>,
    /// The effect's slot 4 (stop).
    pub effect_slot4: Option<unsafe extern "C" fn(user: *mut c_void, handle: u64)>,
    /// Occupant flag 2.
    pub flag2: Option<unsafe extern "C" fn(user: *mut c_void) -> i32>,
}

/// One compiled slot of `cISC4PropOccupant` (§4.1).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_prop_slot_info {
    /// Compiled slot index, 0…23.
    pub slot: u8,
    /// The SDK's method index (compiled slot minus 3).
    pub sdk_index: u8,
    /// Dwords the callee pops after `this`.
    pub callee_pop: u8,
    /// Stack dwords beyond `this`.
    pub arg_dwords: u8,
    /// Byte offset of the vtable entry.
    pub offset: u32,
    /// The Windows address of the implementing body.
    pub address: u32,
    /// The measured return channel: 0 eax, 1 al, 2 void, 3 via-call,
    /// 4 eax-or-void (§4.1, [`RetChannel`]).
    pub ret: u32,
}

// ---- channel constants (§4.1) -----------------------------------------------

/// `RetChannel::Eax` as a C value.
pub const FC4_RET_EAX: u32 = 0;
/// `RetChannel::Al`: only AL is defined; mask to `0xFF`.
pub const FC4_RET_AL: u32 = 1;
/// `RetChannel::Void`: no defined result, x87 empty at return.
pub const FC4_RET_VOID: u32 = 2;
/// `RetChannel::ViaCall`: a tail call.
pub const FC4_RET_VIA_CALL: u32 = 3;
/// `RetChannel::EaxOrVoid`: the one ambiguous slot (§4.1, slot 8).
pub const FC4_RET_EAX_OR_VOID: u32 = 4;

/// `TagMatch` as C values for [`fc4_prop_tag_classify`].
pub const FC4_TAG_MATCHED: i32 = 0;
/// The lot has a configuration but no object names the tag (40,794 corpus).
pub const FC4_TAG_CONFIG_LACKS_TAG: i32 = 1;
/// The lot has no configuration (36 corpus).
pub const FC4_TAG_NO_CONFIGURATION: i32 = 2;
/// The configuration's array could not be read (0 corpus).
pub const FC4_TAG_UNKNOWN_CONFIGURATION: i32 = 3;
/// No tag at all: skipped before the lookup.
pub const FC4_TAG_NONE: i32 = 4;

/// [`ReadOutcome`] as C values for [`fc4_prop_occupant_read_record`].
pub const FC4_PROP_RECORD_OK: i32 = 0;
/// The base block failed and nothing after it was read (`lr` §10.1).
pub const FC4_PROP_RECORD_BASE_FAILED: i32 = 1;
/// The stream ended inside the payload.
pub const FC4_PROP_RECORD_TRUNCATED: i32 = 2;

// ---- handles ----------------------------------------------------------------

struct Table(SharedPropTypeTable);

struct Handle {
    prop: PropOccupant,
    host: fc4_prop_occupant_host,
}

struct EffectHandle {
    effect: VisualEffectProp,
    host: fc4_prop_effect_host,
}

impl PropOccupantHost for Handle {
    fn is_in_city(&self) -> bool {
        match self.host.is_in_city {
            // SAFETY: documented precondition; `user` is the caller's.
            Some(f) => (unsafe { f(self.host.user) }) != 0,
            None => false,
        }
    }

    fn post_occupant_message(&self, msg_type: u32, data1: u32) {
        if let Some(f) = self.host.post_occupant_message {
            // SAFETY: documented precondition.
            unsafe { f(self.host.user, msg_type, data1) };
        }
    }

    fn has_model_state_property(&self) -> Option<bool> {
        match self.host.has_model_state_property {
            // SAFETY: documented precondition. -1 is "no exemplar attached".
            Some(f) => match unsafe { f(self.host.user) } {
                1 => Some(true),
                0 => Some(false),
                _ => None,
            },
            None => None,
        }
    }

    fn set_model_state(&self, state: u8) {
        if let Some(f) = self.host.set_model_state {
            // SAFETY: documented precondition.
            unsafe { f(self.host.user, state) };
        }
    }

    fn set_clock_entry(&self, start_tenths: u8, end_tenths: u8) {
        if let Some(f) = self.host.set_clock_entry {
            // SAFETY: documented precondition.
            unsafe { f(self.host.user, start_tenths, end_tenths) };
        }
    }

    fn draw_chance(&self) -> u8 {
        match self.host.draw_chance {
            // SAFETY: documented precondition.
            Some(f) => unsafe { f(self.host.user) },
            None => 50,
        }
    }

    fn remove_from_city(&self) -> bool {
        match self.host.remove_from_city {
            // SAFETY: documented precondition.
            Some(f) => (unsafe { f(self.host.user) }) != 0,
            None => false,
        }
    }
}

impl EffectHost for EffectHandle {
    fn restart_effect(&self, id: u32, name: &[u8]) -> Option<u64> {
        let f = self.host.restart_effect?;
        // SAFETY: documented precondition; `name` is valid for `name.len()`.
        let h = unsafe { f(self.host.user, id, name.as_ptr(), name.len() as u32) };
        Some(h).filter(|&h| h != 0)
    }

    fn effect_slot3(&self, handle: u64) {
        if let Some(f) = self.host.effect_slot3 {
            // SAFETY: documented precondition.
            unsafe { f(self.host.user, handle) };
        }
    }

    fn effect_slot4(&self, handle: u64) {
        if let Some(f) = self.host.effect_slot4 {
            // SAFETY: documented precondition.
            unsafe { f(self.host.user, handle) };
        }
    }

    fn flag2(&self) -> bool {
        match self.host.flag2 {
            // SAFETY: documented precondition.
            Some(f) => (unsafe { f(self.host.user) }) != 0,
            None => false,
        }
    }
}

fn channel_code(c: RetChannel) -> u32 {
    match c {
        RetChannel::Eax => FC4_RET_EAX,
        RetChannel::Al => FC4_RET_AL,
        RetChannel::Void => FC4_RET_VOID,
        RetChannel::ViaCall => FC4_RET_VIA_CALL,
        RetChannel::EaxOrVoid => FC4_RET_EAX_OR_VOID,
    }
}

fn add_kind(code: u32) -> Option<AddKind> {
    match code {
        1 => Some(AddKind::City),
        2 => Some(AddKind::Lot),
        3 => Some(AddKind::Building),
        4 => Some(AddKind::Street),
        _ => None,
    }
}

// ---- the process-wide prop type table (§5.1) --------------------------------

/// A new, empty process-wide prop type table.
#[no_mangle]
pub extern "C" fn fc4_prop_type_table_create() -> *mut fc4_prop_type_table {
    guard(std::ptr::null_mut(), || {
        Box::into_raw(Box::new(Table(shared_type_table()))) as *mut fc4_prop_type_table
    })
}

/// Drops the caller's handle. Props created from it keep the table.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_type_table_destroy(t: *mut fc4_prop_type_table) {
    if !t.is_null() {
        guard((), || drop(Box::from_raw(t as *mut Table)));
    }
}

/// How many IDs the table holds (§5.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_type_table_len(t: *const fc4_prop_type_table) -> u32 {
    match (t as *const Table).as_ref() {
        Some(t) => guard(0, || t.0.borrow().len() as u32),
        None => 0,
    }
}

/// `GetPropTypeIndex` (`0x005EF2E0`): the index of `id`, interning it at the
/// end when it is not known yet (§5.1, quirk 2).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_type_table_intern(t: *const fc4_prop_type_table, id: u32) -> u32 {
    match (t as *const Table).as_ref() {
        Some(t) => guard(u32::MAX, || u32::from(t.0.borrow_mut().intern(id))),
        None => u32::MAX,
    }
}

/// `GetPropType` for one index (§5.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_type_table_id_at(t: *const fc4_prop_type_table, index: u32) -> u32 {
    match (t as *const Table).as_ref() {
        Some(t) => guard(0, || t.0.borrow().id_of(index as u16)),
        None => 0,
    }
}

// ---- the prop occupant ------------------------------------------------------

/// A new prop with the constructor's defaults (§2.2). `host` is copied; `types`
/// may be NULL, in which case the prop gets its own table (a host that wants
/// the process-wide table shares one handle).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_create(
    host: *const fc4_prop_occupant_host,
    types: *const fc4_prop_type_table,
) -> *mut fc4_prop_occupant {
    guard(std::ptr::null_mut(), || {
        let Some(h) = host.as_ref() else {
            return std::ptr::null_mut();
        };
        if (h.struct_size as usize) < std::mem::size_of::<fc4_prop_occupant_host>() {
            return std::ptr::null_mut();
        }
        let table = match (types as *const Table).as_ref() {
            Some(t) => t.0.clone(),
            None => shared_type_table(),
        };
        let handle = Handle {
            prop: PropOccupant::new(table),
            host: *h,
        };
        Box::into_raw(Box::new(handle)) as *mut fc4_prop_occupant
    })
}

/// Drops the handle. It does **not** release any reference: the core owns no
/// host reference (HANDOFF §5.1b).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_destroy(p: *mut fc4_prop_occupant) {
    if !p.is_null() {
        guard((), || drop(Box::from_raw(p as *mut Handle)));
    }
}

fn with_p<R>(p: *const fc4_prop_occupant, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live handle.
    match unsafe { (p as *const Handle).as_ref() } {
        Some(h) => guard(fallback, || f(h)),
        None => fallback,
    }
}

/// `QueryInterface` (§1.1): the byte offset of the sub-object the IID answers,
/// or `-1` when the base refuses it.
#[no_mangle]
pub extern "C" fn fc4_prop_occupant_query_interface(iid: u32) -> i32 {
    guard(-1, || PropOccupant::query_interface(iid).map_or(-1, |o| o as i32))
}

/// `AddRef` (slot 1). The count lives at `+0x14` (quirk 10).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_add_ref(p: *const fc4_prop_occupant) -> u32 {
    with_p(p, 0, |h| h.prop.add_ref())
}

/// `Release` (slot 2).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_release(p: *const fc4_prop_occupant) -> u32 {
    with_p(p, 0, |h| h.prop.release())
}

/// The reference count at `+0x14`.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_refcount(p: *const fc4_prop_occupant) -> u32 {
    with_p(p, 0, |h| h.prop.refcount())
}

/// `GetPropType` (slot 7, channel eax): `table[+0x5C]`.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_get_prop_type(p: *const fc4_prop_occupant) -> u32 {
    with_p(p, 0, |h| h.prop.get_prop_type())
}

/// `SetPropType` (slot 8). `[choice]` implemented as void (§4.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_set_prop_type(p: *const fc4_prop_occupant, id: u32) {
    with_p(p, (), |h| h.prop.set_prop_type(id));
}

/// The u16 index at `+0x5C` (§5.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_prop_type_index(p: *const fc4_prop_occupant) -> u32 {
    with_p(p, 0, |h| u32::from(h.prop.prop_type_index()))
}

/// `GetOrientation` (slot 9, channel eax).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_get_orientation(p: *const fc4_prop_occupant) -> i32 {
    with_p(p, 0, |h| h.prop.get_orientation())
}

/// `SetOrientation` (slot 10, channel **al**): clamps to 0…3 and returns 1.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_set_orientation(p: *const fc4_prop_occupant, orientation: i32) -> i32 {
    with_p(p, 0, |h| i32::from(h.prop.set_orientation(orientation)))
}

/// `GetState` (slot 12, channel **eax**): a clean zero-extended value.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_get_state(p: *const fc4_prop_occupant) -> u32 {
    with_p(p, 0, |h| h.prop.get_state())
}

/// `SetState` (slot 11, channel void), `nt` §13.1.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_set_state(p: *const fc4_prop_occupant, state: u8) {
    with_p(p, (), |h| h.prop.set_state(state, h));
}

/// `SetTimeOfDayRange` (slot 13, channel void, ST(0) empty at return).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_set_time_of_day_range(p: *const fc4_prop_occupant, start: f32, end: f32) {
    with_p(p, (), |h| h.prop.set_time_of_day_range(start, end, h));
}

/// `GetTimeOfDayRange` (slot 14): two `float*` out-params. **The method returns
/// nothing** and leaves ST(0) empty (§8 item 3); `a` and `b` may be NULL.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_get_time_of_day_range(
    p: *const fc4_prop_occupant,
    a: *mut f32,
    b: *mut f32,
) {
    with_p(p, (), |h| {
        let (x, y) = h.prop.get_time_of_day_range();
        if !a.is_null() {
            *a = x;
        }
        if !b.is_null() {
            *b = y;
        }
    });
}

/// `SetSimulatorDateRange` (slot 15, channel void): the `+0x54` sub-record.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_set_date_range(
    p: *const fc4_prop_occupant,
    interval: u32,
    duration: u32,
    start_day: u32,
    end_day: u32,
) {
    with_p(p, (), |h| {
        h.prop.set_simulator_date_range(DateRange {
            interval,
            duration,
            start_day,
            end_day,
        })
    });
}

/// Whether the prop has a date-range sub-record (record field 8).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_has_date_range(p: *const fc4_prop_occupant) -> i32 {
    with_p(p, 0, |h| i32::from(h.prop.has_date_range()))
}

/// `SetRandomChance` (slot 17, channel void).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_set_random_chance(p: *const fc4_prop_occupant, chance: u8) {
    with_p(p, (), |h| h.prop.set_random_chance(chance));
}

/// `GetRandomChance` (slot 18, channel **al**). The upper 24 bits of EAX are
/// undefined in the original (quirk 3); this zero-extends [choice], so a host
/// must mask to `0xFF` anyway. The value is a percent 1…100, never a bool.
///
/// [abi-ret] Slot 18 is `al` (docs/abi/interfaces.tsv says `eax-or-void`).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_get_random_chance(p: *const fc4_prop_occupant) -> u8 {
    with_p(p, 0, |h| h.prop.get_random_chance())
}

/// `GetPropTimerBitfield` (slot 19, channel eax).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_get_timer_bitfield(p: *const fc4_prop_occupant) -> u32 {
    with_p(p, 0, |h| h.prop.get_prop_timer_bitfield())
}

/// `GetPropTimerState(mask)` (slot 20, channel **eax**): a real 0/1.
///
/// [abi-ret] Slot 20 is `eax` with a real 0/1 (`sbb`/`neg`), not an `al` bool.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_get_timer_state(p: *const fc4_prop_occupant, mask: u8) -> i32 {
    with_p(p, 0, |h| i32::from(h.prop.get_prop_timer_state(mask)))
}

/// `SetPropTimerState(mask, on)` (slot 21, channel void): the transition draw
/// of quirk 7.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_set_timer_state(p: *const fc4_prop_occupant, mask: u8, on: i32) {
    with_p(p, (), |h| h.prop.set_prop_timer_state(mask, on != 0, h));
}

/// `GetPropUsage` (slot 22, channel **al**): a code 1…4, never a bool.
///
/// [abi-ret] Slot 22 is `al` (docs/abi/interfaces.tsv says `eax-or-void`).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_get_prop_usage(p: *const fc4_prop_occupant) -> u8 {
    with_p(p, 0, |h| h.prop.get_prop_usage())
}

/// `SetPropUsage` (slot 23, channel void).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_set_prop_usage(p: *const fc4_prop_occupant, usage: u8) {
    with_p(p, (), |h| h.prop.set_prop_usage(usage));
}

/// The prop usage each `Add*Prop` path writes (§3): `kind` 1 city, 2 lot,
/// 3 building, 4 street.
#[no_mangle]
pub extern "C" fn fc4_prop_add_usage(kind: u32) -> u8 {
    guard(0, || add_kind(kind).map_or(0, add_usage))
}

/// The lot tag at `+0x58` (§5.3); `0xFFFFFFFF` = none.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_lot_tag(p: *const fc4_prop_occupant) -> u32 {
    with_p(p, NO_LOT_TAG, |h| h.prop.lot_tag())
}

/// Occupant slot 27 `GetLotTag`: 1 when the prop has a tag.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_has_lot_tag(p: *const fc4_prop_occupant) -> i32 {
    with_p(p, 0, |h| i32::from(h.prop.has_lot_tag()))
}

/// Occupant slot 28 `SetLotTag`.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_set_lot_tag(p: *const fc4_prop_occupant, tag: u32) {
    with_p(p, (), |h| h.prop.set_lot_tag(tag));
}

/// `SetBoundingBox` (slot 4, channel void). `min` and `max` are three floats
/// each; NULL is ignored.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_set_bounding_box(
    p: *const fc4_prop_occupant,
    min: *const f32,
    max: *const f32,
) {
    with_p(p, (), |h| {
        if min.is_null() || max.is_null() {
            return;
        }
        let lo = [*min, *min.add(1), *min.add(2)];
        let hi = [*max, *max.add(1), *max.add(2)];
        h.prop.set_bounding_box(lo, hi);
    });
}

/// The bounding box into `min` and `max` (three floats each).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_get_bounding_box(p: *const fc4_prop_occupant, min: *mut f32, max: *mut f32) {
    with_p(p, (), |h| {
        let (lo, hi) = h.prop.bounding_box();
        if !min.is_null() {
            *min = lo[0];
            *min.add(1) = lo[1];
            *min.add(2) = lo[2];
        }
        if !max.is_null() {
            *max = hi[0];
            *max.add(1) = hi[1];
            *max.add(2) = hi[2];
        }
    });
}

/// The occupant base's `Init` (`lr` §2.4): 1 when `CreateOccupant` was posted.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_init_after_load(p: *const fc4_prop_occupant) -> i32 {
    with_p(p, 0, |h| i32::from(h.prop.init_after_load(h)))
}

/// `Shutdown` after `RemovePropA` (§3): 1 when it was still in the occupant
/// manager, so a second removal answers 0.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_shutdown(p: *const fc4_prop_occupant) -> i32 {
    with_p(p, 0, |h| i32::from(h.prop.shutdown(h)))
}

/// Mark the prop as loaded (the record reader).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_set_loaded(p: *const fc4_prop_occupant, loaded: i32) {
    with_p(p, (), |h| h.prop.set_loaded(loaded != 0));
}

// ---- the slot table (§4.1) --------------------------------------------------

/// The interface's slot count: 24 (§4.2).
#[no_mangle]
pub extern "C" fn fc4_prop_occupant_slot_count() -> u32 {
    guard(0, || VTABLE_LEN as u32)
}

/// Fills `out` with slot `index`. Returns 1 on success, 0 past the end.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_slot_info(index: u32, out: *mut fc4_prop_slot_info) -> i32 {
    guard(0, || {
        let Some(o) = out.as_mut() else {
            return 0;
        };
        match slot_spec(index as usize) {
            Some(s) => {
                *o = fc4_prop_slot_info {
                    slot: s.slot,
                    sdk_index: s.sdk_index,
                    callee_pop: s.callee_pop,
                    arg_dwords: s.arg_dwords,
                    offset: s.offset,
                    address: s.address,
                    ret: channel_code(s.ret),
                };
                1
            }
            None => 0,
        }
    })
}

/// The measured channel of one slot, or `-1` past the end (§4.1).
#[no_mangle]
pub extern "C" fn fc4_prop_occupant_slot_channel(index: u32) -> i32 {
    guard(-1, || {
        slot_channel(index as usize).map_or(-1, |c| channel_code(c) as i32)
    })
}

/// The object size of one occupant class (§1.2, test 3): `class_id`
/// `0x2977AA47` → `0x68`, `0x2977AA48` → `0xA0`, `0x2977AA49` → `0x94`,
/// `0xC9EFB016` → `0xD8`; anything else answers 0.
#[no_mangle]
pub extern "C" fn fc4_prop_object_size(class_id: u32) -> u32 {
    guard(0, || match class_id {
        CLSID_PROP_OCCUPANT => OBJECT_SIZE as u32,
        0x2977_AA48 => OBJECT_SIZE_TERRAIN_DECAL as u32,
        0x2977_AA49 => OBJECT_SIZE_VISUAL_EFFECT as u32,
        0xC9EF_B016 => OBJECT_SIZE_BURNT_BUILDING as u32,
        _ => 0,
    })
}

// ---- the save record (§7) ---------------------------------------------------

/// `Read` (`0x005EF360`): reads a §10.1 record out of `bytes` into the prop.
/// Returns [`FC4_PROP_RECORD_OK`], [`FC4_PROP_RECORD_BASE_FAILED`] (the quirk:
/// `Read` reports success without reading more) or [`FC4_PROP_RECORD_TRUNCATED`].
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_read_record(p: *const fc4_prop_occupant, bytes: *const u8, len: u32) -> i32 {
    guard(FC4_PROP_RECORD_TRUNCATED, || {
        if bytes.is_null() {
            return FC4_PROP_RECORD_TRUNCATED;
        }
        let data = std::slice::from_raw_parts(bytes, len as usize);
        let mut r = ByteReader::new(data);
        match read_prop_record(&mut r) {
            ReadOutcome::Ok(rec) => {
                with_p(p, (), |h| {
                    // The type is interned on read (§5.1, quirk 2).
                    rec.apply(&h.prop);
                });
                FC4_PROP_RECORD_OK
            }
            ReadOutcome::BaseFailed => FC4_PROP_RECORD_BASE_FAILED,
            ReadOutcome::Truncated => FC4_PROP_RECORD_TRUNCATED,
        }
    })
}

/// `Write` (`0x005ECE90`) of the record built from the live prop. `out` is
/// filled up to `cap`; `written` receives the byte count (which may exceed
/// `cap`, in which case nothing is written).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_occupant_write_record(
    p: *const fc4_prop_occupant,
    out: *mut u8,
    cap: u32,
    written: *mut u32,
) -> i32 {
    with_p(p, 0, |h| {
        let rec = PropRecord::of(&h.prop, OccupantBase::new(OCCUPANT_TYPE_PROP));
        let mut w = ByteWriter::new();
        if !write_prop_record(&rec, &mut w) {
            return 0;
        }
        let bytes = w.into_bytes();
        if !written.is_null() {
            *written = bytes.len() as u32;
        }
        if out.is_null() || (bytes.len() as u32) > cap {
            return 0;
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len());
        1
    })
}

/// Reads one record without touching a prop: `out_prop_type` receives field 3,
/// `out_timer_bits` field 13 as a reader resolves it (§7 field 13, quirk 5).
/// Returns the version, or `-1` when the record could not be read.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_record_read_summary(
    bytes: *const u8,
    len: u32,
    out_prop_type: *mut u32,
    out_timer_bits: *mut u8,
    out_lot_tag: *mut u32,
) -> i32 {
    guard(-1, || {
        if bytes.is_null() {
            return -1;
        }
        let data = std::slice::from_raw_parts(bytes, len as usize);
        let mut r = ByteReader::new(data);
        match read_prop_record(&mut r) {
            ReadOutcome::Ok(rec) => {
                if !out_prop_type.is_null() {
                    *out_prop_type = rec.prop_type;
                }
                if !out_timer_bits.is_null() {
                    *out_timer_bits = rec.effective_timer_bits();
                }
                if !out_lot_tag.is_null() {
                    *out_lot_tag = rec.lot_tag;
                }
                i32::from(rec.version)
            }
            _ => -1,
        }
    })
}

// ---- the lot tag and PV2 (§5.3) ---------------------------------------------

/// Classify a tag against a lot's configuration (§5.3, `pd` §4.3 step 2.3.2).
/// `ids` is the configuration's object IDs in record order; NULL means there is
/// no readable array, and `unknown` then distinguishes the two corpus
/// categories. Answers [`FC4_TAG_NONE`] for a prop with no tag.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_tag_classify(tag: u32, ids: *const u32, ids_len: u32, unknown: i32) -> i32 {
    guard(FC4_TAG_NONE, || {
        let slice = if ids.is_null() {
            None
        } else {
            Some(std::slice::from_raw_parts(ids, ids_len as usize))
        };
        match classify(tag, slice, unknown != 0) {
            None => FC4_TAG_NONE,
            Some(TagMatch::Matched { .. }) => FC4_TAG_MATCHED,
            Some(TagMatch::ConfigLacksTag) => FC4_TAG_CONFIG_LACKS_TAG,
            Some(TagMatch::NoConfiguration) => FC4_TAG_NO_CONFIGURATION,
            Some(TagMatch::UnknownConfiguration) => FC4_TAG_UNKNOWN_CONFIGURATION,
        }
    })
}

/// The PV2 corpus figures (§5.3): `which` 0 tagged, 1 unmatched, 2
/// config-lacks-tag, 3 no-configuration, 4 unknown-configuration,
/// 5 the unmatched percent × 100 (2770 for 27.7 %).
#[no_mangle]
pub extern "C" fn fc4_prop_tag_corpus(which: u32) -> u32 {
    use fc4_core::prop_occupant::corpus;
    guard(0, || match which {
        0 => corpus::TAGGED as u32,
        1 => corpus::UNMATCHED as u32,
        2 => corpus::CONFIG_LACKS_TAG as u32,
        3 => corpus::NO_CONFIGURATION as u32,
        4 => corpus::UNKNOWN_CONFIGURATION as u32,
        5 => (corpus::UNMATCHED_PERCENT * 100.0) as u32,
        _ => 0,
    })
}

// ---- the visual-effect prop (§6.2) ------------------------------------------

/// A new `cSC4PropOccupantVisualEffect` (§6.2). `types` may be NULL.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_effect_create(
    host: *const fc4_prop_effect_host,
    types: *const fc4_prop_type_table,
) -> *mut fc4_prop_effect {
    guard(std::ptr::null_mut(), || {
        let Some(h) = host.as_ref() else {
            return std::ptr::null_mut();
        };
        if (h.struct_size as usize) < std::mem::size_of::<fc4_prop_effect_host>() {
            return std::ptr::null_mut();
        }
        let table = match (types as *const Table).as_ref() {
            Some(t) => t.0.clone(),
            None => shared_type_table(),
        };
        Box::into_raw(Box::new(EffectHandle {
            effect: VisualEffectProp::new(table),
            host: *h,
        })) as *mut fc4_prop_effect
    })
}

/// Drops the handle.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_effect_destroy(e: *mut fc4_prop_effect) {
    if !e.is_null() {
        guard((), || drop(Box::from_raw(e as *mut EffectHandle)));
    }
}

fn with_e<R>(e: *const fc4_prop_effect, fallback: R, f: impl FnOnce(&EffectHandle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live handle.
    match unsafe { (e as *const EffectHandle).as_ref() } {
        Some(h) => guard(fallback, || f(h)),
        None => fallback,
    }
}

/// `SetEffectType(char const* name)` (slot 6): 1 when the type changed.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_effect_set_type_name(
    e: *const fc4_prop_effect,
    name: *const u8,
    name_len: u32,
) -> i32 {
    with_e(e, 0, |h| {
        let n = if name.is_null() {
            &[][..]
        } else {
            std::slice::from_raw_parts(name, name_len as usize)
        };
        i32::from(h.effect.set_effect_type_name(n, h))
    })
}

/// `SetEffectType(uint32 id)` (slot 5): 1 when the type changed.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_effect_set_type_id(e: *const fc4_prop_effect, id: u32) -> i32 {
    with_e(e, 0, |h| i32::from(h.effect.set_effect_type_id(id, h)))
}

/// `StartEffect(b)` (slot 7): 1 only when an effect object exists at `+0x6C`.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_effect_start(e: *const fc4_prop_effect, b: i32) -> i32 {
    with_e(e, 0, |h| i32::from(h.effect.start_effect(b != 0, h)))
}

/// `StopEffect(b)` (slot 8): 1 only when one existed and was started.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_effect_stop(e: *const fc4_prop_effect, b: i32) -> i32 {
    with_e(e, 0, |h| i32::from(h.effect.stop_effect(b != 0, h)))
}

/// Whether either effect handle is non-null (`lr` §10.2).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_effect_is_live(e: *const fc4_prop_effect) -> i32 {
    with_e(e, 0, |h| i32::from(h.effect.is_live()))
}

/// `started` at `+0x90`.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_effect_is_started(e: *const fc4_prop_effect) -> i32 {
    with_e(e, 0, |h| i32::from(h.effect.is_started()))
}

/// The record quirk (`lr` §10.2): 1 when `Write` drops the name and the
/// trailing u32 (test 22).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_effect_should_drop_name(e: *const fc4_prop_effect) -> i32 {
    with_e(e, 0, |h| i32::from(h.effect.should_drop_name_on_write()))
}

/// `AsOccupant` (slot 3) and `AsPropOccupant` (slot 4) offsets (`nt` §13.2).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_effect_sub_object_offsets(out: *mut u32) -> i32 {
    guard(0, || {
        if out.is_null() {
            return 0;
        }
        // SAFETY: documented precondition: `out` has two u32s.
        unsafe {
            *out = VisualEffectProp::as_prop_occupant_offset() as u32;
            *out.add(1) = VisualEffectProp::as_occupant_offset() as u32;
        }
        1
    })
}

/// `Write` of the effect record (`lr` §10.2). `live` selects the record quirk.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_effect_write_record(
    e: *const fc4_prop_effect,
    out: *mut u8,
    cap: u32,
    written: *mut u32,
) -> i32 {
    with_e(e, 0, |h| {
        let live = h.effect.should_drop_name_on_write();
        let rec = EffectRecord {
            version: EFFECT_RECORD_VERSION,
            prop: PropRecord::of(&h.effect.prop, OccupantBase::new(OCCUPANT_TYPE_PROP)),
            head: EffectRecordHead { byte0: 1, byte1: 0 },
            name: h.effect.effect_name(),
            trailer: 0,
        };
        let mut w = ByteWriter::new();
        if !write_effect_record(&rec, live, &mut w) {
            return 0;
        }
        let bytes = w.into_bytes();
        if !written.is_null() {
            *written = bytes.len() as u32;
        }
        if out.is_null() || (bytes.len() as u32) > cap {
            return 0;
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len());
        1
    })
}

/// `Read` of the effect record (`lr` §10.2). Answers 1 on success.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_effect_read_record(
    e: *const fc4_prop_effect,
    bytes: *const u8,
    len: u32,
    out_name_len: *mut u32,
) -> i32 {
    guard(0, || {
        if bytes.is_null() {
            return 0;
        }
        let data = std::slice::from_raw_parts(bytes, len as usize);
        let mut r = ByteReader::new(data);
        match read_effect_record(&mut r) {
            Some(rec) => {
                if !out_name_len.is_null() {
                    *out_name_len = rec.name.len() as u32;
                }
                with_e(e, (), |h| {
                    rec.prop.apply(&h.effect.prop);
                });
                1
            }
            None => 0,
        }
    })
}

// ---- constants ---------------------------------------------------------------

/// The class ID (§1), for a host that has no copy of the header's constants.
#[no_mangle]
pub extern "C" fn fc4_prop_class_id() -> u32 {
    guard(0, || CLSID_PROP_OCCUPANT)
}

/// The occupant type the constructor writes at `+0x2C` (§2.2).
#[no_mangle]
pub extern "C" fn fc4_prop_occupant_type() -> u32 {
    guard(0, || OCCUPANT_TYPE_PROP)
}

/// The empty lot tag (§5.3).
#[no_mangle]
pub extern "C" fn fc4_prop_no_lot_tag() -> u32 {
    guard(NO_LOT_TAG, || NO_LOT_TAG)
}

/// The record versions and the legacy timer bits (§7).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_record_versions(out: *mut u16) -> i32 {
    guard(0, || {
        if out.is_null() {
            return 0;
        }
        // SAFETY: documented precondition: `out` has three u16s.
        unsafe {
            *out = PROP_RECORD_VERSION;
            *out.add(1) = EFFECT_RECORD_VERSION;
            *out.add(2) = u16::from(LEGACY_TIMER_BITS);
            *out.add(3) = OCCUPANT_BASE_VERSION;
        }
        1
    })
}
