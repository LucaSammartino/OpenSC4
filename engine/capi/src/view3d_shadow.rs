// SPDX-License-Identifier: GPL-3.0-or-later
//! Private static-link invocation transport; absent from the public DLL exports.
use fc4_core::ui_windows::view3d::shadow::{replay, WORDS};

#[no_mangle]
pub unsafe extern "C" fn fc4_view3d_shadow_replay(
    slot: u32,
    input: *const [u32; WORDS],
    output: *mut [u32; WORDS],
    result: *mut u32,
    reads: *mut u32,
) -> bool {
    if input.is_null() || output.is_null() || result.is_null() || reads.is_null() {
        return false;
    }
    // SAFETY: the private caller supplies live fixed-size input/output packets.
    let Some((value, calls, state)) = replay(slot, unsafe { &*input }) else {
        return false;
    };
    // SAFETY: all caller-provided outputs were checked above.
    unsafe {
        *output = state;
        *result = value;
        *reads = calls;
    }
    true
}
