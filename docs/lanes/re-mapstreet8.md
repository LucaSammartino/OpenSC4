| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-mapstreet8: the map colour-buffer fill, the MyStreetSim picking control and the coverage row of the already specified grid interpolation, map-color-buffer.md and my-street-sim-control.md (RE spec lane) | `re-batch9` branch (outside contributor checkout, fork amarret/OpenSC4; on main `626043df`, after re-scriptdir8) | **ready** (docs only). Three owners claimed: specified +3,354 bytes (+11 functions) | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
Every function was read in the disassembly; the rotation jump table and the box constants were read in the PE.
Written on 2026-10-25.

### What changed

- docs/specs/map-color-buffer.md (new): `MakeMapColorBuffer` (both instantiations): the power-of-two block size, the
  placement per rotation, the palette lookup (signed grids offset by 128) and the block fill.
- docs/specs/my-street-sim-control.md (new): the MyStreetSim control's state, the hover pick (occupant pick with
  automaton groups, then a box around the terrain point), the cursor, and the click that attaches the street sim.
- `fn:SC4InterpolateGrid`: its two bodies are grid-interpolation.md's owned contract; only the coverage row was
  missing.

### Coverage

- tools/coverage-map.tsv: rows for `fn:SC4InterpolateGrid` and `fn:MakeMapColorBuffer` before `fn:*`; the
  `cSC4MyStreetSimInputControl` row filled.
- Specified bytes 5,115,624 → 5,118,978 (+3,354, 91.7%, `cov_local.py` with owners-c1).
