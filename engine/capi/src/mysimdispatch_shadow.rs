// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded constructor/record-header differential, not a live trip replay.
//! MS20 blocks callable dispatch slots and input interception. Native state is
//! never used to construct the expected DispatchState.
use fc4_core::mysims::{dispatch::DispatchState, DISPATCH_CLSID};

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
