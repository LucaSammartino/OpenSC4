// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cISC4WeatherSimulator` (§3–§10).

use std::any::Any;
use std::cell::{Cell, RefCell, RefMut};
use std::f64::consts::PI;
use std::rc::Rc;

use crate::julian::civil_from_julian;
use crate::property::PropertyHolder;
use crate::props::{self, round_half_even_f32};
use crate::rng::CrtRand;
use crate::stream::{InStream, OutStream, SaveObject};
use crate::variant::{Elem, Variant};
use crate::x87::r24;

use super::{msg, prop, CellIterator, RngState, WeatherHost, WeatherRandom, CLSID_WEATHER_SIMULATOR};

/// Record version written (§10); versions ≥ 3 are rejected.
pub const WEATHER_RECORD_VERSION: u16 = 2;

/// [choice] The largest cell count per axis (SC4 cities have at most 256).
pub const MAX_CELLS: i32 = 1024;

/// Byte grids with a count at or above this are skipped on read (§10).
const GRID_SKIP: u32 = 1_000_000;

/// An x-major byte grid: `grid[x][z]`.
pub type Grid = Vec<Vec<u8>>;

/// The simulator state (§3), in save order, plus the unsaved fields.
#[derive(Clone, Debug, PartialEq)]
pub struct WeatherData {
    pub count_x: i32,
    pub count_z: i32,
    pub cell_width_x: f32,
    pub cell_width_z: f32,
    pub inv_cell_width_x: f32,
    pub inv_cell_width_z: f32,
    pub size_x: f32,
    pub size_z: f32,
    pub city_cells_per_cell_x: i32,
    pub city_cells_per_cell_z: i32,
    pub subdiv_x: i32,
    pub subdiv_z: i32,
    pub blend_radius: i32,
    pub last_season: i32,
    pub base_temp: f32,
    pub temp_amp_a: f32,
    pub temp_amp_b: f32,
    pub ambient_temp: f32,
    pub temperature: Grid,
    /// Bytes 0/1 (kept raw for byte-exact saves).
    pub sea_mask: Grid,
    /// `(x0, z0, x1, z1)`, exclusive ends.
    pub water_rects: Vec<[i32; 4]>,
    pub iterator: CellIterator,
    pub cells_per_week: i32,
    pub sea_radius: i32,
    pub sea_weight: f32,
    pub height_factor: f32,
    pub wind_factor: f32,
    pub moisture: Grid,
    pub moisture_base: Grid,
    pub ambient_moisture: f32,
    pub base_moisture: f32,
    pub moist_amp_a: f32,
    pub moist_amp_b: f32,
    pub slope_k: f32,
    pub high_moisture: f32,
    pub curvature_threshold: f32,
    pub tuning80: u32,
    pub default_wind_speed: f32,
    pub default_wind_dir: [f32; 2],
    pub wind_speed: f32,
    pub wind_dir: [f32; 2],
    pub prev_wind_speed: f32,
    pub prev_wind_dir: [f32; 2],
    pub target_wind_speed: f32,
    pub target_wind_dir: [f32; 2],
    pub wind_month: i32,
    // ---- not saved ----
    pub wind_spread: f32,
    pub tuning81: u32,
    pub initialized: bool,
    pub loaded_from_save: bool,
    pub post_init_done: bool,
}

impl Default for WeatherData {
    /// The constructor values of §3.
    fn default() -> Self {
        WeatherData {
            count_x: 0,
            count_z: 0,
            cell_width_x: 0.0,
            cell_width_z: 0.0,
            inv_cell_width_x: 0.0,
            inv_cell_width_z: 0.0,
            size_x: 0.0,
            size_z: 0.0,
            city_cells_per_cell_x: 1,
            city_cells_per_cell_z: 1,
            subdiv_x: 8,
            subdiv_z: 8,
            blend_radius: 1,
            last_season: -1,
            base_temp: 24.0,
            temp_amp_a: 0.5,
            temp_amp_b: 0.5,
            ambient_temp: 20.0,
            temperature: Vec::new(),
            sea_mask: Vec::new(),
            water_rects: Vec::new(),
            iterator: CellIterator::new(0, 0, -1, -1),
            cells_per_week: 1,
            sea_radius: 6,
            sea_weight: 0.95,
            height_factor: 0.0035,
            wind_factor: 0.25,
            moisture: Vec::new(),
            moisture_base: Vec::new(),
            ambient_moisture: 0.0,
            base_moisture: 0.5,
            moist_amp_a: 0.5,
            moist_amp_b: 0.4,
            slope_k: 128.0,
            high_moisture: 0.12,
            curvature_threshold: 4.0,
            tuning80: 64,
            default_wind_speed: 3.0,
            default_wind_dir: [0.0, 1.0],
            wind_speed: 3.0,
            wind_dir: [1.0, 0.0],
            prev_wind_speed: 3.0,
            prev_wind_dir: [1.0, 0.0],
            target_wind_speed: 3.0,
            target_wind_dir: [1.0, 0.0],
            wind_month: -1,
            wind_spread: 0.25,
            tuning81: 15,
            initialized: false,
            loaded_from_save: false,
            post_init_done: false,
        }
    }
}

/// The shipped "Weather Tuning Parameters" exemplar (§9, `SimCity_1.dat`),
/// for hosts and tests without the game's data files.
pub fn shipped_tuning() -> PropertyHolder {
    let mut h = PropertyHolder::new();
    let s32 = |v: i32| Variant::Scalar(Elem::I32(v));
    let f32v = |v: f32| Variant::Scalar(Elem::F32(v));
    let u32v = |v: u32| Variant::Scalar(Elem::U32(v));
    h.add(prop::SUBDIV_X, s32(1));
    h.add(prop::SUBDIV_Z, s32(1));
    h.add(prop::BLEND_RADIUS, s32(1));
    h.add(prop::BASE_TEMP, f32v(65.0));
    h.add(prop::TEMP_AMP_A, f32v(0.5));
    h.add(prop::TEMP_AMP_B, f32v(0.5));
    h.add(prop::SEA_RADIUS, s32(10));
    h.add(prop::SEA_WEIGHT, f32v(0.9));
    h.add(prop::HEIGHT_FACTOR, f32v(0.082));
    h.add(prop::WIND_FACTOR, f32v(0.25));
    h.add(prop::BASE_MOISTURE, f32v(0.2));
    h.add(prop::MOIST_AMP_A, f32v(0.2));
    h.add(prop::MOIST_AMP_B, f32v(1.0));
    h.add(prop::SLOPE, f32v(0.35));
    h.add(prop::HIGH_MOISTURE, f32v(1.0));
    h.add(prop::CURVATURE_THRESHOLD, f32v(2.5));
    h.add(prop::TUNING_80, u32v(40));
    h.add(prop::TUNING_81, u32v(16));
    h.add(prop::WIND_SPREAD_DEGREES, f32v(60.0));
    h
}

fn get(g: &Grid, x: i32, z: i32) -> Option<u8> {
    if x < 0 || z < 0 {
        return None;
    }
    g.get(x as usize).and_then(|r| r.get(z as usize)).copied()
}

fn put(g: &mut Grid, x: i32, z: i32, v: u8) {
    if x < 0 || z < 0 {
        return;
    }
    if let Some(c) = g.get_mut(x as usize).and_then(|r| r.get_mut(z as usize)) {
        *c = v;
    }
}

fn filled(nx: i32, nz: i32, v: u8) -> Grid {
    vec![vec![v; nz.max(0) as usize]; nx.max(0) as usize]
}

fn moisture_integer(value: f32) -> i32 {
    // Weather §6.5.1: the native bias/bit conversion remains defined outside
    // the range where it agrees with a general nearest-even conversion.
    (value + 12_582_912.0).to_bits().wrapping_sub(0x4B40_0000) as i32
}

#[test]
fn we1_moisture_conversion_ties() {
    for (normal, expected_bits, expected) in [
        (0x3F00_0000, 0x424A_0000, 50),
        (0xBF00_0000, 0x424E_0000, 52),
        (0x3EFF_FF80, 0x424A_0001, 51),
    ] {
        let u = 51.0 - f32::from_bits(normal);
        assert_eq!(u.to_bits(), expected_bits);
        assert_eq!(moisture_integer(u), expected);
    }
}

/// §6.5 step 3, x-major arrays: z pass into a copy, then x pass × 0.0625.
/// Mirror edges; the high-z mirror uses countX [quirk]. Each operation is f32.
fn smoothed_heights(hgt: &[f32], nx: i32, nz: i32, a4: [i32; 4]) -> Vec<f32> {
    let n = hgt.len();
    let at = |x: i32, z: i32| (x * nz + z) as usize;
    let hz = |x: i32, z: i32| -> f32 {
        let z = if z < 0 {
            -z
        } else if z >= nx {
            2 * (nx - 1) - z
        } else {
            z
        };
        if (0..nz).contains(&z) {
            hgt[at(x, z)]
        } else {
            0.0
        }
    };
    let mut tz = vec![0f32; n];
    for x in a4[0]..a4[2] {
        for z in a4[1]..a4[3] {
            tz[at(x, z)] = (hz(x, z - 1) + hz(x, z + 1)) + 2.0 * hgt[at(x, z)];
        }
    }
    let mut sm = vec![0f32; n];
    for x in a4[0]..a4[2] {
        for z in a4[1]..a4[3] {
            sm[at(x, z)] =
                ((tz[at(reflect(x - 1, nx), z)] + tz[at(reflect(x + 1, nx), z)]) + 2.0 * tz[at(x, z)]) * 0.0625;
        }
    }
    sm
}

fn reflect(i: i32, n: i32) -> i32 {
    let r = if i < 0 {
        -i
    } else if i >= n {
        2 * (n - 1) - i
    } else {
        i
    };
    r.clamp(0, (n - 1).max(0))
}

fn clamp_count(v: f32) -> i32 {
    (v as i32).clamp(0, MAX_CELLS)
}

/// The weather simulator.
pub struct WeatherSim {
    host: Rc<dyn WeatherHost>,
    token: Cell<u64>,
    d: RefCell<WeatherData>,
    rng: RefCell<Box<dyn WeatherRandom>>,
    crt: RefCell<CrtRand>,
}

impl WeatherSim {
    /// A new simulator with constructor values (§3) and the given generator
    /// (normally a wall-clock seeded [`super::WeatherRng`]).
    pub fn new(host: Rc<dyn WeatherHost>, token: u64, rng: Box<dyn WeatherRandom>) -> Rc<WeatherSim> {
        Rc::new(WeatherSim {
            host,
            token: Cell::new(token),
            d: RefCell::new(WeatherData::default()),
            rng: RefCell::new(rng),
            crt: RefCell::new(CrtRand::default()),
        })
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    /// Replaces the generator, keeping nothing of the old one.
    pub fn set_random(&self, rng: Box<dyn WeatherRandom>) {
        *self.rng.borrow_mut() = rng;
    }

    pub fn rng_state(&self) -> RngState {
        self.rng.borrow().state()
    }

    /// The C-runtime generator used for the iterator start (§7).
    pub fn crt(&self) -> CrtRand {
        *self.crt.borrow()
    }

    pub fn set_crt(&self, crt: CrtRand) {
        *self.crt.borrow_mut() = crt;
    }

    pub fn snapshot(&self) -> WeatherData {
        self.d.borrow().clone()
    }

    /// Direct state access, for hosts that restore state and for tests.
    pub fn data_mut(&self) -> RefMut<'_, WeatherData> {
        self.d.borrow_mut()
    }

    /// The grid size used by the algorithms (clamped counts).
    fn dims(&self) -> (i32, i32) {
        let d = self.d.borrow();
        (d.count_x.clamp(0, MAX_CELLS), d.count_z.clamp(0, MAX_CELLS))
    }

    fn post(&self, msg_type: u32, data2: u32) {
        self.host.post(msg_type, data2);
    }

    // ---- lifecycle (§8) ----------------------------------------------------------------------------

    /// `Init()` (core part: subscriptions and agents are the shim's).
    pub fn init(&self) -> bool {
        if self.d.borrow().initialized {
            return true;
        }
        self.read_tuning();
        if !self.d.borrow().loaded_from_save {
            let (sx, sz) = self.host.city_size();
            let (cwx, cwz) = self.host.city_cell_width();
            {
                let mut d = self.d.borrow_mut();
                d.size_x = sx;
                d.size_z = sz;
                d.cell_width_x = 16.0;
                d.cell_width_z = 16.0;
                d.inv_cell_width_x = 0.0625;
                d.inv_cell_width_z = 0.0625;
                d.count_x = clamp_count(sx * 0.0625);
                d.count_z = clamp_count(sz * 0.0625);
                d.city_cells_per_cell_x = (16.0 / cwx) as i32;
                d.city_cells_per_cell_z = (16.0 / cwz) as i32;
                let (nx, nz) = (d.count_x, d.count_z);
                d.temperature = filled(nx, nz, 127);
                d.moisture = filled(nx, nz, 64);
                d.sea_mask = filled(nx, nz, 0);
                d.moisture_base = filled(nx, nz, 0);
            }
            self.generate_moisture_variation();
        }
        let (nx, nz) = self.dims();
        {
            let mut d = self.d.borrow_mut();
            d.iterator.set_rect(0, 0, nx - 1, nz - 1);
            d.iterator.mode = 1;
            d.iterator.reset(true, &mut self.crt.borrow_mut());
            d.cells_per_week = ((i64::from(nx) * i64::from(nz)) / i64::from(d.subdiv_x.max(1))) as i32;
            d.ambient_moisture = d.base_moisture;
            d.ambient_temp = d.base_temp;
            d.initialized = true;
        }
        true
    }

    /// `Shutdown()` (core part).
    pub fn shutdown(&self) -> bool {
        self.d.borrow_mut().initialized = false;
        true
    }

    /// `DoPostCityInit()`.
    pub fn do_post_city_init(&self) -> bool {
        if !self.d.borrow().post_init_done {
            self.do_complete_simulation();
            self.d.borrow_mut().post_init_done = true;
        }
        true
    }

    /// `DoCompleteSimulation` (§5.5).
    pub fn do_complete_simulation(&self) {
        self.do_monthly_simulation(1);
        let (nx, nz) = self.dims();
        let per_week = self.d.borrow().cells_per_week;
        let limit = if per_week > 0 {
            (i64::from(nx) * i64::from(nz)) / i64::from(per_week) + 3
        } else {
            1
        };
        for _ in 0..limit {
            if self.do_weekly_simulation() {
                break;
            }
        }
    }

    /// The tuning exemplar (§9); missing properties keep their value.
    pub fn read_tuning(&self) {
        let Some(h) = self.host.tuning() else { return };
        let mut d = self.d.borrow_mut();
        let int = |id| props::i64_of(&h, id).map(|v| v as i32);
        let flt = |id| props::f32_of(&h, id);
        if let Some(v) = int(prop::SUBDIV_X) {
            d.subdiv_x = v.max(1);
        }
        if let Some(v) = int(prop::SUBDIV_Z) {
            d.subdiv_z = v.max(1);
        }
        if let Some(v) = int(prop::BLEND_RADIUS) {
            d.blend_radius = v;
        }
        if let Some(v) = flt(prop::BASE_TEMP) {
            d.base_temp = v;
        }
        if let Some(v) = flt(prop::TEMP_AMP_A) {
            d.temp_amp_a = v.clamp(0.0, 1.0);
        }
        if let Some(v) = flt(prop::TEMP_AMP_B) {
            d.temp_amp_b = v;
        }
        if let Some(v) = int(prop::SEA_RADIUS) {
            d.sea_radius = v;
        }
        if let Some(v) = flt(prop::SEA_WEIGHT) {
            d.sea_weight = if v < 0.0 { 0.0 } else { v };
        }
        if let Some(v) = flt(prop::HEIGHT_FACTOR) {
            d.height_factor = v;
        }
        if let Some(v) = flt(prop::WIND_FACTOR) {
            d.wind_factor = v;
        }
        if let Some(v) = flt(prop::BASE_MOISTURE) {
            d.base_moisture = v;
        }
        if let Some(v) = flt(prop::MOIST_AMP_A) {
            d.moist_amp_a = v.clamp(0.0, 1.0);
        }
        if let Some(v) = flt(prop::MOIST_AMP_B) {
            d.moist_amp_b = v;
        }
        if let Some(v) = flt(prop::SLOPE) {
            d.slope_k = v * 255.0;
        }
        if let Some(v) = flt(prop::HIGH_MOISTURE) {
            d.high_moisture = v;
        }
        if let Some(v) = flt(prop::CURVATURE_THRESHOLD) {
            d.curvature_threshold = v;
        }
        if let Some(v) = props::u32_of(&h, prop::TUNING_80) {
            d.tuning80 = v;
        }
        if let Some(v) = props::u32_of(&h, prop::TUNING_81) {
            d.tuning81 = v;
        }
        if let Some(v) = flt(prop::DEFAULT_WIND_SPEED) {
            d.default_wind_speed = v;
        }
        if let Some(v) = flt(prop::DEFAULT_WIND_DIR_X) {
            d.default_wind_dir[0] = v;
        }
        if let Some(v) = flt(prop::DEFAULT_WIND_DIR_Z) {
            d.default_wind_dir[1] = v;
        }
        if let Some(v) = flt(prop::WIND_SPREAD_DEGREES) {
            d.wind_spread = ((f64::from(v) * PI / 360.0).tan() * 0.5) as f32;
        }
    }

    // ---- interface (§4) -----------------------------------------------------------------------------

    pub fn cell_width_x(&self) -> f32 {
        self.d.borrow().cell_width_x
    }
    pub fn cell_width_z(&self) -> f32 {
        self.d.borrow().cell_width_z
    }
    pub fn cell_count_x(&self) -> u32 {
        self.d.borrow().count_x as u32
    }
    pub fn cell_count_z(&self) -> u32 {
        self.d.borrow().count_z as u32
    }

    /// `PositionToCell(x, z, &cx, &cz)`: (return value, cx, cz); the return
    /// value is 0 ([choice]).
    pub fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32, i32) {
        let d = self.d.borrow();
        (0, (x / d.cell_width_x) as i32, (z / d.cell_width_z) as i32)
    }

    pub fn cell_to_position(&self, cx: i32, cz: i32) -> (f32, f32) {
        let d = self.d.borrow();
        (cx as f32 * d.cell_width_x, cz as f32 * d.cell_width_z)
    }

    /// `CellToStandardCityCell`: the z output uses the x input [quirk].
    pub fn cell_to_standard_city_cell(&self, cx: i32, _cz: i32) -> (i32, i32) {
        let d = self.d.borrow();
        (
            cx.wrapping_mul(d.city_cells_per_cell_x),
            cx.wrapping_mul(d.city_cells_per_cell_z),
        )
    }

    /// `StandardCityCellToCell` (a zero factor gives 0).
    pub fn standard_city_cell_to_cell(&self, sx: i32, sz: i32) -> (i32, i32) {
        let d = self.d.borrow();
        (
            sx.checked_div(d.city_cells_per_cell_x).unwrap_or(0),
            sz.checked_div(d.city_cells_per_cell_z).unwrap_or(0),
        )
    }

    /// `GetCurrentSeasonMonth()`.
    pub fn current_season_month(&self) -> i32 {
        let m = self.host.sim_month();
        if self.host.world_hemisphere() != 1 {
            m.wrapping_add(5) % 12 + 1
        } else {
            m
        }
    }

    /// `GetCurrentSeason()`: 0 winter .. 3 fall.
    pub fn current_season(&self) -> i32 {
        self.current_season_month().wrapping_sub(1) / 3
    }

    /// `UpdateMoisture(rect)` in world units.
    pub fn update_moisture(&self, x0: i32, z0: i32, x1: i32, z1: i32) {
        if !self.host.terrain_available() {
            return;
        }
        self.simulate_moisture(x0 / 16, z0 / 16, x1.wrapping_add(15) / 16, z1.wrapping_add(15) / 16);
    }

    /// `GetMoistureValue(x, z)` (world units).
    pub fn moisture_value(&self, x: f32, z: f32) -> u8 {
        let d = self.d.borrow();
        if !d.initialized {
            return 0;
        }
        let clampw = |v: f32, size: f32| {
            if v >= size {
                size - 1.0
            } else if v < 0.0 {
                0.0
            } else {
                v
            }
        };
        let cx = (clampw(x, d.size_x) / 16.0).floor() as i32;
        let cz = (clampw(z, d.size_z) / 16.0).floor() as i32;
        get(&d.moisture, cx, cz).unwrap_or(0)
    }

    /// The moisture grid (`GetMoistureValueMapPtr` exposes the live object;
    /// the shim keeps an `SC4Vector` mirror of this).
    pub fn moisture_grid(&self) -> Grid {
        self.d.borrow().moisture.clone()
    }

    pub fn moisture_type(&self, x: f32, z: f32) -> u8 {
        self.moisture_value(x, z) >> 4
    }

    /// `SetManualMoistureAddition` (§6.4.1).
    pub fn set_manual_moisture_addition(&self, x0: f32, z0: f32, x1: f32, z1: f32, v: f32) {
        let b = if v < 0.0 {
            0.0
        } else if v <= 255.0 {
            v * 1.275
        } else {
            325.125
        };
        let byte = (round_half_even_f32(b) as i64).rem_euclid(256) as u8;
        let (nx, nz) = self.dims();
        let bounds = |a: f32, b: f32, n: i32| {
            let (mut lo, mut hi) = ((a / 16.0) as i32, (b / 16.0) as i32);
            if lo < 0 {
                lo = 0;
                hi = hi.min(n);
            } else {
                hi = hi.min(n - 1);
            }
            (lo, hi)
        };
        let (xa, xb) = bounds(x0, x1, nx);
        let (za, zb) = bounds(z0, z1, nz);
        let mut d = self.d.borrow_mut();
        for i in xa..xb {
            for j in za..zb {
                put(&mut d.moisture_base, i, j, byte);
            }
        }
    }

    /// `GetHumidity(x, z)`: position ignored [compat].
    pub fn humidity(&self, _x: f32, _z: f32) -> f32 {
        let b = self.d.borrow().base_moisture;
        b + (1.0 - b) * 0.5
    }

    /// `GetAmbientWind(&v)` (weather §5.1.2): measure and rescale the stored direction.
    pub fn ambient_wind(&self) -> (f32, [f32; 2]) {
        let d = self.d.borrow();
        let mut v = d.wind_dir;
        let len = (v[0] * v[0] + v[1] * v[1]).sqrt();
        if len > 0.01 {
            let scale = d.wind_speed / len;
            v = [v[0] * scale, v[1] * scale];
        }
        (d.wind_speed, v)
    }

    pub fn wind_at_cell(&self, _x: i32, _z: i32) -> (f32, [f32; 2]) {
        self.ambient_wind()
    }

    // ---- messages (§5) --------------------------------------------------------------------------------

    /// `DoMessage` for the agent messages and PostCityInit; always true.
    pub fn handle_message(&self, msg_type: u32, data1: u32, data3: u32) -> bool {
        match msg_type {
            msg::SIM_NEW_DAY => self.do_daily(data1 as i32),
            msg::SIM_NEW_WEEK => {
                self.do_weekly_simulation();
            }
            msg::SIM_NEW_MONTH => self.do_monthly_simulation(data3 as i32),
            msg::POST_CITY_INIT => {
                self.do_post_city_init();
            }
            _ => {}
        }
        true
    }

    /// SimNewDay (§5.1): interpolates the wind within the wind month.
    pub fn do_daily(&self, julian: i32) {
        let date = civil_from_julian(julian);
        let mut d = self.d.borrow_mut();
        if date.month != d.wind_month {
            return;
        }
        // WE1: load the binary64 reciprocal before the one 24-bit rounding.
        let t = r24(f64::from(date.day - 1) * f64::from_bits(0x3FA1_1111_1111_1111)) as f32;
        let p = [
            d.prev_wind_speed * d.prev_wind_dir[0],
            d.prev_wind_speed * d.prev_wind_dir[1],
        ];
        let q = [
            d.target_wind_speed * d.target_wind_dir[0],
            d.target_wind_speed * d.target_wind_dir[1],
        ];
        let v = [p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t];
        let len = (v[0] * v[0] + v[1] * v[1]).sqrt();
        d.wind_dir = if len > 0.0 {
            let reciprocal = 1.0 / len;
            [v[0] * reciprocal, v[1] * reciprocal]
        } else {
            v
        };
        d.wind_speed = d.prev_wind_speed + (d.target_wind_speed - d.prev_wind_speed) * t;
    }

    /// `DoWeeklySimulation` (§5.2): true if the iterator wrapped.
    pub fn do_weekly_simulation(&self) -> bool {
        // [choice] At most one grid's worth of cells per pass: Init sets
        // cellsPerWeek = cells / subdivX, so only a hostile save between
        // Read and Init can exceed this.
        let (cx, cz) = self.dims();
        let mut n = i64::from(self.d.borrow().cells_per_week).min(i64::from(cx) * i64::from(cz));
        let mut cycled = false;
        let mut idle_wraps = 0;
        while n > 0 {
            let next = self.d.borrow_mut().iterator.next_cell();
            match next {
                Some((x, z)) => {
                    self.simulate_cell_temperature(x, z);
                    n -= 1;
                    idle_wraps = 0;
                }
                None => {
                    self.d.borrow_mut().iterator.reset(true, &mut self.crt.borrow_mut());
                    self.post(msg::WEATHER_CYCLE_COMPLETED, 0);
                    cycled = true;
                    idle_wraps += 1;
                    if idle_wraps >= 2 {
                        break;
                    }
                }
            }
        }
        let (nx, nz) = self.dims();
        self.simulate_moisture(0, 0, nx, nz);
        cycled
    }

    /// `DoMonthlySimulation(month)` (§5.3).
    pub fn do_monthly_simulation(&self, month: i32) {
        if month == 1 && self.host.terrain_available() {
            self.rebuild_water();
        }
        let m = self.current_season_month();
        self.new_current_ambient_values(m);
        let s = self.current_season();
        let changed = {
            let mut d = self.d.borrow_mut();
            if s != d.last_season {
                d.last_season = s;
                d.wind_speed = d.default_wind_speed;
                d.wind_dir = d.default_wind_dir;
                true
            } else {
                false
            }
        };
        if changed {
            self.post(msg::SEASON_CHANGED, s as u32);
        }
    }

    // ---- algorithms (§6) ------------------------------------------------------------------------------

    /// `GetNewCurrentAmbientValues(m)` (§6.1) and the wind target (§6.2).
    ///
    /// Exact operation order *[compat]* (foundations §18):
    /// `r24(r24(1 + r24(cos(r24(k × w)) × r24(ampB × ampA))) × base)` with
    /// `k` the exact month offset, `w = float32(π/6)` and the cosine taken
    /// unrounded (x87 `fcos` ignores the precision control; FreeCity4 takes
    /// the `double` cosine of the rounded angle [choice]).
    pub fn new_current_ambient_values(&self, m: i32) {
        {
            let mut d = self.d.borrow_mut();
            let w = f64::from(std::f32::consts::FRAC_PI_6);
            let wave = |peak: i32, amp_a: f32, amp_b: f32, base: f32| -> f32 {
                let angle = r24(f64::from(m.wrapping_sub(peak)) * w);
                let p = r24(angle.cos() * r24(f64::from(amp_b) * f64::from(amp_a)));
                r24(r24(1.0 + p) * f64::from(base)) as f32
            };
            d.ambient_temp = wave(8, d.temp_amp_a, d.temp_amp_b, d.base_temp);
            d.ambient_moisture = wave(1, d.moist_amp_a, d.moist_amp_b, d.base_moisture);
        }
        self.new_wind_target();
        self.d.borrow_mut().wind_month = m;
    }

    /// The edge with the most water, as an onshore direction (§6.2).
    pub fn onshore_direction(&self) -> (i32, i32) {
        let (nx, nz) = self.dims();
        let h = &self.host;
        let count = |f: &dyn Fn(i32) -> bool, n: i32| (0..n).filter(|&i| f(i)).count();
        let edges = [
            (count(&|z| h.cell_has_water(0, z), nz), 1, 0),
            (count(&|x| h.cell_has_water(x, 0), nx), 0, 1),
            (count(&|z| h.cell_has_water(nx - 1, z), nz), -1, 0),
            (count(&|x| h.cell_has_water(x, nz - 1), nx), 0, -1),
        ];
        let best = edges.iter().max().copied().unwrap_or((0, 1, 0));
        (best.1, best.2)
    }

    /// The monthly wind target (§6.2), at 24-bit precision *[compat]*:
    /// `s := r24(g × windSpread)` until `|s| ≤ 1` ([choice]: at most 100
    /// draws, else 0); `c := r24(sqrt(r24(1 − r24(s × s))))`;
    /// `x := r24(r24(bx × c) − r24(bz × s))`, `z := r24(r24(bz × c) + r24(bx × s))`.
    fn new_wind_target(&self) {
        let spread = self.d.borrow().wind_spread;
        let speed = self.rng.borrow_mut().range_f64(2.0, 4.0);
        let (bx, bz) = self.onshore_direction();
        let mut s = 0.0f64;
        for _ in 0..100 {
            let g = self.rng.borrow_mut().gaussian();
            let c = r24(g * f64::from(spread));
            if c.abs() <= 1.0 {
                s = c;
                break;
            }
        }
        let c = r24(r24(1.0 - r24(s * s)).sqrt());
        let (bx, bz) = (f64::from(bx), f64::from(bz));
        let mut d = self.d.borrow_mut();
        d.prev_wind_speed = d.target_wind_speed;
        d.prev_wind_dir = d.target_wind_dir;
        d.target_wind_speed = speed as f32;
        d.target_wind_dir = [
            r24(r24(bx * c) - r24(bz * s)) as f32,
            r24(r24(bz * c) + r24(bx * s)) as f32,
        ];
    }

    /// `SimulateCellTemperature(x, z)` (§6.3).
    pub fn simulate_cell_temperature(&self, x: i32, z: i32) {
        let altitude = self.host.altitude_at_vertex(x, z);
        let wind = self.wind_at_cell(x, z).0;
        let mut d = self.d.borrow_mut();
        let mut t = d.ambient_temp + 273.0;
        if get(&d.sea_mask, x, z).unwrap_or(0) != 0 {
            t += (d.base_temp - d.ambient_temp) * d.sea_weight;
        }
        t = t - altitude * d.height_factor - wind * d.wind_factor - 273.0;
        let t = if t < -15.0 { 0.0 } else { t.min(45.0) + 15.0 };
        let value = round_half_even_f32(t * 4.25).clamp(0.0, 255.0) as u8;
        let r = d.blend_radius;
        if r < 1 {
            put(&mut d.temperature, x, z, value);
            return;
        }
        let (nx, nz) = (d.count_x.clamp(0, MAX_CELLS), d.count_z.clamp(0, MAX_CELLS));
        let range = |c: i32, n: i32| {
            let lo = (i64::from(c) - i64::from(r)).max(0);
            let hi = (i64::from(c) + i64::from(r)).min(i64::from(n));
            (lo as i32, hi.max(lo) as i32)
        };
        let (i0, i1) = range(x, nx);
        let (j0, j1) = range(z, nz);
        for i in i0..i1 {
            for j in j0..j1 {
                if let Some(old) = get(&d.temperature, i, j) {
                    put(
                        &mut d.temperature,
                        i,
                        j,
                        ((u16::from(old) + u16::from(value)) / 2) as u8,
                    );
                }
            }
        }
    }

    /// `GenerateMoistureVariation` (§6.4).
    pub fn generate_moisture_variation(&self) {
        let (lo, hi) = self.host.water_prefs().unwrap_or((60, 100));
        let (lo, hi) = (i32::from(lo), i32::from(hi));
        let (nx, nz) = self.dims();
        let mut v = (lo + hi) >> 1;
        for x in 0..nx {
            for z in 0..nz {
                let p = if x > 0 {
                    let prev = get(&self.d.borrow().moisture_base, x - 1, z).unwrap_or(0);
                    (v + i32::from(prev)) >> 1
                } else {
                    v
                };
                let r = self.rng.borrow_mut().range_i32_gaussian_fast(-10, 10);
                let s = p.saturating_add(r);
                v = if s < lo {
                    lo
                } else if s > hi {
                    hi
                } else {
                    s
                };
                put(&mut self.d.borrow_mut().moisture_base, x, z, v.clamp(0, 255) as u8);
            }
        }
    }

    /// `SimulateMoisture(rect)` (§6.5), `[x0, x1) × [z0, z1)` in cells.
    pub fn simulate_moisture(&self, x0: i32, z0: i32, x1: i32, z1: i32) {
        let (nx, nz) = self.dims();
        let a = [x0.clamp(0, nx), z0.clamp(0, nz), x1.clamp(0, nx), z1.clamp(0, nz)];
        if a[0] >= a[2] || a[1] >= a[3] {
            self.post(msg::MOISTURE_MAP_UPDATED, 0);
            return;
        }
        let grow = |k: i32| {
            [
                (a[0] - k).max(0),
                (a[1] - k).max(0),
                (a[2] + k).min(nx),
                (a[3] + k).min(nz),
            ]
        };
        let (a2, a4) = (grow(2), grow(4));
        let n = (nx * nz) as usize;
        let at = |x: i32, z: i32| (x * nz + z) as usize;
        let h = &self.host;
        // 1. heights
        let mut hgt = vec![0f32; n];
        for x in a4[0]..a4[2] {
            for z in a4[1]..a4[3] {
                hgt[at(x, z)] = h.altitude_at_vertex(x, z);
            }
        }
        // 2. water and base pattern
        let mut m = vec![0i32; n];
        {
            let d = self.d.borrow();
            for x in a2[0]..a2[2] {
                for z in a2[1]..a2[3] {
                    m[at(x, z)] = if h.cell_has_water(x, z) {
                        255
                    } else {
                        i32::from(get(&d.moisture_base, x, z).unwrap_or(0))
                    };
                }
            }
        }
        // 3. smoothed heights (§6.5 step 3).
        let sm = smoothed_heights(&hgt, nx, nz, a4);
        // 4. local moisture
        let (ambient, high_m, threshold, slope_k, wind) = {
            let d = self.d.borrow();
            (
                d.ambient_moisture,
                d.high_moisture,
                d.curvature_threshold,
                d.slope_k,
                self.ambient_wind().1,
            )
        };
        let b = moisture_integer(ambient * 255.0);
        let high = moisture_integer(high_m * 255.0).saturating_add(b).clamp(0, 255);
        for x in a2[0]..a2[2] {
            for z in a2[1]..a2[3] {
                let i = at(x, z);
                if m[i] == 255 {
                    continue;
                }
                let s = sm[i];
                let xm = if x - 1 < 0 { x } else { x - 1 };
                let xp = if x + 1 >= nx { x } else { x + 1 };
                let zm = if z - 1 < 0 { z } else { z - 1 };
                let zp = if z + 1 >= nz { z } else { z + 1 };
                let cx = (sm[at(xm, z)] + sm[at(xp, z)]) - (s + s);
                let cz = (sm[at(x, zm)] + sm[at(x, zp)]) - (s + s);
                let add = if (cx >= 0.0 && cz >= 0.0) || hgt[i] <= s {
                    if cx + cz <= threshold || cx <= 0.0 || cz <= 0.0 {
                        b
                    } else {
                        high
                    }
                } else {
                    let nrm = h.vertex_normal(x, z);
                    let dot = (wind[1] * nrm[2] + wind[0] * nrm[0]) + nrm[1] * 0.0;
                    let v = b as f32 - dot * slope_k;
                    moisture_integer(v).clamp(0, 255)
                };
                m[i] = m[i].saturating_add(add).min(255);
            }
        }
        // 5. blur [1 4 6 4 1] / 16 along z (rows of A2), then along x (cells of A)
        const K: [i32; 5] = [1, 4, 6, 4, 1];
        let mut bz = vec![0i32; n];
        for x in a2[0]..a2[2] {
            for z in a[1]..a[3] {
                let sum: i32 = (0..5).map(|k| K[k] * m[at(x, reflect(z + k as i32 - 2, nz))]).sum();
                bz[at(x, z)] = (sum + 8) >> 4;
            }
        }
        {
            let mut d = self.d.borrow_mut();
            for x in a[0]..a[2] {
                for z in a[1]..a[3] {
                    let sum: i32 = (0..5).map(|k| K[k] * bz[at(reflect(x + k as i32 - 2, nx), z)]).sum();
                    put(&mut d.moisture, x, z, ((sum + 8) >> 4).clamp(0, 255) as u8);
                }
            }
        }
        // 6.
        self.post(msg::MOISTURE_MAP_UPDATED, 0);
    }

    /// January: the water rectangles at the city edge and the sea mask (§6.6).
    pub fn rebuild_water(&self) {
        let (nx, nz) = self.dims();
        let h = &self.host;
        let sea = h.sea_level();
        let at = |x: i32, z: i32| (z * nx + x) as usize;
        let water: Vec<bool> = (0..nz)
            .flat_map(|z| (0..nx).map(move |x| (x, z)))
            .map(|(x, z)| h.altitude(x as f32 * 16.0, z as f32 * 16.0) <= sea)
            .collect();
        let mut used = vec![false; (nx * nz) as usize];
        let mut rects = Vec::new();
        for z in 0..nz {
            for x in 0..nx {
                let free = |used: &Vec<bool>, x: i32, z: i32| water[at(x, z)] && !used[at(x, z)];
                if !free(&used, x, z) {
                    continue;
                }
                let mut e = x;
                while e < nx && free(&used, e, z) {
                    e += 1;
                }
                if e - x < 3 {
                    continue;
                }
                let (mut a, mut b) = (x, e);
                let mut zend = z + 1;
                while zend < nz {
                    let (mut na, mut nb) = (a, b);
                    while na < nb && !free(&used, na, zend) {
                        na += 1;
                    }
                    while nb > na && !free(&used, nb - 1, zend) {
                        nb -= 1;
                    }
                    if nb - na < 3 {
                        break;
                    }
                    a = na;
                    b = nb;
                    zend += 1;
                }
                let (w, ht) = (b - a, zend - z);
                if w.min(ht) >= 3 && w.max(ht) >= 5 {
                    for i in a..b {
                        for j in z..zend {
                            used[at(i, j)] = true;
                        }
                    }
                    rects.push([a, z, b, zend]);
                }
            }
        }
        rects.retain(|r| r[0] < 1 || r[1] < 1 || r[2] >= nx - 1 || r[3] >= nz - 1);
        let mut d = self.d.borrow_mut();
        let radius = i64::from(d.sea_radius);
        d.sea_mask = filled(nx, nz, 0);
        for r in &rects {
            let lo_x = (i64::from(r[0]) - radius).max(0);
            let hi_x = (i64::from(r[2]) + radius).min(i64::from(nx) - 1);
            let lo_z = (i64::from(r[1]) - radius).max(0);
            let hi_z = (i64::from(r[3]) + radius).min(i64::from(nz) - 1);
            if lo_x > hi_x || lo_z > hi_z {
                put(&mut d.sea_mask, 0, 0, 1);
                continue;
            }
            for i in lo_x..=hi_x {
                for j in lo_z..=hi_z {
                    put(&mut d.sea_mask, i as i32, j as i32, 1);
                }
            }
        }
        d.water_rects = rects;
    }
}

// ---- persistence (§10) --------------------------------------------------------------------------------

fn write_grid(out: &mut dyn OutStream, g: &Grid) -> bool {
    let rows = g.len();
    let cols = g.first().map_or(0, Vec::len);
    if rows != cols || g.iter().any(|r| r.len() != cols) {
        return false; // [choice] square grids only
    }
    out.put_u32(rows as u32);
    out.put_u32(cols as u32);
    for r in g {
        out.put_bytes(r);
    }
    out.error() == 0
}

fn read_grid(inp: &mut dyn InStream) -> Option<Grid> {
    let rows = inp.get_u32()?;
    let cols = inp.get_u32()?;
    if rows >= GRID_SKIP || cols >= GRID_SKIP {
        return Some(Vec::new());
    }
    let mut g = Vec::new();
    for _ in 0..rows {
        g.push(inp.get_bytes(cols as usize)?);
    }
    Some(g)
}

fn put_vec2(out: &mut dyn OutStream, v: [f32; 2]) {
    out.put_f32(v[0]);
    out.put_f32(v[1]);
}

fn get_vec2(inp: &mut dyn InStream) -> Option<[f32; 2]> {
    Some([inp.get_f32()?, inp.get_f32()?])
}

impl WeatherSim {
    fn read_record(&self, inp: &mut dyn InStream, version: u16) -> Option<(WeatherData, RngState)> {
        let mut d = self.d.borrow().clone();
        d.count_x = inp.get_i32()?;
        d.count_z = inp.get_i32()?;
        d.cell_width_x = inp.get_f32()?;
        d.cell_width_z = inp.get_f32()?;
        d.inv_cell_width_x = inp.get_f32()?;
        d.inv_cell_width_z = inp.get_f32()?;
        d.size_x = inp.get_f32()?;
        d.size_z = inp.get_f32()?;
        d.city_cells_per_cell_x = inp.get_i32()?;
        d.city_cells_per_cell_z = inp.get_i32()?;
        d.subdiv_x = inp.get_i32()?;
        d.subdiv_z = inp.get_i32()?;
        d.blend_radius = inp.get_i32()?;
        d.last_season = inp.get_i32()?;
        d.base_temp = inp.get_f32()?;
        d.temp_amp_a = inp.get_f32()?;
        d.temp_amp_b = inp.get_f32()?;
        d.ambient_temp = inp.get_f32()?;
        d.temperature = read_grid(inp)?;
        d.sea_mask = read_grid(inp)?;
        let n = inp.get_u32()?;
        d.water_rects.clear();
        for _ in 0..n {
            d.water_rects
                .push([inp.get_i32()?, inp.get_i32()?, inp.get_i32()?, inp.get_i32()?]);
        }
        d.iterator = CellIterator::read(inp)?;
        d.cells_per_week = inp.get_i32()?;
        d.sea_radius = inp.get_i32()?;
        d.sea_weight = inp.get_f32()?;
        d.height_factor = inp.get_f32()?;
        d.wind_factor = inp.get_f32()?;
        d.moisture = read_grid(inp)?;
        d.moisture_base = read_grid(inp)?;
        d.ambient_moisture = inp.get_f32()?;
        d.base_moisture = inp.get_f32()?;
        d.moist_amp_a = inp.get_f32()?;
        d.moist_amp_b = inp.get_f32()?;
        d.slope_k = inp.get_f32()?;
        d.high_moisture = inp.get_f32()?;
        d.curvature_threshold = inp.get_f32()?;
        d.tuning80 = inp.get_u32()?;
        d.default_wind_speed = inp.get_f32()?;
        d.default_wind_dir = get_vec2(inp)?;
        d.wind_speed = inp.get_f32()?;
        d.wind_dir = get_vec2(inp)?;
        d.prev_wind_speed = inp.get_f32()?;
        d.prev_wind_dir = get_vec2(inp)?;
        d.target_wind_speed = inp.get_f32()?;
        d.target_wind_dir = get_vec2(inp)?;
        if version < 2 {
            // Legacy block: n × (2 × f32, 2 × f32) and two byte grids, discarded.
            let n = inp.get_u32()?;
            for _ in 0..n {
                for _ in 0..4 {
                    inp.get_f32()?;
                }
            }
            read_grid(inp)?;
            read_grid(inp)?;
        }
        let rng = RngState {
            lcg: inp.get_u32()?,
            mul: inp.get_u32()?,
            has_cached_gaussian: inp.get_i32()?,
            cached_gaussian: inp.get_f64()?,
        };
        d.wind_month = inp.get_i32()?;
        Some((d, rng))
    }
}

impl SaveObject for WeatherSim {
    fn class_id(&self) -> u32 {
        CLSID_WEATHER_SIMULATOR
    }

    fn write(&self, out: &mut dyn OutStream) -> bool {
        if out.error() != 0 {
            return false;
        }
        let d = self.d.borrow();
        out.put_u16(WEATHER_RECORD_VERSION);
        out.put_i32(d.count_x);
        out.put_i32(d.count_z);
        for v in [
            d.cell_width_x,
            d.cell_width_z,
            d.inv_cell_width_x,
            d.inv_cell_width_z,
            d.size_x,
            d.size_z,
        ] {
            out.put_f32(v);
        }
        for v in [
            d.city_cells_per_cell_x,
            d.city_cells_per_cell_z,
            d.subdiv_x,
            d.subdiv_z,
            d.blend_radius,
            d.last_season,
        ] {
            out.put_i32(v);
        }
        for v in [d.base_temp, d.temp_amp_a, d.temp_amp_b, d.ambient_temp] {
            out.put_f32(v);
        }
        if !write_grid(out, &d.temperature) || !write_grid(out, &d.sea_mask) {
            return false;
        }
        out.put_u32(d.water_rects.len() as u32);
        for r in &d.water_rects {
            for &v in r {
                out.put_i32(v);
            }
        }
        if !d.iterator.write(out) {
            return false;
        }
        out.put_i32(d.cells_per_week);
        out.put_i32(d.sea_radius);
        for v in [d.sea_weight, d.height_factor, d.wind_factor] {
            out.put_f32(v);
        }
        if !write_grid(out, &d.moisture) || !write_grid(out, &d.moisture_base) {
            return false;
        }
        for v in [
            d.ambient_moisture,
            d.base_moisture,
            d.moist_amp_a,
            d.moist_amp_b,
            d.slope_k,
            d.high_moisture,
            d.curvature_threshold,
        ] {
            out.put_f32(v);
        }
        out.put_u32(d.tuning80);
        out.put_f32(d.default_wind_speed);
        put_vec2(out, d.default_wind_dir);
        out.put_f32(d.wind_speed);
        put_vec2(out, d.wind_dir);
        out.put_f32(d.prev_wind_speed);
        put_vec2(out, d.prev_wind_dir);
        out.put_f32(d.target_wind_speed);
        put_vec2(out, d.target_wind_dir);
        let rng = self.rng.borrow().state();
        out.put_u32(rng.lcg);
        out.put_u32(rng.mul);
        out.put_i32(rng.has_cached_gaussian);
        out.put_f64(rng.cached_gaussian);
        out.put_i32(d.wind_month);
        out.error() == 0
    }

    /// Sets `loadedFromSave` first; rejects versions ≥ 3 (§10).
    fn read(&self, inp: &mut dyn InStream) -> bool {
        self.d.borrow_mut().loaded_from_save = true;
        if inp.error() != 0 {
            return false;
        }
        let Some(version) = inp.get_u16() else { return false };
        if version >= 3 {
            return false;
        }
        let Some((d, rng)) = self.read_record(inp, version) else {
            return false;
        };
        *self.d.borrow_mut() = d;
        self.rng.borrow_mut().set_state(rng);
        inp.error() == 0
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Weather test 12 (§6.5 step 3): pass 2 reads the pass-1 values. A
    /// height of 8 at (1, 1) of a 4 × 3 grid gives the pass-1 row
    /// `S = [0, 16, 0, 0]` along x at z = 1; the two interior cells become
    /// `(0 + 0 + 32) / 16 = 2` and `(16 + 0 + 0) / 16 = 1` (an in-place
    /// filter would give 0.125 for the second).
    #[test]
    fn pass_two_reads_pass_one() {
        let (nx, nz) = (4, 3);
        let mut hgt = vec![0f32; 12];
        hgt[(nz + 1) as usize] = 8.0;
        let sm = smoothed_heights(&hgt, nx, nz, [0, 0, nx, nz]);
        let at = |x: i32, z: i32| (x * nz + z) as usize;
        assert_eq!((sm[at(1, 1)], sm[at(2, 1)]), (2.0, 1.0));
    }
    use crate::rng::FixedSeeds;
    use crate::stream::{ByteReader, ByteWriter};
    use crate::weather::WeatherRng;
    use std::collections::BTreeMap;

    /// A minimal city: square, flat unless heights are given.
    struct Fake {
        size: Cell<(f32, f32)>,
        month: Cell<i32>,
        hemisphere: Cell<i32>,
        tuning: RefCell<Option<Rc<PropertyHolder>>>,
        heights: RefCell<BTreeMap<(i32, i32), f32>>,
        water: RefCell<Vec<(i32, i32)>>,
        posts: RefCell<Vec<(u32, u32)>>,
    }

    impl Fake {
        fn new(cells: f32) -> Rc<Fake> {
            Rc::new(Fake {
                size: Cell::new((cells * 16.0, cells * 16.0)),
                month: Cell::new(1),
                hemisphere: Cell::new(1),
                tuning: RefCell::new(Some(Rc::new(shipped_tuning()))),
                heights: RefCell::new(BTreeMap::new()),
                water: RefCell::new(Vec::new()),
                posts: RefCell::new(Vec::new()),
            })
        }
        fn posted(&self, m: u32) -> usize {
            self.posts.borrow().iter().filter(|p| p.0 == m).count()
        }
    }

    impl WeatherHost for Fake {
        fn city_size(&self) -> (f32, f32) {
            self.size.get()
        }
        fn city_cell_width(&self) -> (f32, f32) {
            (16.0, 16.0)
        }
        fn world_hemisphere(&self) -> i32 {
            self.hemisphere.get()
        }
        fn sim_month(&self) -> i32 {
            self.month.get()
        }
        fn water_prefs(&self) -> Option<(u8, u8)> {
            None
        }
        fn tuning(&self) -> Option<Rc<PropertyHolder>> {
            self.tuning.borrow().clone()
        }
        fn terrain_available(&self) -> bool {
            true
        }
        fn altitude_at_vertex(&self, x: i32, z: i32) -> f32 {
            self.heights.borrow().get(&(x, z)).copied().unwrap_or(0.0)
        }
        fn vertex_normal(&self, _x: i32, _z: i32) -> [f32; 3] {
            [0.0, 1.0, 0.0]
        }
        fn cell_has_water(&self, x: i32, z: i32) -> bool {
            self.water.borrow().contains(&(x, z))
        }
        fn altitude(&self, x: f32, z: f32) -> f32 {
            self.altitude_at_vertex((x / 16.0).floor() as i32, (z / 16.0).floor() as i32)
        }
        fn sea_level(&self) -> f32 {
            -1.0
        }
        fn post(&self, msg_type: u32, data2: u32) {
            self.posts.borrow_mut().push((msg_type, data2));
        }
    }

    /// A scripted generator: uniform 0.5, Gaussian 0, integers 0.
    struct Flat;

    impl WeatherRandom for Flat {
        fn range_f64(&mut self, a: f64, b: f64) -> f64 {
            a + 0.5 * (b - a)
        }
        fn gaussian(&mut self) -> f64 {
            0.0
        }
        fn range_i32_gaussian_fast(&mut self, _lo: i32, _hi: i32) -> i32 {
            0
        }
        fn state(&self) -> RngState {
            RngState::default()
        }
        fn set_state(&mut self, _s: RngState) {}
    }

    fn seeds() -> FixedSeeds {
        FixedSeeds {
            wall_clock: 5,
            process_clock: 1,
        }
    }

    fn sim(f: &Rc<Fake>) -> Rc<WeatherSim> {
        let s = WeatherSim::new(f.clone() as Rc<dyn WeatherHost>, 1, Box::new(Flat));
        assert!(s.init());
        s
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-5 * b.abs().max(1.0)
    }

    fn save(s: &WeatherSim) -> Vec<u8> {
        let mut w = ByteWriter::new();
        assert!(s.write(&mut w));
        w.into_bytes()
    }

    #[test]
    fn ambient_values_follow_the_season_month() {
        let f = Fake::new(16.0);
        let s = sim(&f);
        assert_eq!(s.snapshot().ambient_temp, 65.0);
        for (m, t) in [(8, 81.25), (2, 48.75)] {
            s.new_current_ambient_values(m);
            assert!(close(s.snapshot().ambient_temp, t));
        }
        for (m, h) in [(1, 0.24), (7, 0.16)] {
            s.new_current_ambient_values(m);
            assert!(close(s.snapshot().ambient_moisture, h));
        }
        *f.tuning.borrow_mut() = None;
        let s = sim(&f);
        s.new_current_ambient_values(8);
        assert!(close(s.snapshot().ambient_temp, 30.0));
        assert_eq!(s.snapshot().cells_per_week, 32);
    }

    #[test]
    fn seasons_shift_with_the_hemisphere_and_post_on_change() {
        let f = Fake::new(4.0);
        let s = sim(&f);
        for (m, season) in [(1, 0), (3, 0), (4, 1), (6, 1), (7, 2), (9, 2), (10, 3), (12, 3)] {
            f.month.set(m);
            assert_eq!(s.current_season(), season);
        }
        f.hemisphere.set(0);
        for (m, sm) in [(1, 7), (7, 1), (12, 6)] {
            f.month.set(m);
            assert_eq!(s.current_season_month(), sm);
        }
        f.hemisphere.set(1);
        f.month.set(1);
        s.data_mut().wind_speed = 7.0;
        s.do_monthly_simulation(1);
        s.do_monthly_simulation(2);
        assert_eq!(f.posted(msg::SEASON_CHANGED), 1);
        assert_eq!(s.snapshot().wind_speed, 3.0);
        f.month.set(4);
        s.do_monthly_simulation(4);
        assert_eq!(*f.posts.borrow().last().unwrap(), (msg::SEASON_CHANGED, 1));
    }

    #[test]
    fn wind_target_and_daily_interpolation() {
        let f = Fake::new(8.0);
        let s = sim(&f);
        f.water.borrow_mut().extend((1..6).map(|x| (x, 0)));
        assert_eq!(s.onshore_direction(), (0, 1));
        s.new_current_ambient_values(3);
        let d = s.snapshot();
        assert_eq!(
            (d.target_wind_speed, d.target_wind_dir, d.wind_month),
            (3.0, [0.0, 1.0], 3)
        );
        {
            let mut d = s.data_mut();
            d.prev_wind_speed = 2.0;
            d.prev_wind_dir = [1.0, 0.0];
            d.target_wind_speed = 4.0;
        }
        s.do_daily(crate::julian::julian_from_civil(2001, 3, 16));
        let d = s.snapshot();
        assert!(close(d.wind_speed, 3.0));
        assert!(close(d.wind_dir[0], 0.4472136) && close(d.wind_dir[1], 0.8944272));
        s.do_daily(crate::julian::julian_from_civil(2001, 4, 1));
        assert!(close(s.snapshot().wind_speed, 3.0));
    }

    #[test]
    fn temperature_bytes() {
        let f = Fake::new(8.0);
        f.heights
            .borrow_mut()
            .extend([((1, 1), 400.0), ((3, 3), 1000.0), ((4, 4), 400.0)]);
        let s = sim(&f);
        {
            let mut d = s.data_mut();
            d.blend_radius = 0;
            d.ambient_temp = 48.75;
            d.wind_speed = 3.0;
            d.sea_mask[4][4] = 1;
        }
        for c in 1..5 {
            s.simulate_cell_temperature(c, c);
        }
        let t = s.snapshot().temperature;
        assert_eq!((t[1][1], t[2][2], t[3][3], t[4][4]), (128, 255, 0, 191));
        s.data_mut().blend_radius = 1;
        s.data_mut().temperature = vec![vec![100; 8]; 8];
        s.simulate_cell_temperature(1, 1);
        let t = s.snapshot().temperature;
        assert_eq!((t[0][0], t[0][1], t[1][0], t[1][1], t[2][2]), (114, 114, 114, 114, 100));
    }

    #[test]
    fn moisture_on_flat_land_and_near_water() {
        let f = Fake::new(16.0);
        let s = sim(&f);
        assert!(s.snapshot().moisture_base.iter().flatten().all(|&b| b == 80));
        s.simulate_moisture(0, 0, 16, 16);
        assert!(s.snapshot().moisture.iter().flatten().all(|&b| b == 131));
        assert_eq!(f.posted(msg::MOISTURE_MAP_UPDATED), 1);
        f.water.borrow_mut().push((5, 5));
        s.simulate_moisture(0, 0, 16, 16);
        let m = s.snapshot().moisture;
        assert_eq!((m[5][5], m[4][5], m[4][4], m[3][3], m[8][8]), (149, 143, 139, 132, 131));
        f.water.borrow_mut().clear();
        s.data_mut().moisture = vec![vec![64; 16]; 16];
        s.simulate_moisture(4, 4, 8, 8);
        let m = s.snapshot().moisture;
        for (x, row) in m.iter().enumerate() {
            for (z, &v) in row.iter().enumerate() {
                let inside = (4..8).contains(&x) && (4..8).contains(&z);
                assert_eq!(v, if inside { 131 } else { 64 }, "({x}, {z})");
            }
        }
    }

    #[test]
    fn weekly_passes_and_complete_simulation() {
        let f = Fake::new(16.0);
        let s = sim(&f);
        assert_eq!(s.snapshot().iterator.step, 179);
        assert!(!s.do_weekly_simulation());
        assert!(s.do_weekly_simulation());
        assert_eq!(f.posted(msg::WEATHER_CYCLE_COMPLETED), 1);
        let s = sim(&f);
        f.posts.borrow_mut().clear();
        assert!(s.do_post_city_init());
        assert!(s.do_post_city_init());
        assert_eq!(f.posted(msg::WEATHER_CYCLE_COMPLETED), 1);
        assert_eq!(f.posted(msg::MOISTURE_MAP_UPDATED), 2);
        // A hostile cellsPerWeek is capped at one grid.
        s.data_mut().cells_per_week = i32::MAX;
        s.do_weekly_simulation();
    }

    #[test]
    fn save_round_trip_and_versions() {
        let f = Fake::new(16.0);
        f.heights
            .borrow_mut()
            .extend((0..4).flat_map(|x| (0..8).map(move |z| ((x, z), -5.0))));
        let s = WeatherSim::new(
            f.clone() as Rc<dyn WeatherHost>,
            1,
            Box::new(WeatherRng::with_seed(7, &seeds())),
        );
        assert!(s.init());
        s.do_monthly_simulation(1);
        assert_eq!(s.snapshot().water_rects, vec![[0, 0, 4, 8]]);
        let bytes = save(&s);
        assert_eq!(bytes.len(), 1313);
        assert_eq!(&bytes[..2], &[2, 0]);
        let fresh = || WeatherSim::new(f.clone() as Rc<dyn WeatherHost>, 2, Box::new(WeatherRng::new(&seeds())));
        let l = fresh();
        assert!(l.read(&mut ByteReader::new(&bytes)));
        assert!(l.snapshot().loaded_from_save);
        assert_eq!(save(&l), bytes);
        assert_eq!(l.rng_state(), s.rng_state());
        // Same draws after Init re-reads windSpread.
        assert!(l.init());
        s.new_current_ambient_values(2);
        l.new_current_ambient_values(2);
        assert_eq!(l.snapshot().target_wind_dir, s.snapshot().target_wind_dir);
        assert_eq!(l.snapshot().ambient_temp, s.snapshot().ambient_temp);
        // Version 3 is rejected; a truncated record fails.
        let mut v3 = bytes.clone();
        v3[0] = 3;
        let l = fresh();
        assert!(!l.read(&mut ByteReader::new(&v3)));
        assert!(l.snapshot().loaded_from_save);
        assert_eq!(l.snapshot().count_x, 0);
        assert!(!fresh().read(&mut ByteReader::new(&bytes[..bytes.len() - 1])));
        // Version 1 with a legacy block.
        let mut v1 = bytes.clone();
        v1[0] = 1;
        let mut legacy = vec![1, 0, 0, 0];
        legacy.extend([0u8; 16]);
        legacy.extend([2, 0, 0, 0, 2, 0, 0, 0, 1, 2, 3, 4, 0, 0, 0, 0, 0, 0, 0, 0]);
        v1.splice(1289..1289, legacy);
        let l = fresh();
        assert!(l.read(&mut ByteReader::new(&v1)));
        assert_eq!(save(&l), bytes);
    }

    #[test]
    fn hostile_inputs_do_not_panic() {
        for size in [(-5.0, 64.0), (f32::NAN, 16.0), (1e9, 16.0), (0.0, 0.0), (17.0, 15.0)] {
            let f = Fake::new(1.0);
            f.size.set(size);
            let s = sim(&f);
            assert!(s.snapshot().count_x <= MAX_CELLS);
            s.do_post_city_init();
            s.do_monthly_simulation(1);
            s.do_weekly_simulation();
            s.do_daily(i32::MIN);
            s.do_daily(i32::MAX);
            s.simulate_cell_temperature(i32::MIN, i32::MAX);
            s.simulate_moisture(i32::MIN, i32::MIN, i32::MAX, i32::MAX);
            s.update_moisture(i32::MIN, i32::MIN, i32::MAX, i32::MAX);
            s.set_manual_moisture_addition(f32::NAN, -1e30, 1e30, f32::INFINITY, f32::NAN);
            let _ = s.moisture_value(f32::NAN, f32::INFINITY);
            let _ = s.position_to_cell(f32::NAN, -1e30);
            let _ = s.cell_to_standard_city_cell(i32::MAX, 0);
            let _ = s.standard_city_cell_to_cell(i32::MIN, -1);
            s.new_current_ambient_values(i32::MIN);
            {
                let mut d = s.data_mut();
                d.blend_radius = i32::MAX;
                d.sea_radius = i32::MIN;
                d.temperature = vec![vec![1; 3]];
                d.moisture.clear();
            }
            s.simulate_cell_temperature(0, 0);
            s.do_monthly_simulation(1);
            s.simulate_moisture(0, 0, 4, 4);
            s.do_weekly_simulation();
            // Non-square grids are refused on write [choice].
            let _ = s.write(&mut ByteWriter::new());
        }
        // Garbage records fail or load without panicking.
        let f = Fake::new(4.0);
        let s = sim(&f);
        let bytes = save(&s);
        for cut in 0..bytes.len() {
            let _ = WeatherSim::new(f.clone() as Rc<dyn WeatherHost>, 3, Box::new(Flat))
                .read(&mut ByteReader::new(&bytes[..cut]));
        }
        let mut huge = bytes.clone();
        huge[74..78].copy_from_slice(&u32::MAX.to_le_bytes());
        let l = WeatherSim::new(f.clone() as Rc<dyn WeatherHost>, 3, Box::new(Flat));
        let _ = l.read(&mut ByteReader::new(&huge));
        l.init();
        l.do_weekly_simulation();
        let _ = l.moisture_value(5.0, 5.0);
    }
}
