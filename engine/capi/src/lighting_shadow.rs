// SPDX-License-Identifier: GPL-3.0-or-later
//! Private static-link comparison of lighting.md §§1.4/5.6 exemplar parameters.
//! Native values enter only the comparator, never the expected-value model.
use fc4_core::lighting::{
    LightingManager, LightingStatics, Parameters, ProjectionTransform, PropertyHolder, RegisterPrecision,
};

const FIELDS: usize = 14;
struct NoOverrides;
impl PropertyHolder for NoOverrides {}

// Six property-presence bits followed by texture, a/b/c, pair and RGB words.
// Captured from the resource manager's exemplar, never the lighting owner.
struct Inputs([u32; 10]);
impl PropertyHolder for Inputs {
    fn unsigned(&self, id: u32) -> Option<u32> {
        (id == 0x0854_89c8 && self.0[0] & 1 != 0).then_some(self.0[1])
    }
    fn float(&self, id: u32) -> Option<f32> {
        let index = match id {
            0x0854_89cc => 2,
            0x0854_89cd => 3,
            0x0854_89ca => 4,
            _ => return None,
        };
        (self.0[0] & (1 << (index - 1)) != 0).then_some(f32::from_bits(self.0[index]))
    }
    fn pair(&self, id: u32) -> Option<[f32; 2]> {
        (id == 0x0854_89c9 && self.0[0] & 16 != 0).then_some([f32::from_bits(self.0[5]), f32::from_bits(self.0[6])])
    }
    fn color(&self, id: u32) -> Option<[f32; 3]> {
        (id == 0x0854_89cb && self.0[0] & 32 != 0).then_some([
            f32::from_bits(self.0[7]),
            f32::from_bits(self.0[8]),
            f32::from_bits(self.0[9]),
        ])
    }
}

fn expected(inputs: &Inputs) -> [u32; FIELDS] {
    // LT14: bounded exemplar scenario, not a replay of dynamic owner state.
    // These getters perform no arithmetic affected by the inherited x87 mode.
    let mut parameters = Parameters::initial();
    parameters.read(inputs);
    let manager = LightingManager::new(LightingStatics::new(parameters), RegisterPrecision::Bits24);
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

fn differences<const N: usize>(native: &[u32; N], expected: &[u32; N]) -> u32 {
    native
        .iter()
        .zip(expected)
        .enumerate()
        .fold(0, |mask, (i, (a, b))| mask | (u32::from(a != b) << i))
}

fn expected_decal(inputs: &Inputs, overrides: &Inputs) -> [u32; 9] {
    let mut parameters = Parameters::initial();
    parameters.read(inputs);
    let manager = LightingManager::new(LightingStatics::new(parameters), RegisterPrecision::Bits24);
    let decal = manager.get_decal_shadow_params(overrides);
    [
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

// Private static-link transport; overrides are caller-owned query inputs.
#[no_mangle]
unsafe extern "C" fn fc4_lighting_shadow_decal_compare(
    inputs: *const u32,
    overrides: *const u32,
    native: *const u32,
    out: *mut u32,
) -> u32 {
    if inputs.is_null() || overrides.is_null() || native.is_null() || out.is_null() {
        return (1 << 9) - 1;
    }
    // SAFETY: plugin supplies two readable ten-word input records and distinct
    // readable/writable nine-word result arrays. Outputs never seed the model.
    let expected = expected_decal(
        &Inputs(unsafe { *inputs.cast() }),
        &Inputs(unsafe { *overrides.cast() }),
    );
    let native = unsafe { &*native.cast::<[u32; 9]>() };
    unsafe { std::ptr::copy_nonoverlapping(expected.as_ptr(), out, 9) };
    differences(native, &expected)
}

fn expected_empty_projection(flags: u8) -> [u32; 17] {
    let manager = LightingManager::new(LightingStatics::new(Parameters::initial()), RegisterPrecision::Bits24);
    let mut matrix = std::array::from_fn(|i| f32::from_bits(0x3e80_0001 + i as u32));
    let transform = ProjectionTransform {
        linear: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        translation: [2.0, 3.0, 4.0],
        scale: 2.0,
        flags,
    };
    let result = manager
        .calc_shadow_projection(&[], &[], transform, &mut matrix)
        .expect("valid empty query");
    std::array::from_fn(|i| {
        if i == 0 {
            u32::from(result)
        } else {
            matrix[i - 1].to_bits()
        }
    })
}

// §5.8: count zero returns false without writing any of the 64 output bytes.
// The conclusion is direction-independent; no native direction seeds this test.
#[no_mangle]
unsafe extern "C" fn fc4_lighting_shadow_empty_compare(flags: u32, native: *const u32, out: *mut u32) -> u32 {
    if native.is_null() || out.is_null() {
        return (1 << 17) - 1;
    }
    let expected = expected_empty_projection(flags as u8);
    // SAFETY: plugin supplies distinct readable/writable seventeen-word arrays.
    let native = unsafe { &*native.cast::<[u32; 17]>() };
    unsafe { std::ptr::copy_nonoverlapping(expected.as_ptr(), out, 17) };
    differences(native, &expected)
}

// Private transport, deliberately absent from fc4.h and the DLL export list.
#[no_mangle]
unsafe extern "C" fn fc4_lighting_shadow_compare(inputs: *const u32, native: *const u32, out: *mut u32) -> u32 {
    if inputs.is_null() || native.is_null() || out.is_null() {
        return (1 << FIELDS) - 1;
    }
    // SAFETY: the plugin supplies a readable, immutable ten-word input capture.
    let inputs = Inputs(unsafe { *inputs.cast::<[u32; 10]>() });
    let expected = expected(&inputs);
    // SAFETY: the plugin supplies distinct, readable/writable 14-word arrays.
    let native = unsafe { &*native.cast::<[u32; FIELDS]>() };
    unsafe { std::ptr::copy_nonoverlapping(expected.as_ptr(), out, FIELDS) };
    differences(native, &expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shipped_inputs() -> Inputs {
        let p = Parameters::shipped().decal;
        Inputs([
            63,
            p.texture,
            p.a.to_bits(),
            p.b.to_bits(),
            p.c.to_bits(),
            p.pair[0].to_bits(),
            p.pair[1].to_bits(),
            p.color[0].to_bits(),
            p.color[1].to_bits(),
            p.color[2].to_bits(),
        ])
    }

    #[test]
    fn shipped_contract_and_each_field_difference() {
        let expected = expected(&shipped_inputs());
        assert_eq!(expected[0], 0.4_f32.to_bits());
        assert_eq!(expected[1], 0.55_f32.to_bits());
        assert_eq!(expected[5], 0x69d3_a23b);
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
        let inputs = shipped_inputs();
        let native = [0; FIELDS];
        let mut out = [0; FIELDS];
        let mask = unsafe { fc4_lighting_shadow_compare(inputs.0.as_ptr(), native.as_ptr(), out.as_mut_ptr()) };
        assert_eq!(out, expected(&inputs));
        assert_eq!(mask.count_ones(), 12); // Two specified zero offset components.
        assert_eq!(
            unsafe { fc4_lighting_shadow_compare(inputs.0.as_ptr(), std::ptr::null(), out.as_mut_ptr()) },
            (1 << FIELDS) - 1
        );
    }

    #[test]
    fn external_exemplar_changes_expectation_but_native_outputs_do_not() {
        let mut inputs = shipped_inputs();
        inputs.0[1] = 0x1234_abcd;
        assert_eq!(expected(&inputs)[5], 0x1234_abcd);
        let mut out = [0; FIELDS];
        let native = [u32::MAX; FIELDS];
        unsafe { fc4_lighting_shadow_compare(inputs.0.as_ptr(), native.as_ptr(), out.as_mut_ptr()) };
        assert_eq!(out, expected(&inputs));
        let missing = expected(&Inputs([0; 10]));
        assert_eq!(missing[5], 0x0917_660e);
        assert_eq!(missing[0], 0.5_f32.to_bits());
        assert_eq!(missing[1], 0_f32.to_bits());
    }

    #[test]
    fn nonnull_holder_all_presence_combinations_and_no_shared_mutation() {
        let inputs = shipped_inputs();
        let base = expected_decal(&inputs, &Inputs([0; 10]));
        let mut overrides = Inputs([
            63,
            0x1234_5678,
            0.25_f32.to_bits(),
            0.75_f32.to_bits(),
            20_f32.to_bits(),
            (-2_f32).to_bits(),
            3_f32.to_bits(),
            0.5_f32.to_bits(),
            0.25_f32.to_bits(),
            0.75_f32.to_bits(),
        ]);
        let full = expected_decal(&inputs, &overrides);
        let owners = [0, 1, 2, 3, 4, 4, 5, 5, 5];
        for mask in 0..64 {
            overrides.0[0] = mask;
            let wanted = std::array::from_fn(|i| if mask & (1 << owners[i]) != 0 { full[i] } else { base[i] });
            let mut out = [0; 9];
            let native = wanted.map(|word| word ^ 1);
            assert_eq!(
                unsafe {
                    fc4_lighting_shadow_decal_compare(
                        inputs.0.as_ptr(),
                        overrides.0.as_ptr(),
                        native.as_ptr(),
                        out.as_mut_ptr(),
                    )
                },
                511
            );
            assert_eq!(out, wanted);
            assert_eq!(expected_decal(&inputs, &Inputs([0; 10])), base);
        }
    }

    #[test]
    fn empty_projection_false_and_all_output_canaries_untouched() {
        for flags in [0, 7] {
            let expected = expected_empty_projection(flags);
            assert_eq!(expected[0], 0);
            for i in 0..16 {
                assert_eq!(expected[i + 1], 0x3e80_0001 + i as u32);
            }
            let native = expected.map(|word| word ^ 1);
            let mut out = [0; 17];
            assert_eq!(
                unsafe { fc4_lighting_shadow_empty_compare(flags.into(), native.as_ptr(), out.as_mut_ptr()) },
                (1 << 17) - 1
            );
            assert_eq!(out, expected);
        }
    }
}
