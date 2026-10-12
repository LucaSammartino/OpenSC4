// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The ten grid element types (§2) and the per-type rules of §4.1, §5 and
//! §7.1.

use std::fmt::Debug;

use crate::stream::{InStream, OutStream};

/// An element type of `cSC4SimGrid<T>`.
pub trait GridElem: Copy + Default + PartialEq + Debug + 'static {
    /// The CLSID a grid of this type stores and saves under (§2).
    const CLSID: u32;
    /// A short type name (`u8`, `i16`, `f32`, …).
    const NAME: &'static str;
    /// Whether this is `float` or `double` (§4.1 step 3).
    const FLOAT: bool;

    /// The value as a term of the §4.1 sum: integers zero- or sign-extended
    /// to 64 bits, floats truncated toward zero (saturating; NaN → 0).
    fn sum_term(self) -> i64;
    /// The §4.1 result from the floating-point quotient: integers truncated
    /// toward zero (then reduced to `T`'s bits), floats the quotient itself.
    fn from_quotient(q: f64) -> Self;
    /// The value as a 32-bit term of the coarse two-grid mean (§7.3):
    /// integers reduced to 32 bits, floats truncated toward zero
    /// ([choice]).
    fn mean_term(self) -> i32;
    /// The coarse two-grid mean converted back to `T` (§7.3).
    fn from_mean(v: i32) -> Self;
    /// The value as `f64` (for reports).
    fn to_f64(self) -> f64;
    /// The value from `f64` (Rust `as` semantics; for tests and reports).
    fn from_f64(v: f64) -> Self;
    /// Writes the value in its native little-endian size (§5).
    fn put(self, out: &mut dyn OutStream) -> bool;
    /// Reads a value written by [`GridElem::put`].
    fn get(inp: &mut dyn InStream) -> Option<Self>;
}

/// The integer types `AddRadiusEffect` has variants for (§7.1).
pub trait RadiusElem: GridElem {
    const MIN: i64;
    const MAX: i64;
    fn to_i64(self) -> i64;
    /// `v` clamped to the type's range.
    fn clamped(v: i64) -> Self;
}

macro_rules! int_elem {
    ($t:ty, $clsid:expr, $name:expr, $put:ident, $get:ident) => {
        impl GridElem for $t {
            const CLSID: u32 = $clsid;
            const NAME: &'static str = $name;
            const FLOAT: bool = false;
            fn sum_term(self) -> i64 {
                self as i64
            }
            fn from_quotient(q: f64) -> Self {
                (q as i64) as $t
            }
            fn mean_term(self) -> i32 {
                self as i32
            }
            fn from_mean(v: i32) -> Self {
                v as $t
            }
            fn to_f64(self) -> f64 {
                self as f64
            }
            fn from_f64(v: f64) -> Self {
                v as $t
            }
            fn put(self, out: &mut dyn OutStream) -> bool {
                out.$put(self)
            }
            fn get(inp: &mut dyn InStream) -> Option<Self> {
                inp.$get()
            }
        }
    };
}

int_elem!(u8, 0x49B9_E602, "u8", put_u8, get_u8);
int_elem!(i8, 0x49B9_E603, "i8", put_i8, get_i8);
int_elem!(u16, 0x49B9_E604, "u16", put_u16, get_u16);
int_elem!(i16, 0x49B9_E605, "i16", put_i16, get_i16);
int_elem!(u32, 0x49B9_E606, "u32", put_u32, get_u32);
int_elem!(i32, 0x49B9_E607, "i32", put_i32, get_i32);
int_elem!(i64, 0x49B9_E609, "i64", put_i64, get_i64);

impl GridElem for u64 {
    const CLSID: u32 = 0x49B9_E608;
    const NAME: &'static str = "u64";
    const FLOAT: bool = false;
    fn sum_term(self) -> i64 {
        self as i64
    }
    fn from_quotient(q: f64) -> Self {
        (q as i64) as u64
    }
    fn mean_term(self) -> i32 {
        self as i32
    }
    fn from_mean(v: i32) -> Self {
        v as i64 as u64
    }
    fn to_f64(self) -> f64 {
        self as f64
    }
    fn from_f64(v: f64) -> Self {
        v as u64
    }
    fn put(self, out: &mut dyn OutStream) -> bool {
        out.put_u64(self)
    }
    fn get(inp: &mut dyn InStream) -> Option<Self> {
        inp.get_u64()
    }
}

macro_rules! float_elem {
    ($t:ty, $clsid:expr, $name:expr, $put:ident, $get:ident) => {
        impl GridElem for $t {
            const CLSID: u32 = $clsid;
            const NAME: &'static str = $name;
            const FLOAT: bool = true;
            fn sum_term(self) -> i64 {
                self as i64
            }
            fn from_quotient(q: f64) -> Self {
                q as $t
            }
            fn mean_term(self) -> i32 {
                self as i32
            }
            fn from_mean(v: i32) -> Self {
                v as $t
            }
            fn to_f64(self) -> f64 {
                self as f64
            }
            fn from_f64(v: f64) -> Self {
                v as $t
            }
            fn put(self, out: &mut dyn OutStream) -> bool {
                out.$put(self)
            }
            fn get(inp: &mut dyn InStream) -> Option<Self> {
                inp.$get()
            }
        }
    };
}

float_elem!(f32, 0x49B9_E60A, "f32", put_f32, get_f32);
float_elem!(f64, 0x49B9_E60B, "f64", put_f64, get_f64);

macro_rules! radius_elem {
    ($t:ty) => {
        impl RadiusElem for $t {
            const MIN: i64 = <$t>::MIN as i64;
            const MAX: i64 = <$t>::MAX as i64;
            fn to_i64(self) -> i64 {
                self as i64
            }
            fn clamped(v: i64) -> Self {
                v.clamp(<Self as RadiusElem>::MIN, <Self as RadiusElem>::MAX) as $t
            }
        }
    };
}

radius_elem!(u8);
radius_elem!(i8);
radius_elem!(i16);
