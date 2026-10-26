// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The growth developer's pure arithmetic (§4–§7), each step in the
//! original's operation order at 24-bit x87 precision (foundations §18).

use crate::curve::ResponseCurve;
use crate::development::GrowthItem;
use crate::rng::{FixedSeeds, RzRandom};
use crate::x87::{ftol, ftol32, r24, rhe};

use super::tuning::PCT;

/// One layer's addition (§4.3): `W := low16(W + low16(ftol(curve output)))`,
/// wrapping in 16 bits (no clamping, §13.1 quirk 1).
pub fn add_curve(w: i16, out: f32) -> i16 {
    w.wrapping_add(ftol(f64::from(out)) as i16)
}

/// The commute layer's addition (§4.3 state 12): the sum is truncated,
/// `W := low16(ftol(r24(float(W) + curve output)))`.
pub fn commute_add(w: i16, out: f32) -> i16 {
    ftol(r24(f64::from(w) + f64::from(out))) as i16
}

/// The commute input of one tract (§4.3 state 12): the smallest non-zero
/// trip length, 255 when there is none.
pub fn commute_minimum(values: impl IntoIterator<Item = u8>) -> u8 {
    values.into_iter().fold(255, |m, v| if v != 0 && v < m { v } else { m })
}

/// The desirability fraction of state 18 (§5.2): `r24(float(W) × 0.01f)`
/// clamped to [0, 1].
pub fn desirability_fraction(w: i16) -> f32 {
    let d = r24(f64::from(w) * PCT);
    let d = if d < 0.0 {
        0.0
    } else if d <= 1.0 {
        d
    } else {
        1.0
    };
    d as f32
}

/// The population of a tract (§5.2): `p := r24(float(capacity) × d)`, stored
/// as the low 16 bits of the bits of `r24(p + 1.5·2²³)`, i.e. `rhe(p)`
/// (values above 32767 wrap negative in the `int16` grid, §13.1 quirk 8).
pub fn population_of(w: i16, capacity: u16) -> i16 {
    let d = desirability_fraction(w);
    let p = r24(f64::from(capacity) * f64::from(d)) as f32;
    rhe(p) as i16
}

/// The phantom-demand term of one growth item (§5.3); `None` when
/// `g = threshold − W ≤ 0` (the item adds nothing).
pub fn phantom_term(growth_threshold: i32, w: i16, population: i16) -> Option<i32> {
    let g = growth_threshold.wrapping_sub(i32::from(w));
    if g <= 0 {
        return None;
    }
    let mut f = r24(f64::from(g) * PCT);
    if f.is_nan() || f > 1.0 {
        f = 1.0;
    }
    Some(ftol32(r24(f64::from(population) * f)))
}

/// The maximum capacity of a tract from its float sum (§6.1): the low 16
/// bits of the bits of `r24(sum + 1.5·2²³)` (= `rhe(sum)` mod 65536).
pub fn round_capacity(sum: f32) -> u16 {
    rhe(sum) as u16
}

/// `addTo` of §6.3: a negative result stores 0, else the sum mod 65536.
pub fn add_to_capacity(old: u16, v: i32) -> u16 {
    let c = i64::from(old) + i64::from(v);
    if c < 0 {
        0
    } else {
        c as u16
    }
}

/// `q := delta / n` (C division, toward zero) and `r := delta − q·n` (§6.3);
/// `None` for `n = 0` ([choice], §13.1 quirk 7).
pub fn split_capacity(delta: i32, n: i32) -> Option<(i32, i32)> {
    if n == 0 {
        return None;
    }
    let q = delta.wrapping_div(n);
    Some((q, delta.wrapping_sub(q.wrapping_mul(n))))
}

/// The seed of a tract's decline key (§5.4 step 3):
/// `(((x << 16) + z) << 4) XOR salt` in 32-bit arithmetic.
pub fn decline_key_seed(x: i32, z: i32, salt: u32) -> u32 {
    ((x as u32).wrapping_shl(16).wrapping_add(z as u32)).wrapping_shl(4) ^ salt
}

/// The decline key (§5.4 step 3): `NextRaw()` of a fresh `cRZRandom` seeded
/// with [`decline_key_seed`]. A seed of `0xFFFFFFFF` takes the wall clock
/// (foundations §6.1, §7 [quirk]).
pub fn decline_key(x: i32, z: i32, salt: u32, wall_clock_seed: u32) -> u32 {
    let seeds = FixedSeeds {
        wall_clock: wall_clock_seed,
        process_clock: 0,
    };
    RzRandom::new(decline_key_seed(x, z, salt), &seeds).next_raw()
}

/// The stage assignment of §5.5 over items in list order; `curves` are the
/// stage curves 1..k and `size` the city size after the multiplier. With
/// no curves or no items nothing changes (every item keeps its stage).
pub fn assign_stages(items: &mut [GrowthItem], curves: &[ResponseCurve], size: f32) {
    let k = curves.len();
    let n = items.len();
    if k == 0 || n == 0 {
        return;
    }
    let eval = |s: usize| curves.get(s.wrapping_sub(1)).map_or(0.0, |c| c.eval(size));
    let mut acc = f64::from(eval(k));
    let mut s = k;
    // `float(n)` loads the count exactly; the division rounds once.
    let inv = r24(1.0 / n as f64);
    let mut i: i32 = 0;
    for item in items.iter_mut() {
        let p = r24(f64::from(i) * inv);
        // §5.5: the x87 C0 flag also stops the descent for unordered inputs.
        while p >= acc && s > 1 {
            s -= 1;
            acc = r24(f64::from(eval(s)) + acc);
        }
        item.max_stage = s as i32;
        i = i.wrapping_add(100);
    }
}

/// The supplies the city size of §5.5 is computed from.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CitySizeInputs {
    /// `GetConnectedCities` / `GetAllConnectedCities` counts.
    pub connected: u32,
    pub all_connected: u32,
    /// `QuerySupplyValue` of `(0x3001, 0)`, `(0x3301, 0x10000)`,
    /// `(0x1001, 0)` and `(0x4001, 0x20000)`.
    pub c_census: f32,
    pub c_region: f32,
    pub r_census: f32,
    pub i_total: f32,
    /// `GetStagePopulationMultiplier()`.
    pub multiplier: f32,
}

/// The city size of §5.5 for a target zone (1 R, 2 C, 3 I, other 0).
pub fn city_size(zone: u8, v: &CitySizeInputs) -> f32 {
    let commercial = || {
        let t = if v.all_connected > 0 {
            let ratio = r24(f64::from(v.connected) / f64::from(v.all_connected));
            r24(f64::from(v.c_region) * ratio)
        } else {
            0.0
        };
        r24(r24(t * 0.5) + f64::from(v.c_census))
    };
    let size = match zone {
        1 => {
            let x = commercial();
            let s1 = f64::from(v.r_census);
            if s1 < x {
                x
            } else {
                s1
            }
        }
        2 => commercial(),
        3 => f64::from(v.i_total),
        _ => 0.0,
    };
    r24(f64::from(v.multiplier) * size) as f32
}

/// `EstimateDesirability`'s input conversions (§4.6): park and landmark
/// (`int16`) and transient (`int8`) as the low byte, unsigned.
pub fn low_byte(v: i32) -> f32 {
    f32::from(v as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn population_examples() {
        // §14 test 1.
        assert_eq!(desirability_fraction(57).to_bits(), 0x3F11_EB85);
        assert_eq!(population_of(57, 1000), 570);
        assert_eq!(population_of(50, 3), 2);
        assert_eq!(population_of(50, 5), 2);
        assert_eq!(desirability_fraction(33).to_bits(), 0x3EA8_F5C2);
        assert_eq!(population_of(33, 7), 2);
        assert_eq!(population_of(99, 1), 1);
        assert_eq!(population_of(-20, 100), 0);
        assert_eq!(population_of(150, 40000), -25536);
    }

    #[test]
    fn wrap_and_commute() {
        // §14 tests 5 and 6.
        assert_eq!(add_curve(32760, 10.7), -32766);
        assert_eq!(commute_add(25, f32::from_bits(0xC0B5_F5F8)), 19);
        assert_eq!(add_curve(25, f32::from_bits(0xC0B5_F5F8)), 20);
        assert_eq!(commute_minimum([0, 0, 0]), 255);
        assert_eq!(commute_minimum([0, 200, 100]), 100);
    }

    #[test]
    fn decline_keys() {
        // §14 test 9.
        let salt = 0x0C63_D3DC;
        let want = [
            ((3, 5), 0x0C53_D38C, 0xD552_F46D),
            ((4, 5), 0x0C23_D38C, 0x8024_3FFD),
            ((10, 2), 0x0CC3_D3FC, 0x9C32_0AE0),
            ((0, 0), 0x0C63_D3DC, 0xF1CC_6936),
        ];
        for ((x, z), seed, key) in want {
            assert_eq!(decline_key_seed(x, z, salt), seed);
            assert_eq!(decline_key(x, z, salt, 0), key);
        }
        // The clock seed is substituted.
        let s = decline_key_seed(0, 0, 0xFFFF_FFFF);
        assert_eq!(s, 0xFFFF_FFFF);
        assert_eq!(decline_key(0, 0, 0xFFFF_FFFF, 1), decline_key(0, 0, 1, 7));
    }

    #[test]
    fn capacity_helpers() {
        // §14 tests 2, 3.
        assert_eq!(round_capacity(73.0), 73);
        assert_eq!(round_capacity(2.5), 2);
        assert_eq!(split_capacity(40, 6), Some((6, 4)));
        assert_eq!(split_capacity(-40, 6), Some((-6, -4)));
        assert_eq!(split_capacity(-40, 0), None);
        assert_eq!(add_to_capacity(4, -6), 0);
        assert_eq!(add_to_capacity(65535, 2), 1);
        // §14 test 11 by the rules of §5.3: 20 × 0.01f is a float32 tie that
        // rounds to even (0x3E4CCCCC), so the term is ftol(39.999996) = 39
        // (the spec's figures say 0.2f and 40).
        assert_eq!(phantom_term(50, 30, 200), Some(39));
        assert_eq!(phantom_term(50, 25, 200), Some(50));
        assert_eq!(phantom_term(50, 60, 200), None);
        assert_eq!(phantom_term(50, -32768, 7), Some(7));
    }
}
