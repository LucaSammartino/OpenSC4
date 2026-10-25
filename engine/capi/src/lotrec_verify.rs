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

/// Private static-link projection for the lot's specified live getters.
/// All five words are written only after accepting the complete record.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotrec_project_view(record: *const u8, len: u32, output: *mut u32) -> bool {
    if record.is_null() || output.is_null() || len > 64 * 1024 * 1024 {
        return false;
    }
    std::panic::catch_unwind(|| {
        let bytes = std::slice::from_raw_parts(record, len as usize);
        let Some((center, flags, altitude)) = fc4_core::lot_records::shadow::project_view(bytes) else {
            return false;
        };
        let words = [
            center[0].to_bits(),
            center[1].to_bits(),
            center[2].to_bits(),
            flags,
            altitude.to_bits(),
        ];
        std::ptr::copy_nonoverlapping(words.as_ptr(), output, words.len());
        true
    })
    .unwrap_or(false)
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

#[cfg(test)]
mod view_tests {
    use super::fc4_lotrec_project_view;
    use fc4_core::lot_records::{shadow::IdentityWriter, LotRecord};
    use fc4_core::stream::ByteWriter;

    #[test]
    fn projection_rejects_invalid_buffers_without_partial_output() {
        let mut out = [0xdeadbeef; 5];
        unsafe {
            assert!(!fc4_lotrec_project_view(std::ptr::null(), 0, out.as_mut_ptr()));
            assert!(!fc4_lotrec_project_view([8u8].as_ptr(), 1, out.as_mut_ptr()));
            assert!(!fc4_lotrec_project_view([8u8].as_ptr(), u32::MAX, out.as_mut_ptr()));
        }
        assert_eq!(out, [0xdeadbeef; 5]);
        let record = LotRecord {
            rect: [10, 20, 12, 21],
            altitude: -0.0,
            flags: 0x80,
            ..LotRecord::default()
        };
        let mut writer = IdentityWriter(ByteWriter::new());
        assert!(record.write(&mut writer));
        let bytes = writer.0.into_bytes();
        unsafe {
            assert!(!fc4_lotrec_project_view(
                bytes.as_ptr(),
                bytes.len() as u32,
                std::ptr::null_mut()
            ));
            assert!(fc4_lotrec_project_view(
                bytes.as_ptr(),
                bytes.len() as u32,
                out.as_mut_ptr()
            ));
        }
        assert_eq!(
            out,
            [
                184.0f32.to_bits(),
                (-0.0f32).to_bits(),
                336.0f32.to_bits(),
                0x80,
                (-0.0f32).to_bits()
            ]
        );
    }
}
