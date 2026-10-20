// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `PlaceNetworkOccupants` (slot 7, `0x00629210`) and
//! `PlaceNetworkOccupantById` (slot 8, `0x00624FD0`), network-tools.md §3
//! and §4.

use super::host::{ToolHost, ToolRun};
use super::state::ToolState;

/// §3.1: the clamped coordinates of the call. `type_arg < 0` takes the
/// tool's own type (step 1).
fn clamp(v: i32, max: u32) -> i32 {
    // Signed clamp into [0, X − 1] (§3.1 step 3).
    v.clamp(0, max.saturating_sub(1) as i32)
}

/// `PlaceNetworkOccupants(x1, z1, x2, z2, mask, callback, real, type)`
/// (slot 7, §3). The arguments are the slot's own (pop 32). `runs` receives one `ToolRun` per straight `PlaceNetwork`
/// call (§3.2 step 2 / §3.3 step 5.4), in call order, so the host can replay
/// the draw/solve/commit pipeline; the pieces and the joint one-cell joins
/// (§3.3 step 6.2) are separate runs.
///
/// Returns AL: the result of the last `PlaceNetwork` (or of the diagonal
/// short-circuit's implicit true, §3.3 step 3).
#[allow(clippy::too_many_arguments)]
pub fn place_network_occupants(
    st: &mut ToolState,
    host: &mut dyn ToolHost,
    x1: i32,
    z1: i32,
    x2: i32,
    z2: i32,
    mask: u32,
    callback: Option<u32>,
    real: bool,
    type_arg: i32,
) -> bool {
    // 1. If `type < 0`: `type := +0x35C` (§3.1 step 1).
    let type_id = if type_arg < 0 { st.type_id } else { type_arg as u32 };
    let mut ok = true;
    place_network_occupants_typed(st, host, x1, z1, x2, z2, mask, callback, real, type_id, &mut ok)
}

/// The body of §3 with the type resolved; `ok` accumulates the result byte.
#[allow(clippy::too_many_arguments)]
fn place_network_occupants_typed(
    st: &mut ToolState,
    host: &mut dyn ToolHost,
    x1: i32,
    z1: i32,
    x2: i32,
    z2: i32,
    mask: u32,
    callback: Option<u32>,
    real: bool,
    type_id: u32,
    ok: &mut bool,
) -> bool {
    // 2. `+0x208 := 3`; `+0x20C := 0` (§3.1 step 2).
    st.data4 = 3;
    st.failure = 0;
    // 3. Clamp each coordinate, signed, into [0, X − 1] and [0, Z − 1].
    let (sx, sz) = (clamp(x1, st.city_x), clamp(z1, st.city_z));
    let (ex, ez) = (clamp(x2, st.city_x), clamp(z2, st.city_z));
    // 4. Clear the solved-cell vector and the step counts `+0x70`, `+0x74`
    // (the geometry core's run fields; §3.1 step 4).
    let mut run = ToolRun::new(type_id, (sx, sz), (ex, ez));
    // 5. If real: clear the world cache (§3.1 step 5; quirk 6 — it stays
    // cleared even when the run then fails, §5.2).
    if real {
        st.world_cache.clear();
    }

    let result = if mask == 0 {
        // §3.2 mask = 0: one straight run.
        straight_run(st, host, &mut run, real, type_id, ok);
        *ok
    } else {
        // §3.3 mask ≠ 0: a run that extends into existing networks.
        masked_run(st, host, x1, z1, x2, z2, mask, callback, real, type_id, ok)
    };

    // §3.4 End: `+0x208 := 1` (`0x00629980`); return the result.
    st.data4 = 1;
    result
}

/// §3.2: one straight run through `DrawNetworkLine` + `PlaceNetwork`.
fn straight_run(
    st: &mut ToolState,
    host: &mut dyn ToolHost,
    run: &mut ToolRun,
    real: bool,
    type_id: u32,
    ok: &mut bool,
) {
    // 1. Anchors := [clamped start, clamped end].
    // 2. `ok := DrawNetworkLine(start, end, false, type)` (slot 18).
    let drawn = host.draw_network_line(run, false);
    let mut good = drawn;
    // 3. If ok: `ok := PlaceNetwork(real, null crew, 0, 0, type)` (§5).
    if good {
        good = place_network(st, host, run, real, None, 0, 0, type_id);
    }
    // 4. If `+0x20C ≠ 0`: `ok := false`.
    if st.failure != 0 {
        good = false;
    }
    *ok = good;
}

/// §3.3: a run that extends into existing networks (`mask ≠ 0`; the zone
/// developer's street drawing passes `0xC49`).
#[allow(clippy::too_many_arguments)]
fn masked_run(
    st: &mut ToolState,
    host: &mut dyn ToolHost,
    x1: i32,
    z1: i32,
    x2: i32,
    z2: i32,
    mask: u32,
    callback: Option<u32>,
    real: bool,
    type_id: u32,
    ok: &mut bool,
) -> bool {
    // 2. `dx := x2 − x1`, `dz := z2 − z1`, from the **unclamped** arguments
    // (quirk 8: a drag that starts outside the city still counts as straight).
    let dx = x2 - x1;
    let dz = z2 - z1;
    // 3. Diagonal (`dx ≠ 0` and `dz ≠ 0`): nothing is placed, and the call
    // returns **true** *[quirk 1]* (the result byte starts at 1).
    if dx != 0 && dz != 0 {
        return true;
    }
    let mut joins: Vec<((i32, i32), (i32, i32))> = Vec::new();
    let mut result = true;

    if dx == 0 && dz == 0 {
        // 4. Single cell: look at the four neighbours with `GetNetworkOccupant`
        // (`0x00624B70`: occupant manager slot 32 with the filter, then QI
        // `0xA821EF94`; false outside the city).
        let mut run = ToolRun::new(type_id, (x1, z1), (x2, z2));
        let west = host.get_network_occupant(x1 - 1, z1, mask).is_some();
        let east = host.get_network_occupant(x1 + 1, z1, mask).is_some();
        let north = host.get_network_occupant(x1, z1 - 1, mask).is_some();
        let south = host.get_network_occupant(x1, z1 + 1, mask).is_some();
        if west || east || north || south {
            // 5. A single cell with a neighbour hit: the straight path below
            // with `d = (sign dx, 0)` and `d = (0, sign dz)` — both zero, so
            // no extension probes run and the cell itself is placed.
            run.start = (x1, z1);
            run.end = (x2, z2);
            straight_run(st, host, &mut run, real, type_id, ok);
            result = *ok;
        } else {
            // 4.3 With no hit, place the single cell as in §3.2.
            straight_run(st, host, &mut run, real, type_id, ok);
            result = *ok;
        }
    } else {
        // 5. Straight (exactly one of `dx`, `dz` non-zero). Directions are
        // `d = (sign dx, 0)`, then `d = (0, sign dz)`. For each direction
        // that is non-zero:
        for (d, along_x) in [((dx.signum(), 0), true), ((0, dz.signum()), false)] {
            if d.0 == 0 && d.1 == 0 {
                continue;
            }
            let mut run = ToolRun::new(type_id, (x1, z1), (x2, z2));
            let (mut sx, mut sz) = (x1, z1);
            let (mut ex, mut ez) = (x2, z2);
            // 5.1 Look one cell before the start (`s' = start − d`) and one
            // cell past the end (`e' = end + d`).
            let (bx, bz) = (sx - d.0, sz - d.1);
            if let Some(other) = host
                .get_network_occupant(bx, bz, mask)
                .filter(|&t| t == type_id || real)
            {
                // 5.2 If a network occupant matching the filter is there:
                // of the same type (`IsOfType(type)`): extend the run to
                // include it; of another type: record a join.
                if other == type_id {
                    sx = bx;
                    sz = bz;
                } else if real {
                    joins.push(((sx, sz), (bx, bz)));
                }
            } else if !real {
                // 5.3 If no occupant is there, in **draft** mode only: if the
                // world-cache cell at that position has flags `& mask ≠ 0`
                // (a drafted piece), extend the run to it.
                if let Some(rec) = st.world_cache.get(&ToolState::cell_key(bx, bz)) {
                    if rec.flags & mask != 0 {
                        sx = bx;
                        sz = bz;
                    }
                }
            }
            let (fx, fz) = (ex + d.0, ez + d.1);
            if let Some(other) = host
                .get_network_occupant(fx, fz, mask)
                .filter(|&t| t == type_id || real)
            {
                if other == type_id {
                    ex = fx;
                    ez = fz;
                } else if real {
                    joins.push(((ex, ez), (fx, fz)));
                }
            } else if !real {
                if let Some(rec) = st.world_cache.get(&ToolState::cell_key(fx, fz)) {
                    if rec.flags & mask != 0 {
                        ex = fx;
                        ez = fz;
                    }
                }
            }
            let _ = along_x;
            // 5.4 Anchors := [start, end]; `DrawNetworkLine`;
            // `PlaceNetwork(real, null, 0, 0, type)`. The result is the last
            // such call's.
            run.start = (sx, sz);
            run.end = (ex, ez);
            straight_run(st, host, &mut run, real, type_id, ok);
            result = *ok;
        }
    }

    // 6. After the loop:
    // 6.1 If `+0x20C ≠ 0`: the result is false.
    if st.failure != 0 {
        result = false;
    }
    // 6.2 If the real byte `+0x50` is set and `callback ≠ null`: for each
    // join, in order, `callback.PlaceNetworkOccupants(a.x, a.z, b.x, b.z, 0,
    // null, real, −1)`. This lays the one-cell link in the callback tool's
    // own type.
    if real {
        if let Some(cb) = callback {
            for (a, b) in joins {
                host.callback_place(cb, a.0, a.1, b.0, b.1, real);
            }
        }
    }
    // 6.3 Free the join list (drop).
    result
}

/// §5 `PlaceNetwork(real, crew, arg3, neighbourMode, type)`, the tool's half
/// of the pipeline. The arguments are the pipeline's own (pop 20). The
/// geometry (the validation passes, the solve, the smoothing) and the commit
/// (the inserts) are the host's, called in §5.1 / §6.1's order; the tool
/// owns the state transitions (the real byte, the failure code, the
/// stop-on-failure refusal, the regional-connection retry).
#[allow(clippy::too_many_arguments)]
pub fn place_network(
    st: &mut ToolState,
    host: &mut dyn ToolHost,
    run: &mut ToolRun,
    real: bool,
    crew: Option<u64>,
    arg3: u32,
    neighbour_mode: u32,
    type_id: u32,
) -> bool {
    // 1. If the dragged step count `+0x70 = 0`: return **true**.
    if run.dragged_count == 0 {
        return true;
    }
    // 2. `+0x50 := real`. Clear the per-run bridge and tunnel vectors.
    st.real = real;
    run.bridges.clear();
    run.tunnels.clear();
    // 3. Network-specific placement (slot 53): the base returns false; the
    // power-line tool's returns true after its own placement → return true
    // at once (§5.1 step 3, §9).
    if host.do_network_specific_placement(run, arg3, neighbour_mode, real) {
        return true;
    }
    // 4-6. The validation passes and the solve, through the host (§4, §8).
    let pass = host.validate_and_solve(run, st.stop_on_failure, real);
    let mut valid = pass.valid_count;
    if let Some(code) = pass.failure {
        st.failure = code;
        if st.stop_on_failure {
            return false;
        }
    }
    // 7. Regional connection, only if `neighbourMode ≠ 0` (§5.1 step 7):
    // `AttemptNeighborConnections` and, if it finds one, a retry of the
    // solve. When real: `PlaceNeighborConnections`.
    if neighbour_mode != 0 && host.attempt_neighbor_connections(run) {
        let retry = host.validate_and_solve(run, st.stop_on_failure, real);
        valid = retry.valid_count;
        if let Some(code) = retry.failure {
            st.failure = code;
            if st.stop_on_failure {
                return false;
            }
        }
        if real {
            host.place_neighbor_connections(run);
        }
    }
    // 8. Commit if `+0x78 > 0`, as in §6. Return **true**.
    if valid > 0 {
        run.data4 = st.data4;
        commit(st, host, run, real, crew, type_id);
    }
    true
}

/// §6 Commit: the insert order (§6.1) and the per-cell rules of §6.2-§6.4,
/// through the host.
fn commit(st: &mut ToolState, host: &mut dyn ToolHost, run: &mut ToolRun, real: bool, crew: Option<u64>, type_id: u32) {
    // 1. Real: slot 70 (a stub); slot 60; slot 57; `MarkPositionChanged…`;
    // `MarkTerrainChanged…`. Draft: `UpdatePositionChangedPreviewsFor…`.
    host.commit_pre(run, real);
    // 2. If preview update is enabled (`+0x234`): the inserts run in order:
    // `InsertOccupantsForSolvedCells`, `InsertBridgePieces`,
    // `InsertTunnelPieces`, `InsertHighwayIntersectionPieces` (§6.1 step 2).
    if st.preview_update {
        host.insert_occupants_for_solved_cells(run, false);
        host.insert_bridge_pieces(run);
        host.insert_tunnel_pieces(run);
        host.insert_highway_intersection_pieces(run);
    }
    // 3. Real:
    if real {
        // 3.1 If `crew ≠ null` and the run qualifies: `crew.AddNetworkSegment`.
        // It qualifies when the straight count equals the valid count, or is
        // 0, or the type's two crew parameters both equal 1 (§6.1 step 3.1).
        if let Some(c) = crew {
            let qualifies = run.straight_count == run.valid_count
                || run.straight_count == 0
                || host.crew_parameters_both_one(type_id);
            if qualifies {
                host.crew_add_network_segment(c, run);
            }
        }
        // 3.2 Always: `MarkOccupantsUsable(&set, release = true)` (§6.3).
        // A code-placed piece is therefore usable as soon as
        // `PlaceNetworkOccupants` returns (quirk 5).
        host.mark_occupants_usable(run, true);
    }
    // 4. Draft: the occupants go into the preview set `+0x228`;
    // `SetTerrainHilight` (§6.1 step 4).
    if !real {
        host.commit_draft(run);
    }
}
