// SPDX-License-Identifier: GPL-3.0-or-later
// Private static-link transport; no public C ABI entry or retained handle.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotrec_verify(
    record: *const u8,
    len: u32,
    output: *mut u8,
    capacity: u32,
    trip_count: *mut u32,
) -> i32 {
    if trip_count.is_null() {
        return -1;
    }
    *trip_count = 0;
    if record.is_null() || output.is_null() || len > 64 * 1024 * 1024 {
        return -1;
    }
    std::panic::catch_unwind(|| {
        let bytes = std::slice::from_raw_parts(record, len as usize);
        let Some((encoded, trips)) = fc4_core::lot_records::shadow::reencode_with_trip_count(bytes) else {
            return -1;
        };
        if encoded.len() > capacity as usize {
            return -1;
        }
        std::ptr::copy_nonoverlapping(encoded.as_ptr(), output, encoded.len());
        *trip_count = trips;
        encoded.len() as i32
    })
    .unwrap_or(-1)
}

// LR15 private static-link transport, deliberately absent from fc4.h/ABI exports.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotowner_verify(
    class_id: u32,
    record: *const u8,
    len: u32,
    output: *mut u8,
    capacity: u32,
) -> i32 {
    if record.is_null() || output.is_null() || len > 64 * 1024 * 1024 {
        return -1;
    }
    std::panic::catch_unwind(|| {
        let bytes = std::slice::from_raw_parts(record, len as usize);
        let Some(encoded) = fc4_core::lot_records::shadow::reencode_owner(class_id, bytes) else {
            return -1;
        };
        if encoded.len() > capacity as usize {
            return -1;
        }
        std::ptr::copy_nonoverlapping(encoded.as_ptr(), output, encoded.len());
        encoded.len() as i32
    })
    .unwrap_or(-1)
}
