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
use crate::traffic_network::{NetworkHost, PathInfo};
use crate::variant::{mem, Elem, Variant};

use super::state::{DamageState, State};
use super::{prop, TrafficSimHost};

/// The number of elements of the trip capacity property (§4.2).
pub const CAPACITY_LEN: usize = 12;

/// The elements of property `0x8A0B4100` of a holder as u32, `None` when the
/// property is missing (or the holder is null).
pub fn capacity_values<H: NetworkHost + ?Sized>(h: &H, holder: u64) -> Option<Vec<u32>> {
    if holder == 0 {
        return None;
    }
    h.holder_property(holder, prop::TRIP_CAPACITY)
        .map(|v| v.items().iter().map(|e| e.as_i64() as u32).collect())
}

/// Element `i` of the capacity property, 0 when missing (§4.2:
/// `GetMaxTripCapacity` is element `k`, arrived this cycle `4 + k`, arrived
/// last cycle `8 + k`).
pub fn capacity_element<H: NetworkHost + ?Sized>(h: &H, holder: u64, i: usize) -> u32 {
    capacity_values(h, holder).and_then(|v| v.get(i).copied()).unwrap_or(0)
}

/// Writes the capacity property from its elements: padded to 12 elements;
/// u16 elements until a value reaches `0xFFFF`, then (and from then on) u32
/// (§4.2) *[choice: the exact promotion rule is not stated]*.
fn store_capacity<H: NetworkHost + ?Sized>(h: &H, holder: u64, mut vals: Vec<u32>, was_wide: bool) {
    if vals.len() < CAPACITY_LEN {
        vals.resize(CAPACITY_LEN, 0);
    }
    let wide = was_wide || vals.iter().any(|&v| v >= 0xFFFF);
    let var = if wide {
        Variant::array(mem::UINT32, vals.into_iter().map(Elem::U32).collect())
    } else {
        Variant::array(mem::UINT16, vals.into_iter().map(|v| Elem::U16(v as u16)).collect())
    };
    h.set_holder_property(holder, prop::TRIP_CAPACITY, var);
}

fn is_wide(v: &Variant) -> bool {
    matches!(
        v.items().first(),
        Some(Elem::U32(_) | Elem::I32(_) | Elem::U64(_) | Elem::I64(_))
    )
}

/// `SetMaxTripCapacity`-style write of element `i` (§4.2: creates the
/// property if missing). An index beyond the property is ignored.
pub fn set_capacity_element<H: NetworkHost + ?Sized>(h: &H, holder: u64, i: usize, value: u32) {
    if holder == 0 {
        return;
    }
    let existing = h.holder_property(holder, prop::TRIP_CAPACITY);
    let wide = existing.as_ref().is_some_and(is_wide);
    let mut vals: Vec<u32> = existing
        .map(|v| v.items().iter().map(|e| e.as_i64() as u32).collect())
        .unwrap_or_default();
    if vals.len() < CAPACITY_LEN {
        vals.resize(CAPACITY_LEN, 0);
    }
    let Some(slot) = vals.get_mut(i) else { return };
    *slot = value;
    store_capacity(h, holder, vals, wide);
}

/// Rewrites an existing capacity property through `f` (no-op when it is
/// missing).
pub fn update_capacity<H: NetworkHost + ?Sized>(h: &H, holder: u64, f: impl FnOnce(&mut Vec<u32>)) {
    if holder == 0 {
        return;
    }
    let Some(existing) = h.holder_property(holder, prop::TRIP_CAPACITY) else {
        return;
    };
    let wide = is_wide(&existing);
    let mut vals: Vec<u32> = existing.items().iter().map(|e| e.as_i64() as u32).collect();
    if vals.len() < CAPACITY_LEN {
        vals.resize(CAPACITY_LEN, 0);
    }
    f(&mut vals);
    store_capacity(h, holder, vals, wide);
}

/// The network model's host: everything forwards to the simulator's host,
/// except the three questions the simulator answers itself (trip capacity,
/// the neighbour pools, the pothole table).
pub struct NetAdapter {
    pub host: Rc<dyn TrafficSimHost>,
    pub damage: Rc<RefCell<DamageState>>,
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
    /// `GetMaxTripCapacity(holder, k)` (slot 20, §4.2).
    fn max_trip_capacity(&self, holder: u64, k: u32) -> u32 {
        capacity_element(&*self.host, holder, k as usize)
    }
    /// `GetNeighborPoolPropertyHolder(1, x, z)` (traffic-network.md §4.12).
    fn neighbor_pool_holder(&self, x: i32, z: i32) -> Option<u64> {
        self.host.neighbor_pool_property_holder(1, x, z)
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
        capacity_element(self.h, holder, k as usize)
    }
    fn used_trip_capacity(&self, holder: u64, k: u32) -> u32 {
        capacity_element(self.h, holder, 4 + k as usize)
    }
    fn neighbor_pool_holder(&self, t: u32, x: i32, z: i32) -> Option<u64> {
        self.h.neighbor_pool_property_holder(t, x, z)
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
