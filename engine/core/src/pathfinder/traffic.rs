// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `ApplyTraffic(style)` (slot 29, §4.17).

use crate::traffic_network::{travel, NetworkModel, NetworkState, TrafficEdgeDensity, TransitSwitch};
use crate::x87::{r24, rhe};

use super::{edge_to, PathCx, Pathfinder, PathfinderHost, CARPOOL_FACTOR_BITS, CARPOOL_ORDINANCE, NETWORK_OF_TRAVEL};

/// Adds `qi` to counter `idx` of the edge-density record `key` (created if
/// missing), saturating at `0xFFFF`. The counters are the record's `current`
/// array, its second u16[16] at `+0x22`; `last` and `usage` are never written
/// here (PF2, §4.17 step 3).
fn add_edge_density(net: &mut NetworkState, key: u32, idx: usize, qi: i32) {
    let rec = net.edge_density.entry(key, TrafficEdgeDensity::default);
    let v = i64::from(rec.current[idx]) + i64::from(qi);
    rec.current[idx] = v.min(0xFFFF) as u16;
}

impl Pathfinder {
    /// `ApplyTraffic(style)` (§4.17): adds the trips of the result to the
    /// simulator; style 0 = outbound, 1 = return, 2 = both directions.
    /// Nothing happens without a result node or with traffic arrived 0.
    pub fn apply_traffic(&mut self, cx: &mut PathCx<'_>, style: u32) {
        let Some(res) = self.result else {
            return;
        };
        if self.arrived == 0 {
            return;
        }
        let host = cx.host;
        let tables = host.travel_tables();
        // N: the u32 loaded exactly, stored as float32; the carpool count.
        let n = self.arrived as f32;
        let nc = if host.is_ordinance_on(CARPOOL_ORDINANCE) {
            let p = r24(f64::from(n) * f64::from(f32::from_bits(CARPOOL_FACTOR_BITS)));
            p.ceil() as f32
        } else {
            n
        };
        // prev: the end cell, then the previous node's cell, as unsigned
        // bytes.
        let mut prev = (self.end_cell.0 as u8, self.end_cell.1 as u8);
        let mut income = 0.0f32;
        let s = self.start_rect;
        for id in self.walk_to_root(res) {
            let node = *self.store.get(id);
            let t = usize::from(node.t);
            let (x, z) = (node.x, node.z);
            let q = if t == travel::CAR { nc } else { n };
            let qi = rhe(q);
            if self.in_city(x, z) {
                let pred = node.parent.map(|p| *self.store.get(p));
                // 1. Transit-switch usage on a type change.
                if pred.is_some_and(|p| p.t != node.t) {
                    let id = cx.net.switch_id_at(x, z);
                    if id != 0 {
                        let a = if style == 2 {
                            r24(f64::from(q) + f64::from(q)) as f32
                        } else {
                            q
                        };
                        let sw = cx.net.switches.entry(id, TransitSwitch::default);
                        sw.load = r24(f64::from(sw.load) + f64::from(a)) as f32;
                    }
                }
                // 2. Volume (two separate roundings for style 2).
                let net_t = NETWORK_OF_TRAVEL[t];
                if cx.net.network_info(net_t, x, z).is_some() && tables.generates_traffic[t] {
                    let mut v = host.volume(net_t, x, z);
                    v = r24(f64::from(v) + f64::from(q)) as f32;
                    if style == 2 {
                        v = r24(f64::from(v) + f64::from(q)) as f32;
                    }
                    host.set_volume(net_t, x, z, v);
                }
                // 3. Edge density: entry from the predecessor (or from the
                // start rect), exit towards prev on unsigned bytes
                // *[quirk]*; style 2 also adds to the key-bit-1 record at the
                // **mirrored** index `4·exit + entry`, as `ApplyRoute` does
                // (PF2, `0x006DA324`–`0x006DA330`).
                let entry = match pred {
                    Some(p) => edge_to((p.x, p.z), (x, z)),
                    None => {
                        if x < s.x1 {
                            2
                        } else if x > s.x2 {
                            0
                        } else if z <= s.z2 {
                            3
                        } else {
                            1
                        }
                    }
                };
                let exit = edge_to((i32::from(prev.0), i32::from(prev.1)), (x, z));
                let idx = 4 * entry + exit;
                let key = NetworkState::edge_key(x, z, t as u32, style == 1);
                add_edge_density(cx.net, key, idx, qi);
                if style == 2 {
                    let mirrored = 4 * exit + entry;
                    add_edge_density(cx.net, NetworkState::edge_key(x, z, t as u32, true), mirrored, qi);
                }
                // 4. Income, with the uncarpooled count.
                let per = r24(f64::from(self.arrived) * f64::from(tables.income_per_tile[t]));
                income = r24(f64::from(income) + per) as f32;
            }
            prev = (x as u8, z as u8);
        }
        // The double accumulator: the add is done at 24-bit precision
        // *[quirk][compat]*.
        let acc = host.income_accumulator();
        host.set_income_accumulator(r24(f64::from(income) + acc));
    }

    /// `ApplyTraffic` on a network kept in a [`NetworkModel`] (see
    /// [`Pathfinder::find_path_on`]).
    pub fn apply_traffic_on(&mut self, model: &NetworkModel, host: &dyn PathfinderHost, style: u32) {
        model.run(|cx| self.apply_traffic(&mut PathCx { net: cx.s, host }, style));
    }
}
