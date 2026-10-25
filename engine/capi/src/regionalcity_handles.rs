// SPDX-License-Identifier: GPL-3.0-or-later
//! Owned host interface references. Callbacks use only the supplied interface;
//! native layouts and Rust neighbour objects never cross this boundary.
use fc4_core::regional_city::{ConnectionHandle, DealHandle};
use std::ffi::c_void;

type Ref = unsafe extern "C" fn(*mut c_void);
type Pair = unsafe extern "C" fn(*mut c_void, *mut i32, *mut i32);
type SetPair = unsafe extern "C" fn(*mut c_void, i32, i32);
type Word = unsafe extern "C" fn(*mut c_void) -> u32;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ConnectionCallbacks {
    pub retain: Ref,
    pub release: Ref,
    pub conn_type: Word,
    pub cell: Pair,
    pub set_cell: SetPair,
    pub repair: unsafe extern "C" fn(*mut c_void, i32, i32, i32, i32, i32, i32),
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DealCallbacks {
    pub retain: Ref,
    pub release: Ref,
    pub city_side: Pair,
    pub other_side: Pair,
    pub set_city_side: SetPair,
    pub set_other_side: SetPair,
    pub deal_type: Word,
    pub amount: unsafe extern "C" fn(*mut c_void) -> i32,
    pub flags: Word,
    pub set_flags: unsafe extern "C" fn(*mut c_void, u32, bool) -> bool,
}

// The integer retains pointer provenance through the exposed-address cast.
// A host grants serialized cross-thread access for the reference lifetime.
pub struct Connection {
    object: usize,
    calls: ConnectionCallbacks,
}
pub struct Deal {
    object: usize,
    calls: DealCallbacks,
}

impl Connection {
    /// Takes the factory's one owned reference, including on later read failure.
    pub unsafe fn from_owned(object: *mut c_void, calls: ConnectionCallbacks) -> Option<Self> {
        if object.is_null() {
            return None;
        }
        Some(Self {
            object: object as usize,
            calls,
        })
    }
    /// Acquire a borrowed interface; callbacks must outlive the acquired ref.
    pub unsafe fn acquire(object: *mut c_void, calls: ConnectionCallbacks) -> Option<Self> {
        if object.is_null() {
            return None;
        }
        (calls.retain)(object);
        Some(Self {
            object: object as usize,
            calls,
        })
    }
    pub(super) fn raw(&self) -> *mut c_void {
        self.object as *mut c_void
    }
}
impl Clone for Connection {
    fn clone(&self) -> Self {
        unsafe {
            (self.calls.retain)(self.raw());
        }
        Self {
            object: self.object,
            calls: self.calls,
        }
    }
}
impl Drop for Connection {
    fn drop(&mut self) {
        unsafe {
            (self.calls.release)(self.raw());
        }
    }
}
impl ConnectionHandle for Connection {
    fn conn_type(&self) -> u32 {
        unsafe { (self.calls.conn_type)(self.raw()) }
    }
    fn cell(&self) -> (i32, i32) {
        let (mut x, mut z) = (0, 0);
        unsafe {
            (self.calls.cell)(self.raw(), &mut x, &mut z);
        }
        (x, z)
    }
    fn set_cell(&self, x: i32, z: i32) {
        unsafe {
            (self.calls.set_cell)(self.raw(), x, z);
        }
    }
    fn repair(&self, counts: (i32, i32), position: (i32, i32), size: (i32, i32)) {
        unsafe {
            (self.calls.repair)(self.raw(), counts.0, counts.1, position.0, position.1, size.0, size.1);
        }
    }
}

impl Deal {
    pub unsafe fn from_owned(object: *mut c_void, calls: DealCallbacks) -> Option<Self> {
        if object.is_null() {
            return None;
        }
        Some(Self {
            object: object as usize,
            calls,
        })
    }
    pub unsafe fn acquire(object: *mut c_void, calls: DealCallbacks) -> Option<Self> {
        if object.is_null() {
            return None;
        }
        (calls.retain)(object);
        Some(Self {
            object: object as usize,
            calls,
        })
    }
    pub(super) fn raw(&self) -> *mut c_void {
        self.object as *mut c_void
    }
    fn pair(&self, call: Pair) -> (i32, i32) {
        let (mut x, mut z) = (0, 0);
        unsafe {
            call(self.raw(), &mut x, &mut z);
        }
        (x, z)
    }
}
impl Clone for Deal {
    fn clone(&self) -> Self {
        unsafe {
            (self.calls.retain)(self.raw());
        }
        Self {
            object: self.object,
            calls: self.calls,
        }
    }
}
impl Drop for Deal {
    fn drop(&mut self) {
        unsafe {
            (self.calls.release)(self.raw());
        }
    }
}
impl DealHandle for Deal {
    fn city_side(&self) -> (i32, i32) {
        self.pair(self.calls.city_side)
    }
    fn other_side(&self) -> (i32, i32) {
        self.pair(self.calls.other_side)
    }
    fn set_city_side(&self, x: i32, z: i32) {
        unsafe {
            (self.calls.set_city_side)(self.raw(), x, z);
        }
    }
    fn set_other_side(&self, x: i32, z: i32) {
        unsafe {
            (self.calls.set_other_side)(self.raw(), x, z);
        }
    }
    fn deal_type(&self) -> u32 {
        unsafe { (self.calls.deal_type)(self.raw()) }
    }
    fn amount(&self) -> i32 {
        unsafe { (self.calls.amount)(self.raw()) }
    }
    fn flags(&self) -> u32 {
        unsafe { (self.calls.flags)(self.raw()) }
    }
    fn set_flags(&self, flags: u32, on: bool) -> bool {
        unsafe { (self.calls.set_flags)(self.raw(), flags, on) }
    }
}
