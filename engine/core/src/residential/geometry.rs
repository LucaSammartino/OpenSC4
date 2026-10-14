// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The proximity maps of §5.3: the seed from the BDS building map and the
//! four-sweep distance passes. The passes give a Manhattan distance field,
//! but the seed fills only the z = 0 edge *[compat]* (RG23), so the shipped
//! maps are 1 on that edge (0 under a key) and 0 everywhere else.

use crate::grid::{process_two_grids, TractGrid, MAX_TRACTS};

/// The "no building here" seed value and the distance cap (§5.3).
pub const FAR: u8 = 255;

/// [choice] A distance pass can only lower values, so the transform
/// converges in at most [`FAR`] passes; the cap bounds a hostile geometry
/// (a grid with a huge tract count cannot make this loop run forever).
pub const MAX_PASSES: u32 = 256;

/// §5.3 step 1 *[compat]* (RG23): the seed. It visits every `target` tract
/// (x, z), x outer and z inner, and computes 0 where `map` holds `key` and
/// [`FAR`] elsewhere, with the spec's shift comparison:
///
/// - equal shifts (the shipped case): `map[x][0]`, with **no z offset**;
/// - `target` finer by d: `map[x >> d][z >> d]`;
/// - `target` coarser by d: 0 when **any** map tract it covers equals `key`.
///
/// Every value is stored in **(x, 0)**, never in (x, z): the original writes
/// the row's first element. So only the z = 0 edge is seeded, the last z
/// (countZ − 1) decides each edge cell, and every other cell keeps what the
/// grid already held (0 on a fresh grid, and 0 off the edge after a previous
/// wealth's passes, since a pass can only lower a value).
///
/// Out-of-range map reads give 0 ([`TractGrid`]'s rule), which is not `key`
/// for the residential keys 6–8, so they seed [`FAR`].
pub fn seed_proximity(target: &dyn TractGrid<u8>, map: &dyn TractGrid<u8>, key: u8) {
    let (st, sm) = (target.shift(), map.shift());
    let (cx, cz) = (
        target.count_x().clamp(0, MAX_TRACTS),
        target.count_z().clamp(0, MAX_TRACTS),
    );
    let hit = |tx: i32, tz: i32| -> u8 {
        let found = if st == sm {
            // [compat] (RG23): the equal-shift source has no z offset either.
            map.get(tx, 0) == key
        } else if st < sm {
            let d = (sm - st) as u32;
            map.get(tx >> d, tz >> d) == key
        } else {
            let d = (st - sm) as u32;
            let side = 1i64 << d.min(31);
            let (x0, z0) = (i64::from(tx) << d, i64::from(tz) << d);
            let (mx, mz) = (
                i64::from(map.count_x().clamp(0, MAX_TRACTS)),
                i64::from(map.count_z().clamp(0, MAX_TRACTS)),
            );
            (x0..(x0 + side).min(mx)).any(|x| (z0..(z0 + side).min(mz)).any(|z| map.get(x as i32, z as i32) == key))
        };
        if found {
            0
        } else {
            FAR
        }
    };
    for x in 0..cx {
        for z in 0..cz {
            // [compat] (RG23): the row's first element, not (x, z).
            target.set(x, 0, hit(x, z));
        }
    }
}

/// [choice] The seed with no building map: [`seed_proximity`] against a map
/// that holds no key, so (x, 0) := [`FAR`] for every row of a grid with at
/// least one z, and every other cell keeps its value (§5.3 step 1, RG23).
pub fn seed_proximity_no_map(target: &dyn TractGrid<u8>) {
    let cx = target.count_x().clamp(0, MAX_TRACTS);
    if target.count_z() <= 0 {
        return;
    }
    for x in 0..cx {
        target.set(x, 0, FAR);
    }
}

/// §5.3 step 2: **one** distance pass over the `count_x × count_z` grid. For
/// each row x it sweeps z forward then backward, and for each column z it
/// sweeps x forward then backward, each sweep keeping a running distance
/// `d := min(d + 1, v)` that starts at [`FAR`]. Reports whether any stored
/// value differs from the one it replaced.
///
/// `d` is a 32-bit value (§5.3 *[compat]*, RS5): the original keeps it in a
/// 32-bit register, so `255 + 1` does not wrap, and `v ≤ 255` bounds the
/// stored result; the clamp to `0..=255` never acts.
pub fn distance_pass(grid: &dyn TractGrid<u8>, count_x: i32, count_z: i32) -> bool {
    let nx = count_x.clamp(0, MAX_TRACTS);
    let nz = count_z.clamp(0, MAX_TRACTS);
    let mut changed = false;
    // One sweep of the `n` tracts along one axis of the line at `outer`:
    // `along_x` picks the axis, `forward` the direction.
    let sweep = |along_x: bool, outer: i32, forward: bool, changed: &mut bool| {
        let n = if along_x { nx } else { nz };
        let mut d = i32::from(FAR);
        for i in 0..n {
            let inner = if forward { i } else { n - 1 - i };
            let (x, z) = if along_x { (inner, outer) } else { (outer, inner) };
            let v = i32::from(grid.get(x, z));
            d = (d + 1).min(v);
            let b = d.clamp(0, i32::from(FAR)) as u8;
            if grid.get(x, z) != b {
                grid.set(x, z, b);
                *changed = true;
            }
        }
    };
    for x in 0..nx {
        sweep(false, x, true, &mut changed);
        sweep(false, x, false, &mut changed);
    }
    for z in 0..nz {
        sweep(true, z, true, &mut changed);
        sweep(true, z, false, &mut changed);
    }
    changed
}

/// §5.3 step 2 to convergence: passes repeat until one reports no change.
/// The result is the Manhattan distance (in proximity tracts = cells) to the
/// nearest seeded tract, capped at [`FAR`]; with no seeded tract every value
/// stays [`FAR`]. Returns the number of passes run (the last one found no
/// change, unless the cap stopped it).
pub fn distance_to_convergence(grid: &dyn TractGrid<u8>, count_x: i32, count_z: i32) -> u32 {
    let mut passes = 0;
    while passes < MAX_PASSES {
        passes += 1;
        if !distance_pass(grid, count_x, count_z) {
            break;
        }
    }
    passes
}

/// §5.3 step 3: copies `from` into `to` with
/// `ProcessTwoGridsWithCallback<uint8, uint8>`. Both grids have the proximity
/// tract size, so the shifts are equal and this is a plain copy.
pub fn copy_proximity(to: &dyn TractGrid<u8>, from: &dyn TractGrid<u8>) {
    process_two_grids(to, from, |_, _, v, b| *v = b);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::{CityDims, SimGrid};

    /// A `u8` grid of `cx × cz` tracts at tract size 1 (the proximity size).
    fn prox(cx: i32, cz: i32) -> std::rc::Rc<SimGrid<u8>> {
        SimGrid::<u8>::create(
            &CityDims {
                cells_x: cx,
                cells_z: cz,
                ..CityDims::square(256)
            },
            0,
            1,
        )
    }

    /// Row-major `[x][z]` values of a `cx × cz` grid.
    fn rows(t: &SimGrid<u8>, cx: i32, cz: i32) -> Vec<Vec<u8>> {
        (0..cx).map(|x| (0..cz).map(|z| t.get(x, z)).collect()).collect()
    }

    /// §15 test 1 (RG23): one source at (2, 2) of a 5 × 5 map. The seed
    /// reads `map[x][0]` only, so the key is never seen: the edge seeds 255,
    /// the passes lower it to 1, and every other cell stays 0.
    #[test]
    fn one_source_seeds_only_the_edge() {
        let map = prox(5, 5);
        map.set(2, 2, 6);
        let t = prox(5, 5);
        seed_proximity(&*t, &*map, 6);
        for x in 0..5 {
            assert_eq!(t.get(x, 0), FAR, "seed ({x},0)");
            for z in 1..5 {
                assert_eq!(t.get(x, z), 0, "seed ({x},{z})");
            }
        }
        distance_to_convergence(&*t, 5, 5);
        for x in 0..5 {
            for z in 0..5 {
                assert_eq!(t.get(x, z), u8::from(z == 0), "({x},{z})");
            }
        }
        assert_eq!((t.get(0, 0), t.get(2, 2), t.get(4, 2)), (1, 0, 0));
        // Key 6 also at (3, 0): R$ (3, 0) = 0; no key 7 anywhere leaves the
        // R$$ edge at 1.
        map.set(3, 0, 6);
        let r1 = prox(5, 5);
        seed_proximity(&*r1, &*map, 6);
        distance_to_convergence(&*r1, 5, 5);
        assert_eq!(
            rows(&r1, 5, 5).iter().map(|r| r[0]).collect::<Vec<_>>(),
            vec![1, 1, 1, 0, 1]
        );
        let r2 = prox(5, 5);
        seed_proximity(&*r2, &*map, 7);
        distance_to_convergence(&*r2, 5, 5);
        assert_eq!(rows(&r2, 5, 5), rows(&t, 5, 5));
    }

    /// §5.3 worked example (RG23): 4 × 4, key 6 at (1, 0), (1, 1), (2, 3),
    /// key 7 at (3, 0), no key 8; one temporary grid reused across wealths.
    #[test]
    fn worked_example_reuses_the_grid() {
        let map = prox(4, 4);
        for (x, z) in [(1, 0), (1, 1), (2, 3)] {
            map.set(x, z, 6);
        }
        map.set(3, 0, 7);
        let t = prox(4, 4);
        let mut got = Vec::new();
        for key in [6, 7, 8] {
            seed_proximity(&*t, &*map, key);
            distance_to_convergence(&*t, 4, 4);
            got.push(rows(&t, 4, 4));
        }
        let edge = |e: [u8; 4]| -> Vec<Vec<u8>> { e.iter().map(|&v| vec![v, 0, 0, 0]).collect() };
        assert_eq!(got[0], edge([1, 0, 1, 1]), "R$");
        assert_eq!(got[1], edge([1, 1, 1, 0]), "R$$");
        assert_eq!(got[2], edge([1, 1, 1, 1]), "R$$$");
        // Two passes: pass 1 lowers the edge 255s to 1, pass 2 sees no change.
        seed_proximity(&*t, &*map, 6);
        assert_eq!(distance_to_convergence(&*t, 4, 4), 2);
    }

    /// [choice] No building map: the edge seeds FAR, the rest is kept.
    #[test]
    fn no_map_seeds_the_edge_far() {
        let t = prox(3, 2);
        t.set(1, 1, 9);
        seed_proximity_no_map(&*t);
        assert_eq!(rows(&t, 3, 2), vec![vec![FAR, 0], vec![FAR, 9], vec![FAR, 0]]);
    }

    /// §15 test 2: a 300 × 1 map with the key at (0, 0) caps at 255.
    #[test]
    fn distance_caps_at_255() {
        let map = prox(300, 1);
        map.set(0, 0, 7);
        let t = prox(300, 1);
        seed_proximity(&*t, &*map, 7);
        distance_to_convergence(&*t, 300, 1);
        assert_eq!((t.get(200, 0), t.get(280, 0)), (200, FAR));
    }

    /// §15 test 3 (RG23): a coarser target (proximity size 2, map size 1) on
    /// a 4 × 4 map with the key at map cell (3, 1). Tract (1, 0) finds the
    /// key and stores 0 in (1, 0); tract (1, 1) (map x 2..3, z 2..3) finds
    /// none and stores 255 in the same cell.
    #[test]
    fn coarser_target_last_z_decides() {
        let city = CityDims {
            cells_x: 4,
            cells_z: 4,
            ..CityDims::square(256)
        };
        let map = SimGrid::<u8>::create(&city, 0, 1);
        map.set(3, 1, 6);
        let t = SimGrid::<u8>::create(&city, 0, 2);
        assert_eq!((t.count_x(), t.count_z()), (2, 2));
        seed_proximity(&*t, &*map, 6);
        assert_eq!(rows(&t, 2, 2), vec![vec![FAR, 0], vec![FAR, 0]]);
        distance_to_convergence(&*t, t.count_x(), t.count_z());
        assert_eq!(rows(&t, 2, 2), vec![vec![1, 0], vec![1, 0]]);
        // A one-z target has no later z to overwrite the hit.
        let line = CityDims {
            cells_x: 4,
            cells_z: 2,
            ..CityDims::square(256)
        };
        let m1 = SimGrid::<u8>::create(&line, 0, 1);
        m1.set(3, 1, 6);
        let t1 = SimGrid::<u8>::create(&line, 0, 2);
        seed_proximity(&*t1, &*m1, 6);
        assert_eq!((t1.get(0, 0), t1.get(1, 0)), (FAR, 0));
    }

    /// A finer target reads the containing coarser map tract of the last z
    /// and stores it in (x, 0) (RG23).
    #[test]
    fn finer_target_reads_last_z_tract() {
        let city = CityDims::square(8);
        let map = SimGrid::<u8>::create(&city, 0, 4);
        // Map tract (1, 1) covers cells x 4..7, z 4..7: the last z (7) of
        // rows 4..7 reads it.
        map.set(1, 1, 8);
        // Map tract (0, 0) covers x 0..3, z 0..3: no last z reads it.
        map.set(0, 0, 8);
        let t = SimGrid::<u8>::create(&city, 0, 1);
        seed_proximity(&*t, &*map, 8);
        for x in 0..8 {
            assert_eq!(t.get(x, 0), if x >= 4 { 0 } else { FAR }, "({x},0)");
            for z in 1..8 {
                assert_eq!(t.get(x, z), 0, "({x},{z})");
            }
        }
    }

    /// A pass reports "changed" only when it stores a different value.
    #[test]
    fn pass_reports_change() {
        let t = prox(3, 1);
        t.fill(FAR);
        t.set(0, 0, 0);
        assert!(distance_pass(&*t, 3, 1));
        assert_eq!(t.values(), vec![0, 1, 2]);
        assert!(!distance_pass(&*t, 3, 1));
        // The byte accumulator would wrap FAR + 1 to 0; widening keeps FAR.
        let flat = prox(3, 1);
        flat.fill(FAR);
        assert!(!distance_pass(&*flat, 3, 1));
        assert!(flat.values().iter().all(|&v| v == FAR));
    }

    /// The copy is a plain copy at equal shifts.
    #[test]
    fn copy_is_plain() {
        let a = prox(3, 3);
        let b = prox(3, 3);
        a.set(1, 2, 7);
        copy_proximity(&*b, &*a);
        assert_eq!(b.values(), a.values());
    }
}
