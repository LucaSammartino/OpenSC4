| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-lua-scriptserver4: ScriptServer owner contract | Codex / `re-lua-scriptserver4` (RE docs, detached worktree) | continue: contract complete; guard stopped queue write; audited 51 functions / 7700B; docs diff-check PASS; tests/game n/a; no questions/ABI changes | 1. Coordinator finalize ready queue at actual HEAD. 2. Implement §6.3; separately reconcile false 42B attribution. |

## Handoff
- Goal: complete ScriptServer owner specification before core implementation.
- Done: 0ff88e05 lifetime/control; 50a497be scheduling/arguments; edead1b4 breakpoints/notifications; cf71d0b2 final report and scope/whitespace check PASS.
- Contract is complete in lua-host.md §6.3; full report preserved below. Only two docs changed from starting commit 0ff88e05^.
- Guard exceeded context at queue-write call; that entire call was denied. No merge-queue file written.
- Exact next step: coordinator publish actual detached HEAD and create ready docs queue, abi false, game_runs empty; no further RE needed.
- No selfrestart/subagents/additional round requested. No engine/tools/ABI/coverage edits, questions, game requests or claims.
- 51 attributed functions / 7700B audited: 50 real / 7658B plus false 42B reach at 00677261 through literal arg at 00AC77E8.
- Inventory scratch: build-runs/owner-inventory-re-lua-scriptserver4.tsv. All critical address evidence is in committed spec.
- REA unavailable (target_unavailable); used export/decomp/native bytes. Final worktree was clean before handoff.

## Report

Completed the cGZLuaScriptServer contract in `docs/specs/lua-host.md` §6.3. Changes stay within that section
and this lane report. No engine, tools, ABI tables, coverage maps, HANDOFF or README edits.

Published primary and secondary interface views, QI/reference lifetime and deleting destruction; initialization,
failure latch and retry; source/file creation and foreign ID protocol; kill/pause/resume and signed counters;
live tick traversal and reentry; all eight breakpoint slots; enumeration, optional info outputs, current/last ID
and auto-yield; global arg userdata, registry-backed per-thread tables, line hooks; notification wire calls,
ownership and mutation order. Three independent input/state/result tables supply 31 specification vectors.
These are derived behavioral expectations, not executed conformance tests or native replay measurements.

The audit reconciles the requested **51 functions / 7700 bytes**. Of these, **50 / 7658B** are actual owner
bodies; the remaining 42B at `00677261` are attributed through the little-endian bytes of `arg` at
`00AC77E8`, with zero direct calls. The spec publishes this evidence and the complete address inventory;
no fictional callback is required. Folded/shared getters and storage helpers were inspected separately.
The raw local inventory is `build-runs/owner-inventory-re-lua-scriptserver4.tsv`; helpers and raw evidence
remain uncommitted under build-runs. No spec/implementation/shadow/verification/replacement coverage
promotion was made, and no ABI proposal file was needed.

Key compatibility results: Init failure requires Shutdown before factory retry; file-read failure returns zero
while source-run failure returns -1; display serial and VM-supplied ID differ; Resume can make pause counts
negative; arg.count is the next index (one for no tokens); breakpoint existence uses a removal flag;
SetCondition copies equal text only; scheduler breakpoint checks ignore enabled/condition; tick and explicit
kill use different DestroyThread/notification order. Native invalidating callback reentry has no safe deferred
control guarantee. Normal, non-invalidating owner decisions are fully specified for a core lane.

Evidence: Windows 1.1.641 export queried before class decompilation; native instruction bytes verify numeric
widths, constants, stack operations, control branches, folded getters and unnamed destruction/storage bodies.
Small PE constant reads confirmed `arg`, `l`, the empty string, 0.0 and the string-interface forwarding target.
REA batch returned `target_unavailable` (no app open); continued with export/decomp per workspace rules.
q.py could not start Bash (E_ACCESSDENIED), so capped direct PowerShell commands were used.

Validation: `git diff --check` PASS; final scope check against the starting commit covers exactly two docs.
Cargo, clippy, ctest and game runs n/a for spec-only work; no game requests, no shadow claims. Questions:
none filed or answered; no question-index regeneration needed. Git printed packed-refs.lock permission
warnings during detached commits, but each commit completed; no refs/config changes were attempted.

Milestones: `0ff88e05` lifetime/control; `50a497be` scheduling/arguments and attribution reconciliation;
`edead1b4` breakpoint/notification completion. Final report commit follows these milestones.

Next: implement §6.3 against deterministic foreign-service inputs. Separately repair the false 42B attribution
through the coordinator's normal tooling review; that correction is outside this lane's allowed edits.
