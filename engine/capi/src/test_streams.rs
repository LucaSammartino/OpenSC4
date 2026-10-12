// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Byte-buffer host streams for the C ABI tests. References are written as
//! `u64 token, u32 class` and read back as the same token and class (the
//! test's `resolve` hook may fill in a core handle).

use std::ffi::c_void;

use crate::stream::{fc4_bytes_sink, fc4_istream, fc4_ostream, fc4_ref};

#[derive(Default)]
pub struct Out {
    pub bytes: Vec<u8>,
    pub refs: Vec<(u64, u32)>,
}

fn out_of(u: *mut c_void) -> &'static mut Out {
    unsafe { &mut *(u as *mut Out) }
}

macro_rules! put {
    ($name:ident, $t:ty) => {
        unsafe extern "C" fn $name(u: *mut c_void, v: $t) -> bool {
            out_of(u).bytes.extend_from_slice(&v.to_le_bytes());
            true
        }
    };
}
put!(put_u8, u8);
put!(put_u16, u16);
put!(put_u32, u32);
put!(put_u64, u64);
put!(put_f32, f32);
put!(put_f64, f64);

unsafe extern "C" fn put_bytes(u: *mut c_void, p: *const u8, n: usize) -> bool {
    if n > 0 {
        out_of(u).bytes.extend_from_slice(std::slice::from_raw_parts(p, n));
    }
    true
}
/// Strings: u32 length, then the bytes.
unsafe extern "C" fn put_str(u: *mut c_void, p: *const u8, n: usize) -> bool {
    put_u32(u, n as u32);
    put_bytes(u, p, n)
}
unsafe extern "C" fn put_ref(u: *mut c_void, r: *const fc4_ref) -> bool {
    let r = &*r;
    let o = out_of(u);
    o.refs.push((r.token, r.class_id));
    o.bytes.extend_from_slice(&r.token.to_le_bytes());
    o.bytes.extend_from_slice(&r.class_id.to_le_bytes());
    true
}
unsafe extern "C" fn no_error(_: *mut c_void) -> i32 {
    0
}

pub fn ostream(o: &mut Out) -> fc4_ostream {
    fc4_ostream {
        struct_size: std::mem::size_of::<fc4_ostream>() as u32,
        user: o as *mut Out as *mut c_void,
        put_u8: Some(put_u8),
        put_u16: Some(put_u16),
        put_u32: Some(put_u32),
        put_u64: Some(put_u64),
        put_f32: Some(put_f32),
        put_f64: Some(put_f64),
        put_bytes: Some(put_bytes),
        put_str: Some(put_str),
        put_ref: Some(put_ref),
        error: Some(no_error),
    }
}

pub struct In<'a> {
    pub bytes: &'a [u8],
    pub pos: usize,
    /// Fills in a reference read back (token, class) → core handle.
    pub resolve: Option<&'a dyn Fn(&mut fc4_ref)>,
}

impl<'a> In<'a> {
    pub fn new(bytes: &'a [u8]) -> In<'a> {
        In {
            bytes,
            pos: 0,
            resolve: None,
        }
    }
}

fn in_of(u: *mut c_void) -> &'static mut In<'static> {
    unsafe { &mut *(u as *mut In<'static>) }
}
fn take<const N: usize>(u: *mut c_void) -> Option<[u8; N]> {
    let i = in_of(u);
    let b: [u8; N] = i.bytes.get(i.pos..i.pos + N)?.try_into().ok()?;
    i.pos += N;
    Some(b)
}

macro_rules! get {
    ($name:ident, $t:ty, $n:expr) => {
        unsafe extern "C" fn $name(u: *mut c_void, v: *mut $t) -> bool {
            take::<$n>(u).map(|b| *v = <$t>::from_le_bytes(b)).is_some()
        }
    };
}
get!(get_u8, u8, 1);
get!(get_u16, u16, 2);
get!(get_u32, u32, 4);
get!(get_u64, u64, 8);
get!(get_f32, f32, 4);
get!(get_f64, f64, 8);

unsafe extern "C" fn get_bytes(u: *mut c_void, p: *mut u8, n: usize) -> bool {
    let i = in_of(u);
    match i.bytes.get(i.pos..i.pos + n) {
        Some(s) => {
            if n > 0 {
                std::ptr::copy_nonoverlapping(s.as_ptr(), p, n);
            }
            i.pos += n;
            true
        }
        None => false,
    }
}
unsafe extern "C" fn get_str(u: *mut c_void, sink: fc4_bytes_sink, ctx: *mut c_void) -> bool {
    let Some(n) = take::<4>(u).map(u32::from_le_bytes) else { return false };
    let i = in_of(u);
    let Some(s) = i.bytes.get(i.pos..i.pos + n as usize) else { return false };
    i.pos += n as usize;
    if let Some(f) = sink {
        f(ctx, s.as_ptr(), s.len());
    }
    true
}
unsafe extern "C" fn get_ref(u: *mut c_void, _iid: u32, out: *mut fc4_ref) -> bool {
    let Some(token) = take::<8>(u).map(u64::from_le_bytes) else { return false };
    let Some(class) = take::<4>(u).map(u32::from_le_bytes) else { return false };
    let mut r = fc4_ref {
        token,
        class_id: class,
        ..fc4_ref::default()
    };
    if let Some(f) = in_of(u).resolve {
        f(&mut r);
    }
    *out = r;
    true
}
unsafe extern "C" fn skip(u: *mut c_void, n: usize) -> bool {
    let i = in_of(u);
    if i.pos + n > i.bytes.len() {
        return false;
    }
    i.pos += n;
    true
}

pub fn istream(i: &mut In) -> fc4_istream {
    fc4_istream {
        struct_size: std::mem::size_of::<fc4_istream>() as u32,
        user: i as *mut In as *mut c_void,
        get_u8: Some(get_u8),
        get_u16: Some(get_u16),
        get_u32: Some(get_u32),
        get_u64: Some(get_u64),
        get_f32: Some(get_f32),
        get_f64: Some(get_f64),
        get_bytes: Some(get_bytes),
        get_str: Some(get_str),
        get_ref: Some(get_ref),
        skip: Some(skip),
        error: Some(no_error),
    }
}
