| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-view-utilities4: SC4ViewUtilities owner | Codex / `core-view-utilities4` (engine lane, detached HEAD) | WIP: 38 described contracts implemented; 49-function/19670-byte owner incomplete; gate on 888e1a3f cargo 4258/0, clippy 0 diagnostics, ctest 67/67; VU4–VU11; ABI unchanged; no game runs | 1. Resolve missing inventory/rotation/lifecycle contracts. 2. Reconcile numeric/dependency choices. 3. Complete owner before claiming coverage. |

## Result and completion limit

Implemented the source's 38 described contracts (constructor plus 37 named methods)
in `engine/core/src/view_utilities`, registered additively at the end of core
`lib.rs`. **The requested complete owner is not finished. Do not credit 49
functions or 19670 implemented bytes, shadow verification, or replacement.**

The spec contains no complete 49-function inventory or missing lifecycle/setter
bodies. It also references `RotateVectorByViewRotation` without supplying its
algorithm in this task's permitted behavior source. Those bodies were not guessed.
The full mapping and explicit omissions are in
`engine/core/src/view_utilities/README.md` (38 rows). At least 11 attributed entries
cannot be reconciled with those rows; rotation may be one, pending ownership facts.

## Changes

- Owned zero-initialized state and constructor constants, raw camera/control
  snapshots, camera update invalidation, angle tables, viewport scales and basis.
- View/draw corners, boxes, planes, ground lines, city polygon clipping and strict
  four-side box tests with the specified near/far omission. Perspective follows
  the spec's explicit corrected-current-near-corners choice.
- World/view/fixed/viewport projection, inverse transforms, 2D/depth projection,
  rotation-specific pixel bounds, city corner/edge helpers, clipping, truncation
  before block rounding, raw-camera eye-ray normalization and ground intersections.
- Dirty-rectangle draw frusta and thread-safe `OnceLock` caches for full viewport
  ground corners and visible rectangles. Both caches clear on update/draw, taking
  the literal cache-clearing sentence; visible area uses viewport points (VU9).
- Owned mesh vertices/u16 indices, stride/count input decoding, all eight primitive
  walks and every transform kind, first-closer triangle stopping, and model walks
  that reset the transform per mesh and continue looking across meshes.
- Malformed resource iteration is bounded by available elements; invalid vertex
  references cannot read outside buffers. No game pointers or reference counts.

External providers supply only the camera's eye ray or an instance's raw transform.
All SC4ViewUtilities geometry/state/picking algorithms are local Rust. Input setters
and inspection getters are Rust adapters, not invented native attributed bodies.
No plugin/CAPI/ABI/tool changes; no generated `abi_entries.inc` edits.

## Verification

| Check | Result |
|---|---|
| Initial `qbuild all` baseline | workspace tests/clippy PASS, ctest 67/67 (119.46s) |
| Iterative `qbuild quick` | PASS, core 2388/0 at that stage; dependents compiled; CMake 4/4 |
| Final `qbuild all`, unchanged committed code `888e1a3f` | PASS, Rust 4258/0; clippy exit 0 / 0 diagnostics; CMake 2/2; ctest 67/67 (77.29s) |
| New contract tests | 33 PASS; core total 2391/0 |
| `qbuild fmt`, five new Rust files, absolute paths | PASS |
| Shared `lib.rs` formatting | Existing module-order debt; baseline and current both fail; only lane change is additive registration |
| `git diff --check` and final code status | PASS, clean before report |
| Real game / shadow | n/a: pure core, no ABI/plugin registration; no requests outstanding |

Tests include independent angle/coordinate/depth/corner/plane/city/rounding/pick
numbers, strict visibility boundaries and missing near/far tests, polygon rejection
despite AABB overlap, supplementary transform roundtrips, exhaustive box extrema,
all primitive/transform kinds, first-closer mesh versus across-mesh traversal,
failed camera inputs, parallel/behind ground rays, malformed resources, snapshot
ownership, draw cache invalidation, and concurrent shared getters.

The initial perspective analytic vector used an overly tight absolute tolerance;
it was corrected to account for near-corner f32 cancellation amplified by far/near.
No original game outputs were fitted or claimed as exact.

Full logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
`fc4verify-build.log`, `fc4verify-ctest.log`. Shared formatting provenance is
`build-runs/check-view-format-baseline.py` with `fmt-baseline-lib.log` and
`fmt-current-lib.log`. The q.py Bash wrapper is unavailable (E_ACCESSDENIED);
formatting new files directly and checking through qbuild worked. All builds/tests
used qbuild. No mass formatting was applied.

## Questions and remaining work

Appended **VU4–VU11** to `docs/specs/questions/view-utilities.md`:

- VU4: exhaustive 49-function ownership, missing lifecycle/setter contracts.
- VU5: exact table bits, float/trig/evaluation/normalization and invalid camera data.
- VU6: triangle/plane/convex-clipping dependency edge and numeric contracts.
- VU7: failed camera-ray temporary/output behavior.
- VU8: tie rounding, conversion/overflow and integer-centre conventions.
- VU9: identity/invalidation of the two caches and viewport versus draw points.
- VU10: malformed resource bounds and transform-matrix orientation.
- VU11: missing `RotateVectorByViewRotation` body and its owner attribution.

Literal choices are marked in code. VU2/VU3 remain inherited open questions. No
spec index regeneration was run because this lane cannot edit that generated file
or execute scripts under tools/. A spec-author/coordinator update must reconcile
the inventory and contracts before owner completion can proceed. Then implement
the missing functions, update the mapped inventory and independent vectors, and
repeat the full gate. No whole-owner publication or coverage promotion is ready.

## Commits

- `935a8ff2`: owned implementation and initial contract tests/questions.
- `37d82b6b`: method inventory, literal cache refinement, stronger vectors and
  bounded resource traversal.
- `888e1a3f`: warning-free snapshot ownership regression; final code gate target.

Supervisor owns branch publication from this detached worktree. No branch switch,
merge, push, new agent, workflow, restart, or game launch occurred.
