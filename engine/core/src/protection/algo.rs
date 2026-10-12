// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The arithmetic of §5–§9 as pure functions. Every float operation is one
//! `f32` rounding (foundations §18); an integer or `double` operand enters
//! unrounded (the operation is computed in `f64` and rounded once).

use crate::crime::rhe as rhe_trick;
use crate::props::round_half_even_f32;
use crate::x87::ftol32;

use super::Effect;

/// The float32 constant `0.01f` (`0x3C23D70A` = 0.0099999998, §6.2).
pub(crate) const ONE_PERCENT: f32 = 0.01;
/// The float32 constant `100.0f` (§6.2).
pub(crate) const HUNDRED: f32 = 100.0;

/// `x × 0.01f` (§6.2, PR3): one `f32` rounding of the product with the
/// float32 constant, so `10 × 0.01f` = 0.099999994, not 0.1. Also the "pct"
/// tuning getter (§6.7).
pub fn pct(x: f32) -> f32 {
    (f64::from(x) * f64::from(ONE_PERCENT)) as f32
}

/// `x × 100.0f`: one `f32` rounding.
pub(crate) fn times_hundred(x: f32) -> f32 {
    (f64::from(x) * f64::from(HUNDRED)) as f32
}

/// `int × float`: one `f32` rounding of the exact product.
pub(crate) fn mul_i(i: f64, f: f32) -> f32 {
    (i * f64::from(f)) as f32
}

/// `int / int` in floating point: one `f32` rounding.
pub(crate) fn div_ii(a: f64, b: f64) -> f32 {
    (a / b) as f32
}

/// `roundHalfEven` to an integer, saturating; NaN → 0 (the adjusted copy,
/// §2, whose inputs are `int16` values).
pub(crate) fn rhe(x: f32) -> i64 {
    round_half_even_f32(x) as i64
}

/// The jails' `roundHalfEven` (§9.1, [quirk]): add `12582912.0f`, store as
/// float32, subtract the bit pattern `0x4B400000`. Exact round half to even
/// for |v| < 2^22; other values above that (never reached by shipped
/// capacities). The `u32` is the 32-bit result as stored.
pub(crate) fn rhe_jail(x: f32) -> u32 {
    rhe_trick(x) as u32
}

/// `trunc` (the CRT `_ftol`) of an `f32` value to 32 bits.
pub(crate) fn trunc(x: f32) -> i32 {
    ftol32(f64::from(x))
}

/// The adjusted copy of an unadjusted value (§2, §5.3):
/// `clamp(roundHalfEven(v × factor), 0, 255)`; NaN reads as 0.
pub fn adjusted_value(v: i16, factor: f32) -> u8 {
    rhe(mul_i(f64::from(v), factor)).clamp(0, 255) as u8
}

/// `AdjustProtectionEffect` (§7.1): radius × e, `trunc(centre × e)`,
/// `trunc(outer × e)`; the original centre is kept.
pub fn scale_effect(e: Effect, efficiency: f32) -> Effect {
    Effect {
        radius: e.radius * efficiency,
        centre: trunc(mul_i(f64::from(e.centre), efficiency)),
        outer: trunc(mul_i(f64::from(e.outer), efficiency)),
        ..e
    }
}

/// `GetOccupantCoverage` (§7.3) for a successful effect: (cover radius,
/// station radius). `t := trunc(originalCentre × p)`; `t > centre`
/// (unsigned) → 0; `t < outer` → radius; else
/// `(centre − t) × radius / (centre − outer)`.
pub fn occupant_coverage(e: &Effect, p: f32) -> (f32, f32) {
    let t = trunc(mul_i(f64::from(e.original_centre), p));
    let cover = if (t as u32) > (e.centre as u32) {
        0.0
    } else if t < e.outer {
        e.radius
    } else {
        let num = mul_i(f64::from(e.centre.wrapping_sub(t)), e.radius);
        (f64::from(num) / f64::from(e.centre.wrapping_sub(e.outer))) as f32
    };
    (cover, e.radius)
}

/// `GetChanceOfStrike` (§6.4) over per-station chances: `p := p + c − p·c`
/// from 0, each operation one `f32` rounding.
pub fn combine_strike_chance(chances: &[f32]) -> f32 {
    chances.iter().fold(0.0f32, |p, &c| (p + c) - p * c)
}

/// `(1 << n) − 1` with the x86 shift count (`n` modulo 32).
pub fn unit_mask(n: u32) -> u32 {
    (1u32 << (n & 31)).wrapping_sub(1)
}

fn bit(i: u32) -> u32 {
    1u32 << (i & 31)
}

/// `DispatchFromStation` (§8.2) on the unit state (`flags` already masked
/// to `count` units): the new mask and the call ID, or `None` when the
/// requested unit index is > count. With `id` = 0 the lowest free unit is
/// taken (none free: index `count`, mask unchanged, quirk 6); with `id` > 0
/// that unit's bit is cleared without checking it.
pub fn dispatch_from_station_mask(flags: u32, base: u32, count: u32, id: u32) -> Option<(u32, u32)> {
    let mut mask = flags;
    let i = if id > 0 {
        let i = id.wrapping_sub(base);
        if i > count {
            return None;
        }
        mask &= !bit(i);
        i
    } else {
        match (0..count.min(32)).find(|&i| mask & bit(i) != 0) {
            Some(i) => {
                mask &= !bit(i);
                i
            }
            None => count,
        }
    };
    Some((mask, base.wrapping_add(i)))
}

/// `RevokeDispatchFromStation` (§8.2): `None` when the number of set bits is
/// ≥ count or the unit's bit is already set; else the new mask.
pub fn revoke_dispatch_mask(flags: u32, base: u32, count: u32, id: u32) -> Option<u32> {
    if flags.count_ones() >= count {
        return None;
    }
    let b = bit(id.wrapping_sub(base));
    if flags & b != 0 {
        return None;
    }
    Some(flags | b)
}

/// The base call number of a new station with `count` units (§8.2): the
/// IDs `[base, base + count)` of the other stations are marked below
/// `next_call`; the result is the start of the first run of `count` free IDs
/// from 1. Computed with intervals in 64 bits ([choice]).
pub fn choose_base_call_number(others: &[(u32, u32)], next_call: u32, count: u32) -> u32 {
    let limit = u64::from(next_call);
    let mut used: Vec<(u64, u64)> = others
        .iter()
        .map(|&(b, c)| (u64::from(b), (u64::from(b) + u64::from(c)).min(limit)))
        .filter(|&(s, e)| s < e)
        .collect();
    used.sort_unstable();
    let mut candidate = 1u64;
    for (s, e) in used {
        if e <= candidate {
            continue;
        }
        if s >= candidate + u64::from(count) {
            break;
        }
        candidate = e;
    }
    candidate as u32
}

/// `capacity(j) := roundHalfEven(capacityProperty × jailEff)` (§9.1): the
/// `u32` property times the efficiency, one `f32` rounding, then the jails'
/// rounding trick.
pub fn jail_capacity(capacity_property: u32, efficiency: f32) -> u32 {
    rhe_jail(mul_i(f64::from(capacity_property), efficiency))
}

/// The x87 default NaN `0xFFC00000` that `0 / 0` stores (§9.1).
pub const DEFAULT_NAN: u32 = 0xFFC0_0000;

/// `GetJailedPercentage` (§9.1, PR15): 1.0 when jailed > capacity (signed),
/// else `jailed / capacity`, one `f32` rounding, with **no guard**: 0/0 is
/// the default NaN `0xFFC00000`, a negative jailed over capacity 0 is −∞.
pub fn jailed_percentage(jailed: i32, capacity: i32) -> f32 {
    if jailed > capacity {
        1.0
    } else if jailed == 0 && capacity == 0 {
        f32::from_bits(DEFAULT_NAN)
    } else {
        div_ii(f64::from(jailed), f64::from(capacity))
    }
}

/// The month's end test for a jail in a jailbreak (§9.5 Comparisons):
/// `jailedPercentage < endThreshold`, ordered, so a NaN percentage (no jail
/// capacity and no jailed) never ends a jailbreak, nor does 1.0 (PR15).
pub fn jailbreak_ends(jailed_percentage: f32, end_threshold: f32) -> bool {
    jailed_percentage < end_threshold
}

/// The month's start test (§9.5): a draw is made only when `chance > 0`
/// (ordered: zero, negative and NaN skip it), and the jailbreak begins when
/// `draw < chance` (ordered). `draw` is called at most once.
pub fn jailbreak_starts(chance: f32, draw: impl FnOnce() -> f64) -> bool {
    chance > 0.0 && draw() < f64::from(chance)
}

/// One jail's population in `Recalculate` (§9.1):
/// `roundHalfEven(capacity(j) × ratio)`.
pub fn ratio_share(capacity: u32, ratio: f32) -> u32 {
    rhe_jail(mul_i(f64::from(capacity), ratio))
}

/// Rehabilitation (§9.5): `k := roundHalfEven(f32(jailed × rate))`;
/// `jailed := max(0, jailed − k)`. Returns (new jailed, k). With the shipped
/// rate 0.099999994, 135 → 13.499999 → 13 and jailed ≤ 5 never decreases.
pub fn rehabilitation(jailed: i32, rate: f32) -> (i32, i32) {
    let k = rhe_trick(mul_i(f64::from(jailed), rate));
    (jailed.wrapping_sub(k).max(0), k)
}

/// `ResetUnjailedCriminals` (§9.3): (released, new jailed) with
/// `cap := trunc(capacity × overflow)`.
pub fn reset_unjailed(capacity: i32, jailed: i32, unarrested: i32, overflow: f32) -> (i32, i32) {
    let cap = trunc(mul_i(f64::from(capacity), overflow));
    let mut released = unarrested;
    let mut jailed = jailed;
    if jailed >= cap {
        released = released.wrapping_add(jailed.wrapping_sub(cap));
        jailed = cap;
    }
    (released, jailed)
}

/// The escapees of a jailbreak (§9.5): `roundHalfEven(capacity(j) × r)`
/// with `r` the generator's double, one `f32` rounding of the product.
pub fn escapees(capacity: u32, r: f64) -> i32 {
    rhe_trick((f64::from(capacity) * r) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_and_calls() {
        // §18 test 16.
        let (m, id) = dispatch_from_station_mask(0b111, 5, 3, 0).unwrap();
        assert_eq!((m, id), (0b110, 5));
        let (m, id) = dispatch_from_station_mask(m, 5, 3, 0).unwrap();
        assert_eq!((m, id), (0b100, 6));
        let m = revoke_dispatch_mask(m, 5, 3, 5).unwrap();
        assert_eq!(m, 0b101);
        assert_eq!(revoke_dispatch_mask(m, 5, 3, 5), None);
        assert_eq!(dispatch_from_station_mask(0, 5, 3, 0), Some((0, 8)));
        assert_eq!(dispatch_from_station_mask(0b111, 5, 3, 9), None);
        assert_eq!(
            dispatch_from_station_mask(0b101, 5, 3, 6),
            Some((0b101, 6)),
            "unchecked"
        );
        assert_eq!(revoke_dispatch_mask(0b111, 5, 3, 5), None, "all free");
        assert_eq!(
            (unit_mask(3), unit_mask(0), unit_mask(32), unit_mask(31)),
            (7, 0, 0, 0x7FFF_FFFF)
        );
        // §18 test 17.
        assert_eq!(choose_base_call_number(&[], 1, 2), 1);
        assert_eq!(choose_base_call_number(&[(1, 2)], 3, 3), 3);
        assert_eq!(choose_base_call_number(&[(3, 3)], 6, 2), 1);
        assert_eq!(choose_base_call_number(&[(3, 3)], 6, 3), 6);
        assert_eq!(choose_base_call_number(&[(4, 6), (2, 1)], 10, 2), 10);
        assert_eq!(choose_base_call_number(&[(1, 9)], 4, 1), 4, "marks stop at nextCall");
        assert_eq!(choose_base_call_number(&[(u32::MAX, u32::MAX)], u32::MAX, u32::MAX), 1);
    }

    #[test]
    fn jail_numbers() {
        // §18 test 9 with the shipped rate pct(10) = 0.099999994.
        let rate = pct(10.0);
        assert_eq!(rate.to_bits(), 0x3DCC_CCCC);
        assert_eq!(rehabilitation(600, rate), (540, 60));
        assert_eq!(rehabilitation(125, rate), (113, 12));
        assert_eq!(rehabilitation(135, rate), (122, 13), "13.499999 -> 13");
        assert_eq!(rehabilitation(6, rate), (5, 1));
        assert_eq!(rehabilitation(5, rate), (5, 0), "0.49999997 -> 0");
        assert_eq!(rehabilitation(3, 2.0), (0, 6));
        assert_eq!(reset_unjailed(1000, 1200, 7, 1.1), (107, 1100));
        assert_eq!(reset_unjailed(1000, 900, 7, 1.1), (7, 900));
        assert_eq!(ratio_share(400, jailed_percentage(600, 1000)), 240);
        assert_eq!(ratio_share(600, 0.6), 360);
        assert!(jailed_percentage(0, 0).is_nan());
        assert_eq!(jailed_percentage(5, 0), 1.0);
        assert_eq!(escapees(600, 0.25), 150);
        assert_eq!(jail_capacity(400, 0.75), 300);
        // The rounding trick (§9.1): exact below 2^22, other values above.
        assert_eq!(jail_capacity(5_000_000, 1.0), 4_597_152);
        assert_eq!(jailed_percentage(0, 0).to_bits(), DEFAULT_NAN);
        assert_eq!(jailed_percentage(-1, 0), f32::NEG_INFINITY);
        assert_eq!(jailed_percentage(800, 1000).to_bits(), 0x3F4C_CCCD);
        // §18 test 24: ordered comparisons; NaN and 1.0 never end a jailbreak.
        let end = pct(80.0);
        assert!(!jailbreak_ends(jailed_percentage(800, 1000), end));
        assert!(jailbreak_ends(jailed_percentage(799, 1000), end));
        assert!(!jailbreak_ends(jailed_percentage(0, 0), end));
        assert!(!jailbreak_ends(jailed_percentage(900, 0), end));
        let mut draws = 0;
        assert!(!jailbreak_starts(f32::from_bits(DEFAULT_NAN), || {
            draws += 1;
            0.0
        }));
        assert!(!jailbreak_starts(0.0, || {
            draws += 1;
            0.0
        }));
        assert_eq!(draws, 0);
        assert!(!jailbreak_starts(pct(40.0), || f64::from(pct(40.0))));
        assert!(jailbreak_starts(pct(40.0), || 0.39));
    }

    #[test]
    fn effects_and_coverage() {
        let e = Effect {
            radius: 300.0,
            centre: 100,
            outer: 21,
            original_centre: 100,
            ..Effect::default()
        };
        let s = scale_effect(e, 0.75);
        assert_eq!((s.centre, s.outer, s.radius, s.original_centre), (75, 15, 225.0, 100));
        let e = Effect {
            radius: 400.0,
            centre: 100,
            outer: 20,
            original_centre: 100,
            ..Effect::default()
        };
        assert_eq!(occupant_coverage(&e, 0.5), (250.0, 400.0));
        assert_eq!(occupant_coverage(&e, 0.1), (400.0, 400.0));
        assert_eq!(occupant_coverage(&e, 1.1), (0.0, 400.0));
        let h = scale_effect(e, 0.5);
        assert_eq!(occupant_coverage(&h, 0.5), (0.0, 200.0));
        // §18 test 5: 0.099999994, 0.39999998 -> 0.45999998.
        assert_eq!(combine_strike_chance(&[pct(10.0), pct(40.0)]).to_bits(), 0x3EEB_851E);
        assert_eq!(combine_strike_chance(&[]), 0.0);
        assert_eq!(
            [
                adjusted_value(-5, 1.0),
                adjusted_value(300, 1.0),
                adjusted_value(128, 1.0)
            ],
            [0, 255, 128]
        );
        assert_eq!(adjusted_value(5, f32::NAN), 0);
        // §18 test 25 (float32 0.01f).
        for (v, b) in [
            (10.0, 0x3DCC_CCCC),
            (20.0, 0x3E4C_CCCC),
            (60.0, 0x3F19_9999),
            (80.0, 0x3F4C_CCCC),
            (90.0, 0x3F66_6666),
            (110.0, 0x3F8C_CCCD),
            (120.0, 0x3F99_9999),
        ] {
            assert_eq!(pct(v).to_bits(), b, "{v}");
        }
    }
}
