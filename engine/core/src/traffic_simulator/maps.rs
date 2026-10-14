// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The map phases: CreateMaps (§4.9), CreateFinalMap (§4.10),
//! ClearTripDestinations and RefreshCommuteMap (§4.11); and the slot-11 and
//! slot-24 queries (§4.8, §4.9).

use crate::curve::ResponseCurve;
use crate::traffic_network::{NetworkState, NET_OF_TRAVEL};
use crate::x87::{r24, rhe};

use super::adapters::{capacity_element, set_capacity_element};
use super::cycle::Run;
use super::state::{AccidentEntry, StatRow, BRIDGE_SLOT};
use super::tuning::SimTuning;
use super::{msg, SimMessage, TrafficSimHost};

/// The BDS developer types of the background traffic (§4.9).
const BG_RESIDENTIAL: [u32; 3] = [0x1010, 0x1020, 0x1030];
const BG_COMMERCIAL: [u32; 5] = [0x3110, 0x3120, 0x3130, 0x3320, 0x3330];
const BG_INDUSTRIAL: [u32; 4] = [0x4100, 0x4200, 0x4300, 0x4400];

/// `0.1f` (`0x00A8C950`).
const TENTH: f32 = 0.1;

/// `max` as the x87 compare does it: `v` replaces `cur` only when `v > cur`
/// (a NaN never does).
fn fmax(cur: f32, v: f32) -> f32 {
    if v > cur {
        v
    } else {
        cur
    }
}

/// `GetBackgroundTraffic(x, z)` (slot 11, §4.9).
pub fn background_traffic(h: &dyn TrafficSimHost, t: &SimTuning, x: i32, z: i32) -> f32 {
    let sum = |devs: &[u32]| {
        devs.iter()
            .fold(0i32, |a, &d| a.wrapping_add(i32::from(h.developer_population(d, x, z))))
    };
    let (r, c, i) = (sum(&BG_RESIDENTIAL), sum(&BG_COMMERCIAL), sum(&BG_INDUSTRIAL));
    background_value(r, c, i, &t.background)
}

/// `r24(r24(r24(R × bg[0]) + r24(C × bg[1])) + r24(I × bg[2]))` (§4.9).
pub fn background_value(r: i32, c: i32, i: i32, bg: &[f32; 3]) -> f32 {
    let a = r24(f64::from(r) * f64::from(bg[0]));
    let b = r24(f64::from(c) * f64::from(bg[1]));
    let d = r24(f64::from(i) * f64::from(bg[2]));
    r24(r24(a + b) + d) as f32
}

/// `GetTravelTimeRatio(x, z, t)` (slot 24, §4.8).
pub fn travel_time_ratio(
    net: &NetworkState,
    affected: bool,
    generates: &[bool; 9],
    x: i32,
    z: i32,
    travel_type: u32,
) -> f32 {
    if !affected {
        return 1.0;
    }
    let Some(&n) = NET_OF_TRAVEL.get(travel_type as usize) else {
        return 1.0;
    };
    let mut u: u32 = 0;
    for (i, &g) in generates.iter().enumerate() {
        if g {
            for evening in [false, true] {
                u = u.wrapping_add(u32::from(net.edge_density_at(x, z, i as u32, evening).usage));
            }
        }
    }
    let Some(info) = net.network_info(n, x, z) else {
        return 1.0;
    };
    let v = r24(r24(f64::from(u)) * f64::from(info.inv_cap)) as f32;
    net.congestion_curve.eval(v)
}

/// The air-polluting map value (§4.10): `min(255, round(r24(b × 0.1f)))`,
/// unsigned compare; 0 for `b = 0`.
pub fn air_polluting_byte(b: f32) -> u8 {
    if b == 0.0 {
        return 0;
    }
    let g = rhe(r24(f64::from(b) * f64::from(TENTH)) as f32) as u32;
    g.min(255) as u8
}

/// The congestion map value (§4.10): 0 for `b = 0`; `c := curve(D)`; 1 if
/// `c ≤ 1`; else `g := round(r24(r24(c − 1) × scale))`, 255 if `g > 255`
/// (unsigned), 1 if `g = 0`.
pub fn congestion_byte(b: f32, d: f32, curve: &ResponseCurve, scale: f32) -> u8 {
    if b == 0.0 {
        return 0;
    }
    let c = curve.eval(d);
    if c <= 1.0 {
        return 1;
    }
    let g = rhe(r24(r24(f64::from(c) - 1.0) * f64::from(scale)) as f32) as u32;
    if g > 255 {
        255
    } else if g == 0 {
        1
    } else {
        g as u8
    }
}

/// The cell value `cv(x, z, k)` of the commercial traffic window
/// (`0x00714940`, §4.9 step 2, TS8): the maximum (u16) of the road sum
/// `usage(Car) + usage(Bus) + usage(FreightTruck)` (u16, wrapping), the
/// el-train `usage(7)` and the monorail `usage(8)`. The record key takes the
/// low byte of `x` (§4.8).
pub fn commercial_cell_value(s: &NetworkState, x: i32, z: i32, evening: bool) -> u16 {
    let u = |t: u32| s.edge_density_at(x, z, t, evening).usage;
    let road = u(1).wrapping_add(u(2)).wrapping_add(u(4));
    road.max(u(7)).max(u(8))
}

/// The commercial traffic window value of block `(bx, bz)` (`0x00714AA0`,
/// §4.9 step 2, TS8). With `X0 = 4·bx`, `Z0 = 4·bz`: a plus shape, z from
/// `max(0, Z0 − 2)` to `min(H, Z0 + 6) − 1`; on the block's own rows x from
/// `max(0, X0 − 2)` to `X0 + 5` (no upper clamp *[compat]*), on the others
/// x from `X0` to `X0 + 3`. `v := (maxM + maxE) mod 2¹⁶`, the morning and
/// evening maxima taken independently.
pub fn commercial_window(s: &NetworkState, bx: i32, bz: i32) -> u16 {
    let (x0, z0) = (4 * bx, 4 * bz);
    let (mut m, mut e) = (0u16, 0u16);
    for z in (z0 - 2).max(0)..(z0 + 6).min(s.count_z) {
        let (xa, xb) = if (z0..z0 + 4).contains(&z) {
            ((x0 - 2).max(0), x0 + 5)
        } else {
            (x0, x0 + 3)
        };
        for x in xa..=xb {
            m = m.max(commercial_cell_value(s, x, z, false));
            e = e.max(commercial_cell_value(s, x, z, true));
        }
    }
    m.wrapping_add(e)
}

impl Run<'_, '_> {
    /// CreateMaps (state 2, `0x0071A0C0`, §4.9).
    pub(crate) fn create_maps(&mut self) {
        while let Some((x, z)) = self.st.scan.next_cell() {
            self.create_maps_cell(x, z);
        }
        // The end of the pass (`0x0071A5D7`–`0x0071A701`).
        // 1. The averages; the five fields of each slot with records go to
        // the published set. A slot with no record publishes (0, 0, 0, 0,
        // 0) *[question: §4.9 says it "keeps (0, 0, 0, 0, 0)" while the
        // entry reset leaves max −1.0]*.
        let w = &mut self.st.stats_work;
        let last = &mut self.st.stats_last;
        let counts = self.st.stat_counts;
        let work_rows = w.rows.iter_mut().chain(std::iter::once(&mut w.bridge));
        let last_rows = last.rows.iter_mut().chain(std::iter::once(&mut last.bridge));
        for ((wr, lr), &n) in work_rows.zip(last_rows).zip(counts.iter()) {
            if n != 0 {
                wr.sum = r24(f64::from(wr.sum) / r24(f64::from(n))) as f32;
                *lr = *wr;
            } else {
                *lr = StatRow::default();
            }
        }
        // 2. The record counts replace the published counts and the cell
        // counts; the bridge records are neither (TS9).
        last.records = w.records;
        self.st.cells_per_type = w.records.map(u32::from);
        // 3. The department costs.
        self.update_roads_budget();
        self.update_mass_transit_budget();
        // 4.
        self.st.accident_check = std::mem::take(&mut self.st.accidents);
    }

    /// `UpdateRoadsBudget` (`0x0070D8A0`, §4.9): the roads department's
    /// monthly costs from the cell counts.
    pub(crate) fn update_roads_budget(&mut self) {
        if let Some(d) = self.st.roads {
            self.h
                .update_network_costs(d, false, &self.st.cells_per_type, &self.st.tuning.tile_costs);
        }
    }

    /// `UpdateMassTransitBudget` (`0x0070DA40`, §4.9).
    pub(crate) fn update_mass_transit_budget(&mut self) {
        if let Some(d) = self.st.mass_transit {
            self.h
                .update_network_costs(d, true, &self.st.cells_per_type, &self.st.tuning.tile_costs);
        }
    }

    fn create_maps_cell(&mut self, x: i32, z: i32) {
        // 1.
        let mut b = 0.0f32;
        let mut d = 0.0f32;
        // 2.
        if x % 4 == 0 && z % 4 == 0 {
            let v = commercial_window(self.net(), x / 4, z / 4);
            let scale = self.st.tuning.commercial_scale;
            // Ties to even (the `+ 12582912.0f` trick, `0x0071A271`).
            let g = rhe(r24(f64::from(v) * f64::from(scale)) as f32) as u32;
            if let Some(grid) = &self.st.commercial_work {
                grid.set(x / 4, z / 4, g.min(255) as u8);
            }
        }
        // 3.
        let infos: Vec<_> = self.net().info_at(x, z).to_vec();
        for e in &infos {
            let n = u32::from(e.net);
            let Some(grid) = self.st.volume_grid(n).cloned() else {
                continue;
            };
            let vol = grid.get(x, z);
            if n != 2 && n != 3 {
                b = fmax(b, vol);
            }
            let r = r24(f64::from(vol) * f64::from(e.inv_cap)) as f32;
            d = fmax(d, r);
            if (n == 1 || n == 2) && e.conn_count > 3 {
                let p = self.st.tuning.accident_congestion_curve.eval(r);
                if p > 0.0 {
                    let cap = r24(1.0 / f64::from(e.inv_cap)) as f32;
                    let q = self.st.tuning.accident_capacity_curve.eval(cap);
                    let p2 = r24(f64::from(q) * f64::from(p)) as f32;
                    if p2 > 0.0 {
                        self.st.accidents.push(AccidentEntry { x, z, p: p2 });
                    }
                }
            }
            // Statistics (TS9): every record counts; flag bit 0 (network
            // flag 0x40000) picks the bridge slot 13, else the type's slot.
            let slot = if e.flags & 1 != 0 {
                Some(BRIDGE_SLOT)
            } else {
                Some(usize::from(e.net_type)).filter(|&t| t < 13)
            };
            if let Some(s) = slot {
                let w = &mut self.st.stats_work;
                let (row, records) = if s == BRIDGE_SLOT {
                    (&mut w.bridge, &mut w.bridge_records)
                } else {
                    (&mut w.rows[s], &mut w.records[s])
                };
                row.sum = r24(f64::from(row.sum) + f64::from(r)) as f32;
                self.st.stat_counts[s] = self.st.stat_counts[s].wrapping_add(1);
                *records = records.wrapping_add(1);
                if r > row.max {
                    row.max = r;
                    row.max_x = x;
                    row.max_z = z;
                }
                if r > 1.0 {
                    row.congested = row.congested.wrapping_add(1);
                }
            }
            let mut nv = r24(f64::from(self.st.tuning.density_reduction) * f64::from(vol)) as f32;
            if n == 1 {
                let bg = background_traffic(self.h, &self.st.tuning, x, z);
                nv = r24(f64::from(nv) + f64::from(bg)) as f32;
            }
            grid.set(x, z, nv);
        }
        // 4.
        let id = self.net().switch_id_at(x, z);
        if id != 0 {
            if let Some(sw) = self.cx.s.switches.get_mut(id) {
                sw.counter_a = sw.load;
                let ratio = r24(f64::from(sw.load) / f64::from(sw.capacity)) as f32;
                d = fmax(d, ratio);
                sw.counter_b = if ratio < 1.0 { ratio } else { 1.0 };
            }
        }
        // 5.
        if !infos.is_empty() {
            b = fmax(b, 1.0);
        }
        if let Some(g) = &self.st.peak_volume {
            g.set(x, z, b);
        }
        if let Some(g) = &self.st.peak_ratio {
            g.set(x, z, d);
        }
    }

    /// CreateFinalMap (state 4, `0x0070F460`, §4.10), then
    /// `TrafficMapChanged`.
    pub(crate) fn create_final_map(&mut self) {
        let curve = self.st.tuning.congestion_curve.clone();
        let scale = self.st.tuning.congestion_scale;
        while let Some((x, z)) = self.st.scan.next_cell() {
            let b = self.st.peak_volume.as_ref().map_or(0.0, |g| g.get(x, z));
            let d = self.st.peak_ratio.as_ref().map_or(0.0, |g| g.get(x, z));
            // Tract size 1 assumed for both grids *[quirk]*.
            if let Some(g) = &self.st.air {
                g.set(x, z, air_polluting_byte(b));
            }
            if let Some(g) = &self.st.congestion {
                g.set(x, z, congestion_byte(b, d, &curve, scale));
            }
        }
        self.h.post_message(SimMessage::of_type(msg::TRAFFIC_MAP_CHANGED));
    }

    /// ClearTripDestinations (state 5, §4.11): each lot once, at its origin
    /// cell; the roll goes through the element store (TS18), so a lot
    /// without the property gets one.
    pub(crate) fn clear_trip_destinations(&mut self) {
        while let Some((x, z)) = self.st.scan.next_cell() {
            let Some(lot) = self.h.lot_at(x, z, true) else { continue };
            let holder = self.h.lot_holder(lot);
            for k in 0..4 {
                let v = capacity_element(self.h, holder, 4 + k);
                set_capacity_element(self.h, holder, 8 + k, v);
                set_capacity_element(self.h, holder, 4 + k, 0);
            }
            for k in 0..3 {
                let c = capacity_element(self.h, holder, k);
                self.st.job_capacity[k] = self.st.job_capacity[k].wrapping_add(c);
            }
        }
    }

    /// RefreshCommuteMap (state 10, §4.11): the destination map. The bits of
    /// the lot are ORed in after `&= 0xF8`, so bit 3 is never cleared
    /// *[quirk]*.
    pub(crate) fn refresh_commute_map(&mut self) {
        while let Some((x, z)) = self.st.scan.next_cell() {
            let mut byte = self.st.dest_byte(x, z) & 0xF8;
            let (lot, count) = match self.h.lot_at(x, z, true) {
                Some(l) => (Some(l), true),
                None => (self.h.lot_at(x, z, false), false),
            };
            if let Some(lot) = lot {
                let holder = self.h.lot_holder(lot);
                for k in 0..4usize {
                    if capacity_element(self.h, holder, k) != 0 {
                        byte |= 1 << k;
                        if count {
                            self.st.set_free(k, self.st.free(k).wrapping_add(1));
                        }
                    }
                }
            }
            self.st.set_dest_byte(x, z, byte);
        }
    }
}
