| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-uiwin-close13 (base-window settled contracts) | `Codex` / `core-uiwin-close13` (engine lane) | ready; WT5 and WT10-WT12 settled contracts implemented; focused 40/40; cargo 7180/0, clippy clean, native build 261/261, ctest 81/81; ABI unchanged; no game runs; questions WT13, WT14 | 1. Land queued tip. 2. Resolve remaining spec gaps separately. |

## Implemented

Merged `core-uiwin-close11` first (`77657872e`), then `re-widgettk-wt13`
(`af47e8a3e`). Implementation commit: `6c07efc0e`; source stayed unchanged
through the final focused and full gates. All settled contracts assigned to these
owners are implemented in the core. Native vtable and explicitly unspecified
graphics/destructor details remain bounded by the questions below.

| Owner | Implemented | Result |
|---|---|---|
| cGZWin | `engine/core/src/ui_windows/base/` (`mod.rs`, `tree.rs`, `dispatch.rs`, `geometry.rs`, `paint.rs`) | Settled WT5, WT10-WT12 contracts and all previously specified base bodies; per-section code/test mapping in `engine/core/src/ui_windows/README.md` |
| cGZFadeTmpWin | `engine/core/src/ui_windows/base/paint.rs`; inherited lifetime in `base/mod.rs` and `base/tree.rs` | Specified constructor, StealBuffer and Plot; inherited settled base lifetime; derived destructor remains explicitly unspecified (WT13) |

Filters now traverse the live list, including same-pass appends and duplicates.
Deferred removals own references, survive message returns and nested passes, and
drain only after an outer paint filter pass. Active references are released before
the pending list's references. Consumed paint still drains; early rejection does not.
The specified sentinel-erasure corruption case is detected by an explicit panic;
the spec promises no stable continuation after that invalid operation.

Type-1 dispatch preserves the explicit slot-16 AL byte through `do_message_byte`
and `send_byte`; ordinary Boolean facades remain available. Constructors start
with zero references. QI preserves failure output and acquires a reference on
success. AddRef wraps; unsigned Release at zero or one calls slot 148(1) without
first changing the count. Deleting-wrapper bit zero controls allocation removal.

Busy manager requests queue raw pointers without detaching or acquiring ownership,
deduplicating windows covered by queued ancestors. Immediate destruction respects
caller and retired COW-block references. Last retired-block unlock can trigger final
destruction. ChildDeleteAll rereads the current head and intentionally retains the
specified nontermination behavior with a persistently busy manager. Raw parent
storage accepts self/unregistered pointers; unsafe later traversal is not made safe.

No plugin/C ABI wiring, ABI version or exports changed. No text-edit implementation
was edited. No coverage-map edits or game requests were made.

## Validation

- Final focused base suite: **40 passed, 0 failed** (14 new closure tests plus the existing suite). Covers all 21 original section-10 cases and every section-10.1 vector. Log: `build-runs/focused-final-console.log`.
- Formatting: **7 explicit owned Rust files, diff_lines=0**; fixes applied with apply_patch.
- Required stable-source `qbuild all`: **PASS**, run once. Workspace totals summed over all 45 result groups: **7180 passed, 0 failed, 0 ignored**. Clippy: **0 diagnostics**, `-D warnings`.
- Native verification was delegated by that same all gate to the coordinator queue as **qbuild-20261024-135625-build-ctest**; terminal **verdict: pass**. Build: **261/261 edges**, 109.5 s; CTest: **81/81**, 208.8 s; overall native verify: **PASS**, 322.9 s. No direct fc4verify invocation or duplicate check was launched.
- Evidence: `build-runs/all-console.log`, `qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. The asynchronous operation was collected to its terminal result.
- Game/shadow: **n/a**, pure-core implementation without plugin/C ABI integration; no outstanding requests.

## Remaining spec boundaries and questions

| Owner / section | Remaining contract | Question |
|---|---|---|
| cGZWin 1 | Own native vtable inventory | WT1 |
| cGZWin 2 | Untouched A0/A4 fields | WT2 |
| cGZWin 3 | Meanings of retained default bits 0x100 and 0x8000 | WT3 |
| cGZWin 8.1 | Initial manager instance-counter value (caller supplies it) | WT6 |
| cGZFadeTmpWin 12 | Derived destructor explicitly not specified | WT13, newly filed |
| cGZWin 12 and 14.5 | Opaque graphics service semantics and concrete slot-150 allocation internals | WT14, newly filed |

WT13 and WT14 were appended to `docs/specs/questions/widget-toolkit.md`.
WT5 and WT10-WT12 are settled by sections 4.2, 7.2-7.3, 8.1 and 10.1, with
no remaining implementation row blocked on those questions. The append-only
question file still contains the imported WT10-WT12 entries; the coordinator can
archive them as answered and regenerate the question index. Remaining questions
are future spec work, not missing settled core bodies.

## Next steps

Publish the detached tip to the reserved lane branch and land the READY queue entry.
Resolve the listed spec gaps in a later lane; no further work is pending for this
settled-contract implementation.
