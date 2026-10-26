| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-rzs29 strings questions; kind docs | Codex / .claude/worktrees/re-rzs29 / detached HEAD (reserved re-rzs29) | blocked: coordinator STOP; RZS1 committed 6d6639953; RZS2 draft and handoff at HEAD; ABI unchanged, no reservations; direct doccheck passed before final refinements; questions check passed; game n/a, shadow n/a; RZS1 answered, RZS2 narrowed | 1. Review pending RZS2 wording below. 2. Run final doccheck/questions checks. 3. Finish and enqueue kind docs after STOP is lifted. |

## Handoff

- Two resume attempts after connection drops: git status was clean both times, but the coordinator STOP guard still rejected further research and validation. No source edits or checks ran during either resume attempt; the pending work below is unchanged. The STOP guard must be cleared before this lane can continue.
- STOP received 2026-10-26 during final RZS2 review; no merge-queue file written.
- RZS1: §7.1 and DT20–DT26 specify live iterators, safe accepted self-removal, unsupported false-return self-removal, other-node deletion, same-pass insertion, replacement, recursion and shared HTML mode. Archived and indexed.
- RZS2 draft: §§4, 5.2, 7.2; RZ66–RZ77 and DT27–DT30. C-locale tables, shipped wide classification ranges, partial-prefix reassignment, mapping-failure identity, unchanged lowered search offsets, and byte registry comparison are specified. Archived and indexed.
- Non-C case pairs depend on the active CRT locale and installed Windows NLS. §5.2 gives the exact remaining runtime observation; no native run or fixed host-Unicode equivalence is claimed.
- Evidence used main-checkout re_query.py, targeted decompilation ranges, and original PE classification/import data. REA was not used. No engine/, docs/abi/, game binaries or decompilation outputs were edited or staged.
- Pending correction: §5.2 currently says a failed setlocale request leaves the previous runtime locale. Replace that unverified assertion with the directly supported rule: language switching calls setlocale at 0x0092806D and does not propagate its failure; capture the actual CRT locale instead of deriving it from the stored language ID. The correction was blocked before application.
- Pending editorial correction: §5.2 runtime-observation paragraph references RZ70–RZ73; extend it to RZ70–RZ77 after the added vectors.
- Reviewed source correction already applied: CRT wide-wrapper support state 0 also attempts the ANSI bridge, while leaving probing unresolved; it does not immediately return zero. State 2 also bridges, state 1 directly calls W.
- Latest direct doccheck and questions check passed after the main RZS2 draft/index update, before the final fallback refinement and RZ75–RZ77 additions. Final validation is still required; do not treat those earlier results as checks of the final committed draft.
- q.py could not start Git Bash (CreateFileMapping error 5); direct PowerShell invocation of python tools/doccheck.py check succeeded. No REA failure occurred.
- Ignored helpers/messages are under build-runs/. No proposed ABI rows, C ABI changes, game runs or shadow measurements.
- Next: apply the two pending wording corrections, review RZS2 against the cited instructions, run python tools/questions.py index and check, python tools/doccheck.py check, and git diff --check; then complete the lane report and queue kind docs when resumed.
