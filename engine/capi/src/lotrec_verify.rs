// SPDX-License-Identifier: GPL-3.0-or-later
// Private static-link transport; no public C ABI entry or retained handle.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotrec_verify(record: *const u8, len: u32, output: *mut u8, capacity: u32) -> i32 {
    if record.is_null() || output.is_null() || len > 64 * 1024 * 1024 {
        return -1;
    }
    std::panic::catch_unwind(|| {
        let bytes = std::slice::from_raw_parts(record, len as usize);
        let Some(encoded) = fc4_core::lot_records::shadow::reencode(bytes) else {
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
