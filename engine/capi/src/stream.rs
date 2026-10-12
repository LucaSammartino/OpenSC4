// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Host streams over the C ABI (`fc4_ostream` / `fc4_istream` in fc4.h),
//! adapted to the core's `OutStream` / `InStream`. The GZCOM shim fills these
//! from the game's `cIGZOStream` / `cIGZIStream` (and segment streams for
//! references).

#![allow(non_camel_case_types)]

use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::stream::{InStream, OutStream, RefValue, SaveObject};

/// An object reference crossing the ABI (foundations §2.7).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct fc4_ref {
    /// Host identity (an interface pointer); 0 = null reference.
    pub token: u64,
    pub class_id: u32,
    pub reserved: u32,
    /// On reads: the core handle when the object is one of ours (e.g. an
    /// `fc4_department*`), else NULL.
    pub core_handle: *mut c_void,
}

impl Default for fc4_ref {
    fn default() -> Self {
        fc4_ref {
            token: 0,
            class_id: 0,
            reserved: 0,
            core_handle: std::ptr::null_mut(),
        }
    }
}

pub type fc4_bytes_sink = Option<unsafe extern "C" fn(ctx: *mut c_void, data: *const u8, len: usize)>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_ostream {
    pub struct_size: u32,
    pub user: *mut c_void,
    pub put_u8: Option<unsafe extern "C" fn(*mut c_void, u8) -> bool>,
    pub put_u16: Option<unsafe extern "C" fn(*mut c_void, u16) -> bool>,
    pub put_u32: Option<unsafe extern "C" fn(*mut c_void, u32) -> bool>,
    pub put_u64: Option<unsafe extern "C" fn(*mut c_void, u64) -> bool>,
    pub put_f32: Option<unsafe extern "C" fn(*mut c_void, f32) -> bool>,
    pub put_f64: Option<unsafe extern "C" fn(*mut c_void, f64) -> bool>,
    pub put_bytes: Option<unsafe extern "C" fn(*mut c_void, *const u8, usize) -> bool>,
    pub put_str: Option<unsafe extern "C" fn(*mut c_void, *const u8, usize) -> bool>,
    pub put_ref: Option<unsafe extern "C" fn(*mut c_void, *const fc4_ref) -> bool>,
    pub error: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_istream {
    pub struct_size: u32,
    pub user: *mut c_void,
    pub get_u8: Option<unsafe extern "C" fn(*mut c_void, *mut u8) -> bool>,
    pub get_u16: Option<unsafe extern "C" fn(*mut c_void, *mut u16) -> bool>,
    pub get_u32: Option<unsafe extern "C" fn(*mut c_void, *mut u32) -> bool>,
    pub get_u64: Option<unsafe extern "C" fn(*mut c_void, *mut u64) -> bool>,
    pub get_f32: Option<unsafe extern "C" fn(*mut c_void, *mut f32) -> bool>,
    pub get_f64: Option<unsafe extern "C" fn(*mut c_void, *mut f64) -> bool>,
    pub get_bytes: Option<unsafe extern "C" fn(*mut c_void, *mut u8, usize) -> bool>,
    pub get_str: Option<unsafe extern "C" fn(*mut c_void, fc4_bytes_sink, *mut c_void) -> bool>,
    /// Reads a reference, QueryInterfaced to `iid` (0 = any). Host objects are
    /// returned with one reference owned by the caller.
    pub get_ref: Option<unsafe extern "C" fn(*mut c_void, u32, *mut fc4_ref) -> bool>,
    pub skip: Option<unsafe extern "C" fn(*mut c_void, usize) -> bool>,
    pub error: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
}

/// Turns a core handle received through `fc4_ref::core_handle` back into the
/// core object, by class. Implemented by the subsystem modules of this crate.
pub(crate) type HandleDecoder = fn(class_id: u32, handle: *mut c_void) -> Option<Rc<dyn SaveObject>>;

/// `OutStream` over an `fc4_ostream`.
pub(crate) struct COut<'a>(pub &'a fc4_ostream);

macro_rules! call_put {
    ($s:expr, $f:ident, $($arg:expr),*) => {
        // SAFETY: the host promised valid callbacks for the stream's lifetime.
        match $s.0.$f { Some(f) => unsafe { f($s.0.user, $($arg),*) }, None => false }
    };
}

impl OutStream for COut<'_> {
    fn put_u8(&mut self, v: u8) -> bool {
        call_put!(self, put_u8, v)
    }
    fn put_u16(&mut self, v: u16) -> bool {
        call_put!(self, put_u16, v)
    }
    fn put_u32(&mut self, v: u32) -> bool {
        call_put!(self, put_u32, v)
    }
    fn put_u64(&mut self, v: u64) -> bool {
        call_put!(self, put_u64, v)
    }
    fn put_f32(&mut self, v: f32) -> bool {
        call_put!(self, put_f32, v)
    }
    fn put_f64(&mut self, v: f64) -> bool {
        call_put!(self, put_f64, v)
    }
    fn put_bytes(&mut self, v: &[u8]) -> bool {
        call_put!(self, put_bytes, v.as_ptr(), v.len())
    }
    fn put_str(&mut self, v: &[u8]) -> bool {
        // Same writer rule as the byte codec: no NUL, at most 200,000 bytes.
        let end = v
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(v.len())
            .min(fc4_core::stream::MAX_STRING_LEN as usize);
        call_put!(self, put_str, v.as_ptr(), end)
    }
    fn put_ref(&mut self, r: &RefValue) -> bool {
        let rf = match r {
            RefValue::Null => fc4_ref::default(),
            RefValue::Object(o) => fc4_ref {
                token: o.host_token(),
                class_id: o.class_id(),
                ..Default::default()
            },
            RefValue::Host { token, class_id } => fc4_ref {
                token: *token,
                class_id: *class_id,
                ..Default::default()
            },
        };
        call_put!(self, put_ref, &rf)
    }
    fn error(&self) -> i32 {
        // SAFETY: as above.
        match self.0.error {
            Some(f) => unsafe { f(self.0.user) },
            None => 0,
        }
    }
}

/// `InStream` over an `fc4_istream`.
pub(crate) struct CIn<'a> {
    pub stream: &'a fc4_istream,
    pub decode: HandleDecoder,
}

macro_rules! call_get {
    ($s:expr, $f:ident, $t:ty) => {{
        let mut v: $t = Default::default();
        // SAFETY: valid callbacks; `v` outlives the call.
        let ok = match $s.stream.$f {
            Some(f) => unsafe { f($s.stream.user, &mut v) },
            None => false,
        };
        ok.then_some(v)
    }};
}

unsafe extern "C" fn collect_bytes(ctx: *mut c_void, data: *const u8, len: usize) {
    let v = &mut *(ctx as *mut Vec<u8>);
    if !data.is_null() && len > 0 {
        v.extend_from_slice(std::slice::from_raw_parts(data, len));
    }
}

impl CIn<'_> {
    fn ref_with(&mut self, iid: u32) -> Option<RefValue> {
        let mut out = fc4_ref::default();
        // SAFETY: valid callback; `out` outlives the call.
        let ok = match self.stream.get_ref {
            Some(f) => unsafe { f(self.stream.user, iid, &mut out) },
            None => false,
        };
        if !ok {
            return None;
        }
        if out.token == 0 {
            return Some(RefValue::Null);
        }
        if !out.core_handle.is_null() {
            if let Some(o) = (self.decode)(out.class_id, out.core_handle) {
                return Some(RefValue::Object(o));
            }
        }
        Some(RefValue::Host {
            token: out.token,
            class_id: out.class_id,
        })
    }
}

impl InStream for CIn<'_> {
    fn get_u8(&mut self) -> Option<u8> {
        call_get!(self, get_u8, u8)
    }
    fn get_u16(&mut self) -> Option<u16> {
        call_get!(self, get_u16, u16)
    }
    fn get_u32(&mut self) -> Option<u32> {
        call_get!(self, get_u32, u32)
    }
    fn get_u64(&mut self) -> Option<u64> {
        call_get!(self, get_u64, u64)
    }
    fn get_f32(&mut self) -> Option<f32> {
        call_get!(self, get_f32, f32)
    }
    fn get_f64(&mut self) -> Option<f64> {
        call_get!(self, get_f64, f64)
    }
    fn get_bytes(&mut self, n: usize) -> Option<Vec<u8>> {
        let mut v = vec![0u8; n];
        // SAFETY: valid callback; `v` has room for n bytes.
        let ok = match self.stream.get_bytes {
            Some(f) => unsafe { f(self.stream.user, v.as_mut_ptr(), n) },
            None => false,
        };
        ok.then_some(v)
    }
    fn get_str(&mut self) -> Option<Vec<u8>> {
        let mut v: Vec<u8> = Vec::new();
        // SAFETY: valid callback; the sink only appends to `v` during the call.
        let ok = match self.stream.get_str {
            Some(f) => unsafe {
                f(
                    self.stream.user,
                    Some(collect_bytes),
                    &mut v as *mut Vec<u8> as *mut c_void,
                )
            },
            None => false,
        };
        ok.then_some(v)
    }
    fn get_ref(&mut self) -> Option<RefValue> {
        self.ref_with(0)
    }
    fn get_ref_as(&mut self, iid: u32) -> Option<RefValue> {
        self.ref_with(iid)
    }
    fn skip(&mut self, n: usize) -> bool {
        // SAFETY: valid callback.
        match self.stream.skip {
            Some(f) => unsafe { f(self.stream.user, n) },
            None => false,
        }
    }
    fn error(&self) -> i32 {
        // SAFETY: valid callback.
        match self.stream.error {
            Some(f) => unsafe { f(self.stream.user) },
            None => 0,
        }
    }
}

/// Checks that a host stream struct is complete enough to use.
pub(crate) fn ostream_ok(s: &fc4_ostream) -> bool {
    (s.struct_size as usize) >= std::mem::size_of::<fc4_ostream>()
}

pub(crate) fn istream_ok(s: &fc4_istream) -> bool {
    (s.struct_size as usize) >= std::mem::size_of::<fc4_istream>()
}
