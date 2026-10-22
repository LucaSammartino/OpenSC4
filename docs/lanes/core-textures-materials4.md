| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-textures-materials4: portable textures and materials | Codex / `core-textures-materials4` (engine lane, detached HEAD) | ready for review; cargo 4321/0, clippy 0 diagnostics, ctest 67/67; 32 new tests; TB4-TB7; ABI unchanged; game/shadow n/a | 1. Publish TB4-TB7 contracts and regenerate question index. 2. Reconcile remaining owner bodies before complete-owner promotion. |

## Result

Implemented the published portable texture/material behavior in `engine/core/src/textures_materials/`.
The exact named inventory was audited against texture-bindings.md sections 1-3/7, the referenced
graphics-formats.md sections 2.10/3.8, public SDK headers, and public ABI tables. No native analysis,
CAPI/plugin/ABI/map edits, or runtime promotion. This is ready implementation work, not a claim that
all 58 requested bodies or all four owners have complete published contracts.

## Changes

- Factory owns power-of-two/LOD sizing, internal-format selection, unclamped mip byte accounting,
  per-frame creation and error cleanup, last-n-level upload order, (group, instance) cache membership,
  caller references, oldest-matching reuse, FIFO eviction, counters, settings and statistics.
- Lifecycle owns ordered service acquisition/release, optional restore subscription, shutdown before
  reinitialization, driver identity changes, flush including live callers, and keyed restoration.
  Driver/resource/image primitives and inherited system-service operations are explicit providers;
  no provider decides binding layout, cache policy, upload selection or material output.
- Binding reference transitions retain the factory's one reference, pool at count one, and destroy at
  zero. Opaque non-recycled handles make the published FlushAll dangling-pointer quirk safe in Rust.
  Getters borrow immutably; no RefCell or unsynchronized interior mutation.
- Material construction copies the published flags/functions/blends/threshold-high-byte and used
  texture bytes, preserves constructor defaults, and marks slots beyond T unused. Unknown material
  lifetime operations are deliberately absent. Manager variant creation returns false and its stats
  return the published string.
- Existing S3D records implement the core material-record view. The existing reader retains version
  defaults, validation and the last-ID-zero count quirk; flag normalization has one core primitive
  used by both reader and owner. No second format parser, texture decoder or draw-context algorithm.

## Covered / missing checklist

- **cS3DTextureBindingFactory:** covered primary slots 0-18, Init/Shutdown, destructor/deleting
  destruction, layout/format helpers, free-binding selection, creation, trimming and destruction.
  Public ABI audit found 38 owned vtable rows including service/thunk views; incidental shared-body
  aliases elsewhere are not extra owner algorithms. Remaining numeric, instance-zero and failure
  domains are TB5-TB7; no unconditional native-parity claim for them.
- **cS3DTextureBinding:** covered published constructor state, AddRef, Release and factory callback
  effect through `release_binding`; overflow is outside the claimed domain (TB5).
- **cS3DMaterialManager:** covered published slots 0-5 and deleting destruction; inherited base
  primitives are explicit. Its 27 owned vtable rows include unpublished primary slots 6/7. Those
  two bodies remain missing under TB4; no complete-manager credit.
- **cS3DMaterial:** published constructor/output state covered. Its remaining method inventory,
  reference lifecycle and held-binding release contract are missing under TB4; no complete-owner credit.

## Verification

- Main baseline was already gated; the task explicitly waived a redundant pre-edit full run.
- Focused `qbuild quick`: PASS, 2603 tests / 0 failed, dependents compile-only, native 206/206 edges
  (303.1s). It reported tree changes from the initial formatting/documentation checkpoint; the final
  run below used committed stable sources.
- Final `qbuild all` on code commit **facaae39**: PASS; workspace cargo **4321 passed / 0 failed**,
  clippy **0 diagnostics**, native build **2/2 edges**, full ctest **67/67** (163.0s).
- **30 new core tests + 2 parsed-S3D integration tests**. Independent expectations cover published
  size/format/LOD/bytes/stats/material vectors, every internal-format branch, byte division order,
  oldest reuse and eviction, exact pool limits, all five reuse fields, source/internal-format distinction,
  zero-size pooled records, unclamped upload dimensions and source-level order, cache type ignoring,
  resource/group quirks, restoration failure, partial creation cleanup, balanced service references,
  reinitialization, deleting flags, stale handles, old-version filters and last-ID-zero propagation.
- `qbuild fmt`: seven implementation/adapter/test files PASS. The additive shared `core/src/lib.rs`
  export was also checked; its existing module ordering/layout plus the append-only export yield fmt
  differences. Left shared formatting untouched as instructed; no mass reformat.
- Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-ctest.log`,
  `fc4verify-quick-test.log`, `fc4verify-quick-deps.log`. No game request, shadow or replacement claim
  for this unbound portable-core change. ABI exports unchanged.

## Questions / next steps

Appended **TB4-TB7** to `docs/specs/questions/texture-bindings.md`: missing owner/body contracts;
numeric widths/overflow/invalid geometry; keyed instance zero; driver/upload failure and restore order.
Code comments identify the provisional safety boundaries. Zero levels, unrepresentable binding sizes,
reference overflow and keyed instance zero are rejected; malformed uploads stop; aggregate counters use
wide arithmetic. These are explicit boundaries, not invented native behavior.

Coordinator: regenerate the question index using the permitted analysis/tooling lane (this lane cannot
run tools/), obtain the missing contracts, and reconcile the 58-body inventory before any blanket
complete-owner credit. Native interface binding and game validation remain separate work.

Commits: **4336efc6** implementation/questions; **7c730cac** checkpoint;
**facaae39** extra lifecycle and parsed-record vectors. Final report commit changes documentation only.

## Coordinator source-phase review

Bounded portable code/findings publication only; TB4-TB7 remain. No complete-owner, shadow or replacement credit is granted for this partial phase. Preserve the parked original worktree and existing round budget. Any engine change requires the joint full native gate and current default-union regression.
