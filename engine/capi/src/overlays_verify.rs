// SPDX-License-Identifier: GPL-3.0-or-later
//! Private, read-only overlay getter-contract transport. Not a public DLL export.
//! OV12 blocks native arena enumeration; these probes do not verify live records.
use fc4_core::terrain_overlays::{Handle, OverlayManager, Placement};

fn contract_words() -> Option<[u32; 25]> {
    let core = OverlayManager::new();
    let mut words = [0; 25];
    words[0] = u32::from(core.shown(Handle::NONE).ok()?.unwrap_or(false));
    words[1] = core.alpha(Handle::NONE).ok()?.to_bits();
    let mut color = [0.75; 3];
    core.color_into(Handle::NONE, &mut color).ok()?;
    for (out, value) in words[2..5].iter_mut().zip(color) {
        *out = value.to_bits();
    }
    words[5] = u32::from(core.draw_state(Handle::NONE).ok()?.unwrap_or(0));
    words[6] = u32::from(core.self_lit(Handle::NONE).ok()?);
    for (probe, handle) in [Handle::NONE, Handle(0)].into_iter().enumerate() {
        let mut placement = Placement(std::array::from_fn(|i| {
            f32::from_bits(0x3e80_0000 + (probe * 16 + i) as u32)
        }));
        core.get_placement(handle, &mut placement).ok()?;
        for (out, value) in words[7 + probe * 9..16 + probe * 9].iter_mut().zip(placement.0) {
            *out = value.to_bits();
        }
    }
    Some(words)
}

// Safety: private caller supplies exactly 25 writable u32s. No game-owned data
// or computed native answers enter the production core.
#[no_mangle]
unsafe extern "C" fn fc4_overlays_verify_contract(out: *mut u32, count: u32) -> u32 {
    if out.is_null() || count != 25 {
        return 0;
    }
    let Some(words) = contract_words() else { return 0 };
    unsafe { std::ptr::copy_nonoverlapping(words.as_ptr(), out, words.len()) };
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn getter_transport_preserves_every_placement_word_and_positive_zero() {
        let mut actual = [u32::MAX; 25];
        assert_eq!(unsafe { fc4_overlays_verify_contract(actual.as_mut_ptr(), 25) }, 1);
        assert_eq!(actual[..7], [0; 7]);
        for probe in 0..2 {
            for i in 0..9 {
                assert_eq!(actual[7 + probe * 9 + i], 0x3e80_0000 + (probe * 16 + i) as u32);
            }
        }
        assert_eq!(unsafe { fc4_overlays_verify_contract(actual.as_mut_ptr(), 24) }, 0);
        assert_eq!(unsafe { fc4_overlays_verify_contract(std::ptr::null_mut(), 25) }, 0);
    }
}
