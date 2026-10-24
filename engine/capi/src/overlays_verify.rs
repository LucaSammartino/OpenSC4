// SPDX-License-Identifier: GPL-3.0-or-later
//! Private, read-only overlay getter-contract transport. Not a public DLL export.
//! OV12 observation diagnostics are not independent creation-history replay.
use fc4_core::terrain_overlays::{BindingLookup, TerrainProvider, TextureBinding, TextureFactory, ViewProvider};
use fc4_core::terrain_overlays::{Handle, OverlayManager, Placement};
use std::sync::Arc;

// Controlled suffix replay only. Init's cached geometry is an external input
// boundary, not independent verification of native Init, resources or pixels.
struct FixtureInputs([u32; 7]);
impl TerrainProvider for FixtureInputs {
    fn cells(&self) -> [u32; 2] {
        [self.0[0], self.0[1]]
    }
    fn cell_width(&self) -> f32 {
        f32::from_bits(self.0[2])
    }
}
impl TextureBinding for FixtureInputs {
    fn size(&self) -> [u32; 2] {
        [self.0[3], self.0[4]]
    }
}
impl TextureFactory for FixtureInputs {
    fn binding(&self, _: u32, _: BindingLookup) -> Option<Arc<dyn TextureBinding>> {
        Some(Arc::new(Self(self.0)))
    }
    fn shadow_binding_slot6(&self, _: [u32; 3]) -> Option<Arc<dyn TextureBinding>> {
        Some(Arc::new(Self(self.0)))
    }
    fn key_word_slot11(&self) -> u32 {
        0
    }
}
impl ViewProvider for FixtureInputs {
    fn terrain(&self) -> Option<Arc<dyn TerrainProvider>> {
        Some(Arc::new(Self(self.0)))
    }
    fn texture_factory(&self) -> Option<Arc<dyn TextureFactory>> {
        Some(Arc::new(Self(self.0)))
    }
    fn shadow_quality(&self) -> i32 {
        0
    }
    fn shadow_debug(&self) -> i32 {
        0
    }
    fn subscribe(&self, _: u32) {}
    fn unsubscribe(&self, _: u32) {}
}

fn replay_decal(args: [u32; 7]) -> Option<[u32; 35]> {
    if args[3] == 0 || args[4] == 0 {
        return None;
    }
    let mut core = OverlayManager::new();
    core.init(Arc::new(FixtureInputs(args)), args[5] != 0, args[6] != 0)
        .ok()?;
    let id = core
        .add_decal_binding(Some(Arc::new(FixtureInputs(args))), [128.0, 128.0], 32.0, 0.0)
        .ok()?;
    core.set_position(id, [144.0, 160.0]).ok()?;
    core.set_alpha(id, 0.5).ok()?;
    core.set_color(id, [0.25, 0.5, 0.75]).ok()?;
    core.set_draw_state(id, 3).ok()?;
    core.enable_self_lighting(id, 1).ok()?;
    let s = core.snapshot(id).ok()??;
    let mut placement = Placement([0.0; 9]);
    core.get_placement(id, &mut placement).ok()?;
    let mut words = [0; 35];
    words[0] = s.flags;
    words[1] = u32::from(s.draw_state);
    words[2..6].copy_from_slice(&[
        s.cells.min[0] as u32,
        s.cells.min[1] as u32,
        s.cells.max[0] as u32,
        s.cells.max[1] as u32,
    ]);
    for (out, value) in words[6..22].iter_mut().zip(s.primary) {
        *out = value.to_bits();
    }
    words[22] = s.alpha.to_bits();
    for (out, value) in words[23..26].iter_mut().zip(s.color) {
        *out = value.to_bits();
    }
    for (out, value) in words[26..35].iter_mut().zip(placement.0) {
        *out = value.to_bits();
    }
    Some(words)
}

// Safety: private adapter passes seven readable input and 35 writable output
// words. Native IDs and computed native geometry are never replay inputs.
#[no_mangle]
unsafe extern "C" fn fc4_overlays_replay_decal(input: *const u32, out: *mut u32) -> u32 {
    if input.is_null() || out.is_null() {
        return 0;
    }
    let mut args = [0; 7];
    unsafe { std::ptr::copy_nonoverlapping(input, args.as_mut_ptr(), 7) };
    let Some(words) = replay_decal(args) else { return 0 };
    unsafe { std::ptr::copy_nonoverlapping(words.as_ptr(), out, words.len()) };
    1
}

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

// Safety: private adapter passes count readable words and 14 writable report
// words. No native address or result is inserted into the production owner.
#[no_mangle]
unsafe extern "C" fn fc4_overlays_observe(input: *const u32, count: u32, out: *mut u32) -> u32 {
    if input.is_null() || out.is_null() || count > 16_777_216 {
        return 0;
    }
    let words = unsafe { std::slice::from_raw_parts(input, count as usize) };
    let Some(r) = fc4_core::terrain_overlays::observation::inspect(words) else {
        return 0;
    };
    let result = [
        r.checks,
        r.differences,
        r.overlays,
        r.handles,
        r.groups,
        r.cells,
        r.entries,
        r.bindings,
        r.sources,
        r.unknown_bindings,
        r.observed_words,
        r.digest as u32,
        (r.digest >> 32) as u32,
        0,
    ];
    unsafe { std::ptr::copy_nonoverlapping(result.as_ptr(), out, result.len()) };
    1
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
    fn controlled_suffix_replays_placement_and_mutations() {
        for dynamic in [0, 1] {
            let r = replay_decal([64, 64, 16.0_f32.to_bits(), 64, 64, dynamic, 0]).unwrap();
            assert_eq!(r[0], 81); // shown | ordinary PART | self-lit
            assert_eq!(r[1], 3);
            assert_eq!(r[2..6], [8, 9, 10, 11]);
            assert_eq!(r[22], 0.5_f32.to_bits());
            assert_eq!(
                r[26..30],
                [144.0_f32.to_bits(), 160.0_f32.to_bits(), 32.0_f32.to_bits(), 0]
            );
        }
        let mut out = [0xdead_beef; 35];
        let invalid = [0; 7];
        assert_eq!(
            unsafe { fc4_overlays_replay_decal(invalid.as_ptr(), out.as_mut_ptr()) },
            0
        );
        assert_eq!(out, [0xdead_beef; 35]);
    }

    #[test]
    fn observation_transport_rejects_truncation_without_touching_report() {
        let mut out = [0xdead_beef; 14];
        let words = [
            1,
            1,
            0,
            2,
            2,
            0,
            0,
            u32::MAX,
            u32::MAX,
            0,
            0,
            u32::MAX,
            u32::MAX,
            0,
            0,
            u32::MAX,
            u32::MAX,
            0,
            0,
            0,
            0,
        ];
        assert_eq!(unsafe { fc4_overlays_observe(words.as_ptr(), 20, out.as_mut_ptr()) }, 0);
        assert_eq!(out, [0xdead_beef; 14]);
        assert_eq!(unsafe { fc4_overlays_observe(words.as_ptr(), 21, out.as_mut_ptr()) }, 1);
        assert_eq!(out[..10], [0; 10]);
        assert_eq!(out[10], 21);
        assert_eq!(
            unsafe { fc4_overlays_observe(std::ptr::null(), 0, out.as_mut_ptr()) },
            0
        );
        assert_eq!(
            unsafe { fc4_overlays_observe(words.as_ptr(), 21, std::ptr::null_mut()) },
            0
        );
    }

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
