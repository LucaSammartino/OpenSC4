// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The weather subsystem (`docs/specs/weather.md`) over the C ABI. See the
//! "weather" section of `include/fc4.h`.

use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::property::PropertyHolder;
use fc4_core::stream::SaveObject;
use fc4_core::weather::{WeatherHost, WeatherRng, WeatherSim};

use crate::guard;
use crate::props::{fc4_props, props_ref};
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};

#[repr(C)]
pub struct fc4_weather {
    _private: [u8; 0],
}

type U = *mut c_void;

/// What the weather needs from the game (fc4.h: `fc4_weather_host`). Missing
/// callbacks read as 0 / false / "no region" / "no terrain".
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_weather_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    pub city_size: Option<unsafe extern "C" fn(U, *mut f32, *mut f32)>,
    pub city_cell_width: Option<unsafe extern "C" fn(U, *mut f32, *mut f32)>,
    pub world_hemisphere: Option<unsafe extern "C" fn(U) -> i32>,
    pub sim_month: Option<unsafe extern "C" fn(U) -> i32>,
    pub water_prefs: Option<unsafe extern "C" fn(U, *mut u8, *mut u8) -> bool>,
    pub terrain_available: Option<unsafe extern "C" fn(U) -> bool>,
    pub altitude_at_vertex: Option<unsafe extern "C" fn(U, i32, i32) -> f32>,
    pub vertex_normal: Option<unsafe extern "C" fn(U, i32, i32, *mut f32)>,
    pub cell_has_water: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    pub altitude: Option<unsafe extern "C" fn(U, f32, f32) -> f32>,
    pub sea_level: Option<unsafe extern "C" fn(U) -> f32>,
    pub post: Option<unsafe extern "C" fn(U, u32, u32)>,
}

/// Diagnostics (`fc4_weather_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_weather_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub loaded_from_save: u8,
    pub post_init_done: u8,
    pub _pad: u8,
    pub count_x: i32,
    pub count_z: i32,
    pub last_season: i32,
    pub cells_per_week: i32,
    pub wind_month: i32,
    pub water_rect_count: u32,
    pub ambient_temp: f32,
    pub ambient_moisture: f32,
    pub wind_speed: f32,
    pub wind_dir_x: f32,
    pub wind_dir_z: f32,
    pub target_wind_speed: f32,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_weather_state>() == 56);
    assert!(std::mem::size_of::<fc4_weather_host>() == if cfg!(target_pointer_width = "32") { 56 } else { 112 });
};

// ---- host adapter -----------------------------------------------------------------------------------

struct CHost {
    h: fc4_weather_host,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
}

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $s.h.$f { Some(f) => unsafe { f($s.h.user $(, $arg)*) }, None => $default }
    };
}

impl WeatherHost for CHost {
    fn city_size(&self) -> (f32, f32) {
        let (mut x, mut z) = (0.0, 0.0);
        cb!(self, city_size, (), &mut x, &mut z);
        (x, z)
    }
    fn city_cell_width(&self) -> (f32, f32) {
        let (mut x, mut z) = (16.0, 16.0);
        cb!(self, city_cell_width, (), &mut x, &mut z);
        (x, z)
    }
    fn world_hemisphere(&self) -> i32 {
        cb!(self, world_hemisphere, 1)
    }
    fn sim_month(&self) -> i32 {
        cb!(self, sim_month, 1)
    }
    fn water_prefs(&self) -> Option<(u8, u8)> {
        let (mut lo, mut hi) = (0u8, 0u8);
        cb!(self, water_prefs, false, &mut lo, &mut hi).then_some((lo, hi))
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn terrain_available(&self) -> bool {
        cb!(self, terrain_available, false)
    }
    fn altitude_at_vertex(&self, x: i32, z: i32) -> f32 {
        cb!(self, altitude_at_vertex, 0.0, x, z)
    }
    fn vertex_normal(&self, x: i32, z: i32) -> [f32; 3] {
        let mut n = [0.0f32, 1.0, 0.0];
        cb!(self, vertex_normal, (), x, z, n.as_mut_ptr());
        n
    }
    fn cell_has_water(&self, x: i32, z: i32) -> bool {
        cb!(self, cell_has_water, false, x, z)
    }
    fn altitude(&self, world_x: f32, world_z: f32) -> f32 {
        cb!(self, altitude, 0.0, world_x, world_z)
    }
    fn sea_level(&self) -> f32 {
        cb!(self, sea_level, 0.0)
    }
    fn post(&self, msg_type: u32, data2: u32) {
        cb!(self, post, (), msg_type, data2)
    }
}

// ---- handles ----------------------------------------------------------------------------------------

struct Handle {
    sim: Rc<WeatherSim>,
    host: Rc<CHost>,
}

fn with_w<R>(w: *const fc4_weather, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_weather*.
    match unsafe { (w as *const Handle).as_ref() } {
        Some(h) => guard(fallback, || f(h)),
        None => fallback,
    }
}

/// Creates the core object. The host table is copied. The generator
/// (`Seed(0xFFFFFFFF)` in the original, §3) takes `derive(S, 0x2990C1BC)` of
/// `session_seed` (foundations §6.3).
#[no_mangle]
pub unsafe extern "C" fn fc4_weather_create(host: *const fc4_weather_host, session_seed: u32) -> *mut fc4_weather {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_weather_host>() => {
            let c = Rc::new(CHost {
                h: *h,
                tuning: RefCell::new(None),
            });
            let sim = WeatherSim::new(c.clone(), 0, Box::new(WeatherRng::from_session(session_seed)));
            Box::into_raw(Box::new(Handle { sim, host: c })) as *mut fc4_weather
        }
        _ => std::ptr::null_mut(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_weather_destroy(w: *mut fc4_weather) {
    if !w.is_null() {
        guard((), || drop(Box::from_raw(w as *mut Handle)));
    }
}

/// `Init` (core part, §8.1 steps 1, 3, 5–8). `tuning` is the "Weather Tuning
/// Parameters" exemplar (copied); NULL = no exemplar (current values stay).
#[no_mangle]
pub unsafe extern "C" fn fc4_weather_init(w: *mut fc4_weather, tuning: *const fc4_props) -> bool {
    let t = props_ref(tuning).cloned().map(Rc::new);
    with_w(w, false, |h| {
        *h.host.tuning.borrow_mut() = t;
        h.sim.init()
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_weather_shutdown(w: *mut fc4_weather) -> bool {
    with_w(w, false, |h| h.sim.shutdown())
}

/// Tooling only: copy the independent CRT generator state. Never writes native state.
#[no_mangle]
pub unsafe extern "C" fn fc4_weather_shadow_crt(w: *mut fc4_weather, state: *mut u32, restore: bool) -> bool {
    if state.is_null() {
        return false;
    }
    with_w(w, false, |h| {
        if restore {
            h.sim.set_crt(fc4_core::rng::CrtRand::new(*state));
        } else {
            *state = h.sim.crt().state();
        }
        true
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_weather_do_post_city_init(w: *mut fc4_weather) -> bool {
    with_w(w, false, |h| h.sim.do_post_city_init())
}

/// `DoMessage` (§5): SimNewDay (data1 = Julian day), SimNewWeek, SimNewMonth
/// (data3 = month), PostCityInit. Always true.
#[no_mangle]
pub unsafe extern "C" fn fc4_weather_handle_message(
    w: *mut fc4_weather,
    msg_type: u32,
    data1: u32,
    data3: u32,
) -> bool {
    with_w(w, true, |h| h.sim.handle_message(msg_type, data1, data3))
}

/// `CellWidthX/Z` (axis 0 / 1).
#[no_mangle]
pub unsafe extern "C" fn fc4_weather_cell_width(w: *const fc4_weather, axis: u32) -> f32 {
    with_w(w, 0.0, |h| {
        if axis == 0 {
            h.sim.cell_width_x()
        } else {
            h.sim.cell_width_z()
        }
    })
}

/// `CellCountX/Z` (axis 0 / 1).
#[no_mangle]
pub unsafe extern "C" fn fc4_weather_cell_count(w: *const fc4_weather, axis: u32) -> u32 {
    with_w(w, 0, |h| {
        if axis == 0 {
            h.sim.cell_count_x()
        } else {
            h.sim.cell_count_z()
        }
    })
}

unsafe fn put<T>(p: *mut T, v: T) {
    if let Some(r) = p.as_mut() {
        *r = v;
    }
}

#[no_mangle]
pub unsafe extern "C" fn fc4_weather_position_to_cell(
    w: *const fc4_weather,
    x: f32,
    z: f32,
    cx: *mut i32,
    cz: *mut i32,
) -> i32 {
    with_w(w, 0, |h| {
        let (r, a, b) = h.sim.position_to_cell(x, z);
        put(cx, a);
        put(cz, b);
        r
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_weather_cell_to_position(
    w: *const fc4_weather,
    cx: i32,
    cz: i32,
    x: *mut f32,
    z: *mut f32,
) {
    with_w(w, (), |h| {
        let (a, b) = h.sim.cell_to_position(cx, cz);
        put(x, a);
        put(z, b);
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_weather_cell_to_standard_city_cell(
    w: *const fc4_weather,
    cx: i32,
    cz: i32,
    sx: *mut i32,
    sz: *mut i32,
) {
    with_w(w, (), |h| {
        let (a, b) = h.sim.cell_to_standard_city_cell(cx, cz);
        put(sx, a);
        put(sz, b);
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_weather_standard_city_cell_to_cell(
    w: *const fc4_weather,
    sx: i32,
    sz: i32,
    cx: *mut i32,
    cz: *mut i32,
) {
    with_w(w, (), |h| {
        let (a, b) = h.sim.standard_city_cell_to_cell(sx, sz);
        put(cx, a);
        put(cz, b);
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_weather_season(w: *const fc4_weather) -> i32 {
    with_w(w, 0, |h| h.sim.current_season())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_weather_season_month(w: *const fc4_weather) -> i32 {
    with_w(w, 1, |h| h.sim.current_season_month())
}

/// `UpdateMoisture(rect)`, world units.
#[no_mangle]
pub unsafe extern "C" fn fc4_weather_update_moisture(w: *mut fc4_weather, x0: i32, z0: i32, x1: i32, z1: i32) {
    with_w(w, (), |h| h.sim.update_moisture(x0, z0, x1, z1))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_weather_moisture_value(w: *const fc4_weather, x: f32, z: f32) -> u8 {
    with_w(w, 0, |h| h.sim.moisture_value(x, z))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_weather_moisture_type(w: *const fc4_weather, x: f32, z: f32) -> u8 {
    with_w(w, 0, |h| h.sim.moisture_type(x, z))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_weather_set_manual_moisture_addition(
    w: *mut fc4_weather,
    x0: f32,
    z0: f32,
    x1: f32,
    z1: f32,
    v: f32,
) {
    with_w(w, (), |h| h.sim.set_manual_moisture_addition(x0, z0, x1, z1, v))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_weather_humidity(w: *const fc4_weather, x: f32, z: f32) -> f32 {
    with_w(w, 0.0, |h| h.sim.humidity(x, z))
}

/// `GetAmbientWind(&v)`: the speed; `v` (2 floats, may be NULL) := direction × speed.
#[no_mangle]
pub unsafe extern "C" fn fc4_weather_ambient_wind(w: *const fc4_weather, v: *mut f32) -> f32 {
    with_w(w, 0.0, |h| {
        let (speed, dir) = h.sim.ambient_wind();
        if !v.is_null() {
            *v = dir[0];
            *v.add(1) = dir[1];
        }
        speed
    })
}

/// Copies the moisture grid, x-major (`out[x * count_z + z]`), and reports its
/// shape. Returns the number of cells (copies at most `capacity`); 0 for an
/// empty or ragged grid.
#[no_mangle]
pub unsafe extern "C" fn fc4_weather_moisture_grid(
    w: *const fc4_weather,
    out: *mut u8,
    capacity: usize,
    count_x: *mut i32,
    count_z: *mut i32,
) -> usize {
    with_w(w, 0, |h| {
        let g = h.sim.moisture_grid();
        let nx = g.len();
        let nz = g.first().map_or(0, Vec::len);
        if g.iter().any(|r| r.len() != nz) {
            put(count_x, 0);
            put(count_z, 0);
            return 0;
        }
        put(count_x, nx as i32);
        put(count_z, nz as i32);
        if !out.is_null() {
            let mut i = 0;
            for row in &g {
                for &v in row {
                    if i >= capacity {
                        break;
                    }
                    *out.add(i) = v;
                    i += 1;
                }
            }
        }
        nx * nz
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_weather_get_state(w: *const fc4_weather, out: *mut fc4_weather_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_w(w, false, |h| {
        let d = h.sim.snapshot();
        *o = fc4_weather_state {
            struct_size: std::mem::size_of::<fc4_weather_state>() as u32,
            initialized: d.initialized.into(),
            loaded_from_save: d.loaded_from_save.into(),
            post_init_done: d.post_init_done.into(),
            _pad: 0,
            count_x: d.count_x,
            count_z: d.count_z,
            last_season: d.last_season,
            cells_per_week: d.cells_per_week,
            wind_month: d.wind_month,
            water_rect_count: d.water_rects.len() as u32,
            ambient_temp: d.ambient_temp,
            ambient_moisture: d.ambient_moisture,
            wind_speed: d.wind_speed,
            wind_dir_x: d.wind_dir[0],
            wind_dir_z: d.wind_dir[1],
            target_wind_speed: d.target_wind_speed,
        };
        true
    })
}

/// `Write` (§10).
#[no_mangle]
pub unsafe extern "C" fn fc4_weather_write(w: *const fc4_weather, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    with_w(w, false, |h| h.sim.write(&mut COut(o)))
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    None
}

/// `Read` (§10).
#[no_mangle]
pub unsafe extern "C" fn fc4_weather_read(w: *mut fc4_weather, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with_w(w, false, |h| {
        h.sim.read(&mut CIn {
            stream: i,
            decode: no_core_objects,
        })
    })
}

#[cfg(test)]
mod shadow_tests {
    use super::*;

    #[test]
    fn observed_crt_is_independent_and_drives_weekly_reset() {
        unsafe extern "C" fn size(_: U, x: *mut f32, z: *mut f32) {
            *x = 32.0;
            *z = 32.0;
        }
        unsafe {
            let mut host: fc4_weather_host = std::mem::zeroed();
            host.struct_size = std::mem::size_of::<fc4_weather_host>() as u32;
            host.city_size = Some(size);
            let w = fc4_weather_create(&host, 7);
            let tuning = crate::props::fc4_props_create();
            assert!(crate::props::fc4_props_set_i32s(tuning, 0xA760_7D70, &1, 1));
            assert!(fc4_weather_init(w, tuning));
            crate::props::fc4_props_destroy(tuning);
            fc4_weather_handle_message(w, 0x6695_6815, 0, 0); // exhaust first pass
            let before = with_w(w, None, |h| Some(h.sim.snapshot()));
            let mut seed = 0xA123_4567;
            assert!(fc4_weather_shadow_crt(w, &mut seed, true));
            assert_eq!(before, with_w(w, None, |h| Some(h.sim.snapshot())));
            fc4_weather_handle_message(w, 0x6695_6815, 0, 0); // exactly one reset/draw
            let mut actual = 0;
            assert!(fc4_weather_shadow_crt(w, &mut actual, false));
            assert_eq!(actual, seed.wrapping_mul(214_013).wrapping_add(2_531_011));
            assert_eq!(seed, 0xA123_4567); // the observed input is never overwritten
            assert!(!fc4_weather_shadow_crt(w, std::ptr::null_mut(), true));
            assert!(!fc4_weather_shadow_crt(std::ptr::null_mut(), &mut seed, true));
            fc4_weather_destroy(w);
        }
    }
}
