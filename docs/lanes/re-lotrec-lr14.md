| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lotrec-lr14: construction lifetime and method inventory | Codex / `re-lotrec-lr14` (RE; detached HEAD) | in progress | Resolve LR14 and inventory non-Write construction methods; doccheck |

## Handoff
- Goal: answer LR14 from original executable evidence, add conformance vectors and a non-Write construction inventory.
- Done: merged glue-construction-lr13 at c6baf4b15; merge completed despite a packed-refs.lock permission diagnostic. Worktree clean afterwards.
- Done: 522470bed specifies teardown's uncleared field and exact bare vtable installation, callback ownership and required passive diagnostics.
- Done: §9.3 inventories non-Write construction methods, inherited surfaces and adjustment thunks; T74–T79 add six normative cases. LR14 archived with MATCHES; questions index regenerated.
- Gate: questions check, doccheck check and git diff --check all pass. No cargo/clippy/CTest/game runs for this RE-only change.
- Next: replace working handoff with final report, commit, write ready queue entry with full actual HEAD.
- Finding: Shutdown releases object+0x13C at 0x005D0F96 without nulling it; effect destructor reaches occupant teardown 0x005E8C40, which installs AA41BC at O+0x10. Run 470 timing is unproven.
- Owner closure table is in an engine module README (per core-construction5b report), prohibited to RE; LR13 lane report has no such table. Use public docs and native evidence, no engine access.
- Evidence: LR13 turbo run 470 failed (107/108 construction); normal run 471 passed (169/169). Neither is a new run from this lane.
- Constraints: no engine reads/edits, no ABI edits, no game runs; export/decomp only, REA unavailable by brief.
