// SPDX-License-Identifier: GPL-3.0-or-later
//! Private static-link comparison of lighting.md §§1.4/5.6 shipped parameters.
//! Native values enter only the comparator, never the expected-value model.
use fc4_core::lighting::{LightingManager, LightingStatics, Parameters, PropertyHolder, RegisterPrecision};

const FIELDS: usize = 14;
struct NoOverrides;
impl PropertyHolder for NoOverrides {}

fn expected() -> [u32; FIELDS] {
    // LT14: bounded shipped-exemplar scenario, not a replay of dynamic owner state.
    // These getters perform no arithmetic affected by the inherited x87 mode.
    let manager = LightingManager::new(LightingStatics::new(Parameters::shipped()), RegisterPrecision::Bits24);
    let (a, b, color) = manager.get_shadow_params();
    let decal = manager.get_decal_shadow_params(&NoOverrides);
    [
        a.to_bits(),
        b.to_bits(),
        color[0].to_bits(),
        color[1].to_bits(),
        color[2].to_bits(),
        decal.texture,
        decal.a.to_bits(),
        decal.b.to_bits(),
        decal.c.to_bits(),
        decal.pair[0].to_bits(),
        decal.pair[1].to_bits(),
        decal.color[0].to_bits(),
        decal.color[1].to_bits(),
        decal.color[2].to_bits(),
    ]
}

fn differences(native: &[u32; FIELDS], expected: &[u32; FIELDS]) -> u32 {
    native
        .iter()
        .zip(expected)
        .enumerate()
        .fold(0, |mask, (i, (a, b))| mask | (u32::from(a != b) << i))
}

// Private transport, deliberately absent from fc4.h and the DLL export list.
#[no_mangle]
unsafe extern "C" fn fc4_lighting_shadow_compare(native: *const u32, out: *mut u32) -> u32 {
    if native.is_null() || out.is_null() {
        return (1 << FIELDS) - 1;
    }
    let expected = expected();
    // SAFETY: the plugin supplies distinct, readable/writable 14-word arrays.
    let native = unsafe { &*native.cast::<[u32; FIELDS]>() };
    unsafe { std::ptr::copy_nonoverlapping(expected.as_ptr(), out, FIELDS) };
    differences(native, &expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_contract_and_each_field_difference() {
        let expected = expected();
        assert_eq!(expected[0], 0.4_f32.to_bits());
        assert_eq!(expected[1], 0.55_f32.to_bits());
        assert_eq!(expected[5], 0x69d3_bc3b);
        assert_eq!(expected[8], 12_f32.to_bits());
        assert_eq!(&expected[9..11], &[0, 0]);
        assert_eq!(differences(&expected, &expected), 0);
        for i in 0..FIELDS {
            let mut native = expected;
            native[i] ^= 1;
            assert_eq!(differences(&native, &expected), 1 << i);
        }
        let changed = expected.map(|word| word ^ 1);
        assert_eq!(differences(&changed, &expected), (1 << FIELDS) - 1);
    }

    #[test]
    fn transport_returns_independent_expected_values_and_refuses_nulls() {
        let native = [0; FIELDS];
        let mut out = [0; FIELDS];
        let mask = unsafe { fc4_lighting_shadow_compare(native.as_ptr(), out.as_mut_ptr()) };
        assert_eq!(out, expected());
        assert_eq!(mask.count_ones(), 12); // Two specified zero offset components.
        assert_eq!(
            unsafe { fc4_lighting_shadow_compare(std::ptr::null(), out.as_mut_ptr()) },
            (1 << FIELDS) - 1
        );
    }
}
