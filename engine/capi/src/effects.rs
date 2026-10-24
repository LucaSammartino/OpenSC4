// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The effects manager over the C ABI (fc4.h "effects manager",
//! docs/specs/effects.md).
//!
//! Two objects: an effect directory ([`fc4_effects_directory`], parsed from
//! the resource `0xEA5118B0` / `0xEA5118B1` / 1 the host reads from the
//! game's packages, immutable, shared by reference) and the manager
//! ([`fc4_effects`]) the host ticks. Effects are named by the core's IDs
//! (from 1, never 0).
//!
//! The host implements [`fc4_effects_host`]: the simulator (its dilation;
//! a missing `simulator` callback, or one returning false, reads as "no
//! simulator"), the pauses, the city size and bounds, the terrain brushes
//! (`apply_brush` takes the terrain's own [`fc4_brush_params`], so the host
//! can forward it to `fc4_terrain_apply_brush`), the long terrain operation,
//! fire (`ignite_cell_radius`, `set_fire_cause`), toxic spills, demolition
//! (with the record's filter, which [`fc4_effects_demolish_filter_skips`]
//! applies to an occupant), extinguishing and the message posts. A missing
//! callback does nothing and reads as 0 / false.
//!
//! Finishing (EF2): in the default mode the host calls
//! [`fc4_effects_finish_hook`] once per frame after the tick; the compat
//! mode finishes only in [`fc4_effects_draw_pass`], as the original does
//! from its draw pass.
//!
//! Re-entrancy: a call into the same manager from inside a callback is
//! refused (the fallback value). Threads: single-threaded; the plugin
//! serialises every call (fc4.h).

use std::cell::RefCell;
use std::ffi::{c_char, c_void, CStr};
use std::rc::Rc;

use fc4_core::effects::{
    component_type, shipped, Component, DemolishFilter, EffectDirectory, EffectId, EffectPost, EffectRect, EffectsHost,
    EffectsManager, FinishMode, OccupantFacts,
};
use fc4_core::terrain::BrushParams;

use crate::guard;
use crate::terrain::fc4_brush_params;

type U = *mut c_void;

#[repr(C)]
pub struct fc4_effects_directory {
    _private: [u8; 0],
}

#[repr(C)]
pub struct fc4_effects {
    _private: [u8; 0],
}

/// The finish modes (`fc4_effects_set_finish_mode`).
pub const FC4_EFFECTS_FINISH_HOOK: u32 = 0;
pub const FC4_EFFECTS_FINISH_DRAW_PASS: u32 = 1;

/// `fc4_effects_directory_find`: the name is unknown.
pub const FC4_EFFECTS_UNKNOWN: i32 = -2;

/// A message the core posts (fc4.h `fc4_effects_post`, effects.md §9.2,
/// §6): the end message of a finished effect (`data1` as set, `data2` =
/// `data3` = 0) or a game record's message (`data1` the instance
/// parameter, `data2`/`data3` the cell).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_effects_post {
    pub msg_type: u32,
    pub data2: i32,
    pub data1: u64,
    pub data3: i32,
    pub reserved: u32,
}

/// An inclusive cell rectangle, clipped to the city.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_effects_rect {
    pub x0: i32,
    pub z0: i32,
    pub x1: i32,
    pub z1: i32,
}

/// A game record's demolition filter (§9.3).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_effects_demolish_filter {
    pub no_networks: bool,
    pub no_flora: bool,
    pub _pad: [u8; 2],
    pub min_size: f32,
    pub max_size: f32,
}

/// What the filter needs of an occupant.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_effects_occupant_facts {
    pub signpost: bool,
    pub text_decal: bool,
    pub network: bool,
    pub flora: bool,
    /// The occupant's extents (x, y, z).
    pub extents: [f32; 3],
}

/// What the effects manager needs from the game (fc4.h
/// `fc4_effects_host`), in the order of `EffectsHost`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_effects_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    /// Whether there is a simulator; `*dilation` is its slot 32.
    pub simulator: Option<unsafe extern "C" fn(U, *mut f32) -> bool>,
    pub sim_pause: Option<unsafe extern "C" fn(U)>,
    pub sim_resume: Option<unsafe extern "C" fn(U)>,
    pub sim_hidden_pause: Option<unsafe extern "C" fn(U)>,
    pub sim_hidden_resume: Option<unsafe extern "C" fn(U)>,
    pub clock_pause: Option<unsafe extern "C" fn(U, bool)>,
    pub city_cells: Option<unsafe extern "C" fn(U, *mut i32, *mut i32)>,
    pub location_in_bounds: Option<unsafe extern "C" fn(U, f32, f32) -> bool>,
    /// `ApplyBrush(brushID, params)` on the terrain.
    pub apply_brush: Option<unsafe extern "C" fn(U, u32, *mut fc4_brush_params) -> bool>,
    pub begin_long_terrain_operation: Option<unsafe extern "C" fn(U)>,
    pub end_long_terrain_operation: Option<unsafe extern "C" fn(U)>,
    /// The fire simulator's `IgniteCellRadius(x, z, r0, r1)`.
    pub ignite_cell_radius: Option<unsafe extern "C" fn(U, i32, i32, i32, i32)>,
    /// The fire simulator's `SetCause`.
    pub set_fire_cause: Option<unsafe extern "C" fn(U, u32)>,
    /// The toxic-spill disaster's attempt at a cell.
    pub toxic_attempt_start: Option<unsafe extern "C" fn(U, i32, i32)>,
    /// Demolish the occupants of `rect` that the filter does not skip
    /// (`flags` the record's demolition flags; `dezone` its dezone flag).
    pub demolish_region:
        Option<unsafe extern "C" fn(U, *const fc4_effects_rect, u32, bool, *const fc4_effects_demolish_filter)>,
    pub extinguish_cell: Option<unsafe extern "C" fn(U, i32, i32, i32)>,
    pub post: Option<unsafe extern "C" fn(U, *const fc4_effects_post)>,
    /// `IsActive` of a render component (`component` its index in the
    /// effect, `kind` its type); missing: inactive.
    pub render_component_active: Option<unsafe extern "C" fn(U, u32, u32, u8) -> bool>,
}

/// A directory's section sizes (`fc4_effects_directory_get_counts`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_effects_directory_counts {
    pub brushes: u32,
    pub attractors: u32,
    pub games: u32,
    pub effects: u32,
    pub names: u32,
    pub root_version: u16,
    pub effects_version: u16,
}

/// The manager's state (`fc4_effects_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_effects_state {
    pub created: u64,
    pub recycled: u64,
    pub end_posts: u64,
    pub select_draws: u64,
    pub ticks: u64,
    pub sweeps: u64,
    /// The accumulated (dilated) microseconds and seconds.
    pub total_us: u64,
    pub total_seconds: f32,
    pub effect_count: u32,
    pub active_count: u32,
    pub level_of_detail: u32,
    pub max_priority: u32,
    pub finish_mode: u32,
}

/// An effect's state (`fc4_effects_get_effect`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_effect_info {
    /// The description (-1: none; the effect is a no-op).
    pub index: i32,
    pub flags: u32,
    pub refs: u32,
    pub started: bool,
    pub stopping: bool,
    pub active: bool,
    pub _pad: u8,
    pub components: u32,
    pub end_type: u32,
    pub end_data1: u64,
    pub position: [f32; 3],
    pub reserved: u32,
}

/// A brush component's state (`fc4_effects_get_brush`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_effects_brush_state {
    pub brush_id: u32,
    pub rate: f32,
    pub length: f32,
    pub strength: f32,
    pub radius: f32,
    pub acc: f32,
    pub t: f32,
    pub prev: [f32; 2],
    pub cur: [f32; 2],
    pub one_shot: bool,
    pub active: bool,
    pub long_op: bool,
    pub _pad: u8,
    pub applications: u64,
}

/// A game component's state (`fc4_effects_get_game`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_effects_game_state {
    pub started: bool,
    pub pending: bool,
    pub map_applied: bool,
    pub _pad: u8,
    pub timer: f32,
    pub count: u32,
    pub applies: u32,
    pub cell_x: i32,
    pub cell_z: i32,
    pub data: u64,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_effects_post>() == 24);
    assert!(std::mem::size_of::<fc4_effects_rect>() == 16);
    assert!(std::mem::size_of::<fc4_effects_demolish_filter>() == 12);
    assert!(std::mem::size_of::<fc4_effects_occupant_facts>() == 16);
    assert!(std::mem::size_of::<fc4_effects_directory_counts>() == 24);
    assert!(std::mem::size_of::<fc4_effects_state>() == 80);
    assert!(std::mem::size_of::<fc4_effect_info>() == 48);
    assert!(std::mem::size_of::<fc4_effects_brush_state>() == 56);
    assert!(std::mem::size_of::<fc4_effects_game_state>() == 32);
    assert!(std::mem::size_of::<fc4_effects_host>() == if cfg!(target_pointer_width = "32") { 80 } else { 160 });
};

// ---- host adapter -----------------------------------------------------------------------------------

struct CHost {
    h: fc4_effects_host,
}

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $s.h.$f { Some(f) => unsafe { f($s.h.user $(, $arg)*) }, None => $default }
    };
}

impl EffectsHost for CHost {
    fn simulator(&mut self) -> Option<f32> {
        let mut d = 0.0f32;
        cb!(self, simulator, false, &mut d).then_some(d)
    }
    fn sim_pause(&mut self) {
        cb!(self, sim_pause, ())
    }
    fn sim_resume(&mut self) {
        cb!(self, sim_resume, ())
    }
    fn sim_hidden_pause(&mut self) {
        cb!(self, sim_hidden_pause, ())
    }
    fn sim_hidden_resume(&mut self) {
        cb!(self, sim_hidden_resume, ())
    }
    fn clock_pause(&mut self, on: bool) {
        cb!(self, clock_pause, (), on)
    }
    fn city_cells(&mut self) -> (i32, i32) {
        let (mut x, mut z) = (0i32, 0i32);
        cb!(self, city_cells, (), &mut x, &mut z);
        (x, z)
    }
    fn location_in_bounds(&mut self, x: f32, z: f32) -> bool {
        cb!(self, location_in_bounds, false, x, z)
    }
    fn apply_brush(&mut self, brush_id: u32, params: &mut BrushParams) -> bool {
        let mut p: fc4_brush_params = (*params).into();
        let ok = cb!(self, apply_brush, false, brush_id, &mut p);
        *params = BrushParams::from(&p);
        ok
    }
    fn begin_long_terrain_operation(&mut self) {
        cb!(self, begin_long_terrain_operation, ())
    }
    fn end_long_terrain_operation(&mut self) {
        cb!(self, end_long_terrain_operation, ())
    }
    fn ignite_cell_radius(&mut self, x: i32, z: i32, r0: i32, r1: i32) {
        cb!(self, ignite_cell_radius, (), x, z, r0, r1)
    }
    fn set_fire_cause(&mut self, cause: u32) {
        cb!(self, set_fire_cause, (), cause)
    }
    fn toxic_attempt_start(&mut self, x: i32, z: i32) {
        cb!(self, toxic_attempt_start, (), x, z)
    }
    fn demolish_region(&mut self, r: EffectRect, flags: u32, dezone: bool, f: &DemolishFilter) {
        let rect = fc4_effects_rect {
            x0: r.x0,
            z0: r.z0,
            x1: r.x1,
            z1: r.z1,
        };
        let filter = filter_to_c(f);
        cb!(self, demolish_region, (), &rect, flags, dezone, &filter)
    }
    fn extinguish_cell(&mut self, x: i32, z: i32, amount: i32) {
        cb!(self, extinguish_cell, (), x, z, amount)
    }
    fn post(&mut self, m: EffectPost) {
        let p = fc4_effects_post {
            msg_type: m.msg_type,
            data2: m.data2,
            data1: m.data1,
            data3: m.data3,
            reserved: 0,
        };
        cb!(self, post, (), &p)
    }
    fn render_component_active(&mut self, effect: EffectId, component: u32, kind: u8) -> bool {
        cb!(self, render_component_active, false, effect, component, kind)
    }
}

fn filter_to_c(f: &DemolishFilter) -> fc4_effects_demolish_filter {
    fc4_effects_demolish_filter {
        no_networks: f.no_networks,
        no_flora: f.no_flora,
        _pad: [0; 2],
        min_size: f.min_size,
        max_size: f.max_size,
    }
}

/// Record-only effects.md sections 3/4 name-membership comparison. `counts`
/// receives directory names, native queries and field differences, in order.
/// No native answer is used to build or modify the parsed directory.
///
/// # Safety
/// `data` spans `len` readable bytes; `counts` spans three writable u32s.
/// The callback accepts a temporary NUL-terminated name and returns 0 or 1.
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_verify_names(
    data: *const u8,
    len: usize,
    has_name: Option<unsafe extern "C" fn(U, *const c_char) -> u32>,
    user: U,
    counts: *mut u32,
) -> bool {
    guard(false, || {
        if counts.is_null() {
            return false;
        }
        let counts = std::slice::from_raw_parts_mut(counts, 3);
        counts.fill(0);
        let has_name = match has_name {
            Some(f) if !data.is_null() => f,
            _ => return false,
        };
        let directory = match EffectDirectory::parse(std::slice::from_raw_parts(data, len)) {
            Ok(d) if !d.names.is_empty() => d,
            _ => return false,
        };
        let mut probes = Vec::new();
        for (name, _) in &directory.names {
            for variant in [name.clone(), name.to_ascii_uppercase(), name.to_ascii_lowercase()] {
                match std::ffi::CString::new(variant) {
                    Ok(name) => probes.push(name),
                    Err(_) => return false,
                }
            }
        }
        // Literal section 4 membership only. EF20 blocks native description
        // capture; this does not claim priority, publication or placement state.
        for name in ["", "end", "__fc4_effects_shadow_absent__"] {
            probes.push(std::ffi::CString::new(name).unwrap());
        }
        counts[0] = directory.names.len() as u32;
        for name in probes {
            let expected = u32::from(directory.find(name.to_str().unwrap(), u32::MAX).is_some());
            let native = has_name(user, name.as_ptr());
            counts[1] += 1;
            counts[2] += u32::from(native != expected);
        }
        true
    })
}

// ---- the directory ----------------------------------------------------------------------------------

fn dir_ref<'a>(d: *const fc4_effects_directory) -> Option<&'a Rc<EffectDirectory>> {
    // SAFETY: documented precondition: NULL or a live fc4_effects_directory*.
    unsafe { (d as *const Rc<EffectDirectory>).as_ref() }
}

fn new_dir(d: EffectDirectory) -> *mut fc4_effects_directory {
    Box::into_raw(Box::new(Rc::new(d))) as *mut fc4_effects_directory
}

/// Parses an effect directory resource (§3); NULL when it does not parse.
/// The bytes are copied as needed; the directory is immutable.
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_directory_parse(data: *const u8, len: usize) -> *mut fc4_effects_directory {
    guard(std::ptr::null_mut(), || {
        if data.is_null() {
            return std::ptr::null_mut();
        }
        let bytes = std::slice::from_raw_parts(data, len);
        EffectDirectory::parse(bytes).map_or(std::ptr::null_mut(), new_dir)
    })
}

/// A small synthetic directory with the shipped brush and game records and
/// the volcano and meteor-crater descriptions (`shipped::volcano_directory`),
/// for tests and hosts without an install.
#[no_mangle]
pub extern "C" fn fc4_effects_directory_volcano() -> *mut fc4_effects_directory {
    guard(std::ptr::null_mut(), || new_dir(shipped::volcano_directory()))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_effects_directory_destroy(d: *mut fc4_effects_directory) {
    if !d.is_null() {
        guard((), || drop(Box::from_raw(d as *mut Rc<EffectDirectory>)));
    }
}

#[no_mangle]
pub unsafe extern "C" fn fc4_effects_directory_get_counts(
    d: *const fc4_effects_directory,
    out: *mut fc4_effects_directory_counts,
) -> bool {
    guard(false, || match (dir_ref(d), out.as_mut()) {
        (Some(d), Some(o)) => {
            *o = fc4_effects_directory_counts {
                brushes: d.brushes.len() as u32,
                attractors: d.attractors.len() as u32,
                games: d.games.len() as u32,
                effects: d.effects.len() as u32,
                names: d.names.len() as u32,
                root_version: d.root_version,
                effects_version: d.effects_version,
            };
            true
        }
        _ => false,
    })
}

/// The description `name` resolves to under `max_priority` (§4): its index,
/// -1 when none qualifies, `FC4_EFFECTS_UNKNOWN` for an unknown name.
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_directory_find(
    d: *const fc4_effects_directory,
    name: *const c_char,
    max_priority: u32,
) -> i32 {
    guard(FC4_EFFECTS_UNKNOWN, || match (dir_ref(d), c_str(name)) {
        (Some(d), Some(n)) => d.find(n, max_priority).unwrap_or(FC4_EFFECTS_UNKNOWN),
        _ => FC4_EFFECTS_UNKNOWN,
    })
}

/// Writes the directory resource (the parser's inverse) into `buf`
/// (`cap` bytes; NULL to measure). Returns the size it needs.
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_directory_write(
    d: *const fc4_effects_directory,
    buf: *mut u8,
    cap: usize,
) -> usize {
    guard(0, || match dir_ref(d) {
        Some(d) => {
            let bytes = d.to_bytes();
            if !buf.is_null() && cap >= bytes.len() {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf, bytes.len());
            }
            bytes.len()
        }
        None => 0,
    })
}

fn c_str<'a>(s: *const c_char) -> Option<&'a str> {
    if s.is_null() {
        return None;
    }
    // SAFETY: documented precondition: a NUL-terminated string.
    unsafe { CStr::from_ptr(s) }.to_str().ok()
}

// ---- the manager ------------------------------------------------------------------------------------

struct Handle {
    mgr: RefCell<EffectsManager>,
    host: RefCell<CHost>,
}

/// Runs `f` on the manager and the host; the fallback for NULL or a
/// re-entrant call.
fn with_fx<R>(fx: *const fc4_effects, fallback: R, f: impl FnOnce(&mut EffectsManager, &mut CHost) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_effects*.
    let Some(h) = (unsafe { (fx as *const Handle).as_ref() }) else {
        return fallback;
    };
    let (Ok(mut m), Ok(mut c)) = (h.mgr.try_borrow_mut(), h.host.try_borrow_mut()) else {
        return fallback;
    };
    guard(fallback, || f(&mut m, &mut c))
}

/// Creates a manager with no directory (nothing can be created until
/// `fc4_effects_set_directory`). The host table is copied. `session_seed`
/// seeds the brush generator (foundations §6.3); the select generator is
/// seeded with the constant `0x1105` (§5.3).
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_create(host: *const fc4_effects_host, session_seed: u32) -> *mut fc4_effects {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_effects_host>() => {
            Box::into_raw(Box::new(Handle {
                mgr: RefCell::new(EffectsManager::new(session_seed)),
                host: RefCell::new(CHost { h: *h }),
            })) as *mut fc4_effects
        }
        _ => std::ptr::null_mut(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_effects_destroy(fx: *mut fc4_effects) {
    if !fx.is_null() {
        guard((), || drop(Box::from_raw(fx as *mut Handle)));
    }
}

/// Shares the directory (the manager keeps its own reference; the caller
/// may destroy its handle).
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_set_directory(fx: *mut fc4_effects, d: *const fc4_effects_directory) -> bool {
    let Some(d) = dir_ref(d).cloned() else { return false };
    with_fx(fx, false, |m, _| {
        m.set_directory(d);
        true
    })
}

/// The detail setting: the highest description priority created (§4;
/// default 5).
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_set_max_priority(fx: *mut fc4_effects, p: u32) {
    with_fx(fx, (), |m, _| m.set_max_priority(p))
}

/// The camera zoom (§5.1): the level of detail becomes
/// `max(zoom - offset + 1, 1)` and every effect is rebuilt.
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_set_zoom(fx: *mut fc4_effects, zoom: i32, offset: i32) {
    with_fx(fx, (), |m, h| m.set_zoom(zoom, offset, h))
}

/// `FC4_EFFECTS_FINISH_HOOK` (default) or `FC4_EFFECTS_FINISH_DRAW_PASS`
/// (EF2); false for another value.
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_set_finish_mode(fx: *mut fc4_effects, mode: u32) -> bool {
    let mode = match mode {
        FC4_EFFECTS_FINISH_HOOK => FinishMode::Hook,
        FC4_EFFECTS_FINISH_DRAW_PASS => FinishMode::DrawPass,
        _ => return false,
    };
    with_fx(fx, false, |m, _| {
        m.set_finish_mode(mode);
        true
    })
}

/// Whether a meta component updates its child too (the original's double
/// update, EF9; default true).
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_set_nested_update(fx: *mut fc4_effects, on: bool) {
    with_fx(fx, (), |m, _| m.set_nested_update(on))
}

/// `HasVisualEffect(name)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_has(fx: *const fc4_effects, name: *const c_char) -> bool {
    let Some(n) = c_str(name) else { return false };
    with_fx(fx, false, |m, _| m.has_visual_effect(n))
}

/// `CreateVisualEffect(name)` (§4): writes the new effect's ID (two
/// references: the caller's and the manager's). False for an unknown name;
/// a name with no description under the detail setting creates a no-op
/// (index -1).
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_create_effect(fx: *mut fc4_effects, name: *const c_char, out: *mut u32) -> bool {
    let Some(n) = c_str(name) else { return false };
    let id = with_fx(fx, None, |m, _| m.create(n));
    write_id(id, out)
}

/// Creates the description `index` directly (no priority filter).
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_create_effect_by_index(fx: *mut fc4_effects, index: u32, out: *mut u32) -> bool {
    let id = with_fx(fx, None, |m, _| m.create_by_index(index));
    write_id(id, out)
}

unsafe fn write_id(id: Option<EffectId>, out: *mut u32) -> bool {
    match id {
        Some(id) => {
            if let Some(o) = out.as_mut() {
                *o = id;
            }
            true
        }
        None => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn fc4_effects_add_ref(fx: *mut fc4_effects, id: u32) -> bool {
    with_fx(fx, false, |m, _| m.add_ref(id))
}

/// Drops a reference; the effect is recycled at the next finishing pass
/// once it is done and only the manager's reference is left.
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_release(fx: *mut fc4_effects, id: u32) -> bool {
    with_fx(fx, false, |m, _| m.release(id))
}

/// `Start(mode)` (§6).
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_start(fx: *mut fc4_effects, id: u32, mode: u32) -> bool {
    with_fx(fx, false, |m, h| m.start(id, mode, h))
}

/// `Stop(mode)` (§6): 0 a soft stop (unless the description's hard-stop
/// flag), otherwise a hard stop.
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_stop(fx: *mut fc4_effects, id: u32, mode: u32) -> bool {
    with_fx(fx, false, |m, h| m.stop(id, mode, h))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_effects_is_started(fx: *const fc4_effects, id: u32) -> bool {
    with_fx(fx, false, |m, _| m.is_started(id))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_effects_is_stopping(fx: *const fc4_effects, id: u32) -> bool {
    with_fx(fx, false, |m, _| m.is_stopping(id))
}

/// The effect's world position (metres), passed on to its components and
/// children.
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_set_position(fx: *mut fc4_effects, id: u32, x: f32, y: f32, z: f32) -> bool {
    with_fx(fx, false, |m, _| m.set_position(id, [x, y, z]))
}

/// The message posted when the effect finishes (type 0: none).
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_set_end_message(fx: *mut fc4_effects, id: u32, msg_type: u32, data1: u64) -> bool {
    with_fx(fx, false, |m, _| m.set_end_message(id, msg_type, data1))
}

/// `SetParameter(index, value)`, forwarded to the components and children
/// (index 13: the game records' message data).
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_forward_parameter(fx: *mut fc4_effects, id: u32, index: u32, value: u64) -> bool {
    with_fx(fx, false, |m, _| m.forward_parameter(id, index, value))
}

/// The animation tick (§7): `us` microseconds, scaled by the simulator's
/// dilation; runs while the simulation is paused.
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_tick(fx: *mut fc4_effects, us: u32) {
    with_fx(fx, (), |m, h| m.tick(us, h))
}

/// The finishing pass in the default mode (EF2); does nothing (false) in
/// the compat mode.
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_finish_hook(fx: *mut fc4_effects) -> bool {
    with_fx(fx, false, |m, h| m.finish_hook(h))
}

/// The draw pass: the finishing pass in the compat mode; does nothing
/// (false) in the default mode.
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_draw_pass(fx: *mut fc4_effects) -> bool {
    with_fx(fx, false, |m, h| m.draw_pass(h))
}

/// Stops every effect (hard) and forgets them all.
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_shutdown(fx: *mut fc4_effects) {
    with_fx(fx, (), |m, h| m.shutdown(h))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_effects_get_state(fx: *const fc4_effects, out: *mut fc4_effects_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_fx(fx, false, |m, _| {
        let s = m.stats();
        let (us, secs) = m.total_time();
        *o = fc4_effects_state {
            created: s.created,
            recycled: s.recycled,
            end_posts: s.end_posts,
            select_draws: s.select_draws,
            ticks: s.ticks,
            sweeps: s.sweeps,
            total_us: us,
            total_seconds: secs,
            effect_count: m.effect_count() as u32,
            active_count: m.active_list().len() as u32,
            level_of_detail: m.level_of_detail(),
            max_priority: m.max_priority(),
            finish_mode: match m.finish_mode() {
                FinishMode::Hook => FC4_EFFECTS_FINISH_HOOK,
                FinishMode::DrawPass => FC4_EFFECTS_FINISH_DRAW_PASS,
            },
        };
        true
    })
}

/// An effect's state; false when the ID is unknown (recycled).
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_get_effect(fx: *const fc4_effects, id: u32, out: *mut fc4_effect_info) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_fx(fx, false, |m, _| match m.effect(id) {
        Some(e) => {
            *o = fc4_effect_info {
                index: e.index,
                flags: e.flags,
                refs: e.refs,
                started: e.started,
                stopping: e.stopping,
                active: e.active,
                _pad: 0,
                components: e.components as u32,
                end_type: e.end_message.0,
                end_data1: e.end_message.1,
                position: e.position,
                reserved: 0,
            };
            true
        }
        None => false,
    })
}

/// The effect's name into `buf` (NUL-terminated, truncated to `cap`);
/// returns the name's length (0 for an unknown ID).
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_get_name(fx: *const fc4_effects, id: u32, buf: *mut c_char, cap: usize) -> usize {
    let name = with_fx(fx, None, |m, _| m.effect(id).map(|e| e.name));
    let Some(name) = name else { return 0 };
    if !buf.is_null() && cap > 0 {
        let n = name.len().min(cap - 1);
        std::ptr::copy_nonoverlapping(name.as_ptr() as *const c_char, buf, n);
        *buf.add(n) = 0;
    }
    name.len()
}

/// The child of the effect's `n`-th built meta component.
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_get_child(fx: *const fc4_effects, id: u32, n: u32, out: *mut u32) -> bool {
    let child = with_fx(fx, None, |m, _| {
        match m.component_of_kind(id, component_type::META, n as usize) {
            Some(Component::Meta(c)) => Some(*c),
            _ => None,
        }
    });
    write_id(child, out)
}

/// The effect's `n`-th built brush component.
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_get_brush(
    fx: *const fc4_effects,
    id: u32,
    n: u32,
    out: *mut fc4_effects_brush_state,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_fx(fx, false, |m, _| {
        match m.component_of_kind(id, component_type::BRUSH, n as usize) {
            Some(Component::Brush(b)) => {
                *o = fc4_effects_brush_state {
                    brush_id: b.record.brush_id,
                    rate: b.record.rate,
                    length: b.record.length,
                    strength: b.strength,
                    radius: b.radius,
                    acc: b.acc,
                    t: b.t,
                    prev: [b.prev.0, b.prev.1],
                    cur: [b.cur.0, b.cur.1],
                    one_shot: b.record.one_shot,
                    active: b.active,
                    long_op: b.long_op,
                    _pad: 0,
                    applications: b.applications,
                };
                true
            }
            _ => false,
        }
    })
}

/// The effect's `n`-th built game component.
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_get_game(
    fx: *const fc4_effects,
    id: u32,
    n: u32,
    out: *mut fc4_effects_game_state,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_fx(fx, false, |m, _| {
        match m.component_of_kind(id, component_type::GAME, n as usize) {
            Some(Component::Game(g)) => {
                *o = fc4_effects_game_state {
                    started: g.started,
                    pending: g.pending,
                    map_applied: g.map_applied,
                    _pad: 0,
                    timer: g.timer,
                    count: g.count,
                    applies: g.applies,
                    cell_x: g.cell.0,
                    cell_z: g.cell.1,
                    data: g.data,
                };
                true
            }
            _ => false,
        }
    })
}

/// Whether the demolition filter keeps (skips) the occupant (§9.3 step 3):
/// signposts and text decals always; networks and flora by the flags; with
/// `max_size > 0` a largest extent at least that; with `min_size > 0` a
/// smallest extent at most that.
#[no_mangle]
pub unsafe extern "C" fn fc4_effects_demolish_filter_skips(
    f: *const fc4_effects_demolish_filter,
    o: *const fc4_effects_occupant_facts,
) -> bool {
    guard(false, || match (f.as_ref(), o.as_ref()) {
        (Some(f), Some(o)) => DemolishFilter {
            no_networks: f.no_networks,
            no_flora: f.no_flora,
            min_size: f.min_size,
            max_size: f.max_size,
        }
        .skips(&OccupantFacts {
            signpost: o.signpost,
            text_decal: o.text_decal,
            network: o.network,
            flora: o.flora,
            extents: o.extents,
        }),
        _ => false,
    })
}
