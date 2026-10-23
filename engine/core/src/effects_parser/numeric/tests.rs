// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
#[test]
fn nq1_through_nq8_raw_scale_store_and_original_comparison() {
    for (se, sig, expected) in [
        (
            0x3fff,
            0x8000004000000000,
            [0x3f800000, 0x3f800000, 0x3f800001, 0x3f800000],
        ),
        (0, 0, [0; 4]),
        (0x8000, 0, [0x80000000; 4]),
        (0x407f, 1 << 63, [0x7f800000, 0x7f7fffff, 0x7f800000, 0x7f7fffff]),
        (0x3f69, 1 << 63, [0, 0, 1, 0]),
        (0xbf69, 1 << 63, [0x80000000, 0x80000001, 0x80000000, 0x80000000]),
        (0x7fff, 1 << 63, [0x7f800000; 4]),
        (0x7fff, 0xc000000000000000, [0x7fc00000; 4]),
    ] {
        for precision in [Precision::P24, Precision::P53, Precision::P64] {
            for (rounding, bits) in [Rounding::Nearest, Rounding::Down, Rounding::Up, Rounding::Zero]
                .into_iter()
                .zip(expected)
            {
                let raw = Extended::new(se, sig);
                assert_eq!(Extended::input(RawFloat::Extended(raw)), Ok(raw));
                assert_eq!(raw.store(Control { precision, rounding }), bits);
                assert!(!raw.is_one());
            }
        }
    }
}
#[test]
fn nq9_through_nq17_probability_intermediates_and_store() {
    for precision in [Precision::P53, Precision::P64] {
        let policy = NumericPolicy::X87(Control {
            precision,
            rounding: Rounding::Nearest,
        });
        for (se, sig, m, a, t) in [
            (
                0x3ffe,
                1 << 63,
                Extended::binary64(32767.5f64.to_bits()),
                Extended::binary64(12615679.5f64.to_bits()),
                0x4b408000,
            ),
            (
                0xbfff,
                1 << 63,
                Extended::binary64((-65535f64).to_bits()),
                Extended::binary64(12517377f64.to_bits()),
                0x4b3f0001,
            ),
            (
                0x4000,
                1 << 63,
                Extended::binary64(131070f64.to_bits()),
                Extended::binary64(12713982f64.to_bits()),
                0x4b41fffe,
            ),
            (
                0x8000,
                0,
                Extended::new(0x8000, 0),
                Extended::binary64(12582912f64.to_bits()),
                0x4b400000,
            ),
            (
                0x7fff,
                1 << 63,
                Extended::new(0x7fff, 1 << 63),
                Extended::new(0x7fff, 1 << 63),
                0x7f800000,
            ),
            (
                0x7fff,
                0xc000000000000000,
                Extended::new(0x7fff, 0xc000000000000000),
                Extended::new(0x7fff, 0xc000000000000000),
                0x7fc00000,
            ),
            (
                0x407f,
                1 << 63,
                Extended::new(0x408e, 0xffff000000000000),
                Extended::new(0x408e, 0xffff000000000000),
                0x7f800000,
            ),
            (
                0xffff,
                1 << 63,
                Extended::new(0xffff, 1 << 63),
                Extended::new(0xffff, 1 << 63),
                0xff800000,
            ),
        ] {
            let raw = Extended::new(se, sig);
            assert_eq!(Extended::input(RawFloat::Extended(raw)), Ok(raw));
            assert_eq!(probability(RawFloat::Extended(raw), policy), Ok((m, a, t)));
        }
    }
    for precision in [Precision::P24, Precision::P53, Precision::P64] {
        for rounding in [Rounding::Nearest, Rounding::Down, Rounding::Up, Rounding::Zero] {
            let high = matches!(rounding, Rounding::Nearest | Rounding::Up);
            let (m, a, t) = probability(
                RawFloat::Extended(Extended::new(0x3ffe, 1 << 63)),
                NumericPolicy::X87(Control { precision, rounding }),
            )
            .unwrap();
            assert_eq!(m, Extended::binary64(32767.5f64.to_bits()));
            let expected: f64 = if precision == Precision::P24 {
                if high {
                    12615680.
                } else {
                    12615679.
                }
            } else {
                12615679.5
            };
            assert_eq!(a, Extended::binary64(expected.to_bits()));
            assert_eq!(t, if high { 0x4b408000 } else { 0x4b407fff });
        }
    }
}
#[test]
fn masked_signaling_nan_canonical_domain_and_extreme_exponents() {
    let c = Control {
        precision: Precision::P64,
        rounding: Rounding::Nearest,
    };
    let snan = Extended::new(0xffff, 0x8000000000000001);
    assert_eq!(snan.store(c), 0xffc00000);
    assert_eq!(snan.significand, 0x8000000000000001);
    assert_eq!(
        Extended::input(RawFloat::Extended(Extended::new(1, 1))),
        Err(Unsupported::Numeric)
    );
    let tiny = Extended::new(0, 1);
    assert_eq!(tiny.store(c), 0);
    assert_eq!(
        tiny.store(Control {
            rounding: Rounding::Up,
            ..c
        }),
        1
    );
    assert_eq!(Extended::new(0x7ffe, u64::MAX).store(c), 0x7f800000);
}

#[test]
fn arithmetic_rounding_is_distinct_from_store_and_preserves_distant_sticky_bits() {
    for precision in [Precision::P24, Precision::P53, Precision::P64] {
        for rounding in [Rounding::Nearest, Rounding::Down, Rounding::Up, Rounding::Zero] {
            let c = Control { precision, rounding };
            let x = Extended::new(0x3fff, 0x8000004000000000);
            let m = x.mul_uint(65535, c);
            let expected = if precision == Precision::P24 {
                if rounding == Rounding::Up {
                    65535.00390625f64
                } else {
                    65535.
                }
            } else {
                65535. + 65535. / 33554432.
            };
            assert_eq!(m, Extended::binary64(expected.to_bits()));
            for (sign, store) in [
                (
                    1,
                    if rounding == Rounding::Up {
                        0x4b400001
                    } else {
                        0x4b400000
                    },
                ),
                (
                    -1,
                    if matches!(rounding, Rounding::Down | Rounding::Zero) {
                        0x4b3fffff
                    } else {
                        0x4b400000
                    },
                ),
            ] {
                let (_, _, bits) = probability(RawFloat::Dyadic(sign, -1000), NumericPolicy::X87(c)).unwrap();
                assert_eq!(bits, store);
            }
            let three = Extended::input(RawFloat::Dyadic(3, 0)).unwrap();
            let significand = match precision {
                Precision::P24 => {
                    if matches!(rounding, Rounding::Nearest | Rounding::Up) {
                        0xaaaaab0000000000
                    } else {
                        0xaaaaaa0000000000
                    }
                }
                Precision::P53 => {
                    if rounding == Rounding::Up {
                        0xaaaaaaaaaaaab000
                    } else {
                        0xaaaaaaaaaaaaa800
                    }
                }
                Precision::P64 => {
                    if matches!(rounding, Rounding::Nearest | Rounding::Up) {
                        0xaaaaaaaaaaaaaaab
                    } else {
                        0xaaaaaaaaaaaaaaaa
                    }
                }
            };
            assert_eq!(three.reciprocal(c), Extended::new(0x3ffd, significand));
            assert_eq!(
                three.reciprocal(c).store(c),
                if matches!(rounding, Rounding::Nearest | Rounding::Up) {
                    0x3eaaaaab
                } else {
                    0x3eaaaaaa
                }
            );
        }
    }
}
