| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-persist-manager4: portable persistence manager | `core-persist-manager4` / detached reserved worktree (engine lane) | ready; code c430e2ae; cargo 4366/4366 PASS; clippy PASS; ctest 67/67 PASS; 40 independent scenarios PASS; PR5 open; ABI unchanged; no game runs | 1. Resolve PR5 before asserting native equivalence for enumeration after external key changes. 2. Coordinator reviews/integrates; native binding remains separate. |

## Implementation

Added `engine/core/src/persistence_manager/` from public persistence-manager.md
sections 1–11 and persistence.md's resource/key-list interface contracts.
Factories and type associations use separate duplicate-preserving chained hash
tables with the specified prime growth, rehash reversal and unsigned iteration.
The segment registry owns 32-element blocks and shorter-side erasure. Cache and
segment registries, dispatch precedence, reference holds/releases, failure
cleanup, generated keys, explicit registration, enumeration, direct records,
write-status quirks and lifecycle decisions execute in Rust.

The built-in cache implements full-key equality with instance hashing, filter
replacement, reference counting, lookup/insertion/erasure, enumeration, signed
maintenance threshold, bounded repeated purge, unsigned count-based eviction,
Shutdown sweeping and the destructor's intentional absence of resource Clear.
Manager Shutdown preserves type associations/key/service state and differs
from final destruction. PR4 Update uses canonical initialized logical OR and
visits every cache; native's undefined initial stack byte is never an oracle.

Foreign `Primitives` supplies only resource/factory/record/segment/filter and
external-cache interface operations. Built-in cache and manager algorithms are
not delegated. Lock observations retain native recursive scope/order, including
the specified mismatched locks; exclusive Rust borrows protect portable state.
These symbolic identities and scopes are not native ABI binding or a new
reentrant-mutation guarantee. No DBPF parsing receives manager owner credit.

`AUDIT.md` maps exactly the spec's 109 attributed function starts (8,437 native
bytes): mechanically checked 109 unique specified, 109 unique mapped, zero
missing/extra. Rust containers/ownership replace native storage layouts and
internal type-filter allocation. This checklist is not a runtime verification
claim; PR5's unresolved predicate input remains explicitly marked in code.

## Measured checks

- Focused qbuild quick: 3,685 tests passed, zero failed; dependent crates compiled; native build 4/4 edges passed.
- Final stable qbuild all on clean code commit `c430e2ae`: **4,366 workspace tests passed, 0 failed, 0 ignored; clippy exit 0 with 0 diagnostics; native build 4/4 edges passed; ctest 67/67 passed** (109.9 seconds for ctest).
- Dedicated conformance runner: all **PM01–PM30**, **PMU01–PMU06**, and EX01–EX04 pass as 40 separately named tests. JSON fixtures provide setup, primitive responses, operations, independent outputs, reference ledgers and callback/lock traces. These are spec-derived tests, not native/game captures.
- Six private core tests cover 32-slot block boundaries/order, three-duplicate growth reversal, PM19 null-only purge, signed/wrapped maintenance, virtual-Clear remainder sweeping, unsigned reference thresholds, generated-ID wrap and shutdown state retention.
- All eight new Rust files pass qbuild fmt; subsequent four-file corrective format check also passed. The required shared `core/src/lib.rs` check reports pre-existing module-order/long-line differences (first at line 32); its only lane change is the additive module export. No mass reformat applied.
- Earlier full run passed cargo/ctest but exposed three new clippy issues; callback alias/test-module ordering were corrected before the final successful stable gate.
- Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. No source edits or commits during the final gate. No ABI exports, CAPI, plugin, tools, maps or unreconciled rows changed. No game requests, shadow measurements or outstanding labels.

## Questions and limits

**PR5**, appended to `docs/specs/questions/persistence-manager.md`: after an
external resource SetKey, do enumeration predicates inspect the stored cache
entry key or the resource's current key? The spec distinguishes lookup/output
keys but does not name this input. The literal implementation filters stored
entry keys and appends current resource keys, with a code comment. Do not assert
native equivalence for this edge until PR5 is resolved. Question-index
regeneration belongs to the coordinator; this lane cannot run tools/ scripts.

This lane adds pure core code/conformance only. It supplies no live manager
replacement, C ABI, native critical-section adapter, shadow or game coverage.
The supervisor owns publishing this detached HEAD to the reserved lane branch;
main was not merged, edited or pushed.

## Commits and next steps

- `92576e9f`: owner implementation and PR5.
- `ec55b72d`: independent conformance inputs/runner, edge tests and exact body audit.
- `c430e2ae`: clippy corrections and explicit destroyed bucket-storage reclamation.

Resolve PR5, review the portable owner and integrate through the coordinator.
Native binding and game comparison are separate follow-up work; retain the
explicit PR4 policy rather than claiming a deterministic native initial byte.
