// SPDX-License-Identifier: GPL-3.0-or-later
//! Private static-link native call transcript; no public DLL exports.
use fc4_core::budget_window::dialog::restore_group_start_settings;

/// Inputs are copied {fraction bits, opaque department} pairs, never pointers
/// into the game. Output records are {slot, department, fraction bits, line}.
/// # Safety
/// The caller supplies disjoint arrays of 2*count and 4*count live u32 words.
#[no_mangle]
unsafe extern "C" fn fc4_budgetwin_restore_replay(input: *const u32, count: u32, output: *mut u32) -> bool {
    if count == 0 {
        return true;
    }
    if input.is_null() || output.is_null() || count > (isize::MAX as usize / 16) as u32 {
        return false;
    }
    let input = std::slice::from_raw_parts(input, count as usize * 2);
    let output = std::slice::from_raw_parts_mut(output, count as usize * 4);
    let saved: Vec<_> = input
        .chunks_exact(2)
        .map(|r| (f32::from_bits(r[0]), u64::from(r[1])))
        .collect();
    let mut at = 0;
    restore_group_start_settings(&saved, |dept, fraction, line| {
        output[at..at + 4].copy_from_slice(&[11, dept as u32, fraction.to_bits(), line]);
        at += 4;
    });
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_restore_transcript_preserves_order_duplicates_and_float_payloads() {
        let input = [0x80000000, 7, 0x7fc12345, 3, 0x3f4ccccd, 7];
        let mut output = [0xdeadbeef; 14];
        assert!(unsafe { fc4_budgetwin_restore_replay(input.as_ptr(), 3, output[1..].as_mut_ptr()) });
        assert_eq!(
            output,
            [0xdeadbeef, 11, 7, 0x80000000, 0, 11, 3, 0x7fc12345, 0, 11, 7, 0x3f4ccccd, 0, 0xdeadbeef]
        );
        assert_eq!(input, [0x80000000, 7, 0x7fc12345, 3, 0x3f4ccccd, 7]);
    }

    #[test]
    fn empty_restore_has_no_calls_and_invalid_transport_is_refused() {
        assert!(unsafe { fc4_budgetwin_restore_replay(std::ptr::null(), 0, std::ptr::null_mut()) });
        let mut output = [99; 4];
        assert!(!unsafe { fc4_budgetwin_restore_replay(std::ptr::null(), 1, output.as_mut_ptr()) });
        assert_eq!(output, [99; 4]);
    }
}
