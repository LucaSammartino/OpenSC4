// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The save record (§6.1, save-records.md §7.1.1, the authoritative layout
//! with every Read gate) and the trip record (§6.3).
//!
//! Write produces version 41 (`A = 13`, `B = 1`, `C = 13`, `D = 9`). Read
//! accepts versions 19…41, and anything newer is read exactly as 41: the
//! original has no upper bound and every gate is an unsigned ordered test
//! (§6.4, TS32). Read parses into copies and commits only when the whole
//! record was read, so a failed Read leaves the simulator unchanged
//! *[choice]*; the original writes as it reads and leaves a partly loaded
//! simulator with null refs (§6.1 "A failed Read", TS30).

use std::collections::BTreeMap;

use crate::grid::{CellRect, GridProvider, ScanIterator, ScanState};
use crate::property::PropertyHolder;
use crate::stl::StlHashMap;
use crate::stream::{InStream, OutStream, RefValue};
use crate::traffic_network::{
    FerryTerminal, GridU32, NeighborLink, NetworkState, Rule, TerminalLink, TrafficEdgeDensity, TransitSwitch,
    TunnelInfo,
};

use super::cycle::Run;
use super::pools::Pools;
use super::state::{DamageBlock, DamageState, StatRow, State, StatsSet, SwitchStat, SWITCH_STATS};
use super::{ref_iid, GridF32, GridU8, TrafficSimHost, TrafficSimulator, TripData, MIN_RECORD_VERSION, RECORD_VERSION};

/// The trip record version written (§6.3).
pub const TRIP_RECORD_VERSION: u32 = 2;
/// A vector count at or above this reads nothing (§6.3, `0x005DB520`).
pub const MAX_VECTOR: u32 = 1_000_000;

/// The dimension counts written (save-records §7.1.1).
const DIM_A: i32 = 13;
const DIM_B: i32 = 1;
const DIM_C: i32 = 13;
const DIM_D: i32 = 9;

/// The legacy row → network type table of the statistics sets (v < 35).
const LEGACY_ROWS: [usize; 6] = [0, 0, 1, 7, 8, 9];

// ---- §6.3 the trip record ----

fn put_vec(out: &mut dyn OutStream, v: &[u8]) {
    out.put_u32(v.len() as u32);
    out.put_bytes(v);
}

/// A `vector<u8>`: a count at or above 1,000,000 reads nothing and keeps
/// the previous contents *[compat]*.
fn get_vec(inp: &mut dyn InStream, dst: &mut Vec<u8>) -> Option<()> {
    let n = inp.get_u32()?;
    if n >= MAX_VECTOR {
        return Some(());
    }
    *dst = inp.get_bytes(n as usize)?;
    Some(())
}

/// `cSC4TripData::Write` (`0x0070E0B0`, §6.3).
pub fn write_trip(out: &mut dyn OutStream, t: &TripData) -> bool {
    out.put_u32(TRIP_RECORD_VERSION);
    put_vec(out, &t.route);
    put_vec(out, &t.return_route);
    out.put_u32(t.count);
    out.put_u8(t.trip_type);
    out.put_u8(t.strategy);
    out.put_u8(t.flags);
    out.put_u8(t.status);
    out.put_i16(t.dest_x);
    out.put_i16(t.dest_z);
    out.put_f32(t.trip_length);
    out.put_u32(t.travel_types_used);
    out.error() == 0
}

/// `cSC4TripData::Read` (`0x0070E160`, §6.3): any version; the mask only
/// when the version is above 1 (else 0).
pub fn read_trip(inp: &mut dyn InStream, t: &mut TripData) -> bool {
    fn body(inp: &mut dyn InStream, t: &mut TripData) -> Option<()> {
        let v = inp.get_u32()?;
        get_vec(inp, &mut t.route)?;
        get_vec(inp, &mut t.return_route)?;
        t.count = inp.get_u32()?;
        t.trip_type = inp.get_u8()?;
        t.strategy = inp.get_u8()?;
        t.flags = inp.get_u8()?;
        t.status = inp.get_u8()?;
        t.dest_x = inp.get_i16()?;
        t.dest_z = inp.get_i16()?;
        t.trip_length = inp.get_f32()?;
        t.travel_types_used = if v > 1 { inp.get_u32()? } else { 0 };
        Some(())
    }
    body(inp, t).is_some() && inp.error() == 0
}

// ---- writing ----

fn grid_ref<T: crate::grid::GridElem>(g: &Option<std::rc::Rc<dyn crate::grid::TractGrid<T>>>) -> RefValue {
    g.as_ref().map_or(RefValue::Null, |g| g.save_ref())
}

/// The scanning iterator; `raw` is the loaded state byte kept while the
/// scan runs (TS31).
fn put_iterator(out: &mut dyn OutStream, it: &ScanIterator, raw: Option<u8>) {
    out.put_u8(match (it.state, raw) {
        (ScanState::Fresh, _) => 0,
        (ScanState::Running, Some(b)) => b,
        (ScanState::Running, None) => 1,
        (ScanState::Done, _) => 2,
    });
    out.put_i32(it.x);
    out.put_i32(it.z);
    out.put_u32(it.step as u32);
    for v in [it.x0, it.z0, it.x1, it.z1] {
        out.put_i32(v);
    }
}

fn put_row(out: &mut dyn OutStream, r: &StatRow) {
    out.put_f32(r.sum);
    out.put_u32(r.congested);
    out.put_i32(r.max_x);
    out.put_i32(r.max_z);
    out.put_f32(r.max);
}

fn put_stats(out: &mut dyn OutStream, s: &StatsSet) {
    for r in &s.rows {
        put_row(out, r);
    }
    put_row(out, &s.bridge);
    for &o in &s.records {
        out.put_u16(o);
    }
    out.put_u16(s.bridge_records);
    out.put_u16(s.potholes);
    out.put_i32(SWITCH_STATS as i32);
    for e in &s.switch_stats {
        out.put_u16(e.a);
        out.put_f32(e.b);
        out.put_i32(e.c);
        out.put_i32(e.d);
        out.put_f32(e.e);
    }
    out.put_f32(s.f1);
    out.put_f32(s.f2);
    out.put_bool(s.flag);
    out.put_f32(s.f3);
    out.put_u8(s.byte);
}

/// The field labels of the two damage blocks (road, rail) for
/// [`OutStream::mark`].
pub const DAMAGE_LABELS: [[&str; 3]; 2] = [
    [
        "road damage: counts, rate",
        "road damage: intact keys",
        "road damage: damaged keys",
    ],
    [
        "rail damage: counts, rate",
        "rail damage: intact keys",
        "rail damage: damaged keys",
    ],
];

fn put_damage(out: &mut dyn OutStream, b: &DamageBlock, labels: [&'static str; 3]) {
    out.mark(labels[0]);
    out.put_u32(b.cell_count);
    out.put_u32(b.damaged_count);
    out.put_f32(b.rate);
    for (set, label) in [(&b.intact, labels[1]), (&b.damaged, labels[2])] {
        out.mark(label);
        let keys = set.iter_order();
        out.put_u32(keys.len() as u32);
        for k in keys {
            out.put_u16(k as u16);
        }
    }
}

/// The four link counters in file order: 0, 2, 1, 3 (save-records §7.1.1).
fn put_counters(out: &mut dyn OutStream, c: &[u16; 4]) {
    for i in [0, 2, 1, 3] {
        out.put_u16(c[i]);
    }
}

fn put_terminal(out: &mut dyn OutStream, t: &FerryTerminal) {
    for v in [t.dep_x, t.dep_z, t.arr_x, t.arr_z, t.water_x, t.water_z] {
        out.put_u8(v as u8);
    }
    out.put_u8(t.ferry_type);
    out.put_u16(t.mask);
    out.put_u16(t.riders_last);
    out.put_u16(t.riders_current);
    out.put_u8(t.lot_x as u8);
    out.put_u8(t.lot_z as u8);
    out.put_u32(t.links.len() as u32);
    for l in &t.links {
        out.put_f32(l.cost);
        out.put_i16(l.arrival_x as i16);
        out.put_i16(l.arrival_z as i16);
        out.put_u16(l.to_key);
        put_vec(out, &l.route);
        put_counters(out, &l.counters);
    }
    out.put_u32(t.neighbour_links.len() as u32);
    for l in &t.neighbour_links {
        out.put_f32(l.cost);
        out.put_i16(l.off_x);
        out.put_i16(l.off_z);
        out.put_i16(l.edge_x as i16);
        out.put_i16(l.edge_z as i16);
        put_vec(out, &l.route_to);
        put_vec(out, &l.route_from);
        put_counters(out, &l.counters);
    }
}

/// `Write` (`0x00712400`): version 41.
fn write_record(
    out: &mut dyn OutStream,
    h: &dyn TrafficSimHost,
    s: &State,
    n: &NetworkState,
    d: &DamageState,
    pools: &Pools,
) {
    let obj = |t: u64| if t == 0 { RefValue::Null } else { h.object_ref(t) };
    out.mark("version");
    out.put_u16(RECORD_VERSION);
    out.mark("dimension counts");
    for v in [DIM_A, DIM_B, DIM_C, DIM_D] {
        out.put_i32(v);
    }
    out.mark("published grids");
    for r in [
        grid_ref(&s.air),
        grid_ref(&s.peak_volume),
        grid_ref(&s.commercial),
        grid_ref(&s.commercial_work),
        grid_ref(&s.trip_length),
        grid_ref(&s.congestion),
    ] {
        out.put_ref(&r);
    }
    out.mark("destination map: size");
    out.put_u32(s.dest_rows as u32);
    out.put_u32(s.dest_cols as u32);
    out.mark("destination map");
    let bytes: Vec<u8> = s.dest.iter().map(|c| c.get()).collect();
    out.put_bytes(&bytes);
    out.mark("volume grids");
    out.put_u32(s.volume.len() as u32);
    for g in &s.volume {
        out.put_ref(&grid_ref(g));
    }
    out.mark("float grids");
    out.put_ref(&grid_ref(&s.float_grid));
    out.put_ref(&grid_ref(&s.peak_ratio));
    out.mark("edge densities: count");
    out.put_u32(n.edge_density.len() as u32);
    out.mark("edge densities");
    for (key, r) in n.edge_density.iter() {
        out.put_u32(key);
        for &v in &r.last {
            out.put_u16(v);
        }
        out.put_u16(r.usage);
        for &v in &r.current {
            out.put_u16(v);
        }
    }
    out.mark("jobCapacity, attempted, routed");
    for k in 0..3 {
        out.put_u32(s.job_capacity[k]);
        out.put_u32(s.attempted[k]);
        out.put_u32(s.routed[k]);
    }
    out.mark("switch grids");
    out.put_ref(&grid_ref(&n.switch_mode));
    out.put_ref(&grid_ref(&n.switch_id));
    out.mark("switches: next ID");
    out.put_u32(n.next_switch_id);
    out.mark("switches: count");
    out.put_u32(n.switches.len() as u32);
    out.mark("switches");
    for (id, sw) in n.switches.iter() {
        out.put_u32(id);
        out.put_ref(&obj(sw.building));
        out.put_f32(sw.load);
        out.put_f32(sw.time_factor);
        out.put_f32(sw.capacity);
        out.put_bool(sw.has_access_point);
        out.put_u8(sw.access_x);
        out.put_u8(sw.access_z);
        out.put_f32(sw.counter_a);
        out.put_f32(sw.counter_b);
        out.put_u32(sw.rules.len() as u32);
        for r in &sw.rules {
            out.put_bool(r.kind != 0);
            for &b in &r.side {
                out.put_bool(b);
            }
            out.put_u16(u16::from(r.from));
            out.put_u16(u16::from(r.to));
        }
        // The 0x0BFC0A4C value, written as the f32 of its bits.
        out.put_u32(sw.simulator_value);
        out.put_bool(sw.counts_as_intersection);
    }
    out.mark("departments");
    out.put_ref(&obj(s.roads.unwrap_or(0)));
    out.put_ref(&obj(s.mass_transit.unwrap_or(0)));
    out.mark("budget total E");
    out.put_i64(s.budget_total);
    out.mark("transitIncome");
    out.put_f32(s.transit_income);
    out.mark("tollIncome");
    out.put_f32(s.toll_income);
    out.mark("lastMonthlyIncome");
    out.put_f32(s.last_monthly_income);
    out.mark("income accumulator");
    out.put_f64(s.income_acc.get());
    out.mark("fee accumulator");
    out.put_f64(s.fee_acc);
    out.mark("cycle start date");
    out.put_u32(s.cycle_start_date);
    out.mark("strike state");
    out.put_u8(s.strike_state);
    out.mark("strike months");
    out.put_i32(s.strike_months);
    put_damage(out, &d.road, DAMAGE_LABELS[0]);
    put_damage(out, &d.rail, DAMAGE_LABELS[1]);
    out.mark("cycle state");
    out.put_u8(s.cycle_state);
    out.mark("scanning iterator");
    put_iterator(out, &s.scan, s.extras.scan_state_byte);
    out.mark("intersection dirty flag, rect");
    out.put_bool(n.intersection_dirty);
    let r = n.intersection_rect;
    for v in [r.x1, r.z1, r.x2, r.z2] {
        out.put_i32(v);
    }
    let e = &s.extras;
    out.mark("network-changed byte");
    out.put_bool(e.network_changed_byte);
    out.mark("strategy bucket counter");
    out.put_u32(s.strategy_bucket_counter);
    out.mark("strategy bucket");
    out.put_u32(s.strategy_bucket);
    out.mark("phase positions");
    out.put_i16(e.phase_positions[0]);
    out.put_i16(e.phase_positions[1]);
    out.mark("~lot iterator");
    out.put_bytes(&e.lot_iter);
    out.mark("~re-route context");
    out.put_bool(e.context_bools[0]);
    out.put_bool(e.context_bools[1]);
    out.put_f32(e.context_f32);
    out.mark("re-route context cell");
    out.put_i32(e.context_x);
    out.put_i32(e.context_z);
    out.mark("~re-route context (after the cell)");
    out.put_u32(e.context_u32);
    for &v in &e.context_u32x4 {
        out.put_u32(v);
    }
    out.put_u8(e.context_u8);
    out.put_u32(e.context_u32b);
    for &v in &e.context_i32x4 {
        out.put_i32(v);
    }
    out.mark("~context table");
    for row in &e.table {
        for &v in row {
            out.put_u32(v);
        }
    }
    out.mark("~context flags");
    out.put_u8(e.context_flags[0]);
    out.put_u8(e.context_flags[1]);
    out.mark("processed");
    out.put_i32(s.processed);
    // `dayIndex` is the s32 right after `processed` (`+0x918`, TS60).
    out.mark("day index");
    out.put_i32(s.day_index as i32);
    out.mark("total");
    out.put_i32(s.total);
    out.mark("days");
    out.put_u32(s.days);
    // Three u32 that only Write and Read touch (TS60).
    out.mark("~pacing unused u32 x3");
    for &v in &e.pacing_unused {
        out.put_u32(v);
    }
    out.mark("passTotal");
    out.put_i32(s.pass_total);
    out.mark("toReRoute");
    out.put_i32(s.to_reroute);
    out.mark("toSearch");
    out.put_i32(s.to_search);
    out.mark("pass counters");
    for &c in &s.pass_counters {
        out.put_u32(c);
    }
    out.mark("statistics (published)");
    put_stats(out, &s.stats_last);
    out.mark("statistics (working)");
    put_stats(out, &s.stats_work);
    out.mark("tunnels: count");
    out.put_u32(n.tunnels.len() as u32);
    out.mark("tunnels");
    for (key, t) in n.tunnels.iter() {
        out.put_u16(key as u16);
        out.put_u8(t.other.0 as u8);
        out.put_u8(t.other.1 as u8);
        out.put_u16(t.mask);
        out.put_u16(u16::from(t.net_type));
        for &c in &t.cost {
            out.put_f32(c);
        }
    }
    out.mark("avgCarCommute");
    out.put_f32(s.avg_car_commute);
    out.mark("car length sum, count");
    out.put_f32(s.car_len_sum);
    out.put_u32(s.car_len_count);
    out.mark("freight length sum, count");
    out.put_f32(s.freight_len_sum);
    out.put_u32(s.freight_count);
    out.mark("cells per network type");
    for &c in &s.cells_per_type {
        out.put_u32(c);
    }
    out.mark("freeDest");
    out.put_u32(4);
    for k in 0..4 {
        out.put_u32(s.free(k));
    }
    // The neighbour pools, by ascending ID (traffic-network.md §2.7.3).
    out.mark("pools: count");
    out.put_u32(pools.map.len() as u32);
    out.mark("pools");
    for (id, holder) in &pools.map {
        out.put_u16(*id);
        holder.write(out);
    }
    // The pairs are (published `+0x424`, building `+0x430`)
    // (`0x00713370`–`0x00713398`, TS56).
    out.mark("travel type counts");
    for i in 0..9 {
        out.put_u32(s.type_count_published[i]);
        out.put_u32(s.type_count_building[i]);
    }
    out.mark("ferry total");
    out.put_u32(s.ferry_total);
    out.mark("ferry terminals: count");
    out.put_u32(n.ferries.len() as u32);
    out.mark("ferry terminals");
    for t in &n.ferries {
        put_terminal(out, t);
    }
}

// ---- reading ----

/// The network-owned part of the record, parsed before the commit.
struct NetPart {
    switch_mode: Option<GridU8>,
    switch_id: Option<GridU32>,
    next_switch_id: u32,
    switches: StlHashMap<TransitSwitch>,
    edge: StlHashMap<TrafficEdgeDensity>,
    tunnels: StlHashMap<TunnelInfo>,
    ferries: Vec<FerryTerminal>,
    intersection_dirty: bool,
    intersection_rect: CellRect,
}

/// The parse in progress.
struct Parse<'a> {
    h: &'a dyn TrafficSimHost,
    v: u16,
    a: usize,
    b: usize,
    c: usize,
    d: usize,
    s: State,
    n: NetPart,
    dmg: DamageState,
    /// The neighbour pools of row 19.
    pools: BTreeMap<u16, PropertyHolder>,
}

/// A float edge-density value of a v < 20 record: round half to even,
/// anything outside 0…65,535 → 65,535.
pub fn legacy_density(v: f32) -> u16 {
    let r = crate::props::round_half_even_f64(f64::from(v));
    if (0.0..=65_535.0).contains(&r) {
        r as u16
    } else {
        65_535
    }
}

/// Read's own mask and flags of a switch (traffic-network.md §4.7.2, after
/// a load *[quirk]*): the mask ORs the raw `to` of the kind-1 rules; the
/// flags map 2 → 1, 3 → 2, 5 → 0x20, 6 → 4, 7 → 8, 8 → 0x10 over all rules,
/// plus 0x40 for a subway–el-train rule (6, 7) or (7, 6) of any kind, as the
/// re-read does (`0x0072225E`–`0x0072227D`, TS33).
pub fn read_mask_and_flags(rules: &[Rule]) -> (u16, u8) {
    let mut mask = 0u16;
    let mut flags = 0u8;
    for r in rules {
        if r.kind == 1 {
            mask |= u16::from(r.to);
        }
        flags |= match r.to {
            2 => 0x01,
            3 => 0x02,
            5 => 0x20,
            6 => 0x04,
            7 => 0x08,
            8 => 0x10,
            _ => 0,
        };
        if (r.from == 6 && r.to == 7) || (r.from == 7 && r.to == 6) {
            flags |= 0x40;
        }
    }
    (mask, flags)
}

impl Parse<'_> {
    fn grid8(&self, inp: &mut dyn InStream) -> Option<Option<GridU8>> {
        let r = inp.get_ref_as(ref_iid::GRID)?;
        Some(if r.is_null() {
            None
        } else {
            GridProvider::<u8>::resolve_grid(self.h, &r)
        })
    }

    fn gridf(&self, inp: &mut dyn InStream) -> Option<Option<GridF32>> {
        let r = inp.get_ref_as(ref_iid::GRID)?;
        Some(if r.is_null() {
            None
        } else {
            GridProvider::<f32>::resolve_grid(self.h, &r)
        })
    }

    fn grid32(&self, inp: &mut dyn InStream) -> Option<Option<GridU32>> {
        let r = inp.get_ref_as(ref_iid::GRID)?;
        Some(if r.is_null() {
            None
        } else {
            GridProvider::<u32>::resolve_grid(self.h, &r)
        })
    }

    fn token(&self, inp: &mut dyn InStream, iid: u32) -> Option<Option<u64>> {
        let r = inp.get_ref_as(iid)?;
        Some(if r.is_null() { None } else { self.h.resolve_ref(&r, iid) })
    }

    /// The scanning iterator and its raw state byte (§6.1, TS31): the inline
    /// GetNext tests only 2 (done) and 0 (fresh), so 1 and every unknown
    /// byte 3…255 act as running; an unknown byte is kept for the next save.
    fn iterator(inp: &mut dyn InStream) -> Option<(ScanIterator, Option<u8>)> {
        let byte = inp.get_u8()?;
        let (state, raw) = match byte {
            0 => (ScanState::Fresh, None),
            2 => (ScanState::Done, None),
            1 => (ScanState::Running, None),
            b => (ScanState::Running, Some(b)),
        };
        let x = inp.get_i32()?;
        let z = inp.get_i32()?;
        let step = inp.get_u32()? as i32;
        let (x0, z0, x1, z1) = (inp.get_i32()?, inp.get_i32()?, inp.get_i32()?, inp.get_i32()?);
        let mut it = ScanIterator::new(x0, z0, x1, z1);
        it.set_step(step);
        it.x = x;
        it.z = z;
        it.state = state;
        Some((it, raw))
    }

    fn row(inp: &mut dyn InStream) -> Option<StatRow> {
        Some(StatRow {
            sum: inp.get_f32()?,
            congested: inp.get_u32()?,
            max_x: inp.get_i32()?,
            max_z: inp.get_i32()?,
            max: inp.get_f32()?,
        })
    }

    fn stats(&self, inp: &mut dyn InStream) -> Option<StatsSet> {
        let v = self.v;
        let mut s = StatsSet::default();
        if v >= 35 {
            for i in 0..self.a {
                let r = Self::row(inp)?;
                if let Some(slot) = s.rows.get_mut(i) {
                    *slot = r;
                }
            }
        } else {
            for i in 0..self.c {
                let r = Self::row(inp)?;
                if let Some(&t) = LEGACY_ROWS.get(i) {
                    s.rows[t] = r;
                }
            }
        }
        if v >= 37 {
            for i in 0..self.b {
                let r = Self::row(inp)?;
                if i == 0 {
                    s.bridge = r;
                }
            }
        }
        for i in 0..self.a {
            let o = inp.get_u16()?;
            if let Some(slot) = s.records.get_mut(i) {
                *slot = o;
            }
        }
        if v >= 37 {
            for i in 0..self.b {
                let o = inp.get_u16()?;
                if i == 0 {
                    s.bridge_records = o;
                }
            }
        }
        s.potholes = inp.get_u16()?;
        let e = if v >= 34 { inp.get_i32()?.max(0) as usize } else { 3 };
        for i in 0..e {
            let a = inp.get_u16()?;
            let mut b = inp.get_f32()?;
            let c = inp.get_i32()?;
            let d = inp.get_i32()?;
            let mut f = inp.get_f32()?;
            if v < 23 {
                b = pct(b);
                f = pct(f);
            }
            if let Some(slot) = s.switch_stats.get_mut(i) {
                *slot = SwitchStat { a, b, c, d, e: f };
            }
        }
        s.f1 = inp.get_f32()?;
        s.f2 = inp.get_f32()?;
        s.flag = inp.get_bool()?;
        s.f3 = inp.get_f32()?;
        s.byte = if v >= 21 { inp.get_u8()? } else { 0x80 };
        Some(s)
    }

    /// A damage block. The keys go, in file order, into fresh containers of
    /// 193 buckets (hint 100, `0x00829BC0`): each insert first grows the
    /// table to the listed prime ≥ count + 1 when that exceeds the bucket
    /// count, then puts the key at the head of its chain (`0x0060D270`,
    /// §6.2, TS54); so does the edge-density map of row 7 (`0x006D6FD0`).
    fn damage(inp: &mut dyn InStream) -> Option<DamageBlock> {
        let mut b = DamageBlock {
            cell_count: inp.get_u32()?,
            damaged_count: inp.get_u32()?,
            rate: inp.get_f32()?,
            ..DamageBlock::default()
        };
        for damaged in [false, true] {
            let n = inp.get_u32()?;
            for _ in 0..n {
                let k = u32::from(inp.get_u16()?);
                if damaged {
                    b.damaged.insert(k);
                } else {
                    b.intact.insert(k);
                }
            }
        }
        Some(b)
    }

    fn counters(&self, inp: &mut dyn InStream) -> Option<[u16; 4]> {
        let mut c = [0u16; 4];
        if self.v >= 36 {
            for i in [0, 2, 1, 3] {
                c[i] = inp.get_u16()?;
            }
        }
        Some(c)
    }

    fn terminal(&self, inp: &mut dyn InStream) -> Option<FerryTerminal> {
        if self.v < 32 {
            inp.get_u16()?;
        }
        let mut b = [0u16; 6];
        for v in b.iter_mut() {
            *v = u16::from(inp.get_u8()?);
        }
        let mut t = FerryTerminal {
            dep_x: b[0],
            dep_z: b[1],
            arr_x: b[2],
            arr_z: b[3],
            water_x: b[4],
            water_z: b[5],
            ferry_type: inp.get_u8()?,
            mask: inp.get_u16()?,
            riders_last: inp.get_u16()?,
            riders_current: inp.get_u16()?,
            ..FerryTerminal::default()
        };
        if self.v >= 38 {
            t.lot_x = u16::from(inp.get_u8()?);
            t.lot_z = u16::from(inp.get_u8()?);
        } else {
            t.lot_x = t.dep_x;
            t.lot_z = t.dep_z;
        }
        let n = inp.get_u32()?;
        for _ in 0..n {
            let mut l = TerminalLink {
                cost: inp.get_f32()?,
                arrival_x: inp.get_i16()? as u16,
                arrival_z: inp.get_i16()? as u16,
                to_key: inp.get_u16()?,
                ..TerminalLink::default()
            };
            get_vec(inp, &mut l.route)?;
            l.counters = self.counters(inp)?;
            t.links.push(l);
        }
        let n = inp.get_u32()?;
        for _ in 0..n {
            let mut l = NeighborLink {
                cost: inp.get_f32()?,
                off_x: inp.get_i16()?,
                off_z: inp.get_i16()?,
                edge_x: inp.get_i16()? as u16,
                edge_z: inp.get_i16()? as u16,
                ..NeighborLink::default()
            };
            get_vec(inp, &mut l.route_to)?;
            if self.v >= 31 {
                get_vec(inp, &mut l.route_from)?;
            }
            l.counters = self.counters(inp)?;
            t.neighbour_links.push(l);
        }
        Some(t)
    }

    fn switch(&self, inp: &mut dyn InStream) -> Option<(u32, TransitSwitch)> {
        let v = self.v;
        let id = inp.get_u32()?;
        let building = self.token(inp, ref_iid::BUILDING)?.unwrap_or(0);
        let mut sw = TransitSwitch {
            building,
            load: inp.get_f32()?,
            time_factor: inp.get_f32()?,
            capacity: inp.get_f32()?,
            ..TransitSwitch::default()
        };
        if v < 29 {
            // Discarded (the re-read of DoPostCityInit rebuilds the rules).
            let n = inp.get_u32()?;
            inp.get_bytes(n as usize)?;
        }
        sw.has_access_point = inp.get_bool()?;
        sw.access_x = inp.get_u8()?;
        sw.access_z = inp.get_u8()?;
        sw.counter_a = inp.get_f32()?;
        sw.counter_b = inp.get_f32()?;
        if v < 23 {
            sw.counter_b = pct(sw.counter_b);
        }
        if v >= 29 {
            let n = inp.get_u32()?;
            for _ in 0..n {
                let kind = u8::from(inp.get_bool()?);
                let mut side = [false; 4];
                for s in side.iter_mut() {
                    *s = inp.get_bool()?;
                }
                let from = inp.get_u16()? as u8;
                let to = inp.get_u16()? as u8;
                sw.rules.push(Rule { kind, side, from, to });
            }
        }
        if v >= 33 {
            sw.simulator_value = inp.get_u32()?;
            sw.counts_as_intersection = inp.get_bool()?;
        }
        let (mask, flags) = read_mask_and_flags(&sw.rules);
        sw.to_mask = mask;
        sw.flags = flags;
        Some((id, sw))
    }

    /// The record in order (save-records §7.1.1).
    fn record(&mut self, inp: &mut dyn InStream) -> Option<()> {
        let v = self.v;
        self.a = if v >= 24 { inp.get_i32()?.max(0) as usize } else { 8 };
        self.b = if v >= 37 { inp.get_i32()?.max(0) as usize } else { 1 };
        self.c = if v >= 28 { inp.get_i32()?.max(0) as usize } else { 4 };
        self.d = if v >= 28 { inp.get_i32()?.max(0) as usize } else { 7 };
        self.s.air = self.grid8(inp)?;
        self.s.peak_volume = self.gridf(inp)?;
        self.s.commercial = self.grid8(inp)?;
        self.s.commercial_work = self.grid8(inp)?;
        self.s.trip_length = self.grid8(inp)?;
        self.s.congestion = self.grid8(inp)?;
        let rows = inp.get_u32()?;
        let cols = inp.get_u32()?;
        let bytes = inp.get_bytes(rows as usize * cols as usize)?;
        self.s.dest_rows = rows as i32;
        self.s.dest_cols = cols as i32;
        self.s.dest = bytes.into_iter().map(std::cell::Cell::new).collect();
        // The volume grids: slots the record does not have keep theirs
        // *[choice]*; extra ones are dropped.
        let n = inp.get_u32()?;
        for i in 0..n as usize {
            let g = self.gridf(inp)?;
            if let Some(slot) = self.s.volume.get_mut(i) {
                *slot = g;
            }
        }
        self.s.float_grid = self.gridf(inp)?;
        self.s.peak_ratio = self.gridf(inp)?;
        let n = inp.get_u32()?;
        for _ in 0..n {
            let key = inp.get_u32()?;
            let mut r = TrafficEdgeDensity::default();
            if v >= 20 {
                for x in r.last.iter_mut() {
                    *x = inp.get_u16()?;
                }
                r.usage = inp.get_u16()?;
                for x in r.current.iter_mut() {
                    *x = inp.get_u16()?;
                }
            } else {
                for x in r.last.iter_mut() {
                    *x = legacy_density(inp.get_f32()?);
                }
                r.usage = legacy_density(inp.get_f32()?);
                for x in r.current.iter_mut() {
                    *x = legacy_density(inp.get_f32()?);
                }
                inp.get_f32()?;
            }
            *self.n.edge.entry(key, TrafficEdgeDensity::default) = r;
        }
        for k in 0..3 {
            self.s.job_capacity[k] = inp.get_u32()?;
            self.s.attempted[k] = inp.get_u32()?;
            self.s.routed[k] = inp.get_u32()?;
        }
        self.n.switch_mode = self.grid8(inp)?;
        self.n.switch_id = self.grid32(inp)?;
        self.n.next_switch_id = inp.get_u32()?;
        let n = inp.get_u32()?;
        for _ in 0..n {
            let (id, sw) = self.switch(inp)?;
            *self.n.switches.entry(id, TransitSwitch::default) = sw;
        }
        self.s.roads = self.token(inp, ref_iid::DEPARTMENT)?;
        self.s.mass_transit = self.token(inp, ref_iid::DEPARTMENT)?;
        self.s.budget_total = inp.get_i64()?;
        self.s.transit_income = inp.get_f32()?;
        self.s.toll_income = if v >= 41 { inp.get_f32()? } else { 0.0 };
        self.s.last_monthly_income = inp.get_f32()?;
        let acc = if v >= 22 {
            inp.get_f64()?
        } else {
            f64::from(inp.get_f32()?)
        };
        self.s.income_acc.set(acc);
        self.s.fee_acc = if v >= 41 { inp.get_f64()? } else { 0.0 };
        self.s.cycle_start_date = inp.get_u32()?;
        self.s.strike_state = inp.get_u8()?;
        self.s.strike_months = inp.get_i32()?;
        self.dmg.road = Self::damage(inp)?;
        if v >= 26 {
            self.dmg.rail = Self::damage(inp)?;
        }
        self.s.cycle_state = inp.get_u8()?;
        (self.s.scan, self.s.extras.scan_state_byte) = Self::iterator(inp)?;
        self.n.intersection_dirty = inp.get_bool()?;
        self.n.intersection_rect = CellRect::new(inp.get_i32()?, inp.get_i32()?, inp.get_i32()?, inp.get_i32()?);
        let e = &mut self.s.extras;
        e.network_changed_byte = inp.get_bool()?;
        self.s.strategy_bucket_counter = inp.get_u32()?;
        self.s.strategy_bucket = inp.get_u32()?;
        let e = &mut self.s.extras;
        if v >= 25 {
            e.phase_positions = [inp.get_i16()?, inp.get_i16()?];
        } else {
            e.phase_positions = [i16::from(inp.get_u8()?), i16::from(inp.get_u8()?)];
        }
        let li = inp.get_bytes(30)?;
        e.lot_iter.copy_from_slice(&li);
        e.context_bools = [inp.get_bool()?, inp.get_bool()?];
        e.context_f32 = inp.get_f32()?;
        e.context_x = inp.get_i32()?;
        e.context_z = inp.get_i32()?;
        e.context_u32 = inp.get_u32()?;
        for x in e.context_u32x4.iter_mut() {
            *x = inp.get_u32()?;
        }
        e.context_u8 = inp.get_u8()?;
        e.context_u32b = inp.get_u32()?;
        for x in e.context_i32x4.iter_mut() {
            *x = inp.get_i32()?;
        }
        let d = self.d;
        let e = &mut self.s.extras;
        e.table = [[0; 9]; 3];
        for row in e.table.iter_mut() {
            for j in 0..d {
                let x = inp.get_u32()?;
                if let Some(slot) = row.get_mut(j) {
                    *slot = x;
                }
            }
        }
        e.context_flags = [inp.get_u8()?, inp.get_u8()?];
        self.s.processed = inp.get_i32()?;
        // `dayIndex`, the s32 after `processed` (TS60).
        self.s.day_index = inp.get_i32()? as u32;
        self.s.total = if v >= 25 { inp.get_i32()? } else { 0 };
        self.s.days = inp.get_u32()?;
        self.s.extras.pacing_unused = [inp.get_u32()?, inp.get_u32()?, inp.get_u32()?];
        if v >= 25 {
            self.s.pass_total = inp.get_i32()?;
            self.s.to_reroute = inp.get_i32()?;
            self.s.to_search = inp.get_i32()?;
            for i in 0..4 {
                self.s.pass_counters[i] = inp.get_u32()?;
            }
        } else {
            self.s.pass_total = 0;
            self.s.to_reroute = 0;
            self.s.to_search = 0;
            for i in 0..4 {
                self.s.pass_counters[i] = 0;
            }
        }
        for i in 4..10 {
            self.s.pass_counters[i] = if v >= 40 { inp.get_u32()? } else { 0 };
        }
        self.s.stats_last = self.stats(inp)?;
        self.s.stats_work = self.stats(inp)?;
        let n = inp.get_u32()?;
        for _ in 0..n {
            let key = u32::from(inp.get_u16()?);
            let ox = i32::from(inp.get_u8()?);
            let oz = i32::from(inp.get_u8()?);
            let mask = inp.get_u16()?;
            let net_type = inp.get_u16()? as u8;
            let mut cost = [0.0f32; 9];
            for j in 0..self.d {
                let c = inp.get_f32()?;
                if let Some(slot) = cost.get_mut(j) {
                    *slot = c;
                }
            }
            *self.n.tunnels.entry(key, TunnelInfo::default) = TunnelInfo {
                other: (ox, oz),
                mask,
                cost,
                net_type,
            };
        }
        self.s.avg_car_commute = inp.get_f32()?;
        self.s.car_len_sum = inp.get_f32()?;
        self.s.car_len_count = inp.get_u32()?;
        if v >= 21 {
            self.s.freight_len_sum = inp.get_f32()?;
            self.s.freight_count = inp.get_u32()?;
        } else {
            self.s.freight_len_sum = 0.0;
            self.s.freight_count = 0;
        }
        self.s.cells_per_type = [0; 13];
        for i in 0..self.a {
            let c = inp.get_u32()?;
            if let Some(slot) = self.s.cells_per_type.get_mut(i) {
                *slot = c;
            }
        }
        let n = inp.get_u32()?;
        let mut free = [u32::MAX; 4];
        for i in 0..n as usize {
            let f = inp.get_u32()?;
            if let Some(slot) = free.get_mut(i) {
                *slot = f;
            }
        }
        for (k, f) in free.iter().enumerate() {
            self.s.set_free(k, *f);
        }
        // The neighbour pools. A repeated ID keeps the last entry (TS41:
        // stored through the map's find-or-insert, `0x00714560` called at
        // `0x00722FA2`; the original never releases the earlier holder,
        // which nothing can reach).
        let n = inp.get_u32()?;
        let mut map = BTreeMap::new();
        for _ in 0..n {
            let key = inp.get_u16()?;
            let mut holder = PropertyHolder::new();
            if !holder.read(inp) {
                return None;
            }
            map.insert(key, holder);
        }
        self.pools = map;
        if v >= 27 {
            // (published, building) pairs (`0x00722FD0`, TS56).
            for i in 0..self.d {
                let published = inp.get_u32()?;
                let building = inp.get_u32()?;
                if i < 9 {
                    self.s.type_count_published[i] = published;
                    self.s.type_count_building[i] = building;
                }
            }
        }
        self.s.ferry_total = if v >= 39 { inp.get_u32()? } else { 0 };
        if v >= 30 {
            let n = inp.get_u32()?;
            for _ in 0..n {
                let t = self.terminal(inp)?;
                self.n.ferries.push(t);
            }
        }
        Some(())
    }
}

/// `× 0.01f` of the v < 23 records.
fn pct(v: f32) -> f32 {
    crate::x87::r24(f64::from(v) * f64::from(0.01f32)) as f32
}

/// A record parsed as `Read` parses it, without committing it anywhere
/// (tooling: the traffic shadow reads the original's records with it). The
/// state starts as a copy of `base`: the fields the record does not hold
/// keep `base`'s values.
#[allow(dead_code)] // the shadow reads `state`; the rest is for diagnostics
pub struct ParsedRecord {
    pub version: u16,
    pub state: State,
    /// WorkOnSimCycle's prelude can clear this without advancing a phase.
    pub intersection_dirty: bool,
    /// The edge-density records (row 7), in the record's order.
    pub edge_density: StlHashMap<TrafficEdgeDensity>,
    /// The switch count (row 10) and the next switch ID.
    pub switches: usize,
    pub next_switch_id: u32,
    pub damage: DamageState,
    /// The neighbour pools (row 19).
    pub pools: usize,
    pub ferries: usize,
    pub tunnels: usize,
}

/// Parses a record with `h` resolving its references (see
/// [`ParsedRecord`]); `None` when `Read` would fail.
pub fn parse_record(h: &dyn TrafficSimHost, base: &State, inp: &mut dyn InStream) -> Option<ParsedRecord> {
    let v = inp.get_u16()?;
    if v < MIN_RECORD_VERSION {
        return None;
    }
    let mut p = Parse {
        h,
        v,
        a: 13,
        b: 1,
        c: 13,
        d: 9,
        s: base.clone(),
        n: NetPart {
            switch_mode: None,
            switch_id: None,
            next_switch_id: 1,
            switches: StlHashMap::new(),
            edge: StlHashMap::new(),
            tunnels: StlHashMap::new(),
            ferries: Vec::new(),
            intersection_dirty: false,
            intersection_rect: CellRect::new(0, 0, 0, 0),
        },
        dmg: DamageState::default(),
        pools: BTreeMap::new(),
    };
    if p.record(inp).is_none() || inp.error() != 0 {
        return None;
    }
    Some(ParsedRecord {
        version: v,
        intersection_dirty: p.n.intersection_dirty,
        switches: p.n.switches.len(),
        next_switch_id: p.n.next_switch_id,
        edge_density: p.n.edge,
        state: p.s,
        damage: p.dmg,
        pools: p.pools.len(),
        ferries: p.n.ferries.len(),
        tunnels: p.n.tunnels.len(),
    })
}

impl Run<'_, '_> {
    /// `Read` (`0x00721990`): parses into copies, commits on success.
    fn read_record(&mut self, inp: &mut dyn InStream) -> bool {
        let Some(v) = inp.get_u16() else { return false };
        if v < MIN_RECORD_VERSION {
            return false;
        }
        let mut p = Parse {
            h: self.h,
            v,
            a: 13,
            b: 1,
            c: 13,
            d: 9,
            s: self.st.clone(),
            n: NetPart {
                switch_mode: None,
                switch_id: None,
                next_switch_id: 1,
                switches: StlHashMap::new(),
                edge: StlHashMap::new(),
                tunnels: StlHashMap::new(),
                ferries: Vec::new(),
                intersection_dirty: false,
                intersection_rect: CellRect::new(0, 0, 0, 0),
            },
            dmg: DamageState::default(),
            pools: BTreeMap::new(),
        };
        if p.record(inp).is_none() || inp.error() != 0 {
            // Release what the parse resolved.
            for (_, sw) in p.n.switches.iter() {
                if sw.building != 0 {
                    self.cx.h.release(sw.building);
                }
            }
            return false;
        }
        // The commit.
        let old_buildings: Vec<u64> = self
            .cx
            .s
            .switches
            .iter()
            .map(|(_, s)| s.building)
            .filter(|&b| b != 0)
            .collect();
        let old_depts: Vec<u64> = [self.st.roads, self.st.mass_transit]
            .into_iter()
            .flatten()
            .filter(|d| Some(*d) != p.s.roads && Some(*d) != p.s.mass_transit)
            .collect();
        // The running source iterator's connection references (§9.1 TG19)
        // go with the state the record replaces.
        let old_conns = std::mem::take(&mut self.st.iter.conns);
        self.hold_source(None);
        *self.st = p.s;
        self.st.loaded = true;
        let s = &mut *self.cx.s;
        s.switch_mode = p.n.switch_mode;
        s.switch_id = p.n.switch_id;
        s.next_switch_id = p.n.next_switch_id;
        s.switches = p.n.switches;
        s.edge_density = p.n.edge;
        s.tunnels = p.n.tunnels;
        s.ferries = p.n.ferries;
        s.intersection_dirty = p.n.intersection_dirty;
        s.intersection_rect = p.n.intersection_rect;
        // The terminal tables and the counts by type (types 0 and 1).
        s.ferry_departures = StlHashMap::new();
        s.ferry_arrivals = StlHashMap::new();
        s.ferry_counts = [0; 2];
        for (pos, t) in s.ferries.iter().enumerate() {
            s.ferry_departures.entry(u32::from(t.departure_key()), || pos as u32);
            s.ferry_arrivals.entry(u32::from(t.arrival_key()), || pos as u32);
            if let Some(c) = s.ferry_counts.get_mut(usize::from(t.ferry_type)) {
                *c += 1;
            }
        }
        *self.dmg.borrow_mut() = p.dmg;
        // The pools are replaced; the default pool is Init's.
        self.pools.borrow_mut().map = p.pools;
        for b in old_buildings {
            self.cx.h.release(b);
        }
        for c in old_conns {
            self.cx.h.release(c);
        }
        for d in old_depts {
            self.h.release_department(d);
        }
        true
    }
}

impl TrafficSimulator {
    /// `Write` (`0x00712400`): the version-41 record. Returns `!GetError()`.
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        self.run(|r| {
            let d = r.dmg.borrow().clone();
            let pools = r.pools.borrow().clone();
            write_record(out, r.h, r.st, r.cx.s, &d, &pools);
            true
        }) && out.error() == 0
    }

    /// `Read` (`0x00721990`): versions 19…41 (save-records §7.1.1); newer
    /// versions read as 41 (§6.4, TS32). Below 19 it returns false having
    /// read only the version; a failed Read leaves the simulator as it was
    /// *[choice]* (§6.1, TS30).
    pub fn read(&self, inp: &mut dyn InStream) -> bool {
        self.run(|r| r.read_record(inp))
    }
}

#[cfg(test)]
mod iterator_tests {
    use super::*;
    use crate::stream::{ByteReader, ByteWriter};

    fn round(it: &ScanIterator, raw: Option<u8>) -> Vec<u8> {
        let mut w = ByteWriter::new();
        put_iterator(&mut w, it, raw);
        w.into_bytes()
    }

    /// §6.1 "The scanning iterator in row 14" (TS31): state byte 7, current
    /// (5, 3), step 1, rectangle [0, 63]²: the next GetNext returns (6, 3)
    /// and the byte is written back unchanged; a byte of 2 ends at once.
    #[test]
    fn unknown_state_byte_acts_as_running() {
        let mut it = ScanIterator::new(0, 0, 63, 63);
        it.x = 5;
        it.z = 3;
        it.state = ScanState::Running;
        let mut bytes = round(&it, None);
        assert_eq!(bytes[0], 1);
        bytes[0] = 7;
        let (mut got, raw) = Parse::iterator(&mut ByteReader::new(&bytes)).unwrap();
        assert_eq!((got.state, raw), (ScanState::Running, Some(7)));
        assert_eq!(got.next_cell(), Some((6, 3)));
        assert_eq!(round(&got, raw)[0], 7);
        // A finished scan writes 2, a reset one 0.
        got.state = ScanState::Done;
        assert_eq!(round(&got, raw)[0], 2);
        got.reset(0, 0);
        assert_eq!(round(&got, None)[0], 0);
        bytes[0] = 2;
        let (mut done, raw) = Parse::iterator(&mut ByteReader::new(&bytes)).unwrap();
        assert_eq!((done.state, raw), (ScanState::Done, None));
        assert_eq!(done.next_cell(), None);
        for b in [0u8, 1, 3, 255] {
            bytes[0] = b;
            let (it, raw) = Parse::iterator(&mut ByteReader::new(&bytes)).unwrap();
            let want = if b == 0 { ScanState::Fresh } else { ScanState::Running };
            assert_eq!(it.state, want, "byte {b}");
            assert_eq!(round(&it, raw)[0], b);
        }
    }
}
