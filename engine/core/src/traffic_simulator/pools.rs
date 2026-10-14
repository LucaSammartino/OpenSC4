// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The neighbour pools (traffic-network.md §2.7.2–§2.7.5; traffic-simulator
//! §4.2, §6.1 row 19): one property holder per established neighbouring
//! city, kept by ID, and the default pool. The simulator owns them; the
//! network model's edge records only hold the IDs.
//!
//! The pathfinders and the network model see a pool as a holder token from
//! a range the simulator reserves ([`pool_token`]). The simulator's own
//! holder accesses go through [`super::adapters::Holders`], which sends
//! these tokens here and every other token to the host.

use std::collections::{BTreeMap, BTreeSet};

use crate::property::PropertyHolder;
use crate::traffic_network::ANY_TRAVEL_TYPE;
use crate::variant::{mem, Elem, Variant};
use crate::x87::{ftol, r24};

use super::adapters::{capacity_element, set_capacity_element, Holders};
use super::cycle::Run;
use super::prop;

/// The first holder token of the pools: `POOL_TOKEN_BASE | id`, id 0 the
/// default pool. A host must not hand out holder tokens in
/// `POOL_TOKEN_BASE ..= POOL_TOKEN_BASE | 0xFFFF` (a FreeCity4 convention
/// with no original counterpart; below 2⁵³ so a JSON vector can name it).
pub const POOL_TOKEN_BASE: u64 = 0x000F_C4F0_0000_0000;

/// The region-cell position of a pool's city (traffic-network.md §2.7.2
/// step 5): s32 x and z.
pub const POOL_CITY_X: u32 = 0x4A23_576B;
pub const POOL_CITY_Z: u32 = 0x4A23_576C;

/// The holder token of pool `id` (0: the default pool).
pub fn pool_token(id: u16) -> u64 {
    POOL_TOKEN_BASE | u64::from(id)
}

/// The pool ID of a holder token, `None` for a host token.
pub fn pool_of_token(token: u64) -> Option<u16> {
    (token & !0xFFFF == POOL_TOKEN_BASE).then_some(token as u16)
}

/// The pools (traffic-network.md §2.7.3).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pools {
    /// The default pool: created by Init with `SetMaxTripCapacity(default,
    /// 3, 0xFFFFFFFF)`. Not saved.
    pub default: PropertyHolder,
    /// The pools by ID (≥ 1), saved in §6.1 row 19.
    pub map: BTreeMap<u16, PropertyHolder>,
}

impl Pools {
    /// The holder of pool `id`; 0 is the default pool.
    pub fn holder(&self, id: u16) -> Option<&PropertyHolder> {
        if id == 0 {
            Some(&self.default)
        } else {
            self.map.get(&id)
        }
    }

    pub fn holder_mut(&mut self, id: u16) -> Option<&mut PropertyHolder> {
        if id == 0 {
            Some(&mut self.default)
        } else {
            self.map.get_mut(&id)
        }
    }

    /// Whether a pool has this ID (the default pool is not in the map).
    pub fn exists(&self, id: u16) -> bool {
        self.map.contains_key(&id)
    }

    /// The first pool, in ascending ID order, whose holder has the s32
    /// properties `0x4A23576B = px` and `0x4A23576C = pz` (§2.7.2 step 5,
    /// §2.7.4 step 4).
    pub fn find(&self, px: i32, pz: i32) -> Option<u16> {
        let is = |h: &PropertyHolder, id: u32, v: i32| {
            h.get(id)
                .and_then(|p| p.items().first().map(|e| e.as_i64() as i32 == v))
                .unwrap_or(false)
        };
        self.map
            .iter()
            .find(|(_, h)| is(h, POOL_CITY_X, px) && is(h, POOL_CITY_Z, pz))
            .map(|(&id, _)| id)
    }

    /// A new pool with only the two position properties, under the
    /// smallest unused ID ≥ 1 (§2.7.2 step 5). `None` when all 65,535 IDs
    /// are taken *[choice: the record then keeps pool 0]*.
    pub fn create(&mut self, px: i32, pz: i32) -> Option<u16> {
        let id = (1..=u16::MAX).find(|id| !self.map.contains_key(id))?;
        let mut h = PropertyHolder::new();
        h.add(POOL_CITY_X, Variant::Scalar(Elem::I32(px)));
        h.add(POOL_CITY_Z, Variant::Scalar(Elem::I32(pz)));
        self.map.insert(id, h);
        Some(id)
    }

    /// The pool sweep's step 2 (§2.7.3): every pool whose ID is not in
    /// `keep` is released and removed.
    pub fn sweep(&mut self, keep: &BTreeSet<u16>) {
        self.map.retain(|id, _| keep.contains(id));
    }
}

// ---------------------------------------------------------------------------
// The neighbour connections (traffic-network.md §2.7.2–§2.7.5)
// ---------------------------------------------------------------------------

impl Run<'_, '_> {
    /// `AddNeighborConnection(type, x, z)` (`0x007153C0`, §2.7.2, TS6).
    pub(crate) fn add_neighbor_connection(&mut self, conn_type: u32, x: i32, z: i32) {
        // 1.
        let Some(conn) = self.h.find_connection(conn_type, x, z) else {
            return;
        };
        // 3. The cost candidate: `commuteMaxTime`, or the trip length the
        // neighbouring city stored on its end of the connection.
        let mut f = self.st.tuning.commute_max_time;
        if let Some(other) = self.h.connection_on_other_side(conn) {
            let holder = self.h.connection_holder(other);
            if holder != 0 {
                if let Some(v) = self
                    .h
                    .holder_property(holder, prop::CONNECTION_TRIP_LENGTH)
                    .and_then(|v| v.items().first().map(|e| e.as_f64() as f32))
                {
                    f = v;
                }
            }
        }
        // 5. The pool of the established city on the other side.
        let pool = match self.h.established_city_position(conn) {
            None => 0,
            Some((px, pz)) => {
                let mut pools = self.pools.borrow_mut();
                match pools.find(px, pz) {
                    Some(id) => id,
                    None => pools.create(px, pz).unwrap_or(0),
                }
            }
        };
        // 2, 4, 5: the record (the cost only grows).
        self.cx.s.add_neighbor_connection_record(conn_type, x, z, f, pool);
    }

    /// `RemoveNeighborConnection(type, x, z)` (`0x00713AB0`, §2.7.2, TS6).
    pub(crate) fn remove_neighbor_connection(&mut self, conn_type: u32, x: i32, z: i32) {
        // 1, 2.
        if !self.cx.s.remove_neighbor_connection_record(conn_type, x, z) {
            return;
        }
        self.pool_sweep();
        // 3. `freeDest[k] −= 1` per set bit (wrapping), then the byte := 0.
        let b = self.st.dest_byte(x, z);
        for k in 0..4 {
            if b & (1 << k) != 0 {
                self.st.set_free(k, self.st.free(k).wrapping_sub(1));
            }
        }
        self.st.set_dest_byte(x, z, 0);
    }

    /// The pool sweep (`0x007138B0`, §2.7.3): pools no swept record names
    /// are released.
    pub(crate) fn pool_sweep(&mut self) {
        let keep = self.cx.s.pool_ids_in_use();
        self.pools.borrow_mut().sweep(&keep);
    }

    /// `RecalculateNeighborConnectionCapacities` (`0x00714E60`, §2.7.4,
    /// TS12), the end of StartRecalc.
    pub(crate) fn recalculate_neighbor_capacities(&mut self) {
        let holders = Holders {
            h: self.h,
            pools: self.pools,
        };
        // 1. The regional jobs, truncated toward zero, as u32 *[choice: 0
        // when the node is missing]*.
        let j: [u32; 3] = std::array::from_fn(|w| {
            self.h
                .regional_demand(w as u32)
                .map_or(0, |v| ftol(f64::from(v)) as u32)
        });
        // 2, 3.
        let cities = self.h.connected_cities();
        let t: [u32; 3] =
            std::array::from_fn(|w| cities.iter().fold(0u32, |a, c| a.wrapping_add(c.population[w] as u32)));
        // 4. Per city with a pool: `trunc(r24(r24(E / T) × J))`, the low 32
        // bits (NaN or out of range → 0); `E` signed, `T` and `J` unsigned.
        for c in &cities {
            let Some(id) = self.pools.borrow().find(c.position.0, c.position.1) else {
                continue;
            };
            let token = pool_token(id);
            for w in 0..3 {
                let q = r24(f64::from(c.population[w]) / f64::from(t[w]));
                let cap = ftol(r24(q * f64::from(j[w]))) as u32;
                set_capacity_element(&holders, token, w, cap);
                set_capacity_element(&holders, token, 4 + w, 0);
            }
            set_capacity_element(&holders, token, 3, u32::MAX);
            set_capacity_element(&holders, token, 7, 0);
        }
        // 5.
        set_capacity_element(&holders, pool_token(0), 7, 0);
        // 6. The edge pass: records 0 … X−2 with a mask, at cells with a map
        // exit; the holder's capacities set the destination bits and count
        // `freeDest` (also when the bit was set, *[quirk]*).
        for (_, (x, z), _) in self.cx.s.edge_pass_records() {
            if !self.cx.s.has_map_exit(x, z) {
                continue;
            }
            let id = {
                let pools = self.pools.borrow();
                self.cx.s.neighbor_pool_id(ANY_TRAVEL_TYPE, x, z, |id| pools.exists(id))
            };
            let token = pool_token(id.unwrap_or(0));
            let mut byte = self.st.dest_byte(x, z);
            for k in 0..4 {
                if capacity_element(&holders, token, k) != 0 {
                    byte |= 1 << k;
                    self.st.set_free(k, self.st.free(k).wrapping_add(1));
                }
            }
            self.st.set_dest_byte(x, z, byte);
        }
    }

    /// `UpdateNeighborConnections` (state 11, `0x00714BE0`, §2.7.5, TS2):
    /// on each connection's own holder, the arrival arrays and counts roll.
    pub(crate) fn update_neighbor_connections(&mut self) {
        let holders = Holders {
            h: self.h,
            pools: self.pools,
        };
        for conn in self.h.all_connections() {
            let holder = self.h.connection_holder(conn);
            if holder == 0 {
                continue;
            }
            // 1.
            for i in 0..3 {
                let id = prop::POOL_ARRIVALS + i;
                if let Some(v) = self.h.holder_property(holder, id) {
                    self.h.set_holder_property(holder, prop::NEIGHBOUR_TARGETS + i, v);
                }
                self.h
                    .set_holder_property(holder, id, Variant::array(mem::UINT32, vec![Elem::U32(0); 9]));
            }
            // 2. Through the element store (the property is created when
            // missing).
            for k in 0..4 {
                let v = capacity_element(&holders, holder, 4 + k);
                set_capacity_element(&holders, holder, 8 + k, v);
                set_capacity_element(&holders, holder, 4 + k, 0);
            }
        }
    }
}
