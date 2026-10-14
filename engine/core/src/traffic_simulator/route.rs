// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The compressed route reader, `ApplyRoute` (§4.5), `ConfirmRoute` (§4.3)
//! and the destination capacities (§4.2).

use crate::grid::CellRect;
use crate::traffic_network::{
    cell_key, side, travel, wormhole, NeighborLink, NetworkState, TrafficEdgeDensity, TransitSwitch, ANY_TRAVEL_TYPE,
    DX, DZ, NET_OF_TRAVEL,
};
use crate::variant::{mem, Elem, Variant};
use crate::x87::{r24, rhe};

use super::adapters::{capacity_element, neighbor_pool_holder, set_capacity_element};
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
/// Off-map roots (§4.5, TS13): the original never produces one (start nodes
/// are seeded on rings clamped to the map), and its reader takes the header
/// bytes zero-extended and decodes every step literally, with no off-map
/// adjustment. This reader does the same *[choice]*; a route whose root was
/// encoded as the inner cell would replay one cell off in both.
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
/// to `b`, `None` for any other move.
fn step_dir(a: (i32, i32), b: (i32, i32)) -> Option<usize> {
    match (b.0.wrapping_sub(a.0), b.1.wrapping_sub(a.1)) {
        (-1, 0) => Some(0),
        (0, -1) => Some(1),
        (1, 0) => Some(2),
        (0, 1) => Some(3),
        _ => None,
    }
}

/// The side from a cell towards a rectangle (§4.5 step 3, `0x00719B2D`,
/// TS24): `x < x0 → +x`, `x > x1 → −x`, else `z < z0 → +z`, else `−z`.
/// Always defined, however far the rectangle is; a cell inside it gives
/// `−z`.
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

/// The exit side of a cell towards a following jump to a cell `(dx, dz)`
/// away (§4.3, `0x006D65E0`–`0x006D666A`, TS14): when both differences are
/// non-zero the one smaller in absolute value is set to 0 (`dx` when they
/// are equal); then `|dx| ≥ 2` gives `±x` by its sign, `dx = ±1` gives none,
/// and `dx = 0` gives `−z` for `dz ≤ −2` and `+z` for every other `dz`
/// (−1 and 0 included) *[quirk]*.
fn jump_exit(dx: i32, dz: i32) -> Option<usize> {
    let (mut dx, mut dz) = (dx, dz);
    if dx != 0 && dz != 0 {
        if dx.unsigned_abs() <= dz.unsigned_abs() {
            dx = 0;
        } else {
            dz = 0;
        }
    }
    if dx >= 2 {
        Some(2)
    } else if dx <= -2 {
        Some(0)
    } else if dx != 0 {
        None
    } else if dz <= -2 {
        Some(1)
    } else {
        Some(3)
    }
}

/// One record of the route reader (`0x006D6510`, §4.3, TS14): a visited
/// cell in route order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RouteStep {
    pub x: i32,
    pub z: i32,
    /// The type of the header or token that brought the route here.
    pub t: u8,
    /// The type of the following run or jump (`t` at the last cell).
    pub t_next: u8,
    /// The side the route entered through (`None` at the first cell, and
    /// around a jump to an x-neighbour).
    pub entry: Option<usize>,
    /// The side the route leaves through (`None` at the last cell and
    /// before a jump to an x-neighbour).
    pub exit: Option<usize>,
    /// The following record is a jump with the `0x80` flag (pathfinder.md
    /// §4.15): the route leaves this cell by ferry or from off the map.
    pub ferry_jump_next: bool,
}

/// The route reader's records (§4.3): the exit is the direction of the
/// following unit step, or for a following jump [`jump_exit`]; a jump to an
/// x-neighbour (exit none) also clears the cell's own entry. The entry of
/// the next cell is the side facing back, `(exit + 2) mod 4`; after a jump
/// whose exit is none the previous cell's entry carries over, which that
/// jump has just cleared *[question: §4.3 gives both rules; the carried
/// entry is therefore none]*.
pub fn read_route(bytes: &[u8]) -> Vec<RouteStep> {
    let cells = decode_route(bytes);
    let mut out = Vec::with_capacity(cells.len());
    let mut entry = None;
    for (i, c) in cells.iter().enumerate() {
        let next = cells.get(i + 1);
        let exit = next.and_then(|n| {
            if n.jump {
                jump_exit(n.x.wrapping_sub(c.x), n.z.wrapping_sub(c.z))
            } else {
                step_dir((c.x, c.z), (n.x, n.z))
            }
        });
        let own_entry = if next.is_some_and(|n| n.jump) && exit.is_none() {
            None
        } else {
            entry
        };
        out.push(RouteStep {
            x: c.x,
            z: c.z,
            t: c.t,
            t_next: next.map_or(c.t, |n| n.t),
            entry: own_entry,
            exit,
            ferry_jump_next: next.is_some_and(|n| n.jump && n.ferry),
        });
        entry = match exit {
            Some(x) => Some((x + 2) % 4),
            None => own_entry,
        };
    }
    out
}

/// `ConfirmRoute(route)` (`0x0070E320`, §4.3, TS14) over the reader's
/// records, `strike` = strike state 1, read once:
///
/// 1. An empty route is false.
/// 2. The first cell (in the city, with an exit): `t = t'` needs some entry
///    side with bit `x + 4e'` of `conn[t]`; `t ≠ t'` needs nibble `x` of
///    `switch[t]` = `t'`. No strike or wormhole test.
/// 3. Every later cell in the city with an exit (the last cell has none;
///    off-map cells are skipped): `t = t'` passes on bit `x + 4e` of
///    `conn[t]` (never with no entry), else on the tunnel wormhole bit, else
///    on the ferry bit with no strike; `t ≠ t'` fails during a strike, else
///    needs nibble `x` of `switch[t]` = `t'`.
///
/// A travel type above 8 fails *[choice]*.
pub fn confirm_route(net: &NetworkState, strike_state: u8, route: &[u8]) -> bool {
    let steps = read_route(route);
    if steps.is_empty() {
        return false;
    }
    let strike = strike_state == 1;
    for (i, s) in steps.iter().enumerate() {
        if !net.in_city(s.x, s.z) {
            continue;
        }
        let Some(x) = s.exit else { continue };
        let t = usize::from(s.t);
        if t >= travel::COUNT {
            return false;
        }
        let c = net.conn_at(s.x, s.z).cloned().unwrap_or_default();
        let same = s.t == s.t_next;
        let ok = if i == 0 {
            if same {
                (0..side::COUNT).any(|e| c.connected(t, e, x))
            } else {
                c.switch_to(t, x) == u32::from(s.t_next)
            }
        } else if same {
            s.entry.is_some_and(|e| c.connected(t, e, x))
                || c.wormhole & wormhole::TUNNEL != 0
                || (c.wormhole & wormhole::FERRY != 0 && !strike)
        } else {
            !strike && c.switch_to(t, x) == u32::from(s.t_next)
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
    /// neighbour pool of the off-map cell (`GetNeighborPoolPropertyHolder(t,
    /// x, z)`, `t` = −1 for CapacityStillAvailable); with the lot when there
    /// is one.
    pub(crate) fn destination_holder(&mut self, x: i32, z: i32, t: u32) -> (Option<u64>, Option<u64>) {
        if self.cx.s.in_city(x, z) {
            let lot = self.h.lot_at(x, z, false);
            let holder = lot.map(|l| self.h.lot_holder(l)).filter(|&h| h != 0);
            (holder, lot)
        } else {
            (neighbor_pool_holder(self.cx.s, self.pools, t, x, z), None)
        }
    }

    /// `CapacityStillAvailable(trip)` (`0x007136B0`, §4.2): `max(k) −
    /// arrivedThisCycle(k) ≥ count` (unsigned), `k` = the trip type; with no
    /// holder `0 ≥ count`.
    pub(crate) fn capacity_still_available(&mut self, trip: &TripData) -> bool {
        let (holder, _) = self.destination_holder(i32::from(trip.dest_x), i32::from(trip.dest_z), ANY_TRAVEL_TYPE);
        match holder {
            Some(holder) => {
                let k = usize::from(trip.trip_type);
                let max = capacity_element(&self.holders(), holder, k);
                let arrived = capacity_element(&self.holders(), holder, 4 + k);
                max.wrapping_sub(arrived) >= trip.count
            }
            None => trip.count == 0,
        }
    }

    /// `ApplyTrafficToDestination(count, x, z, lastTravelType, type)`
    /// (`0x00714740`, §4.2, TS18). An off-map cell asks the pool for
    /// `lastTravelType`. No holder: nothing *[choice]* (an off-map cell then
    /// has no connection for that travel type either). Off the map, the
    /// connection that `FindNeighborConnection(lastTravelType, x, z)` finds
    /// gets, on its **own** holder, `count` added to element
    /// `lastTravelType` of the u32[9] `0xAA9EA81A + type` (type < 3 only)
    /// and the pool's new arrived total in element `4 + type` of its
    /// `0x8A0B4100`. When the arrivals reach the maximum (read before the
    /// store), the destination bit is cleared over the lot's rect (off the
    /// map, at the cell) and `freeDest[type]` is decremented (wrapping, on
    /// every such call *[quirk]*).
    pub(crate) fn apply_traffic_to_destination(&mut self, count: u32, x: i32, z: i32, last_t: u8, trip_type: u8) {
        let (holder, lot) = self.destination_holder(x, z, u32::from(last_t));
        let Some(holder) = holder else { return };
        let k = usize::from(trip_type);
        let conn = if lot.is_none() && !self.cx.s.in_city(x, z) {
            let h = self.h;
            self.cx
                .s
                .find_neighbor_connection(u32::from(last_t), x, z, |c| h.find_connection(c, x, z))
                .map(|c| h.connection_holder(c))
                .filter(|&c| c != 0)
        } else {
            None
        };
        if let (Some(ch), true) = (conn, trip_type < 3) {
            let id = prop::POOL_ARRIVALS.wrapping_add(u32::from(trip_type));
            let mut v: Vec<u32> = self
                .h
                .holder_property(ch, id)
                .map(|v| v.items().iter().map(|e| e.as_i64() as u32).collect())
                .unwrap_or_default();
            if v.len() < 9 {
                v.resize(9, 0);
            }
            if let Some(e) = v.get_mut(usize::from(last_t)) {
                *e = e.wrapping_add(count);
            }
            self.h.set_holder_property(
                ch,
                id,
                Variant::array(mem::UINT32, v.into_iter().map(Elem::U32).collect()),
            );
        }
        let holders = self.holders();
        let max = capacity_element(&holders, holder, k);
        let arrived = capacity_element(&holders, holder, 4 + k).wrapping_add(count);
        set_capacity_element(&holders, holder, 4 + k, arrived);
        if let Some(ch) = conn {
            set_capacity_element(&holders, ch, 4 + k, arrived);
        }
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
        let steps = read_route(route);
        let Some(last) = steps.len().checked_sub(1) else {
            return;
        };
        let n = r24(f64::from(count)) as f32;
        let nc = if self.h.is_ordinance_on(CARPOOL_ORDINANCE) {
            let p = r24(f64::from(n) * f64::from(f32::from_bits(crate::pathfinder::CARPOOL_FACTOR_BITS)));
            r24(p.ceil()) as f32
        } else {
            n
        };
        let q_of = |t: u8| if t == 1 { nc } else { n };
        let fare = f64::from(self.st.tuning.ferry_fare);
        // The start rect R: the source rect, or in mode 1 the destination's
        // (the lot's bounding rect, else the cell) (TS24).
        let r = if mode == mode::RETURN {
            let lot = if self.cx.s.in_city(dest.0, dest.1) {
                self.h.lot_at(dest.0, dest.1, false)
            } else {
                None
            };
            lot.map_or(CellRect::new(dest.0, dest.1, dest.0, dest.1), |l| {
                self.h.lot_bounding_rect(l)
            })
        } else {
            source
        };
        let mut income = 0.0f32;
        let mut fees = 0.0f32;
        // The ferry leg from a neighbour at the first cell (TS16,
        // `0x00719A4E`–`0x00719B29`): a store into `I`.
        {
            let f = steps[0];
            let beside_x = f.x == r.x1 - 1 || f.x == r.x2 + 1;
            let beside_z = f.z == r.z1 - 1 || f.z == r.z2 + 1;
            if beside_x != beside_z {
                let q = q_of(f.t);
                if self.first_cell_ferry_leg((f.x, f.z), (r.x1, r.z1), mode, rhe(q)) {
                    income = r24(f64::from(q) * fare) as f32;
                }
            }
        }
        // The last switch ID seen: follows every in-city cell (TS25).
        let mut last_switch = 0u32;
        for (i, s) in steps.iter().enumerate() {
            let t = usize::from(s.t);
            if t >= travel::COUNT {
                // *[choice]* a damaged type byte adds nothing.
                continue;
            }
            let q = q_of(s.t);
            let qi = rhe(q);
            if self.cx.s.in_city(s.x, s.z) {
                // 1. Volume.
                let netw = NET_OF_TRAVEL[t];
                if self.cx.s.network_info(netw, s.x, s.z).is_some() && self.st.tuning.generates_traffic[t] {
                    if let Some(g) = self.st.volume_grid(netw) {
                        let mut vol = g.get(s.x, s.z);
                        vol = r24(f64::from(vol) + f64::from(q)) as f32;
                        if mode == mode::BOTH {
                            vol = r24(f64::from(vol) + f64::from(q)) as f32;
                        }
                        g.set(s.x, s.z, vol);
                    }
                }
                // 2. Transit switch (TS25): not when this and the next
                // record's type are both subway (the last record is its
                // own next).
                let id = self.cx.s.switch_id_at(s.x, s.z);
                if id != last_switch {
                    let next_t = steps.get(i + 1).map_or(s.t, |nx| nx.t);
                    if id != 0 && !(s.t == 6 && next_t == 6) {
                        let sw = self.cx.s.switches.entry(id, TransitSwitch::default);
                        sw.load = r24(f64::from(sw.load) + f64::from(q)) as f32;
                        // The toll, `0x0BFC0A4C` read as f32 (TS15).
                        let toll = f32::from_bits(sw.simulator_value);
                        let fee = r24(f64::from(q) * f64::from(toll));
                        fees = r24(f64::from(fees) + fee) as f32;
                    }
                    last_switch = id;
                }
                // 3. Edge density (TS24): the reader's sides, the first
                // entry towards R, the last exit towards the destination
                // cell (modes 0, 2) or the source rect (mode 1).
                let e = if i == 0 { Some(rect_dir(s.x, s.z, r)) } else { s.entry };
                let o = if i < last {
                    s.exit
                } else if mode == mode::RETURN {
                    Some(rect_dir(s.x, s.z, source))
                } else {
                    let dx = (dest.0.wrapping_sub(s.x) as i16) as i32;
                    let dz = (dest.1.wrapping_sub(s.z) as i16) as i32;
                    step_dir((0, 0), (dx, dz))
                };
                match (e, o) {
                    (Some(e), Some(o)) => {
                        let key = NetworkState::edge_key(s.x, s.z, u32::from(s.t), mode == mode::RETURN);
                        add_density(self.cx.s, key, 4 * e + o, qi);
                        if mode == mode::BOTH {
                            let key = NetworkState::edge_key(s.x, s.z, u32::from(s.t), true);
                            add_density(self.cx.s, key, 4 * o + e, qi);
                        }
                    }
                    // An undefined side before the last cell ends ApplyRoute
                    // (`0x00719DC3`): `I` and the fees are lost *[quirk]*.
                    _ if i < last => return,
                    _ => {}
                }
                // 4. Ferry legs (TS16).
                if i < last && s.ferry_jump_next {
                    let nx = steps[i + 1];
                    if self.ferry_leg((s.x, s.z), (nx.x, nx.z), mode, qi) {
                        let f = r24(f64::from(q) * fare);
                        income = r24(f64::from(income) + f) as f32;
                    }
                }
            }
            // 5. Income, off-map cells included.
            let inc = r24(f64::from(q) * f64::from(self.st.tuning.income_per_tile[t]));
            income = r24(f64::from(income) + inc) as f32;
        }
        if mode != mode::RETURN {
            self.apply_traffic_to_destination(count, dest.0, dest.1, steps[last].t, trip_type);
        }
        let acc = r24(self.st.income_acc.get() + f64::from(income));
        self.st.income_acc.set(acc);
        self.st.fee_acc = r24(self.st.fee_acc + f64::from(fees));
    }

    /// §4.5 step 4 (TS16) at the departure cell `from` of a terminal `T`
    /// whose next record is the ferry jump to `to`: `T`'s riders `+= Qi`
    /// (twice in mode 2); then, when `to` is a terminal `T2`'s arrival cell,
    /// `T`'s link keyed by `T2`'s departure cell gets current A (mode 0) or B
    /// (mode 1) `+= Qi`, and in mode 2 current A, with `T2`'s link keyed by
    /// `T`'s departure cell getting current B; otherwise `T`'s neighbour link
    /// whose off-map or edge cell is `to` gets A, B or both. Missing links
    /// are skipped; the u16 counters wrap. Returns whether `from` is a
    /// departure cell (the fare is then charged).
    fn ferry_leg(&mut self, from: (i32, i32), to: (i32, i32), mode: u8, qi: i32) -> bool {
        let s = &mut *self.cx.s;
        let Some(ti) = s.terminal_by_departure(cell_key(from.0, from.1)) else {
            return false;
        };
        let q = qi as u16;
        let Some(t) = s.ferries.get_mut(ti) else {
            return false;
        };
        t.riders_current = t.riders_current.wrapping_add(q);
        if mode == mode::BOTH {
            t.riders_current = t.riders_current.wrapping_add(q);
        }
        let t_key = t.departure_key();
        let t2 = s
            .ferry_arrivals
            .get(u32::from(cell_key(to.0, to.1)))
            .map(|&p| p as usize)
            .filter(|&p| p < s.ferries.len());
        if let Some(t2) = t2 {
            let t2_key = s.ferries[t2].departure_key();
            if let Some(l) = s.ferries[ti].links.iter_mut().find(|l| l.to_key == t2_key) {
                let c = if mode == mode::RETURN { 3 } else { 1 };
                l.counters[c] = l.counters[c].wrapping_add(q);
            }
            if mode == mode::BOTH {
                if let Some(l) = s.ferries[t2].links.iter_mut().find(|l| l.to_key == t_key) {
                    l.counters[3] = l.counters[3].wrapping_add(q);
                }
            }
        } else if let Some(l) = s.ferries[ti]
            .neighbour_links
            .iter_mut()
            .find(|l| neighbour_link_at(l, to))
        {
            bump_counters(&mut l.counters, mode, q);
        }
        true
    }

    /// The first-cell ferry leg from a neighbour (§4.5, TS16): when `first`
    /// is a terminal's arrival cell and that terminal has a neighbour link
    /// whose off-map or edge cell is `corner` (R's first corner), the link's
    /// counters are bumped by mode. Returns whether a link was found.
    fn first_cell_ferry_leg(&mut self, first: (i32, i32), corner: (i32, i32), mode: u8, qi: i32) -> bool {
        let s = &mut *self.cx.s;
        let Some(ti) = s
            .ferry_arrivals
            .get(u32::from(cell_key(first.0, first.1)))
            .map(|&p| p as usize)
        else {
            return false;
        };
        let Some(l) = s
            .ferries
            .get_mut(ti)
            .and_then(|t| t.neighbour_links.iter_mut().find(|l| neighbour_link_at(l, corner)))
        else {
            return false;
        };
        bump_counters(&mut l.counters, mode, qi as u16);
        true
    }
}

/// Whether a neighbour link's off-map cell or in-city edge cell is `c`.
fn neighbour_link_at(l: &NeighborLink, c: (i32, i32)) -> bool {
    (i32::from(l.off_x), i32::from(l.off_z)) == c || (i32::from(l.edge_x), i32::from(l.edge_z)) == c
}

/// A link's counters by mode: current A (mode 0), current B (mode 1) or
/// both (mode 2); u16, wrapping.
fn bump_counters(c: &mut [u16; 4], mode: u8, q: u16) {
    if mode != mode::RETURN {
        c[1] = c[1].wrapping_add(q);
    }
    if mode != mode::OUTBOUND {
        c[3] = c[3].wrapping_add(q);
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
