// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The proximity maps of §5.3: the seed from the BDS building map and the
//! four-sweep distance passes that turn it into a Manhattan distance field.

use crate::grid::{process_two_grids, TractGrid, MAX_TRACTS};

/// The "no building here" seed value and the distance cap (§5.3).
pub const FAR: u8 = 255;

/// [choice] A distance pass can only lower values, so the transform
/// converges in at most [`FAR`] passes; the cap bounds a hostile geometry
/// (a grid with a huge tract count cannot make this loop run forever).
pub const MAX_PASSES: u32 = 256;

/// §5.3 step 1: fills `target` (the temporary proximity grid) per tract,
/// 0 where `map` holds `key` and [`FAR`] elsewhere. The three cases are the
/// shift comparison the spec gives:
///
/// - equal shifts: `map[x][z]`;
/// - `target` finer by d: `map[x >> d][z >> d]`;
/// - `target` coarser by d: 0 when **any** map tract it covers equals `key`.
///
/// Out-of-range map reads give 0 ([`TractGrid`]'s rule), which is not `key`
/// for the residential keys 6–8, so they seed [`FAR`].
pub fn seed_proximity(target: &dyn TractGrid<u8>, map: &dyn TractGrid<u8>, key: u8) {
    let (st, sm) = (target.shift(), map.shift());
    let (cx, cz) = (target.count_x().clamp(0, MAX_TRACTS), target.count_z().clamp(0, MAX_TRACTS));
    let hit = |tx: i32, tz: i32| -> u8 {
        let found = if st == sm {
            map.get(tx, tz) == key
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
            (x0..(x0 + side).min(mx)).any(|x| {
                (z0..(z0 + side).min(mz)).any(|z| map.get(x as i32, z as i32) == key)
            })
        };
        if found {
            0
        } else {
            FAR
        }
    };
    for x in 0..cx {
        for z in 0..cz {
            target.set(x, z, hit(x, z));
        }
    }
}

/// §5.3 step 2: **one** distance pass over the `count_x × count_z` grid. For
/// each row x it sweeps z forward then backward, and for each column z it
/// sweeps x forward then backward, each sweep keeping a running distance
/// `d := min(d + 1, v)` that starts at [`FAR`]. Reports whether any stored
/// value differs from the one it replaced.
///
/// `d` is widened to `i32` for the increment ([choice]): the original's
/// `min(d + 1, v)` with a byte accumulator would wrap 255 → 0 and flatten
/// the field, and `v ≤ 255` bounds the stored result anyway, so the values
/// written are exactly the byte values the pass computes.
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

    /// §15 test 1: one source at (2, 2) of a 5 × 5 map gives |x − 2| + |z − 2|.
    #[test]
    fn one_source_is_manhattan() {
        let map = prox(5, 5);
        map.set(2, 2, 6);
        let t = prox(5, 5);
        seed_proximity(&*t, &*map, 6);
        assert_eq!(t.get(2, 2), 0);
        assert_eq!(t.get(0, 0), FAR);
        distance_to_convergence(&*t, 5, 5);
        for x in 0..5 {
            for z in 0..5 {
                assert_eq!(t.get(x, z), (x - 2).abs() as u8 + (z - 2).abs() as u8, "({x},{z})");
            }
        }
        assert_eq!((t.get(0, 0), t.get(4, 2)), (4, 2));
        // No key 7 anywhere: every value stays FAR.
        let t2 = prox(5, 5);
        seed_proximity(&*t2, &*map, 7);
        distance_to_convergence(&*t2, 5, 5);
        assert!(t2.values().iter().all(|&v| v == FAR));
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

    /// §15 test 3: a coarser target (proximity size 2, map size 1) with the
    /// key at map cell (3, 1).
    #[test]
    fn coarser_target_covers_any_cell() {
        let city = CityDims {
            cells_x: 8,
            cells_z: 8,
            ..CityDims::square(256)
        };
        let map = SimGrid::<u8>::create(&city, 0, 1);
        map.set(3, 1, 6);
        let t = SimGrid::<u8>::create(&city, 0, 2);
        seed_proximity(&*t, &*map, 6);
        distance_to_convergence(&*t, t.count_x(), t.count_z());
        assert_eq!((t.get(1, 0), t.get(0, 0)), (0, 1));
    }

    /// A finer target reads the containing coarser map tract.
    #[test]
    fn finer_target_reads_containing_tract() {
        let city = CityDims::square(8);
        let map = SimGrid::<u8>::create(&city, 0, 4);
        map.set(1, 0, 8);
        let t = SimGrid::<u8>::create(&city, 0, 1);
        seed_proximity(&*t, &*map, 8);
        // Map tract (1, 0) covers cells x 4..7, z 0..3.
        for x in 4..8 {
            for z in 0..4 {
                assert_eq!(t.get(x, z), 0, "({x},{z})");
            }
        }
        assert_eq!(t.get(0, 0), FAR);
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
