| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-overlay-views4: pure Rust overlay owners | `core-overlay-views4` / engine lane, detached worktree | WIP: documented subsets, 27 core tests and 9 vectors; cargo 4311 PASS, clippy 0 diagnostics, ctest 67/67; OVV2–OVV7 and NOV2–NOV3 open; ABI unchanged; no game run or owner/runtime promotion | 1. Answer exact public contract questions. 2. Reconcile full 54-function catalogue and implement missing bodies. 3. Re-gate before promotion. |

## Implemented scope

Base was detached HEAD `0168a928`. Substantive implementation is `05ceb9f4`;
conformance and invariant improvements are `e27eb6bb`. Work stayed in Rust core,
its conformance runner/vectors, this report and the two permitted question files.
No CAPI, plugin, ABI, maps, generated ABI entries or terrain-manager code changed.

The exact owners represented in `engine/core/src/overlay_views` are cSC4PathView,
cSC4PathView::PathInfo, SC4MarkedCellView, cSTEVertBufExtn, SC4PathInfoView and
SC4TrainJunctionView. `METHOD_CHECKLIST.md` records each published method and gap.
The brief's 54-function count has no corresponding complete public catalogue;
this work does not claim 54 completed functions or any complete owner.

- Path state: stable ascending priorities, equal-priority stability, independent newest-path identity,
  x-z duplicate suppression, captured texture ownership, clear, semantic render states, explicit-input
  ribbon geometry/normals/bisectors/UVs, ordered draws, identity transform, offset 15/restoration,
  wrapping and plain-state restoration on draw failure. Insertion keys are read-only afterward.
- Marked cells: size/inverse and flags, colour channels, bounds and camera tests, terrain splits,
  non-square clamping/nearest vertices, water and literal lot flags 0x304/0x100000, lot height lift,
  1024-vertex indexed batches, normal/hidden additive passes, identity and offset 4/restoration.
- Vertex-buffer extension: boolean-25 gate, literal interface 0x09CD86F9 and format 11, capacity
  rejection/release, raw 32-byte row copying, foreign terrain index source, compact-coordinate rebasing,
  capacity flush/relock, unlock-before-draw, explicit failure paths and acquired-buffer release.
- Network path-info: pulse wrapping, owned cell/occupant/path traversal, opaque-key byte filter,
  known primary colours, 25-point strip cap, height lookup, provisional cross/arrow/bar geometry.
  Unpublished native format and render fields remain unknown rather than acquiring guessed values.
- Train-junction: literal picture/model resource acquisition with rollback, unit midpoint direction
  table, explicit-entry chord classifier/instance IDs, and 0.8/0.5 transparency. Init, quad construction,
  junction drawing and Update explicitly fail pending NOV3; resource loading does not stand in for Init.

Providers expose foreign terrain/camera/resource/draw/device primitives only.
No provider supplies an overlay-owner ribbon, marked quad, train transform or other
missing owner algorithm output. Owned Arc/Box references use RAII; borrowed providers
are never released. All six state types satisfy Send + Sync without RefCell getters.

## Verification

The final, stable `qbuild all` ran on `e27eb6bb` with no edits or commits while it ran:

- Workspace cargo: **4311 passed, 0 failed, 0 ignored** (`build-runs/qbuild-test.log`).
- Clippy with warnings denied: **exit 0, 0 diagnostics** (`build-runs/qbuild-clippy.log`).
- CMake: **2/2 edges PASS**; full ctest: **67/67 PASS** (`build-runs/fc4verify-build.log`, `fc4verify-ctest.log`).
- Latest focused quick: **3634 PASS**, including **2435 core** and **1196 vector** tests plus runner/turbo-off;
  dependent crates compiled and CMake passed. One intermediate vector failed due to a decimal expected
  arrow-ID typo; corrected before the passing quick and final gate.
- **27 new core tests** independently check asymmetric positions, heights, UVs, normal fallback,
  priority/texture/state order, both terrain splits, batching, opaque flags, bounds, empty/invalid inputs,
  draw/lock failures, exact resource IDs, reference rollback, compact row rebasing, thresholds and thread traits.
- **9 new JSON vectors** cover every prescribed test in overlay-views §7 and network-overlay-views §6;
  the standard runner and its turbo-off replay pass them.
- `qbuild fmt` passes all **7 new Rust files**. Checking all **10 touched Rust files** also reports
  unrelated sorting/formatting in existing shared files; additive registrations were preserved under
  the no-mass-reformat rule. The complete touched-file formatting check is therefore not green.
- No game request, runtime binding, shadow, replacement or reference-audit verdict is claimed.

## Questions and promotion limits

Appended **OVV2–OVV7** to `docs/specs/questions/overlay-views.md` and **NOV2–NOV3**
to `docs/specs/questions/network-overlay-views.md`. Existing OVV1/NOV1 remain intact.

These ask for exact path defaults/attributes and full method catalogue, ribbon/arrow
geometry, packed colour conversion, cell/filter/height edge rules, vertex-buffer
lifecycle/index layout, network numeric colours/geometry/state, and train draw state,
quad/marker transforms and lifecycle. Provisional literal readings are identified in
code and the questions. Missing bodies return named contract errors, never placeholder
success. All six owners are withheld from complete-owner and runtime promotion.

Terrain-view §8 belongs to the terrain manager and supplied no additional declared-owner
methods. The whole terrain manager, parked terrain replacement and TR33–TR37 contracts
were left untouched. There is no ABI change.

## Next steps

1. Obtain public-spec answers to OVV2–OVV7 and NOV2–NOV3, including the exact 54-function checklist.
2. Implement the blocked bodies and reconcile every provisional reading before owner promotion.
3. Re-run focused checks and a stable full gate; runtime validation belongs to a separately authorized binding lane.
