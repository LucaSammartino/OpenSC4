// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Shared pieces of the grid simulators' C ABI (fc4.h "host grids and
//! objects"):
//!
//! - [`fc4_grid_api`]: the game's own `cSC4SimGrid<T>` objects (simgrid.md
//!   §6.4), driven through host callbacks. [`HostGrid`] implements the core's
//!   `TractGrid<T>` over them; a grid is a **token** (an interface pointer
//!   holding one reference, released when the handle drops).
//! - Variants crossing the ABI in the game's native layout (property reads
//!   through a scratch `fc4_props`, property writes as typed bytes).
//! - Token lists (`fc4_token_callback`), cell rectangles and other small types the
//!   grid simulators share.

use std::cell::Cell;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::rc::Rc;

use fc4_core::grid::{CellRect, GridElem, TractGeometry, TractGrid};
use fc4_core::property::PropertyHolder;
use fc4_core::stream::RefValue;
use fc4_core::variant::{mem, Elem, Variant};

use crate::budget::fc4_token_callback;
use crate::landvalue::fc4_grid_geometry;
use crate::props::fc4_props;

type U = *mut c_void;

/// The game's grids (fc4.h `fc4_grid_api`). `kind` is the grid class ID of the
/// element type (`0x49B9E602` uint8 … `0x49B9E60B` double, simgrid §2).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_grid_api {
    pub struct_size: u32,
    pub user: *mut c_void,
    /// A new game grid, `Init`ed, `SetTractSize(tract_size)`,
    /// `SetInstanceID(instance)`; the token holds one reference. 0 = failure.
    pub create: Option<unsafe extern "C" fn(U, u32, u32, i32) -> u64>,
    /// The grid's class ID (`GetGZCLSID`), 0 if unknown.
    pub class_of: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub geometry: Option<unsafe extern "C" fn(U, u64, *mut fc4_grid_geometry)>,
    /// `GetTractValue(tx, tz)` as a double (every element type up to 32 bits
    /// is exact).
    pub get: Option<unsafe extern "C" fn(U, u64, u32, i32, i32) -> f64>,
    /// `SetTractValue(tx, tz, v)`.
    pub set: Option<unsafe extern "C" fn(U, u64, u32, i32, i32, f64)>,
    /// `SetTractValues(v)`.
    pub fill: Option<unsafe extern "C" fn(U, u64, u32, f64)>,
    /// `Shutdown()`.
    pub shutdown: Option<unsafe extern "C" fn(U, u64)>,
    /// `Release()` of the token's reference.
    pub release: Option<unsafe extern "C" fn(U, u64)>,
}

impl fc4_grid_api {
    pub(crate) fn complete(&self) -> bool {
        (self.struct_size as usize) >= std::mem::size_of::<fc4_grid_api>()
    }
}

macro_rules! gcb {
    ($api:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $api.$f { Some(f) => unsafe { f($api.user $(, $arg)*) }, None => $default }
    };
}

/// A game grid as a core `TractGrid<T>`. The geometry is read once (the
/// simulators never resize their grids after creating them).
pub(crate) struct HostGrid<T: GridElem> {
    api: fc4_grid_api,
    token: u64,
    shutdown_on_drop: bool,
    geom: Cell<Option<TractGeometry>>,
    _t: PhantomData<T>,
}

impl<T: GridElem> HostGrid<T> {
    /// Wraps a token that holds one reference (released on drop).
    pub(crate) fn wrap(api: fc4_grid_api, token: u64, shutdown_on_drop: bool) -> Rc<HostGrid<T>> {
        Rc::new(HostGrid {
            api,
            token,
            shutdown_on_drop,
            geom: Cell::new(None),
            _t: PhantomData,
        })
    }
}

impl<T: GridElem> Drop for HostGrid<T> {
    fn drop(&mut self) {
        if self.token == 0 {
            return;
        }
        if self.shutdown_on_drop {
            gcb!(self.api, shutdown, (), self.token);
        }
        gcb!(self.api, release, (), self.token);
    }
}

impl<T: GridElem> TractGrid<T> for HostGrid<T> {
    fn geometry(&self) -> TractGeometry {
        if let Some(g) = self.geom.get() {
            return g;
        }
        let mut g = fc4_grid_geometry::default();
        gcb!(self.api, geometry, (), self.token, &mut g);
        let geom = TractGeometry {
            tract_size: g.tract_size,
            shift: g.shift,
            count_x: g.count_x,
            count_z: g.count_z,
            width_x: g.width_x,
            width_z: g.width_z,
            inv_width_x: g.inv_width_x,
            inv_width_z: g.inv_width_z,
        };
        self.geom.set(Some(geom));
        geom
    }
    fn get(&self, tx: i32, tz: i32) -> T {
        // Out-of-range reads return 0 (the trait's rule; the game does not check).
        if !self.geometry().in_bounds(tx, tz) {
            return T::default();
        }
        T::from_f64(gcb!(self.api, get, 0.0, self.token, T::CLSID, tx, tz))
    }
    fn set(&self, tx: i32, tz: i32, v: T) {
        if !self.geometry().in_bounds(tx, tz) {
            return;
        }
        gcb!(self.api, set, (), self.token, T::CLSID, tx, tz, v.to_f64())
    }
    fn fill(&self, v: T) {
        gcb!(self.api, fill, (), self.token, T::CLSID, v.to_f64())
    }
    fn save_ref(&self) -> RefValue {
        RefValue::Host {
            token: self.token,
            class_id: T::CLSID,
        }
    }
}

/// `GridProvider::create_grid` over the host.
pub(crate) fn create_grid<T: GridElem>(
    api: &fc4_grid_api,
    instance: u32,
    tract_size: i32,
    shutdown_on_drop: bool,
) -> Option<Rc<dyn TractGrid<T>>> {
    let t = gcb!(api, create, 0, T::CLSID, instance, tract_size);
    (t != 0).then(|| HostGrid::<T>::wrap(*api, t, shutdown_on_drop) as Rc<dyn TractGrid<T>>)
}

/// `GridProvider::resolve_grid` over the host: a host reference (with the one
/// reference the stream handed over) of the right class becomes a grid; any
/// other reference is dropped.
pub(crate) fn resolve_grid<T: GridElem>(
    api: &fc4_grid_api,
    r: &RefValue,
    shutdown_on_drop: bool,
) -> Option<Rc<dyn TractGrid<T>>> {
    let RefValue::Host { token, .. } = *r else {
        return None;
    };
    if token == 0 {
        return None;
    }
    let class = gcb!(api, class_of, 0, token);
    // 0x49B9E601 ("bool") is a uint8 grid too (simgrid §2).
    let ok = class == T::CLSID || (T::CLSID == 0x49B9_E602 && class == 0x49B9_E601);
    if !ok {
        gcb!(api, release, (), token);
        return None;
    }
    Some(HostGrid::<T>::wrap(*api, token, shutdown_on_drop) as Rc<dyn TractGrid<T>>)
}

/// The token behind a grid handle (0 for a core grid or none).
pub(crate) fn grid_token<T: GridElem>(g: &Option<Rc<dyn TractGrid<T>>>) -> u64 {
    match g.as_ref().map(|g| g.save_ref()) {
        Some(RefValue::Host { token, .. }) => token,
        _ => 0,
    }
}

// ---- small shared types ---------------------------------------------------------------------------

/// An inclusive cell rectangle (fc4.h `fc4_cell_rect`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_cell_rect {
    pub x1: i32,
    pub z1: i32,
    pub x2: i32,
    pub z2: i32,
}

impl From<CellRect> for fc4_cell_rect {
    fn from(r: CellRect) -> Self {
        fc4_cell_rect {
            x1: r.x1,
            z1: r.z1,
            x2: r.x2,
            z2: r.z2,
        }
    }
}

impl From<fc4_cell_rect> for CellRect {
    fn from(r: fc4_cell_rect) -> Self {
        CellRect {
            x1: r.x1,
            z1: r.z1,
            x2: r.x2,
            z2: r.z2,
        }
    }
}

unsafe extern "C" fn push_token(ctx: *mut c_void, token: u64) {
    if let Some(v) = (ctx as *mut Vec<u64>).as_mut() {
        v.push(token);
    }
}

/// Calls a host list callback with a sink that collects the tokens.
pub(crate) fn collect_tokens(f: impl FnOnce(fc4_token_callback, *mut c_void)) -> Vec<u64> {
    let mut v: Vec<u64> = Vec::new();
    f(Some(push_token), &mut v as *mut Vec<u64> as *mut c_void);
    v
}

// ---- variants -------------------------------------------------------------------------------------

/// Reads property `id` of a host object through a scratch property holder the
/// host fills (`fill(props) -> found`).
pub(crate) fn read_property(id: u32, fill: impl FnOnce(*mut fc4_props) -> bool) -> Option<Variant> {
    let mut scratch = PropertyHolder::new();
    let p = &mut scratch as *mut PropertyHolder as *mut fc4_props;
    if !fill(p) {
        return None;
    }
    scratch.get(id).cloned()
}

/// Takes ownership of a host-built `fc4_props` (NULL = none).
pub(crate) unsafe fn take_props(p: *mut fc4_props) -> Option<Rc<PropertyHolder>> {
    if p.is_null() {
        return None;
    }
    Some(Rc::new(*Box::from_raw(p as *mut PropertyHolder)))
}

/// A variant in the game's native layout: element type (`mem::*`), array
/// flag, element bytes.
pub(crate) struct NativeVariant {
    pub mem_type: u16,
    pub is_array: bool,
    pub count: usize,
    pub bytes: Vec<u8>,
}

fn elem_bytes(e: &Elem, out: &mut Vec<u8>) {
    match *e {
        Elem::Bool(b) => out.push(u8::from(b)),
        Elem::U8(v) | Elem::Char(v) | Elem::RzChar(v) => out.push(v),
        Elem::I8(v) => out.push(v as u8),
        Elem::U16(v) | Elem::UnicodeChar(v) => out.extend_from_slice(&v.to_ne_bytes()),
        Elem::I16(v) => out.extend_from_slice(&v.to_ne_bytes()),
        Elem::U32(v) => out.extend_from_slice(&v.to_ne_bytes()),
        Elem::I32(v) => out.extend_from_slice(&v.to_ne_bytes()),
        Elem::F32(v) => out.extend_from_slice(&v.to_ne_bytes()),
        Elem::U64(v) => out.extend_from_slice(&v.to_ne_bytes()),
        Elem::I64(v) => out.extend_from_slice(&v.to_ne_bytes()),
        Elem::F64(v) => out.extend_from_slice(&v.to_ne_bytes()),
        Elem::VoidPtr(_) => {}
    }
}

/// Encodes a numeric or character variant; `None` for types the game's
/// property helpers do not take from the core (empty, pointers, objects).
pub(crate) fn native_variant(v: &Variant) -> Option<NativeVariant> {
    let mem_type = v.mem_type();
    if !(mem::BOOL..=mem::RZ_CHAR).contains(&mem_type) {
        return None;
    }
    let items = v.items();
    let mut bytes = Vec::new();
    for e in &items {
        elem_bytes(e, &mut bytes);
    }
    Some(NativeVariant {
        mem_type,
        is_array: matches!(v, Variant::Array { .. }),
        count: items.len(),
        bytes,
    })
}

const _: () = {
    assert!(std::mem::size_of::<fc4_cell_rect>() == 16);
    assert!(std::mem::size_of::<fc4_grid_api>() == if cfg!(target_pointer_width = "32") { 40 } else { 80 });
};
