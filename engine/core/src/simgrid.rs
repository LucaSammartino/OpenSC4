// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; see
// the GNU General Public License for more details. You should have received a
// copy of the license along with this program (see LICENSE at the repository
// root); if not, see <https://www.gnu.org/licenses/>.

//! Simulation grids and the shared map utilities (`docs/specs/simgrid.md`).
//!
//! [`SimGrid`] models the game's `cSC4SimGrid<T>` (geometry §3, slots §4,
//! record §5). The plugin never registers that class (§6.3): in the game it
//! uses the game's own grids, and this model serves the conformance vectors
//! and the land-value core's working grid. The free functions are the
//! `cSC4MapUtilities` helpers of §7 on plain `[x][z]` data.
//!
//! Precision: the original evaluates some expressions in x87 extended
//! precision. FreeCity4 evaluates them in f32 with the spec's operation order
//! (the precision a 24-bit x87 precision-control setting gives). Places where
//! the precision mode can change a result are tagged `[precision]`.

use crate::stream::{InStream, OutStream};

/// Class IDs (§2).
pub const CLSID_GRID_U8: u32 = 0x49B9_E602;
pub const CLSID_GRID_I8: u32 = 0x49B9_E603;
pub const CLSID_GRID_U16: u32 = 0x49B9_E604;
pub const CLSID_GRID_I16: u32 = 0x49B9_E605;
pub const CLSID_GRID_U32: u32 = 0x49B9_E606;
pub const CLSID_GRID_I32: u32 = 0x49B9_E607;
pub const CLSID_GRID_U64: u32 = 0x49B9_E608;
pub const CLSID_GRID_I64: u32 = 0x49B9_E609;
pub const CLSID_GRID_F32: u32 = 0x49B9_E60A;
pub const CLSID_GRID_F64: u32 = 0x49B9_E60B;
/// `cISC4SimGrid<T>` (all T) and the serializable sub-object (§2).
pub const IID_SIM_GRID: u32 = 0xC9BD_939A;
pub const IID_SERIALIZABLE: u32 = 0xE4FD_A3D4;
/// Record version (§5).
pub const GRID_RECORD_VERSION: u16 = 1;

/// Tract geometry (§3).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GridGeometry {
    pub tract_size: i32,
    pub shift: i32,
    pub count_x: i32,
    pub count_z: i32,
    pub width_x: f32,
    pub width_z: f32,
    pub inv_width_x: f32,
    pub inv_width_z: f32,
}

/// The city facts a grid is sized from (the game's global city).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CityCells {
    pub cells_x: i32,
    pub cells_z: i32,
    pub cell_width_x: f32,
    pub cell_width_z: f32,
}

impl Default for CityCells {
    fn default() -> Self {
        CityCells {
            cells_x: 256,
            cells_z: 256,
            cell_width_x: 16.0,
            cell_width_z: 16.0,
        }
    }
}

/// How often S can be halved (integer division) while it is > 1 (§3).
pub fn tract_shift(s: i32) -> i32 {
    let mut n = s;
    let mut shift = 0;
    while n > 1 {
        n /= 2;
        shift += 1;
    }
    shift
}

impl GridGeometry {
    /// `SetTractSize(S)` geometry (§3): counts use the shift, widths use S
    /// [quirk].
    pub fn for_city(city: &CityCells, s: i32) -> GridGeometry {
        let shift = tract_shift(s);
        let width_x = city.cell_width_x * s as f32;
        let width_z = city.cell_width_z * s as f32;
        GridGeometry {
            tract_size: s,
            shift,
            count_x: city.cells_x >> shift,
            count_z: city.cells_z >> shift,
            width_x,
            width_z,
            inv_width_x: 1.0 / width_x,
            inv_width_z: 1.0 / width_z,
        }
    }

    /// Number of tracts (0 for negative counts).
    pub fn len(&self) -> usize {
        (self.count_x.max(0) as usize) * (self.count_z.max(0) as usize)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Index of tract (x, z) in x-outer, z-inner order, if in bounds.
    pub fn index(&self, x: i32, z: i32) -> Option<usize> {
        if x < 0 || z < 0 || x >= self.count_x || z >= self.count_z {
            return None;
        }
        Some(x as usize * self.count_z as usize + z as usize)
    }

    /// `TractIsInBounds` (§4 slot 18): unsigned comparison.
    pub fn tract_in_bounds(&self, x: u32, z: u32) -> bool {
        (x as i64) < i64::from(self.count_x) && (z as i64) < i64::from(self.count_z)
    }

    /// `PositionToTract` (§3): truncation toward zero, no bounds check.
    pub fn position_to_tract(&self, px: f32, pz: f32) -> (i32, i32) {
        ((px * self.inv_width_x) as i32, (pz * self.inv_width_z) as i32)
    }

    /// `TractCornerToPosition` (§3).
    pub fn tract_corner(&self, x: i32, z: i32) -> (f32, f32) {
        (x as f32 * self.width_x, z as f32 * self.width_z)
    }

    /// `TractCenterToPosition` (§3).
    pub fn tract_center(&self, x: i32, z: i32) -> (f32, f32) {
        ((x as f32 + 0.5) * self.width_x, (z as f32 + 0.5) * self.width_z)
    }
}

/// An element type of a grid (§2).
pub trait GridElem: Copy + Default + PartialEq + std::fmt::Debug + 'static {
    const CLSID: u32;
    /// Summed as a signed 64-bit integer (§4.1): zero- or sign-extended,
    /// floats truncated toward zero.
    fn to_sum(self) -> i64;
    /// The average's result from the quotient (§4.1 step 3).
    fn from_quotient(q: f64) -> Self;
    fn put(self, out: &mut dyn OutStream) -> bool;
    fn get(inp: &mut dyn InStream) -> Option<Self>;
}

macro_rules! int_elem {
    ($t:ty, $clsid:expr, $put:ident, $get:ident, $via:ty) => {
        impl GridElem for $t {
            const CLSID: u32 = $clsid;
            fn to_sum(self) -> i64 {
                self as i64
            }
            fn from_quotient(q: f64) -> Self {
                // Truncated toward zero, then to T (§4.1 step 3).
                (q as i64) as $t
            }
            fn put(self, out: &mut dyn OutStream) -> bool {
                out.$put(self as $via)
            }
            fn get(inp: &mut dyn InStream) -> Option<Self> {
                inp.$get().map(|v| v as $t)
            }
        }
    };
}

int_elem!(u8, CLSID_GRID_U8, put_u8, get_u8, u8);
int_elem!(i8, CLSID_GRID_I8, put_u8, get_u8, u8);
int_elem!(u16, CLSID_GRID_U16, put_u16, get_u16, u16);
int_elem!(i16, CLSID_GRID_I16, put_u16, get_u16, u16);
int_elem!(u32, CLSID_GRID_U32, put_u32, get_u32, u32);
int_elem!(i32, CLSID_GRID_I32, put_u32, get_u32, u32);
int_elem!(u64, CLSID_GRID_U64, put_u64, get_u64, u64);
int_elem!(i64, CLSID_GRID_I64, put_u64, get_u64, u64);

impl GridElem for f32 {
    const CLSID: u32 = CLSID_GRID_F32;
    fn to_sum(self) -> i64 {
        self as i64
    }
    fn from_quotient(q: f64) -> Self {
        q as f32
    }
    fn put(self, out: &mut dyn OutStream) -> bool {
        out.put_f32(self)
    }
    fn get(inp: &mut dyn InStream) -> Option<Self> {
        inp.get_f32()
    }
}

impl GridElem for f64 {
    const CLSID: u32 = CLSID_GRID_F64;
    fn to_sum(self) -> i64 {
        self as i64
    }
    fn from_quotient(q: f64) -> Self {
        q
    }
    fn put(self, out: &mut dyn OutStream) -> bool {
        out.put_f64(self)
    }
    fn get(inp: &mut dyn InStream) -> Option<Self> {
        inp.get_f64()
    }
}

/// `cSC4SimGrid<T>` (§2–§5).
#[derive(Clone, Debug, PartialEq)]
pub struct SimGrid<T: GridElem> {
    pub clsid: u32,
    pub instance_id: u32,
    pub initialized: bool,
    pub geometry: GridGeometry,
    /// countX rows of countZ values, x outer (§3).
    pub data: Vec<Vec<T>>,
}

impl<T: GridElem> Default for SimGrid<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: GridElem> SimGrid<T> {
    /// A new grid: its own type's CLSID (the constructor's argument is
    /// ignored, §2 [quirk]); S = 0 and no data until `SetTractSize` [choice].
    pub fn new() -> Self {
        SimGrid {
            clsid: T::CLSID,
            instance_id: 0,
            initialized: false,
            geometry: GridGeometry::default(),
            data: Vec::new(),
        }
    }

    pub fn init(&mut self) -> bool {
        self.initialized = true;
        true
    }

    pub fn shutdown(&mut self) -> bool {
        self.initialized = false;
        true
    }

    /// `SetTractSize(S)` (§4.2): existing values are kept where their indices
    /// still exist; new elements are 0. Returns true [choice].
    pub fn set_tract_size(&mut self, s: i32, city: &CityCells) -> bool {
        if s == self.geometry.tract_size {
            return true;
        }
        self.geometry = GridGeometry::for_city(city, s);
        let cx = self.geometry.count_x.max(0) as usize;
        let cz = self.geometry.count_z.max(0) as usize;
        self.data.resize_with(cx, Vec::new);
        for row in &mut self.data {
            row.resize(cz, T::default());
        }
        true
    }

    fn at(&self, x: i32, z: i32) -> T {
        if x < 0 || z < 0 {
            return T::default();
        }
        self.data
            .get(x as usize)
            .and_then(|r| r.get(z as usize))
            .copied()
            .unwrap_or_default()
    }

    /// `GetTractValue` (slot 22). Out of range reads as 0 [choice] (the
    /// original does not check).
    pub fn tract_value(&self, x: i32, z: i32) -> T {
        self.at(x, z)
    }

    /// `GetCellValue` (slot 7).
    pub fn cell_value(&self, cx: i32, cz: i32) -> T {
        self.at(cx >> self.geometry.shift, cz >> self.geometry.shift)
    }

    /// `SetTractValue` (slot 26). Out of range is ignored [choice].
    pub fn set_tract_value(&mut self, x: i32, z: i32, v: T) {
        if x < 0 || z < 0 {
            return;
        }
        if let Some(c) = self.data.get_mut(x as usize).and_then(|r| r.get_mut(z as usize)) {
            *c = v;
        }
    }

    /// `SetTractValues` (slot 27).
    pub fn set_all(&mut self, v: T) {
        for row in &mut self.data {
            row.fill(v);
        }
    }

    fn quotient(sum: i64, w: i32, h: i32) -> T {
        let n = w.wrapping_mul(h);
        if w <= 0 || h <= 0 || n == 0 {
            // 0 / 0 [quirk]; FreeCity4 returns 0 [choice].
            return T::default();
        }
        T::from_quotient(sum as f64 / f64::from(n))
    }

    /// `GetAverageValueInCellRect(x, z, w, h)` (§4.1).
    pub fn average_in_cell_rect(&self, x: i32, z: i32, w: i32, h: i32) -> T {
        let s = self.geometry.shift;
        if x >> s == (x + w - 1) >> s && z >> s == (z + h - 1) >> s {
            return self.cell_value(x, z);
        }
        let mut sum = 0i64;
        for cx in x..x.saturating_add(w) {
            for cz in z..z.saturating_add(h) {
                sum = sum.wrapping_add(self.cell_value(cx, cz).to_sum());
            }
        }
        Self::quotient(sum, w, h)
    }

    /// `GetAverageValueInTractRect(tx, tz, w, h)` (§4.1).
    pub fn average_in_tract_rect(&self, tx: i32, tz: i32, w: i32, h: i32) -> T {
        let mut sum = 0i64;
        for x in tx..tx.saturating_add(w) {
            for z in tz..tz.saturating_add(h) {
                sum = sum.wrapping_add(self.at(x, z).to_sum());
            }
        }
        Self::quotient(sum, w, h)
    }

    /// The record of §5. True unless the stream failed.
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        let g = &self.geometry;
        out.put_u16(GRID_RECORD_VERSION);
        out.put_u8(u8::from(self.initialized));
        out.put_u32(self.clsid);
        out.put_u32(self.instance_id);
        out.put_i32(g.tract_size);
        out.put_i32(g.shift);
        out.put_i32(g.count_x);
        out.put_i32(g.count_z);
        out.put_f32(g.width_x);
        out.put_f32(g.width_z);
        out.put_f32(g.inv_width_x);
        out.put_f32(g.inv_width_z);
        for row in &self.data {
            for &v in row {
                v.put(out);
            }
        }
        // The original always returns true; FreeCity4 reports stream errors.
        out.error() == 0
    }

    /// Reads a record (§5): every field as stored, the CLSID included. False
    /// if the version is not 1, and [choice] on a stream error.
    pub fn read(&mut self, inp: &mut dyn InStream) -> bool {
        if inp.get_u16() != Some(GRID_RECORD_VERSION) {
            return false;
        }
        let ok = (|| -> Option<()> {
            self.initialized = inp.get_u8()? != 0;
            self.clsid = inp.get_u32()?;
            self.instance_id = inp.get_u32()?;
            let g = GridGeometry {
                tract_size: inp.get_i32()?,
                shift: inp.get_i32()?,
                count_x: inp.get_i32()?,
                count_z: inp.get_i32()?,
                width_x: inp.get_f32()?,
                width_z: inp.get_f32()?,
                inv_width_x: inp.get_f32()?,
                inv_width_z: inp.get_f32()?,
            };
            self.geometry = g;
            let cx = g.count_x.max(0) as usize;
            let cz = g.count_z.max(0) as usize;
            self.data = vec![vec![T::default(); cz]; cx];
            for row in &mut self.data {
                for v in row.iter_mut() {
                    *v = T::get(inp)?;
                }
            }
            Some(())
        })();
        ok.is_some() && inp.error() == 0
    }
}

// ---- §7 map utilities ------------------------------------------------------------------------------

/// Round to nearest, ties to even (§7.1).
pub fn round_half_even(x: f32) -> f32 {
    let r = x.round(); // ties away from zero
    if (x - x.trunc()).abs() == 0.5 {
        2.0 * (x / 2.0).round()
    } else {
        r
    }
}

/// `TABLE(dx, dz)` of §7.1: `float32(sqrt(dx² + dz²))` rounded half-up to
/// three decimals.
pub fn table_distance(dx: i32, dz: i32) -> f32 {
    let s = ((dx * dx + dz * dz) as f32).sqrt();
    ((f64::from(s) * 1000.0 + 0.5).floor() / 1000.0) as f32
}

/// Element types `AddRadiusEffect` exists for (§7.1).
pub trait RadiusElem: GridElem {
    const MIN: i64;
    const MAX: i64;
    fn as_i64(self) -> i64;
    fn from_i64(v: i64) -> Self;
}

macro_rules! radius_elem {
    ($t:ty) => {
        impl RadiusElem for $t {
            const MIN: i64 = <$t>::MIN as i64;
            const MAX: i64 = <$t>::MAX as i64;
            fn as_i64(self) -> i64 {
                self as i64
            }
            fn from_i64(v: i64) -> Self {
                v as $t
            }
        }
    };
}
radius_elem!(u8);
radius_elem!(i8);
radius_elem!(i16);

/// `AddRadiusEffect` (§7.1): a linear cone around tract (cx, cz).
#[allow(clippy::too_many_arguments, clippy::neg_cmp_op_on_partial_ord)]
pub fn add_radius_effect<T: RadiusElem>(
    data: &mut [Vec<T>],
    count_x: i32,
    count_z: i32,
    cx: i32,
    cz: i32,
    r: f32,
    c: f32,
    o: f32,
) {
    if !(r > 0.0) {
        return;
    }
    let rr = round_half_even(r) as i32;
    // [precision] f32 division (the spec's float32((c − o) / r)).
    let k = (c - o) / r;
    let x0 = (cx - rr).max(0);
    let x1 = (cx + rr).min(count_x - 1);
    let z0 = (cz - rr).max(0);
    let z1 = (cz + rr).min(count_z - 1);
    for x in x0..=x1 {
        for z in z0..=z1 {
            let (dx, dz) = (x - cx, z - cz);
            let d = if rr <= 15 {
                table_distance(dx, dz)
            } else {
                ((dx * dx + dz * dz) as f32).sqrt()
            };
            if d < r {
                // [precision] the original evaluates c − d·k in extended
                // precision and rounds once; here d·k and the difference are
                // each rounded to f32.
                let delta = round_half_even(c - d * k) as i64;
                if let Some(cell) = data.get_mut(x as usize).and_then(|row| row.get_mut(z as usize)) {
                    *cell = T::from_i64((cell.as_i64() + delta).clamp(T::MIN, T::MAX));
                }
            }
        }
    }
}

/// The distance field of `AddDistanceEffect` (§7.2): Manhattan distance to the
/// nearest tract whose mask value is `m`, up to `n`; 255 = far.
#[allow(clippy::needless_range_loop)]
pub fn distance_field(mask: &[Vec<u8>], count_x: i32, count_z: i32, m: u8, n: i32) -> Vec<Vec<u8>> {
    let cx = count_x.max(0) as usize;
    let cz = count_z.max(0) as usize;
    let mut dist = vec![vec![255u8; cz]; cx];
    for x in 0..cx {
        for z in 0..cz {
            if mask.get(x).and_then(|r| r.get(z)) == Some(&m) {
                dist[x][z] = 0;
            }
        }
    }
    for round in 1..=n.max(0) {
        let want = (round - 1) as u8; // byte arithmetic [quirk]
        let value = round as u8;
        let prev = dist.clone();
        for x in 0..cx {
            for z in 0..cz {
                if prev[x][z] != 255 {
                    continue;
                }
                let near = (x > 0 && prev[x - 1][z] == want)
                    || (x + 1 < cx && prev[x + 1][z] == want)
                    || (z > 0 && prev[x][z - 1] == want)
                    || (z + 1 < cz && prev[x][z + 1] == want);
                if near {
                    dist[x][z] = value;
                }
            }
        }
    }
    dist
}

/// `AddDistanceEffect` (§7.2) with the target also serving as the mask when
/// `mask` is `None`.
#[allow(clippy::too_many_arguments)]
pub fn add_distance_effect(
    target: &mut [Vec<u8>],
    count_x: i32,
    count_z: i32,
    mask: Option<&[Vec<u8>]>,
    m: u8,
    radius: f32,
    near: f32,
    far: f32,
) {
    let n = radius as i32;
    let dist = match mask {
        Some(mk) => distance_field(mk, count_x, count_z, m, n),
        None => distance_field(target, count_x, count_z, m, n),
    };
    // [precision] f32 division.
    let k = (near - far) / radius;
    for (x, row) in dist.iter().enumerate() {
        for (z, &d) in row.iter().enumerate() {
            let df = f32::from(d);
            if d > 0 && df < radius {
                if let Some(cell) = target.get_mut(x).and_then(|r| r.get_mut(z)) {
                    // [precision] (target + near) − d·k, each step in f32.
                    let v = (f32::from(*cell) + near) - df * k;
                    *cell = (v as i64).clamp(0, 255) as u8;
                }
            }
        }
    }
}

/// Source element types of `ProcessTwoGridsWithCallback` (§7.3).
pub trait SourceElem: GridElem {
    fn as_i32(self) -> i32;
    fn from_i32(v: i32) -> Self;
}

macro_rules! source_elem {
    ($t:ty) => {
        impl SourceElem for $t {
            fn as_i32(self) -> i32 {
                self as i32
            }
            fn from_i32(v: i32) -> Self {
                v as $t
            }
        }
    };
}
source_elem!(u8);
source_elem!(i8);
source_elem!(i16);

/// `ProcessTwoGridsWithCallback<A, B>` (§7.3): visits every tract of A once
/// with the matching value of B. Out-of-range indices (grids that do not
/// cover the same city) are skipped [choice] (the original does not check).
pub fn process_two_grids<A: Copy, B: SourceElem>(
    a_geom: &GridGeometry,
    a: &mut [Vec<A>],
    b_geom: &GridGeometry,
    b: &[Vec<B>],
    mut f: impl FnMut(&mut A, B),
) {
    let bv = |x: i32, z: i32| -> Option<B> {
        if x < 0 || z < 0 {
            return None;
        }
        b.get(x as usize).and_then(|r| r.get(z as usize)).copied()
    };
    let mut visit = |a: &mut [Vec<A>], x: i32, z: i32, v: B| {
        if x < 0 || z < 0 {
            return;
        }
        if let Some(cell) = a.get_mut(x as usize).and_then(|r| r.get_mut(z as usize)) {
            f(cell, v);
        }
    };
    let (sa, sb) = (a_geom.shift, b_geom.shift);
    if sa == sb {
        for x in 0..a_geom.count_x {
            for z in 0..a_geom.count_z {
                if let Some(v) = bv(x, z) {
                    visit(a, x, z, v);
                }
            }
        }
    } else if sa > sb {
        let d = sa - sb;
        let n = 1i32 << d;
        for x in 0..a_geom.count_x {
            for z in 0..a_geom.count_z {
                let mut sum = 0i32;
                for i in 0..n {
                    for j in 0..n {
                        sum = sum.wrapping_add(bv((x << d) + i, (z << d) + j).map_or(0, SourceElem::as_i32));
                    }
                }
                visit(a, x, z, B::from_i32((sum >> d) >> d));
            }
        }
    } else {
        let d = sb - sa;
        let n = 1i32 << d;
        for x in 0..b_geom.count_x {
            for z in 0..b_geom.count_z {
                let Some(v) = bv(x, z) else { continue };
                for i in 0..n {
                    for j in 0..n {
                        visit(a, (x << d) + i, (z << d) + j, v);
                    }
                }
            }
        }
    }
}

/// The scanning cell iterator (§7.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScanIterator {
    pub x0: i32,
    pub x1: i32,
    pub z0: i32,
    pub z1: i32,
    pub step: i32,
    pub x: i32,
    pub z: i32,
    pub state: ScanState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanState {
    Fresh,
    Running,
    Done,
}

impl Default for ScanIterator {
    fn default() -> Self {
        ScanIterator {
            x0: i32::MIN,
            x1: i32::MAX,
            z0: i32::MIN,
            z1: i32::MAX,
            step: 1,
            x: 0,
            z: 0,
            state: ScanState::Fresh,
        }
    }
}

impl ScanIterator {
    pub fn with_rect(x0: i32, x1: i32, z0: i32, z1: i32) -> Self {
        ScanIterator {
            x0,
            x1,
            z0,
            z1,
            ..Default::default()
        }
    }

    /// A grid's tract range; an empty range is widened to one tract (§7.4).
    pub fn for_grid(g: &GridGeometry) -> Self {
        Self::with_rect(0, (g.count_x - 1).max(0), 0, (g.count_z - 1).max(0))
    }

    fn contains(&self, x: i32, z: i32) -> bool {
        x >= self.x0 && x <= self.x1 && z >= self.z0 && z <= self.z1
    }

    pub fn reset(&mut self, x: i32, z: i32) {
        self.x = x;
        self.z = z;
        self.state = if self.contains(x, z) {
            ScanState::Fresh
        } else {
            ScanState::Done
        };
    }

    /// `GetNext` (not an `Iterator`: a finished scan stays finished until `reset`).
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<(i32, i32)> {
        match self.state {
            ScanState::Done => None,
            ScanState::Fresh => {
                self.state = ScanState::Running;
                Some((self.x, self.z))
            }
            ScanState::Running => {
                self.x = self.x.wrapping_add(self.step);
                if self.x > self.x1 {
                    self.x = self.x0;
                    self.z = self.z.wrapping_add(self.step);
                    if self.z > self.z1 {
                        self.state = ScanState::Done;
                        return None;
                    }
                }
                Some((self.x, self.z))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shifts() {
        for (s, want) in [(0, 0), (1, 0), (2, 1), (3, 1), (4, 2), (5, 2), (8, 3)] {
            assert_eq!(tract_shift(s), want, "S = {s}");
        }
    }

    #[test]
    fn radius_cone() {
        let mut g = vec![vec![0i16; 20]; 20];
        add_radius_effect(&mut g, 20, 20, 10, 10, 3.0, 100.0, 0.0);
        assert_eq!(
            (g[10][10], g[11][10], g[11][11], g[12][10], g[12][11], g[12][12], g[13][10]),
            (100, 67, 53, 33, 25, 6, 0)
        );
    }

    #[test]
    fn distances_and_iterator() {
        let mut t = vec![vec![0u8], vec![10], vec![10], vec![10], vec![10], vec![10], vec![10]];
        add_distance_effect(&mut t, 7, 1, None, 0, 3.0, 30.0, 0.0);
        let flat: Vec<u8> = t.iter().map(|r| r[0]).collect();
        assert_eq!(flat, [0, 30, 20, 10, 10, 10, 10]);
        let mut it = ScanIterator::with_rect(0, 2, 0, 1);
        it.reset(0, 0);
        let seen: Vec<_> = std::iter::from_fn(|| it.next()).collect();
        assert_eq!(seen, [(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)]);
    }
}
