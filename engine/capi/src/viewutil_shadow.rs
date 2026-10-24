// SPDX-License-Identifier: GPL-3.0-or-later
//! Private static-link word transport to the production view utility.
//! VU16 blocks other native entries; no public C ABI export is added.
use fc4_core::view_utilities::{rotate_vector_by_view_rotation, Vec3};

/// # Safety
/// `words` must point to three readable and writable u32 words.
#[no_mangle]
unsafe extern "C" fn fc4_viewutil_shadow_rotate(words: *mut u32, rotation: i32) {
    if words.is_null() {
        return;
    }
    // SAFETY: the caller supplies exactly three initialized words.
    let words = unsafe { std::slice::from_raw_parts_mut(words, 3) };
    let mut v = Vec3::new(
        f32::from_bits(words[0]),
        f32::from_bits(words[1]),
        f32::from_bits(words[2]),
    );
    rotate_vector_by_view_rotation(&mut v, rotation);
    words.copy_from_slice(&[v.x.to_bits(), v.y.to_bits(), v.z.to_bits()]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_transport_preserves_y_payload_and_signed_zero() {
        let mut words = [0, 0x7fc1_2345, 0x8000_0000];
        // SAFETY: local three-word array.
        unsafe { fc4_viewutil_shadow_rotate(words.as_mut_ptr(), 1) };
        assert_eq!(words, [0, 0x7fc1_2345, 0]);
        words = [0x7fc2_3456, 0x8000_0000, 0x3f80_0000];
        // SAFETY: local three-word array.
        unsafe { fc4_viewutil_shadow_rotate(words.as_mut_ptr(), -1) };
        assert_eq!(words, [0x7fc2_3456, 0x8000_0000, 0x3f80_0000]);
    }
}
