| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-ef31 — effects EF27 (docs) | Codex / `.claude/worktrees/re-ef31` / reserved `re-ef31` (detached HEAD) | ready; spec commit `182a69dc6`; doccheck PASS; EF27 answered; C ABI exports unchanged; game/shadow n/a | 1. Land docs. 2. Effects-resource lane uses §18.9.7 and validates replacement payloads. |

EF27 from `replace-effectsres30` was imported with the requested checkout; its question already matched main.
`docs/specs/effects.md` §18.9.7 specifies grow `00587010`, insert `00589E80`, hidden iterator output, cleanup, EAX, layouts and native copy/allocator ownership.
Duplicate insertion follows the first exact match; spellings remain unchanged. Signed-byte hash, all 28 prime sizes, rechain order and writer order are specified.
EF27-C1–C5 are five proposed conformance fixtures, including retained `old`, two `MiXeD` indices, exact terminator and growth reversal.
Original EF27 moved to `docs/specs/questions/answered/effects.md`; regenerated index reports effects 0 open / 31 answered, next EF28. `questions.py find EF27` resolves the archive.
Proposed inventory rows: `build-runs/abi-rows-re-ef31.tsv` (3 rows, 13 columns verified); insertion's inferred void is corrected to result-storage pointer in EAX. No `docs/abi/` edits or C ABI export changes.
Gate: direct `python tools/doccheck.py check` PASS; `git diff --check` PASS. Cargo/clippy/ctest n/a (docs only).
Global `questions.py check` FAIL: seven pre-existing open/answered conflicts AP19, EX58, NM18–NM20, NO46–NO47; all eight involved files are unchanged from base `75a62e1c7`.
Static image-table/hash arithmetic checks PASS after correcting the helper's initial hand-calculated expectation for `mixed` from 84823 to 84855; no native execution.
The q.py Bash wrapper failed to launch (Win32 error 5); direct doccheck afterward passed. Commit emitted a packed-refs.lock permission error but actual detached HEAD `182a69dc6` and clean status were verified.
Sources: read-only export, decompilation and image constants; numeric details checked against instruction bytes. REA was not used.
Game runs: none; shadow n/a; no replacement acceptance or payload credit. Engine implementation and game validation remain with the effects-resource lane.
