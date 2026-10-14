// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The landfill developer's pure arithmetic (§4.3, §4.6.1, §4.7.3, §4.7.4,
//! §4.8). Every function here is a formula of the spec with no host access, so
//! conformance vectors can pin it directly.
//!
//! Float arithmetic follows foundations §18 (24-bit x87): each `+ − × ÷` is
//! one [`r24`] rounding, and an integer or `double` operand enters that one
//! operation unrounded (§18.3 point 1). `round(v)` in the spec is the class's
//! **magic-number round** `bits(f32(v + 12582912.0)) − 0x4B400000`
//! ([`crate::x87::rhe`], [`magic_round`]), which is round-half-to-even for
//! `0 ≤ v < 2²²` and wrong above it (§4, quirk 14). `float(u32)` is
//! exact-as-int64 then rounded to `f32` ([`crate::x87::u32_to_float`], §4).

use crate::grid::CellRect;
use crate::x87::{r24, rhe, u32_to_float};

use super::{tile, CellRegion};

/// The spec's `round(v)` (§4): the magic-number round
/// `bits(f32(v + 12582912.0)) − 0x4B400000`, as an `i32`. Round-half-to-even
/// for `0 ≤ v < 2²²`; a value in `[2²², 2²³)` comes out as `2²² + (v − 2²²)/2`
/// *[quirk]* (§4, quirk 14).
pub fn magic_round(v: f64) -> i32 {
    rhe(v as f32)
}

/// `float(u32 v)` (§4): exact as `int64`, then rounded to `float32` (values
/// ≥ 2³¹ get `+2³²`). Returned as `f64` holding an `f32`-representable value.
pub fn float_of_u32(v: u32) -> f64 {
    u32_to_float(v as i32)
}

/// The four results of `GetAltitudeVariance` (§4.3).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Variance {
    /// `|max − min|` as `f32`.
    pub variance: f32,
    /// `count > 0 ? sum / count : 0`.
    pub mean: f32,
    pub max: f32,
    pub min: f32,
}

/// `GetAltitudeVariance(rect)` over **every cell of the rect** (not the mask)
/// (§4.3). `altitude` is called with the cell coordinates and must return the
/// cell-centre altitude (`cISTETerrain::GetAltitude` at the cell centre,
/// §4.2); `sea` is `terrain.GetSeaLevel()` (slot 29).
///
/// 1. `max := min := altitude(centre cell)` — **not** sea-level adjusted.
/// 2. For `x` in `x0…x1` (outer), `z` in `z0…z1` (inner): `a := altitude(x,z)`;
///    if `a ≤ sea`: `a := sea + 1.0`; if `a < min`: `min := a`, else if
///    `a > max`: `max := a`; `sum += a`, `count += 1.0` (both `f32`).
/// 3. `mean := count > 0 ? sum / count : 0`; `variance := |max − min|`.
///
/// The centre cell is [`CellRegion::centre_cell`] of the rect. A cell at or
/// below sea level counts as `sea + 1.0` for the variance and the mean, but
/// [`zoning_cost`] uses the **raw** altitudes (§4.3 step 3, conformance
/// test 5).
pub fn altitude_variance(rect: CellRect, sea: f32, altitude: impl Fn(i32, i32) -> f32) -> Variance {
    // 1. the centre cell, unadjusted.
    let (cx, cz) = CellRegion::centre_cell(rect);
    let centre = altitude(cx, cz);
    let (mut max, mut min) = (centre, centre);
    // 2. the sea clamp, one f32 add.
    let above_sea = r24(f64::from(sea) + 1.0) as f32;
    let mut sum = 0.0f64;
    let mut count = 0.0f64;
    for x in rect.x1..=rect.x2 {
        for z in rect.z1..=rect.z2 {
            let a = altitude(x, z);
            let a = if a <= sea { above_sea } else { a };
            let a = f64::from(a);
            if a < f64::from(min) {
                min = a as f32;
            } else if a > f64::from(max) {
                max = a as f32;
            }
            sum = r24(sum + a);
            count = r24(count + 1.0);
        }
    }
    // 3.
    let mean = if count > 0.0 { r24(sum / count) as f32 } else { 0.0 };
    Variance {
        variance: (max - min).abs(),
        mean,
        max,
        min,
    }
}

/// `GetZoningCost(region)` → the int64 whose **high dword is always 0**
/// (§4.3 step 4 *[compat]*, slot 9's EDX:EAX). `target` is the `levelAltitude`
/// the caller picked (§4.3 steps 1–2: a zone that intersects, or the variance
/// mean); `None` means "return 0" (step 2's tolerance test passed).
///
/// 3. `s := 0.0`; for each **mask** cell of the region (`x` outer, `z` inner):
///    `s := s + |target − altitude(x, z)|` — one `f32` add of one `f32`
///    absolute difference, on the **raw** altitude with no sea adjustment.
/// 4. `round(kCostPerCellHeightChange × s)`: one `f32` product, then the magic
///    round, zero-extended to `int64`.
pub fn zoning_cost(
    region: &CellRegion,
    target: Option<f32>,
    cost_per_cell: f32,
    altitude: impl Fn(i32, i32) -> f32,
) -> i64 {
    let Some(target) = target else { return 0 };
    let t = f64::from(target);
    let mut s = 0.0f64;
    for (x, z) in region.mask_cells() {
        let d = (t - f64::from(altitude(x, z))).abs();
        s = r24(s + r24(d));
    }
    // The int64's high dword is always 0 (§4.3 step 4, §7.1 slot 9 [compat]),
    // so the 32-bit round result is zero-extended.
    i64::from(magic_round(r24(f64::from(cost_per_cell) * s)) as u32)
}

/// `CalculateGarbageHeapCapacity(v)` (§4.6.1): `v ≤ 0` → 0; otherwise
/// `round(float(u32(tileCap × cellCount)) / v)`. The product is a 32-bit `imul`
/// (wraps), converted as unsigned, then **one** `float32` division by `v`,
/// then the magic round.
///
/// Total zone capacity is therefore about `tileCap` per mask cell whenever at
/// least one heap is valid, and **0** for a zone with no valid heap (a zone
/// one cell wide, §4.6.1, conformance test 1).
pub fn heap_capacity(tile_capacity: u32, cell_count: i32, valid_heaps: i32) -> u32 {
    if valid_heaps <= 0 {
        return 0;
    }
    let product = tile_capacity.wrapping_mul(cell_count as u32);
    let quotient = r24(float_of_u32(product) / f64::from(valid_heaps));
    magic_round(quotient) as u32
}

/// `DecayGarbage()` (§4.7.3): how much of `contained` decays this month.
///
/// `d := round(float(contained) × kLandfillDecayRate)` (one `f32` product,
/// magic round). If `d = 0`: `d := 1`; else `d := min(d,
/// kMaxGarbageToDecayPerMonthPerZone)`. Then `d := min(d, contained)`. The
/// decayed garbage disappears (no counter). *[quirk]* The "per zone" cap
/// applies to **each heap** (§4.7.3, quirk 8). `contained = 0` decays nothing.
pub fn decay_amount(contained: u32, decay_rate: f32, per_zone_cap: u32) -> u32 {
    if contained == 0 {
        return 0;
    }
    let raw = magic_round(r24(float_of_u32(contained) * f64::from(decay_rate)));
    let d = if raw == 0 { 1u32 } else { (raw as u32).min(per_zone_cap) };
    d.min(contained)
}

/// One heap's new `contained` after `RedistributeGarbage` (§4.7.4): the
/// `x := min(share, rem)` of the loop, for a **valid** heap in vector order.
/// `rem` is what is left for the later heaps. `share` is `cap / v` (a signed
/// 32-bit division) and `min` compares both as signed, as the spec says
/// "signed integer division"; the result is stored as the heap's `u32`
/// `contained`. Invalid heaps are not changed by the caller.
pub fn redistribute_share(capacity_sum: u32, valid_heaps: i32) -> i32 {
    if valid_heaps <= 0 {
        return 0;
    }
    (capacity_sum as i32) / valid_heaps
}

/// One step of the `RedistributeGarbage` loop (§4.7.4): `(contained, rem)`
/// after giving a valid heap `min(share, rem)`.
pub fn redistribute(share: i32, rem: u32) -> (u32, u32) {
    let x = share.min(rem as i32);
    (x as u32, (rem as i32 - x) as u32)
}

/// The tile rule table of §4.8: `mask → (tile, orientation)`. Index 15's
/// orientation is `2 × (rand() & 3)` and is filled in by the caller (the
/// original writes the drawn value into the static table entry, harmless
/// because it is rewritten before every use *[quirk]*).
///
/// | Mask | Tile | Or. | Mask | Tile | Or. |
/// |---|---|---|---|---|---|
/// | 0 | empty | 0 | 8 | corner | 0 |
/// | 1 | corner | 3 | 9 | edge | 0 |
/// | 2 | corner | 2 | 10 | diagonal | 2 |
/// | 3 | edge | 3 | 11 | three | 0 |
/// | 4 | corner | 1 | 12 | edge | 1 |
/// | 5 | diagonal | 0 | 13 | three | 1 |
/// | 6 | edge | 2 | 14 | three | 2 |
/// | 7 | three | 3 | 15 | interior | `2 × (rand() & 3)` |
pub const TILE_RULES: [(u32, i32); 16] = [
    (tile::EMPTY, 0),
    (tile::CORNER, 3),
    (tile::CORNER, 2),
    (tile::EDGE, 3),
    (tile::CORNER, 1),
    (tile::DIAGONAL, 0),
    (tile::EDGE, 2),
    (tile::THREE, 3),
    (tile::CORNER, 0),
    (tile::EDGE, 0),
    (tile::DIAGONAL, 2),
    (tile::THREE, 0),
    (tile::EDGE, 1),
    (tile::THREE, 1),
    (tile::THREE, 2),
    (tile::INTERIOR, 0),
];

/// `tile_of(mask, rand_value)` (§4.8): the tile and orientation of a redrawn
/// cell. `rand_value` is the drawn C `rand()` (0…32767) and is used only for
/// mask 15, whose orientation is `2 × (rand_value & 3)` ∈ {0, 2, 4, 6}.
/// A mask outside 0…15 is taken modulo 16 *[choice: guard]*.
pub fn tile_of(mask: u32, rand_value: u32) -> (u32, i32) {
    let (t, o) = TILE_RULES[(mask & 15) as usize];
    if t == tile::INTERIOR {
        (t, 2 * ((rand_value & 3) as i32))
    } else {
        (t, o)
    }
}

/// `mask := 1·A(cx−1, cz−1) + 2·A(cx−1, cz) + 4·A(cx, cz) + 8·A(cx, cz−1)`
/// (§4.8), where `A(i, j)` is 1 when heap `(i, j)` is active and 0 outside
/// `0 ≤ i < W`, `0 ≤ j < H`. `active(i, j)` is called only for in-range heaps.
pub fn neighbour_mask(cx: i32, cz: i32, w: i32, h: i32, active: impl Fn(i32, i32) -> bool) -> u32 {
    let a = |i: i32, j: i32| u32::from((0..w).contains(&i) && (0..h).contains(&j) && active(i, j));
    a(cx - 1, cz - 1) | (a(cx - 1, cz) << 1) | (a(cx, cz) << 2) | (a(cx, cz - 1) << 3)
}

/// The cells a heap state change redraws, in the order §4.8 gives: `(hx, hz)`,
/// `(hx+1, hz)`, `(hx, hz+1)`, `(hx+1, hz+1)` (local coordinates).
pub fn redrawn_cells(hx: i32, hz: i32) -> [(i32, i32); 4] {
    [(hx, hz), (hx + 1, hz), (hx, hz + 1), (hx + 1, hz + 1)]
}

/// The spiral of §4.7.5 (`AddZot`): the plumbing §7.2 `SpiralIterator` in
/// its clockwise mode, started at `centre` and boxed by `rect` (LF4,
/// `0x006BB3B0`–`0x006BB4DA`).
///
/// The order is `centre`, then ring by ring. Ring `r` starts at `(cx + r,
/// cz − r + 1)` and runs +z up the column `x = cx + r` to `(cx + r, cz + r)`,
/// then −x along the row `z = cz + r`, −z down the column `x = cx − r` and +x
/// along the row `z = cz − r`, ending at the corner `(cx + r, cz − r)`. Ring 1
/// is `(cx+1, cz)`, `(cx+1, cz+1)`, `(cx, cz+1)`, `(cx−1, cz+1)`, `(cx−1, cz)`,
/// `(cx−1, cz−1)`, `(cx, cz−1)`, `(cx+1, cz−1)`. Cells outside `rect` are
/// skipped one by one; the walk ends after ring `max(cx − x0, x1 − cx, cz −
/// z0, z1 − cz)`, so every rect cell is visited exactly once. The original
/// tests the centre twice (before the first `GetNext` and as its result); the
/// repeat has no effect and is not listed. The first mask cell wins; with no
/// mask cell the caller uses the centre cell.
pub fn spiral_cells(rect: CellRect, centre: (i32, i32)) -> Vec<(i32, i32)> {
    let (cx, cz) = centre;
    // The largest ring that still touches the rect.
    let r_max = (cx - rect.x1)
        .max(rect.x2 - cx)
        .max(cz - rect.z1)
        .max(rect.z2 - cz)
        .max(0);
    let mut out = Vec::new();
    push_clipped(&mut out, rect, cx, cz);
    for r in 1..=r_max {
        let (x0, z0, x1, z1) = (cx - r, cz - r, cx + r, cz + r);
        // +z up the column x = cx + r, from cz − r + 1 to cz + r.
        for z in z0 + 1..=z1 {
            push_clipped(&mut out, rect, x1, z);
        }
        // −x along the row z = cz + r, from cx + r − 1 to cx − r.
        for x in (x0..x1).rev() {
            push_clipped(&mut out, rect, x, z1);
        }
        // −z down the column x = cx − r, from cz + r − 1 to cz − r.
        for z in (z0..z1).rev() {
            push_clipped(&mut out, rect, x0, z);
        }
        // +x along the row z = cz − r, from cx − r + 1 to cx + r.
        for x in x0 + 1..=x1 {
            push_clipped(&mut out, rect, x, z0);
        }
    }
    out
}

fn push_clipped(out: &mut Vec<(i32, i32)>, rect: CellRect, x: i32, z: i32) {
    if rect.x1 <= x && x <= rect.x2 && rect.z1 <= z && z <= rect.z2 {
        out.push((x, z));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat field of altitudes with per-cell overrides.
    struct Terrain {
        base: f32,
        over: Vec<((i32, i32), f32)>,
    }

    impl Terrain {
        fn flat(base: f32) -> Terrain {
            Terrain { base, over: Vec::new() }
        }
        fn with(mut self, cells: &[((i32, i32), f32)]) -> Terrain {
            self.over.extend_from_slice(cells);
            self
        }
        fn at(&self, x: i32, z: i32) -> f32 {
            self.over
                .iter()
                .find(|((ox, oz), _)| *ox == x && *oz == z)
                .map_or(self.base, |(_, a)| *a)
        }
    }

    #[test]
    fn magic_round_is_half_to_even_below_2_pow_22() {
        // §4: round-half-to-even for 0 ≤ v < 2²².
        assert_eq!(magic_round(0.35), 0);
        assert_eq!(magic_round(0.5), 0);
        assert_eq!(magic_round(1.5), 2);
        assert_eq!(magic_round(2.5), 2);
        assert_eq!(magic_round(700.0000029802322), 700);
        // Above 2²² it is wrong (quirk 14): a value in [2²², 2²³) comes out as
        // 2²² + (v − 2²²)/2.
        assert_eq!(magic_round(5_000_000.0), 4_597_152);
    }

    #[test]
    fn heap_capacity_matches_test_1() {
        // Conformance test 1: tileCap 30000.
        // 4×4 zone → 16 cells, W = H = 3, 9 valid heaps:
        // c = round(480000 / 9) = 53333.
        assert_eq!(heap_capacity(30000, 16, 9), 53333);
        // A 2×2 zone: 1 heap, capacity 120000.
        assert_eq!(heap_capacity(30000, 4, 1), 120000);
        // v ≤ 0 → 0 (a 1×5 zone has W = 0, no heaps).
        assert_eq!(heap_capacity(30000, 5, 0), 0);
        assert_eq!(heap_capacity(30000, 5, -1), 0);
        // The 32-bit imul wraps: 30000 × 200000 = 6_000_000_000 → 1_705_032_704.
        assert_eq!(heap_capacity(30000, 200_000, 1), magic_round(1_705_032_704.0) as u32);
    }

    #[test]
    fn decay_matches_test_7() {
        // Conformance test 7, shipped tuning: rate 0.07, per-zone cap 1500.
        assert_eq!(decay_amount(10000, 0.07, 1500), 700);
        // round(0.35) = 0 → 1.
        assert_eq!(decay_amount(5, 0.07, 1500), 1);
        // round(2800) capped to 1500.
        assert_eq!(decay_amount(40000, 0.07, 1500), 1500);
        // Nothing to decay.
        assert_eq!(decay_amount(0, 0.07, 1500), 0);
        // d := min(d, contained): 1 of 1.
        assert_eq!(decay_amount(1, 0.07, 1500), 1);
        // The code-default rate 0.1 and cap 3000.
        assert_eq!(decay_amount(10000, 0.1, 3000), 1000);
    }

    #[test]
    fn redistribute_matches_test_13() {
        // Conformance test 13: 3 valid heaps of capacity 100, contained
        // 100, 50, 0 (total 150) → share 100 → 100, 50, 0.
        let share = redistribute_share(300, 3);
        assert_eq!(share, 100);
        let mut rem = 150u32;
        let mut got = Vec::new();
        for _ in 0..3 {
            let (c, r) = redistribute(share, rem);
            got.push(c);
            rem = r;
        }
        assert_eq!(got, [100, 50, 0]);
        assert_eq!(rem, 0);
        // No valid heap: nothing to share.
        assert_eq!(redistribute_share(300, 0), 0);
    }

    #[test]
    fn tile_rules_match_the_table() {
        // §4.8's table, row by row.
        let want = [
            (tile::EMPTY, 0),
            (tile::CORNER, 3),
            (tile::CORNER, 2),
            (tile::EDGE, 3),
            (tile::CORNER, 1),
            (tile::DIAGONAL, 0),
            (tile::EDGE, 2),
            (tile::THREE, 3),
            (tile::CORNER, 0),
            (tile::EDGE, 0),
            (tile::DIAGONAL, 2),
            (tile::THREE, 0),
            (tile::EDGE, 1),
            (tile::THREE, 1),
            (tile::THREE, 2),
        ];
        for (m, w) in want.into_iter().enumerate() {
            assert_eq!(tile_of(m as u32, 0), w, "mask {m}");
        }
        // Mask 15 draws: orientation 2 × (rand & 3) ∈ {0, 2, 4, 6}.
        for (r, o) in [(41u32, 2i32), (0, 0), (1, 2), (2, 4), (3, 6), (4, 0), (7, 6)] {
            assert_eq!(tile_of(15, r), (tile::INTERIOR, o), "rand {r}");
        }
    }

    #[test]
    fn neighbour_mask_matches_tests_6_and_12() {
        // Conformance test 6: a 2×2 zone (W = H = 1) whose one heap became
        // active. Cells (0,0), (1,0), (0,1), (1,1) → masks 4, 2, 8, 1.
        let all = |_: i32, _: i32| true;
        let got: Vec<u32> = [(0, 0), (1, 0), (0, 1), (1, 1)]
            .iter()
            .map(|&(x, z)| neighbour_mask(x, z, 1, 1, all))
            .collect();
        assert_eq!(got, [4, 2, 8, 1]);
        // Conformance test 12: a 3×3 zone (W = H = 2) with all four heaps
        // active; only the centre cell (1,1) has mask 15.
        let got: Vec<u32> = [(1, 1), (2, 1), (1, 2), (2, 2)]
            .iter()
            .map(|&(x, z)| neighbour_mask(x, z, 2, 2, all))
            .collect();
        assert_eq!(got, [15, 3, 9, 1]);
        // A heap that is not active contributes nothing.
        assert_eq!(neighbour_mask(0, 0, 2, 2, |_, _| false), 0);
        assert_eq!(neighbour_mask(1, 1, 2, 2, |i, j| i == 0 && j == 0), 1);
    }

    #[test]
    fn redraw_order() {
        assert_eq!(redrawn_cells(0, 0), [(0, 0), (1, 0), (0, 1), (1, 1)]);
        assert_eq!(redrawn_cells(1, 1), [(1, 1), (2, 1), (1, 2), (2, 2)]);
    }

    #[test]
    fn variance_matches_tests_3_and_5() {
        // Conformance test 3: 2×2 region, altitudes 270, 272, 280, 290 in
        // (x outer, z inner) order → min 270, max 290, variance 20, mean 278.
        let t = Terrain::flat(270.0).with(&[((0, 0), 270.0), ((0, 1), 272.0), ((1, 0), 280.0), ((1, 1), 290.0)]);
        let v = altitude_variance(CellRect::new(0, 0, 1, 1), 250.0, |x, z| t.at(x, z));
        assert_eq!((v.min, v.max, v.variance, v.mean), (270.0, 290.0, 20.0, 278.0));

        // Conformance test 5: the centre cell (x0, z0) is 251, (x0+1, z0) is
        // 240 (below sea 250) so it counts as 251, (x0, z0+1) 260,
        // (x0+1, z0+1) 262 → min 251, max 262, variance 11, mean 256.
        let t = Terrain::flat(251.0).with(&[((0, 0), 251.0), ((1, 0), 240.0), ((0, 1), 260.0), ((1, 1), 262.0)]);
        let v = altitude_variance(CellRect::new(0, 0, 1, 1), 250.0, |x, z| t.at(x, z));
        assert_eq!((v.min, v.max, v.variance, v.mean), (251.0, 262.0, 11.0, 256.0));
        // The cost uses the raw altitudes: Σ|256 − a| = 5 + 16 + 4 + 6 = 31.
        let region = CellRegion::all_set(CellRect::new(0, 0, 1, 1));
        assert_eq!(zoning_cost(&region, Some(256.0), 4.0, |x, z| t.at(x, z)), 124);

        // Conformance test 2: flat ground → variance 0 ≤ 5, so no target and
        // cost 0 (EDX = 0).
        let t = Terrain::flat(270.0);
        let v = altitude_variance(CellRect::new(0, 0, 3, 3), 250.0, |x, z| t.at(x, z));
        assert_eq!((v.variance, v.mean), (0.0, 270.0));
        assert_eq!(zoning_cost(&region, None, 4.0, |x, z| t.at(x, z)), 0);

        // An empty rect: count 0, so mean 0 and variance 0.
        let v = altitude_variance(CellRect::new(5, 5, 4, 4), 250.0, |_, _| 300.0);
        assert_eq!((v.mean, v.variance), (0.0, 0.0));
    }

    #[test]
    fn zoning_cost_matches_tests_3_and_4() {
        // Conformance test 3: mean 278 over cells 270, 272, 280, 290 →
        // Σ|278 − a| = 8 + 6 + 2 + 12 = 28, cost round(4 × 28) = 112.
        let t = Terrain::flat(270.0).with(&[((0, 0), 270.0), ((0, 1), 272.0), ((1, 0), 280.0), ((1, 1), 290.0)]);
        let region = CellRegion::all_set(CellRect::new(0, 0, 1, 1));
        assert_eq!(zoning_cost(&region, Some(278.0), 4.0, |x, z| t.at(x, z)), 112);
        // Conformance test 4: a zone with levelAltitude 275 over cells 270,
        // 280 → Σ = 5 + 5 = 10, cost round(4 × 10) = 40 (no tolerance test).
        let t = Terrain::flat(270.0).with(&[((0, 0), 270.0), ((1, 0), 280.0)]);
        let region = CellRegion::all_set(CellRect::new(0, 0, 1, 0));
        assert_eq!(zoning_cost(&region, Some(275.0), 4.0, |x, z| t.at(x, z)), 40);
        // Only mask cells count: clearing one cell halves the sum.
        let mut region = CellRegion::all_set(CellRect::new(0, 0, 1, 0));
        region.set(1, 0, false);
        assert_eq!(zoning_cost(&region, Some(275.0), 4.0, |x, z| t.at(x, z)), 20);
        // The code-default cost per cell is 1.0.
        assert_eq!(zoning_cost(&region, Some(275.0), 1.0, |x, z| t.at(x, z)), 5);
    }

    #[test]
    fn spiral_starts_at_the_centre_and_covers_the_rect() {
        // §4.7.5: centre first, then outward, clipped to the rect.
        let s = spiral_cells(CellRect::new(0, 0, 2, 2), (1, 1));
        assert_eq!(s[0], (1, 1));
        // A 3×3 rect has 9 cells, each exactly once.
        assert_eq!(s.len(), 9);
        let mut sorted = s.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 9);
        // Ring 1 starts at (cx+1, cz) and runs +z, −x, −z, +x (LF4).
        assert_eq!(
            &s[1..9],
            &[(2, 1), (2, 2), (1, 2), (0, 2), (0, 1), (0, 0), (1, 0), (2, 0)]
        );
        // Clipped: a centre on the rect's corner still visits every cell.
        let s = spiral_cells(CellRect::new(0, 0, 1, 1), (0, 0));
        assert_eq!(s, [(0, 0), (1, 0), (1, 1), (0, 1)]);
        // A 1×1 rect: the centre alone.
        assert_eq!(spiral_cells(CellRect::new(3, 4, 3, 4), (3, 4)), [(3, 4)]);
    }

    #[test]
    fn spiral_ring_two_and_the_worked_examples() {
        // Ring 2 of an unclipped 5×5: starts at (cx+2, cz−1), ends at
        // (cx+2, cz−2), 16 cells.
        let s = spiral_cells(CellRect::new(0, 0, 4, 4), (2, 2));
        assert_eq!(s.len(), 25);
        assert_eq!(s[9], (4, 1));
        assert_eq!(s[24], (4, 0));
        assert_eq!(
            &s[9..25],
            &[
                (4, 1),
                (4, 2),
                (4, 3),
                (4, 4),
                (3, 4),
                (2, 4),
                (1, 4),
                (0, 4),
                (0, 3),
                (0, 2),
                (0, 1),
                (0, 0),
                (1, 0),
                (2, 0),
                (3, 0),
                (4, 0)
            ]
        );
        // §4.7.5 example 1: the 3×3 zone over (10..12, 20..22), centre
        // (11, 21): the first ring-1 cell is (12, 21).
        let s = spiral_cells(CellRect::new(10, 20, 12, 22), (11, 21));
        assert_eq!(s[..2], [(11, 21), (12, 21)]);
        // §4.7.5 example 2: x 10..14, z 20..22, centre (12, 21): ring 2
        // clipped to the rect is (14, 20), (14, 21), (14, 22), (10, 22),
        // (10, 21), (10, 20).
        let s = spiral_cells(CellRect::new(10, 20, 14, 22), (12, 21));
        assert_eq!(s.len(), 15);
        assert_eq!(&s[9..], &[(14, 20), (14, 21), (14, 22), (10, 22), (10, 21), (10, 20)]);
        // With only (10, 20) and (14, 22) set, (14, 22) comes first.
        let hit = s.iter().find(|c| **c == (10, 20) || **c == (14, 22));
        assert_eq!(hit, Some(&(14, 22)));
    }
}
