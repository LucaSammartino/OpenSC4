// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The shared map utilities of §7.1–§7.3: the linear cone
//! (`AddRadiusEffect`), the Manhattan falloff (`AddDistanceEffect`) and the
//! two-grid visit (`ProcessTwoGridsWithCallback`).

use super::elem::{GridElem, RadiusElem};
use super::simgrid::{GridSnapshot, TractGrid, MAX_TRACTS};
use crate::props::{round_half_even_f32, round_half_even_f64};

/// The largest |dx|, |dz| of the distance table (§7.1).
pub const TABLE_RADIUS: i64 = 15;

/// `TABLE(dx, dz)` (§7.1): `float32(sqrt(dx² + dz²))` rounded half-up to
/// three decimals, stored as `float32`. Defined for any offsets; the cone
/// only uses it for |dx|, |dz| ≤ 15.
pub fn table_distance(dx: i64, dz: i64) -> f32 {
    let s = exact_distance(dx, dz);
    ((f64::from(s) * 1000.0 + 0.5).floor() / 1000.0) as f32
}

/// `float32(sqrt(dx² + dz²))` (used when R ≥ 16): one correctly rounded
/// `fsqrt` of the exact integer (foundations §18).
pub fn exact_distance(dx: i64, dz: i64) -> f32 {
    ((dx as f64) * (dx as f64) + (dz as f64) * (dz as f64)).sqrt() as f32
}

/// Clamped inclusive range `[max(0, c − r), min(count − 1, c + r)]`.
fn span(c: i32, r: i64, count: i32) -> (i64, i64) {
    let count = i64::from(count.clamp(0, MAX_TRACTS));
    (
        i64::from(c).saturating_sub(r).max(0),
        i64::from(c).saturating_add(r).min(count - 1),
    )
}

/// `AddRadiusEffect` (§7.1) for the `u8`, `i8` and `i16` variants: a linear
/// cone of centre value `c` and outer value `o` over radius `r` tracts
/// around tract `(cx, cz)`, added to the grid with clamping. `count_x`,
/// `count_z` are the caller's tract counts (the original receives them as
/// arguments); cells outside the grid's own storage are skipped.
#[allow(clippy::too_many_arguments)]
pub fn add_radius_effect<T: RadiusElem>(
    grid: &dyn TractGrid<T>,
    count_x: i32,
    count_z: i32,
    cx: i32,
    cz: i32,
    r: f32,
    c: f32,
    o: f32,
) {
    if r.is_nan() || r <= 0.0 {
        return;
    }
    let big_r = round_half_even_f64(f64::from(r)) as i64;
    // Each operation rounds to float32 (foundations §18).
    let k = (c - o) / r;
    let (x0, x1) = span(cx, big_r, count_x);
    let (z0, z1) = span(cz, big_r, count_z);
    for x in x0..=x1 {
        for z in z0..=z1 {
            let (dx, dz) = (x - i64::from(cx), z - i64::from(cz));
            let d = if big_r <= TABLE_RADIUS {
                table_distance(dx, dz)
            } else {
                exact_distance(dx, dz)
            };
            if d < r {
                let v = c - d * k;
                let delta = round_half_even_f32(v) as i64;
                let (x, z) = (x as i32, z as i32);
                let old = grid.get(x, z);
                grid.set(x, z, T::clamped(old.to_i64().saturating_add(delta)));
            }
        }
    }
}

/// The "far" marker of the distance field (§7.2).
pub const FAR: u8 = 255;

/// The Manhattan distance field of §7.2: 0 at tracts whose mask value is
/// `m`, the exact 4-neighbour distance up to `n`, otherwise [`FAR`]. Rounds
/// stop at 255 ([choice]: the original's byte field collides there) and
/// when a round changes nothing. Index `x × count_z + z`.
pub fn distance_field(mask: &dyn TractGrid<u8>, count_x: i32, count_z: i32, m: u8, n: i64) -> Vec<u8> {
    let (nx, nz) = (
        count_x.clamp(0, MAX_TRACTS) as usize,
        count_z.clamp(0, MAX_TRACTS) as usize,
    );
    let mut dist = vec![FAR; nx * nz];
    for x in 0..nx {
        for z in 0..nz {
            if mask.get(x as i32, z as i32) == m {
                dist[x * nz + z] = 0;
            }
        }
    }
    for round in 1..=n.min(255) {
        let prev = (round - 1) as u8;
        let mut changed = false;
        for x in 0..nx {
            for z in 0..nz {
                if dist[x * nz + z] != FAR {
                    continue;
                }
                let near = (x > 0 && dist[(x - 1) * nz + z] == prev)
                    || (x + 1 < nx && dist[(x + 1) * nz + z] == prev)
                    || (z > 0 && dist[x * nz + z - 1] == prev)
                    || (z + 1 < nz && dist[x * nz + z + 1] == prev);
                if near {
                    dist[x * nz + z] = round as u8;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    dist
}

/// `AddDistanceEffect` (§7.2). `target` and `mask` may be the same grid:
/// the distance field is built before the target changes.
#[allow(clippy::too_many_arguments)]
pub fn add_distance_effect(
    target: &dyn TractGrid<u8>,
    count_x: i32,
    count_z: i32,
    mask: &dyn TractGrid<u8>,
    m: u8,
    radius: f32,
    near: f32,
    far: f32,
) {
    let n = radius as i64;
    let dist = distance_field(mask, count_x, count_z, m, n);
    let k = (near - far) / radius;
    let nz = count_z.clamp(0, MAX_TRACTS) as usize;
    for (i, &d) in dist.iter().enumerate() {
        let inside = f32::from(d) < radius;
        if d == 0 || !inside {
            continue;
        }
        let (x, z) = ((i / nz.max(1)) as i32, (i % nz.max(1)) as i32);
        let t = target.get(x, z);
        let v = (f32::from(t) + near) - f32::from(d) * k;
        target.set(x, z, (v as i64).clamp(0, 255) as u8);
    }
}

/// `ProcessTwoGridsWithCallback<A, B>` (§7.3): visits every tract of `a`
/// once with the matching value of `b`. `f(x, z, &mut value, b)` gets A's
/// tract coordinates (so callers can implement the residential-only
/// variants by skipping tracts) and may change the A value, which is
/// written back when it changed.
///
/// - Same shift: `B[x][z]`.
/// - A coarser by d: floor of the mean of the 2^d × 2^d covered B values,
///   `(sum >> d) >> d` with a wrapping 32-bit sum (float B values are
///   truncated first, [choice]).
/// - A finer by d: each B tract's value for every A tract it covers.
///
/// Out-of-range reads give 0 and writes are ignored ([choice]; the original
/// requires both grids to cover the same city). `a` and `b` are two
/// different grids (each may be read in one call before the walk).
pub fn process_two_grids<A: GridElem, B: GridElem>(
    a: &dyn TractGrid<A>,
    b: &dyn TractGrid<B>,
    mut f: impl FnMut(i32, i32, &mut A, B),
) {
    let (ga, gb) = (a.geometry(), b.geometry());
    let (sa, sb) = (ga.shift.clamp(0, 31), gb.shift.clamp(0, 31));
    // Both grids read in one call each when they offer it (game grids: one
    // host call instead of one per tract); the same values either way.
    let mut a = GridSnapshot::new(a);
    let b = GridSnapshot::new(b);
    let mut visit = |x: i32, z: i32, bv: B| {
        let old = a.get(x, z);
        let mut v = old;
        f(x, z, &mut v, bv);
        if v != old {
            a.set(x, z, v);
        }
    };
    let (ax, az) = (ga.count_x.clamp(0, MAX_TRACTS), ga.count_z.clamp(0, MAX_TRACTS));
    let (bx, bz) = (gb.count_x.clamp(0, MAX_TRACTS), gb.count_z.clamp(0, MAX_TRACTS));
    if sa == sb {
        for x in 0..ax {
            for z in 0..az {
                visit(x, z, b.get(x, z));
            }
        }
    } else if sa > sb {
        let d = (sa - sb) as u32;
        let side = 1i64 << d;
        for x in 0..ax {
            for z in 0..az {
                let mut sum = 0i32;
                let (x0, z0) = (i64::from(x) << d, i64::from(z) << d);
                for i in x0..(x0 + side).min(i64::from(bx)) {
                    for j in z0..(z0 + side).min(i64::from(bz)) {
                        sum = sum.wrapping_add(b.get(i as i32, j as i32).mean_term());
                    }
                }
                visit(x, z, B::from_mean((sum >> d) >> d));
            }
        }
    } else {
        let d = (sb - sa) as u32;
        let side = 1i64 << d;
        for x in 0..bx {
            for z in 0..bz {
                let bv = b.get(x, z);
                let (x0, z0) = (i64::from(x) << d, i64::from(z) << d);
                for i in x0..(x0 + side).min(i64::from(ax)) {
                    for j in z0..(z0 + side).min(i64::from(az)) {
                        visit(i as i32, j as i32, bv);
                    }
                }
            }
        }
    }
}
