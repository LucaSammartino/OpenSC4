| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-view3dwin-vw5: VW6–VW18 owner contracts | `Codex` / `re-view3dwin-vw5` (re lane) | ready; VW6–VW18 answered; index 0 open / 18 answered; doccheck passed; no ABI changes or game runs | 1. Land the queued tip. 2. core-view3dwin5 can consume the closed contracts and vectors. |

## Report

Closed all 13 requested VW6–VW18 questions in [view3d-window.md](../specs/view3d-window.md), with dated
MATCHES/DIFFERS verdicts preserving the questions in
[answered/view3d-window.md](../specs/questions/answered/view3d-window.md). Regenerated the questions index:
**0 open, 18 answered**, next ID VW19. No new questions filed.

The specification now covers the complete ordered 104-record command registry; dormant fields and per-panel
extras; erosion messages; allocation/QI failure boundaries and parent-owned overlay lifetime; save-trigger
overflow, collision/bucket/rehash order and the bundled decimal parser; remaining menu identities and
caller data; paint timing/tail behaviour, status ordering/fading, shutdown and the empty command callback.
VW-T19–VW-T39 provide 21 additional conformance vectors across this lane's two rounds. These are specified
vectors for implementers, not runtime test results.

Round 2 closes the remaining opaque contracts in §6l: mission argument 1 is the message's borrowed
IGZUnknown attachment, while data1 remains the activation flag. The old handoff's indexed-getter assumption
was incorrect: message slot 32 is zero-argument GetIGZUnknown. Advice links use the window-message type and
data1, with exact panel/graph/budget dispatch and guards. Renderer/model-maker/accelerator getters borrow;
the terrain debug-string slot is empty. Background mode latches 2D/3D selection and 3D filenames append the
camera zoom byte. Coverage recalculates whole service lists without explicit arguments and retains the
original's stale-radius/null-effect cache quirks. The two old coverage “effect IDs” actually request the
temporary-list allocator; effects are created by name. Auxiliary lifetime and graph setup calls are stated.

Also corrected [ui-windows.md](../specs/ui-windows.md) slot 38 to the empty-method contract and the earlier
[view3d.md](../specs/view3d.md) paint-tail boundary. Removed the stale open-question summary.

Evidence: main checkout's read-only `tools/re_query.py`, exported instructions and `decomp/`; numeric
coverage factors, transform scale and dispatch widths checked against instructions with byte output.
No REA calls were made: this round was explicitly told its REA session has no binary. Earlier q.py Bash
startup failed with CreateFileMapping error 5; bounded native PowerShell queries completed the work.
No engine/, plugin, capi, docs/abi/, coverage-map, HANDOFF or README edits. The requested owner closure
README is inside forbidden engine/; its filed VW questions supplied the closure requirements instead.

Validation: `python tools/questions.py index` completed; `python tools/doccheck.py check` **passed**;
`git diff --check` **passed** after removing blank-only question-file residue. Cargo/clippy/ctest and game
runs: n/a for this specification-only change. No shadow measurements, C ABI exports or proposed ABI rows.
Git reported inability to lock packed-refs during commits, but each detached-HEAD commit itself succeeded;
no shared refs/config were changed.

Milestones: `928ecdf9` paint/status/shutdown closures; `bb0fbeda` registry and GUI failure/lifetime contract;
`9390fc49` parser/menu continuation; `5fb18000` moved VW6/7/8/9/12/14/17; `f8d9b747` VW18 contracts and
VW-T33–VW-T39. This final commit adds save collision/growth detail, clean question stub and this report.

## Handoff

- No remaining RE question work for this lane. Coordinator publishes the actual detached HEAD to the reserved branch.
- Queue: `build-runs/merge-queue/re-view3dwin-vw5.json`, kind re, ready, full actual HEAD.
- Engine follow-up: implement the closed contracts and exercise the prose conformance vectors; no game verdict is claimed here.
