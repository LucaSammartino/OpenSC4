// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent public lighting.md §11 vectors; expected rows are literal
//! spec results, never computed by the projection implementation under test.
use super::tests::{city, manager};
use super::*;

const R: [[f32; 3]; 3] = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]];
const I: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
const POINTS: [Vector; 3] = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
const UVS: [[f32; 2]; 3] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
fn transform(flags: u8) -> ProjectionTransform {
    ProjectionTransform {
        linear: R,
        translation: [8.0, 16.0, 32.0],
        scale: 2.0,
        flags,
    }
}
fn rows(flat: [f32; 16]) -> [[f32; 4]; 4] {
    std::array::from_fn(|i| std::array::from_fn(|j| flat[4 * j + i]))
}

#[test]
fn vector19_direction_transpose_negation_no_normalization_or_scale() {
    for precision in [RegisterPrecision::Bits53, RegisterPrecision::Bits64] {
        for flag in [0, 2, 4, 6] {
            let expected = if flag & 4 == 0 {
                [[1.0, 0.0, 0.0], [0.0, 1.0, -2.0], [0.0, 2.0, 1.0]]
            } else {
                [[-1.0, 0.0, 0.0], [0.0, 1.0, 2.0], [0.0, 2.0, -1.0]]
            };
            assert_eq!(
                projection::direction_basis([1.0, -2.0, 0.0], transform(flag), precision),
                expected
            );
        }
    }
}
#[test]
fn vector20_whole_byte_flags_inverse_translation_and_scale() {
    let cases = [
        (0, [[1.0, 0.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 1.0, 0.0]]),
        (
            1,
            [[0.0, 0.0, 1.0, -8.0], [-1.0, 0.0, 0.0, -32.0], [-1.0, 0.0, 0.0, -32.0]],
        ),
        (
            128,
            [[0.0, 0.0, 1.0, -8.0], [-1.0, 0.0, 0.0, -32.0], [-1.0, 0.0, 0.0, -32.0]],
        ),
        (
            2,
            [[0.0, 0.0, 0.5, -4.0], [-0.5, 0.0, 0.0, -16.0], [-0.5, 0.0, 0.0, -16.0]],
        ),
        (
            4,
            [[0.0, 0.0, -1.0, 32.0], [1.0, 0.0, 0.0, -8.0], [1.0, 0.0, 0.0, -8.0]],
        ),
        (
            6,
            [[0.0, 0.0, -0.5, 16.0], [0.5, 0.0, 0.0, -4.0], [0.5, 0.0, 0.0, -4.0]],
        ),
    ];
    let mut m = manager();
    m.shadow_direction = [0.0, -1.0, 0.0];
    for precision in [RegisterPrecision::Bits53, RegisterPrecision::Bits64] {
        m.register_precision = precision;
        for (flag, expected) in cases {
            let mut output = [999.0; 16];
            assert_eq!(
                m.calc_shadow_projection(&POINTS, &UVS, transform(flag), &mut output),
                Ok(true)
            );
            let actual = rows(output);
            assert_eq!(&actual[..3], &expected, "flag {flag}");
            assert_eq!(actual[3], [0.0, 0.0, 0.0, 1.0]);
        }
    }
}
#[test]
fn vector21_native_product_spill_rejects_noncollinear_triangle() {
    let mut m = manager();
    m.shadow_direction = [0.0, 0.0, -1.0];
    for precision in [RegisterPrecision::Bits53, RegisterPrecision::Bits64] {
        m.register_precision = precision;
        let mut output = [123.0; 16];
        assert_eq!(
            m.calc_shadow_projection(
                &[[4096.0, 0.0, 0.0], [4097.0, 0.0, 0.0], [4096.0, 1.0, 0.0]],
                &UVS,
                ProjectionTransform {
                    linear: I,
                    ..transform(0)
                },
                &mut output
            ),
            Ok(false)
        );
        assert_eq!(output, [123.0; 16]);
    }
}
#[test]
fn vector22_u_v_translation_spills_differ_with_identical_uv_columns() {
    let mut m = manager();
    m.shadow_direction = [0.0, -1.0, 0.0];
    for precision in [RegisterPrecision::Bits53, RegisterPrecision::Bits64] {
        m.register_precision = precision;
        let mut output = [0.0; 16];
        assert_eq!(
            m.calc_shadow_projection(
                &POINTS,
                &[[1.0, 1.0], [2.0, 2.0], [2.0, 2.0]],
                ProjectionTransform {
                    linear: I,
                    translation: [16777216.0, 0.0, 1.0],
                    ..transform(1)
                },
                &mut output
            ),
            Ok(true)
        );
        assert_eq!(
            rows(output),
            [
                [1.0, 0.0, 1.0, -16777215.0],
                [1.0, 0.0, 1.0, -16777216.0],
                [0.0, 0.0, 1.0, -1.0],
                [0.0, 0.0, 0.0, 1.0]
            ]
        );
    }
}
#[test]
fn vector17_terrain_probes_keep_stopped_vertex() {
    let mut m = manager();
    city(&mut m, 8);
    m.shadow_direction = [1.0, -1.0, -1.0];
    let mut out = Rect::EMPTY;
    assert_eq!(
        m.calc_shadow_terrain_rect(
            Bounds {
                min: [16.0, 0.0, 16.0],
                max: [32.0, 32.0, 32.0]
            },
            &mut out
        ),
        Ok(true)
    );
    assert_eq!(
        out,
        Rect {
            min_x: 1,
            min_z: 0,
            max_x: 4,
            max_z: 2
        }
    );
}
