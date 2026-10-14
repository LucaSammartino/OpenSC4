// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The compressed route reader, `ApplyRoute` (§4.5), `ConfirmRoute` (§4.3)
//! and the destination capacities (§4.2).

use crate::grid::CellRect;
use crate::traffic_network::{cell_key, wormhole, NetworkState, TrafficEdgeDensity, DX, DZ, NET_OF_TRAVEL};
use crate::variant::{mem, Elem, Variant};
use crate::x87::{r24, rhe};

use super::adapters::{capacity_element, set_capacity_element};
use super::cycle::Run;
use super::{prop, TripData, CARPOOL_ORDINANCE};

/// One cell of a decoded compressed route (pathfinder.md §4.15).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RouteCell {
    pub x: i32,
    pub z: i32,
    /// The travel type (`byte & 0x3F`).
    pub t: u8,
    /// Reached by a jump record (not a unit step).
    pub jump: bool,
    /// The jump record's `0x80` flag (from an off-map or ferry cell).
    pub ferry: bool,
}

/// Reads a compressed route (pathfinder.md §4.15; the original reader
/// `0x006D64B0` / `0x006D6510` is not specified beyond the format): the
/// header cell, then each run's unit steps (codes 0 −x, 1 −z, 2 +x, 3 +z,
/// four per byte, least significant first) and each jump's cell. A
/// truncated record ends the route *[choice]*. Coordinates are i32: a unit
/// step may leave the map (an exit node), a jump's bytes are unsigned.
///
/// *[question]* The encoder writes an off-map root as its connection's
/// in-city cell while the unit steps stay relative to the off-map cell, so
/// such a route decodes shifted by one cell; this reader decodes the bytes
/// literally.
pub fn decode_route(bytes: &[u8]) -> Vec<RouteCell> {
    let mut out = Vec::new();
    if bytes.len() < 3 {
        return out;
    }
    let (mut x, mut z) = (i32::from(bytes[1]), i32::from(bytes[2]));
    out.push(RouteCell {
        x,
        z,
        t: bytes[0] & 0x3F,
        jump: false,
        ferry: false,
    });
    let mut i = 3;
    while i < bytes.len() {
        let b = bytes[i];
        let t = b & 0x3F;
        if b & 0x40 != 0 {
            let Some(&count) = bytes.get(i + 1) else { break };
            let packed = usize::from(count).div_ceil(4);
            let Some(codes) = bytes.get(i + 2..i + 2 + packed) else {
                break;
            };
            for j in 0..usize::from(count) {
                let code = usize::from((codes[j / 4] >> (2 * (j % 4))) & 3);
                x += DX[code];
                z += DZ[code];
                out.push(RouteCell {
                    x,
                    z,
                    t,
                    jump: false,
                    ferry: false,
                });
            }
            i += 2 + packed;
        } else {
            let (Some(&bx), Some(&bz)) = (bytes.get(i + 1), bytes.get(i + 2)) else {
                break;
            };
            x = i32::from(bx);
            z = i32::from(bz);
            out.push(RouteCell {
                x,
                z,
                t,
                jump: true,
                ferry: b & 0x80 != 0,
            });
            i += 3;
        }
    }
    out
}

/// The direction code (0 −x, 1 −z, 2 +x, 3 +z) of a one-cell step from `a`
/// to `b`, `None` for any other move (§4.5 step 3).
fn step_dir(a: (i32, i32), b: (i32, i32)) -> Option<usize> {
    match (b.0.wrapping_sub(a.0), b.1.wrapping_sub(a.1)) {
        (-1, 0) => Some(0),
        (0, -1) => Some(1),
        (1, 0) => Some(2),
        (0, 1) => Some(3),
        _ => None,
    }
}

/// The direction from a cell towards a rectangle (§4.5 step 3): `x < x0 →
/// +x`, `x > x1 → −x`, else `z < z0 → +z`, else `−z`. Always defined
/// *[choice: also for a rectangle that is not adjacent]*.
fn rect_dir(x: i32, z: i32, r: CellRect) -> usize {
    if x < r.x1 {
        2
    } else if x > r.x2 {
        0
    } else if z < r.z1 {
        3
    } else {
        1
    }
}

/// The mass-transit travel types a strike blocks in `ConfirmRoute`
/// *[choice: bus, passenger train, subway, el-train, monorail]*.
const STRIKE_TYPES: [u8; 5] = [2, 3, 6, 7, 8];

/// `ConfirmRoute(route)` (`0x0070E320`, §4.3): whether every step of the
/// route still exists and is not blocked by the strike. *[question: the
/// checks are not specified]* FreeCity4: during a strike (state 1) a route
/// using a mass-transit type fails; each unit step that leaves an in-city
/// cell needs, for the same type, an exit through that side
/// (`conn[t]`, any entry side), and for a type change, the cell's switch to
/// the new type through that side. Jumps (tunnels, ferries, type changes in
/// place) and steps from off-map cells are not checked.
pub fn confirm_route(net: &NetworkState, strike_state: u8, route: &[u8]) -> bool {
    let cells = decode_route(route);
    if strike_state == 1 && cells.iter().any(|c| STRIKE_TYPES.contains(&c.t)) {
        return false;
    }
    for w in cells.windows(2) {
        let (a, b) = (w[0], w[1]);
        if b.jump || !net.in_city(a.x, a.z) {
            continue;
        }
        let Some(d) = step_dir((a.x, a.z), (b.x, b.z)) else {
            continue;
        };
        let Some(c) = net.conn_at(a.x, a.z) else {
            return false;
        };
        let ta = usize::from(a.t);
        if ta >= 9 {
            return false;
        }
        let ok = if a.t == b.t {
            c.has_exit_through(ta, d)
        } else {
            c.switch_to(ta, d) == u32::from(b.t)
        };
        if !ok {
            return false;
        }
    }
    true
}

/// `ApplyRoute`'s modes (§4.5).
pub mod mode {
    pub const OUTBOUND: u8 = 0;
    pub const RETURN: u8 = 1;
    pub const BOTH: u8 = 2;
}

impl Run<'_, '_> {
    /// The destination holder (§4.2): the lot at the cell (in city) or the
    /// neighbour pool of the off-map cell (`GetNeighborPoolPropertyHolder(−1,
    /// x, z)`); with the lot when there is one.
    pub(crate) fn destination_holder(&self, x: i32, z: i32) -> (Option<u64>, Option<u64>) {
        if self.cx.s.in_city(x, z) {
            let lot = self.h.lot_at(x, z, false);
            let holder = lot.map(|l| self.h.lot_holder(l)).filter(|&h| h != 0);
            (holder, lot)
        } else {
            (self.h.neighbor_pool_property_holder(u32::MAX, x, z), None)
        }
    }

    /// `CapacityStillAvailable(trip)` (`0x007136B0`, §4.2): `max(k) −
    /// arrivedThisCycle(k) ≥ count` (unsigned), `k` = the trip type; with no
    /// holder `0 ≥ count`.
    pub(crate) fn capacity_still_available(&self, trip: &TripData) -> bool {
        let (holder, _) = self.destination_holder(i32::from(trip.dest_x), i32::from(trip.dest_z));
        match holder {
            Some(holder) => {
                let k = usize::from(trip.trip_type);
                let max = capacity_element(self.h, holder, k);
                let arrived = capacity_element(self.h, holder, 4 + k);
                max.wrapping_sub(arrived) >= trip.count
            }
            None => trip.count == 0,
        }
    }

    /// `ApplyTrafficToDestination(count, x, z, lastTravelType, type)`
    /// (`0x00714740`, §4.2). No holder: nothing *[choice]*. A neighbour pool
    /// also gets `count` added to element `lastTravelType` of its u32[9]
    /// property `0xAA9EA81A + type`. When the arrivals reach the maximum, the
    /// destination bit is cleared over the lot's rect (for a pool, at the
    /// cell *[choice]*) and `freeDest[type]` is decremented (wrapping, on
    /// every such call *[quirk]*).
    pub(crate) fn apply_traffic_to_destination(&mut self, count: u32, x: i32, z: i32, last_t: u8, trip_type: u8) {
        let (holder, lot) = self.destination_holder(x, z);
        let Some(holder) = holder else { return };
        let k = usize::from(trip_type);
        if lot.is_none() {
            let id = prop::POOL_ARRIVALS.wrapping_add(u32::from(trip_type));
            let mut v: Vec<u32> = self
                .h
                .holder_property(holder, id)
                .map(|v| v.items().iter().map(|e| e.as_i64() as u32).collect())
                .unwrap_or_default();
            if v.len() < 9 {
                v.resize(9, 0);
            }
            if let Some(e) = v.get_mut(usize::from(last_t)) {
                *e = e.wrapping_add(count);
            }
            self.h
                .set_holder_property(holder, id, Variant::array(mem::UINT32, v.into_iter().map(Elem::U32).collect()));
        }
        let arrived = capacity_element(self.h, holder, 4 + k).wrapping_add(count);
        set_capacity_element(self.h, holder, 4 + k, arrived);
        let max = capacity_element(self.h, holder, k);
        if arrived >= max {
            let bit = 1u8.checked_shl(u32::from(trip_type)).unwrap_or(0);
            let rect = match lot {
                Some(l) => self.h.lot_bounding_rect(l),
                None => CellRect::new(x, z, x, z),
            };
            for cx in rect.x1..=rect.x2 {
                for cz in rect.z1..=rect.z2 {
                    let b = self.st.dest_byte(cx, cz);
                    self.st.set_dest_byte(cx, cz, b & !bit);
                }
            }
            if k < 4 {
                self.st.set_free(k, self.st.free(k).wrapping_sub(1));
            }
        }
    }

    /// `ApplyRoute(route, count, sourceRect, destX, destZ, mode, type)`
    /// (`0x00719910`, §4.5) *[compat]*.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_route(
        &mut self,
        route: &[u8],
        count: u32,
        source: CellRect,
        dest: (i32, i32),
        mode: u8,
        trip_type: u8,
    ) {
        let cells = decode_route(route);
        if cells.is_empty() {
            return;
        }
        let n = r24(f64::from(count)) as f32;
        let nc = if self.h.is_ordinance_on(CARPOOL_ORDINANCE) {
            let p = r24(f64::from(n) * f64::from(f32::from_bits(crate::pathfinder::CARPOOL_FACTOR_BITS)));
            r24(p.ceil()) as f32
        } else {
            n
        };
        let mut income = 0.0f32;
        let mut fees = 0.0f32;
        // The last switch ID seen on this route *[choice: updated at every
        // in-city cell, so leaving and re-entering a station counts again]*.
        let mut last_switch = 0u32;
        for (i, c) in cells.iter().enumerate() {
            let t = usize::from(c.t);
            let q = if t == 1 { nc } else { n };
            let qi = rhe(q);
            if !self.cx.s.in_city(c.x, c.z) || t >= 9 {
                continue;
            }
            // 1. Volume.
            let netw = NET_OF_TRAVEL[t];
            if self.cx.s.network_info(netw, c.x, c.z).is_some() && self.st.tuning.generates_traffic[t] {
                if let Some(g) = self.st.volume_grid(netw) {
                    let mut vol = g.get(c.x, c.z);
                    vol = r24(f64::from(vol) + f64::from(q)) as f32;
                    if mode == mode::BOTH {
                        vol = r24(f64::from(vol) + f64::from(q)) as f32;
                    }
                    g.set(c.x, c.z, vol);
                }
            }
            // 2. Transit switch.
            let id = self.cx.s.switch_id_at(c.x, c.z);
            let both_subway = t == 6 && i > 0 && cells[i - 1].t == 6;
            if id != 0 && id != last_switch && !both_subway {
                if let Some(sw) = self.cx.s.switches.get_mut(id) {
                    sw.load = r24(f64::from(sw.load) + f64::from(q)) as f32;
                    // switchFee: the switch's 0x0BFC0A4C value read as f32
                    // *[question]*.
                    let fee = f32::from_bits(sw.simulator_value);
                    let f = r24(f64::from(q) * f64::from(fee));
                    fees = r24(f64::from(fees) + f) as f32;
                }
            }
            last_switch = id;
            // 3. Edge density.
            let here = (c.x, c.z);
            let e = if i == 0 {
                Some(rect_dir(c.x, c.z, source))
            } else {
                step_dir(here, (cells[i - 1].x, cells[i - 1].z))
            };
            let o = if let Some(next) = cells.get(i + 1) {
                step_dir(here, (next.x, next.z))
            } else if mode == mode::RETURN {
                Some(rect_dir(c.x, c.z, source))
            } else {
                step_dir(here, dest)
            };
            if let (Some(e), Some(o)) = (e, o) {
                let key = NetworkState::edge_key(c.x, c.z, u32::from(c.t), mode == mode::RETURN);
                add_density(self.cx.s, key, 4 * e + o, qi);
                if mode == mode::BOTH {
                    let key = NetworkState::edge_key(c.x, c.z, u32::from(c.t), true);
                    add_density(self.cx.s, key, 4 * o + e, qi);
                }
            }
            // 4. Ferry legs.
            if c.jump && i > 0 {
                let p = cells[i - 1];
                if self.cx.s.in_city(p.x, p.z) && self.cx.s.wormhole_at(p.x, p.z) & wormhole::FERRY != 0 {
                    if self.apply_ferry_leg((p.x, p.z), here, mode, qi) {
                        let fare = r24(f64::from(q) * f64::from(self.st.tuning.ferry_fare));
                        income = r24(f64::from(income) + fare) as f32;
                    }
                }
            }
            // 5. Income.
            let inc = r24(f64::from(q) * f64::from(self.st.tuning.income_per_tile[t]));
            income = r24(f64::from(income) + inc) as f32;
        }
        if mode != mode::RETURN {
            let last_t = cells.last().map_or(0, |c| c.t);
            self.apply_traffic_to_destination(count, dest.0, dest.1, last_t, trip_type);
        }
        let acc = r24(self.st.income_acc.get() + f64::from(income));
        self.st.income_acc.set(acc);
        self.st.fee_acc = r24(self.st.fee_acc + f64::from(fees));
    }

    /// A ferry step from the departure cell `from` to `to` (§4.5 step 4
    /// *[inferred]*): the departure terminal's link whose arrival cell is
    /// `to` (else its neighbour link whose edge cell is `to`) gets `round(Q)`
    /// added to counter 1 (mode 0), counter 2 (mode 1) or both (mode 2)
    /// *[question: counter meaning]*. Returns whether a link was found.
    fn apply_ferry_leg(&mut self, from: (i32, i32), to: (i32, i32), mode: u8, qi: i32) -> bool {
        let Some(ti) = self.cx.s.terminal_by_departure(cell_key(from.0, from.1)) else {
            return false;
        };
        let Some(term) = self.cx.s.ferries.get_mut(ti) else {
            return false;
        };
        let add = |c: &mut [u16; 4]| {
            let q = qi as u16;
            if mode != mode::RETURN {
                c[1] = c[1].wrapping_add(q);
            }
            if mode != mode::OUTBOUND {
                c[3] = c[3].wrapping_add(q);
            }
        };
        let (tx, tz) = (to.0 as u16, to.1 as u16);
        if let Some(l) = term.links.iter_mut().find(|l| (l.arrival_x, l.arrival_z) == (tx, tz)) {
            add(&mut l.counters);
            return true;
        }
        if let Some(l) = term.neighbour_links.iter_mut().find(|l| (l.edge_x, l.edge_z) == (tx, tz)) {
            add(&mut l.counters);
            return true;
        }
        false
    }
}

/// `current[index] := min(0xFFFF, current + Qi)` on the record with `key`,
/// created when missing *[choice]*.
fn add_density(s: &mut NetworkState, key: u32, index: usize, qi: i32) {
    let rec = s.edge_density.entry(key, TrafficEdgeDensity::default);
    if let Some(c) = rec.current.get_mut(index) {
        let v = i64::from(*c) + i64::from(qi);
        *c = v.clamp(0, 0xFFFF) as u16;
    }
}
