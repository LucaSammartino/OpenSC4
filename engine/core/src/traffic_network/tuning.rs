// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Capacity tuning (§4.5, `ReadSimulatorTunableValues`) and the intersection
//! capacity effect (§4.6). The traffic simulator reads these from its tuning
//! exemplar and hands them to the network model; the shipped values are here
//! for hosts and tests without the game's data files.

use crate::property::PropertyHolder;
use crate::props;
use crate::variant::{mem, Elem, Variant};
use crate::x87;

/// Tuning property IDs the network reads (§4.5).
pub mod id {
    /// Damaged road extra step cost (f32).
    pub const DAMAGED_ROAD_COST: u32 = 0xA923_56B0;
    /// Network Traffic Capacity (13 floats).
    pub const CAPACITY: u32 = 0xA923_56B3;
    /// Intersection and Turn Capacity Effect (N floats).
    pub const INTERSECTION: u32 = 0xA923_56BB;
}

/// The number of network types (§2.2).
pub const TYPE_COUNT: usize = 13;

/// The shipped Network Traffic Capacity values (§4.5).
pub const SHIPPED_CAPACITY: [f32; TYPE_COUNT] = [
    1000.0, 3000.0, 4000.0, 100.0, 0.0, 0.0, 2500.0, 3000.0, 3000.0, 3000.0, 2000.0, 0.0, 4000.0,
];

/// The shipped Intersection and Turn Capacity Effect values (§4.5): 0.7, 0.8,
/// 0.9.
pub const SHIPPED_INTERSECTION: [f32; 3] = [0.7, 0.8, 0.9];

/// The shipped damaged-road extra step cost (§4.5).
pub const SHIPPED_DAMAGED_ROAD_COST: f32 = 0.1;

/// The capacity tables and the intersection effect (§4.5, §4.6).
#[derive(Clone, Debug, PartialEq)]
pub struct Tuning {
    /// `rawCapTable[13]`: the raw capacity values (§4.5).
    pub raw_cap_table: [f32; TYPE_COUNT],
    /// `invCapTable[13]`: `r24(1 / rawCap)` (1.0 when `|raw| ≤ 1e-10`) (§4.5).
    pub inv_cap_table: [f32; TYPE_COUNT],
    /// The damaged-road extra step cost (§2.3, §4.5); code default 32.0.
    pub damaged_road_cost: f32,
    /// The intersection-effect count N (§4.6).
    pub intersection_count: usize,
    /// `values[i] = r24(1.0 / v[i])`, no zero guard (§4.5).
    pub intersection_values: Vec<f32>,
}

impl Default for Tuning {
    /// The constructor values (§2.3): `invCapTable` is not set by the
    /// constructor; FreeCity4 uses 1.0 before the first read *[choice]*. The
    /// damaged-road extra step cost is 32.0.
    fn default() -> Self {
        Tuning {
            raw_cap_table: [0.0; TYPE_COUNT],
            inv_cap_table: [1.0; TYPE_COUNT],
            damaged_road_cost: super::DEFAULT_DAMAGED_ROAD_COST,
            intersection_count: 0,
            intersection_values: Vec::new(),
        }
    }
}

impl Tuning {
    /// The shipped tuning (§4.5): the shipped capacities and intersection
    /// values, with the shipped damaged-road cost.
    pub fn shipped() -> Self {
        let mut t = Tuning::default();
        t.set_capacity(&SHIPPED_CAPACITY);
        t.set_intersection(&SHIPPED_INTERSECTION);
        t.damaged_road_cost = SHIPPED_DAMAGED_ROAD_COST;
        t
    }

    /// Reads the Network Traffic Capacity property (§4.5): for k = 0…12,
    /// `v = (k < count) ? value[k] : 0.0`, `rawCapTable[k] = v`, and
    /// `invCapTable[k] = r24(1.0 / v)` if `|v| > 1e-10` else 1.0.
    pub fn set_capacity(&mut self, values: &[f32]) {
        for k in 0..TYPE_COUNT {
            let v = values.get(k).copied().unwrap_or(0.0);
            self.raw_cap_table[k] = v;
            self.inv_cap_table[k] = if v.abs() > 1e-10 {
                x87::r24(1.0 / f64::from(v)) as f32
            } else {
                1.0
            };
        }
    }

    /// Reads the Intersection and Turn Capacity Effect property (§4.5):
    /// `N = count`, `values[i] = r24(1.0 / v[i])`, no zero guard.
    pub fn set_intersection(&mut self, values: &[f32]) {
        self.intersection_count = values.len();
        self.intersection_values = values
            .iter()
            .map(|&v| x87::r24(1.0 / f64::from(v)) as f32)
            .collect();
    }

    /// Reads the three network properties from a tuning exemplar (§4.5). A
    /// missing capacity or intersection property leaves that table unchanged;
    /// a missing damaged-road cost leaves the code default.
    pub fn reload(&mut self, exemplar: Option<&PropertyHolder>) {
        let Some(h) = exemplar else { return };
        if let Some(v) = props::f32_of(h, id::DAMAGED_ROAD_COST) {
            self.damaged_road_cost = v;
        }
        if let Some(v) = props::f32s(h, id::CAPACITY) {
            self.set_capacity(&v);
        }
        if let Some(v) = props::f32s(h, id::INTERSECTION) {
            self.set_intersection(&v);
        }
    }

    /// The shipped "Traffic Simulator" exemplar's network properties (§4.5),
    /// for hosts and tests without the game's data files.
    pub fn shipped_exemplar() -> PropertyHolder {
        let mut h = PropertyHolder::new();
        let arr = |v: &[f32]| Variant::array(mem::FLOAT32, v.iter().map(|&x| Elem::F32(x)).collect());
        h.add(id::DAMAGED_ROAD_COST, Variant::Scalar(Elem::F32(SHIPPED_DAMAGED_ROAD_COST)));
        h.add(id::CAPACITY, arr(&SHIPPED_CAPACITY));
        h.add(id::INTERSECTION, arr(&SHIPPED_INTERSECTION));
        h
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shipped `invCapTable` bit patterns of §4.5.
    #[test]
    fn shipped_capacity_bit_patterns() {
        let t = Tuning::shipped();
        let bits = |v: f32| v.to_bits();
        // Road 1000, Rail 3000, Highway 4000, Street 100, Pipe 0, Pole 0,
        // Avenue 2500, Subway 3000, LightRail 3000, Monorail 3000, OneWay
        // 2000, Dirt 0, GroundHighway 4000.
        assert_eq!(bits(t.inv_cap_table[0]), 0x3A83_126F); // 1000
        assert_eq!(bits(t.inv_cap_table[1]), 0x39AE_C33E); // 3000
        assert_eq!(bits(t.inv_cap_table[2]), 0x3983_126F); // 4000
        assert_eq!(bits(t.inv_cap_table[3]), 0x3C23_D70A); // 100
        assert_eq!(bits(t.inv_cap_table[4]), 0x3F80_0000); // 0 → 1.0
        assert_eq!(bits(t.inv_cap_table[5]), 0x3F80_0000); // 0 → 1.0
        assert_eq!(bits(t.inv_cap_table[6]), 0x39D1_B717); // 2500
        assert_eq!(bits(t.inv_cap_table[10]), 0x3A03_126F); // 2000
        assert_eq!(bits(t.inv_cap_table[11]), 0x3F80_0000); // 0 → 1.0
    }

    #[test]
    fn shipped_intersection_values() {
        let t = Tuning::shipped();
        assert_eq!(t.intersection_count, 3);
        // §11: values 0x3FB6DB6E (0.7), 0x3FA00000 (0.8), 0x3F8E38E4 (0.9).
        assert_eq!(t.intersection_values[0].to_bits(), 0x3FB6_DB6E);
        assert_eq!(t.intersection_values[1].to_bits(), 0x3FA0_0000);
        assert_eq!(t.intersection_values[2].to_bits(), 0x3F8E_38E4);
    }

    #[test]
    fn capacity_round_trip() {
        // §4.5: GetCapacity reports 1/invCap; 1000 → 999.99994 (0x4479FFFF),
        // 3000 → 3000.0, 4000 → 3999.9998 (0x4579FFFF).
        let t = Tuning::shipped();
        let cap = |inv: f32| (x87::r24(1.0 / f64::from(inv)) as f32).to_bits();
        assert_eq!(cap(t.inv_cap_table[0]), 0x4479_FFFF); // 1000
        assert_eq!(cap(t.inv_cap_table[1]), 0x453A_0000); // 3000
        assert_eq!(cap(t.inv_cap_table[2]), 0x4579_FFFF); // 4000
        assert_eq!(cap(t.inv_cap_table[10]), 0x44F9_FFFF); // OneWay 2000 → 1999.9999
    }

    #[test]
    fn reload_from_exemplar() {
        let mut t = Tuning::default();
        t.reload(Some(&Tuning::shipped_exemplar()));
        assert_eq!(t, Tuning::shipped());
        // A missing exemplar changes nothing.
        let mut u = Tuning::default();
        u.reload(None);
        assert_eq!(u, Tuning::default());
    }
}
