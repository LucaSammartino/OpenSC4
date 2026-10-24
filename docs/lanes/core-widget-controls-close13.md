| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-widget-controls-close13 (partial settled widget owners) | `Codex` / `core-widget-controls-close13` (engine lane, detached HEAD) | WIP: implementation 915505f6a and 1dac37531; focused 26/26, scoped clippy clean, fmt 8 files clean; native 264/264, CTest 81/81; workspace all interrupted without verdict; ABI unchanged; no game runs or new questions | (1) finish remaining owner obligations below; (2) complete workspace all; (3) review closure before READY |

## Report

This is a partial implementation checkpoint, not completed owner closure. The
two-thirds context cutoff was reached before the remaining obligations could
be implemented. The merge-queue entry is WIP because the required workspace
gate has no completed verdict.

The requested first merge of `re-widget-controls-wc7` reported Already up to
date, and its ancestor check succeeded at initial HEAD
`193b315e2e5fd9e07d121229b3ab82c7cbc59e8b`. Git reported a denied shared
packed-refs lock while merging/committing, but detached HEAD commits succeeded;
no shared refs/configuration were changed.

Implemented module paths (the README implemented column names these paths):

- `engine/core/src/widget_controls/list_box.rs`: settled next-page arithmetic,
  removal selection repair, persistent drop sizing, sorting enable/sort-now,
  exact byte search/character navigation, malformed public removal count,
  intensity, color-prefix paint, fitting and bar tracking.
- `engine/core/src/widget_controls/tree_view.rs` and `tree_view/settled.rs`:
  insertion ordering/failure state, property setters, distinct descriptors,
  move/removal, retained images, one-shot resource initialization and flattened
  root membership. Native unsafe domains have explicit Precondition results.
- `engine/core/src/widget_controls/tree_view/input.rs`: cached page navigation,
  host-shared click state, mouse selection/notifications, collapse selection
  repair, drag negotiation/target/drop, borrowed helper state.
- `engine/core/src/widget_controls/tree_view/rendering.rs`: viewport/page/hit
  formulas, content measurement, guarded fills, flat insertion-side colors,
  connectors/expanders and text drawing.
- `engine/core/src/widget_controls/settled_tests.rs`: 12 grouped new scenarios
  alongside 14 existing ListBox/TreeView tests. These use the new vectors as
  their source; they do not yet assert every literal one of the 57 new vectors.
- `engine/core/src/widget_controls/mod.rs`: additional primitive foreign-service
  and drawing operations. No algorithm is wired into plugin/capi.

### Measured checks

Final-source isolated harness compiles the actual owner files and extracts the
existing fixture and owner tests: **26 passed, 0 failed**, 0.01s. Final-source
clippy-driver `--test -D clippy::all` is clean (harness-only dead-code/unused-import
warnings suppressed). Evidence: `build-runs/owner-tests-final-result.log` and
`owner-tests-final-clippy.log`. These are focused checks, not workspace cargo or
workspace clippy. qbuild fmt: **8 explicit owned files, exit 0, diff_lines 0**,
`build-runs/fmt-final-absolute.log` (the initial relative-path invocation failed
to locate files; its evidence is retained).

Queued qbuild verify **PASS**, terminal exit 0, request
`qbuild-20261024-175346-build-ctest`: build **264/264 edges**, 232.7s; CTest
**81/81**, 412.5s; total 650.5s. Logs: `build-runs/verify-final.log`,
`fc4verify-build.log`, `fc4verify-ctest.log`. It reports `[the tree changed during
the run]`: source corrections preceded the request, while documentation and
commit state changed during the run. No game/shadow comparison was performed.

Workspace gate: **incomplete, no PASS claimed**. Focused cargo operation 81749
and first all operation 49012 were interrupted to terminal exit 1; the first
all preceded two scoped clippy source corrections. Corrected all operation
11550 was then run on final source and collected after interruption to terminal
exit 1 at wind-down. Its log reached `Compiling fc4-core`, without a test result
or workspace clippy verdict. Evidence remains distinct in
`build-runs/focused-list.log`, `all-final.log`, `all-corrected.log` and
`qbuild-test.log`. No lane tool operation remains outstanding.

### Remaining owner obligations

The README closure table records implementation work rather than reopening
answered WC questions. Complete these settled obligations before claiming
owner closure:

1. ListBox sections 5.1/5.6: finish residual ABI adapters and malformed storage
   primitive accounting; section 5.2: audit exact destruction ordering;
   section 5.4: complete bounded bar hit/up dispatch. Audit alternate paint
   color policy and every literal section 7 vector.
2. TreeView sections 6.1/6.6: finish ABI enumeration/raw negative property-word
   alias behavior. Sections 6.2/6.5: complete automatic buffer/bar recalculation,
   exact image initialization/centering and ancestor connector geometry.
3. TreeView section 6.7: complete helper integration and exact packed drag
   rendering; audit all input/drawing side effects and each literal vector,
   especially WC-T17/T18/T24/T47/T50/T51 rather than only related scenarios.
4. Finish the full workspace cargo/clippy gate, then review native evidence and
   closure. Existing Grid mapping and separate TextEdit/FileBrowser question
   boundaries were retained; their internals were not modified in this round.

No new questions were filed: WC1-WC6 are answered and remaining gaps above are
implementation work. No ABI exports/version changes, plugin/capi wiring,
legacy_control edits, text-edit internal edits or game requests.
