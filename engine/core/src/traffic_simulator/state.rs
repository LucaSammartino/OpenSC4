// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The simulator's data model (§2). Plain data, so hosts and tests can
//! inspect, save and restore it.

use std::cell::Cell;
use std::collections::{BTreeMap, HashSet};
use std::rc::Rc;

use crate::grid::ScanIterator;
use crate::pathfinder::{Pathfinder, SharedWorkCheck};
use crate::rng::{CrtRand, FixedSeeds, RzRandomSuper};
use crate::stl::{StlHashSet, STL_DEFAULT_HINT};

use super::tuning::SimTuning;
use super::{cell_key16, GridF32, GridU8, Source};

/// The re-route counter: +1 per search ReRouteTrips finds (§4.4, TS21);
/// kept by the pacing reset on entering 16, zeroed on entering 17 (§3.3,
/// TS1).
pub const PC_REROUTE_COUNTER: usize = 1;
/// The pass counter that holds `rerouteBucket` (§4.3, save row 15).
pub const PC_REROUTE_BUCKET: usize = 2;
/// The pass counter that holds the source counter (§4.3).
pub const PC_SOURCE_COUNTER: usize = 3;
/// Routes confirmed without a search: ConfirmTrips, each status := 3 after
/// `ConfirmRoute` (`0x0071DBF5`, §10 Q4, TS59).
pub const PC_CONFIRMED: usize = 0;
/// Destination full: ConfirmTrips' status 3 → 0 (`0x0071DC01`) and
/// ReRouteTrips' capacity send-backs, status 1 → 0 (`0x0071D221`; §4.4,
/// TS21, TS59).
pub const PC_CAPACITY_SEND_BACKS: usize = 4;
/// The ConfirmTrips merges: per routed trip that absorbed at least one
/// trip (`0x0071DA0D`, §4.3, TS55, TS59).
pub const PC_MERGES: usize = 5;
/// Return legs not confirmed: ConfirmTrips, the outbound route confirmed
/// but `ConfirmRoute(returnRoute)` false (`0x0071DBA0`, TS59).
pub const PC_RETURN_NOT_CONFIRMED: usize = 6;
/// The status-0 trips a ConfirmTrips source lists before its merge
/// (`0x0071D95D`, every source, TS59).
pub const PC_SEARCH_TRIPS_MET: usize = 7;
/// Failures retried: ConfirmTrips, status 5 → 0 (`0x0071DC11`, TS59).
pub const PC_FAILURES_RETRIED: usize = 8;
/// FullTrips' splits (`0x0071D75A`/`0x0071D77F`, §4.4, TS21, TS59).
pub const PC_SPLITS: usize = 9;
/// The number of pass counters saved (§6.1 row 15).
pub const PASS_COUNTERS: usize = 10;

/// One accident-prone cell (§4.9 step 3).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AccidentEntry {
    pub x: i32,
    pub z: i32,
    /// The accident probability `p2`.
    pub p: f32,
}

/// One row of the per-network-type statistics (§4.9 step 3 "statistics",
/// TS9; save-records §7.1.1).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StatRow {
    /// The sum of the volume/capacity ratios (the working set) or their
    /// average (the published set).
    pub sum: f32,
    /// The count of records with `r > 1.0` (the saved u32 of the row).
    pub congested: u32,
    pub max_x: i32,
    pub max_z: i32,
    pub max: f32,
}

impl StatRow {
    /// The state-2 entry reset (`0x00721196`–`0x007211D6`, §4.9, TS40): the
    /// five fields 0. The −1.0 start is the separate comparison value
    /// ([`State::stat_max_cmp`]), not the row's `max`.
    pub const RESET: StatRow = StatRow {
        sum: 0.0,
        congested: 0,
        max_x: 0,
        max_z: 0,
        max: 0.0,
    };
}

/// The bridge slot of the statistics (§4.9, TS9): records whose network
/// info has flag bit 0 (network flag `0x40000`).
pub const BRIDGE_SLOT: usize = 13;

/// One per-switch-type record of the statistics block (save-records §7.1.1:
/// `u16, f32, s32, s32, f32`). Not computed by the code of this spec; kept
/// and saved unchanged.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SwitchStat {
    pub a: u16,
    pub b: f32,
    pub c: i32,
    pub d: i32,
    pub e: f32,
}

/// The number of per-switch-type records (save-records §7.1.1, `E = 12`).
pub const SWITCH_STATS: usize = 12;

/// A statistics block (§4.9, the slot-34 `TrafficStats`; §6.1 row 16).
#[derive(Clone, Debug, PartialEq)]
pub struct StatsSet {
    /// One row per network type.
    pub rows: [StatRow; 13],
    /// The bridge row (slot 13, TS9), published by the traffic advisor as
    /// the bridge congestion.
    pub bridge: StatRow,
    /// The network-info records per type (u16, wrapping); in the published
    /// set these are also the cell counts (§4.9 end of pass, TS9).
    pub records: [u16; 13],
    /// The bridge records (never published).
    pub bridge_records: u16,
    /// The u16 after the bridge records: in the published set the pothole
    /// count (`0x134`, §3.6, §4.12); the working set's is saved unchanged.
    pub potholes: u16,
    /// The unnamed fields of the block (save-records §12 Q2), saved
    /// unchanged.
    pub switch_stats: [SwitchStat; SWITCH_STATS],
    pub f1: f32,
    pub f2: f32,
    pub flag: bool,
    pub f3: f32,
    pub byte: u8,
}

impl Default for StatsSet {
    fn default() -> Self {
        StatsSet {
            rows: [StatRow::default(); 13],
            bridge: StatRow::default(),
            records: [0; 13],
            bridge_records: 0,
            potholes: 0,
            switch_stats: [SwitchStat::default(); SWITCH_STATS],
            f1: 0.0,
            f2: 0.0,
            flag: false,
            f3: 0.0,
            byte: 0,
        }
    }
}

/// The damage bookkeeping of one kind (road or rail, §4.12): the cell
/// counts, the damage rate and the two SGI hash sets of u16 cell keys.
#[derive(Clone, Debug)]
pub struct DamageBlock {
    pub cell_count: u32,
    pub damaged_count: u32,
    pub rate: f32,
    pub intact: StlHashSet,
    pub damaged: StlHashSet,
}

impl Default for DamageBlock {
    /// Empty sets with the default bucket hint *[choice: the hint is not
    /// stated]*.
    fn default() -> Self {
        DamageBlock {
            cell_count: 0,
            damaged_count: 0,
            rate: 0.0,
            intact: StlHashSet::with_hint(STL_DEFAULT_HINT),
            damaged: StlHashSet::with_hint(STL_DEFAULT_HINT),
        }
    }
}

impl DamageBlock {
    /// A setup helper for hosts restoring state and for tests, not the
    /// occupant bookkeeping (that is §3.6, `DoRoadChanged`): the key joins
    /// the intact set unless it is in either set already, and `cellCount`
    /// becomes the size of the two sets.
    pub fn insert_cell(&mut self, key: u16) {
        let k = u32::from(key);
        if !self.intact.contains(k) && !self.damaged.contains(k) {
            self.intact.insert(k);
        }
        self.cell_count = (self.intact.len() + self.damaged.len()) as u32;
    }
}

/// The damage state of both kinds (§4.12). Shared with the network
/// adapter, which answers `IsRoadDamaged` from it.
#[derive(Clone, Debug, Default)]
pub struct DamageState {
    pub road: DamageBlock,
    pub rail: DamageBlock,
}

impl DamageState {
    /// `IsRoadDamaged(x, z)` (slot 16, §4.13): membership of the 16-bit key.
    pub fn is_road_damaged(&self, x: i32, z: i32) -> bool {
        self.road.damaged.contains(u32::from(cell_key16(x, z)))
    }
}

/// The traffic-source iterator (`Reset` `0x007107A0`, `GetNext`
/// `0x007107F0`; §4.1, TS20): all lots in the lot manager's list order, then
/// all neighbour connections. The connections are a snapshot taken by
/// `Reset`; the lots are walked live, so a lot removed before the walk
/// reaches it is skipped and a lot added before the cursor reaches the
/// sentinel is visited (zones §8.3.1).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SourceIter {
    /// The connections the last `Reset` listed (counted references in the
    /// original: a connection removed during the phase is still visited).
    pub conns: Vec<u64>,
    /// The next connection to return.
    pub conn_pos: usize,
    /// The lot list as the walk last read it (the host's own list while
    /// that is unchanged: [`Host::lots_shared`](super::TrafficSimHost::lots_shared)).
    pub lots_seen: Rc<[u64]>,
    /// The last lot returned; `None` before the first.
    pub last_lot: Option<u64>,
    /// The cursor has reached the sentinel, including while the last lot
    /// returned is still being worked on.
    pub lots_done: bool,
    /// A cache: the first position of `last_lot` in `lots_seen` (`None`
    /// before the first lot or when it is not listed).
    last_index: Option<usize>,
    /// A cache: `lots_seen` lists some lot twice.
    seen_dups: bool,
}

impl SourceIter {
    /// `Reset`: the connection snapshot; the lot walk back at the start.
    pub fn reset(&mut self, conns: Vec<u64>) {
        *self = SourceIter {
            conns,
            ..SourceIter::default()
        };
    }

    /// The next lot of the live walk, given the lot manager's list now. The
    /// list only appends (zones.md §8.1) and removal keeps the order, so the
    /// lot after the last one returned is the first lot after it, in the
    /// list as last read, that is still listed; else the first lot appended
    /// since. `None` (and the walk leaves the lots) at the end.
    ///
    /// The list is normally the one the walk last read: then every lot
    /// listed was seen and every lot seen is listed, and (no lot listed
    /// twice) the rule gives the lot right after the last one, found from
    /// the cached position in O(1). Any other list takes the general rule.
    pub fn next_lot(&mut self, lots: impl Into<Rc<[u64]>>) -> Option<u64> {
        if self.lots_done {
            return None;
        }
        let lots: Rc<[u64]> = lots.into();
        let same = Rc::ptr_eq(&lots, &self.lots_seen) || *lots == *self.lots_seen;
        let fast = same && !self.seen_dups;
        let next = match self.last_lot {
            None => lots.first().copied(),
            Some(_) if fast => self.last_index.and_then(|i| self.lots_seen.get(i + 1).copied()),
            Some(last) => {
                let now: HashSet<u64> = lots.iter().copied().collect();
                let seen: HashSet<u64> = self.lots_seen.iter().copied().collect();
                let after = match self.lots_seen.iter().position(|&l| l == last) {
                    Some(i) => &self.lots_seen[i + 1..],
                    None => &[][..],
                };
                after
                    .iter()
                    .copied()
                    .find(|l| now.contains(l))
                    .or_else(|| lots.iter().copied().find(|l| !seen.contains(l)))
            }
        };
        if !same {
            let mut set = HashSet::with_capacity(lots.len());
            self.seen_dups = !lots.iter().all(|&l| set.insert(l));
        }
        // The host's list is kept even when equal, so that the next step
        // finds the same list by pointer.
        self.lots_seen = lots;
        match next {
            Some(l) => {
                self.last_index = match (self.last_lot, self.last_index) {
                    (Some(_), Some(i)) if fast => Some(i + 1),
                    _ => self.lots_seen.iter().position(|&x| x == l),
                };
                self.last_lot = Some(l);
                // zones §8.3.1: the safe iterator advances BEFORE yielding
                // a lot. Yielding the tail leaves it at the sentinel now,
                // not on the next call. A frame may append another lot
                // while this source is held across a sliced traffic day;
                // that lot belongs to the next pass.
                self.lots_done = self.last_index == Some(self.lots_seen.len() - 1);
            }
            None => {
                if !fast {
                    let last = self.last_lot;
                    self.last_index = last.and_then(|last| self.lots_seen.iter().position(|&x| x == last));
                }
                self.lots_done = true;
            }
        }
        next
    }

    /// The next connection of the snapshot.
    pub fn next_connection(&mut self) -> Option<u64> {
        let c = self.conns.get(self.conn_pos).copied()?;
        self.conn_pos += 1;
        Some(c)
    }
}

/// The record fields whose meaning is not specified (save-records §12 Q2):
/// Read keeps them and Write writes them back unchanged.
#[derive(Clone, Debug, PartialEq)]
pub struct SaveExtras {
    /// The bool after the intersection dirty rectangle: the network-changed
    /// byte (§3.1, §6.1 row 14, TS5). Set by SimBegin and by every network
    /// occupant message, cleared by the constructor, saved and loaded, read
    /// by nothing.
    pub network_changed_byte: bool,
    /// The raw state byte of a loaded running scan iterator (§6.1 "The
    /// scanning iterator in row 14", TS31): any byte other than 0 (fresh)
    /// and 2 (done) acts as running and is written back unchanged until the
    /// scan ends or the iterator is reset. `None` writes the iterator's own
    /// byte.
    pub scan_state_byte: Option<u8>,
    /// The two phase positions (s16; u8 before version 25), the saved
    /// source or lot cell x, z (§6.1 row 14, +0x90C/+0x90E; TS44, TS45).
    /// The trip phases store the cell of the source they take here and
    /// FinishTripMap each lot's location; the 16–18 entry action zeroes
    /// them; the DoPostCityInit resume (§3.5 step 11) looks for them.
    pub phase_positions: [i16; 2],
    /// The lot iterator position (30 bytes: s8, s8, 7 × s32).
    pub lot_iter: [u8; 30],
    /// The two bools after the lot iterator position.
    pub context_bools: [bool; 2],
    /// The f32 of the re-route context.
    pub context_f32: f32,
    /// The re-route context cell (`+0x83C`, `+0x840`; §6.1 row 14, TS60):
    /// the cell of the traffic source being worked, the s16 phase positions
    /// widened, written as each source is taken in 16, 17 and 18
    /// (`0x0071E26D`, `0x0071D104`, `0x0071D523`); not reset on phase entry.
    pub context_x: i32,
    pub context_z: i32,
    /// The u32 after the context cell (not a count).
    pub context_u32: u32,
    pub context_u32x4: [u32; 4],
    pub context_u8: u8,
    pub context_u32b: u32,
    pub context_i32x4: [i32; 4],
    /// The `3 × D` u32 table (rows zero-filled to 9).
    pub table: [[u32; 9]; 3],
    /// The two context flag bytes.
    pub context_flags: [u8; 2],
    /// The three u32 after `days` (`+0x930`, `+0x934`, `+0x938`; §6.1 row
    /// 15, TS60): no code reads or writes them besides Write and Read.
    pub pacing_unused: [u32; 3],
}

impl Default for SaveExtras {
    fn default() -> Self {
        SaveExtras {
            network_changed_byte: false,
            scan_state_byte: None,
            phase_positions: [0; 2],
            lot_iter: [0; 30],
            context_bools: [false; 2],
            context_f32: 0.0,
            context_x: 0,
            context_z: 0,
            context_u32: 0,
            context_u32x4: [0; 4],
            context_u8: 0,
            context_u32b: 0,
            context_i32x4: [0; 4],
            table: [[0; 9]; 3],
            context_flags: [0; 2],
            pacing_unused: [0; 3],
        }
    }
}

/// The traffic simulator's state (§2).
#[derive(Clone)]
pub struct State {
    /// The reference count (slots 1, 2).
    pub ref_count: u32,
    /// Whether Init ran (and no Shutdown since).
    pub initialised: bool,
    /// The tuning values (§4.14).
    pub tuning: SimTuning,

    // ---- grids (§1 Init step 4) ----
    pub air: Option<GridU8>,
    pub peak_volume: Option<GridF32>,
    pub commercial: Option<GridU8>,
    pub commercial_work: Option<GridU8>,
    pub unused: Option<GridU8>,
    pub trip_length: Option<GridU8>,
    /// One volume grid per transit network 0…5.
    pub volume: Vec<Option<GridF32>>,
    pub float_grid: Option<GridF32>,
    pub congestion: Option<GridU8>,
    pub peak_ratio: Option<GridF32>,

    // ---- destinations (§2, §4.2, §4.11) ----
    /// The destination map's size: `W + 2` rows of `H + 2` bytes.
    pub dest_rows: i32,
    pub dest_cols: i32,
    /// The destination bitmask map, `[x + 1][z + 1]`, x outer. The
    /// pathfinder writes it through its host, hence the cells.
    pub dest: Vec<Cell<u8>>,
    /// `freeDest[k]`, also written by the pathfinder.
    pub free_dest: [Cell<u32>; 4],
    /// `jobCapacity[k]` (§4.11).
    pub job_capacity: [u32; 3],
    /// `attempted[k]`, `routed[k]` (§4.6).
    pub attempted: [u32; 3],
    pub routed: [u32; 3],

    // ---- accidents (§4.9, §4.13) ----
    /// The accident-prone list being built by CreateMaps.
    pub accidents: Vec<AccidentEntry>,
    /// The copy the accident check walks.
    pub accident_check: Vec<AccidentEntry>,
    /// The next accident check, in milliseconds of the host's simulation
    /// clock (§4.13 *[choice]*).
    pub next_accident_check: u64,

    // ---- statistics (§4.9) ----
    /// The published ("last") set: slot 34.
    pub stats_last: StatsSet,
    /// The working set.
    pub stats_work: StatsSet,
    /// The per-slot record counts `count[s]` (u32, s = 0…13) the averages
    /// divide by (§4.9). Not saved: CreateMaps runs in one call.
    pub stat_counts: [u32; 14],
    /// The per-slot comparison value a record's `r` must beat to write the
    /// row's max fields (§4.9 step 1, TS40): −1.0 after the state-2 entry
    /// reset, then the largest `r` so far. Separate from the row's `max`,
    /// which the reset zeroes. Not saved *[choice: −1.0 at construction]*.
    pub stat_max_cmp: [f32; 14],
    /// `cells[t]`, the cell counts per network type (§3.6, saved in §6.1 row
    /// 18): bumped by the occupant messages, replaced by the record counts
    /// at the end of every CreateMaps pass (§4.9, TS9).
    pub cells_per_type: [u32; 13],

    // ---- money (§3.3, §4.5, §4.12) ----
    /// The int64 budget-item total `E` (§4.1.3).
    pub budget_total: i64,
    pub transit_income: f32,
    pub toll_income: f32,
    pub last_monthly_income: f32,
    /// The income accumulator, also written by the pathfinder.
    pub income_acc: Cell<f64>,
    /// The fee accumulator.
    pub fee_acc: f64,

    // ---- commute (§4.6, §4.12) ----
    pub avg_car_commute: f32,
    pub car_len_sum: f32,
    pub car_len_count: u32,
    pub freight_len_sum: f32,
    pub freight_count: u32,
    /// The per-travel-type trip counts being built (FullTrips, `+0x430`) and
    /// published (`+0x424`): the end of FullTrips swaps the two (§4.4,
    /// TS56). `freightByte` is the published statistics set's last byte
    /// ([`StatsSet::byte`] of `stats_last`, §4.12, TS61).
    pub type_count_building: [u32; 9],
    pub type_count_published: [u32; 9],
    /// The ferry riders total (§4.12 history).
    pub ferry_total: u32,

    // ---- cycle (§3) ----
    pub cycle_state: u8,
    /// The scanning iterator over the city rect.
    pub scan: ScanIterator,
    pub cycle_start_date: u32,
    /// The last seen day of month (−1 after construction).
    pub last_day: i32,

    // ---- pacing (§3.3) ----
    /// `dayIndex`, the s32 right after `processed` (§6.1 row 15, **`+0x918`**;
    /// TS60). It counts the days of the running trip phase, so with `days` 1
    /// or 2 it toggles 0/1 (§3.3). `processed` is `+0x914` and `total` is
    /// `+0x91C`.
    /// `dayIndex` (`+0x918`): saved as the s32 right after `processed`
    /// (§6.1 row 15, TS60).
    pub day_index: u32,
    pub days: u32,
    pub total: i32,
    pub processed: i32,
    pub pass_total: i32,
    pub to_reroute: i32,
    pub to_search: i32,
    /// The 10 pass counters; `[2]` = `rerouteBucket`, `[3]` = the source
    /// counter (§10 Q4).
    pub pass_counters: [u32; PASS_COUNTERS],

    // ---- trip phase context (§4.3, §4.4) ----
    /// The traffic-source iterator of the running trip phase (§4.1, TS20).
    pub iter: SourceIter,
    /// The held current source of ReRouteTrips / FullTrips, whose trips are
    /// being examined; `None`: the next unit takes a new source.
    pub current: Option<Source>,
    /// FinishTripMap after a load in state 15 (§3.5 step 11): the lot whose
    /// successor the lot walk starts at; consumed by the phase.
    pub finish_resume: Option<u64>,
    /// The sources the running phase has taken (ConfirmTrips) or finished
    /// (ReRouteTrips, FullTrips).
    pub source_pos: usize,
    /// The next trip of the current source to examine.
    pub trip_pos: usize,
    /// FullTrips: the next search continues the previous one (§4.4).
    pub continue_search: bool,

    // ---- travel strategies (§4.1.3, §3.3) ----
    pub strategy_bucket_counter: u32,
    pub strategy_bucket: u32,
    /// The bucket multimap: bucket → lot cells in insertion order.
    pub buckets: BTreeMap<u32, Vec<(i32, i32)>>,

    // ---- strike (§4.12) ----
    /// 2 normal, 0 strike started, 1 strike.
    pub strike_state: u8,
    pub strike_months: i32,
    pub strike_flag: bool,
    /// The last strike probability `p` (kept for the stats).
    pub strike_probability: f32,

    // ---- departments (§1 Init step 5) ----
    pub roads: Option<u64>,
    pub mass_transit: Option<u64>,

    // ---- generators (§4.15) ----
    pub rng: RzRandomSuper,
    /// The C `rand()` of the accident iterator.
    pub crt: CrtRand,

    // ---- pathfinders (§1 Init step 6) ----
    pub shared: SharedWorkCheck,
    pub pf: Option<Pathfinder>,
    pub pf2: Option<Pathfinder>,

    /// The unnamed record fields.
    pub extras: SaveExtras,

    /// A `Read` committed a record since construction or the last
    /// `Shutdown` (not saved). `Init` keeps what such a record brought: the
    /// grids, the departments, the destination map and the scanning
    /// iterator *[choice]*.
    pub loaded: bool,
    /// The traffic shadow's stepping (tooling, not saved): a ReRouteTrips or
    /// FullTrips slice ends right after its last unit instead of first
    /// running on to the next trip to examine (see [`super::shadow`]).
    pub shadow_lazy: bool,
    /// The traffic shadow's scan stop (tooling, not saved): in the cycle
    /// state given, a cell-scanning phase (2, 4, 5, 6, 10, 13) stops, not
    /// finished, once its scanning iterator rests running at the cell given
    /// (where the original's deadline stopped it).
    pub shadow_scan_stop: Option<(u8, i32, i32)>,
    /// The traffic shadow's FinishTripMap stop (tooling, not saved): the
    /// lot walk of state 15 stops, not finished, after the lot at this cell
    /// (the original's saved cell when its deadline stopped the walk).
    pub shadow_finish_stop: Option<[i16; 2]>,
}

impl State {
    /// The constructor values (§1): cycle state 0, last day −1, strike
    /// state 2, the tuning code defaults; the generator seeded with `seed`.
    pub fn new(seed: u32) -> State {
        let seeds = FixedSeeds {
            wall_clock: seed,
            process_clock: seed,
        };
        State {
            ref_count: 0,
            initialised: false,
            tuning: SimTuning::default(),
            air: None,
            peak_volume: None,
            commercial: None,
            commercial_work: None,
            unused: None,
            trip_length: None,
            volume: vec![None; 6],
            float_grid: None,
            congestion: None,
            peak_ratio: None,
            dest_rows: 0,
            dest_cols: 0,
            dest: Vec::new(),
            free_dest: Default::default(),
            job_capacity: [0; 3],
            attempted: [0; 3],
            routed: [0; 3],
            accidents: Vec::new(),
            accident_check: Vec::new(),
            next_accident_check: 0,
            stats_last: StatsSet::default(),
            stats_work: StatsSet::default(),
            stat_counts: [0; 14],
            stat_max_cmp: [-1.0; 14],
            cells_per_type: [0; 13],
            budget_total: 0,
            transit_income: 0.0,
            toll_income: 0.0,
            last_monthly_income: 0.0,
            income_acc: Cell::new(0.0),
            fee_acc: 0.0,
            avg_car_commute: 0.0,
            car_len_sum: 0.0,
            car_len_count: 0,
            freight_len_sum: 0.0,
            freight_count: 0,
            type_count_building: [0; 9],
            type_count_published: [0; 9],
            ferry_total: 0,
            cycle_state: 0,
            scan: ScanIterator::new(0, 0, -1, -1),
            cycle_start_date: 0,
            last_day: -1,
            day_index: 0,
            days: 0,
            total: 0,
            processed: 0,
            pass_total: 0,
            to_reroute: 0,
            to_search: 0,
            pass_counters: [0; PASS_COUNTERS],
            iter: SourceIter::default(),
            current: None,
            finish_resume: None,
            source_pos: 0,
            trip_pos: 0,
            continue_search: false,
            strategy_bucket_counter: 0,
            strategy_bucket: 0,
            buckets: BTreeMap::new(),
            strike_state: 2,
            strike_months: 0,
            strike_flag: false,
            strike_probability: 0.0,
            roads: None,
            mass_transit: None,
            rng: RzRandomSuper::new(seed, &seeds),
            crt: CrtRand::new(1),
            shared: crate::pathfinder::shared_work_check(),
            pf: None,
            pf2: None,
            extras: SaveExtras::default(),
            loaded: false,
            shadow_lazy: false,
            shadow_scan_stop: None,
            shadow_finish_stop: None,
        }
    }

    /// `rerouteBucket` (pass counter 2).
    pub fn reroute_bucket(&self) -> u32 {
        self.pass_counters[PC_REROUTE_BUCKET]
    }

    /// The index of (x, z) in the destination map, x in −1…W, z in −1…H.
    pub fn dest_index(&self, x: i32, z: i32) -> Option<usize> {
        let (i, j) = (i64::from(x) + 1, i64::from(z) + 1);
        if i < 0 || j < 0 || i >= i64::from(self.dest_rows) || j >= i64::from(self.dest_cols) {
            return None;
        }
        Some((i * i64::from(self.dest_cols) + j) as usize)
    }

    /// The destination byte at (x, z) (0 outside the map).
    pub fn dest_byte(&self, x: i32, z: i32) -> u8 {
        self.dest_index(x, z)
            .and_then(|i| self.dest.get(i))
            .map_or(0, Cell::get)
    }

    /// Writes the destination byte at (x, z) (ignored outside the map).
    pub fn set_dest_byte(&self, x: i32, z: i32, v: u8) {
        if let Some(c) = self.dest_index(x, z).and_then(|i| self.dest.get(i)) {
            c.set(v);
        }
    }

    /// Sizes the destination map for a `w × h` city, zero-filled.
    pub fn size_dest(&mut self, w: i32, h: i32) {
        self.dest_rows = w.max(0) + 2;
        self.dest_cols = h.max(0) + 2;
        self.dest = (0..(self.dest_rows * self.dest_cols) as usize)
            .map(|_| Cell::new(0))
            .collect();
    }

    /// `freeDest[k]`.
    pub fn free(&self, k: usize) -> u32 {
        self.free_dest.get(k).map_or(0, Cell::get)
    }

    /// Sets `freeDest[k]`.
    pub fn set_free(&self, k: usize, v: u32) {
        if let Some(c) = self.free_dest.get(k) {
            c.set(v);
        }
    }

    /// The volume grid of transit network `n`.
    pub fn volume_grid(&self, n: u32) -> Option<&GridF32> {
        self.volume.get(n as usize).and_then(Option::as_ref)
    }
}
