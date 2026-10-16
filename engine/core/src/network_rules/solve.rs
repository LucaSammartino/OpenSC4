// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The rule stages of `SolveNetwork` (§6), the edge subtypes (§7), the
//! multi-type intersections (§8.2) and the tile subsets (§9).

use crate::stl::{StlHashSet, STL_DEFAULT_HINT};

use super::grid::{CellGrid, SolveState, SolvedRecord};
use super::rules::NetworkRules;
use super::tile::{Evaluator, Neighbourhood, Output};
use super::{
    cell_key, covers, d_mask, edge_byte, elevated, key_cell, meta_offset, reset_minor_types, type_bit, type_of,
    with_edge_byte, within, ALL_TYPES, AVENUE, DIRT_ROAD, ONE_WAY_ROAD, PRIORITY, ROAD, SIDE_DX, SIDE_DZ, STREET,
    SUBTYPE_SCHEME, TYPE_COUNT,
};

/// The bucket hint of the sets the spec does not size (`P`, `Q`, `R`): the
/// default 100 (193 buckets) *[choice]*.
const SET_HINT: usize = STL_DEFAULT_HINT;

/// The through-mark walk's step cap (§7.1): the original walks without a
/// limit; a walk longer than this fails *[choice]*.
const WALK_CAP: usize = 1 << 16;

// ---- §6.2 ------------------------------------------------------------------------------------------------

/// `BuildPaddedNetworkSet(src, out, pad)` (`0x00634480`): for each key of
/// `src`, every cell of the square of half-width `pad` around it (clamped to
/// the city) that has a cell info and is not fixed; the source cell itself
/// even when fixed. The square is scanned z outer, x inner *[choice]*; the
/// new set has the default bucket hint *[choice]*.
pub fn build_padded_network_set(grid: &CellGrid, src: &StlHashSet, pad: i32) -> StlHashSet {
    let mut out = StlHashSet::with_hint(SET_HINT);
    let (w, h) = (grid.width as i32, grid.height as i32);
    for key in src.iter_order() {
        let (x, z) = key_cell(key);
        for zz in (z - pad).max(0)..=(z + pad).min(h - 1) {
            for xx in (x - pad).max(0)..=(x + pad).min(w - 1) {
                let k = cell_key(xx, zz);
                let take = k == key || grid.get(k).is_some_and(|c| !c.fixed);
                if take {
                    out.insert(k);
                }
            }
        }
    }
    out
}

// ---- §5.1 ------------------------------------------------------------------------------------------------

/// The neighbourhood of a cell for type `t` over a [`CellGrid`] (§5.1), with
/// the valid-position check on (the solver always builds it so).
pub struct GridNeighbourhood<'a> {
    grid: &'a CellGrid,
    x: i32,
    z: i32,
    t: u8,
}

/// The neighbourhood of the cell `key` for type `t`.
pub fn neighbourhood(grid: &CellGrid, key: u32, t: u8) -> GridNeighbourhood<'_> {
    let (x, z) = key_cell(key);
    GridNeighbourhood { grid, x, z, t }
}

impl Neighbourhood for GridNeighbourhood<'_> {
    fn centre(&self) -> u32 {
        self.grid
            .at(self.x, self.z)
            .map_or(0, |c| c.edges[self.t as usize % TYPE_COUNT])
    }

    fn edges(&self, m: i32) -> u32 {
        let (dx, dz) = meta_offset(m);
        self.grid
            .at(self.x + dx, self.z + dz)
            .map_or(0, |c| c.edges[self.t as usize % TYPE_COUNT])
    }

    fn valid(&self, m: i32) -> bool {
        let (dx, dz) = meta_offset(m);
        let other = type_bit(self.t) ^ ALL_TYPES;
        self.grid
            .at(self.x + dx, self.z + dz)
            .map_or(true, |c| c.flags & other == 0)
    }
}

// ---- §6.4, §6.3 -----------------------------------------------------------------------------------------

/// Condition 2.3 of §6.4 and the rest of "qualifies" except `Evaluate`.
fn qualifies(grid: &CellGrid, c: u32, u: u8) -> bool {
    let Some(info) = grid.get(c) else { return false };
    if info.flags & ALL_TYPES != type_bit(u) {
        return false;
    }
    if !info.node {
        return true;
    }
    // A connection node without an occupant does not qualify *[choice]*.
    let Some(occ) = info.occupant else { return false };
    if occ.network_flags & ALL_TYPES != info.flags {
        return false;
    }
    match occ.node_edge_types {
        None => true,
        Some((a, b)) => {
            let e = info.edges[u as usize % TYPE_COUNT];
            e & 0xF8F8_F8F8 == 0 && within(a, e) && within(e, b)
        }
    }
}

/// `ApplyNetworkRules(P, S, W, evaluator, ·, u)` (`0x006340D0`, §6.4).
pub fn apply_network_rules(
    grid: &mut CellGrid,
    state: &mut SolveState,
    p: &mut StlHashSet,
    w: &mut StlHashSet,
    ev: &Evaluator,
    u: u8,
) {
    while !state.s.is_empty() {
        let Some(c) = p.first() else { break };
        let outputs: Option<Vec<Output>> = if qualifies(grid, c, u) {
            ev.evaluate(&neighbourhood(grid, c, u)).map(<[Output]>::to_vec)
        } else {
            None
        };
        let Some(outputs) = outputs else {
            p.remove(c);
            w.insert(c);
            continue;
        };
        let (cx, cz) = key_cell(c);
        let mut self_claimed = false;
        for o in outputs {
            let (dx, dz) = meta_offset(i32::from(o.m));
            let target = cell_key(cx + dx, cz + dz);
            if target == c {
                self_claimed = true;
            }
            if !p.remove(target) && !w.remove(target) {
                continue;
            }
            if state.s.remove(target) {
                state.append_solved(
                    grid,
                    SolvedRecord {
                        id: o.id,
                        orientation: o.orientation,
                        key: target,
                    },
                );
            }
        }
        if !self_claimed {
            p.remove(c);
            w.insert(c);
        }
    }
}

/// `SolveUsingEvaluator(P, S, evaluator, u)` (`0x006358C0`, §6.3).
pub fn solve_using_evaluator(grid: &mut CellGrid, state: &mut SolveState, p: &mut StlHashSet, ev: &Evaluator, u: u8) {
    let mut w = StlHashSet::with_hint(100);
    apply_network_rules(grid, state, p, &mut w, ev, u);
    for k in w.iter_order() {
        p.insert(k);
    }
}

// ---- §7 -----------------------------------------------------------------------------------------------

fn kinds(e: u32) -> (Vec<usize>, Vec<usize>) {
    let mut orth = Vec::new();
    let mut diag = Vec::new();
    for s in 0..4 {
        match edge_byte(e, s) & 7 {
            2 => orth.push(s),
            1 | 3 => diag.push(s),
            _ => {}
        }
    }
    (orth, diag)
}

/// Purely diagonal (`0x00623C90`): no orthogonal side and one or two
/// diagonal sides.
fn purely_diagonal(e: u32) -> bool {
    let (orth, diag) = kinds(e);
    orth.is_empty() && (1..=2).contains(&diag.len())
}

fn across(key: u32, side: usize) -> u32 {
    let (x, z) = key_cell(key);
    cell_key(x + SIDE_DX[side & 3], z + SIDE_DZ[side & 3])
}

fn set_byte(grid: &mut CellGrid, key: u32, t: u8, side: usize, b: u8) {
    if let Some(c) = grid.get_mut(key) {
        let e = with_edge_byte(c.edges[t as usize], side, b);
        c.set_edges(t, e);
    }
}

/// One walk of the through marks (§7.1).
fn walk(grid: &CellGrid, start: u32, t: u8, mut dir: usize, mut kind: u8) -> bool {
    let mut cur = start;
    for _ in 0..WALK_CAP {
        let n = across(cur, dir);
        let Some(info) = grid.get(n) else { return false };
        let e = info.edges[t as usize];
        if purely_diagonal(e) {
            return true;
        }
        dir = if kind == 3 { (dir + 1) & 3 } else { (dir + 3) & 3 };
        let k2 = edge_byte(e, dir) & 3;
        if k2 + kind != 4 {
            return false;
        }
        cur = n;
        kind = k2;
    }
    false
}

/// The through marks (`0x006335E0`, §7.1).
fn through_marks(grid: &CellGrid, c: u32, t: u8) -> [u8; 4] {
    let e = grid.get(c).map_or(0, |i| i.edges[t as usize]);
    let mut m = [0u8; 4];
    for s in 0..4 {
        if m[s] != 0 {
            continue;
        }
        let k = edge_byte(e, s) & 3;
        let s2 = match k {
            1 => (s + 1) & 3,
            3 => (s + 3) & 3,
            _ => continue,
        };
        let k2 = edge_byte(e, s2) & 3;
        if k + k2 != 4 {
            continue;
        }
        if walk(grid, c, t, s, k) && walk(grid, c, t, s2, k2) {
            m[s] = 1;
            m[s2] = 1;
        }
    }
    m
}

/// Scheme 1, diagonal stubs (`0x00633A60`, §7.1).
fn scheme_1(grid: &mut CellGrid, p: &StlHashSet, c: u32, t: u8, in_p: bool) -> bool {
    let Some(info) = grid.get(c) else { return true };
    let (orth, diag) = kinds(info.edges[t as usize]);
    if orth.is_empty() || diag.is_empty() {
        return true;
    }
    let marks = through_marks(grid, c, t);
    for s in diag {
        if marks[s] != 0 {
            continue;
        }
        let n = across(c, s);
        let Some(ninfo) = grid.get(n) else { continue };
        let pd = purely_diagonal(ninfo.edges[t as usize]);
        let n_fixed = ninfo.fixed;
        let stub = |b: u8| -> u8 {
            let b = (b & 7) | (u8::from(pd) << 3);
            if pd {
                b
            } else {
                (b & 0xF8) | 2
            }
        };
        if in_p {
            let ci = grid.get(c).copied().unwrap_or_default();
            if ci.fixed && pd {
                return false;
            }
            set_byte(grid, c, t, s, stub(edge_byte(ci.edges[t as usize], s)));
        }
        if p.contains(n) {
            let o = (s + 2) & 3;
            let b = grid.get(n).map_or(0, |i| edge_byte(i.edges[t as usize], o));
            set_byte(grid, n, t, o, stub(b));
        } else if pd && n_fixed {
            return false;
        }
    }
    true
}

/// Scheme 2, switches and curves (`0x00633C10`, §7.2).
fn scheme_2(grid: &mut CellGrid, p: &StlHashSet, c: u32, t: u8, in_p: bool) -> bool {
    for i in 0..4usize {
        let Some(info) = grid.get(c) else { return true };
        let e = info.edges[t as usize];
        let kind = |j: usize| edge_byte(e, j) & 3;
        let k = kind(i);
        if k == 0 {
            continue;
        }
        let (opp, left, right) = ((i + 2) & 3, (i + 3) & 3, (i + 1) & 3);
        let subtype: u8 = if k == 2 {
            let f =
                u8::from(kind(opp) != 0) | if kind(left) == 1 { 2 } else { 0 } | if kind(right) == 3 { 4 } else { 0 };
            if f <= 1 {
                continue;
            }
            f
        } else {
            if kind(opp) == 0 || (kind(opp) == k && k + kind(left) == 4) {
                continue;
            }
            let pk = if k == 1 { kind(right) } else { kind(left) };
            if pk == 2 || pk + k == 4 {
                2
            } else {
                1
            }
        };
        let nb = (subtype << 3) | k;
        if in_p {
            if info.fixed {
                return false;
            }
            set_byte(grid, c, t, i, nb);
        }
        let n = across(c, i);
        if p.contains(n) {
            set_byte(grid, n, t, opp, nb);
        } else if grid.get(n).is_some_and(|ni| ni.fixed) {
            return false;
        }
    }
    true
}

/// `AssignEdgeSubtypes(P, R, reset)` (`0x006359B0`, §7). The cells whose
/// subtypes could not be assigned leave `p` and go into `r`.
pub fn assign_edge_subtypes(grid: &mut CellGrid, p: &mut StlHashSet, r: &mut StlHashSet, reset: bool) {
    let q = build_padded_network_set(grid, p, 1);
    let order = q.iter_order();
    if reset {
        for &c in &order {
            let Some(info) = grid.get_mut(c) else { continue };
            if info.flags.count_ones() <= 1 && p.contains(c) {
                // The type of a cell with no bit: typeOf's 12 *[inferred]*.
                let t = type_of(info.flags) as usize;
                let e = reset_minor_types(info.edges[t]);
                info.set_edges(t as u8, e);
            }
        }
    }
    for &c in &order {
        let Some(info) = grid.get(c) else { continue };
        if info.flags.count_ones() != 1 {
            continue;
        }
        let t = type_of(info.flags);
        let in_p = p.contains(c);
        let ok = match SUBTYPE_SCHEME[t as usize] {
            1 => scheme_1(grid, p, c, t, in_p),
            2 => scheme_2(grid, p, c, t, in_p),
            _ => true,
        };
        if !ok {
            p.remove(c);
            r.insert(c);
        }
    }
}

// ---- §8.2 -----------------------------------------------------------------------------------------------

fn beats(j: u8, i: u8, primary: u8) -> bool {
    let (pi, pj) = (PRIORITY[i as usize], PRIORITY[j as usize]);
    pi < pj || (pi == pj && i != primary && j == primary)
}

fn remove_type(grid: &mut CellGrid, c: u32, i: u8) {
    if let Some(info) = grid.get_mut(c) {
        info.flags &= !type_bit(i);
        info.set_edges(i, 0);
        if i == ONE_WAY_ROAD {
            info.one_way = 0;
        }
    }
}

/// `SolveMultiTypeIntersections(S, out, primary)` (`0x00634820`, §8.2).
/// The cells solved here are appended to `out`.
pub fn solve_multi_type_intersections(
    rules: &NetworkRules,
    grid: &mut CellGrid,
    state: &mut SolveState,
    out: &mut Vec<u32>,
    primary: u8,
) {
    for c in state.s.iter_order() {
        let Some(start) = grid.get(c).copied() else { continue };
        // The edge words read when the cell's analysis started.
        let e = start.edges;
        loop {
            let Some(info) = grid.get(c).copied() else { break };
            let f = info.flags;
            let lo = f & f.wrapping_neg();
            let rest = f - lo;
            if rest == 0 {
                break;
            }
            if rest & (rest - 1) != 0 {
                let rl = rest & rest.wrapping_neg();
                let abc = [type_of(lo), type_of(rl), type_of(rest - rl)];
                let found = abc.iter().find_map(|&i| {
                    abc.iter()
                        .any(|&j| {
                            beats(j, i, primary)
                                && covers(e[j as usize], e[i as usize])
                                && elevated(type_bit(i)) == elevated(type_bit(j))
                        })
                        .then_some(i)
                });
                let Some(i) = found else { break };
                remove_type(grid, c, i);
                // A removal that leaves the flags as they were (a non-type
                // flag read as type 12) ends the cell's analysis *[choice]*.
                if grid.get(c).map_or(true, |n| n.flags == f) {
                    break;
                }
                continue;
            }
            if info.node {
                break;
            }
            let (a, b) = (type_of(lo), type_of(rest));
            let (w, l, w_bits, l_bits) = if beats(b, a, primary) {
                (b, a, rest, lo)
            } else {
                (a, b, lo, rest)
            };
            let e_w = e[w as usize];
            let mut e_l = e[l as usize];
            if covers(e_w, e_l) {
                if rules.override_rows[w as usize] & (1 << l) != 0 {
                    if let Some(ci) = grid.get_mut(c) {
                        ci.set_edges(l, 0);
                        ci.flags = w_bits;
                        if l == ONE_WAY_ROAD {
                            ci.one_way = 0;
                        }
                    }
                }
                break;
            }
            let Some(ci) = grid.get_mut(c) else { break };
            if elevated(w_bits) == elevated(l_bits) {
                for side in 0..4 {
                    let bl = edge_byte(e_l, side) & 3;
                    if bl != 0 && bl == edge_byte(e_w, side) & 3 {
                        e_l = with_edge_byte(e_l, side, 0);
                    }
                }
                ci.set_edges(l, e_l);
                if l == ONE_WAY_ROAD {
                    ci.one_way &= d_mask(e_l);
                }
            }
            let u = ci.edges[ROAD as usize]
                | ci.edges[STREET as usize]
                | ci.edges[AVENUE as usize]
                | ci.edges[DIRT_ROAD as usize];
            ci.one_way &= !d_mask(u);
            let (lo_t, hi_t) = (w.min(l), w.max(l));
            let word = |t: u8| if t == l { e_l } else { e_w };
            if let Some((id, o)) = rules.solution(lo_t, hi_t, word(lo_t), word(hi_t)) {
                state.append_solved(
                    grid,
                    SolvedRecord {
                        id,
                        orientation: o,
                        key: c,
                    },
                );
                out.push(c);
                let (x, z) = key_cell(c);
                state.potential.push((x, z, f));
            }
            break;
        }
    }
}

// ---- §9.1 -----------------------------------------------------------------------------------------------

/// A piece: an id and an orientation byte.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TilePiece {
    pub id: u32,
    pub orientation: u8,
}

/// The frame where B is east of A (§9.1 step 1). Bits 2-6 of the
/// orientation are dropped *[choice]*.
fn normalise(o: u8, s: usize) -> u8 {
    let s = s as u8;
    if o & 0x80 == 0 {
        o.wrapping_add(2).wrapping_sub(s) & 3
    } else {
        0x80 | (o.wrapping_add(s).wrapping_add(2) & 3)
    }
}

fn denormalise(o: u8, s: usize) -> u8 {
    let s = s as u8;
    if o & 0x80 == 0 {
        o.wrapping_add(s).wrapping_add(2) & 3
    } else {
        0x80 | (o.wrapping_add(2).wrapping_sub(s) & 3)
    }
}

/// `PatchTilePair(A, B, s)` (`0x006337E0`, §9.1): 0 no match, 1 patched (A
/// and B changed), 2 a conflict that cannot be patched.
pub fn patch_tile_pair(rules: &NetworkRules, a: &mut TilePiece, b: &mut TilePiece, s: usize) -> u8 {
    let Some(patterns) = rules.rule_overrides.get(&a.id) else {
        return 0;
    };
    let (na, nb) = (normalise(a.orientation, s), normalise(b.orientation, s));
    for pat in patterns.iter().filter(|p| p.pattern[1].0 == b.id) {
        let mirrored = (pat.pattern[0].1 ^ na) & 0x80 != 0;
        let (ca, cb) = if mirrored { (na ^ 0x82, nb ^ 0x82) } else { (na, nb) };
        if pat.pattern[0].1 != ca || (b.id != 0 && pat.pattern[1].1 != cb) {
            continue;
        }
        let [(ra, oa), (rb, ob)] = pat.replacement;
        if ra == 0 {
            return 2;
        }
        let fix = |o: u8| denormalise(if mirrored { o ^ 0x82 } else { o }, s);
        *a = TilePiece {
            id: ra,
            orientation: fix(oa),
        };
        *b = TilePiece {
            id: rb,
            orientation: fix(ob),
        };
        return 1;
    }
    0
}

/// `AdjustTileSubsets(solved, resolve)` (`0x00634D70`, §9.1).
pub fn adjust_tile_subsets(rules: &NetworkRules, grid: &mut CellGrid, state: &mut SolveState, resolve: bool) -> bool {
    if rules.rule_overrides.is_empty() {
        return true;
    }
    let mut budget: i64 = (8 * state.solved.len() as i64).max(100);
    'pass: loop {
        let mut changed = false;
        let mut i = 0usize;
        'records: while i < state.solved.len() {
            let mut tries = 0u32;
            let mut s = 0usize;
            while s < 4 {
                let rec = state.solved[i];
                let n = across(rec.key, s);
                let Some(ninfo) = grid.get(n).copied() else {
                    s += 1;
                    continue;
                };
                let listed = (ninfo.solved >= 0)
                    .then(|| state.solved.get(ninfo.solved as usize).copied())
                    .flatten();
                let (mut b, mut existing) = match (listed, ninfo.occupant) {
                    (Some(r), _) => (
                        TilePiece {
                            id: r.id,
                            orientation: r.orientation,
                        },
                        false,
                    ),
                    (None, Some(o)) => (
                        TilePiece {
                            id: o.piece_id,
                            orientation: o.orientation,
                        },
                        true,
                    ),
                    (None, None) => {
                        s += 1;
                        continue;
                    }
                };
                if ninfo.fixed || ninfo.node {
                    b.id = 0;
                    existing = true;
                }
                let mut a = TilePiece {
                    id: rec.id,
                    orientation: rec.orientation,
                };
                let mut r = patch_tile_pair(rules, &mut a, &mut b, s);
                if r == 0 && existing {
                    r = patch_tile_pair(rules, &mut b, &mut a, (s + 2) & 3);
                }
                match r {
                    1 => {
                        budget -= 1;
                        if budget < 0 {
                            return false;
                        }
                        changed = true;
                        state.solved[i].id = a.id;
                        state.solved[i].orientation = a.orientation;
                        if existing {
                            state.append_solved(
                                grid,
                                SolvedRecord {
                                    id: b.id,
                                    orientation: b.orientation,
                                    key: n,
                                },
                            );
                        } else if let Some(slot) = state.solved.get_mut(ninfo.solved as usize) {
                            slot.id = b.id;
                            slot.orientation = b.orientation;
                        }
                        tries += 1;
                        if tries > 100 {
                            i += 1;
                            continue 'records;
                        }
                        s = 0;
                    }
                    2 => {
                        if !resolve {
                            return false;
                        }
                        state.unsolve(grid, i);
                        continue 'pass;
                    }
                    _ => s += 1,
                }
            }
            i += 1;
        }
        if !changed {
            return true;
        }
    }
}

// ---- §9.2 -----------------------------------------------------------------------------------------------

/// The piece at a cell for the verifiers: its solved record, else its
/// occupant's, else id 0 orientation 0.
fn piece_at(grid: &CellGrid, state: &SolveState, x: i32, z: i32) -> Option<TilePiece> {
    let c = grid.at(x, z)?;
    if c.solved >= 0 {
        if let Some(r) = state.solved.get(c.solved as usize) {
            return Some(TilePiece {
                id: r.id,
                orientation: r.orientation,
            });
        }
    }
    Some(c.occupant.map_or(TilePiece::default(), |o| TilePiece {
        id: o.piece_id,
        orientation: o.orientation,
    }))
}

/// `VerifyTileSubsets(solved)` (`0x00635290`, §9.2): true when nothing was
/// removed.
pub fn verify_tile_subsets(rules: &NetworkRules, grid: &mut CellGrid, state: &mut SolveState) -> bool {
    if rules.verifiers.is_empty() {
        return true;
    }
    let mut ok = true;
    let mut i = 0usize;
    while i < state.solved.len() {
        let a = state.solved[i];
        let Some(list) = rules.verifiers.get(&a.id) else {
            i += 1;
            continue;
        };
        let (ax, az) = key_cell(a.key);
        let passes = list.iter().any(|reqs| {
            reqs.iter().all(|q| {
                let (mut dx, mut dz) = meta_offset(q.m);
                if a.orientation & 1 != 0 {
                    (dx, dz) = (-dz, dx);
                }
                if a.orientation & 2 != 0 {
                    (dx, dz) = (-dx, -dz);
                }
                if a.orientation & 0x80 != 0 {
                    dx = -dx;
                }
                let (x, z) = (ax + dx, az + dz);
                if !grid.inside(x, z) {
                    return false;
                }
                let Some(p) = piece_at(grid, state, x, z) else {
                    return false;
                };
                let rel = if (p.orientation ^ a.orientation) & 0x80 != 0 {
                    p.orientation.wrapping_add(a.orientation) & 0x83
                } else {
                    p.orientation.wrapping_sub(a.orientation).wrapping_add(8) & 0x83
                };
                p.id == q.id && rel == q.orientation
            })
        });
        if passes {
            i += 1;
        } else {
            state.unsolve(grid, i);
            ok = false;
        }
    }
    ok
}

// ---- §6.1 -----------------------------------------------------------------------------------------------

/// The unsolved-cell rule (network-geometry.md §8 step 3.5): the valid count
/// `+0x78`, lowered to the smallest step index of an unsolved cell of the
/// run, then by one, once, when an unsolved cell is outside the run (a cell
/// of `S` without a cell info counts as outside *[inferred]*).
pub fn unsolved_cell_result(grid: &CellGrid, state: &SolveState) -> i32 {
    let mut r = state.valid_count;
    let mut outside = false;
    for k in state.s.iter_order() {
        match grid.get(k) {
            Some(c) if c.in_run => r = r.min(c.step),
            _ => outside = true,
        }
    }
    if outside {
        r - 1
    } else {
        r
    }
}

/// `SolveNetwork(type, resolve)`'s rule stages (`0x00635CB0`, §6.1) over an
/// explicit cell grid: returns the result count (a value below
/// `state.valid_count` fails the run with `0xF0000007`).
pub fn solve_network(
    rules: &NetworkRules,
    grid: &mut CellGrid,
    state: &mut SolveState,
    tool_type: u8,
    resolve: bool,
) -> i32 {
    let mut p = build_padded_network_set(grid, &state.s, 2);
    let mut r = StlHashSet::with_hint(SET_HINT);
    assign_edge_subtypes(grid, &mut p, &mut r, true);
    let r_keys = r.iter_order();
    for &k in &r_keys {
        state.s.remove(k);
    }
    let mut out = Vec::new();
    solve_multi_type_intersections(rules, grid, state, &mut out, tool_type);
    for k in out {
        state.s.remove(k);
        p.remove(k);
    }
    for u in 0..TYPE_COUNT as u8 {
        for ev in &rules.evaluators[u as usize] {
            solve_using_evaluator(grid, state, &mut p, ev, u);
        }
    }
    for k in r_keys {
        state.s.insert(k);
    }
    if !resolve && !state.s.is_empty() {
        return unsolved_cell_result(grid, state);
    }
    if !adjust_tile_subsets(rules, grid, state, resolve) || !verify_tile_subsets(rules, grid, state) {
        return state.valid_count - 1;
    }
    unsolved_cell_result(grid, state)
}
