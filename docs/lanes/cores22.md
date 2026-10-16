| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| cores22 (network rules: NG5's open stages and the highway intersections) | `cores22` / (engine lane) | **ready** (core and conformance only, no plugin glue, no C ABI change, ABI stays 69; gate cores22 at lane `6f50dae`: cargo 2870/0, clippy clean, rustfmt clean on every touched file, ctest 57/57 in 243 s; no game runs, no shadow): completes NG5 on top of cores18's rule loader and solver (`engine/core/src/network_rules/`). **Added** `network-rules.md` §6.5 `resolve_one_way_conflicts` (test 26: the seeds from step 0 and the last dragged step, the portal side of a tunnel, the turn rule `to = from ^ 1`, and the conflict sweep with quirk 11), §8.3 `solve_multi_type_separation` (test 29), §10.3/§10.9 the tile grid and map, `get_tile_info` and `intersection_pieces` (test 28: `AutoTileBase` / `AutoPathBase` + tile, one anchor per intersection; the grids turn with `Transpose` / `Rotate` and the map entries' orientations with them), §10.6 `mark_highway_intersection_pieces` and `remove_highway_intersection_pieces_from_solve_set`, and the bridge counterpart of `network-bridges.md` §8.3.1 (`remove_bridge_pieces_from_solve_set`). **Fixed** §10.8 `read_ordering` (test 27, NR3): it implemented the disproven premise (one key per section) and put the `AddTypes` links in the wrong ring; now every line of an `Ordering` section is handled on its own, both keys may share a section, an `AddTypes` line forms a rotation ring of its own and enters the type rings of the previous line's rules. **Pinned** §9.1 `RuleOverrides` order (test 25); the loader already kept file order. 45 `network_rules` unit tests (`t15_01`-`t15_29` and 16 further checks) and 29 `nr15_*` vectors, all of §15 tests 1-29; 6 new conformance ops (`nr_one_way`, `nr_separation`, `nr_tile_info`, `nr_pieces`, `nr_mark_pieces`, `nr_remove_pieces`). Questions NR8-NR13 | (1) a glue lane takes the C surface: `fc4_network_rules_one_way`, `_separation`, `_int_tile` and `_int_pieces`, and two fields `fc4_network_rules_cell` does not carry yet (`+0x57` and the occupant's paired portal) — see questions NR12; (2) `InsertIsolatedHighwayIntersection` (§10.7) is the geometry layer's pipeline and is not here; wire it over `intersection_pieces` once cores17's geometry lands; (3) NR7 (the constraint layout's codes) is still open and belongs with the height solver |

## Implemented versus stubbed

Core-only, as cores18 was: no plugin glue, so no `enable =` key, no shadow and no in-game run. The rule
stages were already reachable from C through `fc4_network_rules_load` / `_evaluate` / `_solve`; what this
lane added is not (question NR12).

**Implemented, with a conformance vector each:** §15 tests 25-29. Tests 1-24 were already cores18's.

**Implemented, no numbered test in the spec:** the §10.6 pair `MarkHighwayIntersectionPieces` /
`RemoveHighwayIntersectionPiecesFromSetOfCellsToBeSolved`, and `RemoveBridgePiecesFromSetOfCellsToBeSolved`
(`network-bridges.md` §8.3.1). All three have a unit test
(`highway_and_bridge_pieces_leave_the_solve_set`).

**Stubbed or out of this lane:** `InsertIsolatedHighwayIntersection` (§10.7) — the intersection tool's
pipeline (demolition region, reserved tiles, cell infos, `DoAdditionalDraws`, the smoothing levels and the
commit), which belongs to the network geometry core; `InsertHighwayIntersectionPiece`'s occupant creation
(§10.9 steps 1-10) — `IntersectionPiece` derives exactly what it would place and leaves the occupant to the
geometry layer; `QueryUserForPotentialIntersections` (§10.7) — UI; the constraint layout's consumer
`InsertIntersectionConstraints` (NR7) — height geometry; the path resource and the path-info transforms
(NR6).

## Questions for the RE side

1. **NR8.** §6.5 step 2.4 says only "Skip `s` when `m` is outside the city, has no cell info, or is part of a
   highway intersection", but §15 test 26's X (one-way `0x15`, with no cell info at (11, 10)) comes out `0x09`,
   which needs the east code dropped. Implemented as: **a side the propagation skips also loses its flow
   code** (`[choice]`, `solve.rs`, `resolve_one_way_conflicts`). The other four values of test 26 follow the
   prose as written. Does the original clear the code of a skipped side, and for all three skip reasons or
   only the missing-neighbour ones?
2. **NR9.** §10.3's tile grid keeps its own width `+0xC4`, and `GetTileInfo` compares `col` against that
   width while quirk 8 makes `maxX` wrong after an odd `Rotate`. Implemented as: the grid is turned as a
   matrix and its width follows, so `GetTileInfo` never reads the bounds for the width; and a map entry's
   `tile` value (`col * 0x100 + row * 0x10`) is fixed at parse time, so a transform changes only the
   orientation and never which piece a cell gets. Confirm both, and that `Transpose` / `Rotate` do not
   rebuild the code bytes (quirk 13's shared entries stay shared).
3. **NR10.** §10.6 names the records `MarkHighwayIntersectionPieces` and
   `RemoveHighwayIntersectionPiecesFromSetOfCellsToBeSolved` walk (`+0xCC` entries whose start step is below
   `+0x78`) but not the cells. Implemented as the rule's occupied-cell list — the same list `MarkIntersection`
   marks (`[inferred]`). Also: `network-bridges.md` §8.3.1 stops at the first record whose **end** step is at
   or above `+0x78`, while §10.6 says "whose **start** step is below `+0x78`"; implemented both ways round
   as their own specs read. Confirm each.
4. **NR11.** §10.8 links `R` after `P` with the head `F` of the **current line's** known rules. When an
   `AddTypes` line re-uses rules an earlier `RotationRing` line already linked, the earlier ring's remaining
   members keep stale links: `RotationRing = 1,2,3` then `AddTypes = 2,3` leaves rule 1's links pointing into
   the closed 2↔3 ring. Is that the original's behaviour, or does the ring stay whole?
5. **NR12.** `fc4_network_rules_cell` carries neither `+0x57` (the one-way pass's re-solve mark) nor the
   occupant's paired portal (interface `0x8A4BD52B` slot 59), so a cell that crosses the C boundary loses
   both; and `resolve_one_way_conflicts`, `solve_multi_type_separation`, `get_tile_info` and
   `intersection_pieces` have no C entry point. Left out of this lane on purpose (cores17 is landing
   `fc4_netgeom` in the same batch and the cell struct is a shared ABI type). Should the cell struct grow
   those fields, or should a glue lane pass them another way?
6. **NR13.** `network-bridges.md` §7.1 (`IdentifyLandBridgeCells`'s preconditions) and §8.2
   (`SelectBridgeElevation`) meet the rule solver only through §8.3.1's
   `RemoveBridgePiecesFromSetOfCellsToBeSolved` and through kind-1 steps being fixed. Is there another
   coupling — for example, does a bridge cell keep an edge word the rule stages would otherwise see?
