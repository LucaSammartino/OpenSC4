// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The terrain over the C ABI (fc4.h "terrain", docs/specs/terrain.md): the
//! height field and the surface water's depths, the queries, the setters,
//! `RedisplayTerrain` and the other messages, kind-0 brushes, navigability,
//! the bodies of water, the volcano lava and the saved records.
//!
//! Messages: the core queues every message it posts (Â§8.1: `MessagePost`,
//! queued). When the host table has `post`, each entry point hands the
//! queued messages to it, in order, after it released the terrain; without
//! `post` they wait for [`fc4_terrain_take_posts`].
//!
//! Threads: the game reads the terrain from more than one thread, so every
//! entry point holds a re-entrant lock (one thread at a time; the owner may
//! re-enter). A call made from inside a callback reads the terrain normally
//! when the outer call only reads it (`is_navigable`, the bodies of water);
//! a call made while the outer call changes the terrain (a brush's
//! `random_uniform`, the lava's fire callbacks) is refused with the fallback
//! value and counted in `fc4_terrain_state.reentrant_refused`.

use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};

use fc4_core::terrain::{
    shipped_brush, BodiesOfWater, Bridge, BrushBitmap, BrushComponent, BrushDescription, BrushParams, Lava, LavaHost,
    LavaTuning, NavigationHost, Post, RecordError, Terrain, TerrainTuning, VertRect,
};

use crate::flora::ReentrantLock;
use crate::guard;
use crate::props::{fc4_props, props_ref};
use crate::{FC4_ERR_INVALID_ARGUMENT, FC4_ERR_TRUNCATED, FC4_ERR_UNSUPPORTED_VERSION, FC4_OK};

type U = *mut c_void;

#[repr(C)]
pub struct fc4_terrain {
    _private: [u8; 0],
}

/// An inclusive vertex rectangle (fc4.h `fc4_terrain_rect`, Â§8.1).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_terrain_rect {
    pub x0: i32,
    pub z0: i32,
    pub x1: i32,
    pub z1: i32,
}

impl From<VertRect> for fc4_terrain_rect {
    fn from(r: VertRect) -> Self {
        fc4_terrain_rect {
            x0: r.x0,
            z0: r.z0,
            x1: r.x1,
            z1: r.z1,
        }
    }
}

impl From<fc4_terrain_rect> for VertRect {
    fn from(r: fc4_terrain_rect) -> Self {
        VertRect::new(r.x0, r.z0, r.x1, r.z1)
    }
}

/// A queued terrain message (`MessagePost(msg, false)` with four data words).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_terrain_post {
    pub msg_type: u32,
    pub data1: u32,
    pub data2: u32,
    pub data3: u32,
    pub data4: u32,
}

impl From<Post> for fc4_terrain_post {
    fn from(p: Post) -> Self {
        fc4_terrain_post {
            msg_type: p.msg,
            data1: p.data1,
            data2: p.data2,
            data3: p.data3,
            data4: p.data4,
        }
    }
}

/// The "Terrain Properties" tuning (Â§2.4).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_terrain_tuning {
    pub struct_size: u32,
    pub max_height: f32,
    pub min_height: f32,
    pub new_sea_level: f32,
    pub new_flat_height: f32,
    pub image_height_scale: f32,
    pub sea_level_min: f32,
    pub sea_level_max: f32,
    pub sea_level_editor: [f32; 2],
    pub cliff_slope: f32,
    pub saved_78: f32,
}

impl From<TerrainTuning> for fc4_terrain_tuning {
    fn from(t: TerrainTuning) -> Self {
        fc4_terrain_tuning {
            struct_size: std::mem::size_of::<fc4_terrain_tuning>() as u32,
            max_height: t.max_height,
            min_height: t.min_height,
            new_sea_level: t.new_sea_level,
            new_flat_height: t.new_flat_height,
            image_height_scale: t.image_height_scale,
            sea_level_min: t.sea_level_min,
            sea_level_max: t.sea_level_max,
            sea_level_editor: t.sea_level_editor,
            cliff_slope: t.cliff_slope,
            saved_78: t.saved_78,
        }
    }
}

impl From<&fc4_terrain_tuning> for TerrainTuning {
    fn from(t: &fc4_terrain_tuning) -> Self {
        TerrainTuning {
            max_height: t.max_height,
            min_height: t.min_height,
            new_sea_level: t.new_sea_level,
            new_flat_height: t.new_flat_height,
            image_height_scale: t.image_height_scale,
            sea_level_min: t.sea_level_min,
            sea_level_max: t.sea_level_max,
            sea_level_editor: t.sea_level_editor,
            cliff_slope: t.cliff_slope,
            saved_78: t.saved_78,
        }
    }
}

/// The "Lava Properties" tuning the flow uses (Â§11.1).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_lava_tuning {
    pub struct_size: u32,
    pub max_flow_rate: f32,
    pub full_rate_difference: f32,
    pub height_factor: f32,
    pub water_multiplier: f32,
    pub ignite_level: f32,
    pub burn_level: f32,
    pub solidify_threshold: f32,
}

impl From<LavaTuning> for fc4_lava_tuning {
    fn from(t: LavaTuning) -> Self {
        fc4_lava_tuning {
            struct_size: std::mem::size_of::<fc4_lava_tuning>() as u32,
            max_flow_rate: t.max_flow_rate,
            full_rate_difference: t.full_rate_difference,
            height_factor: t.height_factor,
            water_multiplier: t.water_multiplier,
            ignite_level: t.ignite_level,
            burn_level: t.burn_level,
            solidify_threshold: t.solidify_threshold,
        }
    }
}

impl From<&fc4_lava_tuning> for LavaTuning {
    fn from(t: &fc4_lava_tuning) -> Self {
        LavaTuning {
            max_flow_rate: t.max_flow_rate,
            full_rate_difference: t.full_rate_difference,
            height_factor: t.height_factor,
            water_multiplier: t.water_multiplier,
            ignite_level: t.ignite_level,
            burn_level: t.burn_level,
            solidify_threshold: t.solidify_threshold,
        }
    }
}

/// One brush component (Â§9.1). `pixels` (`bitmap_width Ã— bitmap_height`
/// bytes, row-major) is copied; NULL or a zero size = no bitmap.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct fc4_brush_component {
    pub kind: u32,
    pub radius: [f32; 5],
    pub strength: [f32; 5],
    pub oriented: u8,
    pub neutral: u8,
    pub _pad: [u8; 2],
    pub bitmap_width: u32,
    pub bitmap_height: u32,
    pub pixels: *const u8,
}

/// `tBrushParameters` (Â§9.2). `out` is written when `out_valid` is set.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_brush_params {
    pub from_x: f32,
    pub from_z: f32,
    pub to_x: f32,
    pub to_z: f32,
    pub level: i32,
    pub redisplay: u8,
    pub out_valid: u8,
    pub _pad: [u8; 2],
    pub strength_scale: f32,
    pub radius_scale: f32,
    pub data4: u32,
    pub out: fc4_terrain_rect,
    pub target_height: f32,
}

impl From<BrushParams> for fc4_brush_params {
    fn from(p: BrushParams) -> Self {
        fc4_brush_params {
            from_x: p.from_x,
            from_z: p.from_z,
            to_x: p.to_x,
            to_z: p.to_z,
            level: p.level,
            redisplay: p.redisplay.into(),
            out_valid: p.out.is_some().into(),
            _pad: [0; 2],
            strength_scale: p.strength_scale,
            radius_scale: p.radius_scale,
            data4: p.data4,
            out: p.out.map(Into::into).unwrap_or_default(),
            target_height: p.target_height,
        }
    }
}

impl From<&fc4_brush_params> for BrushParams {
    fn from(p: &fc4_brush_params) -> Self {
        BrushParams {
            from_x: p.from_x,
            from_z: p.from_z,
            to_x: p.to_x,
            to_z: p.to_z,
            level: p.level,
            redisplay: p.redisplay != 0,
            strength_scale: p.strength_scale,
            radius_scale: p.radius_scale,
            data4: p.data4,
            out: (p.out_valid != 0).then(|| p.out.into()),
            target_height: p.target_height,
        }
    }
}

/// `bridge_at` results (`IsNavigable` step 4).
pub const FC4_TERRAIN_BRIDGE_NONE: i32 = 0;
pub const FC4_TERRAIN_BRIDGE_UNKNOWN_DECK: i32 = 1;
pub const FC4_TERRAIN_BRIDGE_DECK: i32 = 2;

/// What the terrain needs from the game (fc4.h `fc4_terrain_host`). Missing
/// callbacks read as 0 / false / none.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_terrain_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    /// `MessagePost(msg, false)` (queued).
    pub post: Option<unsafe extern "C" fn(U, *const fc4_terrain_post)>,
    /// `RandomUint32Uniform(n)` (the random-walk brushes, Â§9.3).
    pub random_uniform: Option<unsafe extern "C" fn(U, u32) -> u32>,
    /// The lot at the cell centre (`IsNavigable` step 3), 0 = none.
    pub lot_at_cell: Option<unsafe extern "C" fn(U, i32, i32) -> u64>,
    /// A bridge tile at the cell: `FC4_TERRAIN_BRIDGE_*`; for `_DECK` the deck
    /// height is written to the pointer.
    pub bridge_at: Option<unsafe extern "C" fn(U, i32, i32, *mut f32) -> i32>,
    /// A power pole (type `0x2890D4DE`) whose cell is `(x, z)`.
    pub has_power_pole: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    /// Flammability simulator slot 6.
    pub is_cell_flammable: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    /// Fire interface slot 5: `IgniteCell(x, z, false, true, 0)`.
    pub ignite_cell: Option<unsafe extern "C" fn(U, i32, i32)>,
    /// Fire interface slot 7: `QuickBurnCell(x, z)`.
    pub quick_burn_cell: Option<unsafe extern "C" fn(U, i32, i32)>,
    /// Fire interface slot 9: `SetCause(cause)`.
    pub set_cause: Option<unsafe extern "C" fn(U, u32)>,
    /// The lava's burn-rectangle random iterator: the next cell, or false
    /// when it is exhausted (the host resets it; that draw is lost).
    pub next_burn_cell: Option<unsafe extern "C" fn(U, *mut i32, *mut i32) -> bool>,
}

/// Diagnostics (`fc4_terrain_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_terrain_state {
    pub struct_size: u32,
    pub cells_x: u32,
    pub cells_z: u32,
    pub verts_x: u32,
    pub verts_z: u32,
    pub terrain_type: i32,
    pub default_brush_level: i32,
    pub sea_level: f32,
    pub min_height: f32,
    pub max_height: f32,
    pub saved_78: f32,
    pub view_ready: u8,
    pub water_display_state: u8,
    pub lava_active: u8,
    pub bodies_dirty: u8,
    pub posts_pending: u32,
    pub brush_count: u32,
    pub reentrant_refused: u32,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_terrain_rect>() == 16);
    assert!(std::mem::size_of::<fc4_terrain_post>() == 20);
    assert!(std::mem::size_of::<fc4_terrain_tuning>() == 48);
    assert!(std::mem::size_of::<fc4_lava_tuning>() == 32);
    assert!(std::mem::size_of::<fc4_brush_params>() == 56);
    assert!(std::mem::size_of::<fc4_terrain_state>() == 60);
    assert!(std::mem::size_of::<fc4_brush_component>() == if cfg!(target_pointer_width = "32") { 60 } else { 64 });
    assert!(std::mem::size_of::<fc4_terrain_host>() == if cfg!(target_pointer_width = "32") { 48 } else { 96 });
};

// ---- host adapter -----------------------------------------------------------------------------------

macro_rules! cb {
    ($h:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the terrain promised valid callbacks.
        match $h.$f { Some(f) => unsafe { f($h.user $(, $arg)*) }, None => $default }
    };
}

struct CHost<'a>(&'a fc4_terrain_host);

impl NavigationHost for CHost<'_> {
    fn lot_at_cell(&self, x: i32, z: i32) -> u64 {
        cb!(self.0, lot_at_cell, 0, x, z)
    }
    fn bridge_at(&self, x: i32, z: i32) -> Bridge {
        let mut deck = 0.0f32;
        match cb!(self.0, bridge_at, FC4_TERRAIN_BRIDGE_NONE, x, z, &mut deck) {
            FC4_TERRAIN_BRIDGE_UNKNOWN_DECK => Bridge::UnknownDeck,
            FC4_TERRAIN_BRIDGE_DECK => Bridge::Deck(deck),
            _ => Bridge::None,
        }
    }
    fn has_power_pole(&self, x: i32, z: i32) -> bool {
        cb!(self.0, has_power_pole, false, x, z)
    }
}

impl LavaHost for CHost<'_> {
    fn is_cell_flammable(&mut self, x: i32, z: i32) -> bool {
        cb!(self.0, is_cell_flammable, false, x, z)
    }
    fn ignite_cell(&mut self, x: i32, z: i32) {
        cb!(self.0, ignite_cell, (), x, z)
    }
    fn quick_burn_cell(&mut self, x: i32, z: i32) {
        cb!(self.0, quick_burn_cell, (), x, z)
    }
    fn set_cause(&mut self, cause: u32) {
        cb!(self.0, set_cause, (), cause)
    }
    fn next_burn_cell(&mut self) -> Option<(i32, i32)> {
        let (mut x, mut z) = (0, 0);
        cb!(self.0, next_burn_cell, false, &mut x, &mut z).then_some((x, z))
    }
}

// ---- handles ----------------------------------------------------------------------------------------

struct Inner {
    terrain: Terrain,
    bodies: BodiesOfWater,
    lava: Option<Lava>,
}

struct Handle {
    host: fc4_terrain_host,
    lock: ReentrantLock,
    /// Only touched by the thread that holds `lock`.
    inner: RefCell<Inner>,
    refused: AtomicU32,
}

fn with_h<R>(t: *const fc4_terrain, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_terrain*.
    match unsafe { (t as *const Handle).as_ref() } {
        Some(h) => guard(fallback, || f(h)),
        None => fallback,
    }
}

/// Reads the terrain under the lock.
fn read<R: Copy>(t: *const fc4_terrain, fallback: R, f: impl FnOnce(&Inner, &fc4_terrain_host) -> R) -> R {
    with_h(t, fallback, |h| {
        let _g = h.lock.lock();
        match h.inner.try_borrow() {
            Ok(i) => f(&i, &h.host),
            Err(_) => {
                h.refused.fetch_add(1, Ordering::Relaxed);
                fallback
            }
        }
    })
}

/// Changes the terrain under the lock, then hands the queued messages to the
/// host's `post` (when it has one) after the lock is released.
fn write<R: Copy>(t: *mut fc4_terrain, fallback: R, f: impl FnOnce(&mut Inner, &fc4_terrain_host) -> R) -> R {
    with_h(t, fallback, |h| {
        let (r, posts) = {
            let _g = h.lock.lock();
            let Ok(mut i) = h.inner.try_borrow_mut() else {
                h.refused.fetch_add(1, Ordering::Relaxed);
                return fallback;
            };
            let r = f(&mut i, &h.host);
            let posts = if h.host.post.is_some() {
                i.terrain.take_posts()
            } else {
                Vec::new()
            };
            (r, posts)
        };
        for p in posts {
            let c = fc4_terrain_post::from(p);
            cb!(h.host, post, (), &c);
        }
        r
    })
}

unsafe fn put<T>(p: *mut T, v: T) {
    if let Some(r) = p.as_mut() {
        *r = v;
    }
}

unsafe fn bytes<'a>(p: *const u8, n: usize) -> &'a [u8] {
    if p.is_null() || n == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(p, n)
    }
}

/// Copies `src` to `out` when it fits `capacity`; returns `src.len()`.
unsafe fn copy_out<T: Copy>(src: &[T], out: *mut T, capacity: usize) -> usize {
    if !out.is_null() && capacity >= src.len() {
        std::ptr::copy_nonoverlapping(src.as_ptr(), out, src.len());
    }
    src.len()
}

unsafe fn host_ok(host: *const fc4_terrain_host) -> Option<fc4_terrain_host> {
    host.as_ref()
        .filter(|h| (h.struct_size as usize) >= std::mem::size_of::<fc4_terrain_host>())
        .copied()
}

unsafe fn tuning_or_shipped(tuning: *const fc4_terrain_tuning) -> TerrainTuning {
    match tuning.as_ref() {
        Some(t) if (t.struct_size as usize) >= std::mem::size_of::<fc4_terrain_tuning>() => t.into(),
        _ => TerrainTuning::SHIPPED,
    }
}

fn boxed(host: fc4_terrain_host, terrain: Terrain) -> *mut fc4_terrain {
    let bodies = BodiesOfWater::new(terrain.cells_x(), terrain.cells_z());
    Box::into_raw(Box::new(Handle {
        host,
        lock: ReentrantLock::default(),
        inner: RefCell::new(Inner {
            terrain,
            bodies,
            lava: None,
        }),
        refused: AtomicU32::new(0),
    })) as *mut fc4_terrain
}

// ---- tuning -----------------------------------------------------------------------------------------

/// The shipped "Terrain Properties" values (Â§2.4).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_tuning_shipped(out: *mut fc4_terrain_tuning) -> bool {
    guard(false, || match out.as_mut() {
        Some(o) => {
            *o = TerrainTuning::SHIPPED.into();
            true
        }
        None => false,
    })
}

/// Decodes the "Terrain Properties" exemplar (NULL = none: the code defaults
/// 200 / 5 for the height clamps, Â§2.4).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_tuning_from_props(props: *const fc4_props, out: *mut fc4_terrain_tuning) -> bool {
    guard(false, || match out.as_mut() {
        Some(o) => {
            *o = TerrainTuning::from_holder(props_ref(props)).into();
            true
        }
        None => false,
    })
}

/// The shipped "Lava Properties" values (Â§11.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_lava_tuning_shipped(out: *mut fc4_lava_tuning) -> bool {
    guard(false, || match out.as_mut() {
        Some(o) => {
            *o = LavaTuning::SHIPPED.into();
            true
        }
        None => false,
    })
}

// ---- life cycle -------------------------------------------------------------------------------------

/// `Create` (Â§4.6, Â§10.1): a flat terrain of `cells_x Ã— cells_z` cells at the
/// tuning's flat height, the sea at its sea level. NULL `tuning` = shipped.
/// NULL for a bad host table or a size of 0 (or above 4096) cells.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_create(
    host: *const fc4_terrain_host,
    tuning: *const fc4_terrain_tuning,
    cells_x: u32,
    cells_z: u32,
) -> *mut fc4_terrain {
    guard(std::ptr::null_mut(), || {
        let Some(h) = host_ok(host) else {
            return std::ptr::null_mut();
        };
        match Terrain::create(cells_x, cells_z, tuning_or_shipped(tuning)) {
            Some(t) => boxed(h, t),
            None => std::ptr::null_mut(),
        }
    })
}

/// A terrain from `altitudes` (`(cells_x + 1) Ã— (cells_z + 1)` floats, index
/// order), the sea at `sea_level`, the water reset to it (tools and tests).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_create_from_altitudes(
    host: *const fc4_terrain_host,
    tuning: *const fc4_terrain_tuning,
    cells_x: u32,
    cells_z: u32,
    altitudes: *const f32,
    count: usize,
    sea_level: f32,
) -> *mut fc4_terrain {
    guard(std::ptr::null_mut(), || {
        let Some(h) = host_ok(host) else {
            return std::ptr::null_mut();
        };
        if altitudes.is_null() {
            return std::ptr::null_mut();
        }
        let a = std::slice::from_raw_parts(altitudes, count);
        match Terrain::from_altitudes(cells_x, cells_z, a, sea_level, tuning_or_shipped(tuning)) {
            Some(t) => boxed(h, t),
            None => std::ptr::null_mut(),
        }
    })
}

/// `Load` (slot 96, Â§13.1): the misc, altitude and vertex property records.
/// `*error` (may be NULL) := `FC4_OK`, `FC4_ERR_TRUNCATED`,
/// `FC4_ERR_UNSUPPORTED_VERSION` or `FC4_ERR_INVALID_ARGUMENT` (bad sizes or
/// host table). The water is rebuilt from the tuning's sea level (Â§13.3).
#[allow(clippy::too_many_arguments)]
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_load(
    host: *const fc4_terrain_host,
    tuning: *const fc4_terrain_tuning,
    misc: *const u8,
    misc_len: usize,
    altitudes: *const u8,
    altitudes_len: usize,
    properties: *const u8,
    properties_len: usize,
    error: *mut i32,
) -> *mut fc4_terrain {
    guard(std::ptr::null_mut(), || {
        let Some(h) = host_ok(host) else {
            put(error, FC4_ERR_INVALID_ARGUMENT);
            return std::ptr::null_mut();
        };
        match Terrain::load(
            bytes(misc, misc_len),
            bytes(altitudes, altitudes_len),
            bytes(properties, properties_len),
            tuning_or_shipped(tuning),
        ) {
            Ok(t) => {
                put(error, FC4_OK);
                boxed(h, t)
            }
            Err(e) => {
                put(
                    error,
                    match e {
                        RecordError::Truncated => FC4_ERR_TRUNCATED,
                        RecordError::Version(_) => FC4_ERR_UNSUPPORTED_VERSION,
                        RecordError::Size => FC4_ERR_INVALID_ARGUMENT,
                    },
                );
                std::ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_destroy(t: *mut fc4_terrain) {
    if !t.is_null() {
        guard((), || drop(Box::from_raw(t as *mut Handle)));
    }
}

/// `DoPostCityInit` (slot 40): sets the view-ready flag; `RedisplayTerrain`
/// posts nothing before it (Â§8.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_post_city_init(t: *mut fc4_terrain) -> bool {
    write(t, false, |i, _| {
        i.terrain.post_city_init();
        true
    })
}

/// Sets or clears the view-ready flag directly (shutdown, tests).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_set_view_ready(t: *mut fc4_terrain, ready: bool) -> bool {
    write(t, false, |i, _| {
        i.terrain.set_view_ready(ready);
        true
    })
}

/// The queued messages not yet handed to the host, oldest first: copies up
/// to `capacity` and removes them; returns how many were copied.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_take_posts(
    t: *mut fc4_terrain,
    out: *mut fc4_terrain_post,
    capacity: usize,
) -> usize {
    if out.is_null() {
        return 0;
    }
    with_h(t, 0, |h| {
        let _g = h.lock.lock();
        let Ok(mut i) = h.inner.try_borrow_mut() else {
            h.refused.fetch_add(1, Ordering::Relaxed);
            return 0;
        };
        let taken = i.terrain.take_posts_upto(capacity);
        for (k, p) in taken.iter().enumerate() {
            *out.add(k) = (*p).into();
        }
        taken.len()
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_get_state(t: *const fc4_terrain, out: *mut fc4_terrain_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    let refused = with_h(t, 0, |h| h.refused.load(Ordering::Relaxed));
    read(t, false, |i, _| {
        let tr = &i.terrain;
        *o = fc4_terrain_state {
            struct_size: std::mem::size_of::<fc4_terrain_state>() as u32,
            cells_x: tr.cells_x(),
            cells_z: tr.cells_z(),
            verts_x: tr.verts_x(),
            verts_z: tr.verts_z(),
            terrain_type: tr.terrain_type(),
            default_brush_level: tr.default_brush_level(),
            sea_level: tr.sea_level(),
            min_height: tr.min_height(),
            max_height: tr.max_height(),
            saved_78: tr.saved_78(),
            view_ready: tr.view_ready().into(),
            water_display_state: tr.water_display_state(),
            lava_active: i.lava.is_some().into(),
            bodies_dirty: i.bodies.dirty().into(),
            posts_pending: tr.pending_posts() as u32,
            brush_count: tr.brush_count() as u32,
            reentrant_refused: refused,
        };
        true
    })
}

// ---- grid and height queries (Â§2-Â§4) ------------------------------------------------------------------

/// `GetCellCountX/Z` (slots 7, 8; axis 0 / 1).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_cell_count(t: *const fc4_terrain, axis: u32) -> u32 {
    read(t, 0, |i, _| {
        if axis == 0 {
            i.terrain.cells_x()
        } else {
            i.terrain.cells_z()
        }
    })
}

/// `GetVertIndex` (slot 9, no bounds check).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_vert_index(t: *const fc4_terrain, x: i32, z: i32) -> u32 {
    read(t, 0, |i, _| i.terrain.vert_index(x, z))
}

/// `GetVertex` (slot 10).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_vertex_of(t: *const fc4_terrain, index: u32, x: *mut u32, z: *mut u32) -> bool {
    read(t, false, |i, _| {
        let (a, b) = i.terrain.vertex_of(index);
        put(x, a);
        put(z, b);
        true
    })
}

/// `LocationIsInBounds` (slot 11).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_location_is_in_bounds(t: *const fc4_terrain, x: f32, z: f32) -> bool {
    read(t, false, |i, _| i.terrain.location_is_in_bounds(x, z))
}

/// `GetAltitude(x, z)` (slot 13, Â§4.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_altitude(t: *const fc4_terrain, x: f32, z: f32) -> f32 {
    read(t, 0.0, |i, _| i.terrain.altitude(x, z))
}

/// `GetAltitude(index)` (slot 12).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_altitude_at_index(t: *const fc4_terrain, index: u32) -> f32 {
    read(t, 0.0, |i, _| i.terrain.altitude_at_index(index))
}

/// `GetAltitudeAtVertex(x, z)` (slot 14).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_altitude_at_vertex(t: *const fc4_terrain, x: i32, z: i32) -> f32 {
    read(t, 0.0, |i, _| i.terrain.altitude_at_vertex(x, z))
}

/// `GetAltitudes(rect)` (slot 15): the altitudes of the inclusive rectangle
/// (clipped), z outer, x inner. Copies when `capacity` suffices; returns the
/// count.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_altitudes_in(
    t: *const fc4_terrain,
    rect: fc4_terrain_rect,
    out: *mut f32,
    capacity: usize,
) -> usize {
    read(t, 0, |i, _| {
        copy_out(&i.terrain.altitudes_in(rect.into()), out, capacity)
    })
}

/// The whole altitude array (index order). Copies when `capacity` suffices;
/// returns the vertex count.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_altitudes(t: *const fc4_terrain, out: *mut f32, capacity: usize) -> usize {
    read(t, 0, |i, _| copy_out(i.terrain.altitudes(), out, capacity))
}

/// `GetCellCornerAltitudes` (slot 17): `out[4]` in the order `(x, z)`,
/// `(x, z + 1)`, `(x + 1, z + 1)`, `(x + 1, z)`; untouched (false) out of
/// bounds.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_cell_corner_altitudes(
    t: *const fc4_terrain,
    x: i32,
    z: i32,
    out: *mut f32,
) -> bool {
    if out.is_null() {
        return false;
    }
    read(t, false, |i, _| match i.terrain.cell_corner_altitudes(x, z) {
        Some(a) => {
            std::ptr::copy_nonoverlapping(a.as_ptr(), out, 4);
            true
        }
        None => false,
    })
}

/// `GetAltitudeAtNearestGrid` (slot 19).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_altitude_at_nearest_grid(t: *const fc4_terrain, x: f32, z: f32) -> f32 {
    read(t, 0.0, |i, _| i.terrain.altitude_at_nearest_grid(x, z))
}

/// `GetAltitudeCheckSum` (slot 20).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_altitude_checksum(t: *const fc4_terrain, rect: fc4_terrain_rect) -> f32 {
    read(t, 0.0, |i, _| i.terrain.altitude_checksum(rect.into()))
}

/// `MinHeight` / `MaxHeight` (slots 21, 22): the cached, possibly stale values
/// (Â§4.6). `which` 0 = minimum, 1 = maximum.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_cached_height(t: *const fc4_terrain, which: u32) -> f32 {
    read(t, 0.0, |i, _| {
        if which == 0 {
            i.terrain.min_height()
        } else {
            i.terrain.max_height()
        }
    })
}

/// `MinAllowedHeight` / `MaxAllowedHeight` (slots 23, 24); `which` 0 / 1.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_allowed_height(t: *const fc4_terrain, which: u32) -> f32 {
    read(t, 0.0, |i, _| {
        if which == 0 {
            i.terrain.min_allowed_height()
        } else {
            i.terrain.max_allowed_height()
        }
    })
}

/// Sets the cached minimum and maximum (load from another source, tests).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_set_cached_heights(t: *mut fc4_terrain, min: f32, max: f32) -> bool {
    write(t, false, |i, _| {
        i.terrain.set_min_max_cache(min, max);
        true
    })
}

/// `GetFaceNormal(x, z)` (slot 26, Â§4.3): `out[3]`, untouched (false) unless
/// `0 â‰¤ x < sizeX` and `0 â‰¤ z < sizeZ`.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_face_normal(t: *const fc4_terrain, x: f32, z: f32, out: *mut f32) -> bool {
    if out.is_null() {
        return false;
    }
    read(t, false, |i, _| match i.terrain.face_normal(x, z) {
        Some(n) => {
            std::ptr::copy_nonoverlapping(n.as_ptr(), out, 3);
            true
        }
        None => false,
    })
}

// ---- water queries (Â§5, Â§10.2) ------------------------------------------------------------------------

/// `GetSeaLevel` (slot 29).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_sea_level(t: *const fc4_terrain) -> f32 {
    read(t, 0.0, |i, _| i.terrain.sea_level())
}

/// `GetWaterLevel(x, z)` (slot 30).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_water_level(t: *const fc4_terrain, x: f32, z: f32) -> f32 {
    read(t, 0.0, |i, _| i.terrain.water_level(x, z))
}

/// `GetWaterLevel(index)` (slot 31).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_water_level_at_index(t: *const fc4_terrain, index: u32) -> f32 {
    read(t, 0.0, |i, _| i.terrain.water_level_at_index(index))
}

/// `GetWaterDepthAtVertex(x, z)` (slot 33): may be negative.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_water_depth_at_vertex(t: *const fc4_terrain, x: i32, z: i32) -> f32 {
    read(t, 0.0, |i, _| i.terrain.water_depth_at_vertex(x, z))
}

/// `GetWaterDepthAtVertex(index)` (slot 92; surface water slot 5).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_water_depth_at_index(t: *const fc4_terrain, index: u32) -> f32 {
    read(t, 0.0, |i, _| i.terrain.water_depth_at_index(index))
}

/// `GetMaxSurfaceLevel(x, z)` (slot 18).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_max_surface_level(t: *const fc4_terrain, x: i32, z: i32) -> f32 {
    read(t, 0.0, |i, _| i.terrain.max_surface_level(x, z))
}

/// `CellHasWater` (slot 34).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_cell_has_water(t: *const fc4_terrain, x: i32, z: i32) -> bool {
    read(t, false, |i, _| i.terrain.cell_has_water(x, z))
}

/// `CellHasLand` (slot 35).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_cell_has_land(t: *const fc4_terrain, x: i32, z: i32) -> bool {
    read(t, false, |i, _| i.terrain.cell_has_land(x, z))
}

/// `IsLandVertex` (slot 36).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_is_land_vertex(t: *const fc4_terrain, x: i32, z: i32) -> bool {
    read(t, false, |i, _| i.terrain.is_land_vertex(x, z))
}

/// `IsWaterVertex` (slot 37).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_is_water_vertex(t: *const fc4_terrain, x: i32, z: i32) -> bool {
    read(t, false, |i, _| i.terrain.is_water_vertex(x, z))
}

/// `IsSeaVertex` (slot 38).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_is_sea_vertex(t: *const fc4_terrain, x: i32, z: i32) -> bool {
    read(t, false, |i, _| i.terrain.is_sea_vertex(x, z))
}

/// `NumberOfWaterVertices(rect)` (slot 53).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_number_of_water_vertices(t: *const fc4_terrain, rect: fc4_terrain_rect) -> u32 {
    read(t, 0, |i, _| i.terrain.number_of_water_vertices(rect.into()))
}

/// `GetSedimentValue(x, z)` (slot 55).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_sediment(t: *const fc4_terrain, x: i32, z: i32) -> f32 {
    read(t, 0.0, |i, _| i.terrain.sediment(x, z))
}

/// Sets a vertex's water depth directly (the surface water's animation, the
/// god-mode water brushes, tests). False out of bounds.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_set_water_depth(t: *mut fc4_terrain, x: i32, z: i32, depth: f32) -> bool {
    write(t, false, |i, _| i.terrain.set_water_depth(x, z, depth))
}

/// `ResetWaterToSeaLevel` (Â§10.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_reset_water_to_sea_level(t: *mut fc4_terrain) -> bool {
    write(t, false, |i, _| {
        i.terrain.reset_water_to_sea_level();
        true
    })
}

/// Surface water slot 6, `GetWaterDepthForCell(x, z)` (Â§10.2).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_water_depth_for_cell(t: *const fc4_terrain, x: i32, z: i32) -> f32 {
    read(t, 0.0, |i, _| i.terrain.water_depth_for_cell(x, z))
}

/// Surface water slot 13, `HasDraftClearance(x, z)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_has_draft_clearance(t: *const fc4_terrain, x: i32, z: i32) -> bool {
    read(t, false, |i, _| i.terrain.has_draft_clearance(x, z))
}

/// Surface water slot 12, `GetWaterType(x, z)`: 0 land, 2 fresh, 3 sea.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_water_type(t: *const fc4_terrain, x: i32, z: i32) -> u32 {
    read(t, 0, |i, _| i.terrain.water_type(x, z))
}

/// `GetLerpedWaterLevel(x, z)` (slot 32, Â§5.1, ABI 69).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_lerped_water_level(t: *const fc4_terrain, x: f32, z: f32) -> f32 {
    read(t, 0.0, |i, _| i.terrain.lerped_water_level(x, z))
}

/// Surface water slot 10, `AltitudeAtWhichFloodingWillOccur(x, z)` (Â§10.2,
/// ABI 69).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_flooding_altitude(t: *const fc4_terrain, x: i32, z: i32) -> f32 {
    read(t, 0.0, |i, _| i.terrain.altitude_at_which_flooding_will_occur(x, z))
}

/// Surface water slot 11, `GetWaterVelocity(x, z)`: `out[3]` (a zero vector
/// past the arrays, Â§10.5; ABI 69). False only for a NULL handle or `out`.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_water_velocity(t: *const fc4_terrain, x: i32, z: i32, out: *mut f32) -> bool {
    if out.is_null() {
        return false;
    }
    read(t, false, |i, _| {
        let v = i.terrain.water_velocity(x, z);
        std::ptr::copy_nonoverlapping(v.as_ptr(), out, 3);
        true
    })
}

/// `GetSedimentValue(x, z)` (slot 55): world metres, 0.0 unless
/// `LocationIsInBounds` (Â§5; ABI 69).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_sediment_at(t: *const fc4_terrain, x: f32, z: f32) -> f32 {
    read(t, 0.0, |i, _| i.terrain.sediment_at(x, z))
}

/// Surface water slot 8, `IsNavigable(x, z, r, lot)` (Â§10.4), through the
/// host's `lot_at_cell`, `bridge_at` and `has_power_pole`.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_is_navigable(t: *const fc4_terrain, x: i32, z: i32, r: i32, lot: u64) -> bool {
    read(t, false, |i, h| i.terrain.is_navigable(x, z, r, lot, &CHost(h)))
}

// ---- setters (Â§6) -----------------------------------------------------------------------------------

/// `RaiseVertex(i, d)` (Â§6).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_raise_vertex(t: *mut fc4_terrain, index: u32, d: f32) -> bool {
    write(t, false, |i, _| {
        i.terrain.raise_vertex(index, d);
        true
    })
}

/// `LowerVertex(i, d)` (Â§6).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_lower_vertex(t: *mut fc4_terrain, index: u32, d: f32) -> bool {
    write(t, false, |i, _| {
        i.terrain.lower_vertex(index, d);
        true
    })
}

/// `SetAltitudeAtVertex(i, h)` (slot 50).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_set_altitude_at_vertex(t: *mut fc4_terrain, index: u32, h: f32) -> bool {
    write(t, false, |i, _| {
        i.terrain.set_altitude_at_vertex(index, h);
        true
    })
}

/// `SetAltitude(x, z, h)` (slot 51).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_set_altitude(t: *mut fc4_terrain, x: i32, z: i32, h: f32) -> bool {
    write(t, false, |i, _| {
        i.terrain.set_altitude(x, z, h);
        true
    })
}

/// `SetAltitude(rect, h, redisplay, data4)` (slot 49).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_set_altitude_rect(
    t: *mut fc4_terrain,
    rect: fc4_terrain_rect,
    h: f32,
    redisplay: bool,
    data4: u32,
) -> bool {
    write(t, false, |i, _| {
        i.terrain.set_altitude_rect(rect.into(), h, redisplay, data4);
        true
    })
}

/// `LockAltitude(rect, lock)` (slot 52).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_lock_altitude(t: *mut fc4_terrain, rect: fc4_terrain_rect, lock: bool) -> bool {
    write(t, false, |i, _| {
        i.terrain.lock_altitude(rect.into(), lock);
        true
    })
}

/// `SetSeaLevel(h)` (slot 45): false when rejected (out of the tuning range).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_set_sea_level(t: *mut fc4_terrain, h: f32) -> bool {
    write(t, false, |i, _| i.terrain.set_sea_level(h))
}

/// Sets the sea level with no range check, message or water reset (a load
/// that knows the city's sea level).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_set_sea_level_raw(t: *mut fc4_terrain, h: f32) -> bool {
    write(t, false, |i, _| {
        i.terrain.set_sea_level_raw(h);
        true
    })
}

/// `GetTerrainType` (slot 42).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_terrain_type(t: *const fc4_terrain) -> i32 {
    read(t, 0, |i, _| i.terrain.terrain_type())
}

/// `SetTerrainType(t)` (slot 41).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_set_terrain_type(t: *mut fc4_terrain, ty: i32) -> bool {
    write(t, false, |i, _| {
        i.terrain.set_terrain_type(ty);
        true
    })
}

// ---- flags (Â§7) -------------------------------------------------------------------------------------

/// A vertex's flag word (0 out of bounds).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_vertex_flags(t: *const fc4_terrain, x: i32, z: i32) -> u16 {
    read(t, 0, |i, _| i.terrain.vertex_flags(x, z))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_set_vertex_flags(t: *mut fc4_terrain, x: i32, z: i32, flags: u16) -> bool {
    write(t, false, |i, _| i.terrain.set_vertex_flags(x, z, flags))
}

/// `IsTriangulationFlipped(x, z)` (slot 70).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_is_triangulation_flipped(t: *const fc4_terrain, x: i32, z: i32) -> bool {
    read(t, false, |i, _| i.terrain.is_triangulation_flipped(x, z))
}

/// `GetTerrainCellFlags` (slot 82).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_cell_flags(t: *const fc4_terrain, x: i32, z: i32) -> u32 {
    read(t, 0, |i, _| i.terrain.cell_flags(x, z))
}

/// `SetTerrainCellFlags` (slot 81).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_set_cell_flags(t: *mut fc4_terrain, x: i32, z: i32, flags: u32) -> bool {
    write(t, false, |i, _| {
        i.terrain.set_cell_flags(x, z, flags);
        true
    })
}

// ---- RedisplayTerrain and messages (Â§8) ---------------------------------------------------------------

/// `RedisplayTerrain(terrainFlag, waterFlag, rect, data4)` (slot 68).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_redisplay(
    t: *mut fc4_terrain,
    terrain: bool,
    water: bool,
    rect: fc4_terrain_rect,
    data4: u32,
) -> bool {
    write(t, false, |i, _| {
        i.terrain.redisplay_terrain(terrain, water, rect.into(), data4);
        true
    })
}

/// Slot 69: the cell corners `(ax, az)` and `(bx, bz)` â†’ the vertex rectangle.
#[allow(clippy::too_many_arguments)]
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_redisplay_cells(
    t: *mut fc4_terrain,
    terrain: bool,
    water: bool,
    ax: i32,
    az: i32,
    bx: i32,
    bz: i32,
    data4: u32,
) -> bool {
    write(t, false, |i, _| {
        i.terrain
            .redisplay_terrain_cells(terrain, water, (ax, az), (bx, bz), data4);
        true
    })
}

/// `BeginLongTerrainOperation` (slot 87).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_begin_long_operation(t: *mut fc4_terrain) -> bool {
    write(t, false, |i, _| {
        i.terrain.begin_long_terrain_operation();
        true
    })
}

/// `EndLongTerrainOperation` (slot 88).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_end_long_operation(t: *mut fc4_terrain) -> bool {
    write(t, false, |i, _| {
        i.terrain.end_long_terrain_operation();
        true
    })
}

/// `GetWaterSurfaceDisplayState` (slot 83).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_water_display_state(t: *const fc4_terrain) -> u8 {
    read(t, 0, |i, _| i.terrain.water_display_state())
}

/// `SetWaterSurfaceDisplayState` (slot 84): posts `0x6AB97795` only when the
/// state changes (Â§8.4, TR14). True unless the handle is bad.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_set_water_display_state(t: *mut fc4_terrain, state: u8) -> bool {
    write(t, false, |i, _| {
        i.terrain.set_water_display_state(state);
        true
    })
}

// ---- brushes (Â§9) -----------------------------------------------------------------------------------

unsafe fn bitmap_of(width: u32, height: u32, pixels: *const u8) -> Result<Option<BrushBitmap>, ()> {
    if pixels.is_null() || width == 0 || height == 0 {
        return Ok(None);
    }
    let n = (width as usize).checked_mul(height as usize).ok_or(())?;
    let px = std::slice::from_raw_parts(pixels, n).to_vec();
    BrushBitmap::new(width, height, px).map(Some).ok_or(())
}

/// Adds or replaces a brush description (`ReadBrushDescriptions`, Â§9.1):
/// `count` components (at most 4, kinds 0-11), the random-walk step
/// (radians; not 0 sets the walk flag) and `angle`, the walk angle's start
/// (the host's `RandomRange(0, 2Ï€)` draw); the stroke angle starts at 0
/// (TR8). False when rejected.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_add_brush(
    t: *mut fc4_terrain,
    id: u32,
    components: *const fc4_brush_component,
    count: usize,
    random_walk_step: f32,
    angle: f32,
) -> bool {
    if components.is_null() && count != 0 {
        return false;
    }
    let cs = if count == 0 {
        &[][..]
    } else {
        std::slice::from_raw_parts(components, count)
    };
    let mut comps = Vec::with_capacity(cs.len());
    for c in cs {
        let Ok(bitmap) = bitmap_of(c.bitmap_width, c.bitmap_height, c.pixels) else {
            return false;
        };
        comps.push(BrushComponent {
            kind: c.kind,
            radius: c.radius,
            strength: c.strength,
            oriented: c.oriented != 0,
            neutral: c.neutral,
            bitmap,
        });
    }
    let desc = BrushDescription {
        components: comps,
        random_walk_step,
        walk_angle: if random_walk_step != 0.0 { angle } else { 0.0 },
        stroke_angle: 0.0,
    };
    write(t, false, move |i, _| i.terrain.add_brush(id, desc))
}

/// Adds one of the shipped brushes the simulation applies (Â§9.1: `0x11`,
/// `0x12`, `0x14`) with the bitmap the host loaded. False for another ID or
/// a bad bitmap.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_add_shipped_brush(
    t: *mut fc4_terrain,
    id: u32,
    bitmap_width: u32,
    bitmap_height: u32,
    pixels: *const u8,
) -> bool {
    let Ok(bitmap) = bitmap_of(bitmap_width, bitmap_height, pixels) else {
        return false;
    };
    let Some(desc) = shipped_brush(id, bitmap) else {
        return false;
    };
    write(t, false, move |i, _| i.terrain.add_brush(id, desc))
}

/// The stroke angle of a brush description (`+0x6C`, the angle of its
/// oriented components, Â§9.3 step 2.2).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_brush_angle(t: *const fc4_terrain, id: u32, out: *mut f32) -> bool {
    read(t, false, |i, _| match i.terrain.brush(id) {
        Some(d) => {
            put(out, d.stroke_angle);
            true
        }
        None => false,
    })
}

/// Both angles of a brush description: the walk angle (`+0x68`) and the
/// stroke angle (`+0x6C`) (Â§9.1, TR8; ABI 69). Either pointer may be NULL.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_brush_angles(
    t: *const fc4_terrain,
    id: u32,
    walk: *mut f32,
    stroke: *mut f32,
) -> bool {
    read(t, false, |i, _| match i.terrain.brush(id) {
        Some(d) => {
            put(walk, d.walk_angle);
            put(stroke, d.stroke_angle);
            true
        }
        None => false,
    })
}

/// Sets both angles of a brush description (ABI 69): the walk angle's start
/// draw (`RandomRange(0, 2Ï€)` of the terrain generator, which the original
/// makes when it reads a description whose step is not 0), or angles carried
/// over from another terrain (the original's descriptions are process-wide).
/// False for an unknown brush.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_set_brush_angles(t: *mut fc4_terrain, id: u32, walk: f32, stroke: f32) -> bool {
    write(t, false, |i, _| i.terrain.set_brush_angles(id, walk, stroke))
}

/// The talus angle of brush kind 2, radians (the terrain ini's `WindErosion`
/// `TalusAngle` Ã— Ï€/180; 0.79 by default, Â§9.4.1; ABI 69).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_talus_angle(t: *const fc4_terrain) -> f32 {
    read(t, 0.0, |i, _| i.terrain.talus_angle())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_set_talus_angle(t: *mut fc4_terrain, radians: f32) -> bool {
    write(t, false, |i, _| {
        i.terrain.set_talus_angle(radians);
        true
    })
}

/// `ApplyBrush(brushID, params)` (slot 57, Â§9.2): false for an unknown brush.
/// `params.out` / `out_valid` receive the touched vertex rectangle.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_apply_brush(t: *mut fc4_terrain, id: u32, params: *mut fc4_brush_params) -> bool {
    let Some(p) = params.as_mut() else { return false };
    write(t, false, |i, h| {
        let mut bp = BrushParams::from(&*p);
        let ok = i
            .terrain
            .apply_brush(id, &mut bp, &mut |n| cb!(h, random_uniform, 0, n));
        *p = bp.into();
        ok
    })
}

/// The default brush level (`+0xBC`, used for a negative `params.level`).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_default_brush_level(t: *const fc4_terrain) -> i32 {
    read(t, 0, |i, _| i.terrain.default_brush_level())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_set_default_brush_level(t: *mut fc4_terrain, level: i32) -> bool {
    write(t, false, |i, _| {
        i.terrain.set_default_brush_level(level);
        true
    })
}

// ---- bodies of water (Â§10.3) ------------------------------------------------------------------------

/// `UpdateBodiesOfWater` through the host's navigability callbacks.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_update_bodies_of_water(t: *mut fc4_terrain) -> bool {
    write(t, false, |i, h| {
        let Inner { terrain, bodies, .. } = i;
        bodies.update(terrain, &CHost(h));
        true
    })
}

/// SimNewDay: rebuilds the map when the dirty flag is set; true if it did.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_bodies_on_new_day(t: *mut fc4_terrain) -> bool {
    write(t, false, |i, h| {
        let Inner { terrain, bodies, .. } = i;
        bodies.on_new_day(terrain, &CHost(h))
    })
}

/// A power pole inserted or removed at the cell (`0x99EF1142` /
/// `0x99EF1143`): sets the dirty flag on a water cell.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_bodies_note_power_pole(t: *mut fc4_terrain, x: i32, z: i32) -> bool {
    write(t, false, |i, _| {
        let Inner { terrain, bodies, .. } = i;
        bodies.note_power_pole_changed(terrain, x, z);
        true
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_bodies_dirty(t: *const fc4_terrain) -> bool {
    read(t, false, |i, _| i.bodies.dirty())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_set_bodies_dirty(t: *mut fc4_terrain, dirty: bool) -> bool {
    write(t, false, |i, _| {
        i.bodies.set_dirty(dirty);
        true
    })
}

/// The body-of-water ID of a cell (0 = none).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_body_of_water(t: *const fc4_terrain, x: i32, z: i32) -> u16 {
    read(t, 0, |i, _| i.bodies.get(x, z))
}

/// The body-of-water map (`x + z Ã— cellsX`; surface water slot 9). Copies
/// when `capacity` suffices; returns the cell count.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_bodies_map(t: *const fc4_terrain, out: *mut u16, capacity: usize) -> usize {
    read(t, 0, |i, _| copy_out(i.bodies.map(), out, capacity))
}

// ---- lava (Â§11) -------------------------------------------------------------------------------------

/// Starts the volcano lava at `(x, z)` metres with `radius` metres (Â§11.2),
/// replacing any lava. NULL `tuning` = shipped.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_lava_start(
    t: *mut fc4_terrain,
    x: f32,
    z: f32,
    radius: f32,
    tuning: *const fc4_lava_tuning,
) -> bool {
    let lt = match tuning.as_ref() {
        Some(l) if (l.struct_size as usize) >= std::mem::size_of::<fc4_lava_tuning>() => l.into(),
        _ => LavaTuning::SHIPPED,
    };
    write(t, false, |i, _| {
        i.lava = Some(Lava::start(&i.terrain, x, z, radius, lt));
        true
    })
}

/// Ends the lava (not saved, Â§11.2).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_lava_stop(t: *mut fc4_terrain) -> bool {
    write(t, false, |i, _| i.lava.take().is_some())
}

/// The volcano's per-tick production and solidification rates.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_lava_set_rates(t: *mut fc4_terrain, production: f32, solidification: f32) -> bool {
    write(t, false, |i, _| match i.lava.as_mut() {
        Some(l) => {
            l.production_rate = production;
            l.solidification_rate = solidification;
            true
        }
        None => false,
    })
}

/// `DoLavaFlow(dt)` (Â§11.3), dt in microseconds. The fire callbacks run
/// while the terrain is being changed (see the module notes).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_lava_flow(t: *mut fc4_terrain, dt: f32) -> bool {
    write(t, false, |i, h| {
        let Inner { terrain, lava, .. } = i;
        match lava.as_mut() {
            Some(l) => {
                l.flow(terrain, dt, &mut CHost(h));
                true
            }
            None => false,
        }
    })
}

/// `BurnSurroundingStuff` (Â§11.4).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_lava_burn(t: *mut fc4_terrain) -> bool {
    write(t, false, |i, h| match i.lava.as_ref() {
        Some(l) => {
            l.burn_surrounding(&mut CHost(h));
            true
        }
        None => false,
    })
}

/// The lava level at a vertex (0 without lava or out of bounds).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_lava_level(t: *const fc4_terrain, x: i32, z: i32) -> f32 {
    read(t, 0.0, |i, _| {
        i.lava.as_ref().map_or(0.0, |l| l.level(&i.terrain, x, z))
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_lava_set_level(t: *mut fc4_terrain, x: i32, z: i32, v: f32) -> bool {
    write(t, false, |i, _| {
        let Inner { terrain, lava, .. } = i;
        match lava.as_mut() {
            Some(l) => {
                l.set_level(terrain, x, z, v);
                true
            }
            None => false,
        }
    })
}

/// The lava's source, active and burn rectangles (each may be NULL).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_lava_rects(
    t: *const fc4_terrain,
    source: *mut fc4_terrain_rect,
    active: *mut fc4_terrain_rect,
    burn: *mut fc4_terrain_rect,
) -> bool {
    read(t, false, |i, _| match i.lava.as_ref() {
        Some(l) => {
            put(source, l.source.into());
            put(active, l.active.into());
            put(burn, l.burn.into());
            true
        }
        None => false,
    })
}

// ---- records (Â§13) ----------------------------------------------------------------------------------

/// Which record `fc4_terrain_write_record` writes.
pub const FC4_TERRAIN_RECORD_MISC: u32 = 0;
pub const FC4_TERRAIN_RECORD_ALTITUDES: u32 = 1;
pub const FC4_TERRAIN_RECORD_PROPERTIES: u32 = 2;
/// The edge records: `FC4_TERRAIN_RECORD_EDGE + side` (side 0-3).
pub const FC4_TERRAIN_RECORD_EDGE: u32 = 0x64;

/// Writes one saved record's payload (Â§13.1). Copies when `capacity`
/// suffices; returns the payload size (0 for an unknown record).
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_write_record(
    t: *const fc4_terrain,
    record: u32,
    out: *mut u8,
    capacity: usize,
) -> usize {
    read(t, 0, |i, _| {
        let tr = &i.terrain;
        let b = match record {
            FC4_TERRAIN_RECORD_MISC => tr.write_misc(),
            FC4_TERRAIN_RECORD_ALTITUDES => tr.write_altitudes(),
            FC4_TERRAIN_RECORD_PROPERTIES => tr.write_properties(),
            r if (FC4_TERRAIN_RECORD_EDGE..FC4_TERRAIN_RECORD_EDGE + 4).contains(&r) => {
                match tr.write_edge(r - FC4_TERRAIN_RECORD_EDGE) {
                    Some(b) => b,
                    None => return 0,
                }
            }
            _ => return 0,
        };
        copy_out(&b, out, capacity)
    })
}

/// Reads an edge record's altitudes. Copies when `capacity` suffices;
/// returns the count, or -1 for a bad record.
#[no_mangle]
pub unsafe extern "C" fn fc4_terrain_read_edge(record: *const u8, len: usize, out: *mut f32, capacity: usize) -> i64 {
    guard(-1, || match Terrain::read_edge(bytes(record, len)) {
        Ok(v) => copy_out(&v, out, capacity) as i64,
        Err(_) => -1,
    })
}
