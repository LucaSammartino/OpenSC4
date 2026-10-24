// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded constructor/record-header differential, not a live trip replay.
//! MS20 specifies assigned constructor fields and the public observation
//! limitation. Native state never constructs the expected DispatchState.
use fc4_core::mysims::dispatch::DispatchRecordRefs;
use fc4_core::mysims::{dispatch::DispatchState, DISPATCH_CLSID};
use fc4_core::stream::{ByteWriter, RefValue};

struct NoRefs;
impl DispatchRecordRefs for NoRefs {
    fn reference(&self, _: u64) -> RefValue {
        RefValue::Null
    }
    fn token(&self, _: RefValue) -> Option<u64> {
        None
    }
}

fn constructor_payload() -> Vec<u8> {
    let mut out = ByteWriter::new();
    assert!(DispatchState::default().write(&mut out, &NoRefs));
    out.bytes().to_vec()
}

fn assigned_byte(index: usize) -> bool {
    // §7.2.3: +20 location tuning, +28 opaque word, +2C/+30 leg
    // indices and +58 retry count have no promised constructor stores.
    // MS22: +10 remains unpromised. +FC and +100 are compared semantically
    // after the plugin decodes references, without assuming their byte width.
    index < 229 && !matches!(index, 4 | 17..=20 | 25..=36 | 73..=76)
}

// Static-link transport only. Exclude unassigned words, not differences found
// experimentally; expected bytes are produced before reading native outputs.
#[no_mangle]
unsafe extern "C" fn fc4_mysimdispatch_shadow_constructor(native: *const u8, length: usize) -> u32 {
    let expected = constructor_payload();
    if native.is_null() || length < 229 {
        return u32::MAX;
    }
    // SAFETY: plugin supplies its captured payload and exact byte count.
    let native = unsafe { std::slice::from_raw_parts(native, length) };
    native
        .iter()
        .zip(expected)
        .enumerate()
        .filter(|(index, (a, b))| assigned_byte(*index) && **a != *b)
        .count() as u32
}

fn expected() -> [u32; 7] {
    let state = DispatchState::default();
    [
        DISPATCH_CLSID,
        1, // mysims §6.2.1: writer version (the reader accepts other versions).
        state.search.state as u32,
        state.search.prev_state as u32,
        state.search.flags,
        state.minutes as u32,
        state.travel_type,
    ]
}

// Private snapshot-contract checks. These are not independent trip replay:
// the ABI has no observer for direct requests/ordered services (MS20).
#[no_mangle]
unsafe extern "C" fn fc4_mysimdispatch_shadow_auxiliary(native: *const u32) -> u32 {
    if native.is_null() {
        return u32::MAX;
    }
    // SAFETY: plugin supplies [auxiliary-reference-is-null, auxiliary-scalar].
    let values = unsafe { std::slice::from_raw_parts(native, 2) };
    u32::from(values[0] != 1) + u32::from(values[1] != DispatchState::default().auxiliary_value)
}

#[no_mangle]
unsafe extern "C" fn fc4_mysimdispatch_shadow_live_range(native: *const u32) -> u32 {
    if native.is_null() {
        return u32::MAX;
    }
    // SAFETY: plugin supplies [exclusive end, inclusive start, marker count]
    // only for initialized route-ready dispatches. Empty/exhausted is valid.
    let values = unsafe { std::slice::from_raw_parts(native, 3) };
    let (end, start, count) = (values[0] as i32, values[1] as i32, values[2]);
    u32::from(start < 0) + u32::from(end < start) + u32::from(end < 0 || end as u32 > count)
}

// Private static-link transport; no public C ABI export or header entry.
#[no_mangle]
unsafe extern "C" fn fc4_mysimdispatch_shadow_compare(native: *const u32, constructor: u32) -> u32 {
    if native.is_null() || constructor > 1 {
        return u32::MAX;
    }
    let count = if constructor == 1 { 7 } else { 2 };
    // SAFETY: plugin supplies seven readable u32 values for this call.
    let observed = unsafe { std::slice::from_raw_parts(native, count) };
    observed.iter().zip(expected()).filter(|(a, b)| **a != *b).count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ms22_auxiliary_semantics_and_ms21_live_header_bounds() {
        assert_eq!(unsafe { fc4_mysimdispatch_shadow_auxiliary([1, 0].as_ptr()) }, 0);
        assert_eq!(unsafe { fc4_mysimdispatch_shadow_auxiliary([0, 7].as_ptr()) }, 2);
        for range in [[2, 0, 5], [4, 3, 5], [5, 5, 5], [2, 2, 5]] {
            assert_eq!(unsafe { fc4_mysimdispatch_shadow_live_range(range.as_ptr()) }, 0);
        }
        assert_eq!(unsafe { fc4_mysimdispatch_shadow_live_range([6, 0, 5].as_ptr()) }, 1);
        assert_eq!(unsafe { fc4_mysimdispatch_shadow_live_range([2, 3, 5].as_ptr()) }, 1);
        assert_eq!(
            unsafe { fc4_mysimdispatch_shadow_live_range([2, u32::MAX, 5].as_ptr()) },
            1
        );
    }

    #[test]
    fn constructor_compares_all_and_only_promised_payload_bytes() {
        let mut payload = constructor_payload();
        assert_eq!(payload.len(), 269);
        assert_eq!((0..payload.len()).filter(|i| assigned_byte(*i)).count(), 208);
        assert_eq!(
            unsafe { fc4_mysimdispatch_shadow_constructor(payload.as_ptr(), payload.len()) },
            0
        );
        for (i, byte) in payload.iter_mut().enumerate() {
            if !assigned_byte(i) {
                *byte ^= 0xFF;
            }
        }
        assert_eq!(
            unsafe { fc4_mysimdispatch_shadow_constructor(payload.as_ptr(), payload.len()) },
            0
        );
        for (i, byte) in payload.iter_mut().enumerate() {
            if assigned_byte(i) {
                *byte ^= 1;
            }
        }
        assert_eq!(
            unsafe { fc4_mysimdispatch_shadow_constructor(payload.as_ptr(), payload.len()) },
            208
        );
    }

    #[test]
    fn counts_every_compared_field_and_does_not_seed_from_native() {
        let mut native = expected();
        assert_eq!(unsafe { fc4_mysimdispatch_shadow_compare(native.as_ptr(), 1) }, 0);
        for value in &mut native {
            *value ^= 1;
        }
        assert_eq!(unsafe { fc4_mysimdispatch_shadow_compare(native.as_ptr(), 1) }, 7);
        assert_eq!(unsafe { fc4_mysimdispatch_shadow_compare(native.as_ptr(), 0) }, 2);
    }

    #[test]
    fn live_header_never_claims_to_compare_trip_state() {
        let mut native = expected();
        native[2..].fill(123);
        assert_eq!(unsafe { fc4_mysimdispatch_shadow_compare(native.as_ptr(), 0) }, 0);
        assert_eq!(
            unsafe { fc4_mysimdispatch_shadow_compare(std::ptr::null(), 0) },
            u32::MAX
        );
    }
}
