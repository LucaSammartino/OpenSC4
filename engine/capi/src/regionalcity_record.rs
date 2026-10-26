// SPDX-License-Identifier: GPL-3.0-or-later
//! Wire transport with host-owned elements. No neighbour layouts cross FFI.
use super::{handles::*, City, Owner};
use crate::stream::{fc4_istream, fc4_ostream, CIn, COut};
use fc4_core::regional_city::{RecordElements, RecordReader};
use fc4_core::stream::{InStream, OutStream};
use std::ffi::c_void;

/// `read` consumes from the SAME stream as the enclosing record and returns
/// one owned interface reference (null on failure). `write`: -1 means no
/// serializable interface, 0 failure, 1 success. kind: 0 connection, 1 deal.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RecordCallbacks {
    pub context: *mut c_void,
    pub read: unsafe extern "C" fn(*mut c_void, u32, *const fc4_istream) -> *mut c_void,
    pub write: unsafe extern "C" fn(*mut c_void, u32, *mut c_void, *const fc4_ostream) -> i32,
    pub connections: ConnectionCallbacks,
    pub deals: DealCallbacks,
}

struct Reader<'a> {
    calls: &'a RecordCallbacks,
    stream: &'a fc4_istream,
}
impl RecordReader<Connection, Deal> for Reader<'_> {
    fn connection(&mut self, _: &mut dyn InStream) -> Option<Connection> {
        unsafe {
            Connection::from_owned(
                (self.calls.read)(self.calls.context, 0, self.stream),
                self.calls.connections,
            )
        }
    }
    fn deal(&mut self, _: &mut dyn InStream) -> Option<Deal> {
        unsafe { Deal::from_owned((self.calls.read)(self.calls.context, 1, self.stream), self.calls.deals) }
    }
}
struct Writer<'a> {
    calls: &'a RecordCallbacks,
    stream: &'a fc4_ostream,
}
impl Writer<'_> {
    fn write(&self, kind: u32, object: *mut c_void) -> Option<bool> {
        match unsafe { (self.calls.write)(self.calls.context, kind, object, self.stream) } {
            -1 => None,
            1 => Some(true),
            _ => Some(false),
        }
    }
}
impl RecordElements<Connection, Deal> for Writer<'_> {
    fn connection(&mut self, c: &Connection, _: &mut dyn OutStream) -> Option<bool> {
        self.write(0, c.raw())
    }
    fn deal(&mut self, d: &Deal, _: &mut dyn OutStream) -> Option<bool> {
        self.write(1, d.raw())
    }
}

// Private static-link functions: not DLL exports. Pointers/callbacks must be
// live for each call. Callbacks never re-enter this owner. Element reference
// callbacks are copied into handles and must outlive those handles.
#[no_mangle]
pub unsafe extern "C" fn fc4_regionalcity_owner_decode(
    owner: *mut Owner,
    stream: *const fc4_istream,
    difficulty: u8,
    calls: *const RecordCallbacks,
) -> bool {
    let (Some(owner), Some(stream), Some(calls)) = (owner.as_ref(), stream.as_ref(), calls.as_ref()) else {
        return false;
    };
    if stream.struct_size < std::mem::size_of::<fc4_istream>() as u32 {
        return false;
    }
    let mut input = CIn {
        stream,
        decode: |_, _| None,
    };
    let Ok(decoded) = City::decode_record_with(&mut input, difficulty, &mut Reader { calls, stream }) else {
        return false;
    };
    if input.error() != 0 {
        return false;
    }
    // SPEC-GAP: the private decode transport commits atomically; object Read
    // localization, deal claiming and Init remain separate host operations.
    let Ok(mut city) = owner.0.lock() else {
        return false;
    };
    *city = decoded;
    true
}

#[no_mangle]
pub unsafe extern "C" fn fc4_regionalcity_owner_write(
    owner: *const Owner,
    stream: *const fc4_ostream,
    calls: *const RecordCallbacks,
) -> bool {
    let (Some(owner), Some(stream), Some(calls)) = (owner.as_ref(), stream.as_ref(), calls.as_ref()) else {
        return false;
    };
    if stream.struct_size < std::mem::size_of::<fc4_ostream>() as u32 {
        return false;
    }
    // Element serialization can call getters on the COM owner. Pin a coherent
    // snapshot, then release the state lock before entering host interfaces.
    let city = match owner.0.lock() {
        Ok(city) => city.clone(),
        Err(_) => return false,
    };
    city.write_record_with(&mut COut(stream), &mut Writer { calls, stream })
}

#[cfg(test)]
#[path = "regionalcity_record_tests.rs"]
mod tests;
