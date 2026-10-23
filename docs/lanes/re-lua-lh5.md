| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lua-lh5: Lua thread and command-generator lifecycle | Codex / detached `re-lua-lh5` worktree | ready; doccheck passed | Coordinator publishes detached commits; core-lua-host5 implements LH4/LH5 contracts. |

## Report

Extended `docs/specs/lua-host.md` with thread lifecycle §§6.2.1–6.2.3, generator §§6.3.2.1–6.3.2.3,
and 24 independent conformance vectors in §16.1 (specified, not executed). Moved the complete LH4/LH5
entries to `docs/specs/questions/answered/lua-host.md`, marked DIFFERS, and regenerated the question
index. No open Lua-host entries remain. Thread milestone: `7b9a8c21`.

Thread results: constructors and shared zero/one-count destruction; main ID initially 0; monotonic IDs
surviving Close; registry roots/backpointers; CreateThread flags/class signature, one-value transfer,
owned output and factory/hook failure effects; borrowed GetThread; parent restrictions; status,
pause, destroy and resume error/result-count behavior. Registry Init rejection is hook rejection,
not a checked allocation-failure branch; retained owner and hook mutations survive without rollback.

Generator results: generic Init false; specialized Init retains endpoints/name, constructs two sets,
clears bookkeeping without enumerating commands; notifications install/remove callbacks; free indices
are reusable; callbacks convert arguments/results and return status first, or numeric 2 on failed
validation. Saved closures follow current index contents. Repeated Init does not first unregister;
Shutdown clears owned state even on false unregistration, while destruction does not unregister.

Evidence: read-only main-checkout `tools/re_query.py` export queries and class/unnamed decompilation.
Verified numeric widths, masks, counters, vtable order and callback constants against instructions
and original image bytes; relevant addresses are cited in the spec. REA was intentionally not used:
the brief reports no binary open, and the export supplied the required evidence. No REA error or
game-run result is claimed. Local ignored helper: `build-runs/read-pe.py`.

Validation: `python tools/questions.py index` succeeded; `python tools/doccheck.py check` passed;
`git diff --check` passed. Cargo/clippy/ctest and game/shadow tests: n/a for this RE documentation lane.
No engine reads/edits, executable vectors, ABI proposals/exports, plugin/capi changes or coverage-map
changes. No proposed ABI-row file. No new questions filed.

## Handoff
- Goal complete: LH4 and LH5 answered, no open LH rows.
- Next: coordinator publishes detached HEAD to reserved lane branch and lands queued RE docs.
- Implementer next: use the new sections and 24 vectors; actual conformance implementation remains engine work.
- Remaining limits: unchecked storage exhaustion and invalidating/reentrant lifetimes have no invented recovery guarantee.
- Queue uses actual full HEAD, kind `re`, state `ready`, no game runs, ABI false.
