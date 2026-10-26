| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-dispatchview8: the dispatch vehicle view (whole owner, DP10 closed) and the dispatch tool, dispatch-vehicle-view.md (RE spec lane) | `re-batch9` branch (outside contributor checkout, fork amarret/OpenSC4; on main `626043df`) | **ready** (docs only). Three owners claimed: specified +19,491 bytes (+60 functions); DP10 answered | none |

## Report

Static RE only: the instruction and data bytes of the Steam 1.1.641 exe (same code as GOG, `docs/lanes/re-dates.md`).
All 60 functions were read in the decompile; the x87 sequences of the parked DP10 residual were read instruction by
instruction in the disassembly, and the constants in the PE. Written on 2026-10-25.

### What changed

- New docs/specs/dispatch-vehicle-view.md: the view's data model and flag bits, tracking kinds 0–6 (kind 6: the
  animated 3D model), the small entry points (limits, lock, flash, selection, hidden kinds, per-occupant priority,
  window obstacles), the kinds 3/4 images, the camera rotation of the quads, the complete Update walk (flash, the
  building ring, the bounds clamp, the world-space two-pass path, the final rotated 2D quads), the 3D-model
  indicator, and the dispatch tool (`cSC4ViewInputControlDispatch`: init/shutdown, cursor text, messages, picking,
  hover, drag, drop and dispatch).
- docs/specs/dispatch.md §4.1.3: a closing paragraph after the DP10 residual pointing to the new spec.
- DP10 answered: moved to docs/specs/questions/answered/dispatch.md; the open file's header updated;
  QUESTIONS.md regenerated.

### Coverage

- tools/coverage-map.tsv: `cSC4DispatchVehicleView` row filled (dispatch.md + dispatch-vehicle-view.md); new rows
  `cSC4DispatchVehicleView::TrackInfo` and `cSC4ViewInputControlDispatch` (above `cSC4ViewInputControl*`).
- Specified bytes 4,982,240 → 5,001,731 (+19,491, 89.2% → 89.6%, `cov_local.py` with owners-c1).

### Why now

These owners were skipped by earlier batches as "active upstream" (DP10 parked by re-dispatch-completion4 with no
fourth round). Upstream main `626043df` has STOP set and no lane on them.
