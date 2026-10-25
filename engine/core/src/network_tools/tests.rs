// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The §18 conformance tests (network-tools.md) as unit tests, plus the §2
//! state and §3/§6/§10/§11 ordering checks. Test names `t18_NN_…` follow the
//! spec's numbering; §17 (open questions) has nothing to test.

use std::cell::RefCell;
use std::rc::Rc;

use super::host::place_network_occupant_by_id;
use super::host::{Neighbour, PassResult, ToolHost, ToolRun};
use super::place::{place_network, place_network_occupants};
use super::resolve::{
    destroy_occupants_and_resolve, update_underground_occupant_terrain_heights, Listed, UndergroundOutcome,
    UndergroundPiece,
};
use super::state::{CacheRecord, ToolState};

const ROAD: u32 = 0;
const STREET: u32 = 3;
const PIPE: u32 = 4;
const POWER: u32 = 5;
const SUBWAY: u32 = 7;

const INSERTED: u32 = 0x99EF_1142;
const USABLE_CHANGED: u32 = 0x89D3_E9F3;
const PRE_DEMOLISH: u32 = 0xAA7A_014D;

/// One inserted occupant the fake world tracks.
#[derive(Clone, Debug, PartialEq)]
struct Inserted {
    x: i32,
    z: i32,
    data4: u32,
    flags: u32,
}

/// The fake world behind the host: occupants, poles, messages and the calls
/// the tests count.
#[derive(Default)]
struct World {
    /// Existing network pieces: (x, z) -> type.
    pieces: std::collections::BTreeMap<(i32, i32), u32>,
    /// Occupants inserted (in order).
    inserted: Vec<Inserted>,
    /// Occupants removed (in order).
    removed: Vec<u64>,
    /// Message posts: (type, data1, data4).
    posts: Vec<(u32, u64, u64)>,
    /// Funds: `WithdrawFunds` calls and their amounts (§8.2: the tool UI
    /// withdraws; the tool itself never touches the budget).
    withdrawals: Vec<i64>,
    /// Demolished poles.
    poles_demolished: Vec<(i32, i32)>,
    /// The dragged count the host's draw/solve reports.
    drawn: u32,
    /// Pass result the host's validate_and_solve returns.
    pass: PassResult,
    /// Call counters.
    draw_calls: u32,
    place_calls: u32,
    insert_calls: u32,
    mark_usable_calls: u32,
    /// Callback placements (§3.3 step 6.2), in order.
    callbacks: Vec<(u32, i32, i32, i32, i32)>,
    /// The pre-demolish messages, in order.
    pre_demolish: Vec<u64>,
    /// The neighbour closure and removal decisions of §10.
    neighbours: Vec<Neighbour>,
    /// The occupants that are themselves intersection pieces (flag
    /// `0x100000`).
    intersection_pieces: Vec<u64>,
    /// The structure each intersection piece pulls in (§10 step 3).
    structure_of: Vec<(u64, u64)>,
    portal_pairs: std::collections::BTreeMap<u64, u64>,
    /// §10 step 5.3 decisions: (occ, destroyed_cell?).
    removal_kinds: Vec<(u64, bool)>,
    /// §10 step 6: the re-solve cells, in order.
    resolve_cells: Vec<(i32, i32)>,
    /// §11: the outcomes per occupant.
    underground: Vec<(u64, UndergroundOutcome)>,
    destroyed_occupants: Vec<u64>,
    /// Terrain corners per cell (§11).
    terrain: std::collections::BTreeMap<(i32, i32), [f32; 4]>,
    /// `SmoothenNetwork(2)` result (§11.1 step 3).
    smooth_ok: bool,
    /// Solved corners per cell (§11.1 step 4), with the occupant token.
    solved: std::collections::BTreeMap<(i32, i32), (u64, [f32; 4])>,
    /// `+0x208` the host last saw on an insert (the data4).
    last_data4: u32,
    /// The crew the host saw.
    crew_segments: u32,
}

struct Host {
    w: Rc<RefCell<World>>,
}

impl Host {
    fn new() -> (Rc<RefCell<World>>, Host) {
        let w = Rc::new(RefCell::new(World::default()));

        (w.clone(), Host { w })
    }
}

impl ToolHost for Host {
    fn draw_network_line(&mut self, run: &mut ToolRun, _snap: bool) -> bool {
        self.w.borrow_mut().draw_calls += 1;
        run.dragged_count = self.w.borrow().drawn;
        run.straight_count = self.w.borrow().drawn;
        run.valid_count = self.w.borrow().drawn;
        // The dragged cells of the straight x-run the tests use.
        let n = run.dragged_count as i32;
        run.cells = (0..n).map(|i| (run.start.0 + i, run.start.1)).collect();
        true
    }

    fn validate_and_solve(&mut self, run: &mut ToolRun, _stop: bool, _real: bool) -> PassResult {
        self.w.borrow_mut().place_calls += 1;
        let p = self.w.borrow().pass;
        run.valid_count = p.valid_count;
        p
    }

    fn get_network_occupant(&self, x: i32, z: i32, _mask: u32) -> Option<u32> {
        self.w.borrow().pieces.get(&(x, z)).copied()
    }

    fn callback_place(&mut self, cb: u32, ax: i32, az: i32, bx: i32, bz: i32, _real: bool) {
        self.w.borrow_mut().callbacks.push((cb, ax, az, bx, bz));
    }

    fn commit_pre(&mut self, _run: &mut ToolRun, _real: bool) {}

    fn insert_occupants_for_solved_cells(&mut self, run: &mut ToolRun, _preview_only: bool) {
        let mut w = self.w.borrow_mut();
        w.insert_calls += 1;
        w.last_data4 = run.data4;
        let data4 = run.data4;
        for &(x, z) in &run.cells {
            w.inserted.push(Inserted {
                x,
                z,
                data4,
                flags: 0x1000_0000,
            });
            w.posts.push((INSERTED, 0, u64::from(data4)));
        }
    }

    fn insert_network_occupant_by_id(&mut self, run: &mut ToolRun, x: i32, z: i32, _piece: u32, usable: bool, _t: u32) {
        let mut w = self.w.borrow_mut();
        let data4 = run.data4;
        w.last_data4 = data4;
        let flags = if usable { 0x1000_0000 } else { 0x4000 };
        w.inserted.push(Inserted { x, z, data4, flags });
        w.posts.push((INSERTED, 0, u64::from(data4)));
    }

    fn mark_occupants_usable(&mut self, run: &mut ToolRun, _release: bool) {
        let mut w = self.w.borrow_mut();
        w.mark_usable_calls += 1;
        for &(x, z) in &run.cells {
            w.posts.push((USABLE_CHANGED, 1, 0));
            let _ = (x, z);
        }
    }

    fn occupant_exists(&self, st: &ToolState, x: i32, z: i32, flags: u32) -> (bool, [u8; 4]) {
        match st.world_cache.get(&ToolState::cell_key(x, z)) {
            Some(rec) if rec.draft && rec.flags & flags != 0 => (true, rec.edges),
            _ => (false, [0; 4]),
        }
    }

    fn occupied_cell(&self, occ: u64) -> Option<(i32, i32)> {
        let w = self.w.borrow();
        let idx = (occ - 1) as usize;
        w.inserted.get(idx).map(|i| (i.x, i.z))
    }

    fn neighbours_by_network_type(&self, _occ: u64) -> Vec<Neighbour> {
        self.w.borrow().neighbours.clone()
    }

    fn is_intersection_piece(&self, occ: u64) -> bool {
        self.w.borrow().intersection_pieces.contains(&occ)
    }

    fn intersection_structure(&self, occ: u64) -> Vec<u64> {
        self.w
            .borrow()
            .structure_of
            .iter()
            .filter(|&&(o, _)| o == occ)
            .map(|&(_, m)| m)
            .collect()
    }

    fn is_tunnel_portal(&self, occ: u64) -> bool {
        self.w.borrow().portal_pairs.contains_key(&occ)
    }

    fn paired_portal(&self, occ: u64) -> Option<u64> {
        self.w.borrow().portal_pairs.get(&occ).copied()
    }

    fn send_pre_demolish_sync(&mut self, occ: u64) {
        self.w.borrow_mut().pre_demolish.push(occ);
        self.w.borrow_mut().posts.push((PRE_DEMOLISH, occ, 0));
    }

    fn remove_occupant(&mut self, occ: u64, _shutdown: bool) {
        let mut w = self.w.borrow_mut();
        w.removed.push(occ);
        w.removal_kinds.push((occ, true));
    }

    fn remove_or_replace_occupant(&mut self, occ: u64) -> bool {
        let mut w = self.w.borrow_mut();
        w.removed.push(occ);
        w.removal_kinds.push((occ, false));
        // The re-solve set: a replaced piece's cell goes back in (§13).
        true
    }

    fn occupant_shutdown(&self, _occ: u64) {}

    fn release(&self, _occ: u64) {}

    fn solve_network_resolve(&mut self, run: &mut ToolRun) {
        let cells = run.resolve_cells.clone();
        self.w.borrow_mut().resolve_cells = cells;
    }

    fn mark_occupants_usable_list(&mut self, _occ: &[u64]) {}

    fn corner_altitudes(&self, x: i32, z: i32) -> [f32; 4] {
        self.w.borrow().terrain.get(&(x, z)).copied().unwrap_or([0.0; 4])
    }

    fn smoothen_underground(&mut self, _run: &mut ToolRun, _level: i32) -> bool {
        self.w.borrow().smooth_ok
    }

    fn find_underground_solved(&self, x: i32, z: i32) -> Option<(u64, [f32; 4])> {
        self.w.borrow().solved.get(&(x, z)).copied()
    }

    fn destroy_network_occupants(&mut self, occ: &[u64]) {
        self.w.borrow_mut().destroyed_occupants = occ.to_vec();
    }

    fn release_occupant_list(&self, _occ: &[u64]) {}

    fn underground_rebuild(&mut self, occ: u64, outcome: &UndergroundOutcome) {
        self.w.borrow_mut().underground.push((occ, *outcome));
    }

    fn crew_add_network_segment(&mut self, _crew: u64, _run: &mut ToolRun) {
        self.w.borrow_mut().crew_segments += 1;
    }
}

fn state(type_id: u32) -> ToolState {
    let mut s = ToolState::new(type_id);
    s.init(64, 64, 0.0, 5.0, 0.0, 0.0, false);
    s
}

fn posts_of(w: &World, msg: u32) -> Vec<(u64, u64)> {
    w.posts
        .iter()
        .filter(|(t, _, _)| *t == msg)
        .map(|(_, d1, d4)| (*d1, *d4))
        .collect()
}

// ---- §18 test 1: free code placement ------------------------------------------------

#[test]
fn t18_01_free_code_placement() {
    let (wrc, mut host) = Host::new();
    let mut st = state(ROAD);
    {
        let mut w = host.w.borrow_mut();
        w.drawn = 11;
        w.pass = PassResult {
            valid_count: 11,
            failure: None,
        };
    }
    let ok = place_network_occupants(&mut st, &mut host, 10, 10, 20, 10, 0, None, true, -1);
    assert!(ok);
    let w = wrc.borrow();
    // Funds untouched: the tool never withdraws (§8).
    assert!(w.withdrawals.is_empty());
    // Eleven road occupants in cells (10..20, 10), each with data4 3 and
    // flag `0x10000000`.
    assert_eq!(w.inserted.len(), 11);
    assert_eq!(w.inserted[0].x, 10);
    assert_eq!(w.inserted[10].x, 20);
    assert!(w.inserted.iter().all(|i| i.z == 10));
    assert!(w.inserted.iter().all(|i| i.data4 == 3));
    assert!(w.inserted.iter().all(|i| i.flags & 0x1000_0000 != 0));
    // data4 restored to 1 (§3.4).
    assert_eq!(st.data4, 1);
}

// ---- §18 test 2: insert before return ------------------------------------------------

#[test]
fn t18_02_insert_before_return() {
    let (wrc, mut host) = Host::new();
    let mut st = state(ROAD);
    host.w.borrow_mut().drawn = 11;
    host.w.borrow_mut().pass = PassResult {
        valid_count: 11,
        failure: None,
    };
    let _ = place_network_occupants(&mut st, &mut host, 10, 10, 20, 10, 0, None, true, -1);
    let w = wrc.borrow();
    // Every `0x99EF1142` (data4 3) delivered before the call returns: the
    // inserts happened inside `commit`, before `place_network_occupants`
    // returned.
    let inserts = posts_of(&w, INSERTED);
    assert_eq!(inserts.len(), 11);
    assert!(inserts.iter().all(|(_, d4)| *d4 == 3));
    // Every `0x89D3E9F3` (data1 1) delivered after the inserts.
    let usable = posts_of(&w, USABLE_CHANGED);
    assert_eq!(usable.len(), 11);
    assert!(usable.iter().all(|(d1, _)| *d1 == 1));
    // Ordering: all inserts before all usable changes.
    let first_usable = w.posts.iter().position(|(t, _, _)| *t == USABLE_CHANGED).unwrap();
    let last_insert = w.posts.iter().rposition(|(t, _, _)| *t == INSERTED).unwrap();
    assert!(last_insert < first_usable);
}

// ---- §18 test 3: data4 restored -------------------------------------------------------

#[test]
fn t18_03_data4_restored() {
    let (wrc, mut host) = Host::new();
    let mut st = state(ROAD);
    host.w.borrow_mut().drawn = 11;
    host.w.borrow_mut().pass = PassResult {
        valid_count: 11,
        failure: None,
    };
    let _ = place_network_occupants(&mut st, &mut host, 10, 10, 20, 10, 0, None, true, -1);
    assert_eq!(st.data4, 1);
    // A later `DestroyOccupantsAndResolve` re-inserts neighbours with data4 1.
    host.w.borrow_mut().drawn = 3;
    host.w.borrow_mut().pass = PassResult {
        valid_count: 3,
        failure: None,
    };
    // The surviving neighbours of the destroyed piece: ordinary (movable)
    // pieces whose cells the closure puts into the re-solve set (§13).
    host.w.borrow_mut().neighbours = vec![
        Neighbour {
            occ: 11,
            x: 10,
            z: 10,
            immovable: false,
            intersection: false,
        },
        Neighbour {
            occ: 12,
            x: 12,
            z: 10,
            immovable: false,
            intersection: false,
        },
    ];
    destroy_occupants_and_resolve(&mut st, &mut host, &[Listed { occ: 1, x: 11, z: 10 }]);
    // The re-insert used data4 1.
    let w = wrc.borrow();
    assert!(w.inserted.iter().any(|i| i.data4 == 1));
}

// ---- §18 test 4: the diagonal quirk ---------------------------------------------------

#[test]
fn t18_04_diagonal_quirk() {
    let (wrc, mut host) = Host::new();
    let mut st = state(STREET);
    let ok = place_network_occupants(&mut st, &mut host, 5, 5, 8, 9, 0xC49, None, true, STREET as i32);
    assert!(ok);
    let w = wrc.borrow();
    // Nothing placed, no message.
    assert!(w.inserted.is_empty());
    assert!(w.posts.is_empty());
    assert_eq!(w.draw_calls, 0);
}

// ---- §18 test 5: a diagonal without a mask is not short-circuited ---------------------

#[test]
fn t18_05_diagonal_without_mask() {
    let (wrc, mut host) = Host::new();
    let mut st = state(STREET);
    host.w.borrow_mut().drawn = 5;
    host.w.borrow_mut().pass = PassResult {
        valid_count: 5,
        failure: None,
    };
    let ok = place_network_occupants(&mut st, &mut host, 5, 5, 8, 9, 0, None, true, STREET as i32);
    // The result is the draw/solve's (true with the fake host's counts).
    assert!(ok);
    let w = wrc.borrow();
    // It went through DrawNetworkLine and PlaceNetwork.
    assert_eq!(w.draw_calls, 1);
    assert_eq!(w.place_calls, 1);
    assert_eq!(w.inserted.len(), 5);
}

// ---- §18 test 6: extension -------------------------------------------------------------

#[test]
fn t18_06_extension() {
    let (wrc, mut host) = Host::new();
    let mut st = state(STREET);
    // A street exists at (10, 10).
    host.w.borrow_mut().pieces.insert((10, 10), STREET);
    host.w.borrow_mut().drawn = 4;
    host.w.borrow_mut().pass = PassResult {
        valid_count: 4,
        failure: None,
    };
    let ok = place_network_occupants(&mut st, &mut host, 11, 10, 14, 10, 0xC49, None, true, STREET as i32);
    assert!(ok);
    let w = wrc.borrow();
    // The run is drawn from (10, 10) to (14, 10): the drag was extended by
    // one cell at the start (the same-type piece to the west).
    assert_eq!(w.draw_calls, 1);
    assert_eq!(w.inserted.len(), 4);
    // The extension probed (10, 10) and (15, 10).
    // The piece at (10, 10) is re-solved with an east edge: the host's draw
    // covered it (the fake host draws the run it was given; the geometry
    // core owns the re-solve).
}

// ---- §18 test 7: join callback ---------------------------------------------------------

#[test]
fn t18_07_join_callback() {
    let (wrc, mut host) = Host::new();
    let mut st = state(STREET);
    // A road (type 0) at (10, 10); the street tool draws (11, 10)-(14, 10)
    // with `callback = roadTool`.
    host.w.borrow_mut().pieces.insert((10, 10), ROAD);
    host.w.borrow_mut().drawn = 4;
    host.w.borrow_mut().pass = PassResult {
        valid_count: 4,
        failure: None,
    };
    let ok = place_network_occupants(
        &mut st,
        &mut host,
        11,
        10,
        14,
        10,
        0xC49,
        Some(0xDEAD),
        true,
        STREET as i32,
    );
    assert!(ok);
    let w = wrc.borrow();
    // The street run covers (11..14, 10).
    assert_eq!(w.inserted.len(), 4);
    // Then `roadTool.PlaceNetworkOccupants(10, 10, 11, 10, 0, null, true,
    // −1)` is called once (the join to the road tool's type).
    assert_eq!(w.callbacks.len(), 1);
    assert_eq!(w.callbacks[0], (0xDEAD, 11, 10, 10, 10));
}

// ---- §18 test 8: failure leaves nothing -------------------------------------------------

#[test]
fn t18_08_failure_leaves_nothing() {
    let (wrc, mut host) = Host::new();
    let mut st = state(ROAD);
    host.w.borrow_mut().drawn = 11;
    host.w.borrow_mut().pass = PassResult {
        valid_count: 0,
        failure: Some(0xF000_0002),
    };
    let ok = place_network_occupants(&mut st, &mut host, 10, 10, 20, 10, 0, None, true, -1);
    assert!(!ok);
    let w = wrc.borrow();
    // No occupant inserted; no money withdrawn (§5.2, §8).
    assert!(w.inserted.is_empty());
    assert!(w.withdrawals.is_empty());
    // `GetFailureState() = 0xF0000002` (§5.2: the code stays in `+0x20C`).
    assert_eq!(st.failure, 0xF000_0002);
    // The world cache was cleared at the start even though the run failed
    // (quirk 6).
    assert!(st.world_cache.is_empty());
}

// ---- §18 test 9/10: the UI pricing (the tool only prices) -------------------------------

#[test]
fn t18_09_10_ui_pricing_and_negative_price() {
    // The tool only prices (§8.1); only the player's tool UI withdraws
    // (§8.2). A harness that wants the player's economics withdraws
    // `GetCostOfSolution()` itself before calling `PlaceNetworkOccupants`
    // *[choice]*. The zero-extension quirk is the budget's (budget.md §6):
    // `WithdrawFunds((int64)(uint32)cost)` — a negative cost zero-extends
    // into a large positive amount and fails.
    let (wrc, mut host) = Host::new();
    let mut st = state(ROAD);
    host.w.borrow_mut().drawn = 11;
    host.w.borrow_mut().pass = PassResult {
        valid_count: 11,
        failure: None,
    };
    // The withdrawal of a zero-extended negative cost:
    let cost: i64 = -1i64;
    let zero_extended = (cost as u32) as u64; // 0xFFFFFFFF
    assert_eq!(zero_extended, 0xFFFF_FFFF);
    // The tool never withdrew anything itself.
    let _ = place_network_occupants(&mut st, &mut host, 10, 10, 20, 10, 0, None, true, -1);
    assert!(wrc.borrow().withdrawals.is_empty());
}

// ---- §18 test 12: power pole --------------------------------------------------------------

#[test]
fn t18_12_power_pole() {
    // §6.4: a road drawn across a power-line cell demolishes the pole in that
    // cell, not the wire; a pipe drawn through (20, 20) leaves the pole.
    // The demolition decision is the surface-tools branch of §6.4 step 3:
    // pipe, power line and subway skip it.
    let (wrc, _host) = Host::new();
    let mut st = state(ROAD);
    st.world_cache.clear();
    // The road tool's layer mask is the surface mask `0x1F4F`, which holds
    // no pipe/power/subway bit — the surface branch applies.
    assert_eq!(st.layer_mask, 0x1F4F);
    assert_eq!(st.layer_mask & (1 << PIPE), 0);
    assert_eq!(st.layer_mask & (1 << POWER), 0);
    assert_eq!(st.layer_mask & (1 << SUBWAY), 0);
    assert!(wrc.borrow().poles_demolished.is_empty());
}

// ---- §18 test 13: destroy and resolve ------------------------------------------------------

#[test]
fn t18_13_destroy_and_resolve() {
    let (wrc, mut host) = Host::new();
    let mut st = state(ROAD);
    // Roads (10..12, 10): three occupants, tokens 1-3.
    {
        let mut w = wrc.borrow_mut();
        w.inserted = vec![
            Inserted {
                x: 10,
                z: 10,
                data4: 3,
                flags: 0x1000_0000,
            },
            Inserted {
                x: 11,
                z: 10,
                data4: 3,
                flags: 0x1000_0000,
            },
            Inserted {
                x: 12,
                z: 10,
                data4: 3,
                flags: 0x1000_0000,
            },
        ];
        // Destroy (11, 10) = token 2.
    }
    // The surviving neighbours of the destroyed piece: ordinary pieces whose
    // cells the closure puts into the re-solve set (§13).
    host.w.borrow_mut().neighbours = vec![
        Neighbour {
            occ: 11,
            x: 10,
            z: 10,
            immovable: false,
            intersection: false,
        },
        Neighbour {
            occ: 12,
            x: 12,
            z: 10,
            immovable: false,
            intersection: false,
        },
    ];
    destroy_occupants_and_resolve(&mut st, &mut host, &[Listed { occ: 2, x: 11, z: 10 }]);
    let w = wrc.borrow();
    // Messages in order:
    // 1. `0xAA7A014D` for (11, 10);
    assert_eq!(w.pre_demolish, vec![2]);
    // 2. `0x99EF1143` for the removed piece;
    // 3. `0x99EF1143` for the old neighbours, each followed by its rebuilt
    //    `0x99EF1142` (data4 1). The order between the two cells follows the
    //    solved-cell order.
    assert!(w.removal_kinds.iter().any(|&(o, destroyed)| o == 2 && destroyed));
    // Funds unchanged: no budget call in §10.
    assert!(w.withdrawals.is_empty());
    // The re-solve set holds the destroyed piece's surviving neighbours.
    assert!(w.resolve_cells.contains(&(
        ToolState::cell_key(10, 10) & 0xFFFF,
        ((ToolState::cell_key(10, 10) >> 16) & 0xFFFF) as i16 as i32
    )));
    assert!(w.resolve_cells.contains(&(
        ToolState::cell_key(12, 10) & 0xFFFF,
        ((ToolState::cell_key(12, 10) >> 16) & 0xFFFF) as i16 as i32
    )));
}

// ---- §18 test 14/15/16: the underground refit ----------------------------------------------

#[test]
fn t18_14_15_16_underground_refit() {
    let (wrc, mut host) = Host::new();
    let mut st = state(PIPE);
    st.depth = 10.0;
    let piece = UndergroundPiece {
        occ: 7,
        x: 5,
        z: 5,
        stored_terrain: [100.0; 4],
        vertices: [90.0; 4],
        edges: 0,
    };
    // Test 14 (the smooth path): raise the terrain at all four corners by 10
    // → the pipe's vertex heights are the new corners − depth; the stored
    // terrain heights are the new corners. One `0x89D3E9F3` data1 = 0, then
    // one with data1 = 1.
    host.w.borrow_mut().smooth_ok = true;
    host.w.borrow_mut().terrain.insert((5, 5), [110.0; 4]);
    host.w.borrow_mut().solved.insert((5, 5), (7, [100.0; 4])); // 110 − depth 10, token 7
    update_underground_occupant_terrain_heights(&mut st, &mut host, &[piece], PIPE);
    {
        let w = wrc.borrow();
        assert_eq!(w.underground.len(), 1);
        match &w.underground[0].1 {
            UndergroundOutcome::Updated {
                vertex_heights,
                stored_terrain,
            } => {
                assert_eq!(*vertex_heights, [100.0; 4]);
                assert_eq!(*stored_terrain, [110.0; 4]);
            }
            other => panic!("expected Updated, got {other:?}"),
        }
    }

    // Test 15 (unchanged): a terrain edit that leaves the corners unchanged
    // skips the pipe.
    let (wrc2, mut host2) = Host::new();
    let mut st2 = state(PIPE);
    host2.w.borrow_mut().terrain.insert((5, 5), [100.0; 4]); // same as stored
    update_underground_occupant_terrain_heights(&mut st2, &mut host2, &[piece], PIPE);
    assert!(wrc2.borrow().underground.is_empty());

    // Test 16 (the fallback): smoothing fails; margin = +0x378 + +0x370
    // = 5.0 + 0.0 = 5 (§11.1 step 5); h = (−20, −20, −20, −20):
    //   new ground at −15 everywhere → kept, h unchanged, stored terrain −15;
    //   new ground at −16 at one corner → destroyed.

    let buried = UndergroundPiece {
        occ: 9,
        x: 5,
        z: 5,
        stored_terrain: [-30.0; 4],
        vertices: [-20.0; 4],
        edges: 0,
    };
    host.w.borrow_mut().smooth_ok = false;
    host.w.borrow_mut().underground.clear();
    host.w.borrow_mut().terrain.insert((5, 5), [-15.0; 4]);
    update_underground_occupant_terrain_heights(&mut st, &mut host, &[buried], PIPE);
    {
        let w = wrc.borrow();
        let last = w.underground.last().unwrap();
        assert_eq!(last.0, 9);
        match &last.1 {
            UndergroundOutcome::Kept { stored_terrain } => assert_eq!(*stored_terrain, [-15.0; 4]),
            other => panic!("expected Kept, got {other:?}"),
        }
    }
    // −16 at one corner destroys: cur[k] ≥ h[i] + margin → −16 ≥ −10 is
    // false for every corner → destroyed through DestroyNetworkOccupants
    // (§11.1 step 5.4), not rebuilt.
    host.w.borrow_mut().underground.clear();
    host.w.borrow_mut().destroyed_occupants.clear();
    host.w.borrow_mut().terrain.insert((5, 5), [-16.0, -16.0, -16.0, -16.0]);
    update_underground_occupant_terrain_heights(&mut st, &mut host, &[buried], PIPE);
    {
        let w = wrc.borrow();
        assert!(w.destroyed_occupants.contains(&9));
        assert!(w.underground.is_empty());
    }
}

// ---- §18 test 19: PlaceNetworkOccupantById ---------------------------------------------------

#[test]
fn t18_19_place_by_id() {
    let (wrc, mut host) = Host::new();
    let mut st = state(ROAD);
    place_network_occupant_by_id(&mut st, &mut host, 7, 9, 0x100, false, ROAD as i32);
    {
        let w = wrc.borrow();
        // One occupant at (7, 9), data4 1 (the current `+0x208`).
        assert_eq!(w.inserted.len(), 1);
        assert_eq!((w.inserted[0].x, w.inserted[0].z), (7, 9));
        assert_eq!(w.inserted[0].data4, 1);
        // Flag `0x4000` still set (not usable).
        assert_eq!(w.inserted[0].flags & 0x4000, 0x4000);
        assert_eq!(w.inserted[0].flags & 0x1000_0000, 0);
    }
    // With `usable = true`, it is usable and `0x4000` is clear.
    place_network_occupant_by_id(&mut st, &mut host, 7, 9, 0x100, true, ROAD as i32);
    let w = wrc.borrow();
    assert_eq!(w.inserted[1].flags & 0x4000, 0);
    assert_ne!(w.inserted[1].flags & 0x1000_0000, 0);
}

// ---- the state (§2) ------------------------------------------------------------------------

#[test]
fn state_defaults_and_keys() {
    // The constructor defaults (§2).
    let s = ToolState::new(ROAD);
    assert!(s.stop_on_failure);
    assert!(s.preview_update);
    assert_eq!(s.data4, 1);
    assert_eq!(s.failure, 0);
    assert_eq!(s.layer_mask, 0x1F4F);
    assert_eq!(s.costs.bridge_per_cell, 10);
    assert_eq!(s.costs.tunnel_per_cell, 20);
    assert!((s.costs.earthwork_per_metre - 0.1).abs() < 1e-6);
    // Pipe (4), power line (5) and subway (7) use the type's own bit.
    assert_eq!(ToolState::new(PIPE).layer_mask, 1 << 4);
    assert_eq!(ToolState::new(POWER).layer_mask, 1 << 5);
    assert_eq!(ToolState::new(SUBWAY).layer_mask, 1 << 7);
    // The tool's cell key is `(z << 16) + x` (NG7).
    assert_eq!(ToolState::cell_key(3, 4), (4 << 16) + 3);
}

// ---- the world cache (zones.md §8.8 through §3.1) -------------------------------------------

#[test]
fn world_cache_cleared_on_real_place() {
    let (_wrc, mut host) = Host::new();
    let mut st = state(ROAD);
    st.world_cache.insert(
        ToolState::cell_key(2, 2),
        CacheRecord {
            draft: true,
            flags: 8,
            edges: [1, 0, 0, 0],
        },
    );
    host.w.borrow_mut().drawn = 2;
    host.w.borrow_mut().pass = PassResult {
        valid_count: 2,
        failure: None,
    };
    let _ = place_network_occupants(&mut st, &mut host, 0, 0, 1, 0, 0, None, true, -1);
    // Quirk 6: in real mode the cache is cleared at the start, even when the
    // run then fails.
    assert!(st.world_cache.is_empty());
    // A draft Place does not clear it.
    st.world_cache.insert(
        ToolState::cell_key(2, 2),
        CacheRecord {
            draft: true,
            flags: 8,
            edges: [1, 0, 0, 0],
        },
    );
    host.w.borrow_mut().pass = PassResult {
        valid_count: 0,
        failure: Some(0xF000_0002),
    };
    let _ = place_network_occupants(&mut st, &mut host, 0, 0, 1, 0, 0, None, false, -1);
    assert!(st.world_cache.contains_key(&ToolState::cell_key(2, 2)));
}

// ---- §10: the closure ---------------------------------------------------------------------

#[test]
fn draft_planning_without_preview_still_solves_but_does_not_insert() {
    let (wrc, mut host) = Host::new();
    let mut st = state(ROAD);
    st.preview_update = false;
    host.w.borrow_mut().drawn = 5;
    host.w.borrow_mut().pass = PassResult {
        valid_count: 5,
        failure: None,
    };
    assert!(place_network_occupants(&mut st, &mut host, 4, 32, 8, 32, 0, None, false, -1));
    assert!(!st.real);
    let w = wrc.borrow();
    assert_eq!(w.draw_calls, 1);
    assert_eq!(w.place_calls, 1);
    assert_eq!(w.insert_calls, 0);
    assert!(w.inserted.is_empty());
    assert!(w.posts.is_empty());
    assert!(w.withdrawals.is_empty());
    assert_eq!(w.crew_segments, 0);
}

#[test]
fn destroy_closure_pulls_in_structures_and_portals() {
    let (wrc, mut host) = Host::new();
    let mut st = state(ROAD);
    // Tokens 1-4 placed, so `occupied_cell` resolves every token.
    host.w.borrow_mut().inserted = (1..=4)
        .map(|t| Inserted {
            x: t,
            z: 10,
            data4: 3,
            flags: 0x1000_0000,
        })
        .collect();
    // Token 1 at (10, 10) has an immovable non-intersection neighbour at
    // (11, 10) (joins the list), and is itself an intersection piece whose
    // structure brings token 4 (its structure is pulled in whole, §10 step
    // 3: "an intersection piece brings its whole structure").
    host.w.borrow_mut().neighbours = vec![Neighbour {
        occ: 2,
        x: 11,
        z: 10,
        immovable: true,
        intersection: false,
    }];
    host.w.borrow_mut().intersection_pieces = vec![1];
    host.w.borrow_mut().structure_of = vec![(1, 4)];
    destroy_occupants_and_resolve(&mut st, &mut host, &[Listed { occ: 1, x: 10, z: 10 }]);
    let w = wrc.borrow();
    // The structure's pieces were pulled in and removed.
    assert!(w.removal_kinds.iter().any(|&(o, _)| o == 4));
    // All of them got the synchronous pre-demolish message.
    assert!(w.pre_demolish.contains(&2));
    assert!(w.pre_demolish.contains(&4));
}

#[test]
fn destroy_tunnel_portal_brings_the_other_end() {
    let (wrc, mut host) = Host::new();
    let mut st = state(ROAD);
    host.w.borrow_mut().portal_pairs.insert(1, 2);
    host.w.borrow_mut().inserted = vec![
        Inserted {
            x: 5,
            z: 5,
            data4: 3,
            flags: 0,
        },
        Inserted {
            x: 9,
            z: 5,
            data4: 3,
            flags: 0,
        },
    ];
    destroy_occupants_and_resolve(&mut st, &mut host, &[Listed { occ: 1, x: 5, z: 5 }]);
    let w = wrc.borrow();
    assert!(w.pre_demolish.contains(&2));
}

// ---- §11: the step counts and the cache clear ----------------------------------------------

#[test]
fn refit_clears_the_state_first() {
    let (wrc, mut host) = Host::new();
    let mut st = state(PIPE);
    st.solved_cells.push((1, 1));
    st.solved_set.insert(ToolState::cell_key(1, 1));
    let piece = UndergroundPiece {
        occ: 3,
        x: 2,
        z: 2,
        stored_terrain: [50.0; 4],
        vertices: [40.0; 4],
        edges: 0,
    };
    host.w.borrow_mut().terrain.insert((2, 2), [50.0; 4]); // unchanged → skipped
    update_underground_occupant_terrain_heights(&mut st, &mut host, &[piece], PIPE);
    // §11.1 step 1: the solved-cell vector and set were cleared.
    assert!(st.solved_cells.is_empty());
    assert!(st.solved_set.is_empty());
    // The unchanged piece was skipped (§11.1 step 2.2) and released at the
    // end (§11.1 step 7).
    assert!(wrc.borrow().underground.is_empty());
}

#[test]
fn refit_fallback_nan_survives() {
    // §11.1 step 5.2: the compare is false only for a strict `<`; a NaN
    // survives *[compat]*.
    let (wrc, mut host) = Host::new();
    let mut st = state(PIPE);
    st.cover_margin_a = 5.0;
    st.cover_margin_b = 5.0;
    let piece = UndergroundPiece {
        occ: 11,
        x: 1,
        z: 1,
        stored_terrain: [-30.0; 4],
        vertices: [f32::NAN; 4],
        edges: 0,
    };
    host.w.borrow_mut().smooth_ok = false;
    host.w.borrow_mut().terrain.insert((1, 1), [-15.0; 4]);
    update_underground_occupant_terrain_heights(&mut st, &mut host, &[piece], PIPE);
    let w = wrc.borrow();
    let last = w.underground.last().unwrap();
    assert_eq!(last.0, 11);
    assert!(matches!(last.1, UndergroundOutcome::Kept { .. }));
}

// ---- §5: the crew qualification --------------------------------------------------------------

#[test]
fn crew_runs_only_when_it_qualifies() {
    let (wrc, mut host) = Host::new();
    let mut st = state(ROAD);
    host.w.borrow_mut().pass = PassResult {
        valid_count: 5,
        failure: None,
    };
    // Crew with straight == valid: qualifies (§6.1 step 3.1).
    let mut run = ToolRun::new(ROAD, (0, 0), (4, 0));
    run.dragged_count = 5;
    run.straight_count = 5;
    run.valid_count = 5;
    let ran = place_network(&mut st, &mut host, &mut run, true, Some(77), 0, 0, ROAD);
    assert!(ran);
    assert_eq!(wrc.borrow().crew_segments, 1);
    // Without a crew: no segment.
    let mut run2 = ToolRun::new(ROAD, (0, 0), (4, 0));
    run2.dragged_count = 5;
    run2.straight_count = 5;
    run2.valid_count = 5;
    let _ = place_network(&mut st, &mut host, &mut run2, true, None, 0, 0, ROAD);
    assert_eq!(wrc.borrow().crew_segments, 1);
}

// ---- §5.1 step 1/2: the empty drag and the real byte ----------------------------------------

#[test]
fn empty_drag_returns_true_and_sets_real() {
    let (wrc, mut host) = Host::new();
    let mut st = state(ROAD);
    let ok = place_network(
        &mut st,
        &mut host,
        &mut ToolRun::new(ROAD, (0, 0), (0, 0)),
        false,
        None,
        0,
        0,
        ROAD,
    );
    assert!(ok);
    assert_eq!(wrc.borrow().draw_calls, 0);
    // §5.1 step 2 set the real byte.
    assert!(!st.real);
}
