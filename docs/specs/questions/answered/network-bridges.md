# network-bridges: answered questions (archive)
<!-- Archive for [network-bridges.md](../../network-bridges.md). Lanes do not read this file. Open questions are in ../network-bridges.md. -->

<!-- from: From the network bridges spec (2026-10-16) -->
- ~~**NB1 (bridges):** how bridge cells and their corner heights constrain the network height solver
  (`RemoveBridgePiecesFromSetOfCellsToBeSolved` `0x00638B40`, `CopyBridgeElevationConstraintsIntoCellInfo`). [not
  read]~~ **ANSWERED 2026-10-16 (re-answers26):** network-bridges.md §8.3 and network-geometry.md §9.8 (new). The bridge
  cells are erased from the piece solve set `+0x84` (last step of `BuildSetOfCellsToBeSolved`), are fixed (every cell
  of a kind ≠ 0 step, new or not; network-geometry.md §8.1 corrected), and at every `SmoothenNetwork` level, in its step
  2, get their pieces' four corner heights as corner heights and as a collapsed allowed range (`0x0062FB90`); slot 76
  then pins them in the satisfier. Each bridge also appends `{start, end, r24(deck − +0x60[0]), same}` to a constraint
  list that the tunnels extend (`0x006274B0`, which pins the portal cells), and slot 77 `ApplyPlaneConstraintToRegularCells`
  (`0x00637B90`) uses it to clamp every regular corner to a window of ± p+0x20 (`0xC804DD89`: road 35 m, others
  15 m) around the straight plane from one structure to the next, or to the run's ends at the terrain, widened to
  include the corner's current height, and replaces its preferred range by [min(h, k), max(h, k)]. Every surface run
  gets this plane, bridge or not. Side corrections: `ComputeInitialSmoothingSet` is `0x00636AB0` (`0x00637720` is a
  vector resize); tool `+0x2B0` = p+0x20.

<!-- from: From the network bridges spec (2026-10-16) -->
- ~~**NB3 (bridges):** the occupant position height used as the column top in `InsertBridgePiece` (network-bridges.md
  §11.3). [the piece's model height Y *[inferred]*]~~ **ANSWERED 2026-10-16 (re-answers26):** **DIFFERS**
  (network-bridges.md §11.2 step 2, §11.3 step 2). The position is the network occupant's (`0x006193D0`, reached
  through `GetPosition` `0x00604E40` → primary slot 75): x, z = the mean of its four corner points, height = the
  **highest** of the four corner heights, which `CreateNetworkOccupant` took from the cell info, i.e. the piece's
  corners (§8.3). The exemplar's model offset `dy = Y − height` puts the model's origin at `Y`, but the column top is
  `highest corner − depth`. They differ by `+0xD4` (deck), `+0xD4 + +0xC4` (pier), `+0xD4 − b_u` (ramp u) or
  `P.Y + +0xD4 − max(h[k], h[k+1])` (specialised); 0 for the shipped road parametric pieces.
