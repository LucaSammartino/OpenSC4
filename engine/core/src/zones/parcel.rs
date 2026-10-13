// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The pure parcellization helpers of the zone developer: the block depth
//! and zone density of §7.2.1 step 8, the depth rule of §7.6.3, and the walk
//! bound `Parcellize` (§7.6) uses. Section references point at
//! `docs/specs/zones.md`.

use crate::x87::{ftol, ftol32, r24};

/// [choice] The number of row / depth positions `Parcellize` (§7.6) walks at
/// most. The keep conditions of §7.6.1 end a walk on any sane region; this
/// bound only stops a hostile lot width or depth of 0 (clamped to 1 here) or
/// a wrap-around range from looping forever. It is far above any real region.
pub const MAX_WALK_STEPS: i32 = 1 << 20;

/// The default block depth (§3.2): the lot-size parameter's initial value,
/// overwritten by [`block_depth`] before every use.
pub const DEFAULT_BLOCK_DEPTH: i32 = 6;

/// `0x00458960`, the zone density (§7.2.1 step 8): zones 1, 4 → 1; 2, 5 → 2;
/// 3, 6, 7, 8, 9 → 3; any other → 0.
pub fn zone_density(zone: i32) -> i32 {
    match zone {
        1 | 4 => 1,
        2 | 5 => 2,
        3 | 6 | 7 | 8 | 9 => 3,
        _ => 0,
    }
}

/// The capacity value of `DetermineLotSize` (§7.2.1 step 6):
/// `v := ftol(r24(r24(float(k) / float(n)) × float(avg)))`, with `k` the
/// cells per tract (`(1 << s) << s`), `n = sx · sz` (an unsigned product) and
/// `avg = (min + max) / 2` (an integer division). The result is kept as an
/// `int32`.
///
/// *[compat]* The chain as written, not the exact floor: for `k ≤ 64` it
/// equals `⌊k · avg / n⌋` for every u16 `avg`, but not above, e.g. `(256, 3,
/// 49154)` gives 4194475 where the floor is 4194474 (test 30g).
pub fn capacity_value(k: i32, n: u32, avg: i32) -> i32 {
    ftol32(r24(r24(f64::from(k) / f64::from(n)) * f64::from(avg)))
}

/// The depth-range adjustment that ends `DetermineLotSize` step 7 (§7.2.1):
/// if `maxDepth − minDepth < 2` (unsigned): `minDepth := (minDepth > 2) ?
/// maxDepth − 2 : 1` and `maxDepth := minDepth + 2`; otherwise unchanged.
/// The `minDepth > 2` compare is taken unsigned, like the widening compares
/// before it (the spec does not say; depths are small positive sizes, where
/// both readings agree).
pub fn adjust_depth_range(min_depth: i32, max_depth: i32) -> (i32, i32) {
    if (max_depth as u32).wrapping_sub(min_depth as u32) < 2 {
        let lo = if (min_depth as u32) > 2 {
            max_depth.wrapping_sub(2)
        } else {
            1
        };
        (lo, lo.wrapping_add(2))
    } else {
        (min_depth, max_depth)
    }
}

/// The block depth (§7.2.1 step 8). `density` is [`zone_density`] of the
/// current zone, `depth` the chosen lot depth (§7.2.1 step 7) and
/// `residential` whether a best developer exists **and** its purpose is 1
/// (residential).
///
/// - residential: density 1 → `3 · depth`; 2 → `ftol(r24(float(depth) ×
///   2.5))` (depth converted as unsigned; 2 → 5, 3 → 7, 4 → 10); 3 → `2 ·
///   depth`.
/// - otherwise: 1 → 7, 2 → 6, 3 → 5.
/// - density 0 leaves the block depth unchanged. This pure helper has no
///   previous value and returns [`DEFAULT_BLOCK_DEPTH`]; `DetermineLotSize`
///   does not call it for density 0 and keeps its previous block depth
///   (unreachable for zones 1–9).
pub fn block_depth(density: i32, depth: i32, residential: bool) -> i32 {
    match density {
        1 => {
            if residential {
                3 * depth
            } else {
                7
            }
        }
        2 => {
            if residential {
                ftol(r24(f64::from(depth as u32) * 2.5)) as i32
            } else {
                6
            }
        }
        3 => {
            if residential {
                2 * depth
            } else {
                5
            }
        }
        _ => DEFAULT_BLOCK_DEPTH,
    }
}

/// The middle row of §7.6.1. `o0` is the row-walk start, `so` the row step,
/// `b1`/`b2` the row-axis range and `lw` the lot width. Let `n` be the number
/// of row positions `o0, o0 + so, …` that satisfy the keep condition (counted
/// until the first that does not; 0 if `o0` itself fails). `mid := o0 +
/// (⌊n/2⌋ − 1)·so` when `⌊n/2⌋ ≥ 1`, else `mid := o0`.
///
/// Example (§7.6.1): `o0 = 10`, `so = +2`: range 10..15 gives n = 3, mid = 10;
/// 10..17 gives n = 4, mid = 12; 10..21 gives n = 6, mid = 14.
pub fn middle_row(o0: i32, so: i32, b1: i32, b2: i32, lw: i32) -> i32 {
    let low = so > 0;
    let keep = |o: i32| {
        if low {
            o >= b1 && o <= b2
        } else {
            o > b1 - lw && o <= o0
        }
    };
    let mut n = 0i32;
    let mut o = o0;
    while keep(o) && n < MAX_WALK_STEPS {
        n += 1;
        o += so;
    }
    if n / 2 >= 1 {
        o0 + (n / 2 - 1) * so
    } else {
        o0
    }
}

/// `DepthRule(rem, f, farRoad)` (§7.6.3). `front` is whether the facing `f`
/// equals the front facing `A` (side D); `far_road` is `(C[B] = 1)`. `min` =
/// `minDepth`, `max` = `maxDepth`, `ld` = `Ld` (the lot depth `P`), `ps` =
/// `placeStreets` as 0/1. The first matching line of the case wins; every
/// compare is unsigned 32-bit and every `/` is an integer division of
/// positive operands.
///
/// The three cases are keyed on (`far_road`, `front`):
/// - **(a)** `far_road` and `front`.
/// - **(c)** not `far_road` and not `front`.
/// - **(b)** otherwise (`far_road` ≠ `front`).
///
/// A result equal to `ld` means "no change".
pub fn depth_rule(rem: i32, front: bool, far_road: bool, min: i32, max: i32, ld: i32, ps: i32) -> i32 {
    let (rem, m, mx, p, s) = (rem as u32, min as u32, max as u32, ld as u32, ps as u32);
    let half = |v: u32| ((v.wrapping_add(1)) / 2) as i32;
    if far_road && front {
        // (a)
        if rem <= mx {
            return rem as i32;
        }
        if rem < 2 * m {
            return rem as i32;
        }
        if rem < 2 * mx - 1 {
            return half(rem);
        }
        if rem < s + 3 * m {
            return half(rem);
        }
        if rem >= s + 3 * p {
            return p as i32;
        }
        return ((rem.wrapping_add(1)) / 3) as i32;
    }
    if !far_road && !front {
        // (c)
        let near = |r: u32| if r > 1 { (r - 1) as i32 } else { p as i32 };
        if rem < s + 2 * m {
            return near(rem);
        }
        if rem >= s + 2 * p {
            return p as i32;
        }
        if rem.wrapping_sub(1) > mx {
            return (rem / 2) as i32;
        }
        return near(rem);
    }
    // (b)
    if rem < s + 2 * m {
        return rem as i32;
    }
    if rem >= s + 2 * p {
        return p as i32;
    }
    if rem <= mx {
        return rem as i32;
    }
    (rem / 2) as i32
}
