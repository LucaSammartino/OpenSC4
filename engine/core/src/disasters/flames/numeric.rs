// SPDX-License-Identifier: GPL-3.0-or-later
//! FF3 owner-local, qualified PC24/nearest-even/masked-exception arithmetic.
//! No ambient thread-control guarantee. Operands are finite normal binary32 or
//! zero; registers have 24 significant bits and exponents in [-400,400].
//! Products of two 24-bit operands are exact in binary64. Adds within 25 binary
//! orders are exact there too; more distant operands cannot affect PC24 rounding.
//! Thus rounding these binary64 results to 24 bits is exact in this domain.
//! Sqrt is supported ONLY when an exact 24-bit dyadic candidate verifies by
//! exact multiplication. Reciprocal uses exact midpoint products (25*24 bits).
//! No irrational sqrt or PC53 claim.
use super::{PlacementError, Unsupported};
use crate::camera::Vec3;
use crate::network_occupants::Xform;

type R<T> = Result<T, PlacementError>;
const BAD: PlacementError = PlacementError::Unsupported(Unsupported::Numeric);

fn domain(x: f64) -> R<f64> {
    let exponent = ((x.to_bits() >> 52) & 2047) as i32 - 1023;
    if x == 0.0 || (x.is_finite() && (-400..=400).contains(&exponent)) {
        Ok(x)
    } else {
        Err(BAD)
    }
}

/// Round significand only, retaining binary64's (qualified wider) exponent.
fn r(x: f64) -> R<f64> {
    domain(x)?;
    if x == 0.0 {
        return Ok(x);
    }
    let bits = x.to_bits();
    let low = bits & ((1 << 29) - 1);
    let mut high = bits >> 29;
    if low > (1 << 28) || (low == (1 << 28) && high & 1 != 0) {
        high += 1;
    }
    domain(f64::from_bits(high << 29))
}

pub(super) fn input(x: f32) -> R<f64> {
    if x == 0.0 || x.is_normal() {
        Ok(f64::from(x))
    } else {
        Err(BAD)
    }
}
pub(super) fn f(x: f64) -> R<f64> {
    input(x as f32)
}
pub(super) fn add(x: f64, y: f64) -> R<f64> {
    r(x + y)
}
pub(super) fn sub(x: f64, y: f64) -> R<f64> {
    r(x - y)
}
pub(super) fn mul(x: f64, y: f64) -> R<f64> {
    r(x * y)
}

fn sqrt(x: f64) -> R<f64> {
    if x <= 0.0 {
        return Err(PlacementError::Unsupported(Unsupported::DegenerateNormal));
    }
    // Candidate is not a native FSQRT assertion: exact square verification
    // qualifies the result, and rejects all other roots.
    let candidate = r(x.sqrt())?;
    if candidate * candidate == x {
        Ok(candidate)
    } else {
        Err(BAD)
    }
}
fn reciprocal(x: f64) -> R<f64> {
    if x <= 0.0 {
        return Err(BAD);
    }
    let candidate = r(1.0 / x)?;
    let previous = domain(f64::from_bits(candidate.to_bits() - (1 << 29)))?;
    let next = domain(f64::from_bits(candidate.to_bits() + (1 << 29)))?;
    let lower_product = ((previous + candidate) * 0.5) * x;
    let upper_product = ((candidate + next) * 0.5) * x;
    let odd = (candidate.to_bits() >> 29) & 1 != 0;
    // Binary64 division proposes a nearby candidate, not a native result.
    // Exact 49-bit products adjudicate both boundaries, including ties/even.
    if lower_product > 1.0 || (lower_product == 1.0 && odd) {
        Ok(previous)
    } else if upper_product < 1.0 || (upper_product == 1.0 && odd) {
        Ok(next)
    } else {
        Ok(candidate)
    }
}

fn differences(a: Vec3, b: Vec3) -> R<[f64; 3]> {
    Ok([
        sub(input(a[0])?, input(b[0])?)?,
        sub(input(a[1])?, input(b[1])?)?,
        sub(input(a[2])?, input(b[2])?)?,
    ])
}

fn area_cross(v: [Vec3; 3]) -> R<[f64; 3]> {
    let a = differences(v[0], v[1])?;
    let mut b = differences(v[2], v[1])?;
    b[0] = f(b[0])?;
    b[1] = f(b[1])?;
    Ok([
        f(sub(mul(a[1], b[2])?, mul(a[2], b[1])?)?)?,
        f(sub(mul(a[2], b[0])?, mul(a[0], b[2])?)?)?,
        sub(mul(a[0], b[1])?, mul(a[1], b[0])?)?,
    ])
}
fn normal_cross(v: [Vec3; 3]) -> R<[f64; 3]> {
    let mut a = differences(v[2], v[0])?;
    a[1] = f(a[1])?;
    a[2] = f(a[2])?;
    let b = differences(v[1], v[0])?.map(f);
    let b = [b[0]?, b[1]?, b[2]?];
    Ok([
        f(sub(mul(b[1], a[2])?, mul(b[2], a[1])?)?)?,
        f(sub(mul(b[2], a[0])?, mul(b[0], a[2])?)?)?,
        f(sub(mul(b[0], a[1])?, mul(b[1], a[0])?)?)?,
    ])
}

pub(super) fn triangle(v: [Vec3; 3], previous: f32) -> R<(Vec3, f32)> {
    let c = area_cross(v)?;
    let area = sqrt(add(add(mul(c[2], c[2])?, mul(c[1], c[1])?)?, mul(c[0], c[0])?)?)?;
    let cumulative = f(add(input(previous)?, area)?)? as f32;
    let n = normal_cross(v)?;
    let length = sqrt(add(add(mul(n[0], n[0])?, mul(n[1], n[1])?)?, mul(n[2], n[2])?)?)?;
    let inv = reciprocal(length)?;
    Ok((
        [
            f(mul(n[0], inv)?)? as f32,
            f(mul(n[1], inv)?)? as f32,
            f(mul(n[2], inv)?)? as f32,
        ],
        cumulative,
    ))
}

pub(super) fn range(draw: f64, low: f64, high: f64) -> R<f64> {
    // RzRandom supplies only the raw published uniform primitive. Retain its
    // lineage/draw count; use wider-exponent arithmetic for this owner's range.
    add(mul(draw, sub(high, low)?)?, low)
}

pub(super) fn place(v: [Vec3; 3], n: Vec3, t: &Xform, u: f64, second: f64) -> R<Vec3> {
    let u = f(u)?;
    let k = sub(1.0, u)?;
    let second = range(second, 0.0, k)?; // double copy of retained k, NOT F(k)
    place_weights(v, n, t, u, second, sub(f(k)?, second)?)
}

fn place_weights(v: [Vec3; 3], n: Vec3, t: &Xform, u: f64, q: f64, w: f64) -> R<Vec3> {
    let mut p = [[0.0; 3]; 3];
    for (j, weight) in [u, q, w].into_iter().enumerate() {
        for (product, value) in p[j].iter_mut().zip(v[j]) {
            *product = mul(weight, input(value)?)?;
        }
    }
    let x = f(add(f(add(p[0][0], p[1][0])?)?, f(p[2][0])?)?)?;
    let y = f(add(add(f(p[0][1])?, p[1][1])?, f(p[2][1])?)?)?;
    let z = add(p[2][2], f(add(f(p[0][2])?, f(p[1][2])?)?)?)?;
    let lx = f(add(input(n[0])?, x)?)?;
    let ly = add(input(n[1])?, y)?;
    let lz = add(z, input(n[2])?)?;
    let mut result = [0.0; 3];
    for (j, output) in result.iter_mut().enumerate() {
        let row = &t.matrix[j * 3..j * 3 + 3];
        let value = add(
            add(mul(lz, input(row[2])?)?, mul(ly, input(row[1])?)?)?,
            mul(lx, input(row[0])?)?,
        )?;
        // Row 2's output float copies are discarded; even an overflowing copy
        // does not replace the retained register. Rows 0/1 reload their copies.
        let value = if j < 2 { f(value)? } else { value };
        let value = mul(value, input(t.scale)?)?;
        let value = if j < 2 { f(value)? } else { value };
        *output = f(add(value, input(t.translation[j])?)?)? as f32;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cumulative_is_stored_for_every_triangle_not_only_at_end() {
        let big = [[0., 0., 0.], [4096., 0., 0.], [0., 4096., 0.]];
        let small = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let (normal, a) = triangle(big, 0.).unwrap();
        assert_eq!(normal, [0., 0., 1.]);
        let (_, b) = triangle(small, a).unwrap();
        let (_, c) = triangle(small, b).unwrap();
        assert_eq!([a, b, c], [16777216., 16777216., 16777216.]);
    }

    #[test]
    fn pc24_exponent_rescue_and_asymmetric_product_stores() {
        let h = f32::from_bits(0x7f00_0000); // exactly 2^127
        let v = [[0., 0., h], [1., 0., h], [0., 1., h]];
        let t = Xform {
            matrix: [1., 0., 0., 0., 1., 0., 0., 0., 4.],
            scale: 0.25,
            ..Xform::default()
        };
        // Prescribed u=.25, v=.25 (second raw uniform .25/.75=1/3
        // isn't dyadic), so test the independent arithmetic weights directly.
        assert_eq!(
            place_weights(v, [0., 0., 1.], &t, 0.25, 0.25, 0.5).unwrap(),
            [0.0625, 0.125, h]
        );
        // Row0/1 MUST reload their F copy, so the analogous overflowing
        // output is excluded before its unproved continuation (unlike row2).
        for axis in [0, 1] {
            let mut v = [[0.; 3]; 3];
            for vertex in &mut v {
                vertex[axis] = h;
            }
            let mut t = Xform {
                scale: 0.25,
                ..Xform::default()
            };
            t.matrix[axis * 3 + axis] = 4.;
            assert_eq!(place_weights(v, [0.; 3], &t, 0.25, 0.25, 0.5), Err(BAD));
        }
        // Published 53-bit cancellation vector is NOT a PC53 claim. PC24 loses
        // the half-unit in arithmetic itself, independently of output spills.
        let v = [[0., 0., 1.], [1., 0., 1.], [0., 1., 67108864.]];
        let t = Xform {
            translation: [0., 0., -33554432.],
            ..Xform::default()
        };
        assert_eq!(
            place_weights(v, [0., -1., 2f32.powi(-26)], &t, 0.25, 0.25, 0.5).unwrap(),
            [0.25, -0.5, 0.]
        );
    }

    #[test]
    fn precise_exclusions_and_wider_register_rounding() {
        assert_eq!(sqrt(2.), Err(BAD));
        assert_eq!(reciprocal(3.).unwrap(), f64::from(f32::from_bits(0x3eaa_aaab)));
        assert_eq!(input(f32::from_bits(1)), Err(BAD));
        assert_eq!(input(f32::NAN), Err(BAD));
        assert_eq!(mul(2f64.powi(127), 4.).unwrap(), 2f64.powi(129));
        assert_eq!(add(16777216., 1.).unwrap(), 16777216.);
        assert_eq!(add(16777218., 1.).unwrap(), 16777220.);
    }

    #[test]
    fn area_retains_cross_z_but_normal_spills_all_components() {
        let h = f32::from_bits(0x7f00_0000); // 2^127
        let v = [[-h, 0., 0.], [h, 0., 0.], [0., 1., 0.]];
        // a.x=-2^128 retained, b.x=-2^127 stored: area cross z=-2^128.
        assert_eq!(area_cross(v).unwrap(), [0., 0., -2f64.powi(128)]);
        // Normal uses v1-v0=2^128, which its explicit F cannot represent.
        assert_eq!(normal_cross(v), Err(BAD));
    }
}
