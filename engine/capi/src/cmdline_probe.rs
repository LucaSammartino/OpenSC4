// SPDX-License-Identifier: GPL-3.0-or-later
//! Private read-only native-query transport; all answers use production CmdLine.
//! The handle is created ONCE from independent GetCommandLineA bytes. No native
//! query result is accepted as parser state and there is no resynchronization.
use fc4_core::cmdline::CmdLine;
use std::ffi::{c_char, c_void, CStr};
use std::panic::{catch_unwind, AssertUnwindSafe};

const UNCHANGED: &[u8] = b"__fc4_unchanged__";

fn query(cmd: &CmdLine, op: u32, key: &[u8], param: i32) -> Option<(i32, Vec<u8>)> {
    let mut output = UNCHANGED.to_vec();
    let scalar = match op {
        0 => i32::try_from(cmd.argc()).ok()?,
        1 => {
            output = cmd.argv(param).ok()?.to_vec();
            0
        }
        2 => {
            output = cmd.get_command_line().to_vec();
            1
        }
        3 => cmd.find(key, param != 0),
        4 => cmd.find_sub_string(key, param != 0),
        5 if !key.is_empty() => i32::from(cmd.is_switch_present(key)),
        6 if !key.is_empty() => i32::from(cmd.is_switch_present_value(key, &mut output, param != 0)),
        7 if key.len() == 1 => i32::from(cmd.is_switch_present_char(key[0], &mut output, param != 0)),
        8 if !key.is_empty() => cmd.get_index_of_switch(key, param),
        9 if key.len() == 1 => cmd.get_index_of_switch_char(key[0], param),
        _ => return None,
    };
    Some((scalar, output))
}

/// `raw` must be a valid NUL-terminated independently captured process string.
/// Owns only a Rust production parser; never owns a native object.
#[no_mangle]
pub unsafe extern "C" fn fc4_cmdline_probe_create(raw: *const c_char) -> *mut c_void {
    catch_unwind(|| {
        if raw.is_null() {
            return std::ptr::null_mut();
        }
        // SAFETY: caller guarantees a readable terminated string.
        let bytes = unsafe { CStr::from_ptr(raw) }.to_bytes();
        Box::into_raw(Box::new(CmdLine::new(bytes))).cast()
    })
    .unwrap_or(std::ptr::null_mut())
}

/// Destroy exactly once a live handle returned by create; null is allowed.
#[no_mangle]
pub unsafe extern "C" fn fc4_cmdline_probe_destroy(handle: *mut c_void) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !handle.is_null() {
            // SAFETY: caller transfers back the Box created above.
            drop(unsafe { Box::from_raw(handle.cast::<CmdLine>()) });
        }
    }));
}

/// Return 0 on success, -1 invalid/unsupported input, -2 insufficient capacity,
/// -100 panic. Output bytes have an explicit length and no NUL terminator.
/// Caller supplies a live handle, a terminated key, writable length/scalar, and
/// writable output[capacity]. Failures never partially write query results.
#[no_mangle]
pub unsafe extern "C" fn fc4_cmdline_probe_query(
    handle: *const c_void,
    op: u32,
    key: *const c_char,
    param: i32,
    output: *mut u8,
    capacity: u32,
    length: *mut u32,
    scalar: *mut i32,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if handle.is_null() || key.is_null() || output.is_null() || length.is_null() || scalar.is_null() {
            return -1;
        }
        // SAFETY: all pointer validity and lifetime requirements are the caller's.
        let cmd = unsafe { &*handle.cast::<CmdLine>() };
        let key = unsafe { CStr::from_ptr(key) }.to_bytes();
        // CL3 unknown high-byte case behavior is outside this probe's scope.
        if !key.is_ascii() {
            return -1;
        }
        let Some((result, bytes)) = query(cmd, op, key, param) else {
            return -1;
        };
        if bytes.len() > capacity as usize {
            return -2;
        }
        // SAFETY: caller supplies sufficient non-overlapping writable buffers.
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), output, bytes.len());
            *length = bytes.len() as u32;
            *scalar = result;
        }
        0
    }))
    .unwrap_or(-100)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn independent_raw_input_and_query_output_contract() {
        let raw = CString::new("\"C:/Game/Sim City.exe\" -Intro:off /w -r1920x1080x32").unwrap();
        let key = CString::new("INTRO").unwrap();
        // SAFETY: terminated strings and live, distinct writable buffers.
        unsafe {
            let h = fc4_cmdline_probe_create(raw.as_ptr());
            assert!(!h.is_null());
            let mut bytes = [0u8; 128];
            let (mut len, mut scalar) = (0, 0);
            assert_eq!(
                fc4_cmdline_probe_query(h, 6, key.as_ptr(), 1, bytes.as_mut_ptr(), 128, &mut len, &mut scalar),
                0
            );
            assert_eq!((scalar, &bytes[..len as usize]), (1, b"off".as_slice()));
            assert_eq!(
                fc4_cmdline_probe_query(h, 2, key.as_ptr(), 0, bytes.as_mut_ptr(), 128, &mut len, &mut scalar),
                0
            );
            assert_eq!(&bytes[..len as usize], raw.as_bytes());
            // Capacity refusal leaves every result field untouched.
            let before = (bytes, len, scalar);
            assert_eq!(
                fc4_cmdline_probe_query(h, 2, key.as_ptr(), 0, bytes.as_mut_ptr(), 1, &mut len, &mut scalar),
                -2
            );
            assert_eq!((bytes, len, scalar), before);
            fc4_cmdline_probe_destroy(h);
        }
    }

    #[test]
    fn untouched_outputs_negative_queries_and_excluded_inputs() {
        let c = CmdLine::new(b"game -w -intro:off");
        assert_eq!(query(&c, 0, b"", 0), Some((3, UNCHANGED.to_vec())));
        assert_eq!(query(&c, 1, b"", 3), Some((0, Vec::new())));
        assert_eq!(query(&c, 1, b"", -1), None);
        assert_eq!(query(&c, 7, b"w", 1), Some((1, UNCHANGED.to_vec())));
        assert_eq!(query(&c, 6, b"absent", 1), Some((0, UNCHANGED.to_vec())));
        assert_eq!(query(&c, 8, b"w", -1), Some((1, UNCHANGED.to_vec())));
        assert_eq!(query(&c, 8, b"w", 3), Some((-1, UNCHANGED.to_vec())));
        assert_eq!(query(&c, 6, b"", 1), None); // CL2 excluded.
        assert_eq!(query(&c, 10, b"w", 0), None); // No private extra slot.
    }
}
