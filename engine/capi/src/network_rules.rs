// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The network rules over the C ABI (fc4.h "network rules",
//! docs/specs/network-rules.md): a rule database loaded from resource bytes,
//! tile rule evaluation, and the rule stages of `SolveNetwork` over an
//! explicit cell grid (`fc4_network_rules_grid`: the cell infos plus the
//! solve state). No callbacks: the caller fills the grid, solves, and reads
//! the solved records, the potential-intersection list and the cells left
//! unsolved.

use fc4_core::network_rules::{
    cell_key, key_cell, neighbourhood, solve_network, tile_instances, CellGrid, CellInfo, LoadReport, LoadStatus,
    NetworkRules, OccupantView, ResourceId, SolveState, NETWORK_INI_INSTANCE, TYPE_COUNT,
};

use crate::guard;

/// Detached NR16 input: one value per metaposition byte. `edge_known` is
/// 0/1; validity is 0 (uncaptured), 1 (false), or 2 (true).
#[repr(C)]
pub struct fc4_network_rules_capture {
    pub centre: u32,
    pub edges: [u32; 256],
    pub edge_known: [u8; 256],
    pub validity: [u8; 256],
}

/// Semantic in-order evaluator stream: key, eligibility, condition count,
/// output count, then condition triples (m, value, mask) and output triples
/// (id, m, orientation). Capacity/allocation/padding are excluded (NR16).
#[no_mangle]
pub unsafe extern "C" fn fc4_network_rules_snapshot(
    r: *const fc4_network_rules,
    t: u32,
    index: u32,
    out: *mut u32,
    cap: u32,
) -> u32 {
    guard(0, || {
        let Some(ev) = rules_ref(r)
            .and_then(|r| r.evaluators.get(t as usize))
            .and_then(|v| v.get(index as usize))
        else {
            return 0;
        };
        let mut words = Vec::new();
        for rule in ev.rules() {
            words.extend([
                rule.key,
                u32::from(rule.transform),
                rule.conditions.len() as u32,
                rule.outputs.len() as u32,
            ]);
            for c in &rule.conditions {
                words.extend([c.m as u8 as u32, c.value, c.mask]);
            }
            for o in &rule.outputs {
                words.extend([o.id, u32::from(o.m), u32::from(o.orientation)]);
            }
        }
        fill(out, cap, words.into_iter())
    })
}

/// NR16 detached replay. -3 = bad arguments, -2 = incomplete capture,
/// -1 = no match, >=0 = matched output count (including an empty match).
/// No native callbacks are issued; missing inputs can never earn exactness.
#[no_mangle]
pub unsafe extern "C" fn fc4_network_rules_replay(
    r: *const fc4_network_rules,
    t: u32,
    index: u32,
    capture: *const fc4_network_rules_capture,
    out: *mut fc4_network_rules_output,
    cap: u32,
) -> i32 {
    use fc4_core::network_rules::Neighbourhood;
    use std::cell::Cell;
    struct Captured<'a> {
        data: &'a fc4_network_rules_capture,
        incomplete: Cell<bool>,
    }
    impl Neighbourhood for Captured<'_> {
        fn centre(&self) -> u32 {
            self.data.centre
        }
        fn edges(&self, m: i32) -> u32 {
            let m = m as u8 as usize;
            if self.data.edge_known[m] != 1 {
                self.incomplete.set(true);
            }
            self.data.edges[m]
        }
        fn valid(&self, m: i32) -> bool {
            match self.data.validity[m as u8 as usize] {
                1 => false,
                2 => true,
                _ => {
                    self.incomplete.set(true);
                    false
                }
            }
        }
    }
    guard(-3, || {
        let (Some(ev), Some(data)) = (
            rules_ref(r)
                .and_then(|r| r.evaluators.get(t as usize))
                .and_then(|v| v.get(index as usize)),
            capture.as_ref(),
        ) else {
            return -3;
        };
        let n = Captured {
            data,
            incomplete: Cell::new(false),
        };
        let result = ev.evaluate(&n);
        if n.incomplete.get() {
            return -2;
        }
        result.map_or(-1, |outputs| {
            fill(
                out,
                cap,
                outputs.iter().map(|o| fc4_network_rules_output {
                    id: o.id,
                    m: o.m,
                    orientation: o.orientation,
                    reserved: [0; 2],
                }),
            ) as i32
        })
    })
}

#[repr(C)]
pub struct fc4_network_rules {
    _private: [u8; 0],
}

#[repr(C)]
pub struct fc4_network_rules_grid {
    _private: [u8; 0],
}

/// What one load did (fc4.h `fc4_network_rules_report`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_network_rules_report {
    pub struct_size: u32,
    /// A tile file's status: `FC4_NR_STATUS_*`; -1 for an INI resource.
    pub status: i32,
    pub transmogrify: u32,
    pub source_rules: u32,
    pub rules: u32,
    pub dropped_variants: u32,
    pub entries: u32,
    pub ignored: u32,
    pub sections: u32,
}

/// One rule output (fc4.h `fc4_network_rules_output`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_network_rules_output {
    pub id: u32,
    pub m: u8,
    pub orientation: u8,
    pub reserved: [u8; 2],
}

/// A cell info (fc4.h `fc4_network_rules_cell`).
///
/// The C view covers what the rule stages that cores18 exported read and
/// write. It does **not** carry `+0x57` (the one-way pass's re-solve mark) or
/// the occupant's paired portal, so a cell that crosses this boundary loses
/// those two; a glue lane that drives `ResolveOneWayConflicts` over tunnels
/// needs them (cores22's lane report).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_network_rules_cell {
    pub struct_size: u32,
    pub x: i32,
    pub z: i32,
    pub flags: u32,
    pub edges: [u32; 13],
    pub union_edges: u32,
    pub one_way: u8,
    pub in_run: u8,
    pub intersection: u8,
    pub fixed: u8,
    pub node: u8,
    pub has_occupant: u8,
    pub occ_has_node_edges: u8,
    pub occ_has_anchor: u8,
    pub step: i32,
    pub solved: i32,
    pub occ_piece: u32,
    pub occ_orientation: u32,
    pub occ_network_flags: u32,
    pub occ_node_edges: [u32; 2],
    pub occ_anchor: [i32; 2],
}

/// A solved record (fc4.h `fc4_network_rules_solved`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_network_rules_solved {
    pub id: u32,
    pub orientation: u32,
    pub x: i32,
    pub z: i32,
}

/// A potential-intersection entry (fc4.h `fc4_network_rules_potential`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_network_rules_potential {
    pub x: i32,
    pub z: i32,
    pub types: u32,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_network_rules_report>() == 36);
    assert!(std::mem::size_of::<fc4_network_rules_output>() == 8);
    assert!(std::mem::size_of::<fc4_network_rules_cell>() == 116);
    assert!(std::mem::size_of::<fc4_network_rules_solved>() == 16);
    assert!(std::mem::size_of::<fc4_network_rules_potential>() == 12);
};

struct Grid {
    grid: CellGrid,
    state: SolveState,
}

fn status_code(s: LoadStatus) -> i32 {
    match s {
        LoadStatus::Missing => 0,
        LoadStatus::Complete => 1,
        LoadStatus::Rejected => 2,
        LoadStatus::BadCentre => 3,
        LoadStatus::Truncated => 4,
    }
}

impl From<&LoadReport> for fc4_network_rules_report {
    fn from(r: &LoadReport) -> Self {
        fc4_network_rules_report {
            struct_size: std::mem::size_of::<fc4_network_rules_report>() as u32,
            status: r.status.map_or(-1, status_code),
            transmogrify: u32::from(r.transmogrify),
            source_rules: r.source_rules as u32,
            rules: r.rules as u32,
            dropped_variants: r.dropped_variants as u32,
            entries: r.entries as u32,
            ignored: r.ignored as u32,
            sections: r.sections as u32,
        }
    }
}

impl From<&CellInfo> for fc4_network_rules_cell {
    fn from(c: &CellInfo) -> Self {
        let occ = c.occupant.unwrap_or_default();
        fc4_network_rules_cell {
            struct_size: std::mem::size_of::<fc4_network_rules_cell>() as u32,
            x: c.x,
            z: c.z,
            flags: c.flags,
            edges: c.edges,
            union_edges: c.union,
            one_way: c.one_way,
            in_run: u8::from(c.in_run),
            intersection: u8::from(c.intersection),
            fixed: u8::from(c.fixed),
            node: u8::from(c.node),
            has_occupant: u8::from(c.occupant.is_some()),
            occ_has_node_edges: u8::from(occ.node_edge_types.is_some()),
            occ_has_anchor: u8::from(occ.anchor.is_some()),
            step: c.step,
            solved: c.solved,
            occ_piece: occ.piece_id,
            occ_orientation: u32::from(occ.orientation),
            occ_network_flags: occ.network_flags,
            occ_node_edges: occ.node_edge_types.map_or([0, 0], |(a, b)| [a, b]),
            occ_anchor: occ.anchor.map_or([0, 0], |(x, z)| [x, z]),
        }
    }
}

impl fc4_network_rules_cell {
    fn to_info(self) -> CellInfo {
        let mut c = CellInfo::new(self.x, self.z);
        c.flags = self.flags;
        c.edges = self.edges;
        c.recompute_union();
        c.one_way = self.one_way;
        c.in_run = self.in_run != 0;
        c.intersection = self.intersection != 0;
        c.fixed = self.fixed != 0;
        c.node = self.node != 0;
        c.step = self.step;
        c.solved = self.solved;
        if self.has_occupant != 0 {
            c.occupant = Some(OccupantView {
                piece_id: self.occ_piece,
                orientation: self.occ_orientation as u8,
                network_flags: self.occ_network_flags,
                node_edge_types: (self.occ_has_node_edges != 0)
                    .then_some((self.occ_node_edges[0], self.occ_node_edges[1])),
                anchor: (self.occ_has_anchor != 0).then_some((self.occ_anchor[0], self.occ_anchor[1])),
                // `fc4_network_rules_cell` has no paired-portal field yet, so
                // a C host cannot describe a tunnel pair; only the one-way
                // pass (§6.5) reads it. See the cores22 lane report.
                paired_portal: None,
            });
        }
        c
    }
}

/// SAFETY: `r` must be NULL or a live pointer from `fc4_network_rules_create`.
unsafe fn rules_mut<'a>(r: *mut fc4_network_rules) -> Option<&'a mut NetworkRules> {
    (r as *mut NetworkRules).as_mut()
}

/// SAFETY: as `rules_mut`.
unsafe fn rules_ref<'a>(r: *const fc4_network_rules) -> Option<&'a NetworkRules> {
    (r as *const NetworkRules).as_ref()
}

/// SAFETY: `g` must be NULL or a live pointer from `fc4_network_rules_grid_create`.
unsafe fn grid_mut<'a>(g: *mut fc4_network_rules_grid) -> Option<&'a mut Grid> {
    (g as *mut Grid).as_mut()
}

/// SAFETY: as `grid_mut`.
unsafe fn grid_ref<'a>(g: *const fc4_network_rules_grid) -> Option<&'a Grid> {
    (g as *const Grid).as_ref()
}

/// SAFETY: `out` must be NULL or valid for `cap` elements.
unsafe fn fill<T: Copy>(out: *mut T, cap: u32, items: impl Iterator<Item = T>) -> u32 {
    let mut n = 0u32;
    for it in items {
        if !out.is_null() && n < cap {
            *out.add(n as usize) = it;
        }
        n += 1;
    }
    n
}

// ---- the rule database ---------------------------------------------------------------------------------

/// An empty rule database (13 empty evaluator lists, no INI data).
#[no_mangle]
pub extern "C" fn fc4_network_rules_create() -> *mut fc4_network_rules {
    guard(std::ptr::null_mut(), || {
        Box::into_raw(Box::new(NetworkRules::new())) as *mut fc4_network_rules
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_network_rules_destroy(r: *mut fc4_network_rules) {
    if !r.is_null() {
        guard((), || drop(Box::from_raw(r as *mut NetworkRules)));
    }
}

/// `LoadResource` (§2.1) from bytes: `instance` is a rule resource's
/// instance (type 0x0A5BCF4B, group 0xAA5BCF57: a tile file, 0x10000000,
/// 0x10000001 or 0x10000002) or 0x8A5993B9 for the network INI. A tile
/// instance appends one evaluator to its type's list. `bytes` may be NULL
/// with `len` 0 (a missing tile file: the evaluator still joins the list).
#[no_mangle]
pub unsafe extern "C" fn fc4_network_rules_load(
    r: *mut fc4_network_rules,
    instance: u32,
    bytes: *const u8,
    len: usize,
    out: *mut fc4_network_rules_report,
) -> bool {
    let Some(rules) = rules_mut(r) else { return false };
    if bytes.is_null() && len != 0 {
        return false;
    }
    let data: &[u8] = if bytes.is_null() {
        &[]
    } else {
        std::slice::from_raw_parts(bytes, len)
    };
    guard(false, || {
        let id = if instance == NETWORK_INI_INSTANCE {
            ResourceId::NetworkIni
        } else {
            ResourceId::Rule(instance)
        };
        let rep = if bytes.is_null() {
            match fc4_core::network_rules::tile_instance_owner(instance) {
                Some((t, _)) => rules.add_tile_file(t, None),
                None => return false,
            }
        } else {
            rules.load_resource(id, data)
        };
        if let Some(o) = out.as_mut() {
            *o = fc4_network_rules_report::from(&rep);
        }
        true
    })
}

/// The tile rule instances of type `t` in load order (§2.2); returns the
/// count (0 for PowerLine or a bad type), writing at most `cap`.
#[no_mangle]
pub unsafe extern "C" fn fc4_network_rules_tile_instances(t: u32, out: *mut u32, cap: u32) -> u32 {
    if t as usize >= TYPE_COUNT {
        return 0;
    }
    guard(0, || fill(out, cap, tile_instances(t as u8).iter().copied()))
}

/// The number of tile evaluators of type `t` (0 for a bad type or rules).
#[no_mangle]
pub unsafe extern "C" fn fc4_network_rules_evaluator_count(r: *const fc4_network_rules, t: u32) -> u32 {
    match rules_ref(r) {
        Some(rules) if (t as usize) < TYPE_COUNT => guard(0, || rules.evaluators[t as usize].len() as u32),
        _ => 0,
    }
}

/// The number of rules of evaluator `index` of type `t` (0 when absent).
#[no_mangle]
pub unsafe extern "C" fn fc4_network_rules_rule_count(r: *const fc4_network_rules, t: u32, index: u32) -> u32 {
    match rules_ref(r) {
        Some(rules) if (t as usize) < TYPE_COUNT => guard(0, || {
            rules.evaluators[t as usize]
                .get(index as usize)
                .map_or(0, |e| e.rule_count() as u32)
        }),
        _ => 0,
    }
}

/// The number of intersection rules (RUL0) read.
#[no_mangle]
pub unsafe extern "C" fn fc4_network_rules_intersection_count(r: *const fc4_network_rules) -> u32 {
    rules_ref(r).map_or(0, |rules| guard(0, || rules.int_rules.len() as u32))
}

/// `Evaluate` (§5) of evaluator `index` of type `t` at cell (x, z) of the
/// grid. Returns -1 when no rule passes (or on a bad argument), else the
/// rule's output count; at most `cap` outputs are written.
#[no_mangle]
pub unsafe extern "C" fn fc4_network_rules_evaluate(
    r: *const fc4_network_rules,
    t: u32,
    index: u32,
    g: *const fc4_network_rules_grid,
    x: i32,
    z: i32,
    out: *mut fc4_network_rules_output,
    cap: u32,
) -> i32 {
    let (Some(rules), Some(grid)) = (rules_ref(r), grid_ref(g)) else {
        return -1;
    };
    if t as usize >= TYPE_COUNT {
        return -1;
    }
    guard(-1, || {
        let Some(ev) = rules.evaluators[t as usize].get(index as usize) else {
            return -1;
        };
        match ev.evaluate(&neighbourhood(&grid.grid, cell_key(x, z), t as u8)) {
            None => -1,
            Some(outs) => fill(
                out,
                cap,
                outs.iter().map(|o| fc4_network_rules_output {
                    id: o.id,
                    m: o.m,
                    orientation: o.orientation,
                    reserved: [0; 2],
                }),
            ) as i32,
        }
    })
}

/// The rule stages of `SolveNetwork` (§6.1) over the grid with tool type
/// `tool_type`; the result goes to `*result`. False on a bad argument.
#[no_mangle]
pub unsafe extern "C" fn fc4_network_rules_solve(
    r: *const fc4_network_rules,
    g: *mut fc4_network_rules_grid,
    tool_type: u32,
    resolve: bool,
    result: *mut i32,
) -> bool {
    let (Some(rules), Some(grid)) = (rules_ref(r), grid_mut(g)) else {
        return false;
    };
    if tool_type as usize >= TYPE_COUNT {
        return false;
    }
    guard(false, || {
        let v = solve_network(rules, &mut grid.grid, &mut grid.state, tool_type as u8, resolve);
        if let Some(o) = result.as_mut() {
            *o = v;
        }
        true
    })
}

// ---- the cell grid and solve state --------------------------------------------------------------------

/// A cell grid of `width` x `height` cells with an empty solve state.
#[no_mangle]
pub extern "C" fn fc4_network_rules_grid_create(width: u32, height: u32) -> *mut fc4_network_rules_grid {
    guard(std::ptr::null_mut(), || {
        Box::into_raw(Box::new(Grid {
            grid: CellGrid::new(width, height),
            state: SolveState::new(),
        })) as *mut fc4_network_rules_grid
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_network_rules_grid_destroy(g: *mut fc4_network_rules_grid) {
    if !g.is_null() {
        guard((), || drop(Box::from_raw(g as *mut Grid)));
    }
}

/// Puts a cell info (replacing the one at its cell). The union edge word is
/// recomputed from `edges`. False for a NULL or short cell.
#[no_mangle]
pub unsafe extern "C" fn fc4_network_rules_grid_set_cell(
    g: *mut fc4_network_rules_grid,
    cell: *const fc4_network_rules_cell,
) -> bool {
    let (Some(grid), Some(c)) = (grid_mut(g), cell.as_ref()) else {
        return false;
    };
    if (c.struct_size as usize) < std::mem::size_of::<fc4_network_rules_cell>() {
        return false;
    }
    guard(false, || {
        grid.grid.insert(c.to_info());
        true
    })
}

/// The cell info at (x, z); false when there is none.
#[no_mangle]
pub unsafe extern "C" fn fc4_network_rules_grid_get_cell(
    g: *const fc4_network_rules_grid,
    x: i32,
    z: i32,
    out: *mut fc4_network_rules_cell,
) -> bool {
    let (Some(grid), Some(o)) = (grid_ref(g), out.as_mut()) else {
        return false;
    };
    guard(false, || match grid.grid.at(x, z) {
        Some(c) => {
            *o = fc4_network_rules_cell::from(c);
            true
        }
        None => false,
    })
}

/// Removes the cell info at (x, z); whether there was one.
#[no_mangle]
pub unsafe extern "C" fn fc4_network_rules_grid_remove_cell(g: *mut fc4_network_rules_grid, x: i32, z: i32) -> bool {
    grid_mut(g).is_some_and(|grid| guard(false, || grid.grid.remove(cell_key(x, z)).is_some()))
}

/// Adds (x, z) to the set `S` of cells to solve; whether it was new.
#[no_mangle]
pub unsafe extern "C" fn fc4_network_rules_grid_add_to_solve(g: *mut fc4_network_rules_grid, x: i32, z: i32) -> bool {
    grid_mut(g).is_some_and(|grid| guard(false, || grid.state.s.insert(cell_key(x, z))))
}

/// Sets the valid count (the step count of the drag, §6.3).
#[no_mangle]
pub unsafe extern "C" fn fc4_network_rules_grid_set_valid_count(g: *mut fc4_network_rules_grid, count: i32) -> bool {
    grid_mut(g).is_some_and(|grid| {
        guard(false, || {
            grid.state.valid_count = count;
            true
        })
    })
}

/// Clears the solve state (S, the solved list, the potential list, the
/// intersection records, the extra cells, the valid count); the cells stay.
#[no_mangle]
pub unsafe extern "C" fn fc4_network_rules_grid_reset_solve(g: *mut fc4_network_rules_grid) -> bool {
    grid_mut(g).is_some_and(|grid| {
        guard(false, || {
            grid.state = SolveState::new();
            true
        })
    })
}

/// The solved list in index order; returns its length, writing at most `cap`.
#[no_mangle]
pub unsafe extern "C" fn fc4_network_rules_grid_solved(
    g: *const fc4_network_rules_grid,
    out: *mut fc4_network_rules_solved,
    cap: u32,
) -> u32 {
    grid_ref(g).map_or(0, |grid| {
        guard(0, || {
            fill(
                out,
                cap,
                grid.state.solved.iter().map(|r| {
                    let (x, z) = key_cell(r.key);
                    fc4_network_rules_solved {
                        id: r.id,
                        orientation: u32::from(r.orientation),
                        x,
                        z,
                    }
                }),
            )
        })
    })
}

/// The potential-intersection list (`+0x08`) in order.
#[no_mangle]
pub unsafe extern "C" fn fc4_network_rules_grid_potential(
    g: *const fc4_network_rules_grid,
    out: *mut fc4_network_rules_potential,
    cap: u32,
) -> u32 {
    grid_ref(g).map_or(0, |grid| {
        guard(0, || {
            fill(
                out,
                cap,
                grid.state
                    .potential
                    .iter()
                    .map(|&(x, z, types)| fc4_network_rules_potential { x, z, types }),
            )
        })
    })
}

/// The cells left in `S`, in its iteration order, as (x, z) pairs: `xz`
/// holds `2 * cap` integers. Returns the count.
#[no_mangle]
pub unsafe extern "C" fn fc4_network_rules_grid_unsolved(
    g: *const fc4_network_rules_grid,
    xz: *mut i32,
    cap: u32,
) -> u32 {
    grid_ref(g).map_or(0, |grid| {
        guard(0, || {
            let keys = grid.state.s.iter_order();
            for (i, &k) in keys.iter().enumerate() {
                if !xz.is_null() && (i as u32) < cap {
                    let (x, z) = key_cell(k);
                    *xz.add(2 * i) = x;
                    *xz.add(2 * i + 1) = z;
                }
            }
            keys.len() as u32
        })
    })
}
