// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::effects_parser::{parse_options, Index, NumericPolicy, OptionPrimitives, Text, Token};
use std::collections::BTreeMap;

/// Qualified zero-angle backend, plus arithmetic-stage fixtures. This is not a
/// claimed native oracle for nonzero angles or an emulator of 64-bit x87 PC.
#[derive(Default)]
struct Math {
    values: Vec<f64>,
    operations: Vec<(&'static str, f64, f64)>,
}
impl Math {
    fn put(&mut self, value: f64) -> Extended {
        let index = self.values.len();
        self.values.push(value);
        Extended(index as u32)
    }
    fn value(&self, value: Extended) -> f64 {
        self.values[value.0 as usize]
    }
}
impl Arithmetic for Math {
    fn input(&mut self, raw: RawFloat) -> Result<Extended> {
        match raw {
            RawFloat::Binary32(bits) => {
                let value = f64::from(f32::from_bits(bits));
                Ok(self.put(value))
            }
            _ => Err(Unsupported::Numeric),
        }
    }
    fn load(&mut self, value: f32) -> Result<Extended> {
        Ok(self.put(f64::from(value)))
    }
    fn store(&mut self, value: Extended) -> Result<f32> {
        let x = self.value(value);
        self.operations.push(("store", x, 0.));
        Ok(x as f32)
    }
    fn mul(&mut self, a: Extended, b: Extended) -> Result<Extended> {
        let (a, b) = (self.value(a), self.value(b));
        self.operations.push(("mul", a, b));
        Ok(self.put(a * b))
    }
    fn add(&mut self, a: Extended, b: Extended) -> Result<Extended> {
        let (a, b) = (self.value(a), self.value(b));
        self.operations.push(("add", a, b));
        Ok(self.put(a + b))
    }
    fn sub(&mut self, a: Extended, b: Extended) -> Result<Extended> {
        let (a, b) = (self.value(a), self.value(b));
        self.operations.push(("sub", a, b));
        Ok(self.put(a - b))
    }
    fn neg(&mut self, a: Extended) -> Result<Extended> {
        Ok(self.put(-self.value(a)))
    }
    fn cos(&mut self, angle: Extended) -> Result<Extended> {
        if self.value(angle) != 0. {
            return Err(Unsupported::Numeric);
        }
        Ok(self.put(1.))
    }
    fn sin(&mut self, angle: Extended) -> Result<Extended> {
        if self.value(angle) != 0. {
            return Err(Unsupported::Numeric);
        }
        Ok(self.put(self.value(angle)))
    }
}
fn matrix() -> Record {
    let mut record = Record::shared_constructor_projection();
    for (n, value) in (1..=9).enumerate() {
        record.word(4 + n * 4, (value as f32).to_bits());
    }
    record
}
#[derive(Default)]
struct Host {
    math: Math,
    hits: BTreeMap<&'static str, Vec<Index>>,
    floats: Vec<u32>,
    failure: Option<u32>,
}
impl OptionPrimitives for Host {
    fn rotation_arithmetic(&mut self) -> Option<&mut dyn Arithmetic> {
        Some(&mut self.math)
    }
    fn find(&mut self, _: &[Token], key: &'static str, _: u8, scratch: &mut Vec<Index>) -> u8 {
        scratch.clear();
        if let Some(indices) = self.hits.get(key) {
            scratch.clone_from(indices);
            1
        } else {
            0
        }
    }
    fn float(&mut self, token: &Token) -> RawFloat {
        self.floats.push(token.id.0);
        if self.failure == Some(token.id.0) {
            RawFloat::Unsupported
        } else {
            RawFloat::Binary32(0)
        }
    }
    fn uint(&mut self, _: &Token) -> u32 {
        panic!("unexpected uint")
    }
    fn vector(&mut self, _: &Token) -> [u32; 3] {
        panic!("unexpected vector")
    }
}
fn tokens() -> Vec<Token> {
    (0..9)
        .map(|i| Token {
            id: Index(i),
            text: Text(i.to_string()),
        })
        .collect()
}
#[test]
fn ro1_each_zero_axis_preserves_matrix_and_wraps_marks() {
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let mut r = matrix();
        let before = r.component.placement;
        r.component.placement[0] = 255;
        r.component.placement[1] = 0x80;
        apply(&mut r, axis, RawFloat::Binary32(0), &mut Math::default()).unwrap();
        assert_eq!(&r.component.placement[4..40], &before[4..40]);
        assert_eq!(&r.component.placement[..2], &[0, 0x84]);
    }
}
#[test]
fn ro2_all_keys_have_fixed_axis_conversion_order_and_nine_marks() {
    let mut h = Host::default();
    for (key, ids) in [
        ("rotateX", vec![0]),
        ("rotateY", vec![1]),
        ("rotateZ", vec![2]),
        ("rotateXYZ", vec![3, 4, 5]),
        ("rotateZXY", vec![6, 7, 8]),
    ] {
        h.hits.insert(key, ids.into_iter().map(Index).collect());
    }
    let mut r = matrix();
    let before = r.component.placement;
    r.component.placement[0] = 250;
    parse_options(
        &tokens(),
        &mut Vec::new(),
        0,
        &mut r,
        NumericPolicy::NearestAtLeast53,
        &mut h,
    )
    .unwrap();
    assert_eq!(h.floats, (0..9).collect::<Vec<_>>());
    assert_eq!(r.component.placement[0], 3);
    assert_eq!(&r.component.placement[4..40], &before[4..40]);
}
#[test]
fn ro3_ro4_compound_failures_retain_only_completed_axes() {
    for (key, failure, count) in [("rotateXYZ", 2, 12), ("rotateZXY", 0, 10)] {
        let mut h = Host {
            failure: Some(failure),
            ..Host::default()
        };
        h.hits.insert(key, vec![Index(0), Index(1), Index(2)]);
        let mut r = matrix();
        let before = r.component.placement;
        r.component.placement[0] = 10;
        assert_eq!(
            parse_options(
                &tokens(),
                &mut Vec::new(),
                0,
                &mut r,
                NumericPolicy::NearestAtLeast53,
                &mut h
            ),
            Err(Unsupported::Numeric)
        );
        assert_eq!(r.component.placement[0], count);
        assert_eq!(&r.component.placement[4..40], &before[4..40]);
        assert_eq!(h.floats.len(), failure as usize + 1);
    }
}
#[test]
fn ro5_ro6_arithmetic_stage_sine_store_asymmetries() {
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let mut r = matrix();
        for n in 0..9 {
            r.word(4 + n * 4, 1f32.to_bits());
        }
        let mut math = Math::default();
        let c = math.put(0.5);
        let sine = 1. + 2f64.powi(-24);
        let s = math.put(sine);
        compose(&mut r, axis, c, s, &mut math).unwrap();
        let sums: Vec<_> = math
            .operations
            .iter()
            .filter(|(op, _, _)| *op == "add" || *op == "sub")
            .copied()
            .collect();
        let op = if axis == Axis::Y { "sub" } else { "add" };
        assert_eq!(sums[0], (op, 0.5, sine));
        assert_eq!(sums[1], (op, 0.5, 1.));
        assert_eq!(sums[2], (op, 0.5, 1.));
        if axis == Axis::Y {
            assert_eq!(sums[3], ("add", 1., 0.5));
            assert_eq!(sums[4], ("add", 1., 0.5));
            assert_eq!(sums[5], ("add", sine, 0.5));
        } else {
            assert_eq!(sums[3], ("add", -1., 0.5));
            assert_eq!(sums[4], ("add", -1., 0.5));
            assert_eq!(sums[5], ("add", -1., 0.5));
        }
    }
}
