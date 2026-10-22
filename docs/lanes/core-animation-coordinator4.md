| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-animation-coordinator4: cSC4AnimationCoordinator | Codex / `core-animation-coordinator4` (engine lane, detached HEAD) | wip; portable owner partial; cargo 4326 PASS, clippy 0 diagnostics, ctest 67/67; AC3/AC4 withdrawn, AC5–AC9 filed; no ABI/native/shadow promotion | 1. Resolve AC2 and AC5–AC9, publish the full owner catalogue. 2. Complete the owner and native contracts before promotion. |

## Result and coordinator correction

Implemented the published **cSC4AnimationCoordinator** occupant coordinator in
`engine/core/src/animation_coordinator/`. Its Rust owner makes its own registration,
filter, block visibility, zoom, power, tick-manager activation/removal, flag and
message decisions. The host supplies foreign data and primitive operations, not
coordinator decisions. No clock, timeline, play/pause, looping or frame scheduler
was invented.

This corrected task supersedes the earlier task's wrong `cS3DAnimationCoordinator`
identity and fictional scheduler requirements. **AC3/AC4 are withdrawn**, removed
from the question file, and are not RE contract gaps. Genuine existing AC1/AC2
were preserved verbatim. This report supersedes the earlier documentation-only
blocker report; that report remains only in Git history.

The result is **WIP**, not a complete-owner submission. The budget predicate is
explicitly unresolved, and native ownership, message decoding and the remaining
function catalogue are incomplete. No implementation-byte/function coverage,
whole-owner, native-reference, shadow, verification or replacement promotion is
claimed.

## Commits

- `1400b003`: actual Rust occupant owner, 29 core tests, withdrawn questions removed,
  genuine AC5–AC9 filed.
- `5abfa515`: eight independent executable JSON scenarios, dedicated conformance
  fixture/runner, and published function-reference inventory.
- `aa7e03f7`: name the literal filter-vector tuple type, resolving the sole new
  clippy diagnostic. This is the source commit used by the final passing full gate.
- Final report commit follows; no further source changes.

## Published owner inventory and exact residual

The full public-reference checklist is in
[the owner inventory](../../engine/core/src/animation_coordinator/README.md).
It lists **all 19 function references in the supplied spec**: 16 behavior/helper
references, two destructor references, and the model-maker creation context.
The inventory maps each to its Rust path and identifies partial/unimplemented
contracts. This is not a fabricated 45-method catalogue.

| Published responsibility | Implemented portable behavior | Residual |
|---|---|---|
| Construction/state | Initialized flag; zoom 4/minimum 2; enabled true; square grid, per-block sets/counts and total registration count | Native constructor/interface attribution AC9 |
| Init/Shutdown | Once-only Init, filters, service/subscription leases, all five raw subscriptions, city-x/16 grid, visible world-to-block conversion; unsubscribe/release/empty on Shutdown | Native filter/service/reference/deletion rules AC6; failures AC7 |
| Both filters | Either published model class, exclusion-property presence; shown filter adds flag 4 | No invented shown-filter call sites |
| Registration | Position/256 clamped to grid, per-block insertion/removal, duplicate-safe counts | Geometry AC5; moving/filter-changing occupants AC8 |
| Eligibility | Enabled/on, zoom >=2, visible block, own power property/slot-31 decision | **Budget predicate AC2 unimplemented**, returns explicit error without flag change |
| Activation/removal | Target IID 0x452294AA, add/remove with raw 0xC0, flag 0x20 transition only when necessary; preserve other bits | Native query/manager ownership AC6; failures/reentry AC7 |
| Block/all/target refresh | Own block traversal, all-block refresh, budget-target reevaluation | Order/failure choices AC7; registration semantics AC8 |
| Five messages | Power blocks; budget target; opaque all-block refresh; threshold/rotation-triggered view recompute; entering/leaving scroll refresh | **Native power-rectangle conversion and message payload/return decoding unimplemented**, AC5/AC8 |
| Global enabling byte | Literal byte >=2, only with registered occupants; refresh all when enabled changes | **Call sites unresolved AC8**; no guessed meaning for byte or opaque message AC1 |
| Destructors | Rust storage/owned-lease Drop | **Native body/deleting-destructor/refcount contracts unimplemented**, AC6/AC9 |

The coordinator-supplied target remains **45 attributed functions / 7726 bytes**.
The supplied public spec does not enumerate that target or provide per-entry byte
attribution. Exact remaining native function/byte counts cannot be reconciled
without AC9. Mapping a portable operation does not certify its native function
complete; no 16/45 or byte credit is asserted.

## Independent vectors and ownership limits

There are **29 core Rust test cases** and **eight language-neutral JSON scenarios**
under `engine/conformance/vectors/animation_coordinator_*.json`. Expected gates,
state and traces are literal constants, not outputs generated by this owner.

Cases cover model/property/flag filters, (300,700)->(1,2), square sizing/clamps,
offscreen occupants in visible blocks, zoom thresholds/rotation triggers, scroll
overlap and edge changes, rectangle-limited power loss/recovery, raw enabling-byte
boundaries, opaque refresh, duplicates/removal, target/manager failures, partial
Init cleanup, shutdown, primitive call order and lease release. All eight also pass
the workspace turbo-off replay. This owner has no animation clock.

AC2 vectors prove refusal to invent a budget predicate. Order, failure, edge and
reference tests explicitly identify their AC5–AC8 provisional choices. Lease tests
prove that portable acquired services/subscriptions/targets are released exactly
once, including failures; they **do not measure native COM AddRef/Release counts**.
Occupant IDs are borrowed identities. No borrowed native pointer is released.
State getters are immutable; no RefCell is used in owner state.

## Questions and choices

Question file: `docs/specs/questions/animation-coordinator.md`.

- AC1/AC2 preserved. AC1 stays opaque; AC2's missing exact budget test blocks completion.
- AC3/AC4 withdrawn because they arose solely from the coordinator's superseded task.
  Their IDs are not reused.
- AC5: rectangle units/edges/rounding and invalid/empty geometry.
- AC6: services/filters/subscriptions/occupants/targets, shutdown ownership and native deletion.
- AC7: failure rollback/return values, callback reentry/mutation and traversal/flag order.
- AC8: global-byte refresh triggers, native payloads, target registration and occupant movement.
- AC9: actual cSC4AnimationCoordinator full 45-entry attributed behavior catalogue.

Current choices are commented in code, tested as choices and recorded in the
inventory. Budget decisions are never delegated to a guessed provider. The global
byte and message 0x231BBF91 receive no inferred semantic labels in the core.
The generated questions index was not edited; coordinator regeneration should
retain the withdrawal history and choose AC10 next.

## Validation

Final stable-source `qbuild.py --wt <WT> all`: **PASS** at `aa7e03f7`.

- Workspace Cargo: **4326 passed, 0 failed, 0 ignored** (`build-runs/qbuild-test.log`).
- Clippy with `-D warnings`: **exit 0, 0 diagnostics** (`build-runs/qbuild-clippy.log`).
- Native CMake build: **4/4 edges**, 4.7 s (`build-runs/fc4verify-build.log`).
- Full CTest: **67/67 passed**, 157.4 s (`build-runs/fc4verify-ctest.log`).
- New Rust files: **fmt PASS, 3 files** (`build-runs/animation-fmt-new.log`).
  The six-touched-file fmt check reports shared lib/runner formatting; saved
  prior-tip copies also fail. No broad/shared-file reformat was introduced.
- Focused Cargo check: **3649 passed**, dependents compiled. Initial conformance
  turbo-off injection was fixed by accepting explicit off in the clockless fixture.
- Incremental native verification initially reported restat 1/4 with no compiler
  errors. Removing only this worktree's generated DLL/smoke executable forced
  real links; both full gates then passed native 4/4 and CTest 67/67.
- First full gate found one clippy type-complexity diagnostic in the literal test
  table; `aa7e03f7` fixed it. Final full gate above is the authoritative result.
- No pre-edit full baseline was repeated, per the corrected task's freshly gated
  main instruction. Sources were held stable throughout the final full gate.
- No game requests, no native binding or shadow key. Game verdict, ref_warnings
  and shadow exactness are **unmeasured**, not claimed clean/exact.
- No CAPI/plugin/tools/ABI/maps edits; no ABI export/version change. The generated
  `engine/tests/abi_entries.inc` was never manually edited or committed.

## Next steps

1. Spec writers resolve AC2 and AC5–AC9 and publish the full owner inventory.
2. Complete budget gating, exact triggers/geometry, native ownership/deletion and
   message contracts; replace the explicitly provisional choices with published rules.
3. Only then bind and measure runtime/shadow/reference behavior through a separately
   assigned glue task. Retain **WIP / no owner promotion** until completion.
