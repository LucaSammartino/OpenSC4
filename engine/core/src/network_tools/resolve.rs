// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `DestroyOccupantsAndResolve` (slot 50, `0x0062A5C0`, network-tools.md §10)
//! and `UpdateUndergroundOccupantTerrainHeights` (`0x006588D0`, §11).

use std::collections::{BTreeMap, BTreeSet};

use super::host::ToolHost;
use super::state::ToolState;
use super::ToolRun;

/// One collected occupant of §10 step 2: its token, its cell and the flags
/// the closure needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Listed {
    /// The occupant (an opaque host token).
    pub occ: u64,
    /// Its cell (network occupant slot 59).
    pub x: i32,
    pub z: i32,
}

/// §10 `DestroyOccupantsAndResolve(list)`: the closure and the rebuild, in
/// §10's order. The messages, the removals and the re-solve go through the
/// host in the spec's order; the tool owns the real byte (`+0x50`), the
/// destroyed byte (`+0x53`) and the cell sets.
///
/// The caller collects the group's occupants (`list`, in pointer order —
/// `std::set` order, so ascending here) and passes them; the core closes the
/// set (step 3), removes (steps 4-5) and rebuilds (step 6).
pub fn destroy_occupants_and_resolve(st: &mut ToolState, host: &mut dyn ToolHost, list: &[Listed]) {
    if list.is_empty() {
        return;
    }
    // 1. Save the real byte; `+0x50 := 1`; `+0x53 := 1`.
    let saved_real = st.real;
    st.real = true;
    st.destroyed_flag = true;

    // 2. Collect the occupants and their cells. A cell key is `(x << 16) + z`
    // for the **destroyed** set (§13 NT1: the destroyed set's key is the
    // swapped one; the re-solve set uses the tool's own `(z << 16) + x`).
    let mut destroyed_cells: BTreeSet<i32> = BTreeSet::new();
    for o in list {
        destroyed_cells.insert((o.x << 16).wrapping_add(o.z));
    }
    let mut listed: Vec<Listed> = list.to_vec();

    // 3. Close the set. Repeat while the list grows:
    //   - a neighbouring **immovable prebuilt** piece (`0x400000`) that is
    //     not an intersection piece joins the list;
    //   - an **intersection** piece (`0x100000`) brings its whole structure
    //     (network manager slot 12, `GetAllIntersectionPieces`);
    //   - a listed occupant with flag `0x80000` is a tunnel portal (§10.3 of
    //     the geometry): QI `0x8A4BD52B` brings in the portal at the other
    //     end.
    // The cells of everything added this way join the destroyed set (§13).
    let mut seen: BTreeSet<u64> = listed.iter().map(|o| o.occ).collect();
    let mut resolve_cells: BTreeSet<i32> = BTreeSet::new();
    let mut k = 0usize;
    while k < listed.len() {
        let o = listed[k];
        // §13: the closure fills the re-solve set with the cells of the
        // removed pieces' **ordinary** neighbours (movable pieces and
        // connection nodes); an immovable prebuilt non-intersection piece
        // instead joins the destroy list itself.
        for n in host.neighbours_by_network_type(o.occ) {
            if seen.contains(&n.occ) {
                continue;
            }
            if n.immovable && !n.intersection {
                seen.insert(n.occ);
                destroyed_cells.insert((n.x << 16).wrapping_add(n.z));
                resolve_cells.insert(ToolState::cell_key(n.x, n.z));
                listed.push(Listed {
                    occ: n.occ,
                    x: n.x,
                    z: n.z,
                });
            } else {
                resolve_cells.insert(ToolState::cell_key(n.x, n.z));
            }
        }
        if host.is_intersection_piece(o.occ) {
            for n in host.intersection_structure(o.occ) {
                if seen.contains(&n) {
                    continue;
                }
                seen.insert(n);
                if let Some((x, z)) = host.occupied_cell(n) {
                    destroyed_cells.insert((x << 16).wrapping_add(z));
                    resolve_cells.insert(ToolState::cell_key(x, z));
                    listed.push(Listed { occ: n, x, z });
                }
            }
        }
        if host.is_tunnel_portal(o.occ) {
            if let Some(other) = host.paired_portal(o.occ) {
                if !seen.contains(&other) {
                    seen.insert(other);
                    if let Some((x, z)) = host.occupied_cell(other) {
                        destroyed_cells.insert((x << 16).wrapping_add(z));
                        resolve_cells.insert(ToolState::cell_key(x, z));
                        listed.push(Listed { occ: other, x, z });
                    }
                }
            }
        }
        k += 1;
    }

    // 4. Clear the world cache (`0x0062B121`).
    st.world_cache.clear();

    // 5. Remove each occupant, in pointer order (`std::set` order — the list
    // arrived sorted and appends keep it; the removal walks it as given).
    for o in &listed {
        // 5.1 Send `0xAA7A014D` **synchronously** (message server slot 3),
        // IGZUnknown = the occupant. Its neighbours answer by dropping their
        // connection to it (demolition.md §3).
        host.send_pre_demolish_sync(o.occ);
        // 5.2 Get its cell (network occupant slot 59).
        let (x, z) = host.occupied_cell(o.occ).unwrap_or((o.x, o.z));
        // 5.3 If the cell key is in the set of **destroyed cells**
        // (`(x << 16) + z`, NT1): `occupantManager.RemoveOccupant(o, false,
        // 0)`. Otherwise (a piece pulled in by the closure)
        // `networkManager.RemoveOrReplaceOccupant(tool, o, &replaced)`,
        // which may put a plain piece back.
        let dkey = (x << 16).wrapping_add(z);
        let replaced = if destroyed_cells.contains(&dkey) {
            host.remove_occupant(o.occ, false);
            false
        } else {
            host.remove_or_replace_occupant(o.occ)
        };
        // 5.4 If a piece was put back, insert the cell into the re-solve set,
        // else erase it from that set; `AsOccupant().Shutdown()`; Release.
        let tkey = ToolState::cell_key(x, z);
        if replaced {
            resolve_cells.insert(tkey);
        } else {
            resolve_cells.remove(&tkey);
        }
        host.occupant_shutdown(o.occ);
        host.release(o.occ);
    }

    // 6. Rebuild the neighbours:
    //   1. `SolveNetwork(−1, true)` over the re-solve set.
    //   2. `InsertOccupantsForSolvedCells` (§6.2). The re-inserted
    //      neighbours carry `data4 = +0x208 = 1`.
    //   3. `MarkOccupantsUsable(list, true)` (§6.3).
    if !resolve_cells.is_empty() {
        let cells: Vec<(i32, i32)> = resolve_cells
            .iter()
            .map(|&key| {
                let x = key & 0xFFFF;
                let z = (key >> 16) & 0xFFFF;
                (x as i16 as i32, z as i16 as i32)
            })
            .collect();
        let mut run = ToolRun::new(st.type_id, cells.first().copied().unwrap_or((0, 0)), (0, 0));
        run.resolve_cells = cells.clone();
        // The re-solve produces the cells' pieces; the inserts walk the same
        // list (§6.2's solved cells).
        run.cells = cells;
        st.data4 = 1;
        run.data4 = st.data4;
        host.solve_network_resolve(&mut run);
        host.insert_occupants_for_solved_cells(&mut run, false);
        host.mark_occupants_usable(&mut run, true);
    }

    // 7. Restore the real byte.
    st.real = saved_real;
    // This step has no cost, and no budget call (§10 end).
}

/// The underground occupant data §11 needs, from the host.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UndergroundPiece {
    /// The occupant.
    pub occ: u64,
    /// Its cell (slot 59).
    pub x: i32,
    pub z: i32,
    /// Slot 94: the 4 stored terrain heights, in the corner order (x, z),
    /// (x, z+1), (x+1, z+1), (x+1, z).
    pub stored_terrain: [f32; 4],
    /// Slot 34: the 4 vertex heights.
    pub vertices: [f32; 4],
    /// Slot 38 with the type: the edge mask.
    pub edges: u32,
}

/// What one smoothed piece gets (§11.1 steps 4-5).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UndergroundOutcome {
    /// The piece moved to the updated list with new vertex and stored
    /// terrain heights (the smooth path, step 4).
    Updated {
        vertex_heights: [f32; 4],
        stored_terrain: [f32; 4],
    },
    /// The piece survives the fallback with its vertex heights unchanged and
    /// its stored terrain updated (step 5.3, quirk 9).
    Kept { stored_terrain: [f32; 4] },
    /// The piece goes on the destroy list (step 5.4).
    Destroy,
}

/// `UpdateUndergroundOccupantTerrainHeights(cells, list, type)`
/// (`0x006588D0`, §11): the pipe/subway refit after a terrain edit. `type`
/// is 4 or 7; the caller holds the pieces (AddRef'd, §11's `list`).
///
/// The height solve itself is the geometry core's
/// (`fc4_core::network_geometry`); this function decides which pieces move
/// where and drives the host in §11.1's order.
pub fn update_underground_occupant_terrain_heights(
    st: &mut ToolState,
    host: &mut dyn ToolHost,
    pieces: &[UndergroundPiece],
    network_type: u32,
) {
    // 1. Clear the world cache, the solved-cell set `+0xA4` and the
    // solved-cell vector `+0x98`.
    st.world_cache.clear();
    st.solved_cells.clear();
    st.solved_set.clear();

    // 2. For each occupant `n` in `list`, in list order:
    //   2.1 read its stored terrain heights (slot 94), its cell (slot 59) and
    //   the terrain's current corner altitudes (slot 17);
    //   2.2 If all four are **equal** (float compare), skip `n`.
    let mut todo: Vec<UndergroundPiece> = Vec::new();
    for p in pieces {
        let cur = host.corner_altitudes(p.x, p.z);
        if cur == p.stored_terrain {
            continue;
        }
        todo.push(*p);
    }

    // 3. Step counts: dragged := 2, valid := 2, straight := 1. Then
    // `SmoothenNetwork(2)` (`0x00637A80`).
    let mut run = ToolRun::new(network_type, (0, 0), (0, 0));
    run.dragged_count = 2;
    run.valid_count = 2;
    run.straight_count = 1;
    let mut records: BTreeMap<i32, Vec<(i32, [f32; 4])>> = BTreeMap::new();
    for p in &todo {
        // 2.3 Otherwise make a cell record: the occupant's vertex heights
        // (slot 34) and the type's edge mask (slot 38 with the type);
        // recomputed edges; for each corner, the allowed range is from
        // **−500.0** up to `terrain corner − depth` (`+0x36C`), with a target
        // of `terrain corner − depth`.
        let cur = host.corner_altitudes(p.x, p.z);
        let target = [
            cur[0] - st.depth,
            cur[1] - st.depth,
            cur[2] - st.depth,
            cur[3] - st.depth,
        ];
        records
            .entry(ToolState::cell_key(p.x, p.z))
            .or_default()
            .push((p.occ as i32, target));
        // 2.4 Put the record into the world cache and the solved-cell set
        // and vector (§11.1 step 2.4).
        st.solved_cells.push((p.x, p.z));
        st.solved_set.insert(ToolState::cell_key(p.x, p.z));
    }
    let smoothed = host.smoothen_underground(&mut run, 2);

    // Step counts land in the run; the outcomes below follow §11.1.
    let mut updated: Vec<(UndergroundPiece, UndergroundOutcome)> = Vec::new();
    let mut destroyed: Vec<u64> = Vec::new();

    if smoothed {
        // 4. **Smoothing succeeded.** For each cell in the solved-cell set
        // (hash order), with `FindUndergroundOccupant(x, z, class)` (slot 81
        // class) finding a piece:
        for &(x, z) in &st.solved_cells {
            let Some((occ, solved)) = host.find_underground_solved(x, z) else {
                continue;
            };
            let Some(p) = pieces.iter().copied().find(|p| p.occ == occ) else {
                continue;
            };
            // 4.1 For each corner, read the terrain altitude and the smoothed
            // height (`cSC4VertexHtConstraintSatisfier::GetHeight`).
            let cur = host.corner_altitudes(x, z);
            let mut vh = p.vertices;
            for (k, h) in solved.iter().copied().enumerate().take(4) {
                vh[k] = h;
            }
            // 4.2 Set the piece's vertex heights (slot 84) from the solved
            // record and its stored terrain heights (slot 95).
            // 4.3 Move it from `list` (Release) to the **updated** list.
            updated.push((
                p,
                UndergroundOutcome::Updated {
                    vertex_heights: vh,
                    stored_terrain: cur,
                },
            ));
        }
    } else {
        // 5. **Smoothing failed.** `margin := +0x378 + +0x370` (float add).
        let margin = st.cover_margin_a + st.cover_margin_b;
        for p in pieces {
            let cur = host.corner_altitudes(p.x, p.z);
            if cur == p.stored_terrain {
                continue;
            }
            // 5.2 The piece **survives** if, for every corner,
            // `terrain altitude ≥ h[i] + margin`. The compare is false only
            // for a strict `<`; a NaN survives *[compat]*.
            let survives = (0..4).all(|k| {
                let h = p.vertices[k] + margin;
                // The compare is false only for a strict `<`; a NaN survives
                // *[compat]* (§11.1 step 5.2) — `!(a < b)` on purpose.
                #[allow(clippy::neg_cmp_op_on_partial_ord)]
                !(cur[k] < h)
            });
            if survives {
                // 5.3 A survivor gets its stored terrain heights updated
                // (slot 95), but its vertex heights are **unchanged**.
                updated.push((*p, UndergroundOutcome::Kept { stored_terrain: cur }));
            } else {
                // 5.4 A piece that fails goes on a destroy list (its
                // `AsOccupant`).
                destroyed.push(p.occ);
            }
        }
        // Then `networkManager.DestroyNetworkOccupants(destroyList)` (§7.2,
        // which calls back into §10 with tool 4 or 7) and
        // `occupantManager.ReleaseOccupantList` (slot 48).
        if !destroyed.is_empty() {
            host.destroy_network_occupants(&destroyed);
            host.release_occupant_list(&destroyed);
        }
    }

    // 6. For each piece in the updated list:
    //   1. slot 86 (true);
    //   2. `SetUsable(false)`;
    //   3. slot 50;
    //   4. `SetUsable(true)`;
    //   5. Release.
    // The usable toggle posts `0x89D3E9F3` data1 = 0, then data1 = 1, so
    // pipe and subway listeners rebuild the cell.
    for (p, outcome) in &updated {
        host.underground_rebuild(p.occ, outcome);
    }

    // 7. Release every remaining entry of `list` and empty it; clear the
    // solved-cell vector and set; free the cell set.
    let updated_tokens: BTreeSet<u64> = updated.iter().map(|(p, _)| p.occ).collect();
    let destroyed_set: BTreeSet<u64> = destroyed.iter().copied().collect();
    for p in pieces {
        if !updated_tokens.contains(&p.occ) && !destroyed_set.contains(&p.occ) {
            host.release(p.occ);
        }
    }
    st.solved_cells.clear();
    st.solved_set.clear();
}
