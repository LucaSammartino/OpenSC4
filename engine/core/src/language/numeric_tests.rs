//! RZ21–RZ50 exercise the shipped legacy decimal primitive and core assembly.
use super::*;

fn check(format: &[u8], argument: FormatArg<'_>, expected: &[u8]) {
    let mut string = RzString::from_bytes(b"previous");
    let address = &string as *const RzString;
    let returned = string
        .sprintf(format, &[argument], &NumericPrimitives::<()>::default())
        .unwrap();
    assert_eq!(returned as *const RzString, address);
    assert_eq!(string.bytes(), expected, "format {}", String::from_utf8_lossy(format));
}

fn float(format: &[u8], value: f64, expected: &[u8]) {
    check(format, FormatArg::Float(value), expected);
}

#[test]
fn rz21_integer_zero_precision() {
    check(b"%.0d", FormatArg::Signed(0), b"0");
    for c in *b"uoxX" {
        check(&[b'%', b'.', b'0', c], FormatArg::Unsigned(0), b"0");
    }
}

#[test]
fn rz22_alternate_zero() {
    for (format, expected) in [(&b"%#o"[..], &b"00"[..]), (b"%#x", b"0x0"), (b"%#X", b"0X0")] {
        check(format, FormatArg::Unsigned(0), expected);
    }
}

#[test]
fn rz23_prefix_before_precision() {
    for (format, expected) in [
        (&b"%#.3o"[..], &b"0001"[..]),
        (b"%#.3x", b"0x001"),
        (b"%#.3X", b"0X001"),
    ] {
        check(format, FormatArg::Unsigned(1), expected);
    }
}

#[test]
fn rz24_field_zeros_before_prefix() {
    check(b"%08.3u", FormatArg::Unsigned(7), b"00000007");
    check(b"%#08.3x", FormatArg::Unsigned(1), b"0000x001");
    check(b"%#8.3x", FormatArg::Unsigned(1), b"   0x001");
}

#[test]
fn rz25_sign_and_left_alignment() {
    check(b"%+08.3d", FormatArg::Signed(7), b"+0000007");
    check(b"%-08.3u", FormatArg::Unsigned(7), b"007     ");
}

#[test]
fn rz26_pointer_digits() {
    for (pointer, expected) in [(0, b"00000000"), (0x12ab, b"000012AB"), (u32::MAX, b"FFFFFFFF")] {
        check(b"%p", FormatArg::Pointer(pointer), expected);
    }
}

#[test]
fn rz27_pointer_ignores_precision_and_sign() {
    for (format, expected) in [
        (&b"%#+.2p"[..], &b"000012AB"[..]),
        (b"%010p", b"00000012AB"),
        (b"%-10p", b"000012AB  "),
    ] {
        check(format, FormatArg::Pointer(0x12ab), expected);
    }
}

#[test]
fn rz28_float_defaults() {
    for (format, expected) in [
        (&b"%f"[..], &b"1.250000"[..]),
        (b"%e", b"1.250000e+000"),
        (b"%E", b"1.250000E+000"),
        (b"%g", b"1.25"),
    ] {
        float(format, 1.25, expected);
    }
}

#[test]
fn rz29_halfway_away_from_zero() {
    float(b"%.0f", 0.5, b"1");
    float(b"%.0f", 2.5, b"3");
    float(b"%.2f", 0.125, b"0.13");
    float(b"%.2f", -0.125, b"-0.13");
}

#[test]
fn rz30_seventeen_digit_intermediate() {
    float(b"%.2f", f64::from_bits(0x4005_6666_6666_6666), b"2.67");
    float(
        b"%.20f",
        f64::from_bits(0x3fb9_9999_9999_999a),
        b"0.10000000000000001000",
    );
}

#[test]
fn rz31_three_digit_exponents() {
    float(b"%.2e", 12.5, b"1.25e+001");
    float(b"%.2E", 0.125, b"1.25E-001");
    float(b"%.0e", 0.0, b"0e+000");
}

#[test]
fn rz32_alternate_point() {
    float(b"%.0f", 1.0, b"1");
    float(b"%#.0f", 1.0, b"1.");
    float(b"%#.0e", 1.0, b"1.e+000");
}

#[test]
fn rz33_general_notation_thresholds() {
    float(b"%.4g", 0.000125, b"0.000125");
    float(b"%.4g", 0.0000125, b"1.25e-005");
    float(b"%.4G", 10000.0, b"1E+004");
}

#[test]
fn rz34_general_round_before_notation() {
    float(b"%.4g", 9999.5, b"1e+004");
    float(b"%.0g", 12.5, b"1e+001");
    float(b"%#.4g", 12.5, b"12.50");
    float(b"%#.4G", 10000.0, b"1.000E+004");
}

#[test]
fn rz35_float_padding() {
    float(b"%+010.2f", 1.25, b"+000001.25");
    float(b"%010.2f", -1.25, b"-000001.25");
    float(b"%-010.2f", 1.25, b"1.25      ");
}

#[test]
fn rz36_negative_zero_and_rounded_zero() {
    float(b"%f", f64::from_bits(0x8000_0000_0000_0000), b"0.000000");
    float(b"%+g", -0.0, b"+0");
    float(b"%.0f", -0.125, b"-0");
}

#[test]
fn rz37_infinity_markers() {
    float(b"%f", f64::INFINITY, b"1.#INF00");
    float(b"%e", f64::INFINITY, b"1.#INF00e+000");
    float(b"%G", f64::INFINITY, b"1.#INF");
}

#[test]
fn rz38_nan_inner_sign() {
    float(b"%f", f64::from_bits(0x7ff8_0000_0000_0000), b"1.#QNAN0");
    float(b"%g", f64::from_bits(0x7ff8_0000_0000_0000), b"1.#QNAN");
    float(b"%+g", f64::from_bits(0xfff8_0000_0000_0000), b"+-1.#IND");
}

#[test]
fn rz39_rounding_special_letters() {
    float(b"%.0f", f64::INFINITY, b"1");
    float(b"%.2f", f64::INFINITY, b"1.#J");
    float(b"%.3f", f64::INFINITY, b"1.#IO");
}

#[test]
fn rz40_precision_cap_and_general_zero_precision() {
    let mut expected = b"1.".to_vec();
    expected.resize(514, b'0');
    float(b"%.513f", 1.0, &expected);
    float(b"%#.0g", 1.0, b"1.");
}

#[test]
fn default_legacy_provider_formats_finite_and_zero() {
    let mut string = RzString::from_bytes(b"unchanged");
    string
        .sprintf(
            b"prefix %f",
            &[FormatArg::Float(1.25)],
            &NumericPrimitives::<()>::default(),
        )
        .unwrap();
    assert_eq!(string.bytes(), b"prefix 1.250000");
    string
        .sprintf(
            b"%+f %p",
            &[FormatArg::Float(-0.0), FormatArg::Pointer(0x12ab)],
            &NumericPrimitives::<()>::default(),
        )
        .unwrap();
    assert_eq!(string.bytes(), b"+0.000000 000012AB");
}

#[test]
fn rounding_before_first_significant_digit_and_exponent_carry() {
    float(b"%.2f", 0.000125, b"0.00");
    float(b"%.1f", 0.0625, b"0.1");
    float(b"%.0e", 9.5, b"1e+001");
    float(b"%.0e", f64::from_bits(1), b"5e-324");
    float(b"%.0e", f64::MAX, b"2e+308");
    float(b"%+g", f64::NEG_INFINITY, b"-1.#INF");
    float(b"% g", f64::from_bits(0xfff8_0000_0000_0001), b" -1.#QNAN");
}

#[test]
fn invalid_decimal_provider_is_rejected_without_assignment() {
    struct Invalid(DecimalIntermediate);
    impl LegacyDecimal for Invalid {
        fn convert(&self, _: f64) -> Result<DecimalIntermediate, StringFault> {
            Ok(self.0.clone())
        }
    }
    for (digits, position) in [
        (&b""[..], 1),
        (b"123456789012345678", 1),
        (b"1x", 1),
        (b"01", 1),
        (b"1", -324),
        (b"1", 310),
    ] {
        let mut string = RzString::from_bytes(b"unchanged");
        let primitives = NumericPrimitives {
            decimal: Invalid(DecimalIntermediate {
                digits: digits.to_vec(),
                position,
            }),
        };
        assert!(matches!(
            string.sprintf(b"%f", &[FormatArg::Float(1.0)], &primitives),
            Err(StringFault::Argument)
        ));
        assert_eq!(string.bytes(), b"unchanged");
    }
}

fn finite_vector(bits: u64, raw: &[u8; 18], digits: &[u8], position: i32, format: &[u8], output: &[u8]) {
    let value = f64::from_bits(bits);
    assert_eq!(&super::legacy_decimal::raw_digits(value), raw);
    assert_eq!(
        ().convert(value).unwrap(),
        DecimalIntermediate {
            digits: digits.to_vec(),
            position
        }
    );
    float(format, value, output);
}

#[test]
fn rz41_smallest_subnormal() {
    finite_vector(
        1,
        b"494065645841246544",
        b"49406564584124654",
        -323,
        b"%.20e",
        b"4.94065645841246540000e-324",
    );
}

#[test]
fn rz42_largest_finite() {
    finite_vector(
        0x7fef_ffff_ffff_ffff,
        b"179769313486231570",
        b"17976931348623157",
        309,
        b"%.20e",
        b"1.79769313486231570000e+308",
    );
}

#[test]
fn rz43_largest_subnormal() {
    finite_vector(
        0x000f_ffff_ffff_ffff,
        b"222507385850720088",
        b"22250738585072009",
        -307,
        b"%.16e",
        b"2.2250738585072009e-308",
    );
}

#[test]
fn rz44_smallest_normal() {
    finite_vector(
        0x0010_0000_0000_0000,
        b"222507385850720138",
        b"22250738585072014",
        -307,
        b"%.16e",
        b"2.2250738585072014e-308",
    );
}

#[test]
fn rz45_legacy_truncation_quarter() {
    finite_vector(
        0x430c_6bf5_2634_0002,
        b"100000000000000024",
        b"10000000000000002",
        16,
        b"%.2f",
        b"1000000000000000.20",
    );
}

#[test]
fn rz46_legacy_truncation_three_quarters_and_sign() {
    finite_vector(
        0x430c_6bf5_2634_0006,
        b"100000000000000074",
        b"10000000000000007",
        16,
        b"%.2f",
        b"1000000000000000.70",
    );
    float(b"%.2f", f64::from_bits(0xc30c_6bf5_2634_0006), b"-1000000000000000.70");
}

#[test]
fn rz47_legacy_truncation_eighth() {
    finite_vector(
        0x42d6_bcc4_1e90_0008,
        b"100000000000000124",
        b"10000000000000012",
        15,
        b"%.3f",
        b"100000000000000.120",
    );
}

#[test]
fn rz48_binary_tenth_guard() {
    finite_vector(
        0x3fb9_9999_9999_999a,
        b"100000000000000005",
        b"10000000000000001",
        0,
        b"%.20e",
        b"1.00000000000000010000e-001",
    );
}

#[test]
fn rz49_leading_zero_corrected_by_guard() {
    finite_vector(
        0x3ff0_0000_0000_0000,
        b"099999999999999999",
        b"1",
        1,
        b"%.20e",
        b"1.00000000000000000000e+000",
    );
}

#[test]
fn rz50_guard_carry_changes_decimal_position() {
    finite_vector(
        0x4415_af1d_78b5_8c40,
        b"999999999999999999",
        b"1",
        21,
        b"%.16e",
        b"1.0000000000000000e+020",
    );
}
