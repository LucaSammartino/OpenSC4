// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The layer-only disaster layer over the C ABI (fc4.h "layer-only disaster
//! layer", docs/specs/disasters.md §2.1-§2.7 and the §2.8 boundary).
//!
//! `enable = disasters` replaces only `cSC4DisasterLayer`: the seven
//! managers stay the game's own (§2.8 DS44: slot 7 must return the native
//! `cISC4DisasterManager` pointers), so the plugin keeps them and this state
//! holds no manager at all. It keeps the layer's own state: the enable
//! counter (§2.3), the ongoing count and the emergency pause (§2.4), the
//! damage assessment and dispatch timers (§2.5), the cost record (§2.6) and
//! the locations with their cursor (§2.7). Every game call goes through
//! [`fc4_dlayer_host`]; instances and occupants are opaque host tokens.
//!
//! The state sits behind a mutex (the game reads slots 10, 16 and 17 from
//! its second thread, docs/ops/lessons.md §5.1): another thread waits. A call
//! that arrives on the same thread while the handle is busy (a host callback
//! calling back in) is refused with the fallback value and counted in
//! [`fc4_dlayer_refused`].

use std::collections::BTreeMap;
use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use fc4_core::disasters::{DamageAssessment, Location, SpecialBuilding, Timer};

use crate::guard;

type U = *mut c_void;

/// The flag bits of the four kind pairs (§2.5 step 2.2).
const KIND_BITS: [u32; 4] = [0x1, 0x2, 0x4, 0x20];
/// Flags that make a special-building record (§2.5 step 2.3).
const SPECIAL_BITS: u32 = 0x0180_0400;
/// The network-piece cost when the property is absent (§2.5 step 3).
const NETWORK_DEFAULT_COST: i64 = 10;
/// `0xE94671EC` starts a dispatch timer; the other three stop it (§2.5).
const MSG_DISPATCH_START: u32 = 0xE946_71EC;

/// Occupant kinds for [`fc4_dlayer_occupant_removed`].
pub const FC4_DLAYER_OCC_OTHER: u32 = 0;
pub const FC4_DLAYER_OCC_BUILDING: u32 = 1;
pub const FC4_DLAYER_OCC_NETWORK: u32 = 2;

#[repr(C)]
pub struct fc4_dlayer {
    _private: [u8; 0],
}

/// The host table. A missing callback reads as 0 / false and writes nothing.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_dlayer_host {
    pub struct_size: u32,
    pub user: U,
    /// The wall clock in ms (the cRZTimers of §2.5, units 5 = seconds).
    pub now_ms: Option<unsafe extern "C" fn(U) -> u64>,
    /// Simulator slot 11 / slot 14 (§2.4).
    pub emergency_pause: Option<unsafe extern "C" fn(U)>,
    pub emergency_resume: Option<unsafe extern "C" fn(U)>,
    /// City slot 104 / slot 103 (§2.4).
    pub disable_save: Option<unsafe extern "C" fn(U)>,
    pub enable_save: Option<unsafe extern "C" fn(U)>,
    /// Post `0xCA1B1E64` (data1 = ongoing, data2 = x, data3 = z, data4 =
    /// the instance's `GetID`, IGZUnknown = the instance) (§2.4).
    pub post_started: Option<unsafe extern "C" fn(U, u32, i32, i32, u64)>,
    /// Post `0xAA1B2002` (§2.4).
    pub post_ended: Option<unsafe extern "C" fn(U)>,
    /// Subscribe (true) or unsubscribe (false) the five assessment messages
    /// (§2.5).
    pub subscribe: Option<unsafe extern "C" fn(U, bool)>,
    /// AddRef / Release an occupant token kept in a location (§2.7).
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    /// The "follow disasters" preference (§2.7).
    pub follow_disasters: Option<unsafe extern "C" fn(U) -> bool>,
    /// The camera part of `GoToNextDisaster` (cell x, z, zoom level).
    pub go_to: Option<unsafe extern "C" fn(U, i32, i32, u32)>,
    /// Occupant manager slot 27 `FindOccupant` (§2.7 resolution).
    pub occupant_in_manager: Option<unsafe extern "C" fn(U, u64) -> bool>,
    /// The occupant's position as a cell (occupant slot 8, city slot 95).
    pub occupant_cell: Option<unsafe extern "C" fn(U, u64, *mut i32, *mut i32) -> bool>,
    /// The instance's `GetLocation` (instance slot 6).
    pub instance_location: Option<unsafe extern "C" fn(U, u64, *mut i32, *mut i32) -> bool>,
    /// `GetTotalDispatchCount` of the fire (slot 25) and police (slot 26)
    /// simulators; false when either simulator is missing (§2.5 step 2.4).
    pub dispatch_counts: Option<unsafe extern "C" fn(U, *mut u32, *mut u32) -> bool>,
}

/// The statistics values of the §2.6 record (the plugin lays them out).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_dlayer_record {
    pub building_count: u32,
    pub special_count: u32,
    pub total_cost: i64,
    /// Counts of the kind pairs for the flag bits 1, 2, 4 and 0x20.
    pub kind_count: [u32; 4],
    pub kind_cost: [i64; 4],
    pub fire_effectiveness: f32,
    pub police_effectiveness: f32,
}

/// One location entry (§2.1, 20 bytes in the original).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_dlayer_location {
    pub instance: u64,
    pub occupant: u64,
    pub x: i32,
    pub z: i32,
    pub zoom: u32,
    pub reserved: u32,
}

#[derive(Default)]
struct State {
    initialised: bool,
    disable_counter: u32,
    ongoing: u32,
    assessing: bool,
    damage: DamageAssessment,
    fire_timers: BTreeMap<u32, Timer>,
    police_timers: BTreeMap<u32, Timer>,
    overall: Timer,
    fire_eff: f32,
    police_eff: f32,
    /// `Location::occupant` is the occupant token; the instance token is in
    /// the parallel `instances` (core's `Location::instance` is 32-bit).
    locations: Vec<Location>,
    instances: Vec<u64>,
    cursor: Option<usize>,
}

struct Handle {
    host: fc4_dlayer_host,
    state: Mutex<State>,
    refused: AtomicU32,
}

impl Handle {
    fn now(&self) -> u64 {
        self.host.now_ms.map_or(0, |f| unsafe { f(self.host.user) })
    }
    fn call(&self, f: Option<unsafe extern "C" fn(U)>) {
        if let Some(f) = f {
            unsafe { f(self.host.user) }
        }
    }
    fn tok(&self, f: Option<unsafe extern "C" fn(U, u64)>, t: u64) {
        if let (Some(f), true) = (f, t != 0) {
            unsafe { f(self.host.user, t) }
        }
    }
    fn xz(&self, f: Option<unsafe extern "C" fn(U, u64, *mut i32, *mut i32) -> bool>, t: u64) -> Option<(i32, i32)> {
        let f = f?;
        let (mut x, mut z) = (0, 0);
        unsafe { f(self.host.user, t, &mut x, &mut z) }.then_some((x, z))
    }

    // §2.5 start
    fn start_assessment(&self, s: &mut State) {
        if !s.assessing {
            if let Some(f) = self.host.subscribe {
                unsafe { f(self.host.user, true) }
            }
            s.assessing = true;
        }
        s.damage = DamageAssessment::default();
        s.fire_timers.clear();
        s.police_timers.clear();
        s.overall.restart(self.now());
    }

    // §2.5 end: unsubscribe, then EndDispatchTimers.
    fn end_assessment(&self, s: &mut State) {
        if s.assessing {
            if let Some(f) = self.host.subscribe {
                unsafe { f(self.host.user, false) }
            }
            s.assessing = false;
        }
        let now = self.now();
        s.overall.stop(now);
        for t in s.fire_timers.values_mut().chain(s.police_timers.values_mut()) {
            t.stop(now);
        }
        let (mut nf, mut np) = (0u32, 0u32);
        let have = self
            .host
            .dispatch_counts
            .is_some_and(|f| unsafe { f(self.host.user, &mut nf, &mut np) });
        if !have {
            return; // either simulator missing: neither value is written (§2.5 step 2.4)
        }
        let overall = s.overall.elapsed_secs_f32(now);
        let eff = |m: &BTreeMap<u32, Timer>, n: u32| -> f32 {
            let sum: f32 = m.values().map(|t| t.elapsed_secs_f32(now)).sum();
            let den = n as f32 * overall;
            // N = 0 gives 0; a zero overall time gives 0 [choice, §2.5].
            if n == 0 || den == 0.0 {
                0.0
            } else {
                sum / den
            }
        };
        s.fire_eff = eff(&s.fire_timers, nf);
        s.police_eff = eff(&s.police_timers, np);
    }

    // §2.7 ResolveDisasterLocation: the stored x, z change only here.
    fn resolve(&self, s: &mut State, i: usize) -> Option<(i32, i32)> {
        let l = *s.locations.get(i)?;
        let inst = s.instances[i];
        let r = if l.occupant != 0 {
            let held = self
                .host
                .occupant_in_manager
                .is_some_and(|f| unsafe { f(self.host.user, l.occupant) });
            if held {
                self.xz(self.host.occupant_cell, l.occupant)
            } else {
                None
            }
        } else {
            self.xz(self.host.instance_location, inst)
        };
        match r {
            Some((x, z)) => {
                s.locations[i].x = x;
                s.locations[i].z = z;
                Some((x, z))
            }
            None => Some((l.x, l.z)),
        }
    }

    fn most_recent(&self, s: &mut State, inst: u64) -> Option<(i32, i32)> {
        let i = (0..s.instances.len()).rev().find(|&i| inst == 0 || s.instances[i] == inst)?;
        self.resolve(s, i)
    }

    // §2.7 GetNextDisasterLocation + GoToNextDisaster's camera.
    fn go_to_next(&self, s: &mut State) {
        let n = s.locations.len();
        if n == 0 {
            return;
        }
        let c = s.cursor.map_or(n - 1, |c| c.min(n - 1));
        s.cursor = Some(if c == 0 { n - 1 } else { c - 1 });
        let zoom = s.locations[c].zoom;
        if let (Some((x, z)), Some(f)) = (self.resolve(s, c), self.host.go_to) {
            unsafe { f(self.host.user, x, z, zoom) }
        }
    }

    fn remove(&self, s: &mut State, f: impl Fn(u64, &Location) -> bool) {
        let mut keep_l = Vec::with_capacity(s.locations.len());
        let mut keep_i = Vec::with_capacity(s.locations.len());
        let ls = std::mem::take(&mut s.locations);
        let is = std::mem::take(&mut s.instances);
        for (l, i) in ls.into_iter().zip(is) {
            if f(i, &l) {
                self.tok(self.host.release, l.occupant);
            } else {
                keep_l.push(l);
                keep_i.push(i);
            }
        }
        s.locations = keep_l;
        s.instances = keep_i;
    }
}

unsafe fn with<R>(d: *mut fc4_dlayer, fallback: R, f: impl FnOnce(&Handle, &mut State) -> R) -> R {
    let Some(h) = (d as *const Handle).as_ref() else {
        return fallback;
    };
    // Same thread, same handle, already inside: a re-entrant host callback.
    // Another thread waits for the lock instead.
    let key = d as usize;
    if BUSY.with(|b| b.borrow().contains(&key)) {
        h.refused.fetch_add(1, Ordering::Relaxed);
        return fallback;
    }
    let mut s = match h.state.lock() {
        Ok(s) => s,
        Err(p) => p.into_inner(),
    };
    BUSY.with(|b| b.borrow_mut().push(key));
    let r = f(h, &mut s);
    BUSY.with(|b| b.borrow_mut().retain(|&k| k != key));
    r
}

thread_local! {
    static BUSY: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Creates the layer state (not initialised). The host table is copied.
#[no_mangle]
pub unsafe extern "C" fn fc4_dlayer_create(host: *const fc4_dlayer_host) -> *mut fc4_dlayer {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_dlayer_host>() => Box::into_raw(Box::new(Handle {
            host: *h,
            state: Mutex::new(State::default()),
            refused: AtomicU32::new(0),
        })) as *mut fc4_dlayer,
        _ => std::ptr::null_mut(),
    })
}

/// Destroys the state. Occupants still held are **not** released (call
/// [`fc4_dlayer_shutdown`] first).
#[no_mangle]
pub unsafe extern "C" fn fc4_dlayer_destroy(d: *mut fc4_dlayer) {
    if !d.is_null() {
        guard((), || drop(Box::from_raw(d as *mut Handle)));
    }
}

/// The layer part of `Init` (§2.2 step 3, §2.3): once; clears the disable
/// counter and resets the cost. Returns false when already initialised (the
/// plugin creates the native managers only when this is the first time).
#[no_mangle]
pub unsafe extern "C" fn fc4_dlayer_init(d: *mut fc4_dlayer) -> bool {
    with(d, false, |_, s| {
        if s.initialised {
            return false;
        }
        s.disable_counter = 0;
        s.damage = DamageAssessment::default();
        s.initialised = true;
        true
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_dlayer_initialised(d: *mut fc4_dlayer) -> bool {
    with(d, false, |_, s| s.initialised)
}

/// The layer part of `Shutdown` (§2.2), after the plugin has shut the
/// managers down: the locations cleared (occupants released), the timers
/// cleared, the assessment subscriptions dropped.
#[no_mangle]
pub unsafe extern "C" fn fc4_dlayer_shutdown(d: *mut fc4_dlayer) -> bool {
    with(d, false, |h, s| {
        h.remove(s, |_, _| true);
        s.cursor = None;
        s.fire_timers.clear();
        s.police_timers.clear();
        s.overall = Timer::default();
        if s.assessing {
            s.assessing = false;
            if let Some(f) = h.host.subscribe {
                f(h.host.user, false)
            }
        }
        s.initialised = false;
        true
    })
}

/// `EnableDisasters` (slot 5, §2.3): nests, not below 0; returns `counter == 0`.
#[no_mangle]
pub unsafe extern "C" fn fc4_dlayer_enable(d: *mut fc4_dlayer, enable: bool) -> bool {
    with(d, true, |_, s| {
        s.disable_counter = if enable {
            s.disable_counter.saturating_sub(1)
        } else {
            s.disable_counter.saturating_add(1)
        };
        s.disable_counter == 0
    })
}

/// `DisastersEnabled` (slot 6).
#[no_mangle]
pub unsafe extern "C" fn fc4_dlayer_enabled(d: *mut fc4_dlayer) -> bool {
    with(d, true, |_, s| s.disable_counter == 0)
}

/// `DisasterHasBegun(instance)` (slot 8, §2.4).
#[no_mangle]
pub unsafe extern "C" fn fc4_dlayer_begun(d: *mut fc4_dlayer, instance: u64) -> bool {
    with(d, false, |h, s| {
        s.ongoing = s.ongoing.saturating_add(1);
        if s.ongoing == 1 {
            h.call(h.host.emergency_pause);
            h.start_assessment(s);
            h.call(h.host.disable_save);
        }
        let (x, z) = h.most_recent(s, instance).unwrap_or((0, 0));
        if let Some(f) = h.host.post_started {
            f(h.host.user, s.ongoing, x, z, instance);
        }
        true
    })
}

/// `DisasterHasEnded()` (slot 9, §2.4): only while initialised and ongoing.
#[no_mangle]
pub unsafe extern "C" fn fc4_dlayer_ended(d: *mut fc4_dlayer) -> bool {
    with(d, false, |h, s| {
        if !s.initialised || s.ongoing == 0 {
            return false;
        }
        s.ongoing -= 1;
        if s.ongoing == 0 {
            h.call(h.host.enable_save);
            h.call(h.host.emergency_resume);
            h.end_assessment(s);
            h.call(h.host.post_ended);
        }
        true
    })
}

/// The ongoing count (`IsADisasterOngoing`, slot 10, is `!= 0`).
#[no_mangle]
pub unsafe extern "C" fn fc4_dlayer_ongoing(d: *mut fc4_dlayer) -> u32 {
    with(d, 0, |_, s| s.ongoing)
}

/// `DoOnOccupantRemoval` (§2.5), after the plugin has checked
/// `city.GetEstablished()` and read the occupant: `kind` is one of
/// `FC4_DLAYER_OCC_*`, `cost` the `0x8A1C3E72` value when `has_cost`, `flags`
/// the building profile's flag word. Ignored while not assessing.
#[no_mangle]
pub unsafe extern "C" fn fc4_dlayer_occupant_removed(
    d: *mut fc4_dlayer,
    kind: u32,
    has_cost: bool,
    cost: i64,
    flags: u32,
) -> bool {
    with(d, false, |_, s| {
        if !s.assessing {
            return false;
        }
        let mut c = if has_cost { cost } else { 0 };
        let dmg = &mut s.damage;
        if kind == FC4_DLAYER_OCC_BUILDING {
            dmg.building_count = dmg.building_count.wrapping_add(1);
            for (i, bit) in KIND_BITS.iter().enumerate() {
                if flags & bit != 0 {
                    dmg.kinds[i].0 = dmg.kinds[i].0.wrapping_add(1);
                    dmg.kinds[i].1 = dmg.kinds[i].1.wrapping_add(c);
                }
            }
            if flags & SPECIAL_BITS != 0 {
                dmg.special.push(SpecialBuilding {
                    occupant: 0,
                    flags,
                    cost: c,
                });
            }
        } else if kind == FC4_DLAYER_OCC_NETWORK && !has_cost {
            c = NETWORK_DEFAULT_COST;
        }
        dmg.total_cost = dmg.total_cost.wrapping_add(c);
        true
    })
}

/// `DoDispatchMessage` (§2.5): `dispatch_type` = data1, `id` = data2.
#[no_mangle]
pub unsafe extern "C" fn fc4_dlayer_dispatch(d: *mut fc4_dlayer, msg: u32, dispatch_type: i32, id: u32) -> bool {
    with(d, false, |h, s| {
        let now = h.now();
        let map = match dispatch_type {
            1 | 5 => &mut s.fire_timers,
            0 | 6 => &mut s.police_timers,
            _ => return false,
        };
        let t = map.entry(id).or_default();
        if msg == MSG_DISPATCH_START {
            t.start(now);
        } else {
            t.stop(now);
        }
        true
    })
}

/// `ResetDisasterCost` (§2.6): counts, costs and special records zeroed.
#[no_mangle]
pub unsafe extern "C" fn fc4_dlayer_reset_cost(d: *mut fc4_dlayer) -> bool {
    with(d, false, |_, s| {
        s.damage = DamageAssessment::default();
        true
    })
}

/// The values of the §2.6 statistics record.
#[no_mangle]
pub unsafe extern "C" fn fc4_dlayer_record(d: *mut fc4_dlayer, out: *mut fc4_dlayer_record) -> bool {
    if out.is_null() {
        return false;
    }
    with(d, false, |_, s| {
        let dmg = &s.damage;
        *out = fc4_dlayer_record {
            building_count: dmg.building_count,
            special_count: dmg.special.len() as u32,
            total_cost: dmg.total_cost,
            kind_count: dmg.kinds.map(|k| k.0),
            kind_cost: dmg.kinds.map(|k| k.1),
            fire_effectiveness: s.fire_eff,
            police_effectiveness: s.police_eff,
        };
        true
    })
}

/// `RegisterDisasterLocation` (slot 13, §2.7): the occupant AddRef'd, the
/// cursor on the new entry, then `GoToNextDisaster` when `focus` and the
/// preference are set.
#[no_mangle]
pub unsafe extern "C" fn fc4_dlayer_register(
    d: *mut fc4_dlayer,
    instance: u64,
    x: i32,
    z: i32,
    occupant: u64,
    zoom: u32,
    focus: bool,
) -> bool {
    with(d, false, |h, s| {
        h.tok(h.host.add_ref, occupant);
        s.locations.push(Location {
            instance: 0,
            x,
            z,
            zoom,
            occupant,
        });
        s.instances.push(instance);
        s.cursor = Some(s.locations.len() - 1);
        if focus && h.host.follow_disasters.is_some_and(|f| f(h.host.user)) {
            h.go_to_next(s);
        }
        true
    })
}

/// `UnregisterDisasterLocations(instance)` (slot 14).
#[no_mangle]
pub unsafe extern "C" fn fc4_dlayer_unregister(d: *mut fc4_dlayer, instance: u64) -> bool {
    with(d, false, |h, s| {
        h.remove(s, |i, _| i == instance);
        true
    })
}

/// `UnregisterDisasterLocationByOccupant(instance, occupant)` (slot 15).
#[no_mangle]
pub unsafe extern "C" fn fc4_dlayer_unregister_by_occupant(d: *mut fc4_dlayer, instance: u64, occupant: u64) -> bool {
    with(d, false, |h, s| {
        h.remove(s, |i, l| i == instance && l.occupant == occupant);
        true
    })
}

/// `GetMostRecentDisasterLocation(instance, &x, &z)` (slot 12): the newest
/// entry (of `instance`, or overall when it is 0), resolved.
#[no_mangle]
pub unsafe extern "C" fn fc4_dlayer_most_recent(d: *mut fc4_dlayer, instance: u64, x: *mut i32, z: *mut i32) -> bool {
    with(d, false, |h, s| match h.most_recent(s, instance) {
        Some((a, b)) => {
            if !x.is_null() {
                *x = a;
            }
            if !z.is_null() {
                *z = b;
            }
            true
        }
        None => false,
    })
}

/// `GetDisasterLocationCount` (slot 16).
#[no_mangle]
pub unsafe extern "C" fn fc4_dlayer_location_count(d: *mut fc4_dlayer) -> i32 {
    with(d, 0, |_, s| s.locations.len() as i32)
}

/// `GetDisasterLocationByIndex` (slot 17): the stored values, not resolved.
#[no_mangle]
pub unsafe extern "C" fn fc4_dlayer_location_at(d: *mut fc4_dlayer, index: i32, out: *mut fc4_dlayer_location) -> bool {
    if out.is_null() || index < 0 {
        return false;
    }
    with(d, false, |_, s| match s.locations.get(index as usize) {
        Some(l) => {
            *out = fc4_dlayer_location {
                instance: s.instances[index as usize],
                occupant: l.occupant,
                x: l.x,
                z: l.z,
                zoom: l.zoom,
                reserved: 0,
            };
            true
        }
        None => false,
    })
}

/// `GoToNextDisaster` (slot 11, §2.7).
#[no_mangle]
pub unsafe extern "C" fn fc4_dlayer_go_to_next(d: *mut fc4_dlayer) -> bool {
    with(d, false, |h, s| {
        h.go_to_next(s);
        true
    })
}

/// Whether the assessment subscriptions are live (§2.5).
#[no_mangle]
pub unsafe extern "C" fn fc4_dlayer_assessing(d: *mut fc4_dlayer) -> bool {
    with(d, false, |_, s| s.assessing)
}

/// Calls refused because the handle was busy (a re-entrant host callback).
#[no_mangle]
pub unsafe extern "C" fn fc4_dlayer_refused(d: *mut fc4_dlayer) -> u32 {
    (d as *const Handle).as_ref().map_or(0, |h| h.refused.load(Ordering::Relaxed))
}
