// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Transit switches (§2.4): stations and other lots that change the travel
//! type. The switch phase of `DoConnectionsChanged` (§4.7.4), activation
//! (§4.7.1), the rules (§4.7.2) and deactivation (§4.7.5).

use crate::grid::CellRect;
use crate::variant::Variant;
use crate::x87;

use super::model::{NetworkState, NetworkModel};
use super::{net, side, travel, NetworkHost, ROT};

/// A transit-switch rule (§2.4): `kind` (0 or 1), a `side` bool per city
/// side, and the `from` and `to` travel types.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rule {
    /// 0 = leaving the switch, 1 = entering the switch (§4.7.4).
    pub kind: u8,
    /// One bool per city side (§2.4).
    pub side: [bool; 4],
    /// The travel type a traveller has.
    pub from: u16,
    /// The travel type it becomes.
    pub to: u16,
}

/// A transit switch (§2.4).
#[derive(Clone, Debug, PartialEq)]
pub struct TransitSwitch {
    /// The occupant that carries the rules (one reference held); 0 = none.
    pub building: u64,
    /// Traffic through the switch, maintained by the pathfinder and simulator.
    pub load: f32,
    /// Building property `0xE90E25A2`.
    pub time_factor: f32,
    /// Building property `0xE90E25A3`.
    pub capacity: f32,
    /// From the "to" types of the rules (§2.4): 2 → 1, 3 → 2, 6 → 4, 7 → 8,
    /// 8 → 0x10, plus 0x40 when a rule links subway and elevated train.
    pub flags: u8,
    /// The subway access point (§4.7.1 step 3).
    pub has_access_point: bool,
    pub access_x: u8,
    pub access_z: u8,
    /// Two f32 counters, saved, owned by traffic-simulator.md.
    pub counter_a: f32,
    pub counter_b: f32,
    /// The rules (§2.4).
    pub rules: Vec<Rule>,
    /// The kind-0 "to" mask: OR of `1 << to` over the kind-0 rules (§2.4).
    pub to_mask: u16,
    /// Property `0x0BFC0A4C` (u32), simulator use.
    pub simulator_value: u32,
    /// "Counts as intersection", building property `0x4BFC47B0` (§4.6).
    pub counts_as_intersection: bool,
}

impl Default for TransitSwitch {
    fn default() -> Self {
        TransitSwitch {
            building: 0,
            load: 0.0,
            time_factor: 0.0,
            capacity: 0.0,
            flags: 0,
            has_access_point: false,
            access_x: 0,
            access_z: 0,
            counter_a: 0.0,
            counter_b: 0.0,
            rules: Vec::new(),
            to_mask: 0,
            simulator_value: 0,
            counts_as_intersection: false,
        }
    }
}

impl TransitSwitch {
    /// The switch flags from a rule's "to" type (§2.4): 2 → 1, 3 → 2, 6 → 4,
    /// 7 → 8, 8 → 0x10.
    fn flag_for_to(to: u16) -> u8 {
        match to {
            2 => 1,
            3 => 2,
            6 => 4,
            7 => 8,
            8 => 0x10,
            _ => 0,
        }
    }

    /// The mode mask: OR of `1 << from` over the kind-0 rules with
    /// `from < 8` (§4.7.1 step 4).
    pub fn mode_mask(&self) -> u8 {
        let mut m = 0u8;
        for r in &self.rules {
            if r.kind == 0 && r.from < 8 {
                m |= 1 << r.from;
            }
        }
        m
    }
}

/// Reads the transit-switch rules of a building (§4.7.2,
/// `ReadTransitSwitchRules`). `facing` is the lot's `GetFacing()`; `entries`
/// are the u32 entries of property `0xE90E25A1`. With no lot under the
/// building's NW cell the original reads no rules *[quirk]*; the caller
/// decides that and passes an empty `entries`.
pub fn read_rules(facing: u32, entries: &[u32]) -> (Vec<Rule>, u16, u8) {
    let mut rules = Vec::new();
    let mut to_mask = 0u16;
    let mut flags = 0u8;
    let rot = ROT[(facing & 3) as usize];
    for &entry in entries {
        let b0 = (entry & 0xFF) as u8;
        let b1 = ((entry >> 8) & 0xFF) as u8;
        let b2 = ((entry >> 16) & 0xFF) as u16;
        let b3 = ((entry >> 24) & 0xFF) as u16;
        let kind = if b0 == 0x82 { 1 } else { 0 };
        let mut r = Rule {
            kind,
            side: [false; 4],
            from: b2,
            to: b3,
        };
        for i in 0..4 {
            r.side[rot[i]] = b1 & (1 << (7 - i)) != 0;
        }
        if kind == 0 {
            to_mask |= 1 << r.to;
        }
        flags |= TransitSwitch::flag_for_to(r.to);
        // 0x40 when a rule links subway (6) and elevated train (7).
        if (r.from == 6 && r.to == 7) || (r.from == 7 && r.to == 6) {
            flags |= 0x40;
        }
        rules.push(r);
    }
    (rules, to_mask, flags)
}

impl NetworkModel {
    /// `DoLotActivated`, the network part (§4.7.1): builds and registers a
    /// transit switch when the lot's building has property `0xE90E25A1`.
    /// Returns whether a switch was created. `lot_rect` is the lot's bounding
    /// rectangle; the ferry part (§4.9.1) and the trailing rebuild are driven
    /// by the caller.
    pub fn activate_switch(&self, building: u64, lot_rect: CellRect) -> bool {
        let h = self.host.clone();
        self.with(|s| {
            let Some(rules_prop) = h.occupant_property(building, super::prop::SWITCH_RULES) else {
                return false;
            };
            // Step 1: if the property's byte size is not a multiple of 4, stop.
            let entries = match &rules_prop {
                Variant::Array { items, .. } => items.iter().map(|e| e.as_i64() as u32).collect::<Vec<_>>(),
                Variant::Scalar(e) => vec![e.as_i64() as u32],
                _ => return false,
            };
            // The rules need a lot under the building's NW cell (§4.7.2)
            // *[quirk]*; without one, no rules are read.
            let facing = h
                .lot_at(lot_rect.x1, lot_rect.z1, false)
                .map_or(0, |lot| h.lot_facing(lot));
            let (rules, to_mask, mut flags) = read_rules(facing, &entries);

            // Step 2: build the switch.
            let mut sw = TransitSwitch {
                building,
                time_factor: f32_prop(&h, building, super::prop::SWITCH_TIME_FACTOR),
                capacity: f32_prop(&h, building, super::prop::SWITCH_CAPACITY),
                simulator_value: u32_prop(&h, building, super::prop::SWITCH_SIMULATOR),
                counts_as_intersection: bool_prop(&h, building, super::prop::COUNTS_AS_INTERSECTION),
                flags,
                to_mask,
                rules,
                ..TransitSwitch::default()
            };
            h.add_ref(building);

            // Step 3: a Subway "to" rule (flag 4) sets the access point.
            if sw.flags & 4 != 0 {
                let access = h.subway_occupants_in_rect(lot_rect).first().copied();
                let (ax, az) = match access {
                    Some(occ) => {
                        let p = h.occupant_position(occ);
                        h.position_to_cell(p[0], p[2])
                    }
                    // Without one, the NW cell of the building's bounding cells.
                    None => {
                        let cells = h.occupant_bounding_cells(building);
                        (cells.x1, cells.z1)
                    }
                };
                sw.has_access_point = true;
                sw.access_x = (ax & 0xFF) as u8;
                sw.access_z = (az & 0xFF) as u8;
            }

            // Step 4: the mode mask.
            let mask = sw.mode_mask();
            // Step 5: write the ID and mask grids over the lot rectangle,
            // insert the switch and increment the next ID.
            let id = s.next_switch_id;
            write_grid_rect(s, lot_rect, id, mask);
            s.switches.entry(id, || sw);
            s.next_switch_id = id + 1;
            let _ = &mut flags;
            true
        })
    }

    /// `DoLotDeactivated`, the switch part (§4.7.5): reads the switch ID at
    /// the lot rectangle's NW cell, and if the switch exists releases the
    /// building, erases the switch, zeroes the grids over the rectangle and
    /// marks the surrounding rectangle dirty for the caller's rebuild.
    /// Returns the rectangle to rebuild, or `None` when there was no switch.
    pub fn deactivate_switch(&self, lot_rect: CellRect) -> Option<CellRect> {
        let h = self.host.clone();
        self.with(|s| {
            let id = s.switch_id_at(lot_rect.x1, lot_rect.z1);
            if id == 0 {
                return None;
            }
            if let Some(sw) = s.switches.remove(id) {
                if sw.building != 0 {
                    h.release(sw.building);
                }
            }
            zero_grid_rect(s, lot_rect);
            Some(CellRect::new(
                lot_rect.x1 - 1,
                lot_rect.z1 - 1,
                lot_rect.x2 + 1,
                lot_rect.z2 + 1,
            ))
        })
    }

    /// The switch phase of `DoConnectionsChanged` over R1 (§4.7.4). For each
    /// cell of R1 with switch ID ≠ 0, applies the switch's rules to the cell
    /// and its in-city neighbours.
    pub(super) fn apply_switches(&self, s: &mut NetworkState, r1: CellRect) {
        for x in r1.x1..=r1.x2 {
            for z in r1.z1..=r1.z2 {
                let id = s.switch_id_at(x, z);
                if id == 0 {
                    continue;
                }
                // Looked up with insert-if-missing *[quirk]*.
                let sw = s.switches.entry(id, TransitSwitch::default).clone();
                self.apply_switch_cell(s, x, z, &sw);
            }
        }
    }

    fn apply_switch_cell(&self, s: &mut NetworkState, x: i32, z: i32, sw: &TransitSwitch) {
        // Step 1: for every travel type except Subway, clear conn and switch.
        if let Some(c) = s.conn_mut(x, z) {
            for t in 0..travel::COUNT {
                if t != travel::SUBWAY as usize {
                    c.conn[t] = 0;
                    c.switch[t] = 0xFFFF;
                }
            }
        }
        // Step 2: for each side whose neighbour is in the city.
        for d in 0..side::COUNT {
            let (nx, nz) = (x + super::DX[d], z + super::DZ[d]);
            if !s.in_city(nx, nz) {
                continue;
            }
            let same_id = s.switch_id_at(nx, nz) == s.switch_id_at(x, z) && s.switch_id_at(x, z) != 0;
            if same_id {
                // Same switch: for each kind-0 rule with side[d], connect all
                // sides to the rule's "to" type.
                for r in &sw.rules {
                    if r.kind == 0 && r.side[d] {
                        for k in 0..side::COUNT {
                            if r.side[k] {
                                set_connection(s, x, z, k, d, r.to);
                            }
                        }
                    }
                }
            } else {
                let ns = super::side_of((x, z), (nx, nz));
                for r in &sw.rules {
                    if !r.side[d] {
                        continue;
                    }
                    if r.kind == 1 {
                        self.entering_switch(s, x, z, nx, nz, d, ns, r);
                    } else {
                        self.leaving_switch(s, x, z, nx, nz, d, ns, r);
                    }
                }
            }
        }
    }

    /// A kind-1 rule (entering the switch), §4.7.4 step 2 "Other cell".
    fn entering_switch(&self, s: &mut NetworkState, x: i32, z: i32, nx: i32, nz: i32, d: usize, ns: usize, r: &Rule) {
        let to_net = super::NET_OF_TRAVEL[r.to as usize];
        // If N has a record of net(to): when from = to, connect all sides to
        // from; otherwise set nibble d of the switch cell's switch[from] to to.
        if s.network_info(to_net, nx, nz).is_some() {
            if r.from == r.to {
                for k in 0..side::COUNT {
                    set_connection(s, x, z, k, d, r.from);
                }
            } else {
                set_switch_nibble(s, x, z, r.from, d, r.to);
            }
        }
        // Then for each k ≠ ns such that N's conn[to] has an exit k from some
        // entry: SetConnection(N, ns, k, to).
        if let Some(c) = s.conn_at(nx, nz) {
            let t = r.to as usize;
            for k in 0..side::COUNT {
                if k == ns {
                    continue;
                }
                let any = (0..side::COUNT).any(|e| c.conn[t] & (1 << (k + 4 * e)) != 0);
                if any {
                    set_connection(s, nx, nz, ns, k, r.to);
                }
            }
        }
    }

    /// A kind-0 rule (leaving the switch), §4.7.4 step 2 "Other cell".
    fn leaving_switch(&self, s: &mut NetworkState, x: i32, z: i32, nx: i32, nz: i32, d: usize, ns: usize, r: &Rule) {
        let _ = (x, z, d);
        let from_net = super::NET_OF_TRAVEL[r.from as usize];
        // If N has a record of net(from): when from ≠ to, set nibble ns of N's
        // switch[from] to to; otherwise connect each entry j of N with an exit
        // in conn[from] to ns.
        if s.network_info(from_net, nx, nz).is_some() {
            if r.from != r.to {
                set_switch_nibble(s, nx, nz, r.from, ns, r.to);
            } else if let Some(c) = s.conn_at(nx, nz) {
                let t = r.from as usize;
                for j in 0..side::COUNT {
                    if c.conn[t] >> (4 * j) & 0xF != 0 {
                        set_connection(s, nx, nz, j, ns, r.from);
                    }
                }
            }
        }
        // If from = 1 (Car): for each k ≠ ns such that N's conn[1] has an exit
        // k, SetConnection(N, ns, k, 1).
        if r.from == travel::CAR as u16 {
            if let Some(c) = s.conn_at(nx, nz) {
                for k in 0..side::COUNT {
                    if k == ns {
                        continue;
                    }
                    if c.exits(travel::CAR, k) != 0 {
                        set_connection(s, nx, nz, ns, k, travel::CAR as u16);
                    }
                }
            }
        }
    }
}

/// `SetConnection(x, z, e, x', t)` (§4.3): `conn[t] |= 1 << (x' + 4e)` at
/// (x, z); and, if `net(t) = 1` and the neighbour through `x'` is in the city,
/// that neighbour's `priorCar[(x' + 2) mod 4] |= 0x1111 << e`.
pub fn set_connection(s: &mut NetworkState, x: i32, z: i32, e: usize, exit: usize, t: u16) {
    let t = t as usize;
    if t >= travel::COUNT {
        return;
    }
    if let Some(c) = s.conn_mut(x, z) {
        c.conn[t] |= 1 << (exit + 4 * e);
    }
    // Step 2: the car network only.
    if super::NET_OF_TRAVEL[t] == net::CAR {
        let (nx, nz) = (x + super::DX[exit], z + super::DZ[exit]);
        if s.in_city(nx, nz) {
            let pe = OPPOSITE_SIDE[exit];
            if let Some(c) = s.conn_mut(nx, nz) {
                c.prior_car[pe] |= 0x1111 << e;
            }
        }
    }
}

use super::OPPOSITE_SIDE;

/// Sets nibble `d` of `switch[from]` at (x, z) to `to` (§4.7.4).
fn set_switch_nibble(s: &mut NetworkState, x: i32, z: i32, from: u16, d: usize, to: u16) {
    let from = from as usize;
    if from >= travel::COUNT {
        return;
    }
    if let Some(c) = s.conn_mut(x, z) {
        let to = (to & 0xF) as u16;
        c.switch[from] = (c.switch[from] & !(0xF << (4 * d))) | (to << (4 * d));
    }
}

/// Writes the switch ID and mode-mask grids over a lot rectangle (§4.7.1
/// step 5).
fn write_grid_rect(s: &mut NetworkState, rect: CellRect, id: u32, mask: u8) {
    if let Some(g) = &s.switch_id {
        for x in rect.x1..=rect.x2 {
            for z in rect.z1..=rect.z2 {
                g.set(x, z, id);
            }
        }
    }
    if let Some(g) = &s.switch_mode {
        for x in rect.x1..=rect.x2 {
            for z in rect.z1..=rect.z2 {
                g.set(x, z, mask);
            }
        }
    }
}

/// Zeroes the switch ID and mode-mask grids over a lot rectangle (§4.7.5).
fn zero_grid_rect(s: &mut NetworkState, rect: CellRect) {
    write_grid_rect(s, rect, 0, 0);
}

fn f32_prop(h: &dyn NetworkHost, occ: u64, id: u32) -> f32 {
    h.occupant_property(occ, id).and_then(|v| v.items().first().map(|e| e.as_f64() as f32)).unwrap_or(0.0)
}

fn u32_prop(h: &dyn NetworkHost, occ: u64, id: u32) -> u32 {
    h.occupant_property(occ, id).and_then(|v| v.items().first().map(|e| e.as_i64() as u32)).unwrap_or(0)
}

fn bool_prop(h: &dyn NetworkHost, occ: u64, id: u32) -> bool {
    u32_prop(h, occ, id) != 0
}

/// The `q = r24(load / capacity)` ratio (§4.7.3 step 2), exposed for tests.
pub fn switch_ratio(load: f32, capacity: f32) -> f32 {
    x87::r24(f64::from(load) / f64::from(capacity)) as f32
}
