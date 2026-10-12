// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Grid geometry (§3), the grid access trait (§4), the core-owned grid
//! object with its record (§5) and the grid provider callbacks (§6).

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use super::elem::GridElem;
use crate::stream::{InStream, OutStream, RefValue, SaveObject};

/// The record version (§5).
pub const GRID_RECORD_VERSION: u16 = 1;

/// The largest tract count per axis a core grid allocates ([choice]: a
/// lying host or a hostile save cannot make it allocate more).
pub const MAX_TRACTS: i32 = 4096;

/// What a grid reads from the city to size itself (§3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CityDims {
    pub cell_count_x: i32,
    pub cell_count_z: i32,
    pub cell_width_x: f32,
    pub cell_width_z: f32,
}

impl CityDims {
    /// A square city of `cells` cells of 16 m.
    pub fn square(cells: i32) -> CityDims {
        CityDims {
            cell_count_x: cells,
            cell_count_z: cells,
            cell_width_x: 16.0,
            cell_width_z: 16.0,
        }
    }
}

/// The geometry fields of §3.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TractGeometry {
    /// S, the nominal tract size in cells.
    pub size: i32,
    pub shift: i32,
    pub count_x: i32,
    pub count_z: i32,
    pub width_x: f32,
    pub width_z: f32,
    pub inv_width_x: f32,
    pub inv_width_z: f32,
}

/// The number of times `s` can be halved while it is > 1 (§3).
pub fn tract_shift(s: i32) -> i32 {
    let mut s = s;
    let mut shift = 0;
    while s > 1 {
        s /= 2;
        shift += 1;
    }
    shift
}

impl TractGeometry {
    /// The geometry `SetTractSize(s)` computes from the city (§3, §4.2).
    /// Counts are clamped to `0 ..= MAX_TRACTS` ([choice]).
    pub fn for_city(city: &CityDims, s: i32) -> TractGeometry {
        let shift = tract_shift(s);
        let count = |c: i32| (c >> shift).clamp(0, MAX_TRACTS);
        let width_x = city.cell_width_x * s as f32;
        let width_z = city.cell_width_z * s as f32;
        TractGeometry {
            size: s,
            shift,
            count_x: count(city.cell_count_x),
            count_z: count(city.cell_count_z),
            width_x,
            width_z,
            inv_width_x: 1.0 / width_x,
            inv_width_z: 1.0 / width_z,
        }
    }

    /// The shift used for cell arithmetic: 0 ..= 31 (a hostile save may store
    /// anything).
    pub fn safe_shift(&self) -> u32 {
        self.shift.clamp(0, 31) as u32
    }

    /// Cell → tract: arithmetic shift (§3).
    pub fn cell_to_tract(&self, cx: i32, cz: i32) -> (i32, i32) {
        let s = self.safe_shift();
        (cx >> s, cz >> s)
    }

    /// `PositionToTract`: truncation toward zero (§3; saturating, NaN → 0).
    pub fn position_to_tract(&self, px: f32, pz: f32) -> (i32, i32) {
        ((px * self.inv_width_x) as i32, (pz * self.inv_width_z) as i32)
    }

    /// `TractCornerToPosition` (§3).
    pub fn tract_corner(&self, tx: i32, tz: i32) -> (f32, f32) {
        (tx as f32 * self.width_x, tz as f32 * self.width_z)
    }

    /// `TractCenterToPosition` (§3).
    pub fn tract_center(&self, tx: i32, tz: i32) -> (f32, f32) {
        ((tx as f32 + 0.5) * self.width_x, (tz as f32 + 0.5) * self.width_z)
    }

    /// `TractIsInBounds`: unsigned compares (§4 slot 18).
    pub fn in_bounds(&self, tx: i32, tz: i32) -> bool {
        (tx as u32) < (self.count_x as u32) && (tz as u32) < (self.count_z as u32)
    }
}

/// The `cISC4SimGrid<T>` operations a simulator uses (§4). The core's own
/// [`SimGrid`] implements it; in game the plugin implements it over the
/// game's grid object (§6.3 recommends using the game's grid class), so
/// simulators never assume Rust-owned storage.
///
/// Every method takes `&self` (grids are shared objects). Out-of-range
/// reads return 0 and out-of-range writes are ignored ([choice]: the
/// original does not check bounds).
pub trait TractGrid<T: GridElem> {
    /// The geometry fields (§3).
    fn geometry(&self) -> TractGeometry;
    /// `GetTractValue(tx, tz)`.
    fn get(&self, tx: i32, tz: i32) -> T;
    /// `SetTractValue(tx, tz, v)`.
    fn set(&self, tx: i32, tz: i32, v: T);
    /// The reference an owner writes into its record (§5).
    fn save_ref(&self) -> RefValue;

    /// `SetTractValues(v)`.
    fn fill(&self, v: T) {
        let g = self.geometry();
        for x in 0..g.count_x.clamp(0, MAX_TRACTS) {
            for z in 0..g.count_z.clamp(0, MAX_TRACTS) {
                self.set(x, z, v);
            }
        }
    }
    fn count_x(&self) -> i32 {
        self.geometry().count_x
    }
    fn count_z(&self) -> i32 {
        self.geometry().count_z
    }
    fn shift(&self) -> i32 {
        self.geometry().shift
    }
    /// `GetCellValue(cx, cz)`: the containing tract's value (§4 slot 7).
    fn get_cell_value(&self, cx: i32, cz: i32) -> T {
        let (tx, tz) = self.geometry().cell_to_tract(cx, cz);
        self.get(tx, tz)
    }
    /// `TractIsInBounds(tx, tz)`.
    fn tract_in_bounds(&self, tx: i32, tz: i32) -> bool {
        self.geometry().in_bounds(tx, tz)
    }
    /// `GetAverageValueInCellRect(x, z, w, h)` (§4.1).
    fn average_in_cell_rect(&self, x: i32, z: i32, w: i32, h: i32) -> T {
        let g = self.geometry();
        let s = g.safe_shift();
        let x2 = i64::from(x) + i64::from(w) - 1;
        let z2 = i64::from(z) + i64::from(h) - 1;
        if i64::from(x >> s) == x2 >> s && i64::from(z >> s) == z2 >> s {
            return self.get(x >> s, z >> s);
        }
        // Σ over cells of the containing tract's value, grouped per tract.
        let span = |t: i64, lo: i64, len: i64| -> i64 {
            let a = (t << s).max(lo);
            let b = ((t + 1) << s).min(lo + len);
            (b - a).max(0)
        };
        let sum = sum_tracts(self, &g, |tx, tz| {
            span(tx, i64::from(x), i64::from(w)).wrapping_mul(span(tz, i64::from(z), i64::from(h)))
        });
        quotient(sum, w, h)
    }
    /// `GetAverageValueInTractRect(tx, tz, w, h)` (§4.1).
    fn average_in_tract_rect(&self, tx: i32, tz: i32, w: i32, h: i32) -> T {
        let g = self.geometry();
        let inside = |t: i64, lo: i32, len: i32| t >= i64::from(lo) && t < i64::from(lo) + i64::from(len);
        let sum = sum_tracts(self, &g, |x, z| i64::from(inside(x, tx, w) && inside(z, tz, h)));
        quotient(sum, w, h)
    }
}

/// Σ over the grid's tracts of `value × weight(tx, tz)` (wrapping, as the
/// original's 64-bit accumulator). Tracts outside the grid read 0, so only
/// tracts inside contribute.
fn sum_tracts<T: GridElem, G: TractGrid<T> + ?Sized>(
    grid: &G,
    g: &TractGeometry,
    weight: impl Fn(i64, i64) -> i64,
) -> i64 {
    let mut sum = 0i64;
    for x in 0..g.count_x.clamp(0, MAX_TRACTS) {
        for z in 0..g.count_z.clamp(0, MAX_TRACTS) {
            let n = weight(i64::from(x), i64::from(z));
            if n != 0 {
                sum = sum.wrapping_add(grid.get(x, z).sum_term().wrapping_mul(n));
            }
        }
    }
    sum
}

/// `sum / (w × h)` with the 32-bit product, or 0 when the sum is empty
/// ([choice] for `w` or `h` ≤ 0, §4.1).
fn quotient<T: GridElem>(sum: i64, w: i32, h: i32) -> T {
    let n = w.wrapping_mul(h);
    if w <= 0 || h <= 0 || n == 0 {
        return T::default();
    }
    T::from_quotient(sum as f64 / f64::from(n))
}

/// The mutable state of a core grid.
#[derive(Clone, Debug, PartialEq)]
pub struct SimGridData<T: GridElem> {
    pub init: bool,
    /// The CLSID field (§2; `Read` overwrites it, §5).
    pub clsid: u32,
    pub instance: u32,
    pub geom: TractGeometry,
    /// `count_x × count_z` values, x outer.
    pub values: Vec<T>,
}

impl<T: GridElem> SimGridData<T> {
    fn index(&self, tx: i32, tz: i32) -> Option<usize> {
        if self.geom.in_bounds(tx, tz) {
            Some(tx as usize * self.geom.count_z as usize + tz as usize)
        } else {
            None
        }
    }
}

/// The core's own `cSC4SimGrid<T>`: Rust-owned storage, the slots of §4 and
/// the record of §5. Used by tests and by hosts that do not supply the
/// game's grids.
pub struct SimGrid<T: GridElem> {
    me: Weak<SimGrid<T>>,
    token: Cell<u64>,
    pub data: RefCell<SimGridData<T>>,
}

impl<T: GridElem> SimGrid<T> {
    /// A new grid: S = 0, no tracts, not initialised (§2; [choice] for S).
    pub fn new() -> Rc<SimGrid<T>> {
        Rc::new_cyclic(|me| SimGrid {
            me: me.clone(),
            token: Cell::new(0),
            data: RefCell::new(SimGridData {
                init: false,
                clsid: T::CLSID,
                instance: 0,
                geom: TractGeometry::default(),
                values: Vec::new(),
            }),
        })
    }

    /// What the original simulators do (§6.2): `Init`, `SetTractSize`,
    /// `SetInstanceID`.
    pub fn create(city: &CityDims, instance: u32, tract_size: i32) -> Rc<SimGrid<T>> {
        let g = SimGrid::new();
        g.init();
        g.set_tract_size(city, tract_size);
        g.set_instance_id(instance);
        g
    }

    /// The host identity reported by `host_token` (0 = none).
    pub fn set_host_token(&self, token: u64) {
        self.token.set(token);
    }

    pub fn init(&self) -> bool {
        self.data.borrow_mut().init = true;
        true
    }

    pub fn shutdown(&self) -> bool {
        self.data.borrow_mut().init = false;
        true
    }

    pub fn instance_id(&self) -> u32 {
        self.data.borrow().instance
    }

    pub fn set_instance_id(&self, id: u32) {
        self.data.borrow_mut().instance = id;
    }

    /// `SetTractSize(S)` (§4.2): nothing if S is unchanged; otherwise new
    /// geometry, values kept where the indices still exist, new ones 0.
    /// Returns true ([choice]).
    pub fn set_tract_size(&self, city: &CityDims, s: i32) -> bool {
        let mut d = self.data.borrow_mut();
        if d.geom.size == s {
            return true;
        }
        let ng = TractGeometry::for_city(city, s);
        let mut values = vec![T::default(); ng.count_x as usize * ng.count_z as usize];
        for x in 0..ng.count_x.min(d.geom.count_x) {
            for z in 0..ng.count_z.min(d.geom.count_z) {
                if let Some(i) = d.index(x, z) {
                    values[x as usize * ng.count_z as usize + z as usize] = d.values[i];
                }
            }
        }
        d.geom = ng;
        d.values = values;
        true
    }

    /// All values, x outer.
    pub fn values(&self) -> Vec<T> {
        self.data.borrow().values.clone()
    }
}

impl<T: GridElem> TractGrid<T> for SimGrid<T> {
    fn geometry(&self) -> TractGeometry {
        self.data.borrow().geom
    }
    fn get(&self, tx: i32, tz: i32) -> T {
        let d = self.data.borrow();
        d.index(tx, tz)
            .and_then(|i| d.values.get(i).copied())
            .unwrap_or_default()
    }
    fn set(&self, tx: i32, tz: i32, v: T) {
        let mut d = self.data.borrow_mut();
        if let Some(i) = d.index(tx, tz) {
            if let Some(slot) = d.values.get_mut(i) {
                *slot = v;
            }
        }
    }
    fn fill(&self, v: T) {
        for slot in self.data.borrow_mut().values.iter_mut() {
            *slot = v;
        }
    }
    fn save_ref(&self) -> RefValue {
        match self.me.upgrade() {
            Some(rc) => RefValue::Object(rc),
            None => RefValue::Null,
        }
    }
}

impl<T: GridElem> SaveObject for SimGrid<T> {
    fn class_id(&self) -> u32 {
        self.data.borrow().clsid
    }

    /// The §5 record. Returns false only on a stream error.
    fn write(&self, out: &mut dyn OutStream) -> bool {
        let d = self.data.borrow();
        let g = &d.geom;
        out.put_u16(GRID_RECORD_VERSION);
        out.put_u8(u8::from(d.init));
        out.put_u32(d.clsid);
        out.put_u32(d.instance);
        for v in [g.size, g.shift, g.count_x, g.count_z] {
            out.put_i32(v);
        }
        for v in [g.width_x, g.width_z, g.inv_width_x, g.inv_width_z] {
            out.put_f32(v);
        }
        for &v in &d.values {
            v.put(out);
        }
        out.error() == 0
    }

    /// The §5 record: false for a version other than 1; fields as stored.
    /// [choice]: false on a stream error, on negative counts or counts above
    /// [`MAX_TRACTS`]; the grid is then unchanged.
    fn read(&self, inp: &mut dyn InStream) -> bool {
        match read_record::<T>(inp) {
            Some(d) => {
                *self.data.borrow_mut() = d;
                true
            }
            None => false,
        }
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}

fn read_record<T: GridElem>(inp: &mut dyn InStream) -> Option<SimGridData<T>> {
    if inp.get_u16()? != GRID_RECORD_VERSION {
        return None;
    }
    let init = inp.get_u8()? != 0;
    let clsid = inp.get_u32()?;
    let instance = inp.get_u32()?;
    let geom = TractGeometry {
        size: inp.get_i32()?,
        shift: inp.get_i32()?,
        count_x: inp.get_i32()?,
        count_z: inp.get_i32()?,
        width_x: inp.get_f32()?,
        width_z: inp.get_f32()?,
        inv_width_x: inp.get_f32()?,
        inv_width_z: inp.get_f32()?,
    };
    let ok = |c: i32| (0..=MAX_TRACTS).contains(&c);
    if !ok(geom.count_x) || !ok(geom.count_z) {
        return None;
    }
    let n = geom.count_x as usize * geom.count_z as usize;
    let mut values = Vec::with_capacity(n.min(1 << 16));
    for _ in 0..n {
        values.push(T::get(inp)?);
    }
    if inp.error() != 0 {
        return None;
    }
    Some(SimGridData {
        init,
        clsid,
        instance,
        geom,
        values,
    })
}

/// Creates a core grid for a class-object store CLSID (§2 factories):
/// `0x49B9E601` ("bool") makes a `u8` grid; `0x49B9E600` is not registered.
pub fn create_for_clsid(clsid: u32) -> Option<Rc<dyn SaveObject>> {
    Some(match clsid {
        0x49B9_E601 | 0x49B9_E602 => SimGrid::<u8>::new() as Rc<dyn SaveObject>,
        0x49B9_E603 => SimGrid::<i8>::new(),
        0x49B9_E604 => SimGrid::<u16>::new(),
        0x49B9_E605 => SimGrid::<i16>::new(),
        0x49B9_E606 => SimGrid::<u32>::new(),
        0x49B9_E607 => SimGrid::<i32>::new(),
        0x49B9_E608 => SimGrid::<u64>::new(),
        0x49B9_E609 => SimGrid::<i64>::new(),
        0x49B9_E60A => SimGrid::<f32>::new(),
        0x49B9_E60B => SimGrid::<f64>::new(),
        _ => return None,
    })
}

/// How a simulator gets its grids from the host (§6.2). In game the plugin
/// creates the game's own grid objects (§6.3) and wraps them; tests and
/// standalone hosts use [`CoreGrids`].
pub trait GridProvider<T: GridElem> {
    /// A new grid, already `Init`ed, sized with `SetTractSize(tract_size)`
    /// and given `instance` (§6.2). `None` if the host cannot make one.
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<T>>>;

    /// The grid a reference read from a save points at (read with IID
    /// `0xC9BD939A`). The default accepts core [`SimGrid`]s of type `T`.
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<T>>> {
        r.object::<SimGrid<T>>().map(|g| g as Rc<dyn TractGrid<T>>)
    }
}

/// A [`GridProvider`] for every element type that makes core [`SimGrid`]s
/// for a fixed city.
#[derive(Clone, Copy, Debug)]
pub struct CoreGrids {
    pub city: CityDims,
}

impl<T: GridElem> GridProvider<T> for CoreGrids {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<T>>> {
        Some(SimGrid::<T>::create(&self.city, instance, tract_size) as Rc<dyn TractGrid<T>>)
    }
}
