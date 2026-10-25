// SPDX-License-Identifier: GPL-3.0-or-later
//! Private construction record owner transport. Not a registered replacement:
//! the plugin must supply lifecycle, scheduling and rendering before activation.
//! All calls on a handle must be serialized by its plugin owner. In particular,
//! getter calls from the game's second thread use the same owner lock.

use std::ffi::c_void;

use fc4_core::lot_records::construction::ConstructionRecord;
use fc4_core::lot_records::OccupantRecord;

use crate::guard;
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};

/// The C++ adapter owns every reference acquired by its input stream. Keeping
/// references as host tokens prevents a record Read from examining unresolved
/// game objects or making thread-affine Rc objects from another core owner.
#[no_mangle]
pub extern "C" fn fc4_construction_record_create() -> *mut c_void {
    guard(std::ptr::null_mut(), || {
        Box::into_raw(Box::new(ConstructionRecord::new())).cast()
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_construction_record_destroy(owner: *mut c_void) {
    if !owner.is_null() {
        guard((), || drop(Box::from_raw(owner.cast::<ConstructionRecord>())));
    }
}

/// Read into a candidate: a failed record must not leave old live fields
/// pointing at references the adapter has just released on its failure path.
#[no_mangle]
pub unsafe extern "C" fn fc4_construction_record_read(owner: *mut c_void, input: *const fc4_istream) -> bool {
    guard(false, || {
        let (Some(owner), Some(input)) = (owner.cast::<ConstructionRecord>().as_mut(), input.as_ref()) else {
            return false;
        };
        if !istream_ok(input) {
            return false;
        }
        let mut candidate = ConstructionRecord::new();
        let mut stream = CIn {
            stream: input,
            decode: |_, _| None,
        };
        if !candidate.read(&mut stream) {
            return false;
        }
        *owner = candidate;
        true
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_construction_record_write(owner: *const c_void, output: *const fc4_ostream) -> bool {
    guard(false, || {
        let (Some(owner), Some(output)) = (owner.cast::<ConstructionRecord>().as_ref(), output.as_ref()) else {
            return false;
        };
        ostream_ok(output) && owner.write(&mut COut(output))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::{fc4_bytes_sink, fc4_ref};
    use crate::test_streams::{istream, ostream, In, Out};

    // Unlike the shared byte-buffer test stream, a real segment stream reports
    // exhausted input through GetError (lot-records 9.1.1).
    struct CheckedInput {
        base: fc4_istream,
        failed: bool,
    }

    impl CheckedInput {
        fn api(&mut self) -> fc4_istream {
            macro_rules! track {
                ($name:ident $(, $arg:ident: $ty:ty)*) => {
                    unsafe extern "C" fn $name(u: *mut c_void, $($arg: $ty),*) -> bool {
                        let input = &mut *u.cast::<CheckedInput>();
                        let ok = input.base.$name
                            .map(|f| f(input.base.user, $($arg),*))
                            .unwrap_or(false);
                        input.failed |= !ok;
                        ok
                    }
                };
            }
            track!(get_u8, out: *mut u8);
            track!(get_u16, out: *mut u16);
            track!(get_u32, out: *mut u32);
            track!(get_u64, out: *mut u64);
            track!(get_f32, out: *mut f32);
            track!(get_f64, out: *mut f64);
            track!(get_bytes, out: *mut u8, count: usize);
            track!(get_str, sink: fc4_bytes_sink, context: *mut c_void);
            track!(get_ref, iid: u32, out: *mut fc4_ref);
            track!(skip, count: usize);
            unsafe extern "C" fn error(u: *mut c_void) -> i32 {
                let input = &*u.cast::<CheckedInput>();
                i32::from(input.failed)
            }
            fc4_istream {
                struct_size: std::mem::size_of::<fc4_istream>() as u32,
                user: (self as *mut Self).cast(),
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
                error: Some(error),
            }
        }
    }

    #[test]
    fn record_factory_owns_independent_core_records() {
        let a = fc4_construction_record_create();
        let b = fc4_construction_record_create();
        assert!(!a.is_null() && !b.is_null() && a != b);
        unsafe {
            let a_rec = &mut *a.cast::<ConstructionRecord>();
            a_rec.world_bounds = ([1.0, 2.0, 3.0], [5.0, 6.0, 7.0]);
            let b_rec = &*b.cast::<ConstructionRecord>();
            assert_eq!(b_rec.world_bounds, ([0.0; 3], [0.0; 3]));
            fc4_construction_record_destroy(a);
            fc4_construction_record_destroy(b);
            fc4_construction_record_destroy(std::ptr::null_mut());
        }
    }

    // C stream callbacks intentionally reject the first primitive: a failed
    // Read must preserve a previously loaded record, not silently reset it.
    unsafe extern "C" fn reject_u16(_: *mut c_void, _: *mut u16) -> bool {
        false
    }

    #[test]
    fn rejected_read_keeps_existing_record() {
        let a = fc4_construction_record_create();
        unsafe {
            let rec = &mut *a.cast::<ConstructionRecord>();
            rec.scalars.next_day = 12345;
            // Full callback table shape; only the first read is reached.
            let mut input: fc4_istream = std::mem::zeroed();
            input.struct_size = std::mem::size_of::<fc4_istream>() as u32;
            input.get_u16 = Some(reject_u16);
            assert!(!fc4_construction_record_read(a, &input));
            assert_eq!((*a.cast::<ConstructionRecord>()).scalars.next_day, 12345);
            fc4_construction_record_destroy(a);
        }
    }

    #[test]
    fn constructor_record_is_the_spec_version_and_type() {
        let a = fc4_construction_record_create();
        unsafe {
            let rec = &*a.cast::<ConstructionRecord>();
            let mut output = Out::default();
            assert!(fc4_construction_record_write(a, &ostream(&mut output)));
            assert_eq!(&output.bytes[..2], &5u16.to_le_bytes());
            assert_eq!(rec.base.occupant_type, 0x096E763C);
            let mut input = In::new(&output.bytes);
            assert!(fc4_construction_record_read(a, &istream(&mut input)));
            fc4_construction_record_destroy(a);
        }
    }

    #[test]
    fn truncated_read_does_not_replace_a_previously_loaded_record() {
        let a = fc4_construction_record_create();
        unsafe {
            let rec = &mut *a.cast::<ConstructionRecord>();
            rec.scalars.next_day = 12345;
            let mut before = Out::default();
            assert!(fc4_construction_record_write(a, &ostream(&mut before)));
            // Exercise every partial base and construction-field boundary,
            // including failure after the final reference has been consumed.
            for length in 0..before.bytes.len() {
                let mut input = In::new(&before.bytes[..length]);
                let mut checked = CheckedInput {
                    base: istream(&mut input),
                    failed: false,
                };
                assert!(
                    !fc4_construction_record_read(a, &checked.api()),
                    "accepted truncated record of length {length}"
                );
                let mut after = Out::default();
                assert!(fc4_construction_record_write(a, &ostream(&mut after)));
                assert_eq!(before.bytes, after.bytes, "changed at length {length}");
            }
            // A later complete Read must still commit its candidate.
            let b = fc4_construction_record_create();
            (*b.cast::<ConstructionRecord>()).scalars.next_day = 67890;
            let mut replacement = Out::default();
            assert!(fc4_construction_record_write(b, &ostream(&mut replacement)));
            let mut input = In::new(&replacement.bytes);
            let mut checked = CheckedInput {
                base: istream(&mut input),
                failed: false,
            };
            assert!(fc4_construction_record_read(a, &checked.api()));
            assert!(!checked.failed);
            assert_eq!((*a.cast::<ConstructionRecord>()).scalars.next_day, 67890);
            fc4_construction_record_destroy(b);
            fc4_construction_record_destroy(a);
        }
    }
}
