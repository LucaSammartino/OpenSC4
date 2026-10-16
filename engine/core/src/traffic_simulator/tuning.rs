// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `ReadSimulatorTunableValues` (§4.14) and the travel strategy percentages
//! (§4.1.2).

use crate::curve::ResponseCurve;
use crate::property::PropertyHolder;
use crate::props;
use crate::traffic_network::Tuning as NetTuning;
use crate::variant::{mem, Elem, Variant};
use crate::x87::r24;

/// The tuning exemplar's key (§4.14): type, group, instance.
pub const EXEMPLAR_TYPE: u32 = 0x6534_284A;
pub const EXEMPLAR_GROUP: u32 = 0xE7E2_C2DB;
pub const EXEMPLAR_INSTANCE: u32 = 0xC913_3286;

/// The tuning property IDs (§4.14).
pub mod id {
    pub const VOLUME_PER_POPULATION: u32 = 0x4913_32E6;
    /// `MAX_SPEED + t`, t = 0…8: 13 floats each.
    pub const MAX_SPEED: u32 = 0x4913_32E7;
    pub const DENSITY_REDUCTION: u32 = 0x2913_6788;
    pub const UNUSED_29136789: u32 = 0x2913_6789;
    pub const TILE_COSTS: u32 = 0x6A84_493E;
    pub const MAX_ROADS_FUNDING: u32 = 0xA923_56AE;
    pub const MAX_TRANSIT_FUNDING: u32 = 0xA923_56AF;
    pub const DAMAGED_ROAD_COST: u32 = 0xA923_56B0;
    pub const STRATEGY_PERIOD: u32 = 0xA923_56B6;
    pub const STRATEGY_BUCKETS: u32 = 0xA923_56B7;
    pub const MAX_DISTANCE_ORIGIN: u32 = 0xA923_56B8;
    pub const INCOME_PER_TILE: u32 = 0xA923_56B1;
    pub const CAPACITY: u32 = 0xA923_56B3;
    pub const GENERATES_TRAFFIC: u32 = 0xA923_56B4;
    pub const AFFECTED_BY_TRAFFIC: u32 = 0xA923_56BF;
    pub const CAN_REACH: u32 = 0xA923_56B5;
    pub const CONGESTION_CURVE: u32 = 0xA923_56B9;
    pub const COMMUTE_MAX_TIME: u32 = 0xA923_56BA;
    pub const INTERSECTION: u32 = 0xA923_56BB;
    pub const START_NEUTRAL: u32 = 0xA923_56BC;
    pub const START_MASS_TRANSIT: u32 = 0xEA8C_3CDB;
    pub const START_CAR: u32 = 0xCAD6_4136;
    pub const JOB_SCALING: u32 = 0xA923_56BD;
    pub const FREIGHT_SCALING: u32 = 0x49A2_E8BE;
    pub const NEAREST_ATTRACTIVENESS: u32 = 0x4A67_8060;
    pub const TRIP_DISPLAY_MULTIPLIER: u32 = 0xCA76_013B;
    pub const MAX_MASS_TRANSIT_TRIP: u32 = 0xEA7B_5F06;
    pub const BACKGROUND: u32 = 0xA923_56BE;
    pub const SWITCH_CURVE: u32 = 0xCA5F_7821;
    pub const ACCIDENT_CONGESTION_CURVE: u32 = 0x4BE0_9EC9;
    pub const ACCIDENT_CAPACITY_CURVE: u32 = 0x4BE0_9ECA;
    pub const ACCIDENT_DURATION: u32 = 0x4BE0_9ECB;
    pub const ACCIDENT_CHECK_PERIOD: u32 = 0x4BE0_9ECC;
    pub const DAMAGE_CURVE: u32 = 0x0BD8_6FD4;
    pub const RAIL_DAMAGE_ACCIDENT_FACTOR: u32 = 0x0BD8_6FD5;
    pub const FERRY_FARE: u32 = 0x8C3C_AEF5;
    pub const COMMERCIAL_SCALE: u32 = 0x0CAA_AEB8;
    /// `STRATEGY_PCT + w`, w = 0…3: the first 3 bytes.
    pub const STRATEGY_PCT: u32 = 0x4953_E8A3;
}

/// The default `trafficUnitsPerSlice` (§3.4) *[choice: 16]*. The original
/// has no such constant (TS27): it slices by a 4 ms wall-clock deadline
/// tested once per unit, so this stays FreeCity4's choice until the §12
/// counters calibrate it.
pub const DEFAULT_UNITS_PER_SLICE: u32 = 16;

/// The tuning values (§4.14) with the constructor's code defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct SimTuning {
    /// `0x491332E6`, read and unused.
    pub volume_per_population: f32,
    /// The raw "Max speed by Network" values `[t][networkType]` (0 = none).
    pub max_speed: [[f32; 13]; 9],
    /// `densityReduction` (uninitialised in the original; FreeCity4 0).
    pub density_reduction: f32,
    /// `0x29136789`, read and unused.
    pub unused_29136789: f32,
    /// The monthly cost per network tile.
    pub tile_costs: [f32; 13],
    /// The maximum funding fractions (1.2 = 120 %).
    pub max_roads_funding: f32,
    pub max_transit_funding: f32,
    pub strategy_period: u32,
    pub strategy_buckets: u32,
    pub max_distance_origin: u32,
    pub income_per_tile: [f32; 9],
    pub generates_traffic: [bool; 9],
    pub affected_by_traffic: [bool; 9],
    pub can_reach: [bool; 9],
    /// The inverted congestion curve (class static `0x00B4B2F0`).
    pub congestion_curve: ResponseCurve,
    pub congestion_scale: f32,
    pub commute_max_time: f32,
    /// The trip starting costs: neutral (also the default array), mass
    /// transit, car preference.
    pub start_neutral: [f32; 9],
    pub start_mass_transit: [f32; 9],
    pub start_car: [f32; 9],
    pub job_scaling: f32,
    /// Uninitialised in the original; FreeCity4 0.
    pub freight_scaling: f32,
    pub nearest_attractiveness: f32,
    pub trip_display_multiplier: f32,
    pub max_mass_transit_trip_length: f32,
    /// `bg[0…2]`; missing entries 1.0.
    pub background: [f32; 3],
    pub switch_curve: ResponseCurve,
    pub accident_congestion_curve: ResponseCurve,
    pub accident_capacity_curve: ResponseCurve,
    pub accident_duration: f32,
    pub accident_check_period: f32,
    pub damage_curve: ResponseCurve,
    pub rail_damage_accident_factor: f32,
    pub ferry_fare: f32,
    pub commercial_scale: f32,
    /// The tract size of the air-polluting and peak-volume grids (code
    /// default 1, not read from tuning).
    pub tract_size: i32,
    /// `trafficUnitsPerSlice` (§3.4).
    pub traffic_units_per_slice: u32,
    /// `pct[w][s]` (§4.1.2).
    pub pct: [[u8; 3]; 4],
    /// The network part of the tuning (traffic-network.md §4.5).
    pub network: NetTuning,
}

fn speed_one() -> ResponseCurve {
    ResponseCurve::from_points(&[(0.0, 1.0)])
}

impl Default for SimTuning {
    /// The constructor's code defaults (§4.14). The congestion curve has no
    /// code default: FreeCity4 starts with "speed factor 1" and scale 255
    /// *[choice]*.
    fn default() -> Self {
        SimTuning {
            volume_per_population: 0.0,
            max_speed: [[0.0; 13]; 9],
            density_reduction: 0.0,
            unused_29136789: 0.031_25,
            tile_costs: [0.0; 13],
            max_roads_funding: 1.2,
            max_transit_funding: 1.2,
            strategy_period: 365,
            strategy_buckets: 128,
            max_distance_origin: 1,
            income_per_tile: [0.0; 9],
            generates_traffic: [true; 9],
            affected_by_traffic: [true; 9],
            can_reach: [true; 9],
            congestion_curve: speed_one(),
            congestion_scale: 255.0,
            commute_max_time: 90.0,
            start_neutral: [0.0; 9],
            start_mass_transit: [0.0; 9],
            start_car: [0.0; 9],
            job_scaling: 1.0,
            freight_scaling: 0.0,
            nearest_attractiveness: 0.75,
            trip_display_multiplier: 0.5,
            max_mass_transit_trip_length: 4.0,
            background: [1.0; 3],
            switch_curve: ResponseCurve::from_points(&[(0.0, 0.0), (1.0, 1.0)]),
            accident_congestion_curve: ResponseCurve::from_points(&[(0.0, 0.0), (0.25, 0.0), (1.0, 0.05), (2.0, 0.25)]),
            accident_capacity_curve: ResponseCurve::from_points(&[(0.0, 1.0), (1000.0, 1.0)]),
            accident_duration: 10.0,
            accident_check_period: 60.0,
            damage_curve: ResponseCurve::from_points(&[(0.0, 0.02), (100.0, -0.01), (120.0, -0.04)]),
            rail_damage_accident_factor: 1.0,
            ferry_fare: 0.001,
            commercial_scale: 0.128,
            tract_size: 1,
            traffic_units_per_slice: DEFAULT_UNITS_PER_SLICE,
            pct: [[0; 3]; 4],
            network: NetTuning {
                damaged_road_cost: 32.0,
                ..NetTuning::default()
            },
        }
    }
}

/// Copies the first `min(count, N)` floats of a property over `dst` and
/// sets the rest to `fill` (§4.14 "Short and missing arrays", TS28). A
/// missing property leaves `dst` unchanged.
fn read_f32s(h: &PropertyHolder, id: u32, dst: &mut [f32], fill: f32) {
    if let Some(v) = props::f32s(h, id) {
        for (i, d) in dst.iter_mut().enumerate() {
            *d = v.get(i).copied().unwrap_or(fill);
        }
    }
}

/// As [`read_f32s`] for bools (any non-zero value is true); the rest are
/// true (TS28).
fn read_bools(h: &PropertyHolder, id: u32, dst: &mut [bool]) {
    if let Some(v) = props::i64s(h, id) {
        for (i, d) in dst.iter_mut().enumerate() {
            *d = v.get(i).map_or(true, |&s| s != 0);
        }
    }
}

/// The f32 `0.01f` of the percent getter (`0x3C23D70A`, `0x00A823A4`).
const PERCENT: f32 = 0.01;

/// The percent getter `0x005FDAC0` (§4.14 "Funding percents", TS29): any
/// numeric property type (u8, s8, u16, s16, u32 as unsigned, s32, 64-bit,
/// f32, f64) is **multiplied** by `0.01f` and rounded once to f32,
/// `r24(v × 0.01f)`. `None` for a missing property or one of another type
/// (bool, string), which leave the target unchanged: the getter returns
/// without a store, so the code default 1.2 stays (TS37; missing
/// `0x005FDAD6` → `0x005FDCBE`, other types `0x005FDAF3` → `0x005FDCBD`).
fn percent_fraction(h: &PropertyHolder, id: u32) -> Option<f32> {
    let e = match h.get(id)? {
        Variant::Scalar(e) => *e,
        Variant::Array { items, .. } => *items.first()?,
        _ => return None,
    };
    let i = match e {
        Elem::F32(v) => return Some(r24(f64::from(v) * f64::from(PERCENT)) as f32),
        Elem::F64(v) => return Some(mul_f64_f32(v, PERCENT)),
        Elem::U8(_) | Elem::I8(_) | Elem::U16(_) | Elem::I16(_) | Elem::U32(_) | Elem::I32(_) => e.as_i64(),
        Elem::U64(v) => v as i64,
        Elem::I64(v) => v,
        Elem::Bool(_) | Elem::Char(_) | Elem::UnicodeChar(_) | Elem::RzChar(_) | Elem::VoidPtr(_) => return None,
    };
    Some(crate::x87::mul_i64_f32(i, PERCENT))
}

/// `fld qword` then `fmul dword` under 24-bit precision: the f64 enters
/// exactly and the product is rounded once to f32.
fn mul_f64_f32(v: f64, f: f32) -> f32 {
    if !v.is_finite() || v == 0.0 {
        return (v * f64::from(f)) as f32;
    }
    // v = m · 2^e with |m| < 2^53, so the integer product is exact.
    let bits = v.to_bits();
    let exp = ((bits >> 52) & 0x7FF) as i32;
    let frac = (bits & 0xF_FFFF_FFFF_FFFF) as i64;
    let (m, e) = if exp == 0 {
        (frac, -1074)
    } else {
        (frac | (1 << 52), exp - 1075)
    };
    let m = if v < 0.0 { -m } else { m };
    (f64::from(crate::x87::mul_i64_f32(m, f)) * 2f64.powi(e)) as f32
}

/// A property's data as the raw little-endian bytes of its values at their
/// own widths (floats as bits); a string gives its characters, an empty or
/// object property nothing.
fn raw_bytes(v: &Variant) -> Vec<u8> {
    let mut out = Vec::new();
    let mut one = |e: &Elem| match *e {
        Elem::Bool(b) => out.push(u8::from(b)),
        Elem::U8(x) | Elem::Char(x) | Elem::RzChar(x) => out.push(x),
        Elem::I8(x) => out.push(x as u8),
        Elem::U16(x) | Elem::UnicodeChar(x) => out.extend_from_slice(&x.to_le_bytes()),
        Elem::I16(x) => out.extend_from_slice(&x.to_le_bytes()),
        Elem::U32(x) => out.extend_from_slice(&x.to_le_bytes()),
        Elem::I32(x) => out.extend_from_slice(&x.to_le_bytes()),
        Elem::F32(x) => out.extend_from_slice(&x.to_bits().to_le_bytes()),
        Elem::U64(x) | Elem::VoidPtr(x) => out.extend_from_slice(&x.to_le_bytes()),
        Elem::I64(x) => out.extend_from_slice(&x.to_le_bytes()),
        Elem::F64(x) => out.extend_from_slice(&x.to_bits().to_le_bytes()),
    };
    match v {
        Variant::Scalar(e) => one(e),
        Variant::Array { items, .. } => items.iter().for_each(one),
        _ => {}
    }
    out
}

fn read_f32(h: &PropertyHolder, id: u32, dst: &mut f32) {
    if let Some(v) = props::f32_of(h, id) {
        *dst = v;
    }
}

fn read_u32(h: &PropertyHolder, id: u32, dst: &mut u32) {
    if let Some(v) = props::u32_of(h, id) {
        *dst = v;
    }
}

/// The "default" curve pattern (§4.14): the code default, replaced by the
/// property's curve when present.
fn read_default_curve(h: &PropertyHolder, id: u32, default: ResponseCurve) -> ResponseCurve {
    props::curve(h, id).unwrap_or(default)
}

/// The congestion curve transformation (§4.14): each kept point's y is
/// replaced by `r24(1 / y)`, the curve rebuilt from the new points, and
/// `congestionScale := r24(255 / r24(lastY − 1))`. An empty curve becomes
/// "speed factor 1" with scale 255 *[choice]*.
pub fn invert_congestion_curve(c: &ResponseCurve) -> (ResponseCurve, f32) {
    let pts: Vec<(f32, f32)> = c
        .points()
        .into_iter()
        .map(|(x, y)| (x, r24(1.0 / f64::from(y)) as f32))
        .collect();
    let Some(&(_, last_y)) = pts.last() else {
        return (speed_one(), 255.0);
    };
    let inv = ResponseCurve::from_points(&pts);
    let d = r24(f64::from(last_y) - 1.0);
    (inv, r24(255.0 / d) as f32)
}

impl SimTuning {
    /// `ReadSimulatorTunableValues` without `UpdateTravelStrategies` (§4.14):
    /// absent exemplar → nothing is read. Float scalars and vectors are
    /// unchanged when missing; the "default" curves restart from the code
    /// default; the congestion curve is kept when missing *[choice]*.
    pub fn read(&mut self, exemplar: Option<&PropertyHolder>) {
        let Some(h) = exemplar else { return };
        read_f32(h, id::VOLUME_PER_POPULATION, &mut self.volume_per_population);
        // The short and missing arrays (TS28): each has its own fill.
        // Max speed: missing → all 13 of type t reset (speed 0, inverse
        // FLT_MAX); fewer → the rest reset the same way.
        for (t, row) in self.max_speed.iter_mut().enumerate() {
            let v = props::f32s(h, id::MAX_SPEED + t as u32).unwrap_or_default();
            *row = std::array::from_fn(|i| v.get(i).copied().unwrap_or(0.0));
        }
        read_f32(h, id::DENSITY_REDUCTION, &mut self.density_reduction);
        read_f32(h, id::UNUSED_29136789, &mut self.unused_29136789);
        // Tile costs: no presence test (`0x00716D34`), so missing → all 0.0;
        // fewer → the rest 0.0.
        let v = props::f32s(h, id::TILE_COSTS).unwrap_or_default();
        self.tile_costs = std::array::from_fn(|i| v.get(i).copied().unwrap_or(0.0));
        // Percent → fraction: `r24(v × 0.01f)` (TS29; shipped 120 gives
        // 0x3F999999, one ulp below the code default 1.2).
        if let Some(v) = percent_fraction(h, id::MAX_ROADS_FUNDING) {
            self.max_roads_funding = v;
        }
        if let Some(v) = percent_fraction(h, id::MAX_TRANSIT_FUNDING) {
            self.max_transit_funding = v;
        }
        read_u32(h, id::STRATEGY_PERIOD, &mut self.strategy_period);
        read_u32(h, id::STRATEGY_BUCKETS, &mut self.strategy_buckets);
        read_u32(h, id::MAX_DISTANCE_ORIGIN, &mut self.max_distance_origin);
        // Income per tile: missing → unchanged; fewer → the original copies 9
        // with no count check (`0x00716E07`) *[quirk]*; FreeCity4 fills 0.0
        // *[choice]*.
        read_f32s(h, id::INCOME_PER_TILE, &mut self.income_per_tile, 0.0);
        read_bools(h, id::GENERATES_TRAFFIC, &mut self.generates_traffic);
        read_bools(h, id::AFFECTED_BY_TRAFFIC, &mut self.affected_by_traffic);
        read_bools(h, id::CAN_REACH, &mut self.can_reach);
        if let Some(c) = props::curve(h, id::CONGESTION_CURVE) {
            let (inv, scale) = invert_congestion_curve(&c);
            self.congestion_curve = inv;
            self.congestion_scale = scale;
        }
        read_f32(h, id::COMMUTE_MAX_TIME, &mut self.commute_max_time);
        // Start costs: missing → unchanged; fewer → the rest 0.0.
        read_f32s(h, id::START_NEUTRAL, &mut self.start_neutral, 0.0);
        read_f32s(h, id::START_MASS_TRANSIT, &mut self.start_mass_transit, 0.0);
        read_f32s(h, id::START_CAR, &mut self.start_car, 0.0);
        read_f32(h, id::JOB_SCALING, &mut self.job_scaling);
        read_f32(h, id::FREIGHT_SCALING, &mut self.freight_scaling);
        read_f32(h, id::NEAREST_ATTRACTIVENESS, &mut self.nearest_attractiveness);
        read_f32(h, id::TRIP_DISPLAY_MULTIPLIER, &mut self.trip_display_multiplier);
        read_f32(h, id::MAX_MASS_TRANSIT_TRIP, &mut self.max_mass_transit_trip_length);
        // Background: missing → unchanged; fewer → the rest 1.0.
        read_f32s(h, id::BACKGROUND, &mut self.background, 1.0);
        let d = SimTuning::default();
        self.switch_curve = read_default_curve(h, id::SWITCH_CURVE, d.switch_curve);
        self.accident_congestion_curve =
            read_default_curve(h, id::ACCIDENT_CONGESTION_CURVE, d.accident_congestion_curve);
        self.accident_capacity_curve = read_default_curve(h, id::ACCIDENT_CAPACITY_CURVE, d.accident_capacity_curve);
        read_f32(h, id::ACCIDENT_DURATION, &mut self.accident_duration);
        read_f32(h, id::ACCIDENT_CHECK_PERIOD, &mut self.accident_check_period);
        self.damage_curve = read_default_curve(h, id::DAMAGE_CURVE, d.damage_curve);
        read_f32(
            h,
            id::RAIL_DAMAGE_ACCIDENT_FACTOR,
            &mut self.rail_damage_accident_factor,
        );
        read_f32(h, id::FERRY_FARE, &mut self.ferry_fare);
        read_f32(h, id::COMMERCIAL_SCALE, &mut self.commercial_scale);
        // The network properties 0xA92356B0, B3, BB (traffic-network.md §4.5).
        self.network.reload(Some(h));
    }

    /// The strategy re-read of `UpdateTravelStrategies` (§4.1.2, TS28): a
    /// present `0x4953E8A3 + w` is read as raw bytes with no type check, and
    /// its first three bytes are copied whatever its length (`0x00711899`).
    /// With fewer than 3 bytes the original reads past the copied data;
    /// the missing ones keep their value *[choice]*. A missing property
    /// leaves `pct[w]` unchanged.
    pub fn read_strategies(&mut self, h: &PropertyHolder) {
        for (w, row) in self.pct.iter_mut().enumerate() {
            if let Some(v) = h.get(id::STRATEGY_PCT + w as u32) {
                for (d, s) in row.iter_mut().zip(raw_bytes(v)) {
                    *d = s;
                }
            }
        }
    }

    /// Adds an ordinance's offsets (`0x8A612FEE`, s32[9]) to `pct[w][s]` for
    /// w = 1…3, element `3(w − 1) + s`, clamping each sum to 0…100 (§4.1.2).
    /// Missing elements count as 0 *[choice: read without a size check]*.
    pub fn add_ordinance_offsets(&mut self, offsets: &[i64]) {
        for w in 1..4 {
            for s in 0..3 {
                let o = offsets.get(3 * (w - 1) + s).copied().unwrap_or(0) as i32;
                let v = (i32::from(self.pct[w][s]) + o).clamp(0, 100);
                self.pct[w][s] = v as u8;
            }
        }
    }

    /// `GetTravelStrategyPercentages(w)` (slot 26): `r24(float(pct[w][s]) ×
    /// 0.01f)`. A wealth above 3 gives zeros *[choice: the original reads
    /// past the table]*.
    pub fn strategy_percentages(&self, w: u32) -> [f32; 3] {
        let Some(row) = self.pct.get(w as usize) else {
            return [0.0; 3];
        };
        std::array::from_fn(|s| r24(f64::from(row[s]) * f64::from(0.01f32)) as f32)
    }

    /// The default starting-cost array (pathfinder.md §4.4): the neutral
    /// one.
    pub fn default_start_cost(&self) -> [f32; 9] {
        self.start_neutral
    }

    /// The exemplar with the "Shipped" values §4.14 states. The properties
    /// the spec only refers to the tuning dump for (tile costs, income per
    /// tile, the three per-travel-type bool vectors, the strategy
    /// percentages) are left out, so the code defaults stay.
    pub fn spec_exemplar() -> PropertyHolder {
        let mut h = NetTuning::shipped_exemplar();
        let f = |v: f32| Variant::Scalar(Elem::F32(v));
        let arr = |v: &[f32]| Variant::array(mem::FLOAT32, v.iter().map(|&x| Elem::F32(x)).collect());
        let pts = |p: &[(f32, f32)]| {
            let flat: Vec<f32> = p.iter().flat_map(|&(x, y)| [x, y]).collect();
            arr(&flat)
        };
        h.add(id::VOLUME_PER_POPULATION, f(2.0));
        for (t, row) in SHIPPED_MAX_SPEED.iter().enumerate() {
            h.add(id::MAX_SPEED + t as u32, arr(row));
        }
        h.add(id::DENSITY_REDUCTION, f(0.0));
        h.add(id::MAX_ROADS_FUNDING, f(120.0));
        h.add(id::MAX_TRANSIT_FUNDING, f(120.0));
        h.add(id::MAX_DISTANCE_ORIGIN, Variant::Scalar(Elem::U32(1)));
        h.add(
            id::CONGESTION_CURVE,
            pts(&[(0.0, 1.0), (1.0, 1.0), (2.0, 0.65), (3.0, 0.3)]),
        );
        h.add(id::COMMUTE_MAX_TIME, f(6.0));
        h.add(id::START_NEUTRAL, arr(&[0.0, 0.4, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]));
        h.add(
            id::START_MASS_TRANSIT,
            arr(&[0.0, 1.95, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        );
        h.add(id::START_CAR, arr(&[0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]));
        h.add(id::JOB_SCALING, f(1.2));
        h.add(id::FREIGHT_SCALING, f(0.05));
        h.add(id::NEAREST_ATTRACTIVENESS, f(0.09));
        h.add(id::TRIP_DISPLAY_MULTIPLIER, f(25.0));
        h.add(id::MAX_MASS_TRANSIT_TRIP, f(4.0));
        h.add(id::BACKGROUND, arr(&[0.05, 0.2, 0.2]));
        h.add(id::SWITCH_CURVE, pts(&[(0.0, 2.0), (100.0, 1.0)]));
        h.add(
            id::ACCIDENT_CONGESTION_CURVE,
            pts(&[(0.0, 0.0), (0.25, 0.0), (1.0, 0.05), (2.0, 0.25)]),
        );
        h.add(id::ACCIDENT_CAPACITY_CURVE, pts(&[(0.0, 1.0), (1000.0, 1.0)]));
        h.add(id::ACCIDENT_DURATION, f(20.0));
        h.add(id::ACCIDENT_CHECK_PERIOD, f(60.0));
        h.add(id::DAMAGE_CURVE, pts(&[(0.0, 0.02), (100.0, -0.01), (120.0, -0.04)]));
        h.add(id::RAIL_DAMAGE_ACCIDENT_FACTOR, f(1.0));
        h.add(id::FERRY_FARE, f(0.01));
        h.add(id::COMMERCIAL_SCALE, f(0.128));
        h
    }
}

/// The shipped "Max speed by Network" table (§4.14), `[t][networkType]`.
pub const SHIPPED_MAX_SPEED: [[f32; 13]; 9] = [
    [3.5, 0.0, 0.0, 3.5, 0.0, 0.0, 3.5, 0.0, 0.0, 0.0, 3.5, 0.0, 0.0],
    [31.0, 0.0, 82.0, 21.0, 0.0, 0.0, 40.0, 0.0, 0.0, 0.0, 31.0, 0.0, 82.0],
    [46.0, 0.0, 100.0, 31.0, 0.0, 0.0, 60.0, 0.0, 0.0, 0.0, 46.0, 0.0, 100.0],
    [0.0, 110.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [31.0, 0.0, 82.0, 21.0, 0.0, 0.0, 40.0, 0.0, 0.0, 0.0, 31.0, 0.0, 82.0],
    [0.0, 150.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 150.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 150.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 200.0, 0.0, 0.0, 0.0],
];
