// SPDX-License-Identifier: GPL-3.0-or-later
//! Private static-link observation transport; no public DLL exports.
use fc4_core::legacy_control::observation::{replay, Operation, OwnerState};

mod recording;

// Words: refs, control, parent, procedure, own. Output adds a scalar at [5].
// Safety: plugin supplies disjoint live arrays of five and six u32 respectively.
#[no_mangle]
unsafe extern "C" fn fc4_legacyctl_shadow_replay(
    before: *const u32,
    op: u32,
    argument: u32,
    output: *mut u32,
) -> i32 {
    if before.is_null() || output.is_null() {
        return 0;
    }
    let b = unsafe { std::slice::from_raw_parts(before, 5) };
    if b[4] > 1 {
        return 0;
    }
    let handle = |v: u32| if v == 0 { None } else { Some(u64::from(v)) };
    let state = OwnerState {
        refs: b[0],
        control: handle(b[1]),
        parent: handle(b[2]),
        procedure: handle(b[3]),
        own: b[4] != 0,
    };
    let operation = match op {
        0 => Operation::AddRef,
        1 => Operation::SetOwnControl(argument != 0),
        2 => Operation::GetControl,
        3 => Operation::QueryInterface(argument),
        4 => Operation::ImplementInterface([0; 4]),
        5 | 6 => Operation::AreaStub,
        _ => return 0,
    };
    let (s, value) = replay(state, operation);
    let out = unsafe { std::slice::from_raw_parts_mut(output, 6) };
    out.copy_from_slice(&[
        s.refs,
        s.control.unwrap_or(0) as u32,
        s.parent.unwrap_or(0) as u32,
        s.procedure.unwrap_or(0) as u32,
        u32::from(s.own),
        value as u32,
    ]);
    1
}
