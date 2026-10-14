// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The two hosts the simulator builds for its collaborators: the
//! [`NetworkHost`] of the network model ([`NetAdapter`]) and the
//! [`PathfinderHost`] of the pathfinders ([`PfAdapter`]); and the trip
//! capacity property (§4.2).

use std::cell::RefCell;
use std::rc::Rc;

use crate::grid::{CellRect, GridProvider, TractGrid};
use crate::pathfinder::{PathfinderHost, TravelTables};
use crate::stream::RefValue;
use crate::traffic_network::{NetworkHost, NetworkState, PathInfo};
use crate::variant::{mem, Elem, Variant};

use super::pools::{pool_of_token, pool_token, Pools};
use super::state::{DamageState, State};
use super::{prop, TrafficSimHost};

/// The number of elements of the trip capacity property (§4.2).
pub const CAPACITY_LEN: usize = 12;

/// Where a holder's properties live: the host's holders, or the
/// simulator's neighbour pools through [`Holders`].
pub trait HolderStore {
    fn get_property(&self, holder: u64, id: u32) -> Option<Variant>;
    fn put_property(&self, holder: u64, id: u32, value: Variant);
}

impl<T: NetworkHost + ?Sized> HolderStore for T {
    fn get_property(&self, holder: u64, id: u32) -> Option<Variant> {
        self.holder_property(holder, id)
    }
    fn put_property(&self, holder: u64, id: u32, value: Variant) {
        self.set_holder_property(holder, id, value)
    }
}

/// The simulator's view of the holders: a pool token
/// ([`super::pools::pool_token`]) is one of the simulator's pools, every
/// other token the host's. A token of a pool that no longer exists has no
/// properties and ignores writes.
pub struct Holders<'a> {
    pub h: &'a dyn TrafficSimHost,
    pub pools: &'a RefCell<Pools>,
}

impl HolderStore for Holders<'_> {
    fn get_property(&self, holder: u64, id: u32) -> Option<Variant> {
        match pool_of_token(holder) {
            Some(p) => self.pools.borrow().holder(p).and_then(|h| h.get(id).cloned()),
            None => self.h.holder_property(holder, id),
        }
    }
    fn put_property(&self, holder: u64, id: u32, value: Variant) {
        match pool_of_token(holder) {
            Some(p) => {
                if let Some(h) = self.pools.borrow_mut().holder_mut(p) {
                    h.add(id, value);
                }
            }
            None => self.h.set_holder_property(holder, id, value),
        }
    }
}

/// The capacity property as (u32 elements, the 12 values) when it is a
/// 12-element u16 (`0x8004`) or u32 array. The original checks only the
/// variant type and reads any other type as u32 data with no count check;
/// *[choice]* anything else counts as missing (§4.2, TS18).
fn capacity_array(v: &Variant) -> Option<(bool, [u32; CAPACITY_LEN])> {
    match v {
        Variant::Array { elem, items }
            if items.len() == CAPACITY_LEN && (*elem == mem::UINT16 || *elem == mem::UINT32) =>
        {
            let mut out = [0u32; CAPACITY_LEN];
            for (o, e) in out.iter_mut().zip(items) {
                *o = e.as_i64() as u32;
            }
            Some((*elem == mem::UINT32, out))
        }
        _ => None,
    }
}

/// The elements of property `0x8A0B4100` of a holder as u32, `None` when the
/// property is missing (or not a 12-element u16 / u32 array, or the holder
/// is null).
pub fn capacity_values<H: HolderStore + ?Sized>(h: &H, holder: u64) -> Option<Vec<u32>> {
    if holder == 0 {
        return None;
    }
    h.get_property(holder, prop::TRIP_CAPACITY)
        .as_ref()
        .and_then(capacity_array)
        .map(|(_, v)| v.to_vec())
}

/// Element `i` of the capacity property, 0 when missing (§4.2:
/// `GetMaxTripCapacity` is element `k`, arrived this cycle `4 + k`, arrived
/// last cycle `8 + k`).
pub fn capacity_element<H: HolderStore + ?Sized>(h: &H, holder: u64, i: usize) -> u32 {
    capacity_values(h, holder).and_then(|v| v.get(i).copied()).unwrap_or(0)
}

/// The element store `SetTripCapacity` (`0x0070DE70`, §4.2, TS18), the path
/// of every write: a missing property is created as 12 u16 zeros; a u16
/// array takes `value` as u16 while it is below `0xFFFF`, and is promoted
/// to a u32 array (the 12 values zero-extended) by a value `≥ 0xFFFF`; a u32
/// array never goes back. A null holder, or an index past the 12 elements,
/// changes nothing *[choice]*.
pub fn set_capacity_element<H: HolderStore + ?Sized>(h: &H, holder: u64, i: usize, value: u32) {
    if holder == 0 || i >= CAPACITY_LEN {
        return;
    }
    let (mut wide, mut vals) = h
        .get_property(holder, prop::TRIP_CAPACITY)
        .as_ref()
        .and_then(capacity_array)
        .unwrap_or((false, [0; CAPACITY_LEN]));
    if value >= 0xFFFF {
        wide = true;
    }
    vals[i] = value;
    let var = if wide {
        Variant::array(mem::UINT32, vals.into_iter().map(Elem::U32).collect())
    } else {
        Variant::array(mem::UINT16, vals.into_iter().map(|v| Elem::U16(v as u16)).collect())
    };
    h.put_property(holder, prop::TRIP_CAPACITY, var);
}

/// `GetNeighborPoolPropertyHolder(t, x, z)` (`0x007114E0`,
/// traffic-network.md §2.7.1) over the record at the off-map cell: null
/// when the record has none of the wanted connection types and no ferry;
/// else the pool's token, the default pool's for pool 0 or for a stale ID
/// (which the record forgets, *[compat]*).
pub fn neighbor_pool_holder(s: &mut NetworkState, pools: &RefCell<Pools>, t: u32, x: i32, z: i32) -> Option<u64> {
    let p = pools.borrow();
    s.neighbor_pool_id(t, x, z, |id| p.exists(id)).map(pool_token)
}

/// The network model's host: everything forwards to the simulator's host,
/// except the three questions the simulator answers itself (trip capacity,
/// the neighbour pools, the pothole table).
pub struct NetAdapter {
    pub host: Rc<dyn TrafficSimHost>,
    pub damage: Rc<RefCell<DamageState>>,
    pub pools: Rc<RefCell<Pools>>,
}

impl NetAdapter {
    fn holders(&self) -> Holders<'_> {
        Holders {
            h: &*self.host,
            pools: &self.pools,
        }
    }
}

impl GridProvider<u32> for NetAdapter {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u32>>> {
        GridProvider::<u32>::create_grid(&*self.host, instance, tract_size)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<u32>>> {
        GridProvider::<u32>::resolve_grid(&*self.host, r)
    }
}

impl GridProvider<u8> for NetAdapter {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        GridProvider::<u8>::create_grid(&*self.host, instance, tract_size)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<u8>>> {
        GridProvider::<u8>::resolve_grid(&*self.host, r)
    }
}

impl NetworkHost for NetAdapter {
    fn cell_counts(&self) -> (i32, i32) {
        self.host.cell_counts()
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        self.host.position_to_cell(x, z)
    }
    fn network_occupants_in_rect(&self, rect: CellRect) -> Vec<u64> {
        self.host.network_occupants_in_rect(rect)
    }
    fn subway_occupants_in_rect(&self, rect: CellRect) -> Vec<u64> {
        self.host.subway_occupants_in_rect(rect)
    }
    fn prop_occupants_in_rect(&self, rect: CellRect) -> Vec<u64> {
        self.host.prop_occupants_in_rect(rect)
    }
    fn occupant_position(&self, occ: u64) -> [f32; 3] {
        self.host.occupant_position(occ)
    }
    fn occupant_bounding_cells(&self, occ: u64) -> CellRect {
        self.host.occupant_bounding_cells(occ)
    }
    fn prop_type(&self, occ: u64) -> u32 {
        self.host.prop_type(occ)
    }
    fn occupant_property(&self, occ: u64, id: u32) -> Option<Variant> {
        self.host.occupant_property(occ, id)
    }
    fn has_network_flag(&self, occ: u64, flag: u32) -> bool {
        self.host.has_network_flag(occ, flag)
    }
    fn has_any_network_flag(&self, occ: u64, mask: u32) -> bool {
        self.host.has_any_network_flag(occ, mask)
    }
    fn is_of_type(&self, occ: u64, network_type: u32) -> bool {
        self.host.is_of_type(occ, network_type)
    }
    fn path_connection(&self, occ: u64, path_type: u32, entry: u32, exit: u32) -> bool {
        self.host.path_connection(occ, path_type, entry, exit)
    }
    fn path_info(&self, occ: u64) -> Option<PathInfo> {
        self.host.path_info(occ)
    }
    fn path_info_turn_around(&self, pi: PathInfo, path_type: u32, entry: u32) -> bool {
        self.host.path_info_turn_around(pi, path_type, entry)
    }
    fn path_info_exits_from_entry(&self, pi: PathInfo, path_type: u32, entry: u32) -> Vec<(u8, u64)> {
        self.host.path_info_exits_from_entry(pi, path_type, entry)
    }
    fn path_info_entries_near(&self, pi: PathInfo, path_type: u32, vertex: u64, exit: u32, radius: f32) -> Vec<u32> {
        self.host.path_info_entries_near(pi, path_type, vertex, exit, radius)
    }
    fn is_tunnel(&self, occ: u64) -> bool {
        self.host.is_tunnel(occ)
    }
    fn tunnel_paired_portal(&self, occ: u64) -> Option<u64> {
        self.host.tunnel_paired_portal(occ)
    }
    fn lot_at(&self, x: i32, z: i32, anchor_only: bool) -> Option<u64> {
        self.host.lot_at(x, z, anchor_only)
    }
    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        self.host.lot_bounding_rect(lot)
    }
    fn lot_location(&self, lot: u64) -> (i32, i32) {
        self.host.lot_location(lot)
    }
    fn lot_facing(&self, lot: u64) -> u32 {
        self.host.lot_facing(lot)
    }
    fn lot_building(&self, lot: u64) -> Option<u64> {
        self.host.lot_building(lot)
    }
    fn lot_populations(&self, lot: u64) -> Vec<Vec<(u32, u32)>> {
        self.host.lot_populations(lot)
    }
    fn lot_holder(&self, lot: u64) -> u64 {
        self.host.lot_holder(lot)
    }
    fn holder_property(&self, holder: u64, id: u32) -> Option<Variant> {
        self.host.holder_property(holder, id)
    }
    fn set_holder_property(&self, holder: u64, id: u32, value: Variant) {
        self.host.set_holder_property(holder, id, value)
    }
    fn remove_holder_property(&self, holder: u64, id: u32) -> bool {
        self.host.remove_holder_property(holder, id)
    }
    /// `GetMaxTripCapacity(holder, k)` (slot 20, §4.2), pools included.
    fn max_trip_capacity(&self, holder: u64, k: u32) -> u32 {
        capacity_element(&self.holders(), holder, k as usize)
    }
    /// Not asked: the model asks [`NetworkHost::neighbor_pool_holder_in`].
    fn neighbor_pool_holder(&self, _x: i32, _z: i32) -> Option<u64> {
        None
    }
    /// `GetNeighborPoolPropertyHolder(1, x, z)` (traffic-network.md §4.12)
    /// over the simulator's pools.
    fn neighbor_pool_holder_in(&self, s: &mut NetworkState, x: i32, z: i32) -> Option<u64> {
        neighbor_pool_holder(s, &self.pools, 1, x, z)
    }
    /// `IsRoadDamaged` (slot 16): the damaged road set.
    fn is_road_damaged(&self, x: i32, z: i32) -> bool {
        self.damage.try_borrow().is_ok_and(|d| d.is_road_damaged(x, z))
    }
    fn cell_under_water(&self, x: i32, z: i32) -> bool {
        self.host.cell_under_water(x, z)
    }
    fn is_navigable(&self, x: i32, z: i32) -> bool {
        self.host.is_navigable(x, z)
    }
    fn water_depth(&self, x: i32, z: i32) -> f32 {
        self.host.water_depth(x, z)
    }
    fn body_of_water(&self, x: i32, z: i32) -> Option<u16> {
        self.host.body_of_water(x, z)
    }
    fn connections_of_type(&self, connection_type: u32) -> Vec<u64> {
        self.host.connections_of_type(connection_type)
    }
    fn all_connections(&self) -> Vec<u64> {
        self.host.all_connections()
    }
    fn connection_cell(&self, conn: u64) -> (i32, i32) {
        self.host.connection_cell(conn)
    }
    fn post(&self, msg_type: u32) {
        self.host.post(msg_type)
    }
    fn add_ref(&self, token: u64) {
        NetworkHost::add_ref(&*self.host, token)
    }
    fn release(&self, token: u64) {
        NetworkHost::release(&*self.host, token)
    }
}

/// The pathfinders' host: the simulator's state (tables, volume grids, the
/// destination map and free counts, the income accumulator, the strike)
/// and the game through the simulator's host.
pub struct PfAdapter<'a> {
    pub h: &'a dyn TrafficSimHost,
    pub st: &'a State,
    pub pools: &'a RefCell<Pools>,
}

impl PfAdapter<'_> {
    fn holders(&self) -> Holders<'_> {
        Holders {
            h: self.h,
            pools: self.pools,
        }
    }
}

impl PathfinderHost for PfAdapter<'_> {
    fn cell_counts(&self) -> (i32, i32) {
        self.h.cell_counts()
    }
    /// Searches never suspend (§3.4: max work time 0), so the timer is never
    /// read.
    fn work_deadline(&self, _ms: u32) -> u64 {
        0
    }
    fn work_deadline_reached(&self, _deadline: u64) -> bool {
        false
    }
    fn travel_tables(&self) -> TravelTables {
        let t = &self.st.tuning;
        TravelTables {
            default_start_cost: t.default_start_cost(),
            can_reach_destination: t.can_reach,
            generates_traffic: t.generates_traffic,
            income_per_tile: t.income_per_tile,
            affected_by_traffic: t.affected_by_traffic,
        }
    }
    fn max_distance_origin_to_network(&self) -> u32 {
        self.st.tuning.max_distance_origin
    }
    fn nearest_destination_attractiveness(&self) -> f32 {
        self.st.tuning.nearest_attractiveness
    }
    fn strike_state(&self) -> u8 {
        self.st.strike_state
    }
    fn trip_scale_for_display(&self) -> f32 {
        self.st.tuning.trip_display_multiplier
    }
    fn volume(&self, n: u32, x: i32, z: i32) -> f32 {
        self.st.volume_grid(n).map_or(0.0, |g| g.get_cell_value(x, z))
    }
    fn set_volume(&self, n: u32, x: i32, z: i32, v: f32) {
        if let Some(g) = self.st.volume_grid(n) {
            g.set(x, z, v);
        }
    }
    fn destination_byte(&self, x: i32, z: i32) -> u8 {
        self.st.dest_byte(x, z)
    }
    fn set_destination_byte(&self, x: i32, z: i32, bits: u8) {
        self.st.set_dest_byte(x, z, bits);
    }
    fn free_destinations(&self, k: u32) -> u32 {
        self.st.free(k as usize)
    }
    fn set_free_destinations(&self, k: u32, n: u32) {
        self.st.set_free(k as usize, n);
    }
    fn max_trip_capacity(&self, holder: u64, k: u32) -> u32 {
        capacity_element(&self.holders(), holder, k as usize)
    }
    fn used_trip_capacity(&self, holder: u64, k: u32) -> u32 {
        capacity_element(&self.holders(), holder, 4 + k as usize)
    }
    /// Not asked: the search asks [`PathfinderHost::neighbor_pool_holder_in`].
    fn neighbor_pool_holder(&self, _t: u32, _x: i32, _z: i32) -> Option<u64> {
        None
    }
    fn neighbor_pool_holder_in(&self, net: &mut NetworkState, t: u32, x: i32, z: i32) -> Option<u64> {
        neighbor_pool_holder(net, self.pools, t, x, z)
    }
    fn income_accumulator(&self) -> f64 {
        self.st.income_acc.get()
    }
    fn set_income_accumulator(&self, v: f64) {
        self.st.income_acc.set(v);
    }
    /// `GetLot(x, z, false)`: any cell of a lot; the pathfinder always passes
    /// false (PF3, pathfinder.md §8).
    fn lot_at(&self, x: i32, z: i32) -> Option<u64> {
        self.h.lot_at(x, z, false)
    }
    fn lot_holder(&self, lot: u64) -> u64 {
        self.h.lot_holder(lot)
    }
    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        self.h.lot_bounding_rect(lot)
    }
    fn occupants_in_cells(&self, rect: CellRect) -> Vec<u64> {
        self.h.occupants_in_cells(rect)
    }
    fn occupant_filter_includes(&self, filter: u64, occupant: u64) -> bool {
        self.h.occupant_filter_includes(filter, occupant)
    }
    fn is_ordinance_on(&self, id: u32) -> bool {
        self.h.is_ordinance_on(id)
    }
}
