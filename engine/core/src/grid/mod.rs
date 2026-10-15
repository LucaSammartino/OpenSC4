// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Simulation grids and the shared map utilities
//! (`docs/specs/simgrid.md`). Section references (`§n`) point at that spec.
//!
//! # Structure
//!
//! - [`GridElem`]: the ten element types (§2) with their per-type rules.
//! - [`TractGeometry`] / [`CityDims`]: tract size, shift, counts, widths and
//!   the coordinate conversions of §3.
//! - [`TractGrid`]: the `cISC4SimGrid<T>` operations simulators use (§4),
//!   with the averages of §4.1 as provided methods. **Grid storage is a
//!   trait**: per the ownership rule (§6.4) the plugin implements it over the
//!   game's own grid objects; simulators never assume Rust-owned memory.
//! - [`SimGrid`]: the core's own grid (Rust storage, §4.2 resize, §5
//!   record), for tests, standalone hosts and working copies
//!   ([`SimGrid::from_values`], as land value's scratch grids);
//!   [`create_for_clsid`] is the §2 factory table. `crate::simgrid` only
//!   keeps the older names `GridGeometry` and `CityCells` for the land value
//!   C API.
//! - [`GridProvider`]: the host callback a simulator uses to create grids
//!   and to resolve grid references read from a save; [`CoreGrids`] makes
//!   core grids.
//! - [`add_radius_effect`], [`add_distance_effect`], [`process_two_grids`]:
//!   §7.1–§7.3.
//! - [`ScanIterator`] (§7.4) and [`CellIterator`] (the random cell iterator,
//!   prime-step and table modes; shared with the weather).
//! - [`Bitmap`]: one-bit-per-cell maps with the rectangle helpers and the
//!   selection-grow flood fill of power.md §2.1 (also plumbing's visited
//!   bitmap).
//!
//! # Choices made where the spec is open (`[choice]`)
//!
//! - A new grid has S = 0 and no tracts (confirmed by §6.4).
//! - Out-of-range reads return 0 and out-of-range writes are ignored (the
//!   original does not check bounds). Tract counts are clamped to
//!   `0 ..= MAX_TRACTS` (4096) when computed from the city or read from a
//!   save; larger or negative stored counts fail the read.
//! - Averages with w or h ≤ 0, or a 32-bit `w × h` that wraps to 0, return 0
//!   (§4.1). Averages sum per tract (value × covered cells), which equals the
//!   per-cell sum of §4.1 because the 64-bit sum wraps the same way.
//! - `SetTractSize` returns true (§4.2). `Read` returns false on a stream
//!   error and then leaves the grid unchanged (§5).
//! - Arithmetic follows foundations §18 (the x87 at 24-bit precision): every
//!   float operation rounds to `f32` once. `AddRadiusEffect` computes
//!   `k = (c − o) / r`, then `d × k` and `c − d × k` in `f32`; with R ≥ 16
//!   the distance is the correctly rounded `float32(sqrt(dx² + dz²))`.
//!   Averages divide the exact 64-bit sum by `w × h` once, rounded to
//!   `f32` (a `double` grid therefore returns a 24-bit quotient, and integer
//!   grids truncate that rounded quotient).
//! - `AddDistanceEffect` runs at most 255 expansion rounds (the byte field's
//!   far marker collides beyond that, §7.2) and stops early when a round
//!   changes nothing, so a huge radius costs no more than 255 rounds;
//!   `(target + near) − dist × k` is three `f32` operations. The cone's
//!   window `c ± R` is computed in 64 bits, so centres and radii near the
//!   int32 limits cannot overflow.
//! - `ProcessTwoGrids` passes A's tract coordinates to the callback (the
//!   residential-only variants are callers that skip tracts) and writes the
//!   A value back only when the callback changed it. Float B values are
//!   truncated to `i32` before the coarse mean.
//! - The scanning iterator's step is at least 1.
//! - The random cell iterator's table mode: see [`CellIterator`]'s module.

mod bitmap;
mod effects;
mod elem;
mod random_iter;
mod scan;
mod simgrid;

pub use bitmap::{Bitmap, CellRect, GROW_MAX_ITERATIONS, MAX_BITMAP_SIDE};
pub use effects::{
    add_distance_effect, add_radius_effect, distance_field, exact_distance, process_two_grids, table_distance, FAR,
    TABLE_RADIUS,
};
pub use elem::{GridElem, RadiusElem};
pub use random_iter::{CellIterator, IteratorState, MODE_PRIME, MODE_TABLE, PRIME_TABLE};
pub use scan::{ScanIterator, ScanState};
pub use simgrid::{
    create_for_clsid, tract_shift, CityDims, CoreGrids, GridProvider, GridSnapshot, SimGrid, SimGridData, TractGeometry,
    TractGrid,
    GRID_RECORD_VERSION, MAX_TRACTS,
};

/// The grid interface ID (all element types, §2).
pub const IID_SIM_GRID: u32 = 0xC9BD_939A;
/// The "bool" factory CLSID: creates a `u8` grid (§2).
pub const CLSID_BOOL_GRID: u32 = 0x49B9_E601;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::{ByteReader, ByteWriter, RefValue, SaveObject};
    use std::rc::Rc;

    fn city() -> CityDims {
        CityDims::square(256)
    }

    #[test]
    fn geometry() {
        let g = TractGeometry::for_city(&city(), 2);
        assert_eq!((g.shift, g.count_x, g.width_x, g.inv_width_x), (1, 128, 32.0, 0.03125));
        let g = TractGeometry::for_city(&city(), 3);
        assert_eq!((g.shift, g.count_x, g.width_x), (1, 128, 48.0));
        for (s, shift) in [(0, 0), (1, 0), (2, 1), (3, 1), (4, 2), (5, 2), (8, 3), (-5, 0)] {
            assert_eq!(tract_shift(s), shift);
        }
        let g = TractGeometry::for_city(&city(), 2);
        assert_eq!(g.position_to_tract(100.0, -10.0), (3, 0));
        // `_ftol` kept to 32 bits (foundations §18.1): NaN and ±inf → 0; a
        // position of 2^32 tracts wraps to 0 and one of −2^31 stays (no saturation).
        assert_eq!(g.position_to_tract(f32::NAN, f32::NEG_INFINITY), (0, 0));
        assert_eq!(
            g.position_to_tract(32.0 * 4_294_967_296.0, -32.0 * 2_147_483_648.0),
            (0, i32::MIN)
        );
        assert_eq!(g.tract_center(3, 4), (112.0, 144.0));
        assert_eq!(g.tract_corner(3, 4), (96.0, 128.0));
        assert!(!g.in_bounds(-1, 0) && g.in_bounds(127, 127) && !g.in_bounds(128, 0));
    }

    #[test]
    fn averages_and_resize() {
        let g = SimGrid::<u8>::create(&CityDims::square(4), 0, 2);
        g.set(0, 0, 10);
        g.set(1, 0, 20);
        g.set(0, 1, 30);
        g.set(1, 1, 41);
        assert_eq!(g.average_in_cell_rect(0, 0, 2, 2), 10);
        assert_eq!(g.average_in_cell_rect(0, 0, 4, 4), 25);
        assert_eq!(g.average_in_tract_rect(0, 0, 2, 2), 25);
        assert_eq!(g.average_in_tract_rect(0, 0, 0, 2), 0);
        assert_eq!(g.get_cell_value(3, 1), 20);
        let f = SimGrid::<f32>::create(&CityDims::square(2), 0, 1);
        f.set(0, 0, 1.9);
        f.set(1, 0, 2.9);
        assert_eq!(f.average_in_tract_rect(0, 0, 2, 1), 1.5);
        let s = SimGrid::<i8>::create(&CityDims::square(2), 0, 1);
        s.set(0, 0, -3);
        assert_eq!(s.average_in_tract_rect(0, 0, 2, 1), -1);
        let big = SimGrid::<u8>::create(&city(), 0, 2);
        big.set_tract_size(&city(), 2);
        big.set(1, 1, 7);
        big.set_tract_size(&city(), 4);
        assert_eq!((big.count_x(), big.get(1, 1)), (64, 7));
        // Hostile rectangles stay bounded.
        assert_eq!(big.average_in_cell_rect(i32::MIN, i32::MIN, i32::MAX, i32::MAX), 0);
        let _ = big.average_in_tract_rect(i32::MAX, 0, i32::MAX, 5);
    }

    #[test]
    fn record_round_trip() {
        let g = SimGrid::<u8>::create(&city(), 0x89D5_BB39, 2);
        let mut w = ByteWriter::new();
        assert!(g.write(&mut w));
        let bytes = w.into_bytes();
        assert_eq!(bytes.len(), 43 + 16384);
        assert_eq!(&bytes[..7], &[1, 0, 1, 0x02, 0xE6, 0xB9, 0x49]);
        let h = SimGrid::<u8>::new();
        assert!(h.read(&mut ByteReader::new(&bytes)));
        assert_eq!(*h.data.borrow(), *g.data.borrow());
        let mut bad = bytes.clone();
        bad[0] = 2;
        assert!(!h.read(&mut ByteReader::new(&bad)));
        assert!(!h.read(&mut ByteReader::new(&bytes[..100])));
        let mut neg = bytes.clone();
        neg[23..27].copy_from_slice(&(-1i32).to_le_bytes());
        assert!(!h.read(&mut ByteReader::new(&neg)));
        assert_eq!(
            create_for_clsid(CLSID_BOOL_GRID).map(|o| o.class_id()),
            Some(0x49B9_E602)
        );
        assert!(create_for_clsid(0x49B9_E600).is_none());
    }

    #[test]
    fn radius_effect() {
        let g = SimGrid::<i16>::create(&CityDims::square(32), 0, 1);
        add_radius_effect(&*g, 32, 32, 10, 10, 3.0, 100.0, 0.0);
        let want = [
            ((10, 10), 100),
            ((11, 10), 67),
            ((11, 11), 53),
            ((12, 10), 33),
            ((12, 11), 25),
            ((12, 12), 6),
        ];
        for ((x, z), v) in want {
            assert_eq!(g.get(x, z), v, "({x},{z})");
        }
        assert_eq!(g.get(13, 10), 0);
        assert_eq!(table_distance(2, 1), 2.236);
        let u = SimGrid::<u8>::create(&CityDims::square(8), 0, 1);
        u.set(0, 0, 250);
        add_radius_effect(&*u, 8, 8, 0, 0, 1.0, 10.0, 10.0);
        assert_eq!(u.get(0, 0), 255);
        for r in [f32::NAN, 0.0, -1.0] {
            add_radius_effect(&*u, 8, 8, 0, 0, r, 10.0, 10.0);
        }
        add_radius_effect(&*u, i32::MAX, i32::MAX, i32::MIN, i32::MAX, 1e30, 1e30, -1e30);
        add_radius_effect(&*u, 8, 8, 3, 3, 40.0, 1.0, 0.0);
    }

    #[test]
    fn distance_effect() {
        let g = SimGrid::<u8>::create(
            &CityDims {
                cells_x: 7,
                cells_z: 1,
                ..city()
            },
            0,
            1,
        );
        for x in 1..7 {
            g.set(x, 0, 10);
        }
        add_distance_effect(&*g, 7, 1, &*g, 0, 3.0, 30.0, 0.0);
        assert_eq!(g.values(), vec![0, 30, 20, 10, 10, 10, 10]);
        for r in [f32::NAN, -3.0, 0.0, 1e9] {
            add_distance_effect(&*g, 7, 1, &*g, 0, r, 30.0, 0.0);
        }
    }

    #[test]
    fn distance_rounds_stop_at_255() {
        let city = CityDims {
            cells_x: 300,
            cells_z: 1,
            ..city()
        };
        let g = SimGrid::<u8>::create(&city, 0, 1);
        g.fill(1);
        g.set(0, 0, 0);
        let d = distance_field(&*g, 300, 1, 0, i64::MAX);
        assert_eq!((d[0], d[1], d[254], d[255], d[256], d[299]), (0, 1, 254, FAR, FAR, FAR));
        add_distance_effect(&*g, 300, 1, &*g, 0, 1000.0, 200.0, 0.0);
        assert_eq!(
            (g.get(1, 0), g.get(100, 0), g.get(255, 0), g.get(299, 0)),
            (200, 181, 150, 150)
        );
    }

    #[test]
    fn working_grids() {
        let geo = TractGeometry::for_city(&CityDims::square(8), 2);
        assert_eq!(
            (geo.len(), geo.index(1, 2), geo.index(4, 0), geo.index(-1, 0)),
            (16, Some(6), None, None)
        );
        let g = SimGrid::<u8>::from_values(geo, &[1, 2, 3]);
        assert_eq!((g.get(0, 0), g.get(0, 2), g.get(0, 3), g.values().len()), (1, 3, 0, 16));
        let copy = g.duplicate();
        g.set(0, 0, 9);
        assert_eq!((copy.get(0, 0), copy.geometry(), g.get(0, 0)), (1, geo, 9));
        g.set_values(&[7; 40]);
        assert_eq!(g.values(), vec![7; 16]);
        assert_eq!(CityDims::default(), CityDims::square(256));
    }

    #[test]
    fn two_grids() {
        let a = SimGrid::<u8>::create(&CityDims::square(4), 0, 2);
        let b = SimGrid::<i16>::create(&CityDims::square(4), 0, 1);
        for (x, z, v) in [
            (0, 0, 1),
            (1, 0, 2),
            (0, 1, 3),
            (1, 1, -7),
            (2, 0, 1),
            (3, 0, 1),
            (2, 1, 1),
            (3, 1, 2),
        ] {
            b.set(x, z, v);
        }
        let mut seen = Vec::new();
        process_two_grids(&*a, &*b, |x, z, _, v| seen.push((x, z, v)));
        assert_eq!(seen, vec![(0, 0, -1), (0, 1, 0), (1, 0, 1), (1, 1, 0)]);
        let mut n = 0;
        process_two_grids(&*b, &*a, |_, _, v, a| {
            *v = i16::from(a) + 1;
            n += 1;
        });
        assert_eq!((n, b.get(3, 3)), (16, 1));
    }

    /// A grid that hands out its values in one call and counts tract reads.
    struct Bulk<T: GridElem>(Rc<SimGrid<T>>, std::cell::Cell<usize>);

    impl<T: GridElem> TractGrid<T> for Bulk<T> {
        fn geometry(&self) -> TractGeometry {
            self.0.geometry()
        }
        fn get(&self, tx: i32, tz: i32) -> T {
            self.1.set(self.1.get() + 1);
            self.0.get(tx, tz)
        }
        fn set(&self, tx: i32, tz: i32, v: T) {
            self.0.set(tx, tz, v)
        }
        fn save_ref(&self) -> RefValue {
            RefValue::Null
        }
        fn snapshot_values(&self) -> Option<Vec<T>> {
            let g = self.0.geometry();
            Some((0..g.count_x).flat_map(|x| (0..g.count_z).map(move |z| (x, z))).map(|(x, z)| self.0.get(x, z)).collect())
        }
    }

    #[test]
    fn snapshot_reads_equal_tract_reads() {
        // Every shift relation; A changes as it goes (writes reach the grid
        // and the copy), and the walk with copies matches the plain one.
        for (sa, sb) in [(2, 2), (4, 1), (1, 4)] {
            let fill = |g: &SimGrid<i16>, k: i32| {
                let geo = g.geometry();
                for x in 0..geo.count_x {
                    for z in 0..geo.count_z {
                        g.set(x, z, ((x * 37 + z * 11 + k) % 61 - 30) as i16);
                    }
                }
            };
            let (a1, b1) = (SimGrid::<i16>::create(&CityDims::square(16), 0, sa), SimGrid::<i16>::create(&CityDims::square(16), 0, sb));
            let (a2, b2) = (a1.duplicate(), b1.duplicate());
            fill(&a1, 3);
            fill(&a2, 3);
            fill(&b1, 7);
            fill(&b2, 7);
            let walk = |a: &dyn TractGrid<i16>, b: &dyn TractGrid<i16>| {
                let mut seen = Vec::new();
                process_two_grids(a, b, |x, z, v, bv| {
                    seen.push((x, z, *v, bv));
                    *v = v.wrapping_add(bv).wrapping_add(1);
                });
                seen
            };
            let (ba, bb) = (Bulk(a2.clone(), Default::default()), Bulk(b2.clone(), Default::default()));
            assert_eq!(walk(&*a1, &*b1), walk(&ba, &bb), "shifts {sa} {sb}");
            assert_eq!((a1.values(), b1.values()), (a2.values(), b2.values()));
            assert_eq!((ba.1.get(), bb.1.get()), (0, 0));
            let snap = GridSnapshot::new(&bb);
            assert!(snap.is_copy());
            assert_eq!((snap.get(-1, 0), snap.get(0, 99), snap.get(1, 2), snap.get_cell_value(5, 9)), (0, 0, b2.get(1, 2), b2.get_cell_value(5, 9)));
            assert!(!GridSnapshot::new(&*b1).is_copy());
        }
    }
}
