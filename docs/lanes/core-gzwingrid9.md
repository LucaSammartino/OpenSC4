| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-gzwingrid9 (cGZWinGrid core owner closure) | Codex / core-gzwingrid9 | ready; ABI unchanged; Rust 6993/0, clippy clean, CTest 79/79 | Coordinator review; resolve WG5–WG9; geometry/input shadow feasible next |

## Implementation

The implemented column is `engine/core/src/widget_controls/grid.rs`, extended by
`grid/data.rs`, `grid/runtime.rs` and `grid/paint.rs`. The existing sparse cell,
row/column and ownership model now has live mutation hooks, recursive layout,
scroll-bar creation/removal/position/ranges/images, hit testing, mouse/modifier
selection, keyboard/wheel/focus routing, editable cell windows and painting.
`GridHost`/`PaintHost` extend the existing widget host interfaces; drawing emits
ordered primitives, clipping/capability operations and owner-draw callbacks.
No capi, plugin, ABI exports or coverage-map changes were made.

Preserved quirks include the horizontal bar's row-limit test, page scrolling by
four cells, negative scroll positions, two-level shared layout recursion, delayed
cell gutters, independent handle-keyed child-window ordering and unchanged window
set coordinates after a cell move. Default color setters and whole-default-format
replacement now update the same logical storage. Code milestones: `c780222cc`
(runtime/painting), `129c929f3` (owner quirks), `b06a63a4f` (color alias regression).

The section-by-section **Owner closure** table is in
`engine/core/src/widget_controls/grid/README.md`. All six win-grid conformance
vectors remain covered; additional owner tests cover input, layout, resources,
painting, window movement and default-format/color aliasing.

## Measured results

- Final six-file `qbuild fmt`: PASS, zero diff lines.
- Focused intermediate snapshots: 50/50 and 53/53 widget tests passed.
- Corrected stable-source `qbuild all`: PASS, **6993 passed, 0 failed, 0 ignored**;
  clippy **0 diagnostics**; native build **47/47** edges (74.5 s); full CTest
  **79/79** (151.2 s), native total 227.6 s.
- Explicit `qbuild verify`: PASS, native build **46/46** (4.0 s), CTest **79/79**
  (104.5 s), total 110.6 s. Request `qbuild-20261024-094413-build-ctest` completed.
  Its tree-change note was the handoff-only commit while waiting; Rust source
  stayed unchanged. All asynchronous operations reached terminal PASS.
- Logs: `build-runs/qbuild-{test,clippy}.log`,
  `build-runs/final-all-fc4verify-{build,ctest}.log`, and final
  `build-runs/fc4verify-{build,ctest}.log`. The earlier full gate (6992 tests,
  253/253 build, 79/79 CTest) is preserved separately as
  `build-runs/before-color-fix-*.log`; the repeat followed the actual color-storage
  diagnostic and changed source/test inputs.

## Questions and remaining spec contracts

Appended **WG7–WG9** to `docs/specs/questions/win-grid.md`. Existing WG3/WG4
quirks are preserved. The generated question index was not edited or regenerated
with prohibited `tools/` scripts; the coordinator can regenerate it.

Spec-only open sections (no full parity claim for these contracts):

- **3 and 5.7:** pending owner-slot bodies and comparator locale/encoding (WG8).
  Named pending operations explicitly return Unsupported.
- **5.1:** default-format initialization (WG6); Init still returns true.
- **5.3, 5.8, 5.9, 5.10, 5.11:** bounds, relative-scroll body, selection predicate
  and effective format inheritance remain WG5. Existing literal choices are
  documented; painting takes resolved formats explicitly.
- **5.11:** failed editable-window factory result (WG7), reported Unsupported.
- **5.12:** inherited formats (WG5), horizontal text alignments 3/4/5 (WG6),
  header rectangles/content-right paint field (WG9). Unknown text positioning
  returns Unsupported; literal header/clip choices are marked in code.
- **5.14:** axis-format size-word copy remains Unsupported (WG6).

## Game and next steps

No game run was requested: this lane was explicitly core-only. No shadow result
or reference-warning count is claimed. An in-game **geometry/selection/input
shadow is feasible next** through a coordinator-owned probe and renderer/host
adapter. Full drawing comparison needs WG5/WG6/WG9 resolved. The coordinator
should review the owner table, claim coverage rows, and publish these detached
HEAD commits to the reserved branch before landing.
