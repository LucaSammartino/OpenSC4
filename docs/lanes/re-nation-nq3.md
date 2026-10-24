| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-nation-nq3: nation NQ1–NQ3 | `re-nation-nq3` / RE lane, detached HEAD | ready: NQ1–NQ3 answered; doccheck and questions check pass; no ABI changes or game runs | 1. Land specs. 2. core-nation10 implements the settled owner contracts. |

## Report

Extended nation.md with lifecycle, directory discovery/order, preference matching, add/delete, tutorial
reset/copy, index and callback-failure rules, safe malformed-input outcomes, and 23 owner-contract vectors.
Corrected demand.md §11.2's once-per-process tuning claim: a repeated live Init is a no-op, but Init after
Shutdown rereads tuning. Directory StaticInit alone is process guarded.

Important implementation changes relative to core-nation10's provisional contract:

- Same-index selection has no callbacks; ordinary switch callbacks may fail without rollback.
- Construction stores active index 0; Init chooses the preference or first/tutorial candidate itself.
- Shutdown clears/releases the catalog and curves. It retains the difficulty vector and directory guard.
- Filesystem operations preserve partial effects. Delete selects the target first, then its neighbor.
- Downloads exclusion is case insensitive; preference matching is a case-sensitive 64-byte display-name comparison.
- Explicit empty IDs return MissingCurve. Explicit short difficulty vectors return InvalidDifficulty only
  with a city. Those proposed core errors are adopted as replacement choices, not attributed to the original.

NQ1 and NQ2: DIFFERS. NQ3: MATCHES. Entries are preserved in
docs/specs/questions/answered/nation.md; the open file points there and the questions index is regenerated.
The base and main checkout lacked these entries; their authoritative text was read from core-nation10's
worktree. Its lane report's Open owner contracts was used because the closure table itself is under engine/,
which this lane was expressly forbidden to read. No engine files were read or changed.

Evidence: local re_query.py export and bounded decomp reads. ResetTutorialCities pseudocode is absent due
to an export decompilation failure; its instructions were traced in bounded pages through 0x00499D3F.
Numeric widths and economy failure accesses were verified against instruction bytes. No REA was queried
(the task states its session has no binary), no binary execution, and no external tools or agents were used.

Validation: `python tools/doccheck.py check` PASS; `python tools/questions.py check` PASS;
`git diff --check` PASS. Cargo, clippy, CTest and game/shadow runs: n/a, spec-only change.
No C ABI exports changed; no proposed ABI rows. No game requests are outstanding.
Commits succeeded on detached HEAD despite a nonfatal packed-refs.lock permission diagnostic; no shared
ref/config/hook was changed by this lane.

## Handoff

- Goal: close nation NQ1–NQ3 with behavior and conformance vectors; doccheck must pass.
- Done: initial handoff f6bee5d41; behavior and question closure b9eed76f1; final report and case-comparison clarification at tip.
- Source questions/report found under C:/Users/tlori/FreeCity4/.claude/worktrees/core-nation10/docs/.
- Owner closure table is under engine/, prohibited to this lane; used the lane report's Open owner contracts instead.
- Key corrections: same-index selection no-op; switch ignores failures; Shutdown clears catalog/curves; reinit rereads tuning.
- Next: coordinator lands the queued tip; core-nation10 consumes nation.md §§2.1, 5.1, 6–8.
- Read-only originals: C:/Users/tlori/FreeCity4; edits only in this worktree; no engine reads.
- No REA queries: task explicitly requires the local export.
