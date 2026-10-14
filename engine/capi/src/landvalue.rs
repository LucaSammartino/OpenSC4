// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The land value subsystem (`docs/specs/landvalue.md`) over the C ABI. See
//! the "land value" section of `include/fc4.h`.
//!
//! The grids are the game's own `cSC4SimGrid` objects: the core holds them as
//! **tokens** (interface pointers owned by the core, one reference each) and
//! reads and writes them through the host callbacks.

use std::cell::RefCell;
use std::ffi::c_void;

use fc4_core::landvalue::{GridKind, LandValueHost, LandValueSim, TuningInput};
use fc4_core::simgrid::GridGeometry;

use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};
use crate::{guard, slice_arg};

#[repr(C)]
pub struct fc4_landvalue {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_grid_geometry {
    pub tract_size: i32,
    pub shift: i32,
    pub count_x: i32,
    pub count_z: i32,
    pub width_x: f32,
    pub width_z: f32,
    pub inv_width_x: f32,
    pub inv_width_z: f32,
}

/// Grid element kinds of `create_grid`.
pub const FC4_GRID_U8: u32 = 0;
pub const FC4_GRID_F32: u32 = 1;

type U = *mut c_void;

/// What land value needs from the game (fc4.h: `fc4_landvalue_host`).
/// Missing callbacks behave as "nothing there" (0, false, no grid).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_landvalue_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    pub terrain_cells: Option<unsafe extern "C" fn(U, *mut i32, *mut i32)>,
    pub is_land_vertex: Option<unsafe extern "C" fn(U, u32, u32) -> bool>,
    pub altitude: Option<unsafe extern "C" fn(U, f32, f32) -> f32>,
    pub min_allowed_height: Option<unsafe extern "C" fn(U) -> f32>,
    pub max_allowed_height: Option<unsafe extern "C" fn(U) -> f32>,
    pub zone_type: Option<unsafe extern "C" fn(U, i32, i32) -> u32>,
    pub area_slope: Option<unsafe extern "C" fn(U, i32, i32, i32, i32) -> f32>,
    pub developer: Option<unsafe extern "C" fn(U, u32) -> u64>,
    pub desirability_map: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub create_grid: Option<unsafe extern "C" fn(U, u32, i32, u32) -> u64>,
    pub grid_geometry: Option<unsafe extern "C" fn(U, u64, *mut fc4_grid_geometry)>,
    pub grid_set_tract_size: Option<unsafe extern "C" fn(U, u64, i32)>,
    pub grid_read_u8: Option<unsafe extern "C" fn(U, u64, *mut u8, usize)>,
    pub grid_read_i16: Option<unsafe extern "C" fn(U, u64, *mut i16, usize)>,
    pub grid_write_u8: Option<unsafe extern "C" fn(U, u64, *const u8, usize)>,
    pub grid_write_f32: Option<unsafe extern "C" fn(U, u64, *const f32, usize)>,
    pub grid_shutdown: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    pub set_history_queue_length: Option<unsafe extern "C" fn(U, u32, u32)>,
    pub record_history: Option<unsafe extern "C" fn(U, u32, i32)>,
    pub post: Option<unsafe extern "C" fn(U, u32)>,
    /// Growth developer slot 10 `GrowthAndDeclineListsAreValid` (`state <
    /// 17`) of a developer token: true while its desirability map cannot
    /// change (landvalue.md §7.4, LV2). Read by the land value shadow only;
    /// NULL = unknown.
    pub developer_lists_valid: Option<unsafe extern "C" fn(U, u64) -> bool>,
}

/// The tuning exemplar (§8). A NULL array pointer means the property is
/// absent; a present empty array needs a non-NULL pointer.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_landvalue_tuning {
    pub struct_size: u32,
    pub has_tract_size: u32,
    pub tract_size: i64,
    pub intrinsic: *const i64,
    pub intrinsic_count: usize,
    pub wealth: *const i64,
    pub wealth_count: usize,
    pub water: *const i64,
    pub water_count: usize,
    pub altitude_curve: *const f32,
    pub altitude_curve_count: usize,
    pub developer_ids: *const u32,
    pub developer_id_count: usize,
    pub developer_weights: *const f32,
    pub developer_weight_count: usize,
    pub desirability_curve: *const f32,
    pub desirability_curve_count: usize,
}

/// Diagnostics (`fc4_landvalue_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_landvalue_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub dirty: u8,
    pub ignore_zoning: u8,
    pub average_land_value: u8,
    pub pipeline_state: u32,
    pub developer_count: u32,
    pub sum: i32,
    pub count: i32,
    pub min_alt: f32,
    pub max_alt: f32,
    pub mean_alt: f32,
    pub mode_alt: f32,
    pub pivot: f32,
    pub low_range: f32,
    pub high_range: f32,
    pub top_index: i32,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_grid_geometry>() == 32);
    assert!(std::mem::size_of::<fc4_landvalue_state>() == 56);
    assert!(std::mem::size_of::<fc4_landvalue_host>() == if cfg!(target_pointer_width = "32") { 96 } else { 192 });
    assert!(std::mem::size_of::<fc4_landvalue_tuning>() == if cfg!(target_pointer_width = "32") { 72 } else { 128 });
};

// ---- host adapter -----------------------------------------------------------------------------------

pub(crate) struct CHost(pub(crate) fc4_landvalue_host);

impl CHost {
    /// Growth developer slot 10 of `developer` (LV2); `None` when the host
    /// does not answer it.
    pub(crate) fn developer_lists_valid(&self, developer: u64) -> Option<bool> {
        // SAFETY: whoever created the object promised valid callbacks.
        self.0
            .developer_lists_valid
            .map(|f| unsafe { f(self.0.user, developer) })
    }
}

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $s.0.$f { Some(f) => unsafe { f($s.0.user $(, $arg)*) }, None => $default }
    };
}

impl LandValueHost for CHost {
    fn terrain_cells(&self) -> (i32, i32) {
        let (mut x, mut z) = (0, 0);
        cb!(self, terrain_cells, (), &mut x, &mut z);
        (x, z)
    }
    fn is_land_vertex(&self, vx: u32, vz: u32) -> bool {
        cb!(self, is_land_vertex, false, vx, vz)
    }
    fn altitude(&self, px: f32, pz: f32) -> f32 {
        cb!(self, altitude, 0.0, px, pz)
    }
    fn min_allowed_height(&self) -> f32 {
        cb!(self, min_allowed_height, 0.0)
    }
    fn max_allowed_height(&self) -> f32 {
        cb!(self, max_allowed_height, 0.0)
    }
    fn zone_type(&self, x: i32, z: i32) -> u32 {
        cb!(self, zone_type, 0, x, z)
    }
    fn area_slope(&self, x1: i32, z1: i32, x2: i32, z2: i32) -> f32 {
        cb!(self, area_slope, 0.0, x1, z1, x2, z2)
    }
    fn developer(&self, developer_type: u32) -> u64 {
        cb!(self, developer, 0, developer_type)
    }
    fn desirability_map(&self, developer: u64) -> u64 {
        cb!(self, desirability_map, 0, developer)
    }
    fn create_grid(&self, kind: GridKind, tract_size: i32, instance_id: u32) -> u64 {
        let k = match kind {
            GridKind::U8 => FC4_GRID_U8,
            GridKind::F32 => FC4_GRID_F32,
        };
        cb!(self, create_grid, 0, k, tract_size, instance_id)
    }
    fn grid_geometry(&self, grid: u64) -> GridGeometry {
        let mut g = fc4_grid_geometry::default();
        if grid != 0 {
            cb!(self, grid_geometry, (), grid, &mut g);
        }
        GridGeometry {
            tract_size: g.tract_size,
            shift: g.shift,
            count_x: g.count_x,
            count_z: g.count_z,
            width_x: g.width_x,
            width_z: g.width_z,
            inv_width_x: g.inv_width_x,
            inv_width_z: g.inv_width_z,
        }
    }
    fn grid_set_tract_size(&self, grid: u64, tract_size: i32) {
        cb!(self, grid_set_tract_size, (), grid, tract_size)
    }
    fn grid_read_u8(&self, grid: u64, out: &mut [u8]) {
        cb!(self, grid_read_u8, (), grid, out.as_mut_ptr(), out.len())
    }
    fn grid_read_i16(&self, grid: u64, out: &mut [i16]) {
        cb!(self, grid_read_i16, (), grid, out.as_mut_ptr(), out.len())
    }
    fn grid_write_u8(&self, grid: u64, values: &[u8]) {
        cb!(self, grid_write_u8, (), grid, values.as_ptr(), values.len())
    }
    fn grid_write_f32(&self, grid: u64, values: &[f32]) {
        cb!(self, grid_write_f32, (), grid, values.as_ptr(), values.len())
    }
    fn grid_shutdown(&self, grid: u64) {
        cb!(self, grid_shutdown, (), grid)
    }
    fn release(&self, token: u64) {
        cb!(self, release, (), token)
    }
    fn set_history_queue_length(&self, series: u32, length: u32) {
        cb!(self, set_history_queue_length, (), series, length)
    }
    fn record_history(&self, series: u32, value: i32) {
        cb!(self, record_history, (), series, value)
    }
    fn post(&self, msg_type: u32) {
        cb!(self, post, (), msg_type)
    }
}

// ---- handles ----------------------------------------------------------------------------------------

type Handle = RefCell<LandValueSim>;

/// Runs `f` on the simulator; the fallback for NULL, a re-entrant call or a
/// panic.
fn with_lv<R>(lv: *const fc4_landvalue, fallback: R, f: impl FnOnce(&mut LandValueSim) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_landvalue*.
    let Some(h) = (unsafe { (lv as *const Handle).as_ref() }) else {
        return fallback;
    };
    let Ok(mut s) = h.try_borrow_mut() else {
        return fallback;
    };
    guard(fallback, || f(&mut s))
}

/// Creates the core object. The host table is copied.
#[no_mangle]
pub unsafe extern "C" fn fc4_landvalue_create(host: *const fc4_landvalue_host) -> *mut fc4_landvalue {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_landvalue_host>() => {
            let sim = LandValueSim::new(Box::new(CHost(*h)));
            Box::into_raw(Box::new(RefCell::new(sim))) as *mut fc4_landvalue
        }
        _ => std::ptr::null_mut(),
    })
}

/// Destroys the core object; grid references still held (a loaded object that
/// was never initialised) are released.
#[no_mangle]
pub unsafe extern "C" fn fc4_landvalue_destroy(lv: *mut fc4_landvalue) {
    if lv.is_null() {
        return;
    }
    guard((), || {
        let b = Box::from_raw(lv as *mut Handle);
        if let Ok(mut s) = b.try_borrow_mut() {
            s.release_grids();
        }
        drop(b);
    });
}

pub(crate) unsafe fn tuning_of(t: &fc4_landvalue_tuning) -> TuningInput {
    let ints = |p: *const i64, n: usize| slice_arg(p, n).map(<[i64]>::to_vec);
    let floats = |p: *const f32, n: usize| slice_arg(p, n).map(<[f32]>::to_vec);
    TuningInput {
        tract_size: (t.has_tract_size != 0).then_some(t.tract_size),
        intrinsic: ints(t.intrinsic, t.intrinsic_count),
        wealth: ints(t.wealth, t.wealth_count),
        water: ints(t.water, t.water_count),
        altitude_curve: floats(t.altitude_curve, t.altitude_curve_count),
        developer_ids: slice_arg(t.developer_ids, t.developer_id_count).map(<[u32]>::to_vec),
        developer_weights: floats(t.developer_weights, t.developer_weight_count),
        desirability_curve: floats(t.desirability_curve, t.desirability_curve_count),
    }
}

/// `Init` (core part, §4.1 steps 2–7, 10). `tuning` NULL = no exemplar.
#[no_mangle]
pub unsafe extern "C" fn fc4_landvalue_init(lv: *mut fc4_landvalue, tuning: *const fc4_landvalue_tuning) -> bool {
    let t = match tuning.as_ref() {
        Some(t) if (t.struct_size as usize) >= std::mem::size_of::<fc4_landvalue_tuning>() => Some(tuning_of(t)),
        Some(_) => return false,
        None => None,
    };
    with_lv(lv, false, |s| s.init(t.as_ref()))
}

/// `Shutdown` (core part, §4.2): grids shut down and released.
#[no_mangle]
pub unsafe extern "C" fn fc4_landvalue_shutdown(lv: *mut fc4_landvalue) -> bool {
    with_lv(lv, false, LandValueSim::shutdown)
}

/// `DoMessage` (§5): always true for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_landvalue_handle_message(lv: *mut fc4_landvalue, msg_type: u32, data3: i64) -> bool {
    with_lv(lv, true, |s| s.handle_message(msg_type, data3))
}

/// `Step(budget)` (§6): completes at most one pipeline state.
#[no_mangle]
pub unsafe extern "C" fn fc4_landvalue_step(lv: *mut fc4_landvalue, budget_ms: u32) {
    with_lv(lv, (), |s| s.step(budget_ms))
}

/// Grid tokens (borrowed; 0 = none): 0 intrinsic, 1 total, 2 slope.
#[no_mangle]
pub unsafe extern "C" fn fc4_landvalue_grid(lv: *const fc4_landvalue, which: u32) -> u64 {
    with_lv(lv, 0, |s| match which {
        0 => s.intrinsic_grid(),
        1 => s.total_grid(),
        2 => s.slope_grid(),
        _ => 0,
    })
}

/// `GetLandValueType` for a land value byte: 1 low, 2 medium, 3 high.
#[no_mangle]
pub unsafe extern "C" fn fc4_landvalue_type_of(lv: *const fc4_landvalue, value: u8) -> u32 {
    with_lv(lv, 1, |s| s.land_value_type(value))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_landvalue_average(lv: *const fc4_landvalue) -> u8 {
    with_lv(lv, 0, |s| s.average_land_value())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_landvalue_mode_altitude(lv: *const fc4_landvalue) -> f32 {
    with_lv(lv, 0.0, |s| s.mode_altitude())
}

/// Diagnostics; false for NULL or an in-use object.
#[no_mangle]
pub unsafe extern "C" fn fc4_landvalue_get_state(lv: *const fc4_landvalue, out: *mut fc4_landvalue_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_lv(lv, false, |s| {
        let st = s.snapshot();
        *o = fc4_landvalue_state {
            struct_size: std::mem::size_of::<fc4_landvalue_state>() as u32,
            initialized: st.initialized.into(),
            dirty: st.dirty.into(),
            ignore_zoning: st.ignore_zoning.into(),
            average_land_value: st.average_land_value,
            pipeline_state: st.pipeline_state,
            developer_count: st.developers.len() as u32,
            sum: st.sum,
            count: st.count,
            min_alt: st.min_alt,
            max_alt: st.max_alt,
            mean_alt: st.mean_alt,
            mode_alt: st.mode_alt,
            pivot: st.altitude.pivot,
            low_range: st.altitude.low_range,
            high_range: st.altitude.high_range,
            top_index: st.altitude.top_index,
        };
        true
    })
}

/// `Write` (§10). Grid references go out as `fc4_ref`s with the token and the
/// grid CLSID.
#[no_mangle]
pub unsafe extern "C" fn fc4_landvalue_write(lv: *const fc4_landvalue, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    with_lv(lv, false, |s| s.write(&mut COut(o)))
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<std::rc::Rc<dyn fc4_core::stream::SaveObject>> {
    None
}

/// `Read` (§10). The three references are requested with IID `0xC9BD939A`;
/// each comes with one reference that the core then owns.
#[no_mangle]
pub unsafe extern "C" fn fc4_landvalue_read(lv: *mut fc4_landvalue, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with_lv(lv, false, |s| {
        s.read(&mut CIn {
            stream: i,
            decode: no_core_objects,
        })
    })
}
