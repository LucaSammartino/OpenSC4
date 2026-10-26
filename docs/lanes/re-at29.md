| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-at29 / AT25 construction continuation; kind docs (RE) | Codex / .claude/worktrees/re-at29 / detached HEAD; reserved branch re-at29 | ready; merge 8da76e628, spec 3d3e302a0; questions check and diff whitespace check passed (2/2); cargo/clippy/ctest n/a; game n/a; shadow n/a; no C ABI export edits by this docs lane; AT25 answered | 1. Engine lane implement/test §7.6.4–7.6.5. 2. Repeat full-house s1/s2 acceptance. |

- Merged automata-actions29 as requested, retaining its balanced slot correction unchanged.
- Corrected §7.6.1: GetPosition copies x/y/z and returns the supplied output pointer in EAX; callee pops 4 bytes, no x87 result.
- Added §7.6.4: construction ignores that EAX, reloads the saved primary interface and changes only the local position buffer's y before slot 8.
- Traced the first roof movement inside SetLot, separate daily/animation clocks, phase transitions, rebuild replay and final removal/Release.
- No additional concrete worker view or callback appears in this continuation with replacement manager creation/removal bodies.
- Added §7.6.5's D0–D0+8 crew/lifetime oracle; checked initial y bits 0x40E00831 and heights 8, 9, 10.5, 12, 13.5, 15, 11, 7 against instruction order and constants.
- +0x1CC3B3 dispatches slot 7 before the getter executes; reported EAX is the loaded vtable word, so getter return changes alone cannot fix that dispatch. No runtime root cause inferred.
- Moved AT25 to docs/specs/questions/answered/automata.md and regenerated the question index; questions check passed.
- Proposed interface-table correction: build-runs/abi-rows-re-at29.tsv (ignored); docs/abi/ unchanged.
- RE evidence came from main's export and decompilation; numeric widths/order checked in instruction bytes. REA was not needed.
- No engine files read or edited by this lane; the requested branch merge imported the engine lane's existing changes.
- No new game runs. Existing s1/s2 failures remain acceptance work for the engine lane; this docs result does not claim a runtime fix.
- Gate: questions.py check passed; git diff --check 8da76e628 HEAD passed. Code-build gates are n/a for the authored docs delta.
